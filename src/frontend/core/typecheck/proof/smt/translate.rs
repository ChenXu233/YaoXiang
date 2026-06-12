//! ConstExpr → SMTCommand 翻译器
//!
//! 纯函数，无外部依赖。将编译期约束表达式翻译为 SMT-LIB 命令序列。
//! 假设栈的每一项成为背景断言，目标约束取反后送 Z3 做 check-sat：
//!   unsat = 目标在假设下成立 → Proved
//!   sat   = 存在反例 → Disproved

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue, UnOp};
use super::ast::{SMTCommand, SMTExpr, SMTSort};

/// 翻译编译期约束为 SMT 命令序列
///
/// # 参数
/// - `constraint`: 待验证的约束表达式
/// - `assumptions`: 当前程序点的假设栈
/// - `var_sorts`: 变量名 → SMT 排序的映射
///
/// # 返回
/// 完整的 SMT 命令序列（declare-const + assert + 目标取反 + check-sat + get-model）
pub fn translate_constraint(
    constraint: &ConstExpr,
    assumptions: &[ConstExpr],
    var_sorts: &HashMap<String, SMTSort>,
) -> Vec<SMTCommand> {
    let mut commands = Vec::new();

    // 1. 声明所有变量
    for (var, sort) in var_sorts {
        commands.push(SMTCommand::DeclareConst(var.clone(), sort.clone()));
    }

    // 2. 背景断言（假设栈）
    for assumption in assumptions {
        commands.push(SMTCommand::Assert(translate_expr(assumption)));
    }

    // 3. 目标取反：(assert (not constraint))
    //    Z3 返回 unsat = 目标在所有假设下成立
    let goal = translate_expr(constraint);
    let not_goal = SMTExpr::App("not".into(), vec![goal]);
    commands.push(SMTCommand::Assert(not_goal));

    // 4. check-sat + get-model
    commands.push(SMTCommand::CheckSat);
    commands.push(SMTCommand::GetModel);

    commands
}

/// ConstExpr → SMTExpr 翻译
fn translate_expr(expr: &ConstExpr) -> SMTExpr {
    match expr {
        // 字面量
        ConstExpr::Lit(value) => match value {
            ConstValue::Int(n) => SMTExpr::Atom(n.to_string()),
            ConstValue::Bool(true) => SMTExpr::Atom("true".into()),
            ConstValue::Bool(false) => SMTExpr::Atom("false".into()),
            _ => SMTExpr::Atom(format!("{:?}", value)),
        },

        // 变量引用
        ConstExpr::NamedVar(name) => SMTExpr::Atom(name.clone()),

        // Const 变量引用
        ConstExpr::Var(var) => SMTExpr::Atom(var.to_string()),

        // 二元运算
        ConstExpr::BinOp { op, left, right } => {
            let l = translate_expr(left);
            let r = translate_expr(right);
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
                _ => {
                    return SMTExpr::Atom(format!("{:?}", expr));
                }
            };
            SMTExpr::App(smt_op.into(), vec![l, r])
        }

        // 一元运算
        ConstExpr::UnOp { op, expr: inner } => {
            let i = translate_expr(inner);
            let smt_op = match op {
                UnOp::Not => "not",
                UnOp::Neg => "-",
                UnOp::Pos => return i, // 正号直接透传
                _ => {
                    return SMTExpr::Atom(format!("{:?}", expr));
                }
            };
            SMTExpr::App(smt_op.into(), vec![i])
        }

        // If/Range/Call 在阶段 2 不做 SMT 翻译——走 Evaluator 直接求值路径
        _ => SMTExpr::Atom(format!("{:?}", expr)),
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

fn infer_sorts_in_expr(expr: &ConstExpr, sorts: &mut HashMap<String, SMTSort>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    fn make_gt(var: &str, n: i128) -> ConstExpr {
        ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar(var.into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
        }
    }

    #[test]
    fn test_translate_simple_constraint() {
        let constraint = make_gt("x", 0);
        let assumptions = vec![];
        let mut var_sorts = HashMap::new();
        var_sorts.insert("x".into(), SMTSort::Int);

        let commands = translate_constraint(&constraint, &assumptions, &var_sorts);

        // declare-const + assert(not goal) + check-sat + get-model
        assert!(commands.len() >= 3);
        match &commands[0] {
            SMTCommand::DeclareConst(name, sort) => {
                assert_eq!(name, "x");
                assert_eq!(*sort, SMTSort::Int);
            }
            _ => panic!("Expected DeclareConst"),
        }
    }

    #[test]
    fn test_translate_with_assumptions() {
        let constraint = make_gt("y", 0);
        let assumptions = vec![make_gt("y", 5)];
        let mut var_sorts = HashMap::new();
        var_sorts.insert("y".into(), SMTSort::Int);

        let commands = translate_constraint(&constraint, &assumptions, &var_sorts);

        // declare-const + 1 assert(assumption) + 1 assert(not goal) + check-sat + get-model
        assert_eq!(commands.len(), 5);
    }

    #[test]
    fn test_translate_expr_gt() {
        let expr = make_gt("x", 10);
        let result = translate_expr(&expr);
        assert_eq!(result.to_string(), "(> x 10)");
    }

    #[test]
    fn test_translate_expr_and() {
        let expr = ConstExpr::BinOp {
            op: BinOp::And,
            left: Box::new(make_gt("x", 0)),
            right: Box::new(ConstExpr::BinOp {
                op: BinOp::Lt,
                left: Box::new(ConstExpr::NamedVar("x".into())),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
            }),
        };
        let result = translate_expr(&expr);
        assert!(result.to_string().contains("and"));
        assert!(result.to_string().contains("(> x 0)"));
        assert!(result.to_string().contains("(< x 10)"));
    }

    #[test]
    fn test_infer_var_sorts() {
        let constraint = ConstExpr::BinOp {
            op: BinOp::And,
            left: Box::new(make_gt("x", 0)),
            right: Box::new(make_gt("y", 100)),
        };
        let bindings = HashMap::new();
        let sorts = infer_var_sorts(&constraint, &bindings);
        assert!(sorts.contains_key("x"));
        assert!(sorts.contains_key("y"));
    }

    #[test]
    fn test_translate_expr_not() {
        let expr = ConstExpr::UnOp {
            op: UnOp::Not,
            expr: Box::new(make_gt("z", 0)),
        };
        let result = translate_expr(&expr);
        assert_eq!(result.to_string(), "(not (> z 0))");
    }
}
