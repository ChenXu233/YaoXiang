---
title: 'std.string'
description: 'String searching, splitting, formatting, and parsing'
---

# std.string

The string operations module. Except for `format`, all functions take read-only borrows (`&String`)
of their inputs, so the source string remains usable after the call.

When the argument types do not match, all functions degrade to **empty-string semantics** (rather
than reporting an error): `split` / `trim` / `upper` and the like treat non-`String` inputs as `""`.
This means a wrong argument type will not abort execution, but it will not produce the expected
result either — it is recommended that the type checker intercept such errors at compile time.

```yaoxiang
use std.string
```

## Function Overview

<!-- stdlib:table:string start -->

| Function      | Signature                                            |
| ------------- | ---------------------------------------------------- |
| `split`       | `(s: &String, sep: &String) -> Vec(String)`          |
| `trim`        | `(s: &String) -> String`                             |
| `upper`       | `(s: &String) -> String`                             |
| `lower`       | `(s: &String) -> String`                             |
| `replace`     | `(s: &String, old: &String, new: &String) -> String` |
| `contains`    | `(s: &String, sub: &String) -> Bool`                 |
| `starts_with` | `(s: &String, prefix: &String) -> Bool`              |
| `ends_with`   | `(s: &String, suffix: &String) -> Bool`              |
| `index_of`    | `(s: &String, sub: &String) -> Int`                  |
| `substring`   | `(s: &String, start: Int, end: Int) -> String`       |
| `is_empty`    | `(s: &String) -> Bool`                               |
| `len`         | `(s: &String) -> Int`                                |
| `chars`       | `(s: &String) -> Vec(String)`                        |
| `concat`      | `(s1: &String, s2: &String) -> String`               |
| `repeat`      | `(s: &String, n: Int) -> String`                     |
| `reverse`     | `(s: &String) -> String`                             |
| `format`      | `(format: &String, ...args) -> String`               |
| `parse_int`   | `(s: &String) -> Result(Int, Error)`                 |
| `parse_float` | `(s: &String) -> Result(Float, Error)`               |

<!-- stdlib:table:string end -->

## Functions

### split

<!-- stdlib:sig:string.split start -->

```yaoxiang
split: (s: &String, sep: &String) -> Vec(String)
```

<!-- stdlib:sig:string.split end -->

Splits `s` by `sep` and returns a list of substrings.

- `s` —— the string to be split
- `sep` —— the separator; **when empty, splits character by character**

Returns: `List(String)`. When the separator is not found, a single-element list is returned.

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    parts = string.split("a,b,c", ",")
    assert(list.len(parts) == 3)
    assert(list.get(parts, 0) == "a")

    // Empty separator → character-by-character
    cs = string.split("abc", "")
    assert(list.len(cs) == 3)
}
```

### trim

<!-- stdlib:sig:string.trim start -->

```yaoxiang
trim: (s: &String) -> String
```

<!-- stdlib:sig:string.trim end -->

Removes leading and trailing Unicode whitespace characters.

Returns: a new string with the leading and trailing whitespace removed (`s` is not modified).

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.trim("  hi  ") == "hi")
}
```

### upper

<!-- stdlib:sig:string.upper start -->

```yaoxiang
upper: (s: &String) -> String
```

<!-- stdlib:sig:string.upper end -->

Converts to uppercase (Unicode-aware).

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.upper("abc") == "ABC")
}
```

### lower

<!-- stdlib:sig:string.lower start -->

```yaoxiang
lower: (s: &String) -> String
```

<!-- stdlib:sig:string.lower end -->

Converts to lowercase (Unicode-aware).

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.lower("ABC") == "abc")
}
```

### replace

<!-- stdlib:sig:string.replace start -->

```yaoxiang
replace: (s: &String, old: &String, new: &String) -> String
```

<!-- stdlib:sig:string.replace end -->

Replaces **all** occurrences of `old` in `s` with `new`.

- `old` —— when empty, **`s` is returned as-is** (no insertion performed)

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.replace("a-b-c", "-", "+") == "a+b+c")
    assert(string.replace("abc", "", "x") == "abc")
}
```

### contains

<!-- stdlib:sig:string.contains start -->

```yaoxiang
contains: (s: &String, sub: &String) -> Bool
```

<!-- stdlib:sig:string.contains end -->

Whether `sub` appears in `s`. Always `true` for an empty string.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.contains("hello", "ell"))
    assert(!string.contains("hello", "xyz"))
}
```

### starts_with

<!-- stdlib:sig:string.starts_with start -->

```yaoxiang
starts_with: (s: &String, prefix: &String) -> Bool
```

<!-- stdlib:sig:string.starts_with end -->

Whether `s` starts with `prefix`. Always `true` for an empty `prefix`.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.starts_with("hello", "he"))
}
```

### ends_with

<!-- stdlib:sig:string.ends_with start -->

```yaoxiang
ends_with: (s: &String, suffix: &String) -> Bool
```

<!-- stdlib:sig:string.ends_with end -->

Whether `s` ends with `suffix`. Always `true` for an empty `suffix`.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.ends_with("hello", "lo"))
}
```

### index_of

<!-- stdlib:sig:string.index_of start -->

