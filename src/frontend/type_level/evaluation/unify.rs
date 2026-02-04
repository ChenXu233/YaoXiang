//! RFC-011 类型级统一
//!
//! 实现类型级统一算法，用于：
//! - 类型等价性检查
//! - 类型变量绑定
//! - 联合类型求解
//!
//! 复用 core/type_system/substitute.rs 中的公共替换实现

use crate::frontend::core::type_system::{MonoType, Substitution, Substituter};
use crate::frontend::type_level::evaluation::ReductionConfig;

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

/// 类型级统一器
///
/// 执行类型表达式的统一，复用 Substituter 进行替换操作
#[derive(Debug, Clone)]
pub struct TypeUnifier {
    /// 统一配置
    config: ReductionConfig,

    /// 当前替换
    substitution: Substitution,

    /// 替换器实例
    substituter: Substituter,
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
            substituter: Substituter::new(),
        }
    }

    /// 创建带配置的统一器
    pub fn with_config(config: ReductionConfig) -> Self {
        Self {
            config,
            substitution: Substitution::new(),
            substituter: Substituter::new(),
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
        let applied1 = self.substituter.substitute(ty1, current);
        let applied2 = self.substituter.substitute(ty2, current);

        self.unify_internal(&applied1, &applied2)
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
