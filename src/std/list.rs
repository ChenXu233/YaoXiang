//! Standard List library (YaoXiang)
//!
//! This module provides list manipulation functions for YaoXiang programs.

use crate::backends::common::{RuntimeValue, HeapValue};
use crate::backends::ExecutorError;
use crate::std::{expect_list, NativeContext, NativeExport, StdModule};

// ListModule - StdModule Implementation

/// List module implementation.
#[derive(Default)]
pub struct ListModule;

impl StdModule for ListModule {
    fn module_path(&self) -> &str {
        "std.list"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            export!(
                "push",
                "std.list.push",
                "(A: Type)(list: List(A), item: Any) -> List(A)",
                native_push
            ),
            export!(
                "pop",
                "std.list.pop",
                "(A: Type)(list: &List(A)) -> Any",
                native_pop
            ),
            export!(
                "append",
                "std.list.append",
                "(A: Type)(list: List(A), item: Any) -> List(A)",
                native_append
            ),
            export!(
                "prepend",
                "std.list.prepend",
                "(A: Type)(list: List(A), item: Any) -> List(A)",
                native_prepend
            ),
            export!(
                "remove_at",
                "std.list.remove_at",
                "(A: Type)(list: &List(A), index: Int) -> Any",
                native_remove_at
            ),
            export!(
                "reverse",
                "std.list.reverse",
                "(A: Type)(list: &List(A)) -> List(A)",
                native_reverse
            ),
            export!(
                "concat",
                "std.list.concat",
                "(A: Type)(a: &List(A), b: &List(A)) -> List(A)",
                native_concat
            ),
            export!(
                "map",
                "std.list.map",
                "(T: Type)(list: &List(T), fn: (item: T) -> T) -> List(T)",
                native_map
            ),
            export!(
                "filter",
                "std.list.filter",
                "(T: Type)(list: &List(T), fn: (item: T) -> Bool) -> List(T)",
                native_filter
            ),
            export!(
                "reduce",
                "std.list.reduce",
                "(T: Type)(list: &List(T), fn: (acc: Any, item: T) -> Any, init: Any) -> Any",
                native_reduce
            ),
            export!(
                "len",
                "std.list.len",
                "(A: Type)(list: &List(A)) -> Int",
                native_len
            ),
            export!(
                "is_empty",
                "std.list.is_empty",
                "(A: Type)(list: &List(A)) -> Bool",
                native_is_empty
            ),
            export!(
                "get",
                "std.list.get",
                "(A: Type)(list: &List(A), index: Int) -> Any",
                native_get
            ),
            export!(
                "set",
                "std.list.set",
                "(A: Type)(list: List(A), index: Int, value: Any) -> List(A)",
                native_set
            ),
            export!(
                "first",
                "std.list.first",
                "(A: Type)(list: &List(A)) -> Any",
                native_first
            ),
            export!(
                "last",
                "std.list.last",
                "(A: Type)(list: &List(A)) -> Any",
                native_last
            ),
            export!(
                "slice",
                "std.list.slice",
                "(A: Type)(list: &List(A), start: Int, end: Int) -> List(A)",
                native_slice
            ),
            export!(
                "contains",
                "std.list.contains",
                "(A: Type)(list: &List(A), item: Any) -> Bool",
                native_contains
            ),
            export!(
                "find_index",
                "std.list.find_index",
                "(A: Type)(list: &List(A), item: Any) -> Int",
                native_find_index
            ),
            // 迭代器协议函数
            export!(
                "iter",
                "std.list.iter",
                "(A: Type)(list: &List(A)) -> Tuple",
                native_iter
            ),
            export!(
                "next",
                "std.list.next",
                "(iterator: Tuple) -> Any",
                native_next
            ),
            export!(
                "has_next",
                "std.list.has_next",
                "(iterator: Tuple) -> Bool",
                native_has_next
            ),
        ]
    }
}

/// Singleton instance for std.list module.
pub const LIST_MODULE: ListModule = ListModule;

// Native function implementations

