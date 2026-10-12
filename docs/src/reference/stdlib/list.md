---
title: 'std.list'
description: '列表增删、切片、高阶函数与迭代器协议'
---

# std.list

列表操作模块。**移动语义需要特别留意**：有两类函数——只读借用源列表的，与消耗（移动）源列表的。
`&` 形参的自动借用规则见 RFC-009 §2.8：实参在调用后仍被使用时，编译器自动创建只读令牌。

```yaoxiang
use std.list
```

## 语义分类

签名中带 `&` 的参数为只读借用，调用后源值仍可用；不带 `&`
的参数按值传入，调用后源值**已被移动**，再次使用会报 `E2014`。

| 类别               | 函数                                                                                                                    | 行为                       |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| **消耗源列表** | `push` `append` `prepend` `set` `pop` `remove_at` `iter` | 源列表被移动，之后不可再用 |
| 只读借用       | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` | 源列表可反复使用；`contains`/`find_index` 的 `item` 亦按借用（`&A`） |
| 迭代协议       | `has_next`（`&Iter(T)`）`next`（`&mut Iter(T)`） | 两者都**借用**迭代器，不消耗 |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 只读借用：nums 可反复使用
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // 消耗：base 在此之后不可再用
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

## 函数一览

<!-- stdlib:table:list start -->

| 函数         | 签名                                                                                       |
| ------------ | ------------------------------------------------------------------------------------------ |
| `push`       | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `pop`        | `(A: Type) -> (list: Vec(A)) -> Vec(A)`                                                    |
| `append`     | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `prepend`    | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `remove_at`  | `(A: Type) -> (list: Vec(A), index: Int) -> Vec(A)`                                        |
| `reverse`    | `(A: Type) -> (list: &Vec(A)) -> Vec(A)`                                                   |
| `concat`     | `(A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)`                                          |
| `map`        | `(T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)`                       |
| `filter`     | `(T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)`                          |
| `reduce`     | `(T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc` |
| `len`        | `(A: Type) -> (list: &Vec(A)) -> Int`                                                      |
| `is_empty`   | `(A: Type) -> (list: &Vec(A)) -> Bool`                                                     |
| `get`        | `(A: Type) -> (list: &Vec(A), index: Int) -> A`                                            |
| `set`        | `(A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)`                              |
| `first`      | `(A: Type) -> (list: &Vec(A)) -> A`                                                        |
| `last`       | `(A: Type) -> (list: &Vec(A)) -> A`                                                        |
| `slice`      | `(A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)`                             |
| `contains`   | `(A: Type) -> (list: &Vec(A), item: &A) -> Bool`                                            |
| `find_index` | `(A: Type) -> (list: &Vec(A), item: &A) -> Int`                                             |
| `Iter`       | `(T: Type) -> Type`                                                                         |
| `iter`       | `(T: Type) -> (list: Vec(T)) -> Iter(T)`                                                   |
| `next`       | `(T: Type) -> (it: &mut Iter(T)) -> T`                                                     |
| `has_next`   | `(T: Type) -> (it: &Iter(T)) -> Bool`                                                      |
| `empty`      | `(T: Type) -> Vec(T)`                                                                      |
| `of`         | `(T: Type) -> (data: Vec(T)) -> Vec(T)`                                                    |

<!-- stdlib:table:list end -->## 函数

### push

<!-- stdlib:sig:list.push start -->

```yaoxiang
push: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.push end -->

返回在 `list` 末尾追加 `item` 的**新列表**。`list` 按值传入，调用后即被 **移动**，不可再用。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

### append

<!-- stdlib:sig:list.append start -->

```yaoxiang
append: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.append end -->

`push` 的别名，行为完全一致。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    extended = list.append([1, 2], 3)
    assert(list.len(extended) == 3)
}
```

### prepend

<!-- stdlib:sig:list.prepend start -->

```yaoxiang
prepend: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.prepend end -->

返回在 `list` 头部插入 `item` 的新列表。`list` 按值传入，调用后即被**移动**。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = list.prepend([2, 3], 1)
    assert(list.first(l) == 1)
}
```

### pop

<!-- stdlib:sig:list.pop start -->

```yaoxiang
pop: (A: Type) -> (list: Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.pop end -->

移除末元素并返回**缩短后的列表**（值语义）。源列表被消费，不再是 native 版
「签名带 `&` 却原地改动源值」的例外形态。

返回：去掉末元素的新列表；列表为空时原样返回。要读取被移除的元素，在调用前
先用 `last` 取值。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [1, 2, 3]
    rest = list.pop(l)               // l 被消费，rest 是缩短后的新列表
    assert(list.len(rest) == 2)
    assert(list.last(rest) == 2)     // 末元素 3 已被移除

    // 要读取被移除的元素，先用 last 取值再 pop
    l2 = [1, 2, 3]
    removed = list.last(l2)
    assert(removed == 3)

    empty = list.empty(Int)
    assert(list.is_empty(list.pop(empty)))
}
```

### remove_at

<!-- stdlib:sig:list.remove_at start -->

```yaoxiang
remove_at: (A: Type) -> (list: Vec(A), index: Int) -> Vec(A)
```

<!-- stdlib:sig:list.remove_at end -->

移除下标 `index` 处的元素并返回**缩短后的新列表**（值语义）。源列表被消费。

- `index` —— 元素下标

返回：去掉该元素的新列表。错误：下标为**负**时抛出 `E6003`（`src/std/list.yx:156-158`
的 `list[i + 1]` 越界）。

> **越界不报错**：下标 ≥ 长度时搬运循环一次都不进，只把 `length` 减 1，静默截断——
> `list.remove_at([1, 2, 3], 5)` 返回两元素列表且**不报错**。调用前请自行判界。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [10, 20, 30]
    got = list.remove_at(l, 1)
    assert(list.len(got) == 2)
    assert(list.get(got, 0) == 10)
    assert(list.get(got, 1) == 30)
}
```

### set

<!-- stdlib:sig:list.set start -->

```yaoxiang
set: (A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)
```

<!-- stdlib:sig:list.set end -->

返回把下标 `index` 改写为 `value` 的新列表。`list` 按值传入，调用后即被**移动**。

- `index` —— 下标
- `value` —— 新值

错误：下标为负或 ≥ 长度时抛出 `E6003`（越界写不再静默丢弃）。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = list.set([1, 2, 3], 1, 99)
    assert(list.get(l, 1) == 99)
}
```

