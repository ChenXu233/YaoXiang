---
title: 'match Basics'
---

# match Basics

`match` is YaoXiang's most powerful control-flow structure. It lets you choose different handling
paths based on the shape of a **value**. If you have used `switch` in other languages, you will find
that `match` is a fully upgraded version.

## Basic Syntax

The syntax specification defines the `match` expression as follows:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

Breaking this down:

- `match` is followed by the value to match.
- `{}` contains one or more **match arms** (MatchArm).
- Each match arm consists of a **pattern** followed by `=>`, then a **result expression**.
- Each arm ends with a comma.

The simplest example is:

```yaoxiang
number = 2

text = match number {
    0 => "零",
    1 => "一",
    2 => "二",
}
print(text)  // "二"
```

## match Is an Expression

Like `if`, `match` is also an **expression**—it evaluates to a value. All match arms must have the
same return type:

```yaoxiang
score = 85

grade = match score {
    n if n >= 90 => "A",
    80..89 => "B",     // Range pattern: matches 80 through 89 (inclusive on both ends)
    n if n >= 70 => "C",
    n if n >= 60 => "D",
    _ => "F",          // Wildcard: matches all remaining cases
}
print(grade)  // "B"
```

The range pattern `80..89` is a **closed interval on both ends**—both `80` and `89` match (B grade =
80-89, ten values). The endpoints must be integer literals; write negative endpoints directly (e.g.,
`-5..-1`). A reversed range (e.g., `10..0`) is empty, and the compiler reports it as unreachable at
compile time. A `match` with a range arm still requires the wildcard `_` as a fallback because the
range does not cover every possible Int value. Guard expressions (`pattern if condition`) are
covered in depth in [Advanced Pattern Matching](../pattern-matching/index.md).

> **Note**: Pattern-position `lo..hi` (closed-interval matching) and expression-position `a..b`
> (`for` iteration and `in` range checks, left-closed and right-open) have different endpoint
> semantics: the pattern form makes a branch decision and does not construct a Range value.

## Basic Patterns

### Literal Patterns

Match specific values:

```yaoxiang
response = 404

message = match response {
    200 => "OK",
    301 => "Moved",
    404 => "Not Found",
    500 => "Server Error",
    _ => "Unknown",
}
print(message)  // "Not Found"
```

### Identifier Patterns

Use a variable name to capture the matched value:

```yaoxiang
// Result variant constructors must be type-qualified: writing ok(42) bare reports E1001
use std.result

r: Result(Int, String) = Result(Int, String).ok(42)

description = match r {
    ok(value) => "成功，值是: {value}",
    err(e) => "失败，原因: " + e,
}
print(description)  // "成功，值是 42"
```

The `value` in `ok(value)` is an identifier pattern—it captures the actual value wrapped by `ok`,
and you can use it in the expression after `=>`.

### Range Patterns

Use `lo..hi` to match a range of integers (**inclusive on both ends**):

```yaoxiang
temp = 37

state = match temp {
    35..36 => "偏低",
    36..38 => "正常",
    _ => "异常",
}
print(state)  // "正常"
```

`36..38` covers both `36` and `38`. The endpoints must be integer literals; write negative endpoints
directly (e.g., `-5..-1`). Range patterns do not exhaust the integers—they cover only a section of
the range—so a `match` with range arms still requires the wildcard `_` as a fallback.

### Wildcard Patterns

`_` is the wildcard and matches **any value**. It is usually placed last as a fallback:

```yaoxiang
command = "exit"

action = match command {
    "start" => "启动服务",
    "stop" => "停止服务",
    "restart" => "重启服务",
    _ => "未知指令: " + command,
}
print(action)  // "未知指令: exit"
```

## Matches Must Be Exhaustive

YaoXiang's `match` requires every possible case to be covered—if the compiler finds that you omitted
a possible value, it reports an error directly. This is what makes `match` safe.

```yaoxiang
// This code will fail to compile
// value = true
// result = match value {
//     true => "是",
//     // Missing the false branch—compile error!
// }

// Correct—use _ as a fallback
value = true
result = match value {
    true => "是",
    _ => "否",      // _ ensures false is also handled
}
```

When you know that there are only a finite number of possible cases (for example, when matching an
enum), the compiler checks that every variant is covered. This is an effective safeguard against
missing-branch bugs.

## Combining Multiple Patterns

A match arm can match multiple patterns, separated by `|`:

```yaoxiang
day = "sunday"

type = match day {
    "monday" | "tuesday" | "wednesday" | "thursday" | "friday" => "工作日",
    "saturday" | "sunday" => "休息日",
    _ => "无效",
}
print(type)  // "休息日"
```

## Match Arms Execute in Order

`match` tries arms starting from the first one. The **first branch that matches successfully takes
effect**, and later branches are not executed:

<!-- docs-example: skip -->

```yaoxiang
number = 5

result = match number {
    _ => "其他",     // The wildcard matches everything, so it matches here
    5 => "五",       // It will never be executed—the arm above has already matched
}
print(result)  // "其他"
```

> The example above is **intentionally made to fail the check**—the compiler reports
> `E1031 Unreachable pattern`. It uses a real diagnostic to demonstrate the rule that “the first
> matching arm takes effect.”

This feature makes it a good practice to **place the wildcard `_` last**.

## Summary

| Key                  | Description                                                        |
| -------------------- | ------------------------------------------------------------------ |
| Syntax               | `match value { pattern => expression, ... }`                       |
| Expression           | `match` evaluates to a value; all branches have the same type      |
| Literal pattern      | Matches a specific value exactly: `200 => "OK"`                    |
| Identifier pattern   | Captures the value in a variable: `ok(value) => ...`               |
| Range pattern        | Matches an integer range (inclusive on both ends): `80..89 => "B"` |
| Wildcard `_`         | Matches any value as a fallback                                    |
| Exhaustiveness       | Must cover every possible case; the compiler checks this           |
| Multiple patterns    | `pattern1 \| pattern2 => expression`                               |
| Sequential execution | Runs from top to bottom; the first matching branch takes effect    |

> **Next**: This article covers the basic use of `match`. For more advanced patterns (nested
> patterns, guard expressions, struct destructuring, etc.), see
> [Advanced Pattern Matching](../pattern-matching/index.md).
