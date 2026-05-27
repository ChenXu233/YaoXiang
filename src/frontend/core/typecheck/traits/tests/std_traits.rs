//! 标准库 trait 测试 — 基于语言规范 §3.5.2 & RFC-011 §2
//!
//! §3.5.2: 标准库接口（Clone, Equal, Debug, Send, Sync, Iterator）
//! RFC-011 §2: 标准库 trait 定义
//!
//! 规范 v1.9.0 原类型实现表：
//! - Int, Float, Bool, Char, String: Clone, Equal, Debug, Send, Sync
//! - Void: Equal, Debug, Send, Sync（不实现 Clone）
//! - Bytes: Debug, Send, Sync（不实现 Clone, Equal）

use crate::frontend::core::typecheck::traits::std_traits::{
    init_std_traits, init_primitive_impls, is_primitive_type, std_trait_names, STD_TRAITS,
};
use crate::frontend::core::types::base::{MonoType, TraitTable};

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_init_std_traits_registers_clone() {
    // Arrange
    let mut trait_table = TraitTable::default();

    // Act
    init_std_traits(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_trait("Clone"),
        "Clone trait 应在标准库初始化后存在"
    );
    let clone = trait_table.get_trait("Clone").unwrap();
    assert!(
        clone.methods.contains_key("clone"),
        "Clone trait 应包含 clone 方法"
    );
}

#[test]
fn test_init_std_traits_registers_equal() {
    // Arrange - 规范 §3.5.2: Equal 合并了 PartialEq + Eq
    let mut trait_table = TraitTable::default();

    // Act
    init_std_traits(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_trait("Equal"),
        "Equal trait 应在标准库初始化后存在（规范 §3.5.2）"
    );
    let equal = trait_table.get_trait("Equal").unwrap();
    assert!(
        equal.methods.contains_key("equal"),
        "Equal trait 应包含 equal 方法"
    );
}

#[test]
fn test_init_std_traits_registers_debug() {
    // Arrange
    let mut trait_table = TraitTable::default();

    // Act
    init_std_traits(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_trait("Debug"),
        "Debug trait 应在标准库初始化后存在"
    );
    let debug = trait_table.get_trait("Debug").unwrap();
    assert!(
        debug.methods.contains_key("debug"),
        "Debug trait 应包含 debug 方法（规范 §3.5.2）"
    );
}

#[test]
fn test_init_std_traits_registers_send() {
    // Arrange - 规范 §3.5.2: Send 是标记接口，无方法
    let mut trait_table = TraitTable::default();

    // Act
    init_std_traits(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_trait("Send"),
        "Send trait 应在标准库初始化后存在（规范 §3.5.2）"
    );
    let send = trait_table.get_trait("Send").unwrap();
    assert!(send.methods.is_empty(), "Send 是标记接口，不应包含任何方法");
}

#[test]
fn test_init_std_traits_registers_sync() {
    // Arrange - 规范 §3.5.2: Sync 是标记接口，无方法
    let mut trait_table = TraitTable::default();

    // Act
    init_std_traits(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_trait("Sync"),
        "Sync trait 应在标准库初始化后存在（规范 §3.5.2）"
    );
    let sync = trait_table.get_trait("Sync").unwrap();
    assert!(sync.methods.is_empty(), "Sync 是标记接口，不应包含任何方法");
}

#[test]
fn test_init_std_traits_registers_iterator() {
    // Arrange - 规范 §3.5.2: Iterator 接口方法为 next: (Self) -> Option(Item)
    let mut trait_table = TraitTable::default();

    // Act
    init_std_traits(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_trait("Iterator"),
        "Iterator trait 应在标准库初始化后存在"
    );
    let iterator = trait_table.get_trait("Iterator").unwrap();
    assert!(
        iterator.methods.contains_key("next"),
        "Iterator trait 应包含 next 方法"
    );
}

#[test]
fn test_init_primitive_impls_int_clone() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_impl("Clone", "Int"),
        "Int 应实现 Clone trait"
    );
}

#[test]
fn test_init_primitive_impls_int_equal() {
    // Arrange - 规范 §3.5.2: Int 实现 Equal
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_impl("Equal", "Int"),
        "Int 应实现 Equal trait（规范 §3.5.2）"
    );
}

#[test]
fn test_init_primitive_impls_int_debug() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_impl("Debug", "Int"),
        "Int 应实现 Debug trait"
    );
}

#[test]
fn test_init_primitive_impls_int_send() {
    // Arrange - 规范 §3.5.2: 所有原类型自动实现 Send + Sync
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_impl("Send", "Int"),
        "Int 应实现 Send trait（规范 §3.5.2）"
    );
}

