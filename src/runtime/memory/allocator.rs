//! Allocator interface for YaoXiang runtime
//!
//! This module defines the core `Allocator` trait that abstracts memory allocation.
//! Follows RFC-009 ownership model where:
//! - Memory allocation/deallocation is separate from ownership semantics
//! - `ref` keyword uses `Arc` at runtime, not managed by allocator
//! - RAII is handled by Rust's Drop trait
//!
//! # Design Principles
//! - Simple trait: alloc / dealloc / alloc_zeroed
//! - No ownership logic, just raw memory
//! - Send + Sync for thread safety

use core::alloc::Layout;
use core::ptr::NonNull;
use std::fmt;

/// Memory allocation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocError {
    /// Not enough memory to satisfy the allocation
    OutOfMemory,
    /// Alignment requirements cannot be satisfied
    AlignmentError,
}

impl fmt::Display for AllocError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            AllocError::OutOfMemory => write!(f, "out of memory"),
            AllocError::AlignmentError => write!(f, "alignment error"),
        }
    }
}

/// Memory layout helper
///
/// Wraps `std::alloc::Layout` with convenient constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLayout {
    size: usize,
    align: usize,
}

impl MemoryLayout {
    /// Create a layout from size and alignment
    ///
    /// # Returns
    /// `Some(MemoryLayout)` if alignment is valid (power of 2), `None` otherwise.
    pub fn from_size_align(
        size: usize,
        align: usize,
    ) -> Option<Self> {
        if align == 0 || !align.is_power_of_two() {
            return None;
        }

        // Adjust size to meet alignment requirements
        let aligned_size = if size % align == 0 {
            size
        } else {
            (size + align - 1) & !(align - 1)
        };

        Some(Self {
            size: aligned_size,
            align,
        })
    }

    /// Create a layout for type T
    pub fn new<T>() -> Self {
        Self {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }

    /// Get the size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the alignment
    pub fn align(&self) -> usize {
        self.align
    }

    /// Convert to std::alloc::Layout
    pub fn to_std_layout(&self) -> Layout {
        // Safety: size and align are validated in from_size_align
        unsafe { Layout::from_size_align_unchecked(self.size, self.align) }
    }
}

/// Core allocator trait
///
/// Provides a simple interface for memory allocation and deallocation.
/// This trait is separate from ownership semantics - ownership is handled
/// by `RuntimeValue` and the ownership model (RFC-009).
///
/// # Implementations
/// - `BumpAllocator`: High-throughput, low-fragmentation
/// - `GlobalAllocator`: Wraps the global allocator
pub trait Allocator: Send + Sync {
    /// Allocate memory with the given layout
    ///
    /// # Arguments
    /// * `layout` - Size and alignment requirements
    ///
    /// # Returns
    /// `Ok(NonNull<u8>)` on success, `Err(AllocError)` on failure
    fn alloc(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError>;

    /// Allocate zeroed memory with the given layout
    ///
    /// Equivalent to alloc + memset to 0
    ///
    /// # Arguments
    /// * `layout` - Size and alignment requirements
    ///
    /// # Returns
    /// `Ok(NonNull<u8>)` on success, `Err(AllocError)` on failure
    fn alloc_zeroed(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError>;

    /// Deallocate memory
    ///
    /// # Arguments
    /// * `ptr` - Pointer returned by a previous `alloc` or `alloc_zeroed` call
    /// * `layout` - Layout used for the original allocation
    fn dealloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: MemoryLayout,
    );
}

/// Bump allocator for high-throughput allocation
///
/// A bump allocator allocates memory by incrementing a pointer.
/// Deallocations are not supported (memory is reclaimed when the allocator is dropped).
/// This provides:
/// - O(1) allocation
/// - Zero fragmentation (for single-threaded use)
/// - Simple implementation
///
/// # Use Cases
/// - Short-lived allocations
/// - High-throughput scenarios
/// - Testing/fuzzing
///
/// # Note
/// Per RFC-009, `ref` keyword uses `Arc` which manages its own memory.
/// The bump allocator is used for RuntimeValue's heap storage.
#[derive(Debug)]
pub struct BumpAllocator {
    /// Current position in the buffer
    next: usize,
    /// Total capacity
    capacity: usize,
    /// Memory buffer
    buffer: Vec<u8>,
}

impl BumpAllocator {
    /// Create a new bump allocator with default capacity (64KB)
    pub fn new() -> Self {
        Self::with_capacity(64 * 1024)
    }

    /// Create a bump allocator with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next: 0,
            capacity,
            buffer: vec![0u8; capacity],
        }
    }

    /// Get remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity - self.next
    }

    /// Get total capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get used bytes
    pub fn used(&self) -> usize {
        self.next
    }

    /// Reset the allocator (discard all allocations)
    pub fn reset(&mut self) {
        self.next = 0;
    }
}

