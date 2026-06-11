//! SMT-LIB 2.6 中间表示
//!
//! ConstExpr（用户写了什么）和 Z3 API（求解器需要什么）之间的纯数据结构层。
//! Display 输出标准 SMT-LIB 2.6 文本用于调试。

use std::fmt;

/// SMT 排序
#[derive(Debug, Clone, PartialEq)]
pub enum SMTSort {
    Bool,
    Int,
    Real,
}

impl fmt::Display for SMTSort {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            SMTSort::Bool => write!(f, "Bool"),
            SMTSort::Int => write!(f, "Int"),
            SMTSort::Real => write!(f, "Real"),
        }
    }
}

/// SMT-LIB 2.6 表达式
#[derive(Debug, Clone, PartialEq)]
pub enum SMTExpr {
    /// 原子：变量名、数值字面量、布尔常量
    Atom(String),
    /// 函数应用：(op args...)
    App(String, Vec<SMTExpr>),
    /// 全称量化：(forall ((x Sort)) body)
    Forall {
        vars: Vec<(String, SMTSort)>,
        body: Box<SMTExpr>,
    },
    /// 存在量化：(exists ((x Sort)) body)
    Exists {
        vars: Vec<(String, SMTSort)>,
        body: Box<SMTExpr>,
    },
}

impl fmt::Display for SMTExpr {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            SMTExpr::Atom(s) => write!(f, "{}", s),
            SMTExpr::App(op, args) => {
                write!(f, "({}", op)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            }
            SMTExpr::Forall { vars, body } => {
                write!(f, "(forall (")?;
                for (i, (name, sort)) in vars.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "({} {})", name, sort)?;
                }
                write!(f, ") {})", body)
            }
            SMTExpr::Exists { vars, body } => {
                write!(f, "(exists (")?;
                for (i, (name, sort)) in vars.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "({} {})", name, sort)?;
                }
                write!(f, ") {})", body)
            }
        }
    }
}

/// 一条 SMT 命令
#[derive(Debug, Clone, PartialEq)]
pub enum SMTCommand {
    DeclareConst(String, SMTSort),
    Assert(SMTExpr),
    CheckSat,
    GetModel,
}

impl fmt::Display for SMTCommand {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            SMTCommand::DeclareConst(name, sort) => {
                write!(f, "(declare-const {} {})", name, sort)
            }
            SMTCommand::Assert(expr) => write!(f, "(assert {})", expr),
            SMTCommand::CheckSat => write!(f, "(check-sat)"),
            SMTCommand::GetModel => write!(f, "(get-model)"),
        }
    }
}

/// Z3 求解结果
#[derive(Debug, Clone)]
pub enum SMTResult {
    /// unsat — 目标在所有假设下成立
    Unsat,
    /// sat — 存在反例
    Sat { model: SMTModel },
    /// unknown — 超时或超出求解能力
    Unknown { reason: String },
}

/// Z3 返回的反例模型
#[derive(Debug, Clone)]
pub struct SMTModel {
    /// (变量名, 值) 对
    pub assignments: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_atom_int() {
        assert_eq!(SMTExpr::Atom("42".into()).to_string(), "42");
    }

    #[test]
    fn test_display_atom_var() {
        assert_eq!(SMTExpr::Atom("x".into()).to_string(), "x");
    }

    #[test]
    fn test_display_app_gt() {
        let expr = SMTExpr::App(
            ">".into(),
            vec![SMTExpr::Atom("x".into()), SMTExpr::Atom("0".into())],
        );
        assert_eq!(expr.to_string(), "(> x 0)");
    }

    #[test]
    fn test_display_app_and() {
        let expr = SMTExpr::App(
            "and".into(),
            vec![
                SMTExpr::App(
                    ">".into(),
                    vec![SMTExpr::Atom("x".into()), SMTExpr::Atom("0".into())],
                ),
                SMTExpr::App(
                    "<".into(),
                    vec![SMTExpr::Atom("x".into()), SMTExpr::Atom("10".into())],
                ),
            ],
        );
        assert_eq!(expr.to_string(), "(and (> x 0) (< x 10))");
    }

    #[test]
    fn test_display_command_assert() {
        let cmd = SMTCommand::Assert(SMTExpr::App(
            ">".into(),
            vec![SMTExpr::Atom("y".into()), SMTExpr::Atom("0".into())],
        ));
        assert_eq!(cmd.to_string(), "(assert (> y 0))");
    }

    #[test]
    fn test_display_forall() {
        let expr = SMTExpr::Forall {
            vars: vec![("i".into(), SMTSort::Int)],
            body: Box::new(SMTExpr::App(
                ">=".into(),
                vec![SMTExpr::Atom("i".into()), SMTExpr::Atom("0".into())],
            )),
        };
        assert_eq!(expr.to_string(), "(forall ((i Int)) (>= i 0))");
    }
}
