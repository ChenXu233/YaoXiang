---
title: if-elif-else
---

# if-elif-else

`if-elif-else` is the most fundamental decision-making tool in programming. The logic is intuitive — **if a condition is true, run some code; otherwise, check the next condition; if none hold, take the default path**.

## Basic Syntax

The `if` expression and `if` statement share the same definition in the language specification:

```
if Expr Block ('elif' Expr Block)* ('else' Block)?
```

In plain terms: `if` followed by a condition expression and a block, then zero or more `elif condition block` pairs, and finally an optional `else block`.

The simplest form — just `if`:

```yaoxiang
if temperature > 30 {
    println("It's hot — turn on the AC")
}
```

With `else` added:

```yaoxiang
if is_raining {
    println("Bring an umbrella")
} else {
    println("No umbrella needed")
}
```

Multiple conditions with `elif`:

```yaoxiang
score = 85

if score >= 90 {
    println("Excellent")
} elif score >= 80 {
    println("Good")
} elif score >= 60 {
    println("Pass")
} else {
    println("Needs improvement")
}
```

Notice that YaoXiang uses `elif`, not `else if`. This is part of the language's philosophy of keeping a minimal keyword set.

## if Is an Expression

This is one of the most important characteristics of YaoXiang's control flow: **`if` can be used as an expression that computes a value**.

```yaoxiang
# if expression: the value of each branch is assigned to result
result = if x > 0 {
    "positive"
} elif x < 0 {
    "negative"
} else {
    "zero"
}
# result is now one of "positive", "negative", or "zero"
```

When `if` is used as an expression, all branches must return the same type:

```yaoxiang
score = 88

# All branches return String — types are consistent, no problem
grade = if score >= 90 {
    "A"
} elif score >= 80 {
    "B"
} elif score >= 60 {
    "C"
} else {
    "D"
}
println(grade)  # "B"
```

In each branch's block, the last expression's value becomes that branch's return value. You can also use an explicit `return`, but in branches you can usually just write the expression directly.

```yaoxiang
# Write the expression directly — recommended
category = if age < 18 { "minor" } else { "adult" }

# Explicit return also works — same effect
category = if age < 18 {
    return "minor"
} else {
    return "adult"
}
```

If you only need conditional logic without a value, `if` is just a normal statement — fully compatible with the expression form.

## Nested if

You can nest `if` inside `if` for multi-level conditionals:

```yaoxiang
age = 25
has_ticket = true

if age >= 18 {
    if has_ticket {
        println("Welcome in")
    } else {
        println("Please purchase a ticket first")
    }
} else {
    println("Minors require a parent or guardian")
}
```

When nesting expressions, YaoXiang does not have C's "dangling else" ambiguity — each `else` always belongs to the nearest unmatched `if`.

## Combining Conditions with Boolean Operators

Use `and`, `or`, and `not` to combine multiple conditions:

```yaoxiang
username = "admin"
password = "123456"

# and: both conditions must hold
if username == "admin" and password == "123456" {
    println("Login successful")
}

# or: at least one condition must hold
if role == "admin" or role == "moderator" {
    println("Has management privileges")
}

# not: negate a condition
if not is_banned {
    println("Allowed to post")
}

# Combined use
if (age >= 18 and age <= 60) or is_vip {
    println("Eligible to participate")
}
```

In terms of precedence, `not` binds tighter than `and`, and `and` binds tighter than `or`. When in doubt, add parentheses to make intent clear.

## Summary

| Point | Description |
|-------|-------------|
| Basic structure | `if condition { ... } elif condition { ... } else { ... }` |
| elif | YaoXiang uses `elif`, not `else if` |
| Expression | `if` can return a value; all branches must have the same type |
| Branch return value | The last expression in the branch block is the return value |
| Nesting | `if` can contain `if`; no dangling else ambiguity |
| Boolean operators | `and`, `or`, `not` for combining conditions |

Next chapter: `for` loops — the standard way to iterate over ranges and collections.
