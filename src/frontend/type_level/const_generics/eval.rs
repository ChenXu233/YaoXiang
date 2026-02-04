//! RFC-011 Const泛型求值
//!
//! 实现Const泛型的编译期求值。
//!
//! 支持：
//! - Const表达式求值
//! - Const函数调用
//! - Const参数替换

use crate::frontend::core::type_system::ConstValue;
use super::ConstGenericError;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::cmp::{PartialEq, Eq};

/// Const表达式
#[derive(Debug, Clone)]
pub enum ConstExpr {
    /// 整数字面量
    Int(i128),

    /// 浮点数字面量 (使用 f32，与 ConstValue 保持一致)
    Float(f32),

    /// 布尔字面量
    Bool(bool),

    /// 变量引用
    Var(String),

    /// 二元运算
    BinOp {
        op: ConstBinOp,
        lhs: Box<ConstExpr>,
        rhs: Box<ConstExpr>,
    },

    /// 一元运算
    UnOp { op: ConstUnOp, expr: Box<ConstExpr> },

    /// 函数调用
    Call { name: String, args: Vec<ConstExpr> },

    /// 条件表达式
    If {
        condition: Box<ConstExpr>,
        true_branch: Box<ConstExpr>,
        false_branch: Box<ConstExpr>,
    },
}

impl PartialEq for ConstExpr {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        match (self, other) {
            (ConstExpr::Int(a), ConstExpr::Int(b)) => a == b,
            (ConstExpr::Float(a), ConstExpr::Float(b)) => a.to_bits() == b.to_bits(),
            (ConstExpr::Bool(a), ConstExpr::Bool(b)) => a == b,
            (ConstExpr::Var(a), ConstExpr::Var(b)) => a == b,
            (
                ConstExpr::BinOp {
                    op: o1,
                    lhs: l1,
                    rhs: r1,
                },
                ConstExpr::BinOp {
                    op: o2,
                    lhs: l2,
                    rhs: r2,
                },
            ) => o1 == o2 && l1 == l2 && r1 == r2,
            (ConstExpr::UnOp { op: o1, expr: e1 }, ConstExpr::UnOp { op: o2, expr: e2 }) => {
                o1 == o2 && e1 == e2
            }
            (ConstExpr::Call { name: n1, args: a1 }, ConstExpr::Call { name: n2, args: a2 }) => {
                n1 == n2 && a1 == a2
            }
            (
                ConstExpr::If {
                    condition: c1,
                    true_branch: t1,
                    false_branch: f1,
                },
                ConstExpr::If {
                    condition: c2,
                    true_branch: t2,
                    false_branch: f2,
                },
            ) => c1 == c2 && t1 == t2 && f1 == f2,
            _ => false,
        }
    }
}

impl Eq for ConstExpr {}

impl Hash for ConstExpr {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        match self {
            ConstExpr::Int(n) => {
                0u8.hash(state);
                n.hash(state);
            }
            ConstExpr::Float(f) => {
                1u8.hash(state);
                f.to_bits().hash(state);
            }
            ConstExpr::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            ConstExpr::Var(s) => {
                3u8.hash(state);
                s.hash(state);
            }
            ConstExpr::BinOp { op, lhs, rhs } => {
                4u8.hash(state);
                op.hash(state);
                lhs.hash(state);
                rhs.hash(state);
            }
            ConstExpr::UnOp { op, expr } => {
                5u8.hash(state);
                op.hash(state);
                expr.hash(state);
            }
            ConstExpr::Call { name, args } => {
                6u8.hash(state);
                name.hash(state);
                args.hash(state);
            }
            ConstExpr::If {
                condition,
                true_branch,
                false_branch,
            } => {
                7u8.hash(state);
                condition.hash(state);
                true_branch.hash(state);
                false_branch.hash(state);
            }
        }
    }
}

/// Const二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstBinOp {
    /// 加法
    Add,

    /// 减法
    Sub,

    /// 乘法
    Mul,

    /// 除法
    Div,

    /// 取模
    Mod,

    /// 等于
    Eq,

    /// 不等于
    Neq,

    /// 小于
    Lt,

    /// 大于
    Gt,

    /// 小于等于
    Lte,

    /// 大于等于
    Gte,

    /// 位与
    BitAnd,

    /// 位或
    BitOr,

    /// 位异或
    BitXor,

    /// 左移
    Shl,

    /// 右移
    Shr,
}

/// Const一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstUnOp {
    /// 负号
    Neg,

    /// 逻辑非
    Not,
}

