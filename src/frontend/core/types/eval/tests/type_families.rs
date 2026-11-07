use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::type_families::{
    Bool, Nat, bool_family, nat_family, TypeFamilyOps,
};
use crate::frontend::core::types::eval::TypeLevelResult;

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

// RFC-011 §5.2: Bool/Nat 补充测试

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
fn test_nat_to_usize_values() {
    assert_eq!(Nat::Zero.to_usize(), 0);
    assert_eq!(Nat::succ(Nat::Zero).to_usize(), 1);
    assert_eq!(Nat::succ(Nat::succ(Nat::Zero)).to_usize(), 2);
    assert_eq!(Nat::succ(Nat::succ(Nat::succ(Nat::Zero))).to_usize(), 3);
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
