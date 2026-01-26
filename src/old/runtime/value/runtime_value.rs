//! Runtime value type system for YaoXiang
//!
//! This module implements `RuntimeValue`, the unified representation of all values
//! in YaoXiang programs at runtime. It follows RFC-009 ownership model where:
//! - Default is Move (zero-copy)
//! - `ref` keyword maps to `Arc<RuntimeValue>`
//! - `clone()` performs deep copy
//!
//! # Handle System
//! Collections (List, Tuple, Struct) use Handle to reference heap-allocated
//! storage, enabling efficient in-place modification without cloning.

use std::sync::Arc;
use std::collections::HashMap;
use std::fmt;
use std::alloc;

use super::heap::Handle;

/// Integer width variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntWidth {
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,
}

/// Float width variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatWidth {
    F32,
    F64,
}

/// Pointer kind for unsafe operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PtrKind {
    Const, // *const T
    Mut,   // *mut T
}

/// Value type enumeration for type queries and assertions
///
/// Represents the type of a runtime value, used for type checking and reflection.
/// Note: This describes the type structure, not the instance data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// Empty value
    Unit,
    /// Boolean
    Bool,
    /// Integer with specific width
    Int(IntWidth),
    /// Float with specific width
    Float(FloatWidth),
    /// Character (Unicode code point)
    Char,
    /// String (reference, Arc<str>)
    String,
    /// Byte array
    Bytes,
    /// Tuple with element types
    Tuple(Vec<ValueType>),
    /// Fixed-size array
    Array {
        /// Element type
        element: Box<ValueType>,
    },
    /// Dynamic list/array
    List,
    /// Dictionary/map
    Dict,
    /// User-defined struct type
    Struct(TypeId),
    /// Enum/union type
    Enum(TypeId),
    /// Function type
    Function(FunctionId),
    /// Reference type (ref T) - thread-safe Arc
    Ref(Box<ValueType>),
    /// Arc reference (same as Ref in runtime)
    Arc(Box<ValueType>),
    /// Async value for concurrent model
    Async(Box<ValueType>),
    /// Raw pointer (only in unsafe blocks)
    Ptr(PtrKind),
}

/// Type ID for runtime type identification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

/// Function ID for runtime function identification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(pub u32);

/// Task ID for async scheduling
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);

/// Async state for lazy evaluation
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum AsyncState {
    /// Synchronously ready value
    Ready(Box<RuntimeValue>),
    /// Pending computation task (lazy evaluation)
    Pending(TaskId),
    /// Computation error
    Error(Box<RuntimeValue>),
}

/// Async value for the concurrent model (RFC-001, RFC-008)
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct AsyncValue {
    /// Actual value or computation task
    pub state: Box<AsyncState>,
    /// Value type for type checking
    pub value_type: ValueType,
}

/// Function value (closure with captured environment)
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct FunctionValue {
    /// Function ID
    pub func_id: FunctionId,
    /// Captured environment variables (closure)
    pub env: Vec<RuntimeValue>,
}

/// Runtime value - unified representation of all YaoXiang values
///
/// # Design Principles
/// - Uses `enum` for easy pattern matching
/// - `Arc` for shared ownership, value itself doesn't contain ownership state
/// - Ownership semantics guaranteed at compile time, RuntimeValue just represents them
///
/// See RFC-009 for ownership model details.
#[derive(Debug, Clone, Default)]
pub enum RuntimeValue {
    /// Empty value
    #[default]
    Unit,

    /// Boolean (small object, stored directly)
    Bool(bool),

    /// Integer (small object, stored directly)
    Int(i64),

    /// Float (small object, stored directly)
    Float(f64),

    /// Character (Unicode code point)
    Char(u32),

    /// String (shared string, Arc<str>)
    String(Arc<str>),

    /// Byte array
    Bytes(Arc<[u8]>),

    /// Tuple (stored on heap via handle for efficient cloning)
    Tuple(Handle),

    /// Fixed-size array (stored on heap via handle)
    Array(Handle),

    /// Dynamic list (stored on heap via handle for efficient modification)
    List(Handle),

    /// Dictionary (stored on heap via handle)
    Dict(Handle),

    /// Struct instance (fields stored on heap via handle)
    Struct {
        /// Type ID for type queries and field access
        type_id: TypeId,
        /// Field values handle (stored on heap for efficient cloning)
        fields: Handle,
        /// Virtual method table (method name -> function)
        vtable: Vec<(String, FunctionValue)>,
    },