impl Default for BumpAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Allocator for BumpAllocator {
    fn alloc(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError> {
        let size = layout.size();
        let align = layout.align();

        // Align the next pointer
        let aligned = (self.next + align - 1) & !(align - 1);

        // Check if there's enough space
        if aligned + size > self.capacity {
            return Err(AllocError::OutOfMemory);
        }

        // Allocate
        // Safety: ptr is within buffer bounds, aligned as required
        let ptr = unsafe { self.buffer.as_mut_ptr().add(aligned) };
        self.next = aligned + size;

        // Safety: ptr is valid and aligned
        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }

    fn alloc_zeroed(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError> {
        let ptr = self.alloc(layout)?;
        // Safety: ptr is valid and allocated
        unsafe {
            std::ptr::write_bytes(ptr.as_ptr(), 0, layout.size());
        }
        Ok(ptr)
    }

    fn dealloc(
        &mut self,
        _ptr: NonNull<u8>,
        _layout: MemoryLayout,
    ) {
        // Bump allocator doesn't support individual deallocation
        // Memory is reclaimed when the allocator is dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_trait_object_safe() {
        fn is_send_sync<T: Send + Sync>() {}
        fn check<A: Allocator>() {
            is_send_sync::<A>()
        }
        // This test ensures Allocator is object-safe (Send + Sync)
        check::<BumpAllocator>();
    }

    #[test]
    fn test_memory_layout_new() {
        let layout = MemoryLayout::new::<i64>();
        assert_eq!(layout.size(), 8);
        assert!(layout.align() >= 8);
    }

    #[test]
    fn test_memory_layout_from_size_align() {
        // Size already aligned (100 % 16 = 4, so it needs adjustment)
        // Layout adjusts size to meet alignment requirements
        let layout = MemoryLayout::from_size_align(100, 16).unwrap();
        assert_eq!(layout.size(), 112); // Adjusted to next 16-byte boundary
        assert_eq!(layout.align(), 16);

        // Size not aligned, adjusted to alignment
        let layout = MemoryLayout::from_size_align(7, 8).unwrap();
        assert_eq!(layout.size(), 8); // Adjusted to 8
        assert_eq!(layout.align(), 8);

        // Size already aligned
        let layout = MemoryLayout::from_size_align(16, 8).unwrap();
        assert_eq!(layout.size(), 16); // No adjustment needed
        assert_eq!(layout.align(), 8);

        // Invalid alignment (0)
        assert!(MemoryLayout::from_size_align(100, 0).is_none());

        // Invalid alignment (not power of 2)
        assert!(MemoryLayout::from_size_align(100, 3).is_none());
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

        // Safety: ptr was just allocated, cast to write i64
        unsafe {
            ptr.as_ptr().cast::<i64>().write(42i64);
            assert_eq!(ptr.as_ptr().cast::<i64>().read(), 42i64);
        }

        assert_eq!(allocator.used(), 8);
    }

    #[test]
    fn test_bump_allocator_alignment() {
        let mut allocator = BumpAllocator::new();

        // Allocate with 16-byte alignment
        let layout = MemoryLayout::from_size_align(1, 16).unwrap();
        let ptr = allocator.alloc(layout).unwrap();

        // Check alignment
        assert_eq!(ptr.as_ptr() as usize % 16, 0);
    }

    #[test]
    fn test_bump_allocator_out_of_memory() {
        let mut allocator = BumpAllocator::with_capacity(100);

        // Allocate all available space
        let layout = MemoryLayout::from_size_align(100, 1).unwrap();
        assert!(allocator.alloc(layout).is_ok());

        // Next allocation should fail
        let layout = MemoryLayout::from_size_align(1, 1).unwrap();
        assert_eq!(allocator.alloc(layout), Err(AllocError::OutOfMemory));
    }

    #[test]
    fn test_bump_allocator_alloc_zeroed() {
        let mut allocator = BumpAllocator::new();

        let layout = MemoryLayout::from_size_align(16, 8);
        let ptr = allocator.alloc_zeroed(layout.unwrap()).unwrap();

        // Safety: ptr was just allocated
        unsafe {
            let slice = std::slice::from_raw_parts(ptr.as_ptr(), 16);
            assert!(slice.iter().all(|&b| b == 0));
        }
    }

    #[test]
    fn test_bump_allocator_reset() {
        let mut allocator = BumpAllocator::with_capacity(100);

        let layout = MemoryLayout::from_size_align(50, 1).unwrap();
        allocator.alloc(layout).unwrap();
        assert_eq!(allocator.used(), 50);

        allocator.reset();
        assert_eq!(allocator.used(), 0);

        // Can allocate again
        let layout = MemoryLayout::from_size_align(50, 1).unwrap();
        assert!(allocator.alloc(layout).is_ok());
    }
}
