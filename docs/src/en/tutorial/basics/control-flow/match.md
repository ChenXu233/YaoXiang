---
title: 'match Basics'
---

# match Basics

`match` is YaoXiang’s most powerful control-flow structure. It lets you choose different handling
paths based on the shape of a **value**. If you have used `switch` in other languages, you will find
that `match` is a comprehensive upgrade.

## Basic Syntax

The syntax specification defines the `match` expression as follows:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '->' Expr ','
```

Breaking it down:

- `match` is followed by the value to match
- `{}` contains one or more **match arms** (MatchArm)
- Each match arm consists of a **pattern**, followed by `->`, and then a **result expression**
- Each arm ends with a comma

The simplest example:

```yaoxiang
number = 2

text = match number {
    0 -> "零",
    1 -> "一",
    2 -> "二",
}
print(text)  // "二"
```

## match Is an Expression

Like `if`, `match` is also an **expression**—it computes a value. All match arms must have the same
return type:

```yaoxiang
score = 85

grade = match score {
    n if n >= 90 -> "A",
    80..89 -> "B",     // Range pattern: matches 80 through 89 (inclusive)
    n if n >= 70 -> "C",
    n if n >= 60 -> "D",
    _ -> "F",          // Wildcard: matches all remaining cases
}
print(grade)  // "B"
```

The range pattern `80..89` is a **closed interval**—both `80` and `89` match (B grade = the ten
values from 80 to 89). The endpoints must be integer literals; write negative endpoints directly
(e.g., `-5..-1`). A reversed range (e.g., `10..0`) is an empty range and is reported as unreachable
at compile time. A `match` with a range arm still needs a `_` wildcard as a fallback—the range does
not cover all possible values of `Int`. Guard expressions (`pattern if condition`) are covered in
depth in [Advanced Pattern Matching](../pattern-matching/index.md).

> **Note**: Pattern-position `lo..hi` (closed-range testing) and expression-position `a..b` (`for`
> iteration and `in` range checks, left-closed and right-open) have different endpoint semantics:
> pattern-position ranges determine a branch and do not construct a Range value.

## Basic Patterns

### Literal Patterns

Use concrete values to match:

```yaoxiang
response = 404

message = match response {
    200 -> "OK",
    301 -> "Moved",
    404 -> "Not Found",
    500 -> "Server Error",
    _ -> "Unknown",
}
print(message)  // "Not Found"
```

### Identifier Patterns

Use a variable name to capture the matched value:

```yaoxiang
// Result variant constructors must be "type-qualified": writing ok(42) without qualification reports E1001
use std.result

r: Result(Int, String) = Result(Int, String).ok(42)

description = match r {
    ok(value) -> "成功，值是: {value}",
    err(e) -> "失败，原因: " + e,
}
print(description)  // "成功，值是: 42"
```

The `value` in `ok(value)` is an identifier pattern—it captures the actual value wrapped by `ok`,
and you can use it in the expression after `->`.

### Range Patterns

Use `lo..hi` to match an integer interval (**inclusive of both endpoints**):

```yaoxiang
temp = 37

state = match temp {
    35..36 -> "偏低",
    36..38 -> "正常",
    _ -> "异常",
}
print(state)  // "正常"
```

`36..38` covers both `36` and `38`. The endpoints must be integer literals; write negative endpoints
directly (e.g., `-5..-1`). Range patterns do not exhaust the integers—they cover only a portion of
the interval—so a `match` with a range arm still requires the `_` wildcard as a fallback.

### Wildcard Patterns

`_` is the wildcard pattern and matches **any value**. It is usually placed last as a fallback:

```yaoxiang
command = "exit"

action = match command {
    "start" -> "启动服务",
    "stop" -> "停止服务",
    "restart" -> "重启服务",
    _ -> "未知指令: " + command,
}
print(action)  // "未知指令: exit"
```

## match Must Be Exhaustive

YaoXiang’s `match` requires coverage of all possible cases—if the compiler finds that you have
omitted a possible value, it reports an error immediately. This is part of what makes `match` safe.

```yaoxiang
// This code will fail to compile
// value = true
// result = match value {
//     true -> "是",
//     // Missing false branch—compile error!
// }

// Correct—use _ as a fallback
value = true
result = match value {
    true -> "是",
    _ -> "否",      // _ ensures false is handled as well
}
```

When you clearly know that only a finite number of cases are possible (for example, when matching an
enum), the compiler checks whether every variant has been covered. This is an effective safeguard
against bugs caused by missing branches.

## Combining Multiple Patterns

A single match arm can match multiple patterns, separated by `|`:

```yaoxiang
day = "sunday"

type = match day {
    "monday" | "tuesday" | "wednesday" | "thursday" | "friday" -> "工作日",
    "saturday" | "sunday" -> "休息日",
    _ -> "无效",
}
print(type)  // "休息日"
```

## Match Arms Execute in Order

`match` starts trying from the first arm, and **the first branch that matches successfully takes
effect**; later arms are not executed:

<!-- docs-example: skip -->

```yaoxiang
number = 5

result = match number {
    _ -> "其他",     // The wildcard matches everything, so this arm matches
    5 -> "五",       // This is never executed—the preceding arm already matched
}
print(result)  // "其他"
```

> The example above **deliberately fails the check**—the compiler reports
> `E1031 Unreachable pattern`. It uses a real diagnostic to demonstrate the rule that **the first
> matching arm takes effect**.

This feature means it is good practice to **place the `_` wildcard last**.

## Summary

| Key point            | Description                                                          |
| -------------------- | -------------------------------------------------------------------- |
| Syntax               | `match value { pattern -> expression, ... }`                         |
| Expression           | `match` computes a value; all branches have the same type            |
| Literal pattern      | Matches an exact value: `200 -> "OK"`                                |
| Identifier pattern   | Captures a value in a variable: `ok(value) -> ...`                   |
| Range pattern        | Matches an integer interval (inclusive): `80..89 -> "B"`             |
| Wildcard `_`         | Matches any value as a fallback                                      |
| Exhaustiveness       | Must cover all possible cases; the compiler checks this              |
| Multiple patterns    | `pattern1 \| pattern2 -> expression`                                 |
| Sequential execution | Evaluates from top to bottom; the first matching branch takes effect |

> **Next**: This article covers the basics of `match`. For more advanced patterns (nested patterns,
> guard expressions, struct destructuring, etc.), see
> [Advanced Pattern Matching](../pattern-matching/index.md).
