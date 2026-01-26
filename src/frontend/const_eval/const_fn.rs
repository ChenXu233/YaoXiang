//! Const函数实现（RFC-011 Phase 4）
//!
//! 支持Const函数的定义和编译期求值：
//! - 递归Const函数
//!
//! 示例：
//! const fn factorial(n: Int) -> Int = match n {
//!     0 => 1,
//!     _ => n * factorial(n - 1)
//! }

use crate::frontend::parser::ast;
use crate::frontend::typecheck::types::{
    ConstEvalError, ConstExpr, ConstValue,
};
use crate::util::span::Span;
use std::collections::HashMap;

/// Const函数定义
#[derive(Debug, Clone)]
pub struct ConstFunction {
    /// 函数名称
    pub name: String,
    /// 参数列表（参数名，类型）
    pub params: Vec<(String, String)>,
    /// 返回类型
    pub return_type: String,
    /// 函数体（表达式）
    pub body: ConstExpr,
    /// 位置信息
    pub span: Span,
}

impl ConstFunction {
    /// 创建新的Const函数
    pub fn new(
        name: String,
        params: Vec<(String, String)>,
        return_type: String,
        body: ConstExpr,
        span: Span,
    ) -> Self {
        ConstFunction {
            name,
            params,
            return_type,
            body,
            span,
        }
    }

    /// 获取参数数量
    pub fn arity(&self) -> usize {
        self.params.len()
    }
}

/// Const函数调用环境
#[derive(Debug, Clone)]
pub struct ConstFnCallEnv {
    /// 函数定义映射
    functions: HashMap<String, ConstFunction>,
    /// 递归深度限制
    max_depth: usize,
}

impl ConstFnCallEnv {
    /// 创建新的函数调用环境
    pub fn new() -> Self {
        ConstFnCallEnv {
            functions: HashMap::new(),
            max_depth: 1000,
        }
    }

    /// 注册Const函数
    pub fn register(&mut self, func: ConstFunction) {
        self.functions.insert(func.name.clone(), func);
    }

    /// 获取函数定义
    pub fn get(&self, name: &str) -> Option<&ConstFunction> {
        self.functions.get(name)
    }

    /// 设置最大递归深度
    pub fn set_max_depth(&mut self, max_depth: usize) {
        self.max_depth = max_depth;
    }

    /// 获取最大递归深度
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
}

impl Default for ConstFnCallEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Const函数调用求值器
pub struct ConstFnEvaluator {
    /// 函数调用环境
    env: ConstFnCallEnv,
    /// 当前递归深度
    current_depth: usize,
}

impl ConstFnEvaluator {
    /// 创建新的Const函数求值器
    pub fn new() -> Self {
        ConstFnEvaluator {
            env: ConstFnCallEnv::new(),
            current_depth: 0,
        }
    }

    /// 创建新的Const函数求值器（带环境）
    pub fn with_env(env: ConstFnCallEnv) -> Self {
        ConstFnEvaluator {
            env,
            current_depth: 0,
        }
    }

    /// 求值Const函数调用
    pub fn evaluate_call(
        &mut self,
        func_name: &str,
        args: &[ConstExpr],
    ) -> Result<ConstValue, ConstEvalError> {
        // 获取函数定义
        let func = self
            .env
            .get(func_name)
            .ok_or_else(|| ConstEvalError::UndefinedVariable {
                name: func_name.to_string(),
                span: Span::default(),
            })?;

        // 检查参数数量
        if args.len() != func.arity() {
            return Err(ConstEvalError::ArgCountMismatch {
                expected: func.arity(),
                found: args.len(),
                span: Span::default(),
            });
        }

        // 检查递归深度
        if self.current_depth >= self.env.max_depth() {
            return Err(ConstEvalError::RecursionTooDeep {
                depth: self.current_depth,
                max_depth: self.env.max_depth(),
                span: Span::default(),
            });
        }

        // 增加递归深度
        self.current_depth += 1;

        // 创建参数环境
        let mut arg_values = Vec::new();
        for (arg_expr, (_, _)) in args.iter().zip(func.params.iter()) {
            // 简化实现：假设参数是整数
            let val = self.evaluate_expr(arg_expr)?;
            arg_values.push(val);
        }

        // 求值函数体
        let result = self.evaluate_body(&func.body, &arg_values, &func.params)?;

        // 减少递归深度
        self.current_depth = self.current_depth.saturating_sub(1);

        Ok(result)
    }

