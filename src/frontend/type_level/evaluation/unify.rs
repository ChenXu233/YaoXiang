//! RFC-011 类型级统一
//!
//! 实现类型级统一算法，用于：
//! - 类型等价性检查
//! - 类型变量绑定
//! - 联合类型求解

use crate::frontend::core::type_system::{MonoType, TypeVar};
use crate::frontend::type_level::evaluation::ReductionConfig;
use std::collections::{HashMap, HashSet};

/// 统一结果
#[derive(Debug, Clone, PartialEq)]
pub enum UnificationResult {
    /// 统一成功，生成替换映射
    Success(Substitution),

    /// 统一失败
    Failure(String),

    /// 需要进一步归约
    NeedReduction(MonoType, MonoType),
}

/// 类型替换映射
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Substitution {
    /// 类型变量替换（使用索引）
    bindings: HashMap<usize, MonoType>,
}

impl Substitution {
    /// 创建新的替换
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// 添加绑定
    pub fn bind(
        &mut self,
        tv: TypeVar,
        ty: MonoType,
    ) {
        self.bindings.insert(tv.index(), ty);
    }

    /// 获取绑定
    pub fn get(
        &self,
        index: &usize,
    ) -> Option<&MonoType> {
        self.bindings.get(index)
    }

    /// 检查是否包含变量
    pub fn contains_var(
        &self,
        index: &usize,
    ) -> bool {
        self.bindings.contains_key(index)
    }

    /// 合并替换
    pub fn merge(
        &self,
        other: &Substitution,
    ) -> Substitution {
        let mut result = self.clone();
        for (k, v) in &other.bindings {
            result.bindings.insert(*k, v.clone());
        }
        result
    }

    /// 获取所有绑定的变量索引
    pub fn bound_vars(&self) -> HashSet<usize> {
        self.bindings.keys().cloned().collect()
    }
}

/// 类型级统一器
///
/// 执行类型表达式的统一
#[derive(Debug, Clone)]
pub struct TypeUnifier {
    /// 统一配置
    config: ReductionConfig,

    /// 当前替换
    substitution: Substitution,
}

impl Default for TypeUnifier {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeUnifier {
    /// 创建新的统一器
    pub fn new() -> Self {
        Self {
            config: ReductionConfig::default(),
            substitution: Substitution::new(),
        }
    }

    /// 创建带配置的统一器
    pub fn with_config(config: ReductionConfig) -> Self {
        Self {
            config,
            substitution: Substitution::new(),
        }
    }

    /// 统一两个类型
    pub fn unify(
        &mut self,
        ty1: &MonoType,
        ty2: &MonoType,
    ) -> UnificationResult {
        self.unify_internal(ty1, ty2)
    }

    /// 内部统一逻辑
    fn unify_internal(
        &mut self,
        ty1: &MonoType,
        ty2: &MonoType,
    ) -> UnificationResult {
        match (ty1, ty2) {
            // 两个类型变量
            (MonoType::TypeVar(tv1), MonoType::TypeVar(tv2)) => {
                if tv1.index() == tv2.index() {
                    UnificationResult::Success(self.substitution.clone())
                } else {
                    // 绑定 tv1 到 tv2
                    self.substitution.bind(*tv1, MonoType::TypeVar(*tv2));
                    UnificationResult::Success(self.substitution.clone())
                }
            }

            // 类型变量和具体类型
            (MonoType::TypeVar(tv), other) | (other, MonoType::TypeVar(tv)) => {
                self.substitution.bind(*tv, other.clone());
                UnificationResult::Success(self.substitution.clone())
            }

            // 基本类型
            _ if ty1 == ty2 => UnificationResult::Success(self.substitution.clone()),

            // 元组统一
            (MonoType::Tuple(types1), MonoType::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    return UnificationResult::Failure("Tuple arity mismatch".to_string());
                }
                let mut result = self.substitution.clone();
                for (t1, t2) in types1.iter().zip(types2.iter()) {
                    let arg_result = self.unify_with_substitution(t1, t2, &result);
                    match arg_result {
                        UnificationResult::Success(sub) => result = sub,
                        _ => return arg_result,
                    }
                }
                UnificationResult::Success(result)
            }

            // 列表统一
            (MonoType::List(t1), MonoType::List(t2)) => self.unify_internal(t1, t2),

            // 函数统一
            (
                MonoType::Fn {
                    params: params1,
                    return_type: ret1,
                    ..
                },
                MonoType::Fn {
                    params: params2,
                    return_type: ret2,
                    ..
                },
            ) => {
                if params1.len() != params2.len() {
                    return UnificationResult::Failure("Function arity mismatch".to_string());
                }
                let mut result = self.substitution.clone();
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let param_result = self.unify_with_substitution(p1, p2, &result);
                    match param_result {
                        UnificationResult::Success(sub) => result = sub,
                        _ => return param_result,
                    }
                }
                self.unify_with_substitution(ret1, ret2, &result)
            }

            // 其他情况失败
            _ => UnificationResult::Failure(format!("Cannot unify {:?} and {:?}", ty1, ty2)),
        }
    }

    /// 带替换的统一
    fn unify_with_substitution(
        &mut self,
        ty1: &MonoType,
        ty2: &MonoType,
        current: &Substitution,
    ) -> UnificationResult {
        // 应用当前替换
        let applied1 = self.apply_substitution(ty1, current);
        let applied2 = self.apply_substitution(ty2, current);

        self.unify_internal(&applied1, &applied2)
    }

    /// 应用替换到类型
    #[allow(clippy::only_used_in_recursion)]
    fn apply_substitution(
        &self,
        ty: &MonoType,
        sub: &Substitution,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(tv) => sub
                .get(&tv.index())
                .cloned()
                .unwrap_or(MonoType::TypeVar(*tv)),
            MonoType::Tuple(types) => MonoType::Tuple(
                types
                    .iter()
                    .map(|t| self.apply_substitution(t, sub))
                    .collect(),
            ),
            MonoType::List(t) => MonoType::List(Box::new(self.apply_substitution(t, sub))),
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(self.apply_substitution(k, sub)),
                Box::new(self.apply_substitution(v, sub)),
            ),
            MonoType::Set(t) => MonoType::Set(Box::new(self.apply_substitution(t, sub))),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|p| self.apply_substitution(p, sub))
                    .collect(),
                return_type: Box::new(self.apply_substitution(return_type, sub)),
                is_async: *is_async,
            },
            _ => ty.clone(),
        }
    }

    /// 获取当前替换
    pub fn substitution(&self) -> &Substitution {
        &self.substitution
    }

    /// 重置统一器
    pub fn reset(&mut self) {
        self.substitution = Substitution::new();
    }
}
