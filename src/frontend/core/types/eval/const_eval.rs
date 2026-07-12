//! RFC-011 Const泛型支持
//!
//! 提供Const泛型的编译期求值和尺寸计算能力。
//!
//! # 示例
//! ```yaoxiang
//! type Array[T, N: Int] = {
//!     data: T[N],
//!     length: N,
//! }
//!
//! const SIZE: Int = factorial(5)  # 120
//!
//! type IntArray[10] = Array[Int, 10]
//! ```

// 重新导出主要类型
use crate::frontend::core::types::ConstValue;
use crate::frontend::core::parser::ast::{Expr, BinOp as AstBinOp, UnOp as AstUnOp};
use crate::frontend::core::lexer::tokens::Literal;
use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::codes::ErrorCodeDefinition;

/// Const泛型求值结果
#[derive(Debug, Clone, PartialEq)]
pub struct ConstGenericResult {
    /// 求值结果
    pub value: ConstValue,

    /// 是否是编译期常量
    pub is_const: bool,
}

impl ConstGenericResult {
    /// 创建新的结果
    pub fn new(
        value: ConstValue,
        is_const: bool,
    ) -> Self {
        Self { value, is_const }
    }

    /// 检查是否是常量
    pub fn is_const(&self) -> bool {
        self.is_const
    }

