//! 编译期证明函数执行（RFC-027 Phase 2.5）——编排层共享原语
//!
//! 职责：把 typecheck 产出的证明义务（`TypeCheckResult::proof_calls`，经
//! getter 取得）逐一编译执行，返回错误诊断列表（空 = 全部证明通过）。
//!
//! # 为什么独立成模块（而不是留在 pipeline.rs）
//!
//! 义务的**生产端**在 checker（三处 extend 分支），**消费端**却分布在每条
//! 编译路径：单文件 Pipeline 与 orchestrator 四入口（compile_project /
//! check_project / check_source_in_project / compile_embedded_module）。
//! 此前实现只挂在 pipeline.rs 一个方法里，orchestrator 够不到，义务在
//! 多文件路径被静默丢弃（02-stage-contract §正确性漏洞证据链，WBS 3.1.2）。
//! 禁令三要求同一行为只有一处实现，故抽为中立模块供各入口共享。
//!
//! # 为什么不放 sema/proof/
//!
//! 执行要经 middle（IR/codegen）与 backends（解释器）——那是 L3 语义层
//! 依赖 L4 执行层的反向依赖（check-boundary 判据 B3 拦截）。编排层
//! （frontend 根部）是它唯一合法的家；P4 统一 Driver 时按 02 §改动清单
//! 移入 driver 作为 `Stage::ProofExecution` 的实现。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::ast::Module;
use crate::frontend::core::sema::proof::dep_graph::constraint_is_terminates;
use crate::frontend::core::sema::proof::verdict::ProofFunctionCall;
use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

/// 嵌套义务展开的编译期预算上限（RFC-027 §8：预算硬限制是停机问题的工程解）。
///
/// 展开是**广度优先工作清单**：被调证明函数的形参精化以实参代入后，Call 形态
/// 入队继续执行。两条非终止面在此收口——①精化链自引用（`A: (x: Int, p: A(x, 0))
/// -> Type`：A(5, 0) 的嵌套义务还是 A(5, 0)）；②实参逐层变化的链（`p: A(x - 1, 0)`
/// 展开出逐个不同的调用）。①由去重集钉死，②由本预算钉死；合法程序的义务链
/// 远小于此界。触界即**停止展开**（不判 ≠ 判伪——已展开的义务照常判定）。
const NESTED_OBLIGATION_BUDGET: usize = 128;

/// 嵌套义务展开的共享工作台：待执行清单 / 模块常量值环境 / 去重集 / 预算 /
/// 展开期直接判伪的闭合约束诊断汇。
///
/// 执行与展开共用一个状态对象（广度优先清单是**递归展开的扁平化**——Rust 侧
/// 不递归，栈深度与义务链深度解耦）。生产端只写 `pending` / `errors`，其余字段
/// 在 `execute_proof_calls` 构造后只读。
struct ProofExecState {
    /// 待执行的证明调用（初始义务 + 展开产出的嵌套义务）
    pending: VecDeque<ProofFunctionCall>,
    /// 模块级常量名 → 值（嵌套义务闭合的值环境；实参代入之外的唯一值源）
    constants: HashMap<String, ConstValue>,
    /// 已登记的证明调用去重键（`"{func}:{args:?}"`）——初始义务种入，
    /// 嵌套义务命中即跳过（自引用循环与逐字重复诊断的收口）
    seen: HashSet<String>,
    /// 嵌套展开剩余预算
    nested_budget: usize,
    /// 展开期直接判伪的**闭合 Bool 约束**诊断（Call 形态走 pending 执行，
    /// 不落这里）
    errors: Vec<Diagnostic>,
}

