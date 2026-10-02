---
title: 'std.os'
description: '文件句柄、环境变量与工作目录'
---

# std.os

操作系统接口模块：文件句柄读写、环境变量与工作目录。路径级文件操作（整文件读写、
目录、元数据、路径运算）见 [`std.fs`](./fs)。

```yaoxiang
use std.os
```

> 本模块全部函数依赖操作系统能力，在 `wasm32` 目标上**不导出**。

## 文件句柄模型

> **句柄按引用传递（#337 已修复）**：`read` / `write` / `seek` / `tell` / `flush` /
> `close` 的签名均为 `(file: &File, ...)`，故句柄可反复使用：
>
> ```yaoxiang
> f = os.open(p, "w")
> os.write(f, "hello world")
> os.close(f)
> ```
>
> 定位后读写（`seek` 存在的理由）也已可用：
>
> ```yaoxiang
> r = os.open(p, "r")
> os.seek(r, 6)
> tail = os.read(r, 5)     // "world"
> os.close(r)
> ```
>
> 修复前的签名无 `&`，句柄按值传入 → 线性所有权 → 用一次即失效，
> `open → write → close` 会报 `E2014`。

若想避免手工管理句柄，可用不开句柄的便捷函数——它们在
[`std.fs`](./fs)（**不在** `std.io`）：`fs.read_file` / `fs.write_file` /
`fs.append_file`。本模块（`std.os`）只有句柄级增量读写，没有 `append_file`
（`os.append_file` 报 `E1042`）。

`open` 返回的类型标注是 `File`——引擎内部维护一张句柄表（`src/std/os.rs:98`），
底层是 `std::fs::File`，对用户是**不透明的句柄值**。

写入后内容立即落盘，无需显式 `close`：

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_open.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    assert(fs.read_file(p) == "hello")
    fs.remove(p)
}
```

`open` 支持的模式：

| 模式 | 含义                   |
| ---- | ---------------------- |
| `r`  | 只读，文件须存在       |
| `w`  | 只写，创建或清空       |
| `a`  | 追加，创建或追加到末尾 |
| `r+` | 读写，文件须存在       |
| `w+` | 读写，创建或清空       |
| `a+` | 读写，创建或追加       |

## 函数一览

<!-- stdlib:table:os start -->

| 函数 | 签名 |
| ---- | ---- |
| `open` | `(path: &String, mode: &String) -> File` |
| `close` | `(file: &File) -> Void` |
| `read` | `(file: &File, n: Int) -> String` |
| `write` | `(file: &File, content: String) -> Int` |
| `seek` | `(file: &File, offset: Int) -> Bool` |
| `tell` | `(file: &File) -> Int` |
| `flush` | `(file: &File) -> Void` |
| `get_env` | `(name: &String) -> String` |
| `set_env` | `(name: &String, value: &String) -> Void` |
| `args` | `() -> String` |
| `chdir` | `(path: &String) -> Bool` |
| `getcwd` | `() -> String` |

<!-- stdlib:table:os end -->## 文件操作

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

打开文件并返回句柄。

- `path` —— 文件路径（只读借用）
- `mode` —— 打开模式，见上表

返回：`File` 句柄值。**句柄按引用传递**——`write` / `seek` / `read` / `tell` / `flush` /
`close` 的形参都是 `&File`（`src/std/os.rs:35-65`），所以同一个句柄可以反复使用，
直到显式 [`close`](#close)。通常把 `open` 内联进单次调用以省去收尾。

错误：模式非法、文件不存在或无权限时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_open_only.txt"
    f = os.open(p, "w")
    assert(fs.exists(p))
    os.close(f)
    fs.remove(p)
}
```

### close

<!-- stdlib:sig:os.close start -->

```yaoxiang
close: (file: &File) -> Void
```

<!-- stdlib:sig:os.close end -->

关闭文件句柄并释放表项。

