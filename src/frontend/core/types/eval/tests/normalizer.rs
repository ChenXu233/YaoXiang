//! 类型范式化器测试
//!
//! TypeNormalizer: 范式化
//! NormalizationContext: 替换上下文
//! ReductionConfig: 归约配置

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::normalizer::{
    EvaluationStrategy, NormalForm, NormalizationContext, ReductionConfig, TypeNormalizer,
};

// ReductionConfig

#[test]
fn test_reduction_config_default() {
    let c = ReductionConfig::default();
    assert!(c.enable_delta);
    assert!(c.enable_iota);
    assert_eq!(c.max_steps, 1000);
}

#[test]
fn test_reduction_config_custom() {
    let config = ReductionConfig {
        enable_delta: false,
        enable_iota: false,
        max_steps: 50,
        evaluation_strategy: EvaluationStrategy::Lazy,
    };
    assert!(!config.enable_delta);
    assert!(!config.enable_iota);
    assert_eq!(config.max_steps, 50);
    assert_eq!(config.evaluation_strategy, EvaluationStrategy::Lazy);
}

// TypeNormalizer

#[test]
fn test_normalizer_normalize_primitives() {
    let mut n = TypeNormalizer::new();
    assert_eq!(n.normalize(&MonoType::Int(32)), NormalForm::Normalized);
    assert_eq!(n.normalize(&MonoType::Bool), NormalForm::Normalized);
    assert_eq!(
        n.normalize(&MonoType::make_string()),
        NormalForm::Normalized
    );
    assert_eq!(n.normalize(&MonoType::Void), NormalForm::Normalized);
}

// Context 相关

#[test]
fn test_normalizer_context() {
    let n = TypeNormalizer::new();
    let _ctx = n.context();
}

// NormalizationContext

#[test]
fn test_normalization_context_apply_no_change() {
    let ctx = NormalizationContext::new();
    assert_eq!(
        ctx.apply_substitution(&MonoType::Int(32)),
        MonoType::Int(32)
    );
}

#[test]
fn test_normalization_context_substitute_var() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Int(32));
    assert_eq!(
        ctx.apply_substitution(&MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(0)
        )),
        MonoType::Int(32)
    );
}

#[test]
fn test_normalization_context_add_substitutions() {
    let mut ctx = NormalizationContext::new();
    let mut subs = std::collections::HashMap::new();
    subs.insert(0, MonoType::Int(32));
    subs.insert(1, MonoType::make_string());
    ctx.add_substitutions(subs);
    assert_eq!(
        ctx.apply_substitution(&MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(0)
        )),
        MonoType::Int(32)
    );
    assert_eq!(
        ctx.apply_substitution(&MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(1)
        )),
        MonoType::make_string()
    );
}

#[test]
fn test_normalization_context_cache_mut() {
    let mut ctx = NormalizationContext::new();
    let cache = ctx.cache_mut();
    assert!(cache.is_empty());
}

#[test]
fn test_normalization_context_cache_ref() {
    let ctx = NormalizationContext::new();
    let cache = ctx.cache();
    assert!(cache.is_empty());
}
