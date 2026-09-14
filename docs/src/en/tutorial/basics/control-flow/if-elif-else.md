---
title: 'if-else-if-else'
---

# if-else-if-else

`if-else-if-else` is the most fundamental decision-making tool in programming. Its logic is very
intuitive—**if a condition holds, execute some code; otherwise, check the next condition; if none
hold, take the default path**.

## Basic Syntax

In the syntax specification, the definition of an `if` expression and an `if` statement is exactly
the same:

```
if Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

Translated into everyday language: it starts with `if`, followed by a condition expression and a
code block, then you can have zero or more `else if condition block` clauses, and finally an
optional `else block`.

The simplest form—only `if`:

```yaoxiang
if temperature > 30 {
    print("天热了，开空调吧")
}
```

Add an `else`:

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

## if as an Expression

This is one of the most important features of YaoXiang control flow: **`if` can be used as an
expression, computing a value**.

```yaoxiang
// if expression: the values from each branch are assigned to result
result = if x > 0 {
    "正数"
} else if x < 0 {
    "负数"
} else {
    "零"
}
// result is now one of "正数", "负数", or "零"
```

When `if` is used as an expression, all branches must return the same type:

```yaoxiang
score = 88

// All branches return String, types are consistent, no problem
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

In each branch's code block, **the value of the last expression is the return value of that
branch**. You can also use `return` to explicitly return, but in branches you usually just write the
expression directly.

```yaoxiang
// Directly write the expression—recommended
category = if age < 18 { "未成年" } else { "成年" }

// You can also use explicit return—same effect
category = if age < 18 {
    return "未成年"
} else {
    return "成年"
}
```

If you only use `if` for a conditional check and don't need a value, it's a regular statement—fully
compatible with the expression form.

## Nested if

You can write `if` inside an `if` to handle multi-level conditional logic:

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

When expressions are nested, YaoXiang does not have the "dangling else" ambiguity of C—each `else`
always belongs to the nearest unmatched `if`.

## Combining Conditions with Boolean Operators

You can use `and`, `or`, and `not` in conditions to combine multiple checks:

```yaoxiang
username = "admin"
password = "123456"

// and: both conditions hold
if username == "admin" and password == "123456" {
    print("登录成功")
}

// or: either condition holds
if role == "admin" or role == "moderator" {
    print("有管理权限")
}

// not: negation
if not is_banned {
    print("允许发言")
}

// Combined usage
if (age >= 18 and age <= 60) or is_vip {
    print("可以参加活动")
}
```

In terms of operator precedence, `not` is higher than `and`, and `and` is higher than `or`. When in
doubt, add parentheses to make your intent clearer.

## Summary

| Key Point           | Description                                                              |
| ------------------- | ------------------------------------------------------------------------ |
| Basic structure     | `if condition { ... } else if condition { ... } else { ... }`            |
| else if             | YaoXiang uses `else if` for multi-way branching                          |
| Expression          | `if` can return a value; all branches must have the same type            |
| Branch return value | The value of the last expression in the branch block is the return value |
| Nesting             | You can write `if` inside an `if`; no dangling else ambiguity            |
| Boolean operators   | `and`, `or`, `not` to combine conditions                                 |

In the next chapter you'll learn `for` loops—the standard way to iterate over collections and
ranges.
