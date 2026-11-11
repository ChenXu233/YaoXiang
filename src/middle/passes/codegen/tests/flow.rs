//! 流状态管理单元测试
//!
//! 测试 LabelGenerator、RegisterAllocator、FlowManager 和 SymbolScopeManager 的功能。

use crate::frontend::core::sema::MonoType;
use crate::middle::passes::codegen::flow::{
    LabelGenerator, RegisterAllocator, Storage, Symbol, SymbolScopeManager,
};

// ── 辅助函数（规则 5.3：集中在文件顶部）──────────────

/// Helper: 构造作用域内的符号记录。
fn symbol(
    name: &str,
    ty: MonoType,
    storage: Storage,
    is_mut: bool,
    scope_level: usize,
) -> Symbol {
    Symbol {
        name: name.to_string(),
        ty,
        storage,
        is_mut,
        scope_level,
    }
}

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
fn test_scope_nesting() {
    let mut manager = SymbolScopeManager::new();
    manager.insert(
        "a".to_string(),
        symbol("a", MonoType::Int(64), Storage::Local(0), false, 0),
    );
    manager.push_scope();
    assert_eq!(manager.scope_level(), 1);
    manager.insert(
        "b".to_string(),
        symbol("b", MonoType::make_string(), Storage::Local(1), true, 1),
    );
    assert!(manager.lookup("a").is_some());
    assert!(manager.lookup("b").is_some());
    manager.pop_scope();
    assert_eq!(manager.scope_level(), 0);
    assert!(manager.lookup("b").is_none());
    assert!(manager.lookup("a").is_some());
}
