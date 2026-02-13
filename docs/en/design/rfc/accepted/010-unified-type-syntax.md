---
title: 'RFC-010: Unified Type Syntax'
---

# RFC-010: Unified Type Syntax - name: type = value Model

> **Status**: Accepted
>
> **Author**: ChenXu
>
> **Created Date**: 2025-01-20
>
> **Last Updated**: 2026-02-12 (Unified syntax rule: identifier : type = expression, no `type`/`fn`/`struct`/`trait`/`impl` keywords)

## Summary

This RFC proposes an extremely minimalist unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression.
**No `fn`, no `struct`, no `trait`, no `impl`, no lowercase `type` keyword (but `Type` as meta-type keyword)**.

> **Key Design**: `Type` itself is a generic type. `Type[T]` means "a type that accepts type parameter T".

| Concept | Code |
|---------|------|
| Variable | `x: Int = 42` |
| Function | `add: (a: Int, b: Int) -> Int = a + b` |
| Record Type | `Point: Type = { x: Float, y: Float }` |
| Interface | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic Type | `List: Type[T] = { data: Array[T], length: Int }` |
| Generic Type (multiple params) | `Map: Type[K, V] = { keys: Array[K], values: Array[V] }` |
| Method | `Point.draw: (self: Point, s: Surface) -> Void = ...` |
| Generic Function | `map: [A,B](list: List[A], f: (A)->B) -> List[B] = ...` |

**`Type` is the only meta-type keyword in the language**.
It is used to annotate type levels, with the compiler automatically handling Type0, Type1, Type2... distinctions, transparent to users.

```yaoxiang
# Core Syntax: Unified + Distinguished

# Variable
x: Int = 42

# Function (parameter names in signature)
add: (a: Int, b: Int) -> Int = a + b

# Record Type
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

# Interface (record type with all function fields)
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

# Method definition (Type.method syntax)
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

# Generic Type (Type[T] = generic type)
List: Type[T] = {
    data: Array[T],
    length: Int
}

Map: Type[K, V] = {
    keys: Array[K],
    values: Array[V]
}

# Usage
p: Point = Point(1.0, 2.0)
p.draw(screen)           # Sugar → Point.draw(p, screen)
s: Drawable = p           # Structural subtyping: Point implements Drawable
```

## Motivation

### Why is this feature needed?

Current type system has multiple separated concepts:
- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack unity, leading to syntax fragmentation and high learning cost.

### Design Goals

1. **Extreme Unification**: One syntax rule covers all cases
2. **Clear Distinction**: Different contexts use same syntax but with clear markers
3. **Intuitive Learning**: One principle to learn, apply everywhere
4. **Backward Compatible**: Existing code can be migrated gradually

## Core Syntax

### Unified Model

```
┌─────────────────────────────────────────────────────────────────┐
│              identifier : type = expression                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Variable          │ name: Type = value                         │
│  Function          │ name: (params) -> Return = body            │
│  Record Type       │ Name: Type = { fields }                    │
│  Interface         │ Name: Type = { methods }                   │
│  Generic Type      │ Name: Type[T] = { ... }                    │
│  Generic Type      │ Name: Type[K, V] = { ... }                 │
│  Method            │ Type.method: (self: Type) -> Ret = body    │
│  Generic Function  │ name: [A,B](params) -> Return = body       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Meta-type Hierarchy (Compiler Internal)

**Internally**, the compiler maintains a universe level `level: selfpointnum` (stored as string, theoretically infinite).

| Level | Description |
|-------|-------------|
| `Type0` | Regular types (`Int`, `Float`, `Point`) |
| `Type1` | Type constructors (`List`, `Maybe`) |
| `Type2+` | Higher-order constructors |

**Users never see these numbers**, only `: Type`.

### Type Definition Syntax

```yaoxiang
# Record Type
Point: Type = {
    x: Float,
    y: Float
}

# Type with interface constraints
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}

# Generic Type (Type[T] = generic type)
List: Type[T] = {
    data: Array[T],
    length: Int
}

# Generic Type with multiple parameters
Map: Type[K, V] = {
    keys: Array[K],
    values: Array[V]
}

# Interface (record with all function fields)
Serializable: Type = {
    serialize: () -> String
}
```

### Method Syntax

```yaoxiang
# Type method
Point.distance: (self: Point, other: Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}

