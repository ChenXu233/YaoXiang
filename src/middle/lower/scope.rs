//! 作用域与寄存器帧（`AstToIrGenerator` 的作用域/槽位族）
//!
//! 符号表栈、局部槽位分配（`cur_locals` / `next_temp` / `temp_high_water` /
//! 具名高水位）、全局槽位查询，以及 `note_*` written-type/量词事实的写入与读回。
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.1），语义未改。

use super::{AstToIrGenerator, LoopTargets, SymbolEntry};
use crate::ast;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{FrameId, Instruction, LocalSlot, Operand, ValueId, ValueInfo};
use crate::tlog;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::i18n::MSG;
use crate::util::span::Span;
use std::collections::HashMap;

/// 值表里「类型不可得」的**显式哨兵**（09a §P7 7b.2 契约 §二 第 2 条）。
///
/// `MonoType::TypeRef` 是 verify 既有「信息不足」口径（与具体类型占位不同），
/// 快照会原样渲染成 `TypeRef("?")`——不允许用 `Int(64)` 之类静默占位。
pub(super) const UNKNOWN_VALUE_TYPE: &str = "?";

/// 寄存器索引是 u8（middle/passes/codegen/operand.rs 的 OperandResolver 上限）：
/// 一个函数体的局部槽 + 临时槽总数不得超过它。
///
/// **唯一定义点**：预算闸只有 [`AstToIrGenerator::take_body_locals`] 一处，所有
/// 函数体形态（函数 / 方法 / lambda / 匿名绑定 / curry 层）都经那里取槽位表。
pub(super) const MAX_REGISTERS: usize = 255;

impl AstToIrGenerator {
    /// 进入新的作用域
    pub(in crate::middle) fn enter_scope(&mut self) {
        tlog!(debug, MSG::IrGenEnterScope, &self.symbols.len().to_string());
        self.symbols.push(HashMap::new());
        tlog!(debug, MSG::IrGenEnterScope, &self.symbols.len().to_string());
    }

    /// 退出当前作用域
    pub(in crate::middle) fn exit_scope(&mut self) {
        tlog!(debug, MSG::IrGenExitScope, &self.symbols.len().to_string());
        self.symbols.pop();
        tlog!(debug, MSG::IrGenExitScope, &self.symbols.len().to_string());
    }

    /// 注册局部变量
    pub(in crate::middle) fn register_local(
        &mut self,
        name: &str,
        local_idx: usize,
    ) {
        tlog!(
            debug,
            MSG::IrGenRegisterLocal,
            &name.to_string(),
            &local_idx.to_string()
        );
        if let Some(scope) = self.symbols.last_mut() {
            scope.insert(
                name.to_string(),
                SymbolEntry {
                    local_idx,
                    written_type: None,
                },
            );
        }
        // 把源码名写进当前函数的槽位表：槽位可能尚未扩展到 local_idx
        // （临时寄存器先于具名变量分配），先补齐空槽。
        while self.cur_locals.len() <= local_idx {
            self.cur_locals
                .push(LocalSlot::temp_in(MonoType::Int(64), self.cur_frame));
        }
        self.cur_locals[local_idx].name = Some(name.to_string());
        self.cur_locals[local_idx].scope_depth = self.symbols.len().saturating_sub(1);
        // 帧归属随名字一起写：槽位可能被本帧**覆盖**（内层用同名变量复用旧槽号），
        // 此时它已经属于本帧——不重盖会把本帧的槽误判成外层来的泄漏。
        self.cur_locals[local_idx].origin = Some(self.cur_frame);
    }

    /// 发放一个新生成帧号并切入（帧号的**唯一发放点**，WBS 2.1.4）。
    ///
    /// 只有 `NestedBodyGuard` 的两个构造函数调用它；入口点仍是那 7 处嵌套函数体
    /// （函数 / 方法 / lambda / 匿名绑定 / 柯里层 / spawn 体 / 模块级声明），
    /// 因此「帧边界」与「槽位表边界」是同一件事。
    fn alloc_frame(&mut self) -> FrameId {
        self.next_frame += 1;
        FrameId(self.next_frame)
    }

