---
title: 'std.convert'
description: 'Conversion from any value to String'
---

# std.convert

Type conversion module, currently providing conversion of values to `String`.

```yaoxiang
use std.convert
```

## Conversion Rules

`to_string` formats according to the runtime shape of the value:

| Type     | Output form                      | Example                               |
| -------- | -------------------------------- | ------------------------------------- |
| `Void`   | `void`                           | `void`                                |
| `Bool`   | `true` / `false`                 | `true`                                |
| `Int`    | decimal                          | `42`                                  |
| `Float`  | see below                        | `3.14` / `2.0`                        |
| `Char`   | the character itself             | `a`                                   |
| `String` | original content (**no quotes**) | `hello`                               |
| `List`   | `[element, ...]`                 | `[1, 2, 3]`                           |
| `Dict`   | `{k: v, ...}`                    | `{a: 1}`                              |
| `Tuple`  | `(element, ...)`                 | `(1, hello)`                          |
| `Array`  | `[element, ...]`                 | `[1, 2]`                              |
| `Range`  | `start..end`                     | `1..5` (step is omitted when it is 1) |
| `Bytes`  | `bytes[length]`                  | `bytes[3]`                            |

Integer values of `Float` are padded with one decimal place (`2.0` rather than `2`), to distinguish
`Int` from `Float`.

## Function Overview

<!-- stdlib:table:convert start -->

| Function           | Signature           |
| ------------------ | ------------------- |
| `to_string`        | `(value) -> String` |
| `int.to_string`    | `(self) -> String`  |
| `float.to_string`  | `(self) -> String`  |
| `bool.to_string`   | `(self) -> String`  |
| `char.to_string`   | `(self) -> String`  |
| `string.to_string` | `(self) -> String`  |
| `list.to_string`   | `(self) -> String`  |
| `dict.to_string`   | `(self) -> String`  |
| `tuple.to_string`  | `(self) -> String`  |
| `set.to_string`    | `(self) -> String`  |
| `range.to_string`  | `(self) -> String`  |

<!-- stdlib:table:convert end -->

## Functions

### to_string

<!-- stdlib:sig:convert.to_string start -->

```yaoxiang
to_string: (value) -> String
```

<!-- stdlib:sig:convert.to_string end -->

Convert any value to its string representation.

- `value` — a value of any type

Returns: the formatted string. Returns `"()"` when the argument is missing.

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string(42) == "42")
    assert(convert.to_string(true) == "true")
    assert(convert.to_string(false) == "false")
}
```

The string itself is not quoted:

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string("hi") == "hi")
}
```

Composite types are expanded recursively (the format is not asserted to avoid fragility):

```yaoxiang
use std.assert
use std.convert
use std.string

main: () -> Void = {
    s_list = convert.to_string([1, 2, 3])
    assert(string.len(s_list) > 0)

    s_dict = convert.to_string({})
    assert(string.len(s_dict) > 0)
}
```

### Type Method Form

In addition to the general `convert.to_string`, the module also exports the following type-bound
functions with the same name, whose behavior is exactly the same:

| Export name        | Signature          |
| ------------------ | ------------------ |
| `int.to_string`    | `(self) -> String` |
| `float.to_string`  | `(self) -> String` |
| `bool.to_string`   | `(self) -> String` |
| `char.to_string`   | `(self) -> String` |
| `string.to_string` | `(self) -> String` |
| `list.to_string`   | `(self) -> String` |
| `dict.to_string`   | `(self) -> String` |
| `tuple.to_string`  | `(self) -> String` |
| `set.to_string`    | `(self) -> String` |
| `range.to_string`  | `(self) -> String` |

These bindings are used for `Stringable` dispatch at runtime; for everyday code, just use
[`convert.to_string`](#to_string).

> The `Set` type has been removed at the language level, and `set.to_string` is retained as a
> compatibility placeholder.

## Related

- [`std.io`](io) — internal formatting for `print` / `println` uses the same set of rules
- [`std.string`](string) — string operations
