---
title: Variable Declarations
---

# Variable Declarations

This chapter introduces the core syntax for variable declarations in YaoXiang. If you have experience with other programming languages, you will find YaoXiang's variable system refreshingly concise—all declarations share a single syntax model.

## The Unified Syntax Model

YaoXiang's design philosophy is "everything is unified." Whether you declare an integer, define a function, or create a type, they all use the same syntax:

```
name: type = value
```

This is YaoXiang's most fundamental design principle. A few examples show this consistency:

```yaoxiang
# Variable declaration
x: Int = 42
name: String = "YaoXiang"

# Function definition
add: (a: Int, b: Int) -> Int = a + b

# Type definition
type Point = { x: Float, y: Float }
```

The formal definition of variable declaration in the language specification is:

```
('mut')? Identifier (':' TypeExpr)? '=' Expr
```

In plain English: an optional `mut` keyword, followed by a name, then an optional `: type`, and finally `= value`. This structure is used throughout the language—learn it once and you are set.

## Immutable Variables (Default Behavior)

In YaoXiang, all variables are **immutable by default**. Once assigned, they cannot be changed. This is a deliberate safety design.

```yaoxiang
x = 10
# x = 20   // Compile error! x is immutable
```

When you use `=` to declare a variable, the compiler looks outward through the scope chain for a variable with the same name. If it finds one, it attempts to assign to it; if not, it creates a new immutable variable in the current scope.

```yaoxiang
x = 1       // No x in outer scope, so x is declared as new
x = 2       // Found outer x, attempt assignment → compile error! x is immutable
```

This might seem counterintuitive—"if the compiler can find it, why can't it assign?" The answer is that YaoXiang prioritizes safety: immutability by default means you never have to worry about a variable being accidentally modified somewhere deep in your code.

## Mut Variables

When you genuinely need to mutate a variable, use the `mut` keyword to declare it explicitly:

```yaoxiang
mut counter = 0
counter = counter + 1   // Allowed
counter = 100           // Also allowed
```

`mut` has several important rules:

**Rule 1**: `mut` is always an explicit new declaration. The compiler will NOT look for existing variables in outer scopes.

```yaoxiang
mut x = 10      // Creates a new mutable variable x in the current scope
mut x = 20      // Compile error! x is already declared in this scope
```

**Rule 2**: A `mut` declaration cannot shadow a variable from an outer scope.

```yaoxiang
x = 10
{
    mut x = 20   // Compile error! x already declared in outer scope; shadowing is forbidden
}
```

**Rule 3**: Within the same scope, each name can only be declared once—whether with `=` or `mut`.

```yaoxiang
x = 10
mut x = 20   // Compile error! x is already declared
```

These rules ensure that every variable name is unique within its scope. You will never face the confusion of "which x am I actually referring to?"

## Type Inference vs. Explicit Type Annotation

YaoXiang uses the Hindley-Milner (HM) type inference algorithm. The compiler can infer types from the values you write, so you rarely need to annotate types by hand.

```yaoxiang
x = 42              // Compiler infers Int
name = "YaoXiang"   // Infers String
pi = 3.14159        // Infers Float
is_valid = true     // Infers Bool
```

When you want to be explicit about a type (for documentation purposes, or when the compiler needs a hint), use the `: Type` syntax:

```yaoxiang
count: Int = 100
greeting: String = "Hello"
ratio: Float = 0.618
```

Both styles are equivalent. You can start coding without types and add annotations later when needed. This makes prototyping fast while keeping your final code type-safe.

## Basic Types at a Glance

YaoXiang provides a small set of built-in types that cover the vast majority of day-to-day programming.

### Int (Integer)

```yaoxiang
a = 42              // Decimal
b = 0o52            // Octal (0o prefix)
c = 0x2A            // Hexadecimal (0x prefix)
d = 0b101010        // Binary (0b prefix)
e = 1_000_000       // Underscores for readability
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
escape = "Hello\nWorld" // Escape sequences: \n newline, \t tab, \\ backslash, \" quote
unicode = "\u{4F60}\u{597D}"  // Unicode escapes
```

### Bool (Boolean)

```yaoxiang
is_ready = true
is_done = false
```

Booleans are typically used in conditionals:

```yaoxiang
if is_ready {
    println("Starting processing")
}
```

## Variable Scope

Scope determines where a variable is visible. YaoXiang's scope rules are simple: **each `{}` block creates a new scope**.

### Basic Rules

```yaoxiang
{
    x = 10
    println(x)   // Works: x is in the current scope
}
// println(x)    // Error: x is not visible outside the scope
```

Inner scopes can access variables from outer scopes:

```yaoxiang
outer = "I am outside"
{
    println(outer)   // Can access outer
    inner = "I am inside"
}
// println(inner)    // Error: inner is not visible outside
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

In YaoXiang, `{}` blocks are also expressions that can return values:

```yaoxiang
result = {
    x = 10
    y = 20
    return x + y   // Returns 30 to the outer scope
}
// result has the value 30
// x and y are not visible outside the block
```

For more details on block return values, see the later chapter on functions. For now, just remember: **curly braces create scopes; inner sees outer, outer does not see inner**.

## Summary

You have now learned the core concepts of YaoXiang's variable system:

| Concept | Key Point |
|---------|-----------|
| Unified syntax model | `name: type = value` — used for variables, functions, and types |
| Immutable by default | `x = 10` means `x` cannot be reassigned |
| Mutable variables | Use `mut` explicitly: `mut x = 10` |
| No shadowing | Each name can only be declared once per scope |
| Type inference | HM algorithm infers types; optionally write `: Type` |
| Scope | Each `{}` creates a scope; inner sees outer, outer cannot see inner |

Next, you can continue to [Basic Types](./types.md) for more details, or jump straight into [Control Flow](./control-flow.md).
