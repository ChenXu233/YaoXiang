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
//! `Heap` **不持有任何强引用**：分配只创建 `Handle`（`Arc` 自带引用计数），
//! 内存回收完全由 `Arc` 完成——最后一个 `Handle` 离开作用域即释放。
//!
//! # 为何不再是「分配注册表」
//!
//! 早期版本用 `allocated: HashSet<Handle>` 追踪活句柄，但那个集合**自身持强
//! 引用**，而解释器执行期从不 `deallocate`（仅 `reset()` 时 `clear`）。于是
//! 每个分配至少被注册表钉住一份，`Arc` 计数永不归零，**引用计数形同虚设**：
//! 实测 `list_ops` 基准（每轮 `xs = xs + [x]` 建 1000 元素列表 ×500 轮）
//! 常驻内存涨到约 3.9 GB——每个 `concat` 新建的列表都被永久保留。
//!
//! 而注册表的读接口（`is_valid` / `len` / `is_empty`）在生产代码中**零调用**，
//! 仅测试使用，即该集合的唯一实际效果就是泄漏。故整体移除。

use std::collections::HashMap;
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

    /// 内部 `Arc` 的只读借用——供测试观测引用计数 / 构造 `Weak` 用。
    ///
    /// 生产代码不应使用（句柄语义已足够）；暴露它是为了让测试能断言
    /// 「最后一个句柄释放即回收」这一内存模型契约。
    pub fn arc(&self) -> &Arc<Mutex<HeapValue>> {
        &self.0
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

    /// Whether this collection is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Heap storage for runtime values.
///
/// **无状态**：数据本体在 `Handle` 的 `Arc` 内，生命周期由 `Arc` 引用计数管理。
/// 保留本类型是为了沿用 `heap.allocate(...)` 的调用形式（28 处）与后续可能
/// 引入的分配策略（如 arena / 上限统计），而非承载状态。
#[derive(Debug, Clone, Copy, Default)]
pub struct Heap;

impl Heap {
    /// Create a new empty heap
    pub fn new() -> Self {
        Self
    }

    /// Allocate a heap value and return a handle.
    ///
    /// 只创建 `Handle`，不登记任何强引用——释放时机完全由持有者决定，
    /// 最后一个 `Handle` 离开作用域即回收。
    pub fn allocate(
        &mut self,
        value: HeapValue,
    ) -> Handle {
        Handle::new(value)
    }
}
