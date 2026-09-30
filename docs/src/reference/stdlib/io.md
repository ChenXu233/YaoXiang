---
title: 'std.io'
description: '标准输出、标准输入与格式化'
---

# std.io

输入输出模块。提供标准输出与标准输入读取。整文件读写、目录与路径操作见
[`std.fs`](./fs)；句柄级增量读写见 [`std.os`](./os)。

```yaoxiang
use std.io
```

## 平台可用性

`read_line` 依赖操作系统 I/O，在 `wasm32` 目标上**不导出**。`print` / `println` /
`format_fallback` 在所有目标上可用。

## 函数一览

<!-- stdlib:table:io start -->

| 函数 | 签名 |
| ---- | ---- |
| `print` | `(...args) -> Void` |
| `println` | `(...args) -> ()` |
| `read_line` | `() -> String` |
| `format_fallback` | `(value, type_name: &String) -> String` |

<!-- stdlib:table:io end -->## 函数

### print

<!-- stdlib:sig:io.print start -->

```yaoxiang
print: (...args) -> Void
```

<!-- stdlib:sig:io.print end -->

按顺序输出所有参数，**不加换行**。多个参数之间以单个空格分隔。

参数会被格式化：`String` 直接输出内容；`List` / `Dict` / `Tuple` 递归展开；其余值按字面量输出。

```yaoxiang

main: () -> Void = {
    print("hello")
    print(" ")
    print("world")
    println("")
}
```

### println

<!-- stdlib:sig:io.println start -->

```yaoxiang
println: (...args) -> ()
```

<!-- stdlib:sig:io.println end -->

同 [`print`](#print)，但在输出末尾追加换行。

`println()` 不带参数时输出一个空行：

```yaoxiang
main: () -> Void = {
    println("Hello, YaoXiang!")
}
```

### read_line

<!-- stdlib:sig:io.read_line start -->

```yaoxiang
read_line: () -> String
```

<!-- stdlib:sig:io.read_line end -->

从标准输入读取一行。

返回：读到的整行内容，**已去除结尾的换行符**（`\n` 或 `\r\n`）。错误：读取失败时抛出 `E6007`。

> 交互式示例无法在文档中自动运行，以下写法仅供参考。

```yaoxiang
use std.io

main: () -> Void = {
    println("请输入你的名字：")
    name = io.read_line()
    println("你好，" + name)
}
```

### format_fallback

<!-- stdlib:sig:io.format_fallback start -->

```yaoxiang
format_fallback: (value, type_name: &String) -> String
```

<!-- stdlib:sig:io.format_fallback end -->

按类型名格式化值，输出形如 `int(42)` / `list@3` 的带前缀表示。

这是内部辅助函数，供运行时的通用格式化路径回调使用，日常代码应直接用
[`std.convert.to_string`](./convert#to_string)。

- `value` —— 任意值
- `type_name` —— 类型名字符串

返回：带类型前缀的字符串表示。

```yaoxiang
use std.assert
use std.io
use std.string

main: () -> Void = {
    s = io.format_fallback(42, "int")
    assert(string.contains(s, "42"))
}
```

## 相关

- [`std.fs`](./fs) —— 文件、目录与路径操作
- [`std.os`](./os) —— 文件句柄与环境变量
- [`std.convert`](./convert) —— 值转字符串
