//! Range 标准库模块（#302）
//!
//! `Range(Int)` 的迭代器协议与区间谓词。Range 值本体是三标量不可变记录
//! （`RuntimeValue::Range { start, end, step }`，`a..b` / `a..b..c` 字面量构造），
//! 本模块提供：
//! - 迭代器协议：`iter`/`has_next`/`next`（`for i in r` 经 ir_gen 类型派发调用）
//! - 区间谓词：`contains`（`x in r` 的运行时求值路径；证明管道直接识别命题，不走此调用）
//!
//! 迭代器载体：4 元组 `(start, end, step, cur)`，`next` 原地推进（List 迭代器先例）。
//! step=0：字面量编译期拒绝；动态零在 `iter`/`contains` 返回 `Err(Error)`
//! （#316 Result 化——`?` 传播或 `result.unwrap` 分流；`for`/`in` 糖降级在
//! ir_gen 解包，Err 分支走 `abort_invalid_step` 显式失败）。

use crate::backends::common::{HeapValue, RuntimeValue};
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

/// Singleton instance for std::range module.
pub const RANGE_MODULE: RangeModule = RangeModule;

#[derive(Default)]
pub struct RangeModule;

impl StdModule for RangeModule {
    fn module_path(&self) -> &str {
        "std.range"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            export!(
                "iter",
                "std.range.iter",
                "(r: Range(Int)) -> Result(Iterator(Any), Error)",
                native_iter
            ),
            export!(
                "has_next",
                "std.range.has_next",
                "(it: Iterator(Any)) -> Bool",
                native_has_next
            ),
            export!(
                "next",
                "std.range.next",
                "(it: &Iterator(Any)) -> Any",
                native_next
            ),
            export!(
                "contains",
                "std.range.contains",
                "(r: Range(Int), x: Int) -> Result(Bool, Error)",
                native_contains
            ),
            export!(
                "abort_invalid_step",
                "std.range.abort_invalid_step",
                "(r: Range(Int)) -> Any",
                native_abort_invalid_step
            ),
            export!(
                "map",
                "std.range.map",
                "(it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)",
                native_map
            ),
            export!(
                "filter",
                "std.range.filter",
                "(it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)",
                native_filter
            ),
            export!(
                "collect",
                "std.range.collect",
                "(it: Iterator(Any)) -> List(Any)",
                native_collect
            ),
            export!(
                "reduce",
                "std.range.reduce",
                "(it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any",
                native_reduce
            ),
            export!(
                "for_each",
                "std.range.for_each",
                "(it: Iterator(Any), f: (Any) -> Void) -> Void",
                native_for_each
            ),
        ]
    }
}

// 迭代器协议实现（载体：4 元组 [start, end, step, cur]，next 原地推进）
//
// 适配器载体（§6.2，#302）：5 元组 [kind, base_iter, f, lookahead, has_lookahead]
//   kind = "map" | "filter"；lookahead 缓存已预取元素，保证每元素只调一次 f。
//   has_next/next 按载体长度派发：len==4 原生迭代器，len==5 适配器。

/// 从迭代器元组读槽位（0=start, 1=end, 2=step, 3=cur）
fn iter_parts(args: &[RuntimeValue]) -> Option<[i64; 4]> {
    let handle = match args.first() {
        Some(RuntimeValue::Tuple(h)) => h,
        _ => return None,
    };
    let guard = handle.lock();
    let items = match &*guard {
        HeapValue::Tuple(items) if items.len() == 4 => items,
        _ => return None,
    };
    let mut out = [0i64; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        match items.get(i) {
            Some(RuntimeValue::Int(n)) => *slot = *n,
            _ => return None,
        }
    }
    Some(out)
}

/// 适配器内槽位下标
const ADAPTER_KIND: usize = 0;
const ADAPTER_BASE: usize = 1;
const ADAPTER_FN: usize = 2;
const ADAPTER_LOOKAHEAD: usize = 3;
const ADAPTER_HAS: usize = 4;

