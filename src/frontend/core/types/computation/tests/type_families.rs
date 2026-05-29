use crate::frontend::core::types::base::MonoType;
use crate::frontend::core::types::computation::type_families::{
    Bool, IsFalse, IsTrue, IsZero, IsSucc, Nat, bool_family, nat_family, TypeFamilyOps,
};
use crate::frontend::core::types::computation::TypeLevelResult;

#[test]
fn test_bool() {
    assert!(Bool::True.as_bool() && Bool::True.is_true() && !Bool::True.is_false());
    assert!(!Bool::False.as_bool() && Bool::False.is_false());
    assert_eq!(Bool::True.eval(), TypeLevelResult::Normalized(Bool::True));
    assert_eq!(
        Bool::True.to_mono_type(),
        MonoType::TypeRef("True".to_string())
    );
}

#[test]
fn test_nat() {
    assert!(Nat::Zero.is_zero());
    assert_eq!(Nat::Zero.to_usize(), 0);
    let one = Nat::succ(Nat::Zero);
    assert!(!one.is_zero());
    assert_eq!(one.to_usize(), 1);
    assert_eq!(Nat::from_usize(0), Nat::Zero);
    assert_eq!(Nat::from_usize(1), Nat::succ(Nat::Zero));
    assert_eq!(Nat::from_usize(2), Nat::succ(Nat::succ(Nat::Zero)));
    assert_eq!(Nat::Zero.eval(), TypeLevelResult::Normalized(Nat::Zero));
    assert_eq!(
        Nat::Zero.to_mono_type(),
        MonoType::TypeRef("Zero".to_string())
    );
}

#[test]
fn test_is_true_false() {
    assert_eq!(
        IsTrue::new(Bool::True).eval(),
        TypeLevelResult::Normalized(Bool::True)
    );
    assert_eq!(
        IsFalse::new(Bool::False).eval(),
        TypeLevelResult::Normalized(Bool::True)
    );
}

#[test]
fn test_is_zero_succ() {
    assert_eq!(
        IsZero::new(Nat::Zero).eval(),
        TypeLevelResult::Normalized(Bool::True)
    );
    assert_eq!(
        IsZero::new(Nat::succ(Nat::Zero)).eval(),
        TypeLevelResult::Normalized(Bool::False)
    );
    assert_eq!(
        IsSucc::new(Nat::Zero).eval(),
        TypeLevelResult::Normalized(Bool::False)
    );
    assert_eq!(
        IsSucc::new(Nat::succ(Nat::Zero)).eval(),
        TypeLevelResult::Normalized(Bool::True)
    );
}

#[test]
fn test_helpers() {
    assert!(bool_family::true_().as_bool());
    assert!(!bool_family::false_().as_bool());
    assert!(nat_family::zero().is_zero());
    assert_eq!(
        nat_family::to_usize(&nat_family::succ(nat_family::zero())),
        1
    );
    assert_eq!(nat_family::to_usize(&nat_family::from_usize(5)), 5);
}

// ===================================================================
// RFC-011 §5.2: Bool/Nat 补充测试
// ===================================================================

#[test]
fn test_bool_from_bool() {
    assert!(
        Bool::from_bool(true).is_true(),
        "from_bool(true) should be True"
    );
    assert!(
        Bool::from_bool(false).is_false(),
        "from_bool(false) should be False"
    );
}

#[test]
fn test_bool_as_bool_roundtrip() {
    assert!(Bool::from_bool(true).as_bool());
    assert!(!Bool::from_bool(false).as_bool());
}

#[test]
fn test_nat_to_usize_values() {
    assert_eq!(Nat::Zero.to_usize(), 0);
    assert_eq!(Nat::succ(Nat::Zero).to_usize(), 1);
    assert_eq!(Nat::succ(Nat::succ(Nat::Zero)).to_usize(), 2);
    assert_eq!(Nat::succ(Nat::succ(Nat::succ(Nat::Zero))).to_usize(), 3);
}

#[test]
fn test_nat_from_usize_roundtrip() {
    for n in 0..10 {
        assert_eq!(
            Nat::from_usize(n).to_usize(),
            n,
            "roundtrip failed for {}",
            n
        );
    }
}

#[test]
fn test_nat_is_zero() {
    assert!(Nat::Zero.is_zero(), "Zero should be zero");
    assert!(!Nat::succ(Nat::Zero).is_zero(), "Succ should not be zero");
    assert!(
        !Nat::succ(Nat::succ(Nat::Zero)).is_zero(),
        "Succ(Succ) should not be zero"
    );
}

#[test]
fn test_nat_eval() {
    assert_eq!(Nat::Zero.eval(), TypeLevelResult::Normalized(Nat::Zero));
    let one = Nat::succ(Nat::Zero);
    assert_eq!(
        one.eval(),
        TypeLevelResult::Normalized(Nat::succ(Nat::Zero))
    );
}

#[test]
fn test_nat_to_mono_type() {
    assert_eq!(
        Nat::Zero.to_mono_type(),
        MonoType::TypeRef("Zero".to_string())
    );
    // Succ(Zero) displays as "Succ(0)" since Zero.to_usize() == 0
    assert_eq!(
        Nat::succ(Nat::Zero).to_mono_type(),
        MonoType::TypeRef("Succ(0)".to_string())
    );
}

#[test]
fn test_bool_eval() {
    assert_eq!(Bool::True.eval(), TypeLevelResult::Normalized(Bool::True));
    assert_eq!(Bool::False.eval(), TypeLevelResult::Normalized(Bool::False));
}

#[test]
fn test_bool_to_mono_type() {
    assert_eq!(
        Bool::True.to_mono_type(),
        MonoType::TypeRef("True".to_string())
    );
    assert_eq!(
        Bool::False.to_mono_type(),
        MonoType::TypeRef("False".to_string())
    );
}

#[test]
fn test_is_true_false_eval() {
    // IsTrue(True) = True
    assert_eq!(
        IsTrue::new(Bool::True).eval(),
        TypeLevelResult::Normalized(Bool::True)
    );
    // IsTrue(False) = False
    assert_eq!(
        IsTrue::new(Bool::False).eval(),
        TypeLevelResult::Normalized(Bool::False)
    );
    // IsFalse(True) = False
    assert_eq!(
        IsFalse::new(Bool::True).eval(),
        TypeLevelResult::Normalized(Bool::False)
    );
    // IsFalse(False) = True
    assert_eq!(
        IsFalse::new(Bool::False).eval(),
        TypeLevelResult::Normalized(Bool::True)
    );
}

#[test]
fn test_is_zero_succ_eval() {
    // IsZero(Zero) = True
    assert_eq!(
        IsZero::new(Nat::Zero).eval(),
        TypeLevelResult::Normalized(Bool::True)
    );
    // IsZero(Succ) = False
    assert_eq!(
        IsZero::new(Nat::succ(Nat::Zero)).eval(),
        TypeLevelResult::Normalized(Bool::False)
    );
    // IsSucc(Zero) = False
    assert_eq!(
        IsSucc::new(Nat::Zero).eval(),
        TypeLevelResult::Normalized(Bool::False)
    );
    // IsSucc(Succ) = True
    assert_eq!(
        IsSucc::new(Nat::succ(Nat::Zero)).eval(),
        TypeLevelResult::Normalized(Bool::True)
    );
}