    /// 取出当前函数的局部槽位，补齐到 `total_locals` 长度后交给函数体。
    ///
    /// `register_local` 在生成期把源码名写进 `self.cur_locals`；
    /// 临时寄存器分配的槽位没有名字，这里补空槽保证长度覆盖全部槽位。
    /// 嵌套函数体在进入前已 `std::mem::take` 掉父函数的表，
    /// 因此这里拿到的一定是本函数自己的名字。
    pub(in crate::middle) fn take_cur_locals(
        &mut self,
        total_locals: usize,
    ) -> Vec<LocalSlot> {
        let mut slots = std::mem::take(&mut self.cur_locals);
        // 必须**恰好** total_locals 个：该长度即 local_count，
        // 多一个少一个都会改变字节码。register_local 会为槽位号较大的
        // 具名变量临时补空槽，而最终寄存器数以 total_locals 为准。
        // 补出来的空槽属于本帧（它们由本帧的寄存器预算产生），故带本帧号。
        slots.resize(
            total_locals,
            LocalSlot::temp_in(MonoType::Int(64), self.cur_frame),
        );
        slots
    }

    /// 查找局部变量
    pub(in crate::middle) fn lookup_local(
        &self,
        name: &str,
    ) -> Option<usize> {
        for scope in self.symbols.iter().rev() {
            if let Some(entry) = scope.get(name) {
                tlog!(
                    debug,
                    MSG::IrGenLookupLocal,
                    &name.to_string(),
                    &entry.local_idx.to_string()
                );
                return Some(entry.local_idx);
            }
        }
        tlog!(debug, MSG::IrGenLookupLocalNotFound, &name.to_string());
        None
    }

    /// 查找全局变量，返回**绝对槽位号**（`Operand::Global(idx)` 的值）。
    ///
    /// 先查跨文件布局（限定名 → 绝对槽位）；未命中则按本文件声明顺序，
    /// 基址由编排器分配（单文件为 0）。
    pub(in crate::middle) fn lookup_global(
        &self,
        name: &str,
    ) -> Option<usize> {
        if let Some(&idx) = self.global_slot_layout.get(name) {
            return Some(idx);
        }
        for (i, (var_name, _, _)) in self.global_vars.iter().enumerate() {
            if var_name == name {
                return Some(self.global_slot_base + i);
            }
        }
        None
    }

    /// `use lib.{v}` / `use lib.{v as w}` 的本地名解析到的**跨文件全局槽位**。
    ///
    /// 两表联查的唯一实现：`use_aliases`（本地名 → 限定名）取限定名，
    /// `global_slot_layout`（限定名 → 槽位号）取槽位。变量求值路径
    /// （[`Self::generate_var_expr_ir`] 的别名臂）与调用形态判据
    /// （[`Self::is_static_fn_name`]）**同经此处**——同一解析规则只有一份，
    /// 两条路径不会各自猜一个答案。
    ///
    /// 布局只收**值绑定**（`allocate_global_slots` 的判据：非函数），故导入的
    /// 函数定义（`use lib.{double}`）不命中此路——它按名进函数表，仍是静态调用。
    pub(in crate::middle) fn use_alias_value_slot(
        &self,
        name: &str,
    ) -> Option<usize> {
        self.use_aliases
            .get(name)
            .and_then(|qualified| self.global_slot_layout.get(qualified).copied())
    }

    /// 分配一个**具名**寄存器槽（形参 / env / 具名绑定）：`next_temp` 单调递增。
    ///
    /// 7b.4 起临时值不再经过这里——值槽由 [`super::regalloc`] 的线性扫描分配器
    /// 按活跃区间复用，故 `temp_high_water`（真高水位）自此是**具名**高水位，
    /// 也是分配器的具名基址 `named_base`。
    fn alloc_register_slot(&mut self) -> usize {
        let reg = self.next_temp;
        self.next_temp += 1;
        // 具名高水位只增不减（7b.4：值槽由分配器另算，不经过这里）
        // 最高占用为准（少一都会让尾部语句的临时落进未分配槽位）。
        self.temp_high_water = self.temp_high_water.max(reg + 1);
        reg
    }

