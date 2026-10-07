---
title: 'std.string'
description: 'String search, splitting, formatting, and parsing'
---

# std.string

String operations module. Except for `format`, all functions take read-only borrows (`&String`) of
their arguments, so the source string remains usable after the call.

When argument types do not match, all functions degrade to **empty string semantics** (rather than
reporting an error): `split`/`trim`/`upper` and others treat non-`String` arguments as `""`. This
means a wrong argument type will not be interrupted, but it will not produce the expected result —
it is recommended to rely on the type checker to catch this at compile time.

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

Splits `s` by `sep`, returning a list of substrings.

- `s` — the source string to split
- `sep` — the separator; **when empty, splits by individual characters**

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

Removes leading and trailing Unicode whitespace.

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

Replaces **all** occurrences of `old` in `s` with `new`.

- `old` — when empty, returns `s` **as-is** (no insertion)

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

The **scalar value (code point) index** of the first occurrence of `sub` (#385).

Returns: the index if found; `-1` if not found.

> The return value is in the same domain as `substring` / `char_code` / `s[i]`, and can be composed
> directly with them.

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

Takes the `[start, end)` range by **character** index.

- `start` — start character index, default `0`
- `end` — end character index (exclusive), defaults to the end of the string

Returns: the substring. Out-of-range bounds are **clamped** to the valid range without error; when
`start > end`, the result is clamped to an empty string.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.substring("hello", 1, 4) == "ell")
    assert(string.substring("hello", 1, 99) == "ello")   // Upper bound clamped
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

Returns the **count of Unicode scalar values (code points)** — same domain as `substring` / `chars`
/ `char_code` / `index_of` (#385).

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.len("hello") == 5)
    assert(string.len("中") == 1)    // Code point count, not UTF-8 byte count
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

Concatenates two strings. You can also use the `+` operator directly.

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

- `n` — repeat count; returns an empty string when `n <= 0`

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
| `{0}`    | the 0th argument (arguments after `format` are numbered from 0) |
| `{0:03}` | width 3                                                         |
| `{0:>3}` | width 3, right-aligned (default)                                |
| `{0:<3}` | width 3, left-aligned                                           |
| `{0:^3}` | width 3, centered                                               |

Literal curly braces are written by doubling: two left braces produce one literal left brace, same
for right braces.

Return value: the formatted string. Arguments are first converted to strings (same as
`convert.to_string`); out-of-range indices yield an empty string, and illegal widths are treated as
`0`.

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

Parses a decimal integer (automatically strips leading and trailing whitespace).

Returns: `Result.ok(Int)` on success; `Result.err(Error)` on failure, whose `code` is `E6010`.
**Does not throw**.

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

Parses a floating-point number (automatically strips leading and trailing whitespace).

Returns: `Result.ok(Float)` on success; `Result.err(Error)` on failure, whose `code` is `E6011`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_float("3.14")))
    assert(result.is_err(string.parse_float("xxx")))
}
```

### char_code

<!-- stdlib:sig:string.char_code start -->

```yaoxiang
char_code: (s: &String, i: Int) -> Int
```

<!-- stdlib:sig:string.char_code end -->

Returns the Unicode code point of the `i`-th character (scalar value domain, same as
[`chars`](#chars) / [`substring`](#substring)).

- `s` — source string (read-only borrow)
- `i` — character index (Unicode scalar value domain, same unit as [`index_of`](#index_of))

Returns: the code point; **returns `-1` if the index is out of range** (same convention as a miss in
[`index_of`](#index_of) — code points are non-negative, so `-1` is unambiguous). **Does not throw**.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.char_code("A", 0) == 65)
    assert(string.char_code("中", 0) == 0x4E2D)     // Scalar value index
    assert(string.char_code("A", 9) == -1)          // Out of range returns -1
}
```

### from_char_code

<!-- stdlib:sig:string.from_char_code start -->

```yaoxiang
from_char_code: (n: Int) -> Result(String, Error)
```

<!-- stdlib:sig:string.from_char_code end -->

Converts a code point to a single-character string.

- `n` — Unicode code point

Returns: `Result.ok(String)` (a single-character string) on success; `Result.err(Error)` for an
illegal code point, whose `code` is `E6012`. **Does not throw**. Validity is determined by
`char::from_u32` (`src/std/string.rs:613-619`), the same set of rules as the lexer-level `\u{…}`
escape: negative numbers, the surrogate range (U+D800–U+DFFF), and values greater than `U+10FFFF`
are all illegal. `U+10000..=U+10FFFF` directly produces valid 4-byte UTF-8; UTF-16 surrogate pair
composition is left to the yx layer.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.unwrap(string.from_char_code(65)) == "A")
    assert(result.unwrap(string.from_char_code(0x4E2D)) == "中")

    // Illegal code point goes to the Err branch
    assert(result.is_err(string.from_char_code(-1)))
    assert(result.code(result.unwrap_err(string.from_char_code(-1))) == "E6012")

    // Inverse of char_code
    assert(result.unwrap(string.from_char_code(string.char_code("A", 0))) == "A")
}
```

## Related

- [`std.convert`](convert) — number to string conversion
- [`std.result`](result) — unpack results of `parse_*` / `from_char_code`
- [`std.json`](json) — the Unicode primitives used for string escaping come from this module
