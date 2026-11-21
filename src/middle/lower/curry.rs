//! 柯里化族（`AstToIrGenerator` 的 curry 切层与生成）
//!
//! `split_curry` 按 `Type::Fn` 链切层，`generate_curry_*` 生成中间层/最内层与完整 IR 链；
//! `CurryLayer` 是一层签名的载体。方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use super::scope::NestedBodyGuard;
use super::AstToIrGenerator;
use crate::ast;
use crate::frontend::core::sema::{is_constraint_name, MonoType};
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, Operand,
};
use crate::util::diagnostic::Diagnostic;

/// 一层 curry 签名
///
/// 由 `split_curry` 从嵌套的 `Type::Fn` 拆出。
/// `params` 是这层的参数（带名字），从拍平的 curry 形参切出。
/// `return_type` 是这层的返回类型：还有下一层时是 `Type::Fn`，最后一层是值类型。
/// 一层 curry 签名
struct CurryLayer {
    params: Vec<ast::Param>,
    return_type: ast::Type,
}

impl AstToIrGenerator {
    ///
    /// 输入：`type_annotation` 是 `Type::Fn` 的嵌套结构（如 `(N: Int) -> (n: N) -> Int`）
    /// 输入：`curry_params` 是拍平的所有 curry 参数（如 `[N: Int, n: N]`）
    /// 输出：按层切分的 `CurryLayer` 列表
    ///
    /// 非 curry 函数（如 `(a: Int, b: Int) -> Int`）返回 1 个 layer，
    /// 其 `return_type` 不是 `Type::Fn`。
    #[allow(clippy::while_let_loop)]
    fn split_curry(
        type_ann: &ast::Type,
        curry_params: &[ast::Param],
    ) -> Vec<CurryLayer> {
        let mut layers = Vec::new();
        // 用索引游标而非迭代器：Paren 分支需要把已取走的层参数“退回”。
        let mut cursor = 0usize;
        let mut current_type = type_ann.clone();

        loop {
            match current_type {
                ast::Type::Fn {
                    params: type_params,
                    return_type,
                } => {
                    let start = cursor;
                    // 运行期形参位只数**带类型**的形参（基线口径）：参数位的
                    // 裸标识符（`(Int64)`→name="Int64"、ty=None）不占运行期槽位。
                    // 它是否算参数由 WBS 8.9.1（参数位形态归一）裁决，6.5 只做
                    // 零行为差分的表示重塑（锁定测试：middle/ir/tests/fn_param_slots.rs）。
                    let runtime_param_count = type_params
                        .iter()
                        .filter(|p| p.occupies_runtime_param_slot())
                        .count();
                    cursor = (cursor + runtime_param_count).min(curry_params.len());
                    let layer_params: Vec<ast::Param> = curry_params[start..cursor].to_vec();
                    let ret_type = *return_type;
                    current_type = ret_type.clone();
                    // 类型参数层（如 `(T: Type)`）是编译期参数：不占运行时参数位，
                    // 该层擦除（不生成运行时函数层）。调用点 T 由类型推断填充（RFC-011）。
                    // ponytail: 仅处理纯类型参数层；类型/值参数混合同层视为值层（罕见，暂不拆）
                    let is_type_layer = !type_params.is_empty()
                        && type_params.iter().all(|p| {
                            p.ty.as_ref().is_some_and(|t| {
                                Self::is_type_param_ann(t) || Self::is_constraint_param_ann(t)
                            })
                        });

                    // RFC-004 括号语义：返回位置的 `Paren` ⇔ 链条在此终止。
                    // 该括号内的参数**不属于本函数**——它们是被返回函数的参数。
                    // parser 把签名拍平成 `curry_params`（内层名供 body 对齐类型），
                    // 故此处必须把游标退回，否则 `f: () -> ((a:Int)->Int)` 的 `a`
                    // 会被当成本层参数，`f` 变成接受 `a` 而非返回闭包。
                    // RFC-027 §3：具名括号（`(r: P(r))`）与 `Paren` 同款——
                    // 括号声明“这是一个完整类型”，链条到此终止。
                    // 判定收敛到 `Type::is_paren_complete` 单点（#510 收口备案
                    // 的第三处：parser paren_return / typecheck 值级分解 /
                    // 此处 split_curry 三点共用，防判据漂移）。
                    let next_is_paren = ret_type.is_paren_complete();
                    if next_is_paren {
                        cursor = start;
                    }

                    if !is_type_layer {
                        layers.push(CurryLayer {
                            params: layer_params,
                            return_type: ret_type,
                        });
                    }
                    if next_is_paren {
                        break;
                    }
                }
                _ => break,
            }
        }
        layers
    }

