---
title: 'match Basics'
---

# match Basics

`match` is YaoXiang's most powerful control flow structure. It lets you choose different handling
paths based on the form of a **value**. If you've used `switch` in other languages, you'll find that
`match` is a comprehensively upgraded version.

## Basic Syntax

The definition of the `match` expression in the language specification:

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

Breaking it down:

- After `match` comes the value to match against
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
print(text)  // "二"
```

## match is an Expression

Just like `if`, `match` is also an **expression**—it evaluates to a value. All match arms must
return the same type:

```yaoxiang
score = 85

grade = match score {
    90..100 => "A",    // 范围模式（进阶内容）
    80..89 => "B",
    70..79 => "C",
    60..69 => "D",
    _ => "F",          // 通配符：匹配所有剩余情况
}
print(grade)  // "B"
```

> **Note**: Range patterns like `90..100` are advanced content and will be covered in detail in
> [Advanced Pattern Matching](../pattern-matching.md). This chapter focuses on basic patterns first.

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
which you can use in the expression after `=>`.

### Wildcard Pattern

`_` is a wildcard that matches **any value**. It's usually placed last as a catch-all:

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

YaoXiang's `match` requires you to cover all possible cases—if the compiler finds that you've missed
some possible values, it will error directly. This reflects the safety of `match`.

```yaoxiang
// 这段代码会编译失败
// value = true
// result = match value {
//     true => "是",
//     // 缺少 false 分支——编译错误！
// }

// 正确——使用 _ 兜底
value = true
result = match value {
    true => "是",
    _ => "否",      // _ 确保 false 也有处理
}
```

When you know there are only a limited number of cases (e.g., matching an enum), the compiler checks
whether every variant is covered. This is a powerful tool for preventing bugs caused by missing
branches.

## Multi-Pattern Combinations

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

`match` starts trying to match from the first arm—the **first branch that successfully matches takes
effect**, and the following ones are not executed:

```yaoxiang
number = 5

result = match number {
    _ => "其他",     // 通配符匹配一切，这里会匹配
    5 => "五",       // 永远不会被执行——上面已经匹配了
}
print(result)  // "其他"
```

This feature means it's a good practice to place the wildcard `_` last.

## Summary

| Point               | Description                                                   |
| ------------------- | ------------------------------------------------------------- |
| Syntax              | `match value { pattern => expression, ... }`                  |
| Expression          | `match` evaluates to a value; all branches have the same type |
| Literal Patterns    | Match exact values: `200 => "OK"`                             |
| Identifier Patterns | Capture value into a variable: `ok(value) => ...`             |
| Wildcard `_`        | Matches any value, used as a catch-all                        |
| Exhaustiveness      | Must cover all cases; the compiler checks this                |
| Multi-Pattern       | `pattern1 \| pattern2 => expression`                          |
| Ordered Execution   | From top to bottom, the first matching branch takes effect    |

> **Next**: This article covers the basic usage of `match`. For more advanced patterns (nested
> patterns, guards, struct destructuring, etc.), see
> [Advanced Pattern Matching](../pattern-matching.md).
