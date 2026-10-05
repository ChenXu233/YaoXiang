---
title: 'Function Definition and Invocation'
---

# Function Definition and Invocation

In the previous chapter, you learned how to declare variables. This chapter will guide you through
the core of YaoXiang—functions. YaoXiang's function syntax shares the same `name: type = value`
model as variable declarations, so it should feel familiar.

## Functions Are Lambdas

Let's start with the most important concept: **in YaoXiang, a function is essentially a lambda
expression**. There's no special `fn` keyword, no complex ceremony. Defining a function is just
giving a lambda a name.

```
# Any function is essentially a combination of these four things:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- Function body (lambda expression or code block)
 |       |        +-- Return type
 |       +-- Parameter list (signature)
 +-- Function name
```

This is completely consistent with the `name: type = value` you learned in the previous
chapter—except the "type" here happens to be a function type.

---

## Expression Form: Direct Return

The simplest functions don't need the `return` keyword. When the function body is a single
expression, it is directly used as the return value:

```yaoxiang
// Expression form—direct return value, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "你好, " + name
```

Call them:

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
block), its value is directly used as the function's return value. There's no need to write
`return`—writing it would actually be a mistake.

```yaoxiang
// Correct: the expression is directly used as the return value
double: (x: Int) -> Int = x * 2

// Wrong: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Code Block Form: Explicit return

When a function contains multiple steps of computation, wrap the body in a `{ }` code block. **In a
code block, you must use the `return` statement to return a value**:

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

The rule is simple: **the expression form returns directly; the code block form requires an explicit
`return`**. If you forget to write `return` in a code block, the function defaults to returning
`Void`.

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

| Form            | Syntax                | Return Method                                     |
| --------------- | --------------------- | ------------------------------------------------- |
| Expression form | `name: ... = expr`    | The expression value is the return value directly |
| Code block form | `name: ... = { ... }` | Must use `return` to return explicitly            |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be annotated with a type:

```yaoxiang
// Two parameters, both annotated with types
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in the Signature or the Lambda Head

The rule in YaoXiang is: **when there are input parameters, the parameter type must appear
explicitly in at least one of the signature or the lambda head**. Omitting it on both sides will be
rejected by the compiler.

```yaoxiang
// Method 1: Parameter types written in the signature (omitting the lambda head)
add: (a: Int, b: Int) -> Int = a + b
```

```yaoxiang
// Method 2: Parameter types written in the lambda head (omitting the signature)
add = (a: Int, b: Int) => a + b
```

```yaoxiang
// Method 3: Complete form (both signature and lambda head have types)
add: (a: Int, b: Int) -> Int = (a, b) => a + b
```

In all three forms, the parameter types appear at least once:

```yaoxiang
// Wrong: types not written on either side
// add = (a, b) => a + b   // ❌ The compiler cannot infer the parameter types
```

**Method 1 is recommended**—write the parameter types in the signature and omit the lambda head.
This is the most concise and clearest way.

---

## Return Value

The return value type of a function is written after `->`. `->` is the marker of a function type and
cannot be omitted (if omitted, it will be parsed as a different type).

```yaoxiang
use std.string

// Returns Int
add_one: (x: Int) -> Int = x + 1

// Returns String
// Note: Int has no to_string() method (`n.to_string()` reports E1053),
// use std.string.format or f-string to convert
to_str: (n: Int) -> String = string.format("{0}", n)

// Returns Void (no return value)
log: (msg: String) -> Void = {
    print(msg)    // No return, defaults to returning Void
}
```

The return value type can also be omitted and let HM type inference handle it for you:

```yaoxiang
// The compiler infers the return type as Int
add = (a: Int, b: Int) => a + b

// The compiler infers the return type as String
greet = (name: String) => "你好, " + name
```

---

## Function Invocation

### Positional Arguments

The most basic call style—pass arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

The formal definition of function invocation in the grammar specification is:

```
Expr '(' ArgList? ')'
```

In everyday language: an expression followed by a pair of parentheses, with an optional argument
list inside.

### Named Arguments

In addition to passing arguments by position, YaoXiang also supports **named arguments**—specifying
values by parameter name, in any order:

