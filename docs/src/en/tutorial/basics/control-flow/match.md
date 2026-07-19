---
title: match Basics
---

# match Basics

`match` is the most powerful control flow structure in YaoXiang. It lets you choose different handling paths based on the **shape** of a **value**. If you've used `switch` in other languages, you'll find that `match` is a comprehensive upgrade.

## Basic Syntax

The `match` expression as defined in the syntax specification:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
`

Breaking it down:

- After `match` comes the value to be matched
- `{}` contains one or more **match arms** (MatchArm)
- Each match arm: **pattern** followed by `=>`, then a **result expression**
- Each arm ends with a comma

A simple example:

```yaoxiang
number = 2

text = match number {
    0 => "zero",
    1 => "one",
    2 => "two",
}
print(text)  // "two"
```

## match Is an Expression

Like `if`, `match` is also an **expression**—it evaluates to a value. All match arm return values must be of the same type:

```yaoxiang
score = 85

grade = match score {
    90..100 => "A",    // Range pattern (advanced topic)
    80..89 => "B",
    70..79 => "C",
    60..69 => "D",
    _ => "F",          // Wildcard: matches all remaining cases
}
print(grade)  // "B"
```

> **Note**: Range patterns like `90..100` are advanced topics and will be covered in detail in [Pattern Matching Advanced](../pattern-matching.md). This chapter focuses on basic patterns first.

## Basic Patterns

### Literal Pattern

Match against a concrete value:

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

### Identifier Pattern

Use a variable name to capture the matched value:

```yaoxiang
result: Result(Int, String) = ok(42)

description = match result {
    ok(value) => "Success, value is: " + value.to_string(),
    err(error) => "Failed, reason: " + error,
}
print(description)  // "Success, value is: 42"
```

The `value` in `ok(value)` is an identifier pattern—it captures the actual value wrapped by `ok`, and you can use it in the expression after `=>`.

### Wildcard Pattern

`_` is a wildcard, matching **any value**. It's typically placed last as a catch-all:

```yaoxiang
command = "exit"

action = match command {
    "start" => "Start service",
    "stop" => "Stop service",
    "restart" => "Restart service",
    _ => "Unknown command: " + command,
}
print(action)  // "Unknown command: exit"
```

## Matches Must Be Exhaustive

YaoXiang's `match` requires all possible cases to be covered—if the compiler finds that you've missed some possible values, it will report an error directly. This reflects the safety guarantees of `match`.

```yaoxiang
// This code will fail to compile
// value = true
// result = match value {
//     true => "yes",
//     // Missing false branch—compile error!
// }

// Correct—use _ as catch-all
value = true
result = match value {
    true => "yes",
    _ => "no",      // _ ensures false is also handled
}
```

When you know for certain that there are only a finite number of cases (such as matching an enum), the compiler will help you check whether every variant is covered. This is a powerful tool to prevent bugs from missing branches.

## Combining Multiple Patterns

A single match arm can match multiple patterns, separated by `|`:

```yaoxiang
day = "sunday"

type = match day {
    "monday" | "tuesday" | "wednesday" | "thursday" | "friday" => "weekday",
    "saturday" | "sunday" => "weekend",
    _ => "invalid",
}
print(type)  // "weekend"
```

## Match Arms Execute in Order

`match` starts trying to match from the first arm, and **the first branch that successfully matches takes effect**; subsequent branches will not be executed:

```yaoxiang
number = 5

result = match number {
    _ => "other",     // Wildcard matches everything, this will match
    5 => "five",      // Will never be executed—already matched above
}
print(result)  // "other"
```

This feature means it's a good habit to **place the wildcard `_` last**.

## Summary

| Key Point | Description |
|------|------|
| Syntax | `match value { pattern => expression, ... }` |
| Expression | `match` evaluates to a value; all branches must have the same type |
| Literal pattern | Precisely match concrete values: `200 => "OK"` |
| Identifier pattern | Capture value into a variable: `ok(value) => ...` |
| Wildcard `_` | Matches any value, used as catch-all |
| Exhaustiveness | All possible cases must be covered; the compiler checks |
| Multiple patterns | `pattern1 \| pattern2 => expression` |
| Order of execution | From top to bottom, the first matching branch takes effect |

> **Next**: This article covers the basic usage of `match`. For more advanced patterns (nested patterns, guard expressions, struct destructuring, etc.), please refer to [Pattern Matching Advanced](../pattern-matching.md).