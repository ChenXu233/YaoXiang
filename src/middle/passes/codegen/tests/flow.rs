//! 流状态管理单元测试
//!
//! 测试 LabelGenerator、RegisterAllocator、FlowManager 和 SymbolScopeManager 的功能。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::passes::codegen::flow::{
    FlowManager, LabelGenerator, RegisterAllocator, Storage, Symbol, SymbolScopeManager,
};

#[test]
fn test_label_generator() {
    let mut gen = LabelGenerator::new();
    assert_eq!(gen.next_label(), 0);
    assert_eq!(gen.next_label(), 1);
    gen.reset();
    assert_eq!(gen.next_label(), 0);
}

#[test]
fn test_register_allocator() {
    let mut reg = RegisterAllocator::new();
    assert_eq!(reg.alloc_local(), 0);
    assert_eq!(reg.alloc_local(), 1);
    assert_eq!(reg.alloc_temp(), 0);
    assert_eq!(reg.alloc_temp(), 1);
}

#[test]
fn test_flow_manager() {
    let mut cfm = FlowManager::new();
    assert_eq!(cfm.alloc_local(), 0);
    assert_eq!(cfm.alloc_temp(), 0);
    assert_eq!(cfm.next_label(), 0);
    cfm.add_function_index("main".to_string(), 0);
    assert_eq!(cfm.get_function_index("main"), Some(&0));
    cfm.set_loop_label(10, 20);
    assert_eq!(cfm.loop_label(), Some((10, 20)));
}

#[test]
fn test_scope_manager_basic() {
    let mut manager = SymbolScopeManager::new();
    manager.insert(
        "x".to_string(),
        Symbol {
            name: "x".to_string(),
            ty: MonoType::Int(64),
            storage: Storage::Local(0),
            is_mut: false,
            scope_level: 0,
        },
    );
    assert!(manager.lookup("x").is_some());
    assert!(manager.lookup("y").is_none());
}

#[test]
fn test_scope_nesting() {
    let mut manager = SymbolScopeManager::new();
    manager.insert(
        "a".to_string(),
        Symbol {
            name: "a".to_string(),
            ty: MonoType::Int(64),
            storage: Storage::Local(0),
            is_mut: false,
            scope_level: 0,
        },
    );
    manager.push_scope();
    assert_eq!(manager.scope_level(), 1);
    manager.insert(
        "b".to_string(),
        Symbol {
            name: "b".to_string(),
            ty: MonoType::String,
            storage: Storage::Local(1),
            is_mut: true,
            scope_level: 1,
        },
    );
    assert!(manager.lookup("a").is_some());
    assert!(manager.lookup("b").is_some());
    manager.pop_scope();
    assert_eq!(manager.scope_level(), 0);
    assert!(manager.lookup("b").is_none());
    assert!(manager.lookup("a").is_some());
}