/// 判断迭代器参数是否为适配器载体（5 元组）
fn is_adapter(args: &[RuntimeValue]) -> bool {
    matches!(args.first(), Some(RuntimeValue::Tuple(h))
        if matches!(&*h.lock(), HeapValue::Tuple(items) if items.len() == 5))
}

/// 取适配器元组内槽位的克隆
fn adapter_slot(
    adapter: &crate::backends::common::Handle,
    idx: usize,
) -> Option<RuntimeValue> {
    let guard = adapter.lock();
    match &*guard {
        HeapValue::Tuple(items) => items.get(idx).cloned(),
        _ => None,
    }
}

/// 适配器预取：从 base 拉取下一个被接受的元素，写入 lookahead 槽。
/// 返回 true 表示有元素（has_lookahead 已置位）。
/// 每元素只调一次 f（map 的映射 / filter 的谓词）。
fn adapter_fill(
    adapter: &crate::backends::common::Handle,
    ctx: &mut NativeContext<'_>,
) -> Result<bool, ExecutorError> {
    let kind = match adapter_slot(adapter, ADAPTER_KIND) {
        Some(RuntimeValue::String(s)) => s.to_string(),
        _ => return Ok(false),
    };
    let func = adapter_slot(adapter, ADAPTER_FN).unwrap_or(RuntimeValue::Void);

    loop {
        // base 是否还有元素
        let base = adapter_slot(adapter, ADAPTER_BASE).unwrap_or(RuntimeValue::Void);
        if !matches!(native_has_next(&[base], ctx)?, RuntimeValue::Bool(true)) {
            return Ok(false);
        }

        // v = next(base)
        let base = adapter_slot(adapter, ADAPTER_BASE).unwrap_or(RuntimeValue::Void);
        let v = native_next(&[base], ctx)?;
        if matches!(v, RuntimeValue::Void) {
            return Ok(false);
        }

        if kind == "filter" {
            let keep = ctx.call_function(&func, std::slice::from_ref(&v))?;
            if !matches!(keep, RuntimeValue::Bool(true)) {
                continue;
            }
        }
        // map：预取映射结果；filter：预取原元素
        let accepted = if kind == "map" {
            ctx.call_function(&func, std::slice::from_ref(&v))?
        } else {
            v
        };
        let mut guard = adapter.lock();
        if let HeapValue::Tuple(items) = &mut *guard {
            items[ADAPTER_LOOKAHEAD] = accepted;
            items[ADAPTER_HAS] = RuntimeValue::Bool(true);
        }
        return Ok(true);
    }
}

/// Native implementation: iter - 创建 Range 迭代器
/// 返回 4 元组 (start, end, step, cur)，cur 初始 = start
/// #316：签名 Result(Iterator(Any), Error)——动态 step=0 不再硬崩，
/// 返回 Err(Error) 值，`?` 沿调用栈传播或 result.unwrap 显式分流
fn native_iter(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let (start, end, step) = match args.first() {
        Some(RuntimeValue::Range { start, end, step }) => (*start, *end, *step),
        _ => {
            return Err(ExecutorError::runtime_only(
                "std.range.iter: expected Range value",
            ))
        }
    };
    // #316：动态 step=0 → Err(Error) 值（构造点已拦字面量零；旧运行时硬崩废除）
    if step == 0 {
        let err = crate::std::result::error_new("Range step must be non-zero", ctx);
        return Ok(crate::std::result::result_err(err));
    }
    let items = vec![
        RuntimeValue::Int(start),
        RuntimeValue::Int(end),
        RuntimeValue::Int(step),
        RuntimeValue::Int(start),
    ];
    let handle = ctx.heap.allocate(HeapValue::Tuple(items));
    Ok(crate::std::result::result_ok(RuntimeValue::Tuple(handle)))
}

