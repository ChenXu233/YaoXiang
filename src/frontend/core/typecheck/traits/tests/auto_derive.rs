//! 自动派生测试 — 基于语言规范 §3.5.2 & §3.5.3 & RFC-011 §2
//!
//! §3.5.2: 标准库接口（Clone, Equal, Debug）
//! §3.5.3: 自动派生机制
//! RFC-011 §2: 类型约束系统

use crate::frontend::core::typecheck::traits::auto_derive::{
    is_builtin_derive, is_primitive_type, can_auto_derive, generate_auto_derive,
    field_type_satisfies, BUILTIN_DERIVES,
};
use crate::frontend::core::types::{TraitTable, TraitImplementation, MonoType};
use crate::frontend::core::parser::ast::{Type, StructField};
use crate::util::span::Span;

// ===================================================================
// 辅助函数
// ===================================================================

/// 创建用于测试的 Span（dummy 值）
fn dummy_span() -> Span {
    Span::dummy()
}

/// 创建一个 Name 类型字段
fn make_name_field(
    name: &str,
    type_name: &str,
) -> StructField {
    StructField::new(
        name.to_string(),
        false,
        Type::Name {
            name: type_name.to_string(),
            span: dummy_span(),
        },
    )
}

// ===================================================================
// Happy path 测试 — is_builtin_derive
// ===================================================================

#[test]
fn test_is_builtin_derive_clone() {
    // Arrange & Act
    let result = is_builtin_derive("Clone");

    // Assert
    assert!(result, "Clone should be builtin derive");
}

#[test]
fn test_is_builtin_derive_debug() {
    // Arrange & Act
    let result = is_builtin_derive("Debug");

    // Assert
    assert!(result, "Debug should be builtin derive");
}

#[test]
fn test_is_builtin_derive_equal() {
    // Arrange & Act
    let result = is_builtin_derive("Equal");

    // Assert
    assert!(
        result,
        "Equal should be builtin derive (规范 §3.5.2, 合并了 PartialEq + Eq)"
    );
}

// ===================================================================
// Happy path 测试 — is_primitive_type
// ===================================================================

#[test]
fn test_is_primitive_type_int() {
    // Arrange & Act
    let result = is_primitive_type("Int");

    // Assert
    assert!(result, "Int should be primitive type");
}

#[test]
fn test_is_primitive_type_float() {
    // Arrange & Act
    let result = is_primitive_type("Float");

    // Assert
    assert!(result, "Float should be primitive type");
}

#[test]
fn test_is_primitive_type_bool() {
    // Arrange & Act
    let result = is_primitive_type("Bool");

    // Assert
    assert!(result, "Bool should be primitive type");
}

#[test]
fn test_is_primitive_type_string() {
    // Arrange & Act
    let result = is_primitive_type("String");

    // Assert
    assert!(result, "String should be primitive type");
}

#[test]
fn test_is_primitive_type_void() {
    // Arrange & Act
    let result = is_primitive_type("Void");

    // Assert
    assert!(result, "Void should be primitive type");
}

#[test]
fn test_is_primitive_type_char() {
    // Arrange & Act
    let result = is_primitive_type("Char");

    // Assert
    assert!(result, "Char should be primitive type");
}

// ===================================================================
// Happy path 测试 — generate_auto_derive
// ===================================================================

#[test]
fn test_generate_auto_derive_clone_returns_some() {
    // Arrange & Act
    let result = generate_auto_derive("Point", "Clone");

    // Assert
    assert!(
        result.is_some(),
        "Clone derive should return Some for valid type"
    );
}

#[test]
fn test_generate_auto_derive_clone_has_correct_trait_name() {
    // Arrange
    let result = generate_auto_derive("Point", "Clone").unwrap();

    // Assert
    assert_eq!(result.trait_name, "Clone", "Trait name should be Clone");
}

#[test]
fn test_generate_auto_derive_clone_has_correct_for_type() {
    // Arrange
    let result = generate_auto_derive("Point", "Clone").unwrap();

    // Assert
    assert_eq!(
        result.for_type_name, "Point",
        "for_type_name should match input"
    );
}

#[test]
fn test_generate_auto_derive_clone_has_clone_method() {
    // Arrange
    let result = generate_auto_derive("Point", "Clone").unwrap();

    // Assert
    assert!(
        result.methods.contains_key("clone"),
        "Clone derive should have 'clone' method"
    );
}

