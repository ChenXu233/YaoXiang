---
title: 'std.fs'
description: '文件、目录与路径操作'
---

# std.fs

路径级文件系统操作：整文件读写、目录管理与遍历、文件元数据、临时文件与路径运算。

需要按句柄增量读写（open/read/seek）时用 [`std.os`](./os)；控制台输入输出用
[`std.io`](./io)。

```yaoxiang
use std.fs
```

## 平台可用性

本模块在 `wasm32` 目标上**不导出**（无文件系统语义）。`mkdtemp` / `tmpfile` /
`temp_dir` 依赖安全临时文件创建，仅在原生目标可用。

## 函数一览

<!-- stdlib:table:fs start -->

| 函数 | 签名 |
| ---- | ---- |
| `read_file` | `(path: &String) -> String` |
| `write_file` | `(path: &String, content: &String) -> Bool` |
| `append_file` | `(path: &String, content: &String) -> Bool` |
| `exists` | `(path: &String) -> Bool` |
| `is_file` | `(path: &String) -> Bool` |
| `is_dir` | `(path: &String) -> Bool` |
| `mkdir` | `(path: &String) -> Bool` |
| `mkdir_all` | `(path: &String) -> Bool` |
| `rmdir` | `(path: &String) -> Bool` |
| `remove` | `(path: &String) -> Bool` |
| `copy` | `(src: &String, dst: &String) -> Bool` |
| `rename` | `(src: &String, dst: &String) -> Bool` |
| `read_dir` | `(path: &String) -> Vec(String)` |
| `walk` | `(path: &String) -> Vec(String)` |
| `stat` | `(path: &String) -> Dict(String, Any)` |
| `temp_dir` | `() -> String` |
| `mkdtemp` | `(prefix: &String) -> String` |
| `tmpfile` | `(prefix: &String) -> String` |
| `path_join` | `(base: &String, rel: &String) -> String` |
| `path_basename` | `(path: &String) -> String` |
| `path_dirname` | `(path: &String) -> String` |
| `path_extension` | `(path: &String) -> String` |

<!-- stdlib:table:fs end -->## 函数

### read_file

<!-- stdlib:sig:fs.read_file start -->

```yaoxiang
read_file: (path: &String) -> String
```

<!-- stdlib:sig:fs.read_file end -->

一次性读取整个文件内容为字符串。

- `path` —— 文件路径（只读借用）

返回：文件全部内容。错误：文件不存在或无权限时抛出 `E6007`。**不返回空串**。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_read_file.txt"
    fs.write_file(p, "hello")

    content = fs.read_file(p)
    assert(content == "hello")

    fs.remove(p)
}
```

### write_file

<!-- stdlib:sig:fs.write_file start -->

```yaoxiang
write_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:fs.write_file end -->

把 `content` 写入 `path`，**覆盖**原有内容；文件不存在时创建。

- `path` —— 文件路径（只读借用）
- `content` —— 要写入的内容（只读借用）

返回：写入成功返回 `true`。错误：目录不存在或无权限时抛出 `E6007`（不返回 `false`）。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_write_file.txt"
    ok = fs.write_file(p, "hello")
    assert(ok)
    assert(fs.exists(p))
    fs.remove(p)
}
```

### append_file

<!-- stdlib:sig:fs.append_file start -->

```yaoxiang
append_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:fs.append_file end -->

把 `content` **追加**到 `path` 末尾；文件不存在时创建。

返回：写入成功返回 `true`。错误：无权限时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_append_file.txt"
    fs.write_file(p, "hello")
    fs.append_file(p, " world")
    assert(fs.read_file(p) == "hello world")
    fs.remove(p)
}
```

### exists

<!-- stdlib:sig:fs.exists start -->

```yaoxiang
exists: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.exists end -->

路径是否存在（文件或目录均可）。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_exists.txt"
    assert(!fs.exists(p), "absent before write")
    fs.write_file(p, "x")
    assert(fs.exists(p), "present after write")
    fs.remove(p)
}
```

### is_file

<!-- stdlib:sig:fs.is_file start -->

```yaoxiang
is_file: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.is_file end -->

路径是否指向常规文件。路径不存在时返回 `false`（不报错）。

### is_dir

<!-- stdlib:sig:fs.is_dir start -->