/// Native implementation: push - add item to end of list
/// Returns new list with item added
fn native_push(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "push")?;
    let item = args.get(1).cloned().unwrap_or(RuntimeValue::Void);

    let mut items = ctx.heap_list(list_handle)?;
    items.push(item);
    let new_handle = ctx.heap.allocate(HeapValue::List(items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: pop - remove and return last item
fn native_pop(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "pop")?;

    let mut guard = ctx.heap_list_mut(&list_handle)?;
    let items = match &mut *guard {
        HeapValue::List(items) => items,
        _ => unreachable!("shape validated by heap_list_mut"),
    };

    Ok(items.pop().unwrap_or(RuntimeValue::Void))
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
    let list_handle = expect_list(args, "prepend")?;
    let item = args.get(1).cloned().unwrap_or(RuntimeValue::Void);

    let mut items = ctx.heap_list(list_handle)?;
    items.insert(0, item);
    let new_handle = ctx.heap.allocate(HeapValue::List(items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: remove_at - remove item at index
fn native_remove_at(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "remove_at")?;
    // #271 #6：缺参保留默认 0；非 Int 索引不再静默当 0
    let index = match args.get(1) {
        None => 0,
        Some(v) => v.to_int().ok_or_else(|| {
            ExecutorError::type_only("list.remove_at expects an Int index".to_string())
        })?,
    };
    // #300 D 项：负索引归并 E6003，诊断保真（原始 i64），与 l[-1] 下标路径一致
    let list_len = ctx
        .heap_list(list_handle.clone())
        .map(|i| i.len())
        .unwrap_or(0);
    let index = usize::try_from(index).map_err(|_| ExecutorError::IndexOutOfBounds {
        max: list_len,
        index,
        stack: None,
    })?;

    let mut guard = ctx.heap_list_mut(&list_handle)?;
    let items = match &mut *guard {
        HeapValue::List(items) => items,
        _ => unreachable!("shape validated by heap_list_mut"),
    };

    if index < items.len() {
        Ok(items.remove(index))
    } else {
        // #280：越界报专用码 E6003（原 E6007 通用）
        Err(ExecutorError::index_out_of_bounds(
            items.len(),
            index as i64,
            None,
        ))
    }
}

/// Native implementation: reverse - reverse list
fn native_reverse(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "reverse")?;

    let mut items = ctx.heap_list(list_handle)?;
    items.reverse();
    let new_handle = ctx.heap.allocate(HeapValue::List(items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: concat - concatenate two lists
fn native_concat(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let handle_a = expect_list(args, "concat")?;
    let handle_b = match args.get(1) {
        Some(RuntimeValue::List(h)) => h.clone(),
        _ => {
            return Err(ExecutorError::type_only(
                "concat expects a List as second argument".to_string(),
            ))
        }
    };

    let items_a = ctx.heap_list(handle_a)?;
    let items_b = ctx.heap_list(handle_b)?;

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
    let list_handle = expect_list(args, "map")?;
    let func_value = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::type_only("map expects a function as second argument".to_string())
    })?;

    let items = ctx.heap_list(list_handle)?;

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
    let list_handle = expect_list(args, "filter")?;
    let func_value = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::type_only("filter expects a function as second argument".to_string())
    })?;

    let items = ctx.heap_list(list_handle)?;

    let mut result_items = Vec::new();
    for item in items {
        let result = ctx.call_function(&func_value, std::slice::from_ref(&item))?;
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
    let list_handle = expect_list(args, "reduce")?;
    let func_value = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::type_only("reduce expects a function as second argument".to_string())
    })?;
    let mut accumulator = args.get(2).cloned().unwrap_or(RuntimeValue::Void);

    let items = ctx.heap_list(list_handle)?;

    for item in items {
        accumulator = ctx.call_function(&func_value, &[accumulator, item])?;
    }

    Ok(accumulator)
}

/// Native implementation: len - get list length
fn native_len(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.first() {
        Some(RuntimeValue::List(h)) => h.clone(),
        // #271 #5：参数类型错不再静默返回 0
        _ => {
            return Err(ExecutorError::type_only(
                "list.len expects a List as first argument".to_string(),
            ))
        }
    };

    let guard = list_handle.lock();
    match &*guard {
        HeapValue::List(items) => Ok(RuntimeValue::Int(items.len() as i64)),
        // #271 #5：悬垂句柄是内部错误，不再静默返回 0
        _ => Err(ExecutorError::runtime_only(
            "internal: dangling list handle in list.len".to_string(),
        )),
    }
}

