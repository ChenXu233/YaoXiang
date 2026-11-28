//! 调用与存在类型变体分发
//!
//! `generate_call_expr_ir`（866 行）原样搬入，方法级拆分归 7d.1；
//! `emit_variant_dispatch` 是存在类型方法调用的变体分发生成器。
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast::Expr;
use crate::frontend::core::sema::MonoType;
use crate::frontend::core::types::identity::DefPath;
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

    /// 生成 Call 表达式 IR
    ///
    /// `call_plan_id` = **本调用节点的身份**（`Expr::Call::plan_id`，RFC-039 D20）：
    /// 重载决议表的消费键就是它——与 typecheck 产出侧读同一个节点上的同一个值。
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
        // RFC-010: 变体构造——`Result(Int, String).ok(5)` / `Color.red()`。
        // 整个调用替换为 CreateVariant：载荷求值进寄存器（多载荷先打包
        // Tuple、零载荷用 Void 哨兵），group 携带和类型名（解释器 intern
        // 为类型身份），variant 为声明序号。
        if let Some((type_name, variant_index, _)) = self.detect_variant_ctor_call(func) {
            let payload_count = args.len();
            let payload_reg = self.next_temp_reg();
            if payload_count == 0 {
                instructions.push(Instruction::Load {
                    dst: payload_reg.clone(),
                    src: Operand::Const(ConstValue::Void),
                    span: *span,
                });
            } else if payload_count == 1 {
                self.generate_expr_ir(&args[0], payload_reg.clone(), instructions, constants)?;
            } else {
                // 多载荷打包 Tuple（与 match 变体模式的多参解包对称）
                let mut elem_regs = Vec::with_capacity(payload_count);
                for a in args {
                    let r = self.next_temp_reg();
                    self.generate_expr_ir(a, r.clone(), instructions, constants)?;
                    elem_regs.push(r.clone());
                }
                instructions.push(Instruction::NewTuple {
                    dst: payload_reg.clone(),
                    items: elem_regs,
                    span: *span,
                });
            }
            instructions.push(Instruction::CreateVariant {
                dst: result_reg.clone(),
                group: type_name,
                variant: variant_index as u32,
                payload: payload_reg.clone(),
                span: *span,
            });
            return Ok(());
        }

        // RFC-011 §「Vec(T) 的构造形式」：内置容器的两层构造
        // `Vec(Int)()` / `Vec(Int)(1, 2, 3)` / `Vec(Int)(len = 64)`。
        //
        // 内层 `Vec(Int)` 是**类型实参**，运行期擦除（与 struct 的
        // `Container(Int)(42, 43)` 同一规则）；真正要生成的是外层构造：
        //   - 无构造实参 → 空容器
        //   - 位置实参   → 元素构造（长度 = 元素个数）
        //   - `len = n`  → 槽位分配（n 个零值）
        // 此前这条路径根本不存在：`Vec(Int)()` 在类型检查阶段就报
        // E1001 unknown variable 'Vec'（类型名不在值位置的白名单里）。
        if let Expr::Call { func: inner, .. } = func {
            if let Some(_ctor) = self.builtin_container_ctor_name(inner) {
                return self.generate_builtin_container_ir(
                    args,
                    named_args,
                    span,
                    result_reg,
                    instructions,
                    constants,
                );
            }
        }
        // 检查是否是方法调用：func 是 FieldAccess
        if let Expr::FieldAccess { expr, field, .. } = func {
            // 方法调用 - 转换为普通函数调用
            // 命名空间机制：p.method() -> method(p)

            // 只有非命名空间调用才需要添加 self 参数
            // 命名空间调用（如 std.io.println）不需要隐式参数
            if self.is_namespace_receiver(expr) {
                // 命名空间调用：不需要隐式参数
                //
                // 命名参数（`list.push(list = v, item = 9)`）在命名空间路径上
                // 同样要按形参名重排，否则实参一个不进——与普通函数调用同类
                // 缺口（那边在 6.5k 行修，这里是它的镜像）。
                let mut arg_regs = Vec::new();
                for arg in args.iter() {
                    let arg_reg = self.next_temp_reg();
                    self.generate_expr_ir(arg, arg_reg.clone(), instructions, constants)?;
                    arg_regs.push(arg_reg.clone());
                }
                let method_function_name = self.resolve_field_path(expr, field);

                // WBS 3.6.4：命名空间限定名若落在**跨文件全局槽位**，它是顶层值绑定
                // （函数类型值），不是函数表条目——按名 `Call` 在运行期查扁平函数表
                // 落空（`E6006 Native function not found: lib.f`）。改走间接调用：
                // 读槽位 → `CallDyn`。限定名解析与槽位查询与
                // `generate_field_access_expr_ir` 的命名空间数据访问**同一分支**
                // （`resolve_namespace_data_access` + `global_slot_layout` 两表联查），
                // 调用机制与闭包/函数值调用同一实现（`CallDyn`），不新增解析表。
                if named_args.is_empty() {
                    if let Some(slot) = self
                        .resolve_namespace_data_access(expr, field)
                        .and_then(|qualified| self.global_slot_layout.get(&qualified))
                        .copied()
                    {
                        let func_reg = self.next_temp_reg();
                        instructions.push(Instruction::Load {
                            dst: func_reg.clone(),
                            src: Operand::Global(slot),
                            span: *span,
                        });
                        instructions.push(Instruction::CallDyn {
                            dst: Some(result_reg.clone()),
                            func: func_reg.clone(),
                            args: arg_regs,
                            span: *span,
                        });
                        return Ok(());
                    }
                }
                if !named_args.is_empty() {
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
                        let positions = crate::frontend::core::sema::inference::arg_binding::bind_args_to_params(
                            &method_function_name,
                            names,
                            args,
                            named_args,
                            0, // 命名空间调用无接收者位
                            &optional,
                            *span,
                        )?;
                        let mut slots: Vec<Option<Operand>> = vec![None; names.len()];
                        for (arg_pos, pos) in positions.iter().enumerate() {
                            if *pos >= slots.len() {
                                continue; // 超签实参：由类型检查阶段报 E1010
                            }
                            let reg = if arg_pos < arg_regs.len() {
                                arg_regs[arg_pos].clone()
                            } else {
                                let arg_expr = &named_args[arg_pos - arg_regs.len()].1;
                                let arg_reg = self.next_temp_reg();
                                self.generate_expr_ir(
                                    arg_expr,
                                    arg_reg.clone(),
                                    instructions,
                                    constants,
                                )?;
                                arg_reg.clone()
                            };
                            slots[*pos] = Some(reg);
                        }
                        arg_regs = slots.into_iter().flatten().collect();
                    }
                }
                instructions.push(Instruction::Call {
                    dst: Some(result_reg.clone()),
                    func: Operand::Const(ConstValue::String(method_function_name.to_string())),
                    args: arg_regs,
                    span: *span,
                    def: None,
                });
            } else {
                // 非命名空间调用：检查是否有绑定信息（RFC-004）
                let binding_info = self.get_expr_struct_type_name(expr).and_then(|type_name| {
                    self.type_bindings
                        .get(&type_name)
                        .and_then(|bindings| bindings.get(field).cloned())
                });

                if let Some(binding) = binding_info {
                    // RFC-011a §3：方法级重载决议命中时，binding.function
                    //（后定义覆盖的单值）替换为决议出的混编名
                    let binding =
                        if let Some(resolution) = self.overload_resolutions.get(call_plan_id) {
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
                        .lookup_param_info(
                            &binding.function,
                            Self::call_name_leaf(&binding.function),
                        )
                        .map(|info| info.names);
                    let mut method_arg_regs = Vec::new();
                    if !named_args.is_empty() {
                        if let Some(names) = bound_params {
                            // F3：诊断用真实方法名（不是硬编 <method>，也不是 self）
                            let diag_key = if field.is_empty() {
                                Self::call_name_leaf(&binding.function)
                            } else {
                                field.as_str()
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
                            let positions = crate::frontend::core::sema::inference::arg_binding::bind_args_to_params(
                                diag_key,
                                &names,
                                args,
                                named_args,
                                1, // 方法形态：param_names[0] 是接收者位
                                &vec![true; names.len()],
                                *span,
                            )?;
                            for arg_pos in Self::slot_order_arg_indices(&positions) {
                                let arg_expr = if arg_pos < args.len() {
                                    &args[arg_pos]
                                } else {
                                    &named_args[arg_pos - args.len()].1
                                };
                                let arg_reg = self.next_temp_reg();
                                self.generate_expr_ir(
                                    arg_expr,
                                    arg_reg.clone(),
                                    instructions,
                                    constants,
                                )?;
                                method_arg_regs.push(arg_reg.clone());
                            }
                        }
                    }
                    if method_arg_regs.is_empty() {
                        for arg in args.iter() {
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
                        span: *span,
                        def: None,
                    });
                } else {
                    // 常规方法调用（无绑定）：obj.method(args) → method(obj, args)
                    // RFC-011a §3：方法级重载决议命中时直接调用混编名
                    //（typecheck 按 实参兼容 + 期望返回 决议，键 = 调用点 PlanId）
                    if let Some(resolution) = self.overload_resolutions.get(call_plan_id).cloned() {
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
                        let positions = crate::frontend::core::sema::inference::arg_binding::bind_args_to_params(
                            &leaf,
                            &names,
                            args,
                            named_args,
                            1, // 方法形态：param_names[0] 是接收者位
                            &vec![true; names.len()],
                            *span,
                        )?;
                        let obj_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, obj_reg.clone(), instructions, constants)?;
                        let mut call_args = vec![obj_reg.clone()];
                        for arg_pos in Self::slot_order_arg_indices(&positions) {
                            let arg_expr = if arg_pos < args.len() {
                                &args[arg_pos]
                            } else {
                                &named_args[arg_pos - args.len()].1
                            };
                            let r = self.next_temp_reg();
                            self.generate_expr_ir(arg_expr, r.clone(), instructions, constants)?;
                            call_args.push(r.clone());
                        }
                        instructions.push(Instruction::Call {
                            dst: Some(result_reg.clone()),
                            func: Operand::Const(ConstValue::String(dispatch_name)),
                            args: call_args,
                            span: *span,
                            def: None,
                        });
                        return Ok(());
                    }
                    // 接口直接赋值优化：检查对象是否是约束变量
                    let mut arg_regs = Vec::new();

                    // 生成对象表达式 IR（作为第一个参数）
                    let obj_reg = self.next_temp_reg();
                    self.generate_expr_ir(expr, obj_reg.clone(), instructions, constants)?;
                    arg_regs.push(obj_reg.clone());

                    // 生成方法参数 IR
                    for arg in args.iter() {
                        let arg_reg = self.next_temp_reg();
                        self.generate_expr_ir(arg, arg_reg.clone(), instructions, constants)?;
                        arg_regs.push(arg_reg.clone());
                    }

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
                                *span,
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
                            span: *span,
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
                            span: *span,
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
                                while let crate::frontend::core::types::MonoType::Ref {
                                    inner,
                                    ..
                                } = ty
                                {
                                    ty = *inner;
                                }
                                let base = match &ty {
                                    crate::frontend::core::types::MonoType::Generic {
                                        name,
                                        ..
                                    } => Some(name.clone()),
                                    crate::frontend::core::types::MonoType::Struct(s) => {
                                        Some(s.name.clone())
                                    }
                                    crate::frontend::core::types::MonoType::TypeRef(n) => {
                                        Some(n.clone())
                                    }
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
                            span: *span,
                            def: None,
                        });
                    }
                }
            }
        } else {
            // 普通函数调用
            let mut arg_regs = Vec::new();
            for arg in args.iter() {
                let arg_reg = self.next_temp_reg();
                self.generate_expr_ir(arg, arg_reg.clone(), instructions, constants)?;
                arg_regs.push(arg_reg.clone());
            }

            // RFC-010: 处理命名参数构造 `Point(x=1, y=2)`
            if !named_args.is_empty() {
                if let Expr::Var(name, _) = func {
                    if let Some(fields) = self.struct_definitions.get(name).cloned() {
                        // 生成命名参数的 IR
                        let mut named_regs: Vec<(String, Operand)> = Vec::new();
                        for (arg_name, arg_expr) in named_args.iter() {
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
                        for (i, reg) in arg_regs.iter().enumerate() {
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
                                    instructions.push(Instruction::Load {
                                        dst: default_reg.clone(),
                                        src: Operand::Const(ConstValue::Int(0)),
                                        span: self.cur_span,
                                    });
                                }
                                *slot = Some(default_reg.clone());
                            }
                        }

                        arg_regs = final_args.into_iter().map(|s| s.unwrap()).collect();
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
                if let Some(fields) = self.struct_definitions.get(&name).cloned() {
                    // 这是一个结构体构造器调用
                    // 数据字段数才是构造参数上限；witness 字段不占槽位
                    let data_fields = Self::data_ctor_fields(&fields);
                    if arg_regs.len() < data_fields.len() {
                        for (_, field) in data_fields.iter().skip(arg_regs.len()) {
                            let default_reg = self.next_temp_reg();
                            if let Some(default_expr) = &field.default {
                                // 有默认值：生成默认值表达式 IR
                                self.generate_expr_ir(
                                    default_expr,
                                    default_reg.clone(),
                                    instructions,
                                    constants,
                                )?;
                            } else {
                                // 无默认值：用零值填充（语义检查阶段应已报错）
                                instructions.push(Instruction::Load {
                                    dst: default_reg.clone(),
                                    src: Operand::Const(ConstValue::Int(0)),
                                    span: self.cur_span,
                                });
                            }
                            arg_regs.push(default_reg.clone());
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
            if !named_args.is_empty() {
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
                            diag_key, names, args, named_args,
                            0, // 命名空间调用无接收者位
                            &optional, *span,
                        )?;
                    let mut slots: Vec<Option<Operand>> = vec![None; names.len()];
                    for (arg_pos, pos) in positions.iter().enumerate() {
                        if *pos >= slots.len() {
                            continue; // 超签实参：由类型检查阶段报 E1010
                        }
                        let reg = if arg_pos < arg_regs.len() {
                            arg_regs[arg_pos].clone()
                        } else {
                            let arg_expr = &named_args[arg_pos - arg_regs.len()].1;
                            let arg_reg = self.next_temp_reg();
                            self.generate_expr_ir(
                                arg_expr,
                                arg_reg.clone(),
                                instructions,
                                constants,
                            )?;
                            arg_reg.clone()
                        };
                        slots[*pos] = Some(reg);
                    }
                    // 未覆盖的槽位：保持缺席，由类型检查阶段报 E1010。
                    // 运行期为安全起见补 0（正常路径不会到这里）。
                    arg_regs = slots
                        .into_iter()
                        .map(|s| {
                            s.unwrap_or_else(|| {
                                let r = self.next_temp_reg();
                                instructions.push(Instruction::Load {
                                    dst: r.clone(),
                                    src: Operand::Const(ConstValue::Int(0)),
                                    span: self.cur_span,
                                });
                                r.clone()
                            })
                        })
                        .collect();
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
                    args: arg_regs,
                    span: *span,
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
                if is_print_call && !args.is_empty() {
                    let arg_expr = &args[0];
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

                            // 调用 to_string 方法
                            let to_string_reg = self.next_temp_reg();
                            instructions.push(Instruction::Call {
                                dst: Some(to_string_reg.clone()),
                                func: Operand::Const(ConstValue::String(func_name)),
                                args: arg_regs_for_method,
                                span: *span,
                                def: None,
                            });

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
                                span: *span,
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
                                span: *span,
                                def: None,
                            });

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
                                span: *span,
                                def: None,
                            });
                        }
                    } else {
                        // 无法获取类型，使用默认处理
                        let func_operand = self.resolve_function_name(func)?;
                        instructions.push(Instruction::Call {
                            dst: Some(result_reg.clone()),
                            func: func_operand,
                            args: arg_regs,
                            span: *span,
                            def: None,
                        });
                    }
                } else {
                    // 非 print 调用或无参数，使用默认处理
                    // ========== 默认函数调用处理 ==========
                    let final_args: Vec<Operand> = arg_regs.clone();

                    let func_operand = self.resolve_function_name(func)?;
                    instructions.push(Instruction::Call {
                        dst: Some(result_reg.clone()),
                        func: func_operand,
                        args: final_args,
                        span: *span,
                        def: None,
                    });
                }
            }
        }
        Ok(())
    }
}
