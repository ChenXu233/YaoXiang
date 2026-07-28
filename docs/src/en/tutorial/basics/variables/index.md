---
title: Variable Declarations
---

# Variable Declarations

This chapter introduces the core syntax for variable declarations in YaoXiang. If you have experience with other programming languages, you'll find YaoXiang's variable system quite elegant—all declarations share the same syntax model.

## Unified Syntax Model

YaoXiang's design philosophy is "everything is unified." Whether you're declaring an integer, defining a function, or creating a type, they all use the same syntax:

```
name: type = value
```

This is YaoXiang's most fundamental design principle. A few examples will show you this consistency:

```yaoxiang
// Variable declaration
x: Int = 42
name: String = "YaoXiang"

// Function definition
add: (a: Int, b: Int) -> Int = a + b

// Type definition
Point: Type = { x: Float, y: Float }
```

The formal grammar for variable declarations is:

```
('mut')? Identifier (':' TypeExpr)? '=' Expr
```

Translated into plain language: an optional `mut` keyword, followed by the variable name, then an optional `: type`, and finally `= value`. This structure runs throughout the entire language—learn it once and you're done.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This is a deliberate safety feature of the language.

```yaoxiang
x = 10
// x = 20   // Compile error! x is immutable
```

When you declare a variable using `=`, the compiler searches outward along the scope chain for a variable with the same name. If found, it attempts to assign to it; if not found, it creates a new immutable variable in the current scope.

```yaoxiang
x = 1       // No x in outer scope, so declares a new variable
x = 2       // Found the outer x, tries to assign → Compile error! x is immutable
```

This might seem counterintuitive—if it can find the variable, why can't you assign to it? This is because YaoXiang prioritizes safety: immutability by default means you never have to worry about a variable being accidentally modified somewhere in your code.

## mut Mutable Variables

When you genuinely need to modify a variable, use the `mut` keyword to explicitly declare it:

```yaoxiang
mut counter = 0
counter = counter + 1   // Can modify
counter = 100           // Also works
```

There are several important rules for `mut`:

**Rule One**: `mut` is an explicit new declaration—the compiler won't search outer scopes for a variable with the same name.

```yaoxiang
mut x = 10      // Creates a new mutable variable x in current scope
mut x = 20      // Compile error! x already declared in this scope
```

**Rule Two**: Variables declared with `mut` cannot have the same name as variables in outer scopes (shadowing is forbidden).

```yaoxiang
x = 10
{
    mut x = 20   // Compile error! x already declared in outer scope, shadowing not allowed
}
```

**Rule Three**: Within the same scope, each name can only be declared once—whether using `=` or `mut`.

```yaoxiang
x = 10
mut x = 20   // Compile error! x already declared
```

These rules ensure that each variable name is unique within the current scope—you'll never be confused about which variable a name refers to.

## Type Inference vs Explicit Type Annotations

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can automatically infer types from the values you write, so most of the time you don't need to write types manually.

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

Both styles are completely equivalent. You can start writing code without types and add type annotations when needed. This makes prototyping very fast while still ensuring type safety in the final code.

## Basic Types Overview

YaoXiang has several built-in basic types that cover the vast majority of everyday programming scenarios.

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

Boolean values are typically used in conditional checks:

```yaoxiang
if is_ready {
    print("Start processing")
}
```

## Variable Scope

Scope determines the visibility of variables. YaoXiang's scoping rules are very simple: **every `{}` block creates a new scope**.

### Basic Rules

```yaoxiang
{
    x = 10
    print(x)   // Accessible: x is within current scope
}
// print(x)    // Error: x is not visible outside the scope
```

Inner scopes can access variables from outer scopes:

```yaoxiang
outer = "I'm outside"
{
    print(outer)   // Can access outer from the outer scope
    inner = "I'm inside"
}
// print(inner)    // Error: inner is not visible outside the scope
```

### Function Parameter Scope

```yaoxiang
greet: (name: String) -> Void = {
    print("Hello, " + name)
    // name is visible within this function body
}
// name is not visible outside the function
```

### Block Expressions

In YaoXiang, `{}` blocks are also expressions and can return values:

```yaoxiang
result = {
    x = 10
    y = 20
    return x + y   // Returns 30 to the outer scope
}
// result's value is 30
// x and y are not visible outside the block
```

For detailed explanations of block return values, see the functions chapter later. For now, just remember: **braces create scopes, inner can see outer, outer cannot see inner**.

## Summary

You've now mastered the core concepts of YaoXiang's variable system:

| Concept         | Key Points                                          |
| --------------- | --------------------------------------------------- |
| Unified syntax  | `name: type = value`—variables, functions, types all use it |
| Immutable by default | `x = 10` means `x` cannot be changed         |
| Mutable variables | Use `mut` to explicitly declare `mut x = 10`    |
| No shadowing   | Each name can only be declared once per scope        |
| Type inference | HM algorithm infers automatically, or write `: Type` for clarity |
| Scoping        | Every `{}` creates a scope, inner sees outer, outer doesn't see inner |

Next, you can continue learning more details about [Basic Types](./types.md), or jump directly to the [Control Flow](./control-flow.md) chapter.