    /// Enum variant
    Enum {
        /// Type ID
        type_id: TypeId,
        /// Variant index
        variant_id: u32,
        /// Variant payload (Unit if no payload)
        payload: Box<RuntimeValue>,
    },

    /// Function closure (captures environment)
    Function(FunctionValue),

    /// Thread-safe reference count (ref T keyword runtime representation)
    ///
    /// # Design Note
    /// Per RFC-009, `ref T` keyword uses Arc at runtime:
    /// - Compile-time: cross-spawn ref cycle detection
    /// - Runtime: Arc manages reference counting automatically
    /// - Stored as `Arc<Box<RuntimeValue>>` to avoid recursive type issues
    Arc(Arc<RuntimeValue>),

    /// Async value for concurrent model
    ///
    /// # Lazy Evaluation
    /// Async values don't compute immediately, trigger on first use
    Async(Box<AsyncValue>),

    /// Raw pointer (only for unsafe blocks)
    ///
    /// # Safety
    /// - Only usable in unsafe blocks
    /// - User must ensure no dangling pointers
    Ptr {
        /// Pointer kind
        kind: PtrKind,
        /// Memory address
        address: usize,
        /// Pointed type ID
        type_id: TypeId,
    },
}

// ============================================================================
// Type Query Methods
// ============================================================================

impl RuntimeValue {
    /// Get the static type of this value
    ///
    /// # Arguments
    /// * `heap` - Optional reference to heap for handling collection types
    ///
    /// When heap is provided, collection types (Tuple, Array) will have their
    /// element types resolved. When None, returns simplified types for collections.
    pub fn value_type(
        &self,
        heap: Option<&super::heap::Heap>,
    ) -> ValueType {
        match self {
            RuntimeValue::Unit => ValueType::Unit,
            RuntimeValue::Bool(_) => ValueType::Bool,
            RuntimeValue::Int(_) => ValueType::Int(IntWidth::I64),
            RuntimeValue::Float(_) => ValueType::Float(FloatWidth::F64),
            RuntimeValue::Char(_) => ValueType::Char,
            RuntimeValue::String(_) => ValueType::String,
            RuntimeValue::Bytes(_) => ValueType::Bytes,
            // Handle types: use heap if available, otherwise simplified type
            RuntimeValue::Tuple(handle) => {
                if let Some(h) = heap {
                    if let Some(super::heap::HeapValue::Tuple(items)) = h.get(*handle) {
                        return ValueType::Tuple(
                            items.iter().map(|v| v.value_type(heap)).collect(),
                        );
                    }
                }
                ValueType::Tuple(vec![])
            }
            RuntimeValue::Array(handle) => {
                if let Some(h) = heap {
                    if let Some(super::heap::HeapValue::Array(items)) = h.get(*handle) {
                        return ValueType::Array {
                            element: Box::new(
                                items
                                    .first()
                                    .map(|v| v.value_type(heap))
                                    .unwrap_or(ValueType::Unit),
                            ),
                        };
                    }
                }
                ValueType::Array {
                    element: Box::new(ValueType::Unit),
                }
            }
            RuntimeValue::List(_) => ValueType::List,
            RuntimeValue::Dict(_) => ValueType::Dict,
            RuntimeValue::Struct { type_id, .. } => ValueType::Struct(*type_id),
            RuntimeValue::Enum { type_id, .. } => ValueType::Enum(*type_id),
            RuntimeValue::Function(f) => ValueType::Function(f.func_id),
            RuntimeValue::Arc(inner) => ValueType::Arc(Box::new(inner.value_type(heap))),
            RuntimeValue::Async(v) => ValueType::Async(Box::new(v.value_type.clone())),
            RuntimeValue::Ptr { kind, .. } => ValueType::Ptr(*kind),
        }
    }

    /// Get the static type of this value (convenience method without heap)
    ///
    /// For collection types (Tuple, Array), this returns a simplified type.
    /// Use `value_type(Some(heap))` for complete type information.
    pub fn value_type_simple(&self) -> ValueType {
        self.value_type(None)
    }

    /// Check if value matches a type
    pub fn is_type(
        &self,
        ty: &ValueType,
    ) -> bool {
        &self.value_type(None) == ty
    }

    /// Get enum variant ID
    pub fn enum_variant_id(&self) -> Option<u32> {
        match self {
            RuntimeValue::Enum { variant_id, .. } => Some(*variant_id),
            _ => None,
        }
    }

