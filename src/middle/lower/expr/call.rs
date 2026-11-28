//! 调用与存在类型变体分发
//!
//! `generate_call_expr_ir` 只做形态分派：一次调用点的实参面装在 `CallArgs` 里，
//! 六种调用形态各落一个 `emit_*`（分支 → 落点映射表见 `generate_call_expr_ir`
//! 的文档注释）；`CallArgs` 同时是实参收集 / 命名重排 / 形参槽位绑定的唯一载体。
//!
//! `emit_variant_dispatch` 是存在类型方法调用的变体分发生成器（RFC-011a §6.4）。
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast::Expr;
use crate::frontend::core::sema::MonoType;
use crate::frontend::core::types::identity::{DefPath, Nominal};
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::middle::lower::BindingInfo;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::plan_id::PlanId;
use crate::util::span::Span;

/// 检查类型是否实现了 Stringable 接口（即有 to_string 方法）
/// 用于 print/println 的零开销分发
fn type_implements_stringable(mono_type: &MonoType) -> bool {
    match mono_type {
        // 具体类型：检查方法表中是否有 to_string
        MonoType::Struct(struct_type) => struct_type.methods.contains_key("to_string"),
        // 基础类型默认都有字符串表示
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool | MonoType::Char => true,
        // #299：String/Bytes 现为 Generic，属可字符串化基础类型
        MonoType::Generic { name, .. } if matches!(name.as_str(), "String" | "Bytes") => true,
        // 其他类型使用兜底实现
        _ => false,
    }
}

/// 获取类型的字符串表示（用于兜底实现）
fn get_type_fallback_string(mono_type: &MonoType) -> String {
    match mono_type {
        MonoType::Void => "void".to_string(),
        MonoType::Bool => "bool".to_string(),
        MonoType::Int(_) => "int".to_string(),
        MonoType::Float(_) => "float".to_string(),
        MonoType::Char => "char".to_string(),
        MonoType::Struct(s) => s.name.bare().to_string(),
        MonoType::Enum(e) => e.name.bare().to_string(),
        MonoType::Generic { name, .. } => match name.as_str() {
            "Tuple" => "tuple".to_string(),
            "List" => "list".to_string(),
            "Dict" => "dict".to_string(),
            "Set" => "set".to_string(),
            "String" => "string".to_string(),
            "Bytes" => "bytes".to_string(),
            "Range" => "Range".to_string(),
            _ => "unknown".to_string(),
        },
        MonoType::Fn { .. } => "function".to_string(),
        MonoType::TypeRef(name) => name.bare().to_string(),
        // 其他类型使用默认名称
        _ => "unknown".to_string(),
    }
}

/// 一次调用点的**实参面**共用载体。
///
/// 六种调用形态——变体构造 / 内置容器构造 / 命名空间调用 / 跨文件全局槽位间接
/// 调用 / 方法调用重排 / 普通与重载调用——都从这一份载荷取实参：位置实参与命名
/// 实参的表达式序、调用节点 span、调用点身份（重载决议表的消费键），以及位置
/// 实参的寄存器（首次需要时求值一次，之后各形态共用）。
///
/// 收敛的是**搬运**而不是判定：实参 → 形参槽位的判定只有一处
/// （`frontend::core::sema::inference::arg_binding::bind_args_to_params`，六条规则
/// 全在那边），本类型只按它给出的形参位搬运寄存器。
pub(super) struct CallArgs<'a> {
    /// 位置实参表达式（书写序）
    args: &'a [Expr],
    /// 命名实参 `(名, 表达式)`（书写序）
    named: &'a [(String, Expr)],
    /// 调用节点 span：指令 span 与诊断定位同源
    span: Span,
    /// 调用点身份：重载决议表的消费键（RFC-039 D20）
    plan_id: PlanId,
    /// 位置实参寄存器（与 `args` 同序；`collect_regs` 之前为空）
    regs: Vec<Operand>,
}

impl<'a> CallArgs<'a> {
    /// 从调用节点的实参面构造（寄存器为空，等 `collect_regs` 求值）。
    pub(super) fn new(
        args: &'a [Expr],
        named: &'a [(String, Expr)],
        span: Span,
        plan_id: PlanId,
    ) -> Self {
        Self {
            args,
            named,
            span,
            plan_id,
            regs: Vec::new(),
        }
    }

    pub(super) fn args(&self) -> &'a [Expr] {
        self.args
    }

    pub(super) fn named(&self) -> &'a [(String, Expr)] {
        self.named
    }

    fn has_named(&self) -> bool {
        !self.named.is_empty()
    }

    pub(super) fn span(&self) -> Span {
        self.span
    }

    fn plan_id(&self) -> PlanId {
        self.plan_id
    }

    fn regs(&self) -> &[Operand] {
        &self.regs
    }

    /// 逐实参表达式：位置实参在前、命名实参按书写序——与 `bind_args_to_params`
    /// 返回的「逐实参形参位」同一坐标。
    fn arg_expr(
        &self,
        arg_pos: usize,
    ) -> &'a Expr {
        if arg_pos < self.args.len() {
            &self.args[arg_pos]
        } else {
            &self.named[arg_pos - self.args.len()].1
        }
    }

    /// 求值全部**位置实参**，寄存器按书写序留存（幂等：重复调用不再求值）。
    fn collect_regs(
        &mut self,
        gen: &mut AstToIrGenerator,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        if !self.regs.is_empty() {
            return Ok(());
        }
        for arg in self.args {
            let reg = gen.next_temp_reg();
            gen.generate_expr_ir(arg, reg.clone(), instructions, constants)?;
            self.regs.push(reg);
        }
        Ok(())
    }

    /// 命名实参 → 形参槽位：按形参位表把**已求值**的位置实参寄存器放进槽位，
    /// 位置实参覆盖不到的位由对应命名实参就地求值填入。
    ///
    /// 返回槽位向量（长度 = 形参表长度）；缺席槽位的处置归调用方——命名空间调用
    /// 丢弃缺席位（被调方自行处理可选位），普通函数调用补零占位。两者语义不同，
    /// 不在此合并。
    ///
    /// 前置：调用方已 `collect_regs`（`regs` 与 `args` 等长）。
    fn bind_named_slots(
        &mut self,
        gen: &mut AstToIrGenerator,
        slots_len: usize,
        positions: &[usize],
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Vec<Option<Operand>>, Diagnostic> {
        let mut slots: Vec<Option<Operand>> = vec![None; slots_len];
        for (arg_pos, pos) in positions.iter().enumerate() {
            if *pos >= slots.len() {
                continue; // 超签实参：由类型检查阶段报 E1010
            }
            let reg = if arg_pos < self.regs.len() {
                self.regs[arg_pos].clone()
            } else {
                let arg_reg = gen.next_temp_reg();
                gen.generate_expr_ir(
                    self.arg_expr(arg_pos),
                    arg_reg.clone(),
                    instructions,
                    constants,
                )?;
                arg_reg
            };
            slots[*pos] = Some(reg);
        }
        Ok(slots)
    }

    /// 按**槽位序**逐实参求值：形参位表 → 实参下标序（`slot_order_arg_indices`），
    /// 每个实参求值一次，返回按形参声明序排列的寄存器。
    fn collect_regs_in_slot_order(
        &mut self,
        gen: &mut AstToIrGenerator,
        positions: &[usize],
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Vec<Operand>, Diagnostic> {
        let mut regs = Vec::with_capacity(positions.len());
        for arg_pos in AstToIrGenerator::slot_order_arg_indices(positions) {
            let reg = gen.next_temp_reg();
            gen.generate_expr_ir(self.arg_expr(arg_pos), reg.clone(), instructions, constants)?;
            regs.push(reg);
        }
        Ok(regs)
    }

    /// 命名实参槽位必须**全部**被覆盖：收下槽位向量并按形参声明序落成寄存器。
    ///
    /// 与 `bind_named_slots` 的分工：那边只按形参位搬运，缺席位**合法**——命名
    /// 空间调用丢弃缺席位、由被调方自行处理可选位（`assert.assert(cond = ...)`
    /// 省略 `?msg` 就走那条）。本函数服务普通函数调用路径，那条路径的形参名表
    /// 不含可选位（`fn_param_names` 与导出 `param_names` 的必需下限都是表长），
    /// 所以任何缺席位都意味着类型检查与 lowering 判定不一致。
    pub(super) fn require_complete_slots(
        &mut self,
        key: &str,
        param_names: &[String],
        slots: Vec<Option<Operand>>,
    ) -> Result<(), Diagnostic> {
        let missing: Vec<usize> = slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.is_none())
            .map(|(index, _)| index)
            .collect();
        if let Some(&first) = missing.first() {
            let param = param_names
                .get(first)
                .map(String::as_str)
                .unwrap_or("<超签位>");
            return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "调用 {} 的实参槽位未覆盖：{} 个形参位缺席（首个槽位 {}，形参 {}）——类型检查与 lowering 判定不一致",
                key,
                missing.len(),
                first,
                param
            ))
            .at(self.span)
            .build());
        }
        self.regs = slots.into_iter().flatten().collect();
        Ok(())
    }

    /// 用重排结果替换位置实参寄存器（结构体构造器按字段名重排后使用）。
    fn set_regs(
        &mut self,
        regs: Vec<Operand>,
    ) {
        self.regs = regs;
    }

    /// 追加一个已求值的构造槽位（结构体构造器补默认值）。
    fn push_reg(
        &mut self,
        reg: Operand,
    ) {
        self.regs.push(reg);
    }

    /// 取出寄存器交给 `Instruction::Call` / `CallDyn`。
    fn take_regs(&mut self) -> Vec<Operand> {
        std::mem::take(&mut self.regs)
    }
}