```yaoxiang
index_of: (s: &String, sub: &String) -> Int
```

<!-- stdlib:sig:string.index_of end -->

The **byte** index of the first occurrence of `sub`.

Returns: the index when found; `-1` when not found.

> The return value is a byte offset. When the string contains multi-byte characters, you can convert
> with `chars` first to locate character indices.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.index_of("hello", "ll") == 2)
    assert(string.index_of("hello", "xyz") == -1)
}
```

### substring

<!-- stdlib:sig:string.substring start -->

```yaoxiang
substring: (s: &String, start: Int, end: Int) -> String
```

<!-- stdlib:sig:string.substring end -->

Extracts the `[start, end)` range by **character** index.

- `start` —— starting character index; defaults to `0`
- `end` —— ending character index (exclusive); defaults to the end of the string

Returns: the extracted result. Out-of-range bounds are **clamped** to the valid range and do not
produce an error; when `start > end` it is clamped to an empty string.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.substring("hello", 1, 4) == "ell")
    assert(string.substring("hello", 1, 99) == "ello")   // upper bound clamped
}
```

### is_empty

<!-- stdlib:sig:string.is_empty start -->

```yaoxiang
is_empty: (s: &String) -> Bool
```

<!-- stdlib:sig:string.is_empty end -->

Whether `s` is an empty string.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.is_empty(""))
    assert(!string.is_empty("x"))
}
```

### len

<!-- stdlib:sig:string.len start -->

```yaoxiang
len: (s: &String) -> Int
```

<!-- stdlib:sig:string.len end -->

Returns the **UTF-8 byte length**, not the number of characters.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.len("hello") == 5)
    assert(string.len("中") == 3)   // byte length
}
```

### chars

<!-- stdlib:sig:string.chars start -->

```yaoxiang
chars: (s: &String) -> Vec(String)
```

<!-- stdlib:sig:string.chars end -->

Splits into a list of single-character strings (by Unicode scalar values).

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    cs = string.chars("ab")
    assert(list.len(cs) == 2)
    assert(cs[0] == "a")
}
```

### concat

<!-- stdlib:sig:string.concat start -->

```yaoxiang
concat: (s1: &String, s2: &String) -> String
```

<!-- stdlib:sig:string.concat end -->

Concatenates two strings. The `+` operator can also be used directly.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.concat("a", "b") == "ab")
}
```

### repeat

<!-- stdlib:sig:string.repeat start -->

```yaoxiang
repeat: (s: &String, n: Int) -> String
```

<!-- stdlib:sig:string.repeat end -->

Repeats `s` `n` times.

- `n` —— the number of repetitions; when `n <= 0` an empty string is returned

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.repeat("ab", 3) == "ababab")
    assert(string.repeat("ab", 0) == "")
}
```

### reverse

<!-- stdlib:sig:string.reverse start -->

```yaoxiang
reverse: (s: &String) -> String
```

<!-- stdlib:sig:string.reverse end -->

Reverses the string by character.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.reverse("abc") == "cba")
}
```

### format

<!-- stdlib:sig:string.format start -->

```yaoxiang
format: (format: &String, ...args) -> String
```

<!-- stdlib:sig:string.format end -->

Formats using `{index}` placeholders, with optional width/alignment specifiers.

Placeholder syntax:

| Form     | Meaning                                                         |
| -------- | --------------------------------------------------------------- |
| `{0}`    | the 0th argument (arguments after `format` are numbered from 0) |
| `{0:03}` | width 3                                                         |
| `{0:>3}` | width 3, right-aligned (default)                                |
| `{0:<3}` | width 3, left-aligned                                           |
| `{0:^3}` | width 3, centered                                               |

Literal curly braces are written by doubling: two left braces produce one literal left brace, and
the same applies to right braces.

Return value: the formatted string. Arguments are first converted to strings (same as
`convert.to_string`); an out-of-range index yields an empty string, and an illegal width is treated
as `0`.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.format("{0}-{1}", "a", "b") == "a-b")
    assert(string.format("[{0:>5}]", "ab") == "[   ab]")
    assert(string.format("[{0:<5}]", "ab") == "[ab   ]")
}
```

### parse_int

<!-- stdlib:sig:string.parse_int start -->

```yaoxiang
parse_int: (s: &String) -> Result(Int, Error)
```

<!-- stdlib:sig:string.parse_int end -->

Parses a decimal integer (leading and trailing whitespace are removed automatically).

Returns: `Result.ok(Int)` on success; `Result.err(Error)` on failure, with `code` `E6010`. **No
error is thrown.**

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

### parse_float

<!-- stdlib:sig:string.parse_float start -->

```yaoxiang
parse_float: (s: &String) -> Result(Float, Error)
```

<!-- stdlib:sig:string.parse_float end -->

Parses a float (leading and trailing whitespace are removed automatically).

Returns: `Result.ok(Float)` on success; `Result.err(Error)` on failure, with `code` `E6011`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_float("3.14")))
    assert(result.is_err(string.parse_float("xxx")))
}
```

## Related

- [`std.convert`](./convert) —— convert numbers to strings
- [`std.result`](./result) —— unpack results from `parse_*`