    /// 该类型注解是否为**类型位**——编译期擦除，不占运行时参数位。
    ///
    /// 两类：具体元类型 `Type`（`T: Type`），以及**高阶类型参数**的标注
    /// `(T: Type) -> Type`（构造器变量，RFC-010 元类型层级的 Type1/Type2+）。
    /// 后者此前被当成值层，`(F: (T: Type) -> Type, A: Type) -> (x: A) -> A`
    /// 的 curry 外壳被保留，构造器类型实参对应的值级函数永不生成
    /// （运行期返回值是闭包而非调用结果）。判据委托 parser 的
    /// `is_type_level_fn`——与声明侧类型参数分类同一实现，不内联第二份。
    fn is_type_param_ann(ty: &ast::Type) -> bool {
        match ty {
            ast::Type::MetaType { .. } => true,
            ast::Type::Name { name, .. } => name == "Type",
            ast::Type::Fn {
                params,
                return_type,
            } => crate::ast::is_type_level_fn(params, return_type),
            _ => false,
        }
    }

    /// 约束形参判定（RFC-011 §2.1）：标注是运算符接口/约束名（`T: Add` 的
    /// `Add`）→ 类型位。split_curry 的层擦除判据与它同源——漏判会让约束
    /// 形参落成运行时参数（sum 外层 Parameters: (Add)），调用 arity 全错。
    fn is_constraint_param_ann(ty: &ast::Type) -> bool {
        match ty {
            ast::Type::Name { name, .. } => is_constraint_name(name),
            _ => false,
        }
    }

    /// 把**一层 curry 的具名形参**写进槽位并携带量词事实——中间层与最内层共用。
    ///
    /// `env_params` 是外层各层累积的参数（按 `args` 槽序 0..），`layer_params` 是本层
    /// 参数（`env_count..`）；这正是 `MakeClosure.env` 需要的顺序，也与最内层 body 的
    /// 槽布局一致。
    ///
    /// 事实来源只有一个：`ast::Param::quantity`（声明侧由唯一解析源
    /// `ast::resolve_param_quantities` 写入）。此处**不推导、不默认**——`None`（尚未
    /// 解析）原样落进槽位，绝不当 `Some(Zero)`。
    fn register_curry_layer_bindings(
        &mut self,
        env_params: &[ast::Param],
        layer_params: &[ast::Param],
        env_count: usize,
    ) {
        for (i, param) in env_params.iter().enumerate() {
            self.register_local(&param.name, i);
            self.note_slot_quantity(&param.name, param.quantity);
        }
        for (i, param) in layer_params.iter().enumerate() {
            self.register_local(&param.name, env_count + i);
            self.note_slot_quantity(&param.name, param.quantity);
        }
    }

