---
title: Variable Declaration
---

# Variable Declaration

This chapter introduces the core syntax of variable declaration in YaoXiang. If you have experience with other programming languages, you'll find YaoXiang's variable system very clean—**all declarations share a single syntax model**.

## Unified Syntax Model

YaoXiang's design philosophy is "everything is unified." Whether you're declaring an integer, defining a function, or creating a type, they all use the same syntax:

```
name: type = value
```

This is YaoXiang's most fundamental design concept. A few examples will give you a feel for this consistency:

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

In plain language: there's an optional `mut` keyword, followed by the variable name, then an optional `: Type`, and finally `= value`. This structure runs through the entire language—learn it once, and you're set.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This is a safety design of the language.

```yaoxiang
x = 10
// x = 20   // Compile error! x is immutable
```

For variables declared with `=`, the compiler looks outward along the scope chain for a variable with the same name. If it finds one, it tries to assign to it; if not, it creates a new immutable variable in the current scope.

```yaoxiang
x = 1       // No x in the outer scope, so it's declared as a new variable
x = 2       // Found the outer x, attempts to assign → Compile error! x is immutable
```

This may seem a bit counterintuitive—if you've learned other languages, you might think "if it can find it, why can't it assign?". The reason is that YaoXiang puts safety first: immutability by default means you never have to worry about a variable being accidentally modified somewhere in your code.

## mut Mutable Variables

When you really do need to modify a variable, use the `mut` keyword to declare it explicitly:

```yaoxiang
mut counter = 0
counter = counter + 1   // OK to modify
counter = 100           // Also OK
```

`mut` has several important rules:

**Rule One**: `mut` always creates a new declaration; the compiler will not look for a same-named variable in an outer scope.

```yaoxiang
mut x = 10      // Creates a new mutable variable x in the current scope
mut x = 20      // Compile error! x has already been declared in the same scope
```

**Rule Two**: A `mut` variable cannot share a name with a variable in an outer scope (shadowing is forbidden).

```yaoxiang
x = 10
{
    mut x = 20   // Compile error! x was already declared in the outer scope; shadowing is not allowed
}
```

**Rule Three**: Within the same scope, each name can only be declared once—whether you use `=` or `mut`.

```yaoxiang
x = 10
mut x = 20   // Compile error! x has already been declared
```

These rules ensure that every variable name is unique within the current scope, so you'll never be confused about which variable a same name actually refers to.

## Type Inference vs Explicit Type Annotations

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can automatically infer types from the values you write, so in most cases you don't need to write types manually.

```yaoxiang
x = 42              // Inferred as Int
name = "YaoXiang"   // Inferred as String
pi = 3.14159        // Inferred as Float
is_valid = true     // Inferred as Bool
```

When you want to explicitly annotate the type (for instance, to improve code readability, or when the compiler cannot infer the type), use the `: Type` syntax:

```yaoxiang
count: Int = 100
greeting: String = "Hello"
ratio: Float = 0.618
```

The two writing styles are completely equivalent. You can start writing code by omitting types, and add type annotations when needed. This makes prototyping extremely fast, without sacrificing type safety in the final code.

## Overview of Basic Types

YaoXiang comes with several built-in basic types, covering the vast majority of everyday programming scenarios.

### Int (Integer)

```yaoxiang
a = 42              // Decimal
b = 0o52            // Octal (0o prefix)
c = 0x2A            // Hexadecimal (0x prefix)
d = 0b101010        // Binary (0b prefix)
e = 1_000_000       // Underscores can be used to separate digits for readability
```

### Float (Floating-Point Number)

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

Boolean values are typically used in conditional checks:

```yaoxiang
if is_ready {
    print("Start processing")
}
```

## Variable Scope

Scope determines the visible range of a variable. YaoXiang's scope rules are very simple: **every `{}` block creates a new scope**.

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

In YaoXiang, `{}` blocks are also expressions and can return a value:

```yaoxiang
result = {
    x = 10
    y = 20
    return x + y   // Returns 30 to the enclosing scope
}
// result's value is 30
// x and y are not visible outside the block
```

For a detailed explanation of block return values, please refer to the function chapter that follows. For now, you only need to remember: **curly braces create a scope, inner scopes can see outer ones, outer scopes cannot see inner ones**.

## Summary

You've now mastered the core concepts of YaoXiang's variable system:

| Concept | Key Points |
|------|------|
| Unified syntax model | `name: type = value`, used for variables, functions, and types alike |
| Immutable by default | After `x = 10`, `x` cannot be changed |
| Mutable variables | Use `mut` to explicitly declare: `mut x = 10` |
| Shadowing forbidden | The same name can only be declared once in the same scope |
| Type inference | HM algorithm infers automatically; can also be annotated explicitly with `: Type` |
| Scope | Every `{}` creates a scope; inner can see outer, outer cannot see inner |

Next, you can continue learning more details about [Basic Types](./types.md), or jump directly to the [Control Flow](./control-flow.md) chapter.