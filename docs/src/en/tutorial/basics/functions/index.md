---
title: Function Definition and Invocation
---

# Function Definition and Invocation

In the previous chapter, you learned how to declare variables. This chapter will walk you through the core of YaoXiang—functions. YaoXiang's function syntax shares the same `name: type = value` model as variable declarations, so you should find it familiar.

## Functions Are Lambdas

Let me start with the most important concept: **In YaoXiang, functions are essentially lambda expressions**. There's no special `fn` keyword, no complex ceremony. To define a function, you simply give a lambda a name.

```
# Any function is essentially a combination of these four things:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- Function body (lambda expression or code block)
 |       |        +-- Return type
 |       +-- Parameter list (signature)
 +-- Function name
```

This is exactly the same as the `name: type = value` you learned in the previous chapter—just that the "type" here happens to be a function type.

---

## Expression Form: Direct Return Value

The simplest functions don't need the `return` keyword. When the function body is a single expression, it is directly used as the return value:

```yaoxiang
# Expression form—returns the value directly, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "Hello, " + name
```

Call them:

```yaoxiang
sum = add(3, 5)          # sum = 8
sq = square(4)           # sq = 16
msg = greet("World")     # msg = "Hello, World"
```

This is called the **expression form**. When the function body is an expression (not a `{ }` code block), its value is directly used as the function's return value. You don't need to write `return`, and writing it would actually be an error.

```yaoxiang
# Correct: the expression is the return value directly
double: (x: Int) -> Int = x * 2

# Wrong: writing return in expression form is a syntax error
# double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Code Block Form: Explicit `return`

When a function contains multiple steps of computation, wrap the function body in a `{ }` code block. **In a code block, you must use a `return` statement to return a value**:

```yaoxiang
# Code block form—must use return to return a value
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

# Compute the result
f5 = factorial(5)        # f5 = 120
```

The rule is simple: **expression form returns the value directly; code block form requires an explicit `return`**. If you forget to write `return` in a code block, the function defaults to returning `Void`.

```yaoxiang
# Note: this function has a bug
# bad_add: (a: Int, b: Int) -> Int = {
#     a + b   # No return! The block defaults to returning Void, but the signature requires Int → type error
# }

# Correct version
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form | Syntax | How to return |
|------|------|------------|
| Expression form | `name: ... = expr` | The expression value is directly the return value |
| Code block form | `name: ... = { ... }` | Must use `return` explicitly |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be annotated with a type:

```yaoxiang
# Two parameters, both annotated with types
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in the Signature or Lambda Head

YaoXiang's rule is: **when there are input parameters, the parameter type must appear explicitly in at least one of the signature or the lambda head**. Omitting types on both sides will be rejected by the compiler.

```yaoxiang
# Approach 1: parameter types in the signature (omit the lambda head)
add: (a: Int, b: Int) -> Int = a + b

# Approach 2: parameter types in the lambda head (omit the signature)
add = (a: Int, b: Int) => a + b

# Approach 3: full form (both signature and lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Wrong: no types on either side
# add = (a, b) => a + b   // ❌ The compiler cannot infer the parameter types
```

**Approach 1 is recommended**—write the parameter types in the signature, and omit the lambda head. This is the most concise and clearest style.

---

## Return Value

The function's return type is written after `->`. The `->` is the marker of a function type and cannot be omitted (omitting it will be parsed as another type).

```yaoxiang
# Returns Int
add_one: (x: Int) -> Int = x + 1

# Returns String
to_string: (n: Int) -> String = n.to_string()

# Returns Void (no return value)
log: (msg: String) -> Void = {
    println(msg)    # No return, defaults to Void
}
```

The return type can also be omitted, letting HM type inference handle it for you:

```yaoxiang
# The compiler infers the return type as Int
add = (a: Int, b: Int) => a + b

# The compiler infers the return type as String
greet = (name: String) => "Hello, " + name
```

---

## Function Invocation

### Positional Arguments

The most basic call style—pass arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        # result = 3
```

The form definition of a function call in the language specification is:

```
Expr '(' ArgList? ')'
```

In everyday language: an expression followed by a pair of parentheses, with an optional argument list inside.