/// Native implementation: has_next - 符号派发：pos → cur < end，neg → cur > end
/// 适配器载体：已有预取 → true，否则预取后判定
fn native_has_next(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if is_adapter(args) {
        let handle = match args.first() {
            Some(RuntimeValue::Tuple(h)) => h.clone(),
            _ => return Ok(RuntimeValue::Bool(false)),
        };
        let has = matches!(
            adapter_slot(&handle, ADAPTER_HAS),
            Some(RuntimeValue::Bool(true))
        );
        if has {
            return Ok(RuntimeValue::Bool(true));
        }
        let filled = adapter_fill(&handle, ctx)?;
        return Ok(RuntimeValue::Bool(filled));
    }
    let has_more = match iter_parts(args) {
        Some([_s, e, p, cur]) => {
            if p >= 0 {
                cur < e
            } else {
                cur > e
            }
        }
        None => false,
    };
    Ok(RuntimeValue::Bool(has_more))
}

/// Native implementation: next - 取 cur 并按 step 推进（原地写回迭代器元组）
/// 适配器载体：消费预取槽（空则先预取）
fn native_next(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if is_adapter(args) {
        let handle = match args.first() {
            Some(RuntimeValue::Tuple(h)) => h.clone(),
            _ => return Ok(RuntimeValue::Void),
        };
        let has = matches!(
            adapter_slot(&handle, ADAPTER_HAS),
            Some(RuntimeValue::Bool(true))
        );
        if !has {
            let filled = adapter_fill(&handle, ctx)?;
            if !filled {
                return Ok(RuntimeValue::Void);
            }
        }
        let out = adapter_slot(&handle, ADAPTER_LOOKAHEAD).unwrap_or(RuntimeValue::Void);
        let mut guard = handle.lock();
        if let HeapValue::Tuple(items) = &mut *guard {
            items[ADAPTER_HAS] = RuntimeValue::Bool(false);
        }
        return Ok(out);
    }
    let handle = match args.first() {
        Some(RuntimeValue::Tuple(h)) => h.clone(),
        _ => return Ok(RuntimeValue::Void),
    };
    let parts = match iter_parts(args) {
        Some(p) => p,
        None => return Ok(RuntimeValue::Void),
    };
    let [start, end, step, cur] = parts;

    // 终界守护：耗尽后返回 Void（与 List.next 耗尽行为一致）
    let exhausted = if step >= 0 { cur >= end } else { cur <= end };
    if exhausted {
        return Ok(RuntimeValue::Void);
    }

    // 原地推进 cur += step
    {
        let mut guard = handle.lock();
        if let HeapValue::Tuple(items) = &mut *guard {
            items[3] = RuntimeValue::Int(cur + step);
        }
    }
    let _ = start;
    Ok(RuntimeValue::Int(cur))
}

/// Native implementation: contains - 区间谓词 `x in r` 的运行时求值
///
/// 语义与 ir_gen 旧脱糖一致：界检查（符号派发）+ 步长对齐
/// `x in r ⟺ x >= start && x < end && (x - start) % step == 0`（负 step 方向取反）。
/// #316：签名 Result(Bool, Error)——动态 step=0 返回 Err(Error) 值（`x in r`
/// 糖降级在 ir_gen 解包，Err 分支显式失败）；构造点已拦字面量零。
fn native_contains(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let (start, end, step) = match args.first() {
        Some(RuntimeValue::Range { start, end, step }) => (*start, *end, *step),
        _ => {
            return Err(ExecutorError::runtime_only(
                "std.range.contains: expected Range value",
            ))
        }
    };
    let x = match args.get(1) {
        Some(RuntimeValue::Int(n)) => *n,
        _ => {
            return Err(ExecutorError::runtime_only(
                "std.range.contains: expected Int element",
            ))
        }
    };

    // #316：动态 step=0 → Err(Error) 值
    if step == 0 {
        let err = crate::std::result::error_new("Range step must be non-zero", ctx);
        return Ok(crate::std::result::result_err(err));
    }

    // 界检查（符号派发）+ 步长对齐
    let in_bounds = if step > 0 {
        x >= start && x < end
    } else {
        x <= start && x > end
    };
    let aligned = (x - start) % step == 0;
    Ok(crate::std::result::result_ok(RuntimeValue::Bool(
        in_bounds && aligned,
    )))
}

