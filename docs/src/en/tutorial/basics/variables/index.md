---
title: Variable Declarations
---

# Variable Declarations

This chapter introduces the core syntax of variable declarations in YaoXiang. If you have experience with other programming languages, you will find that YaoXiang's variable system is very concise—all declarations share the same syntactic model.

## Unified Syntax Model

YaoXiang's design philosophy is "everything is unified." Whether declaring an integer, defining a function, or creating a type, they all use the same syntax:

```
name: type = value
```

This is YaoXiang's most core design concept. A few examples will give you a sense of this consistency:

```yaoxiang
# Variable declaration
x: Int = 42
name: String = "YaoXiang"

# Function definition
add: (a: Int, b: Int) -> Int = a + b

# Type definition
type Point = { x: Float, y: Float }
```

The formal definition of variable declarations in the syntax specification is:

```
('mut')? Identifier (':' TypeExpr)? '=' Expr
```

In plain language: there is an optional `mut` keyword, followed by a variable name, then an optional `: Type`, and finally `= value`. This structure runs throughout the entire language—learn it once, and that's enough.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This is a safety design of the language.

```yaoxiang
x = 10
# x = 20   // Compile error! x is immutable
```

For a variable declared with `=`, the compiler will search outward along the scope chain for a variable with the same name. If found, it tries to assign to it; if not found, it creates a new immutable variable in the current scope.

```yaoxiang
x = 1       // No x in the outer scope, so declared as a new variable
x = 2       // Found the outer x, attempt to assign → Compile error! x is immutable
```

This may seem a bit counterintuitive—if you've learned other languages, you might wonder "if it can find it, why can't it assign?" This is because YaoXiang puts safety first: immutability by default means you don't have to worry about a variable being accidentally modified in some corner of the code.

## mut Mutable Variables

When you do need to modify a variable, use the `mut` keyword to declare it explicitly:

```yaoxiang
mut counter = 0
counter = counter + 1   // OK to modify
counter = 100           // Also OK
```

`mut` has several important rules:

**Rule One**: `mut` is an explicit new declaration; the compiler does not look up variables with the same name in outer scopes.

```yaoxiang
mut x = 10      // Creates a new mutable variable x in the current scope
mut x = 20      // Compile error! x has already been declared in the same scope
```

**Rule Two**: A variable declared with `mut` cannot have the same name as a variable in an outer scope (shadowing is prohibited).

```yaoxiang
x = 10
{
    mut x = 20   // Compile error! x has already been declared in the outer scope; shadowing is not allowed
}
```

**Rule Three**: Within the same scope, each name can be declared only once—whether using `=` or `mut`.

```yaoxiang
x = 10
mut x = 20   // Compile error! x has already been declared
```

These rules ensure that each variable name is unique within the current scope, so you will never be confused about which variable a name refers to.

## Type Inference vs. Explicit Type Annotations

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can automatically infer the type from the value you write, so in most cases you don't need to specify the type manually.

```yaoxiang
x = 42              // Compiler infers Int
name = "YaoXiang"   // Infers String
pi = 3.14159        // Infers Float
is_valid = true     // Infers Bool
```

When you want to explicitly annotate the type (for example, to improve code readability, or when the compiler cannot infer it), use the `: Type` syntax:

```yaoxiang
count: Int = 100
greeting: String = "Hello"
ratio: Float = 0.618
```

The two ways of writing are completely equivalent. You can start by omitting types, and add type annotations later when needed. This makes prototyping very fast, without sacrificing type safety in the final code.

## Overview of Basic Types

YaoXiang has several built-in basic types that cover the vast majority of everyday programming scenarios.

### Int (Integer)

```yaoxiang
a = 42              // Decimal
b = 0o52            // Octal (0o prefix)
c = 0x2A            // Hexadecimal (0x prefix)
d = 0b101010        // Binary (0b prefix)
e = 1_000_000       // Underscores can be used as digit separators for readability
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

Boolean values are typically used in conditional judgments:

```yaoxiang
if is_ready {
    println("Start processing")
}
```

## Variable Scope

Scope determines the visibility range of a variable. YaoXiang's scoping rules are very simple: **each `{}` block creates a new scope**.

### Basic Rules

```yaoxiang
{
    x = 10
    println(x)   // Accessible: x is in the current scope
}
// println(x)    // Error: x is not visible outside the scope
```

Inner scopes can access variables from outer scopes:

```yaoxiang
outer = "I'm outside"
{
    println(outer)   // Can access outer
    inner = "I'm inside"
}
// println(inner)    // Error: inner is not visible outside the scope
```

### Function Parameter Scope

```yaoxiang
greet: (name: String) -> Void = {
    println("Hello, " + name)
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
// The value of result is 30
// x and y are not visible outside the block
```

For a detailed explanation of block return values, please refer to the function chapter that follows. For now, you just need to remember: **braces create a scope—the inner can see the outer, the outer cannot see the inner**.

## Summary

You have now mastered the core concepts of YaoXiang's variable system:

| Concept | Key Points |
|------|------|
| Unified Syntax Model | `name: type = value`, used for variables, functions, and types |
| Immutable by Default | After `x = 10`, `x` cannot be changed |
| Mutable Variables | Declare explicitly with `mut`, e.g., `mut x = 10` |
| No Shadowing | The same name can be declared only once in the same scope |
| Type Inference | The HM algorithm infers automatically; `: Type` can be used for explicit annotation |
| Scope | Each `{}` creates a scope—the inner can see the outer, the outer cannot see the inner |

You can continue to learn more details about [Basic Types](./types.md), or jump straight into the [Control Flow](./control-flow.md) chapter.