### get

<!-- stdlib:sig:list.get start -->

```yaoxiang
get: (A: Type) -> (list: &Vec(A), index: Int) -> A
```

<!-- stdlib:sig:list.get end -->

读下标 `index` 处的元素（只读借用，`list` 可复用）。

- `index` —— 下标

返回：元素值。

错误：下标为负或 ≥ 长度时抛出 `E6003`（`src/std/list.yx:78` 直接用 `list[index]`，
越界由 `[]` 的边界检查兜住，**不返回 `Void`**）。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3, 4]
    assert(list.get(nums, 1) == 2)
}
```

### first

<!-- stdlib:sig:list.first start -->

```yaoxiang
first: (A: Type) -> (list: &Vec(A)) -> A
```

<!-- stdlib:sig:list.first end -->

返回首元素。

错误：**空列表会抛出 `E6003`**——`src/std/list.yx:82-89` 的兜底分支读 `zero[0]`，
空 `Vec` 的下标 0 越界。请先判 [`is_empty`](#is_empty)。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.first([1, 2, 3]) == 1)
}
```

### last

<!-- stdlib:sig:list.last start -->

```yaoxiang
last: (A: Type) -> (list: &Vec(A)) -> A
```

<!-- stdlib:sig:list.last end -->

返回末元素。

