//! 内存分配器测试
//!
//! 测试覆盖内容：
//! - MemoryLayout 结构体的创建和属性
//! - BumpAllocator 的分配、重置、容量管理
//! - 内存对齐和越界处理

use crate::backends::common::allocator::{
    AllocError, BumpAllocator, MemoryLayout,
};
use core::ptr::NonNull;

#[test]
fn test_memory_layout_new() {
    let layout = MemoryLayout::new::<i64>();
    assert_eq!(layout.size(), 8);
    assert!(layout.align() >= 8);
}

#[test]
fn test_bump_allocator_new() {
    let allocator = BumpAllocator::new();
    assert_eq!(allocator.capacity(), 64 * 1024);
    assert_eq!(allocator.used(), 0);
}

#[test]
fn test_bump_allocator_alloc() {
    let mut allocator = BumpAllocator::new();
    let layout = MemoryLayout::new::<i64>();
    let ptr = allocator.alloc(layout).unwrap();
    unsafe {
        ptr.as_ptr().cast::<i64>().write(42i64);
        assert_eq!(ptr.as_ptr().cast::<i64>().read(), 42i64);
    }
    assert_eq!(allocator.used(), 8);
}

#[test]
fn test_bump_allocator_out_of_memory() {
    let mut allocator = BumpAllocator::with_capacity(100);
    let layout = MemoryLayout::from_size_align(100, 1).unwrap();
    assert!(allocator.alloc(layout).is_ok());
    let layout = MemoryLayout::from_size_align(1, 1).unwrap();
    assert_eq!(allocator.alloc(layout), Err(AllocError::OutOfMemory));
}

#[test]
fn test_bump_allocator_reset() {
    let mut allocator = BumpAllocator::with_capacity(100);
    let layout = MemoryLayout::from_size_align(50, 1).unwrap();
    allocator.alloc(layout).unwrap();
    assert_eq!(allocator.used(), 50);
    allocator.reset();
    assert_eq!(allocator.used(), 0);
}
