---
title: 'Variable Declaration'
---

# Variable Declaration

This chapter introduces the core syntax of variable declaration in YaoXiang. If you have experience
with other programming languages, you will find YaoXiang's variable system very concise—all
declarations share the same syntax model.

## Unified Syntax Model

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

The formal definition of variable declaration in the language specification is:

```
('mut')? Identifier (':' TypeExpr)? '=' Expr
```

To put it in plain language: there can be an optional `mut` keyword, followed by the variable name,
then an optional `: type`, and finally `= value`. This structure runs through the entire
language—learn it once and you're set.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This
is a safety design choice in the language.

```yaoxiang
x = 10
// x = 20   // Compilation error! x is immutable
```

For variables declared with `=`, the compiler will look up the variable with the same name along the
scope chain. If found, it tries to assign to it; if not found, it creates a new immutable variable
in the current scope.

```yaoxiang
x = 1       // No x in the outer scope, so a new variable is declared
x = 2       // Found the outer x, attempts to assign → Compilation error! x is immutable
```

This may seem counter-intuitive—if you've learned other languages, you might wonder "if it can find
it, why can't it assign?". This is because YaoXiang puts safety first: being immutable by default
means you don't have to worry about a variable being accidentally modified in some corner of your
code.

## mut Mutable Variables

When you really need to modify a variable, use the `mut` keyword to explicitly declare it:

```yaoxiang
mut counter = 0
counter = counter + 1   // Can be modified
counter = 100           // Also allowed
```

`mut` has several important rules:

**Rule One**: `mut` is an explicit new declaration, and the compiler will not look up variables with
the same name in outer scopes.

```yaoxiang
mut x = 10      // Creates a new mutable variable x in the current scope
mut x = 20      // Compilation error! x has already been declared in the same scope
```

**Rule Two**: Variables declared with `mut` cannot have the same name as variables in an outer scope
(shadowing is prohibited).

```yaoxiang
x = 10
{
    mut x = 20   // Compilation error! x has already been declared in the outer scope; shadowing is not allowed
}
```

**Rule Three**: Within the same scope, each name can only be declared once—whether you use `=` or
`mut`.

```yaoxiang
x = 10
mut x = 20   // Compilation error! x has already been declared
```

These rules ensure that each variable name is unique within the current scope, so you will never be
confused about which variable a name refers to.

## Type Inference vs Explicit Type Annotations

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can automatically infer
the type from the value you write, and in most cases you don't need to write the type manually.

```yaoxiang
x = 42              // Compiler infers Int
name = "YaoXiang"   // Infers String
pi = 3.14159        // Infers Float
is_valid = true     // Infers Bool
```

When you want to explicitly annotate the type (for example, to improve code readability, or when the
compiler cannot infer), use the `: Type` syntax:

```yaoxiang
count: Int = 100
greeting: String = "Hello"
ratio: Float = 0.618
```

The two ways of writing are completely equivalent. You can start writing code by omitting types, and
add type annotations when needed. This makes prototyping very fast without sacrificing type safety
in the final code.

## Basic Types Overview

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

### Float (Floating Point)

```yaoxiang
pi = 3.14159
speed = 2.998e8         // Scientific notation: 2.998 × 10^8
tiny = 1.6e-19
```

### String (String)

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

An inner scope can access variables from the outer scope:

```yaoxiang
outer = "I am outside"
{
    print(outer)   // Can access the outer variable outer
    inner = "I am inside"
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

YaoXiang's `{}` blocks are also expressions and can return a value:

```yaoxiang
result = {
    x = 10
    y = 20
    return x + y   // Returns 30 to the parent scope
}
// result's value is 30
// x and y are not visible outside the block
```

For a detailed explanation of block return values, please refer to the subsequent function chapter.
All you need to remember here is: **curly braces create scope, inner can see outer, outer cannot see
inner**.

## Summary

You have now mastered the core concepts of YaoXiang's variable system:

| Concept              | Key Points                                                                         |
| -------------------- | ---------------------------------------------------------------------------------- |
| Unified syntax model | `name: type = value`, used for variables, functions, and types                     |
| Immutable by default | After `x = 10`, `x` cannot be changed                                              |
| Mutable variables    | Use `mut` to explicitly declare `mut x = 10`                                       |
| No shadowing         | The same name can only be declared once in the same scope                          |
| Type inference       | HM algorithm automatically infers, can also write `: Type` for explicit annotation |
| Scope                | Each `{}` creates a scope, inner can see outer, outer cannot see inner             |

Next, you can continue learning the [Basic Types](./types.md) in more detail, or jump directly to
the [Control Flow](./control-flow.md) chapter.