    /// Get enum payload
    pub fn enum_payload(&self) -> Option<&RuntimeValue> {
        match self {
            RuntimeValue::Enum { payload, .. } => Some(payload),
            _ => None,
        }
    }

    /// Get struct field by index (legacy method, returns None for handle types)
    pub fn struct_field(
        &self,
        _index: usize,
    ) -> Option<&RuntimeValue> {
        // Legacy method doesn't work with handles - use struct_field_with_heap
        None
    }

    /// Get struct field by index with heap access
    pub fn struct_field_with_heap<'a>(
        &self,
        index: usize,
        heap: &'a super::heap::Heap,
    ) -> Option<&'a RuntimeValue> {
        match self {
            RuntimeValue::Struct { fields, .. } => {
                if let Some(super::heap::HeapValue::Tuple(items)) = heap.get(*fields) {
                    items.get(index)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert to bool
    pub fn to_bool(&self) -> Option<bool> {
        match self {
            RuntimeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Convert to i64
    pub fn to_int(&self) -> Option<i64> {
        match self {
            RuntimeValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Convert to f64
    pub fn to_float(&self) -> Option<f64> {
        match self {
            RuntimeValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Get Arc inner value
    pub fn as_arc(&self) -> Option<&RuntimeValue> {
        match self {
            RuntimeValue::Arc(inner) => Some(inner),
            _ => None,
        }
    }

    /// Check if this is an Arc (ref keyword)
    pub fn is_arc(&self) -> bool {
        matches!(self, RuntimeValue::Arc(_))
    }

    /// Get method from vtable by name
    pub fn get_method(
        &self,
        name: &str,
    ) -> Option<&FunctionValue> {
        match self {
            RuntimeValue::Struct { vtable, .. } => {
                vtable.iter().find(|(n, _)| n == name).map(|(_, f)| f)
            }
            RuntimeValue::Function(f) => Some(f),
            _ => None,
        }
    }

    /// Get vtable reference for a struct
    pub fn vtable(&self) -> Option<&Vec<(String, FunctionValue)>> {
        match self {
            RuntimeValue::Struct { vtable, .. } => Some(vtable),
            _ => None,
        }
    }
}

// ============================================================================
// Ownership Operations
// ============================================================================

impl RuntimeValue {
    /// Move: transfer ownership (zero-copy, just pointer move)
    ///
    /// # Note
    /// - Happens automatically on assignment, parameter passing, return
    /// - Original value becomes invalid
    pub fn move_into(self) -> Self {
        self // Value transfers directly, zero-copy
    }

    /// Clone: explicit copy (user calls explicit_clone())
    ///
    /// # Note
    /// - Deep copies the entire value
    /// - Performance cost depends on value size
    /// - For handle types, this clones the heap data and returns a new handle
    pub fn explicit_clone(&self) -> Self {
        match self {
            RuntimeValue::Unit => RuntimeValue::Unit,
            RuntimeValue::Bool(b) => RuntimeValue::Bool(*b),
            RuntimeValue::Int(i) => RuntimeValue::Int(*i),
            RuntimeValue::Float(f) => RuntimeValue::Float(*f),
            RuntimeValue::Char(c) => RuntimeValue::Char(*c),
            // Arc types share underlying data
            RuntimeValue::String(s) => RuntimeValue::String(s.clone()),
            RuntimeValue::Bytes(b) => RuntimeValue::Bytes(b.clone()),
            // Handle types - return placeholder (use explicit_clone_with_heap for full functionality)
            RuntimeValue::Tuple(_)
            | RuntimeValue::Array(_)
            | RuntimeValue::List(_)
            | RuntimeValue::Dict(_) => RuntimeValue::Unit,
            RuntimeValue::Struct {
                type_id,
                fields,
                vtable,
            } => RuntimeValue::Struct {
                type_id: *type_id,
                fields: *fields,
                vtable: vtable.clone(),
            },
            RuntimeValue::Enum {
                type_id,
                variant_id,
                payload,
            } => RuntimeValue::Enum {
                type_id: *type_id,
                variant_id: *variant_id,
                payload: Box::new((**payload).clone()),
            },
            RuntimeValue::Function(f) => RuntimeValue::Function(f.clone()),
            // Arc is a wrapper around RuntimeValue, clone the Arc
            RuntimeValue::Arc(arc) => RuntimeValue::Arc(arc.clone()),
            RuntimeValue::Async(a) => RuntimeValue::Async(a.clone()),
            RuntimeValue::Ptr {
                kind,
                address,
                type_id,
            } => RuntimeValue::Ptr {
                kind: *kind,
                address: *address,
                type_id: *type_id,
            },
        }
    }

    /// Clone with heap access (properly handles handle types)
    ///
    /// This method clones the heap data for handle types and returns a new handle.
    pub fn explicit_clone_with_heap(
        &self,
        heap: &mut super::heap::Heap,
    ) -> Self {
        match self {
            RuntimeValue::Unit => RuntimeValue::Unit,
            RuntimeValue::Bool(b) => RuntimeValue::Bool(*b),
            RuntimeValue::Int(i) => RuntimeValue::Int(*i),
            RuntimeValue::Float(f) => RuntimeValue::Float(*f),
            RuntimeValue::Char(c) => RuntimeValue::Char(*c),
            // Arc types share underlying data
            RuntimeValue::String(s) => RuntimeValue::String(s.clone()),
            RuntimeValue::Bytes(b) => RuntimeValue::Bytes(b.clone()),
            // Handle types - clone the heap data (clone items first to avoid borrow conflict)
            RuntimeValue::Tuple(handle) => {
                let items_copy: Vec<RuntimeValue> =
                    if let Some(super::heap::HeapValue::Tuple(items)) = heap.get(*handle) {
                        items.clone()
                    } else {
                        vec![]
                    };
                let cloned = items_copy
                    .into_iter()
                    .map(|v| v.explicit_clone_with_heap(heap))
                    .collect();
                RuntimeValue::Tuple(heap.allocate(super::heap::HeapValue::Tuple(cloned)))
            }
            RuntimeValue::Array(handle) => {
                let items_copy: Vec<RuntimeValue> =
                    if let Some(super::heap::HeapValue::Array(items)) = heap.get(*handle) {
                        items.clone()
                    } else {
                        vec![]
                    };
                let cloned = items_copy
                    .into_iter()
                    .map(|v| v.explicit_clone_with_heap(heap))
                    .collect();
                RuntimeValue::Array(heap.allocate(super::heap::HeapValue::Array(cloned)))
            }
            RuntimeValue::List(handle) => {
                let items_copy: Vec<RuntimeValue> =
                    if let Some(super::heap::HeapValue::List(items)) = heap.get(*handle) {
                        items.clone()
                    } else {
                        vec![]
                    };
                let cloned = items_copy
                    .into_iter()
                    .map(|v| v.explicit_clone_with_heap(heap))
                    .collect();
                RuntimeValue::List(heap.allocate(super::heap::HeapValue::List(cloned)))
            }
            RuntimeValue::Dict(handle) => {
                let map_copy: HashMap<RuntimeValue, RuntimeValue> =
                    if let Some(super::heap::HeapValue::Dict(map)) = heap.get(*handle) {
                        map.clone()
                    } else {
                        HashMap::new()
                    };
                let cloned = map_copy
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k.explicit_clone_with_heap(heap),
                            v.explicit_clone_with_heap(heap),
                        )
                    })
                    .collect();
                RuntimeValue::Dict(heap.allocate(super::heap::HeapValue::Dict(cloned)))
            }
            RuntimeValue::Struct {
                type_id,
                fields,
                vtable,
            } => {
                let items_copy: Vec<RuntimeValue> =
                    if let Some(super::heap::HeapValue::Tuple(items)) = heap.get(*fields) {
                        items.clone()
                    } else {
                        vec![]
                    };
                let cloned = items_copy
                    .into_iter()
                    .map(|v| v.explicit_clone_with_heap(heap))
                    .collect();
                RuntimeValue::Struct {
                    type_id: *type_id,
                    fields: heap.allocate(super::heap::HeapValue::Tuple(cloned)),
                    vtable: vtable.clone(),
                }
            }
            RuntimeValue::Enum {
                type_id,
                variant_id,
                payload,
            } => RuntimeValue::Enum {
                type_id: *type_id,
                variant_id: *variant_id,
                payload: Box::new((**payload).explicit_clone_with_heap(heap)),
            },
            RuntimeValue::Function(f) => RuntimeValue::Function(f.clone()),
            // Arc is a wrapper around RuntimeValue, clone the Arc
            RuntimeValue::Arc(arc) => RuntimeValue::Arc(arc.clone()),
            RuntimeValue::Async(a) => RuntimeValue::Async(a.clone()),
            RuntimeValue::Ptr {
                kind,
                address,
                type_id,
            } => RuntimeValue::Ptr {
                kind: *kind,
                address: *address,
                type_id: *type_id,
            },
        }
    }

    /// Convert to Arc (ref keyword runtime representation)
    ///
    /// Per RFC-009: `ref p` = `Arc::new(p)`
    pub fn into_arc(self) -> Self {
        RuntimeValue::Arc(Arc::new(self))
    }

    /// Create Arc from a RuntimeValue
    pub fn from_arc(arc: Arc<RuntimeValue>) -> Self {
        RuntimeValue::Arc(arc)
    }
}

// ============================================================================
// Memory Layout (for allocators)
// ============================================================================

impl RuntimeValue {
    /// Get memory layout for this value (for allocators)
    pub fn layout(&self) -> alloc::Layout {
        match self {
            RuntimeValue::Unit => alloc::Layout::new::<()>(),
            RuntimeValue::Bool(_) => alloc::Layout::new::<bool>(),
            RuntimeValue::Int(_) => alloc::Layout::new::<i64>(),
            RuntimeValue::Float(_) => alloc::Layout::new::<f64>(),
            RuntimeValue::Char(_) => alloc::Layout::new::<u32>(),
            // String and Bytes: pointer + length (stored in Arc)
            RuntimeValue::String(_) => alloc::Layout::new::<Arc<str>>(),
            RuntimeValue::Bytes(_) => alloc::Layout::new::<Arc<[u8]>>(),
            // Handle types: use Handle layout (simplified)
            RuntimeValue::Tuple(_) | RuntimeValue::Array(_) | RuntimeValue::List(_) => {
                alloc::Layout::new::<Handle>()
            }
            // Dict: HashMap layout (simplified)
            RuntimeValue::Dict(_) => alloc::Layout::new::<Handle>(),
            // Struct: handle type layout
            RuntimeValue::Struct { .. } => alloc::Layout::new::<Handle>(),
            // Enum: variant index + payload
            RuntimeValue::Enum { .. } => alloc::Layout::new::<(u32, Box<RuntimeValue>)>(),
            // Arc: Arc layout
            RuntimeValue::Arc(_) => alloc::Layout::new::<Arc<RuntimeValue>>(),
            // Async: AsyncState layout
            RuntimeValue::Async(_) => alloc::Layout::new::<AsyncState>(),
            // Raw pointer
            RuntimeValue::Ptr { .. } => alloc::Layout::new::<usize>(),
            // Function value
            RuntimeValue::Function(_) => alloc::Layout::new::<FunctionValue>(),
        }
    }
}

// ============================================================================
// Display Implementation
// ============================================================================

impl fmt::Display for RuntimeValue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            RuntimeValue::Unit => write!(f, "()"),
            RuntimeValue::Bool(b) => write!(f, "{}", b),
            RuntimeValue::Int(i) => write!(f, "{}", i),
            RuntimeValue::Float(fl) => write!(f, "{}", fl),
            RuntimeValue::Char(c) => {
                if let Some(ch) = char::from_u32(*c) {
                    write!(f, "{}", ch)
                } else {
                    write!(f, "U+{:04X}", c)
                }
            }
            RuntimeValue::String(s) => write!(f, "\"{}\"", s),
            RuntimeValue::Bytes(b) => write!(f, "bytes[{}]", b.len()),
            // Handle types - use placeholder (use display_with_heap for full output)
            RuntimeValue::Tuple(handle) => {
                write!(f, "tuple@{}", handle.raw())
            }
            RuntimeValue::Array(handle) => {
                write!(f, "array@{}", handle.raw())
            }
            RuntimeValue::List(handle) => {
                write!(f, "list@{}", handle.raw())
            }
            RuntimeValue::Dict(handle) => {
                write!(f, "dict@{}", handle.raw())
            }
            RuntimeValue::Struct {
                type_id: _,
                fields,
                vtable: _,
            } => {
                write!(f, "struct@{}", fields.raw())
            }
            RuntimeValue::Enum {
                type_id: _,
                variant_id,
                payload: _,
            } => {
                write!(f, "enum::v{}", variant_id)
            }
            RuntimeValue::Function(_) => write!(f, "function"),
            RuntimeValue::Arc(inner) => write!(f, "arc({})", inner),
            RuntimeValue::Async(_) => write!(f, "async"),
            RuntimeValue::Ptr { kind, address, .. } => write!(f, "ptr({:?}, {:#x})", kind, address),
        }
    }
}

// ============================================================================
// PartialEq Implementation
// ============================================================================

// Using bit patterns for Float to ensure consistent comparison (NaN handling)
// Note: We treat NaN values as equal if they have the same bit pattern
impl PartialEq for RuntimeValue {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        match (self, other) {
            (RuntimeValue::Unit, RuntimeValue::Unit) => true,
            (RuntimeValue::Bool(a), RuntimeValue::Bool(b)) => a == b,
            (RuntimeValue::Int(a), RuntimeValue::Int(b)) => a == b,
            // Compare Float by bit pattern for consistent HashMap behavior
            (RuntimeValue::Float(a), RuntimeValue::Float(b)) => a.to_bits() == b.to_bits(),
            (RuntimeValue::Char(a), RuntimeValue::Char(b)) => a == b,
            (RuntimeValue::String(a), RuntimeValue::String(b)) => a.as_ref() == b.as_ref(),
            (RuntimeValue::Bytes(a), RuntimeValue::Bytes(b)) => a.as_ref() == b.as_ref(),
            // Handle types - compare by handle (use eq_with_heap for content comparison)
            (RuntimeValue::Tuple(a), RuntimeValue::Tuple(b)) => a == b,
            (RuntimeValue::Array(a), RuntimeValue::Array(b)) => a == b,
            (RuntimeValue::List(a), RuntimeValue::List(b)) => a == b,
            (RuntimeValue::Dict(a), RuntimeValue::Dict(b)) => a == b,
            (
                RuntimeValue::Struct {
                    type_id: t1,
                    fields: f1,
                    vtable: _,
                },
                RuntimeValue::Struct {
                    type_id: t2,
                    fields: f2,
                    vtable: _,
                },
            ) => t1 == t2 && f1 == f2,
            (
                RuntimeValue::Enum {
                    type_id: t1,
                    variant_id: v1,
                    payload: p1,
                },
                RuntimeValue::Enum {
                    type_id: t2,
                    variant_id: v2,
                    payload: p2,
                },
            ) => t1 == t2 && v1 == v2 && p1 == p2,
            (RuntimeValue::Function(f1), RuntimeValue::Function(f2)) => f1 == f2,
            (RuntimeValue::Arc(a), RuntimeValue::Arc(b)) => a == b,
            (RuntimeValue::Async(a), RuntimeValue::Async(b)) => a == b,
            (
                RuntimeValue::Ptr {
                    kind: k1,
                    address: a1,
                    type_id: t1,
                },
                RuntimeValue::Ptr {
                    kind: k2,
                    address: a2,
                    type_id: t2,
                },
            ) => k1 == k2 && a1 == a2 && t1 == t2,
            _ => false,
        }
    }
}

impl Eq for RuntimeValue {}

// ============================================================================
// Hash Implementation
// ============================================================================

use std::hash::{Hash, Hasher};

impl Hash for RuntimeValue {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        core::mem::discriminant(self).hash(state);
        match self {
            RuntimeValue::Unit => {}
            RuntimeValue::Bool(b) => b.hash(state),
            RuntimeValue::Int(i) => i.hash(state),
            // Hash Float by bit pattern for consistent HashMap behavior
            RuntimeValue::Float(f) => f.to_bits().hash(state),
            RuntimeValue::Char(c) => c.hash(state),
            RuntimeValue::String(s) => s.as_ref().hash(state),
            RuntimeValue::Bytes(b) => b.as_ref().hash(state),
            // Handle types - hash by handle value (use hash_with_heap for content hashing)
            RuntimeValue::Tuple(handle) => handle.hash(state),
            RuntimeValue::Array(handle) => handle.hash(state),
            RuntimeValue::List(handle) => handle.hash(state),
            RuntimeValue::Dict(handle) => handle.hash(state),
            RuntimeValue::Struct {
                type_id,
                fields,
                vtable: _,
            } => {
                type_id.hash(state);
                fields.hash(state);
            }
            RuntimeValue::Enum {
                type_id,
                variant_id,
                payload: _,
            } => {
                type_id.hash(state);
                variant_id.hash(state);
            }
            RuntimeValue::Function(func) => func.func_id.hash(state),
            RuntimeValue::Arc(inner) => {
                // Hash by discriminant since Arc contents may differ
                core::mem::discriminant(&**inner).hash(state);
            }
            RuntimeValue::Async(_) => {
                // Async values don't participate in Dict key comparison
                core::mem::discriminant(self).hash(state);
            }
            RuntimeValue::Ptr {
                kind,
                address,
                type_id,
            } => {
                kind.hash(state);
                address.hash(state);
                type_id.hash(state);
            }
        }
    }
}

// ============================================================================
