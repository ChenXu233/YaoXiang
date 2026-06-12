//! 类型等价性检查单元测试
//!
//! RFC-027 Section 2: Type Equivalence Tests

use crate::frontend::core::typecheck::layers::equivalence::structurally_equal;
use crate::frontend::core::types::mono::MonoType;

#[test]
fn test_structurally_equal_int() {
    assert!(structurally_equal(&MonoType::Int(64), &MonoType::Int(64)));
    assert!(!structurally_equal(&MonoType::Int(32), &MonoType::Int(64)));
}

#[test]
fn test_structurally_equal_fn() {
    let f1 = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Bool),
    };
    let f2 = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Bool),
    };
    assert!(structurally_equal(&f1, &f2));
}

#[test]
fn test_structurally_not_equal_different_types() {
    assert!(!structurally_equal(&MonoType::Int(64), &MonoType::Bool));
}

#[test]
fn test_structurally_equal_list() {
    let l1 = MonoType::List(Box::new(MonoType::Int(64)));
    let l2 = MonoType::List(Box::new(MonoType::Int(64)));
    assert!(structurally_equal(&l1, &l2));
}

#[test]
fn test_structurally_not_equal_list() {
    let l1 = MonoType::List(Box::new(MonoType::Int(64)));
    let l2 = MonoType::List(Box::new(MonoType::Bool));
    assert!(!structurally_equal(&l1, &l2));
}

#[test]
fn test_structurally_equal_tuple() {
    let t1 = MonoType::Tuple(vec![MonoType::Int(64), MonoType::Bool]);
    let t2 = MonoType::Tuple(vec![MonoType::Int(64), MonoType::Bool]);
    assert!(structurally_equal(&t1, &t2));
}

#[test]
fn test_structurally_not_equal_tuple_different_length() {
    let t1 = MonoType::Tuple(vec![MonoType::Int(64)]);
    let t2 = MonoType::Tuple(vec![MonoType::Int(64), MonoType::Bool]);
    assert!(!structurally_equal(&t1, &t2));
}

#[test]
fn test_structurally_equal_refined() {
    // Refined 只比较基类型，忽略约束不同
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    let r1 = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("x".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    };
    let r2 = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("x".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
        },
    };
    // 基类型相同 → 结构等价（约束由 Layer 3 处理）
    assert!(structurally_equal(&r1, &r2));
}
