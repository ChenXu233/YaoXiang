//! 堆存储测试
//!
//! 测试覆盖内容：
//! - Handle 的创建和属性
//! - Heap 的分配、访问、释放
//! - HeapValue 的操作
//! - Handle 跨线程共享（#278）

use crate::backends::common::heap::{Heap, HeapValue};
use crate::backends::common::RuntimeValue;

#[test]
fn test_heap_allocate() {
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));
    assert_eq!(heap.len(), 1);
    assert!(heap.is_valid(&handle));
}

#[test]
fn test_heap_get() {
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));
    let guard = handle.lock();
    match &*guard {
        HeapValue::List(items) => {
            assert_eq!(items.len(), 1);
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn test_heap_deallocate() {
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));
    assert_eq!(heap.len(), 1);
    assert!(heap.deallocate(&handle));
    assert_eq!(heap.len(), 0);
    assert!(!heap.is_valid(&handle));
}

#[test]
fn test_heap_mutate_in_place() {
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));
    {
        let mut guard = handle.lock();
        if let HeapValue::List(items) = &mut *guard {
            items.push(RuntimeValue::Int(2));
        }
    }
    let guard = handle.lock();
    match &*guard {
        HeapValue::List(items) => assert_eq!(items.len(), 2),
        _ => panic!("expected List"),
    }
}

/// #278：句柄是 Arc——克隆 O(1)、可跨线程使用、写回可见
#[test]
fn test_handle_cross_thread_share() {
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(7)]));

    let shared = handle.clone();
    let t = std::thread::spawn(move || {
        let mut guard = shared.lock();
        if let HeapValue::List(items) = &mut *guard {
            items.push(RuntimeValue::Int(8));
        }
    });
    t.join().unwrap();

    let guard = handle.lock();
    match &*guard {
        HeapValue::List(items) => assert_eq!(items.len(), 2),
        _ => panic!("expected List"),
    }
}

/// 句柄相等按指针身份：同一分配 ⇔ 相等；克隆句柄相等；不同分配不等
#[test]
fn test_handle_identity_equality() {
    let mut heap = Heap::new();
    let a = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));
    let a2 = a.clone();
    let b = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));
    assert_eq!(a, a2);
    assert_ne!(a, b);
}
