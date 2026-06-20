---
title: match Basics
---

# match Basics

`match` is the most powerful control flow structure in YaoXiang. It lets you choose different handling paths based on the **shape** of a value. If you've used `switch` in other languages, you'll find that `match` is a comprehensive upgrade.

## Basic Syntax

The definition of a `match` expression in the grammar specification:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
`

Breaking it down:

- `match` is followed by a value to match against
- `{}` contains one or more **match arms** (MatchArm)
- Each match arm: a **pattern** followed by `=>`, then a **result expression**
- Each arm ends with a comma

A simple example:

```yaoxiang
number = 2

text = match number {
    0 => "零",
    1 => "一",
    2 => "二",
}
println(text)  # "二"
```

## match Is an Expression

Like `if`, `match` is also an **expression**—it evaluates to a value. The return types of all match arms must be consistent:

```yaoxiang
score = 85

grade = match score {
    90..100 => "A",    # Range pattern (advanced topic)
    80..89 => "B",
    70..79 => "C",
    60..69 => "D",
    _ => "F",          # Wildcard: matches all remaining cases
}
println(grade)  # "B"
```

> **Note**: Range patterns like `90..100` are advanced content, and will be covered in depth in [Pattern Matching Advanced](../pattern-matching.md). This chapter first focuses on basic patterns.

## Basic Patterns

### Literal Pattern

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
println(message)  # "Not Found"
```

### Identifier Pattern

Use a variable name to capture the matched value:

```yaoxiang
result: Result(Int, String) = ok(42)

description = match result {
    ok(value) => "成功，值是: " + value.to_string(),
    err(error) => "失败，原因: " + error,
}
println(description)  # "成功，值是: 42"
```

The `value` in `ok(value)` is an identifier pattern—it captures the actual value wrapped by `ok`, which you can use in the expression after `=>`.

### Wildcard Pattern

`_` is a wildcard that matches **any value**. It's usually placed at the end as a catch-all:

```yaoxiang
command = "exit"

action = match command {
    "start" => "启动服务",
    "stop" => "停止服务",
    "restart" => "重启服务",
    _ => "未知指令: " + command,
}
println(action)  # "未知指令: exit"
```

## Matching Must Be Exhaustive

YaoXiang's `match` requires you to cover all possible cases—if the compiler finds that you've missed some possible values, it will report an error directly. This reflects the safety of `match`.

```yaoxiang
# This code will fail to compile
# value = true
# result = match value {
#     true => "是",
#     # Missing the false branch—compile error!
# }

# Correct—use _ as catch-all
value = true
result = match value {
    true => "是",
    _ => "否",      # _ ensures false is also handled
}
```

When you know for certain that there are only a finite number of cases (for example, matching an enum), the compiler will help you check whether every variant is covered. This is a powerful tool for preventing bugs caused by missing branches.

## Multiple Pattern Combination

A single match arm can match multiple patterns, separated by `|`:

```yaoxiang
day = "sunday"

type = match day {
    "monday" | "tuesday" | "wednesday" | "thursday" | "friday" => "工作日",
    "saturday" | "sunday" => "休息日",
    _ => "无效",
}
println(type)  # "休息日"
```

## Match Arms Execute in Order

`match` tries to match starting from the first arm—**the first successfully matched branch takes effect**, and the ones after it are not executed:

```yaoxiang
number = 5

result = match number {
    _ => "其他",     # Wildcard matches everything, matches here
    5 => "五",       # Never gets executed—the one above already matched
}
println(result)  # "其他"
```

This feature means that **placing the wildcard `_` at the end** is a good habit.

## Summary

| Key Point | Description |
|------|------|
| Syntax | `match value { pattern => expression, ... }` |
| Expression | `match` evaluates to a value; all branches have consistent types |
| Literal Pattern | Exactly match a specific value: `200 => "OK"` |
| Identifier Pattern | Capture value into a variable: `ok(value) => ...` |
| Wildcard `_` | Match any value, serves as catch-all |
| Exhaustiveness | Must cover all possibilities; compiler checks |
| Multiple Patterns | `pattern1 \| pattern2 => expression` |
| Sequential Execution | Top to bottom, the first matching branch takes effect |

> **Next**: This article covers the basic usage of `match`. For more advanced patterns (nested patterns, guard expressions, struct destructuring, etc.), please refer to [Pattern Matching Advanced](../pattern-matching.md).