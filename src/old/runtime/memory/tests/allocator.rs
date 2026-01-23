//! Allocator integration tests
//!
//! Tests the Allocator trait implementation with Heap and RuntimeValue integration.
//! Follows RFC-009 ownership model.

use crate::runtime::memory::{Allocator, BumpAllocator, Heap, MemoryLayout};
use std::alloc::NonNull;

/// Test basic Allocator trait implementation for Heap
#[test]
fn test_heap_allocator_alloc() {
    let mut heap = Heap::new();

    let layout = MemoryLayout::from_size_align(64, 8).unwrap();
    let ptr = heap.alloc(layout).unwrap();

    assert!(!ptr.as_ptr().is_null());

    // Write and read
    unsafe {
        ptr.as_ptr().write(42i64);
        assert_eq!(ptr.as_ptr().read(), 42i64);
    }
}

/// Test alloc_zeroed for Heap
#[test]
fn test_heap_allocator_alloc_zeroed() {
    let mut heap = Heap::new();

    let layout = MemoryLayout::from_size_align(64, 8).unwrap();
    let ptr = heap.alloc_zeroed(layout).unwrap();

    // Check memory is zeroed
    unsafe {
        let slice = std::slice::from_raw_parts(ptr.as_ptr(), 64);
        assert!(slice.iter().all(|&b| b == 0));
    }
}

/// Test dealloc for Heap (noop for region-based)
#[test]
fn test_heap_allocator_dealloc() {
    let mut heap = Heap::new();

    let layout = MemoryLayout::from_size_align(64, 8).unwrap();
    let ptr = heap.alloc(layout).unwrap();

    // dealloc is a noop for region-based allocation
    // Memory is reclaimed when Heap is dropped
    heap.dealloc(ptr, layout);

    // Should still be able to allocate more
    let layout = MemoryLayout::from_size_align(32, 8).unwrap();
    assert!(heap.alloc(layout).is_ok());
}

/// Test Heap Allocator out of memory
#[test]
fn test_heap_allocator_out_of_memory() {
    let mut heap = Heap::with_capacity(100);

    // Allocate until full
    for _ in 0..10 {
        let layout = MemoryLayout::from_size_align(64, 1).unwrap();
        let _ = heap.alloc(layout);
    }

    // Next allocation should fail
    let layout = MemoryLayout::from_size_align(64, 1).unwrap();
    assert_eq!(heap.alloc(layout), Err(crate::runtime::memory::AllocError::OutOfMemory));
}

/// Test BumpAllocator with Allocator trait
#[test]
fn test_bump_allocator_trait() {
    let mut allocator = BumpAllocator::new();

    let layout = MemoryLayout::from_size_align(100, 16).unwrap();
    let ptr = allocator.alloc(layout).unwrap();

    // Check alignment
    assert_eq!(ptr.as_ptr() as usize % 16, 0);

    // Write value
    unsafe {
        ptr.as_ptr().write(123i32);
        assert_eq!(ptr.as_ptr().read(), 123i32);
    }
}

/// Test BumpAllocator alloc_zeroed
#[test]
fn test_bump_allocator_zeroed() {
    let mut allocator = BumpAllocator::new();

    let layout = MemoryLayout::from_size_align(100, 8).unwrap();
    let ptr = allocator.alloc_zeroed(layout).unwrap();

    unsafe {
        let slice = std::slice::from_raw_parts(ptr.as_ptr(), 100);
        assert!(slice.iter().all(|&b| b == 0));
    }
}

/// Test Allocator trait object safety (Send + Sync)
#[test]
fn test_allocator_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Heap>();
    assert_send_sync::<BumpAllocator>();
}

/// Test memory layout alignment
#[test]
fn test_memory_layout_alignment() {
    // Test various alignments
    for align in [1, 2, 4, 8, 16, 32] {
        let layout = MemoryLayout::from_size_align(10, align).unwrap();
        assert_eq!(layout.align(), align);
    }

    // Test invalid alignments
    assert!(MemoryLayout::from_size_align(10, 0).is_none());
    assert!(MemoryLayout::from_size_align(10, 3).is_none());
    assert!(MemoryLayout::from_size_align(10, 7).is_none());
}

/// Test size adjustment for alignment
#[test]
fn test_memory_layout_size_adjustment() {
    // Size not aligned
    let layout = MemoryLayout::from_size_align(7, 8).unwrap();
    assert_eq!(layout.size(), 8);

    // Size already aligned
    let layout = MemoryLayout::from_size_align(16, 8).unwrap();
    assert_eq!(layout.size(), 16);
}

/// Test allocator reset behavior
#[test]
fn test_allocator_reset() {
    let mut allocator = BumpAllocator::with_capacity(200);

    let layout = MemoryLayout::from_size_align(100, 1).unwrap();
    assert!(allocator.alloc(layout).is_ok());
    assert_eq!(allocator.used(), 100);

    allocator.reset();
    assert_eq!(allocator.used(), 0);

    // Can allocate again
    let layout = MemoryLayout::from_size_align(100, 1).unwrap();
    assert!(allocator.alloc(layout).is_ok());
}

/// Test region rollover behavior for Heap
#[test]
fn test_heap_region_rollover() {
    let mut heap = Heap::with_capacity(100);

    // Fill the first region
    let layout = MemoryLayout::from_size_align(50, 1).unwrap();
    let _ = heap.alloc(layout);
    let _ = heap.alloc(layout);

    assert_eq!(heap.used(), 100);

    // Next allocation triggers region rollover
    let layout = MemoryLayout::from_size_align(30, 1).unwrap();
    let ptr = heap.alloc(layout).unwrap();

    // Should have created a new region
    assert!(heap.capacity() > 100);
    assert!(!ptr.as_ptr().is_null());
}