### Named Arguments

In addition to positional arguments, YaoXiang also supports **named arguments**—use parameter names to specify values, in any order:

```yaoxiang
# Named arguments—parameter name followed by a colon, then the value
result = add(a: 3, b: 5)     # result = 8
result = add(b: 5, a: 3)     # Any order, same result

# Can be mixed with positional arguments, but positional ones must come first
result = add(3, b: 5)        # OK
```

Named arguments make calls more readable, which is especially helpful when there are many parameters:

```yaoxiang
# Function signature
send: (to: String, title: String, body: String) -> Void = {
    println("To: " + to)
    println("Title: " + title)
    println("Body: " + body)
}

# Named arguments make the call's intent crystal clear
send(
    to: "alice@example.com",
    title: "Meeting Notice",
    body: "Meeting at 3 PM tomorrow"
)
```

---

## Parameterless Functions

Functions that don't need parameters can omit the parameter list:

```yaoxiang
# Full form: explicitly declare empty parameters
hello: () -> Void = {
    println("Hello!")
}

# Shortest form: omit the signature, the compiler infers () -> Void automatically
hello = {
    println("Hello!")
}

# Call the parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
# Several ways to write the main function

# Full form
main: () -> Void = {
    println("Hello, YaoXiang!")
}

# Shortest form (recommended)
main = {
    println("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When a function's logic is more complex, use the code block form to organize the code. YaoXiang enforces 4-space indentation:

```yaoxiang
# Multi-step computation
calculate_stats: (numbers: List(Int)) -> Float = {
    # Declare local variables
    mut total = 0
    mut count = 0

    # Accumulate in a loop
    for n in numbers {
        total = total + n
        count = count + 1
    }

    # Avoid division by zero
    if count == 0 {
        return 0.0
    }

    # Return the average
    return total:as(Float) / count:as(Float)
}
```

In a multi-line function, you can use `#` to write comments, declare `mut` local variables, and use `for` and `if` to build logic.

---

## `pub` and Auto-Binding

In a module, functions declared with the `pub` keyword can be imported and used by other modules. Even more interestingly, **`pub` functions are automatically bound to types defined in the same file**, letting you call them in an OOP style.

```yaoxiang
# point.yx

# Define a type
type Point = { x: Float, y: Float }

# pub function: the compiler automatically binds it as Point.distance
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# Both call styles work
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

d1 = distance(p1, p2)       # Functional call
d2 = p1.distance(p2)        # OOP-style call (syntactic sugar)
```

When the compiler sees `pub distance(p1: Point, p2: Point)` and notices that `Point` is defined in the same file, it automatically creates the `Point.distance` binding. You don't need to write any extra `impl` code.

---

## Quick Reference

```yaoxiang
# ── Function Definition Syntax Overview ──

# Expression form (most common)
add: (a: Int, b: Int) -> Int = a + b

# Code block form (multi-step logic)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Parameterless function (shortest)
main = { println("Hello!") }

# With parameters—omitted signature
double = (x: Int) => x * 2

# With parameters—omitted lambda head (recommended)
triple: (x: Int) -> Int = x * 3

# pub export + auto-binding
pub add: (a: Int, b: Int) -> Int = a + b

# ── Invocation Syntax ──

result = add(1, 2)          # Positional arguments
result = add(a: 1, b: 2)    # Named arguments
result = add(1, b: 2)       # Mixed (positional first)
```

---

## Summary

You've now mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, sharing the same origin as `name: type = value` for variable declarations
- **Expression form**: `= expr`, the expression value is the return value directly, no `return` needed
- **Code block form**: `= { ...; return expr }`, must use `return` explicitly inside the block
- **Parameter type annotation**: must be written in at least one of the signature or the lambda head; recommended in the signature
- **Invocation**: positional or named arguments; named arguments can be in any order
- **`pub` auto-binding**: `pub` functions are automatically bound to types in the same file, supporting `obj.method()` calls
- **Shortest parameterless form**: `name = { ... }`, the compiler infers `() -> Void` automatically

Next, you can continue with the [Control Flow](./control-flow.md) chapter to learn how to use `if`, `for`, and `while` inside functions.