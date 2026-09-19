---
title: 'Function Definition and Invocation'
---

# Function Definition and Invocation

In the previous chapter, you learned how to declare variables. This chapter will guide you through
the core of YaoXiang — functions. YaoXiang's function syntax shares the same `name: type = value`
model as variable declarations, so you should find it familiar.

## Functions Are Lambdas

Let's start with the most important concept: **In YaoXiang, a function is essentially a lambda
expression**. There is no special `fn` keyword, no complex ceremony. Defining a function is simply
giving a lambda a name.

```
# Any function is essentially a combination of these four things:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- function body (lambda expression or code block)
 |       |        +-- return value type
 |       +-- parameter list (signature)
 +-- function name
```

This is exactly the same as the `name: type = value` you learned in the previous chapter — it's just
that the "type" here happens to be a function type.

---

## Expression Form: Returning a Value Directly

The simplest function doesn't need the `return` keyword. When the function body is a single
expression, it serves as the return value directly:

```yaoxiang
// Expression form — return value directly, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "Hello, " + name
```

Call them:

```yaoxiang
sum = add(3, 5)          // sum = 8
sq = square(4)           // sq = 16
msg = greet("World")      // msg = "Hello, World"
```

This is called the **expression form**. When the function body is an expression (not a `{ }` code
block), its value serves directly as the function's return value. There is no need to write
`return`, and writing it would actually be an error.

```yaoxiang
// Correct: the expression serves directly as the return value
double: (x: Int) -> Int = x * 2

// Wrong: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Code Block Form: Explicit return

When a function contains multiple steps of computation, wrap the function body in a `{ }` code
block. **In a code block, you must use the `return` statement to return a value**:

```yaoxiang
// Code block form — must use return to return a value
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// Compute result
f5 = factorial(5)        // f5 = 120
```

The rule is simple: **the expression form returns the value directly; the code block form must
explicitly `return`**. If you forget to write `return` in a code block, the function defaults to
returning `Void`.

```yaoxiang
// Note: this function has a bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // No return! The block defaults to returning Void, but the signature requires Int → type error
// }

// Correct way to write it
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form            | Syntax                | Return Value Method                              |
| --------------- | --------------------- | ------------------------------------------------ |
| Expression form | `name: ... = expr`    | Expression value serves as return value directly |
| Code block form | `name: ... = { ... }` | Must use `return` explicitly                     |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be annotated with a type:

```yaoxiang
// Two parameters, both annotated with types
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in Either the Signature or the Lambda Header

YaoXiang's rule is: **when there are input parameters, the parameter types must appear explicitly in
at least one of the signature or the lambda header**. Omitting them on both sides will be rejected
by the compiler.

```yaoxiang
// Method 1: parameter types written in the signature (omit lambda header)
add: (a: Int, b: Int) -> Int = a + b

// Method 2: parameter types written in the lambda header (omit signature)
add = (a: Int, b: Int) => a + b

// Method 3: complete form (both signature and lambda header)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Wrong: no types written on either side
// add = (a, b) => a + b   // ❌ Compiler cannot infer parameter types
```

**Method 1 is recommended** — write parameter types in the signature and omit the lambda header.
This is the most concise and clearest way to write it.

---

## Return Value

The function's return value type is written after `->`. The `->` is the marker of a function type
and cannot be omitted (omitting it would be parsed as a different type).

```yaoxiang
// Returns Int
add_one: (x: Int) -> Int = x + 1

// Returns String
to_string: (n: Int) -> String = n.to_string()

// Returns Void (no return value)
log: (msg: String) -> Void = {
    print(msg)    // No return, defaults to returning Void
}
```

The return value type can also be omitted, letting HM type inference handle it:

```yaoxiang
// Compiler infers the return type as Int
add = (a: Int, b: Int) => a + b

// Compiler infers the return type as String
greet = (name: String) => "Hello, " + name
```

---

## Function Invocation

### Positional Arguments

The most basic way to call — passing arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

The syntactic specification defines function invocation as:

```
Expr '(' ArgList? ')'
```

Translated into everyday language: an expression followed by a pair of parentheses, with an optional
argument list inside the parentheses.

### Named Arguments

Besides passing arguments by position, YaoXiang also supports **named arguments** — using parameter
names to specify values, in any order:

```yaoxiang
// Named arguments — parameter name followed by a colon, then the value
result = add(a: 3, b: 5)     // result = 8
result = add(b: 5, a: 3)     // Any order, same result