/// Native implementation: is_empty - check if list is empty
fn native_is_empty(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.first() {
        Some(RuntimeValue::List(h)) => h.clone(),
        // #271 #5：参数类型错不再静默返回 true
        _ => {
            return Err(ExecutorError::type_only(
                "list.is_empty expects a List as first argument".to_string(),
            ))
        }
    };

    let guard = list_handle.lock();
    match &*guard {
        HeapValue::List(items) => Ok(RuntimeValue::Bool(items.is_empty())),
        _ => Err(ExecutorError::runtime_only(
            "internal: dangling list handle in list.is_empty".to_string(),
        )),
    }
}

/// Native implementation: get - get item at index
fn native_get(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "get")?;
    // #271 #6：缺参保留默认 0；非 Int 索引不再静默当 0
    let index = match args.get(1) {
        None => 0,
        Some(v) => v
            .to_int()
            .ok_or_else(|| ExecutorError::type_only("list.get expects an Int index".to_string()))?,
    };
    let index = usize::try_from(index)
        .map_err(|_| ExecutorError::runtime_only(format!("list.get: negative index {}", index)))?;

    let guard = list_handle.lock();
    match &*guard {
        HeapValue::List(items) => Ok(items.get(index).cloned().unwrap_or(RuntimeValue::Void)),
        // #271 #5：悬垂句柄是内部错误
        _ => Err(ExecutorError::runtime_only(
            "internal: dangling list handle in list.get".to_string(),
        )),
    }
}

/// Native implementation: set - set item at index
fn native_set(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "set")?;
    // #271 #6：缺参保留默认 0；非 Int 索引不再静默当 0
    let index = match args.get(1) {
        None => 0,
        Some(v) => v
            .to_int()
            .ok_or_else(|| ExecutorError::type_only("list.set expects an Int index".to_string()))?,
    };
    let list_len = ctx
        .heap_list(list_handle.clone())
        .map(|i| i.len())
        .unwrap_or(0);
    let index = usize::try_from(index).map_err(|_| {
        // #300 D 项：负索引归并 E6003，诊断保真（原始 i64）
        ExecutorError::IndexOutOfBounds {
            max: list_len,
            index,
            stack: None,
        }
    })?;
    let value = args.get(2).cloned().unwrap_or(RuntimeValue::Void);

    let mut items = ctx.heap_list(list_handle)?;

    if index < items.len() {
        items[index] = value;
    } else {
        // #279/#280：越界写不再静默丢弃，报专用码 E6003
        return Err(ExecutorError::index_out_of_bounds(
            items.len(),
            index as i64,
            None,
        ));
    }
    let new_handle = ctx.heap.allocate(HeapValue::List(items));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: first - get first element
fn native_first(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "first")?;

    let guard = list_handle.lock();
    match &*guard {
        HeapValue::List(items) => Ok(items.first().cloned().unwrap_or(RuntimeValue::Void)),
        _ => Ok(RuntimeValue::Void),
    }
}

/// Native implementation: last - get last element
fn native_last(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "last")?;

    let guard = list_handle.lock();
    match &*guard {
        HeapValue::List(items) => Ok(items.last().cloned().unwrap_or(RuntimeValue::Void)),
        _ => Ok(RuntimeValue::Void),
    }
}

/// Native implementation: slice - get sublist
fn native_slice(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "slice")?;
    // #271 #6：缺参保留默认（start=0 / end=末尾）；非 Int 边界不再静默当 0
    let start = match args.get(1) {
        None => 0,
        Some(v) => v
            .to_int()
            .ok_or_else(|| ExecutorError::type_only("list.slice expects Int bounds".to_string()))?,
    };
    let end = match args.get(2) {
        None => i64::MAX,
        Some(v) => v
            .to_int()
            .ok_or_else(|| ExecutorError::type_only("list.slice expects Int bounds".to_string()))?,
    };
    let start = usize::try_from(start)
        .map_err(|_| ExecutorError::runtime_only("list.slice: negative start".to_string()))?;
    let end = usize::try_from(end)
        .map_err(|_| ExecutorError::runtime_only("list.slice: negative end".to_string()))?;

    let items = ctx.heap_list(list_handle)?;

    let end = end.min(items.len());
    let start = start.min(end);
    let sliced = items[start..end].to_vec();
    let new_handle = ctx.heap.allocate(HeapValue::List(sliced));
    Ok(RuntimeValue::List(new_handle))
}

