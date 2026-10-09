---
title: 'std.option'
description: 'Option 可选值与和类型'
---

# std.option

`Option(T)`——「有值（`some`）或无值（`none`）」的可选值类型。它是本模块导出的
**记录式和类型**（RFC-010），与用户自定义和类型同一套机制：变体构造、match 变体解构、
穷尽性检查。使用前 `use std.option`。

```yaoxiang
use std.option
```

> **本页是手写页**。本模块由纯 YaoXiang 源码实现（`src/std/option.yx`），不在
> `StdModule::exports()` 的生成范围内，因此没有 `<!-- stdlib:... -->` 生成区标记。
> 下表的签名逐字取自该文件的导出声明。

## 导出

| 导出     | 签名       | 说明                                          |
| -------- | ---------- | --------------------------------------------- |
| `Option` | `(T: Type) -> Type` | 和类型，含 `some` / `none` 两个变体 |

模块**只导出 `Option` 这一个绑定**。`Try` 的四个方法在类型体里声明（见下文），
**不是模块级导出**——`option.is_failure(...)` 报 `E1042 field 'is_failure' not found in
struct 'option'`。它们以**方法调用**语法 `o.is_failure()` 访问。

## 类型体

```
Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

- `some` —— 携带一个 `T` 的有值变体
- `none` —— 不携带载荷的无值变体
- `Try(Option(T), T, Void)` —— 失败时残余类型为 `Void`（对应 Rust `Try` 实验的
  `NoneT` 语义）

变体构造在表达式位置**必须类型限定**：`Option(Int).some(5)`、`Option(Int).none()`；
裸名 `some(5)` 不是构造器调用（见[语法规范 §1.4.2](../language-spec/syntax.md)）。

## 函数

类型体里定义的 `Try` 方法（`src/std/option.yx:23-45`）：

| 方法                      | 签名                                          | `some` 上 | `none` 上        |
| ------------------------- | --------------------------------------------- | --------- | ---------------- |
| `is_failure()`            | `(self: &Option(T)) -> Bool`                   | `false`   | `true`           |
| `success()`               | `(self: &Option(T)) -> T`                     | 返回载荷  | `E6005`（死路）  |
| `residual()`              | `(self: &Option(T)) -> Void`                  | `E6005`（死路） | 返回 `void` |
| `from_error(v: Void)`     | `(v: Void) -> Option(T)`                      | 见[已知缺口](#已知缺口) | 见[已知缺口](#已知缺口) |

后三者中的「死路」是类型体里显式写的 `assert(false)`（`src/std/option.yx:33,40`），
`Never <: T` 让这些分支在类型层合法、在运行时以 `E6005` 终止。

```yaoxiang
use std.option

main: () -> Void = {
    s = Option(Int).some(7)
    println(s.is_failure())   // false
    println(s.success())      // 7
    // s.residual() 会发散：some 臂是死路（E6005），不可调用

    n = Option(Int).none()
    println(n.is_failure())   // true
    println(n.residual())     // void
    // n.success() 同样发散：none 臂是死路（E6005）
}
```

## match 变体解构

解构 `Option` 需要变体集在场，即先 `use std.option`（见[语法规范 §2.8](../language-spec/syntax.md)）。
穷尽性检查按变体集全查——`some` / `none` 两个臂都要写，兜底臂 `_` 可豁免。

```yaoxiang
use std.option

name_of: (Int) -> Option(Int) = (x) => {
    if x > 0 {
        return Option(Int).some(x)
    }
    return Option(Int).none()
}

main: () -> Void = {
    for x in [1, -1] {
        v = name_of(x)
        match v {
            some(n) => println(n),
            none() => println("no value"),
        }
    }
}
```

## 已知缺口

以下三处「声明了但当前跑不通」，逐条都经 `yaoxiang-rs run` 实测确认：

1. **`?` 传播不适用于 `Option`。** 类型体实例化了 `Try(Option(T), T, Void)`，但类型检查器
   只承认 `Result`：`o?` 报 `E1081 ? is only allowed in functions returning a type that
   implements Try`。`Result` 上的 `?` 正常工作，可作对照。
2. **`from_error` 无法调用。** 写成方法 `o.from_error()` 编译通过但运行时报
   `E6006 Function not found: from_error`；写成 `Option(T).from_error(...)` 编译期即报
   `E1042`。它是 `?` 传播的内部桥，既然 `?` 不可用，这座桥也就没有调用点。
3. **没有 `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`。** 这些名字在标准库里
   0 命中；判定失败用 `is_failure()`，取载荷用 `success()`。`Result` 那侧的对应工具在
   [`std.result`](result)。

## 相关

- [`std.result`](result) —— `Result(T, E)`；`?` 传播当前**只**在它上面可用
- [`std.assert`](assert) —— 死路分支的 `assert(false)` 来源
- [语法规范 §2.8](../language-spec/syntax.md) —— 变体解构与穷尽性检查
