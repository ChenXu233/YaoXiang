//! Standard List library (YaoXiang)
//!
//! This module provides list manipulation functions for YaoXiang programs.

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeExport, StdModule, NativeHandler};

// ============================================================================
// ListModule - StdModule Implementation
// ============================================================================

/// List module implementation.
pub struct ListModule;

impl Default for ListModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for ListModule {
    fn module_path(&self) -> &str {
        "std.list"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new(
                "push",
                "std.list.push",
                "(list: List, item: Any) -> List",
                native_push as NativeHandler,
            ),
            NativeExport::new(
                "pop",
                "std.list.pop",
                "(list: List) -> Any",
                native_pop as NativeHandler,
            ),
            NativeExport::new(
                "append",
                "std.list.append",
                "(list: List, item: Any) -> List",
                native_append as NativeHandler,
            ),
            NativeExport::new(
                "prepend",
                "std.list.prepend",
                "(list: List, item: Any) -> List",
                native_prepend as NativeHandler,
            ),
            NativeExport::new(
                "remove_at",
                "std.list.remove_at",
                "(list: List, index: Int) -> Any",
                native_remove_at as NativeHandler,
            ),
            NativeExport::new(
                "reverse",
                "std.list.reverse",
                "(list: List) -> List",
                native_reverse as NativeHandler,
            ),
            NativeExport::new(
                "concat",
                "std.list.concat",
                "(a: List, b: List) -> List",
                native_concat as NativeHandler,
            ),
            NativeExport::new(
                "map",
                "std.list.map",
                "(list: List, fn: Fn) -> List",
                native_map as NativeHandler,
            ),
            NativeExport::new(
                "filter",
                "std.list.filter",
                "(list: List, fn: Fn) -> List",
                native_filter as NativeHandler,
            ),
            NativeExport::new(
                "reduce",
                "std.list.reduce",
                "(list: List, fn: Fn, init: Any) -> Any",
                native_reduce as NativeHandler,
            ),
            NativeExport::new(
                "len",
                "std.list.len",
                "(list: List) -> Int",
                native_len as NativeHandler,
            ),
            NativeExport::new(
                "is_empty",
                "std.list.is_empty",
                "(list: List) -> Bool",
                native_is_empty as NativeHandler,
            ),
            NativeExport::new(
                "get",
                "std.list.get",
                "(list: List, index: Int) -> Any",
                native_get as NativeHandler,
            ),
            NativeExport::new(
                "set",
                "std.list.set",
                "(list: List, index: Int, value: Any) -> List",
                native_set as NativeHandler,
            ),
            NativeExport::new(
                "first",
                "std.list.first",
                "(list: List) -> Any",
                native_first as NativeHandler,
            ),
            NativeExport::new(
                "last",
                "std.list.last",
                "(list: List) -> Any",
                native_last as NativeHandler,
            ),
            NativeExport::new(
                "slice",
                "std.list.slice",
                "(list: List, start: Int, end: Int) -> List",
                native_slice as NativeHandler,
            ),
            NativeExport::new(
                "contains",
                "std.list.contains",
                "(list: List, item: Any) -> Bool",
                native_contains as NativeHandler,
            ),
            NativeExport::new(
                "find_index",
                "std.list.find_index",
                "(list: List, item: Any) -> Int",
                native_find_index as NativeHandler,
            ),
        ]
    }
}

/// Singleton instance for std.list module.
pub const LIST_MODULE: ListModule = ListModule;

// ============================================================================
// Native function implementations
// ============================================================================

/// Native implementation: push - add item to end of list (consumes original)
/// Returns new list with item added
fn native_push(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return new List
    // For now, return a placeholder
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: pop - remove and return last item
fn native_pop(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to modify and return
    Ok(RuntimeValue::Unit)
}

/// Native implementation: append - alias for push
fn native_append(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    native_push(args)
}

/// Native implementation: prepend - add item to beginning of list
fn native_prepend(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return new List
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: remove_at - remove item at index
fn native_remove_at(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Unit)
}

/// Native implementation: reverse - reverse list (consumes original)
fn native_reverse(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return new List
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: concat - concatenate two lists
fn native_concat(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return new List
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: map - apply function to each element
fn native_map(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access and function call support
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: filter - keep elements where function returns true
fn native_filter(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access and function call support
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: reduce - accumulate values
fn native_reduce(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access and function call support
    Ok(RuntimeValue::Unit)
}

/// Native implementation: len - get list length
fn native_len(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    match args.get(0) {
        Some(RuntimeValue::List(handle)) => {
            // We can't access heap here without passing it
            // Return a placeholder
            Ok(RuntimeValue::Int(0))
        }
        _ => Ok(RuntimeValue::Int(0)),
    }
}

/// Native implementation: is_empty - check if list is empty
fn native_is_empty(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Bool(true))
}

/// Native implementation: get - get item at index
fn native_get(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Unit)
}

/// Native implementation: set - set item at index (consumes original)
fn native_set(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return new List
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: first - get first element
fn native_first(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Unit)
}

/// Native implementation: last - get last element
fn native_last(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Unit)
}

/// Native implementation: slice - get sublist
fn native_slice(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: contains - check if list contains item
fn native_contains(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access for proper comparison
    Ok(RuntimeValue::Bool(false))
}

/// Native implementation: find_index - find index of item
fn native_find_index(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Int(-1))
}