错误：**空列表会抛出 `E6003`**（同 [`first`](#first)，`src/std/list.yx:92-99`）。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.last([1, 2, 3]) == 3)
}
```

### slice

<!-- stdlib:sig:list.slice start -->

```yaoxiang
slice: (A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)
```

<!-- stdlib:sig:list.slice end -->

取 `[start, end)` 区间的子列表。

- `start` —— 起始下标
- `end` —— 结束下标（不含）

返回：新列表。`start >= end` 时返回空列表。

错误：**边界不做钳制**——`src/std/list.yx:196-205` 逐个读 `list[i]`，下标越界（含
`start` / `end` 为负）直接抛 `E6003`。`list.slice([1, 2, 3], 1, 99)` 会报错而不是
返回两元素列表。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    sub = list.slice([1, 2, 3, 4], 1, 3)
    assert(list.len(sub) == 2)
    assert(list.first(sub) == 2)
}
```

### reverse

<!-- stdlib:sig:list.reverse start -->

```yaoxiang
reverse: (A: Type) -> (list: &Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.reverse end -->

返回元素顺序反转的新列表；源列表不变。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    rev = list.reverse([1, 2, 3])
    assert(list.first(rev) == 3)
}
```

### concat

<!-- stdlib:sig:list.concat start -->

```yaoxiang
concat: (A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.concat end -->

拼接两个列表，返回新列表。两个源列表都不变。

类型检查在编译期完成：`concat` 是静态泛型（`src/std/list.yx:179`），第二个参数不是
列表时报类型错误，**不存在运行时的 `E6007` 路径**。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    joined = list.concat([1, 2], [3, 4])
    assert(list.len(joined) == 4)
}
```

### len

<!-- stdlib:sig:list.len start -->

```yaoxiang
len: (A: Type) -> (list: &Vec(A)) -> Int
```

<!-- stdlib:sig:list.len end -->

元素个数。只读借用，`list` 可反复使用。

`len` 是静态泛型（`src/std/list.yx:67`），非列表实参是编译期类型错误，不是运行时异常。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)      // 可复用
}
```

### is_empty

<!-- stdlib:sig:list.is_empty start -->

```yaoxiang
is_empty: (A: Type) -> (list: &Vec(A)) -> Bool
```

<!-- stdlib:sig:list.is_empty end -->

是否为空列表。

同 `len`，静态泛型（`src/std/list.yx:72`），非列表实参是编译期类型错误。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.is_empty([]))
    assert(!list.is_empty([1]))
}
```

### contains

<!-- stdlib:sig:list.contains start -->

```yaoxiang
contains: (A: Type) -> (list: &Vec(A), item: &A) -> Bool
```

<!-- stdlib:sig:list.contains end -->

`item` 是否在列表中（按值相等比较；元素类型须支持 `==`——基础类型原生支持，记录类型由 RFC-011b 的 `Equal` 自动派生或显式实例化提供）。`item`
按只读借用传入（调用端自动创建 `&A` 令牌），调用后实参仍可用。

返回：存在为 `true`；不存在为 `false`（线性扫描，不抛错）。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3, 4]
    assert(list.contains(nums, 3))
    assert(!list.contains(nums, 99))
}
```

### find_index

<!-- stdlib:sig:list.find_index start -->

```yaoxiang
find_index: (A: Type) -> (list: &Vec(A), item: &A) -> Int
```

<!-- stdlib:sig:list.find_index end -->

`item` 首次出现的下标。`item` 按只读借用传入（调用端自动创建 `&A` 令牌），调用后实参仍可用。

返回：找到返回下标；未找到返回 `-1`。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.find_index([1, 2, 3, 4], 3) == 2)
    assert(list.find_index([1, 2], 99) == -1)
}
```

### map

<!-- stdlib:sig:list.map start -->

```yaoxiang
map: (T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)
```

<!-- stdlib:sig:list.map end -->

对每个元素调用 `fn`，返回结果组成的新列表。传函数值是**柯里化**形态：
`list.map(nums, x => x * 2)`。源列表不变。

