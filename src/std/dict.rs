//! Standard Dict library (YaoXiang)
//!
//! This module provides dictionary manipulation functions for YaoXiang programs.

use crate::backends::common::{RuntimeValue, HeapValue};
use crate::backends::ExecutorError;
use crate::std::{expect_dict, NativeContext, NativeExport, StdModule};

// DictModule - StdModule Implementation

/// Dict module implementation.
#[derive(Default)]
pub struct DictModule;

impl StdModule for DictModule {
    fn module_path(&self) -> &str {
        "std.dict"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            export!(
                "get",
                "std.dict.get",
                "(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any",
                native_get
            ),
            export!(
                "set",
                "std.dict.set",
                "(K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)",
                native_set
            ),
            export!(
                "has",
                "std.dict.has",
                "(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool",
                native_has
            ),
            export!(
                "values",
                "std.dict.values",
                "(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)",
                native_values
            ),
            export!(
                "keys",
                "std.dict.keys",
                "(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)",
                native_keys
            ),
            export!(
                "entries",
                "std.dict.entries",
                "(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)",
                native_entries
            ),
            export!(
                "delete",
                "std.dict.delete",
                "(K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)",
                native_delete
            ),
            export!(
                "len",
                "std.dict.len",
                "(K: Type, V: Type)(dict: &Dict(K, V)) -> Int",
                native_len
            ),
            export!(
                "is_empty",
                "std.dict.is_empty",
                "(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool",
                native_is_empty
            ),
            export!(
                "merge",
                "std.dict.merge",
                "(A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)",
                native_merge
            ),
        ]
    }
}

/// Singleton instance for std.dict module.
pub const DICT_MODULE: DictModule = DictModule;

// Native function implementations

/// Native implementation: get - get value by key
fn native_get(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = expect_dict(args, "dict.get")?;
    let key = args.get(1).cloned().unwrap_or(RuntimeValue::Void);

    let guard = dict_handle.lock();
    match &*guard {
        // #299：缺键不再静默返回 void（与 `[]` 运算符、#279 方向一致）
        HeapValue::Dict(map) => match map.get(&key) {
            Some(v) => Ok(v.clone()),
            None => Err(ExecutorError::KeyNotFound {
                key: format!("{}", key),
                stack: None,
            }),
        },
        _ => Ok(RuntimeValue::Void),
    }
}

/// Native implementation: set - set key-value pair (returns new dict)
fn native_set(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = expect_dict(args, "dict.set")?;
    let key = args.get(1).cloned().unwrap_or(RuntimeValue::Void);
    let value = args.get(2).cloned().unwrap_or(RuntimeValue::Void);

    let mut map = match &*dict_handle.lock() {
        HeapValue::Dict(map) => map.clone(),
        _ => return Err(ExecutorError::runtime_only("Invalid dict handle")),
    };
    map.insert(key, value);
    let new_handle = ctx.heap.allocate(HeapValue::Dict(map));
    Ok(RuntimeValue::Dict(new_handle))
}

/// Native implementation: has - check if key exists
fn native_has(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.first() {
        Some(RuntimeValue::Dict(h)) => h.clone(),
        // #271 #5：参数类型错不再静默返回 false
        _ => {
            return Err(ExecutorError::type_only(
                "dict.has expects a Dict as first argument".to_string(),
            ))
        }
    };
    let key = args.get(1).cloned().unwrap_or(RuntimeValue::Void);

    let guard = dict_handle.lock();
    match &*guard {
        HeapValue::Dict(map) => Ok(RuntimeValue::Bool(map.contains_key(&key))),
        _ => Err(ExecutorError::runtime_only(
            "internal: dangling dict handle in dict.has".to_string(),
        )),
    }
}

/// Native implementation: values - get all values as list
fn native_values(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = expect_dict(args, "dict.values")?;

    let values: Vec<RuntimeValue> = match &*dict_handle.lock() {
        HeapValue::Dict(map) => map.values().cloned().collect(),
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
    let dict_handle = expect_dict(args, "dict.keys")?;

    let keys: Vec<RuntimeValue> = match &*dict_handle.lock() {
        HeapValue::Dict(map) => map.keys().cloned().collect(),
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
    let dict_handle = expect_dict(args, "dict.entries")?;

    let map = match &*dict_handle.lock() {
        HeapValue::Dict(map) => map.clone(),
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
    let dict_handle = expect_dict(args, "dict.delete")?;
    let key = args.get(1).cloned().unwrap_or(RuntimeValue::Void);

    let mut map = match &*dict_handle.lock() {
        HeapValue::Dict(map) => map.clone(),
        _ => return Err(ExecutorError::runtime_only("Invalid dict handle")),
    };
    map.remove(&key);
    let new_handle = ctx.heap.allocate(HeapValue::Dict(map));
    Ok(RuntimeValue::Dict(new_handle))
}

/// Native implementation: len - get number of entries
fn native_len(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.first() {
        Some(RuntimeValue::Dict(h)) => h.clone(),
        // #271 #5：参数类型错不再静默返回 0
        _ => {
            return Err(ExecutorError::type_only(
                "dict.len expects a Dict as first argument".to_string(),
            ))
        }
    };

    let guard = dict_handle.lock();
    match &*guard {
        HeapValue::Dict(map) => Ok(RuntimeValue::Int(map.len() as i64)),
        // #271 #5：悬垂句柄是内部错误，不再静默返回 0
        _ => Err(ExecutorError::runtime_only(
            "internal: dangling dict handle in dict.len".to_string(),
        )),
    }
}

/// Native implementation: is_empty - check if dict is empty
fn native_is_empty(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let dict_handle = match args.first() {
        Some(RuntimeValue::Dict(h)) => h.clone(),
        // #271 #5：参数类型错不再静默返回 true
        _ => {
            return Err(ExecutorError::type_only(
                "dict.is_empty expects a Dict as first argument".to_string(),
            ))
        }
    };

    let guard = dict_handle.lock();
    match &*guard {
        HeapValue::Dict(map) => Ok(RuntimeValue::Bool(map.is_empty())),
        _ => Err(ExecutorError::runtime_only(
            "internal: dangling dict handle in dict.is_empty".to_string(),
        )),
    }
}

/// Native implementation: merge - merge two dicts (second overrides first)
fn native_merge(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let handle_a = expect_dict(args, "dict.merge")?;
    let handle_b = match args.get(1) {
        Some(RuntimeValue::Dict(h)) => h.clone(),
        _ => {
            return Err(ExecutorError::type_only(
                "dict.merge expects a Dict as second argument",
            ))
        }
    };

    let map_a = match &*handle_a.lock() {
        HeapValue::Dict(map) => map.clone(),
        _ => return Err(ExecutorError::runtime_only("Invalid dict handle")),
    };
    let map_b = match &*handle_b.lock() {
        HeapValue::Dict(map) => map.clone(),
        _ => return Err(ExecutorError::runtime_only("Invalid dict handle")),
    };

    let mut merged = map_a;
    merged.extend(map_b);
    let new_handle = ctx.heap.allocate(HeapValue::Dict(merged));
    Ok(RuntimeValue::Dict(new_handle))
}
