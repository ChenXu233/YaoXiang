---
title: 'std.string'
description: 'String search, splitting, formatting, and parsing'
---

# std.string

String operation module. Except for `format`, all functions take read-only borrows (`&String`) of
their inputs, and the source string can still be used after the call.

All functions degrade to **empty string semantics** (rather than reporting an error) when the
argument types do not match: `split`/`trim`/`upper` etc. treat non-`String` inputs as `""`. This
means wrong argument types won't interrupt execution, but you also won't get the expected result —
it is recommended to rely on the type checker to catch this at compile-time.

```yaoxiang
use std.string
```

## Function overview

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

<!-- stdlib:table:string end -->## Functions

### split

<!-- stdlib:sig:string.split start -->

```yaoxiang
split: (s: &String, sep: &String) -> Vec(String)
```

<!-- stdlib:sig:string.split end -->

Split `s` by `sep`, returns a list of substrings.

- `s` —— string to split
- `sep` —— separator; **when empty, splits character by character**

Returns: `List(String)`. Returns a single-element list when the separator is not found.

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

Remove leading and trailing Unicode whitespace characters.

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

Convert to uppercase (Unicode-aware).

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

Convert to lowercase (Unicode-aware).

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

Replace **all** occurrences of `old` in `s` with `new`.

- `old` —— when empty, **returns `s` as-is** (no insertion)

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

Whether `sub` appears in `s`. An empty string is always `true`.

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

Whether `s` starts with `prefix`. An empty `prefix` is always `true`.

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

Whether `s` ends with `suffix`. An empty `suffix` is always `true`.

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

**Byte** index of the first occurrence of `sub`.

Returns: returns the index when found; returns `-1` when not found.

> The return value is a byte offset. When multi-byte characters are involved, you can use `chars` to
> convert first and then locate the character index.

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

Take the `[start, end)` range by **character** index.

- `start` —— starting character index, default `0`
- `end` —— ending character index (exclusive), default is the end of the string

Returns: the substring result. Out-of-range boundaries are **clamped** to the valid range, no error;
when `start > end`, clamped to an empty string.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.substring("hello", 1, 4) == "ell")
    assert(string.substring("hello", 1, 99) == "ello")   // upper bound clamp
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

Split into a list of single-character strings (by Unicode scalar value).

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

Concatenate two strings. You can also use the `+` operator directly.

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

Repeat `s` `n` times.

- `n` —— number of repetitions; `n <= 0` returns an empty string

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

Reverse by character.

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

Format using `{index}` placeholders, with optional width/alignment specifiers.

Placeholder syntax:

| Form     | Meaning                                                         |
| -------- | --------------------------------------------------------------- |
| `{0}`    | The 0th argument (arguments are numbered from 0 after `format`) |
| `{0:03}` | Width 3                                                         |
| `{0:>3}` | Width 3, right-aligned (default)                                |
| `{0:<3}` | Width 3, left-aligned                                           |
| `{0:^3}` | Width 3, centered                                               |

Literal curly braces are written by doubling: two left braces produce one literal left brace, and
similarly for right braces.

Return value: the formatted string. Arguments are first converted to strings (same as
`convert.to_string`); out-of-range indices yield an empty string, and invalid widths are treated as
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

Parse a decimal integer (automatically trims leading and trailing whitespace).

Returns: on success, `Result.ok(Int)`; on failure, `Result.err(Error)` with `code` `E6010`. **Does
not throw**.

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

Parse a floating-point number (automatically trims leading and trailing whitespace).

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

### char_code

<!-- stdlib:sig:string.char_code start -->

```yaoxiang
char_code: (s: &String, i: Int) -> Int
```

<!-- stdlib:sig:string.char_code end -->

Get the Unicode code point of the `i`-th character (scalar value semantics, in the same domain as
[`chars`](#chars) / [`substring`](#substring)).

- `s` —— source string (read-only borrow)
- `i` —— character index (**character** index, not byte offset)

Returns: the code point; **returns `-1` for an out-of-range index** (same convention as
[`index_of`](#index_of) miss — code points are non-negative, so `-1` is unambiguous). **Does not
throw**.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.char_code("A", 0) == 65)
    assert(string.char_code("中", 0) == 0x4E2D)     // character index, not byte offset
    assert(string.char_code("A", 9) == -1)          // out-of-range returns -1
}
```

### from_char_code

<!-- stdlib:sig:string.from_char_code start -->

```yaoxiang
from_char_code: (n: Int) -> Result(String, Error)
```

<!-- stdlib:sig:string.from_char_code end -->

Code point to single-character string.

- `n` —— Unicode code point

Returns: on success, `Result.ok(String)` (a single string); on invalid code point,
`Result.err(Error)` with `code` `E6012`. **Does not throw**. Validity is determined using
`char::from_u32` (`src/std/string.rs:613-619`), the same rules as the lexer-level `\u{…}` escape:
negatives, the surrogate range (U+D800–U+DFFF), and values greater than `U+10FFFF` are all invalid.
`U+10000..=U+10FFFF` directly produces valid 4-byte UTF-8; UTF-16 surrogate pair composition is left
to the yx layer.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.unwrap(string.from_char_code(65)) == "A")
    assert(result.unwrap(string.from_char_code(0x4E2D)) == "中")

    // invalid code point goes to the Err branch
    assert(result.is_err(string.from_char_code(-1)))
    assert(result.code(result.unwrap_err(string.from_char_code(-1))) == "E6012")

    // inverse of char_code
    assert(result.unwrap(string.from_char_code(string.char_code("A", 0))) == "A")
}
```

## Related

- [`std.convert`](convert) —— number to string
- [`std.result`](result) —— unwrap results of `parse_*` / `from_char_code`
- [`std.json`](json) —— Unicode primitives for string escaping come from this module
