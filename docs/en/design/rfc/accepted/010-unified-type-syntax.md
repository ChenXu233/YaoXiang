---
title: 'RFC-010: Unified Type Syntax'
---

# RFC-010: Unified Type Syntax - name: type = value Model

> **Status**: Accepted
> **Author**: ChenXu
> **Created Date**: 2025-01-20
> **Last Updated**: 2025-02-03 (Unified syntax style: parameter names declared in signature, function body with = { ... })

## Summary

This RFC proposes an extremely minimalist unified type syntax model: **everything is `name: type = value`**.

Core Ideas:
- Variables/Functions: `name: type = value`
- Type definition: `type Name = { ... }`
- Interface definition: `type InterfaceName = { method: (param: Type) -> Ret }`
- Type method: `Type.method: (self: Type, param: Type) -> Ret = { ... }`
- Regular method: `name: (param: Type) -> Ret = { ... }`
- Auto-binding: `pub name: (param: Type) -> Ret = ...` → auto-bind to type

```yaoxiang
# Core Syntax: Unified + Distinguished

# Variable
x: Int = 42

# Function (parameter names in signature)
add: (a: Int, b: Int) -> Int = a + b

# Type definition (type keyword first, more intuitive)
type Point = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}

# Interface definition
type Drawable = {
    draw: (self: Self, surface: Surface) -> Void
}

type Serializable = {
    serialize: (self: Self) -> String
}

# Method definition (using Type.method syntax sugar)
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

# Usage
p: Point = Point(1.0, 2.0)
p.draw(screen)           # Sugar → Point.draw(p, screen)
s: Drawable = p      # Interface assignment
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
│                    name: type = value                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Variable        │ name: Type = value                            │
│  Function        │ name: (params) -> Return = body               │
│  Type Definition │ type Name = { fields }                       │
│  Interface       │ type Interface = { methods }                 │
│  Type Method     │ Type.method: (self: Type) -> Ret = body      │
│  Regular Method  │ name: (params) -> Ret = body                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Type Definition Syntax

```yaoxiang
# Basic struct
type Point = {
    x: Float,
    y: Float
}

# Enum variant
type Color = {
    red,
    green,
    blue
}

# Generic type
type Result[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self
}

# Interface type
type Serializable = {
    serialize: () -> String
}
```

### Method Syntax

```yaoxiang
# Type method (sugar for regular function)
Point.distance: (self: Point, other: Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}

# Auto-binding with pub
pub distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}
# Automatically: Point.distance = distance[0]
```

### Interface Implementation

```yaoxiang
# Type implementing interfaces
type Point = {
    x: Float,
    y: Float,
    Drawable,           # Implement Drawable
    Serializable        # Implement Serializable
}

# Usage as interface
p: Point = Point(1.0, 2.0)
drawable: Drawable = p  # ✓ Point implements Drawable
```

## Grammar

```
Declaration     ::= VariableDecl | FunctionDef | TypeDef | MethodDef

VariableDecl    ::= Identifier ':' Type '=' Expression

FunctionDef     ::= Identifier ':' FunctionSignature '=' FunctionBody
FunctionSignature::= '(' ParameterList? ')' '->' Type
ParameterList   ::= Parameter (',' Parameter)*
Parameter       ::= Identifier ':' Type

TypeDef         ::= 'type' Identifier '=' TypeBody
TypeBody        ::= RecordType | EnumType | InterfaceType

MethodDef       ::= Type '.' Identifier ':' FunctionSignature '=' FunctionBody
```

## Implementation

### Parser Changes

| Feature | Description |
|---------|-------------|
| Unified declaration | Parse `name: type = value` |
| Type definition | Parse `type Name = ...` |
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
| `struct Point { x: Float, y: Float }` | `type Point = { x: Float, y: Float }` |

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
| Type Definition | Defining new types |
| Interface | Contract specifying required methods |
| Method Binding | Associating function with type |
| Auto-binding | Automatic method binding with `pub` |