    /// 分配一个**具名变量**的寄存器槽（`Operand::Local` 的槽号）。
    ///
    /// 与临时值的区别只是身份：本入口不产生 `ValueId`、不写值表。
    pub(in crate::middle) fn next_local_slot(&mut self) -> usize {
        self.alloc_register_slot()
    }

    /// 分配一个临时值：返回 `Operand::Value`，并在当前函数体的值表里登记一条
    /// `ValueInfo`（`ty` 先落显式未知，随后由 [`Self::note_value_type`] 或函数体
    /// 收尾的 [`infer_value_types`] 填真实类型）。
    ///
    /// `value_mode == false`（模块 init / 顶层语句，没有值表）时退化为旧的
    /// `Operand::Local` 槽分配——init 序列与其 codegen 路径零变化。
    pub(in crate::middle) fn next_temp_reg(&mut self) -> Operand {
        if !self.value_mode {
            return Operand::Local(self.alloc_register_slot());
        }
        let value = self.next_value;
        self.next_value += 1;
        self.cur_values.push(ValueInfo {
            ty: MonoType::TypeRef(UNKNOWN_VALUE_TYPE.to_string()),
            debug_name: None,
            defining_block: self.cur_block_label,
        });
        Operand::Value(ValueId(value))
    }

    /// 取 `Operand::Local` 的槽号。
    ///
    /// 7b.4 起值槽由分配器**晚分配**，`Value` 不再有「已占用的槽」可复用；
    /// 具名绑定复用值槽的路径改走 [`Self::pin_value_to_local_slot`]（值钉子）。
    pub(in crate::middle) fn slot_for_operand(
        &self,
        op: &Operand,
    ) -> Option<usize> {
        match op {
            Operand::Local(slot) => Some(*slot),
            _ => None,
        }
    }

    /// 把具名绑定钉在某个 SSA 值的槽上（`slot_for_operand` 耦合的正向替代）。
    ///
    /// 模式绑定 `s` 复用「值已占用的槽」：分配器把该值固定落具名槽 `S`，
    /// 于是 `Local(S)` 与 `Value(v)` 指向同一帧槽——**不新增指令、不改赋值形态**
    /// （09a §P7 7b.4 契约 §二 C）。`S` 由具名计数器发放，恒小于具名基址。
    pub(in crate::middle) fn pin_value_to_local_slot(
        &mut self,
        op: &Operand,
    ) -> Option<usize> {
        let Operand::Value(id) = op else {
            return None;
        };
        let slot = self.alloc_register_slot();
        self.cur_pins.push((*id, slot));
        Some(slot)
    }

    /// 给值表里的一条值补真实类型（只填仍为显式未知的项，先到先得）。
    pub(in crate::middle) fn note_value_type(
        &mut self,
        dst: &Operand,
        ty: MonoType,
    ) {
        let Operand::Value(id) = dst else {
            return;
        };
        let Some(info) = self.cur_values.get_mut(id.0 as usize) else {
            return;
        };
        if is_unknown_value_type(&info.ty) {
            info.ty = ty;
        }
    }

    /// 取走当前函数体的值表与「值 → 槽」映射，并在取走前做一次**前向类型推导**
    /// （09a §P7 7b.2 契约 §二 第 2 条）：只补确定类型的指令形态，其余保持显式
    /// 未知 `TypeRef("?")`。
    pub(in crate::middle) fn take_body_values(
        &mut self,
        instructions: &[Instruction],
        params: &[MonoType],
    ) -> Vec<ValueInfo> {
        let mut values = std::mem::take(&mut self.cur_values);
        infer_value_types(&mut values, instructions, params);
        values
    }

