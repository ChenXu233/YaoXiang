---
title: 'Variable Declaration'
---

# Variable Declaration

This chapter introduces the core syntax for variable declaration in YaoXiang. If you have experience with other programming languages, you'll find YaoXiang's variable system very simple—all declarations share the same syntax model.

## Unified Syntax Model

YaoXiang's design philosophy is "everything is unified." Whether declaring an integer, defining a function, or creating a type, they all use the same syntax:

```
name: type = value
```

This is YaoXiang's most fundamental design concept. A few examples will show you this consistency:

```yaoxiang
// Variable declaration
x: Int = 42
name: String = "YaoXiang"

// Function definition
add: (a: Int, b: Int) -> Int = a + b

// Type definition
Point: Type = { x: Float, y: Float }
```

In formal syntax specification, variable declaration is defined as:

```
('mut')? Identifier (':' TypeExpr)? '=' Expr
```

Translated into plain language: an optional `mut` keyword, then the variable name, followed by an optional `: Type`, and finally `= value`. This structure runs throughout the entire language—learn it once and you're done.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This is a deliberate safety feature of the language.

```yaoxiang
x = 10
// x = 20   // Compile error! x is immutable
```

Variables declared with `=` search outward through the scope chain for a variable with the same name. If found, it assigns to that variable; if not found, it creates a new immutable variable in the current scope.

```yaoxiang
x = 1       // No x in outer scope, so declares a new variable
x = 2       // Found x in outer scope, attempts assignment → Compile error! x is immutable
```

This might seem counter-intuitive—if you can find it, why can't you assign to it? This is because YaoXiang prioritizes safety: immutable by default means you don't need to worry about a variable being accidentally modified somewhere in your code.

## mut Mutable Variables

When you genuinely need to modify a variable, use the `mut` keyword to explicitly declare it:

```yaoxiang
mut counter = 0
counter = counter + 1   // Can be modified
counter = 100           // Also works
```

There are several important rules for `mut`:

**Rule one**: `mut` is an explicit new declaration—the compiler will not search outer scopes for variables with the same name.

```yaoxiang
mut x = 10      // Creates a new mutable variable x in current scope
mut x = 20      // Compile error! x has already been declared in this scope
```

**Rule two**: Variables declared with `mut` cannot have the same name as variables in outer scopes (shadowing is prohibited).

```yaoxiang
x = 10
{
    mut x = 20   // Compile error! x is already declared in outer scope, shadowing is not allowed
}
```

**Rule three**: Within the same scope, each name can only be declared once—whether using `=` or `mut`.

```yaoxiang
x = 10
mut x = 20   // Compile error! x has already been declared
```

These rules ensure that each variable name is unique within the current scope, so you'll never encounter confusion about which variable a name refers to.

## Type Inference vs. Explicit Type Annotations

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can automatically infer types from the values you write, so in most cases you don't need to write types manually.

```yaoxiang
x = 42              // Compiler infers Int
name = "YaoXiang"   // Inferred as String
pi = 3.14159        // Inferred as Float
is_valid = true     // Inferred as Bool
```

When you want to explicitly annotate a type (for readability, or when the compiler can't infer it), use the `: Type` syntax:

```yaoxiang
count: Int = 100
greeting: String = "Hello"
ratio: Float = 0.618
```

Both styles are completely equivalent. You can start writing code without types and add type annotations when needed. This makes prototyping very fast while still maintaining type safety in the final code.

## Overview of Basic Types

YaoXiang has several built-in primitive types that cover the vast majority of everyday programming scenarios.

### Int (Integer)

```yaoxiang
a = 42              // Decimal
b = 0o52            // Octal (0o prefix)
c = 0x2A            // Hexadecimal (0x prefix)
d = 0b101010        // Binary (0b prefix)
e = 1_000_000       // Underscores can separate digits for readability
```

### Float (Floating-point)

```yaoxiang
pi = 3.14159
speed = 2.998e8         // Scientific notation: 2.998 × 10^8
tiny = 1.6e-19
```

### String

```yaoxiang
name = "YaoXiang"
empty = ""              // Empty string
escape = "Hello\nWorld" // Escape sequences: \n newline, \t tab, \\ backslash, \" double quote
unicode = "\u{4F60}\u{597D}"  // Unicode escape
```

### Bool (Boolean)

```yaoxiang
is_ready = true
is_done = false
```

Boolean values are typically used in conditional statements:

```yaoxiang
if is_ready {
    print("Starting processing")
}
```

## Variable Scope

Scope determines the visibility range of variables. YaoXiang's scope rules are very simple: **every `{}` block creates a new scope**.

### Basic Rules

```yaoxiang
{
    x = 10
    print(x)   // Can access: x is within current scope
}
// print(x)    // Error: x is not visible outside scope
```

Inner scopes can access variables from outer scopes:

```yaoxiang
outer = "I'm outside"
{
    print(outer)   // Can access outer's outer variable
    inner = "I'm inside"
}
// print(inner)    // Error: inner is not visible outside scope
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

In YaoXiang, `{}` blocks are also expressions and can return values:

```yaoxiang
result = {
    x = 10
    y = 20
    return x + y   // Returns 30 to the outer scope
}
// The value of 'result' is 30
// 'x' and 'y' are not visible outside the block
```

For detailed explanation of block return values, please refer to the functions chapter later. For now, just remember: **curly braces create scope, inner can see outer, outer cannot see inner**.

## Summary

You've now mastered the core concepts of YaoXiang's variable system:

| Concept          | Key Points                                          |
| ---------------- | --------------------------------------------------- |
| Unified syntax   | `name: type = value`, used for variables, functions, and types |
| Immutable by default | `x = 10` means `x` cannot be changed afterward  |
| Mutable variables | Use `mut` to explicitly declare `mut x = 10`       |
| No shadowing     | Each name can only be declared once per scope       |
| Type inference   | HM algorithm infers automatically, or write `: Type` for explicit annotation |
| Scope            | Every `{}` creates a scope; inner sees outer, outer doesn't see inner |

Next, you can continue learning more details about [Primitive Types](../../../design/formatter/formatting-rules/types.md), or proceed directly to the [Control Flow](../../../design/formatter/formatting-rules/control-flow.md) chapter.