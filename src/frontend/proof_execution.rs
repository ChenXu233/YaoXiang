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
//! # 为什么不放 typecheck/proof/
//!
//! 执行要经 middle（IR/codegen）与 backends（解释器）——那是 L3 语义层
//! 依赖 L4 执行层的反向依赖（check-boundary 判据 B3 拦截）。编排层
//! （frontend 根部）是它唯一合法的家；P4 统一 Driver 时按 02 §改动清单
//! 移入 driver 作为 `Stage::ProofExecution` 的实现。

use crate::frontend::core::parser::ast::Module;
use crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall;
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

/// 执行全部证明义务，返回错误诊断（空 = 全部证明通过）。
///
/// 调用方契约：**仅在类型检查无错误时调用**（与 pipeline 的门控一致——
/// 带病 AST 执行证明函数只会产出噪声诊断）。
pub(crate) fn execute_proof_calls(
    proof_calls: &[ProofFunctionCall],
    ast: &Module,
    type_result: &TypeCheckResult,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    for call in proof_calls {
        match execute_single_proof_fn(call, ast, type_result) {
            Ok(true) => {
                // 证明通过，继续
            }
            Ok(false) => {
                // #322 M3：走注册表快捷方法（i18n 模板渲染）；
                // 证明函数返回 false 属"无具体反例"的证伪场景
                errors.push(
                    ErrorCodeDefinition::refinement_violated(&format!(
                        "证明函数 '{}' 返回 false，约束不满足（参数: {:?}）；检查约束条件或修改传入值",
                        call.func_name, call.args,
                    ))
                    .param("counterexample", "（证明函数返回 false，无具体反例）")
                    .build(),
                );
            }
            Err(e) => {
                errors.push(
                    ErrorCodeDefinition::refinement_violated(&format!(
                        "证明函数 '{}' 执行失败: {}",
                        call.func_name, e
                    ))
                    .param("counterexample", "（证明函数执行出错，无具体反例）")
                    .build(),
                );
            }
        }
    }
    errors
}

/// 执行单个证明函数（RFC-027 Phase 2.5）
///
/// 优先使用 const 求值（约束表达式），回退到 IR/字节码管线（return 形式）
pub(crate) fn execute_single_proof_fn(
    call: &ProofFunctionCall,
    ast: &Module,
    type_result: &TypeCheckResult,
) -> Result<bool, String> {
    use crate::frontend::core::parser::ast::StmtKind;

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
                use crate::frontend::core::parser::ast::Expr;
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
                    signature_params,
                    definition,
                    ..
                } => {
                    if *name != call.func_name {
                        return None;
                    }
                    use crate::frontend::core::parser::ast::{Type, TypeBodyItem};
                    if let Type::Struct { body } = definition {
                        for item in body {
                            if let TypeBodyItem::Expr(Type::ConstExpr(expr)) = item {
                                let constraint_stmt = crate::frontend::core::parser::ast::Stmt {
                                    kind: StmtKind::Expr(expr.clone()),
                                    span: crate::util::span::Span::dummy(),
                                };
                                return Some((
                                    signature_params.clone(),
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

    let mut ir_gen = middle::core::ir_gen::AstToIrGenerator::new_with_type_result(
        type_result,
        crate::frontend::module::registry::ModuleRegistry::with_std(),
        None,
    );
    let mut constants: Vec<middle::core::ir::ConstValue> = Vec::new();
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

    let mut bytecode_module = crate::middle::core::bytecode::BytecodeModule::from(bytecode_file);
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
