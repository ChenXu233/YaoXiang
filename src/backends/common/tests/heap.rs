//! 堆存储测试 — 基于 RFC-024（spawn 并发模型）
//!
//! Heap 是解释器运行时存储（`src/backends/common/heap.rs`），句柄语义：
//! - 分配 / 读取 / 释放（#278：句柄从 usize 索引改为 Arc<Mutex<HeapValue>>）
//! - 句柄跨线程共享与写回可见（RFC-024 spawn 跨线程调度，#278）
//!
//! #278: 句柄 Arc 化后自包含数据，跨线程捕获 Struct/List 无需共享 Heap

use crate::backends::common::heap::{Heap, HeapValue};
use crate::backends::common::RuntimeValue;

/// allocate 只创建句柄，**不保留**任何强引用（故 `Arc` 计数恰为 1）。
///
/// 这是内存回收的前提：注册表若持强引用，`Arc` 永不归零（实测曾导致
/// `list_ops` 基准常驻 3.9 GB）。
#[test]
fn test_heap_allocate_keeps_no_strong_ref() {
    // Arrange
    let mut heap = Heap::new();

    // Act
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));

    // Assert：堆内没有第二份强引用，计数恰为 1
    assert_eq!(
        std::sync::Arc::strong_count(handle.arc()),
        1,
        "allocate 不应在堆里留下强引用（否则引用计数永不归零）"
    );
    assert!(
        matches!(&*handle.lock(), HeapValue::List(items) if items.len() == 1),
        "分配后应能读到值"
    );
}

#[test]
fn test_handle_lock_reads_stored_value() {
    // Arrange
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));

    // Act
    let guard = handle.lock();

    // Assert
    assert!(
        matches!(&*guard, HeapValue::List(items) if items.len() == 1),
        "lock should expose the stored List with 1 item"
    );
}

/// 最后一个句柄离开作用域 → `Arc` 归零 → 内存即时回收（无需 GC、无需显式释放）。
///
/// 用 `Weak` 观测：强引用全消失后 `upgrade()` 必须失败。
#[test]
fn test_last_handle_drop_frees_value() {
    // Arrange：取一个 Weak 观测点，随后让唯一的强引用离开作用域
    let mut heap = Heap::new();
    let weak = {
        let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));
        std::sync::Arc::downgrade(handle.arc())
    };

    // Act & Assert：唯一强引用已随作用域结束而释放
    assert!(
        weak.upgrade().is_none(),
        "最后一个句柄 drop 后值应立即回收（引用计数归零）"
    );
}

#[test]
fn test_handle_lock_mutates_in_place() {
    // Arrange
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));

    // Act
    if let HeapValue::List(items) = &mut *handle.lock() {
        items.push(RuntimeValue::Int(2));
    }

    // Assert
    assert!(
        matches!(&*handle.lock(), HeapValue::List(items) if items.len() == 2),
        "in-place push should grow the list to 2 items"
    );
}

/// #278：句柄是 Arc——克隆 O(1)、可跨线程使用、写回可见
#[test]
fn test_handle_write_from_another_thread_visible() {
    // Arrange
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(7)]));
    let shared = handle.clone();

    // Act：worker 线程通过克隆句柄写共享列表
    let t = std::thread::spawn(move || {
        if let HeapValue::List(items) = &mut *shared.lock() {
            items.push(RuntimeValue::Int(8));
        }
    });
    t.join().unwrap();

    // Assert：主线程经原句柄读到写回
    assert!(
        matches!(&*handle.lock(), HeapValue::List(items) if items.len() == 2),
        "worker write should be visible through the original handle"
    );
}

#[test]
fn test_handle_clone_equals_original() {
    // Arrange
    let mut heap = Heap::new();
    let original = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));

    // Act
    let clone = original.clone();

    // Assert
    assert_eq!(original, clone);
}

#[test]
fn test_distinct_allocations_are_unequal() {
    // Arrange
    let mut heap = Heap::new();

    // Act：两次分配内容相同的列表
    let a = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));
    let b = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));

    // Assert：身份相等按分配而非内容
    assert_ne!(a, b, "distinct allocations should not be equal");
}
