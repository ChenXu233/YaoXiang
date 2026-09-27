//! Standard Weak library
//!
//! Weak references that don't prevent value drop.
//! Used for caches, observer patterns, and breaking reference cycles.

use crate::backends::common::value::TypeId;
use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};
/// `Weak(T)` - A weak reference type that doesn't increase reference count.
///
/// # Example
/// ```yaoxiang
/// use std.weak.Weak
///
/// # let node = Node { value: 42, next: None }
/// let arc: Arc(Node) = ref node
/// let weak: Weak(Node) = Weak.new(arc)
///
/// # if let Some(arc2) = weak.upgrade() {
/// #     use(arc2)
/// # }
/// ```
///
/// # Methods
/// - `Weak.new(arc: Arc(T)) -> Weak(T)` - Create weak reference from Arc
/// - `weak.upgrade() -> Option[Arc(T)]` - Upgrade to Arc if still alive
///
/// Create a new Weak reference from an Arc
///
/// This function wraps the Arc in a Weak, allowing it to be upgraded later
/// without preventing the Arc's drop.
pub fn weak_new(
    arc: &crate::backends::common::value::RuntimeValue
) -> crate::backends::common::value::RuntimeValue {
    crate::backends::common::value::RuntimeValue::from_arc_into_weak(
        crate::backends::common::value::RuntimeValue::Arc(std::sync::Arc::new(arc.clone())),
    )
}

/// Upgrade a Weak reference to an Arc
///
/// Returns `Some(Arc(T))` if the value is still alive,
/// or `None` if the value has been dropped.
pub fn weak_upgrade(
    weak: &crate::backends::common::value::RuntimeValue
) -> Option<crate::backends::common::value::RuntimeValue> {
    weak.upgrade()
}

// WeakModule - StdModule Implementation

/// Weak module implementation.
#[derive(Default)]
pub struct WeakModule;

impl StdModule for WeakModule {
    fn module_path(&self) -> &str {
        "std.weak"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            export!(
                "new",
                "std.weak.new",
                "(T: Type)(arc: Arc(T)) -> Weak(T)",
                native_weak_new
            ),
            export!(
                "upgrade",
                "std.weak.upgrade",
                "(T: Type)(weak: Weak(T)) -> Option(Arc(T))",
                native_weak_upgrade
            ),
        ]
    }
}

// Native Handler Wrappers

/// Native handler wrapper for weak_new.
fn native_weak_new(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "std.weak.new expects 1 argument (arc: Arc(T))".to_string(),
        ));
    }
    Ok(weak_new(&args[0]))
}

/// Native handler wrapper for weak_upgrade.
///
/// Returns Option[Arc(T)] as a RuntimeValue::Enum:
/// - Some(arc): Enum { type_id: OPTION, variant_id: 0, payload: arc }
/// - None:      Enum { type_id: OPTION, variant_id: 1, payload: Void }
///
/// RFC-010：类型身份必须是 `TypeId::OPTION`（与 `Option(T).some/none()` 的
/// `intern_sum_type("Option")` 同值）。此前用 `TypeId::ENUM`（21）——与构造器
/// 路径的 OPTION（101）不同，于是 `weak.upgrade(w) == Option(Int).none()`
/// 静默 false（Enum 相等现在比 type_id）。
fn native_weak_upgrade(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "std.weak.upgrade expects 1 argument (weak: Weak(T))".to_string(),
        ));
    }
    match weak_upgrade(&args[0]) {
        Some(val) => Ok(RuntimeValue::Enum {
            type_id: TypeId::OPTION,
            variant_id: 0,
            payload: Box::new(val),
        }),
        None => Ok(RuntimeValue::Enum {
            type_id: TypeId::OPTION,
            variant_id: 1,
            payload: Box::new(RuntimeValue::Void),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::common::value::TypeId;
    use std::sync::Arc;

    /// RFC-010：`weak.upgrade` 产出的 Option 必须带**和构造器路径相同的类型身份**。
    ///
    /// 回归 #377 同族缺陷：本函数曾用 `TypeId::ENUM`(21)，而 `Option(T).some/none()`
    /// 经 `intern_sum_type("Option")` 得 `TypeId::OPTION`(101)。运行时 Enum 相等
    /// 比 type_id，两条构造路径的 none 因此静默判不等。
    ///
    /// 该缺陷无法从 .yx 源码观察（`Arc` 不在值位置类型名白名单里，
    /// 无法书写 `Option(Arc(Int)).none()` 做对比），故在 Rust 层锁定。
    #[test]
    fn test_weak_upgrade_option_type_identity() {
        // Arrange：构造 none（Arc 已释放）
        let dead_weak = {
            let arc_inner: Arc<RuntimeValue> = Arc::new(RuntimeValue::Int(7));
            RuntimeValue::from_arc_into_weak(RuntimeValue::Arc(arc_inner))
        };
        let mut heap = crate::backends::common::Heap::new();

        // Act
        let none_val = native_weak_upgrade(&[dead_weak], &mut NativeContext::new(&mut heap))
            .expect("native upgrade");

        // Assert：类型身份必须是 OPTION（与构造器路径同值）
        match none_val {
            RuntimeValue::Enum {
                type_id,
                variant_id,
                ..
            } => {
                assert_eq!(
                    type_id,
                    TypeId::OPTION,
                    "weak.upgrade 的 Option 必须用 TypeId::OPTION（构造器路径同值），\
                     否则 weak 的 none 与 Option(T).none() 静默判不等"
                );
                assert_eq!(variant_id, 1, "none 是 variant 1");
            }
            other => panic!("期望 Enum，实际 {:?}", other),
        }
    }
}
