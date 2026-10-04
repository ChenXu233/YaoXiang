---
title: 'std.os'
description: 'File handles, environment variables, and working directory'
---

# std.os

Operating system interface module: file handle read/write, environment variables, and working
directory. Path-level file operations (whole-file read/write, directory, metadata, path
manipulation) are documented in [`std.fs`](./fs).

```yaoxiang
use std.os
```

> All functions in this module rely on operating system capabilities and are **not exported** on the
> `wasm32` target.

## File Handle Model

> **File handles are passed by reference (#337 fixed)**: The signatures of `read` / `write` / `seek`
> / `tell` / `flush` / `close` are all `(file: &File, ...)`, so handles can be reused:
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
> Before the fix, the signatures had no `&`, handles were passed by value → linear ownership →
> became invalid after one use. `open → write → close` would report `E2014`.

If you want to avoid manually managing handles, you can use the no-handle convenience functions —
they live in [`std.fs`](./fs) (**not** in `std.io`): `fs.read_file` / `fs.write_file` /
`fs.append_file`. This module (`std.os`) only has handle-level incremental read/write, and has no
`append_file` (`os.append_file` reports `E1042`).

The return type annotation of `open` is `File` — the engine internally maintains a handle table
(`src/std/os.rs:98`), the underlying type is `std::fs::File`, and it is an **opaque handle value**
to the user.

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

Supported modes for `open`:

| Mode | Meaning                         |
| ---- | ------------------------------- |
| `r`  | Read-only, file must exist      |
| `w`  | Write-only, create or truncate  |
| `a`  | Append, create or append to end |
| `r+` | Read-write, file must exist     |
| `w+` | Read-write, create or truncate  |
| `a+` | Read-write, create or append    |

## Function List

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

Open a file and return a handle.

- `path` — file path (read-only borrow)
- `mode` — open mode, see the table above

Returns: a `File` handle value. **Handles are passed by reference** — the parameters of `write` /
`seek` / `read` / `tell` / `flush` / `close` are all `&File` (`src/std/os.rs:35-65`), so the same
handle can be reused until explicitly [`close`](#close)d. Usually `open` is inlined into a single
call to avoid cleanup.

Errors: throws `E6007` when the mode is invalid, the file does not exist, or permission is denied.

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

Close the file handle and release its table entry.

The parameter is `&File`, so `close` can be placed at the end of a sequence of read/write
operations, releasing the handle after it has been used up. Written content is flushed to disk when
[`write`](#write) returns (`os.open(…, "w")` goes through `OpenOptions::create(true)`), so explicit
close is usually unnecessary — but cleanup should be done when holding many handles for a long time.

Errors: throws `E6007` when the handle is invalid (not opened or already closed).

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

Read **at most** `n` bytes from the current read/write position.

- `file` — file handle
- `n` — expected number of bytes to read

Returns: the actual content read (may be shorter than `n`; returns an empty string when reaching end
of file). Invalid UTF-8 bytes are returned as replacement characters, no error is raised.

Errors: throws `E6007` when the handle is invalid or the read fails.

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

Write all of `content` at the current read/write position.

- `content` — passed by value

Returns: the number of **bytes written**.

Errors: throws `E6007` when the handle is invalid or the write fails.

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

Move the read/write position to an **absolute** offset `offset` (relative to the beginning of the
file).

- `offset` — target byte offset, must be non-negative

Returns: `true` on success.

Errors: throws `E6007` when the handle is invalid or the offset is invalid.

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

Errors: throws `E6007` when the handle is invalid.

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

Flush buffered content to disk.

Errors: throws `E6007` when the handle is invalid or the flush fails.

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

Read an environment variable.

Returns: the variable value; **returns an empty string when the variable does not exist** (no error
is raised). As a result, it is impossible to distinguish between "not set" and "set to empty
string".

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    // PATH must exist on mainstream platforms
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

Returns command line arguments.

Returns: a single string with all argv **joined by `\n`** (not a `List`). The first item is the
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

Switch the current working directory.

Returns: `true` on success.

Errors: throws `E6007` when the directory does not exist.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    before = os.getcwd()
    assert(os.chdir(".."))
    assert(os.chdir(before))     // Switch back
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

Errors: throws `E6007` when it cannot be obtained.

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

- [`std.fs`](./fs) — Path-level file and directory operations
- [Error code reference](../error-code/) — `E6007` generic runtime error