```yaoxiang
main = () => {
    add = (a: Int, b: Int) => a + b

    // Named arguments—parameter name followed by equals sign, then the value
    result = add(a = 3, b = 5)     // result = 8
    result2 = add(b = 5, a = 3)    // Any order, same result

    // Can be mixed with positional arguments, but positional arguments must come first
    result3 = add(3, b = 5)        // OK
    print(result)
    print(result2)
    print(result3)
}
```

Named arguments make calls more readable, which is especially useful when there are many parameters:

```yaoxiang
main = () => {
    // Function signature
    send = (to: String, title: String, body: String) => to + "|" + title + "|" + body

    // Named arguments make the intent of the call clear at a glance
    msg = send(
        to = "alice@example.com",
        title = "会议通知",
        body = "明天下午 3 点开会"
    )
    print(msg)
}
```

Misspelling parameter names or specifying them more than once will cause a compile-time error; the
compiler will not silently fall back to positional matching:

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

> The three lines above **intentionally fail to compile**—they use real compile errors to
> demonstrate the boundaries of named arguments.

---

## Parameterless Functions

Functions that don't need parameters can omit the parameter list:

```yaoxiang
// Complete form: explicitly declare empty parameters
hello: () -> Void = {
    print("Hello!")
}

// Simplest form: omit the signature, the compiler infers () -> Void
hello = {
    print("Hello!")
}

// Call a parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
// Complete form of main: explicitly declare empty parameters and return type
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Simplest form (recommended)—omit the signature, the compiler infers `() -> Void`:

```yaoxiang
main = {
    print("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When a function's logic is more complex, use the code block form to organize the code. YaoXiang
enforces 4-space indentation:

```yaoxiang
// Multi-step computation
// Note: parameter type is written as Vec(Int) instead of List(Int)—values annotated with `List(T)`
// are currently not accepted by for / subscript (E1002)
calculate_stats: (numbers: Vec(Int)) -> Float = {
    // Declare local variables
    mut total = 0
    mut count = 0

    // Accumulate via a loop
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

In multi-line functions, you can use `#` for comments, declare `mut` local variables, and use `for`
and `if` to build logic.

---

## pub and Automatic Binding

Within a module, functions declared with the `pub` keyword can be imported and used by other
modules. More interestingly, **`pub` functions are automatically bound to types defined in the same
file**, allowing you to call them in an OOP style.

```yaoxiang
// point.yx

use std.math

// Define a type
Point: Type = { x: Float, y: Float }

// pub function: the compiler automatically binds it as Point.distance
// Note: methods cannot be attached directly to expressions—`(dx*dx+dy*dy).sqrt()` reports E1053, use math.sqrt
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    d = dx * dx + dy * dy
    return math.sqrt(d)
}

main: () -> Void = {
    // Both call styles work
    p1 = Point(x=3.0, y=4.0)
    p2 = Point(x=1.0, y=2.0)

    d1 = distance(p1, p2)       // Functional call
    d2 = p1.distance(p2)        // OOP-style call (syntactic sugar)
    print(d1)
    print(d2)
}
```

When the compiler sees `pub distance(p1: Point, p2: Point)` and finds that `Point` is defined in the
same file, it automatically creates the `Point.distance` binding. You don't need to write any extra
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

// With parameters—omitting the signature
double = (x: Int) => x * 2

// With parameters—omitting the lambda head (recommended)
triple: (x: Int) -> Int = x * 3

// pub export + automatic binding
pub negate: (x: Int) -> Int = 0 - x

// ── Invocation Syntax ──

result = add(1, 2)          // Positional arguments
result = add(a = 1, b = 2)   // Named arguments
result = add(1, b = 2)      // Mixed (positional first)
```

---

## Summary

You've now mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, sharing the same origin as the
  `name: type = value` variable declaration
- **Expression form**: `= expr`, the expression value is the return value directly, no `return`
  needed
- **Code block form**: `= { ...; return expr }`, must use `return` explicitly inside the block
- **Parameter type annotation**: types must be written in at least one of the signature or the
  lambda head; recommended to put them in the signature
- **Invocation**: positional or named arguments; named arguments can be in any order
- **pub automatic binding**: `pub` functions are automatically bound to types in the same file,
  supporting `obj.method()` calls
- **Parameterless simplest form**: `name = { ... }`, the compiler infers `() -> Void`

Next, you can continue to the
[Control Flow](../../../dev/design/formatter/formatting-rules/control-flow.md) chapter to learn how
to use `if`, `for`, and `while` inside functions.