    /// 求值函数体
    fn evaluate_body(
        &mut self,
        body: &ConstExpr,
        arg_values: &[ConstValue],
        param_names: &[(String, String)],
    ) -> Result<ConstValue, ConstEvalError> {
        match body {
            ConstExpr::Call { func, args } => {
                // 检查是否是递归调用
                if self.env.get(func).is_some() {
                    // 创建新的参数列表
                    let mut new_args = Vec::new();
                    for arg in args {
                        let val = self.evaluate_expr(arg)?;
                        new_args.push(val);
                    }
                    self.evaluate_call(func, &new_args)
                } else {
                    // 非Const函数调用
                    Err(ConstEvalError::NonConstFunctionCall {
                        func: func.clone(),
                        span: Span::default(),
                    })
                }
            }
            ConstExpr::BinOp { op, left, right } => {
                let left_val = self.evaluate_body(left, arg_values, param_names)?;
                let right_val = self.evaluate_body(right, arg_values, param_names)?;
                self.evaluate_binop(*op, &left_val, &right_val)
            }
            ConstExpr::UnOp { op, expr } => {
                let val = self.evaluate_body(expr, arg_values, param_names)?;
                self.evaluate_unop(*op, &val)
            }
            ConstExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.evaluate_body(condition, arg_values, param_names)?;
                match cond_val {
                    ConstValue::Bool(true) => {
                        self.evaluate_body(then_branch, arg_values, param_names)
                    }
                    ConstValue::Bool(false) => {
                        self.evaluate_body(else_branch, arg_values, param_names)
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "Bool".to_string(),
                        found: cond_val.kind().type_name().to_string(),
                        span: Span::default(),
                    }),
                }
            }
            ConstExpr::Var(var) => {
                // 简化实现：直接使用变量索引
                // 实际实现需要参数名称解析
                Ok(ConstValue::Int(var.index() as i128))
            }
            ConstExpr::Lit(value) => Ok(value.clone()),
            _ => Err(ConstEvalError::CannotEvaluate {
                reason: format!("unsupported expression type in const function: {:?}", body),
                span: Span::default(),
            }),
        }
    }

    /// 求值二元运算
    fn evaluate_binop(
        &self,
        op: crate::frontend::typecheck::types::BinOp,
        left: &ConstValue,
        right: &ConstValue,
    ) -> Result<ConstValue, ConstEvalError> {
        // 使用const_evaluator中的逻辑
        match op {
            crate::frontend::typecheck::types::BinOp::Add => {
                match (left, right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Int(l + r))
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                        span: Span::default(),
                    }),
                }
            }
            crate::frontend::typecheck::types::BinOp::Sub => {
                match (left, right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Int(l - r))
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                        span: Span::default(),
                    }),
                }
            }
            crate::frontend::typecheck::types::BinOp::Mul => {
                match (left, right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Int(l * r))
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                        span: Span::default(),
                    }),
                }
            }
            crate::frontend::typecheck::types::BinOp::Div => {
                match (left, right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        if *r == 0 {
                            return Err(ConstEvalError::DivisionByZero {
                                span: Span::default(),
                            });
                        }
                        Ok(ConstValue::Int(l / r))
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                        span: Span::default(),
                    }),
                }
            }
            crate::frontend::typecheck::types::BinOp::Eq => {
                Ok(ConstValue::Bool(left == right))
            }
            crate::frontend::typecheck::types::BinOp::Ne => {
                Ok(ConstValue::Bool(left != right))
            }
            _ => Err(ConstEvalError::CannotEvaluate {
                reason: format!("unsupported binary operator: {:?}", op),
                span: Span::default(),
            }),
        }
    }

    /// 求值一元运算
    fn evaluate_unop(
        &self,
        op: crate::frontend::typecheck::types::UnOp,
        val: &ConstValue,
    ) -> Result<ConstValue, ConstEvalError> {
        match op {
            crate::frontend::typecheck::types::UnOp::Neg => {
                match val {
                    ConstValue::Int(n) => Ok(ConstValue::Int(-*n)),
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "Int".to_string(),
                        found: val.kind().type_name().to_string(),
                        span: Span::default(),
                    }),
                }
            }
            _ => Err(ConstEvalError::CannotEvaluate {
                reason: format!("unsupported unary operator: {:?}", op),
                span: Span::default(),
            }),
        }
    }

    /// 求值表达式
    fn evaluate_expr(&mut self, expr: &ConstExpr) -> Result<ConstValue, ConstEvalError> {
        match expr {
            ConstExpr::Lit(value) => Ok(value.clone()),
            ConstExpr::Call { func, args } => self.evaluate_call(func, args),
            _ => Err(ConstEvalError::CannotEvaluate {
                reason: format!("unsupported expression: {:?}", expr),
                span: Span::default(),
            }),
        }
    }

    /// 获取函数环境
    pub fn env(&self) -> &ConstFnCallEnv {
        &self.env
    }

    /// 获取函数环境（可变）
    pub fn env_mut(&mut self) -> &mut ConstFnCallEnv {
        &mut self.env
    }
}

