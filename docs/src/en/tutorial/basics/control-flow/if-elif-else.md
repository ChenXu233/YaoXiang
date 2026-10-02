---
title: 'if-else-if-else'
---

# if-else-if-else

`if-else-if-else` is the most fundamental decision-making tool in programming. Its logic is very
straightforward—**if a condition holds, execute some code; otherwise, check the next condition; if
none hold, take the default path**.

## Basic Syntax

In the syntax specification, the definitions of an `if` expression and an `if` statement are
identical:

```
if Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

Translated into everyday language: it starts with `if`, followed by a condition expression and a
code block, then optionally zero or more `else if condition code block` clauses, and finally an
optional `else code block`.

The simplest form—just `if`:

<!-- docs-example: skip -->

```yaoxiang
if temperature > 30 {
    print("天热了，开空调吧")
}
```

Add `else`:

<!-- docs-example: skip -->

```yaoxiang
if is_raining {
    print("带伞")
} else {
    print("不用带伞")
}
```

Multiple conditions with `else if`:

```yaoxiang
score = 85

if score >= 90 {
    print("优秀")
} else if score >= 80 {
    print("良好")
} else if score >= 60 {
    print("及格")
} else {
    print("需要努力")
}
```

## `if` as an Expression

This is one of the most important features of YaoXiang's control flow: **`if` can be used as an
expression that computes a value**.

```yaoxiang
// if expression: the value of each branch is assigned to result
x = 42

result = if x > 0 {
    "正数"
} else if x < 0 {
    "负数"
} else {
    "零"
}
// result is now one of "正数", "负数", or "零"
```

When `if` is used as an expression, the return types of all branches must be the same:

```yaoxiang
score = 88

// all branches return String — types match, no problem
grade = if score >= 90 {
    "A"
} else if score >= 80 {
    "B"
} else if score >= 60 {
    "C"
} else {
    "D"
}
print(grade)  // "B"
```

Inside each branch's code block, **the value of the last expression is the return value of that
branch**. You can also use `return` to return explicitly, but in branches it's usually enough to
just write the expression directly.

```yaoxiang
age = 15

// Write the expression directly — recommended
category = if age < 18 { "未成年" } else { "成年" }

// Or use explicit return — same effect
category2 = if age < 18 {
    return "未成年"
} else {
    return "成年"
}
```

If you only use `if` for conditional branching and don't need a value, it's just an ordinary
statement—fully compatible with the expression form.

## Nested `if`

You can write another `if` inside an `if` to handle multi-level conditional checks:

```yaoxiang
age = 25
has_ticket = true

if age >= 18 {
    if has_ticket {
        print("欢迎入场")
    } else {
        print("请先购票")
    }
} else {
    print("未成年人需家长陪同")
}
```

When expressions are nested, YaoXiang has no "dangling else" ambiguity like in C—every `else` always
belongs to the nearest unmatched `if`.

## Combining Conditions with Boolean Operators

In conditions, you can use `and` and `or` to combine multiple checks, and the unary prefix `!` for
negation. ⚠️ YaoXiang **does not have a `not` keyword** (only `and` / `or` are reserved words);
writing `not is_banned` triggers a parse error `E0010`, so you must write `!is_banned`.

```yaoxiang
username = "admin"
password = "123456"
role = "admin"
is_banned = false
is_vip = true
age = 25

// and: both conditions must hold
if username == "admin" and password == "123456" {
    print("登录成功")
}

// or: either condition holds
if role == "admin" or role == "moderator" {
    print("有管理权限")
}

// !: negation
if !is_banned {
    print("允许发言")
}

// combined usage
if (age >= 18 and age <= 60) or is_vip {
    print("可以参加活动")
}
```

In terms of operator precedence, `!` is higher than `and`, and `and` is higher than `or`. If you're
unsure, add parentheses to make your intent clearer.

## Summary

| Key Point       | Description                                                      |
| --------------- | ---------------------------------------------------------------- |
| Basic Structure | `if condition { ... } else if condition { ... } else { ... }`    |
| `else if`       | YaoXiang uses `else if` to implement multi-way branching         |
| Expression      | `if` can return a value; all branches must have the same type    |
| Branch Return   | The value of the last expression in a branch is its return value |
| Nesting         | You can write `if` inside `if`, no dangling-else ambiguity       |
| Boolean Ops     | `and`, `or` to combine conditions, `!` to negate                 |

In the next chapter you'll learn about the `for` loop—the standard way to iterate over collections
and ranges.
