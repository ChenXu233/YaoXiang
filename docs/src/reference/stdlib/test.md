---
title: 'std.test'
description: '测试断言库（值语义）'
---

# std.test

测试断言库。YaoXiang 的测试库用 YaoXiang 自己写——这是第一个 dogfooding 模块
（RFC-036 §3）。

**值语义契约**：断言失败以 `Err(诊断信息)` 表达，**不 abort 进程**。每个断言函数的返回
类型都是 `Result(Void, String)`，因此可以在一个程序里连跑多条断言、由调用方决定成败。
进程级 abort 语义保留在 [`std.assert`](assert)（运行时守卫），不进入测试断言路径。

```yaoxiang
use std.test
```

> **本页是手写页**。本模块由纯 YaoXiang 源码实现（`src/std/test.yx`），不在
> `StdModule::exports()` 的生成范围内，因此没有 `<!-- stdlib:... -->` 生成区标记。
> 下表的签名逐字取自该文件的顶层绑定声明。

## 导出

| 导出              | 签名                                                              | 说明                         |
| ----------------- | ----------------------------------------------------------------- | ---------------------------- |
| `assert_eq`       | `(a: Any, b: Any) -> Result(Void, String)`                          | 相等断言                     |
| `assert_ne`       | `(a: Any, b: Any) -> Result(Void, String)`                          | 不等断言                     |
| `assert_true`     | `(cond: Bool) -> Result(Void, String)`                              | 为真断言                     |
| `assert_false`    | `(cond: Bool) -> Result(Void, String)`                              | 为假断言                     |
| `assert_not`      | `(cond: Bool) -> Result(Void, String)`                              | `assert_false` 同体          |
| `assert_err`      | `(T: Type, E: Type)(r: Result(T, E)) -> Result(Void, String)`       | `Result` 为 `Err`            |
| `assert_err_code` | `(T: Type)(r: Result(T, Error), want: String) -> Result(Void, String)` | 错误码相等断言           |
| `assert_approx_eq`| `(a: Float, b: Float, eps: Float) -> Result(Void, String)`         | 浮点近似相等断言             |
| `suite`           | `(tests: Vec((String, () -> Result(Void, String)))) -> Void`         | 逐个跑并汇总                 |

## 断言函数

前五个的形态一致：`Ok(void)` 表示通过，`Err(消息)` 表示失败，**不中断执行**。

| 函数          | 失败消息                                     |
| ------------- | -------------------------------------------- |
| `assert_eq`   | `Expected {b}, got {a}`                      |
| `assert_ne`   | `Expected not equal to {b}, got {a}`         |
| `assert_true` | `Expected true, got {cond}`                  |
| `assert_false`| `Expected false, got {cond}`                 |
| `assert_not`  | 同 `assert_false`                             |

`assert_not` 与 `assert_false` 当前是同一份实现——一元形态 `!assert` 要等 `not` 语法落地
后才换装（RFC-036 §8.1）。

```yaoxiang
use std.assert
use std.result
use std.test

main: () -> Void = {
    assert(result.is_ok(test.assert_eq(1, 1)))
    assert(result.is_err(test.assert_eq(1, 2)))
    assert(result.unwrap_err(test.assert_eq(1, 2)) == "Expected 2, got 1")

    assert(result.is_ok(test.assert_ne(1, 2)))
    assert(result.is_err(test.assert_ne(1, 1)))

    assert(result.is_ok(test.assert_true(true)))
    assert(result.is_err(test.assert_true(false)))
    assert(result.is_ok(test.assert_false(false)))
    assert(result.is_ok(test.assert_not(false)))

    // 失败也不中断：下面这几行照常执行
    _ = test.assert_eq(1, 2)
    println("still running")
}
```

`assert_eq` / `assert_ne` 的实参类型是 `Any`——比较走值级 `==`，元素类型须支持相等比较
（基础类型原生支持）。

## assert_err

