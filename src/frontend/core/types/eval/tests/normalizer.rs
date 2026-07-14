//! 类型范式化器测试
//!
//! TypeNormalizer: 范式化
//! NormalizationContext: 替换上下文
//! ReductionConfig: 归约配置

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::normalizer::{
    EvaluationStrategy, NormalForm, NormalizationContext, ReductionConfig, TypeNormalizer,
};

// ===================================================================
// ReductionConfig
// ===================================================================

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

// ===================================================================
// TypeNormalizer
// ===================================================================

#[test]
fn test_normalizer_normalize_primitives() {
    let mut n = TypeNormalizer::new();
    assert_eq!(n.normalize(&MonoType::Int(32)), NormalForm::Normalized);
    assert_eq!(n.normalize(&MonoType::Bool), NormalForm::Normalized);
    assert_eq!(n.normalize(&MonoType::String), NormalForm::Normalized);
    assert_eq!(n.normalize(&MonoType::Void), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_composite() {
    let mut n = TypeNormalizer::new();
    let list = MonoType::List(Box::new(MonoType::Int(32)));
    assert_eq!(n.normalize(&list), NormalForm::Normalized);

    let tuple = MonoType::Tuple(vec![MonoType::Int(32), MonoType::Bool]);
    assert_eq!(n.normalize(&tuple), NormalForm::Normalized);

    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::Bool),
    };
    assert_eq!(n.normalize(&fn_type), NormalForm::Normalized);
}

#[test]
fn test_normalizer_with_config() {
    let mut n = TypeNormalizer::new();
    // Normalizer should work with default config
    let _ = n.normalize(&MonoType::Int(32));
}

