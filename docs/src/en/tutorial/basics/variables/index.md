---
title: 'Variable Declaration'
---

# Variable Declaration

This chapter introduces the core syntax of variable declarations in YaoXiang. If you have experience
with other programming languages, you will find that YaoXiang's variable system is very concise—all
declarations share the same syntactic model.

## Unified Syntactic Model

YaoXiang's design philosophy is "everything is unified." Whether you are declaring an integer,
defining a function, or creating a type, they all use the same syntax:

```
name: type = value
```

This is YaoXiang's most core design principle. A few examples will give you a feel for this
consistency:

```yaoxiang
// Variable declaration
x: Int = 42
name: String = "YaoXiang"

// Function definition
add: (a: Int, b: Int) -> Int = a + b

// Type definition
Point: Type = { x: Float, y: Float }
```

The formal definition of variable declaration in the syntax specification is:

```
('mut')? Identifier (':' TypeExpr)? '=' Expr
```

In plain terms: there can be an optional `mut` keyword, followed by a variable name, then an
optional `: type`, and finally `= value`. This structure runs throughout the entire language—learn
it once and you're done.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This
is a safety design of the language.

```yaoxiang
x = 10
// x = 20   // Compile error! x is immutable
```

When a variable is declared with `=`, the compiler looks up the scope chain outward for a variable
with the same name. If found, it tries to assign to it; if not found, it creates a new immutable
variable in the current scope.

```yaoxiang
x = 1       // No x in outer scope, so declare as new variable
x = 2       // Found outer x, try to assign → Compile error! x is immutable
```

This may seem a bit counter-intuitive—if you've learned other languages, you might think "if it can
be found, why can't I assign to it?" This is because YaoXiang puts safety first: immutability by
default means you don't have to worry about a variable being accidentally modified in some corner of
the code.

## `mut` Mutable Variables

When you really need to modify a variable, use the `mut` keyword to declare it explicitly:

```yaoxiang
mut counter = 0
counter = counter + 1   // Can modify
counter = 100           // Also can
```

`mut` has several important rules:

**Rule One**: `mut` is an explicit new declaration; the compiler will not look up a variable with
the same name in the outer scope.

<!-- docs-example: skip -->

```yaoxiang
mut x = 10      // Create new mutable variable x in current scope
mut x = 20      // Compile error! x is already declared in same scope
```

> The two lines above **deliberately fail the
> check**—`E2002 duplicate definition: 'x' is already defined in the current scope`.

**Rule Two**: A variable declared with `mut` cannot have the same name as a variable in the outer
scope (shadowing is prohibited).

<!-- docs-example: skip -->

```yaoxiang
x = 10
{
    mut x = 20   // Compile error! x is already declared in outer scope, shadowing not allowed
}
```

> The above code **deliberately fails the check**—`E2013 Cannot shadow existing variable 'x'`.

**Rule Three**: Within the same scope, each name can only be declared once—whether using `=` or
`mut`.

<!-- docs-example: skip -->

```yaoxiang
x = 10
mut x = 20   // Compile error! x is already declared
```

> The above code **deliberately fails the check**—also `E2002`.

These rules ensure that each variable name is unique in the current scope, so you'll never be
confused about which variable a name refers to.

## Type Inference vs. Explicit Type Annotations

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can automatically infer
types from the values you write, so in most cases you don't need to manually write types.

```yaoxiang
x = 42              // Compiler infers as Int
name = "YaoXiang"   // Infers as String
pi = 3.14159        // Infers as Float
is_valid = true     // Infers as Bool
```

When you want to explicitly annotate a type (for example, to improve code readability, or when the
compiler cannot infer), use the `: Type` syntax:

```yaoxiang
count: Int = 100
greeting: String = "Hello"
ratio: Float = 0.618
```

The two notations are completely equivalent. You can start by omitting types when writing code, and
add type annotations later when needed. This makes prototyping very fast, without sacrificing type
safety in the final code.

## Overview of Basic Types

YaoXiang has several built-in basic types that cover the vast majority of everyday programming
scenarios.

### Int (Integer)

```yaoxiang
a = 42              // Decimal
b = 0o52            // Octal (0o prefix)
c = 0x2A            // Hexadecimal (0x prefix)
d = 0b101010        // Binary (0b prefix)
e = 1_000_000       // Underscores can be used to separate digits for readability
```

### Float (Floating-Point)

```yaoxiang
pi = 3.14159
speed = 2.998e8         // Scientific notation: 2.998 × 10^8
tiny = 1.6e-19
```

### String

```yaoxiang
name = "YaoXiang"
empty = ""              // Empty string
escape = "Hello\nWorld" // Supports escape: \n newline, \t tab, \\ backslash, \" double quote
unicode = "\u{4F60}\u{597D}"  // Unicode escape
```

### Bool (Boolean)

```yaoxiang
is_ready = true
is_done = false
```

Boolean values are typically used in conditional judgments:

```yaoxiang
if is_ready {
    print("开始处理")
}
```

## Variable Scope

Scope determines the visibility of a variable. YaoXiang's scope rules are very simple: **each `{}`
block creates a new scope**.

### Basic Rules

```yaoxiang
{
    x = 10
    print(x)   // Can access: x is in current scope
}
// print(x)    // Error: x is not visible outside scope
```

An inner scope can access variables from the outer scope:

```yaoxiang
outer = "我在外面"
{
    print(outer)   // Can access outer outer
    inner = "我在里面"
}
// print(inner)    // Error: inner is not visible outside scope
```

### Function Parameter Scope

```yaoxiang
greet: (name: String) -> Void = {
    print("你好, " + name)
    // name is visible inside this function body
}
// name is not visible outside the function
```

### Block Expressions

A `{}` block in YaoXiang is also an expression and can return a value:

```yaoxiang
result = {
    x = 10
    y = 20
    return x + y   // Returns 30 to the enclosing scope
}
// result's value is 30
// x and y are not visible outside the block
```

For a detailed explanation of block return values, please refer to the subsequent function chapter.
All you need to remember here is: **braces create scope, the inner can see the outer, the outer
cannot see the inner**.

## Summary

You have now mastered the core concepts of YaoXiang's variable system:

| Concept                 | Key Points                                                                 |
| ----------------------- | -------------------------------------------------------------------------- |
| Unified syntactic model | `name: type = value`, used for variables, functions, and types             |
| Immutable by default    | `x` cannot change after `x = 10`                                           |
| Mutable variables       | Use `mut` to explicitly declare `mut x = 10`                               |
| No shadowing            | Same name can only be declared once in the same scope                      |
| Type inference          | HM algorithm infers automatically, or use `: Type` for explicit annotation |
| Scope                   | Each `{}` creates a scope, inner can see outer, outer cannot see inner     |

You can continue to learn more details about
[basic types](../../../dev/design/formatter/formatting-rules/types.md), or jump straight into the
[control flow](../../../dev/design/formatter/formatting-rules/control-flow.md) chapter.