/// 执行全部证明义务，返回错误诊断（空 = 全部证明通过）。
///
/// 执行是**广度优先工作清单**：初始义务逐一执行；执行某条义务时，被调证明
/// 函数**自身的形参精化**以调用实参代入（RFC-027 §3.3 调用点前置条件——
/// issue #537），产生的嵌套义务追加进清单继续执行。诊断只从两条既有形态出：
/// 被调函数体判 false（`proof_false.note`）与执行失败（`proof_error.note`）；
/// 嵌套闭合 Bool 约束判伪的 E4018 由展开期直接写入 [`ProofExecState::errors`]。
///
/// 调用方契约：**仅在类型检查无错误时调用**（与 pipeline 的门控一致——
/// 带病 AST 执行证明函数只会产出噪声诊断）。
// pub(crate)：driver（L1）的 Stage::ProofExecution 臂是唯一新调用方
// （WBS 4.1.3，02 §改动清单）；orchestrator 存量调用点随 4.2 迁走后归位再议
pub(crate) fn execute_proof_calls(
    proof_calls: &[ProofFunctionCall],
    ast: &Module,
    type_result: &TypeCheckResult,
) -> Vec<Diagnostic> {
    let mut state = ProofExecState {
        pending: proof_calls.iter().cloned().collect(),
        constants: collect_module_constants(ast),
        seen: proof_calls
            .iter()
            .map(|call| proof_call_key(&call.func_name, &call.args))
            .collect(),
        nested_budget: NESTED_OBLIGATION_BUDGET,
        errors: Vec::new(),
    };
    while let Some(call) = state.pending.pop_front() {
        match execute_single_proof_fn(&call, ast, type_result, &mut state) {
            Ok(true) => {
                // 证明通过，继续
            }
            Ok(false) => {
                // #322 M3：走注册表快捷方法（i18n 模板渲染）；
                // 证明函数返回 false 属"无具体反例"的证伪场景——说明文字走
                // locales per-site 键（issue #469 处 1/2a），随 error_lang 渲染
                state.errors.push(
                    ErrorCodeDefinition::refinement_violated(&format!(
                        "证明函数 '{}' 返回 false，约束不满足（参数: {:?}）；检查约束条件或修改传入值",
                        call.func_name, call.args,
                    ))
                    .param_from_key("counterexample", "E4018.proof_false.note")
                    .build(),
                );
            }
            Err(e) => {
                state.errors.push(
                    ErrorCodeDefinition::refinement_violated(&format!(
                        "证明函数 '{}' 执行失败: {}",
                        call.func_name, e
                    ))
                    .param_from_key("counterexample", "E4018.proof_error.note")
                    .build(),
                );
            }
        }
    }
    state.errors
}

/// 证明调用去重键：函数名 + 实参 Debug 文本。ConstValue 无 Hash/Eq，用
/// Debug 文本作键只服务**循环与逐字重复**的识别（RFC-027 §8 预算的工程形态），
/// 不承担任何语义身份判定。
fn proof_call_key(
    func: &str,
    args: &[ConstValue],
) -> String {
    format!("{func}:{args:?}")
}

/// 折叠模块级**不可变绑定**的字面值（`K: Int = 7` → `K ⇒ Int(7)`）。
///
/// 嵌套义务（被调证明函数形参精化）的自由变量是**被调方声明作用域**的名字：
/// 它的形参名（由实参代入）或模块级常量。后者在此求值——按声明序逐条折叠，
/// 前面的常量供后面的初始化式引用（`K = 7; L = K + 1` ⇒ `L = 8`）。
/// `mut` 绑定与非字面可折叠值不收（值不固定，义务不闭合——宁可少报不判，
/// 与 `check_call_arg_refinements` 的缺值闸门同一保守方向）。
fn collect_module_constants(ast: &Module) -> HashMap<String, ConstValue> {
    use crate::ast::{Expr, StmtKind};

    let mut constants = HashMap::new();
    let mut evaluator = crate::frontend::core::types::eval::const_eval::ConstGenericEval::new();
    for stmt in &ast.items {
        let StmtKind::Assign {
            target,
            value: Some(value),
            is_mut,
            ..
        } = &stmt.kind
        else {
            continue;
        };
        if *is_mut {
            continue;
        }
        let Expr::Var(name, _) = target.as_ref() else {
            continue;
        };
        let Some(const_expr) =
            crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(value)
        else {
            continue;
        };
        if let Ok(value) = evaluator.eval(&const_expr) {
            evaluator.bind_var(name.clone(), value.clone());
            constants.insert(name.clone(), value);
        }
    }
    constants
}

