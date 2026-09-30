---
title: 'std.os'
description: 'File handles, environment variables, and working directory'
---

# std.os

Operating-system interface module: file-handle read/write, environment variables, and working
directory. For path-level file operations (whole-file read/write, directories, metadata, path
manipulation), see [`std.fs`](./fs).

```yaoxiang
use std.os
```

> All functions in this module rely on OS capabilities and are **not exported** on the `wasm32`
> target.

## File-handle model

> **Handles are passed by reference (issue #337 fixed)**: the signatures of `read` / `write` /
> `seek` / `tell` / `flush` / `close` are all `(file: &File, ...)`, so handles can be reused:
>
> ```yaoxiang
> f = os.open(p, "w")
> os.write(f, "hello world")
> os.close(f)
> ```
>
> Read/write after positioning (the reason `seek` exists) is now also available:
>
> ```yaoxiang
> r = os.open(p, "r")
> os.seek(r, 6)
> tail = os.read(r, 5)     // "world"
> os.close(r)
> ```
>
> Before the fix, the signatures had no `&`, so handles were passed by value → linear ownership →
> invalidated after one use, and `open → write → close` would report `E2014`.
>
> If you want to avoid manual handle management, you can still use the handle-free convenience
> functions: [`std.io.read_file`](./io#read_file) / [`write_file`](./io#write_file) /
> [`append_file`](./io#append_file), or this module's [`append_file`](#append_file).

`open` returns a file descriptor of type **`Int`** (the engine internally maintains a handle table),
so the `File` in the signature is effectively an `Int`.

Content is persisted to disk immediately after writing; explicit `close` is not required:

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
| `r`  | Read-only; file must exist      |
| `w`  | Write-only; create or truncate  |
| `a`  | Append; create or append to end |
| `r+` | Read/write; file must exist     |
| `w+` | Read/write; create or truncate  |
| `a+` | Read/write; create or append    |

## Function reference

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

<!-- stdlib:table:os end -->## File operations

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

Open a file and return a file descriptor.

- `path` — file path (read-only borrow)
- `mode` — open mode; see table above

Returns: an `Int` descriptor allocated from the internal handle table. **The handle can only be used
once** — any downstream call will move it (see [File-handle model](#file-handle-model)), so `open`
is usually inlined into a single call.

Errors: throws `E6007` if the mode is invalid, the file does not exist, or permission is denied.

> Since a handle can only be used once (issue #337), the return value is usually inlined directly
> into a downstream call.

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_open_only.txt"
    f = os.open(p, "w")
    assert(fs.exists(p))
    fs.remove(p)
}
```

### close

<!-- stdlib:sig:os.close start -->

```yaoxiang
close: (file: &File) -> Void
```

<!-- stdlib:sig:os.close end -->

Close a file handle and release the table entry.

Because a handle can only be used once, `close` is only meaningful in scenarios where the handle is
"opened and not used for anything else"; the written content is already persisted to disk when
[`write`](#write) returns, so an explicit close is usually unnecessary.

Errors: throws `E6007` if the descriptor is invalid (not opened or already closed).

```yaoxiang
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_close.txt"
    f = os.open(p, "w")
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

- `file` — file descriptor
- `n` — expected number of bytes to read

Returns: the content actually read (may be shorter than `n`; returns an empty string at end of
file). Invalid UTF-8 bytes are returned as replacement characters, without raising an error. Errors:
throws `E6007` if the descriptor is invalid or the read fails.

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

Returns: the **number of bytes** written. Errors: throws `E6007` if the descriptor is invalid or the
write fails.

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

Move the read/write position to the **absolute** offset `offset` (relative to the start of the
file).

- `offset` — target byte offset; must be non-negative

Returns: `true` on success. Errors: throws `E6007` if the descriptor is invalid or the offset is
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

Return the byte offset of the current read/write position.

Errors: throws `E6007` if the descriptor is invalid.

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

Errors: throws `E6007` if the descriptor is invalid or flushing fails.

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

## Environment variables

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

Read an environment variable.

Returns: the variable's value; **returns an empty string if the variable does not exist** (no
error). Thus "not set" cannot be distinguished from "set to empty string".

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

Set an environment variable (affects the current process).

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    os.set_env("__YX_DOC_ENV", "hello")
    assert(os.get_env("__YX_DOC_ENV") == "hello")
}
```

## Process and working directory

### args

<!-- stdlib:sig:os.args start -->

```yaoxiang
args: () -> String
```

<!-- stdlib:sig:os.args end -->

Return the command-line arguments.

Returns: a single string with all argv values **joined by `\n`** (not a `List`). The first element
is the path to the program itself.

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

Change the current working directory.

Returns: `true` on success. Errors: throws `E6007` if the directory does not exist.

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

Errors: throws `E6007` if it cannot be obtained.

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

- [`std.fs`](./fs) — path-level file and directory operations
- [Error code reference](../error-code/) — `E6007` generic runtime error
