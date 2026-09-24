---
title: 'Function Definition and Calling'
---

# Function Definition and Calling

In the previous chapter, you learned how to declare variables. This chapter will take you through the core of YaoXiang—functions. YaoXiang's function syntax shares the same
`name: type = value` model with variable declarations, so it should feel familiar.

## Functions Are Lambdas

Let's start with the most important concept: **In YaoXiang, a function is essentially a lambda expression**. No special `fn` keyword, no complex ceremony. Defining a function is simply giving a name to a lambda.

```
# Any function is essentially a combination of these four parts:
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- Function body (lambda expression or code block)
 |       |        +-- Return type
 |       +-- Parameter list (signature)
 +-- Function name
```

This is completely consistent with the `name: type = value` you learned in the previous chapter—the only difference is that the "type" here happens to be a function type.

---

## Expression Form: Return Value Directly

The simplest functions don't need a `return` keyword. When the function body is a single expression, it serves directly as the return value:

```yaoxiang
// Expression form—return value directly, no return needed
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "Hello, " + name
```

Calling them:

```yaoxiang
sum = add(3, 5)          // sum = 8
sq = square(4)           // sq = 16
msg = greet("World")     // msg = "Hello, World"
```

This is called the **expression form**. When the function body is an expression (not a `{ }` code block), its value serves directly as the function's return value. No need to write `return`, and writing it would actually be wrong.

```yaoxiang
// Correct: expression serves directly as return value
double: (x: Int) -> Int = x * 2

// Wrong: writing return in expression form is a syntax error
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## Block Form: Explicit Return

When a function involves multi-step calculations, wrap the function body in a `{ }` code block. **In code blocks, you must use the `return` statement to return a value**:

```yaoxiang
// Block form—must use return to return a value
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// Calculate result
f5 = factorial(5)        // f5 = 120
```

The rule is simple: **expression form returns value directly; block form must use explicit `return`**. If you forget to write `return` in a block, the function defaults to returning `Void`.

```yaoxiang
// Note: this function has a bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // No return! Block defaults to Void, but signature requires Int → type error
// }

// Correct写法
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

Summary:

| Form         | Syntax               | Return Value Method           |
| ------------ | -------------------- | ----------------------------- |
| Expression   | `name: ... = expr`   | Expression value directly     |
| Block        | `name: ... = { ... }`| Must use explicit `return`    |

---

## Parameter Definitions

### Basic Parameters

Parameters are written in the function signature, and each parameter can have its type annotated:

```yaoxiang
// Two parameters, both with type annotations
multiply: (a: Int, b: Int) -> Int = a * b
```

### Parameter Types Must Be Annotated in Either Signature or Lambda Head

YaoXiang's rule is: **when there are input parameters, the parameter types must be explicitly shown in at least one of the signature or lambda head**. Omitting from both sides will be rejected by the compiler.

```yaoxiang
// Method 1: Parameter types in signature (omit lambda head)
add: (a: Int, b: Int) -> Int = a + b

// Method 2: Parameter types in lambda head (omit signature)
add = (a: Int, b: Int) => a + b

// Method 3: Full form (both signature and lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Wrong: no type on either side
// add = (a, b) => a + b   // ❌ compiler cannot infer parameter types
```

**Method 1 is recommended**—parameter types in the signature, omit the lambda head. This is the most concise and clearest style.

---

## Return Values

The return value type of a function is written after `->`. `->` is the function type marker and cannot be omitted (if omitted, it will be parsed as other types).

```yaoxiang
// Return Int
add_one: (x: Int) -> Int = x + 1

// Return String
to_string: (n: Int) -> String = n.to_string()

// Return Void (no return value)
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

The most basic calling method—passing arguments in order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

In the syntax specification, function call form is defined as:

```
Expr '(' ArgList? ')'
```

Translated to everyday language: an expression followed by a pair of parentheses, with an optional parameter list inside the parentheses.

### Named Arguments

In addition to positional arguments, YaoXiang also supports **named arguments**—specifying values by parameter name, order doesn't matter:

```yaoxiang
// Named arguments—parameter name followed by equals, then value
result = add(a = 3, b = 5)     // result = 8
result = add(b = 5, a = 3)     // any order, same result

// Can mix with positional, but positional must come first
result = add(3, b = 5)        // OK
```

Named arguments make calls more readable, especially useful when there are many parameters:

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

Misspelling a parameter name or specifying it twice will result in a compile-time error, not silently treating it as positional:

```yaoxiang
// ❌ add has no parameter named c → E1014
result = add(b = 5, c = 1)

// ❌ a is both passed positionally and by name → E1015
result = add(1, a = 2)

// ❌ One argument missing → E1010
result = add(a = 1)
```

---

## Functions with No Parameters

Functions that take no parameters can omit the parameter list:

```yaoxiang
// Full form: explicitly declare empty parameters
hello: () -> Void = {
    print("Hello!")
}

// Simplest form: omit signature, compiler infers () -> Void
hello = {
    print("Hello!")
}

// Call function with no parameters
hello()
```

The `main` function is the most common no-parameter function:

```yaoxiang
// Different ways to write main function

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

When function logic is more complex, use the block form to organize code. YaoXiang requires 4 spaces for indentation:

```yaoxiang
// Multi-step calculation
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

In multi-line functions, you can use `#` for comments, declare `mut` local variables, and use `for` and `if` to build logic.

---

## pub and Auto Binding

In a module, functions declared with the `pub` keyword can be imported and used by other modules. More interestingly, **`pub` functions automatically bind to types defined in the same file**, allowing OOP-style calls.

```yaoxiang
// point.yx

// Define type
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
d2 = p1.distance(p2)        // OOP style call (syntactic sugar)
```

When the compiler sees `pub distance(p1: Point, p2: Point)`, it finds that `Point` is defined in the same file and automatically creates a `Point.distance` binding. You don't need to write any extra `impl` code.

---

## Quick Reference

```yaoxiang
// ── Function Definition Syntax Overview ──

// Expression form (most common)
add: (a: Int, b: Int) -> Int = a + b

// Block form (multi-step logic)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// No parameters (simplest)
main: () -> Void = { print("Hello!") }

// With parameters—omit signature
double = (x: Int) => x * 2

// With parameters—omit lambda head (recommended)
triple: (x: Int) -> Int = x * 3

// pub export + auto binding
pub add: (a: Int, b: Int) -> Int = a + b

// ── Calling Syntax ──

result = add(1, 2)          // Positional arguments
result = add(a = 1, b = 2)   // Named arguments
result = add(1, b = 2)      // Mixed (positional first)
```

---

## Summary

You've mastered the core knowledge of YaoXiang functions:

- **Unified syntax**: `name: (params) -> Return = body`, derived from variable declaration `name: type = value`
- **Expression form**: `= expr`, expression value serves directly as return value, no `return` needed
- **Block form**: `= { ...; return expr }`, must use explicit `return` inside blocks
- **Parameter type annotations**: must write types in either signature or lambda head, recommended in signature
- **Calling**: positional or named arguments, named arguments can be in any order
- **pub auto binding**: `pub` functions automatically bind to types in the same file, supporting `obj.method()` calls
- **Simplest no-parameter**: `name = { ... }`, compiler automatically infers as `() -> Void`

Next, you can continue learning the [Control Flow](../../../design/formatter/formatting-rules/control-flow.md) chapter to understand how to use `if`, `for`, and `while` in functions.