/// RFC-027 §3.3 接线：展开**被调证明函数自身的形参精化**（issue #537）。
///
/// 被调签名取自 typecheck 的 `bindings`（checker 经 `resolve_type_annotation`
/// 归一化的唯一真相源——精化约束已是 `ConstExpr`，此处不做第二份 AST 解析）。
/// 形参名取自声明 AST（`MonoType` 侧无名字——与 `check_call_arg_refinements`
/// 读 `declared_value_params` 同一理由）。对每个非 `Terminates` 的精化形参：
/// 约束以调用实参**按声明名代入**，再按形态分派——
/// - `Call`：实参全为字面量/模块常量时构造嵌套证明调用入队（去重 + 预算收口）；
/// - 其余（谓词内联的 Bool 约束）：以形参实参 + 模块常量为值环境求值，
///   判 false 直接出 E4018（闭合约束，与 `revalidate_refined` 闭合支同形态）。
///
/// 三条**保守边界**（不判 ≠ 判伪，登记于 3.20.22 实录）：代入后仍有不可解析
/// 自由变量（非形参名的非常量绑定）不判；嵌套调用实参含不可折叠形态不判；
/// 预算耗尽不再展开（已展开的义务照常判定）。
fn expand_callee_param_refinements(
    call: &ProofFunctionCall,
    declared_params: &[crate::ast::Param],
    type_result: &TypeCheckResult,
    state: &mut ProofExecState,
) {
    let Some(poly) = type_result.bindings.get(&call.func_name) else {
        return;
    };
    let MonoType::Fn {
        params: mono_params,
        ..
    } = &poly.body
    else {
        return;
    };
    // 实参按声明位对齐；三条长度不一致即放弃展开（元数诊断归推断层/IR 层，
    // 本展开不越权、也不按猜测的位置造义务）
    if mono_params.len() != call.args.len() || declared_params.len() != call.args.len() {
        return;
    }
    let declared_names: Vec<String> = declared_params.iter().map(|p| p.name.clone()).collect();
    let actuals: Vec<ConstExpr> = call
        .args
        .iter()
        .map(|value| ConstExpr::Lit(value.clone()))
        .collect();
    for mono_param in mono_params {
        let MonoType::Refined { constraint, .. } = mono_param else {
            continue;
        };
        if constraint_is_terminates(mono_param) {
            continue;
        }
        let substituted = crate::frontend::core::sema::layers::termination::substitute_const_expr(
            constraint,
            &declared_names,
            &actuals,
        );
        match substituted {
            ConstExpr::Call { func, args } => {
                let Some(values) = const_expr_args_to_values(&args, &state.constants) else {
                    // 嵌套实参含不可折叠形态：义务不闭合，保守不判
                    continue;
                };
                let key = proof_call_key(&func, &values);
                if !state.seen.insert(key) {
                    // 自引用精化链（A 的精化引用 A）与逐字重复：跳过，预算面收口
                    continue;
                }
                if state.nested_budget == 0 {
                    continue;
                }
                state.nested_budget -= 1;
                state.pending.push_back(ProofFunctionCall {
                    func_name: func,
                    args: values,
                });
            }
            ref constraint => {
                // 谓词内联形态：闭合 Bool 约束直接判定（值环境 = 形参实参 + 模块常量）
                let mut bindings = state.constants.clone();
                for (name, value) in declared_names.iter().zip(call.args.iter()) {
                    bindings.insert(name.clone(), value.clone());
                }
                let mut evaluator =
                    crate::frontend::core::types::eval::const_eval::ConstGenericEval::new();
                for (name, value) in &bindings {
                    evaluator.bind_var(name.clone(), value.clone());
                }
                if let Ok(ConstValue::Bool(false)) = evaluator.eval(constraint) {
                    state.errors.push(
                        ErrorCodeDefinition::refinement_violated(&constraint.to_string())
                            .param_from_key("counterexample", "E4018.no_free_var.note")
                            .build(),
                    );
                }
            }
        }
    }
}