#[test]
fn test_init_primitive_impls_int_sync() {
    // Arrange - 规范 §3.5.2: 所有原类型自动实现 Send + Sync
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_impl("Sync", "Int"),
        "Int 应实现 Sync trait（规范 §3.5.2）"
    );
}

#[test]
fn test_init_primitive_impls_float_traits() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(trait_table.has_impl("Clone", "Float"), "Float 应实现 Clone");
    assert!(trait_table.has_impl("Equal", "Float"), "Float 应实现 Equal");
    assert!(trait_table.has_impl("Debug", "Float"), "Float 应实现 Debug");
    assert!(trait_table.has_impl("Send", "Float"), "Float 应实现 Send");
    assert!(trait_table.has_impl("Sync", "Float"), "Float 应实现 Sync");
}

#[test]
fn test_init_primitive_impls_bool_traits() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(trait_table.has_impl("Clone", "Bool"), "Bool 应实现 Clone");
    assert!(trait_table.has_impl("Equal", "Bool"), "Bool 应实现 Equal");
    assert!(trait_table.has_impl("Debug", "Bool"), "Bool 应实现 Debug");
    assert!(trait_table.has_impl("Send", "Bool"), "Bool 应实现 Send");
    assert!(trait_table.has_impl("Sync", "Bool"), "Bool 应实现 Sync");
}

#[test]
fn test_init_primitive_impls_string_traits() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_impl("Clone", "String"),
        "String 应实现 Clone"
    );
    assert!(
        trait_table.has_impl("Equal", "String"),
        "String 应实现 Equal"
    );
    assert!(
        trait_table.has_impl("Debug", "String"),
        "String 应实现 Debug"
    );
    assert!(trait_table.has_impl("Send", "String"), "String 应实现 Send");
    assert!(trait_table.has_impl("Sync", "String"), "String 应实现 Sync");
}

#[test]
fn test_init_primitive_impls_void_traits() {
    // Arrange - 规范 §3.5.2: Void 实现 Equal, Debug, Send, Sync（不实现 Clone）
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        !trait_table.has_impl("Clone", "Void"),
        "Void 不应实现 Clone"
    );
    assert!(trait_table.has_impl("Equal", "Void"), "Void 应实现 Equal");
    assert!(trait_table.has_impl("Debug", "Void"), "Void 应实现 Debug");
    assert!(trait_table.has_impl("Send", "Void"), "Void 应实现 Send");
    assert!(trait_table.has_impl("Sync", "Void"), "Void 应实现 Sync");
}

#[test]
fn test_init_primitive_impls_bytes_traits() {
    // Arrange - 规范 §3.5.2: Bytes 实现 Debug, Send, Sync（不实现 Clone, Equal）
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        !trait_table.has_impl("Clone", "Bytes"),
        "Bytes 不应实现 Clone"
    );
    assert!(
        !trait_table.has_impl("Equal", "Bytes"),
        "Bytes 不应实现 Equal"
    );
    assert!(trait_table.has_impl("Debug", "Bytes"), "Bytes 应实现 Debug");
    assert!(trait_table.has_impl("Send", "Bytes"), "Bytes 应实现 Send");
    assert!(trait_table.has_impl("Sync", "Bytes"), "Bytes 应实现 Sync");
}

#[test]
fn test_is_primitive_type_all_primitives() {
    // Arrange & Act & Assert
    assert!(is_primitive_type("Int"), "Int 应为 primitive 类型");
    assert!(is_primitive_type("Float"), "Float 应为 primitive 类型");
    assert!(is_primitive_type("Bool"), "Bool 应为 primitive 类型");
    assert!(is_primitive_type("String"), "String 应为 primitive 类型");
    assert!(is_primitive_type("Void"), "Void 应为 primitive 类型");
    assert!(is_primitive_type("Char"), "Char 应为 primitive 类型");
}

#[test]
fn test_std_trait_names_contains_all_traits() {
    // Arrange & Act - 规范 §3.5.2: 标准库接口为 Clone, Equal, Debug, Send, Sync, Iterator
    let names = std_trait_names();

    // Assert
    assert_eq!(names.len(), 6, "应有 6 个标准库 trait（规范 §3.5.2）");
    assert!(names.contains(&"Clone"), "应包含 Clone");
    assert!(names.contains(&"Equal"), "应包含 Equal");
    assert!(names.contains(&"Debug"), "应包含 Debug");
    assert!(names.contains(&"Send"), "应包含 Send");
    assert!(names.contains(&"Sync"), "应包含 Sync");
    assert!(names.contains(&"Iterator"), "应包含 Iterator");
}