# Generic Method
List.map: [A,B](self: List[A], f: (A) -> B) -> List[B] = {
    # ...
}
```

### Interface Implementation

```yaoxiang
# Type with interface constraints
Point: Type = {
    x: Float,
    y: Float,
    Drawable,           # Implement Drawable
    Serializable        # Implement Serializable
}

# Usage as interface
p: Point = Point(1.0, 2.0)
drawable: Drawable = p  # ✓ Structural subtyping
```

## Grammar

```
program         ::= statement*

statement       ::= declaration | expression

# Unified declaration: name: type = expression
declaration     ::= Identifier ':' type '=' expression

# Type expression
type            ::= Identifier
                  | Identifier '[' type (',' type)* ']'  # Type constructor application
                  | '(' type (',' type)* ')' '->' type   # Function type
                  | '{' type_field* '}'                  # Record/Interface type
                  | 'Type'                               # Meta-type

type_field      ::= Identifier ':' type
                  | Identifier                            # Interface constraint

# Generic parameters
generic_params  ::= '[' Identifier (':' type)? (',' Identifier (':' type)?)* ']'

# Function signature
function_signature ::= Identifier generic_params? '(' parameter_list? ')' '->' type

parameter_list  ::= parameter (',' parameter)*

parameter       ::= Identifier ':' type

# Expression
expression      ::= literal
                  | Identifier
                  | Identifier '[' expression (',' expression)* ']'  # Constructor call
                  | '(' expression (',' expression)* ')'              # Tuple
                  | expression '.' Identifier '(' arguments? ')'     # Method call
                  | lambda
                  | '{' field ':' expression (',' field ':' expression)* '}'

arguments       ::= expression (',' expression)*

lambda          ::= '(' parameter_list? ')' '=>' block

block           ::= expression | '{' expression* '}'
```

## Implementation

### Parser Changes

| Feature | Description |
|---------|-------------|
| Unified declaration | Parse `name: type = value` |
| Type definition | Parse `Name: Type = {...}` |
| Type method | Parse `Type.method: ...` |

### Type System Changes

| Feature | Description |
|---------|-------------|
| Type as value | Types can be assigned to variables |
| Interface implementation | Structural subtyping |
| Method binding | Auto-binding with `pub` |

## Migration Strategy

### Automated Migration

| Old Syntax | New Syntax |
|-----------|------------|
| `var x: Int = 42` | `x: Int = 42` |
| `fn add(a: Int, b: Int) -> Int { a + b }` | `add: (a: Int, b: Int) -> Int = a + b` |
| `struct Point { x: Float, y: Float }` | `Point: Type = { x: Float, y: Float }` |

---

## 🎮 Easter Egg: The Origin of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
# Attempting to define the type of types...
Type: Type = Type
```

**Warning**: This is the **Ineffable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One generates Two, Two generates Three,                   ║
║   Three generates all things.                                 ║
║   In the Beginning, there was Taiji, which begat Liangyi.    ║
║                                                              ║
║   Type: Type = Type                                         ║
║   This is the source of YaoXiang, the edge of language.     ║
║   The compiler falls silent here; philosophy stands still.   ║
║                                                              ║
║   Thank you for reaching the philosophical boundary.         ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot properly handle `Type: Type = Type` (it leads to a Type0/Type1 universe paradox), but we've deliberately kept this "easter egg" — when you try to compile it, you'll receive a Zen message from the language creator. This is not just a technical boundary, but YaoXiang's tribute to the philosophy of types.

---

## Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Unified syntax | `name: type = value` | 2025-01-20 | ChenXu |
| Type keyword | `type Name = {...}` | 2025-01-20 | ChenXu |
| Interface syntax | `{ method: (params) -> Ret }` | 2025-01-20 | ChenXu |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| Declaration | Statement in form `name: type = value` |
| Record Type | Type with named fields `{ ... }` |
| Interface | Record type with all function fields |
| Generic Type | Type defined as `Name: Type[T] = { ... }`, accepts type parameters |
| Method | `Type.method` form, associated with specific type |
| Generic Function | Function with `[...]` type parameters |
| Meta-type | `Type`, the only type-level marker in the language |