    /// 记下局部变量**写出**的类型名（注解原文或构造器名）。
    ///
    /// 只记「用户写出来的名字」，不重判类型：`x: TempB` 记 `TempB`，
    /// `u = TempB(2.0)` 记 `TempB`，`u = b.Temp(2.0)` 记 `b.Temp`。
    /// 「这个名字属于哪个单元」的判定只在 [`Self::imported_type_owner`] 一处。
    pub(in crate::middle) fn note_local_written_type(
        &mut self,
        name: &str,
        written: &str,
    ) {
        for scope in self.symbols.iter_mut().rev() {
            if let Some(entry) = scope.get_mut(name) {
                entry.written_type = Some(written.to_string());
                return;
            }
        }
    }

    /// 形参写出的类型名（注解原文）——与局部变量走同一条派发归因链。
    ///
    /// `declared` 是同一形参位在**声明**里的形参（`declaration_runtime_param_slots`）：
    /// parser 不再往实现头填声明类型（WBS 6.8.3），故第 i 位「写出的是什么类型名」
    /// 按「作者标注 → 声明同类位」取——与形参类型、调用侧可用名同一来源。
    pub(in crate::middle) fn note_param_written_types(
        &mut self,
        params: &[ast::Param],
        declared: &[ast::Param],
    ) {
        let written: Vec<(String, String)> = params
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let ty =
                    p.ty.as_ref()
                        .or_else(|| declared.get(i).and_then(|d| d.ty.as_ref()))
                        .and_then(Self::written_type_name_of)?;
                Some((p.name.clone(), ty.to_string()))
            })
            .collect();
        for (name, ty) in written {
            self.note_local_written_type(&name, &ty);
        }
    }

    /// 把量词事实写进**当前作用域**里该名字的槽位（`LocalSlot::quantity` 的唯一写入点）。
    ///
    /// 只写**已解析**的事实（`Some`）：`None` 是「尚未解析」，写进去不产生信息，
    /// 还会让下游把它误读成「已判定为非 q=0」。临时槽与无名来源因此保持 `None`。
    pub(in crate::middle) fn note_slot_quantity(
        &mut self,
        name: &str,
        quantity: Option<ast::Quantity>,
    ) {
        let Some(quantity) = quantity else {
            return;
        };
        let Some(slot) = self.lookup_local(name) else {
            return;
        };
        if let Some(local) = self.cur_locals.get_mut(slot) {
            local.quantity = Some(quantity);
        }
    }

    /// 实现头第 i 位 ↔ 声明运行期值位第 i 位（与 `note_param_written_types` **同一**
    /// 对齐口径）：把声明侧已解析的量词事实写到该形参的槽位上。
    ///
    /// 按位而不是按名：实现头可以改名（`f: (a: Int) -> Int = (b) => b`），名字不是
    /// 绑定身份，位次才是。
    pub(in crate::middle) fn note_param_quantities(
        &mut self,
        params: &[ast::Param],
        declared: &[ast::Param],
    ) {
        for (i, param) in params.iter().enumerate() {
            let quantity = declared.get(i).and_then(|d| d.quantity);
            self.note_slot_quantity(&param.name, quantity);
        }
    }

    /// 局部变量写出类型名的查找（作用域由符号表本身给出）。
    fn local_written_type(
        &self,
        name: &str,
    ) -> Option<&str> {
        for scope in self.symbols.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return entry.written_type.as_deref();
            }
        }
        None
    }

    /// 变量写出的类型名：先查局部作用域，再查本文件顶层绑定位。
    pub(super) fn binding_written_type(
        &self,
        name: &str,
    ) -> Option<&str> {
        self.local_written_type(name)
            .or_else(|| self.global_written_types.get(name).map(String::as_str))
    }
}

/// `finalize_code_body` 的四件产物：具名槽表、值表、值 → 槽映射、块图。
///
/// 名字化是为了让「收尾单点」的返回面自解释（`locals` 是帧槽表、`value_slots`
/// 是分配器给的映射，两者长度口径不同）。
pub(in crate::middle) struct FinalizedCodeBody {
    pub locals: Vec<LocalSlot>,
    pub values: Vec<ValueInfo>,
    pub value_slots: Vec<usize>,
    pub blocks: Vec<crate::middle::ir::ir::BasicBlock>,
}

