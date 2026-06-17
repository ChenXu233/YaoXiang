---
title: Defining and Calling Functions
---

# Defining and Calling Functions

In the previous chapter, you learned how to declare variables. This chapter takes you into the heart of YaoXiang — functions. YaoXiang's function syntax shares the same `name: type = value` model as variable declarations, so much of this should feel familiar.

## Functions Are Lambdas

The most important concept first: **in YaoXiang, a function is simply a lambda expression with a name.** There is no special `fn` keyword, no ceremony. Defining a function just means giving a name to a lambda.

```
# Every function is fundamentally these four pieces:
name: (params) -> Return = body
 ^        ^        ^        ^
 |        |        |        +-- Body (lambda expression or code block)
 |        |        +-- Return type
 |        +-- Parameter list (signature)
 +-- Function name
```

This is exactly the same `name: type = value` model you already know — the "type" here just happens to be a function type.

---

## Expression Form: Direct Return Value

The simplest functions do not need `return`. When the function body is a single expression, its value is automatically returned:

```yaoxiang
# Expression form — directly returns the value, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "Hello, " + name
```

Calling them:

```yaoxiang
sum = add(3, 5)          # sum = 8
sq = square(4)           # sq = 16
msg = greet("world")     # msg = "Hello, world"
```

This is the **expression form**. When the function body is an expression (not a `{ }` block), its value becomes the return value. No `return` needed — writing one would be an error.

```yaoxiang
# Correct: expression is directly the return value
double: (x: Int) -> Int = x * 2

# Error: writing return in expression form is a syntax error
# double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Block Form: Explicit Return

When a function needs multiple steps of computation, use a `{ }` block as the body. **Inside a block, you must use `return` to produce a value**:

```yaoxiang
# Block form — must use return to produce a value
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

# Compute
f5 = factorial(5)        # f5 = 120
```

The rule is straightforward: **expression form returns directly; block form requires explicit `return`**. If you forget `return` inside a block, the function defaults to returning `Void`.

```yaoxiang
# Note: this function has a bug
# bad_add: (a: Int, b: Int) -> Int = {
#     a + b   // No return! Block defaults to Void, but signature expects Int → type error
# }

# Correct version
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form | Syntax | How to return a value |
|------|--------|-----------------------|
| Expression form | `name: ... = expr` | Expression value is the return value |
| Block form | `name: ... = { ... }` | Must use `return` explicitly |

---

## Parameters

### Basic Parameters

Parameters are listed in the function signature. Each parameter may have a type annotation:

```yaoxiang
# Two parameters, both with type annotations
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types: Signature or Lambda Head

YaoXiang's rule: **when the function has input parameters, their types must appear in at least one of the signature or the lambda head**. Omitting both will be rejected by the compiler.

```yaoxiang
# Style 1: types in the signature (omit lambda head)
add: (a: Int, b: Int) -> Int = a + b

# Style 2: types in the lambda head (omit signature)
add = (a: Int, b: Int) => a + b

# Style 3: full form (both signature and lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Error: types omitted from both
# add = (a, b) => a + b   // ❌ compiler cannot infer parameter types
```

**Style 1 is recommended** — types in the signature, lambda head omitted. It is the most concise and clear.

---

## Return Type

The return type comes after the `->`. The arrow `->` is the marker of a function type and cannot be omitted (without it the syntax would be parsed as something else).

```yaoxiang
# Returns Int
add_one: (x: Int) -> Int = x + 1

# Returns String
to_string: (n: Int) -> String = n.to_string()

# Returns Void (no meaningful return value)
log: (msg: String) -> Void = {
    println(msg)    # No return, defaults to Void
}
```

The return type can be omitted, letting HM type inference handle it:

```yaoxiang
# Compiler infers return type as Int
add = (a: Int, b: Int) => a + b

# Compiler infers return type as String
greet = (name: String) => "Hello, " + name
```

---

## Calling Functions

### Positional Arguments

The simplest way to call a function — pass arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        # result = 3
```

