---
title: 'Variable Declarations'
---

# Variable Declarations

This chapter introduces the core syntax for variable declarations in YaoXiang. If you have
experience with other programming languages, you'll find YaoXiang's variable system very concise—all
declarations share the same syntactic model.

## Unified Syntax Model

YaoXiang's design philosophy is "everything is unified." Whether you're declaring an integer,
defining a function, or creating a type, they all use the same syntax:

```
name: type = value
```

This is YaoXiang's most central design concept. A few examples will give you a feel for this
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

The formal definition of a variable declaration in the language specification is:

```
('mut')? Identifier (':' TypeExpr)? '=' Expr
```

In plain words: there can be an optional `mut` keyword, followed by the variable name, then an
optional `: Type`, and finally `= value`. This structure runs throughout the entire language—learn
it once and you're set.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This
is a safety design choice in the language.

```yaoxiang
x = 10
// x = 20   // Compile error! x is immutable
```

When a variable is declared with `=`, the compiler searches outward along the scope chain for a
variable with the same name. If one is found, it assigns to it; if not, it creates a new immutable
variable in the current scope.

```yaoxiang
x = 1       // No x in the outer scope, so declared as a new variable
x = 2       // Found the outer x, attempts to assign → Compile error! x is immutable
```

This may feel a bit counterintuitive—if you've learned other languages, you might wonder "if it can
find it, why can't it assign?" That's because YaoXiang puts safety first: immutability by default
means you don't have to worry about some variable being accidentally modified in some corner of your
code.

## `mut` Mutable Variables

When you really need to modify a variable, use the `mut` keyword to declare it explicitly:

```yaoxiang
mut counter = 0
counter = counter + 1   // Can be modified
counter = 100           // Also fine
```

`mut` has several important rules:

**Rule One**: `mut` is an explicit new declaration; the compiler does not look up a variable with
the same name in the outer scope.

<!-- docs-example: skip -->

```yaoxiang
mut x = 10      // Creates a new mutable variable x in the current scope
mut x = 20      // Compile error! x has already been declared in the same scope
```

> The two lines above **intentionally fail the
> check**——`E2002 duplicate definition: 'x' is already defined in the current scope`.

**Rule Two**: A variable declared with `mut` cannot share its name with a variable in the outer
scope (shadowing is prohibited).

<!-- docs-example: skip -->

```yaoxiang
x = 10
{
    mut x = 20   // Compile error! x was already declared in the outer scope; shadowing is not allowed
}
```

> This snippet above **intentionally fails the check**——`E2013 Cannot shadow existing variable 'x'`.

**Rule Three**: Within the same scope, each name can only be declared once—whether you use `=` or
`mut`.

<!-- docs-example: skip -->

```yaoxiang
x = 10
mut x = 20   // Compile error! x has already been declared
```

> This snippet above **intentionally fails the check**——also `E2002`.

These rules ensure that each variable name is unique within the current scope, so you'll never be
confused about which variable a name actually points to.

## Type Inference vs. Explicit Type Annotations

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can automatically infer
types from the values you write, so in most cases you don't need to write types manually.

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

The two forms are completely equivalent. You can start writing code without specifying types, and
add type annotations later when needed. This makes prototyping very fast, while still preserving
type safety in the final code.

## Overview of Basic Types

YaoXiang has several built-in basic types that cover the vast majority of everyday programming
scenarios.

### Int (Integer)

```yaoxiang
a = 42              // Decimal
b = 0o52            // Octal (0o prefix)
c = 0x2A            // Hexadecimal (0x prefix)
d = 0b101010        // Binary (0b prefix)
e = 1_000_000       // Underscores can be used as digit separators for readability
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
escape = "Hello\nWorld" // Supports escapes: \n newline, \t tab, \\ backslash, \" double quote
unicode = "\u{4F60}\u{597D}"  // Unicode escape
```

### Bool (Boolean)

```yaoxiang
is_ready = true
is_done = false
```

Boolean values are typically used in conditionals:

```yaoxiang
if is_ready {
    print("Start processing")
}
```

## Variable Scope

Scope determines the visibility range of a variable. YaoXiang's scope rules are very simple: **every
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
outer = "I'm outside"
{
    print(outer)   // Can access outer
    inner = "I'm inside"
}
// print(inner)    // Error: inner is not visible outside the scope
```

### Function Parameter Scope

```yaoxiang
greet: (name: String) -> Void = {
    print("Hello, " + name)
    // name is visible inside this function body
}
// name is not visible outside the function
```

### Block Expressions

YaoXiang's `{}` blocks are also expressions that can return a value:

```yaoxiang
result = {
    x = 10
    y = 20
    return x + y   // Returns 30 to the enclosing scope
}
// result is 30
// x and y are not visible outside the block
```

For a detailed explanation of block return values, please refer to the later chapter on functions.
For now, you only need to remember: **braces create scopes; inner scopes can see outer ones, but
outer scopes cannot see inner ones**.

## Summary

You've now mastered the core concepts of YaoXiang's variable system:

| Concept              | Key Points                                                          |
| -------------------- | ------------------------------------------------------------------- |
| Unified syntax model | `name: type = value`—used for variables, functions, and types       |
| Immutable by default | After `x = 10`, `x` cannot be changed                               |
| Mutable variables    | Declare explicitly with `mut`, e.g. `mut x = 10`                    |
| No shadowing         | The same name can only be declared once within a single scope       |
| Type inference       | HM algorithm infers automatically; can also write `: Type`          |
| Scope                | Each `{}` creates a scope; inner sees outer, outer cannot see inner |

Next, you can continue learning more details about
[basic types](../../../dev/design/formatter/formatting-rules/types.md), or jump directly to the
[control flow](../../../dev/design/formatter/formatting-rules/control-flow.md) chapter.
