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
| **消耗源列表** | `push` `append` `prepend` `set` `pop` `remove_at` | 源列表被移动，之后不可再用 |
| 只读借用       | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` | 源列表可反复使用 |
| 迭代协议       | `iter`（消耗源列表，返回迭代器）`has_next` `next`（借用 / 可变借用迭代器） | 见下方说明 |

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
| `contains`   | `(A: Type) -> (list: &Vec(A), item: A) -> Bool`                                            |
| `find_index` | `(A: Type) -> (list: &Vec(A), item: A) -> Int`                                             |
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

返回：去掉该元素的新列表。错误：下标为负或 ≥ 长度时抛出 `E6003`（索引越界）。

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

- `index` —— 下标；缺省 `0`
- `value` —— 新值；缺省 `Void`

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

- `index` —— 下标；缺省 `0`

返回：元素值；**越界返回 `Void`**（不抛错）。错误：下标为负时抛出 `E6007`。

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

返回首元素；空列表返回 `Void`。

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

返回末元素；空列表返回 `Void`。

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

- `start` —— 起始下标；缺省 `0`
- `end` —— 结束下标（不含）；缺省为列表末尾

返回：新列表。边界被**钳制**到合法范围，不报错。错误：`start` 或 `end` 为负时抛出 `E6007`。

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

错误：第二个参数不是列表时抛出 `E6007`。

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

错误：参数不是列表时抛出 `E6007`。

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

错误：参数不是列表时抛出 `E6007`。

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
contains: (A: Type) -> (list: &Vec(A), item: A) -> Bool
```

<!-- stdlib:sig:list.contains end -->

`item` 是否在列表中（按值相等比较）。

返回：存在为 `true`；参数不是列表时返回 `false`。

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
find_index: (A: Type) -> (list: &Vec(A), item: A) -> Int
```

<!-- stdlib:sig:list.find_index end -->

`item` 首次出现的下标。

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

错误：第二个参数不是函数时抛出 `E6007`。

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

错误：第二个参数不是函数时抛出 `E6007`。

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

错误：第二个参数不是函数时抛出 `E6007`。

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

创建迭代器。迭代器是一个 `(列表, 下标)` 元组状态载体，创建后在 `next` 中
**顺序消费**。源列表为只读借用，迭代期间仍可使用。

返回：迭代器元组，交给 `next` / `has_next` 使用。

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

取出当前元素并把内部下标前移一位。

返回：当前元素；迭代结束时返回 `Void`。

> `next` 与 `has_next` 都**移动**迭代器（签名无 `&`），因此每次取用都需要重新创建迭代器，或直接用
> `for ... in` 遍历。这与 [`std.range.next`](./range#next) 的借用形态不同。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([7, 8])
    assert(list.next(it) == 7)
}
```

### has_next

<!-- stdlib:sig:list.has_next start -->

```yaoxiang
has_next: (T: Type) -> (it: &Iter(T)) -> Bool
```

<!-- stdlib:sig:list.has_next end -->

是否还有未消费的元素。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1])
    assert(list.has_next(it))
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

- [`std.range`](./range) —— 区间迭代与惰性适配器
- [`std.assert`](./assert) —— 示例中的断言工具
