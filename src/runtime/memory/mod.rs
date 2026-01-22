//! Memory management with ownership model
//!
//! This module implements region-based memory allocation for the ownership model.
//! Memory is allocated in regions, and entire regions are deallocated at once
//! when the scope ends, providing deterministic memory management without GC.
//!
//! # RFC-009 Compliance
//! - Memory allocation/deallocation is separate from ownership semantics
//! - `ref` keyword uses `Arc` at runtime (see runtime_value)
//! - RAII is handled by Rust's Drop trait

mod allocator;

pub use allocator::{AllocError, Allocator, BumpAllocator, MemoryLayout};

use std::mem;

/// A memory region for scope-based allocation
///
/// Objects allocated in a region are deallocated together when the region
/// is dropped, providing zero-cost deterministic cleanup.
#[derive(Debug)]
pub struct Region {
    /// Memory buffer for this region
    buffer: Vec<u8>,
    /// Allocation markers for cleanup
    markers: Vec<AllocationMarker>,
}

#[derive(Debug, Clone)]
struct AllocationMarker {
    /// Offset in the buffer
    offset: usize,
    /// Size of the allocation
    size: usize,
}

impl Region {
    /// Create a new region with default capacity
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(64 * 1024), // 64KB default
            markers: Vec::new(),
        }
    }

    /// Allocate memory in this region
    ///
    /// Returns the offset in the buffer, or None if allocation fails.
    pub fn alloc(
        &mut self,
        size: usize,
    ) -> Option<usize> {
        let offset = self.buffer.len();

        // Ensure capacity
        if self.buffer.len() + size > self.buffer.capacity() {
            // Try to grow the buffer
            let new_capacity = (self.buffer.capacity() * 2).max(offset + size);
            self.buffer.reserve(new_capacity - self.buffer.capacity());
        }

        // Resize buffer and zero-initialize
        let new_len = self.buffer.len() + size;
        self.buffer.resize(new_len, 0);

        // Record the allocation marker for cleanup
        self.markers.push(AllocationMarker { offset, size });

        Some(offset)
    }

    /// Read a value from the region
    pub fn read<T>(
        &self,
        offset: usize,
    ) -> &T {
        unsafe { &*self.buffer.as_ptr().add(offset).cast::<T>() }
    }

    /// Write a value to the region
    pub fn write<T>(
        &mut self,
        offset: usize,
        value: &T,
    ) {
        unsafe {
            std::ptr::copy(
                value as *const T as *const u8,
                self.buffer.as_mut_ptr().add(offset),
                mem::size_of::<T>(),
            );
        }
    }

    /// Get a mutable reference to a value in the region
    pub fn get_mut<T>(
        &mut self,
        offset: usize,
    ) -> &mut T {
        unsafe { &mut *self.buffer.as_mut_ptr().add(offset).cast::<T>() }
    }

    /// Get total capacity of the region
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Get used size of the region
    pub fn used(&self) -> usize {
        self.buffer.len()
    }

    /// Get remaining capacity
    pub fn remaining(&self) -> usize {
        self.buffer.capacity() - self.buffer.len()
    }
}

impl Default for Region {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // Simply drop the buffer - individual allocations are not tracked for destruction
        // In a full implementation with typed objects, we would iterate markers and call drop functions
        // But for this simple byte-region implementation, the Vec's automatic cleanup is sufficient
    }
}

/// Heap using region-based allocation
///
/// The heap manages multiple regions for different scopes.
#[derive(Debug)]
pub struct Heap {
    /// Current region for allocations
    current_region: Region,
    /// Free regions for reuse
    free_regions: Vec<Region>,
    /// Total allocations
    total_allocations: usize,
}

impl Heap {
    /// Create a new heap
    pub fn new() -> Self {
        Self {
            current_region: Region::new(),
            free_regions: Vec::new(),
            total_allocations: 0,
        }
    }

    /// Allocate memory on the heap
    ///
    /// Returns the offset in the buffer, or None if allocation fails.
    /// This is the internal method that works with raw sizes.
    /// For the Allocator trait implementation, see `Allocator::alloc`.
    pub fn alloc(
        &mut self,
        size: usize,
    ) -> Option<usize> {
        // Try to allocate in current region
        if let Some(offset) = self.current_region.alloc(size) {
            self.total_allocations += 1;
            return Some(offset);
        }

        // Current region is full, create a new one
        self.free_regions
            .push(std::mem::take(&mut self.current_region));

        // Try again with new region
        self.current_region.alloc(size).inspect(|_offset| {
            self.total_allocations += 1;
        })
    }

    /// Read a value from the heap
    pub fn read<T>(
        &self,
        offset: usize,
    ) -> &T {
        self.current_region.read(offset)
    }

    /// Write a value to the heap
    pub fn write<T>(
        &mut self,
        offset: usize,
        value: &T,
    ) {
        self.current_region.write(offset, value);
    }

    /// Get total capacity
    pub fn capacity(&self) -> usize {
        self.current_region.capacity()
            + self
                .free_regions
                .iter()
                .map(|r| r.capacity())
                .sum::<usize>()
    }

    /// Get used size
    pub fn used(&self) -> usize {
        self.current_region.used() + self.free_regions.iter().map(|r| r.used()).sum::<usize>()
    }

    /// Get total allocations
    pub fn total_allocations(&self) -> usize {
        self.total_allocations
    }

    /// Force cleanup of all free regions
    pub fn cleanup(&mut self) {
        self.free_regions.clear();
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Allocator trait implementation
// ============================================================================

use core::ptr::NonNull;

impl Allocator for Heap {
    fn alloc(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError> {
        let size = layout.size();

        // Try to allocate in current region
        if let Some(offset) = Heap::alloc(self, size) {
            // Safety: offset is valid within the buffer
            let ptr = unsafe {
                NonNull::new_unchecked(self.current_region.buffer.as_mut_ptr().add(offset))
            };
            return Ok(ptr);
        }

        // Current region is full, create a new one
        self.free_regions
            .push(std::mem::take(&mut self.current_region));

        // Try again with new region
        if let Some(offset) = Heap::alloc(self, size) {
            self.total_allocations += 1;
            // Safety: offset is valid within the new buffer
            let ptr = unsafe {
                NonNull::new_unchecked(self.current_region.buffer.as_mut_ptr().add(offset))
            };
            return Ok(ptr);
        }

        Err(AllocError::OutOfMemory)
    }

    fn alloc_zeroed(
        &mut self,
        layout: MemoryLayout,
    ) -> Result<NonNull<u8>, AllocError> {
        let ptr = Allocator::alloc(self, layout)?;
        // Safety: ptr is valid and allocated
        unsafe {
            std::ptr::write_bytes(ptr.as_ptr(), 0, layout.size());
        }
        Ok(ptr)
    }

    fn dealloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: MemoryLayout,
    ) {
        // Region-based allocation doesn't support individual deallocation
        // Memory is reclaimed when regions are dropped (on cleanup or Heap drop)
        let _ = ptr;
        let _ = layout;
    }
}

#[cfg(test)]
mod tests;
