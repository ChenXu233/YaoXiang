//! Memory management

use std::mem;

/// Heap using Vec for simple allocation
#[derive(Debug)]
pub struct Heap {
    /// Memory buffer
    buffer: Vec<u8>,
}

impl Heap {
    /// Create a new heap
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(64 * 1024 * 1024), // 64MB default
        }
    }

    /// Allocate memory
    pub fn alloc(&mut self, size: usize) -> Option<usize> {
        let offset = self.buffer.len();
        self.buffer.resize(offset + size, 0);
        Some(offset)
    }

    /// Allocate zeroed memory
    pub fn alloc_zeroed(&mut self, size: usize) -> Option<usize> {
        let offset = self.buffer.len();
        self.buffer.extend_from_slice(&vec![0; size]);
        Some(offset)
    }

    /// Reallocate memory
    pub fn realloc(&mut self, _offset: usize, new_size: usize) -> Option<usize> {
        let offset = self.buffer.len();
        self.buffer.resize(new_size, 0);
        Some(offset)
    }

    /// Deallocate memory (no-op for Vec-based heap)
    pub fn dealloc(&mut self, _offset: usize, _size: usize) {
        // No-op: Vec doesn't support deallocation of individual blocks
    }

    /// Read a value from the heap
    pub fn read<T>(&self, offset: usize) -> &T {
        unsafe { &*self.buffer.as_ptr().add(offset).cast::<T>() }
    }

    /// Write a value to the heap
    pub fn write<T>(&mut self, offset: usize, value: &T) {
        unsafe {
            std::ptr::copy(
                value as *const T as *const u8,
                self.buffer.as_mut_ptr().add(offset),
                mem::size_of::<T>(),
            );
        }
    }

    /// Get total capacity
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Get used size
    pub fn used(&self) -> usize {
        self.buffer.len()
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}
