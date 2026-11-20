---
title: 'match Basics'
---

# match Basics

`match` is the most powerful control flow structure in YaoXiang. It lets you choose different
handling paths based on the **shape** of a value. If you've used `switch` in other languages, you'll
find `match` to be a comprehensive upgrade.

## Basic Syntax

The definition of a `match` expression in the language specification:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

Breaking it down:

- `match` is followed by the value to match on
- `{}` contains one or more **match arms** (MatchArm)
- Each match arm: a **pattern** followed by `=>`, then the **result expression**
- Each arm ends with a comma

A minimal example:

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

Like `if`, `match` is also an **expression**—it evaluates to a value. The return types of all match
arms must be consistent:

```yaoxiang
score = 85

grade = match score {
    n if n >= 90 => "A",
    80..89 => "B",     // Range pattern: 80 through 89 (both ends included) hits
    n if n >= 70 => "C",
    n if n >= 60 => "D",
    _ => "F",          // Wildcard: matches all remaining cases
}
print(grade)  // "B"
```

The range pattern `80..89` is a **closed interval**—both `80` and `89` hit (grade B covers the ten
values 80-89). Bounds must be integer literals; negative bounds are written directly (such as
`-5..-1`). A reversed range (such as `10..0`) is an empty range and reports unreachable at compile
time. A `match` with range arms still needs the wildcard `_` as a fallback—a range cannot cover all
integer values. Guard expressions (`pattern if condition`) are covered in detail in
[Pattern Matching Advanced](../pattern-matching/index.md).

> **Note**: the pattern-position `lo..hi` (closed-interval test) and the expression-position `a..b`
> (`for` iteration, `in` membership checks, half-open) have different endpoint semantics: the
> pattern position tests branches and does not construct a Range value.

## Basic Patterns

### Literal Patterns

Match against specific values:

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

Capture the matched value into a variable name:

```yaoxiang
// Result variant constructors must be "type-qualified": writing bare ok(42) reports E1001
use std.result

r: Result(Int, String) = Result(Int, String).ok(42)

description = match r {
    ok(value) => "成功，值是: {value}",
    err(e) => "失败，原因: " + e,
}
print(description)  // "成功，值是: 42"
```

The `value` in `ok(value)` is an identifier pattern—it captures the actual value wrapped by `ok`,
and you can use it in the expression after `=>`.

### Range Patterns

Match an integer interval with `lo..hi` (**both ends included**):

```yaoxiang
temp = 37

state = match temp {
    35..36 => "偏低",
    36..38 => "正常",
    _ => "异常",
}
print(state)  // "正常"
```

`36..38` covers both `36` and `38`. Bounds must be integer literals; negative bounds are written
directly (such as `-5..-1`). A range pattern cannot exhaust the integers—it covers only the span
inside the interval, so a `match` with range arms still needs the wildcard `_` as a fallback.

### Wildcard Pattern

`_` is the wildcard, matching **any value**. It's typically placed last as a fallback:

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

## Match Must Be Exhaustive

YaoXiang's `match` requires you to cover every possible case—if the compiler finds that you've
missed some possible values, it errors out directly. This reflects the safety guarantees of `match`.

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

When you know there are only a finite number of cases (e.g., when matching an enum), the compiler
will check whether every variant is covered. This is a powerful tool for preventing bugs from
missing branches.

## Multiple Patterns

A single match arm can match multiple patterns, separated by `|`:

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

`match` tries arms starting from the first one; **the first matching branch takes effect**, and the
ones after it are not executed:

<!-- docs-example: skip -->

```yaoxiang
number = 5

result = match number {
    _ => "其他",     // Wildcard matches everything; it will match here
    5 => "五",       // Will never be executed—the one above already matched
}
print(result)  // "其他"
```

> The code above is **intentionally failing the check**—the compiler will report
> `E1031 Unreachable pattern`. It uses a real diagnostic to demonstrate the rule "the first matching
> arm wins".

This means that **placing the wildcard `_` last** is a good habit.

## Summary

| Key Point          | Description                                                       |
| ------------------ | ----------------------------------------------------------------- |
| Syntax             | `match value { pattern => expr, ... }`                            |
| Expression         | `match` evaluates to a value; all branches have the same type     |
| Literal pattern    | Matches a specific value: `200 => "OK"`                           |
| Identifier pattern | Captures a value into a variable: `ok(value) => ...`              |
| Range pattern      | Matches an integer interval (both ends included): `80..89 => "B"` |
| Wildcard `_`       | Matches any value, used as a fallback                             |
| Exhaustive         | Must cover all possibilities; the compiler checks                 |
| Multiple patterns  | `pattern1 \| pattern2 => expr`                                    |
| Order of execution | From top to bottom, the first matching branch wins                |

> **Next**: This article covers the basics of `match`. For more advanced patterns (nested patterns,
> guard expressions, struct destructuring, etc.), see
> [Pattern Matching Advanced](../pattern-matching/index.md).