/// Const泛型求值器
#[derive(Debug, Clone, Default)]
pub struct ConstGenericEval {
    /// 函数定义
    functions: HashMap<String, ConstFunction>,

    /// 变量绑定
    bindings: HashMap<String, ConstValue>,
}

impl ConstGenericEval {
    /// 创建新的求值器
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            bindings: HashMap::new(),
        }
    }

    /// 注册Const函数
    pub fn register_function(
        &mut self,
        name: String,
        func: ConstFunction,
    ) {
        self.functions.insert(name, func);
    }

    /// 绑定变量
    pub fn bind_var(
        &mut self,
        name: String,
        value: ConstValue,
    ) {
        self.bindings.insert(name, value);
    }

    /// 求值Const表达式
    pub fn eval(
        &self,
        expr: &ConstExpr,
    ) -> Result<ConstValue, ConstGenericError> {
        match expr {
            ConstExpr::Int(n) => Ok(ConstValue::Int(*n)),
            ConstExpr::Float(f) => Ok(ConstValue::Float(*f)),
            ConstExpr::Bool(b) => Ok(ConstValue::Bool(*b)),
            ConstExpr::Var(name) => self.bindings.get(name).cloned().ok_or_else(|| {
                ConstGenericError::EvalFailed(format!("Undefined variable: {}", name))
            }),
            ConstExpr::BinOp { op, lhs, rhs } => self.eval_binop(op, lhs, rhs),
            ConstExpr::UnOp { op, expr } => self.eval_unop(op, expr),
            ConstExpr::Call { name, args } => self.eval_call(name, args),
            ConstExpr::If {
                condition,
                true_branch,
                false_branch,
            } => self.eval_if(condition, true_branch, false_branch),
        }
    }

    /// 求值二元运算
    fn eval_binop(
        &self,
        op: &ConstBinOp,
        lhs: &ConstExpr,
        rhs: &ConstExpr,
    ) -> Result<ConstValue, ConstGenericError> {
        let left = self.eval(lhs)?;
        let right = self.eval(rhs)?;

        match (op, &left, &right) {
            // 整数运算
            (ConstBinOp::Add, ConstValue::Int(a), ConstValue::Int(b)) => {
                Ok(ConstValue::Int(a.saturating_add(*b)))
            }
            (ConstBinOp::Sub, ConstValue::Int(a), ConstValue::Int(b)) => {
                Ok(ConstValue::Int(a.saturating_sub(*b)))
            }
            (ConstBinOp::Mul, ConstValue::Int(a), ConstValue::Int(b)) => {
                Ok(ConstValue::Int(a.saturating_mul(*b)))
            }
            (ConstBinOp::Div, ConstValue::Int(a), ConstValue::Int(b)) => {
                if *b == 0 {
                    Err(ConstGenericError::EvalFailed(
                        "Division by zero".to_string(),
                    ))
                } else {
                    Ok(ConstValue::Int(a.saturating_div(*b)))
                }
            }
            (ConstBinOp::Mod, ConstValue::Int(a), ConstValue::Int(b)) => {
                if *b == 0 {
                    Err(ConstGenericError::EvalFailed("Modulo by zero".to_string()))
                } else {
                    Ok(ConstValue::Int(a % b))
                }
            }

            // 浮点数运算（仅限加法和乘法，保持编译期可确定性）
            (ConstBinOp::Add, ConstValue::Float(a), ConstValue::Float(b)) => {
                Ok(ConstValue::Float(*a + *b))
            }
            (ConstBinOp::Mul, ConstValue::Float(a), ConstValue::Float(b)) => {
                Ok(ConstValue::Float(*a * *b))
            }

            // 位运算（仅限整数）
            (ConstBinOp::BitAnd, ConstValue::Int(a), ConstValue::Int(b)) => {
                Ok(ConstValue::Int(a & b))
            }
            (ConstBinOp::BitOr, ConstValue::Int(a), ConstValue::Int(b)) => {
                Ok(ConstValue::Int(a | b))
            }
            (ConstBinOp::BitXor, ConstValue::Int(a), ConstValue::Int(b)) => {
                Ok(ConstValue::Int(a ^ b))
            }
            (ConstBinOp::Shl, ConstValue::Int(a), ConstValue::Int(b)) => {
                // 检查移位是否超出范围
                if *b < 0 || *b >= 128 {
                    Err(ConstGenericError::EvalFailed(
                        "Shift amount out of range".to_string(),
                    ))
                } else {
                    Ok(ConstValue::Int(a.checked_shl(*b as u32).unwrap_or(0)))
                }
            }
            (ConstBinOp::Shr, ConstValue::Int(a), ConstValue::Int(b)) => {
                if *b < 0 || *b >= 128 {
                    Err(ConstGenericError::EvalFailed(
                        "Shift amount out of range".to_string(),
                    ))
                } else {
                    Ok(ConstValue::Int(a.checked_shr(*b as u32).unwrap_or(0)))
                }
            }

            // 比较运算（整数）
            (ConstBinOp::Eq, _, _) => Ok(ConstValue::Bool(left == right)),
            (ConstBinOp::Neq, _, _) => Ok(ConstValue::Bool(left != right)),
            (ConstBinOp::Lt, ConstValue::Int(a), ConstValue::Int(b)) => Ok(ConstValue::Bool(a < b)),
            (ConstBinOp::Gt, ConstValue::Int(a), ConstValue::Int(b)) => Ok(ConstValue::Bool(a > b)),
            (ConstBinOp::Lte, ConstValue::Int(a), ConstValue::Int(b)) => {
                Ok(ConstValue::Bool(a <= b))
            }
            (ConstBinOp::Gte, ConstValue::Int(a), ConstValue::Int(b)) => {
                Ok(ConstValue::Bool(a >= b))
            }
            // 比较运算（浮点数）
            (ConstBinOp::Lt, ConstValue::Float(a), ConstValue::Float(b)) => {
                Ok(ConstValue::Bool(a < b))
            }
            (ConstBinOp::Gt, ConstValue::Float(a), ConstValue::Float(b)) => {
                Ok(ConstValue::Bool(a > b))
            }
            (ConstBinOp::Lte, ConstValue::Float(a), ConstValue::Float(b)) => {
                Ok(ConstValue::Bool(a <= b))
            }
            (ConstBinOp::Gte, ConstValue::Float(a), ConstValue::Float(b)) => {
                Ok(ConstValue::Bool(a >= b))
            }

            _ => Err(ConstGenericError::EvalFailed(format!(
                "Unsupported operation: {:?} for {:?} and {:?}",
                op, left, right
            ))),
        }
    }

    /// 求值一元运算
    fn eval_unop(
        &self,
        op: &ConstUnOp,
        expr: &ConstExpr,
    ) -> Result<ConstValue, ConstGenericError> {
        let value = self.eval(expr)?;

        match (op, &value) {
            (ConstUnOp::Neg, ConstValue::Int(n)) => Ok(ConstValue::Int(-*n)),
            (ConstUnOp::Not, ConstValue::Bool(b)) => Ok(ConstValue::Bool(!*b)),
            _ => Err(ConstGenericError::EvalFailed(format!(
                "Unsupported unary operation: {:?} for {:?}",
                op, value
            ))),
        }
    }

    /// 求值函数调用
    fn eval_call(
        &self,
        name: &str,
        args: &[ConstExpr],
    ) -> Result<ConstValue, ConstGenericError> {
        // 查找内置函数
        if let Some(result) = self.eval_builtin(name, args) {
            return result;
        }

        // 查找用户定义函数
        if let Some(func) = self.functions.get(name) {
            return self.eval_function_call(func, args);
        }

        Err(ConstGenericError::EvalFailed(format!(
            "Undefined function: {}",
            name
        )))
    }

    /// 求值内置函数
    fn eval_builtin(
        &self,
        name: &str,
        args: &[ConstExpr],
    ) -> Option<Result<ConstValue, ConstGenericError>> {
        match name {
            "abs" => {
                if let Ok(ConstValue::Int(n)) = self.eval(&args[0]) {
                    Some(Ok(ConstValue::Int(n.abs())))
                } else {
                    None
                }
            }
            "min" => match (self.eval(&args[0]), self.eval(&args[1])) {
                (Ok(ConstValue::Int(x)), Ok(ConstValue::Int(y))) => {
                    Some(Ok(ConstValue::Int(x.min(y))))
                }
                _ => None,
            },
            "max" => match (self.eval(&args[0]), self.eval(&args[1])) {
                (Ok(ConstValue::Int(x)), Ok(ConstValue::Int(y))) => {
                    Some(Ok(ConstValue::Int(x.max(y))))
                }
                _ => None,
            },
            "sizeof" => {
                // 期望参数是类型名称字符串
                let type_name = match args.first() {
                    Some(ConstExpr::Var(name)) => name.as_str(),
                    _ => {
                        return Some(Err(ConstGenericError::EvalFailed(
                            "sizeof expects a type name".to_string(),
                        )))
                    }
                };

                // 基础类型大小表
                let size = match type_name {
                    "Void" => 0,
                    "Bool" => 1,
                    "Char" => 4,
                    "Int" | "Uint" | "Float" | "String" => 8,
                    _ => {
                        return Some(Err(ConstGenericError::EvalFailed(format!(
                            "Unknown type: {}",
                            type_name
                        ))))
                    }
                };
                Some(Ok(ConstValue::Int(size as i128)))
            }
            // RFC-011: compile_time - 检查表达式是否可以在编译期求值
            "compile_time" => {
                // compile_time() 总是返回 true（表示可以在编译期求值）
                Some(Ok(ConstValue::Bool(true)))
            }
            _ => None,
        }
    }

    /// 求值用户函数调用
    fn eval_function_call(
        &self,
        func: &ConstFunction,
        args: &[ConstExpr],
    ) -> Result<ConstValue, ConstGenericError> {
        if args.len() != func.params.len() {
            return Err(ConstGenericError::EvalFailed(format!(
                "Function {} expects {} arguments, got {}",
                func.name,
                func.params.len(),
                args.len()
            )));
        }

        // 创建新的作用域
        let mut eval = self.clone();
        for (param, arg) in func.params.iter().zip(args.iter()) {
            let value = eval.eval(arg)?;
            eval.bind_var(param.clone(), value);
        }

        eval.eval(&func.body)
    }

    /// 求值条件表达式
    fn eval_if(
        &self,
        condition: &ConstExpr,
        true_branch: &ConstExpr,
        false_branch: &ConstExpr,
    ) -> Result<ConstValue, ConstGenericError> {
        match self.eval(condition)? {
            ConstValue::Bool(true) => self.eval(true_branch),
            ConstValue::Bool(false) => self.eval(false_branch),
            _ => Err(ConstGenericError::EvalFailed(
                "Condition must be a boolean".to_string(),
            )),
        }
    }
}