```yaoxiang
is_dir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.is_dir end -->

路径是否指向目录。路径不存在时返回 `false`（不报错）。

### mkdir

<!-- stdlib:sig:fs.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.mkdir end -->

创建**单层**目录，父目录必须已存在。需要递归创建时用
[`mkdir_all`](#mkdir_all)。

返回：成功返回 `true`。错误：父目录缺失或目录已存在时抛出 `E6007`。

### mkdir_all

<!-- stdlib:sig:fs.mkdir_all start -->

```yaoxiang
mkdir_all: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.mkdir_all end -->

递归创建目录，缺失的父目录一并创建；目录已存在时视为成功。

返回：成功返回 `true`。错误：无权限时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    d = "__yx_doc_fs_mkdir_all/a/b"
    ok = fs.mkdir_all(d)
    assert(ok)
    assert(fs.is_dir(d))
    fs.rmdir("__yx_doc_fs_mkdir_all/a/b")
    fs.rmdir("__yx_doc_fs_mkdir_all/a")
    fs.rmdir("__yx_doc_fs_mkdir_all")
}
```

### rmdir

<!-- stdlib:sig:fs.rmdir start -->

```yaoxiang
rmdir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.rmdir end -->

删除**空**目录。非空目录删除失败并抛出 `E6007`。

### remove

<!-- stdlib:sig:fs.remove start -->

```yaoxiang
remove: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.remove end -->

删除文件。删除目录用 [`rmdir`](#rmdir)（仅限空目录）。

返回：成功返回 `true`。错误：文件不存在时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_remove.txt"
    fs.write_file(p, "x")
    assert(fs.remove(p), "remove ok")
    assert(!fs.exists(p), "gone after remove")
}
```

### copy

<!-- stdlib:sig:fs.copy start -->

```yaoxiang
copy: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:fs.copy end -->

复制文件内容与权限位到目标路径；目标已存在时被覆盖（与 Rust `fs::copy` 一致，
不复制元数据时间戳）。

返回：成功返回 `true`。错误：源不存在时抛出 `E6007`。

### rename

<!-- stdlib:sig:fs.rename start -->

```yaoxiang
rename: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:fs.rename end -->

重命名/移动文件或目录。跨设备移动时抛出 `E6007`（先 copy 后 remove 替代）。

返回：成功返回 `true`。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    a = "__yx_doc_fs_rename_a.txt"
    b = "__yx_doc_fs_rename_b.txt"
    fs.write_file(a, "data")
    assert(fs.rename(a, b), "rename ok")
    assert(fs.exists(b), "target exists")
    assert(!fs.exists(a), "source gone")
    fs.remove(b)
}
```

### read_dir

<!-- stdlib:sig:fs.read_dir start -->

```yaoxiang
read_dir: (path: &String) -> Vec(String)
```

<!-- stdlib:sig:fs.read_dir end -->

列出目录下的条目**名称**（不含路径前缀），返回 `List(String)`，按名称排序。

> 与原 `std.os.read_dir`（返回 `"\n"` 连接的字符串）不同，本函数返回类型正确的
> List——这是 `std.os` 文件操作迁入 `std.fs` 时的语义升级。

```yaoxiang
use std.assert
use std.fs
use std.list

main: () -> Void = {
    d = "__yx_doc_fs_readdir"
    fs.mkdir_all(d)
    fs.write_file(d + "/b.txt", "b")
    fs.write_file(d + "/a.txt", "a")

    names = fs.read_dir(d)
    assert(list.len(names) == 2, "two entries")
    assert(list.get(names, 0) == "a.txt", "sorted first")
    assert(list.get(names, 1) == "b.txt", "sorted second")

    fs.remove(d + "/a.txt")
    fs.remove(d + "/b.txt")
    fs.rmdir(d)
}
```

### walk

<!-- stdlib:sig:fs.walk start -->

```yaoxiang
walk: (path: &String) -> Vec(String)
```

<!-- stdlib:sig:fs.walk end -->

递归遍历目录树，返回**全部条目的完整路径** `List(String)`；每层按名称排序，
目录先于其内容出现（深度优先）。符号链接本身作为条目列出但不深入。

错误：`path` 不是目录时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs
use std.list

main: () -> Void = {
    d = "__yx_doc_fs_walk"
    fs.mkdir_all(d + "/sub")
    fs.write_file(d + "/a.txt", "a")
    fs.write_file(d + "/sub/b.txt", "b")

    paths = fs.walk(d)
    // a.txt、sub 目录、sub/b.txt 三个条目
    assert(list.len(paths) == 3, "2 files + 1 dir")

    fs.remove(d + "/a.txt")
    fs.remove(d + "/sub/b.txt")
    fs.rmdir(d + "/sub")
    fs.rmdir(d)
}
```

