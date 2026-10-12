---
title: 'std.result'
description: 'Result 与 Error 的构造和拆包'
---

# std.result

`Result(T, E)` 的拆包与 `Error` 载体的字段访问。`Result` 本身是 `std.result`
导出的记录式和类型（RFC-010）：构造用**变体构造语法**，解构用 `match` 变体模式，
`?` 传播由 `Try` 接口驱动（见下文）。

```yaoxiang
use std.result

r = Result(Int, String).ok(5)
e = Result(Int, String).err("boom")
```

> **本模块是「双实现」合并面**：`Result` 类型与 `Try` 四方法来自纯 yx 的
> `src/std/result.yx`，`is_ok` / `is_err` / `unwrap` / `unwrap_or` / `unwrap_err` /
> `code` / `message` / `error` 这 8 个工具来自 native 的 `src/std/result.rs`。
> 两者注册到同一导出面（`src/frontend/module/registry.rs:315-334` 显式做并集合并，
> 同名冲突以 yx 为准），所以 `use std.result` 一次即可拿到全部 9 个绑定。
> 但**模块级 `Try` 方法不可见**——`result.is_failure(...)` 报 `E1042`；`Try` 四方法
> 只能以方法调用语法 `r.is_failure()` 访问。

## 运行时表示

| 值                            | 表示                             |
| ----------------------------- | -------------------------------- |
| `Result(T, E).ok(value)`      | 枚举变体，携带 `value`           |
| `Result(T, E).err(error)`     | 枚举变体，携带 `error`           |
| `Error`                       | 结构体，字段为 `(code, message)` |

`Error.code` 为 RFC-013 的 `E6xxx` / `E7xxx` 段注册码（跨版本稳定契约）， `Error.message`
为人类可读描述。

## Try 接口（`?` 传播）

`Result` 在类型体里实例化了 `Try(Result(T, E), T, E)` 四方法接口（`src/std/result.yx:22`），
`?` 运算符据此驱动。这些方法也可显式调用，但**只能走方法语法**（模块级
`result.is_failure(...)` 报 `E1042`）：

| 方法                       | 签名                                       |
| -------------------------- | ------------------------------------------ |
| `r.is_failure()`           | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool` |
| `r.success()`              | `(T: Type, E: Type)(self: &Result(T, E)) -> T`     |
| `r.residual()`             | `(T: Type, E: Type)(self: &Result(T, E)) -> E`     |
| `Result(T, E).from_error(e)` | `(T: Type, E: Type)(e: E) -> Result(T, E)`   |

与 [`std.option`](option) 不同，`Result` 上的 `?` 传播是**可用**的：

```yaoxiang
use std.result
use std.string

parse_then_add_one: (String) -> Result(Int, Error) = (s) => {
    n = string.parse_int(s)?      // Err 原样上抛
    return Result(Int, Error).ok(n + 1)
}

main: () -> Void = {
    println(result.unwrap(parse_then_add_one("41")))   // 42
}
```

> **没有 `map` / `map_err`**：本模块的 8 个 native 导出
> （`src/std/result.rs:71-126`）不含这两个名字，`result.map(...)` 报
> `E1042 field 'map' not found in struct 'result'`。需要变换成功值请用
> `match` 变体解构。

## 函数一览

<!-- stdlib:table:result start -->

| 函数 | 签名 |
| ---- | ---- |
| `is_ok` | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool` |
| `is_err` | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool` |
| `unwrap` | `(T: Type, E: Type)(self: &Result(T, E)) -> T` |
| `unwrap_or` | `(T: Type, E: Type)(self: &Result(T, E), default: T) -> T` |
| `unwrap_err` | `(T: Type, E: Type)(self: &Result(T, E)) -> E` |
| `code` | `(self: &Error) -> String` |
| `message` | `(self: &Error) -> String` |
| `error` | `(code: &String, message: &String) -> Error` |

<!-- stdlib:table:result end -->## 判定

### is_ok

<!-- stdlib:sig:result.is_ok start -->

```yaoxiang
is_ok: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_ok end -->

