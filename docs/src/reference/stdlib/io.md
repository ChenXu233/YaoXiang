---
title: 'std.io'
description: '标准输出、标准输入与文件整体读写'
---

# std.io

输入输出模块。提供标准输出、标准输入读取，以及“一次性读写整个文件”的便捷函数。需要按句柄增量读写文件时用
[`std.os`](./os)。

```yaoxiang
use std.io
```

## 平台可用性

`read_line` 起以下函数依赖操作系统 I/O，在 `wasm32` 目标上**不导出**：
`read_line`、`read_file`、`write_file`、`append_file`。 `print` / `println` / `format_fallback`
在所有目标上可用。

## 函数一览

<!-- stdlib:table:io start -->

| 函数              | 签名                                        |
| ----------------- | ------------------------------------------- |
| `print`           | `(...args) -> Void`                         |
| `println`         | `(...args) -> ()`                           |
| `read_line`       | `() -> String`                              |
| `read_file`       | `(path: &String) -> String`                 |
| `write_file`      | `(path: &String, content: &String) -> Bool` |
| `append_file`     | `(path: &String, content: &String) -> Bool` |
| `format_fallback` | `(value, type_name: &String) -> String`     |

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
use std.io

main = {
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
main = {
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

main = {
    println("请输入你的名字：")
    name = io.read_line()
    println("你好，" + name)
}
```

### read_file

<!-- stdlib:sig:io.read_file start -->

```yaoxiang
read_file: (path: &String) -> String
```

<!-- stdlib:sig:io.read_file end -->

一次性读取整个文件内容为字符串。

- `path` —— 文件路径（只读借用）

返回：文件全部内容。错误：文件不存在或无权限时抛出 `E6007`。**不返回空串**。

```yaoxiang
use std.assert
use std.io
use std.os
use std.string

main = {
    p = "__yx_doc_read_file.txt"
    io.write_file(p, "hello")

    content = io.read_file(p)
    assert(content == "hello")

    os.remove(p)
}
```

### write_file

<!-- stdlib:sig:io.write_file start -->

```yaoxiang
write_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:io.write_file end -->

把 `content` 写入 `path`，**覆盖**原有内容；文件不存在时创建。

- `path` —— 文件路径（只读借用）
- `content` —— 要写入的内容（只读借用）

返回：写入成功返回 `true`。错误：目录不存在或无权限时抛出 `E6007`（不返回 `false`）。

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_write_file.txt"
    ok = io.write_file(p, "hello")
    assert(ok)
    assert(os.exists(p))
    os.remove(p)
}
```

### append_file

<!-- stdlib:sig:io.append_file start -->

```yaoxiang
append_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:io.append_file end -->

把 `content` **追加**到 `path` 末尾；文件不存在时创建。

返回：写入成功返回 `true`。错误：无权限时抛出 `E6007`。

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_append_file.txt"
    io.write_file(p, "hello")
    io.append_file(p, " world")
    assert(io.read_file(p) == "hello world")
    os.remove(p)
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

main = {
    s = io.format_fallback(42, "int")
    assert(string.contains(s, "42"))
}
```

## 相关

- [`std.os`](./os) —— 文件句柄、目录与环境变量
- [`std.convert`](./convert) —— 值转字符串