impl AstToIrGenerator {
    /// 函数体收尾的**唯一入口**：值类型 → 块切分 → 槽分配 → 预算闸 → 取表。
    ///
    /// 6 处 `FunctionBody::Code` 构造点（函数 / 方法 / lambda / 匿名绑定 / curry
    /// 中间层 / curry 最内层）全部走这里——块切分、槽位分配、locals 取表三件事
    /// 不再在调用点各写一份（禁令三 A；09a §P7 7b.4 契约 §二 A/E）。
    ///
    /// - 值槽由 [`super::regalloc::allocate_value_slots`] 按活跃区间复用，
    ///   `frame_slots = named_base + max_live`；
    /// - 预算闸仍在这里报 E3014（`register_overflow` + `.at(cur_span)`）——
    ///   越界必须在 IR 层带 span，留到下游 `OperandResolver` 就只剩无 span 的
    ///   构造点（#271/#324）；
    /// - `value_mode=false`（模块 init / 顶层语句）不走本函数：没有值表也没有
    ///   块图，`take_cur_locals` 的旧口径继续服务 `ModuleIR.init_locals`。
    pub(in crate::middle) fn finalize_code_body(
        &mut self,
        instructions: Vec<Instruction>,
        params: &[MonoType],
    ) -> Result<FinalizedCodeBody, Diagnostic> {
        let span = self.cur_span;
        let values = self.take_body_values(&instructions, params);
        let blocks = crate::middle::ir::ir::partition_blocks(instructions);
        // 具名基址 = 本帧具名槽的上界：高水位与当前计数器取大（语句级回收
        // 可能把 next_temp 回滚到高水位之下，而任何具名分配都抬高两者之一）。
        let named_base = self.temp_high_water.max(self.next_temp);
        let pinned = std::mem::take(&mut self.cur_pins);
        let allocation = super::regalloc::allocate_value_slots(
            &blocks,
            &values,
            named_base,
            &pinned,
            MAX_REGISTERS,
        )
        .map_err(|budget| {
            ErrorCodeDefinition::register_overflow(
                &budget.required.to_string(),
                &budget.limit.to_string(),
            )
            .at(span)
            .build()
        })?;
        Ok(FinalizedCodeBody {
            locals: self.take_cur_locals(allocation.frame_slots),
            values,
            value_slots: allocation.value_slots,
            blocks,
        })
    }
}

fn is_unknown_value_type(ty: &MonoType) -> bool {
    matches!(ty, MonoType::TypeRef(name) if name == UNKNOWN_VALUE_TYPE)
}

fn string_type() -> MonoType {
    MonoType::Generic {
        name: "String".to_string(),
        args: Vec::new(),
    }
}

/// 操作数的可信类型：常量 / 参数 / 已定型的值；`Local`（lower 不写
/// `LocalSlot.ty`）与 `Global`（无注解占位）一律不可信，返回 `None`。
fn operand_value_type(
    op: &Operand,
    values: &[ValueInfo],
    params: &[MonoType],
) -> Option<MonoType> {
    match op {
        Operand::Const(c) => c.mono_type(),
        Operand::Arg(i) => params.get(*i).cloned(),
        Operand::Value(id) => values
            .get(id.0 as usize)
            .filter(|v| !is_unknown_value_type(&v.ty))
            .map(|v| v.ty.clone()),
        _ => None,
    }
}