    /// 生成 curry 中间层的 FunctionIR
    ///
    /// 中间层职责：LoadArg 本层参数 → MakeClosure 包装下一层 → Ret(闭包)
    #[allow(clippy::too_many_arguments)]
    fn generate_curry_intermediate_func(
        &mut self,
        func_name: &str,
        layer: &CurryLayer,
        env_params: &[ast::Param],
        next_func_name: &str,
        constants: &mut Vec<ConstValue>,
        generic_params: Option<Vec<String>>,
    ) -> Result<FunctionIR, Diagnostic> {
        let _ = constants; // 中间层不生成常量
        let mut instructions = Vec::new();

        // 无需 LoadArg 搬动：args 槽布局 = [外层 env(0..env_count), 本层参数(env_count..)]
        // 正是下一层闭包 env 需要的顺序。搬动会因 dst/src 槽重叠互相覆盖（#294）。
        let param_count = layer.params.len();
        let env_count = env_params.len();
        // P0（6.9.2-C2a）：包装层与最内层**同一份**具名绑定事实——此前这里既不
        // register_local 也不写量词，擦除侧只能靠 generic_params + params.len() 反推
        // 哪一槽是 q=0（D3 第 1 条）。事实来自声明侧 `Param::quantity`，无第二套推导。
        self.register_curry_layer_bindings(env_params, &layer.params, env_count);
        let full_env: Vec<Operand> = (0..env_count + param_count).map(Operand::Local).collect();
        self.next_temp = env_count + param_count;
        self.temp_high_water = env_count + param_count;
        let closure_dst = self.next_temp_reg();
        instructions.push(Instruction::MakeClosure {
            dst: closure_dst.clone(),
            func: next_func_name.to_string(),
            env: full_env,
            def: None,
            span: self.cur_span,
        });

        // Ret(closure)
        instructions.push(Instruction::Ret {
            value: Some(closure_dst.clone()),
            span: self.cur_span,
        });

        // 5. 构建 FunctionIR
        // WBS 2.1.2：同上——无注解参数占位而非丢弃
        let param_types: Vec<MonoType> = layer
            .params
            .iter()
            .map(|p| {
                p.ty.clone()
                    .map(MonoType::from)
                    .unwrap_or(MonoType::Int(64))
            })
            .collect();
        let return_type: MonoType = layer.return_type.clone().into();
        // 槽位表 + 寄存器预算（E3014）单点
        let locals_types: Vec<LocalSlot> = self.take_body_locals()?;
        let (values, value_slots) = self.take_body_values(&instructions, &param_types);

        Ok(FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: func_name.to_string(),
            params: param_types,
            return_type,
            generic_params,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals_types,
                values,
                value_slots,
            },
        })
    }

    /// 生成 curry 最内层的 FunctionIR
    ///
    /// 最内层职责：LoadUpvalue 读所有外层参数 → LoadArg 本层参数 → 执行原 body → Ret
    /// env_param_names：所有外层参数名（按累积顺序）
    #[allow(clippy::too_many_arguments)]
    fn generate_curry_innermost_func(
        &mut self,
        func_name: &str,
        layer: &CurryLayer,
        env_params: &[ast::Param],
        body: &[ast::Stmt],
        constants: &mut Vec<ConstValue>,
        generic_params: Option<Vec<String>>,
    ) -> Result<FunctionIR, Diagnostic> {
        let env_count = env_params.len();
        // WBS 7a.2：curry 最内层是独立指令流——父帧（curry 函数那一层）整体摘进
        // guard，Drop 时原样返回。
        let mut frame = NestedBodyGuard::enter(self);

        let mut instructions = Vec::new();

        // 参数原位注册，不生成 LoadArg：args 槽布局 = [外层参数(0..env_count),
        // 本层参数(env_count..)]，正是 body 需要的最终布局。LoadArg 搬动会因
        // dst/src 槽重叠互相覆盖（#294：先写 slot[i] 再读 slot[i] 已坏）。
        //
        // 量词事实随形参一起落到槽位（`LocalSlot::quantity` 的唯一写入点之一）：
        // 外层零号位（`(N: Int) -> (n: N)` 的 N）在本层正是「q=0 具名绑定」，
        // 擦除前识别体内对它的引用要靠这条事实。
        // 与中间层**同一处**（`register_curry_layer_bindings`）写绑定与量词事实：
        // 两个层的槽布局相同（env 在前、本层参数在后），事实同源（`Param::quantity`）。
        frame.register_curry_layer_bindings(env_params, &layer.params, env_count);
        // curry 层形参全部来自声明注解本身（层参数由 `split_curry` 从注解切出），
        // 无需声明补位。
        frame.note_param_written_types(&layer.params, &[]);

        // next_temp 起始 = 参数总数
        frame.next_temp = env_count + layer.params.len();
        frame.temp_high_water = env_count + layer.params.len();
        frame.temp_floor = 0;

        let return_type: MonoType = layer.return_type.clone().into();

        // RFC-010a 规则①：尾表达式给出函数值。沿用共用的 `generate_tail_stmt_ir`，
        // 否则 `f: (a: Int) -> (b: Int) -> Int = { a + b }` 返回 Void；
        // 取值不以注解为条件（#343 根因一）。
        let last_idx = body.len().checked_sub(1);
        let mut tail_handled = false;
        for (i, stmt) in body.iter().enumerate() {
            // 语句边界设定当前 span，供本条语句的所有指令构造就地捕获
            frame.set_cur_span(stmt.span);
            if Some(i) == last_idx {
                let result_reg = frame.next_temp_reg();
                if frame.generate_tail_stmt_ir(
                    stmt,
                    result_reg.clone(),
                    &mut instructions,
                    constants,
                )? {
                    // 尾语句路径（RFC-039 D20 逐案核实项）：本条语句经
                    // `generate_tail_stmt_ir` 提前返回，**必须**在同款语句边界上
                    // 消费它的释放条目——漏这一处，尾语句的 Drop 就静默丢失
                    //（实测红态里正是这一类：尾位置语句的计划键永不命中）。
                    // 取值已完成（值在 result_reg），故 Drop 排在 Ret 之前。
                    frame.emit_release_drops(stmt.plan_id, &mut instructions);
                    instructions.push(Instruction::Ret {
                        value: Some(result_reg.clone()),
                        span: frame.cur_span,
                    });
                    frame.set_cur_span(stmt.span);
                    tail_handled = true;
                    break;
                }
            }
            frame.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
            // NLL Release: 语句边界消费释放计划（RFC-039 D20）
            frame.emit_release_drops(stmt.plan_id, &mut instructions);
            frame.set_cur_span(stmt.span);
        }
        if !tail_handled {
            instructions.push(Instruction::Ret {
                value: None,
                span: frame.cur_span,
            });
        }

        // WBS 2.1.2：同上——无注解参数占位而非丢弃
        let param_types: Vec<MonoType> = layer
            .params
            .iter()
            .map(|p| {
                p.ty.clone()
                    .map(MonoType::from)
                    .unwrap_or(MonoType::Int(64))
            })
            .collect();
        // 槽位表 + 寄存器预算（E3014）单点
        let locals_types: Vec<LocalSlot> = frame.take_body_locals()?;
        let (values, value_slots) = frame.take_body_values(&instructions, &param_types);

        Ok(FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: func_name.to_string(),
            params: param_types,
            return_type,
            generic_params,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals_types,
                values,
                value_slots,
            },
        })
    }

    /// 生成 curry 函数的完整 IR 链
    ///
    /// 从外（layer 0）往内（layer N-1）遍历 layers，每层生成一个 FunctionIR。
    /// 最外层用原名 `name`，内层用 `__{name}_l{index}`。
    ///
    /// env_param_names 在循环中累积：进入第 k 层时，它包含 layer 0..k-1 的所有参数名。
    /// 中间层函数从 Arg(0..env_count) 加载外层参数作为 env，从 Arg(env_count..) 加载本层参数。
    /// 最内层函数同理加载后执行原 body。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn generate_curry_function_ir(
        &mut self,
        name: &str,
        type_annotation: &ast::Type,
        all_params: &[ast::Param],
        body: &[ast::Stmt],
        constants: &mut Vec<ConstValue>,
        generic_params: Option<Vec<String>>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        // 括号返回 / 闭包包装：把声明**返回位**内层 Fn 的形参递给闭包 IR 定型
        //（parser 不再填 AST 标注；入口 save、出口还原，见 generate_function_ir 同款）。
        let closure_decl_params = self.paren_return_value_params(Some(type_annotation));
        let saved_closure_decl =
            std::mem::replace(&mut self.closure_decl_params, closure_decl_params);
        // 量词事实（唯一解析源）：层形参与 env 名都来自这份声明事实，故先把
        // 「声明形参」解析成带 quantity 的同一批 `Param`，再按层切分——切分本身
        // 仍是 `split_curry`（顺序/长度不变），不新增第二套参数数组。
        let resolved_all_params = self.with_param_classifier(|cls| {
            ast::resolve_param_quantities(all_params, &[type_annotation], cls)
        });
        let layers = Self::split_curry(type_annotation, &resolved_all_params);

        let mut layer_funcs: Vec<FunctionIR> = Vec::new();
        // env 追踪携带**完整形参**（名字 + 量词事实），不是名字平行数组。
        let mut env_params: Vec<ast::Param> = Vec::new();

        // WBS 7a.2：curry 的各层是独立指令流——父帧整体摘进 guard，Drop 时原样
        // 返回；每一层在循环内自设基态（见下）。
        let mut frame = NestedBodyGuard::enter(self);

        for (i, layer) in layers.iter().enumerate() {
            let is_innermost = i == layers.len() - 1;
            let is_outermost = i == 0;
            let func_name = if i == 0 {
                name.to_string()
            } else {
                format!("__{}_l{}", name, i - 1)
            };

            // 只有最外层函数保留 generic_params（用于单态化）
            let layer_generic_params = if is_outermost {
                generic_params.clone()
            } else {
                None
            };

            // 进入新作用域
            frame.enter_scope();
            // 重置本层的临时寄存器（每层独立计数）
            frame.next_temp = 0;
            frame.temp_high_water = 0;
            frame.temp_floor = 0;
            // 每层是独立 FunctionIR：值表与 ValueId 也按层重置（7b.2）。
            frame.next_value = 0;
            frame.cur_values.clear();
            frame.cur_value_slots.clear();

            let func = if is_innermost {
                frame.generate_curry_innermost_func(
                    &func_name,
                    layer,
                    &env_params,
                    body,
                    constants,
                    layer_generic_params,
                )?
            } else {
                let next_name = format!("__{}_l{}", name, i);
                frame.generate_curry_intermediate_func(
                    &func_name,
                    layer,
                    &env_params,
                    &next_name,
                    constants,
                    layer_generic_params,
                )?
            };
            frame.exit_scope();

            // 累积本层参数到 env 追踪（给 innermost_func 用）
            env_params.extend(layer.params.iter().cloned());
            layer_funcs.push(func);
        }

        // 内层函数加入 nested_functions，最外层返回给调用者
        // 第 0 个是最外层，其余是内层
        let outer = layer_funcs.remove(0);
        frame.nested_functions.extend(layer_funcs);
        // 还原闭包定型上下文（避免串到下一个函数）——非寄存器状态，仍手工配对
        frame.closure_decl_params = saved_closure_decl;

        Ok(Some(outer))
    }
}