/// 嵌套证明调用的实参转换：字面量直取，名字查模块常量值环境；其余形态
/// （运算、嵌套调用等不可折叠项）返回 None——义务不闭合，保守不判。
fn const_expr_args_to_values(
    args: &[ConstExpr],
    constants: &HashMap<String, ConstValue>,
) -> Option<Vec<ConstValue>> {
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        match arg {
            ConstExpr::Lit(value) => values.push(value.clone()),
            ConstExpr::NamedVar(name) => values.push(constants.get(name).cloned()?),
            _ => return None,
        }
    }
    Some(values)
}

/// 执行单个证明函数（RFC-027 Phase 2.5）
///
/// 先展开被调函数**自身的形参精化**（RFC-027 §3.3 调用点前置条件，issue #537——
/// 嵌套义务追加进 [`ProofExecState::pending`] / 判伪诊断写入
/// [`ProofExecState::errors`]），再求值函数体：优先使用 const 求值（约束表达式），
/// 回退到 IR/字节码管线（return 形式）。
fn execute_single_proof_fn(
    call: &ProofFunctionCall,
    ast: &Module,
    type_result: &TypeCheckResult,
    state: &mut ProofExecState,
) -> Result<bool, String> {
    use crate::ast::StmtKind;

    // 1. 在 AST 中查找函数定义
    let (params, body_stmts, type_ann) = ast
        .items
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation,
                value,
                ..
            } => {
                use crate::ast::Expr;
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    _ => return None,
                };
                if name != call.func_name {
                    return None;
                }
                if let Some(v) = value {
                    if let Expr::Lambda { params, body, .. } = v.as_ref() {
                        Some((params.clone(), body.stmts.clone(), type_annotation.clone()))
                    } else if let Expr::Block(block) = v.as_ref() {
                        Some((Vec::new(), block.stmts.clone(), type_annotation.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
        .or_else(|| {
            // 同时搜索 TypeDefinition 项（类型级证明函数语法）
            ast.items.iter().find_map(|stmt| match &stmt.kind {
                StmtKind::TypeDefinition {
                    name,
                    type_annotation,
                    definition,
                    ..
                } => {
                    if *name != call.func_name {
                        return None;
                    }
                    use crate::ast::{Type, TypeBodyItem};
                    if let Type::Struct { body } = definition {
                        for item in body {
                            if let TypeBodyItem::Expr(Type::ConstExpr(expr)) = item {
                                // 合成语句（非解析产物）：无释放计划身份
                                let constraint_stmt = crate::ast::Stmt {
                                    kind: StmtKind::Expr(expr.clone()),
                                    span: crate::util::span::Span::dummy(),
                                    plan_id: crate::util::plan_id::PlanId::UNPLANNED,
                                };
                                return Some((
                                    type_annotation
                                        .as_ref()
                                        .map(crate::ast::Type::fn_params_flat)
                                        .unwrap_or_default(),
                                    vec![constraint_stmt],
                                    None,
                                ));
                            }
                        }
                    }
                    None
                }
                _ => None,
            })
        })
        .ok_or_else(|| format!("证明函数 '{}' 未在 AST 中找到", call.func_name))?;

    // 1.5 RFC-027 §3.3：被调函数自身的形参精化是本次调用的前置义务——先展开
    // （嵌套义务入队 / 闭合约束判伪），再求值体。没有这一步，`Check(7, 42)` 只看
    // 体 `n > 0` 为真就放行，`Check` 自己的 `p: Eq(n, 0)` 在 n=7 处违反 `Eq(7, 0)`
    // 的事实被静默吞掉（issue #537 的缺口）。
    expand_callee_param_refinements(call, &params, type_result, state);

    // 2. 提取约束表达式（Expr 或 Return 中的表达式）
    let constraint_expr = body_stmts.iter().find_map(|s| match &s.kind {
        StmtKind::Expr(e) => Some(e.as_ref().clone()),
        StmtKind::Return(Some(e)) => Some(e.as_ref().clone()),
        _ => None,
    });

    // 3. 优先 const 求值（精化约束语义：表达式是约束，不是返回值）
    if let Some(ref expr) = constraint_expr {
        if let Some(const_expr) =
            crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(expr)
        {
            // 实参数必须与形参一致：缺参静默绑定会让解释器把未绑定
            // 参数读成 Void，产出「Void vs Int」式错译（宁显式失败）
            if params.len() != call.args.len() {
                return Err(format!(
                    "证明函数 '{}' 期望 {} 个实参，得到 {} 个",
                    call.func_name,
                    params.len(),
                    call.args.len()
                ));
            }
            let mut evaluator =
                crate::frontend::core::types::eval::const_eval::ConstGenericEval::new();
            // 绑定参数：param name → proof call arg value
            for (i, param) in params.iter().enumerate() {
                if let Some(arg) = call.args.get(i) {
                    evaluator.bind_var(param.name.clone(), arg.clone());
                }
            }
            if let Ok(result) = evaluator.eval(&const_expr) {
                match result {
                    crate::frontend::core::types::ConstValue::Bool(b) => return Ok(b),
                    other => {
                        return Err(format!(
                            "证明函数 '{}' 约束求值结果不是 Bool: {:?}",
                            call.func_name, other
                        ))
                    }
                }
            }
        }
    }

    // 4. 回退：IR 生成 → 字节码 → 解释器（支持 return 形式的复杂证明函数）
    use crate::backends::common::value::from_const_value;
    use crate::backends::common::RuntimeValue;
    use crate::backends::interpreter::Interpreter;
    use crate::backends::Executor;
    use crate::middle;

    // 本路径的谓词命名空间显式传**空集**（6.9.2-A 把它从构造器默认值改为必需
    // 输入，此处是显式选择）：`generate_function_ir` 的 SPEC §7.1 分类器问
    // 「这个名字是不是谓词」，空集答「都不是」⇒ 谓词应用的实参位按类型引用计。
    // 它**不是** #537 静默丢义务的根因——隔离结论（3.20.22 探针矩阵）是执行器
    // 只求值体、不展开被调方自身形参精化；该缺口已由 1.5 步的
    // `expand_callee_param_refinements` 接线（RFC-027 §3.3）。单文件证明执行
    // 不消费跨模块谓词名，空集与本路径的裁决面一致。
    let mut ir_gen = middle::ir::ir_gen::AstToIrGenerator::new_with_type_result(
        type_result,
        crate::frontend::module::registry::ModuleRegistry::with_std(),
        None,
        crate::frontend::core::sema::probe::TypeLevelNamespace::default(),
    );
    let mut constants: Vec<middle::ir::ir::ConstValue> = Vec::new();
    let func_ir = ir_gen
        .generate_function_ir(
            &call.func_name,
            type_ann.as_ref(),
            &params,
            &body_stmts,
            &mut constants,
            None,
        )
        .map_err(|e| format!("证明函数 '{}' IR 生成失败: {}", call.func_name, e))?;

    let func_ir = func_ir.ok_or_else(|| {
        format!(
            "证明函数 '{}' 是 native 函数，不能编译期执行",
            call.func_name
        )
    })?;

    let module_ir = middle::ModuleIR {
        functions: vec![func_ir],
        ..Default::default()
    };

    let mut codegen = middle::passes::codegen::CodegenContext::new(module_ir);
    let bytecode_file = codegen
        .generate()
        .map_err(|e| format!("证明函数 '{}' 字节码编译失败: {}", call.func_name, e))?;

    let mut bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);
    bytecode_module.entry_point = None;

    let args: Vec<RuntimeValue> = call.args.iter().map(from_const_value).collect();

    let mut interpreter = Interpreter::new();
    interpreter
        .execute_module(&bytecode_module)
        .map_err(|e| format!("证明函数 '{}' 模块加载失败: {}", call.func_name, e))?;
    let func_id = bytecode_module
        .functions
        .iter()
        .position(|f| f.name == call.func_name)
        .ok_or_else(|| format!("证明函数 '{}' 在模块中未找到", call.func_name))?;
    let result = interpreter
        .call_function_by_id(
            crate::backends::common::value::FunctionId(func_id as u32),
            &args,
        )
        .map_err(|e| format!("证明函数 '{}' 执行失败: {}", call.func_name, e))?;

    match result {
        RuntimeValue::Bool(b) => Ok(b),
        other => Err(format!(
            "证明函数 '{}' 必须返回 Bool，实际返回: {:?}",
            call.func_name, other
        )),
    }
}