// Can be mixed with positional arguments, but positional arguments must come first
result = add(3, b: 5)        // OK
```

Named arguments make calls more readable, which is especially useful when there are many parameters:

```yaoxiang
// Function signature
send: (to: String, title: String, body: String) -> Void = {
    print("To: " + to)
    print("Title: " + title)
    print("Body: " + body)
}

// Named arguments make the call's intent crystal clear
send(
    to: "alice@example.com",
    title: "Meeting Notice",
    body: "Meeting tomorrow at 3 PM"
)
```

---

## Parameterless Functions

Functions that don't need parameters can omit the parameter list:

```yaoxiang
// Complete form: explicitly declare empty parameter list
hello: () -> Void = {
    print("Hello!")
}

// Most concise form: omit the signature, compiler automatically infers () -> Void
hello = {
    print("Hello!")
}

// Call the parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
// Several ways to write the main function

// Complete form
main: () -> Void = {
    print("Hello, YaoXiang!")
}

// Most concise form (recommended)
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When a function's logic is more complex, use the code block form to organize the code. YaoXiang
enforces 4-space indentation:

```yaoxiang
// Multi-step computation
calculate_stats: (numbers: List(Int)) -> Float = {
    // Declare local variables
    mut total = 0
    mut count = 0

    // Loop and accumulate
    for n in numbers {
        total = total + n
        count = count + 1
    }

    // Avoid division by zero
    if count == 0 {
        return 0.0
    }

    // Return the average
    return total:as(Float) / count:as(Float)
}
```

In multi-line functions, you can use `#` to write comments, declare `mut` local variables, and use
`for` and `if` to build logic.

---

## pub and Automatic Binding

In a module, functions declared with the `pub` keyword can be imported and used by other modules.
What's even more interesting is that **`pub` functions are automatically bound to types defined in
the same file**, allowing you to call them in an OOP style.

```yaoxiang
// point.yx

// Define a type
Point: Type = { x: Float, y: Float }

// pub function: compiler automatically binds it to Point.distance
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// Both calling styles work
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

d1 = distance(p1, p2)       // Functional call
d2 = p1.distance(p2)        // OOP-style call (syntactic sugar)
```

When the compiler sees `pub distance(p1: Point, p2: Point)`, it finds that `Point` is defined in the
same file, and automatically creates the `Point.distance` binding. You don't need to write any extra
`impl` code.

---

## Quick Reference

```yaoxiang
// ── Function Definition Syntax Overview ──

// Expression form (most commonly used)
add: (a: Int, b: Int) -> Int = a + b

// Code block form (multi-step logic)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Parameterless function (most concise)
main: () -> Void = { print("Hello!") }

// With parameters — omit signature
double = (x: Int) => x * 2

// With parameters — omit lambda header (recommended)
triple: (x: Int) -> Int = x * 3

// pub export + automatic binding
pub add: (a: Int, b: Int) -> Int = a + b

// ── Invocation Syntax ──

result = add(1, 2)          // Positional arguments
result = add(a: 1, b: 2)    // Named arguments
result = add(1, b: 2)       // Mixed (positional first)
```

---

## Summary

You've now mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, sharing the same origin as variable
  declaration's `name: type = value`
- **Expression form**: `= expr`, the expression value serves as the return value directly, no
  `return` needed
- **Code block form**: `= { ...; return expr }`, must use `return` explicitly inside the block
- **Parameter type annotation**: types must be written in at least one of the signature or lambda
  header; recommended to write in the signature
- **Invocation**: positional or named arguments; named arguments can be in any order
- **pub automatic binding**: `pub` functions are automatically bound to types in the same file,
  supporting `obj.method()` calls
- **Parameterless most concise**: `name = { ... }`, compiler automatically infers as `() -> Void`

Next, you can continue to the [Control Flow](./control-flow.md) chapter to learn how to use `if`,
`for`, and `while` inside functions.
