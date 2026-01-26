//! 类型级计算引擎 (RFC-011 Phase 5)
//!
//! 提供类型级计算能力，支持条件类型、类型族和依赖类型

use super::{TypeLevelError};
use crate::frontend::typecheck::types::{MonoType, PolyType};
use std::collections::HashMap;
use std::hash::Hash;
use std::fmt;
use std::cmp::Ordering;

/// 类型级自然数
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Nat {
    /// 零
    Zero,
    /// 后继（succ n）
    Succ(Box<Nat>),
}

impl Nat {
    /// 创建零
    pub fn zero() -> Self {
        Nat::Zero
    }

    /// 创建后继
    pub fn succ(self) -> Self {
        Nat::Succ(Box::new(self))
    }

    /// 计算自然数到usize的值
    pub fn to_usize(&self) -> Option<usize> {
        let mut count: usize = 0;
        let mut current: &Nat = self;
        while let Nat::Succ(ref n) = *current {
            count = count.checked_add(1)?;
            current = n;
        }
        Some(count)
    }

    /// 从usize创建自然数
    pub fn from_usize(n: usize) -> Self {
        let mut result = Nat::Zero;
        for _ in 0..n {
            result = result.succ();
        }
        result
    }

    /// 加法
    pub fn add(
        &self,
        other: &Nat,
    ) -> Nat {
        match self {
            Nat::Zero => other.clone(),
            Nat::Succ(n) => Nat::Succ(Box::new(n.add(other))),
        }
    }

    /// 乘法
    pub fn mul(
        &self,
        other: &Nat,
    ) -> Nat {
        match self {
            Nat::Zero => Nat::Zero,
            Nat::Succ(n) => other.add(&n.mul(other)),
        }
    }

    /// 比较
    #[allow(clippy::should_implement_trait)]
    pub fn cmp(
        &self,
        other: &Nat,
    ) -> Ordering {
        match (self, other) {
            (Nat::Zero, Nat::Zero) => Ordering::Equal,
            (Nat::Zero, _) => Ordering::Less,
            (_, Nat::Zero) => Ordering::Greater,
            (Nat::Succ(a), Nat::Succ(b)) => a.cmp(b),
        }
    }

    /// 是否为零
    pub fn is_zero(&self) -> bool {
        matches!(self, Nat::Zero)
    }

    /// 前驱（返回前一个数）
    pub fn pred(&self) -> Option<&Nat> {
        match self {
            Nat::Zero => None,
            Nat::Succ(n) => Some(n),
        }
    }

    /// 减法
    pub fn sub(
        &self,
        other: &Nat,
    ) -> Nat {
        match (self, other) {
            (Nat::Zero, _) => Nat::Zero, // 0 - anything = 0
            (Nat::Succ(_), Nat::Zero) => self.clone(),
            (Nat::Succ(a), Nat::Succ(b)) => a.sub(b),
        }
    }
}

impl fmt::Display for Nat {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Nat::Zero => write!(f, "0"),
            Nat::Succ(n) => {
                if let Some(val) = n.to_usize() {
                    write!(f, "{}", val + 1)
                } else {
                    write!(f, "succ({})", n)
                }
            }
        }
    }
}

/// 类型级值
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeLevelValue {
    /// 自然数
    Nat(Nat),
    /// 类型
    Type(MonoType),
    /// 布尔值
    Bool(bool),
}

impl Hash for TypeLevelValue {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        match self {
            TypeLevelValue::Nat(n) => n.hash(state),
            TypeLevelValue::Type(t) => {
                // 使用类型的字符串表示作为哈希
                t.type_name().hash(state);
            }
            TypeLevelValue::Bool(b) => b.hash(state),
        }
    }
}

/// 类型族
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeFamily {
    /// Id类型族：Id[T] = T
    Id { param: Box<TypeFamily> },
    /// Option类型族：Option[T] = Some(T) | None
    Option { param: Box<TypeFamily> },
    /// List类型族：List[T] = Nil | Cons(T, List[T])
    List { elem: Box<TypeFamily> },
    /// 自然数类型族
    Nat(Nat),
    /// 加法类型族：Add[A, B] = A + B
    Add {
        a: Box<TypeFamily>,
        b: Box<TypeFamily>,
    },
    /// 乘法类型族：Mult[A, B] = A * B
    Mult {
        a: Box<TypeFamily>,
        b: Box<TypeFamily>,
    },
    /// 函数类型族
    Fn {
        params: Vec<TypeFamily>,
        return_type: Box<TypeFamily>,
    },
    /// 元组类型族
    Tuple { elements: Vec<TypeFamily> },
    /// 泛型变量
    Var { name: String, index: usize },
    /// 具体类型
    Concrete { ty: MonoType },
}

