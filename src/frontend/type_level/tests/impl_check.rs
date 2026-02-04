//! Trait 实现检查器测试

use crate::frontend::core::parser::ast::{Param, Type, TraitImpl, MethodImpl};
use crate::frontend::type_level::impl_check::{TraitImplChecker, TraitImplError};
use crate::frontend::type_level::trait_bounds::{TraitDefinition, TraitMethodSignature, TraitTable};

fn create_trait_table_with_clone() -> TraitTable {
    let mut table = TraitTable::default();

    let trait_def = TraitDefinition {
        name: "Clone".to_string(),
        methods: vec![(
            "clone".to_string(),
            TraitMethodSignature {
                name: "clone".to_string(),
                params: vec![],
                return_type: crate::frontend::core::type_system::MonoType::TypeRef("Self".to_string()),
                is_static: false,
            },
        )]
        .into_iter()
        .collect(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    };

    table.add_trait(trait_def);
    table
}

#[test]
fn test_impl_has_required_method() {
    let table = create_trait_table_with_clone();
    let checker = TraitImplChecker::new(&table);

    let impl_ = TraitImpl {
        trait_name: "Clone".to_string(),
        for_type: Type::Name("Point".to_string()),
        methods: vec![MethodImpl {
            name: "clone".to_string(),
            params: vec![Param {
                name: "self".to_string(),
                ty: Some(Type::Name("Self".to_string())),
                span: crate::util::span::Span::dummy(),
            }],
            return_type: Some(Type::Name("Point".to_string())),
            body: (vec![], None),
            span: crate::util::span::Span::dummy(),
        }],
        span: crate::util::span::Span::dummy(),
    };

    assert!(checker.check_impl(&impl_).is_ok());
}

#[test]
fn test_impl_missing_method() {
    let table = create_trait_table_with_clone();
    let checker = TraitImplChecker::new(&table);

    let impl_ = TraitImpl {
        trait_name: "Clone".to_string(),
        for_type: Type::Name("Point".to_string()),
        methods: vec![MethodImpl {
            name: "other".to_string(),
            params: vec![],
            return_type: None,
            body: (vec![], None),
            span: crate::util::span::Span::dummy(),
        }],
        span: crate::util::span::Span::dummy(),
    };

    let result = checker.check_impl(&impl_);
    assert!(result.is_err());

    if let Err(TraitImplError::MissingMethod { methods, .. }) = result {
        assert_eq!(methods, vec!["clone"]);
    }
}

#[test]
fn test_trait_not_found() {
    let table = TraitTable::default();
    let checker = TraitImplChecker::new(&table);

    let impl_ = TraitImpl {
        trait_name: "UnknownTrait".to_string(),
        for_type: Type::Name("Point".to_string()),
        methods: vec![],
        span: crate::util::span::Span::dummy(),
    };

    let result = checker.check_impl(&impl_);
    assert!(result.is_err());
    assert!(matches!(result, Err(TraitImplError::TraitNotFound { .. })));
}