`map` 是静态泛型（`src/std/list.yx:210`），第二个参数不是函数时报编译期类型错误。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    doubled = list.map([1, 2, 3], x => x * 2)
    assert(list.get(doubled, 0) == 2)
}
```

### filter

<!-- stdlib:sig:list.filter start -->

```yaoxiang
filter: (T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)
```

<!-- stdlib:sig:list.filter end -->

保留使 `fn` 为真的元素。源列表不变。

`filter` 是静态泛型（`src/std/list.yx:222`），第二个参数不是函数时报编译期类型错误。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    evens = list.filter([1, 2, 3, 4], x => x % 2 == 0)
    assert(list.len(evens) == 2)
}
```

### reduce

<!-- stdlib:sig:list.reduce start -->

```yaoxiang
reduce: (T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc
```

<!-- stdlib:sig:list.reduce end -->

从左到右折叠：以 `init` 为初值，依次调用 `fn(acc, item)`。

- `fn` —— 归约函数 `(累加值, 元素) -> 新累加值`
- `init` —— 初始累加值

返回：最终累加值。列表为空时返回 `init`。

`reduce` 是静态泛型（`src/std/list.yx:241`），第二个参数不是函数时报编译期类型错误。
注意 `f` 按值接收累加器与元素——每轮把当前累加器交给 `f`，用返回值作为下一轮的累加器。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    total = list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0)
    assert(total == 10)
}
```

### iter

<!-- stdlib:sig:list.iter start -->

```yaoxiang
iter: (T: Type) -> (list: Vec(T)) -> Iter(T)
```

<!-- stdlib:sig:list.iter end -->

创建迭代器。迭代器是本模块导出的记录类型 `Iter(T)`，含两个具名字段——缓冲与游标
（`src/std/list.yx:259-262`）：

```
Iter: (T: Type) -> Type = {
    buf: Vec(T),
    pos: Int,
}
```

源列表按值传入，调用后即被**移动**。

返回：`Iter(T)` 值，交给 [`next`](#next) / [`has_next`](#has_next) 使用。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))
}
```

### next

<!-- stdlib:sig:list.next start -->

```yaoxiang
next: (T: Type) -> (it: &mut Iter(T)) -> T
```

<!-- stdlib:sig:list.next end -->

取出当前元素并把内部游标 `pos` **原地**前移一位。

返回：当前元素。

> **借用语义**：`next` 取 `&mut Iter(T)`、`has_next` 取 `&Iter(T)`，**都不移动**迭代器，
> 因此同一个迭代器可以连续取元素。调用前请先判 [`has_next`](#has_next)：迭代结束后再取
> 会由 `Vec` 的边界检查兜住并报 `E6003`，**不返回 `Void`**
> （`src/std/list.yx:277-285`）。
>
> 这与 [`std.range.next`](range#next) 相反——`range.has_next` 的签名无 `&`，
> 会按值消耗迭代器。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([7, 8])

    // 同一个迭代器连续取——因为 next 借用而不消耗
    assert(list.next(it) == 7)
    assert(list.next(it) == 8)
    assert(!list.has_next(it))
}
```

### has_next

<!-- stdlib:sig:list.has_next start -->

```yaoxiang
has_next: (T: Type) -> (it: &Iter(T)) -> Bool
```

<!-- stdlib:sig:list.has_next end -->

是否还有未消费的元素。只读借用 `&Iter(T)`，不消耗迭代器。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1])
    assert(list.has_next(it))
    assert(list.next(it) == 1)
    assert(!list.has_next(it))
}
```

### for ... in 遍历

列表可直接用 `for ... in` 遍历，无需手工调用 `next`：

```yaoxiang
use std.assert

main: () -> Void = {
    mut sum = 0
    for x in [1, 2, 3] {
        sum = sum + x
    }
    assert(sum == 6)
}
```

## 相关

- [`std.range`](range) —— 区间迭代与惰性适配器
- [`std.assert`](assert) —— 示例中的断言工具
