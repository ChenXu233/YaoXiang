---
title: 'Function Definition and Invocation'
---

# Function Definition and Invocation

In the previous chapter, you learned how to declare variables. This chapter will lead you into the
core of YaoXiang—functions. YaoXiang's function syntax shares the same `name: type = value` model as
variable declarations, so it should feel familiar.

## Functions Are Lambdas

Here is the most important concept: **In YaoXiang, a function is essentially a lambda expression**.
There is no special `fn` keyword, no complicated ceremony. Defining a function means giving a lambda
a name.

```
# Any function is essentially a combination of these four things:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- function body (lambda expression or code block)
 |       |        +-- return value type
 |       +-- parameter list (signature)
 +-- function name
```

This is completely consistent with the `name: type = value` you learned in the previous chapter—it's
just that the "type" here happens to be a function type.

---

## Expression Form: Direct Return Value

The simplest function does not need the `return` keyword. When the function body is a single
expression, it directly serves as the return value:

```yaoxiang
// Expression form—direct return value, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "Hello, " + name
```

Calling them:

```yaoxiang
sum = add(3, 5)          // sum = 8
sq = square(4)           // sq = 16
msg = greet("World")       // msg = "Hello, World"
```

This is called the **expression form**. When the function body is an expression (not a `{ }` code
block), its value directly serves as the function's return value. There is no need to write
`return`; writing it would actually be wrong.

```yaoxiang
// Correct: the expression directly serves as the return value
double: (x: Int) -> Int = x * 2

// Wrong: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Code Block Form: Explicit return

When a function contains multiple steps of computation, wrap the function body in `{ }`. **In a code
block, you must use the `return` statement to return a value**:

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

The rule is simple: **the expression form directly returns the value; the code block form must
explicitly `return`**. If you forget to write `return` in a code block, the function returns `Void`
by default.

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

| Form            | Syntax                | Return Method                                            |
| --------------- | --------------------- | -------------------------------------------------------- |
| Expression form | `name: ... = expr`    | The expression value directly serves as the return value |
| Code block form | `name: ... = { ... }` | Must explicitly `return`                                 |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be annotated with a type:

```yaoxiang
// Two parameters, both annotated with types
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in Either the Signature or the Lambda Header

YaoXiang's rule is: **when there are input parameters, the parameter types must explicitly appear in
at least one of the signature or the lambda header**. Omitting them on both sides will be rejected
by the compiler.

```yaoxiang
// Method 1: parameter types written in the signature (lambda header omitted)
add: (a: Int, b: Int) -> Int = a + b

// Method 2: parameter types written in the lambda header (signature omitted)
add = (a: Int, b: Int) => a + b

// Method 3: full form (both signature and lambda header present)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Wrong: omitting types on both sides
// add = (a, b) => a + b   // ❌ The compiler cannot infer parameter types
```

**Method 1 is recommended**—write the parameter types in the signature, and omit the lambda header.
This is the most concise and clearest way to write it.

---

## Return Value

The function's return value type is written after `->`. `->` is the marker of a function type and
cannot be omitted (omitting it will cause it to be parsed as a different type).

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

The return value type can also be omitted and let HM type inference handle it:

```yaoxiang
// Compiler infers return type as Int
add = (a: Int, b: Int) => a + b

// Compiler infers return type as String
greet = (name: String) => "Hello, " + name
```

---

## Function Invocation

### Positional Arguments

The most basic way to call—pass arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

The formal definition of function invocation in the syntax specification is:

```
Expr '(' ArgList? ')'
```

In everyday language: an expression followed by a pair of parentheses, which may contain an argument
list.

### Named Arguments

In addition to passing arguments by position, YaoXiang also supports **named arguments**—using the
parameter name to specify a value, in any order:

```yaoxiang
// Named arguments—the parameter name is followed by an equals sign and then the value
result = add(a = 3, b = 5)     // result = 8
result = add(b = 5, a = 3)     // Any order, same result

// Can be mixed with positional arguments, but positional arguments must come first
result = add(3, b = 5)        // OK
```

Named arguments make calls more readable, which is especially useful when there are many parameters:

```yaoxiang
// Function signature
send: (to: String, title: String, body: String) -> String = to + "|" + title + "|" + body

// Named arguments make the call's intent clear at a glance
msg = send(
    to = "alice@example.com",
    title = "Meeting Notice",
    body = "Meeting tomorrow at 3 PM"
)
```

Mistyped parameter names or duplicate specifications will be reported as errors at compile-time,
never silently falling back to positional lookup:

```yaoxiang
// ❌ add has no parameter named c → E1014
result = add(b = 5, c = 1)

// ❌ a is passed both positionally and by name → E1015
result = add(1, a = 2)

// ❌ Missing one parameter → E1010
result = add(a = 1)
```

---

## Parameterless Functions

Functions that don't need parameters can omit the parameter list:

```yaoxiang
// Full form: explicit empty parameter declaration
hello: () -> Void = {
    print("Hello!")
}

// Minimal form: omit the signature, the compiler infers () -> Void automatically
hello = {
    print("Hello!")
}

// Call the parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
// Several ways to write the main function

// Full form
main: () -> Void = {
    print("Hello, YaoXiang!")
}

// Minimal form (recommended)
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When function logic is more complex, organize the code in code block form. YaoXiang enforces 4-space
indentation:

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

In multi-line functions, you can write comments with `#`, declare `mut` local variables, and build
logic with `for` and `if`.

---

## pub and Automatic Binding

In a module, functions declared with the `pub` keyword can be imported and used by other modules.
More interestingly, **`pub` functions are automatically bound to types defined in the same file**,
allowing you to call them in an OOP style.

```yaoxiang
// point.yx

// Define the type
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
d2 = p1.distance(p2)        // OOP-style call (syntax sugar)
```

When the compiler sees `pub distance(p1: Point, p2: Point)` and finds that `Point` is defined in the
same file, it automatically creates a `Point.distance` binding. You don't need to write any extra
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

// Parameterless function (minimal)
main: () -> Void = { print("Hello!") }

// With parameters—signature omitted
double = (x: Int) => x * 2

// With parameters—lambda header omitted (recommended)
triple: (x: Int) -> Int = x * 3

// pub export + automatic binding
pub add: (a: Int, b: Int) -> Int = a + b

// ── Invocation Syntax ──

result = add(1, 2)          // Positional arguments
result = add(a = 1, b = 2)   // Named arguments
result = add(1, b = 2)      // Mixed (positional first)
```

---

## Summary

You have now mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, derived from the same source as the
  variable declaration `name: type = value`
- **Expression form**: `= expr`, the expression value directly serves as the return value, no
  `return` needed
- **Code block form**: `= { ...; return expr }`, the block must explicitly `return`
- **Parameter type annotation**: types must be written in at least one of the signature or the
  lambda header; recommended to write in the signature
- **Invocation**: positional or named arguments; named arguments can be in any order
- **pub automatic binding**: `pub` functions are automatically bound to types in the same file,
  supporting `obj.method()` calls
- **Parameterless minimal form**: `name = { ... }`, the compiler automatically infers `() -> Void`

Next, you can continue to learn the [Control Flow](./control-flow.md) chapter to understand how to
use `if`, `for`, and `while` inside functions.
