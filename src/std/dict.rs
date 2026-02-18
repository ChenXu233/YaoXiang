//! Standard Dict library (YaoXiang)
//!
//! This module provides dictionary manipulation functions for YaoXiang programs.

use crate::backends::common::{RuntimeValue, HeapValue};
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule, NativeHandler};

// ============================================================================
// DictModule - StdModule Implementation
// ============================================================================

/// Dict module implementation.
pub struct DictModule;

impl Default for DictModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for DictModule {
    fn module_path(&self) -> &str {
        "std.dict"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new(
                "get",
                "std.dict.get",
                "(dict: Dict, key: Any) -> Any",
                native_get as NativeHandler,
            ),
            NativeExport::new(
                "set",
                "std.dict.set",
                "(dict: Dict, key: Any, value: Any) -> Dict",
                native_set as NativeHandler,
            ),
            NativeExport::new(
                "has",
                "std.dict.has",
                "(dict: Dict, key: Any) -> Bool",
                native_has as NativeHandler,
            ),
            NativeExport::new(
                "values",
                "std.dict.values",
                "(dict: Dict) -> List",
                native_values as NativeHandler,
            ),
            NativeExport::new(
                "keys",
                "std.dict.keys",
                "(dict: Dict) -> List",
                native_keys as NativeHandler,
            ),
            NativeExport::new(
                "entries",
                "std.dict.entries",
                "(dict: Dict) -> List",
                native_entries as NativeHandler,
            ),
            NativeExport::new(
                "delete",
                "std.dict.delete",
                "(dict: Dict, key: Any) -> Dict",
                native_delete as NativeHandler,
            ),
            NativeExport::new(
                "len",
                "std.dict.len",
                "(dict: Dict) -> Int",
                native_len as NativeHandler,
            ),
            NativeExport::new(
                "is_empty",
                "std.dict.is_empty",
                "(dict: Dict) -> Bool",
                native_is_empty as NativeHandler,
            ),
            NativeExport::new(
                "merge",
                "std.dict.merge",
                "(a: Dict, b: Dict) -> Dict",
                native_merge as NativeHandler,
            ),
        ]
    }
}

/// Singleton instance for std.dict module.
pub const DICT_MODULE: DictModule = DictModule;

// ============================================================================
// Native function implementations
// ============================================================================

/// Native implementation: get - get value by key
fn native_get(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "dict.get expects a Dict as first argument".to_string(),
            ))
        }
    };
    let key = args.get(1).cloned().unwrap_or(RuntimeValue::Unit);

    match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => Ok(map.get(&key).cloned().unwrap_or(RuntimeValue::Unit)),
        _ => Ok(RuntimeValue::Unit),
    }
}

/// Native implementation: set - set key-value pair (returns new dict)
fn native_set(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "dict.set expects a Dict as first argument".to_string(),
            ))
        }
    };
    let key = args.get(1).cloned().unwrap_or(RuntimeValue::Unit);
    let value = args.get(2).cloned().unwrap_or(RuntimeValue::Unit);

    let mut map = match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => map.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid dict handle".to_string())),
    };
    map.insert(key, value);
    let new_handle = ctx.heap.allocate(HeapValue::Dict(map));
    Ok(RuntimeValue::Dict(new_handle))
}

/// Native implementation: has - check if key exists
fn native_has(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => return Ok(RuntimeValue::Bool(false)),
    };
    let key = args.get(1).cloned().unwrap_or(RuntimeValue::Unit);

    match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => Ok(RuntimeValue::Bool(map.contains_key(&key))),
        _ => Ok(RuntimeValue::Bool(false)),
    }
}

/// Native implementation: values - get all values as list
fn native_values(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "dict.values expects a Dict as first argument".to_string(),
            ))
        }
    };

    let values: Vec<RuntimeValue> = match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => map.values().cloned().collect(),
        _ => Vec::new(),
    };
    let list_handle = ctx.heap.allocate(HeapValue::List(values));
    Ok(RuntimeValue::List(list_handle))
}

/// Native implementation: keys - get all keys as list
fn native_keys(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "dict.keys expects a Dict as first argument".to_string(),
            ))
        }
    };

    let keys: Vec<RuntimeValue> = match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => map.keys().cloned().collect(),
        _ => Vec::new(),
    };
    let list_handle = ctx.heap.allocate(HeapValue::List(keys));
    Ok(RuntimeValue::List(list_handle))
}

/// Native implementation: entries - get all key-value pairs as list of tuples
fn native_entries(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "dict.entries expects a Dict as first argument".to_string(),
            ))
        }
    };

    let map = match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => map.clone(),
        _ => {
            return Ok(RuntimeValue::List(
                ctx.heap.allocate(HeapValue::List(Vec::new())),
            ))
        }
    };

    let entries: Vec<RuntimeValue> = map
        .iter()
        .map(|(k, v)| {
            let tuple_handle = ctx
                .heap
                .allocate(HeapValue::Tuple(vec![k.clone(), v.clone()]));
            RuntimeValue::Tuple(tuple_handle)
        })
        .collect();

    let list_handle = ctx.heap.allocate(HeapValue::List(entries));
    Ok(RuntimeValue::List(list_handle))
}

/// Native implementation: delete - remove key-value pair (returns new dict)
fn native_delete(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "dict.delete expects a Dict as first argument".to_string(),
            ))
        }
    };
    let key = args.get(1).cloned().unwrap_or(RuntimeValue::Unit);

    let mut map = match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => map.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid dict handle".to_string())),
    };
    map.remove(&key);
    let new_handle = ctx.heap.allocate(HeapValue::Dict(map));
    Ok(RuntimeValue::Dict(new_handle))
}

/// Native implementation: len - get number of entries
fn native_len(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => return Ok(RuntimeValue::Int(0)),
    };

    match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => Ok(RuntimeValue::Int(map.len() as i64)),
        _ => Ok(RuntimeValue::Int(0)),
    }
}

/// Native implementation: is_empty - check if dict is empty
fn native_is_empty(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => return Ok(RuntimeValue::Bool(true)),
    };

    match ctx.heap.get(dict_handle) {
        Some(HeapValue::Dict(map)) => Ok(RuntimeValue::Bool(map.is_empty())),
        _ => Ok(RuntimeValue::Bool(true)),
    }
}

/// Native implementation: merge - merge two dicts (second overrides first)
fn native_merge(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let handle_a = match args.get(0) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "dict.merge expects a Dict as first argument".to_string(),
            ))
        }
    };
    let handle_b = match args.get(1) {
        Some(RuntimeValue::Dict(h)) => *h,
        _ => {
            return Err(ExecutorError::Type(
                "dict.merge expects a Dict as second argument".to_string(),
            ))
        }
    };

    let map_a = match ctx.heap.get(handle_a) {
        Some(HeapValue::Dict(map)) => map.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid dict handle".to_string())),
    };
    let map_b = match ctx.heap.get(handle_b) {
        Some(HeapValue::Dict(map)) => map.clone(),
        _ => return Err(ExecutorError::Runtime("Invalid dict handle".to_string())),
    };

    let mut merged = map_a;
    merged.extend(map_b);
    let new_handle = ctx.heap.allocate(HeapValue::Dict(merged));
    Ok(RuntimeValue::Dict(new_handle))
}