/// Const函数定义
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstFunction {
    /// 函数名称
    pub name: String,

    /// 参数列表
    pub params: Vec<String>,

    /// 函数体
    pub body: ConstExpr,
}

impl ConstFunction {
    /// 创建新的Const函数
    pub fn new(
        name: String,
        params: Vec<String>,
        body: ConstExpr,
    ) -> Self {
        Self { name, params, body }
    }
}

/// 预定义的Const函数
pub mod predefined {
    use super::*;

    /// Factorial 函数
    pub fn factorial() -> ConstFunction {
        ConstFunction::new(
            "factorial".to_string(),
            vec!["n".to_string()],
            ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Lte,
                    lhs: Box::new(ConstExpr::Var("n".to_string())),
                    rhs: Box::new(ConstExpr::Int(1)),
                }),
                true_branch: Box::new(ConstExpr::Int(1)),
                false_branch: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Mul,
                    lhs: Box::new(ConstExpr::Var("n".to_string())),
                    rhs: Box::new(ConstExpr::Call {
                        name: "factorial".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: ConstBinOp::Sub,
                            lhs: Box::new(ConstExpr::Var("n".to_string())),
                            rhs: Box::new(ConstExpr::Int(1)),
                        }],
                    }),
                }),
            },
        )
    }

    /// Fibonacci 函数
    pub fn fibonacci() -> ConstFunction {
        ConstFunction::new(
            "fibonacci".to_string(),
            vec!["n".to_string()],
            ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Lte,
                    lhs: Box::new(ConstExpr::Var("n".to_string())),
                    rhs: Box::new(ConstExpr::Int(1)),
                }),
                true_branch: Box::new(ConstExpr::Var("n".to_string())),
                false_branch: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Add,
                    lhs: Box::new(ConstExpr::Call {
                        name: "fibonacci".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: ConstBinOp::Sub,
                            lhs: Box::new(ConstExpr::Var("n".to_string())),
                            rhs: Box::new(ConstExpr::Int(1)),
                        }],
                    }),
                    rhs: Box::new(ConstExpr::Call {
                        name: "fibonacci".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: ConstBinOp::Sub,
                            lhs: Box::new(ConstExpr::Var("n".to_string())),
                            rhs: Box::new(ConstExpr::Int(2)),
                        }],
                    }),
                }),
            },
        )
    }
}
