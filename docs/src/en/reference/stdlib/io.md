---
title: 'std.io'
description: 'Standard output, standard input, and formatting'
---

# std.io

Input/output module. Provides standard output and standard input reading. Whole-file
reading/writing, directory, and path operations see [`std.fs`](./fs); handle-level incremental
reading/writing see [`std.os`](./os).

```yaoxiang
use std.io
```

## Platform availability

`read_line` depends on operating system I/O and is **not exported** on the `wasm32` target. `print`
/ `println` / `format_fallback` are available on all targets.

## Function overview

<!-- stdlib:table:io start -->

| Function          | Signature                               |
| ----------------- | --------------------------------------- |
| `print`           | `(...args) -> Void`                     |
| `println`         | `(...args) -> ()`                       |
| `read_line`       | `() -> String`                          |
| `format_fallback` | `(value, type_name: &String) -> String` |

<!-- stdlib:table:io end -->## Functions

### print

<!-- stdlib:sig:io.print start -->

```yaoxiang
print: (...args) -> Void
```

<!-- stdlib:sig:io.print end -->

Outputs all arguments in order, **without appending a newline**. Multiple arguments are separated by
a single space.

Arguments are formatted: `String` outputs its content directly; `List` / `Dict` / `Tuple` are
recursively expanded; other values are output as literals.

```yaoxiang

main: () -> Void = {
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

`println()` outputs an empty line when called without arguments:

```yaoxiang
main: () -> Void = {
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

Returns: the entire line read, **with the trailing newline stripped** (`\n` or `\r\n`). Errors:
throws `E6007` on read failure.

> Interactive examples cannot be run automatically in the documentation; the following code is for
> reference only.

```yaoxiang
use std.io

main: () -> Void = {
    println("请输入你的名字：")
    name = io.read_line()
    println("你好，" + name)
}
```

### format_fallback

<!-- stdlib:sig:io.format_fallback start -->

```yaoxiang
format_fallback: (value, type_name: &String) -> String
```

<!-- stdlib:sig:io.format_fallback end -->

Formats a value by its type name, outputting a prefixed representation like `int(42)` / `list@3`.

This is an internal helper function, used by the runtime's generic formatting path as a callback;
regular code should directly use [`std.convert.to_string`](./convert#to_string).

- `value` — any value
- `type_name` — the type name string

Returns: a string representation with a type prefix.

```yaoxiang
use std.assert
use std.io
use std.string

main: () -> Void = {
    s = io.format_fallback(42, "int")
    assert(string.contains(s, "42"))
}
```

## Related

- [`std.fs`](./fs) — File, directory, and path operations
- [`std.os`](./os) — File handles and environment variables
- [`std.convert`](./convert) — Convert value to string
