---
title: 'std.convert'
description: 'Conversion of arbitrary values to String'
---

# std.convert

The type conversion module, currently providing value-to-`String` conversion.

```yaoxiang
use std.convert
```

## Conversion Rules

`to_string` formats according to the runtime form of the value:

| Type     | Output Form                      | Example                         |
| -------- | -------------------------------- | ------------------------------- |
| `Void`   | `void`                           | `void`                          |
| `Bool`   | `true` / `false`                 | `true`                          |
| `Int`    | decimal                          | `42`                            |
| `Float`  | see below                        | `3.14` / `2.0`                  |
| `Char`   | the character itself             | `a`                             |
| `String` | original content (**no quotes**) | `hello`                         |
| `List`   | `[element, ...]`                 | `[1, 2, 3]`                     |
| `Dict`   | `{k: v, ...}`                    | `{a: 1}`                        |
| `Tuple`  | `(element, ...)`                 | `(1, hello)`                    |
| `Array`  | `[element, ...]`                 | `[1, 2]`                        |
| `Range`  | `start..end`                     | `1..5` (omitted when step is 1) |
| `Bytes`  | `bytes[length]`                  | `bytes[3]`                      |

Integer values of `Float` will have a decimal place appended (`2.0` instead of `2`), to distinguish
between `Int` and `Float`.

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

Converts an arbitrary value to its string representation.

- `value` —— a value of any type

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

The string itself has no quotes added:

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string("hi") == "hi")
}
```

Compound types are recursively expanded (the format is intentionally not asserted to avoid
fragility):

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

In addition to the general-purpose `convert.to_string`, the module also exports the following
same-named functions bound by type, with behavior identical to it:

| Export Name        | Signature          |
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

These bindings are used for runtime `Stringable` dispatch; for everyday code, simply use
[`convert.to_string`](#to_string).

> The `Set` type has been removed at the language level; `set.to_string` is retained as a
> compatibility placeholder.

## Related

- [`std.io`](./io) —— `print` / `println`'s internal formatting follows the same set of rules
- [`std.string`](./string) —— string operations