#[test]
fn test_generate_auto_derive_debug_has_debug_method() {
    // Arrange
    let result = generate_auto_derive("Point", "Debug").unwrap();

    // Assert
    assert!(
        result.methods.contains_key("debug"),
        "Debug derive should have 'debug' method"
    );
}

#[test]
fn test_generate_auto_derive_equal_has_equal_method() {
    // Arrange - 规范 §3.5.2: Equal 接口方法为 equal: (Self, Self) -> Bool
    let result = generate_auto_derive("Point", "Equal").unwrap();

    // Assert
    assert!(
        result.methods.contains_key("equal"),
        "Equal derive should have 'equal' method (规范 §3.5.2)"
    );
}

// ===================================================================
// Happy path 测试 — can_auto_derive
// ===================================================================

#[test]
fn test_can_auto_derive_all_fields_implement_trait() {
    // Arrange
    let mut table = TraitTable::default();
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    let fields = vec![make_name_field("x", "Int")];

    // Act
    let result = can_auto_derive(&table, "Clone", &fields);

    // Assert
    assert!(
        result,
        "Should be able to auto-derive when all fields implement the trait"
    );
}

#[test]
fn test_can_auto_derive_empty_fields() {
    // Arrange
    let table = TraitTable::default();
    let fields: Vec<StructField> = vec![];

    // Act
    let result = can_auto_derive(&table, "Clone", &fields);

    // Assert
    assert!(
        result,
        "Empty struct should be auto-derivable for builtin trait"
    );
}

#[test]
fn test_can_auto_derive_multiple_fields_all_implement() {
    // Arrange
    let mut table = TraitTable::default();
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "String".to_string(),
        methods: Default::default(),
    });
    let fields = vec![make_name_field("x", "Int"), make_name_field("y", "String")];

    // Act
    let result = can_auto_derive(&table, "Clone", &fields);

    // Assert
    assert!(
        result,
        "Should auto-derive when all multiple fields implement the trait"
    );
}

// ===================================================================
// Error path 测试 — is_builtin_derive
// ===================================================================

#[test]
fn test_is_builtin_derive_display() {
    // Arrange & Act
    let result = is_builtin_derive("Display");

    // Assert
    assert!(!result, "Display should not be builtin derive");
}

#[test]
fn test_is_builtin_derive_unknown() {
    // Arrange & Act
    let result = is_builtin_derive("UnknownTrait");

    // Assert
    assert!(!result, "UnknownTrait should not be builtin derive");
}

#[test]
fn test_is_builtin_derive_empty_string() {
    // Arrange & Act
    let result = is_builtin_derive("");

    // Assert
    assert!(!result, "Empty string should not be builtin derive");
}

#[test]
fn test_is_builtin_derive_lowercase_clone() {
    // Arrange & Act
    let result = is_builtin_derive("clone");

    // Assert
    assert!(
        !result,
        "Lowercase 'clone' should not match builtin derive 'Clone'"
    );
}

// ===================================================================
// Error path 测试 — is_primitive_type
// ===================================================================

#[test]
fn test_is_primitive_type_custom() {
    // Arrange & Act
    let result = is_primitive_type("CustomType");

    // Assert
    assert!(!result, "CustomType should not be primitive type");
}

#[test]
fn test_is_primitive_type_empty_string() {
    // Arrange & Act
    let result = is_primitive_type("");

    // Assert
    assert!(!result, "Empty string should not be primitive type");
}

#[test]
fn test_is_primitive_type_lowercase_int() {
    // Arrange & Act
    let result = is_primitive_type("int");

    // Assert
    assert!(
        !result,
        "Lowercase 'int' should not match primitive type 'Int'"
    );
}

#[test]
fn test_is_primitive_type_array() {
    // Arrange & Act
    let result = is_primitive_type("Array");

    // Assert
    assert!(!result, "Array should not be primitive type");
}

// ===================================================================
// Error path 测试 — generate_auto_derive
// ===================================================================

#[test]
fn test_generate_auto_derive_unknown_trait_returns_none() {
    // Arrange & Act
    let result = generate_auto_derive("Point", "Hash");

    // Assert
    assert!(result.is_none(), "Unknown trait 'Hash' should return None");
}

#[test]
fn test_generate_auto_derive_empty_trait_name_returns_none() {
    // Arrange & Act
    let result = generate_auto_derive("Point", "");

    // Assert
    assert!(result.is_none(), "Empty trait name should return None");
}

// ===================================================================
// Error path 测试 — can_auto_derive
// ===================================================================

