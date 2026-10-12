//! `std::weak` 模块的单元测试
//!
//! 覆盖 RFC-010「运行时表示与相等」在和类型身份上的要求：
//! `std.weak.upgrade` 返回的 `Option(Arc(T))` 必须带**和构造器路径相同的
//! 类型身份**（`TypeId::OPTION`）。运行时 Enum 相等比较 `type_id`，
//! 身份不一致会让两条构造路径的 `none` 静默判不等。
//!
//! 另覆盖 RFC-009 弱引用存活语义（#384）：`weak.new` 必须降级**传入的
//! Arc 句柄**——目标存活时 upgrade 得 some、目标释放后得 none。
//!
//! 为何在 Rust 层而非 `.yx` 语料锁定：`Arc` 不在值位置类型名白名单里
//! （`is_builtin_generic_type_name` 只含 `Vec`/`Array`），源码无法书写
//! `Option(Arc(Int)).none()` 与 weak 结果做对比，该契约在语言层不可达。

use std::sync::Arc;

use crate::backends::common::value::TypeId;
use crate::backends::common::{Heap, RuntimeValue};
use crate::std::weak::{native_weak_upgrade, weak_new};
use crate::std::NativeContext;

/// RFC-010：`weak.upgrade` 的 none 必须带 `TypeId::OPTION`（与构造器路径同值）。
///
/// 断言取自 RFC-010 的类型身份约定，非当前实现输出。此前实现用
/// `TypeId::ENUM`(21)，而 `Option(T).none()` 经和类型登记得 `OPTION`(101)，
/// 两者不等 → 同一变体静默判不等。
#[test]
fn test_weak_upgrade_none_uses_option_type_identity() {
    // Arrange：Arc 提前释放，upgrade 必得 none
    let dead_weak = {
        let arc_inner: Arc<RuntimeValue> = Arc::new(RuntimeValue::Int(7));
        RuntimeValue::from_arc_into_weak(RuntimeValue::Arc(arc_inner))
    };
    let mut heap = Heap::new();

    // Act
    let upgraded = native_weak_upgrade(&[dead_weak], &mut NativeContext::new(&mut heap))
        .expect("weak.upgrade 调用不应失败");

    // Assert：必须是带 OPTION 身份的 none 变体
    assert!(
        matches!(
            &upgraded,
            RuntimeValue::Enum {
                type_id: TypeId::OPTION,
                variant_id: 1,
                ..
            }
        ),
        "weak.upgrade 的 none 必须是 Enum{{type_id: OPTION, variant_id: 1}}，\
         否则与 Option(T).none() 的相等判定静默为 false；实际: {upgraded:?}"
    );
}

/// #384：`weak.new` 降级的是**传入的 Arc 句柄**——原 Arc 存活期间
/// upgrade 必得 some（此前克隆进全新 Arc，upgrade 恒 none）。
#[test]
fn test_weak_new_upgrade_some_while_original_alive() {
    // Arrange：既有 Arc 与由它降级的弱引用
    let arc = RuntimeValue::Arc(Arc::new(RuntimeValue::Int(42)));
    let weak = weak_new(&arc);

    // Act
    let upgraded = weak.upgrade();

    // Assert：同源 Arc 存活 → some，且载荷与原 Arc 内的值同源
    assert!(
        upgraded.is_some(),
        "upgrade of a weak taken from a live Arc must be Some（#384）；实际: {upgraded:?}"
    );
}

/// #384：原 Arc 释放后 upgrade 必得 none——弱引用不得延长目标寿命。
#[test]
fn test_weak_new_upgrade_none_after_original_dropped() {
    // Arrange：降级后立刻释放原 Arc
    let arc = RuntimeValue::Arc(Arc::new(RuntimeValue::Int(42)));
    let weak = weak_new(&arc);
    drop(arc);

    // Act
    let upgraded = weak.upgrade();

    // Assert：唯一强句柄已释放 → none
    assert!(
        upgraded.is_none(),
        "upgrade must be None after the original Arc is dropped；实际: {upgraded:?}"
    );
}
