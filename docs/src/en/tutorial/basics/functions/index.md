---
title: 'Function Definition and Calling'
---

# Function Definition and Calling

In the previous chapter, you learned how to declare variables. This chapter will walk you through
the core of YaoXiang—functions. YaoXiang's function syntax shares the same `name: type = value`
model as variable declarations, so it should feel familiar.

## Functions Are Lambdas

The most important concept: **In YaoXiang, functions are essentially lambda expressions**. There is
no special `fn` keyword, no complicated ceremony. Defining a function means giving a name to a
lambda.

```
# Any function is essentially a combination of these four parts:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- function body (lambda expression or code block)
 |       |        +-- return type
 |       +-- parameter list (signature)
 +-- function name
```

This is exactly the same as `name: type = value` you learned in the previous chapter—the "type" just
happens to be a function type.

---

## Expression Form: Direct Return Value

The simplest function doesn't need the `return` keyword. When the function body is a single
expression, it serves directly as the return value:

```yaoxiang
// Expression form—direct return, no return needed
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
block), its value serves directly as the function's return value. There's no need to write
`return`—doing so would be a syntax error.

```yaoxiang
// Correct: the expression directly serves as the return value
double: (x: Int) -> Int = x * 2

// Wrong: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Code Block Form: Explicit return

When a function contains multi-step computation, wrap the body in a `{ }` code block. **In a code
block, you must use the `return` statement to return a value**:

```yaoxiang
// Code block form—must use return to return a value
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// Compute result
f5 = factorial(5)        // f5 = 120
```

The rule is simple: **expression form directly returns; code block form must explicitly `return`**.
If you forget to write `return` in a code block, the function defaults to returning `Void`.

```yaoxiang
// Note: this function has a bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // No return! Block defaults to Void, but signature requires Int → type error
// }

// Correct version
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form            | Syntax                | Return Method                       |
| --------------- | --------------------- | ----------------------------------- |
| Expression form | `name: ... = expr`    | Expression value directly as return |
| Code block form | `name: ... = { ... }` | Must explicitly `return`            |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be annotated with a type:

```yaoxiang
// Two parameters, both type-annotated
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in the Signature or Lambda Header

YaoXiang's rule is: **When there are input parameters, the parameter types must appear explicitly in
at least one of the signature or the lambda header**. Omitting types on both sides will be rejected
by the compiler.

```yaoxiang
// Method 1: Parameter types in the signature (omit lambda header)
add: (a: Int, b: Int) -> Int = a + b

// Method 2: Parameter types in the lambda header (omit signature)
add = (a: Int, b: Int) => a + b

// Method 3: Full form (both signature and lambda header)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Wrong: types omitted on both sides
// add = (a, b) => a + b   // ❌ Compiler cannot infer parameter types
```

**Method 1 is recommended**—parameter types are written in the signature, and the lambda header is
omitted. This is the most concise and clearest style.

---

## Return Value

The function's return type is written after `->`. The `->` is the function type marker and cannot be
omitted (omitting it would be parsed as another type).

```yaoxiang
// Returns Int
add_one: (x: Int) -> Int = x + 1

// Returns String
to_string: (n: Int) -> String = n.to_string()

// Returns Void (no return value)
log: (msg: String) -> Void = {
    print(msg)    // No return, defaults to Void
}
```

The return type can also be omitted, letting HM type inference handle it:

```yaoxiang
// Compiler infers return type as Int
add = (a: Int, b: Int) => a + b

// Compiler infers return type as String
greet = (name: String) => "Hello, " + name
```

---

## Function Calls

### Positional Arguments

The most basic calling style—pass arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

The formal definition of a function call in the syntax spec is:

```
Expr '(' ArgList? ')'
```

In everyday language: an expression followed by a pair of parentheses, which may contain an argument
list.

### Named Arguments

Beyond positional arguments, YaoXiang also supports **named arguments**—specifying values by
parameter name, in any order:

```yaoxiang
// Named arguments—parameter name followed by equals, then value
result = add(a = 3, b = 5)     // result = 8
result = add(b = 5, a = 3)     // Any order, same result

// Can be mixed with positional arguments, but positional ones must come first
result = add(3, b = 5)        // OK
```

Named arguments make calls more readable, especially when there are many parameters:

```yaoxiang
// Function signature
send: (to: String, title: String, body: String) -> String = to + "|" + title + "|" + body

// Named arguments make the call's intent immediately clear
msg = send(
    to = "alice@example.com",
    title = "Meeting Notice",
    body = "Meeting at 3 PM tomorrow"
)
```

Misspelled or duplicated parameter names will cause a compile-time error, never silently fall back
to positional binding:

```yaoxiang
// ❌ add has no parameter named c → E1014
result = add(b = 5, c = 1)

// ❌ a is passed both positionally and by name → E1015
result = add(1, a = 2)

// ❌ One argument missing → E1010
result = add(a = 1)
```

---

## Parameterless Functions

Functions that take no parameters can omit the parameter list:

```yaoxiang
// Full form: explicit empty parameter list
hello: () -> Void = {
    print("Hello!")
}

// Simplest form: omit signature, compiler infers () -> Void
hello = {
    print("Hello!")
}

// Calling a parameterless function
hello()
```

The `main` function is the most common parameterless function:

```yaoxiang
// A few ways to write main

// Full form
main: () -> Void = {
    print("Hello, YaoXiang!")
}

// Simplest form (recommended)
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

---

## Multi-line Functions

When function logic gets complex, organize the code with the code block form. YaoXiang enforces
4-space indentation:

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

    // Return average
    return total:as(Float) / count:as(Float)
}
```

In multi-line functions, you can use `#` for comments, declare `mut` local variables, and use `for`
and `if` to build logic.

---

## pub and Automatic Binding

In a module, functions declared with the `pub` keyword can be imported and used by other modules.
Even more interesting is that **`pub` functions are automatically bound to types defined in the same
file**, allowing OOP-style calls.

```yaoxiang
// point.yx

// Define a type
Point: Type = { x: Float, y: Float }

// pub function: compiler automatically binds it as Point.distance
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
same file and automatically creates a `Point.distance` binding. You don't need to write any extra
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

// pub export + automatic binding
pub add: (a: Int, b: Int) -> Int = a + b

// ── Calling Syntax ──

result = add(1, 2)          // Positional arguments
result = add(a = 1, b = 2)   // Named arguments
result = add(1, b = 2)      // Mixed (positional first)
```

---

## Summary

You've now mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, sharing the same origin as variable
  declaration's `name: type = value`
- **Expression form**: `= expr`, the expression value directly serves as the return value, no
  `return` needed
- **Code block form**: `= { ...; return expr }`, must explicitly `return` inside the block
- **Parameter type annotation**: Types must appear in at least the signature or the lambda header;
  recommended to put them in the signature
- **Calling**: Positional or named arguments; named arguments can be in any order
- **pub automatic binding**: `pub` functions are automatically bound to types in the same file,
  supporting `obj.method()` calls
- **Simplest parameterless**: `name = { ... }`, compiler infers `() -> Void`

Next, you can continue to the
[Control Flow](../../../design/formatter/formatting-rules/control-flow.md) chapter to learn how to
use `if`, `for`, and `while` inside functions.