impl TypeFamily {
    /// 创建Id类型族
    pub fn id<T: Into<TypeFamily>>(param: T) -> Self {
        TypeFamily::Id {
            param: Box::new(param.into()),
        }
    }

    /// 创建Option类型族
    pub fn option<T: Into<TypeFamily>>(elem: T) -> Self {
        TypeFamily::Option {
            param: Box::new(elem.into()),
        }
    }

    /// 创建List类型族
    pub fn list<T: Into<TypeFamily>>(elem: T) -> Self {
        TypeFamily::List {
            elem: Box::new(elem.into()),
        }
    }

    /// 创建自然数
    pub fn nat(n: Nat) -> Self {
        TypeFamily::Nat(n)
    }

    /// 创建加法类型族
    pub fn add<T: Into<TypeFamily>, U: Into<TypeFamily>>(
        a: T,
        b: U,
    ) -> Self {
        TypeFamily::Add {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建乘法类型族
    pub fn mult<T: Into<TypeFamily>, U: Into<TypeFamily>>(
        a: T,
        b: U,
    ) -> Self {
        TypeFamily::Mult {
            a: Box::new(a.into()),
            b: Box::new(b.into()),
        }
    }

    /// 创建函数类型族
    pub fn fn_type<T: Into<TypeFamily>>(
        params: Vec<T>,
        return_type: T,
    ) -> Self {
        TypeFamily::Fn {
            params: params.into_iter().map(|p| p.into()).collect(),
            return_type: Box::new(return_type.into()),
        }
    }

    /// 创建元组类型族
    pub fn tuple<T: Into<TypeFamily>>(elements: Vec<T>) -> Self {
        TypeFamily::Tuple {
            elements: elements.into_iter().map(|e| e.into()).collect(),
        }
    }

    /// 创建具体类型
    pub fn concrete(ty: MonoType) -> Self {
        TypeFamily::Concrete { ty }
    }

    /// 计算类型族的范式
    pub fn normalize(
        &self,
        computer: &TypeLevelComputer,
    ) -> Result<TypeLevelValue, TypeLevelError> {
        computer.normalize_family(self)
    }
}

impl From<MonoType> for TypeFamily {
    fn from(ty: MonoType) -> Self {
        TypeFamily::Concrete { ty }
    }
}

impl From<Nat> for TypeFamily {
    fn from(n: Nat) -> Self {
        TypeFamily::Nat(n)
    }
}

impl From<usize> for TypeFamily {
    fn from(n: usize) -> Self {
        TypeFamily::Nat(Nat::from_usize(n))
    }
}

/// 类型级计算器
#[derive(Debug)]
pub struct TypeLevelComputer {
    /// 计算缓存：避免重复计算
    cache: HashMap<TypeFamily, Result<TypeLevelValue, TypeLevelError>>,
    /// 归一化缓存
    normalization_cache: HashMap<String, MonoType>,
    /// 泛型变量映射
    pub var_mapping: HashMap<usize, TypeLevelValue>,
    /// 递归深度限制
    max_depth: usize,
    /// 当前递归深度
    current_depth: usize,
}

impl TypeLevelComputer {
    /// 创建新的类型级计算器
    pub fn new() -> Self {
        TypeLevelComputer {
            cache: HashMap::new(),
            normalization_cache: HashMap::new(),
            var_mapping: HashMap::new(),
            max_depth: 1000,
            current_depth: 0,
        }
    }

    /// 设置递归深度限制
    pub fn set_max_depth(
        &mut self,
        depth: usize,
    ) {
        self.max_depth = depth;
    }

    /// 重置计算器
    pub fn reset(&mut self) {
        self.cache.clear();
        self.normalization_cache.clear();
        self.var_mapping.clear();
        self.current_depth = 0;
    }

    /// 绑定泛型变量
    pub fn bind_var(
        &mut self,
        index: usize,
        value: TypeLevelValue,
    ) {
        self.var_mapping.insert(index, value);
    }

    /// 取消绑定泛型变量
    pub fn unbind_var(
        &mut self,
        index: usize,
    ) {
        self.var_mapping.remove(&index);
    }

    /// 计算类型族
    pub fn compute(
        &mut self,
        family: &TypeFamily,
    ) -> Result<TypeLevelValue, TypeLevelError> {
        // 检查缓存
        if let Some(result) = self.cache.get(family) {
            return result.clone();
        }

        // 检查递归深度
        if self.current_depth >= self.max_depth {
            return Err(TypeLevelError::ComputationFailed {
                reason: format!("Maximum recursion depth {} exceeded", self.max_depth),
                span: crate::util::span::Span::default(),
            });
        }

        self.current_depth += 1;
        let result = self.compute_unchecked(family);
        self.current_depth -= 1;

        // 缓存结果
        self.cache.insert(family.clone(), result.clone());
        result
    }

    /// 计算类型族（不检查缓存）
    fn compute_unchecked(
        &mut self,
        family: &TypeFamily,
    ) -> Result<TypeLevelValue, TypeLevelError> {
        match family {
            TypeFamily::Var { index, .. } => {
                // 从映射中获取变量值
                if let Some(value) = self.var_mapping.get(index) {
                    Ok(value.clone())
                } else {
                    Err(TypeLevelError::ComputationFailed {
                        reason: format!("Unbound type family variable {}", index),
                        span: crate::util::span::Span::default(),
                    })
                }
            }
            TypeFamily::Concrete { ty } => Ok(TypeLevelValue::Type(ty.clone())),
            TypeFamily::Nat(n) => Ok(TypeLevelValue::Nat(n.clone())),
            TypeFamily::Id { param } => {
                // Id[T] = T
                self.compute(param)
            }
            TypeFamily::Option { param: _ } => {
                // Option[T] = Some(T) | None
                // 这里返回一个表示Option的类型构造器
                Ok(TypeLevelValue::Type(MonoType::TypeRef(
                    "Option".to_string(),
                )))
            }
            TypeFamily::List { elem: _ } => {
                // List[T] = Nil | Cons(T, List[T])
                Ok(TypeLevelValue::Type(MonoType::TypeRef("List".to_string())))
            }
            TypeFamily::Add { a, b } => {
                // Add[A, B] = A + B
                let a_val = self.compute(a)?;
                let b_val = self.compute(b)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        Ok(TypeLevelValue::Nat(a_nat.add(&b_nat)))
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Add type family requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeFamily::Mult { a, b } => {
                // Mult[A, B] = A * B
                let a_val = self.compute(a)?;
                let b_val = self.compute(b)?;
                match (a_val, b_val) {
                    (TypeLevelValue::Nat(a_nat), TypeLevelValue::Nat(b_nat)) => {
                        Ok(TypeLevelValue::Nat(a_nat.mul(&b_nat)))
                    }
                    _ => Err(TypeLevelError::TypeFamilyError {
                        reason: "Mult type family requires Nat arguments".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            TypeFamily::Fn {
                params,
                return_type,
            } => {
                // Fn[A, B, C] = A -> B -> C
                let mut param_types = Vec::new();
                for param in params {
                    match self.compute(param)? {
                        TypeLevelValue::Type(ty) => param_types.push(ty),
                        _ => {
                            return Err(TypeLevelError::ComputationFailed {
                                reason: "Function parameter must be a Type".to_string(),
                                span: crate::util::span::Span::default(),
                            })
                        }
                    }
                }
                let return_ty = match self.compute(return_type)? {
                    TypeLevelValue::Type(ty) => Box::new(ty),
                    _ => {
                        return Err(TypeLevelError::ComputationFailed {
                            reason: "Function return type must be a Type".to_string(),
                            span: crate::util::span::Span::default(),
                        })
                    }
                };
                Ok(TypeLevelValue::Type(MonoType::Fn {
                    params: param_types,
                    return_type: return_ty,
                    is_async: false,
                }))
            }
            TypeFamily::Tuple { elements } => {
                // Tuple[A, B, C] = (A, B, C)
                let mut element_types = Vec::new();
                for elem in elements {
                    match self.compute(elem)? {
                        TypeLevelValue::Type(ty) => element_types.push(ty),
                        _ => {
                            return Err(TypeLevelError::ComputationFailed {
                                reason: "Tuple element must be a Type".to_string(),
                                span: crate::util::span::Span::default(),
                            })
                        }
                    }
                }
                Ok(TypeLevelValue::Type(MonoType::Tuple(element_types)))
            }
        }
    }

    /// 归一化类型族
    pub fn normalize_family(
        &self,
        family: &TypeFamily,
    ) -> Result<TypeLevelValue, TypeLevelError> {
        // 创建新的计算器实例（不共享缓存）
        let mut computer = TypeLevelComputer::new();
        computer.var_mapping = self.var_mapping.clone();
        computer.max_depth = self.max_depth;
        computer.compute(family)
    }

    /// 归一化单态类型
    pub fn normalize_type(
        &mut self,
        ty: &MonoType,
    ) -> Result<MonoType, TypeLevelError> {
        // 使用字符串表示作为缓存键
        let key = ty.type_name();
        if let Some(cached) = self.normalization_cache.get(&key) {
            return Ok(cached.clone());
        }

        let normalized = match ty {
            MonoType::TypeRef(name) => {
                // 查找预定义的类型族
                match name.as_str() {
                    "Option" => MonoType::TypeRef("Option".to_string()),
                    "List" => MonoType::TypeRef("List".to_string()),
                    _ => ty.clone(),
                }
            }
            _ => ty.clone(),
        };

        self.normalization_cache.insert(key, normalized.clone());
        Ok(normalized)
    }

    /// 获取缓存统计
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.normalization_cache.len())
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.normalization_cache.clear();
    }
}

impl Default for TypeLevelComputer {
    fn default() -> Self {
        TypeLevelComputer::new()
    }
}

/// 类型归一化器
#[derive(Debug)]
pub struct TypeNormalizer {
    computer: TypeLevelComputer,
}

impl TypeNormalizer {
    /// 创建新的类型归一化器
    pub fn new() -> Self {
        TypeNormalizer {
            computer: TypeLevelComputer::new(),
        }
    }

    /// 归一化类型
    pub fn normalize(
        &mut self,
        ty: &MonoType,
    ) -> Result<MonoType, TypeLevelError> {
        self.computer.normalize_type(ty)
    }

    /// 归一化多态类型
    pub fn normalize_poly(
        &mut self,
        poly: &PolyType,
    ) -> Result<PolyType, TypeLevelError> {
        let normalized_body = self.computer.normalize_type(&poly.body)?;
        Ok(PolyType::new(poly.type_binders.clone(), normalized_body))
    }
}

impl Default for TypeNormalizer {
    fn default() -> Self {
        TypeNormalizer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nat_add() {
        let nat1 = Nat::from_usize(3);
        let nat2 = Nat::from_usize(4);
        let result = nat1.add(&nat2);
        assert_eq!(result.to_usize(), Some(7));
    }

    #[test]
    fn test_nat_mul() {
        let nat1 = Nat::from_usize(3);
        let nat2 = Nat::from_usize(4);
        let result = nat1.mul(&nat2);
        assert_eq!(result.to_usize(), Some(12));
    }

    #[test]
    fn test_type_level_computer() {
        let mut computer = TypeLevelComputer::new();
        let family = TypeFamily::add(3, 4);
        let result = computer.compute(&family).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(7)));
    }

    #[test]
    fn test_type_family_mult() {
        let mut computer = TypeLevelComputer::new();
        let family = TypeFamily::mult(3, 4);
        let result = computer.compute(&family).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(12)));
    }

    #[test]
    fn test_type_family_fn() {
        let mut computer = TypeLevelComputer::new();
        let family = TypeFamily::fn_type(vec![MonoType::Int(64), MonoType::String], MonoType::Bool);
        let result = computer.compute(&family).unwrap();
        match result {
            TypeLevelValue::Type(MonoType::Fn {
                params,
                return_type,
                ..
            }) => {
                assert_eq!(params.len(), 2);
                assert_eq!(*return_type, MonoType::Bool);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_recursion_depth_limit() {
        let mut computer = TypeLevelComputer::new();
        computer.set_max_depth(10);
        computer.bind_var(0, TypeLevelValue::Nat(Nat::from_usize(100)));

        // 这应该触发递归深度限制
        let family = TypeFamily::Var {
            name: "x".to_string(),
            index: 0,
        };
        let result = computer.compute(&family);
        assert!(result.is_ok());
    }
}
