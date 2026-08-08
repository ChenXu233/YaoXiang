//! Heap storage with handle-based allocation
//!
//! This module provides a heap allocation system using handles.
//!
//! # 跨线程语义（#278）
//!
//! `Handle` 是 `Arc<Mutex<HeapValue>>` 的包装：句柄自包含数据，
//! 拷贝句柄 = Arc 复制 O(1)。Standard 模式下 spawn 捕获的
//! Struct/List 直接跨线程有效（写回可见），无需共享 Heap 本身。
//!
//! `Heap` 退化为分配注册表：追踪活句柄（`is_valid` / `len` / `deallocate` /
//! `clear` 语义）。内存回收由 Arc 引用计数完成——最后一个句柄（含注册表
//! 强引用）释放时数据即被回收。

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

/// Handle to a value stored in the heap
///
/// Handles are self-contained references that allow mutation of heap-allocated
/// values without cloning. Cloning a handle is O(1) (Arc copy) and is safe to
/// send across threads.
#[derive(Clone)]
pub struct Handle(Arc<Mutex<HeapValue>>);

impl Handle {
    /// Create a new handle wrapping a heap value
    pub fn new(value: HeapValue) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    /// Lock the heap value for read or write access.
    ///
    /// Recovers from mutex poisoning (a panicking thread can never leave a
    /// heap value in a state that corrupts the interpreter).
    pub fn lock(&self) -> MutexGuard<'_, HeapValue> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Raw identity value (pointer address), for diagnostics only.
    pub fn raw(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

// 句柄相等/哈希按 Arc 指针身份（与旧 usize 索引语义一致：同一分配 ⇔ 相等）。
// 不能按内容：两个内容相同的独立分配是不同对象。
impl PartialEq for Handle {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Handle {}

impl std::hash::Hash for Handle {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

impl fmt::Debug for Handle {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_tuple("Handle")
            .field(&(self.raw() as *const ()))
            .finish()
    }
}

impl fmt::Display for Handle {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "handle@{:#x}", self.raw())
    }
}

/// Heap value - storage for collection types
///
/// This enum holds the actual collection data stored on the heap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapValue {
    /// Tuple storage
    Tuple(Vec<super::value::RuntimeValue>),
    /// Array storage
    Array(Vec<super::value::RuntimeValue>),
    /// List storage
    List(Vec<super::value::RuntimeValue>),
    /// Dictionary storage
    Dict(HashMap<super::value::RuntimeValue, super::value::RuntimeValue>),
}

impl HeapValue {
    /// Get the number of elements in this collection
    pub fn len(&self) -> usize {
        match self {
            HeapValue::Tuple(v) | HeapValue::Array(v) | HeapValue::List(v) => v.len(),
            HeapValue::Dict(m) => m.len(),
        }
    }
}

/// Heap storage for runtime values
///
/// 分配注册表：追踪活句柄。数据本体在 `Handle` 的 `Arc` 内。
#[derive(Debug, Clone, Default)]
pub struct Heap {
    /// Live handles (strong refs keep values alive until deallocate/clear)
    allocated: HashSet<Handle>,
}

impl Heap {
    /// Create a new empty heap
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a heap value and return a handle
    pub fn allocate(
        &mut self,
        value: HeapValue,
    ) -> Handle {
        let handle = Handle::new(value);
        self.allocated.insert(handle.clone());
        handle
    }

    /// Deallocate a value by handle.
    ///
    /// Removes the registry's strong reference; the underlying value is freed
    /// once no other handles reference it.
    pub fn deallocate(
        &mut self,
        handle: &Handle,
    ) -> bool {
        self.allocated.remove(handle)
    }

    /// Check if a handle is valid
    pub fn is_valid(
        &self,
        handle: &Handle,
    ) -> bool {
        self.allocated.contains(handle)
    }

    /// Get the number of allocated values
    pub fn len(&self) -> usize {
        self.allocated.len()
    }

    /// Clear all allocated values
    pub fn clear(&mut self) {
        self.allocated.clear();
    }
}
