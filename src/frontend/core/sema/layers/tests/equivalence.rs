//! 类型等价性检查单元测试
//!
//! RFC-027 Section 2: Type Equivalence Tests

use crate::frontend::core::sema::layers::equivalence::structurally_equal;
use crate::frontend::core::types::mono::MonoType;

#[test]
fn test_structurally_equal_int() {
    assert!(structurally_equal(&MonoType::Int(64), &MonoType::Int(64)));
    assert!(!structurally_equal(&MonoType::Int(32), &MonoType::Int(64)));
}