The formal definition of a function call in the language specification is:

```
Expr '(' ArgList? ')'
```

In plain terms: an expression followed by parentheses, optionally containing an argument list.

### Named Arguments

In addition to positional arguments, YaoXiang supports **named arguments** — specifying values by parameter name, in any order:

```yaoxiang
# Named arguments — parameter name followed by colon, then value
result = add(a: 3, b: 5)     # result = 8
result = add(b: 5, a: 3)     # Any order, same result

# Can mix positional and named, but positional must come first
result = add(3, b: 5)        # OK
```

Named arguments make calls more readable, especially with many parameters:

```yaoxiang
# Function signature
send: (to: String, title: String, body: String) -> Void = {
    println("Sending to: " + to)
    println("Title: " + title)
    println("Body: " + body)
}

# Named arguments make intent crystal clear
send(
    to: "alice@example.com",
    title: "Meeting Notice",
    body: "Meeting tomorrow at 3pm"
)
```

---

## Functions Without Parameters

Functions that take no parameters can omit the parameter list:

```yaoxiang
# Full form: explicitly declares empty parameters
hello: () -> Void = {
    println("Hello!")
}

# Simplest form: omit signature, compiler infers () -> Void
hello = {
    println("Hello!")
}

# Calling a parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
# Several ways to write main

# Full form
main: () -> Void = {
    println("Hello, YaoXiang!")
}

# Simplest form (recommended)
main = {
    println("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When function logic is more complex, organize your code with the block form. YaoXiang requires 4-space indentation:

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

    # Guard against division by zero
    if count == 0 {
        return 0.0
    }

    # Return the average
    return total:as(Float) / count:as(Float)
}
```

In multi-line functions, you can write comments with `#`, declare `mut` local variables, and build logic with `for` and `if`.

---

## pub and Auto-binding

In modules, functions declared with `pub` can be imported and used by other modules. Even more interesting: **`pub` functions are automatically bound to types defined in the same file**, giving you OOP-style calling.

```yaoxiang
# point.yx

# Define a type
type Point = { x: Float, y: Float }

# pub function: compiler automatically creates Point.distance binding
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# Both calling styles work
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

d1 = distance(p1, p2)       # Functional style
d2 = p1.distance(p2)        # OOP style (syntactic sugar)
```

The compiler sees `pub distance(p1: Point, p2: Point)`, notices that `Point` is defined in the same file, and automatically creates a `Point.distance` binding. You do not need to write any `impl` code.

---

## Quick Reference

```yaoxiang
# ── Function definition syntax at a glance ──

# Expression form (most common)
add: (a: Int, b: Int) -> Int = a + b

# Block form (multi-step logic)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Parameterless function (simplest)
main = { println("Hello!") }

# With params — omit signature
double = (x: Int) => x * 2

# With params — omit lambda head (recommended)
triple: (x: Int) -> Int = x * 3

# pub export + auto-binding
pub add: (a: Int, b: Int) -> Int = a + b

# ── Call syntax ──

result = add(1, 2)          # Positional arguments
result = add(a: 1, b: 2)    # Named arguments
result = add(1, b: 2)       # Mixed (positional first)
```

---

## Summary

You have now mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, sharing the same `name: type = value` model as variable declarations
- **Expression form**: `= expr`, the expression value is directly the return value — no `return` needed
- **Block form**: `= { ...; return expr }`, must use `return` to produce a value from the block
- **Parameter type annotations**: types must appear in at least one of the signature or lambda head; writing them in the signature is recommended
- **Calling**: positional or named arguments; named arguments can be in any order
- **pub auto-binding**: `pub` functions automatically bind to types in the same file, enabling `obj.method()` calling style
- **Simplest form**: `name = { ... }`, compiler infers as `() -> Void`

Next, you can continue to [Control Flow](./control-flow.md) to learn about using `if`, `for`, and `while` in your functions.