#[test]
fn test_std_traits_constant_matches_function() {
    // Arrange & Act
    let constant_list = STD_TRAITS;
    let function_list = std_trait_names();

    // Assert
    assert_eq!(
        constant_list, function_list,
        "STD_TRAITS 常量与 std_trait_names() 应返回相同内容"
    );
}

#[test]
fn test_clone_method_signature() {
    // Arrange - 规范 §3.5.2: clone: (Self) -> Self
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    let clone = trait_table.get_trait("Clone").unwrap();
    let clone_sig = clone.methods.get("clone").unwrap();

    // Assert
    assert_eq!(clone_sig.name, "clone", "方法名应为 clone");
    assert!(!clone_sig.is_static, "clone 不应为静态方法");
    assert_eq!(clone_sig.params.len(), 1, "clone 应有 1 个参数 (self)");
    assert_eq!(
        clone_sig.return_type,
        MonoType::TypeRef("Self".to_string()),
        "clone 返回类型应为 Self"
    );
}

#[test]
fn test_equal_method_signature() {
    // Arrange - 规范 §3.5.2: equal: (Self, Self) -> Bool
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    let equal = trait_table.get_trait("Equal").unwrap();
    let equal_sig = equal.methods.get("equal").unwrap();

    // Assert
    assert_eq!(equal_sig.name, "equal", "方法名应为 equal");
    assert_eq!(
        equal_sig.params.len(),
        2,
        "equal 应有 2 个参数 (self, other)"
    );
    assert_eq!(
        equal_sig.return_type,
        MonoType::TypeRef("Bool".to_string()),
        "equal 返回类型应为 Bool"
    );
}

#[test]
fn test_debug_method_signature() {
    // Arrange - 规范 §3.5.2: debug: (Self, Formatter) -> Void
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    let debug = trait_table.get_trait("Debug").unwrap();
    let debug_sig = debug.methods.get("debug").unwrap();

    // Assert
    assert_eq!(debug_sig.name, "debug", "方法名应为 debug");
    assert_eq!(
        debug_sig.return_type,
        MonoType::TypeRef("Void".to_string()),
        "debug 返回类型应为 Void（规范 §3.5.2）"
    );
}

#[test]
fn test_iterator_method_signature() {
    // Arrange - 规范 §3.5.2: next: (Self) -> Option(Item)
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    let iterator = trait_table.get_trait("Iterator").unwrap();
    let next_sig = iterator.methods.get("next").unwrap();

    // Assert
    assert_eq!(next_sig.name, "next", "方法名应为 next");
    assert_eq!(
        next_sig.return_type,
        MonoType::TypeRef("Option".to_string()),
        "next 返回类型应为 Option"
    );
}

#[test]
fn test_int_clone_impl_method() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);
    init_primitive_impls(&mut trait_table);

    // Act
    let impl_ = trait_table.get_impl("Clone", "Int").unwrap();

    // Assert
    assert!(
        impl_.methods.contains_key("clone"),
        "Int 的 Clone 实现应包含 clone 方法"
    );
    let clone_fn = impl_.methods.get("clone").unwrap();
    match clone_fn {
        MonoType::Fn {
            params,
            return_type,
            is_async,
        } => {
            assert_eq!(params.len(), 1, "clone 应有 1 个参数");
            assert!(!is_async, "clone 不应是 async");
            match return_type.as_ref() {
                MonoType::TypeRef(name) => assert_eq!(name, "Self", "返回类型应为 Self"),
                other => panic!("返回类型应为 TypeRef，实际: {:?}", other),
            }
        }
        other => panic!("clone 方法应为 Fn 类型，实际: {:?}", other),
    }
}

#[test]
fn test_int_equal_impl_method() {
    // Arrange - 规范 §3.5.2: equal: (Self, Self) -> Bool
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);
    init_primitive_impls(&mut trait_table);

    // Act
    let impl_ = trait_table.get_impl("Equal", "Int").unwrap();

    // Assert
    assert!(
        impl_.methods.contains_key("equal"),
        "Int 的 Equal 实现应包含 equal 方法"
    );
    let equal_fn = impl_.methods.get("equal").unwrap();
    match equal_fn {
        MonoType::Fn {
            params,
            return_type,
            is_async,
        } => {
            assert_eq!(params.len(), 2, "equal 应有 2 个参数");
            assert!(!is_async, "equal 不应是 async");
            match return_type.as_ref() {
                MonoType::TypeRef(name) => assert_eq!(name, "Bool", "返回类型应为 Bool"),
                other => panic!("返回类型应为 TypeRef，实际: {:?}", other),
            }
        }
        other => panic!("equal 方法应为 Fn 类型，实际: {:?}", other),
    }
}