    /// 获取整数值
    pub fn as_int(&self) -> Option<i128> {
        match &self.value {
            ConstValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// 获取布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            ConstValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

// ====================================================================
// ConstGenericEval
// ====================================================================
/// RFC-011 Const泛型求值
///
/// 实现Const泛型的编译期求值。
///
/// 支持：
/// - Const表达式求值
/// - Const函数调用
/// - Const参数替换
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

/// 将 AST Expr 转换为 ConstExpr
///
/// 只支持常量约束表达式：字面量、变量引用、二元运算、一元运算。
/// 其他表达式（函数调用、if、match 等）返回 None。
pub fn convert_expr_to_const_expr(expr: &Expr) -> Option<ConstExpr> {
    match expr {
        Expr::Lit(lit, _) => match lit {
            Literal::Int(n) => Some(ConstExpr::Int(*n)),
            Literal::Bool(b) => Some(ConstExpr::Bool(*b)),
            Literal::Float(f) => Some(ConstExpr::Float(*f as f32)),
            _ => None,
        },
        Expr::Var(name, _) => Some(ConstExpr::Var(name.clone())),
        Expr::BinOp {
            op, left, right, ..
        } => {
            let const_op = convert_binop(op)?;
            let lhs = convert_expr_to_const_expr(left)?;
            let rhs = convert_expr_to_const_expr(right)?;
            Some(ConstExpr::BinOp {
                op: const_op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
        }
        Expr::UnOp { op, expr, .. } => {
            let const_op = convert_unop(op)?;
            let inner = convert_expr_to_const_expr(expr)?;
            Some(ConstExpr::UnOp {
                op: const_op,
                expr: Box::new(inner),
            })
        }
        _ => None,
    }
}

fn convert_binop(op: &AstBinOp) -> Option<ConstBinOp> {
    match op {
        AstBinOp::Add => Some(ConstBinOp::Add),
        AstBinOp::Sub => Some(ConstBinOp::Sub),
        AstBinOp::Mul => Some(ConstBinOp::Mul),
        AstBinOp::Div => Some(ConstBinOp::Div),
        AstBinOp::Mod => Some(ConstBinOp::Mod),
        AstBinOp::Eq => Some(ConstBinOp::Eq),
        AstBinOp::Neq => Some(ConstBinOp::Neq),
        AstBinOp::Lt => Some(ConstBinOp::Lt),
        AstBinOp::Le => Some(ConstBinOp::Lte),
        AstBinOp::Gt => Some(ConstBinOp::Gt),
        AstBinOp::Ge => Some(ConstBinOp::Gte),
        _ => None,
    }
}

fn convert_unop(op: &AstUnOp) -> Option<ConstUnOp> {
    match op {
        AstUnOp::Neg => Some(ConstUnOp::Neg),
        AstUnOp::Not => Some(ConstUnOp::Not),
        _ => None,
    }
}

/// 默认最大递归深度
const DEFAULT_MAX_CONST_EVAL_DEPTH: usize = 256;

/// Const泛型求值器
#[derive(Debug, Clone)]
pub struct ConstGenericEval {
    /// 函数定义
    functions: HashMap<String, ConstFunction>,

    /// 变量绑定
    bindings: HashMap<String, ConstValue>,

    /// 当前递归深度
    current_depth: usize,

    /// 最大递归深度
    max_depth: usize,
}

impl Default for ConstGenericEval {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstGenericEval {
    /// 创建新的求值器
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            bindings: HashMap::new(),
            current_depth: 0,
            max_depth: DEFAULT_MAX_CONST_EVAL_DEPTH,
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

    /// 设置最大递归深度
    pub fn set_max_depth(
        &mut self,
        max_depth: usize,
    ) {
        self.max_depth = max_depth;
    }

    /// 求值Const表达式
    pub fn eval(
        &self,
        expr: &ConstExpr,
    ) -> Result<ConstValue, Diagnostic> {
        match expr {
            ConstExpr::Int(n) => Ok(ConstValue::Int(*n)),
            ConstExpr::Float(f) => Ok(ConstValue::Float(*f)),
            ConstExpr::Bool(b) => Ok(ConstValue::Bool(*b)),
            ConstExpr::Var(name) => self.bindings.get(name).cloned().ok_or_else(|| {
                ErrorCodeDefinition::const_eval_failed(&format!("Undefined variable: {}", name))
                    .build()
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
    ) -> Result<ConstValue, Diagnostic> {
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
                    Err(ErrorCodeDefinition::const_division_by_zero().build())
                } else {
                    Ok(ConstValue::Int(a.saturating_div(*b)))
                }
            }
            (ConstBinOp::Mod, ConstValue::Int(a), ConstValue::Int(b)) => {
                if *b == 0 {
                    Err(ErrorCodeDefinition::const_division_by_zero().build())
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
                    Err(ErrorCodeDefinition::const_overflow().build())
                } else {
                    Ok(ConstValue::Int(a.checked_shl(*b as u32).unwrap_or(0)))
                }
            }
            (ConstBinOp::Shr, ConstValue::Int(a), ConstValue::Int(b)) => {
                if *b < 0 || *b >= 128 {
                    Err(ErrorCodeDefinition::const_overflow().build())
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

            _ => Err(ErrorCodeDefinition::const_eval_failed(&format!(
                "Unsupported operation: {:?} for {:?} and {:?}",
                op, left, right
            ))
            .build()),
        }
    }

    /// 求值一元运算
    fn eval_unop(
        &self,
        op: &ConstUnOp,
        expr: &ConstExpr,
    ) -> Result<ConstValue, Diagnostic> {
        let value = self.eval(expr)?;

        match (op, &value) {
            (ConstUnOp::Neg, ConstValue::Int(n)) => Ok(ConstValue::Int(-*n)),
            (ConstUnOp::Not, ConstValue::Bool(b)) => Ok(ConstValue::Bool(!*b)),
            _ => Err(ErrorCodeDefinition::const_eval_failed(&format!(
                "Unsupported unary operation: {:?} for {:?}",
                op, value
            ))
            .build()),
        }
    }

    /// 求值函数调用
    fn eval_call(
        &self,
        name: &str,
        args: &[ConstExpr],
    ) -> Result<ConstValue, Diagnostic> {
        // 查找内置函数
        if let Some(result) = self.eval_builtin(name, args) {
            return result;
        }

        // 查找用户定义函数
        if let Some(func) = self.functions.get(name) {
            return self.eval_function_call(func, args);
        }

        Err(
            ErrorCodeDefinition::const_eval_failed(&format!("Undefined function: {}", name))
                .build(),
        )
    }

    /// 求值内置函数
    fn eval_builtin(
        &self,
        name: &str,
        args: &[ConstExpr],
    ) -> Option<Result<ConstValue, Diagnostic>> {
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
                        return Some(Err(ErrorCodeDefinition::const_eval_failed(
                            "sizeof expects a type name",
                        )
                        .build()))
                    }
                };

                // 基础类型大小表
                let size = match type_name {
                    "Void" => 0,
                    "Never" => 0,
                    "Bool" => 1,
                    "Char" => 4,
                    "Int" | "Uint" | "Float" | "String" => 8,
                    _ => {
                        return Some(Err(ErrorCodeDefinition::const_eval_failed(&format!(
                            "Unknown type: {}",
                            type_name
                        ))
                        .build()))
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
    ) -> Result<ConstValue, Diagnostic> {
        if args.len() != func.params.len() {
            return Err(ErrorCodeDefinition::const_eval_failed(&format!(
                "Function {} expects {} arguments, got {}",
                func.name,
                func.params.len(),
                args.len()
            ))
            .build());
        }

        if self.current_depth >= self.max_depth {
            return Err(ErrorCodeDefinition::const_recursion_too_deep(self.max_depth).build());
        }

        // 创建新的作用域，递归深度 +1
        let mut eval = self.clone();
        eval.current_depth += 1;
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
    ) -> Result<ConstValue, Diagnostic> {
        match self.eval(condition)? {
            ConstValue::Bool(true) => self.eval(true_branch),
            ConstValue::Bool(false) => self.eval(false_branch),
            _ => Err(ErrorCodeDefinition::const_eval_failed("Condition must be a boolean").build()),
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
pub mod functions {
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

// ====================================================================
// GenericSize
// ====================================================================
/// RFC-011 泛型尺寸计算
///
/// 实现泛型类型的尺寸计算，用于Const泛型和数组类型。
use crate::frontend::core::types::MonoType;

/// 泛型尺寸计算器
///
/// 计算泛型类型的编译期尺寸
#[derive(Debug, Clone, Default)]
pub struct GenericSize {
    /// 基础类型大小（字节）
    base_sizes: std::collections::HashMap<&'static str, usize>,
}

impl GenericSize {
    /// 创建新的尺寸计算器
    pub fn new() -> Self {
        let mut base_sizes = std::collections::HashMap::new();
        base_sizes.insert("Bool", 1);
        base_sizes.insert("Int", 8);
        base_sizes.insert("Float", 8);
        base_sizes.insert("String", 8); // 指针
        base_sizes.insert("Void", 0);
        base_sizes.insert("Never", 0);

        Self { base_sizes }
    }

    /// 计算类型的尺寸
    pub fn size_of(
        &self,
        ty: &MonoType,
    ) -> Result<usize, String> {
        match ty {
            MonoType::Bool => self
                .base_sizes
                .get("Bool")
                .cloned()
                .ok_or("Bool not found".to_string()),
            MonoType::Int(_) => self
                .base_sizes
                .get("Int")
                .cloned()
                .ok_or("Int not found".to_string()),
            MonoType::Float(_) => self
                .base_sizes
                .get("Float")
                .cloned()
                .ok_or("Float not found".to_string()),
            MonoType::String => self
                .base_sizes
                .get("String")
                .cloned()
                .ok_or("String not found".to_string()),
            MonoType::Void => self
                .base_sizes
                .get("Void")
                .cloned()
                .ok_or("Void not found".to_string()),
            MonoType::Never => Ok(0),
            MonoType::TypeRef(name) => {
                // 检查是否是 Array<T, N> 类型
                if let Some((elem_type, count)) = self.parse_array_type(name) {
                    return self.size_of_array(elem_type.as_ref(), count);
                }
                // 对于类型引用，尝试查找基础大小
                self.base_sizes
                    .get(name.as_str())
                    .cloned()
                    .ok_or_else(|| format!("TypeRef {} not found", name))
            }
            MonoType::Tuple(types) => {
                let mut total = 0;
                for ty in types {
                    total += self.size_of(ty)?;
                }
                Ok(total)
            }
            MonoType::List(_elem_type) => {
                // List<T> 大小未知（动态大小），返回错误
                Err("Cannot compute size of dynamic List type".to_string())
            }
            MonoType::TypeVar(_) => Err("Cannot compute size of type variable".to_string()),
            MonoType::Fn { .. } => Ok(8), // 指针
            _ => Err(format!("Unknown type: {:?}", ty)),
        }
    }

    /// 解析 Array<T, N> 类型的元素类型和数量
    fn parse_array_type(
        &self,
        type_name: &str,
    ) -> Option<(Box<MonoType>, usize)> {
        if !type_name.starts_with("Array(") {
            return None;
        }

        // 提取泛型参数部分
        let args_str = &type_name["Array(".len()..type_name.len().saturating_sub(1)];

        // 分割参数，找到元素类型和数量
        let mut args = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in args_str.chars() {
            match c {
                ',' if depth == 0 => {
                    if !current.trim().is_empty() {
                        args.push(current.trim().to_string());
                    }
                    current = String::new();
                }
                '(' => {
                    depth += 1;
                    current.push(c);
                }
                ')' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    current.push(c);
                }
                _ => current.push(c),
            }
        }

        if !current.trim().is_empty() {
            args.push(current.trim().to_string());
        }

        if args.len() < 2 {
            return None;
        }

        // 解析元素类型
        let elem_type = Box::new(MonoType::TypeRef(args[0].clone()));

        // 解析数组长度（尝试解析为整数）
        let count = args[1].parse::<usize>().ok()?;

        Some((elem_type, count))
    }

    /// 计算数组的尺寸
    fn size_of_array(
        &self,
        elem_type: &MonoType,
        count: usize,
    ) -> Result<usize, String> {
        let elem_size = self.size_of(elem_type)?;
        Ok(elem_size.saturating_mul(count))
    }
}

/// 尺寸表达式
///
/// 用于表示类型尺寸的表达式
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SizeExpr {
    /// 常量
    Const(usize),

    /// 乘法
    Mul(Box<SizeExpr>, Box<SizeExpr>),

    /// 加法
    Add(Box<SizeExpr>, Box<SizeExpr>),
}

impl SizeExpr {
    /// 计算表达式
    pub fn eval(&self) -> Result<SizeResult, String> {
        match self {
            SizeExpr::Const(n) => Ok(SizeResult::new(*n, true)),
            SizeExpr::Mul(a, b) => {
                let a_result = a.eval()?;
                let b_result = b.eval()?;
                Ok(SizeResult::new(
                    a_result.size.saturating_mul(b_result.size),
                    a_result.is_const && b_result.is_const,
                ))
            }
            SizeExpr::Add(a, b) => {
                let a_result = a.eval()?;
                let b_result = b.eval()?;
                Ok(SizeResult::new(
                    a_result.size.saturating_add(b_result.size),
                    a_result.is_const && b_result.is_const,
                ))
            }
        }
    }
}

/// 泛型尺寸计算结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeResult {
    /// 尺寸值
    pub size: usize,

    /// 是否是常量
    pub is_const: bool,
}

impl SizeResult {
    /// 创建成功结果
    pub fn new(
        size: usize,
        is_const: bool,
    ) -> Self {
        Self { size, is_const }
    }
}

/// 预定义的尺寸计算
pub mod predefined_sizes {
    use super::*;

    /// 计算类型数组的尺寸
    pub fn array_type_size(
        elem_type: MonoType,
        count: usize,
    ) -> Result<SizeResult, String> {
        let elem_size = GenericSize::new().size_of(&elem_type)?;
        Ok(SizeResult::new(elem_size * count, true))
    }

    /// 计算元组的尺寸
    pub fn tuple_size(types: &[MonoType]) -> Result<SizeResult, String> {
        let mut total = 0;
        let mut all_const = true;

        for ty in types {
            let size = GenericSize::new().size_of(ty)?;
            total += size;
            if matches!(ty, MonoType::TypeVar(_)) {
                all_const = false;
            }
        }

        Ok(SizeResult::new(total, all_const))
    }
}

// ====================================================================
// LiteralTypeValidator
// ====================================================================
/// 字面量类型验证
///
/// 实现 Const 泛型的字面量类型验证和类型检查。
///
/// 支持：
/// - 字面量类型到 ConstValue 的转换
/// - Const 参数的类型验证
/// - 字面量类型约束检查
use crate::frontend::core::parser::ast::{Type, GenericParam, GenericParamKind};
use crate::frontend::core::types::ConstKind;

/// Integer types for const parameters
const INT_TYPES: &[&str] = &["Int", "I8", "I16", "I32", "I64", "U8", "U16", "U32", "U64"];
/// Float types for const parameters
const FLOAT_TYPES: &[&str] = &["Float", "F32", "F64"];

/// 字面量类型信息
#[derive(Debug, Clone)]
pub struct LiteralTypeInfo {
    /// 字面量名称
    pub name: String,
    /// 对应的 ConstValue
    pub value: ConstValue,
    /// 基础类型
    pub base_type: MonoType,
}

/// Helper to convert type name to ConstKind
fn type_name_to_const_kind(name: &str) -> Option<ConstKind> {
    if INT_TYPES.contains(&name) {
        Some(ConstKind::Int(None))
    } else if name == "Bool" {
        Some(ConstKind::Bool)
    } else if FLOAT_TYPES.contains(&name) {
        Some(ConstKind::Float(None))
    } else if name == "Char" {
        Some(ConstKind::Int(None))
    } else {
        None
    }
}

/// 字面量类型验证器
#[derive(Debug, Clone, Default)]
pub struct LiteralTypeValidator {
    /// 注册的字面量类型
    literal_types: HashMap<String, LiteralTypeInfo>,
    /// Const 参数绑定
    const_params: HashMap<String, ConstValue>,
}

impl LiteralTypeValidator {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self {
            literal_types: HashMap::new(),
            const_params: HashMap::new(),
        }
    }

    /// 注册字面量类型
    pub fn register_literal_type(
        &mut self,
        name: String,
        value: ConstValue,
        base_type: MonoType,
    ) {
        self.literal_types.insert(
            name.clone(),
            LiteralTypeInfo {
                name,
                value,
                base_type,
            },
        );
    }

    /// 绑定 Const 参数
    pub fn bind_const_param(
        &mut self,
        name: String,
        value: ConstValue,
    ) {
        self.const_params.insert(name, value);
    }

    /// 解析 AST 类型为字面量类型信息
    pub fn parse_literal_type<'a>(
        &'a self,
        ty: &'a Type,
    ) -> Option<(String, ConstValue)> {
        match ty {
            Type::Literal {
                name, base_type: _, ..
            } => {
                // 首先检查是否是已注册的 Const 参数
                if let Some(value) = self.const_params.get(name) {
                    return Some((name.clone(), value.clone()));
                }
                // 然后检查是否是已注册的字面量类型
                if let Some(info) = self.literal_types.get(name) {
                    return Some((info.name.clone(), info.value.clone()));
                }
                // 尝试从名称解析
                if let Some(value) = ConstValue::from_literal_name(name) {
                    return Some((name.clone(), value));
                }
                None
            }
            Type::Name { name, .. } => {
                // 检查是否是已注册的 Const 参数
                if let Some(value) = self.const_params.get(name) {
                    return Some((name.clone(), value.clone()));
                }
                None
            }
            _ => None,
        }
    }

    /// 验证类型是否是有效的 Const 类型
    pub fn validate_const_type(
        &self,
        ty: &Type,
    ) -> Option<ConstKind> {
        match ty {
            Type::Name { name, .. } => type_name_to_const_kind(name),
            Type::Literal { name, .. } => ConstValue::from_literal_name(name).map(|v| v.kind()),
            _ => None,
        }
    }

    /// 检查值是否是给定类型的有效值
    pub fn matches_type(
        &self,
        value: &ConstValue,
        kind: &ConstKind,
    ) -> bool {
        kind.matches(value)
    }

    /// 获取所有注册的 Const 参数
    pub fn const_params(&self) -> &HashMap<String, ConstValue> {
        &self.const_params
    }

    /// 获取所有注册的字面量类型
    pub fn literal_types(&self) -> &HashMap<String, LiteralTypeInfo> {
        &self.literal_types
    }

    /// 清除所有绑定
    pub fn clear(&mut self) {
        self.literal_types.clear();
        self.const_params.clear();
    }
}

/// 从 AST GenericParam 提取 Const 参数信息
pub fn extract_const_param_info(param: &GenericParam) -> Option<(String, ConstKind)> {
    match &param.kind {
        GenericParamKind::Const { const_type } => {
            let name = param.name.clone();
            if let Type::Name {
                name: type_name, ..
            } = const_type.as_ref()
            {
                type_name_to_const_kind(type_name).map(|kind| (name, kind))
            } else {
                None
            }
        }
        GenericParamKind::Type => None,
        GenericParamKind::Platform => None, // 平台参数不是常量参数
    }
}

/// 将 AST 类型转换为 MonoType
pub fn ast_type_to_mono_type(ty: &Type) -> Option<MonoType> {
    match ty {
        Type::Name { name, .. } => Some(MonoType::TypeRef(name.clone())),
        Type::Int(n) => Some(MonoType::Int(*n)),
        Type::Float(n) => Some(MonoType::Float(*n)),
        Type::Char => Some(MonoType::Char),
        Type::String => Some(MonoType::String),
        Type::Bytes => Some(MonoType::Bytes),
        Type::Bool => Some(MonoType::Bool),
        Type::Void => Some(MonoType::Void),
        _ => None,
    }
}