/// Native implementation: abort_invalid_step - `for`/`in` 糖降级的 Err 分支
///
/// #316：iter/contains Result 化后，语法糖形态（for 循环、`x in r` 谓词）
/// 不产 Result 值可传播——隐式 `?` 对非 Result 函数是类型违约。糖降级统一
/// 解包，Err 分支调用本 native 显式失败（替代此前 native 深处的硬崩，
/// 失败点在用户代码的消费处，消息明确）。
fn native_abort_invalid_step(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Err(ExecutorError::runtime_only(
        "Range step must be non-zero (for/in consumption)",
    ))
}

// 迭代器适配器（§6.2）：map/filter 惰性，collect/reduce/for_each 消费

fn expect_iterator(
    args: &[RuntimeValue],
    who: &str,
) -> Result<RuntimeValue, ExecutorError> {
    args.first().cloned().ok_or_else(|| {
        ExecutorError::type_only(format!("std.range.{who}: expected iterator argument"))
    })
}

/// Native implementation: map - 惰性映射，返回适配器迭代器
fn native_map(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let base = expect_iterator(args, "map")?;
    let func = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::type_only("std.range.map: expected function as second argument".to_string())
    })?;
    let items = vec![
        RuntimeValue::String("map".into()),
        base,
        func,
        RuntimeValue::Void,
        RuntimeValue::Bool(false),
    ];
    let handle = ctx.heap.allocate(HeapValue::Tuple(items));
    Ok(RuntimeValue::Tuple(handle))
}

/// Native implementation: filter - 惰性过滤，返回适配器迭代器
fn native_filter(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let base = expect_iterator(args, "filter")?;
    let func = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::type_only(
            "std.range.filter: expected predicate as second argument".to_string(),
        )
    })?;
    let items = vec![
        RuntimeValue::String("filter".into()),
        base,
        func,
        RuntimeValue::Void,
        RuntimeValue::Bool(false),
    ];
    let handle = ctx.heap.allocate(HeapValue::Tuple(items));
    Ok(RuntimeValue::Tuple(handle))
}

/// Native implementation: collect - 物化迭代器为 List（显式消费；区间本体不物化）
fn native_collect(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let it = expect_iterator(args, "collect")?;
    let mut out = Vec::new();
    loop {
        if !matches!(
            native_has_next(std::slice::from_ref(&it), ctx)?,
            RuntimeValue::Bool(true)
        ) {
            break;
        }
        match native_next(std::slice::from_ref(&it), ctx)? {
            RuntimeValue::Void => break,
            v => out.push(v),
        }
    }
    let handle = ctx.heap.allocate(HeapValue::List(out));
    Ok(RuntimeValue::List(handle))
}

/// Native implementation: reduce - 折叠：acc = f(acc, x)
fn native_reduce(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let it = expect_iterator(args, "reduce")?;
    let mut acc = args.get(1).cloned().unwrap_or(RuntimeValue::Void);
    let func = args.get(2).cloned().ok_or_else(|| {
        ExecutorError::type_only(
            "std.range.reduce: expected function as third argument".to_string(),
        )
    })?;
    loop {
        if !matches!(
            native_has_next(std::slice::from_ref(&it), ctx)?,
            RuntimeValue::Bool(true)
        ) {
            break;
        }
        match native_next(std::slice::from_ref(&it), ctx)? {
            RuntimeValue::Void => break,
            v => acc = ctx.call_function(&func, &[acc, v])?,
        }
    }
    Ok(acc)
}

/// Native implementation: for_each - 消费迭代器执行副作用
fn native_for_each(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let it = expect_iterator(args, "for_each")?;
    let func = args.get(1).cloned().ok_or_else(|| {
        ExecutorError::type_only(
            "std.range.for_each: expected function as second argument".to_string(),
        )
    })?;
    loop {
        if !matches!(
            native_has_next(std::slice::from_ref(&it), ctx)?,
            RuntimeValue::Bool(true)
        ) {
            break;
        }
        match native_next(std::slice::from_ref(&it), ctx)? {
            RuntimeValue::Void => break,
            v => {
                ctx.call_function(&func, &[v])?;
            }
        }
    }
    Ok(RuntimeValue::Void)
}
