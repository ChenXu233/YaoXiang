---
title: match Basics
---

# match Basics

`match` is YaoXiang's most powerful control flow structure. It lets you choose different processing paths based on the **shape** of a value. If you've used `switch` in other languages, think of `match` as a comprehensive upgrade.

## Basic Syntax

The `match` expression in the language specification:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

Breaking it down:

- `match` followed by the value to match against
- `{}` contains one or more **match arms**
- Each arm: a **pattern** followed by `=>`, then a **result expression**
- Each arm ends with a comma

A minimal example:

```yaoxiang
number = 2

text = match number {
    0 => "zero",
    1 => "one",
    2 => "two",
}
println(text)  # "two"
```

## match Is an Expression

Like `if`, `match` is an **expression** — it computes a value. All arms must return the same type:

```yaoxiang
score = 85

grade = match score {
    90..100 => "A",    # Range pattern (advanced topic)
    80..89 => "B",
    70..79 => "C",
    60..69 => "D",
    _ => "F",          # Wildcard: matches everything else
}
println(grade)  # "B"
```

> **Note**: Range patterns like `90..100` are an advanced topic covered in [Advanced Pattern Matching](../pattern-matching.md). This chapter focuses on the basic patterns.

## Basic Patterns

### Literal Patterns

Match against concrete values:

```yaoxiang
response = 404

message = match response {
    200 => "OK",
    301 => "Moved",
    404 => "Not Found",
    500 => "Server Error",
    _ => "Unknown",
}
println(message)  # "Not Found"
```

### Identifier Patterns

Use a variable name to capture the matched value:

```yaoxiang
result: Result(Int, String) = ok(42)

description = match result {
    ok(value) => "Success, value: " + value.to_string(),
    err(error) => "Failure, reason: " + error,
}
println(description)  # "Success, value: 42"
```

The `value` in `ok(value)` is an identifier pattern — it captures the actual value wrapped by `ok`, which you can then use in the expression after `=>`.

### Wildcard Pattern

`_` is the wildcard — it matches **any value**. It is typically placed last as a catch-all:

```yaoxiang
command = "exit"

action = match command {
    "start" => "Starting service",
    "stop" => "Stopping service",
    "restart" => "Restarting service",
    _ => "Unknown command: " + command,
}
println(action)  # "Unknown command: exit"
```

## Matching Must Be Exhaustive

YaoXiang's `match` requires covering all possible cases — if the compiler finds you've missed a possible value, it will report an error. This is a manifestation of `match`'s safety guarantees.

```yaoxiang
# This code will fail to compile
# value = true
# result = match value {
#     true => "yes",
#     # Missing the false arm — compile error!
# }

# Correct — use _ as a catch-all
value = true
result = match value {
    true => "yes",
    _ => "no",       # _ ensures false is also handled
}
```

When you know there are only a finite set of cases (e.g., matching an enum), the compiler checks that every variant is covered. This catches missing-branch bugs.

## Multiple Patterns in One Arm

A single match arm can accept multiple patterns separated by `|`:

```yaoxiang
day = "sunday"

type = match day {
    "monday" | "tuesday" | "wednesday" | "thursday" | "friday" => "weekday",
    "saturday" | "sunday" => "weekend",
    _ => "invalid",
}
println(type)  # "weekend"
```

## Match Arms Execute in Order

`match` tries arms from top to bottom — **the first arm that matches wins**, and later arms are not checked:

```yaoxiang
number = 5

result = match number {
    _ => "other",     # Wildcard matches everything — this arm matches
    5 => "five",      # Never reached — already matched above
}
println(result)  # "other"
```

This means keeping the wildcard `_` at the end is a good practice.

## Summary

| Point | Description |
|-------|-------------|
| Syntax | `match value { pattern => expression, ... }` |
| Expression | `match` computes a value; all arms must have the same type |
| Literal patterns | Exact match: `200 => "OK"` |
| Identifier patterns | Capture value into variable: `ok(value) => ...` |
| Wildcard `_` | Matches anything, used as fallback |
| Exhaustiveness | Must cover all cases; compiler enforces this |
| Multiple patterns | `pattern1 \| pattern2 => expression` |
| Execution order | Top to bottom; first match wins |

> **Next steps**: This chapter covered the basic usage of `match`. For more advanced patterns (nested patterns, guard expressions, struct destructuring, etc.), see [Advanced Pattern Matching](../pattern-matching.md).
