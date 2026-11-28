//! ConstExpr → SMTCommand 翻译器
//!
//! 纯函数，无外部依赖。将编译期约束表达式翻译为 SMT-LIB 命令序列。
//! 假设栈的每一项成为背景断言，目标约束取反后送 Z3 做 check-sat：
//!   unsat = 目标在假设下成立 → Proved
//!   sat   = 存在反例 → Disproved

use std::collections::HashMap;

use super::ast::{SMTCommand, SMTExpr, SMTSort};
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue, UnOp};

/// 翻译编译期约束为 SMT 命令序列
///
/// # 参数
/// - `constraint`: 待验证的约束表达式
/// - `assumptions`: 当前程序点的假设栈
/// - `var_sorts`: 变量名 → SMT 排序的映射
///
/// # 返回
/// - `Some(commands)`：完整命令序列（declare-const + assert + 目标取反 + check-sat +
///   get-model）——**永不为空**（最少 CheckSat + GetModel 两条）；
/// - `None`：约束或任一假设**不可翻译**（位运算 / Float 字面量 / FieldPath 等，
///   见 [`translate_expr`]）。
///
/// 调用方必须把 `None` 当「**无法判断**」（Unproven / 升级），**不得**把空断言集
/// 交给求解器：空集 check-sat 恒 `sat`，会被读成反例（F10，提交信息）。
pub fn translate_constraint(
    constraint: &ConstExpr,
    assumptions: &[ConstExpr],
    var_sorts: &HashMap<String, SMTSort>,
) -> Option<Vec<SMTCommand>> {
    let mut commands = Vec::new();

    // 1. 声明所有变量
    for (var, sort) in var_sorts {
        commands.push(SMTCommand::DeclareConst(var.clone(), sort.clone()));
    }

    // 2. 背景断言（假设栈）
    // 不可翻译项（含证据字段路径）：整组命令不产出——显式 None，由调用方 gate 到
    // 非 SMT 路径（**不**返回空表：空表在求解器侧等价「无条件成立」= 伪反例）。
    for assumption in assumptions {
        let expr = translate_expr(assumption)?;
        commands.push(SMTCommand::Assert(expr));
    }

    // 3. 目标取反：(assert (not constraint))
    //    Z3 返回 unsat = 目标在所有假设下成立
    let goal = translate_expr(constraint)?;
    let not_goal = SMTExpr::App("not".into(), vec![goal]);
    commands.push(SMTCommand::Assert(not_goal));

    // 4. check-sat + get-model
    commands.push(SMTCommand::CheckSat);
    commands.push(SMTCommand::GetModel);

    Some(commands)
}

/// ConstExpr → SMTExpr 翻译。
///
/// 返回 `None` 表示该项不是 SMT 可翻译项——**不产出伪造 atom**。
/// V1 的 `FieldPath`（证据字段路径，RFC-027 §14.5）走此路：SMT 面无项语言。
pub(crate) fn translate_expr(expr: &ConstExpr) -> Option<SMTExpr> {
    match expr {
        // 字面量
        ConstExpr::Lit(value) => Some(match value {
            ConstValue::Int(n) => SMTExpr::Atom(n.to_string()),
            ConstValue::Bool(true) => SMTExpr::Atom("true".into()),
            ConstValue::Bool(false) => SMTExpr::Atom("false".into()),
            _ => return None,
        }),

        // 变量引用
        ConstExpr::NamedVar(name) => Some(SMTExpr::Atom(name.clone())),

        // Const 变量引用
        ConstExpr::Var(var) => Some(SMTExpr::Atom(var.to_string())),

        // 证据字段路径不是 SMT 项（RFC-027 §14.5）：显式拒绝，不产物化 atom。
        ConstExpr::FieldPath { .. } => None,

        // 二元运算
        ConstExpr::BinOp { op, left, right } => {
            let l = translate_expr(left)?;
            let r = translate_expr(right)?;
            let smt_op = match op {
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Eq => "=",
                BinOp::Ne => "distinct",
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "div",
                BinOp::Mod => "mod",
                BinOp::And => "and",
                BinOp::Or => "or",
                // 位运算暂不支持 SMT 翻译
                _ => return None,
            };
            Some(SMTExpr::App(smt_op.into(), vec![l, r]))
        }

        // 一元运算
        ConstExpr::UnOp { op, expr: inner } => {
            let i = translate_expr(inner)?;
            let smt_op = match op {
                UnOp::Not => "not",
                UnOp::Neg => "-",
                UnOp::Pos => return Some(i), // 正号直接透传
                _ => return None,
            };
            Some(SMTExpr::App(smt_op.into(), vec![i]))
        }

        // If/Range/Call 在阶段 2 不做 SMT 翻译——走 Evaluator 直接求值路径
        _ => None,
    }
}

/// 从约束表达式和绑定推断变量 SMT 排序
///
/// Int 字面量相关变量 → SMTSort::Int
/// Bool 字面量相关变量 → SMTSort::Bool
pub fn infer_var_sorts(
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
) -> HashMap<String, SMTSort> {
    let mut sorts = HashMap::new();
    infer_sorts_in_expr(constraint, &mut sorts);

    // 从 bindings 补充类型信息
    for (name, value) in bindings {
        if !sorts.contains_key(name) {
            let sort = match value {
                ConstValue::Int(_) => SMTSort::Int,
                ConstValue::Bool(_) => SMTSort::Bool,
                _ => SMTSort::Int, // 默认 Int
            };
            sorts.insert(name.clone(), sort);
        }
    }

    sorts
}

fn infer_sorts_in_expr(
    expr: &ConstExpr,
    sorts: &mut HashMap<String, SMTSort>,
) {
    match expr {
        ConstExpr::NamedVar(name) => {
            sorts.entry(name.clone()).or_insert(SMTSort::Int);
        }
        ConstExpr::BinOp { left, right, .. } => {
            infer_sorts_in_expr(left, sorts);
            infer_sorts_in_expr(right, sorts);
        }
        ConstExpr::UnOp { expr: inner, .. } => {
            infer_sorts_in_expr(inner, sorts);
        }
        _ => {}
    }
}