是否为成功变体。只读借用，`self` 可反复使用。

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).ok(1)
    assert(result.is_ok(r))
    assert(result.is_ok(r))      // 可复用
}
```

### is_err

<!-- stdlib:sig:result.is_err start -->

```yaoxiang
is_err: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_err end -->

是否为错误变体。只读借用。

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).err("e")
    assert(result.is_err(r))
}
```

## 取值

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

取出成功值。

返回：`Ok` 变体携带的值。错误：对 `Err` 值调用时抛出 `E6007`，消息中**附带原始错误码与描述**，形如
`unwrap called on Err value (E6010: parse_int: ...)`，因此无需先 `unwrap_err` 就能看到失败原因。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("42")
    assert(result.unwrap(r) == 42)
}
```

### unwrap_or

<!-- stdlib:sig:result.unwrap_or start -->

```yaoxiang
unwrap_or: (T: Type, E: Type)(self: &Result(T, E), default: T) -> T
```

<!-- stdlib:sig:result.unwrap_or end -->

取出成功值，或在 `Err` 时返回 `default`。

- `default` —— `Err` 时的兜底值

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    good = string.parse_int("42")
    assert(result.unwrap_or(good, 0) == 42)

    bad = string.parse_int("abc")
    assert(result.unwrap_or(bad, 0) == 0)
}
```

### unwrap_err

<!-- stdlib:sig:result.unwrap_err start -->

```yaoxiang
unwrap_err: (T: Type, E: Type)(self: &Result(T, E)) -> E
```

<!-- stdlib:sig:result.unwrap_err end -->

取出错误值。

返回：`Err` 变体携带的值。错误：对 `Ok` 值调用时抛出 `E6007`。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(result.is_err(r))
}
```

## Error 字段

### code

<!-- stdlib:sig:result.code start -->

```yaoxiang
code: (self: &Error) -> String
```

<!-- stdlib:sig:result.code end -->

读取错误码字符串，如 `"E6010"`。

> 签名类型为 `Error`，但运行时错误载体是以 `(code, message)` 为字段的结构体。直接对 `Error` 值调用。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(result.code(e) == "E6010")
}
```

### message

<!-- stdlib:sig:result.message start -->

```yaoxiang
message: (self: &Error) -> String
```

<!-- stdlib:sig:result.message end -->

读取错误描述文本。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(string.len(result.message(e)) > 0)
}
```

## 构造

### error

<!-- stdlib:sig:result.error start -->

```yaoxiang
error: (code: &String, message: &String) -> Error
```

<!-- stdlib:sig:result.error end -->

构造一个 `Error` 值。这是纯 yx 层构造 `Error` 的**唯一通道**——`Error` 类型族只注册
类型身份、没有值空间构造子，所以 yx 里写 `Error("E…", msg)` 会在 IR 层报 `E3006`
（`src/std/result.rs:117-124`）。

- `code` —— 错误码字符串（RFC-013 的 `E6xxx` / `E7xxx` 段注册码）
- `message` —— 人类可读描述

返回：新造的 `Error` 值，可直接交给 [`code`](#code) / [`message`](#message) 读取，
或作为 `Result.err(...)` 的载荷。

> **码表是注册制**：`src/std/result.rs:26-32` 的 `RUNTIME_ERROR_CODES` 只列出 5 个已注册码
> （`E6009` / `E6010` / `E6011` / `E6012` / `E6013`）。自造码（如 `"E9999"`）在程序内
> 可以流通，但 `yx explain` 查不到文档——只在自己约定的子域内用。

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    e = result.error("E6010", "parse_int failed")
    assert(result.code(e) == "E6010")
    assert(result.message(e) == "parse_int failed")

    r = Result(Int, Error).err(e)
    assert(result.is_err(r))
    assert(result.code(result.unwrap_err(r)) == "E6010")
}
```

## 相关

- [`std.string`](string#parse_int) —— 产生 `Result` 的解析函数
- [`std.range`](range#iter) —— `step=0` 时返回带 `E6009` 的 `Err`
- [`std.option`](option) —— `Option(T)`；其 `?` 传播当前**不可用**，见该页「已知缺口」
- [错误码参考](../error-code/) —— `E6010` / `E6011` 等运行时错误值码