#[test]
fn test_send_impl_has_no_methods() {
    // Arrange - 规范 §3.5.2: Send 是标记接口，无方法
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);
    init_primitive_impls(&mut trait_table);

    // Act
    let impl_ = trait_table.get_impl("Send", "Int").unwrap();

    // Assert
    assert!(
        impl_.methods.is_empty(),
        "Send 是标记接口，Int 的实现不应包含任何方法"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_primitive_type_not_recognized() {
    // Arrange & Act & Assert
    assert!(!is_primitive_type("Array"), "Array 不应为 primitive 类型");
    assert!(
        !is_primitive_type("HashMap"),
        "HashMap 不应为 primitive 类型"
    );
    assert!(!is_primitive_type("Custom"), "Custom 不应为 primitive 类型");
}

#[test]
fn test_primitive_type_empty_string() {
    // Arrange & Act & Assert
    assert!(!is_primitive_type(""), "空字符串不应为 primitive 类型");
}

#[test]
fn test_has_impl_for_unimplemented_trait() {
    // Arrange - 规范 §3.5.2: Bytes 不实现 Clone 和 Equal
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);
    init_primitive_impls(&mut trait_table);

    // Act & Assert
    assert!(
        !trait_table.has_impl("Clone", "Bytes"),
        "Bytes 不应实现 Clone trait（规范 §3.5.2）"
    );
    assert!(
        !trait_table.has_impl("Equal", "Bytes"),
        "Bytes 不应实现 Equal trait（规范 §3.5.2）"
    );
}

#[test]
fn test_get_trait_unknown_returns_none() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    let result = trait_table.get_trait("NonExistent");

    // Assert
    assert!(result.is_none(), "不存在的 trait 应返回 None");
}

#[test]
fn test_get_impl_for_unknown_type_returns_none() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);
    init_primitive_impls(&mut trait_table);

    // Act
    let result = trait_table.get_impl("Clone", "UnknownType");

    // Assert
    assert!(result.is_none(), "未知类型的实现应返回 None");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_init_std_traits_idempotent() {
    // Arrange
    let mut trait_table = TraitTable::default();

    // Act - 多次初始化应幂等
    init_std_traits(&mut trait_table);
    init_std_traits(&mut trait_table);
    init_std_traits(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_trait("Clone"),
        "多次初始化后 Clone 仍应存在"
    );
    // 规范 §3.5.2: 应有 6 个标准库 trait
    let trait_count = trait_table.trait_names().count();
    assert_eq!(trait_count, 6, "多次初始化后仍应只有 6 个 trait，不应重复");
}

#[test]
fn test_init_primitive_impls_idempotent() {
    // Arrange
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act
    init_primitive_impls(&mut trait_table);
    init_primitive_impls(&mut trait_table);

    // Assert
    assert!(
        trait_table.has_impl("Clone", "Int"),
        "多次初始化后 Int 仍应实现 Clone"
    );
}

#[test]
fn test_all_std_traits_have_no_parent() {
    // Arrange - 规范 §3.5.2: 所有标准库接口都是独立的，无继承关系
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);

    // Act & Assert
    let trait_names = ["Clone", "Equal", "Debug", "Send", "Sync", "Iterator"];
    for name in &trait_names {
        let def = trait_table.get_trait(name).unwrap();
        assert!(
            def.parent_traits.is_empty(),
            "{} 不应有父 trait（规范 §3.5.2），但实际有 {:?}",
            name,
            def.parent_traits
        );
    }
}

#[test]
fn test_void_does_not_implement_clone() {
    // Arrange - 规范 §3.5.2: Void 不实现 Clone（无法复制空值）
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);
    init_primitive_impls(&mut trait_table);

    // Act & Assert
    assert!(
        !trait_table.has_impl("Clone", "Void"),
        "Void 不应实现 Clone trait（规范 §3.5.2）"
    );
}

#[test]
fn test_bytes_does_not_implement_clone_or_equal() {
    // Arrange - 规范 §3.5.2: Bytes 不实现 Clone 和 Equal（原始字节无法比较）
    let mut trait_table = TraitTable::default();
    init_std_traits(&mut trait_table);
    init_primitive_impls(&mut trait_table);

    // Act & Assert
    assert!(
        !trait_table.has_impl("Clone", "Bytes"),
        "Bytes 不应实现 Clone trait（规范 §3.5.2）"
    );
    assert!(
        !trait_table.has_impl("Equal", "Bytes"),
        "Bytes 不应实现 Equal trait（规范 §3.5.2）"
    );
}