### stat

<!-- stdlib:sig:fs.stat start -->

```yaoxiang
stat: (path: &String) -> Dict(String, Any)
```

<!-- stdlib:sig:fs.stat end -->

读取文件/目录元数据，返回字典：

| 键 | 类型 | 含义 |
| --- | --- | --- |
| `size` | `Int` | 字节数 |
| `is_dir` | `Bool` | 是否目录 |
| `is_file` | `Bool` | 是否常规文件 |
| `readonly` | `Bool` | 是否只读权限 |
| `mtime` | `Int` | 修改时间（Unix 秒） |

错误：路径不存在时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_stat.txt"
    fs.write_file(p, "12345")

    st = fs.stat(p)
    assert(st["size"] == 5, "byte size")
    assert(st["is_file"], "is a file")

    fs.remove(p)
}
```

### temp_dir

<!-- stdlib:sig:fs.temp_dir start -->

```yaoxiang
temp_dir: () -> String
```

<!-- stdlib:sig:fs.temp_dir end -->

系统临时目录路径（Windows 为 `%TEMP%`，Unix 为 `$TMPDIR` 或 `/tmp`）。

### mkdtemp

<!-- stdlib:sig:fs.mkdtemp start -->

```yaoxiang
mkdtemp: (prefix: &String) -> String
```

<!-- stdlib:sig:fs.mkdtemp end -->

在系统临时目录内创建**唯一**临时目录（名字以 `prefix` 开头），返回其完整路径。

> 目录**不会**被自动回收：脚本用完自行 [`rmdir`](#rmdir)。这个语义是刻意的——
> 显式生命周期比隐式 drop 钩子可预测。

返回：新目录路径。错误：创建失败时抛出 `E6007`。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    d = fs.mkdtemp("yx_doc_")
    assert(fs.is_dir(d), "temp dir created")
    fs.rmdir(d)
}
```

### tmpfile

<!-- stdlib:sig:fs.tmpfile start -->

```yaoxiang
tmpfile: (prefix: &String) -> String
```

<!-- stdlib:sig:fs.tmpfile end -->

在系统临时目录内创建**唯一**空临时文件（名字以 `prefix` 开头），返回其完整路径。
同样**不会**自动回收，用完自行 [`remove`](#remove)。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = fs.tmpfile("yx_doc_")
    assert(fs.is_file(p), "temp file created")
    fs.write_file(p, "scratch")
    assert(fs.read_file(p) == "scratch")
    fs.remove(p)
}
```

### path_join

<!-- stdlib:sig:fs.path_join start -->

```yaoxiang
path_join: (base: &String, rel: &String) -> String
```

<!-- stdlib:sig:fs.path_join end -->

拼接路径分量。`rel` 为绝对路径时**直接替换** `base`（与 Rust/Python 一致）。
分隔符按平台（Windows 为 `\`）。

```yaoxiang
use std.assert
use std.fs
use std.string

main: () -> Void = {
    joined = fs.path_join("dir", "file.txt")
    assert(string.ends_with(joined, "file.txt"), "suffix kept")
    assert(string.starts_with(joined, "dir"), "base kept")
}
```

### path_basename

<!-- stdlib:sig:fs.path_basename start -->

```yaoxiang
path_basename: (path: &String) -> String
```

<!-- stdlib:sig:fs.path_basename end -->

路径的最终分量；无最终分量（如 `/`、`..`）返回空串。

### path_dirname

<!-- stdlib:sig:fs.path_dirname start -->

```yaoxiang
path_dirname: (path: &String) -> String
```

<!-- stdlib:sig:fs.path_dirname end -->

路径的目录部分；无父目录（如 `a.txt`）返回空串。

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    assert(fs.path_basename("dir/file.txt") == "file.txt")
    assert(fs.path_dirname("dir/file.txt") == "dir")
    assert(fs.path_extension("a.tar.gz") == "gz")
    assert(fs.path_extension("noext") == "")
}
```

### path_extension

<!-- stdlib:sig:fs.path_extension start -->

```yaoxiang
path_extension: (path: &String) -> String
```

<!-- stdlib:sig:fs.path_extension end -->

扩展名（不含点，取最后一个）；无扩展名返回空串。

## 相关

- [`std.os`](./os) —— 文件句柄级增量读写与环境变量
- [`std.io`](./io) —— 控制台输入输出
- [错误码参考](../error-code/) —— `E6007` 通用运行时错误
