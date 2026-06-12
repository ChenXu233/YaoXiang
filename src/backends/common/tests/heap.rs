//! 堆存储测试
//!
//! 测试覆盖内容：
//! - Handle 的创建和属性
//! - Heap 的分配、访问、释放
//! - HeapValue 的操作

use crate::backends::common::heap::{Handle, Heap, HeapValue};
use crate::backends::common::RuntimeValue;

#[test]
fn test_heap_allocate() {
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));
    assert_eq!(heap.len(), 1);
    assert!(heap.is_valid(handle));
}

#[test]
fn test_heap_get() {
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(42)]));
    let value = heap.get(handle);
    match value {
        Some(HeapValue::List(items)) => {
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
    heap.deallocate(handle);
    assert_eq!(heap.len(), 0);
    assert!(!heap.is_valid(handle));
}
