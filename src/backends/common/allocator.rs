//! Memory allocator interface
//!
//! This module defines the core `Allocator` trait that abstracts memory allocation.

use core::alloc::Layout;
use core::ptr::NonNull;
use std::fmt;

/// Memory allocation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocError {
    /// Not enough memory
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLayout {
    size: usize,
    align: usize,
}

impl MemoryLayout {
    /// Create a layout from size and alignment
    pub fn from_size_align(
        size: usize,
        align: usize,
    ) -> Option<Self> {
        if align == 0 || !align.is_power_of_two() {
            return None;
        }
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
        unsafe { Layout::from_size_align_unchecked(self.size, self.align) }
    }
}

/// Core allocator trait
pub trait Allocator: Send + Sync {
    /// Allocate memory with the given layout
    fn alloc(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError>;

    /// Allocate zeroed memory
    fn alloc_zeroed(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError>;

    /// Deallocate memory
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
#[derive(Debug)]
pub struct BumpAllocator {
    /// Current position in the buffer
    next: usize,
    /// Total capacity
    capacity: usize,
    /// Memory buffer
    pub buffer: Vec<u8>,
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

    /// Reset the allocator
    pub fn reset(&mut self) {
        self.next = 0;
    }

    /// Get mutable access to the internal buffer
    pub fn memory_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
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

        let aligned = (self.next + align - 1) & !(align - 1);

        if aligned + size > self.capacity {
            return Err(AllocError::OutOfMemory);
        }

        let ptr = unsafe { self.buffer.as_mut_ptr().add(aligned) };
        self.next = aligned + size;

        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }

    fn alloc_zeroed(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError> {
        let ptr = self.alloc(layout)?;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
