use crate::frontend::core::types::computation::operations::{
    ArithOp, BoolType, CmpOp, LogicOp, TypeArithmetic, TypeComparison, TypeLogic, TypeLevelValue,
};
use crate::frontend::core::types::MonoType;

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
    assert!(!c.types_equal(&MonoType::Int(32), &MonoType::String));
}

#[test]
fn test_cmp_op() {
    assert_eq!(
        CmpOp::Eq.apply(&TypeLevelValue::Int(1), &TypeLevelValue::Int(1)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        CmpOp::Lt.apply(&TypeLevelValue::Int(1), &TypeLevelValue::Int(2)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(CmpOp::Eq.name(), "Eq");
    assert_eq!(CmpOp::Neq.name(), "Neq");
    assert_eq!(CmpOp::Lt.name(), "Lt");
}

#[test]
fn test_logic() {
    let l = TypeLogic::new();
    assert_eq!(
        l.and(&TypeLevelValue::Bool(true), &TypeLevelValue::Bool(true)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        l.or(&TypeLevelValue::Bool(true), &TypeLevelValue::Bool(false)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        l.not(&TypeLevelValue::Bool(true)),
        Some(TypeLevelValue::Bool(false))
    );
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
fn test_arith_div_by_zero() {
    let _a = TypeArithmetic::new();
    // ArithOp::Div with zero → None branch
    assert_eq!(
        ArithOp::Div.apply(&TypeLevelValue::Int(10), &TypeLevelValue::Int(0)),
        None
    );
    // ArithOp::Mod with zero → None branch
    assert_eq!(
        ArithOp::Mod.apply(&TypeLevelValue::Int(10), &TypeLevelValue::Int(0)),
        None
    );
}

#[test]
fn test_comparison_extra_ops() {
    let c = TypeComparison::new();
    assert_eq!(
        c.neq(&TypeLevelValue::Int(5), &TypeLevelValue::Int(3)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.neq(&TypeLevelValue::Int(5), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(false))
    );
    assert_eq!(
        c.gt(&TypeLevelValue::Int(5), &TypeLevelValue::Int(3)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        c.gt(&TypeLevelValue::Int(3), &TypeLevelValue::Int(5)),
        Some(TypeLevelValue::Bool(false))
    );
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
fn test_cmp_op_all_variants() {
    assert_eq!(
        CmpOp::Neq.apply(&TypeLevelValue::Int(1), &TypeLevelValue::Int(2)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        CmpOp::Gt.apply(&TypeLevelValue::Int(2), &TypeLevelValue::Int(1)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        CmpOp::Lte.apply(&TypeLevelValue::Int(1), &TypeLevelValue::Int(1)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        CmpOp::Gte.apply(&TypeLevelValue::Int(1), &TypeLevelValue::Int(1)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(CmpOp::Gt.name(), "Gt");
    assert_eq!(CmpOp::Lte.name(), "Lte");
    assert_eq!(CmpOp::Gte.name(), "Gte");
}

#[test]
fn test_cmp_op_bool_eq() {
    assert_eq!(
        CmpOp::Eq.apply(&TypeLevelValue::Bool(true), &TypeLevelValue::Bool(true)),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        CmpOp::Neq.apply(&TypeLevelValue::Bool(true), &TypeLevelValue::Bool(false)),
        Some(TypeLevelValue::Bool(true))
    );
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
fn test_logic_extra() {
    let l = TypeLogic::new();
    assert_eq!(
        l.and(&TypeLevelValue::Bool(false), &TypeLevelValue::Bool(true)),
        Some(TypeLevelValue::Bool(false))
    );
    assert_eq!(
        l.or(&TypeLevelValue::Bool(false), &TypeLevelValue::Bool(false)),
        Some(TypeLevelValue::Bool(false))
    );
    let ls = TypeLogic::new().with_short_circuit(true);
    assert_eq!(
        ls.and(&TypeLevelValue::Bool(false), &TypeLevelValue::Bool(true)),
        Some(TypeLevelValue::Bool(false))
    );
}

#[test]
fn test_logic_op_apply() {
    assert_eq!(
        LogicOp::And.apply(
            Some(&TypeLevelValue::Bool(true)),
            Some(&TypeLevelValue::Bool(true))
        ),
        Some(TypeLevelValue::Bool(true))
    );
    assert_eq!(
        LogicOp::Or.apply(
            Some(&TypeLevelValue::Bool(false)),
            Some(&TypeLevelValue::Bool(false))
        ),
        Some(TypeLevelValue::Bool(false))
    );
    assert_eq!(
        LogicOp::Not.apply(Some(&TypeLevelValue::Bool(true)), None),
        Some(TypeLevelValue::Bool(false))
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

#[test]
fn test_arithmetic_float_binary_op() {
    let a = TypeArithmetic::new();
    // Float multiplication: both sides Float→Int(2) cast, result Int(4) scaled
    assert_eq!(
        a.binary_op(
            ArithOp::Mul,
            &TypeLevelValue::Int(3),
            &TypeLevelValue::Int(2)
        ),
        Some(TypeLevelValue::Int(6))
    );
}
