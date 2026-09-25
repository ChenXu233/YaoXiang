---
title: 'match Basics'
---

# match Basics

`match` is YaoXiang's most powerful control flow construct. It lets you choose different handling
paths based on the **shape** of a value. If you've used `switch` in other languages, you'll find
that `match` is a comprehensive upgrade.

## Basic Syntax

The definition of the `match` expression in the syntax specification:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

Breaking it down:

- `match` is followed by a value to match against
- `{}` contains one or more **match arms** (MatchArm)
- Each match arm: **pattern** followed by `=>`, then a **result expression**
- Each arm ends with a comma

A simple example:

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
    90..100 => "A",    // range pattern (advanced topic)
    80..89 => "B",
    70..79 => "C",
    60..69 => "D",
    _ => "F",          // wildcard: matches all remaining cases
}
print(grade)  // "B"
```

> **Note**: Range patterns like `90..100` are advanced topics and will be covered in depth in
> [Pattern Matching Advanced](../pattern-matching/index.md). This chapter focuses on basic patterns
> first.

## Basic Patterns

### Literal Patterns

Match against a specific value:

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
result: Result(Int, String) = ok(42)

description = match result {
    ok(value) => "成功，值是: " + value.to_string(),
    err(error) => "失败，原因: " + error,
}
print(description)  // "成功，值是: 42"
```

The `value` in `ok(value)` is an identifier pattern—it captures the actual value wrapped by `ok`,
and you can use it in the expression after `=>`.

### Wildcard Pattern

`_` is the wildcard, matching **any value**. It's typically placed at the end as a fallback:

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

YaoXiang's `match` requires covering all possible cases—if the compiler finds that you've missed
certain possible values, it will report an error directly. This reflects the safety of `match`.

```yaoxiang
// This code will fail to compile
// value = true
// result = match value {
//     true => "是",
//     // Missing false branch—compile error!
// }

// Correct—use _ as fallback
value = true
result = match value {
    true => "是",
    _ => "否",      // _ ensures false is also handled
}
```

When you know there are only a limited number of cases (for example, matching an enum), the compiler
will help you check whether every variant is covered. This is a powerful tool for preventing bugs
from missing branches.

## Combining Multiple Patterns

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

`match` starts trying to match from the first arm; the **first branch that successfully matches
takes effect**, and the ones after it will not be executed:

```yaoxiang
number = 5

result = match number {
    _ => "其他",     // wildcard matches everything, this will match
    5 => "五",       // will never be executed—already matched above
}
print(result)  // "其他"
```

This feature means that **placing the wildcard `_` at the end** is a good habit.

## Summary

| Key Point          | Description                                             |
| ------------------ | ------------------------------------------------------- |
| Syntax             | `match value { pattern => expression, ... }`            |
| Expression         | `match` evaluates to a value, all branches share a type |
| Literal pattern    | Match an exact value: `200 => "OK"`                     |
| Identifier pattern | Capture a value into a variable: `ok(value) => ...`     |
| Wildcard `_`       | Match any value, used as a fallback                     |
| Exhaustiveness     | Must cover all possibilities; the compiler will check   |
| Multiple patterns  | `pattern1 \| pattern2 => expression`                    |
| Order of execution | Top to bottom, the first matching branch takes effect   |

> **Next**: This article covers the basic usage of `match`. For more advanced patterns (nested
> patterns, guard expressions, struct destructuring, etc.), please refer to
> [Pattern Matching Advanced](../pattern-matching/index.md).
