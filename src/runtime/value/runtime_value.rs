//! Runtime value type system for YaoXiang
//!
//! This module implements `RuntimeValue`, the unified representation of all values
//! in YaoXiang programs at runtime. It follows RFC-009 ownership model where:
//! - Default is Move (zero-copy)
//! - `ref` keyword maps to `Arc<RuntimeValue>`
//! - `clone()` performs deep copy

use std::sync::Arc;
use std::collections::HashMap;
use std::fmt;
use std::alloc;

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
pub struct TaskId(pub u32);

/// Async state for lazy evaluation
#[derive(Debug, Clone)]
pub enum AsyncState {
    /// Synchronously ready value
    Ready(Box<RuntimeValue>),
    /// Pending computation task (lazy evaluation)
    Pending(TaskId),
    /// Computation error
    Error(Box<RuntimeValue>),
}

/// Async value for the concurrent model (RFC-001, RFC-008)
#[derive(Debug, Clone)]
pub struct AsyncValue {
    /// Actual value or computation task
    pub state: Box<AsyncState>,
    /// Value type for type checking
    pub value_type: ValueType,
}

/// Function value (closure with captured environment)
#[derive(Debug, Clone)]
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

    /// Tuple
    Tuple(Vec<RuntimeValue>),

    /// Fixed-size array
    Array(Vec<RuntimeValue>),

    /// Dynamic list
    List(Vec<RuntimeValue>),

    /// Dictionary
    Dict(HashMap<RuntimeValue, RuntimeValue>),

    /// Struct instance
    Struct {
        /// Type ID for type queries and field access
        type_id: TypeId,
        /// Field values (stored in definition order)
        fields: Vec<RuntimeValue>,
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
    pub fn value_type(&self) -> ValueType {
        match self {
            RuntimeValue::Unit => ValueType::Unit,
            RuntimeValue::Bool(_) => ValueType::Bool,
            RuntimeValue::Int(_) => ValueType::Int(IntWidth::I64),
            RuntimeValue::Float(_) => ValueType::Float(FloatWidth::F64),
            RuntimeValue::Char(_) => ValueType::Char,
            RuntimeValue::String(_) => ValueType::String,
            RuntimeValue::Bytes(_) => ValueType::Bytes,
            RuntimeValue::Tuple(fields) => {
                ValueType::Tuple(fields.iter().map(|v| v.value_type()).collect())
            }
            RuntimeValue::Array(fields) => ValueType::Array {
                element: Box::new(
                    fields
                        .first()
                        .map(|v| v.value_type())
                        .unwrap_or(ValueType::Unit),
                ),
            },
            RuntimeValue::List(_) => ValueType::List,
            RuntimeValue::Dict(_) => ValueType::Dict,
            RuntimeValue::Struct { type_id, .. } => ValueType::Struct(*type_id),
            RuntimeValue::Enum { type_id, .. } => ValueType::Enum(*type_id),
            RuntimeValue::Function(f) => ValueType::Function(f.func_id),
            RuntimeValue::Arc(inner) => ValueType::Arc(Box::new(inner.value_type())),
            RuntimeValue::Async(v) => ValueType::Async(Box::new(v.value_type.clone())),
            RuntimeValue::Ptr { kind, .. } => ValueType::Ptr(*kind),
        }
    }

    /// Check if value matches a type
    pub fn is_type(
        &self,
        ty: &ValueType,
    ) -> bool {
        &self.value_type() == ty
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

    /// Get struct field by index
    pub fn struct_field(
        &self,
        index: usize,
    ) -> Option<&RuntimeValue> {
        match self {
            RuntimeValue::Struct { fields, .. } => fields.get(index),
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
            // Vec needs deep copy
            RuntimeValue::Tuple(v) => RuntimeValue::Tuple(v.to_vec()),
            RuntimeValue::Array(v) => RuntimeValue::Array(v.to_vec()),
            RuntimeValue::List(v) => RuntimeValue::List(v.to_vec()),
            RuntimeValue::Dict(m) => RuntimeValue::Dict(m.clone()),
            RuntimeValue::Struct { type_id, fields } => RuntimeValue::Struct {
                type_id: *type_id,
                fields: fields.to_vec(),
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
        use std::mem;
        match self {
            RuntimeValue::Unit => alloc::Layout::new::<()>(),
            RuntimeValue::Bool(_) => alloc::Layout::new::<bool>(),
            RuntimeValue::Int(_) => alloc::Layout::new::<i64>(),
            RuntimeValue::Float(_) => alloc::Layout::new::<f64>(),
            RuntimeValue::Char(_) => alloc::Layout::new::<u32>(),
            // String and Bytes: pointer + length (stored in Arc)
            RuntimeValue::String(_) => alloc::Layout::new::<Arc<str>>(),
            RuntimeValue::Bytes(_) => alloc::Layout::new::<Arc<[u8]>>(),
            // Collection types: Vec layout
            RuntimeValue::Tuple(_) | RuntimeValue::Array(_) | RuntimeValue::List(_) => {
                alloc::Layout::new::<Vec<RuntimeValue>>()
            }
            // Dict: HashMap layout
            RuntimeValue::Dict(_) => alloc::Layout::new::<HashMap<RuntimeValue, RuntimeValue>>(),
            // Struct: combined field layouts
            RuntimeValue::Struct { fields, .. } => {
                let size = mem::size_of::<RuntimeValue>() * fields.len();
                let align = mem::align_of::<RuntimeValue>();
                alloc::Layout::from_size_align(size, align).unwrap()
            }
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
            RuntimeValue::Unit => write!(f, "unit"),
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
            RuntimeValue::Tuple(fields) => {
                write!(
                    f,
                    "({})",
                    fields
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            RuntimeValue::Array(fields) => {
                write!(
                    f,
                    "[{}]",
                    fields
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            RuntimeValue::List(fields) => {
                write!(
                    f,
                    "[{}]",
                    fields
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            RuntimeValue::Dict(m) => {
                write!(
                    f,
                    "{{{}}}",
                    m.iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            RuntimeValue::Struct { type_id: _, fields } => {
                write!(
                    f,
                    "struct{{ {} }}",
                    fields
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
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
