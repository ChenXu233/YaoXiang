---
title: 'std.assert'
description: '断言'
---

# std.assert

断言模块。测试与示例里最常用的工具。

```yaoxiang
use std.assert
```

## 函数一览

<!-- stdlib:table:assert start -->

| 函数 | 签名 |
| ---- | ---- |
| `assert` | `(cond: Bool, ?msg: String) -> Never` |

<!-- stdlib:table:assert end -->

## 函数

### assert

<!-- stdlib:sig:assert.assert start -->

```yaoxiang
assert: (cond: Bool, ?msg: String) -> Never
```

<!-- stdlib:sig:assert.assert end -->

断言 `cond` 为真。

- `cond` —— 待判定的布尔表达式
- `msg` —— 可选消息，`?` 表示可省略；条件不成立时随诊断一起输出

返回：条件成立时返回 `Void`，不中断执行（`src/std/assert.rs:82-83` 实现在通过分支
返回 `RuntimeValue::Void`）。**声明的返回类型却是 `Never`**——签名
`(cond: Bool, ?msg: String) -> Never` 来自 `src/std/assert.rs:24-29`；两者不矛盾：
`Void` 的使用点（如 `x = assert(true)`）本来就接受 `Never <: T`（爆炸原理，见
[类型系统 §2.2](../language-spec/type-system.md)）。

错误：条件为假时抛出 `E6005`（断言失败），程序以非零码退出。

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "这个字面断言必然成立")
}
```

断言是**测试语料的主要判定手段**——`src/std/tests/*.yx` 与 `tests/yaoxiang/**` 都以 `assert`
失败即进程报错的方式工作：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## 类型族（未文档化）

`std.assert` 除 1 个 native 导出外，还注册了两个**类型族**（`TypeFamilyExport`），
它们属于类型宇宙而非值宇宙，因此**不计入上表的「函数一览」**：

| 名称     | 定义                                          | 来源                       |
| -------- | --------------------------------------------- | -------------------------- |
| `IsTrue` | `(b: Bool) -> Type`，`true => Void` / `false => Never` | `src/std/assert.rs:34-44` |
| `Assert` | `(cond: Bool) -> Type`，即 `IsTrue(cond)`     | `src/std/assert.rs:45-49`  |

另有 `EffectSpec::new("assert", [GammaAssume { predicate_arg: 0 }], true)`
（`src/std/assert.rs:53-59`），供流敏感假设集 `Γ` 注入。

> **当前无法在类型位置书写这两个名字**：`Assert(true)` 报 `E0010 Expected a type,
> found BoolLiteral(true)`，`assert.Assert(true)` 报 `E0012 Invalid syntax:
> declaration requires an initializer`。也就是说**只有运行时 `assert` 可用，
> 编译期精化原语尚未接通**。规范层面的记法见
> [标准库规范 §1.5](../language-spec/stdlib.md)。

## 相关

- [测试规范](../../dev/test-specification.md) —— 语料组织与判定约定
- [错误码参考](../error-code/) —— `E6005` 断言失败