形参是 `&File`，所以 `close` 可以放在一串读写的末尾，把句柄用完再释放。写入内容在
[`write`](#write) 返回时已落盘（`os.open(…, "w")` 走 `OpenOptions::create(true)`），
通常无需显式关闭——但长时间持有大量句柄时应当收尾。

错误：句柄无效（未打开或已关闭）时抛出 `E6007`。

```yaoxiang
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_close.txt"
    f = os.open(p, "w")
    os.write(f, "data")
    os.close(f)
    fs.remove(p)
}
```

### read

<!-- stdlib:sig:os.read start -->

```yaoxiang
read: (file: &File, n: Int) -> String
```

<!-- stdlib:sig:os.read end -->

从当前读写位置读取**最多** `n` 字节。

- `file` —— 文件句柄
- `n` —— 期望读取的字节数

返回：实际读到的内容（可能短于
`n`，到达文件末尾时为空串）。非法 UTF-8 字节以替换字符形式返回，不报错。错误：句柄无效或读取失败时抛出
`E6007`。

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_read.txt"
    fs.write_file(p, "abcdef")

    part = os.read(os.open(p, "r"), 3)
    assert(part == "abc")
    fs.remove(p)
}
```

### write

<!-- stdlib:sig:os.write start -->

```yaoxiang
write: (file: &File, content: String) -> Int
```

<!-- stdlib:sig:os.write end -->

向当前读写位置写入全部 `content`。

- `content` —— 按值传入

返回：写入的**字节数**。错误：句柄无效或写入失败时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_write.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    fs.remove(p)
}
```

### seek

<!-- stdlib:sig:os.seek start -->

```yaoxiang
seek: (file: &File, offset: Int) -> Bool
```

<!-- stdlib:sig:os.seek end -->

把读写位置移动到**绝对**偏移 `offset`（相对文件开头）。

- `offset` —— 目标字节偏移，须非负

返回：成功返回 `true`。错误：句柄无效或偏移非法时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_seek.txt"
    fs.write_file(p, "abcdef")

    ok = os.seek(os.open(p, "r"), 2)
    assert(ok)
    fs.remove(p)
}
```

### tell

<!-- stdlib:sig:os.tell start -->

```yaoxiang
tell: (file: &File) -> Int
```

<!-- stdlib:sig:os.tell end -->

返回当前读写位置的字节偏移。

错误：句柄无效时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_tell.txt"
    pos = os.tell(os.open(p, "w"))
    assert(pos == 0)
    fs.remove(p)
}
```

### flush

<!-- stdlib:sig:os.flush start -->

```yaoxiang
flush: (file: &File) -> Void
```

<!-- stdlib:sig:os.flush end -->

把缓冲内容刷入磁盘。

错误：句柄无效或刷新失败时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_flush.txt"
    os.flush(os.open(p, "w"))
    assert(fs.exists(p))
    fs.remove(p)
}
```

## 环境变量

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

读取环境变量。

返回：变量值；**变量不存在时返回空串**（不报错）。因此无法区分“未设置”与“设置为空串”。

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    // PATH 在主流平台必然存在
    path = os.get_env("PATH")
    assert(string.len(path) > 0)

    // 不存在的变量返回空串
    assert(string.is_empty(os.get_env("__YX_DEFINITELY_MISSING__")))
}
```

### set_env

<!-- stdlib:sig:os.set_env start -->

```yaoxiang
set_env: (name: &String, value: &String) -> Void
```

<!-- stdlib:sig:os.set_env end -->

设置环境变量（影响当前进程）。

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    os.set_env("__YX_DOC_ENV", "hello")
    assert(os.get_env("__YX_DOC_ENV") == "hello")
}
```

## 进程与工作目录

### args

<!-- stdlib:sig:os.args start -->

```yaoxiang
args: () -> String
```

<!-- stdlib:sig:os.args end -->

返回命令行参数。

返回：所有 argv 以 **`\n` 连接**的单个字符串（不是 `List`）。首项是程序自身路径。

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    argv = os.args()
    assert(string.len(argv) > 0)
}
```

### chdir

<!-- stdlib:sig:os.chdir start -->

```yaoxiang
chdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.chdir end -->

切换当前工作目录。

返回：成功返回 `true`。错误：目录不存在时抛出 `E6007`。

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    before = os.getcwd()
    assert(os.chdir(".."))
    assert(os.chdir(before))     // 切回
    assert(os.getcwd() == before)
}
```

### getcwd

<!-- stdlib:sig:os.getcwd start -->

```yaoxiang
getcwd: () -> String
```

<!-- stdlib:sig:os.getcwd end -->

返回当前工作目录的绝对路径。

错误：无法获取时抛出 `E6007`。

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    cwd = os.getcwd()
    assert(string.len(cwd) > 0)
}
```

## 相关

- [`std.fs`](./fs) —— 路径级文件与目录操作
- [错误码参考](../error-code/) —— `E6007` 通用运行时错误
