//! 作用域与寄存器帧（`AstToIrGenerator` 的作用域/槽位族）
//!
//! 符号表栈、局部槽位分配（`cur_locals` / `next_temp` / `temp_high_water` /
//! `temp_floor`）、全局槽位查询，以及 `note_*` written-type/量词事实的写入与读回。
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.1），语义未改。

use super::{AstToIrGenerator, LoopTargets, SymbolEntry};
use crate::ast;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::LocalSlot;
use crate::tlog;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::i18n::MSG;
use crate::util::span::Span;
use std::collections::HashMap;

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
            self.cur_locals.push(LocalSlot::temp(MonoType::Int(64)));
        }
        self.cur_locals[local_idx].name = Some(name.to_string());
        self.cur_locals[local_idx].scope_depth = self.symbols.len().saturating_sub(1);
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
        slots.resize(total_locals, LocalSlot::temp(MonoType::Int(64)));
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

    /// 获取下一个临时寄存器编号
    pub(in crate::middle) fn next_temp_reg(&mut self) -> usize {
        let reg = self.next_temp;
        self.next_temp += 1;
        // #393：真高水位——语句级回收会回滚 next_temp，槽位总数以历史
        // 最高占用为准（少一都会让尾部语句的临时落进未分配槽位）。
        self.temp_high_water = self.temp_high_water.max(reg + 1);
        reg
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

impl AstToIrGenerator {
    /// 取本函数体的槽位表，并检查寄存器预算（E3014）——**唯一取表点**。
    ///
    /// 所有函数体形态（函数 / 方法 / lambda / 匿名绑定 / curry 层）都从这里取表，
    /// 因此预算闸覆盖全部形态、槽位数口径只有一份：本次生成期间的真高水位
    /// （`temp_high_water`）。越界必须在 IR 层带 span 报错——留到下游
    /// `OperandResolver` 就只剩无 span 的构造点（debug 直接 panic，见 #271/#324）。
    pub(in crate::middle) fn take_body_locals(&mut self) -> Result<Vec<LocalSlot>, Diagnostic> {
        let total_locals = self.temp_high_water;
        if total_locals > MAX_REGISTERS {
            return Err(ErrorCodeDefinition::register_overflow(
                &total_locals.to_string(),
                &MAX_REGISTERS.to_string(),
            )
            .at(self.cur_span)
            .build());
        }
        Ok(self.take_cur_locals(total_locals))
    }
}

/// 被 [`NestedBodyGuard`] 摘下来的父帧。
///
/// `counters` 为 `None` 表示这次帧边界**不回滚**三项计数（声明面口径，见
/// [`NestedBodyGuard::enter_decl`]）；表与当前 span 一律回滚。
struct SavedFrame {
    counters: Option<(usize, usize, usize)>,
    cur_span: Span,
    cur_locals: Vec<LocalSlot>,
    loop_stack: Vec<LoopTargets>,
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
    /// **本体帧入口**：摘走父帧（槽位表 / 循环栈 / 当前 span / 三项计数）。
    ///
    /// 父确实是一个帧（函数体 / 方法体 / lambda / 匿名绑定 / curry 层）时用它：
    /// 调用点随后按本体的口径把计数装到基态（见各调用点的注释），`Drop` 时父帧
    /// 原样放回——`next_temp` / `temp_high_water` / `temp_floor` 不再串到父体。
    pub(super) fn enter(gen: &'a mut AstToIrGenerator) -> Self {
        let saved = SavedFrame {
            counters: Some((gen.next_temp, gen.temp_high_water, gen.temp_floor)),
            cur_span: gen.cur_span,
            cur_locals: std::mem::take(&mut gen.cur_locals),
            loop_stack: std::mem::take(&mut gen.loop_stack),
        };
        Self { gen, saved }
    }

    /// **声明面入口**：帧边界只覆盖槽位表 / 循环栈 / 当前 span，**三项计数不回滚**。
    ///
    /// 模块级声明（顶层函数、类型定义里的方法）用它：模块初始化序列不是函数体帧，
    /// 它的寄存器计数沿用既有口径——接着用子体的终值（WBS 7a.2 保持 C1 zero-diff；
    /// 计数的帧回滚只发生在父确实是一个帧的场合，见 [`Self::enter`]）。
    pub(super) fn enter_decl(gen: &'a mut AstToIrGenerator) -> Self {
        let saved = SavedFrame {
            counters: None,
            cur_span: gen.cur_span,
            cur_locals: std::mem::take(&mut gen.cur_locals),
            loop_stack: std::mem::take(&mut gen.loop_stack),
        };
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
        self.gen.loop_stack = std::mem::take(&mut self.saved.loop_stack);
        self.gen.cur_span = self.saved.cur_span;
        if let Some((next_temp, temp_high_water, temp_floor)) = self.saved.counters {
            self.gen.next_temp = next_temp;
            self.gen.temp_high_water = temp_high_water;
            self.gen.temp_floor = temp_floor;
        }
    }
}
