---
title: 'std.os'
description: 'File handles, environment variables, and working directory'
---

# std.os

Operating system interface module: file handle read/write, environment variables, and working
directory. Path-level file operations (whole-file read/write, directories, metadata, path
arithmetic) are documented in [`std.fs`](fs).

```yaoxiang
use std.os
```

> All functions in this module depend on operating system capabilities and are **not exported** on
> the `wasm32` target.

## File Handle Model

> **Handles are passed by reference (issue #337 fixed)**: The signatures of `read` / `write` /
> `seek` / `tell` / `flush` / `close` are all `(file: &File, ...)`, so handles can be reused:
>
> ```yaoxiang
> f = os.open(p, "w")
> os.write(f, "hello world")
> os.close(f)
> ```
>
> Positioned read/write (the reason `seek` exists) is also available:
>
> ```yaoxiang
> r = os.open(p, "r")
> os.seek(r, 6)
> tail = os.read(r, 5)     // "world"
> os.close(r)
> ```
>
> Before the fix, the signatures had no `&`, so handles were passed by value → linear ownership →
> invalid after one use; `open → write → close` would report `E2014`.

If you want to avoid manually managing handles, you can use the handle-free convenience functions —
they live in [`std.fs`](fs) (**not** in `std.io`): `fs.read_file` / `fs.write_file` /
`fs.append_file`. This module (`std.os`) only provides handle-level incremental read/write, and does
not have `append_file` (`os.append_file` reports `E1042`).

The return type annotation of `open` is `File` — the engine internally maintains a handle table
(`src/std/os.rs:98`), backed by `std::fs::File`, and is an **opaque handle value** to the user.

Content is flushed to disk immediately after writing, no explicit `close` is needed:

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

Modes supported by `open`:

| Mode | Meaning                         |
| ---- | ------------------------------- |
| `r`  | Read-only, file must exist      |
| `w`  | Write-only, create or truncate  |
| `a`  | Append, create or append to end |
| `r+` | Read/write, file must exist     |
| `w+` | Read/write, create or truncate  |
| `a+` | Read/write, create or append    |

## Function Overview

<!-- stdlib:table:os start -->

| Function  | Signature                                 |
| --------- | ----------------------------------------- |
| `open`    | `(path: &String, mode: &String) -> File`  |
| `close`   | `(file: &File) -> Void`                   |
| `read`    | `(file: &File, n: Int) -> String`         |
| `write`   | `(file: &File, content: String) -> Int`   |
| `seek`    | `(file: &File, offset: Int) -> Bool`      |
| `tell`    | `(file: &File) -> Int`                    |
| `flush`   | `(file: &File) -> Void`                   |
| `get_env` | `(name: &String) -> String`               |
| `set_env` | `(name: &String, value: &String) -> Void` |
| `args`    | `() -> String`                            |
| `chdir`   | `(path: &String) -> Bool`                 |
| `getcwd`  | `() -> String`                            |

<!-- stdlib:table:os end -->

## File Operations

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

Opens a file and returns a handle.

- `path` — file path (read-only borrow)
- `mode` — open mode, see the table above

Returns: a `File` handle value. **Handles are passed by reference** — the parameters of `write` /
`seek` / `read` / `tell` / `flush` / `close` are all `&File` (`src/std/os.rs:35-65`), so the same
handle can be reused until explicit [`close`](#close). It is common to inline `open` into a single
call to avoid cleanup.

Errors: Throws `E6007` when the mode is invalid, the file does not exist, or there are insufficient
permissions.

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

Closes the file handle and releases the table entry.

The parameter is `&File`, so `close` can be placed at the end of a sequence of reads and writes,
releasing the handle after it is no longer needed. Written content is flushed to disk when
[`write`](#write) returns (`os.open(…, "w")` goes through `OpenOptions::create(true)`), so explicit
closing is usually unnecessary — but should be done when holding many handles for a long time.

Errors: Throws `E6007` when the handle is invalid (not opened or already closed).

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

Reads **at most** `n` bytes from the current read/write position.

- `file` — file handle
- `n` — expected number of bytes to read

Returns: the actual content read (may be shorter than `n`; returns an empty string at end-of-file).
Invalid UTF-8 bytes are returned as replacement characters without an error. Errors: Throws `E6007`
when the handle is invalid or the read fails.

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

Writes all of `content` to the current read/write position.

- `content` — passed by value

Returns: the number of bytes written. Errors: Throws `E6007` when the handle is invalid or the write
fails.

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

Moves the read/write position to the **absolute** offset `offset` (relative to the start of the
file).

- `offset` — target byte offset, must be non-negative

Returns: `true` on success. Errors: Throws `E6007` when the handle is invalid or the offset is
illegal.

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

Returns the byte offset of the current read/write position.

Errors: Throws `E6007` when the handle is invalid.

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

Flushes buffered content to disk.

Errors: Throws `E6007` when the handle is invalid or the flush fails.

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

## Environment Variables

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

Reads an environment variable.

Returns: the variable value; **returns an empty string when the variable does not exist** (no
error). Therefore you cannot distinguish between "not set" and "set to an empty string".

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
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

Returns command-line arguments.

Returns: a single string of all argv joined by **`\n`** (not a `List`). The first item is the
program path itself.

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

Changes the current working directory.

Returns: `true` on success. Errors: Throws `E6007` when the directory does not exist.

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

Returns the absolute path of the current working directory.

Errors: Throws `E6007` when it cannot be obtained.

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

- [`std.fs`](fs) — path-level file and directory operations
- [Error Code Reference](../error-code/) — `E6007` generic runtime error
