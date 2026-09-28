//! `backends::common::value` 模块的单元测试
//!
//! 覆盖 RFC-010「运行时表示与相等」中的**和类型身份**约定：
//! Enum 相等比较 `type_id`，故同一和类型名在任何时刻、任何解释器实例、
//! 任何构造顺序下都必须得到同一个 `TypeId`，否则同名类型会静默判不等。
//!
//! RFC-010 明文：运行时表示中，和类型的相等 = 类型身份 + 变体序 + 载荷逐值相等。

use crate::backends::common::value::TypeId;

/// RFC-010：身份是**名字的确定性函数**，且不同名字不碰撞。
///
/// 断言取自 RFC-010 的「类型身份」约定（名字定身份），非当前实现输出。
#[test]
fn test_sum_type_id_deterministic_and_collision_free() {
    // Arrange：同一名字多次取、中间夹杂别的名字
    let color_first = TypeId::for_sum_name("Color");

    // Act
    let other = TypeId::for_sum_name("Other");
    let color_again = TypeId::for_sum_name("Color");

    // Assert：同名恒等、异名不等
    assert_eq!(color_first, color_again, "同名和类型必须恒得同一身份");
    assert_ne!(color_first, other, "不同和类型必须得到不同身份");
}

/// RFC-010：预置和类型（`Result` / `Option`）的身份必须与 native 侧常量同值。
///
/// 存在意义：`Option` 有两条构造路径——yx 侧 `Option(T).some/none()` 经
/// `for_sum_name`，Rust 侧 `std.weak.upgrade` 直接用常量。两者必须同值，
/// 否则 `weak.upgrade(w) == Option(Int).none()` 静默判 false。
#[test]
fn test_preset_sum_type_ids_match_native_constants() {
    // Arrange + Act + Assert
    assert_eq!(
        TypeId::for_sum_name("Result"),
        TypeId::RESULT,
        "Result 的登记身份必须等于 TypeId::RESULT"
    );
    assert_eq!(
        TypeId::for_sum_name("Option"),
        TypeId::OPTION,
        "Option 的登记身份必须等于 TypeId::OPTION"
    );
}

/// 预置段不与内建小 ID（ENUM / STRUCT）相撞——撞了会让类型分类错乱。
#[test]
fn test_sum_type_ids_do_not_collide_with_builtin_ids() {
    // Arrange
    let user = TypeId::for_sum_name("Color");

    // Act + Assert
    assert_ne!(user, TypeId::ENUM, "用户和类型身份不得与 ENUM 相撞");
    assert_ne!(user, TypeId::STRUCT, "用户和类型身份不得与 STRUCT 相撞");
}

/// 大量相异名字不产生碰撞（实测登记表，不假定分配策略的性质）。
#[test]
fn test_many_distinct_names_yield_distinct_ids() {
    // Arrange
    const COUNT: usize = 500;

    // Act
    let mut ids: Vec<TypeId> = (0..COUNT)
        .map(|i| TypeId::for_sum_name(&format!("UserSumType{i}")))
        .collect();

    // Assert
    ids.sort_by_key(|t| t.0);
    ids.dedup();
    assert_eq!(ids.len(), COUNT, "{COUNT} 个相异名字不得有身份碰撞");
}

/// 已登记名字的 ID 不随**后续**登记而变（顺序无关）。
///
/// 契约意义：身份只由名字决定。若改成「按登记顺序分配」，先构造 `Other`
/// 的解释器会把 `Color` 排到另一个号，跨解释器比较即静默判不等。
/// 本测试只看全进程登记表是否满足该契约（单进程下单实例调用测不出
/// 「两个解释器各持一表」的旧实现差异，故这是契约锁，不是缺陷复现）。
#[test]
fn test_id_stable_under_subsequent_registrations() {
    // Arrange
    let baseline = TypeId::for_sum_name("OrderProbeA");

    // Act：掺入一批新名字，可能改变内部计数
    for i in 0..50 {
        let _ = TypeId::for_sum_name(&format!("OrderNoise{i}"));
    }

    // Assert
    assert_eq!(
        TypeId::for_sum_name("OrderProbeA"),
        baseline,
        "已登记名字的 ID 不得随后续登记而变（身份必须只由名字决定）"
    );
}