#[test]
fn test_can_auto_derive_non_builtin_trait_returns_false() {
    // Arrange
    let table = TraitTable::default();
    let fields = vec![make_name_field("x", "Int")];

    // Act
    let result = can_auto_derive(&table, "Hash", &fields);

    // Assert
    assert!(!result, "Non-builtin trait should not be auto-derivable");
}

#[test]
fn test_can_auto_derive_field_not_implementing_trait() {
    // Arrange
    let table = TraitTable::default(); // Int 没有注册 Clone 实现
    let fields = vec![make_name_field("x", "Int")];

    // Act
    let result = can_auto_derive(&table, "Clone", &fields);

    // Assert
    assert!(
        !result,
        "Should not auto-derive when field type lacks trait implementation"
    );
}

#[test]
fn test_can_auto_derive_partial_implementation() {
    // Arrange
    let mut table = TraitTable::default();
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    // String 没有 Clone 实现
    let fields = vec![make_name_field("x", "Int"), make_name_field("y", "String")];

    // Act
    let result = can_auto_derive(&table, "Clone", &fields);

    // Assert
    assert!(
        !result,
        "Should not auto-derive when only some fields implement the trait"
    );
}

#[test]
fn test_can_auto_derive_complex_type_field_returns_false() {
    // Arrange
    let table = TraitTable::default();
    let fields = vec![StructField::new(
        "data".to_string(),
        false,
        Type::Bool, // 非 Name 类型，走 _ => return false 分支
    )];

    // Act
    let result = can_auto_derive(&table, "Clone", &fields);

    // Assert
    assert!(
        !result,
        "Complex (non-Name) field type should not support auto-derive"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_builtin_derives_list_is_non_empty() {
    // Arrange & Act & Assert
    assert!(
        !BUILTIN_DERIVES.is_empty(),
        "BUILTIN_DERIVES should not be empty"
    );
}

#[test]
fn test_builtin_derives_contains_exactly_expected_traits() {
    // Arrange - 规范 §3.5.2: 标准库接口为 Clone, Equal, Debug
    let expected = ["Clone", "Equal", "Debug"];

    // Act & Assert
    assert_eq!(
        BUILTIN_DERIVES.len(),
        expected.len(),
        "BUILTIN_DERIVES should contain exactly 3 traits (规范 §3.5.2)"
    );
    for name in &expected {
        assert!(
            BUILTIN_DERIVES.contains(name),
            "BUILTIN_DERIVES should contain '{}'",
            name
        );
    }
}

#[test]
fn test_generate_auto_derive_clone_method_return_type() {
    // Arrange
    let result = generate_auto_derive("MyType", "Clone").unwrap();
    let clone_fn = result.methods.get("clone").unwrap();

    // Assert
    match clone_fn {
        MonoType::Fn { return_type, .. } => {
            assert_eq!(
                **return_type,
                MonoType::TypeRef("MyType".to_string()),
                "Clone method should return Self (MyType)"
            );
        }
        other => panic!("Expected MonoType::Fn for clone method, got {:?}", other),
    }
}

#[test]
fn test_generate_auto_derive_equal_method_return_type() {
    // Arrange - 规范 §3.5.2: equal: (Self, Self) -> Bool
    let result = generate_auto_derive("Point", "Equal").unwrap();
    let equal_fn = result.methods.get("equal").unwrap();

    // Assert
    match equal_fn {
        MonoType::Fn { return_type, .. } => {
            assert_eq!(
                **return_type,
                MonoType::TypeRef("Bool".to_string()),
                "Equal equal method should return TypeRef(\"Bool\") (规范 §3.5.2)"
            );
        }
        other => panic!("Expected MonoType::Fn for equal method, got {:?}", other),
    }
}

#[test]
fn test_generate_auto_derive_debug_method_param_count() {
    // Arrange
    let result = generate_auto_derive("Point", "Debug").unwrap();
    let debug_fn = result.methods.get("debug").unwrap();

    // Assert
    match debug_fn {
        MonoType::Fn { params, .. } => {
            assert_eq!(
                params.len(),
                2,
                "Debug debug method should have 2 params (self, formatter)"
            );
        }
        other => panic!("Expected MonoType::Fn for debug method, got {:?}", other),
    }
}

#[test]
fn test_generate_auto_derive_preserves_type_name_in_implementation() {
    // Arrange
    let type_names = ["Point", "UserRecord", "MyStruct"];

    // Act & Assert
    for type_name in &type_names {
        let result = generate_auto_derive(type_name, "Clone");
        assert!(
            result.is_some(),
            "generate_auto_derive should work for '{}'",
            type_name
        );
        assert_eq!(
            result.unwrap().for_type_name,
            *type_name,
            "for_type_name should match input type '{}'",
            type_name
        );
    }
}

#[test]
fn test_can_auto_derive_with_primitive_fields_registered() {
    // Arrange
    let trait_table = crate::frontend::core::types::TraitTable::default();
    let trait_name = "Clone";
    let fields = vec![];

    // Act
    let result = can_auto_derive(&trait_table, trait_name, &fields);

    // Assert
    assert!(
        result,
        "Empty fields with builtin trait should allow auto-derive"
    );
}

// ===================================================================
// field_type_satisfies 测试
// ===================================================================

#[test]
fn test_field_type_satisfies_name() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    let ty = Type::Name {
        name: "Int".to_string(),
        span: dummy_span(),
    };
    assert!(
        field_type_satisfies(&table, "Dup", &ty),
        "Int should satisfy Dup"
    );
}

#[test]
fn test_field_type_satisfies_name_fails() {
    let table = TraitTable::new();
    let ty = Type::Name {
        name: "Int".to_string(),
        span: dummy_span(),
    };
    assert!(
        !field_type_satisfies(&table, "Dup", &ty),
        "Int without impl should not satisfy Dup"
    );
}

#[test]
fn test_field_type_satisfies_generic() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "List".to_string(),
        methods: Default::default(),
    });
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    let ty = Type::Generic {
        name: "List".to_string(),
        name_span: dummy_span(),
        args: vec![Type::Name {
            name: "Int".to_string(),
            span: dummy_span(),
        }],
    };
    assert!(
        field_type_satisfies(&table, "Dup", &ty),
        "List(Int) should satisfy Dup when both satisfy"
    );
}

