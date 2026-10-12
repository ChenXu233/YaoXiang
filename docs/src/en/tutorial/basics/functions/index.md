---
title: 'Function Definition and Call'
---

# Function Definition and Call

In the previous chapter, you learned how to declare variables. This chapter will guide you through
the core of YaoXiang—functions. YaoXiang's function syntax shares the same `name: type = value`
model as variable declarations, so you should find it familiar.

## Functions Are Lambdas

Let's start with the most important concept: **In YaoXiang, functions are essentially lambda
expressions**. No special `fn` keyword, no complex ceremony. To define a function is to give a
lambda a name.

```
# Any function is essentially a combination of these four things:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- Function body (lambda expression or code block)
 |       |        +-- Return type
 |       +-- Parameter list (signature)
 +-- Function name
```

This is exactly the same as the `name: type = value` you learned in the previous chapter—except that
the "type" here happens to be a function type.

---

## Expression Form: Direct Return Value

The simplest functions don't need the `return` keyword. When the function body is a single
expression, it is used directly as the return value:

```yaoxiang
// Expression form—direct return value, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "你好, " + name
```

Calling them:

```yaoxiang
main = () => {
    // The three functions defined in the previous section are rebound here for independent execution
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

This is called the **expression form**. When the function body is an expression (not a `{ }` code
block), its value is used directly as the function's return value. There's no need to write
`return`; writing it would actually be an error.

```yaoxiang
// Correct: expression used directly as the return value
double: (x: Int) -> Int = x * 2

// Error: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Block Form: Explicit return

When a function involves multiple steps, wrap the body in a `{ }` code block. **Inside a code block,
you must use the `return` statement to return a value**:

```yaoxiang
// Block form—must use return to return a value
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// Computed result
f5 = factorial(5)        // f5 = 120
```

The rule is simple: **expression form returns directly; block form requires an explicit `return`**.
If you forget to write `return` in a code block, the function defaults to returning `Void`.

```yaoxiang
// Note: this function has a bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // No return! The block defaults to returning Void, but the signature requires Int → type error
// }

// Correct way
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form            | Syntax                | Return method                            |
| --------------- | --------------------- | ---------------------------------------- |
| Expression form | `name: ... = expr`    | Expression value used directly as return |
| Block form      | `name: ... = { ... }` | Must use an explicit `return`            |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be annotated with a type:

```yaoxiang
// Two parameters, both with types annotated
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in Either the Signature or the Lambda Head

YaoXiang's rule is: **when there are input parameters, the parameter types must appear explicitly in
at least one of the signature or the lambda head**. Omitting both will be rejected by the compiler.

```yaoxiang
// Method 1: parameter types written in the signature (lambda head omitted)
add: (a: Int, b: Int) -> Int = a + b
```

```yaoxiang
// Method 2: parameter types written in the lambda head (signature omitted)
add = (a: Int, b: Int) => a + b
```

```yaoxiang
// Method 3: complete form (both signature and lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b
```

In all three forms, the parameter types appear at least once:

```yaoxiang
// Error: no types on either side
// add = (a, b) => a + b   // ❌ Compiler cannot infer parameter types
```

**Method 1 is recommended**—write the parameter types in the signature and omit the lambda head.
This is the most concise and clearest form.

---

## Return Value

The function's return type is written after `->`. The `->` is the marker of a function type and
cannot be omitted (omitting it would cause it to be parsed as another type).

```yaoxiang
use std.string

// Return Int
add_one: (x: Int) -> Int = x + 1

// Return String
// Note: Int does not have a to_string() method (`n.to_string()` reports E1053),
// use std.string.format or an f-string to convert
to_str: (n: Int) -> String = string.format("{0}", n)

// Return Void (no return value)
log: (msg: String) -> Void = {
    print(msg)    // No return, defaults to returning Void
}
```

The return type can also be omitted and let HM type inference handle it for you:

```yaoxiang
// Compiler infers the return type as Int
add = (a: Int, b: Int) => a + b

// Compiler infers the return type as String
greet = (name: String) => "你好, " + name
```

---

## Function Calls

### Positional Arguments

The most basic calling convention—passing arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

In the language specification, the formal definition of a function call is:

```
Expr '(' ArgList? ')'
```

Translated into everyday language: an expression followed by a pair of parentheses, which may
contain an argument list.

### Named Arguments

In addition to positional arguments, YaoXiang also supports **named arguments**—specifying values by
parameter name, in any order:

