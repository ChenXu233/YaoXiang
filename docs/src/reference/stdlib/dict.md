---
title: 'std.dict'
description: '字典读写、键值视图与合并'
---

# std.dict

字典（`Dict(K, V)`）操作模块。

```yaoxiang
use std.dict
```

## 语义分类

| 类别           | 函数                                                           | 行为                       |
| -------------- | -------------------------------------------------------------- | -------------------------- |
| 只读借用       | `get` `has` `values` `keys` `entries` `len` `is_empty` `merge` | 源字典可反复使用           |
| **消耗源字典** | `set` `delete`                                                 | 源字典被移动，之后不可再用 |

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)

    // 只读借用：d 可反复使用
    assert(dict.get(d, "a") == 1)
    assert(dict.len(d) == 1)
    assert(dict.has(d, "a"))
}
```

## 函数一览

<!-- stdlib:table:dict start -->

| 函数 | 签名 |
| ---- | ---- |
| `get` | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any` |
| `set` | `(K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)` |
| `has` | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool` |
| `values` | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)` |
| `keys` | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)` |
| `entries` | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)` |
| `delete` | `(K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)` |
| `len` | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Int` |
| `is_empty` | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool` |
| `merge` | `(A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)` |
| `new` | `(K: Type, V: Type)() -> Dict(K, V)` |

<!-- stdlib:table:dict end -->## 函数

### set

<!-- stdlib:sig:dict.set start -->

```yaoxiang
set: (K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.set end -->

返回写入 `key` → `value` 的**新字典**。`dict` 按值传入，调用后即被**移动**。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d1 = dict.set(dict.new(), "a", 1)
    d2 = dict.set(d1, "b", 2)
    assert(dict.len(d2) == 2)
}
```

### get

<!-- stdlib:sig:dict.get start -->

```yaoxiang
get: (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any
```

<!-- stdlib:sig:dict.get end -->

按键取值（只读借用，`dict` 可复用）。

返回：键对应的值。错误：**键不存在时抛出 `E6008`**（键缺失）。取值前可用 [`has`](#has) 先判断。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.get(d, "a") == 1)
}
```

存在性判断后再取：

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(!dict.has(d, "nope"))
    if dict.has(d, "a") {
        assert(dict.get(d, "a") == 1)
    }
}
```

### has

<!-- stdlib:sig:dict.has start -->

```yaoxiang
has: (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool
```

<!-- stdlib:sig:dict.has end -->

`key` 是否存在于字典中。

错误：第一个参数不是字典时抛出 `E6007`。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.has(d, "a"))
    assert(!dict.has(d, "zzz"))
}
```

### delete

<!-- stdlib:sig:dict.delete start -->

```yaoxiang
delete: (K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.delete end -->

返回移除 `key` 后的**新字典**。`dict` 按值传入，调用后即被**移动**。

返回：新字典。删除不存在的键不会报错，字典保持不变。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    deleted = dict.delete(d, "a")
    assert(!dict.has(deleted, "a"))
}
```

### keys

<!-- stdlib:sig:dict.keys start -->

```yaoxiang
keys: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.keys end -->

返回全部键组成的列表（只读借用）。

> 返回顺序与哈希实现相关，**不保证稳定**。需要有序输出时请自行排序。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    ks = dict.keys(d)
    assert(list.len(ks) == 1)
}
```

### values

<!-- stdlib:sig:dict.values start -->

```yaoxiang
values: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.values end -->

返回全部值组成的列表（只读借用）。顺序不保证稳定。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    vs = dict.values(d)
    assert(list.len(vs) == 1)
}
```

### entries

<!-- stdlib:sig:dict.entries start -->

```yaoxiang
entries: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.entries end -->

返回键值对列表，每项为 `(key, value)` 元组。顺序不保证稳定。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    es = dict.entries(d)
    assert(list.len(es) == 1)
}
```

### len

<!-- stdlib:sig:dict.len start -->

```yaoxiang
len: (K: Type, V: Type)(dict: &Dict(K, V)) -> Int
```

<!-- stdlib:sig:dict.len end -->

条目数量。只读借用，`dict` 可反复使用。

错误：参数不是字典时抛出 `E6007`。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.len(d) == 1)
    assert(dict.len(d) == 1)      // 可复用
}
```

### is_empty

<!-- stdlib:sig:dict.is_empty start -->

```yaoxiang
is_empty: (K: Type, V: Type)(dict: &Dict(K, V)) -> Bool
```

<!-- stdlib:sig:dict.is_empty end -->

字典是否为空。

错误：参数不是字典时抛出 `E6007`。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    assert(dict.is_empty(dict.new()))
    d = dict.set(dict.new(), "a", 1)
    assert(!dict.is_empty(d))
}
```

### merge

<!-- stdlib:sig:dict.merge start -->

```yaoxiang
merge: (A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)
```

<!-- stdlib:sig:dict.merge end -->

合并两个字典，返回新字典。两个源字典都是只读借用，均不变。

键冲突时**以 `b` 的值覆盖 `a`**。

错误：任一参数不是字典时抛出 `E6007`。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    m = dict.merge(dict.set(dict.new(), "x", 10), dict.set(dict.new(), "y", 20))
    assert(dict.get(m, "x") == 10)
    assert(dict.get(m, "y") == 20)
}
```

## 相关

- [`std.list`](./list) —— 处理 `keys` / `values` / `entries` 的返回值
- [错误码参考](../error-code/) —— `E6008` 键缺失
