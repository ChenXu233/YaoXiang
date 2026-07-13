//! 分发管道 —— 编译期 vs 运行时检查分派
//!
//! RFC-027 §4.1 四级分派的顶层调度层：
//! 1. 提取约束中的自由变量
//! 2. 判断每个变量能否在编译期解析（bindings / Γ 中）→ CompileTime or Runtime
//! 3. CompileTime: 调用 check_predicate 做完整编译期证明
//! 4. Runtime: 注入 Γ 使其在运行时生效
//! 5. 编译期未证明但 Runtime 可行时：InsertCheck + 警告

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::verdict::{ProofResult, UnprovenReason};
use crate::util::diagnostic::Diagnostic;

use super::predicate::check_predicate;

// ===================================================================
// 自由变量提取
// ===================================================================

/// 从 ConstExpr 中提取所有自由变量名（去重后的 Vec）
pub fn extract_free_vars(expr: &ConstExpr) -> Vec<String> {
    let mut vars = Vec::new();
    collect_free_vars(expr, &mut vars);
    vars.sort();
    vars.dedup();
    vars
}

fn collect_free_vars(
    expr: &ConstExpr,
    out: &mut Vec<String>,
) {
    match expr {
        ConstExpr::NamedVar(name) => out.push(name.clone()),
        ConstExpr::Var(var) => out.push(var.to_string()),
        ConstExpr::BinOp { left, right, .. } => {
            collect_free_vars(left, out);
            collect_free_vars(right, out);
        }
        ConstExpr::UnOp { expr: inner, .. } => collect_free_vars(inner, out),
        ConstExpr::Call { args, .. } => {
            for a in args {
                collect_free_vars(a, out);
            }
        }
        // Lit, If, Range 不含变量引用
        _ => {}
    }
}

// ===================================================================
// 分派模式 —— 编译期 vs 运行时
// ===================================================================

/// 变量分发模式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchMode {
    /// 所有变量都在编译期可解析 → 走 check_predicate
    CompileTime,
    /// 存在变量仅在运行时可知 → 走 gamma 注入
    Runtime,
}

/// 判断约束分派模式
///
/// 提取约束中的自由变量，逐一检查：
/// - 在 bindings 中 → CompileTime
/// - 在 Γ 活跃假设中 → CompileTime
/// - 否则 → Runtime（只有运行时才知道值）
pub fn dispatch(
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
    gamma: &FlowSensitiveGamma,
) -> DispatchMode {
    let free_vars = extract_free_vars(constraint);

    if free_vars.is_empty() {
        // 纯字面量约束（如 5 > 0），无需变量绑定 → CompileTime
        return DispatchMode::CompileTime;
    }

    let gamma_exprs = gamma.current();
    for var in &free_vars {
        let in_bindings = bindings.contains_key(var);
        let in_gamma = gamma_exprs.iter().any(|expr| {
            // 检查 gamma 假设中是否引用了该变量
            references_var_in_expr(expr, var)
        });

        if !in_bindings && !in_gamma {
            return DispatchMode::Runtime;
        }
    }

    DispatchMode::CompileTime
}

/// 递归检查表达式中是否引用了指定变量名
fn references_var_in_expr(
    expr: &ConstExpr,
    var_name: &str,
) -> bool {
    match expr {
        ConstExpr::NamedVar(n) => n == var_name,
        ConstExpr::BinOp { left, right, .. } => {
            references_var_in_expr(left, var_name) || references_var_in_expr(right, var_name)
        }
        ConstExpr::UnOp { expr: inner, .. } => references_var_in_expr(inner, var_name),
        _ => false,
    }
}

// ===================================================================
// CompileTime 结果
// ===================================================================

/// 编译期证明结果
#[derive(Debug, Clone)]
pub enum CompileTimeOutcome {
    /// 证明成立 → 约束已满足，无需运行时检查
    Erased,
    /// 证伪 → 编译期错误
    Error(Diagnostic),
    /// 无法证明 → 可降级为运行时检查（需要程序员提供证明函数）
    RequiresProof(Diagnostic),
}

