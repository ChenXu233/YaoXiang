//! 平台特化器单元测试
//!
//! 测试 PlatformConstraint、PlatformSpecializer 的平台约束匹配和特化选择功能。

use crate::frontend::core::parser::ast::{GenericParam, GenericParamKind, Type};
use crate::middle::passes::mono::platform_info::{PlatformDetector, TargetPlatform};
use crate::middle::passes::mono::platform_specializer::{
    FunctionPlatformInfo, PlatformConstraint, PlatformSpecializer,
};
use crate::util::span::Span;

#[test]
fn test_platform_constraint_matches() {
    let constraint = PlatformConstraint::new("X86_64".to_string());

    assert!(constraint.matches(&TargetPlatform::X86_64));
    assert!(!constraint.matches(&TargetPlatform::AArch64));
}

#[test]
fn test_wildcard_constraint() {
    let constraint = PlatformConstraint::any();

    assert!(constraint.is_any());
    assert!(constraint.matches(&TargetPlatform::X86_64));
    assert!(constraint.matches(&TargetPlatform::AArch64));
}

#[test]
fn test_extract_platform_constraint() {
    let param_p = GenericParam {
        name: "P".to_string(),
        kind: GenericParamKind::Type,
        constraints: vec![Type::Name {
            name: "X86_64".to_string(),
            span: Span::dummy(),
        }],
    };

    let param_t = GenericParam {
        name: "T".to_string(),
        kind: GenericParamKind::Type,
        constraints: vec![Type::Name {
            name: "Clone".to_string(),
            span: Span::dummy(),
        }],
    };

    let params = vec![param_p.clone(), param_t.clone()];

    let (constraint, filtered) = PlatformSpecializer::extract_platform_constraint(&params);

    assert!(constraint.is_some());
    assert_eq!(constraint.unwrap().platform_type(), "X86_64");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "T");
}

#[test]
fn test_select_specialization() {
    let platform_info = PlatformDetector::detect_from_target("x86_64-linux-gnu");
    let mut specializer = PlatformSpecializer::new(platform_info);

    // 注册两个特化版本
    specializer.register_function(FunctionPlatformInfo::new(
        "sum".to_string(),
        vec![],
        Some(PlatformConstraint::new("X86_64".to_string())),
    ));

    specializer.register_function(FunctionPlatformInfo::new(
        "sum".to_string(),
        vec![],
        Some(PlatformConstraint::new("AArch64".to_string())),
    ));

    // 在 X86_64 平台上，应该选择 X86_64 版本
    let selected = specializer.select_specialization("sum", &[]);
    assert!(selected.is_some());
    assert_eq!(
        selected
            .unwrap()
            .platform_constraint()
            .unwrap()
            .platform_type(),
        "X86_64"
    );
}