/// Native implementation: contains - check if list contains item
fn native_contains(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.first() {
        Some(RuntimeValue::List(h)) => h.clone(),
        _ => return Ok(RuntimeValue::Bool(false)),
    };
    let target = args.get(1).cloned().unwrap_or(RuntimeValue::Void);

    let guard = list_handle.lock();
    match &*guard {
        HeapValue::List(items) => Ok(RuntimeValue::Bool(items.contains(&target))),
        _ => Ok(RuntimeValue::Bool(false)),
    }
}

/// Native implementation: find_index - find index of item
fn native_find_index(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = match args.first() {
        Some(RuntimeValue::List(h)) => h.clone(),
        _ => return Ok(RuntimeValue::Int(-1)),
    };
    let target = args.get(1).cloned().unwrap_or(RuntimeValue::Void);

    let guard = list_handle.lock();
    match &*guard {
        HeapValue::List(items) => match items.iter().position(|item| item == &target) {
            Some(idx) => Ok(RuntimeValue::Int(idx as i64)),
            None => Ok(RuntimeValue::Int(-1)),
        },
        _ => Ok(RuntimeValue::Int(-1)),
    }
}

// 迭代器协议实现

/// Native implementation: iter - 创建迭代器
/// 返回一个 Tuple (原始列表, 当前索引)
/// 使用 Tuple 存储迭代器状态: (List, Int)
fn native_iter(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let list_handle = expect_list(args, "iter")?;

    // 创建一个 Tuple 存储迭代器状态 (原始列表, 索引 0)
    let iterator_items = vec![RuntimeValue::List(list_handle), RuntimeValue::Int(0)];
    let iterator_handle = ctx.heap.allocate(HeapValue::Tuple(iterator_items));
    Ok(RuntimeValue::Tuple(iterator_handle))
}

/// Native implementation: next - 获取下一个元素
/// 迭代器格式: Tuple (原始列表, 当前索引)
/// 返回下一个元素，并递增索引
fn native_next(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let iter_handle = match args.first() {
        Some(RuntimeValue::Tuple(h)) => h.clone(),
        _ => return Ok(RuntimeValue::Void),
    };

    let iterator_items = match &*iter_handle.lock() {
        HeapValue::Tuple(items) => items.clone(),
        _ => return Ok(RuntimeValue::Void),
    };

    // 获取原始列表和当前索引
    let list_handle = match iterator_items.first() {
        Some(RuntimeValue::List(h)) => h.clone(),
        _ => return Ok(RuntimeValue::Void),
    };
    let current_idx = match iterator_items.get(1) {
        Some(RuntimeValue::Int(idx)) => *idx as usize,
        _ => return Ok(RuntimeValue::Void),
    };

    // 获取元素
    let element = match &*list_handle.lock() {
        HeapValue::List(items) if current_idx < items.len() => items[current_idx].clone(),
        _ => RuntimeValue::Void,
    };

    // 更新索引
    let new_idx = current_idx + 1;
    let mut new_iterator_items = iterator_items;
    new_iterator_items[1] = RuntimeValue::Int(new_idx as i64);
    *iter_handle.lock() = HeapValue::Tuple(new_iterator_items);

    Ok(element)
}

/// Native implementation: has_next - 检查是否还有更多元素
fn native_has_next(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let iter_handle = match args.first() {
        Some(RuntimeValue::Tuple(h)) => h.clone(),
        _ => return Ok(RuntimeValue::Bool(false)),
    };

    let iterator_items = match &*iter_handle.lock() {
        HeapValue::Tuple(items) => items.clone(),
        _ => return Ok(RuntimeValue::Bool(false)),
    };

    // 获取原始列表和当前索引
    let list_handle = match iterator_items.first() {
        Some(RuntimeValue::List(h)) => h.clone(),
        _ => return Ok(RuntimeValue::Bool(false)),
    };
    let current_idx = match iterator_items.get(1) {
        Some(RuntimeValue::Int(idx)) => *idx as usize,
        _ => return Ok(RuntimeValue::Bool(false)),
    };

    // 检查是否有更多元素
    let has_more = match &*list_handle.lock() {
        HeapValue::List(items) => current_idx < items.len(),
        _ => false,
    };

    Ok(RuntimeValue::Bool(has_more))
}