#[test]
fn test_field_type_satisfies_generic_container_fails() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    let ty = Type::Generic {
        name: "List".to_string(),
        name_span: dummy_span(),
        args: vec![Type::Name {
            name: "Int".to_string(),
            span: dummy_span(),
        }],
    };
    assert!(
        !field_type_satisfies(&table, "Dup", &ty),
        "List(Int) should fail when List doesn't satisfy"
    );
}

#[test]
fn test_field_type_satisfies_generic_arg_fails() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "List".to_string(),
        methods: Default::default(),
    });
    let ty = Type::Generic {
        name: "List".to_string(),
        name_span: dummy_span(),
        args: vec![Type::Name {
            name: "Buffer".to_string(),
            span: dummy_span(),
        }],
    };
    assert!(
        !field_type_satisfies(&table, "Dup", &ty),
        "List(Buffer) should fail when Buffer doesn't satisfy"
    );
}

#[test]
fn test_field_type_satisfies_tuple() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "Float".to_string(),
        methods: Default::default(),
    });
    let ty = Type::Tuple(vec![
        Type::Name {
            name: "Int".to_string(),
            span: dummy_span(),
        },
        Type::Name {
            name: "Float".to_string(),
            span: dummy_span(),
        },
    ]);
    assert!(
        field_type_satisfies(&table, "Dup", &ty),
        "Tuple of Dup types should satisfy"
    );
}

#[test]
fn test_field_type_satisfies_tuple_fails() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    let ty = Type::Tuple(vec![
        Type::Name {
            name: "Int".to_string(),
            span: dummy_span(),
        },
        Type::Name {
            name: "Buffer".to_string(),
            span: dummy_span(),
        },
    ]);
    assert!(
        !field_type_satisfies(&table, "Dup", &ty),
        "Tuple with non-Dup element should fail"
    );
}

#[test]
fn test_field_type_satisfies_fn_returns_false() {
    let table = TraitTable::new();
    let ty = Type::Fn {
        params: vec![],
        return_type: Box::new(Type::Name {
            name: "Void".to_string(),
            span: dummy_span(),
        }),
    };
    assert!(
        !field_type_satisfies(&table, "Dup", &ty),
        "Fn types should not satisfy Dup"
    );
}

#[test]
fn test_field_type_satisfies_builtin_int() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "Int".to_string(),
        methods: Default::default(),
    });
    let ty = Type::Int(64);
    assert!(
        field_type_satisfies(&table, "Dup", &ty),
        "Int(64) should satisfy Dup via builtin mapping"
    );
}

#[test]
fn test_field_type_satisfies_builtin_bool() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Dup".to_string(),
        for_type_name: "Bool".to_string(),
        methods: Default::default(),
    });
    let ty = Type::Bool;
    assert!(
        field_type_satisfies(&table, "Dup", &ty),
        "Bool should satisfy Dup via builtin mapping"
    );
}
