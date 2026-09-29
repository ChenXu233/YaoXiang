---
title: 'std.string'
description: 'String search, splitting, formatting and parsing'
---

# std.string

String manipulation module. Except for `format`, all functions perform read-only borrowing
(`&String`) on their arguments; the source string remains usable after the call.

When the argument types do not match, all functions degrade to **empty string semantics** (rather
than reporting an error): `split` / `trim` / `upper` and the like treat a non-`String` argument as
`""`. This means incorrect argument types will not be interrupted, but they also won't produce the
expected result — relying on the type checker to catch this at compile-time is recommended.

```yaoxiang
use std.string
```

## Function Overview

<!-- stdlib:table:string start -->

| Function         | Signature                                            |
| ---------------- | ---------------------------------------------------- |
| `split`          | `(s: &String, sep: &String) -> Vec(String)`          |
| `trim`           | `(s: &String) -> String`                             |
| `upper`          | `(s: &String) -> String`                             |
| `lower`          | `(s: &String) -> String`                             |
| `replace`        | `(s: &String, old: &String, new: &String) -> String` |
| `contains`       | `(s: &String, sub: &String) -> Bool`                 |
| `starts_with`    | `(s: &String, prefix: &String) -> Bool`              |
| `ends_with`      | `(s: &String, suffix: &String) -> Bool`              |
| `index_of`       | `(s: &String, sub: &String) -> Int`                  |
| `substring`      | `(s: &String, start: Int, end: Int) -> String`       |
| `is_empty`       | `(s: &String) -> Bool`                               |
| `len`            | `(s: &String) -> Int`                                |
| `chars`          | `(s: &String) -> Vec(String)`                        |
| `concat`         | `(s1: &String, s2: &String) -> String`               |
| `repeat`         | `(s: &String, n: Int) -> String`                     |
| `reverse`        | `(s: &String) -> String`                             |
| `format`         | `(format: &String, ...args) -> String`               |
| `parse_int`      | `(s: &String) -> Result(Int, Error)`                 |
| `parse_float`    | `(s: &String) -> Result(Float, Error)`               |
| `char_code`      | `(s: &String, i: Int) -> Int`                        |
| `from_char_code` | `(n: Int) -> Result(String, Error)`                  |

<!-- stdlib:table:string end -->

## Functions

### split

<!-- stdlib:sig:string.split start -->

```yaoxiang
split: (s: &String, sep: &String) -> Vec(String)
```

<!-- stdlib:sig:string.split end -->

Split `s` by `sep` and return a list of substrings.

- `s` — the string to split
- `sep` — the separator; **when empty, splits character by character**

Returns: `List(String)`. When the separator is not found, returns a single-element list.

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    parts = string.split("a,b,c", ",")
    assert(list.len(parts) == 3)
    assert(list.get(parts, 0) == "a")

    // Empty separator → character by character
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

Returns: a new string with leading and trailing whitespace removed (does not modify `s`).

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

Replaces **every** occurrence of `old` in `s` with `new`.

- `old` — when empty, **returns `s` as-is** (no insertion)

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

Whether `sub` appears in `s`. An empty string always returns `true`.

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

Whether `s` starts with `prefix`. An empty `prefix` always returns `true`.

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

Whether `s` ends with `suffix`. An empty `suffix` always returns `true`.

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

> The returned value is a byte offset. When the string contains multi-byte characters, you can
> convert to a character list with `chars` first to locate a character index.

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

Takes the range `[start, end)` by **character** index.

- `start` — starting character index, defaults to `0`
- `end` — ending character index (exclusive), defaults to the end of the string

Returns: the sliced result. Out-of-bounds boundaries are **clamped** to the valid range rather than
reported as an error; when `start > end`, the result is clamped to an empty string.

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

Splits into a list of single-character strings (by Unicode scalar value).

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

- `n` — the number of repetitions; `n <= 0` returns an empty string

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

Reverses by character.

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
| `{0}`    | The 0th argument (arguments after `format` are numbered from 0) |
| `{0:03}` | Width 3                                                         |
| `{0:>3}` | Width 3, right-aligned (default)                                |
| `{0:<3}` | Width 3, left-aligned                                           |
| `{0:^3}` | Width 3, centered                                               |

Literal curly braces are written by doubling: two left braces produce one literal left brace, and
the same applies to right braces.

Return value: the formatted string. Arguments are first converted to strings (same as
`convert.to_string`); out-of-range indices yield the empty string, and an invalid width is treated
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

Parses a decimal integer (leading and trailing whitespace is trimmed automatically).

Returns: on success, `Result.ok(Int)`; on failure, `Result.err(Error)` with `code` `E6010`. **Does
not throw.**

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

Parses a floating-point number (leading and trailing whitespace is trimmed automatically).

Returns: on success, `Result.ok(Float)`; on failure, `Result.err(Error)` with `code` `E6011`.

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

- [`std.convert`](./convert) — number-to-string conversion
- [`std.result`](./result) — unwrap results from `parse_*`