/// 函数体收尾的**前向类型推导**：只补确定类型的指令形态，其余保持显式未知。
///
/// 规则集（契约 §二 第 2 条）：`Move`/`Load` 取 `src`、算术取 `lhs`、`Neg` 取
/// `src`；比较/逻辑 → `Bool`；字符串长度 → `Int(64)`；字符串拼接/取字符/
/// 数值转字符串 → `String`。调用、字段、索引等跨过程或依赖声明面的形态不猜，
/// 留在显式未知（`TypeRef("?")`）。
fn infer_value_types(
    values: &mut [ValueInfo],
    instructions: &[Instruction],
    params: &[MonoType],
) {
    for instr in instructions {
        let Some((dst, ty)) = infer_instruction_value_type(instr, values, params) else {
            continue;
        };
        let Operand::Value(id) = dst else {
            continue;
        };
        let Some(info) = values.get_mut(id.0 as usize) else {
            continue;
        };
        if is_unknown_value_type(&info.ty) {
            info.ty = ty;
        }
    }
}

fn infer_instruction_value_type<'a>(
    instr: &'a Instruction,
    values: &[ValueInfo],
    params: &[MonoType],
) -> Option<(&'a Operand, MonoType)> {
    match instr {
        Instruction::Move { dst, src, .. } | Instruction::Load { dst, src, .. } => {
            operand_value_type(src, values, params).map(|ty| (dst, ty))
        }
        Instruction::Add { dst, lhs, .. }
        | Instruction::Sub { dst, lhs, .. }
        | Instruction::Mul { dst, lhs, .. }
        | Instruction::Div { dst, lhs, .. }
        | Instruction::Mod { dst, lhs, .. }
        | Instruction::And { dst, lhs, .. }
        | Instruction::Or { dst, lhs, .. }
        | Instruction::Xor { dst, lhs, .. }
        | Instruction::Shl { dst, lhs, .. }
        | Instruction::Shr { dst, lhs, .. }
        | Instruction::Sar { dst, lhs, .. } => {
            operand_value_type(lhs, values, params).map(|ty| (dst, ty))
        }
        Instruction::Neg { dst, src, .. } => {
            operand_value_type(src, values, params).map(|ty| (dst, ty))
        }
        Instruction::Not { dst, .. }
        | Instruction::Eq { dst, .. }
        | Instruction::Ne { dst, .. }
        | Instruction::Lt { dst, .. }
        | Instruction::Le { dst, .. }
        | Instruction::Gt { dst, .. }
        | Instruction::Ge { dst, .. } => Some((dst, MonoType::Bool)),
        Instruction::StringLength { dst, .. } => Some((dst, MonoType::Int(64))),
        Instruction::StringConcat { dst, .. }
        | Instruction::StringGetChar { dst, .. }
        | Instruction::StringFromInt { dst, .. }
        | Instruction::StringFromFloat { dst, .. } => Some((dst, string_type())),
        _ => None,
    }
}

/// 被 [`NestedBodyGuard`] 摘下来的父帧。
///
/// `counters` 为 `None` 表示这次帧边界**不回滚**两项槽计数（声明面口径，见
/// [`NestedBodyGuard::enter_decl`]）；槽位表、值表、当前 span 与 `value_mode`
/// 一律回滚（值表/`next_value` 是**每个函数体独立**的，与槽计数的帧口径不同）。
struct SavedFrame {
    counters: Option<(usize, usize)>,
    cur_span: Span,
    cur_locals: Vec<LocalSlot>,
    cur_frame: FrameId,
    loop_stack: Vec<LoopTargets>,
    next_value: u32,
    cur_values: Vec<ValueInfo>,
    cur_pins: Vec<(ValueId, usize)>,
    value_mode: bool,
}

/// 嵌套函数体的寄存器帧 guard（WBS 7a.2）。
///
/// 一个**独立指令流**——函数 / 方法 / lambda / 匿名绑定 / curry 层——有自己一套
/// 寄存器帧：临时槽从本体基数起算，槽位表与循环栈从空开始。进入时把父帧整体摘进
/// 本 guard，离开时由 [`Drop`] 整体放回。
///
/// **类型强制**：帧的「摘走」只发生在两个构造函数里，「放回」只发生在 `Drop` 里。
/// guard 在生命周期内**独占借用**生成器（`&'a mut AstToIrGenerator`），本体代码
/// 只能经 `Deref` / `DerefMut` 触达它——作用域里不再有第二份 `&mut` 可用。于是
/// 「重置了却忘了恢复」「恢复写在 `?`/`return` 之后」这两类手工纪律都失去了可写的
/// 落点：恢复只有一份实现，且它在 guard 离开作用域时无条件执行。
#[must_use = "寄存器帧由 guard 持有：立即丢弃等于没有隔离，函数体必须在它上面生成"]
pub(super) struct NestedBodyGuard<'a> {
    gen: &'a mut AstToIrGenerator,
    saved: SavedFrame,
}

