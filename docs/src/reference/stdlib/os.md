---
title: 'std.os'
description: '文件句柄、目录、环境变量与工作目录'
---

# std.os

操作系统接口模块：文件句柄读写、目录操作、环境变量与工作目录。

```yaoxiang
use std.os
```

> 本模块全部函数依赖操作系统能力，在 `wasm32` 目标上**不导出**。

## 文件句柄模型

> **重要限制（#337）**：`open` 返回的句柄是**一次性的**。它没有 `&`，所以第一次传给 `read` / `write`
> / `seek` / `tell` / `flush` / `close` 时就被**移动**，之后不可再用。因此 `open` → `write` →
> `close` 这种常见写法在目前实现下 **无法编译**（会报 `E2014`）。
>
> 可行的两种写法：
>
> 1. **把 `open` 内联进单次调用**——句柄产生后立即被消费：
>
>    ```yaoxiang
>    n = os.write(os.open(p, "w"), "hello")
>    ```
>
> 2. **用不开句柄的便捷函数**——[`std.io.read_file`](./io#read_file) /
>    [`write_file`](./io#write_file) / [`append_file`](./io#append_file)，或本模块的
>    [`append_file`](#append_file)。
>
> 句柄以句柄表条目形式存在，进程退出时随进程回收；因为一次使用后即不可再引用，显式 `close`
> 在多数场景无法写上（但见下文单次调用形态）。

`open` 返回的是一个 **`Int` 类型的文件描述符**（引擎内部维护句柄表），因此签名中的 `File` 实为
`Int`。

写入后内容立即落盘，无需显式 `close`：

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_open.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    assert(io.read_file(p) == "hello")
    os.remove(p)
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

| 函数          | 签名                                        |
| ------------- | ------------------------------------------- |
| `open`        | `(path: &String, mode: &String) -> File`    |
| `close`       | `(file: File) -> Void`                      |
| `read`        | `(file: File, n: Int) -> String`            |
| `write`       | `(file: File, content: String) -> Int`      |
| `seek`        | `(file: File, offset: Int) -> Bool`         |
| `tell`        | `(file: File) -> Int`                       |
| `flush`       | `(file: File) -> Void`                      |
| `mkdir`       | `(path: &String) -> Bool`                   |
| `rmdir`       | `(path: &String) -> Bool`                   |
| `read_dir`    | `(path: &String) -> String`                 |
| `remove`      | `(path: &String) -> Bool`                   |
| `exists`      | `(path: &String) -> Bool`                   |
| `is_file`     | `(path: &String) -> Bool`                   |
| `is_dir`      | `(path: &String) -> Bool`                   |
| `copy`        | `(src: &String, dst: &String) -> Bool`      |
| `rename`      | `(old: &String, new: &String) -> Bool`      |
| `get_env`     | `(name: &String) -> String`                 |
| `set_env`     | `(name: &String, value: &String) -> Void`   |
| `args`        | `() -> String`                              |
| `chdir`       | `(path: &String) -> Bool`                   |
| `getcwd`      | `() -> String`                              |
| `append_file` | `(path: &String, content: &String) -> Bool` |

<!-- stdlib:table:os end -->## 文件操作

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

打开文件并返回文件描述符。

- `path` —— 文件路径（只读借用）
- `mode` —— 打开模式，见上表

返回：内部句柄表分配的 `Int`
描述符。**该句柄只能使用一次**——任一下游调用都会移动它（见[文件句柄模型](#文件句柄模型)），因此通常把
`open` 内联进单次调用。

错误：模式非法、文件不存在或无权限时抛出 `E6007`。

> 句柄只能使用一次（#337），因此该返回值通常直接内联进下游调用。

```yaoxiang
use std.assert
use std.os

main = {
    p = "__yx_doc_open_only.txt"
    f = os.open(p, "w")
    assert(os.exists(p))
    os.remove(p)
}
```

### close

<!-- stdlib:sig:os.close start -->

```yaoxiang
close: (file: File) -> Void
```

<!-- stdlib:sig:os.close end -->

关闭文件句柄并释放表项。

因为句柄只能使用一次，`close` 只在“打开后不作他用”的场景有意义；写入内容在 [`write`](#write)
返回时已落盘，通常无需显式关闭。

错误：描述符无效（未打开或已关闭）时抛出 `E6007`。

```yaoxiang
use std.os

main = {
    p = "__yx_doc_close.txt"
    f = os.open(p, "w")
    os.close(f)
    os.remove(p)
}
```

### read

<!-- stdlib:sig:os.read start -->

```yaoxiang
read: (file: File, n: Int) -> String
```

<!-- stdlib:sig:os.read end -->

从当前读写位置读取**最多** `n` 字节。

- `file` —— 文件描述符
- `n` —— 期望读取的字节数

返回：实际读到的内容（可能短于
`n`，到达文件末尾时为空串）。非法 UTF-8 字节以替换字符形式返回，不报错。错误：描述符无效或读取失败时抛出
`E6007`。

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_read.txt"
    io.write_file(p, "abcdef")

    part = os.read(os.open(p, "r"), 3)
    assert(part == "abc")
    os.remove(p)
}
```

### write

<!-- stdlib:sig:os.write start -->

```yaoxiang
write: (file: File, content: String) -> Int
```

<!-- stdlib:sig:os.write end -->

向当前读写位置写入全部 `content`。

- `content` —— 按值传入

返回：写入的**字节数**。错误：描述符无效或写入失败时抛出 `E6007`。

```yaoxiang
use std.assert
use std.os

main = {
    p = "__yx_doc_write.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    os.remove(p)
}
```

### seek

<!-- stdlib:sig:os.seek start -->

```yaoxiang
seek: (file: File, offset: Int) -> Bool
```

<!-- stdlib:sig:os.seek end -->

把读写位置移动到**绝对**偏移 `offset`（相对文件开头）。

- `offset` —— 目标字节偏移，须非负

返回：成功返回 `true`。错误：描述符无效或偏移非法时抛出 `E6007`。

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_seek.txt"
    io.write_file(p, "abcdef")

    ok = os.seek(os.open(p, "r"), 2)
    assert(ok)
    os.remove(p)
}
```

### tell

<!-- stdlib:sig:os.tell start -->

```yaoxiang
tell: (file: File) -> Int
```

<!-- stdlib:sig:os.tell end -->

返回当前读写位置的字节偏移。

错误：描述符无效时抛出 `E6007`。

```yaoxiang
use std.assert
use std.os

main = {
    p = "__yx_doc_tell.txt"
    pos = os.tell(os.open(p, "w"))
    assert(pos == 0)
    os.remove(p)
}
```

### flush

<!-- stdlib:sig:os.flush start -->

```yaoxiang
flush: (file: File) -> Void
```

<!-- stdlib:sig:os.flush end -->

把缓冲内容刷入磁盘。

错误：描述符无效或刷新失败时抛出 `E6007`。

```yaoxiang
use std.assert
use std.os

main = {
    p = "__yx_doc_flush.txt"
    os.flush(os.open(p, "w"))
    assert(os.exists(p))
    os.remove(p)
}
```

## 目录操作

### mkdir

<!-- stdlib:sig:os.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.mkdir end -->

创建**单层**目录（不递归创建父目录）。

返回：成功返回 `true`。错误：父目录不存在或目录已存在时抛出 `E6007`。

```yaoxiang
use std.assert
use std.os

main = {
    d = "__yx_doc_mkdir"
    assert(os.mkdir(d))
    assert(os.is_dir(d))
    os.rmdir(d)
}
```

### rmdir

<!-- stdlib:sig:os.rmdir start -->

```yaoxiang
rmdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.rmdir end -->

删除**空**目录。

返回：成功返回 `true`。错误：目录不存在或非空时抛出 `E6007`。

```yaoxiang
use std.assert
use std.os

main = {
    d = "__yx_doc_rmdir"
    os.mkdir(d)
    assert(os.rmdir(d))
    assert(!os.exists(d))
}
```

### read_dir

<!-- stdlib:sig:os.read_dir start -->

```yaoxiang
read_dir: (path: &String) -> String
```

<!-- stdlib:sig:os.read_dir end -->

列出目录下的条目名。

返回：入口名称以 **`\n` 连接**的单个字符串（不是 `List`）。错误：目录不存在或无权限时抛出 `E6007`。

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    d = "__yx_doc_read_dir"
    os.mkdir(d)
    names = os.read_dir(d)
    // 空目录返回空串
    assert(string.is_empty(names))
    os.rmdir(d)
}
```

## 路径与文件工具

### remove

<!-- stdlib:sig:os.remove start -->

```yaoxiang
remove: (path: &String) -> Bool
```

<!-- stdlib:sig:os.remove end -->

删除文件，语义等同 `remove_file`（**不能删目录**，删目录用 [`rmdir`](#rmdir)）。

返回：成功返回 `true`。错误：文件不存在或路径是目录时抛出 `E6007`。

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_remove.txt"
    io.write_file(p, "x")
    assert(os.remove(p))
    assert(!os.exists(p))
}
```

### exists

<!-- stdlib:sig:os.exists start -->

```yaoxiang
exists: (path: &String) -> Bool
```

<!-- stdlib:sig:os.exists end -->

路径是否存在（文件或目录皆可）。**不报错**，不存在返回 `false`。

```yaoxiang
use std.assert
use std.os

main = {
    assert(os.exists("."))
    assert(!os.exists("__yx_definitely_missing_path__"))
}
```

### is_file

<!-- stdlib:sig:os.is_file start -->

```yaoxiang
is_file: (path: &String) -> Bool
```

<!-- stdlib:sig:os.is_file end -->

路径是否为**普通文件**。目录返回 `false`，不存在返回 `false`。

```yaoxiang
use std.assert
use std.os

main = {
    assert(!os.is_file("."))
}
```

### is_dir

<!-- stdlib:sig:os.is_dir start -->

```yaoxiang
is_dir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.is_dir end -->

路径是否为**目录**。文件返回 `false`，不存在返回 `false`。

```yaoxiang
use std.assert
use std.os

main = {
    assert(os.is_dir("."))
}
```

### copy

<!-- stdlib:sig:os.copy start -->

```yaoxiang
copy: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:os.copy end -->

复制文件。目标已存在时**覆盖**。

返回：成功返回 `true`。错误：源文件不存在或无权限时抛出 `E6007`。

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    a = "__yx_doc_copy_a.txt"
    b = "__yx_doc_copy_b.txt"
    io.write_file(a, "data")
    assert(os.copy(a, b))
    assert(io.read_file(b) == "data")
    os.remove(a)
    os.remove(b)
}
```

### rename

<!-- stdlib:sig:os.rename start -->

```yaoxiang
rename: (old: &String, new: &String) -> Bool
```

<!-- stdlib:sig:os.rename end -->

重命名或移动文件。

返回：成功返回 `true`。错误：源文件不存在或目标已存在时抛出 `E6007`。

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    a = "__yx_doc_rename_a.txt"
    b = "__yx_doc_rename_b.txt"
    io.write_file(a, "data")
    assert(os.rename(a, b))
    assert(os.exists(b))
    os.remove(b)
}
```

### append_file

<!-- stdlib:sig:os.append_file start -->

```yaoxiang
append_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:os.append_file end -->

追加写入（不开句柄的便捷函数）。文件不存在时创建。

返回：成功返回 `true`。错误：无权限时抛出 `E6007`。

> 这是 [`std.io.append_file`](./io#append_file) 的同名同类接口，两个模块都提供，行为一致。

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_os_append.txt"
    io.write_file(p, "a")
    os.append_file(p, "b")
    assert(io.read_file(p) == "ab")
    os.remove(p)
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

main = {
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

main = {
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

main = {
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

main = {
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

main = {
    cwd = os.getcwd()
    assert(string.len(cwd) > 0)
}
```

## 相关

- [`std.io`](./io) —— 整文件读写便捷函数
- [错误码参考](../error-code/) —— `E6007` 通用运行时错误
