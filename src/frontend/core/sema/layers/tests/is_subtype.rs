//! is_subtype 单元测试 — spec §3.x-L0 + RFC-027 §3.2

use crate::frontend::core::sema::layers::equivalence::is_subtype;
use crate::frontend::core::types::MonoType;

#[test]
fn reflexivity_int64() {
    assert!(is_subtype(&MonoType::Int(64), &MonoType::Int(64), None));
}
