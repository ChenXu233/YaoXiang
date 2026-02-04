//! RFC-011 类型范式化
//!
//! 实现类型范式化算法，将类型表达式转换为标准形式。
//!
//! 范式化是类型级计算的基础，确保：
//! - 类型表达式的唯一表示
//! - 冗余结构被消除
//! - 嵌套类型被扁平化
//!
//! 复用 core/type_system/substitute.rs 中的公共替换实现

use crate::frontend::core::type_system::{MonoType, Substitution, Substituter};
use crate::frontend::type_level::evaluation::{NormalForm, ReductionConfig};
use std::collections::HashMap;

/// 范式化上下文
#[derive(Debug, Clone, Default)]
pub struct NormalizationContext {
    /// 类型替换映射
    substitutions: Substitution,

    /// 替换器实例
    substituter: Substituter,

    /// 范式化缓存
    cache: HashMap<MonoType, NormalForm>,
}

impl NormalizationContext {
    /// 创建新的范式化上下文
    pub fn new() -> Self {
        Self {
            substitutions: Substitution::new(),
            substituter: Substituter::new(),
            cache: HashMap::new(),
        }
    }

    /// 添加类型变量替换
    pub fn add_substitution(
        &mut self,
        index: usize,
        ty: MonoType,
    ) {
        self.substitutions.insert(index, ty);
    }

    /// 批量添加替换
    pub fn add_substitutions(
        &mut self,
        subs: HashMap<usize, MonoType>,
    ) {
        for (k, v) in subs {
            self.substitutions.insert(k, v);
        }
    }

    /// 应用替换到类型
    pub fn apply_substitution(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        self.substituter.substitute(ty, &self.substitutions)
    }
}

/// 类型范式化器
///
/// 将类型表达式转换为范式形式
#[derive(Debug, Clone)]
pub struct TypeNormalizer {
    /// 范式化配置
    config: ReductionConfig,

    /// 上下文
    context: NormalizationContext,
}

impl Default for TypeNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeNormalizer {
    /// 创建新的范式化器
    pub fn new() -> Self {
        Self {
            config: ReductionConfig::default(),
            context: NormalizationContext::new(),
        }
    }

    /// 创建带配置的范式化器
    pub fn with_config(config: ReductionConfig) -> Self {
        Self {
            config,
            context: NormalizationContext::new(),
        }
    }

    /// 范式化类型
    pub fn normalize(
        &mut self,
        ty: &MonoType,
    ) -> NormalForm {
        // 检查缓存
        if let Some(cached) = self.context.cache.get(ty).cloned() {
            return cached;
        }

        let result = self.normalize_internal(ty);

        // 缓存结果
        self.context.cache.insert(ty.clone(), result.clone());

        result
    }

    /// 内部范式化逻辑
    fn normalize_internal(
        &mut self,
        ty: &MonoType,
    ) -> NormalForm {
        match ty {
            // 基本类型已经是范式
            MonoType::Void
            | MonoType::Bool
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => NormalForm::Normalized,
            MonoType::Int(_) | MonoType::Float(_) => NormalForm::Normalized,
            MonoType::TypeVar(_) => NormalForm::NeedsReduction,
            MonoType::TypeRef(_) => NormalForm::Normalized,
            MonoType::Struct(_) | MonoType::Enum(_) => NormalForm::Normalized,
            MonoType::Tuple(types) => {
                if types
                    .iter()
                    .all(|t| matches!(self.normalize(t), NormalForm::Normalized))
                {
                    NormalForm::Normalized
                } else {
                    NormalForm::NeedsReduction
                }
            }
            MonoType::List(t) => {
                if matches!(self.normalize(t), NormalForm::Normalized) {
                    NormalForm::Normalized
                } else {
                    NormalForm::NeedsReduction
                }
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                let params_normalized = params
                    .iter()
                    .all(|p| matches!(self.normalize(p), NormalForm::Normalized));
                let ret_normalized = matches!(self.normalize(return_type), NormalForm::Normalized);

                if params_normalized && ret_normalized {
                    NormalForm::Normalized
                } else {
                    NormalForm::NeedsReduction
                }
            }
            _ => NormalForm::Normalized,
        }
    }

    /// 获取上下文
    pub fn context(&self) -> &NormalizationContext {
        &self.context
    }
}
