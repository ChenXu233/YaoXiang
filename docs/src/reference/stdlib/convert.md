---
title: 'std.convert'
description: '任意值到 String 的转换'
---

# std.convert

类型转换模块，目前提供值到 `String` 的转换。

```yaoxiang
use std.convert
```

## 转换规则

`to_string` 按值的运行时形态格式化：

| 类型     | 输出形态             | 示例                       |
| -------- | -------------------- | -------------------------- |
| `Void`   | `void`               | `void`                     |
| `Bool`   | `true` / `false`     | `true`                     |
| `Int`    | 十进制               | `42`                       |
| `Float`  | 见下                 | `3.14` / `2.0`             |
| `Char`   | 字符本身             | `a`                        |
| `String` | 原内容（**无引号**） | `hello`                    |
| `List`   | `[元素, ...]`        | `[1, 2, 3]`                |
| `Dict`   | `{k: v, ...}`        | `{a: 1}`                   |
| `Tuple`  | `(元素, ...)`        | `(1, hello)`               |
| `Array`  | `[元素, ...]`        | `[1, 2]`                   |
| `Range`  | `起始..结束`         | `1..5`（step 为 1 时省略） |
| `Bytes`  | `bytes[长度]`        | `bytes[3]`                 |

`Float` 的整数值会补一位小数（`2.0` 而非 `2`），便于区分 `Int` 与 `Float`。

## 函数一览

<!-- stdlib:table:convert start -->

| 函数               | 签名                |
| ------------------ | ------------------- |
| `to_string`        | `(value) -> String` |
| `int.to_string`    | `(self) -> String`  |
| `float.to_string`  | `(self) -> String`  |
| `bool.to_string`   | `(self) -> String`  |
| `char.to_string`   | `(self) -> String`  |
| `string.to_string` | `(self) -> String`  |
| `list.to_string`   | `(self) -> String`  |
| `dict.to_string`   | `(self) -> String`  |
| `tuple.to_string`  | `(self) -> String`  |
| `set.to_string`    | `(self) -> String`  |
| `range.to_string`  | `(self) -> String`  |

<!-- stdlib:table:convert end -->## 函数

### to_string

<!-- stdlib:sig:convert.to_string start -->

```yaoxiang
to_string: (value) -> String
```

<!-- stdlib:sig:convert.to_string end -->

把任意值转为其字符串表示。

- `value` —— 任意类型的值

返回：格式化后的字符串。参数缺失时返回 `"()"`。

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string(42) == "42")
    assert(convert.to_string(true) == "true")
    assert(convert.to_string(false) == "false")
}
```

字符串本身不加引号：

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string("hi") == "hi")
}
```

复合类型递归展开（格式不断言，以免脆弱）：

```yaoxiang
use std.assert
use std.convert
use std.string

main: () -> Void = {
    s_list = convert.to_string([1, 2, 3])
    assert(string.len(s_list) > 0)

    s_dict = convert.to_string({})
    assert(string.len(s_dict) > 0)
}
```

### 类型方法形态

除通用的 `convert.to_string` 外，模块还导出以下按类型绑定的同名函数，行为与其完全一致：

| 导出名             | 签名               |
| ------------------ | ------------------ |
| `int.to_string`    | `(self) -> String` |
| `float.to_string`  | `(self) -> String` |
| `bool.to_string`   | `(self) -> String` |
| `char.to_string`   | `(self) -> String` |
| `string.to_string` | `(self) -> String` |
| `list.to_string`   | `(self) -> String` |
| `dict.to_string`   | `(self) -> String` |
| `tuple.to_string`  | `(self) -> String` |
| `set.to_string`    | `(self) -> String` |
| `range.to_string`  | `(self) -> String` |

这些绑定供运行时的 `Stringable` 分派使用，日常代码直接用 [`convert.to_string`](#to_string) 即可。

> `Set` 类型已在语言层除名，`set.to_string` 保留为兼容占位。

## 相关

- [`std.io`](./io) —— `print` / `println` 的内部格式化走同一套规则
- [`std.string`](./string) —— 字符串操作
