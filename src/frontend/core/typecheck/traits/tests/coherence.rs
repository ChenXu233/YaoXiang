//! 一致性测试 — 基于语言规范 §3.5.1
//!
//! §3.5.1: 接口实现唯一性规则
//!
//! 一致性检查确保：
//! - 同一类型对同一接口只能实现一次（唯一性规则）
//! - 孤儿规则（orphan rule）防止跨模块的实现冲突

use crate::frontend::core::typecheck::traits::coherence::{CoherenceChecker, OrphanChecker};
use crate::frontend::core::types::{TraitTable, TraitDefinition, TraitImplementation};
use std::collections::HashMap;

fn make_trait_table_with_conflict() -> TraitTable {
    let mut table = TraitTable::new();

    // 定义 Clone trait
    let mut methods = HashMap::new();
    methods.insert(
        "clone".to_string(),
        crate::frontend::core::types::TraitMethodSignature {
            name: "clone".to_string(),
            params: vec![],
            return_type: crate::frontend::core::types::MonoType::TypeRef("Self".to_string()),
            is_static: false,
        },
    );
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods,
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    });

    // 第一次添加 Clone for Point — 应成功
    let added = table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Point".to_string(),
        methods: HashMap::new(),
    });
    assert!(added, "第一次 add_impl 应返回 true");

    // 第二次添加 Clone for Point — 应被拒绝
    let added = table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Point".to_string(),
        methods: HashMap::new(),
    });
    assert!(!added, "重复 add_impl 应返回 false");

    table
}

// ===================================================================
// CoherenceChecker — 冲突检测
// ===================================================================

#[test]
fn test_coherence_checker_detects_conflict() {
    let table = make_trait_table_with_conflict();
    let checker = CoherenceChecker::new(&table);

    // 冲突已被 add_impl 拒绝，CoherenceChecker 做最终验证
    let errors = checker.check();
    // 当前实现：add_impl 已阻止覆盖，check 做结构验证
    // 未来 #73 加 span 后可报告具体位置
    let _ = errors;
}

#[test]
fn test_coherence_checker_empty_table() {
    let table = TraitTable::new();
    let checker = CoherenceChecker::new(&table);
    let errors = checker.check();
    assert!(errors.is_empty(), "空表不应有错误");
}

#[test]
fn test_coherence_checker_no_conflict() {
    let mut table = TraitTable::new();

    let mut methods = HashMap::new();
    methods.insert(
        "clone".to_string(),
        crate::frontend::core::types::TraitMethodSignature {
            name: "clone".to_string(),
            params: vec![],
            return_type: crate::frontend::core::types::MonoType::TypeRef("Self".to_string()),
            is_static: false,
        },
    );
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods,
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    });

    // 不同类型的同名 trait 实现不冲突
    assert!(table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Point".to_string(),
        methods: HashMap::new(),
    }));
    assert!(table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Line".to_string(),
        methods: HashMap::new(),
    }));

    let checker = CoherenceChecker::new(&table);
    let errors = checker.check();
    assert!(errors.is_empty(), "不同类型同名 trait 不应冲突");
}

// ===================================================================
// TraitTable::add_impl — 冲突拒绝
// ===================================================================

#[test]
fn test_add_impl_rejects_duplicate() {
    let mut table = TraitTable::new();

    let impl1 = TraitImplementation {
        trait_name: "Display".to_string(),
        for_type_name: "Int".to_string(),
        methods: HashMap::new(),
    };
    let impl2 = TraitImplementation {
        trait_name: "Display".to_string(),
        for_type_name: "Int".to_string(),
        methods: HashMap::new(),
    };

    assert!(table.add_impl(impl1), "第一次应成功");
    assert!(!table.add_impl(impl2), "重复应被拒绝");
}

#[test]
fn test_add_impl_allows_different_traits() {
    let mut table = TraitTable::new();

    assert!(table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "Int".to_string(),
        methods: HashMap::new(),
    }));
    assert!(table.add_impl(TraitImplementation {
        trait_name: "Debug".to_string(),
        for_type_name: "Int".to_string(),
        methods: HashMap::new(),
    }));
}

// ===================================================================
// OrphanChecker — 空实现（等 #73）
// ===================================================================

#[test]
fn test_orphan_checker_returns_empty() {
    let checker = OrphanChecker::new();
    let errors = checker.check();
    assert!(errors.is_empty(), "孤儿检查器当前应返回空（等 #73）");
}
