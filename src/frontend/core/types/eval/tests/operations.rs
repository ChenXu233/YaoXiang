use crate::frontend::core::types::eval::operations::{
    ArithOp, BoolType, CmpOp, LogicOp, TypeArithmetic, TypeComparison, TypeLevelValue,
};
use crate::frontend::core::types::MonoType;

/// Assert: neq 真值（5!=3 true、5!=5 false）。
fn assert_neq_truth_table(c: &TypeComparison) {
    assert_eq!(
        c.neq(&TypeLevelValue::Int(5), &TypeLevelValue::Int(3)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.neq(&TypeLevelValue::Int(5), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(false))
    );
}

/// Assert: gt 真值（5>3 true、3>5 false）。
fn assert_gt_truth_table(c: &TypeComparison) {
    assert_eq!(
        c.gt(&TypeLevelValue::Int(5), &TypeLevelValue::Int(3)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.gt(&TypeLevelValue::Int(3), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(false))
    );
}

/// Assert: lte / gte 真值（含相等边界）。
fn assert_lte_gte_truth_table(c: &TypeComparison) {
    assert_eq!(
        c.lte(&TypeLevelValue::Int(5), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.lte(&TypeLevelValue::Int(6), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(false))
    );
    assert_eq!(
        c.gte(&TypeLevelValue::Int(5), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.gte(&TypeLevelValue::Int(4), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(false))
    );
}

#[test]
fn test_arithmetic() {
    let a = TypeArithmetic::new();
    assert_eq!(
        a.add(&TypeLevelValue::Int(3), &TypeLevelValue::Int(4)),
        Some(TypeLevelValue::Int(7))
    );
    assert_eq!(
        a.sub(&TypeLevelValue::Int(10), &TypeLevelValue::Int(3)),
        Some(TypeLevelValue::Int(7))
    );
    assert_eq!(
        a.mul(&TypeLevelValue::Int(3), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Int(15))
    );
    assert_eq!(
        a.div(&TypeLevelValue::Int(10), &TypeLevelValue::Int(2)),
        Some(TypeLevelValue::Int(5))
    );
    assert_eq!(
        a.neg(&TypeLevelValue::Int(42)),
        Some(TypeLevelValue::Int(-42))
    );
    assert!(a
        .add(&TypeLevelValue::Bool(true), &TypeLevelValue::Bool(false))
        .is_none());
}

#[test]
fn test_arith_op() {
    assert_eq!(
        ArithOp::Add.apply(&TypeLevelValue::Int(3), &TypeLevelValue::Int(4)),
        Some(TypeLevelValue::Int(7))
    );
    assert_eq!(ArithOp::Add.name(), "Add");
    assert_eq!(ArithOp::Sub.name(), "Sub");
    assert_eq!(ArithOp::Mul.name(), "Mul");
    assert_eq!(ArithOp::Div.name(), "Div");
    assert_eq!(ArithOp::Mod.name(), "Mod");
}

#[test]
fn test_arithmetic_binary_op() {
    let a = TypeArithmetic::new();
    assert_eq!(
        a.binary_op(
            ArithOp::Add,
            &TypeLevelValue::Int(2),
            &TypeLevelValue::Int(3)
        ),
        Some(TypeLevelValue::Int(5))
    );
}

#[test]
fn test_arithmetic_unary_op() {
    let a = TypeArithmetic::new();
    assert_eq!(
        a.unary_op(ArithOp::Sub, &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Int(-5))
    );
    assert!(a.unary_op(ArithOp::Add, &TypeLevelValue::Int(5)).is_none());
}

#[test]
fn test_comparison() {
    let c = TypeComparison::new();
    assert_eq!(
        c.eq(&TypeLevelValue::Int(5), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.lt(&TypeLevelValue::Int(3), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(true))
    );
    assert!(c.types_equal(&MonoType::Int(32), &MonoType::Int(32)));
    assert!(!c.types_equal(&MonoType::Int(32), &MonoType::make_string()));
}

#[test]
fn test_logic_op_name() {
    assert_eq!(LogicOp::And.name(), "And");
    assert_eq!(LogicOp::Or.name(), "Or");
    assert_eq!(LogicOp::Not.name(), "Not");
}

#[test]
fn test_bool_type() {
    assert_eq!(
        BoolType::from_value(&TypeLevelValue::Bool(true)),
        Some(BoolType::True)
    );
    assert_eq!(BoolType::from_value(&TypeLevelValue::Int(0)), None);
    assert_eq!(BoolType::True.to_value(), TypeLevelValue::Bool(true));
}

// ============ 补充测试：覆盖缺口 ============

#[test]
fn test_comparison_extra_ops() {
    // Arrange
    let c = TypeComparison::new();

    // Act & Assert — 额外比较运算的真值（neq / gt / lte / gte）
    assert_neq_truth_table(&c);
    assert_gt_truth_table(&c);
    assert_lte_gte_truth_table(&c);
}

#[test]
fn test_comparison_compare() {
    let c = TypeComparison::new();
    assert_eq!(
        c.compare(CmpOp::Eq, &TypeLevelValue::Int(5), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.compare(CmpOp::Lt, &TypeLevelValue::Int(3), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.compare(CmpOp::Gt, &TypeLevelValue::Int(5), &TypeLevelValue::Int(3)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.compare(CmpOp::Lte, &TypeLevelValue::Int(3), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.compare(CmpOp::Gte, &TypeLevelValue::Int(5), &TypeLevelValue::Int(3)),
        Some(TypeLevelValue::Bool(true))
    );
}

#[test]
fn test_arithmetic_extra() {
    let a = TypeArithmetic::new();
    assert_eq!(
        a.rem(&TypeLevelValue::Int(10), &TypeLevelValue::Int(3)),
        Some(TypeLevelValue::Int(1))
    );
    assert!(a
        .rem(&TypeLevelValue::Int(10), &TypeLevelValue::Int(0))
        .is_none());
}