判定一个 `Result` 是 `Err` 变体。`T` / `E` 由调用点实参推断，不必显式给。

```yaoxiang
use std.assert
use std.result
use std.string
use std.test

main: () -> Void = {
    // 解析失败 → Err
    assert(result.is_ok(test.assert_err(string.parse_int("abc"))))

    // 解析成功 → Ok，断言失败
    assert(result.is_err(test.assert_err(string.parse_int("12"))))
}
```

## assert_err_code

进一步断言 `Err` 载体的**错误码**等于预期值。`E` 钉死为 `Error`——码只存在于标准库
`Error` 载体上（`code` / `message` 两个字段，见 [`std.result`](result)）。

失败消息为 `Expected code {want}, got {c}`。

```yaoxiang
use std.assert
use std.result
use std.string
use std.test

main: () -> Void = {
    // string.parse_int 失败码为 E6010
    assert(result.is_ok(test.assert_err_code(string.parse_int("abc"), "E6010")))

    r = test.assert_err_code(string.parse_int("abc"), "E9999")
    assert(result.is_err(r))
    assert(result.unwrap_err(r) == "Expected code E9999, got E6010")
}
```

## assert_approx_eq

按 `|a - b| <= eps` 判定浮点近似相等。

- `eps` —— **由调用方显式给出**，容差是测试契约的一部分，不设隐藏默认值
- `eps < 0` —— 恒 `Err`（不可能通过的契约不该等到断言失败才发现）
- `NaN` —— 恒 `Err`（`NaN` 减任何值都是 `NaN`，比较恒假）

失败消息为 `Expected {b} (±{eps}), got {a} (diff {d})`。

```yaoxiang
use std.assert
use std.result
use std.test

main: () -> Void = {
    assert(result.is_ok(test.assert_approx_eq(1.0, 1.05, 0.1)))
    assert(result.is_err(test.assert_approx_eq(1.0, 1.5, 0.1)))
    assert(result.is_err(test.assert_approx_eq(1.0, 1.0, -1.0)))
}
```

## suite

逐个调用测试函数并收集 per-test 判定（RFC-036 §7 值化模型）。

```
suite: (tests: Vec((String, () -> Result(Void, String)))) -> Void
```

- `tests` —— `(名字, 测试函数)` 对的列表；测试函数**零参**、返回
  `Result(Void, String)`

行为：

- **全 Ok 静默**，文件退出码 `0`
- 任一 `Err`：汇总失败明细（名字 + 诊断）后 abort，文件退出码非 `0`——
  这里的 abort 是**测试二进制的运行时守卫**（走 `assert.assert(failed == 0, …)`），
  不是断言路径
- 某个测试失败后其余测试**照常运行**

> 顶层测试函数名暂不能作为值引用（IR 层限制），入列时用闭包：
> `("name", () => test_fn())`。

```yaoxiang
use std.assert
use std.test

t_add: () -> Result(Void, String) = () => test.assert_eq(1 + 1, 2)

main: () -> Void = {
    test.suite([
        ("add", () => t_add()),
        ("truth", () => test.assert_true(true)),
    ])
    println("all passed")
}
```

套件里有失败项时报 `E6005`，消息形如：

```
1 of 1 test(s) failed
  [FAIL] bad: Expected 2, got 1
```

——**失败示例，仅说明行为**（刻意不用 `yaoxiang` 围栏，示例门禁只跑可运行块）：

```
test.suite([("bad", () => test.assert_eq(1, 2))])
// → E6005: assertion failed: 1 of 1 test(s) failed
//              [FAIL] bad: Expected 2, got 1
```

## 相关

- [`std.assert`](assert) —— 进程级运行时守卫（`E6005`），与本模块的值语义互补
- [`std.result`](result) —— `Result` 的拆包工具族
- [错误码参考](../error-code/) —— `E6010` / `E6011` 等 `Err` 载体上的错误码