#[test]
fn test_normalizer_normalize_struct() {
    let mut n = TypeNormalizer::new();
    let s = MonoType::Struct(crate::frontend::core::types::StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Float(64))],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
        constraints: Vec::new(),
    });
    assert_eq!(n.normalize(&s), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_dict() {
    let mut n = TypeNormalizer::new();
    let dict = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)));
    assert_eq!(n.normalize(&dict), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_set() {
    let mut n = TypeNormalizer::new();
    let set = MonoType::Set(Box::new(MonoType::Bool));
    assert_eq!(n.normalize(&set), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_range() {
    let mut n = TypeNormalizer::new();
    let range = MonoType::Range {
        elem_type: Box::new(MonoType::Int(64)),
    };
    assert_eq!(n.normalize(&range), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_arc_weak() {
    let mut n = TypeNormalizer::new();
    let arc = MonoType::Arc(Box::new(MonoType::Int(32)));
    assert_eq!(n.normalize(&arc), NormalForm::Normalized);
    let weak = MonoType::Weak(Box::new(MonoType::String));
    assert_eq!(n.normalize(&weak), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_union_intersection() {
    let mut n = TypeNormalizer::new();
    let union = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    assert_eq!(n.normalize(&union), NormalForm::Normalized);
    let inter = MonoType::Intersection(vec![
        MonoType::TypeRef("Clone".to_string()),
        MonoType::TypeRef("Display".to_string()),
    ]);
    assert_eq!(n.normalize(&inter), NormalForm::Normalized);
}

// ===================================================================
// TypeNormalizer - normalize_internal 路径
// ===================================================================

#[test]
fn test_normalizer_normalize_type_var() {
    let mut n = TypeNormalizer::new();
    let tv = MonoType::TypeVar(crate::frontend::core::types::TypeVar::new(0));
    assert_eq!(n.normalize(&tv), NormalForm::NeedsReduction);
}

#[test]
fn test_normalizer_normalize_type_ref() {
    let mut n = TypeNormalizer::new();
    let ty = MonoType::TypeRef("MyType".to_string());
    assert_eq!(n.normalize(&ty), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_tuple_elements() {
    let mut n = TypeNormalizer::new();
    // All normalized elements
    let tuple = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    assert_eq!(n.normalize(&tuple), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_list_element() {
    let mut n = TypeNormalizer::new();
    let list = MonoType::List(Box::new(MonoType::Int(32)));
    assert_eq!(n.normalize(&list), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_fn_params() {
    let mut n = TypeNormalizer::new();
    let f = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::String),
    };
    assert_eq!(n.normalize(&f), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_enum() {
    let mut n = TypeNormalizer::new();
    let e = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string()],
    });
    assert_eq!(n.normalize(&e), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_option() {
    let mut n = TypeNormalizer::new();
    let opt = MonoType::Option(Box::new(MonoType::Int(32)));
    assert_eq!(n.normalize(&opt), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_result() {
    let mut n = TypeNormalizer::new();
    let res = MonoType::Result(Box::new(MonoType::Int(32)), Box::new(MonoType::String));
    assert_eq!(n.normalize(&res), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_dict_type() {
    let mut n = TypeNormalizer::new();
    let dict = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)));
    assert_eq!(n.normalize(&dict), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_set_type() {
    let mut n = TypeNormalizer::new();
    let set = MonoType::Set(Box::new(MonoType::Bool));
    assert_eq!(n.normalize(&set), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_range_type() {
    let mut n = TypeNormalizer::new();
    let range = MonoType::Range {
        elem_type: Box::new(MonoType::Int(64)),
    };
    assert_eq!(n.normalize(&range), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_arc_type() {
    let mut n = TypeNormalizer::new();
    let arc = MonoType::Arc(Box::new(MonoType::Int(32)));
    assert_eq!(n.normalize(&arc), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_weak_type() {
    let mut n = TypeNormalizer::new();
    let weak = MonoType::Weak(Box::new(MonoType::String));
    assert_eq!(n.normalize(&weak), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_union_type() {
    let mut n = TypeNormalizer::new();
    let union = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    assert_eq!(n.normalize(&union), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_intersection_type() {
    let mut n = TypeNormalizer::new();
    let inter = MonoType::Intersection(vec![
        MonoType::TypeRef("Clone".to_string()),
        MonoType::TypeRef("Display".to_string()),
    ]);
    assert_eq!(n.normalize(&inter), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_assoc_type() {
    let mut n = TypeNormalizer::new();
    let assoc = MonoType::AssocType {
        host_type: Box::new(MonoType::TypeRef("Iter".to_string())),
        assoc_name: "Item".to_string(),
        assoc_args: vec![],
    };
    assert_eq!(n.normalize(&assoc), NormalForm::Normalized);
}

#[test]
fn test_normalizer_normalize_meta_type() {
    let mut n = TypeNormalizer::new();
    let meta = MonoType::MetaType {
        universe_level: crate::frontend::core::types::UniverseLevel::type0(),
        type_params: vec![],
    };
    assert_eq!(n.normalize(&meta), NormalForm::Normalized);
}

// ===================================================================
// Evaluator 相关
// ===================================================================

#[test]
fn test_normalizer_evaluator() {
    let mut n = TypeNormalizer::new();
    let _evaluator = n.evaluator();
}

#[test]
fn test_normalizer_context() {
    let n = TypeNormalizer::new();
    let _ctx = n.context();
}

// ===================================================================
// NormalizationContext
// ===================================================================

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
fn test_normalization_context_substitute_unbound_var() {
    let ctx = NormalizationContext::new();
    assert_eq!(
        ctx.apply_substitution(&MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(99)
        )),
        MonoType::TypeVar(crate::frontend::core::types::TypeVar::new(99))
    );
}

#[test]
fn test_normalization_context_cache() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Bool);
    let result = ctx.apply_substitution(&MonoType::TypeVar(
        crate::frontend::core::types::TypeVar::new(0),
    ));
    assert_eq!(result, MonoType::Bool);
}

#[test]
fn test_normalization_context_add_substitutions() {
    let mut ctx = NormalizationContext::new();
    let mut subs = std::collections::HashMap::new();
    subs.insert(0, MonoType::Int(32));
    subs.insert(1, MonoType::String);
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
        MonoType::String
    );
}

#[test]
fn test_normalization_context_apply_through_struct() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Int(32));
    let s = MonoType::Struct(crate::frontend::core::types::StructType {
        name: "Wrapper".to_string(),
        fields: vec![(
            "v".to_string(),
            MonoType::TypeVar(crate::frontend::core::types::TypeVar::new(0)),
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
        constraints: Vec::new(),
    });
    let result = ctx.apply_substitution(&s);
    match result {
        MonoType::Struct(ref st) => assert_eq!(st.fields[0].1, MonoType::Int(32)),
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn test_normalization_context_apply_through_tuple() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Bool);
    let tuple = MonoType::Tuple(vec![
        MonoType::TypeVar(crate::frontend::core::types::TypeVar::new(0)),
        MonoType::Int(32),
    ]);
    let result = ctx.apply_substitution(&tuple);
    assert_eq!(
        result,
        MonoType::Tuple(vec![MonoType::Bool, MonoType::Int(32)])
    );
}

#[test]
fn test_normalization_context_apply_through_fn() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::String);
    let f = MonoType::Fn {
        params: vec![MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(0),
        )],
        return_type: Box::new(MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(0),
        )),
    };
    let result = ctx.apply_substitution(&f);
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params[0], MonoType::String);
            assert_eq!(*return_type, MonoType::String);
        }
        _ => panic!("Expected Fn"),
    }
}

#[test]
fn test_normalization_context_apply_through_list() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Float(64));
    let list = MonoType::List(Box::new(MonoType::TypeVar(
        crate::frontend::core::types::TypeVar::new(0),
    )));
    let result = ctx.apply_substitution(&list);
    assert_eq!(result, MonoType::List(Box::new(MonoType::Float(64))));
}

#[test]
fn test_normalization_context_apply_through_dict() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::String);
    let dict = MonoType::Dict(
        Box::new(MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(0),
        )),
        Box::new(MonoType::Int(32)),
    );
    let result = ctx.apply_substitution(&dict);
    assert_eq!(
        result,
        MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)))
    );
}

#[test]
fn test_normalization_context_apply_through_arc() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Bool);
    let arc = MonoType::Arc(Box::new(MonoType::TypeVar(
        crate::frontend::core::types::TypeVar::new(0),
    )));
    let result = ctx.apply_substitution(&arc);
    assert_eq!(result, MonoType::Arc(Box::new(MonoType::Bool)));
}

#[test]
fn test_normalization_context_apply_through_union() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Int(64));
    let union = MonoType::Union(vec![
        MonoType::TypeVar(crate::frontend::core::types::TypeVar::new(0)),
        MonoType::String,
    ]);
    let result = ctx.apply_substitution(&union);
    assert_eq!(
        result,
        MonoType::Union(vec![MonoType::Int(64), MonoType::String])
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

#[test]
fn test_normalization_context_apply_through_weak() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Bool);
    let weak = MonoType::Weak(Box::new(MonoType::TypeVar(
        crate::frontend::core::types::TypeVar::new(0),
    )));
    let result = ctx.apply_substitution(&weak);
    assert_eq!(result, MonoType::Weak(Box::new(MonoType::Bool)));
}

#[test]
fn test_normalization_context_apply_through_set() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Int(32));
    let set = MonoType::Set(Box::new(MonoType::TypeVar(
        crate::frontend::core::types::TypeVar::new(0),
    )));
    let result = ctx.apply_substitution(&set);
    assert_eq!(result, MonoType::Set(Box::new(MonoType::Int(32))));
}

#[test]
fn test_normalization_context_apply_through_range() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Float(64));
    let range = MonoType::Range {
        elem_type: Box::new(MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(0),
        )),
    };
    let result = ctx.apply_substitution(&range);
    assert_eq!(
        result,
        MonoType::Range {
            elem_type: Box::new(MonoType::Float(64))
        }
    );
}

#[test]
fn test_normalization_context_apply_through_intersection() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::TypeRef("Clone".to_string()));
    let inter = MonoType::Intersection(vec![
        MonoType::TypeVar(crate::frontend::core::types::TypeVar::new(0)),
        MonoType::TypeRef("Display".to_string()),
    ]);
    let result = ctx.apply_substitution(&inter);
    assert_eq!(
        result,
        MonoType::Intersection(vec![
            MonoType::TypeRef("Clone".to_string()),
            MonoType::TypeRef("Display".to_string()),
        ])
    );
}

#[test]
fn test_normalization_context_apply_through_enum() {
    let mut ctx = NormalizationContext::new();
    ctx.add_substitution(0, MonoType::Int(32));
    let e = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string()],
    });
    // Enum has no type vars, so should return same
    let result = ctx.apply_substitution(&e);
    assert_eq!(result, e);
}