impl<'a> NestedBodyGuard<'a> {
    /// **本体帧入口**：摘走父帧（槽位表 / 循环栈 / 当前 span / 两项计数）。
    ///
    /// 父确实是一个帧（函数体 / 方法体 / lambda / 匿名绑定 / curry 层）时用它：
    /// 调用点随后按本体的口径把计数装到基态（见各调用点的注释），`Drop` 时父帧
    /// 原样放回——`next_temp` / `temp_high_water` 不再串到父体。
    pub(super) fn enter(gen: &'a mut AstToIrGenerator) -> Self {
        let saved = SavedFrame {
            counters: Some((gen.next_temp, gen.temp_high_water)),
            cur_span: gen.cur_span,
            cur_locals: std::mem::take(&mut gen.cur_locals),
            cur_frame: gen.cur_frame,
            loop_stack: std::mem::take(&mut gen.loop_stack),
            next_value: gen.next_value,
            cur_values: std::mem::take(&mut gen.cur_values),
            cur_pins: std::mem::take(&mut gen.cur_pins),
            value_mode: gen.value_mode,
        };
        gen.cur_frame = gen.alloc_frame();
        gen.next_value = 0;
        gen.value_mode = true;
        Self { gen, saved }
    }

    /// **声明面入口**：帧边界只覆盖槽位表 / 循环栈 / 当前 span，**两项计数不回滚**。
    ///
    /// 模块级声明（顶层函数、类型定义里的方法）用它：模块初始化序列不是函数体帧，
    /// 它的寄存器计数沿用既有口径——接着用子体的终值（WBS 7a.2 保持 C1 zero-diff；
    /// 计数的帧回滚只发生在父确实是一个帧的场合，见 [`Self::enter`]）。
    pub(super) fn enter_decl(gen: &'a mut AstToIrGenerator) -> Self {
        let saved = SavedFrame {
            counters: None,
            cur_span: gen.cur_span,
            cur_locals: std::mem::take(&mut gen.cur_locals),
            cur_frame: gen.cur_frame,
            loop_stack: std::mem::take(&mut gen.loop_stack),
            next_value: gen.next_value,
            cur_values: std::mem::take(&mut gen.cur_values),
            cur_pins: std::mem::take(&mut gen.cur_pins),
            value_mode: gen.value_mode,
        };
        gen.cur_frame = gen.alloc_frame();
        gen.next_value = 0;
        gen.value_mode = true;
        Self { gen, saved }
    }
}

impl std::ops::Deref for NestedBodyGuard<'_> {
    type Target = AstToIrGenerator;

    fn deref(&self) -> &Self::Target {
        self.gen
    }
}

impl std::ops::DerefMut for NestedBodyGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.gen
    }
}

impl Drop for NestedBodyGuard<'_> {
    fn drop(&mut self) {
        self.gen.cur_locals = std::mem::take(&mut self.saved.cur_locals);
        self.gen.cur_frame = self.saved.cur_frame;
        self.gen.loop_stack = std::mem::take(&mut self.saved.loop_stack);
        self.gen.cur_span = self.saved.cur_span;
        self.gen.next_value = self.saved.next_value;
        self.gen.cur_values = std::mem::take(&mut self.saved.cur_values);
        self.gen.cur_pins = std::mem::take(&mut self.saved.cur_pins);
        self.gen.value_mode = self.saved.value_mode;
        if let Some((next_temp, temp_high_water)) = self.saved.counters {
            self.gen.next_temp = next_temp;
            self.gen.temp_high_water = temp_high_water;
        }
    }
}
