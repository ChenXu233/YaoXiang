//! Standard List library (YaoXiang)
//!
//! This module provides list manipulation functions for YaoXiang programs.

use crate::backends::common::{RuntimeValue, HeapValue};
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule, NativeHandler};

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

/// Native implementation: push - add item to end of list
/// Returns new list with item added
fn native_push(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "push expects a List as first argument".to_string(),
            ))
        }
    };
    let item = args.get(1).cloned().unwrap_or(RuntimeValue::Unit);

    let mut items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };
    items.push(item);
    let new_handle = ctx.heap.allocate(HeapValue::List(items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: pop - remove and return last item
fn native_pop(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "pop expects a List as first argument".to_string(),
            ))
        }
    };

    let mut items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };

    match items.pop() {
        Some(val) => {
            // Update the list in-place (write back without the last element)
            let _ = ctx.heap.write(list_handle, HeapValue::List(items));
            Ok(val)
        }
        None => Ok(RuntimeValue::Unit),
    }
}

/// Native implementation: append - alias for push
fn native_append(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    native_push(args, ctx)
}

/// Native implementation: prepend - add item to beginning of list
fn native_prepend(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "prepend expects a List as first argument".to_string(),
            ))
        }
    };
    let item = args.get(1).cloned().unwrap_or(RuntimeValue::Unit);

    let mut items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };
    items.insert(0, item);
    let new_handle = ctx.heap.allocate(HeapValue::List(items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: remove_at - remove item at index
fn native_remove_at(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "remove_at expects a List as first argument".to_string(),
            ))
        }
    };
    let index = args.get(1).and_then(|v| v.to_int()).unwrap_or(0) as usize;

    let mut items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };

    if index < items.len() {
        let removed = items.remove(index);
        let _ = ctx.heap.write(list_handle, HeapValue::List(items));
        Ok(removed)
    } else {
        Err(ExecutorError::Runtime(format!(
            "Index {} out of bounds for list of length {}",
            index,
            items.len()
        )))
    }
}

/// Native implementation: reverse - reverse list
fn native_reverse(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "reverse expects a List as first argument".to_string(),
            ))
        }
    };

    let mut items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };
    items.reverse();
    let new_handle = ctx.heap.allocate(HeapValue::List(items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: concat - concatenate two lists
fn native_concat(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let handle_a = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "concat expects a List as first argument".to_string(),
            ))
        }
    };
    let handle_b = match args.get(1) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "concat expects a List as second argument".to_string(),
            ))
        }
    };

    let items_a = match ctx.heap.get(handle_a) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };
    let items_b = match ctx.heap.get(handle_b) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };

    let mut merged = items_a;
    merged.extend(items_b);
    let new_handle = ctx.heap.allocate(HeapValue::List(merged));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: map - apply function to each element
fn native_map(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "map expects a List as first argument".to_string(),
            ))
        }
    };
    let func_value = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::Type("map expects a function as second argument".to_string())
    })?;

    let items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };

    let mut result_items = Vec::with_capacity(items.len());
    for item in items {
        let mapped = ctx.call_function(&func_value, &[item])?;
        result_items.push(mapped);
    }

    let new_handle = ctx.heap.allocate(HeapValue::List(result_items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: filter - keep elements where function returns true
fn native_filter(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "filter expects a List as first argument".to_string(),
            ))
        }
    };
    let func_value = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::Type("filter expects a function as second argument".to_string())
    })?;

    let items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };

    let mut result_items = Vec::new();
    for item in items {
        let result = ctx.call_function(&func_value, &[item.clone()])?;
        if result.to_bool().unwrap_or(false) {
            result_items.push(item);
        }
    }

    let new_handle = ctx.heap.allocate(HeapValue::List(result_items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: reduce - accumulate values
fn native_reduce(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "reduce expects a List as first argument".to_string(),
            ))
        }
    };
    let func_value = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::Type("reduce expects a function as second argument".to_string())
    })?;
    let mut accumulator = args.get(2).cloned().unwrap_or(RuntimeValue::Unit);

    let items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };

    for item in items {
        accumulator = ctx.call_function(&func_value, &[accumulator, item])?;
    }

    Ok(accumulator)
}

/// Native implementation: len - get list length
fn native_len(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => return Ok(RuntimeValue::Int(0)),
    };

    match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => Ok(RuntimeValue::Int(items.len() as i64)),
        _ => Ok(RuntimeValue::Int(0)),
    }
}

/// Native implementation: is_empty - check if list is empty
fn native_is_empty(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => return Ok(RuntimeValue::Bool(true)),
    };

    match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => Ok(RuntimeValue::Bool(items.is_empty())),
        _ => Ok(RuntimeValue::Bool(true)),
    }
}

/// Native implementation: get - get item at index
fn native_get(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "get expects a List as first argument".to_string(),
            ))
        }
    };
    let index = args.get(1).and_then(|v| v.to_int()).unwrap_or(0) as usize;

    match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => Ok(items.get(index).cloned().unwrap_or(RuntimeValue::Unit)),
        _ => Ok(RuntimeValue::Unit),
    }
}

/// Native implementation: set - set item at index
fn native_set(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "set expects a List as first argument".to_string(),
            ))
        }
    };
    let index = args.get(1).and_then(|v| v.to_int()).unwrap_or(0) as usize;
    let value = args.get(2).cloned().unwrap_or(RuntimeValue::Unit);

    let mut items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };

    if index < items.len() {
        items[index] = value;
    }
    let new_handle = ctx.heap.allocate(HeapValue::List(items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: first - get first element
fn native_first(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "first expects a List as first argument".to_string(),
            ))
        }
    };

    match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => Ok(items.first().cloned().unwrap_or(RuntimeValue::Unit)),
        _ => Ok(RuntimeValue::Unit),
    }
}

/// Native implementation: last - get last element
fn native_last(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "last expects a List as first argument".to_string(),
            ))
        }
    };

    match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => Ok(items.last().cloned().unwrap_or(RuntimeValue::Unit)),
        _ => Ok(RuntimeValue::Unit),
    }
}

/// Native implementation: slice - get sublist
fn native_slice(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "slice expects a List as first argument".to_string(),
            ))
        }
    };
    let start = args.get(1).and_then(|v| v.to_int()).unwrap_or(0) as usize;
    let end = args.get(2).and_then(|v| v.to_int()).unwrap_or(i64::MAX) as usize;

    let items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid list handle".to_string())),
    };

    let end = end.min(items.len());
    let start = start.min(end);
    let sliced = items[start..end].to_vec();
    let new_handle = ctx.heap.allocate(HeapValue::List(sliced));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: contains - check if list contains item
fn native_contains(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => return Ok(RuntimeValue::Bool(false)),
    };
    let target = args.get(1).cloned().unwrap_or(RuntimeValue::Unit);

    match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => Ok(RuntimeValue::Bool(items.contains(&target))),
        _ => Ok(RuntimeValue::Bool(false)),
    }
}

/// Native implementation: find_index - find index of item
fn native_find_index(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.get(0) {
        Some(RuntimeValue::List(h)) => *h,
        _ => return Ok(RuntimeValue::Int(-1)),
    };
    let target = args.get(1).cloned().unwrap_or(RuntimeValue::Unit);

    match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => match items.iter().position(|item| item == &target) {
            Some(idx) => Ok(RuntimeValue::Int(idx as i64)),
            None => Ok(RuntimeValue::Int(-1)),
        },
        _ => Ok(RuntimeValue::Int(-1)),
    }
}
