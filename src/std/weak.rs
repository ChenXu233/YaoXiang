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
///
/// #384：必须从传入的 `RuntimeValue::Arc` 提取**既有句柄**做降级——
/// 此前把值克隆进全新 `Arc` 再降级，该新 Arc 强计数 1 且随即消亡，
/// `upgrade` 对全程存活的目标也恒返回 none，弱引用三大语义全部落空。
pub fn weak_new(
    arc: &crate::backends::common::value::RuntimeValue
) -> crate::backends::common::value::RuntimeValue {
    use crate::backends::common::value::RuntimeValue;
    match arc {
        RuntimeValue::Arc(inner) => {
            // 句柄 clone（强计数临时 +1）→ downgrade → 临时句柄随函数返回
            // 消亡：调用方 Arc 的强计数净变化为零，Weak 与其同源。
            RuntimeValue::from_arc_into_weak(RuntimeValue::Arc(inner.clone()))
        }
        // 类型层签名 `(arc: Arc(T))` 已保证 Arc 形态；非 Arc 防御路径维持
        // 旧行为（独立 Arc 包装），仅供运行时内部误用兜底。
        other => {
            RuntimeValue::from_arc_into_weak(RuntimeValue::Arc(std::sync::Arc::new(other.clone())))
        }
    }
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
pub(crate) fn native_weak_upgrade(
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