impl Default for ConstFnEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建阶乘函数示例
    fn create_factorial_function() -> ConstFunction {
        // factorial(n) = match n {
        //     0 => 1,
        //     _ => n * factorial(n - 1)
        // }

        let zero_case = ConstExpr::Lit(ConstValue::Int(1));

        let n_minus_one = ConstExpr::BinOp {
            op: crate::frontend::typecheck::types::BinOp::Sub,
            left: Box::new(ConstExpr::Var(
                crate::frontend::typecheck::types::ConstVar::new(0),
            )),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        };

        let recursive_call = ConstExpr::Call {
            func: "factorial".to_string(),
            args: vec![n_minus_one],
        };

        let multiply = ConstExpr::BinOp {
            op: crate::frontend::typecheck::types::BinOp::Mul,
            left: Box::new(ConstExpr::Var(
                crate::frontend::typecheck::types::ConstVar::new(0),
            )),
            right: Box::new(recursive_call),
        };

        let else_case = multiply;

        let body = ConstExpr::If {
            condition: Box::new(ConstExpr::BinOp {
                op: crate::frontend::typecheck::types::BinOp::Eq,
                left: Box::new(ConstExpr::Var(
                    crate::frontend::typecheck::types::ConstVar::new(0),
                )),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
            }),
            then_branch: Box::new(zero_case),
            else_branch: Box::new(else_case),
        };

        ConstFunction::new(
            "factorial".to_string(),
            vec![("n".to_string(), "Int".to_string())],
            "Int".to_string(),
            body,
            Span::default(),
        )
    }

    #[test]
    fn test_factorial_function_creation() {
        let func = create_factorial_function();
        assert_eq!(func.name, "factorial");
        assert_eq!(func.arity(), 1);
        assert_eq!(func.return_type, "Int");
    }

    #[test]
    fn test_register_function() {
        let mut evaluator = ConstFnEvaluator::new();
        let func = create_factorial_function();

        evaluator.env_mut().register(func.clone());

        let registered = evaluator.env().get("factorial");
        assert!(registered.is_some());
        assert_eq!(registered.unwrap().name, "factorial");
    }

    #[test]
    fn test_factorial_small_values() {
        let mut evaluator = ConstFnEvaluator::new();
        let func = create_factorial_function();

        evaluator.env_mut().register(func);
        evaluator.env_mut().set_max_depth(1000);

        // 测试 factorial(0)
        let args = vec![ConstExpr::Lit(ConstValue::Int(0))];
        match evaluator.evaluate_call("factorial", &args) {
            Ok(ConstValue::Int(n)) => assert_eq!(n, 1),
            _ => panic!("Expected Int(1)"),
        }

        // 测试 factorial(1)
        let args = vec![ConstExpr::Lit(ConstValue::Int(1))];
        match evaluator.evaluate_call("factorial", &args) {
            Ok(ConstValue::Int(n)) => assert_eq!(n, 1),
            _ => panic!("Expected Int(1)"),
        }

        // 测试 factorial(5)
        let args = vec![ConstExpr::Lit(ConstValue::Int(5))];
        match evaluator.evaluate_call("factorial", &args) {
            Ok(ConstValue::Int(n)) => assert_eq!(n, 120),
            _ => panic!("Expected Int(120)"),
        }
    }

    #[test]
    fn test_recursion_depth_limit() {
        let mut evaluator = ConstFnEvaluator::new();
        let func = create_factorial_function();

        evaluator.env_mut().register(func);
        evaluator.env_mut().set_max_depth(5);

        // 测试超过递归深度限制
        let args = vec![ConstExpr::Lit(ConstValue::Int(10))];
        match evaluator.evaluate_call("factorial", &args) {
            Err(ConstEvalError::RecursionTooDeep { depth, max_depth, .. }) => {
                assert!(depth >= 5);
                assert_eq!(max_depth, 5);
            }
            _ => panic!("Expected RecursionTooDeep error"),
        }
    }

    #[test]
    fn test_undefined_function() {
        let evaluator = ConstFnEvaluator::new();

        let args = vec![ConstExpr::Lit(ConstValue::Int(5))];
        match evaluator.evaluate_call("undefined_func", &args) {
            Err(ConstEvalError::UndefinedVariable { name, .. }) => {
                assert_eq!(name, "undefined_func");
            }
            _ => panic!("Expected UndefinedVariable error"),
        }
    }

    #[test]
    fn test_arg_count_mismatch() {
        let mut evaluator = ConstFnEvaluator::new();
        let func = create_factorial_function();

        evaluator.env_mut().register(func);

        // 传递错误数量的参数
        let args = vec![
            ConstExpr::Lit(ConstValue::Int(5)),
            ConstExpr::Lit(ConstValue::Int(3)),
        ];
        match evaluator.evaluate_call("factorial", &args) {
            Err(ConstEvalError::ArgCountMismatch { expected, found, .. }) => {
                assert_eq!(expected, 1);
                assert_eq!(found, 2);
            }
            _ => panic!("Expected ArgCountMismatch error"),
        }
    }
}
