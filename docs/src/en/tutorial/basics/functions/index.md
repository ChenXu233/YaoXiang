---
title: 'Function Definition and Invocation'
---

# Function Definition and Invocation

In the previous chapter, you learned how to declare variables. This chapter will walk you through
the core of YaoXiang—functions. YaoXiang's function syntax shares the same `name: type = value`
model as variable declarations, so it should feel familiar.

## Functions Are Lambdas

Here's the most important concept up front: **In YaoXiang, a function is essentially a lambda
expression**. There's no special `fn` keyword, no complicated ceremony. Defining a function is just
giving a lambda a name.

```
# Any function is essentially a combination of these four things:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- function body (lambda expression or code block)
 |       |        +-- return type
 |       +-- parameter list (signature)
 +-- function name
```

This is completely consistent with the `name: type = value` you learned in the previous chapter—it's
just that the "type" here happens to be a function type.

---

## Expression Form: Direct Return Value

The simplest functions don't need a `return` keyword. When the function body is a single expression,
it serves as the return value directly:

```yaoxiang
// Expression form—direct return value, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "Hello, " + name
```

Call them:

```yaoxiang
sum = add(3, 5)          // sum = 8
sq = square(4)           // sq = 16
msg = greet("World")     // msg = "Hello, World"
```

This is called the **expression form**. When the function body is an expression (not a `{ }` code
block), its value is used directly as the function's return value. You don't need to write
`return`—writing it would actually be wrong.

```yaoxiang
// Correct: expression directly serves as the return value
double: (x: Int) -> Int = x * 2

// Wrong: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Code Block Form: Explicit return

When a function contains multi-step computation, wrap the function body in a `{ }` code block.
**Inside a code block, you must use a `return` statement to return a value**:

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

The rule is simple: **the expression form directly returns the value; the code block form requires
an explicit `return`**. If you forget to write `return` in a code block, the function defaults to
returning `Void`.

```yaoxiang
// Note: this function has a bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // No return! Block defaults to returning Void, but the signature requires Int → type error
// }

// Correct写法
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form            | Syntax                | Return Value Method            |
| --------------- | --------------------- | ------------------------------ |
| Expression Form | `name: ... = expr`    | Expression value used directly |
| Code Block Form | `name: ... = { ... }` | Must use explicit `return`     |

---

## Parameter Definition

### Basic Parameters

Parameters are written in the function signature, and each parameter can be annotated with a type:

```yaoxiang
// Two parameters, both annotated with types
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in Either the Signature or the Lambda Head

YaoXiang's rule is: **when there are input parameters, the parameter type must appear explicitly in
at least one of the signature or the lambda head**. Omitting types on both sides will be rejected by
the compiler.

```yaoxiang
// Method 1: parameter types in the signature (omitting lambda head)
add: (a: Int, b: Int) -> Int = a + b

// Method 2: parameter types in the lambda head (omitting signature)
add = (a: Int, b: Int) => a + b

// Method 3: complete form (both signature and lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Wrong: no types on either side
// add = (a, b) => a + b   // ❌ Compiler cannot infer parameter types
```

**Method 1 is recommended**—write parameter types in the signature and omit the lambda head. This is
the most concise and clearest写法.

---

## Return Value

The function's return type is written after `->`. `->` is the marker of a function type and cannot
be omitted (if omitted, it will be parsed as another type).

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

The return type can also be omitted and let HM type inference handle it:

```yaoxiang
// Compiler infers return type as Int
add = (a: Int, b: Int) => a + b

// Compiler infers return type as String
greet = (name: String) => "Hello, " + name
```

---

## Function Invocation

### Positional Arguments

The most basic invocation—passing arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

The formal definition of function invocation in the syntax specification is:

```
Expr '(' ArgList? ')'
```

In everyday language: an expression followed by a pair of parentheses, with an optional argument
list inside.

### Named Arguments

In addition to positional arguments, YaoXiang also supports **named arguments**—specifying values by
parameter name, in any order:

```yaoxiang
// Named arguments—parameter name followed by colon, then value
result = add(a: 3, b: 5)     // result = 8
result = add(b: 5, a: 3)     // Any order, same result

// Can be mixed with positional arguments, but positional arguments must come first
result = add(3, b: 5)        // OK
```

Named arguments make calls more readable, especially useful when there are many parameters:

```yaoxiang
// Function signature
send: (to: String, title: String, body: String) -> Void = {
    print("To: " + to)
    print("Title: " + title)
    print("Body: " + body)
}

// Named arguments make the call's intent clear at a glance
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
// Complete form: explicit empty parameter list
hello: () -> Void = {
    print("Hello!")
}

// Most concise form: omit signature, compiler infers as () -> Void
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

In modules, functions declared with the `pub` keyword can be imported and used by other modules.
Even more interesting, **`pub` functions are automatically bound to types defined in the same
file**, allowing you to call them in OOP style.

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

When the compiler sees `pub distance(p1: Point, p2: Point)`, it notices that `Point` is defined in
the same file, and automatically creates a `Point.distance` binding. You don't need to write any
extra `impl` code.

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

// Parameterless function (most concise)
main: () -> Void = { print("Hello!") }

// With parameters—omitting signature
double = (x: Int) => x * 2

// With parameters—omitting lambda head (recommended)
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

- **Unified syntax**: `name: (params) -> Return = body`, originating from the same source as
  `name: type = value` for variable declarations
- **Expression form**: `= expr`, the expression value is used directly as the return value, no
  `return` needed
- **Code block form**: `= { ...; return expr }`, the block must use an explicit `return`
- **Parameter type annotation**: write the type in at least one of the signature or lambda head;
  recommended in the signature
- **Invocation**: positional or named arguments; named arguments can be in any order
- **pub automatic binding**: `pub` functions are automatically bound to types in the same file,
  supporting `obj.method()` calls
- **Simplest parameterless form**: `name = { ... }`, compiler automatically infers as `() -> Void`

Next, you can continue with the [Control Flow](./control-flow.md) chapter to learn how to use `if`,
`for`, and `while` in functions.
