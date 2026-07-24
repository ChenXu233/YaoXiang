---
title: if-elif-else
---

# if-elif-else

`if-elif-else` is the most fundamental decision-making tool in programming. Its logic is very intuitive—**if the condition holds, execute a block of code; otherwise, check the next condition; if none hold, take the default path**.

## Basic Syntax

In the syntax specification, `if` expressions and `if` statements are defined identically:

```
if Expr Block ('elif' Expr Block)* ('else' Block)?
```

Translated into everyday language: it starts with `if`, followed by a condition expression and a code block, then optionally zero or more `elif condition block` pairs, and finally an optional `else block`.

The simplest form—just `if`:

```yaoxiang
if temperature > 30 {
    print("It's hot, turn on the AC")
}
```

Adding `else`:

```yaoxiang
if is_raining {
    print("Bring an umbrella")
} else {
    print("No umbrella needed")
}
```

Multiple conditions with `elif`:

```yaoxiang
score = 85

if score >= 90 {
    print("Excellent")
} elif score >= 80 {
    print("Good")
} elif score >= 60 {
    print("Pass")
} else {
    print("Needs improvement")
}
```

Note that YaoXiang's keyword is `elif`, not `else if`. This reflects the language's deliberate effort to keep its keywords concise.

## if as an Expression

This is one of YaoXiang's most important control flow features: **`if` can be used as an expression that computes a value**.

```yaoxiang
// if expression: the value of each branch is assigned to result
result = if x > 0 {
    "Positive"
} elif x < 0 {
    "Negative"
} else {
    "Zero"
}
// result is now one of "Positive", "Negative", or "Zero"
```

When `if` is used as an expression, the return value types of all branches must be consistent:

```yaoxiang
score = 88

// All branches return String, types are consistent, no problem
grade = if score >= 90 {
    "A"
} elif score >= 80 {
    "B"
} elif score >= 60 {
    "C"
} else {
    "D"
}
print(grade)  // "B"
```

In each branch's code block, **the value of the last expression is that branch's return value**. You can also use `return` to return explicitly, but inside branches, simply writing the expression is usually enough.

```yaoxiang
// Directly write the expression—recommended
category = if age < 18 { "Minor" } else { "Adult" }

// You can also use return explicitly—same effect
category = if age < 18 {
    return "Minor"
} else {
    return "Adult"
}
```

If you only use `if` for conditional judgment and don't need a value, it's an ordinary statement—fully compatible with the expression form.

## Nested if

You can write another `if` inside an `if` to handle multi-level conditional logic:

```yaoxiang
age = 25
has_ticket = true

if age >= 18 {
    if has_ticket {
        print("Welcome in")
    } else {
        print("Please purchase a ticket first")
    }
} else {
    print("Minors must be accompanied by a parent")
}
```

When expressions are nested, YaoXiang has no C-style "dangling else" ambiguity—each `else` always belongs to the nearest unmatched `if`.

## Combining Conditions with Boolean Operators

In conditions, you can use `and`, `or`, `not` to combine multiple judgments:

```yaoxiang
username = "admin"
password = "123456"

// and: both conditions must hold
if username == "admin" and password == "123456" {
    print("Login successful")
}

// or: either condition holds
if role == "admin" or role == "moderator" {
    print("Has admin permissions")
}

// not: negation
if not is_banned {
    print("Allowed to post")
}

// Combined usage
if (age >= 18 and age <= 60) or is_vip {
    print("Can attend the event")
}
```

In operator precedence, `not` is higher than `and`, and `and` is higher than `or`. When in doubt, add parentheses to make your intent clearer.

## Summary

| Key Point | Description |
|------|------|
| Basic structure | `if condition { ... } elif condition { ... } else { ... }` |
| elif | YaoXiang uses `elif`, not `else if` |
| Expression | `if` can return a value; all branches must have consistent types |
| Branch return value | The value of the last expression in the branch block is the return value |
| Nesting | `if` can contain another `if`; no dangling else ambiguity |
| Boolean operators | `and`, `or`, `not` combine conditions |

The next chapter will cover `for` loops—the standard way to iterate over collections and ranges.