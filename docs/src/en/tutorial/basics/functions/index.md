---
title: 'Function Definition and Invocation'
---

# Function Definition and Invocation

In the previous chapter, you learned how to declare variables. This chapter will lead you to master
the core of YaoXiang—functions. YaoXiang's function syntax shares the same `name: type = value`
model with variable declarations, so it should feel familiar.

## Functions Are Lambdas

Let's start with the most important concept: **In YaoXiang, functions are essentially lambda
expressions**. No special `fn` keyword, no complex ceremony. Defining a function is just giving a
lambda a name.

```
# Any function is essentially a combination of these four things:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- Function body (lambda expression or code block)
 |       |        +-- Return value type
 |       +-- Parameter list (signature)
 +-- Function name
```

This is exactly the same as the `name: type = value` you learned in the previous chapter—except here
the "type" happens to be a function type.

---

## Expression Form: Direct Return Value

The simplest functions don't need the `return` keyword. When the function body is a single
expression, it's used directly as the return value:

```yaoxiang
// Expression form—direct return value, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "你好, " + name
```

Call them:

```yaoxiang
main = () => {
    // The three functions defined in the previous section, rebound in this block for independent execution
    add = (a: Int, b: Int) => a + b
    square = (x: Int) => x * x
    greet = (name: String) => "你好, " + name

    sum = add(3, 5)          // sum = 8
    sq = square(4)           // sq = 16
    msg = greet("世界")       // msg = "你好, 世界"
    print(sum)
    print(sq)
    print(msg)
}
```

This is called **expression form**. When the function body is an expression (not a `{ }` code
block), its value is used directly as the function's return value. You don't need to write `return`;
writing it would actually be wrong.

```yaoxiang
// Correct: expression used directly as return value
double: (x: Int) -> Int = x * 2

// Error: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Code Block Form: Explicit return

When a function contains multiple steps of computation, wrap the function body in a `{ }` code
block. **In a code block, you must use a `return` statement to return a value**:

```yaoxiang
// Code block form—must use return to return a value
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// Calculate result
f5 = factorial(5)        // f5 = 120
```

The rules are simple: **Expression form returns the value directly; code block form must explicitly
`return`**. If you forget to write `return` in a code block, the function returns `Void` by default.

```yaoxiang
// Note: this function has a bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // No return! The block returns Void by default, but the signature requires Int → type error
// }

// Correct way
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form            | Syntax                | Return method                                  |
| --------------- | --------------------- | ---------------------------------------------- |
| Expression form | `name: ... = expr`    | Expression value used directly as return value |
| Code block form | `name: ... = { ... }` | Must explicitly return with `return`           |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be annotated with a type:

```yaoxiang
// Two parameters, both annotated with types
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in Either the Signature or the Lambda Header

YaoXiang's rule is: **When there are input parameters, the parameter types must explicitly appear in
at least one of the signature or the lambda header**. Omitting them on both sides will be rejected
by the compiler.

```yaoxiang
// Method 1: Parameter types written in the signature (omitting the lambda header)
add: (a: Int, b: Int) -> Int = a + b
```

```yaoxiang
// Method 2: Parameter types written in the lambda header (omitting the signature)
add = (a: Int, b: Int) => a + b
```

```yaoxiang
// Method 3: Complete form (both signature and lambda header)
add: (a: Int, b: Int) -> Int = (a, b) => a + b
```

In all three writing styles, the parameter types appear at least once:

```yaoxiang
// Error: omitting types on both sides
// add = (a, b) => a + b   // ❌ Compiler cannot infer parameter types
```

**Method 1 is recommended**—write the parameter types in the signature and omit the lambda header.
This is the most concise and clearest way to write it.

---

## Return Value

The function's return value type is written after `->`. The `->` is the marker for a function type
and cannot be omitted (omitting it would be parsed as another type).

```yaoxiang
use std.string

// Returns Int
add_one: (x: Int) -> Int = x + 1

// Returns String
// Note: Int has no to_string() method (`n.to_string()` reports E1053),
// use std.string.format or f-string for conversion
to_str: (n: Int) -> String = string.format("{0}", n)

// Returns Void (no return value)
log: (msg: String) -> Void = {
    print(msg)    // No return, returns Void by default
}
```

The return value type can also be omitted, letting HM type inference handle it for you:

```yaoxiang
// Compiler infers the return type as Int
add = (a: Int, b: Int) => a + b

