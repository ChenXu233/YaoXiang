---
title: match Basics
---

# match Basics

`match`
is YaoXiang's most powerful control flow construct. It lets you choose different execution paths based on the **shape** of a value. If you've used `switch`
in other languages, you'll find `match` is a fully upgraded version.

## Basic Syntax

Definition of the `match` expression in the syntax specification:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

Breaking it down:

- `match` is followed by a value to match
- `{}` contains one or more **match arms**
- Each match arm: **pattern** followed by `=>`, then the **result expression**
- Each arm ends with a comma

The simplest example:

```yaoxiang
number = 2

text = match number {
    0 => "零",
    1 => "一",
    2 => "二",
}
print(text)  // "二"
```

## match is an Expression

Like `if`, `match` is also an **expression**—it computes a value. All match arms must return the same type:

```yaoxiang
score = 85

grade = match score {
    90..100 => "A",    // range pattern (advanced topic)
    80..89 => "B",
    70..79 => "C",
    60..69 => "D",
    _ => "F",          // wildcard: matches all remaining cases
}
print(grade)  // "B"
```

> **Note**: Range patterns like `90..100` are advanced topics, covered in depth in [Advanced Pattern Matching](../pattern-matching.md). This chapter focuses on basic patterns.

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

Use variable names to capture matched values:

```yaoxiang
result: Result(Int, String) = ok(42)

description = match result {
    ok(value) => "成功，值是: " + value.to_string(),
    err(error) => "失败，原因: " + error,
}
print(description)  // "成功，值是: 42"
```

In `ok(value)`, `value` is an identifier pattern—it captures the actual value wrapped in `ok`, which you can use in the expression after `=>`.

### Wildcard Patterns

`_` is a wildcard that matches **any value**. Usually placed at the end as a fallback:

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

## Matching Must Be Exhaustive

YaoXiang's `match` requires covering all possible cases—if the compiler finds you've missed some possible values, it will error directly. This is an expression of `match`'s safety.

```yaoxiang
// This code will fail to compile
// value = true
// result = match value {
//     true => "是",
//     // missing false branch—compile error!
// }

// Correct—use _ as fallback
value = true
result = match value {
    true => "是",
    _ => "否",      // _ ensures false is also handled
}
```

When you explicitly know there are only limited cases (like matching an enum), the compiler helps you check whether every variant is covered. This is a powerful tool for preventing missing branch bugs.

## Multiple Pattern Combination

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

`match` tries to match from the first arm—the **first successful match takes effect**, and subsequent arms won't be executed:

```yaoxiang
number = 5

result = match number {
    _ => "其他",     // wildcard matches everything, this will match
    5 => "五",       // never executed—already matched above
}
print(result)  // "其他"
```

This characteristic means **putting the wildcard `_` last** is a good practice.

## Summary

| Key Point       | Description                                             |
| --------------- | ------------------------------------------------------ |
| Syntax          | `match value { pattern => expression, ... }`           |
| Expression      | `match` computes a value, all branches must have same type |
| Literal Pattern | Exact match: `200 => "OK"`                             |
| Identifier Pattern | Capture value to variable: `ok(value) => ...`       |
| Wildcard `_`    | Matches any value, used as fallback                    |
| Exhaustiveness  | Must cover all possibilities, compiler checks          |
| Multiple Patterns | `pattern1 \| pattern2 => expression`                |
| Sequential Execution | From top to bottom, first matching branch takes effect |

> **Next Step**: This article covers `match` basics. For more advanced patterns (nested patterns, guard expressions, struct destructuring, etc.), see [Advanced Pattern Matching](../pattern-matching.md).
