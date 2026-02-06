//! Standard Weak library
//!
//! Weak references that don't prevent value drop.
//! Used for caches, observer patterns, and breaking reference cycles.

/// Weak[T] - A weak reference type that doesn't increase reference count.
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
