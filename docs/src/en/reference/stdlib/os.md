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

> All functions in this module depend on operating system capabilities, and are **not exported** on
> the `wasm32` target.

## File Handle Model

> **Important limitation (#337)**: The handle returned by `open` is **single-use**. It has no `&`,
> so when it's first passed to `read` / `write` / `seek` / `tell` / `flush` / `close`, it's
> **moved**, and cannot be used again afterwards. Therefore the common pattern `open` → `write` →
> `close` **cannot compile** under the current implementation (it will report `E2014`).
>
> Two viable approaches:
>
> 1. **Inline `open` into a single call** — the handle is consumed immediately after creation:
>
>    ```yaoxiang
>    n = os.write(os.open(p, "w"), "hello")
>    ```
>
> 2. **Use the convenience functions that don't open handles** —
>    [`std.io.read_file`](./io#read_file) / [`write_file`](./io#write_file) /
>    [`append_file`](./io#append_file), or this module's [`append_file`](#append_file).
>
> Handles exist as entries in a handle table and are reclaimed with the process when it exits. Since
> a handle cannot be referenced again after a single use, explicit `close` cannot be written in most
> scenarios (but see the inline single-call form below).

`open` returns an **`Int`-typed file descriptor** (the engine internally maintains a handle table),
so `File` in the signature is actually `Int`.

Content is flushed to disk immediately after writing, with no need for explicit `close`:

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

Modes supported by `open`:

| Mode | Meaning                         |
| ---- | ------------------------------- |
| `r`  | Read-only, file must exist      |
| `w`  | Write-only, create or truncate  |
| `a`  | Append, create or append to end |
| `r+` | Read-write, file must exist     |
| `w+` | Read-write, create or truncate  |
| `a+` | Read-write, create or append    |

## Function Overview

<!-- stdlib:table:os start -->

| Function      | Signature                                   |
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

<!-- stdlib:table:os end -->

## File Operations

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

Opens a file and returns a file descriptor.

- `path` — File path (read-only borrow)
- `mode` — Open mode, see table above

Returns: an `Int` descriptor allocated from the internal handle table. **This handle can only be
used once** — any downstream call will move it (see [File Handle Model](#file-handle-model)), so
`open` is usually inlined into a single call.

Errors: throws `E6007` for invalid mode, non-existent file, or lack of permission.

> Since the handle can only be used once (#337), the return value is usually inlined directly into a
> downstream call.

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

Closes the file handle and frees the table entry.

Since the handle can only be used once, `close` only makes sense in scenarios where the file is
opened and not used further. Written content is already flushed to disk when [`write`](#write)
returns, so explicit close is usually unnecessary.

Errors: throws `E6007` when the descriptor is invalid (not opened or already closed).

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

Reads at most `n` bytes from the current read/write position.

- `file` — File descriptor
- `n` — Number of bytes expected to be read

Returns: the actually read content (may be shorter than `n`, returns an empty string when reaching
the end of file). Invalid UTF-8 bytes are returned as replacement characters, no error reported.
Errors: throws `E6007` when the descriptor is invalid or reading fails.

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

Writes all of `content` at the current read/write position.

- `content` — passed by value

Returns: the **number of bytes written**. Errors: throws `E6007` when the descriptor is invalid or
writing fails.

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

Moves the read/write position to the **absolute** offset `offset` (relative to the beginning of the
file).

- `offset` — Target byte offset, must be non-negative

Returns: `true` on success. Errors: throws `E6007` when the descriptor is invalid or offset is
invalid.

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

Returns the byte offset of the current read/write position.

Errors: throws `E6007` when the descriptor is invalid.

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

Flushes buffered content to disk.

Errors: throws `E6007` when the descriptor is invalid or flush fails.

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

## Directory Operations

### mkdir

<!-- stdlib:sig:os.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.mkdir end -->

Creates a **single-level** directory (does not recursively create parent directories).

Returns: `true` on success. Errors: throws `E6007` when parent directory does not exist or directory
already exists.

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

Removes an **empty** directory.

Returns: `true` on success. Errors: throws `E6007` when directory does not exist or is not empty.

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

Lists the entry names under a directory.

Returns: entry names joined by **`\n`** as a single string (not a `List`). Errors: throws `E6007`
when directory does not exist or lack of permission.

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    d = "__yx_doc_read_dir"
    os.mkdir(d)
    names = os.read_dir(d)
    // An empty directory returns an empty string
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

Removes a file, semantically equivalent to `remove_file` (**cannot remove directories**; use
[`rmdir`](#rmdir) to remove directories).

Returns: `true` on success. Errors: throws `E6007` when file does not exist or path is a directory.

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

Whether the path exists (file or directory). **No error** is reported; returns `false` if it does
not exist.

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

Whether the path is a **regular file**. Returns `false` for directories, `false` for non-existent
paths.

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

Whether the path is a **directory**. Returns `false` for files, `false` for non-existent paths.

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

Copies a file. **Overwrites** if the destination already exists.

Returns: `true` on success. Errors: throws `E6007` when source file does not exist or lack of
permission.

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

Renames or moves a file.

Returns: `true` on success. Errors: throws `E6007` when source file does not exist or destination
already exists.

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

Appends content (convenience function that doesn't open a handle). Creates the file if it does not
exist.

Returns: `true` on success. Errors: throws `E6007` on permission denied.

> This is a same-name counterpart to [`std.io.append_file`](./io#append_file); both modules provide
> it with identical behavior.

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

## Environment Variables

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

Reads an environment variable.

Returns: the variable value; **returns an empty string if the variable does not exist** (no error).
Therefore you cannot distinguish between "unset" and "set to empty string".

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    // PATH is guaranteed to exist on mainstream platforms
    path = os.get_env("PATH")
    assert(string.len(path) > 0)

    // Non-existent variables return an empty string
    assert(string.is_empty(os.get_env("__YX_DEFINITELY_MISSING__")))
}
```

### set_env

<!-- stdlib:sig:os.set_env start -->

```yaoxiang
set_env: (name: &String, value: &String) -> Void
```

<!-- stdlib:sig:os.set_env end -->

Sets an environment variable (affects the current process).

```yaoxiang
use std.assert
use std.os

main = {
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

Returns command-line arguments.

Returns: all argv as a single string joined by **`\n`** (not a `List`). The first item is the
program's own path.

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

Changes the current working directory.

Returns: `true` on success. Errors: throws `E6007` when directory does not exist.

```yaoxiang
use std.assert
use std.os

main = {
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

Returns the absolute path of the current working directory.

Errors: throws `E6007` when it cannot be retrieved.

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    cwd = os.getcwd()
    assert(string.len(cwd) > 0)
}
```

## Related

- [`std.io`](./io) — Whole-file read/write convenience functions
- [Error code reference](../error-code/) — `E6007` general runtime error
