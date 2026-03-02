//! Trait 继承测试

use crate::frontend::core::parser::ast::Type;
use crate::frontend::type_level::inheritance::InheritanceChecker;
use crate::util::span::Span;

#[test]
fn test_simple_inheritance() {
    let mut checker = InheritanceChecker::new();
    checker.register_trait("Clone");
    checker.register_trait("PartialEq");

    // type Eq = PartialEq {}
    checker.add_trait(
        "Eq",
        &[Type::Name {
            name: "PartialEq".to_string(),
            span: Span::dummy(),
        }],
    );

    assert!(checker.validate().is_ok());
}

#[test]
fn test_multiple_inheritance() {
    let mut checker = InheritanceChecker::new();
    checker.register_trait("Clone");
    checker.register_trait("PartialEq");

    // type Eq = Clone + PartialEq {}
    checker.add_trait(
        "Eq",
        &[
            Type::Name {
                name: "Clone".to_string(),
                span: Span::dummy(),
            },
            Type::Name {
                name: "PartialEq".to_string(),
                span: Span::dummy(),
            },
        ],
    );

    assert!(checker.validate().is_ok());
}

#[test]
fn test_cycle_detection() {
    let mut checker = InheritanceChecker::new();

    // A extends B, B extends C, C extends A
    checker.add_trait(
        "A",
        &[Type::Name {
            name: "B".to_string(),
            span: Span::dummy(),
        }],
    );
    checker.add_trait(
        "B",
        &[Type::Name {
            name: "C".to_string(),
            span: Span::dummy(),
        }],
    );
    checker.add_trait(
        "C",
        &[Type::Name {
            name: "A".to_string(),
            span: Span::dummy(),
        }],
    );

    assert!(checker.validate().is_err());
}

#[test]
fn test_undefined_parent() {
    let mut checker = InheritanceChecker::new();
    checker.register_trait("A");

    // B extends Unknown
    checker.add_trait(
        "B",
        &[Type::Name {
            name: "Unknown".to_string(),
            span: Span::dummy(),
        }],
    );

    assert!(checker.validate().is_err());
}
