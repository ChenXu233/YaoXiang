---
title: 'match Basics'
---

# match Basics

`match` is YaoXiang's most powerful control flow construct. It lets you select different processing paths based on the **shape** of a value. If you've used `switch` in other languages, you'll find `match` is a comprehensive upgrade.

## Basic Syntax

The definition of a `match` expression in the grammar specification:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

Breaking it down:

- `match` is followed by a value to match
- `{}` contains one or more **match arms** (MatchArm)
- Each match arm: a **pattern** followed by `=>`, then a **result expression**
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

## match is an Expression

Like `if`, `match` is also an **expression** — it computes a value. All match arms must return the same type:

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

> **Note**: Range patterns like `90..100` are advanced topics covered in depth in [Advanced Pattern Matching](../pattern-matching/index.md). This chapter focuses on basic patterns.

## Basic Patterns

### Literal Patterns

Match with concrete values:

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

Use variable names to capture matched values:

```yaoxiang
result: Result(Int, String) = ok(42)

description = match result {
    ok(value) => "Success, value is: " + value.to_string(),
    err(error) => "Failed, reason: " + error,
}
print(description)  // "Success, value is: 42"
```

The `value` in `ok(value)` is an identifier pattern — it captures the actual value wrapped by `ok`, which you can use in the expression after `=>`.

### Wildcard Patterns

`_` is a wildcard that matches **any value**. It's typically placed at the end as a fallback:

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

## Matching Must Be Exhaustive

YaoXiang's `match` requires covering all possible cases — if the compiler finds you've missed some possible values, it will error directly. This is an expression of `match`'s safety.

```yaoxiang
// This code will fail to compile
// value = true
// result = match value {
//     true => "Yes",
//     // Missing false branch — compile error!
// }

// Correct — use _ as fallback
value = true
result = match value {
    true => "Yes",
    _ => "No",         // _ ensures false is also handled
}
```

When you know there are only a limited number of cases (like matching an enum), the compiler will help you check if every variant is covered. This is a powerful tool for preventing missing branch bugs.

## Multiple Pattern Combination

A single match arm can match multiple patterns, separated by `|`:

```yaoxiang
day = "sunday"

type = match day {
    "monday" | "tuesday" | "wednesday" | "thursday" | "friday" => "Weekday",
    "saturday" | "sunday" => "Weekend",
    _ => "Invalid",
}
print(type)  // "Weekend"
```

## Match Arms Execute in Order

`match` starts trying to match from the first arm, and **the first successfully matched branch takes effect** — subsequent arms won't be executed:

```yaoxiang
number = 5

result = match number {
    _ => "Other",      // Wildcard matches everything, this will match
    5 => "Five",       // Never executed — already matched above
}
print(result)  // "Other"
```

This feature means **putting the wildcard `_` at the end** is a good habit.

## Summary

| Key Point     | Description                                               |
| ------------- | --------------------------------------------------------- |
| Syntax        | `match value { pattern => expression, ... }`              |
| Expression    | `match` computes a value, all branches have same type    |
| Literal       | Exact match of concrete values: `200 => "OK"`            |
| Identifier    | Capture value to variable: `ok(value) => ...`           |
| Wildcard `_`  | Matches any value, serves as fallback                    |
| Exhaustive    | Must cover all possibilities, compiler checks            |
| Multiple      | `pattern1 \| pattern2 => expression`                     |
| Sequential    | Top to bottom, first matched branch takes effect         |

> **Next Step**: This article covers the basics of `match`. For more advanced patterns (nested patterns, guard expressions, struct destructuring, etc.), see [Advanced Pattern Matching](../pattern-matching/index.md).