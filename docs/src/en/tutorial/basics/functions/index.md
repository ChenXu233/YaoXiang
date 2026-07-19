---
title: Function Definition and Calling
---

# Function Definition and Calling

In the previous chapter, you learned how to declare variables. This chapter will walk you through the core of YaoXiang—functions. YaoXiang's function syntax shares the same `name: type = value` model with variable declarations, so it should feel familiar.

## Functions Are Lambdas

Let's start with the most important concept: **In YaoXiang, functions are essentially lambda expressions**. There's no special `fn` keyword, no complex ceremony. To define a function is to give a name to a lambda.

```
# Any function is essentially a combination of these four things:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- function body (lambda expression or code block)
 |       |        +-- return type
 |       +-- parameter list (signature)
 +-- function name
```

This is exactly the same as the `name: type = value` you learned in the previous chapter—just that the "type" here happens to be a function type.

---

## Expression Form: Direct Return Value

The simplest function doesn't need the `return` keyword. When the function body is a single expression, it serves directly as the return value:

```yaoxiang
// Expression form—return value directly, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "Hello, " + name
```

Call them:

```yaoxiang
sum = add(3, 5)          // sum = 8
sq = square(4)           // sq = 16
msg = greet("world")     // msg = "Hello, world"
```

This is called the **expression form**. When the function body is an expression (not a `{ }` code block), its value serves directly as the function's return value. There's no need to write `return`, and writing it would actually be wrong.

```yaoxiang
// Correct: the expression serves directly as the return value
double: (x: Int) -> Int = x * 2

// Wrong: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Code Block Form: Explicit `return`

When a function contains multiple steps of computation, wrap the function body in `{ }`. **In a code block, you must use the `return` statement to return a value**:

```yaoxiang
// Code block form—must use return to return a value
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// Compute the result
f5 = factorial(5)        // f5 = 120
```

The rule is simple: **the expression form returns directly; the code block form requires an explicit `return`**. If you forget to write `return` in a code block, the function defaults to returning `Void`.

```yaoxiang
// Note: this function has a bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // No return! The block defaults to returning Void, but the signature requires Int → type error
// }

// Correct form
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form | Syntax | How to Return |
|------|------|------------|
| Expression form | `name: ... = expr` | The expression value serves directly as the return value |
| Code block form | `name: ... = { ... }` | Must use `return` to return explicitly |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be type-annotated:

```yaoxiang
// Two parameters, both type-annotated
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in Either the Signature or the Lambda Header

YaoXiang's rule is: **when there are input parameters, the parameter types must appear explicitly in either the signature or the lambda header**. Omitting them in both places will be rejected by the compiler.

```yaoxiang
// Way 1: parameter types written in the signature (omit the lambda header)
add: (a: Int, b: Int) -> Int = a + b

// Way 2: parameter types written in the lambda header (omit the signature)
add = (a: Int, b: Int) => a + b

// Way 3: complete form (both signature and lambda header present)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Wrong: types omitted in both places
// add = (a, b) => a + b   // ❌ The compiler cannot infer the parameter types
```

**The recommended way is Way 1**—write the parameter types in the signature and omit the lambda header. This is the most concise and clearest way to write.

---

## Return Value

The function's return type is written after `->`. `->` is the marker of a function type and cannot be omitted (omitting it would be parsed as another type).

```yaoxiang
// Return Int
add_one: (x: Int) -> Int = x + 1

// Return String
to_string: (n: Int) -> String = n.to_string()

// Return Void (no return value)
log: (msg: String) -> Void = {
    print(msg)    // No return, defaults to returning Void
}
```

The return type can also be omitted, letting HM type inference handle it:

```yaoxiang
// The compiler infers the return type as Int
add = (a: Int, b: Int) => a + b

// The compiler infers the return type as String
greet = (name: String) => "Hello, " + name
```

---

## Function Calling

### Positional Arguments

The most basic way to call—by passing arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

The grammar's form definition for function calling is:

```
Expr '(' ArgList? ')'
```

In everyday language: an expression followed by a pair of parentheses, with an optional argument list inside.

### Named Arguments

In addition to passing arguments by position, YaoXiang also supports **named arguments**—specify the value by parameter name, with no restriction on order:

```yaoxiang
// Named arguments—the parameter name is followed by a colon, then the value
result = add(a: 3, b: 5)     // result = 8
result = add(b: 5, a: 3)     // Order doesn't matter, same result

// Can be mixed with positional arguments, but positional arguments must come first
result = add(3, b: 5)        // OK
```

Named arguments make calls more readable, especially useful when there are many parameters:

```yaoxiang
// Function signature
send: (to: String, title: String, body: String) -> Void = {
    print("Send to: " + to)
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
// Complete form: explicitly declare empty parameters
hello: () -> Void = {
    print("Hello!")
}

// Shortest form: omit the signature, the compiler automatically infers it as () -> Void
hello = {
    print("Hello!")
}

// Call a parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
// Several ways to write the main function

// Complete form
main: () -> Void = {
    print("Hello, YaoXiang!")
}

// Shortest form (recommended)
main = {
    print("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When the function logic is more complex, use the code block form to organize the code. YaoXiang enforces 4-space indentation:

```yaoxiang
// Multi-step computation
calculate_stats: (numbers: List(Int)) -> Float = {
    // Declare local variables
    mut total = 0
    mut count = 0

    // Loop to accumulate
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

In multi-line functions, you can use `#` to write comments, declare `mut` local variables, and use `for` and `if` to build logic.

---

## `pub` and Automatic Binding

In a module, functions declared with the `pub` keyword can be imported and used by other modules. Even more interestingly, **`pub` functions are automatically bound to types defined in the same file**, allowing you to call them in an OOP style.

```yaoxiang
// point.yx

// Define a type
Point: Type = { x: Float, y: Float }

// pub function: the compiler automatically binds it as Point.distance
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

When the compiler sees `pub distance(p1: Point, p2: Point)`, and finds that `Point` is defined in the same file, it automatically creates the `Point.distance` binding. You don't need to write any extra `impl` code.

---

## Quick Reference

```yaoxiang
// ── Function Definition Syntax Overview ──

// Expression form (most common)
add: (a: Int, b: Int) -> Int = a + b

// Code block form (multi-step logic)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Parameterless function (shortest)
main = { print("Hello!") }

// With parameters—omit signature
double = (x: Int) => x * 2

// With parameters—omit lambda header (recommended)
triple: (x: Int) -> Int = x * 3

// pub export + automatic binding
pub add: (a: Int, b: Int) -> Int = a + b

// ── Calling Syntax ──

result = add(1, 2)          // Positional arguments
result = add(a: 1, b: 2)    // Named arguments
result = add(1, b: 2)       // Mixed (positional first)
```

---

## Summary

You've now mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, sharing the same origin as the `name: type = value` variable declaration
- **Expression form**: `= expr`, the expression value serves directly as the return value, no `return` needed
- **Code block form**: `= { ...; return expr }`, must use `return` explicitly inside the block
- **Parameter type annotation**: types must be written in at least one of the signature or lambda header; recommended to write them in the signature
- **Calling**: positional arguments or named arguments; named arguments can be in any order
- **`pub` automatic binding**: `pub` functions are automatically bound to types in the same file, supporting `obj.method()` calls
- **Shortest parameterless form**: `name = { ... }`, the compiler automatically infers it as `() -> Void`

Next, you can continue to the [Control Flow](./control-flow.md) chapter to learn how to use `if`, `for`, and `while` in functions.