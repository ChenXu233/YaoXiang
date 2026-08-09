//! 堆存储测试 — 基于 RFC-024（spawn 并发模型）
//!
//! Heap 是解释器运行时存储（`src/backends/common/heap.rs`），句柄语义：
//! - 分配 / 读取 / 释放（#278：句柄从 usize 索引改为 Arc<Mutex<HeapValue>>）
//! - 句柄跨线程共享与写回可见（RFC-024 spawn 跨线程调度，#278）
//!
//! #278: 句柄 Arc 化后自包含数据，跨线程捕获 Struct/List 无需共享 Heap

use crate::backends::common::heap::{Heap, HeapValue};
use crate::backends::common::RuntimeValue;

#[test]
fn test_heap_allocate_tracks_live_handle() {
    // Arrange
    let mut heap = Heap::new();

    // Act
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));

    // Assert
    assert_eq!(heap.len(), 1);
    assert!(
        heap.is_valid(&handle),
        "allocated handle should be tracked as valid"
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

#[test]
fn test_heap_deallocate_untracks_handle() {
    // Arrange
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));

    // Act
    let removed = heap.deallocate(&handle);

    // Assert
    assert!(removed, "deallocate should remove a live handle");
    assert_eq!(heap.len(), 0);
    assert!(
        !heap.is_valid(&handle),
        "deallocated handle should no longer be valid"
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
