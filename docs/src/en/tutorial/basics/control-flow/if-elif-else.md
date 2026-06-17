---
title: if-elif-else
---

# if-elif-else

`if-elif-else` is the most fundamental decision-making tool in programming. Its logic is very intuitive—**if the condition holds, execute a block of code; otherwise, check the next condition; if none hold, take the default path**.

## Basic Syntax

The grammar spec defines `if` expressions and `if` statements identically:

```
if Expr Block ('elif' Expr Block)* ('else' Block)?
```

In plain language: start with `if`, followed by a condition expression and a code block, then optionally followed by zero or more `elif condition block` pairs, and finally an optional `else block`.

The simplest form—only `if`:

```yaoxiang
if temperature > 30 {
    println("It's hot, turn on the air conditioner")
}
```

Add an `else`:

```yaoxiang
if is_raining {
    println("Bring an umbrella")
} else {
    println("No need to bring an umbrella")
}
```

Use `elif` for multiple conditions:

```yaoxiang
score = 85

if score >= 90 {
    println("Excellent")
} elif score >= 80 {
    println("Good")
} elif score >= 60 {
    println("Pass")
} else {
    println("Need to work harder")
}
```

Note that YaoXiang's keyword is `elif`, not `else if`. This reflects the language's deliberate effort to keep its keyword set concise.

## `if` as an Expression

This is one of the most important features of YaoXiang's control flow: **`if` can be used as an expression that evaluates to a value**.

```yaoxiang
# if expression: the value of each branch is assigned to result
result = if x > 0 {
    "Positive"
} elif x < 0 {
    "Negative"
} else {
    "Zero"
}
# result is now one of "Positive", "Negative", or "Zero"
```

When `if` is used as an expression, the return value types of all branches must be the same:

```yaoxiang
score = 88

# All branches return String, types match—fine
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

Within each branch's block, **the value of the last expression is the return value of that branch**. You can also use `return` to explicitly return, but in branches it's usually enough to just write the expression directly.

```yaoxiang
# Write the expression directly—recommended
category = if age < 18 { "Minor" } else { "Adult" }

# Or use return explicitly—same effect
category = if age < 18 {
    return "Minor"
} else {
    return "Adult"
}
```

If you only use `if` for conditional judgment and don't need a value, it's just a regular statement—fully compatible with the expression form.

## Nested `if`

You can write `if` inside `if` to handle multi-level conditional logic:

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
    println("Minors must be accompanied by a parent")
}
```

When expressions are nested, YaoXiang has no "dangling else" ambiguity like in C—every `else` always binds to the nearest unmatched `if`.

## Combining Conditions with Boolean Operators

You can use `and`, `or`, `not` to combine multiple predicates in a condition:

```yaoxiang
username = "admin"
password = "123456"

# and: both conditions must hold
if username == "admin" and password == "123456" {
    println("Login successful")
}

# or: either condition holds
if role == "admin" or role == "moderator" {
    println("Has admin privileges")
}

# not: negate
if not is_banned {
    println("Allowed to speak")
}

# Combined usage
if (age >= 18 and age <= 60) or is_vip {
    println("Can attend the event")
}
```

In terms of operator precedence, `not` is higher than `and`, and `and` is higher than `or`. When in doubt, add parentheses to make your intent clearer.

## Summary

| Key Point | Description |
|------|------|
| Basic structure | `if condition { ... } elif condition { ... } else { ... }` |
| elif | YaoXiang uses `elif`, not `else if` |
| Expression | `if` can return a value; all branches must have the same type |
| Branch return value | The last expression in a branch block is the return value |
| Nesting | `if` can contain another `if`; no dangling-else ambiguity |
| Boolean operators | `and`, `or`, `not` for combining conditions |

In the next chapter you'll learn about the `for` loop—the standard way to iterate over collections and ranges.