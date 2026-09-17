---
title: 'std.range'
description: '区间迭代、谓词与惰性适配器'
---

# std.range

区间（`Range`）迭代与适配器。

```yaoxiang
use std.range
```

## 区间字面量

| 写法      | 含义                    |
| --------- | ----------------------- |
| `a..b`    | 从 `a` 到 `b`，步长 `1` |
| `a..b..s` | 从 `a` 到 `b`，步长 `s` |

区间**不包含结束值**（左闭右开）。步长可为负，表示递减。

## 迭代器协议

[`iter`](#iter) 返回 `Result`——步长为 `0` 时是错误路径（`E6009`），所以要先 `unwrap` 或显式处理：

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

> **移动语义**：`has_next` 与 `next` 的签名都不带 `&`，会把迭代器**移动**
> 掉。因此每次取用都要重新创建迭代器，或直接用 `for ... in` 遍历。这与 [`std.list`](./list)
> 的迭代器一致。

```yaoxiang
use std.assert
use std.range
use std.result

main = {
    // 逐个新建迭代器
    a = result.unwrap(range.iter(1..3))
    assert(range.has_next(a))

    b = result.unwrap(range.iter(1..3))
    assert(range.next(b) == 1)
}
```

日常遍历直接用 `for ... in`：

```yaoxiang
use std.assert
use std.list

main = {
    nums = [1, 2, 3]
    mut sum = 0
    for x in nums {
        sum = sum + x
    }
    assert(sum == 6)
}
```

## 函数一览

<!-- stdlib:table:range start -->

| 函数                 | 签名                                                          |
| -------------------- | ------------------------------------------------------------- |
| `iter`               | `(r: Range(Int)) -> Result(Iterator(Any), Error)`             |
| `has_next`           | `(it: Iterator(Any)) -> Bool`                                 |
| `next`               | `(it: &Iterator(Any)) -> Any`                                 |
| `contains`           | `(r: Range(Int), x: Int) -> Result(Bool, Error)`              |
| `abort_invalid_step` | `(r: Range(Int)) -> Any`                                      |
| `map`                | `(it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)`       |
| `filter`             | `(it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)`      |
| `collect`            | `(it: Iterator(Any)) -> List(Any)`                            |
| `reduce`             | `(it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any` |
| `for_each`           | `(it: Iterator(Any), f: (Any) -> Void) -> Void`               |

<!-- stdlib:table:range end -->## 迭代器协议

### iter

<!-- stdlib:sig:range.iter start -->

```yaoxiang
iter: (r: Range(Int)) -> Result(Iterator(Any), Error)
```

<!-- stdlib:sig:range.iter end -->

由区间创建迭代器。

- `r` —— 区间，如 `1..6` 或 `3..0..-1`

返回：成功为 `Result.ok(迭代器)`；步长为 `0` 时为 `Result.err`，`code` 为 `E6009`。

```yaoxiang
use std.assert
use std.range
use std.result

main = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

### has_next

<!-- stdlib:sig:range.has_next start -->

```yaoxiang
has_next: (it: Iterator(Any)) -> Bool
```

<!-- stdlib:sig:range.has_next end -->

是否还有未消费的元素。

> 会**移动**迭代器。

```yaoxiang
use std.assert
use std.range
use std.result

main = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

### next

<!-- stdlib:sig:range.next start -->

```yaoxiang
next: (it: &Iterator(Any)) -> Any
```

<!-- stdlib:sig:range.next end -->

取出当前元素并把内部游标前移一位。

返回：当前元素；迭代结束时返回 `Void`。

> 会**移动**迭代器。

```yaoxiang
use std.assert
use std.range
use std.result

main = {
    it = result.unwrap(range.iter(1..4))
    assert(range.next(it) == 1)
}
```

递减区间同样支持：

```yaoxiang
use std.assert
use std.range
use std.result

main = {
    desc = result.unwrap(range.iter(3..0..-1))
    assert(range.next(desc) == 3)
}
```

### contains

<!-- stdlib:sig:range.contains start -->

```yaoxiang
contains: (r: Range(Int), x: Int) -> Result(Bool, Error)
```

<!-- stdlib:sig:range.contains end -->

判断 `x` 是否落在区间内。

- `r` —— 区间
- `x` —— 待判定值

返回：`Result.ok(Bool)`。结束值为**开区间**（不包含）；带步长时只匹配与步长对齐的元素。

```yaoxiang
use std.assert
use std.range
use std.result

main = {
    assert(result.unwrap(range.contains(1..10, 5)))
    assert(!result.unwrap(range.contains(1..10, 10)))     // 结束值不含

    assert(result.unwrap(range.contains(0..10..2, 4)))    // 与步长对齐
    assert(!result.unwrap(range.contains(0..10..2, 3)))   // 未对齐
}
```

### abort_invalid_step

<!-- stdlib:sig:range.abort_invalid_step start -->

```yaoxiang
abort_invalid_step: (r: Range(Int)) -> Any
```

<!-- stdlib:sig:range.abort_invalid_step end -->

步长非法的中止钩子，供 `for ... in` 消费到步长为 `0` 的区间时调用。

**总是**抛出 `E6007`，消息为
`Range step must be non-zero (for/in consumption)`。正常代码无需直接调用。

```yaoxiang
use std.range

main = {
    // 直接用 iter 会得到 Err，不需要走这个钩子
    r = range.iter(1..3)
}
```

## 适配器

`map` 与 `filter` 返回**惰性**适配器——它们不立即计算，需由 [`collect`](#collect) /
[`reduce`](#reduce) / [`for_each`](#for_each) / `for ... in` 消费后才产生结果。

### map

<!-- stdlib:sig:range.map start -->

```yaoxiang
map: (it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)
```

<!-- stdlib:sig:range.map end -->

把 `f` 映射到每个元素上，返回新的惰性迭代器。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(list.len(doubled) == 3)
    assert(list.get(doubled, 0) == 2)
}
```

### filter

<!-- stdlib:sig:range.filter start -->

```yaoxiang
filter: (it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)
```

<!-- stdlib:sig:range.filter end -->

保留使 `p` 为真的元素，返回新的惰性迭代器。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main = {
    big = range.collect(range.filter(result.unwrap(range.iter(1..6)), x => x > 3))
    assert(list.len(big) == 2)
    assert(list.get(big, 0) == 4)
}
```

适配器可链式组合：

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main = {
    r = 1..6
    chained = range.collect(range.map(range.filter(result.unwrap(range.iter(r)), x => x % 2 == 0), x => x * 10))
    assert(list.get(chained, 0) == 20)
    assert(list.get(chained, 1) == 40)
}
```

### collect

<!-- stdlib:sig:range.collect start -->

```yaoxiang
collect: (it: Iterator(Any)) -> List(Any)
```

<!-- stdlib:sig:range.collect end -->

消费迭代器，把全部元素收集为 `List`。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main = {
    xs = range.collect(result.unwrap(range.iter(1..4)))
    assert(list.len(xs) == 3)
}
```

### reduce

<!-- stdlib:sig:range.reduce start -->

```yaoxiang
reduce: (it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any
```

<!-- stdlib:sig:range.reduce end -->

消费迭代器并折叠。

- `it` —— 迭代器
- `init` —— 初始累加值
- `f` —— 归约函数 `(累加值, 元素) -> 新累加值`

> 注意参数顺序与 [`std.list.reduce`](./list#reduce) 不同：本模块是
> `(迭代器, 初值, 函数)`，`std.list` 是 `(列表, 函数, 初值)`。

```yaoxiang
use std.assert
use std.range
use std.result

main = {
    total = range.reduce(result.unwrap(range.iter(1..6)), 0, (acc, x) => acc + x)
    assert(total == 15)
}
```

### for_each

<!-- stdlib:sig:range.for_each start -->

```yaoxiang
for_each: (it: Iterator(Any), f: (Any) -> Void) -> Void
```

<!-- stdlib:sig:range.for_each end -->

对每个元素执行 `f`，用于副作用。

```yaoxiang
use std.assert
use std.range
use std.result

main = {
    // 输出 1、2、3
    range.for_each(result.unwrap(range.iter(1..4)), x => println(x))
    assert(true)
}
```

> 闭包目前**不能捕获并改写**外层的 `mut` 变量，所以用 `for_each` 做累加是行不通的（会报
> `E1001`）——累加请用 [`reduce`](#reduce)。

## 相关

- [`std.list`](./list) —— 列表与其迭代器
- [`std.result`](./result) —— 解包 `iter` / `contains` 的返回值
- [错误码参考](../error-code/) —— `E6009` 步长非法