```yaoxiang
main = () => {
    add = (a: Int, b: Int) => a + b

    // Named arguments—parameter name followed by an equals sign, then the value
    result = add(a = 3, b = 5)     // result = 8
    result2 = add(b = 5, a = 3)    // Any order, same result

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

Misspelled parameter names or duplicates cause compile-time errors rather than silently falling back
to positional:

<!-- docs-example: skip -->

```yaoxiang
// ❌ add has no parameter named c → E1014
add: (a: Int, b: Int) -> Int = a + b
result = add(b = 5, c = 1)

// ❌ a is passed both positionally and by name → E1015
result2 = add(1, a = 2)

// ❌ Missing one argument → E1010
result3 = add(a = 1)
```

> The three cases above are **intentionally failing checks**—they demonstrate the boundaries of
> named arguments with real compile errors.

---

## Parameterless Functions

For functions that don't need parameters, the parameter list can be omitted:

```yaoxiang
// Complete form: explicitly declare an empty parameter list
hello: () -> Void = {
    print("Hello!")
}

// Simplest form: omit the signature, compiler infers () -> Void
hello = {
    print("Hello!")
}

// Calling a parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
// Complete form of main: explicitly declare empty parameter list and return type
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Simplest form (recommended)—omit the signature, compiler infers `() -> Void`:

```yaoxiang
main = {
    print("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When function logic is complex, organize the code in block form. YaoXiang enforces 4-space
indentation:

```yaoxiang
// Multi-step computation
// Note: the parameter type is written as Vec(Int) rather than List(Int)—values annotated with `List(T)`
// are not currently accepted by for / indexing (E1002)
calculate_stats: (numbers: Vec(Int)) -> Float = {
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

    // Return the average (note: `:as(Float)` parsing reports E0011, use Float conversion)
    return Float(total) / Float(count)
}

main: () -> Void = {
    print(calculate_stats([1, 2, 3, 4]))
}
```

In multi-line functions, you can use `#` to write comments, declare `mut` local variables, and use
`for` and `if` to build logic.

---

## Methods and Explicit Binding

Methods are not auto-generated by modifiers—**the method form is an explicit composition**: write
the first parameter as `self`, name it with `Type.method`.

```yaoxiang
// point.yx

use std.math

// Define the type
Point: Type = { x: Float, y: Float }

// Explicit method: the first parameter self determines the receiver
// Note: methods cannot be attached directly to expressions—`(dx*dx+dy*dy).sqrt()` reports E1053, use math.sqrt
Point.distance: (self: Point, other: Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    d = dx * dx + dy * dy
    return math.sqrt(d)
}

main: () -> Void = {
    p1 = Point(x=3.0, y=4.0)
    p2 = Point(x=1.0, y=2.0)

    d = p1.distance(p2)         // Dot call
    print(d)
}
```

At the call site, `p1.distance(p2)` is the method call for `Point.distance`; writing
`Point.distance(p1, p2)` reports E1010. The `self` in the signature must be written
explicitly—methods have no implicit binding, and no `impl` block is required.

---

## Quick Reference

```yaoxiang
// ── Function definition syntax overview ──

// Expression form (most common)
add: (a: Int, b: Int) -> Int = a + b

// Block form (multi-step logic)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Parameterless function (simplest)
main: () -> Void = { print("Hello!") }

// With parameters—omit signature
double = (x: Int) => x * 2

// With parameters—omit lambda head (recommended)
triple: (x: Int) -> Int = x * 3

// Top-level bindings are public by default—no pub/private-like modifiers
negate: (x: Int) -> Int = 0 - x

// ── Call syntax ──

result = add(1, 2)          // Positional arguments
result = add(a = 1, b = 2)   // Named arguments
result = add(1, b = 2)      // Mixed (positional first)
```

---

## Summary

You now have a grasp of the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, sharing the same origin as the variable
  declaration `name: type = value`
- **Expression form**: `= expr`, the expression value is used directly as the return value, no
  `return` needed
- **Block form**: `= { ...; return expr }`, must use `return` explicitly inside the block
- **Parameter type annotation**: write the type in at least one of the signature or lambda head,
  recommended to write in the signature
- **Calls**: positional or named arguments, named arguments can be in any order
- **Explicit method binding**: `Type.method: (self: T, …) -> R = {...}`, no implicit binding, use
  `.` at the call site
- **Simplest parameterless**: `name = { ... }`, compiler infers `() -> Void`

Next, you can continue with the
[Control Flow](../../../dev/design/formatter/formatting-rules/control-flow.md) chapter to learn how
to use `if`, `for`, and `while` inside functions.
