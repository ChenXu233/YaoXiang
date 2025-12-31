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
pub fn list_len<T>(list: &Vec<T>) -> usize {
    list.len()
}

/// Check if list is empty
pub fn list_is_empty<T>(list: &Vec<T>) -> bool {
    list.is_empty()
}

/// Get element at index
pub fn list_get<T>(list: &Vec<T>, index: usize) -> Option<&T> {
    list.get(index)
}

/// Get mutable element at index
pub fn list_get_mut<T>(list: &mut Vec<T>, index: usize) -> Option<&mut T> {
    list.get_mut(index)
}

/// Push element to list
pub fn list_push<T>(list: &mut Vec<T>, value: T) {
    list.push(value);
}

/// Pop element from list
pub fn list_pop<T>(list: &mut Vec<T>) -> Option<T> {
    list.pop()
}

/// Insert element at index
pub fn list_insert<T>(list: &mut Vec<T>, index: usize, value: T) -> bool {
    if index <= list.len() {
        list.insert(index, value);
        true
    } else {
        false
    }
}

/// Remove element at index
pub fn list_remove<T>(list: &mut Vec<T>, index: usize) -> Option<T> {
    if index < list.len() {
        Some(list.remove(index))
    } else {
        None
    }
}

/// Clear list
pub fn list_clear<T>(list: &mut Vec<T>) {
    list.clear()
}

/// Create list from slice
pub fn list_from_slice<T: Clone>(slice: &[T]) -> Vec<T> {
    slice.to_vec()
}
