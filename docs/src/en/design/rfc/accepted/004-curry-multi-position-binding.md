---
title: 'RFC-004: Multi-Position Joint Binding Design for Curried Methods'
---

# RFC-004: Multi-Position Joint Binding Design for Curried Methods

> **Status**: Accepted
> **Author**: ChenXu
> **Created Date**: 2025-01-05
> **Last Updated**: 2026-02-18 (Added builtin binding and anonymous function binding syntax)

## Summary

This RFC proposes a brand-new **multi-position joint binding** syntax, allowing functions to be precisely bound to arbitrary parameter positions of types, supporting single-position binding and multi-position joint binding, fundamentally solving the "who is the caller" problem in curried binding, without introducing the `self` keyword.

## Motivation

### Why is this feature needed?

When binding standalone functions as type methods in current language design, the following problems exist:

1. **Caller position inflexible**: Traditional binding can only fix `obj` in `obj.method(args)` as the first parameter
2. **Multi-parameter binding difficult**: When methods need to receive multiple same-type parameters, cannot express elegantly
3. **Currying semantic ambiguity**: When partially applying, difficult to distinguish "which position to bind"

### Design Goal: Unify Two Programming Perspectives

This design aims to **unify functional and OOP programming perspectives**:

```yaoxiang
# Functional perspective: explicitly pass all parameters
distance(p1, p2)

# OOP perspective: implicit this
p1.distance(p2)

# [positions] syntactic sugar makes the two equivalent, essentially both are function calls
Point.distance = distance[0]   # this bound to position 0
```

**Core Value**:
- Underlying is functions, upper is method syntax
- No `self` keyword introduced, maintaining language simplicity
- Fully functional: method call essence is parameter passing
- `[0]`, `[1]`, `[-1]` flexibly control this binding position
- **Syntax Unification**: Function definition uses `name: (params) -> Return = body` format

### Current Problems

```yaoxiang
# Problems with existing design:
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# Can only bind to first parameter
Point.distance = distance  # Equivalent to distance[0]
# p1.distance(p2) → distance(p1, p2) ✓

# But what if transform signature is transform(Vector, Point)?
# Cannot express p1.transform(v1) → transform(v1, p1) semantics
```

## Proposal

### Core Design: Default Binding + Optional Position Specification

#### Default: Bind to First Type-Matching Position

**Default Behavior**: `Type.method = function` automatically finds the first position matching that type and binds

```yaoxiang
# Default: bind to first type-matching position
Point.distance = distance           # Compiler automatically finds first Point parameter position
p1.distance(p2)                     # → distance(p1, p2)

# If function has two Point parameters, bind to first matching position
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}
# Binding: Point.distance = distance
# Call: p1.distance(p2) → distance(p1, p2) ✓

# Only explicitly specify when need special position (not first match)
Point.compare = distance[1]        # Bind to second Point parameter
p1.compare(p2)                    # → distance(p2, p1)
```

**Binding Failure Handling**:
- **Cannot find matching type**: Error or warning if no such type in function parameters
- **Factory function pattern**: If no parameters match, may be used as factory function

```yaoxiang
# Case 1: Cannot find matching type
create_point: () -> Point = { ... }
Point.create = create_point        # Error: no Point type parameter
```

#### Multi-Position Binding Syntax

**Full Syntax**: `Type.method = function[positions]`

```yaoxiang
# Single position binding
Point.distance = distance[0]        # Bind to position 0

# Multi-position binding
Point.transform = transform[0, 1]   # Bind to positions 0 and 1

# Negative index (last parameter)
Point.method = func[-1]             # Bind to last parameter

# Placeholder
Point.calc = func[0, _, 2]         # Skip position 1
```

#### Binding Semantics

| Binding | Call | Mapping |
|---------|------|---------|
| `Point.distance = distance[0]` | `p1.distance(p2)` | `distance(p1, p2)` |
| `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| `Point.transform = transform[0, 1]` | `p1.transform(v)` | `transform(p1, v)` |

#### Currying Behavior

Binding naturally supports currying:

```yaoxiang
# Original function: 5 parameters
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

# Binding: Point.calc = calculate[1, 2]
# After binding, remaining parameters: scale, x, y

