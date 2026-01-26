//! 类型族系统 (RFC-011 Phase 5)
//!
//! 实现类型级算术、自然数类型系统和复杂的类型族

use super::{TypeLevelError, TypeLevelValue, Nat};
use std::fmt;

/// 类型级算术运算
#[derive(Debug, Clone)]
pub enum TypeLevelArithmetic {
    /// 加法：Add[A, B]
    Add {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 减法：Sub[A, B]
    Sub {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 乘法：Mult[A, B]
    Mult {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 除法：Div[A, B]
    Div {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 模运算：Mod[A, B]
    Mod {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 指数运算：Pow[A, B]
    Pow {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 自然数常量
    Nat(Nat),
    /// 变量
    Var { name: String, index: usize },
}

impl TypeLevelArithmetic {
    /// 创建加法
    pub fn add<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelArithmetic::Add {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建减法
    pub fn sub<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelArithmetic::Sub {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建乘法
    pub fn mult<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelArithmetic::Mult {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建除法
    pub fn div<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelArithmetic::Div {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建模运算
    pub fn mod_<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelArithmetic::Mod {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建指数运算
    pub fn pow<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelArithmetic::Pow {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建自然数
    pub fn nat(n: Nat) -> Self {
        TypeLevelArithmetic::Nat(n)
    }

    /// 创建变量
    pub fn var(
        name: &str,
        index: usize,
    ) -> Self {
        TypeLevelArithmetic::Var {
            name: name.to_string(),
            index,
        }
    }

    /// 计算算术表达式
    pub fn evaluate(
        &self,
        computer: &mut super::TypeLevelComputer,
    ) -> Result<TypeLevelValue, TypeLevelError> {
        match self {
            TypeLevelArithmetic::Nat(n) => Ok(TypeLevelValue::Nat(n.clone())),
            TypeLevelArithmetic::Var { index, .. } => {
                if let Some(value) = computer.var_mapping.get(index) {
                    Ok(value.clone())
                } else {
                    Err(TypeLevelError::TypeFamilyError {
                        reason: format!("Unbound variable {}", index),
                        span: crate::util::span::Span::default(),
                    })
                }
            }
            TypeLevelArithmetic::Add { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        Ok(TypeLevelValue::Nat(a_nat.add(&b_nat)))
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Add requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeLevelArithmetic::Sub { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        if a_nat.cmp(&b_nat) == std::cmp::Ordering::Less {
                            Err(TypeLevelError::TypeFamilyError {
                                reason: "Subtraction would result in negative number".to_string(),
                                span: crate::util::span::Span::default(),
                            })
                        } else {
                            let _result = Nat::Zero;
                            let mut a_temp = a_nat.clone();
                            let mut b_temp = b_nat.clone();
                            while b_temp.cmp(&Nat::Zero) == std::cmp::Ordering::Greater {
                                if let Some(pred) = a_temp.pred() {
                                    a_temp = pred.clone();
                                }
                                if let Some(pred) = b_temp.pred() {
                                    b_temp = pred.clone();
                                }
                            }
                            Ok(TypeLevelValue::Nat(a_temp))
                        }
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Sub requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeLevelArithmetic::Mult { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        Ok(TypeLevelValue::Nat(a_nat.mul(&b_nat)))
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Mult requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeLevelArithmetic::Div { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        if b_nat.is_zero() {
                            Err(TypeLevelError::TypeFamilyError {
                                reason: "Division by zero".to_string(),
                                span: crate::util::span::Span::default(),
                            })
                        } else {
                            // 使用减法实现除法：计算 a / b
                            let mut quotient = Nat::Zero;
                            let mut remainder = a_nat.clone();
                            while remainder.cmp(&b_nat) == std::cmp::Ordering::Greater
                                || remainder.cmp(&b_nat) == std::cmp::Ordering::Equal
                            {
                                // remainder = remainder - b_nat
                                remainder = remainder.sub(&b_nat);
                                quotient = quotient.succ();
                            }
                            Ok(TypeLevelValue::Nat(quotient))
                        }
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Div requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeLevelArithmetic::Mod { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        if b_nat.is_zero() {
                            Err(TypeLevelError::TypeFamilyError {
                                reason: "Modulo by zero".to_string(),
                                span: crate::util::span::Span::default(),
                            })
                        } else {
                            // 计算模运算
                            let mut remainder = a_nat.clone();
                            while remainder.cmp(&b_nat) == std::cmp::Ordering::Greater
                                || remainder.cmp(&b_nat) == std::cmp::Ordering::Equal
                            {
                                let mut temp = remainder.clone();
                                for _ in 0..b_nat.to_usize().unwrap_or(0) {
                                    temp = match temp.pred() {
                                        Some(p) => p.clone(),
                                        None => Nat::Zero,
                                    };
                                }
                                remainder = temp;
                            }
                            Ok(TypeLevelValue::Nat(remainder))
                        }
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Mod requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeLevelArithmetic::Pow { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        if b_nat.is_zero() {
                            Ok(TypeLevelValue::Nat(Nat::from_usize(1)))
                        } else {
                            let mut result = Nat::from_usize(1);
                            let mut exp = b_nat.clone();
                            let base = a_nat.clone();
                            while !exp.is_zero() {
                                if exp.cmp(&Nat::Zero) == std::cmp::Ordering::Greater {
                                    result = result.mul(&base);
                                    exp = match exp.pred() {
                                        Some(p) => p.clone(),
                                        None => Nat::Zero,
                                    };
                                }
                            }
                            Ok(TypeLevelValue::Nat(result))
                        }
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Pow requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
        }
    }
}

impl fmt::Display for TypeLevelArithmetic {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            TypeLevelArithmetic::Nat(n) => write!(f, "{}", n),
            TypeLevelArithmetic::Var { name, .. } => write!(f, "{}", name),
            TypeLevelArithmetic::Add { a, b } => write!(f, "({} + {})", a, b),
            TypeLevelArithmetic::Sub { a, b } => write!(f, "({} - {})", a, b),
            TypeLevelArithmetic::Mult { a, b } => write!(f, "({} * {})", a, b),
            TypeLevelArithmetic::Div { a, b } => write!(f, "({} / {})", a, b),
            TypeLevelArithmetic::Mod { a, b } => write!(f, "({} % {})", a, b),
            TypeLevelArithmetic::Pow { a, b } => write!(f, "({} ^ {})", a, b),
        }
    }
}

impl From<Nat> for TypeLevelArithmetic {
    fn from(n: Nat) -> Self {
        TypeLevelArithmetic::Nat(n)
    }
}

impl From<usize> for TypeLevelArithmetic {
    fn from(n: usize) -> Self {
        TypeLevelArithmetic::Nat(Nat::from_usize(n))
    }
}

/// 比较操作
#[derive(Debug, Clone)]
pub enum TypeLevelComparison {
    /// 等于：Eq[A, B]
    Eq {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 不等于：Neq[A, B]
    Neq {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 小于：Lt[A, B]
    Lt {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 小于等于：Le[A, B]
    Le {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 大于：Gt[A, B]
    Gt {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 大于等于：Ge[A, B]
    Ge {
        a: Box<TypeLevelArithmetic>,
        b: Box<TypeLevelArithmetic>,
    },
    /// 变量
    Var { name: String, index: usize },
}

impl TypeLevelComparison {
    /// 创建等于比较
    pub fn eq<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelComparison::Eq {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建不等于比较
    pub fn neq<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelComparison::Neq {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建小于比较
    pub fn lt<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelComparison::Lt {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建小于等于比较
    pub fn le<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelComparison::Le {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建大于比较
    pub fn gt<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelComparison::Gt {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建大于等于比较
    pub fn ge<A, B>(
        a: A,
        b: B,
    ) -> Self
    where
        A: Into<TypeLevelArithmetic>,
        B: Into<TypeLevelArithmetic>,
    {
        TypeLevelComparison::Ge {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建变量
    pub fn var(
        name: &str,
        index: usize,
    ) -> Self {
        TypeLevelComparison::Var {
            name: name.to_string(),
            index,
        }
    }

    /// 计算比较结果
    pub fn evaluate(
        &self,
        computer: &mut super::TypeLevelComputer,
    ) -> Result<TypeLevelValue, TypeLevelError> {
        match self {
            TypeLevelComparison::Var { index, .. } => {
                if let Some(value) = computer.var_mapping.get(index) {
                    Ok(value.clone())
                } else {
                    Err(TypeLevelError::TypeFamilyError {
                        reason: format!("Unbound variable {}", index),
                        span: crate::util::span::Span::default(),
                    })
                }
            }
            TypeLevelComparison::Eq { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                Ok(TypeLevelValue::Bool(a_val == b_val))
            }
            TypeLevelComparison::Neq { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                Ok(TypeLevelValue::Bool(a_val != b_val))
            }
            TypeLevelComparison::Lt { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => Ok(
                        TypeLevelValue::Bool(a_nat.cmp(&b_nat) == std::cmp::Ordering::Less),
                    ),
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Lt requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeLevelComparison::Le { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        Ok(TypeLevelValue::Bool(
                            a_nat.cmp(&b_nat) == std::cmp::Ordering::Less
                                || a_nat.cmp(&b_nat) == std::cmp::Ordering::Equal,
                        ))
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Le requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeLevelComparison::Gt { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => Ok(
                        TypeLevelValue::Bool(a_nat.cmp(&b_nat) == std::cmp::Ordering::Greater),
                    ),
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Gt requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeLevelComparison::Ge { a, b } => {
                let a_val = a.evaluate(computer)?;
                let b_val = b.evaluate(computer)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        Ok(TypeLevelValue::Bool(
                            a_nat.cmp(&b_nat) == std::cmp::Ordering::Greater
                                || a_nat.cmp(&b_nat) == std::cmp::Ordering::Equal,
                        ))
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Ge requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
        }
    }
}

impl From<TypeLevelArithmetic> for TypeLevelComparison {
    fn from(arith: TypeLevelArithmetic) -> Self {
        match arith {
            TypeLevelArithmetic::Var { name, index } => TypeLevelComparison::var(&name, index),
            _ => TypeLevelComparison::Var {
                name: arith.to_string(),
                index: 0,
            },
        }
    }
}

/// 类型级算术运算器
#[derive(Debug)]
pub struct TypeLevelArithmeticProcessor {
    computer: super::TypeLevelComputer,
}

impl TypeLevelArithmeticProcessor {
    /// 创建新的算术处理器
    pub fn new() -> Self {
        TypeLevelArithmeticProcessor {
            computer: super::TypeLevelComputer::new(),
        }
    }

    /// 计算算术表达式
    pub fn compute_arith(
        &mut self,
        expr: &TypeLevelArithmetic,
    ) -> Result<TypeLevelValue, TypeLevelError> {
        expr.evaluate(&mut self.computer)
    }

    /// 计算比较表达式
    pub fn compute_cmp(
        &mut self,
        expr: &TypeLevelComparison,
    ) -> Result<TypeLevelValue, TypeLevelError> {
        expr.evaluate(&mut self.computer)
    }

    /// 绑定变量
    pub fn bind(
        &mut self,
        index: usize,
        value: TypeLevelValue,
    ) {
        self.computer.bind_var(index, value);
    }

    /// 取消绑定变量
    pub fn unbind(
        &mut self,
        index: usize,
    ) {
        self.computer.unbind_var(index);
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.computer.clear_cache();
    }
}

impl Default for TypeLevelArithmeticProcessor {
    fn default() -> Self {
        TypeLevelArithmeticProcessor::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_level_add() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::add(3, 4);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(7)));
    }

    #[test]
    fn test_type_level_mult() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::mult(3, 4);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(12)));
    }

    #[test]
    fn test_type_level_pow() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::pow(2, 3);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(8)));
    }

    #[test]
    fn test_type_level_div() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::div(10, 3);
        let result = processor.compute_arith(&expr).unwrap();
        // 10 / 3 = 3
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(3)));
    }

    #[test]
    fn test_type_level_mod() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::mod_(10, 3);
        let result = processor.compute_arith(&expr).unwrap();
        // 10 % 3 = 1
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(1)));
    }

    #[test]
    fn test_type_level_comparison() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelComparison::lt(3, 4);
        let result = processor.compute_cmp(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Bool(true));
    }

    #[test]
    fn test_complex_expression() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        // (3 + 4) * 2 = 14
        let expr = TypeLevelArithmetic::mult(TypeLevelArithmetic::add(3, 4), 2);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(14)));
    }
}
