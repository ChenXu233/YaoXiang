//! RFC-010/011 标准库 Derive 测试
//!
//! 测试编译器内置的自动派生机制：
//! - Record 类型自动派生 Clone, Copy, Debug 等
//! - 字段全实现某 trait → Record 自动实现该 trait

use crate::frontend::type_level::auto_derive::{
    BUILTIN_DERIVES, is_primitive_type, is_builtin_derive, can_auto_derive, generate_auto_derive,
};
use crate::frontend::type_level::trait_bounds::TraitTable;
use crate::frontend::type_level::std_traits::{init_std_traits, init_primitive_impls};
use crate::frontend::core::parser::ast::{Type, StructField};

#[test]
fn test_builtin_derives_list() {
    // 测试内置可派生 trait 列表
    let builtin = BUILTIN_DERIVES;
    assert!(builtin.contains(&"Clone"));
    assert!(builtin.contains(&"Copy"));
    assert!(builtin.contains(&"Debug"));
    assert!(builtin.contains(&"PartialEq"));
    assert!(builtin.contains(&"Eq"));
}

#[test]
fn test_is_builtin_derive() {
    assert!(is_builtin_derive("Clone"));
    assert!(is_builtin_derive("Copy"));
    assert!(is_builtin_derive("Debug"));
    assert!(is_builtin_derive("PartialEq"));
    assert!(is_builtin_derive("Eq"));
    assert!(!is_builtin_derive("Unknown"));
    assert!(!is_builtin_derive("CloneTrait"));
}

#[test]
fn test_is_primitive_type() {
    // 测试 primitive 类型判断
    assert!(is_primitive_type("Int"));
    assert!(is_primitive_type("Float"));
    assert!(is_primitive_type("Bool"));
    assert!(is_primitive_type("String"));
    assert!(is_primitive_type("Void"));
    assert!(is_primitive_type("Char"));

    // 非 primitive 类型
    assert!(!is_primitive_type("Point"));
    assert!(!is_primitive_type("List"));
    assert!(!is_primitive_type("UserDefined"));
}

#[test]
fn test_record_deriver_creation() {
    let table = TraitTable::new();
    // 检查 BUILTIN_DERIVES 常量是否有内容
    assert!(BUILTIN_DERIVES.len() > 0);
    // 验证 can_auto_derive 函数存在
    let _ = can_auto_derive;
}

#[test]
fn test_builtin_derives_length() {
    // 确保内置派生列表有5个标准 trait
    assert_eq!(BUILTIN_DERIVES.len(), 5);
}

#[test]
fn test_auto_derive_with_primitive_fields() {
    // 测试：Int 字段的 Record 应该自动派生 Clone
    let mut table = TraitTable::new();

    // 初始化标准库 traits 和 primitive 实现
    init_std_traits(&mut table);
    init_primitive_impls(&mut table);

    // 定义 Point 类型（Int 字段）
    let point_fields: Vec<StructField> = vec![
        StructField::new("x".to_string(), false, Type::Name("Int".to_string())),
        StructField::new("y".to_string(), false, Type::Name("Int".to_string())),
    ];

    // 检查是否可以自动派生 Clone
    let can_clone = can_auto_derive(&table, "Clone", &point_fields);
    assert!(
        can_clone,
        "Point with Int fields should be able to derive Clone"
    );

    // 检查是否可以自动派生 PartialEq
    let can_partial_eq = can_auto_derive(&table, "PartialEq", &point_fields);
    assert!(
        can_partial_eq,
        "Point with Int fields should be able to derive PartialEq"
    );
}

#[test]
fn test_auto_derive_not_derive_non_primitive() {
    // 测试：非 primitive 类型的 Record 不能自动派生（因为还没有实现 Clone）
    let mut table = TraitTable::new();

    // 只初始化标准库 traits，不初始化 primitive 实现
    init_std_traits(&mut table);

    // 定义 Point 类型
    let point_fields: Vec<StructField> = vec![
        StructField::new("x".to_string(), false, Type::Name("Int".to_string())),
        StructField::new("y".to_string(), false, Type::Name("Int".to_string())),
    ];

    // 检查是否可以自动派生 Clone（Int 还没有实现 Clone）
    let can_clone = can_auto_derive(&table, "Clone", &point_fields);
    assert!(
        !can_clone,
        "Point should NOT be able to derive Clone when Int doesn't implement Clone"
    );
}

#[test]
fn test_generate_auto_derive_clone() {
    // 测试生成 Clone 自动派生实现
    let impl_ = generate_auto_derive("Point", "Clone");
    assert!(impl_.is_some(), "Should generate Clone impl");

    let impl_ = impl_.unwrap();
    assert_eq!(impl_.trait_name, "Clone");
    assert_eq!(impl_.for_type_name, "Point");
    assert!(
        impl_.methods.contains_key("clone"),
        "Clone impl should have clone method"
    );
}

#[test]
fn test_generate_auto_derive_partial_eq() {
    // 测试生成 PartialEq 自动派生实现
    let impl_ = generate_auto_derive("Point", "PartialEq");
    assert!(impl_.is_some(), "Should generate PartialEq impl");

    let impl_ = impl_.unwrap();
    assert_eq!(impl_.trait_name, "PartialEq");
    assert_eq!(impl_.for_type_name, "Point");
    assert!(
        impl_.methods.contains_key("eq"),
        "PartialEq impl should have eq method"
    );
}

#[test]
fn test_std_traits_initialization() {
    // 测试标准库 traits 初始化
    let mut table = TraitTable::new();

    init_std_traits(&mut table);

    // 检查 traits 是否已定义
    assert!(table.has_trait("Clone"));
    assert!(table.has_trait("Copy"));
    assert!(table.has_trait("Debug"));
    assert!(table.has_trait("PartialEq"));
    assert!(table.has_trait("Eq"));
}

#[test]
fn test_primitive_impls_initialization() {
    // 测试 primitive 类型实现初始化
    let mut table = TraitTable::new();

    init_std_traits(&mut table);
    init_primitive_impls(&mut table);

    // 检查 Int 是否实现了 Clone
    assert!(table.has_impl("Clone", "Int"), "Int should implement Clone");

    // 检查 Int 是否实现了 PartialEq
    assert!(
        table.has_impl("PartialEq", "Int"),
        "Int should implement PartialEq"
    );

    // 检查 Int 是否实现了 Debug
    assert!(table.has_impl("Debug", "Int"), "Int should implement Debug");
}
