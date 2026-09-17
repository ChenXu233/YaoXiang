---
title: 'std.io'
description: 'Standard output, standard input, and whole-file read/write'
---

# std.io

Input/output module. Provides standard output, standard input reading, and convenience functions for
one-shot whole-file read/write. For incremental file I/O via handles, use [`std.os`](./os).

```yaoxiang
use std.io
```

## Platform availability

The following functions starting from `read_line` depend on operating system I/O and are **not
exported** on the `wasm32` target: `read_line`, `read_file`, `write_file`, `append_file`. `print` /
`println` / `format_fallback` are available on all targets.

## Function summary

<!-- stdlib:table:io start -->

| Function          | Signature                                   |
| ----------------- | ------------------------------------------- |
| `print`           | `(...args) -> Void`                         |
| `println`         | `(...args) -> ()`                           |
| `read_line`       | `() -> String`                              |
| `read_file`       | `(path: &String) -> String`                 |
| `write_file`      | `(path: &String, content: &String) -> Bool` |
| `append_file`     | `(path: &String, content: &String) -> Bool` |
| `format_fallback` | `(value, type_name: &String) -> String`     |

<!-- stdlib:table:io end -->

## Functions

### print

<!-- stdlib:sig:io.print start -->

```yaoxiang
print: (...args) -> Void
```

<!-- stdlib:sig:io.print end -->

Outputs all arguments in order, **without appending a newline**. Multiple arguments are separated by
a single space.

Arguments are formatted: `String` outputs its content directly; `List` / `Dict` / `Tuple` are
expanded recursively; other values are output as literals.

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

Same as [`print`](#print), but appends a newline at the end of the output.

`println()` called without arguments outputs an empty line:

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

Reads a line from standard input.

Returns: the entire line read, **with the trailing newline removed** (`\n` or `\r\n`). Error: throws
`E6007` when the read fails.

> Interactive examples cannot be auto-run in the docs; the following is for reference only.

```yaoxiang
use std.io

main = {
    println("Please enter your name: ")
    name = io.read_line()
    println("Hello, " + name)
}
```

### read_file

<!-- stdlib:sig:io.read_file start -->

```yaoxiang
read_file: (path: &String) -> String
```

<!-- stdlib:sig:io.read_file end -->

Reads the entire file content as a string in one go.

- `path` — file path (read-only borrow)

Returns: the entire file content. Error: throws `E6007` when the file does not exist or lacks
permission. **Does not return an empty string.**

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

Writes `content` to `path`, **overwriting** any existing content; creates the file if it does not
exist.

- `path` — file path (read-only borrow)
- `content` — content to write (read-only borrow)

Returns: `true` on successful write. Error: throws `E6007` when the directory does not exist or
lacks permission (does not return `false`).

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

**Appends** `content` to the end of `path`; creates the file if it does not exist.

Returns: `true` on successful write. Error: throws `E6007` when lacking permission.

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

Formats a value by its type name, producing a prefixed representation such as `int(42)` / `list@3`.

This is an internal helper function used by the runtime's generic formatting path as a callback;
everyday code should use [`std.convert.to_string`](./convert#to_string) directly.

- `value` — any value
- `type_name` — type-name string

Returns: a string representation with a type prefix.

```yaoxiang
use std.assert
use std.io
use std.string

main = {
    s = io.format_fallback(42, "int")
    assert(string.contains(s, "42"))
}
```

## See also

- [`std.os`](./os) — file handles, directories, and environment variables
- [`std.convert`](./convert) — value to string
