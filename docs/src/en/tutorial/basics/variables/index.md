---
title: 'Variable Declaration'
---

# Variable Declaration

This chapter introduces the core syntax of variable declaration in YaoXiang. If you have experience
with other programming languages, you'll find YaoXiang's variable system very concise—every
declaration shares the same syntactic model.

## Unified Syntax Model

YaoXiang's design philosophy is "everything is unified". Whether you're declaring an integer,
defining a function, or creating a type, they all use the same syntax:

```
name: type = value
```

This is YaoXiang's most fundamental design concept. A few examples will convey this consistency:

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

In plain language: there can be an optional `mut` keyword, followed by the variable name, then an
optional `: Type`, and finally `= value`. This structure runs through the entire language—learn it
once and you're done.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This
is a safety design of the language.

```yaoxiang
x = 10
// x = 20   // Compile error! x is immutable
```

For variables declared with `=`, the compiler searches up the scope chain for a variable with the
same name. If one is found, it tries to assign to it; if not, it creates a new immutable variable in
the current scope.

```yaoxiang
x = 1       // No x in outer scope, so declared as a new variable
x = 2       // Found outer x, tries to assign → Compile error! x is immutable
```

This may seem counterintuitive—if you've learned other languages, you might wonder "if it can be
found, why can't I assign to it?". This is because YaoXiang puts safety first: immutability by
default means you don't have to worry about some variable being accidentally modified in some corner
of your code.

## mut Mutable Variables

When you actually need to modify a variable, use the `mut` keyword to declare it explicitly:

```yaoxiang
mut counter = 0
counter = counter + 1   // Can be modified
counter = 100           // Also fine
```

`mut` has several important rules:

**Rule One**: `mut` is an explicit new declaration—the compiler does not search outer scopes for a
variable with the same name.

```yaoxiang
mut x = 10      // Creates a new mutable variable x in the current scope
mut x = 20      // Compile error! x is already declared in the same scope
```

**Rule Two**: A variable declared with `mut` cannot share a name with a variable in an outer scope
(shadowing is forbidden).

```yaoxiang
x = 10
{
    mut x = 20   // Compile error! x is already declared in the outer scope, shadowing is not allowed
}
```

**Rule Three**: Within the same scope, each name can only be declared once—whether you use `=` or
`mut`.

```yaoxiang
x = 10
mut x = 20   // Compile error! x is already declared
```

These rules ensure that each variable name is unique within the current scope, so you'll never face
the confusion of "which variable does this name actually refer to?".

## Type Inference vs. Explicit Type Annotations

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can infer types from
the values you write, so in most cases you don't need to specify types manually.

```yaoxiang
x = 42              // Compiler infers Int
name = "YaoXiang"   // Infers String
pi = 3.14159        // Infers Float
is_valid = true     // Infers Bool
```

When you want to annotate types explicitly (for example, to improve code readability, or when the
compiler cannot infer), use the `: Type` syntax:

```yaoxiang
count: Int = 100
greeting: String = "Hello"
ratio: Float = 0.618
```

The two writing styles are completely equivalent. You can start writing code without types and add
type annotations later when needed. This makes prototyping very fast while still preserving type
safety in your final code.

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
escape = "Hello\nWorld" // Escape sequences supported: \n newline, \t tab, \\ backslash, \" double quote
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
    print("Start processing")
}
```

## Variable Scope

Scope determines the visibility range of a variable. YaoXiang's scope rules are very simple: **each
`{}` block creates a new scope**.

### Basic Rules

```yaoxiang
{
    x = 10
    print(x)   // Accessible: x is in the current scope
}
// print(x)    // Error: x is not visible outside the scope
```

Inner scopes can access variables from outer scopes:

```yaoxiang
outer = "I am outside"
{
    print(outer)   // Can access outer 'outer'
    inner = "I am inside"
}
// print(inner)    // Error: 'inner' is not visible outside the scope
```

### Function Parameter Scope

```yaoxiang
greet: (name: String) -> Void = {
    print("Hello, " + name)
    // 'name' is visible inside this function body
}
// 'name' is not visible outside the function
```

### Block Expressions

A `{}` block in YaoXiang is also an expression and can return a value:

```yaoxiang
result = {
    x = 10
    y = 20
    return x + y   // Returns 30 to the enclosing scope
}
// The value of 'result' is 30
// 'x' and 'y' are not visible outside the block
```

For detailed information about block return values, please refer to the function chapter. For now,
you only need to remember: **braces create a scope; inner scopes can see outer ones, but outer
scopes cannot see inner ones**.

## Summary

You have now mastered the core concepts of YaoXiang's variable system:

| Concept              | Key Points                                                                 |
| -------------------- | -------------------------------------------------------------------------- |
| Unified syntax model | `name: type = value`, used for variables, functions, and types             |
| Immutable by default | `x = 10` and then `x` cannot be changed                                    |
| Mutable variables    | Use `mut` to explicitly declare `mut x = 10`                               |
| No shadowing         | The same name can only be declared once in the same scope                  |
| Type inference       | HM algorithm infers automatically; `: Type` can also be written explicitly |
| Scope                | Each `{}` creates a scope; inner can see outer, outer cannot see inner     |

Next, you can continue learning about
[basic types](../../../design/formatter/formatting-rules/types.md) in more detail, or jump straight
into the [control flow](../../../design/formatter/formatting-rules/control-flow.md) chapter.
