---
title: if-else-if-else
---

# if-else-if-else

`if-else-if-else` is the most fundamental decision-making tool in programming. Its logic is very
intuitive — **if the condition is true, execute some code; otherwise, check the next condition; if
none are true, go the default path**.

## Basic Syntax

In the syntax specification, the definitions of `if` expression and `if` statement are identical:

```
if Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

Translated into everyday language: starts with `if`, followed by a conditional expression and a code
block, then zero to many `else if condition code block`, and finally an optional `else code block`.

The simplest form — only `if`:

```yaoxiang
if temperature > 30 {
    print("It's hot, turn on the AC")
}
```

With `else`:

```yaoxiang
if is_raining {
    print("Bring an umbrella")
} else {
    print("No need for an umbrella")
}
```

Multiple conditions with `else if`:

```yaoxiang
score = 85

if score >= 90 {
    print("Excellent")
} else if score >= 80 {
    print("Good")
} else if score >= 60 {
    print("Pass")
} else {
    print("Needs improvement")
}
```

## if is an Expression

This is one of the most important features of YaoXiang's control flow: **`if` can be used as an
expression to compute a value**.

```yaoxiang
// if expression: the value from each branch is assigned to result
result = if x > 0 {
    "positive"
} else if x < 0 {
    "negative"
} else {
    "zero"
}
// result is now one of "positive", "negative", or "zero"
```

When `if` is used as an expression, the return value types from all branches must be consistent:

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

In the code block of each branch, **the value of the last expression is the return value of that
branch**. You can also use `return` to explicitly return, but in branches it's usually sufficient to
just write the expression.

```yaoxiang
// Writing the expression directly — recommended
category = if age < 18 { "minor" } else { "adult" }

// Can also explicitly return — same effect
category = if age < 18 {
    return "minor"
} else {
    return "adult"
}
```

If you only use `if` for conditional logic without needing a value, it's just a regular statement —
fully compatible with the expression form.

## Nested if

You can write `if` inside `if` to handle multi-level conditional logic:

```yaoxiang
age = 25
has_ticket = true

if age >= 18 {
    if has_ticket {
        print("Welcome")
    } else {
        print("Please purchase a ticket first")
    }
} else {
    print("Minors must be accompanied by a guardian")
}
```

When dealing with nested expressions, YaoXiang doesn't have the "dangling else" ambiguity found in C
— each `else` always belongs to the nearest unmatched `if`.

## Combining Conditions with Boolean Operators

You can use `and`, `or`, and `not` to combine multiple conditions:

```yaoxiang
username = "admin"
password = "123456"

// and: both conditions must be true
if username == "admin" and password == "123456" {
    print("Login successful")
}

// or: either condition can be true
if role == "admin" or role == "moderator" {
    print("Has admin privileges")
}

// not: negation
if not is_banned {
    print("Allowed to post")
}

// Combined usage
if (age >= 18 and age <= 60) or is_vip {
    print("Eligible to participate")
}
```

In terms of operator precedence, `not` is higher than `and`, and `and` is higher than `or`. When in
doubt, add parentheses to make your intent clearer.

## Summary

| Key Point       | Description                                                      |
| --------------- | ---------------------------------------------------------------- |
| Basic Structure | `if condition { ... } else if condition { ... } else { ... }`    |
| else if         | YaoXiang uses `else if` for multi-way branching                  |
| Expression      | `if` can return a value, all branches must have consistent types |
| Branch Return   | The last expression in a branch block is the return value        |
| Nesting         | You can write `if` inside `if`, no dangling else ambiguity       |
| Boolean Ops     | `and`, `or`, `not` to combine conditions                         |

In the next chapter you will learn about `for` loops — the standard way to iterate over collections
and ranges.
