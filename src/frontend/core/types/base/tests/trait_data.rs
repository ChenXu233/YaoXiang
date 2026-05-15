//! Trait 数据测试 — 基于语言规范 §3.5
//!
//! §3.5: 接口类型
//! TraitTable 的添加、查询、方法查找、实现管理

use crate::frontend::core::types::base::{
    MonoType, TraitBound, TraitDefinition, TraitImplementation, TraitMethodSignature, TraitTable,
};
use std::collections::HashMap;

// ===== TraitTable 基础操作 =====

#[test]
fn test_trait_table_new() {
    assert!(TraitTable::new().trait_names().next().is_none());
}

#[test]
fn test_trait_table_add_and_get() {
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    assert!(table.has_trait("Clone"));
    assert!(table.get_trait("Clone").is_some());
    assert!(!table.has_trait("NonExistent"));
}

#[test]
fn test_trait_table_has_trait_returns_false_for_missing() {
    let table = TraitTable::new();
    assert!(
        !table.has_trait("Missing"),
        "empty table should not have any trait"
    );
}

#[test]
fn test_trait_table_get_trait_returns_none_for_missing() {
    let table = TraitTable::new();
    assert!(
        table.get_trait("Missing").is_none(),
        "empty table should return None"
    );
}

// ===== TraitTable 实现管理 =====

#[test]
fn test_trait_table_impl() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Point".to_string(),
        methods: HashMap::new(),
    });
    assert!(table.has_impl("Clone", "Point"));
    assert!(table.get_impl("Clone", "Point").is_some());
}

#[test]
fn test_trait_table_has_impl_returns_false_for_missing() {
    let table = TraitTable::new();
    assert!(
        !table.has_impl("Clone", "Point"),
        "empty table should not have impl"
    );
}

#[test]
fn test_trait_table_get_impl_returns_none_for_missing() {
    let table = TraitTable::new();
    assert!(
        table.get_impl("Clone", "Point").is_none(),
        "empty table should return None"
    );
}

// ===== TraitTable 方法查找 =====

#[test]
fn test_trait_table_get_method_impl() {
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    let mut methods = HashMap::new();
    methods.insert(
        "clone".to_string(),
        MonoType::Fn {
            params: vec![MonoType::TypeRef("Self".to_string())],
            return_type: Box::new(MonoType::TypeRef("Self".to_string())),
            is_async: false,
        },
    );
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Point".to_string(),
        methods,
    });
    let method = table.get_method_impl("Clone", "Point", "clone");
    assert!(method.is_some(), "should find clone method");
    assert!(
        matches!(method.unwrap(), MonoType::Fn { .. }),
        "method should be Fn type"
    );
}

#[test]
fn test_trait_table_get_method_impl_missing_method() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Point".to_string(),
        methods: HashMap::new(),
    });
    let method = table.get_method_impl("Clone", "Point", "nonexistent");
    assert!(method.is_none(), "should return None for missing method");
}

#[test]
fn test_trait_table_get_method_impl_missing_trait() {
    let table = TraitTable::new();
    let method = table.get_method_impl("Missing", "Point", "clone");
    assert!(method.is_none(), "should return None for missing trait");
}

#[test]
fn test_trait_table_get_method_impl_missing_type() {
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    let method = table.get_method_impl("Clone", "MissingType", "clone");
    assert!(method.is_none(), "should return None for missing type");
}

// ===== TraitTable 多 trait 管理 =====

#[test]
fn test_trait_table_multiple_traits() {
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    table.add_trait(TraitDefinition {
        name: "Display".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    assert!(table.has_trait("Clone"), "should have Clone");
    assert!(table.has_trait("Display"), "should have Display");
    assert!(!table.has_trait("Debug"), "should not have Debug");
}

#[test]
fn test_trait_table_multiple_impls() {
    let mut table = TraitTable::new();
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Point".to_string(),
        methods: HashMap::new(),
    });
    table.add_impl(TraitImplementation {
        trait_name: "Display".to_string(),
        for_type_name: "Point".to_string(),
        methods: HashMap::new(),
    });
    assert!(
        table.has_impl("Clone", "Point"),
        "Point should implement Clone"
    );
    assert!(
        table.has_impl("Display", "Point"),
        "Point should implement Display"
    );
    assert!(
        !table.has_impl("Debug", "Point"),
        "Point should not implement Debug"
    );
}

