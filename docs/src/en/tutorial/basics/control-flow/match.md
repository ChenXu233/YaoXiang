---
title: 'match Basics'
---

# match Basics

`match` is one of YaoXiang's most powerful control-flow structures. It lets you select different
handling paths based on the shape of a **value**. If you have used `switch` in other languages,
you'll find that `match` is a comprehensive upgrade.

## Basic Syntax

The specification defines the `match` expression as follows:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '->' Expr ','
```

Broken down:

- A value to match comes after `match`
- `{}` contains one or more **match arms** (MatchArm)
- Each match arm consists of a **pattern** followed by `->`, and then a **result expression**
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

Like `if`, `match` is also an **expression**—it computes a value. The return types of all match arms
must be identical:

```yaoxiang
score = 85

grade = match score {
    n if n >= 90 -> "A",
    80..89 -> "B",     // Range pattern: both 80 and 89 match
    n if n >= 70 -> "C",
    n if n >= 60 -> "D",
    _ -> "F",          // Wildcard: matches all remaining cases
}
print(grade)  // "B"
```

The range pattern `80..89` is a **closed interval**—both `80` and `89` match (the B grade covers the
ten values from 80 to 89). The endpoints must be integer literals. Write negative endpoints
directly, such as `-5..-1`. A reversed range, such as `10..0`, is an empty range and produces an
unreachable-code error at compile time. A `match` containing a range arm still needs the wildcard
`_` as a fallback, because ranges do not cover every possible `Int` value. Guard expressions
(`pattern if condition`) are covered in detail in
[Advanced Pattern Matching](../pattern-matching/index.md).

> **Note**: The endpoints of `lo..hi` in pattern position (a closed-interval check) have different
> semantics from those of `a..b` in expression position (`for` iteration and `in` range checks,
> which are half-open). Pattern-position ranges determine branches and do not construct `Range`
> values.

## Basic Patterns

### Literal Patterns

Match against a specific value:

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
// Result variant constructors must be type-qualified: writing bare ok(42) reports E1001
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

Use `lo..hi` to match an integer interval (**including both endpoints**):

```yaoxiang
temp = 37

state = match temp {
    35..36 -> "偏低",
    36..38 -> "正常",
    _ -> "异常",
}
print(state)  // "正常"
```

`36..38` covers both `36` and `38`. The endpoints must be integer literals. Write negative endpoints
directly, such as `-5..-1`. Range patterns do not exhaust the integer type—they cover only a segment
of the possible values—so a `match` containing a range arm still requires the wildcard `_` as a
fallback.

### Wildcard Patterns

`_` is the wildcard and matches **any value**. It is usually placed last as a fallback:

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

## Matches Must Be Exhaustive

YaoXiang's `match` requires all possible cases to be covered. If the compiler finds that you have
omitted a possible value, it reports an error directly. This is one of the safety guarantees
provided by `match`.

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
    _ -> "否",      // _ ensures false is also handled
}
```

When you know that only a finite number of cases is possible—for example, when matching an enum—the
compiler checks whether every variant has been covered. This is an effective way to prevent
missing-branch bugs.

## Multiple Pattern Combinations

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

`match` starts with the first arm and tries to match it. The **first successfully matching branch
takes effect**, and later arms are not executed:

<!-- docs-example: skip -->

```yaoxiang
number = 5

result = match number {
    _ -> "其他",     // The wildcard matches everything, so it matches here
    5 -> "五",       // This will never execute—the arm above has already matched
}
print(result)  // "其他"
```

> The example above **intentionally fails validation**—the compiler reports
> `E1031 Unreachable pattern`. It uses an actual diagnostic to demonstrate the rule that the first
> matching arm takes effect.

This behavior means that **placing the wildcard `_` last** is good practice.

## Summary

| Key                  | Description                                              |
| -------------------- | -------------------------------------------------------- |
| Syntax               | `match value { pattern -> expression, ... }`             |
| Expression           | `match` computes a value; all branch types must match    |
| Literal pattern      | Matches an exact value: `200 -> "OK"`                    |
| Identifier pattern   | Captures a value in a variable: `ok(value) -> ...`       |
| Range pattern        | Matches an integer interval inclusively: `80..89 -> "B"` |
| Wildcard `_`         | Matches any value as a fallback                          |
| Exhaustiveness       | Must cover all possibilities; the compiler checks it     |
| Multiple patterns    | `pattern1 \| pattern2 -> expression`                     |
| Sequential execution | Runs from top to bottom; the first matching arm wins     |

> **Next**: This article covers the basic usage of `match`. For more advanced patterns, such as
> nested patterns, guard expressions, and struct destructuring, see
> [Advanced Pattern Matching](../pattern-matching/index.md).
