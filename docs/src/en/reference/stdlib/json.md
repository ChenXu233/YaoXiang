---
title: 'std.json'
description: 'JSON parsing and serialization'
---

# std.json

JSON parsing and serialization module (RFC 8259, issue #55). `Json` is the **record-style sum type**
exported by this module, sharing the same mechanism as user-defined sum types: variant construction
`Json.num(1.5)`, `match` variant deconstruction, and exhaustiveness checking.

```yaoxiang
use std.json
```

> **This page is hand-written**. This module is implemented in pure YaoXiang source
> (`src/std/json.yx`) and is not in the generation scope of `StdModule::exports()`, so there is no
> `<!-- stdlib:... -->` generation marker—the signatures in the table below are taken verbatim from
> the export declarations in `src/std/json.yx`.

## Exports

| Export      | Signature                             | Description                       |
| ----------- | ------------------------------------- | --------------------------------- |
| `Json`      | Sum type (six variants)               | JSON value                        |
| `parse`     | `(s: &String) -> Result(Json, Error)` | Parse JSON text; failure is `Err` |
| `stringify` | `(v: &Json) -> String`                | Compact serialization             |
| `pretty`    | `(v: &Json) -> String`                | 2-space indented serialization    |

## Json Type

Six variants aligned with RFC 8259 §3:

```
Json: Type = {
    null: () -> Json,
    bool: (Bool) -> Json,
    num: (Float) -> Json,
    str: (String) -> Json,
    arr: (Vec(Json)) -> Json,
    obj: (Dict(String, Json)) -> Json,
}
```

Variant construction in expression position **requires a type qualifier** (RFC-010), i.e.
`Json.num(1.5)` rather than bare `num(1.5)`; deconstruction uses `match` variant patterns (requires
`use std.json` to import the variant set, see
[Syntax Specification §2.8](../language-spec/syntax.md)).

Numbers are uniformly carried via `Float`: the JSON specification itself does not distinguish
between integer and float types (`1` and `1.0` are the same value); after parsing, they are always
`Json.num`, and during serialization they are output in the shortest `Float` representation.
Therefore `Json.num(1.0)` serializes to `1.0`.

## Functions

### parse

```yaoxiang
parse: (s: &String) -> Result(Json, Error)
```

Parse JSON text into `Json`.

- `s` — Text to parse (read-only borrow)

Returns: `Result.ok(Json)`; on failure, `Result.err(Error)`, **does not throw**. The error code is
always `E6013` (`src/std/result.rs:31`), and the message has the form
`<reason> at line <line>, column <column>`, with line and column starting from 1.

Grammar is strictly implemented per RFC 8259: trailing commas, trailing content, single quotes, and
comments are rejected; object keys must be strings.

```yaoxiang
use std.assert
use std.json
use std.result
use std.string

main: () -> Void = {
    v = result.unwrap(json.parse("{\"n\": 2.0}"))
    assert(json.stringify(v) == "{\"n\":2.0}")

    // Numbers are uniformly carried via Float
    assert(json.stringify(Json.num(1.0)) == "1.0")

    // Top-level scalars can also be parsed
    assert(json.stringify(result.unwrap(json.parse("null"))) == "null")
    assert(json.stringify(result.unwrap(json.parse("[true]"))) == "[true]")

    // Invalid input goes to the Err branch without interrupting
    bad = json.parse("{oops}")
    assert(result.is_err(bad))
    assert(result.code(result.unwrap_err(bad)) == "E6013")
    assert(string.contains(result.message(result.unwrap_err(bad)), "line 1"))
}
```

### stringify

```yaoxiang
stringify: (v: &Json) -> String
```

Compact serialization (without any whitespace).

- `v` — JSON value (read-only borrow; `Json` is a record type, copying by value is expensive, so a
  borrow is taken)

Returns: JSON text. String escaping is aligned with RFC 8259 §7: `"`, `\`, and U+0000..U+001F (`\b`
`\t` `\n` `\f` `\r` use the short form, others are written as `\u00xx`); non-ASCII characters are
output as-is.

> **Key order is not guaranteed**: object keys come from `std.dict`, and the output order follows
> the dictionary's traversal order, which is not equal to the source text's writing order. When
> stable output is needed, sort manually, or use [`pretty`](#pretty) for human reading.

### pretty

```yaoxiang
pretty: (v: &Json) -> String
```

Formatted serialization: 2-space indentation, arrays and objects expand element by element with
newlines; **empty containers still take the compact form** (`[]` / `{}`), no newlines. Shares the
same string escaping rules with [`stringify`](#stringify).

```yaoxiang
use std.json
use std.result

main: () -> Void = {
    v = result.unwrap(json.parse("{\"a\":[1.0,null]}"))

    // Compact: no whitespace
    s = json.stringify(v)
    println(s)     // {"a":[1.0,null]}

    // Indented: 2 spaces, empty containers still compact
    println(json.pretty(Json.arr([])))   // []
    println(json.pretty(v))
}
```

## Constructing Json

The `obj` variant takes a `Dict(String, Json)`, constructed key by key using [`std.dict`](./dict):

```yaoxiang
use std.dict
use std.json
use std.string

main: () -> Void = {
    d = dict.set(dict.new(), "x", Json.num(3.0))
    o = Json.obj(d)
    println(json.stringify(o))     // {"x":3.0}
    println(string.contains(json.pretty(o), "\n"))   // Indented form contains newlines
}
```

## Error Model

| Scenario                                     | Behavior                                     |
| -------------------------------------------- | -------------------------------------------- |
| Syntax error (missing key, trailing content) | `Result.err`, `code` is `E6013`              |
| Line/column location                         | Message has the form `… at line 1, column 2` |
| Type mismatch (passing non-`&String`)        | Compile-time type error                      |

Parse failure **does not interrupt execution**; propagate with `?` or branch explicitly with
[`result.unwrap`](./result#unwrap). Note that `?` propagation requires the outer function to return
`Result` (see the explanation of `Try` instantiation on the [Option page](./option#已知缺口)).

## Known Gaps

- Object key order is not guaranteed (see the [`stringify`](#stringify) note).
- `\uXXXX` escaping supports surrogate pair composition (high surrogate must be immediately followed
  by low surrogate); code point validity is checked by `string.from_char_code`, with invalid code
  points returning `E6012`.

## Related

- [`std.dict`](./dict) — Dictionary used to construct and read `Json.obj`
- [`std.string`](./string) — `char_code` / `from_char_code` are this module's escaping primitives
- [`std.result`](./result) — Unwraps the return value of `parse`
- [Error Code Reference](../error-code/) — `E6013` JSON parse failure