/// 执行编译期谓词检查
///
/// 调用 `check_predicate` 并将 `ProofResult` 映射为 `CompileTimeOutcome`。
pub fn dispatch_compiletime(
    ctx: &ProofContext<'_>,
    refined: &MonoType,
    bindings: &HashMap<String, ConstValue>,
) -> CompileTimeOutcome {
    match check_predicate(ctx, refined, bindings) {
        ProofResult::Proved => CompileTimeOutcome::Erased,
        ProofResult::Disproved(model) => CompileTimeOutcome::Error(model.into_diagnostic()),
        ProofResult::Unproven {
            reason: UnprovenReason::ProofFunctionRequired,
            ..
        } => {
            // 需要证明函数 → RequiresProof 而非 Error
            let diagnostic = Diagnostic::error(
                "E8002".to_string(),
                "需要证明函数来验证约束".to_string(),
                "添加证明函数或提供运行时检查".to_string(),
                None,
            );
            CompileTimeOutcome::RequiresProof(diagnostic)
        }
        ProofResult::Unproven { reason, .. } => {
            // 超期预算/超出规则 → 错误
            let diagnostic = Diagnostic::error(
                "E8001".to_string(),
                format!("无法证明: {:?}", reason),
                String::new(),
                None,
            );
            CompileTimeOutcome::Error(diagnostic)
        }
    }
}

// ===================================================================
// Runtime 结果
// ===================================================================

/// 运行时检查结果
#[derive(Debug, Clone)]
pub enum RuntimeOutcome {
    /// 插入运行时约束检查
    InsertCheck {
        /// 需在运行时验证的约束表达式
        constraint: ConstExpr,
    },
}

/// 执行运行时路径 —— 将约束注入 Γ
pub fn dispatch_runtime(
    gamma: &mut FlowSensitiveGamma,
    constraint: &ConstExpr,
) -> RuntimeOutcome {
    gamma.inject(constraint.clone());
    RuntimeOutcome::InsertCheck {
        constraint: constraint.clone(),
    }
}

// ===================================================================
// 分发总结果
// ===================================================================

/// 分发管道的最终结果
#[derive(Debug, Clone)]
pub enum DispatchOutcome {
    /// 编译期分支 —— 约束在编译期已被处理
    CompileTime(CompileTimeOutcome),
    /// 运行时分支 —— 约束被注入 Γ，需要在运行时检查
    Runtime(RuntimeOutcome),
    /// 编译期无法完全证明，但运行时检查已注入（含警告）
    CompileTimeWithWarning {
        /// 编译期子结果
        outcome: CompileTimeOutcome,
        /// 警告诊断
        warning: Diagnostic,
    },
}

/// 完整分发管道
///
/// 1. `dispatch` 确定分派模式
/// 2. CompileTime → `dispatch_compiletime` → 编译期证明
/// 3. Runtime → `dispatch_runtime` → Γ 注入（运行时检查）
/// 4. 编译期 Unproven 但 Runtime 可行时 → CompileTimeWithWarning + 注入
///
/// # 参数
/// - `gamma`: 流敏感假设集（可变引用，用于 inject）
/// - `ctx`: 证明上下文
/// - `constraint`: 约束表达式（用于分派 + runtime inject）
/// - `refined`: 精化类型（用于 check_predicate）
/// - `bindings`: 变量名 → 具体值映射
///
/// # 返回
/// `DispatchOutcome` 枚举
pub fn dispatch_pipeline(
    gamma: &mut FlowSensitiveGamma,
    ctx: &ProofContext<'_>,
    constraint: &ConstExpr,
    refined: &MonoType,
    bindings: &HashMap<String, ConstValue>,
) -> DispatchOutcome {
    let mode = dispatch(constraint, bindings, gamma);

    match mode {
        DispatchMode::CompileTime => {
            let outcome = dispatch_compiletime(ctx, refined, bindings);
            match &outcome {
                CompileTimeOutcome::Erased => DispatchOutcome::CompileTime(outcome),
                CompileTimeOutcome::Error(_) => DispatchOutcome::CompileTime(outcome),
                CompileTimeOutcome::RequiresProof(_) => {
                    // 编译期未证明 → 尝试注入运行时检查 + 警告
                    let warning = Diagnostic::warning(
                        "W8001".to_string(),
                        "编译期无法证明约束，已降级为运行时检查".to_string(),
                        "考虑添加证明函数以提高安全性".to_string(),
                        None,
                    );
                    dispatch_runtime(gamma, constraint);
                    DispatchOutcome::CompileTimeWithWarning { outcome, warning }
                }
            }
        }
        DispatchMode::Runtime => {
            let outcome = dispatch_runtime(gamma, constraint);
            DispatchOutcome::Runtime(outcome)
        }
    }
}