// Compiler infers the return type as String
greet = (name: String) => "你好, " + name
```

---

## Function Calls

### Positional Arguments

The most basic calling style—passing arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

The formal definition of a function call in the grammar specification is:

```
Expr '(' ArgList? ')'
```

Translated into everyday language: an expression followed by a pair of parentheses, with an optional
argument list inside.

### Named Arguments

In addition to passing arguments by position, YaoXiang also supports **named arguments**—specifying
values by parameter name, in any order:

```yaoxiang
main = () => {
    add = (a: Int, b: Int) => a + b

    // Named arguments—parameter name followed by equals sign, then value
    result = add(a = 3, b = 5)     // result = 8
    result2 = add(b = 5, a = 3)    // Order is arbitrary, same result

    // Can be mixed with positional arguments, but positional arguments must come first
    result3 = add(3, b = 5)        // OK
    print(result)
    print(result2)
    print(result3)
}
```

Named arguments make calls more readable, especially useful when there are many parameters:

```yaoxiang
main = () => {
    // Function signature
    send = (to: String, title: String, body: String) => to + "|" + title + "|" + body

    // Named arguments make the call's intent clear at a glance
    msg = send(
        to = "alice@example.com",
        title = "会议通知",
        body = "明天下午 3 点开会"
    )
    print(msg)
}
```

Wrong parameter names or duplicate specifications will cause compile-time errors, not silently fall
back to positional lookup:

<!-- docs-example: skip -->

```yaoxiang
// ❌ add has no parameter named c → E1014
add: (a: Int, b: Int) -> Int = a + b
result = add(b = 5, c = 1)

// ❌ a is passed both positionally and by name → E1015
result2 = add(1, a = 2)

// ❌ One argument missing → E1010
result3 = add(a = 1)
```

> The three lines above are **intentionally not passing checks**—they use real compile errors to
> demonstrate the boundaries of named arguments.

---

## Parameterless Functions

Functions that don't need parameters can omit the parameter list:

```yaoxiang
// Complete form: explicitly declare empty parameters
hello: () -> Void = {
    print("Hello!")
}

// Simplest form: omit the signature, compiler infers () -> Void automatically
hello = {
    print("Hello!")
}

// Call the parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
// Complete form of main: explicitly declare empty parameters and return type
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Simplest form (recommended)—omit the signature, compiler infers `() -> Void` automatically:

```yaoxiang
main = {
    print("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When the function logic is complex, organize the code with the code block form. YaoXiang enforces
4-space indentation:

```yaoxiang
// Multi-step calculation
// Note: parameter type is written as Vec(Int) instead of List(Int)—values annotated
// with `List(T)` are currently not accepted by for / subscript (E1002)
calculate_stats: (numbers: Vec(Int)) -> Float = {
    // Declare local variables
    mut total = 0
    mut count = 0

    // Loop accumulation
    for n in numbers {
        total = total + n
        count = count + 1
    }

    // Avoid division by zero
    if count == 0 {
        return 0.0
    }

    // Return average (note: `:as(Float)` parsing reports E0011, use Float conversion)
    return Float(total) / Float(count)
}

main: () -> Void = {
    print(calculate_stats([1, 2, 3, 4]))
}
```

In multi-line functions, you can use `#` to write comments, declare `mut` local variables, and use
`for` and `if` to build logic.

---

## pub and Auto-binding

In modules, functions declared with the `pub` keyword can be imported and used by other modules.
Even more interestingly, **`pub` functions are automatically bound to types defined in the same
file**, allowing you to call them in OOP style.

```yaoxiang
// point.yx

use std.math

// Define type
Point: Type = { x: Float, y: Float }

// pub function: compiler automatically binds it as Point.distance
// Note: expressions cannot directly have methods—`(dx*dx+dy*dy).sqrt()` reports E1053, use math.sqrt
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    d = dx * dx + dy * dy
    return math.sqrt(d)
}

main: () -> Void = {
    // Both calling styles work
    p1 = Point(x=3.0, y=4.0)
    p2 = Point(x=1.0, y=2.0)

    d1 = distance(p1, p2)       // Functional call
    d2 = p1.distance(p2)        // OOP-style call (syntactic sugar)
    print(d1)
    print(d2)
}
```

When the compiler sees `pub distance(p1: Point, p2: Point)`, it finds that `Point` is defined in the
same file, and automatically creates the `Point.distance` binding. You don't need to write any extra
`impl` code.

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

// Parameterless function (simplest)
main: () -> Void = { print("Hello!") }

// With parameters—omit signature
double = (x: Int) => x * 2

// With parameters—omit lambda header (recommended)
triple: (x: Int) -> Int = x * 3

// pub export + auto-binding
pub negate: (x: Int) -> Int = 0 - x

// ── Call Syntax ──

result = add(1, 2)          // Positional arguments
result = add(a = 1, b = 2)   // Named arguments
result = add(1, b = 2)      // Mixed (positional first)
```

---

## Summary

You've mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, sharing the same origin as variable
  declaration's `name: type = value`
- **Expression form**: `= expr`, the expression value is used directly as the return value, no
  `return` needed
- **Code block form**: `= { ...; return expr }`, must explicitly return with `return` inside the
  block
- **Parameter type annotation**: write types in at least one of the signature or the lambda header,
  recommended to be written in the signature
- **Calling**: positional arguments or named arguments, named arguments can be in any order
- **pub auto-binding**: `pub` functions are automatically bound to types in the same file,
  supporting `obj.method()` calls
- **Simplest parameterless**: `name = { ... }`, compiler infers `() -> Void` automatically

Next, you can continue learning the
[Control Flow](../../../design/formatter/formatting-rules/control-flow.md) chapter to understand how
to use `if`, `for`, and `while` in functions.