#[test]
fn test_trait_table_trait_names() {
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "A".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    table.add_trait(TraitDefinition {
        name: "B".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    let names: Vec<&String> = table.trait_names().collect();
    assert_eq!(names.len(), 2, "should have 2 trait names");
    assert!(names.contains(&&"A".to_string()), "should contain A");
    assert!(names.contains(&&"B".to_string()), "should contain B");
}

#[test]
fn test_trait_table_overwrite_trait() {
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    let mut methods = HashMap::new();
    methods.insert(
        "clone".to_string(),
        TraitMethodSignature {
            name: "clone".to_string(),
            params: vec![],
            return_type: MonoType::TypeRef("Self".to_string()),
            is_static: false,
        },
    );
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods,
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
    });
    let def = table.get_trait("Clone").unwrap();
    assert!(
        def.methods.contains_key("clone"),
        "overwritten trait should have clone method"
    );
}

// ===== TraitMethodSignature 测试 =====

#[test]
fn test_trait_method_signature_creation() {
    let sig = TraitMethodSignature {
        name: "clone".to_string(),
        params: vec![MonoType::TypeRef("Self".to_string())],
        return_type: MonoType::TypeRef("Self".to_string()),
        is_static: false,
    };
    assert!(!sig.is_static);
    assert_eq!(sig.name, "clone");
    assert_eq!(sig.params.len(), 1, "should have 1 param");
    assert_eq!(sig.return_type, MonoType::TypeRef("Self".to_string()));
}

#[test]
fn test_trait_method_signature_static() {
    let sig = TraitMethodSignature {
        name: "new".to_string(),
        params: vec![],
        return_type: MonoType::TypeRef("Self".to_string()),
        is_static: true,
    };
    assert!(sig.is_static, "should be static");
    assert!(sig.params.is_empty(), "static method should have no params");
}

// ===== TraitDefinition 测试 =====

#[test]
fn test_trait_definition_with_parents() {
    let def = TraitDefinition {
        name: "PartialOrd".to_string(),
        methods: HashMap::new(),
        parent_traits: vec!["Eq".to_string()],
        generic_params: vec![],
        span: None,
    };
    assert_eq!(def.parent_traits[0], "Eq");
    assert_eq!(def.parent_traits.len(), 1);
}

#[test]
fn test_trait_definition_with_generic_params() {
    let def = TraitDefinition {
        name: "From".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec!["T".to_string()],
        span: None,
    };
    assert_eq!(def.generic_params.len(), 1, "should have 1 generic param");
    assert_eq!(def.generic_params[0], "T");
}

// ===== TraitBound 测试 =====

#[test]
fn test_trait_bound() {
    let b = TraitBound {
        trait_name: "Clone".to_string(),
        self_type: MonoType::TypeRef("T".to_string()),
    };
    assert_eq!(b.trait_name, "Clone");
    assert_eq!(b.self_type, MonoType::TypeRef("T".to_string()));
}

#[test]
fn test_trait_bound_equality() {
    let b1 = TraitBound {
        trait_name: "Clone".to_string(),
        self_type: MonoType::TypeRef("T".to_string()),
    };
    let b2 = TraitBound {
        trait_name: "Clone".to_string(),
        self_type: MonoType::TypeRef("T".to_string()),
    };
    assert_eq!(b1, b2, "identical bounds should be equal");
}

// ===== TraitImplementation 测试 =====

#[test]
fn test_trait_implementation_creation() {
    let mut methods = HashMap::new();
    methods.insert(
        "clone".to_string(),
        MonoType::Fn {
            params: vec![],
            return_type: Box::new(MonoType::TypeRef("Self".to_string())),
            is_async: false,
        },
    );
    let impl_ = TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Point".to_string(),
        methods,
    };
    assert_eq!(impl_.trait_name, "Clone");
    assert_eq!(impl_.for_type_name, "Point");
    assert!(impl_.methods.contains_key("clone"));
}
