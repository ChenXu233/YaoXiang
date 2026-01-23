//! Heap storage with handle-based allocation
//!
//! This module provides a heap allocation system using handles (indices)
//! to enable efficient in-place modification of collection types.

use std::collections::HashMap;
use std::fmt;

/// Handle to a value stored in the heap
///
/// Handles are opaque references that allow mutation of heap-allocated
/// values without cloning. Each handle uniquely identifies a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(pub usize);

impl Handle {
    /// Create a new handle from a raw value
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    /// Get the raw handle value
    pub fn raw(&self) -> usize {
        self.0
    }
}

impl fmt::Display for Handle {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "handle@{}", self.0)
    }
}

/// Heap allocation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapError {
    /// Attempted to access an invalid handle
    InvalidHandle(Handle),
    /// Handle allocation failed (out of handles)
    OutOfHandles,
}

impl fmt::Display for HeapError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            HeapError::InvalidHandle(h) => write!(f, "invalid handle: {}", h),
            HeapError::OutOfHandles => write!(f, "out of handle space"),
        }
    }
}

/// Heap value - storage for collection types
///
/// This enum holds the actual collection data stored on the heap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapValue {
    /// Tuple storage
    Tuple(Vec<super::value::RuntimeValue>),
    /// Array storage
    Array(Vec<super::value::RuntimeValue>),
    /// List storage
    List(Vec<super::value::RuntimeValue>),
    /// Dictionary storage
    Dict(HashMap<super::value::RuntimeValue, super::value::RuntimeValue>),
    /// Struct storage (field values)
    Struct(Vec<super::value::RuntimeValue>),
}

impl HeapValue {
    /// Get the number of elements in this collection
    pub fn len(&self) -> usize {
        match self {
            HeapValue::Tuple(v)
            | HeapValue::Array(v)
            | HeapValue::List(v)
            | HeapValue::Struct(v) => v.len(),
            HeapValue::Dict(m) => m.len(),
        }
    }

    /// Check if this collection is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Heap storage for runtime values
///
/// The heap provides allocation, access, and management of runtime values
/// using handles. This enables:
/// - Efficient in-place modification of collections
/// - Shared references via handle copying
/// - Potential for future garbage collection
#[derive(Debug, Clone)]
pub struct Heap {
    /// Handle generator for allocation
    next_handle: usize,
    /// Handle to value mapping
    values: HashMap<Handle, HeapValue>,
    /// Free list for handle reuse
    free_list: Vec<Handle>,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    /// Create a new empty heap
    pub fn new() -> Self {
        Self {
            next_handle: 0usize,
            values: HashMap::new(),
            free_list: Vec::new(),
        }
    }

    /// Allocate a heap value and return a handle
    pub fn allocate(
        &mut self,
        value: HeapValue,
    ) -> Handle {
        let handle = if let Some(h) = self.free_list.pop() {
            h
        } else {
            let h = Handle(self.next_handle);
            self.next_handle = self.next_handle.wrapping_add(1);
            h
        };
        self.values.insert(handle, value);
        handle
    }

    /// Get an immutable reference to a heap value by handle
    pub fn get(
        &self,
        handle: Handle,
    ) -> Option<&HeapValue> {
        self.values.get(&handle)
    }

    /// Get a mutable reference to a heap value by handle
    pub fn get_mut(
        &mut self,
        handle: Handle,
    ) -> Option<&mut HeapValue> {
        self.values.get_mut(&handle)
    }

    /// Write a heap value to an existing handle
    pub fn write(
        &mut self,
        handle: Handle,
        value: HeapValue,
    ) -> Result<(), HeapError> {
        if let std::collections::hash_map::Entry::Occupied(mut e) = self.values.entry(handle) {
            e.insert(value);
            Ok(())
        } else {
            Err(HeapError::InvalidHandle(handle))
        }
    }

    /// Deallocate a value by handle
    pub fn deallocate(
        &mut self,
        handle: Handle,
    ) -> Option<HeapValue> {
        if self.values.remove(&handle).is_some() {
            self.free_list.push(handle);
            Some(HeapValue::List(vec![]))
        } else {
            None
        }
    }

    /// Check if a handle is valid
    pub fn is_valid(
        &self,
        handle: Handle,
    ) -> bool {
        self.values.contains_key(&handle)
    }

    /// Get the number of allocated values
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the heap is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Clear all allocated values
    pub fn clear(&mut self) {
        self.values.clear();
        self.free_list.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
