//! Standard Weak library
//!
//! Weak references that don't prevent value drop.
//! Used for caches, observer patterns, and breaking reference cycles.

use crate::backends::common::value::TypeId;
use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, NativeHandler, StdModule};
/// `Weak[T]` - A weak reference type that doesn't increase reference count.
///
/// # Example
/// ```yaoxiang
/// use std.weak.Weak
///
/// # let node = Node { value: 42, next: None }
/// let arc: Arc[Node] = ref node
/// let weak: Weak[Node] = Weak.new(arc)
///
/// # if let Some(arc2) = weak.upgrade() {
/// #     use(arc2)
/// # }
/// ```
///
/// # Methods
/// - `Weak.new(arc: Arc[T]) -> Weak[T]` - Create weak reference from Arc
/// - `weak.upgrade() -> Option[Arc[T]]` - Upgrade to Arc if still alive
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
/// Returns `Some(Arc[T])` if the value is still alive,
/// or `None` if the value has been dropped.
pub fn weak_upgrade(
    weak: &crate::backends::common::value::RuntimeValue
) -> Option<crate::backends::common::value::RuntimeValue> {
    weak.upgrade()
}

// ============================================================================
// WeakModule - StdModule Implementation
// ============================================================================

/// Weak module implementation.
pub struct WeakModule;

impl Default for WeakModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for WeakModule {
    fn module_path(&self) -> &str {
        "std.weak"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new(
                "new",
                "std.weak.new",
                "(arc: Arc[T]) -> Weak[T]",
                native_weak_new as NativeHandler,
            ),
            NativeExport::new(
                "upgrade",
                "std.weak.upgrade",
                "(weak: Weak[T]) -> Option[Arc[T]]",
                native_weak_upgrade as NativeHandler,
            ),
        ]
    }
}

// ============================================================================
// Native Handler Wrappers
// ============================================================================

/// Native handler wrapper for weak_new.
fn native_weak_new(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "std.weak.new expects 1 argument (arc: Arc[T])".to_string(),
        ));
    }
    Ok(weak_new(&args[0]))
}

/// Native handler wrapper for weak_upgrade.
///
/// Returns Option[Arc[T]] as a RuntimeValue::Enum:
/// - Some(arc): Enum { type_id: ENUM, variant_id: 0, payload: arc }
/// - None:      Enum { type_id: ENUM, variant_id: 1, payload: Unit }
fn native_weak_upgrade(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "std.weak.upgrade expects 1 argument (weak: Weak[T])".to_string(),
        ));
    }
    match weak_upgrade(&args[0]) {
        Some(val) => Ok(RuntimeValue::Enum {
            type_id: TypeId::ENUM,
            variant_id: 0,
            payload: Box::new(val),
        }),
        None => Ok(RuntimeValue::Enum {
            type_id: TypeId::ENUM,
            variant_id: 1,
            payload: Box::new(RuntimeValue::Unit),
        }),
    }
}
