//! Standard List library

use std::vec::Vec;

/// Create an empty list
pub fn list_new<T>() -> Vec<T> {
    Vec::new()
}

/// Create a list with capacity
pub fn list_with_capacity<T>(capacity: usize) -> Vec<T> {
    Vec::with_capacity(capacity)
}

/// Get list length
pub fn list_len<T>(list: &[T]) -> usize {
    list.len()
}

/// Check if list is empty
pub fn list_is_empty<T>(list: &[T]) -> bool {
    list.is_empty()
}

/// Get element at index
pub fn list_get<T>(list: &[T], index: usize) -> Option<&T> {
    list.get(index)
}

/// Get mutable element at index
pub fn list_get_mut<T>(list: &mut [T], index: usize) -> Option<&mut T> {
    list.get_mut(index)
}

/// Push element to list
#[allow(clippy::ptr_arg)]
pub fn list_push<T>(list: &mut Vec<T>, value: T) {
    list.push(value);
}

/// Pop element from list
#[allow(clippy::ptr_arg)]
pub fn list_pop<T>(list: &mut Vec<T>) -> Option<T> {
    list.pop()
}

/// Insert element at index
#[allow(clippy::ptr_arg)]
pub fn list_insert<T>(list: &mut Vec<T>, index: usize, value: T) -> bool {
    if index <= list.len() {
        list.insert(index, value);
        true
    } else {
        false
    }
}

/// Remove element at index
#[allow(clippy::ptr_arg)]
pub fn list_remove<T>(list: &mut Vec<T>, index: usize) -> Option<T> {
    if index < list.len() {
        Some(list.remove(index))
    } else {
        None
    }
}

/// Clear list
#[allow(clippy::ptr_arg)]
pub fn list_clear<T>(list: &mut Vec<T>) {
    list.clear()
}

/// Create list from slice
pub fn list_from_slice<T: Clone>(slice: &[T]) -> Vec<T> {
    slice.to_vec()
}