impl AstToIrGenerator {
    /// RFC-011a §6.4: 生成变体分发的 IR——
    ///   tag = VariantTag(obj)
    ///   逐变体: tag == i → payload = VariantPayload(obj) → Call 具体方法
    /// 调用目标解析复用 RFC-004 绑定表（声明形式与 `Type.method = fn[n]`
    /// 重绑定形式都支持），无绑定时静态 `Type.method`。
    #[allow(clippy::too_many_arguments)]
    fn emit_variant_dispatch(
        &mut self,
        iface: &str,
        method: &str,
        obj_reg: Operand,
        method_args: &[Operand],
        result_reg: Operand,
        span: Span,
        instructions: &mut Vec<Instruction>,
        _constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        let variants = self
            .interface_variants
            .get(iface)
            .cloned()
            .unwrap_or_default();
        if variants.is_empty() {
            // 编译期响亮失败：无实现收集（典型于跨模块存在类型——v1 边界）
            return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "接口 {} 在本编译单元无实现证明，无法生成变体分发",
                iface
            ))
            .build());
        }
        let group = format!("{}$Group", iface);

        let tag_reg = self.next_temp_reg();
        instructions.push(Instruction::VariantTag {
            dst: tag_reg.clone(),
            obj: obj_reg.clone(),
            group: group.clone(),
            span,
        });

        let mut end_jumps: Vec<usize> = Vec::new();
        for (i, impl_type) in variants.iter().enumerate() {
            // tag == i ?
            let const_reg = self.next_temp_reg();
            instructions.push(Instruction::Load {
                dst: const_reg.clone(),
                src: Operand::Const(ConstValue::Int(i as i128)),
                span: self.cur_span,
            });
            let eq_reg = self.next_temp_reg();
            instructions.push(Instruction::Eq {
                dst: eq_reg.clone(),
                lhs: tag_reg.clone(),
                rhs: const_reg.clone(),
                span: self.cur_span,
            });
            let next_idx = instructions.len();
            instructions.push(Instruction::JmpIfNot {
                cond: eq_reg.clone(),
                target: 0,
                span: self.cur_span,
            });

            // 解包负载
            let payload_reg = self.next_temp_reg();
            instructions.push(Instruction::VariantPayload {
                dst: payload_reg.clone(),
                obj: obj_reg.clone(),
                group: group.clone(),
                span,
            });

            // 调用目标：RFC-004 绑定重排优先，回退静态 Type.method
            let binding = self
                .type_bindings
                .get(impl_type)
                .and_then(|b| b.get(method).cloned());
            match binding {
                Some(b) => {
                    let total = b.positions.len() + method_args.len();
                    let positions: Vec<i64> = b
                        .positions
                        .iter()
                        .map(|&p| if p < 0 { p + total as i64 } else { p })
                        .collect();
                    let mut final_args: Vec<Operand> = Vec::with_capacity(total);
                    let mut extra = method_args.iter().cloned();
                    for pos in 0..total {
                        if positions.contains(&(pos as i64)) {
                            final_args.push(payload_reg.clone());
                        } else if let Some(a) = extra.next() {
                            final_args.push(a);
                        }
                    }
                    let func_name = self
                        .registry
                        .short_to_qualified_map()
                        .get(&b.function)
                        .cloned()
                        .unwrap_or_else(|| b.function.clone());
                    instructions.push(Instruction::Call {
                        dst: Some(result_reg.clone()),
                        func: Operand::Const(ConstValue::String(func_name.to_string())),
                        args: final_args,
                        span,
                        def: None,
                    });
                }
                None => {
                    let mut final_args = vec![payload_reg.clone()];
                    final_args.extend(method_args.iter().cloned());
                    instructions.push(Instruction::Call {
                        dst: Some(result_reg.clone()),
                        func: Operand::Const(ConstValue::String(format!(
                            "{}.{}",
                            impl_type, method
                        ))),
                        args: final_args,
                        span,
                        def: None,
                    });
                }
            }

            let end_idx = instructions.len();
            instructions.push(Instruction::Jmp {
                target: 0,
                span: self.cur_span,
            });
            end_jumps.push(end_idx);

            // 回填：tag != i → 下一变体臂
            let next_target = instructions.len();
            if let Instruction::JmpIfNot {
                cond: _,
                ref mut target,
                span: _,
            } = instructions[next_idx]
            {
                *target = next_target;
            }
        }

        // 兜底臂：tag 不在实现集合内。VariantTag 是**运行期**取值，静态没有
        // 「tag 恒在集合内」的守卫（此处原先的注释正是这个错误假设）：末臂
        // JmpIfNot 失败时直接落到汇合点，结果槽从未被写——`s = a.speak()`
        // 这类把具名槽直接交给分发链的构造因此触犯支配性不变量
        // （verify_loose: 读取槽位 Local(n) 存在无定义路径）。
        //
        // 修复口径：未知 tag 路径也**定义结果槽**，并且不静默返回陈旧值——
        // 走既有「Never 返回的 native 即陷阱」的写法（`std.assert.assert` 声明
        // 返回 Never，`Never <: T` 使其可写进任意结果槽；IR 层的 Call dst 即
        // 一次定义）。失败点在运行期、消息写明接口与方法，不新增诊断码，
        // 也不改动任何命中分支的指令与顺序。
        let false_reg = self.next_temp_reg();
        instructions.push(Instruction::Load {
            dst: false_reg.clone(),
            src: Operand::Const(ConstValue::Bool(false)),
            span,
        });
        let msg_reg = self.next_temp_reg();
        instructions.push(Instruction::Load {
            dst: msg_reg.clone(),
            src: Operand::Const(ConstValue::String(format!(
                "interface dispatch: unknown variant tag for {}::{}",
                iface, method
            ))),
            span,
        });
        instructions.push(Instruction::Call {
            dst: Some(result_reg.clone()),
            func: Operand::Const(ConstValue::String("std.assert.assert".to_string())),
            args: vec![false_reg.clone(), msg_reg.clone()],
            span,
            def: None,
        });

        // 汇合点：命中臂的 Jmp 跳过兜底臂，末臂 JmpIfNot 正落在兜底臂入口。
        let end = instructions.len();
        for idx in end_jumps {
            if let Instruction::Jmp {
                ref mut target,
                span: _,
            } = instructions[idx]
            {
                *target = end;
            }
        }
        Ok(())
    }

    /// 生成 Call 表达式 IR：**只做形态分派**，各形态的生成器是六个 `emit_*`。
    ///
    /// `call_plan_id` = **本调用节点的身份**（`Expr::Call::plan_id`，RFC-039 D20）：
    /// 重载决议表的消费键就是它——与 typecheck 产出侧读同一个节点上的同一个值。
    ///
    /// | 分支判据（自上而下，先命中先返回） | 落点 |
    /// | --- | --- |
    /// | `detect_variant_ctor_call` 命中（RFC-010） | `emit_variant_ctor` |
    /// | 实参位是内置容器构造器（RFC-011 §Vec） | `emit_builtin_container_ctor`（`container.rs`） |
    /// | 接收者是命名空间，且限定名落跨文件全局槽位（WBS 3.6.4） | `emit_global_slot_indirect_call` |
    /// | 接收者是命名空间 | `emit_namespace_call` |
    /// | RFC-004 类型绑定表命中 | `emit_method_call_reordered` |
    /// | 其余（方法无绑定 / 函数形态） | `emit_plain_call` |
    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_call_expr_ir(
        &mut self,
        func: &Expr,
        args: &[Expr],
        named_args: &[(String, Expr)],
        span: &Span,
        call_plan_id: PlanId,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        let mut call = CallArgs::new(args, named_args, *span, call_plan_id);

        // 变体构造（RFC-010）。
        if let Some((type_name, variant_index, _)) = self.detect_variant_ctor_call(func) {
            return self.emit_variant_ctor(
                type_name,
                variant_index,
                &call,
                result_reg,
                instructions,
                constants,
            );
        }

        // 内置容器构造（RFC-011 §「Vec(T) 的构造形式」）——三条形态与类型实参擦除
        // 规则见 `emit_builtin_container_ctor`。
        if let Expr::Call { func: inner, .. } = func {
            if self.builtin_container_ctor_name(inner).is_some() {
                return self.emit_builtin_container_ctor(
                    &call,
                    result_reg,
                    instructions,
                    constants,
                );
            }
        }

        // 方法调用统一转换为普通函数调用（`p.method()` → `method(p)`）：接收者不是
        // 命名空间时才补隐式接收者（见 `emit_namespace_call` 与 `emit_plain_call`）。
        if let Expr::FieldAccess { expr, field, .. } = func {
            if self.is_namespace_receiver(expr) {
                // 命名空间限定名若落**跨文件全局槽位**，它是顶层值绑定而不是函数表
                // 条目，改走间接调用（见 `emit_global_slot_indirect_call`）。
                if call.named().is_empty() {
                    if let Some(slot) = self
                        .resolve_namespace_data_access(expr, field)
                        .and_then(|qualified| self.global_slot_layout.get(&qualified))
                        .copied()
                    {
                        return self.emit_global_slot_indirect_call(
                            slot,
                            &mut call,
                            result_reg,
                            instructions,
                            constants,
                        );
                    }
                }
                return self.emit_namespace_call(
                    expr,
                    field,
                    &mut call,
                    result_reg,
                    instructions,
                    constants,
                );
            }
            // RFC-004 绑定表命中：绑定位置决定接收者落在哪个形参槽（见
            // `emit_method_call_reordered`）。
            if let Some(binding) = self.get_expr_struct_type_name(expr).and_then(|type_name| {
                self.type_bindings
                    .get(&type_name)
                    .and_then(|bindings| bindings.get(field).cloned())
            }) {
                return self.emit_method_call_reordered(
                    expr,
                    field,
                    binding,
                    &mut call,
                    result_reg,
                    instructions,
                    constants,
                );
            }
        }

        // 其余：不带 RFC-004 绑定的方法调用与函数形态（见 `emit_plain_call`）。
        self.emit_plain_call(func, &mut call, result_reg, instructions, constants)
    }

    /// 形态①：变体构造（RFC-010）——`Result(Int, String).ok(5)` / `Color.red()`。
    ///
    /// 整个调用替换为 CreateVariant：载荷求值进寄存器（多载荷先打包 Tuple、
    /// 零载荷用 Void 哨兵），group 携带和类型名（解释器 intern 为类型身份），
    /// variant 为声明序号。
    fn emit_variant_ctor(
        &mut self,
        type_name: String,
        variant_index: usize,
        call: &CallArgs<'_>,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        let payload_count = call.args().len();
        let payload_reg = self.next_temp_reg();
        if payload_count == 0 {
            instructions.push(Instruction::Load {
                dst: payload_reg.clone(),
                src: Operand::Const(ConstValue::Void),
                span: call.span(),
            });
        } else if payload_count == 1 {
            self.generate_expr_ir(
                &call.args()[0],
                payload_reg.clone(),
                instructions,
                constants,
            )?;
        } else {
            // 多载荷打包 Tuple（与 match 变体模式的多参解包对称）
            let mut elem_regs = Vec::with_capacity(payload_count);
            for a in call.args() {
                let r = self.next_temp_reg();
                self.generate_expr_ir(a, r.clone(), instructions, constants)?;
                elem_regs.push(r.clone());
            }
            instructions.push(Instruction::NewTuple {
                dst: payload_reg.clone(),
                items: elem_regs,
                span: call.span(),
            });
        }
        instructions.push(Instruction::CreateVariant {
            dst: result_reg.clone(),
            group: type_name,
            variant: variant_index as u32,
            payload: payload_reg.clone(),
            span: call.span(),
        });
        Ok(())
    }

    /// 形态③：命名空间调用——`std.io.println(...)` / `list.push(...)`。
    ///
    /// 方法调用统一转换成普通函数调用（`p.method()` → `method(p)`），但接收者是
    /// 命名空间（std 子模块 / 用户模块别名）时**不加隐式接收者**：实参原样按书写序
    /// 传给限定名。
    ///
    /// 命名参数（`list.push(list = v, item = 9)`）在命名空间路径上同样要按形参名
    /// 重排，否则实参一个不进——与普通函数调用同类缺口。
    fn emit_namespace_call(
        &mut self,
        expr: &Expr,
        field: &str,
        call: &mut CallArgs<'_>,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 命名空间调用：不需要隐式参数
        //
        // 命名参数（`list.push(list = v, item = 9)`）在命名空间路径上
        // 同样要按形参名重排，否则实参一个不进——与普通函数调用同类
        // 缺口（那边在 6.5k 行修，这里是它的镜像）。
        call.collect_regs(self, instructions, constants)?;
        let method_function_name = self.resolve_field_path(expr, field);

        if call.has_named() {
            let leaf = method_function_name
                .rsplit('.')
                .next()
                .unwrap_or(&method_function_name);
            // 本模块表 → 跨模块导出携带的 `param_names`
            if let Some(info) = self.lookup_param_info(&method_function_name, leaf) {
                let names = &info.names;
                // t12-F1 / t14：槽位判定收敛到共享判据（此前是与另一处互抄的第二份
                // 内联副本，槽冲突报旧码 E1015、缺规则④）；可选位与形参名同出一源。
                let optional: Vec<bool> =
                    (0..names.len()).map(|i| i >= info.required_min).collect();
                let positions =
                    crate::frontend::core::sema::inference::arg_binding::bind_args_to_params(
                        &method_function_name,
                        names,
                        call.args(),
                        call.named(),
                        0, // 命名空间调用无接收者位
                        &optional,
                        call.span(),
                    )?;
                let slots =
                    call.bind_named_slots(self, names.len(), &positions, instructions, constants)?;
                // 缺席槽位丢弃：被调方自行处理可选位缺席
                call.set_regs(slots.into_iter().flatten().collect());
            }
        }
        instructions.push(Instruction::Call {
            dst: Some(result_reg.clone()),
            func: Operand::Const(ConstValue::String(method_function_name.to_string())),
            args: call.take_regs(),
            span: call.span(),
            def: None,
        });
        Ok(())
    }

    /// 形态④：命名空间限定名落在**跨文件全局槽位**时的间接调用（WBS 3.6.4）。
    ///
    /// 这类名字是顶层值绑定（函数类型值）、不是函数表条目——按名 `Call` 在运行期
    /// 查扁平函数表落空（`E6006 Native function not found: lib.f`）。改走间接调用：
    /// 读槽位 → `CallDyn`。限定名解析与槽位查询与 `generate_field_access_expr_ir`
    /// 的命名空间数据访问**同一分支**（`resolve_namespace_data_access` +
    /// `global_slot_layout` 两表联查），调用机制与闭包/函数值调用同一实现
    /// （`CallDyn`），不新增解析表。调用点无命名实参时才可能是这种形态——命名
    /// 实参要按形参名重排，只能走 `emit_namespace_call`。
    fn emit_global_slot_indirect_call(
        &mut self,
        slot: usize,
        call: &mut CallArgs<'_>,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        call.collect_regs(self, instructions, constants)?;
        let func_reg = self.next_temp_reg();
        instructions.push(Instruction::Load {
            dst: func_reg.clone(),
            src: Operand::Global(slot),
            span: call.span(),
        });
        instructions.push(Instruction::CallDyn {
            dst: Some(result_reg.clone()),
            func: func_reg.clone(),
            args: call.take_regs(),
            span: call.span(),
        });
        Ok(())
    }

    /// 形态⑤：方法调用重排（RFC-004 绑定表）。
    ///
    /// `obj.method(arg1, arg2)` + 绑定位置 [0] → `original(obj, arg1, arg2)`；
    /// 绑定位置 [1] → `original(arg1, obj)`。位置是**被绑函数**的形参位，负索引
    /// 按元数归一化（[-1] = 最后一个参数）。
    ///
    /// 方法级重载决议命中时 `binding.function`（后定义覆盖的单值）替换为决议出的
    /// 混编名；命名实参调用同样消费决议（消费侧不带形态门）。
    #[allow(clippy::too_many_arguments)]
    fn emit_method_call_reordered(
        &mut self,
        expr: &Expr,
        field: &str,
        binding: BindingInfo,
        call: &mut CallArgs<'_>,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // RFC-011a §3：方法级重载决议命中时，binding.function
        //（后定义覆盖的单值）替换为决议出的混编名
        let binding = if let Some(resolution) = self.overload_resolutions.get(call.plan_id()) {
            // RFC-039 D20：按**调用点身份**消费重载决议（命中即记账）。
            // #452：命名实参调用同样消费——决议已把命名实参计入匹配，
            // 这里的名字必须真的被用到（消费侧不得再带形态门）。
            BindingInfo {
                function: resolution.target.clone(),
                positions: binding.positions.clone(),
            }
        } else {
            binding
        };
        // 绑定方法调用：按 RFC-004 进行参数重排
        // obj.method(arg1, arg2) + binding positions [0]
        // → original_function(obj, arg1, arg2)
        //
        // obj.method(arg1) + binding positions [1]
        // → original_function(arg1, obj)

        // 首先生成对象表达式 IR
        let obj_reg = self.next_temp_reg();
        self.generate_expr_ir(expr, obj_reg.clone(), instructions, constants)?;

        // 生成所有方法参数 IR。
        // #452：命名实参先按绑定函数的**值形参名**入槽（位置实参占前导
        // 槽位），再交给绑定位置重排——两步各自有据，不互相顶替。
        // 只用**被绑函数**的形参名：绑定路径把实参交给 binding.positions
        // 重排，槽序以被绑函数的形参表为准；用方法键的声明序形参名会把
        // 接收者位的占位错算成实参（类型体绑定形态实测 E1010）。
        let bound_params = self
            .lookup_param_info(&binding.function, Self::call_name_leaf(&binding.function))
            .map(|info| info.names);
        let mut method_arg_regs = Vec::new();
        if call.has_named() {
            if let Some(names) = bound_params {
                // F3：诊断用真实方法名（不是硬编 <method>，也不是 self）
                let diag_key = if field.is_empty() {
                    Self::call_name_leaf(&binding.function)
                } else {
                    field
                };
                // #456：绑定**判定**走共享判据（形参表[0] 是接收者位，
                // 实参从槽 1 起）；本处只按槽位序搬运实参。
                //
                // `optional` 全置 true = 只重放**槽位绑定**（②未知名 / ③争槽 /
                // ④命名后接位置 / ⑤重复命名），不在 IR 侧重判⑥「缺参」：
                // 元数与缺参归 typecheck。RFC-004 绑定路径的实参是**被绑函数**
                // 调用的一部分，IR 按形参名表重建调用序时区分不了「缺席的接收者」
                // 与「缺席的必需位」；旧实现同样以空槽跳过，注释即「诊断归
                // typecheck」。
                let positions =
                    crate::frontend::core::sema::inference::arg_binding::bind_args_to_params(
                        diag_key,
                        &names,
                        call.args(),
                        call.named(),
                        1, // 方法形态：param_names[0] 是接收者位
                        &vec![true; names.len()],
                        call.span(),
                    )?;
                method_arg_regs =
                    call.collect_regs_in_slot_order(self, &positions, instructions, constants)?;
            }
        }
        if method_arg_regs.is_empty() {
            for arg in call.args() {
                let arg_reg = self.next_temp_reg();
                self.generate_expr_ir(arg, arg_reg.clone(), instructions, constants)?;
                method_arg_regs.push(arg_reg.clone());
            }
        }

        // 按绑定位置重排参数。
        // total_params = 绑定位数 + 方法实参数 = 被绑函数元数（调用方
        // 恰好提供剩余参数），负索引据此归一化（[-1] = 最后一个参数）。
        let total_params = binding.positions.len() + method_arg_regs.len();
        let positions: Vec<i64> = binding
            .positions
            .iter()
            .map(|&p| if p < 0 { p + total_params as i64 } else { p })
            .collect();
        let mut final_args: Vec<Operand> = Vec::with_capacity(total_params);
        let mut method_arg_iter = method_arg_regs.into_iter();

        for pos in 0..total_params {
            if positions.contains(&(pos as i64)) {
                final_args.push(obj_reg.clone());
            } else if let Some(arg_reg) = method_arg_iter.next() {
                final_args.push(arg_reg);
            }
        }

        // 解析函数名
        let func_name = if let Some(qualified) = self
            .registry
            .short_to_qualified_map()
            .get(&binding.function)
        {
            qualified.clone()
        } else {
            binding.function.clone()
        };
        // 调用名就是决议/注册表给出的名字（WBS 4.9.10 D-8）：
        // 定义单元限定已在 typecheck 侧落进方法身份，这里不再改写
        let dispatch_name = func_name;

        instructions.push(Instruction::Call {
            dst: Some(result_reg.clone()),
            func: Operand::Const(ConstValue::String(dispatch_name)),
            args: final_args,
            span: call.span(),
            def: None,
        });
        Ok(())
    }

    /// 形态⑥：普通与重载调用——不带 RFC-004 绑定的方法调用，以及函数形态。
    ///
    /// 方法形态（`obj.method(args)`）：先消费方法级重载决议（RFC-011a §3，键 =
    /// 调用点身份），再按「约束变量具体类型已知 → 直接调用 / 未知 → `CallVirt` /
    /// 其余 → `Type.method`」三路分派。
    ///
    /// 函数形态：结构体构造器（默认值填充、命名实参按字段名重排）→ 命名实参按
    /// 声明序形参名重排 → 静态名与否（`CallDyn` 间接调用）→ print/println 零开销
    /// 分发 → 默认调用。
    fn emit_plain_call(
        &mut self,
        func: &Expr,
        call: &mut CallArgs<'_>,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        if let Expr::FieldAccess { expr, field, .. } = func {
            // 常规方法调用（无绑定）：obj.method(args) → method(obj, args)
            // RFC-011a §3：方法级重载决议命中时直接调用混编名
            //（typecheck 按 实参兼容 + 期望返回 决议，键 = 调用点 PlanId）
            if let Some(resolution) = self.overload_resolutions.get(call.plan_id()).cloned() {
                // #452：命名实参调用同样消费决议（消费侧不得再带
                // named_args.is_empty() 门——那会让决议出的名字被丢弃，
                // 实参也按位置错配）。实参按**形参声明序**重建：位置实参占
                // 前导槽位、命名实参按名入槽。
                //
                // WBS 4.9.10 D-3：**两件都在载荷里**——被调名（D-8 的 IR 最终
                // 函数名）与胜出候选的声明期形参名。名字由 typecheck 按方法身份
                // 直查候选组取得，本生成器因此既不改写被调名、也不查任何名字表
                //（旧的 `method_param_info` / `lookup_param_info` 方法面分支已删）。
                let dispatch_name = resolution.target.clone();
                let leaf = Self::call_name_leaf(&dispatch_name).to_string();
                let names = resolution.param_names.clone();
                // F3：诊断用决议出的真实方法名（叶子段），不再硬编 <method>
                // #456：绑定**判定**走共享判据（接收者占槽 0，实参从 1 起）；
                // 本处只按槽位序搬运实参。
                //
                // `optional` 全置 true：同上一处——IR 只重放槽位绑定，不重判
                // ⑥「缺参」。这里形参名表来自**声明**（同名重载取最后一条），
                // 而调用按 typecheck 决议出的候选（元数可能更小）发实参；
                // 按声明表重判缺参会把合法决议误报 E1010（`t.pick2(a = 2)`
                // 命中一元候选、声明表是二元那条的实测）。
                let positions =
                    crate::frontend::core::sema::inference::arg_binding::bind_args_to_params(
                        &leaf,
                        &names,
                        call.args(),
                        call.named(),
                        1, // 方法形态：param_names[0] 是接收者位
                        &vec![true; names.len()],
                        call.span(),
                    )?;
                let obj_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, obj_reg.clone(), instructions, constants)?;
                let mut call_args = vec![obj_reg.clone()];
                call_args.extend(call.collect_regs_in_slot_order(
                    self,
                    &positions,
                    instructions,
                    constants,
                )?);
                instructions.push(Instruction::Call {
                    dst: Some(result_reg.clone()),
                    func: Operand::Const(ConstValue::String(dispatch_name)),
                    args: call_args,
                    span: call.span(),
                    def: None,
                });
                return Ok(());
            }
            // 接口直接赋值优化：检查对象是否是约束变量
            // 生成对象表达式 IR（作为第一个参数）
            let obj_reg = self.next_temp_reg();
            self.generate_expr_ir(expr, obj_reg.clone(), instructions, constants)?;
            // 生成方法参数 IR（接收者在前，与方法形态的实参序一致）
            call.collect_regs(self, instructions, constants)?;
            let mut arg_regs: Vec<Operand> = Vec::with_capacity(1 + call.regs().len());
            arg_regs.push(obj_reg.clone());
            arg_regs.extend(call.regs().iter().cloned());

            // RFC-011a §6.4: 存在类型接收者 → 编译期变体分发。
            // 门控：接口须有本单元实现证明（隔离遗留 trait 约束路径）
            if let Some(iface) = self.existential_receiver_interface(expr) {
                if self.interface_variants.contains_key(&iface) {
                    let method_args: Vec<Operand> = arg_regs[1..].to_vec();
                    self.emit_variant_dispatch(
                        &iface,
                        field,
                        obj_reg,
                        &method_args,
                        result_reg,
                        call.span(),
                        instructions,
                        constants,
                    )?;
                    return Ok(());
                }
            }

            // 检查对象是否是约束变量（接口直接赋值优化）
            let var_name = if let Expr::Var(name, _) = expr.as_ref() {
                Some(name.clone())
            } else {
                None
            };

            let concrete_type = var_name
                .as_ref()
                .and_then(|name| self.get_constraint_var_concrete_type(name).cloned());

            if let Some(concrete_type_name) = concrete_type {
                // 编译期可确定具体类型 → 直接调用（零开销）
                // d.draw(screen) → ConcreteType.draw(d, screen)
                let qualified_name = format!("{}.{}", concrete_type_name, field);

                let final_args: Vec<Operand> = arg_regs.clone();

                instructions.push(Instruction::Call {
                    dst: Some(result_reg.clone()),
                    func: Operand::Const(ConstValue::String(qualified_name)),
                    args: final_args,
                    span: call.span(),
                    def: None,
                });
            } else if var_name.as_ref().is_some_and(|name| {
                // 检查变量的类型标注是否是约束类型（但具体类型未知）
                self.local_var_types
                    .get(name)
                    .and_then(|type_name| {
                        // 如果变量类型是约束类型且不在 constraint_var_concrete_types 中
                        // 说明具体类型无法在编译期确定，需要 vtable 调用
                        if !self.struct_definitions.contains_key(type_name)
                            && !self.constraint_var_concrete_types.contains_key(name)
                        {
                            // 简单启发式：如果变量类型不是已知结构体，可能是约束类型
                            Some(true)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false)
            }) {
                // 编译期无法确定具体类型 → CallVirt（vtable 调用）
                instructions.push(Instruction::CallVirt {
                    dst: Some(result_reg.clone()),
                    obj: obj_reg.clone(),
                    method_name: field.to_string(),
                    args: arg_regs,
                    span: call.span(),
                });
            } else {
                // 普通方法调用
                // 优先使用类型名（而非变量名）构建函数名
                // 例如：a.is_greater(b) 中 a 的类型是 Node
                // → 函数名应为 "Node.is_greater" 而非 "a.is_greater"
                let func_name = if let Expr::Var(name, _) = expr.as_ref() {
                    if let Some(type_name) = self.local_var_types.get(name) {
                        // #266: &mut 令牌穿透——变量类型可能是
                        // "&mut Point"，方法名应基于底层结构体 "Point"
                        let base = Self::strip_ref_prefix(type_name);
                        format!("{}.{}", base, field)
                    } else if let Some(ty) = self.get_expr_mono_type(expr) {
                        // local_var_types 未收录时从类型检查结果取接收者
                        // 类型名（`r.is_failure()` 的 r: Result(..)）；
                        // std yx 方法经注册表限定名映射改写
                        // （"Result.is_failure" → "std.result.is_failure"）。
                        let mut ty = ty;
                        while let crate::frontend::core::types::MonoType::Ref { inner, .. } = ty {
                            ty = *inner;
                        }
                        let base = match &ty {
                            crate::frontend::core::types::MonoType::Generic { name, .. } => {
                                Some(name.clone())
                            }
                            crate::frontend::core::types::MonoType::Struct(s) => {
                                Some(s.name.clone())
                            }
                            crate::frontend::core::types::MonoType::TypeRef(n) => Some(n.clone()),
                            _ => None,
                        };
                        match base {
                            Some(base) => {
                                // std 面方法身份查询（D-1）：旧的字符串限定名
                                // 映射已由身份表回答同一问题——模块键取自
                                // DefPath，不再给字符串加前缀。
                                self.registry
                                    .std_method_path(base.bare(), field)
                                    .map(|path| path.project())
                                    .unwrap_or_else(|| format!("{}.{}", base, field))
                            }
                            None => self.resolve_field_path(expr, field),
                        }
                    } else {
                        self.resolve_field_path(expr, field)
                    }
                } else {
                    self.resolve_field_path(expr, field)
                };

                let final_args: Vec<Operand> = arg_regs.clone();

                // 调用名即上游给出的名字（D-8）：不再按接收者二次限定
                let dispatch_name = func_name;
                instructions.push(Instruction::Call {
                    dst: Some(result_reg.clone()),
                    func: Operand::Const(ConstValue::String(dispatch_name)),
                    args: final_args,
                    span: call.span(),
                    def: None,
                });
            }
        } else {
            // 普通函数调用
            call.collect_regs(self, instructions, constants)?;

            // RFC-010: 处理命名参数构造 `Point(x=1, y=2)`
            if call.has_named() {
                if let Expr::Var(name, _) = func {
                    // 构造实参按**该调用点的结果类型身份**取字段表：同名类型
                    // （a.T / b.T）各用自己定义的字段序与默认值。只在写名确实是
                    // 结构体构造器（裸名表有它）且身份定义名与写名一致时用身份
                    // 视图——否则会把「返回结构体的普通函数」误当构造器补默认值。
                    let by_identity = self
                        .call_result_identity(call_plan_id)
                        .filter(|identity| identity.bare() == name.as_str())
                        .and_then(|identity| self.struct_fields_by_identity(&identity).cloned());
                    if let Some(fields) =
                        by_identity.or_else(|| self.struct_definitions.get(name).cloned())
                    {
                        // 生成命名参数的 IR
                        let mut named_regs: Vec<(String, Operand)> = Vec::new();
                        for (arg_name, arg_expr) in call.named() {
                            let arg_reg = self.next_temp_reg();
                            self.generate_expr_ir(
                                arg_expr,
                                arg_reg.clone(),
                                instructions,
                                constants,
                            )?;
                            named_regs.push((arg_name.clone(), arg_reg.clone()));
                        }

                        // 按**数据字段**顺序重排参数（witness 字段不占构造槽位）
                        let data_fields = Self::data_ctor_fields(&fields);
                        let mut final_args: Vec<Option<Operand>> = vec![None; data_fields.len()];

                        // 先放置位置参数
                        for (i, reg) in call.regs().iter().enumerate() {
                            if i < data_fields.len() {
                                final_args[i] = Some(reg.clone());
                            }
                        }

                        // 再放置命名参数（按字段名匹配）
                        for (name, reg) in &named_regs {
                            if let Some(idx) = data_fields
                                .iter()
                                .position(|(_, field)| &field.name == name)
                            {
                                final_args[idx] = Some(reg.clone());
                            }
                        }

                        // 填充默认值
                        for (i, slot) in final_args.iter_mut().enumerate() {
                            if slot.is_none() {
                                let default_reg = self.next_temp_reg();
                                if let Some(default_expr) = &data_fields[i].1.default {
                                    self.generate_expr_ir(
                                        default_expr,
                                        default_reg.clone(),
                                        instructions,
                                        constants,
                                    )?;
                                } else {
                                    // 命名式构造没有「部分构造」形态：缺位由类型检查按
                                    // E1010 拒绝。走到这里 = 两侧判定不一致，响亮失败
                                    // （旧行为：静默补 Int(0)，把未指定的字段当零值传下去）。
                                    return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                                        "结构体 {} 的字段 {} 既无实参也无默认值——类型检查与 lowering 判定不一致",
                                        name, data_fields[i].1.name
                                    ))
                                    .at(call.span())
                                    .build());
                                }
                                *slot = Some(default_reg.clone());
                            }
                        }

                        call.set_regs(final_args.into_iter().map(|s| s.unwrap()).collect());
                    }
                }
            }

            // 检查是否是结构体构造器调用，需要填充默认值
            // 两层调用 X(类型参数)(构造参数)：func 是 Call{func: Var(name)}，
            // 内层类型实参运行期擦除，外层实参按字段位置填充。
            let struct_ctor_name: Option<String> = match func {
                Expr::Var(name, _) => Some(name.clone()),
                Expr::Call { func: inner, .. } => match inner.as_ref() {
                    Expr::Var(name, _) => Some(name.clone()),
                    _ => None,
                },
                _ => None,
            };
            if let Some(name) = struct_ctor_name {
                // 同命名实参分支：只有裸名表认得这个写名（真是构造器）时才用
                // 身份视图；「返回结构体的普通函数」不得被补默认值。
                let by_identity = self
                    .call_result_identity(call_plan_id)
                    .filter(|identity| identity.bare() == name.as_str())
                    .and_then(|identity| self.struct_fields_by_identity(&identity).cloned());
                if let Some(fields) =
                    by_identity.or_else(|| self.struct_definitions.get(&name).cloned())
                {
                    // 这是一个结构体构造器调用
                    // 数据字段数才是构造参数上限；witness 字段不占槽位
                    let data_fields = Self::data_ctor_fields(&fields);
                    let present = call.regs().len();
                    // 空构造（`Pair(Int)()`）的字段取**零值**，数据事后赋值——SPEC
                    // §4.3 与语料 `generic_constructor.yx` 的「空构造」用例，零值可被
                    // 用户程序观察到（那边断言 `z.first == 0`），属**合法零初始化**，
                    // 不是兜底。
                    // 部分构造（`Pair(Int)(1)`）没有对应语义：缺位由类型检查按 E1010
                    // 拒绝，走到这里是内部不一致，响亮失败（旧行为静默补 Int(0)）。
                    let is_empty_construction = present == 0;
                    if present < data_fields.len() {
                        for (data_index, (_, field)) in data_fields.iter().enumerate().skip(present)
                        {
                            let default_reg = self.next_temp_reg();
                            match &field.default {
                                // 有默认值：生成默认值表达式 IR
                                Some(default_expr) => self.generate_expr_ir(
                                    default_expr,
                                    default_reg.clone(),
                                    instructions,
                                    constants,
                                )?,
                                None if is_empty_construction => {
                                    instructions.push(Instruction::Load {
                                        dst: default_reg.clone(),
                                        src: Operand::Const(ConstValue::Int(0)),
                                        span: self.cur_span,
                                    });
                                }
                                None => {
                                    return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                                        "结构体 {} 的第 {} 个数据字段 {} 既无实参也无默认值——类型检查与 lowering 判定不一致",
                                        name, data_index, field.name
                                    ))
                                    .at(call.span())
                                    .build())
                                }
                            }
                            call.push_reg(default_reg.clone());
                        }
                    }
                }
            }

            // 命名参数调用普通**函数**：`add(b = 5, a = 3)`。
            //
            // 上面的 struct 分支按字段名重排；普通函数原先无人处理 `named_args`，
            // 于是它们被静默丢弃——实参一个不进调用，运行期报 e6007「类型不符」，
            // 与真正的病因（参数没传进去）完全搭不上。这里按**声明序形参名**重排：
            // 位置实参填前导槽位，命名实参按名入槽，重名/未知名报错。
            if call.has_named() {
                let callee = match func {
                    Expr::Var(n, _) => Some(n.clone()),
                    // 限定名调用 `list.push(...)`：取叶子名（表中键就是声明处的短名）
                    Expr::FieldAccess { field, .. } => Some(field.clone()),
                    // 两层调用 `f(T)(...)`：外层才是传值那层
                    Expr::Call { func: inner, .. } => match inner.as_ref() {
                        Expr::Var(n, _) => Some(n.clone()),
                        Expr::FieldAccess { field, .. } => Some(field.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                let is_ctor = callee
                    .as_ref()
                    .is_some_and(|n| self.struct_definitions.contains_key(n));
                // 本模块表 → 跨模块导出携带的 `param_names`。
                // 限定名（`list.push`）保留完整路径去查导出。
                let qualified = match func {
                    Expr::FieldAccess { expr, field, .. } => {
                        Some(self.resolve_field_path(expr, field))
                    }
                    _ => None,
                };
                if let Some(info) = callee
                    .as_ref()
                    .filter(|_| !is_ctor)
                    .and_then(|n| self.lookup_param_info(qualified.as_deref().unwrap_or(n), n))
                {
                    let names = &info.names;
                    // t12-F1 / t14：槽位判定收敛到共享判据（与方法与 typecheck 原生臂
                    // 同一实现）——此前这里是第三份内联副本，槽冲突还报旧码
                    // E1015、且缺规则④（命名后接位置）；可选位与形参名同出一源。
                    let diag_key = callee.as_deref().unwrap_or("<fn>");
                    let optional: Vec<bool> =
                        (0..names.len()).map(|i| i >= info.required_min).collect();
                    let positions =
                        crate::frontend::core::sema::inference::arg_binding::bind_args_to_params(
                            diag_key,
                            names,
                            call.args(),
                            call.named(),
                            0, // 命名空间调用无接收者位
                            &optional,
                            call.span(),
                        )?;
                    let slots = call.bind_named_slots(
                        self,
                        names.len(),
                        &positions,
                        instructions,
                        constants,
                    )?;
                    // 未覆盖的槽位 = 类型检查与 lowering 判定不一致：响亮失败，
                    // 不补 0（旧行为把未指定的形参当零值传下去，错误无声无息）。
                    call.require_complete_slots(diag_key, names, slots)?;
                }
            }

            // 检查是否是动态函数调用（非静态函数名的 Var）
            if !self.is_static_fn_name(func) {
                // 闭包调用：先加载函数值，然后使用 CallDyn
                let func_reg = self.next_temp_reg();
                self.generate_expr_ir(func, func_reg.clone(), instructions, constants)?;

                instructions.push(Instruction::CallDyn {
                    dst: Some(result_reg.clone()),
                    func: func_reg.clone(),
                    args: call.take_regs(),
                    span: call.span(),
                });
            } else {
                // ========== print/println 零开销分发处理 ==========
                // 检查是否是 print 或 println 调用。入口闸一处覆盖本特例内
                // print 名的两个计算点（Stringable 路与兜底路），判据单源 =
                // `declared_fn_names`（与 `resolve_function_name` 的乙案臂同一
                // 集合，query.rs）：**本单元已声明的函数名不走特例**——特例的
                // print 名计算直接查 std 短名映射（short_to_qualified_map /
                // format!("std.io.{}", name)），绕过 `resolve_function_name`
                // 的用户定义优先臂；用户自定义 `print`/`println` 的 Var 实参
                // 裸名调用会 check 绿（sema 按用户签名检查）而 run 派发
                // std.io.print——exit 0 静默错配（issue #477 / WBS 3.24.4，
                // 按实测锚定：字面量实参的类型在特例入口不可取
                // （`get_expr_mono_type` 落 `_ => None`），本就落「无法获取
                // 类型」默认路径、乙案已覆盖；真正红态是 Var 实参形态）。
                // 命中闸时落下方 `else` 的默认处理路径，由
                // `resolve_function_name` / `qualify_names` 按乙案解析到用户
                // 函数；要访问被遮蔽的 std 名，写限定名 `std.io.print`。
                // std 嵌入单元各自做 IR 生成（`declared_fn_names` 按单元填充），
                // std 源内部无裸名 print 调用点，std 面语义零变化。
                let is_print_call = if let Expr::Var(name, _) = func {
                    matches!(
                        name.as_str(),
                        "print" | "println" | "std.io.print" | "std.io.println"
                    ) && !self.declared_fn_names.contains(name)
                } else {
                    false
                };

                // 如果是 print/println 且有参数，尝试零开销分发
                if is_print_call && !call.args().is_empty() {
                    let arg_expr = &call.args()[0];
                    // 获取参数的类型信息
                    let arg_type = self.get_expr_mono_type(arg_expr);

                    if let Some(mono_type) = arg_type {
                        // 检查类型是否实现了 Stringable（to_string 方法）
                        if type_implements_stringable(&mono_type) {
                            // 零开销路径：直接调用 to_string 方法
                            // 生成: arg.to_string()
                            // 调用名 = 短类型名 + 方法名（D-8）：std 面由身份表给限定名，
                            // 基础类型 / 本单元类型保持裸名。
                            let base = get_type_fallback_string(&mono_type);
                            let short_name = format!("{}.to_string", base);
                            let func_name = self
                                .registry
                                .std_method_path(&base, "to_string")
                                .map(|path| path.project())
                                .unwrap_or(short_name);
                            let mut arg_regs_for_method = Vec::new();

                            // 先计算参数值
                            let arg_reg = self.next_temp_reg();
                            self.generate_expr_ir(
                                arg_expr,
                                arg_reg.clone(),
                                instructions,
                                constants,
                            )?;
                            arg_regs_for_method.push(arg_reg.clone());

                            // 调用 to_string 方法（合成调用：Stringable 的
                            // `to_string` 声明返回位恒为 String）
                            let to_string_reg = self.next_temp_reg();
                            instructions.push(Instruction::Call {
                                dst: Some(to_string_reg.clone()),
                                func: Operand::Const(ConstValue::String(func_name)),
                                args: arg_regs_for_method,
                                span: call.span(),
                                def: None,
                            });
                            self.note_value_type(
                                &to_string_reg,
                                MonoType::Generic {
                                    name: Nominal::flat("String").expect("String 是裸类型名"),
                                    args: Vec::new(),
                                },
                            );

                            // 然后调用 std.io.print 输出字符串
                            // 使用 resolved name
                            let print_func_name = if let Expr::Var(name, _) = func {
                                if name == "print" || name == "println" {
                                    if let Some(qualified) =
                                        self.registry.short_to_qualified_map().get(name)
                                    {
                                        qualified.clone()
                                    } else {
                                        DefPath::item("std.io", name).project()
                                    }
                                } else {
                                    name.clone()
                                }
                            } else {
                                "std.io.print".to_string()
                            };

                            instructions.push(Instruction::Call {
                                dst: Some(result_reg.clone()),
                                func: Operand::Const(ConstValue::String(print_func_name)),
                                args: vec![to_string_reg.clone()],
                                span: call.span(),
                                def: None,
                            });
                        } else {
                            // 兜底路径：类型未实现 Stringable，调用 std.io.print 输出类型信息
                            // 生成: std.io.format_fallback(arg, type_name)
                            let type_name = get_type_fallback_string(&mono_type);

                            // 先计算参数值
                            let arg_reg = self.next_temp_reg();
                            self.generate_expr_ir(
                                arg_expr,
                                arg_reg.clone(),
                                instructions,
                                constants,
                            )?;

                            // 调用 format_fallback 获取类型信息字符串
                            // 类型名常量先 Load 进寄存器——Call 参数必须寄存器态
                            // （Const 直传会被 codegen to_reg 拒收，老 bug，tuple 同款）
                            let type_name_reg = self.next_temp_reg();
                            instructions.push(Instruction::Load {
                                dst: type_name_reg.clone(),
                                src: Operand::Const(ConstValue::String(type_name.clone())),
                                span: self.cur_span,
                            });
                            let fallback_reg = self.next_temp_reg();
                            instructions.push(Instruction::Call {
                                dst: Some(fallback_reg.clone()),
                                func: Operand::Const(ConstValue::String(
                                    "std.io.format_fallback".to_string(),
                                )),
                                args: vec![arg_reg.clone(), type_name_reg.clone()],
                                span: call.span(),
                                def: None,
                            });
                            // 合成调用：声明签名 `format_fallback -> String`
                            self.note_value_type(
                                &fallback_reg,
                                MonoType::Generic {
                                    name: Nominal::flat("String").expect("String 是裸类型名"),
                                    args: Vec::new(),
                                },
                            );

                            // 然后调用 std.io.print 输出
                            let print_func_name = if let Expr::Var(name, _) = func {
                                if name == "print" || name == "println" {
                                    if let Some(qualified) =
                                        self.registry.short_to_qualified_map().get(name)
                                    {
                                        qualified.clone()
                                    } else {
                                        DefPath::item("std.io", name).project()
                                    }
                                } else {
                                    name.clone()
                                }
                            } else {
                                "std.io.print".to_string()
                            };

                            instructions.push(Instruction::Call {
                                dst: Some(result_reg.clone()),
                                func: Operand::Const(ConstValue::String(print_func_name)),
                                args: vec![fallback_reg.clone()],
                                span: call.span(),
                                def: None,
                            });
                        }
                    } else {
                        // 无法获取类型，使用默认处理
                        let func_operand = self.resolve_function_name(func)?;
                        instructions.push(Instruction::Call {
                            dst: Some(result_reg.clone()),
                            func: func_operand,
                            args: call.take_regs(),
                            span: call.span(),
                            def: None,
                        });
                    }
                } else {
                    // 非 print 调用或无参数，使用默认处理
                    // ========== 默认函数调用处理 ==========
                    let final_args = call.take_regs();

                    let func_operand = self.resolve_function_name(func)?;
                    instructions.push(Instruction::Call {
                        dst: Some(result_reg.clone()),
                        func: func_operand,
                        args: final_args,
                        span: call.span(),
                        def: None,
                    });
                }
            }
        }
        Ok(())
    }
}