# Call scenarios
p1.calc(2.0, 10.0, 20.0)              # Provide 3 arguments → Direct call
p1.calc(2.0)                            # Provide 1 argument → Returns (Float, Float) -> Float
p1.calc()                                # Provide 0 arguments → Returns (Float, Float, Float) -> Float
```

### pub Automatic Binding

Functions declared with `pub` automatically bind:

```yaoxiang
# In Point.yx
pub distance: (a: Point, b: Point) -> Float = { ... }
pub transform: (p: Point, v: Vector) -> Point = { ... }

# Compiler automatically:
# 1. Point is defined in current file
# 2. Functions have Point as first parameter
# 3. Execute Point.distance = distance[0]
# 4. Execute Point.transform = transform[0]
```

### Builtin Binding

Bindings can be directly written inside type definition body, without separate binding statements:

```yaoxiang
# Way 1: Direct binding of external function inside type definition
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # Bind to position 0
}

# Way 2: Anonymous function + position binding
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
# Syntax: ((params) => body)[position]
# Call: p1.distance(p2) → distance(p1, p2)
```

**Currying semantics**:
- When binding `distance = distance[0]`, original function signature `(a: Point, b: Point) -> Float`
- Generated method signature: `b: Point -> Float` (position 0 filled by caller)

## Detailed Design

### Grammar

```
BindingDeclaration ::= Type '.' Identifier '=' FunctionName '[' PositionList ']'
PositionList       ::= Position (',' Position)* ','?
Position           ::= Integer | '-' Integer | '_'
```

### Type Checking Rules

| Rule | Description |
|------|-------------|
| Position starts from 0 | `func[0]` binds 1st parameter |
| Maximum position | Must be < number of function parameters |
| Negative index | `[-1]` means last parameter |
| Placeholder | `_` skips that position |

```yaoxiang
# ✅ Valid binding
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(Float, Point, Point, ...)

# ❌ Invalid binding (compile error)
Point.wrong = distance[5]             # 5 >= 2 (parameter count)
Point.wrong = distance[0, 0]        # Duplicate position (if not allowed)
Point.wrong = distance[-2]            # -2 out of range
```

### pub Binding Rules

| Rule | Description |
|------|-------------|
| Function defined in module file | YES |
| Function's 0th parameter type matches module name | YES |
| Function must be `pub` | YES for auto-binding outside module |

## Implementation Requirements

### Lexer Additions

| Token | Description |
|-------|-------------|
| `[` | Position list start |
| `]` | Position list end |
| `,` | Position separator |
| `_` | Placeholder |

### Parser Additions

| Grammar Rule | Description |
|--------------|-------------|
| `BindingDeclaration` | Parse binding syntax |
| `PositionList` | Parse position list |

### Type Checker Additions

| Check | Description |
|-------|-------------|
| `binding_position_check` | Verify position validity |
| `binding_type_check` | Verify type compatibility |

## Backward Compatibility

| Feature | Backward Compatible | Migration Path |
|---------|-------------------|---------------|
| Default binding | ✅ Yes | Auto-upgrade |
| Position specification | ✅ Yes | Optional |
| pub auto-binding | ✅ Yes | Optional |

## Trade-offs

### Advantages

- **Flexible binding**: Can bind to any position
- **No self keyword**: Keep language simple
- **Unified paradigm**: Functional + OOP perspective
- **Currying support**: Natural multi-argument handling

### Disadvantages

- **Learning curve**: Position index concept
- **Complex binding**: Multi-position may be confusing

## References

- [Rust method syntax](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell currying](https://www.haskell.org/tutorial/patterns.html)

---

## Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Position syntax | Use `[0]`, `[1]`, `[-1]` | 2025-01-05 | ChenXu |
| Default binding | First type-matching position | 2025-01-05 | ChenXu |
| Placeholder | Use `_` for skipping | 2025-01-05 | ChenXu |
| pub auto-binding | Auto-bind to first parameter | 2025-01-05 | ChenXu |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| Currying | Converting multi-argument function to single-argument chain |
| Binding | Associate function with type for method syntax |
| Position Index | 0-based index of function parameter |
| Placeholder | Skip specific position in binding |
