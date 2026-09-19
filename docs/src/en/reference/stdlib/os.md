---
title: 'std.os'
description: 'File handles, directories, environment variables, and working directory'
---

# std.os

Operating system interface module: file handle read/write, directory operations, environment
variables, and working directory.

```yaoxiang
use std.os
```

> All functions in this module depend on operating system capabilities and are **not exported** on
> the `wasm32` target.

## File Handle Model

> **Handles are passed by reference (#337 fixed)**: The signatures of `read` / `write` / `seek` /
> `tell` / `flush` / `close` are all `(file: &File, ...)`, so handles can be used repeatedly:
>
> ```yaoxiang
> f = os.open(p, "w")
> os.write(f, "hello world")
> os.close(f)
> ```
>
> Read/write after positioning (the reason `seek` exists) is also available:
>
> ```yaoxiang
> r = os.open(p, "r")
> os.seek(r, 6)
> tail = os.read(r, 5)     // "world"
> os.close(r)
> ```
>
> Before the fix, signatures had no `&`, so handles were passed by value → linear ownership → became
> invalid after one use, and `open → write → close` would report `E2014`.
>
> If you want to avoid manually managing handles, you can still use the convenience functions that
> don't open handles: [`std.io.read_file`](./io#read_file) / [`write_file`](./io#write_file) /
> [`append_file`](./io#append_file), or this module's [`append_file`](#append_file).

`open` returns an **`Int`-typed file descriptor** (the engine internally maintains a handle table),
so the `File` in signatures is actually `Int`.

Content is flushed to disk immediately after writing — no explicit `close` is needed:

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_open.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    assert(io.read_file(p) == "hello")
    os.remove(p)
}
```

Modes supported by `open`:

| Mode | Meaning                         |
| ---- | ------------------------------- |
| `r`  | Read-only, file must exist      |
| `w`  | Write-only, create or truncate  |
| `a`  | Append, create or append to end |
| `r+` | Read/write, file must exist     |
| `w+` | Read/write, create or truncate  |
| `a+` | Read/write, create or append    |

## Function List

<!-- stdlib:table:os start -->

| Function      | Signature                                   |
| ------------- | ------------------------------------------- |
| `open`        | `(path: &String, mode: &String) -> File`    |
| `close`       | `(file: &File) -> Void`                     |
| `read`        | `(file: &File, n: Int) -> String`           |
| `write`       | `(file: &File, content: String) -> Int`     |
| `seek`        | `(file: &File, offset: Int) -> Bool`        |
| `tell`        | `(file: &File) -> Int`                      |
| `flush`       | `(file: &File) -> Void`                     |
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

<!-- stdlib:table:os end -->## File Operations

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

Open a file and return a file descriptor.

- `path` —— file path (read-only borrow)
- `mode` —— open mode, see the table above

Returns: an `Int` descriptor allocated from the internal handle table. **This handle can only be
used once** — any downstream call will move it (see [File Handle Model](#file-handle-model)), so
`open` is usually inlined into a single call.

Error: throws `E6007` if the mode is invalid, the file does not exist, or there are insufficient
permissions.

> Since the handle can only be used once (#337), the return value is usually inlined directly into a
> downstream call.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    p = "__yx_doc_open_only.txt"
    f = os.open(p, "w")
    assert(os.exists(p))
    os.remove(p)
}
```

### close

<!-- stdlib:sig:os.close start -->

```yaoxiang
close: (file: &File) -> Void
```

<!-- stdlib:sig:os.close end -->

Close the file handle and release the table entry.

Since the handle can only be used once, `close` only makes sense in scenarios where the file is
"opened and not used for anything else"; written content is already flushed to disk when
[`write`](#write) returns, so explicit closing is usually not needed.

Error: throws `E6007` if the descriptor is invalid (not opened or already closed).

```yaoxiang
use std.os

main: () -> Void = {
    p = "__yx_doc_close.txt"
    f = os.open(p, "w")
    os.close(f)
    os.remove(p)
}
```

### read

<!-- stdlib:sig:os.read start -->

```yaoxiang
read: (file: &File, n: Int) -> String
```

<!-- stdlib:sig:os.read end -->

Read **at most** `n` bytes from the current read/write position.

- `file` —— file descriptor
- `n` —— expected number of bytes to read

Returns: the actual content read (may be shorter than `n`, returns an empty string when the end of
file is reached). Invalid UTF-8 bytes are returned as replacement characters, without error. Error:
throws `E6007` if the descriptor is invalid or the read fails.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
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
write: (file: &File, content: String) -> Int
```

<!-- stdlib:sig:os.write end -->

Write all of `content` at the current read/write position.

- `content` —— passed by value

Returns: the **number of bytes** written. Error: throws `E6007` if the descriptor is invalid or the
write fails.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    p = "__yx_doc_write.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    os.remove(p)
}
```

### seek

<!-- stdlib:sig:os.seek start -->

```yaoxiang
seek: (file: &File, offset: Int) -> Bool
```

<!-- stdlib:sig:os.seek end -->

Move the read/write position to **absolute** offset `offset` (relative to the start of the file).

- `offset` —— target byte offset, must be non-negative

Returns: `true` on success. Error: throws `E6007` if the descriptor is invalid or the offset is
invalid.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
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
tell: (file: &File) -> Int
```

<!-- stdlib:sig:os.tell end -->

Return the byte offset of the current read/write position.

Error: throws `E6007` if the descriptor is invalid.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    p = "__yx_doc_tell.txt"
    pos = os.tell(os.open(p, "w"))
    assert(pos == 0)
    os.remove(p)
}
```

### flush

<!-- stdlib:sig:os.flush start -->

```yaoxiang
flush: (file: &File) -> Void
```

<!-- stdlib:sig:os.flush end -->

Flush buffered content to disk.

Error: throws `E6007` if the descriptor is invalid or flushing fails.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    p = "__yx_doc_flush.txt"
    os.flush(os.open(p, "w"))
    assert(os.exists(p))
    os.remove(p)
}
```

## Directory Operations

### mkdir

<!-- stdlib:sig:os.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.mkdir end -->

Create a **single-level** directory (does not recursively create parent directories).

Returns: `true` on success. Error: throws `E6007` if the parent directory does not exist or the
directory already exists.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
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

Delete an **empty** directory.

Returns: `true` on success. Error: throws `E6007` if the directory does not exist or is not empty.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
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

List entry names in the directory.

Returns: a single string with entry names **joined by `\n`** (not a `List`). Error: throws `E6007`
if the directory does not exist or there are insufficient permissions.

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    d = "__yx_doc_read_dir"
    os.mkdir(d)
    names = os.read_dir(d)
    // Empty directory returns empty string
    assert(string.is_empty(names))
    os.rmdir(d)
}
```

## Path and File Utilities

### remove

<!-- stdlib:sig:os.remove start -->

```yaoxiang
remove: (path: &String) -> Bool
```

<!-- stdlib:sig:os.remove end -->

Delete a file; semantically equivalent to `remove_file` (**cannot delete directories**; use
[`rmdir`](#rmdir) for directories).

Returns: `true` on success. Error: throws `E6007` if the file does not exist or the path is a
directory.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
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

Whether the path exists (file or directory). **Does not throw**; returns `false` if it does not
exist.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
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

Whether the path is a **regular file**. Returns `false` for directories and `false` for non-existent
paths.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    assert(!os.is_file("."))
}
```

### is_dir

<!-- stdlib:sig:os.is_dir start -->

```yaoxiang
is_dir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.is_dir end -->

Whether the path is a **directory**. Returns `false` for files and `false` for non-existent paths.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    assert(os.is_dir("."))
}
```

### copy

<!-- stdlib:sig:os.copy start -->

```yaoxiang
copy: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:os.copy end -->

Copy a file. **Overwrites** if the destination already exists.

Returns: `true` on success. Error: throws `E6007` if the source file does not exist or there are
insufficient permissions.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
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

Rename or move a file.

Returns: `true` on success. Error: throws `E6007` if the source file does not exist or the
destination already exists.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
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

Append write (convenience function that does not open a handle). Creates the file if it does not
exist.

Returns: `true` on success. Error: throws `E6007` if there are insufficient permissions.

> This is a same-name, same-kind interface to [`std.io.append_file`](./io#append_file); both modules
> provide it with consistent behavior.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_os_append.txt"
    io.write_file(p, "a")
    os.append_file(p, "b")
    assert(io.read_file(p) == "ab")
    os.remove(p)
}
```

## Environment Variables

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

Read an environment variable.

Returns: the variable's value; **returns an empty string if the variable does not exist** (does not
throw). Therefore, it cannot distinguish "not set" from "set to empty string".

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    // PATH is guaranteed to exist on mainstream platforms
    path = os.get_env("PATH")
    assert(string.len(path) > 0)

    // Non-existent variable returns empty string
    assert(string.is_empty(os.get_env("__YX_DEFINITELY_MISSING__")))
}
```

### set_env

<!-- stdlib:sig:os.set_env start -->

```yaoxiang
set_env: (name: &String, value: &String) -> Void
```

<!-- stdlib:sig:os.set_env end -->

Set an environment variable (affects the current process).

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    os.set_env("__YX_DOC_ENV", "hello")
    assert(os.get_env("__YX_DOC_ENV") == "hello")
}
```

## Process and Working Directory

### args

<!-- stdlib:sig:os.args start -->

```yaoxiang
args: () -> String
```

<!-- stdlib:sig:os.args end -->

Return command-line arguments.

Returns: a single string with all argv values **joined by `\n`** (not a `List`). The first item is
the path of the program itself.

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

Switch the current working directory.

Returns: `true` on success. Error: throws `E6007` if the directory does not exist.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    before = os.getcwd()
    assert(os.chdir(".."))
    assert(os.chdir(before))     // switch back
    assert(os.getcwd() == before)
}
```

### getcwd

<!-- stdlib:sig:os.getcwd start -->

```yaoxiang
getcwd: () -> String
```

<!-- stdlib:sig:os.getcwd end -->

Return the absolute path of the current working directory.

Error: throws `E6007` if it cannot be obtained.

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    cwd = os.getcwd()
    assert(string.len(cwd) > 0)
}
```

## Related

- [`std.io`](./io) —— whole-file read/write convenience functions
- [Error Code Reference](../error-code/) —— `E6007` general runtime error
