---
title: "RFC-004: Multi-Position Joint Binding Design for Curried Methods"
status: "Accepted"
author: "Chenxu"
created: "2025-01-05"
updated: "2026-02-18 (Added builtin binding and postfix binding syntax)"
issue: "#132"
---

# RFC-004: Multi-Position Joint Binding Design for Curried Methods

## Summary

This RFC proposes a brand-new **multi-position joint binding** syntax that allows functions to be precisely bound to any parameter position of a type, supporting both single-position binding and multi-position joint binding, fundamentally solving the problem of "who is the caller" in curried binding without introducing the `self` keyword.

## Motivation

### Why is this feature needed?

In the current language design, binding standalone functions as type methods faces the following problems:

1. **Inflexible caller position**: Traditional binding can only fix `obj` as the first parameter in `obj.method(args)`
2. **Difficulty with multi-parameter binding**: When a method needs to receive multiple parameters of the same type, it cannot be expressed elegantly
3. **Currying semantic ambiguity**: During partial application, it is hard to distinguish "which position to bind to"

### Design Goal: Unifying Two Programming Perspectives

This design aims to **unify the functional and OOP programming perspectives**:

```yaoxiang
# Functional perspective: explicitly pass all arguments
distance(p1, p2)

# OOP perspective: implicit this
p1.distance(p2)

# The [positions] syntax sugar makes the two notations equivalent;
# in essence both are function calls
Point.distance = distance[0]   # this binds to position 0
```

**Core values**:
- The bottom layer is a function; the top layer is method syntax
- No `self` keyword introduced, keeping the language concise
- Fully functional: method calls are essentially argument passing
- `[0]`, `[1]`, `[-1]` flexibly control where this binds
- **Unified syntax**: function definitions use the `name: (params) -> Return = body` format

### Current Problems

```yaoxiang
# Problems with the current design:
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# Can only bind to the first argument
Point.distance = distance  # equivalent to distance[0]
# p1.distance(p2) → distance(p1, p2) ✓

# But what if the signature of transform is transform(Vector, Point)?
# Cannot express the semantics p1.transform(v1) → transform(v1, p1)
```

## Proposal

### Core Design: Explicit Position Specification

**Core rule: omitting `[n]` means no binding.** `Point.name = func` is merely a namespace alias and does not trigger any implicit binding. To make the `.` call syntax `p.name(args)` work, you must explicitly specify: `Point.name = func[n]`.

#### Single-Position Binding

```yaoxiang
# Explicitly bind to the first Point parameter position (indices start at 0)
Point.distance = distance[0]
p1.distance(p2)                     # → distance(p1, p2)

# Bind to the second Point parameter position
Point.compare = distance[1]         # bind to the second Point argument
p1.compare(p2)                      # → distance(p2, p1)
```

**Omitting `[n]` means no binding**:

```yaoxiang
# No [n] → pure namespace alias, no . call syntax
Point.distance = distance            # only Point.distance(p1, p2)
# p1.distance(p2)  ❌  no binding

# Factory functions are naturally legal; no special handling required
create_point: () -> Point = { ... }
Point.create = create_point          # Point.create()   ✅
```
- Type safety: binding only happens when types match, avoiding mistakes
- Flexible control: `[n]` precisely controls the binding position

#### Curried Binding

When the number of function parameters exceeds the number of binding positions, a curried function is automatically generated. **Binding is always an explicit operation.**

```yaoxiang
Point: Type = { x: Float, y: Float }

# Base function
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# Explicitly bind to position 0 → currying: the remaining argument factor is provided by the caller
Point.scale = scale[0]

# Invocation
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0)

# Chained calls are more elegant
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### Position Index Binding Syntax

The `[position]` syntax is introduced to precisely control the binding relationship between function parameters and types:

```yaoxiang
# Syntax format: Type.method = function[positions]

# === Basic binding ===

# Single-position binding
Point.distance = distance[1]           # bind to the 1st argument (indices start at 0)
# Usage: p1.distance(p2) → distance(p2, p1)

# Multi-position joint binding (tuple destructuring)
Point.transform = transform[1, 2]      # bind to the 1st and 2nd arguments
# Usage: p1.transform(v1) → transform(v1, p1)
# Original function signature: transform(Point, Vector) → Point
# After binding: Point.transform(Vector) → Point
```

### Detailed Syntax Definition

```
BindingDecl   ::= Type '.' Identifier '=' FuncName '[' PositionList ']'
PositionList  ::= Position (',' Position)*
Position      ::= Integer                # placeholder
               | '_'                    # skip this position (placeholder)
               | Integer '..' Integer   # position range (future extension)

FuncName      ::= Identifier
Type          ::= Identifier (GenericParams)?
```

### Builtin Binding

Binding can be written directly inside the type definition body, without a separate binding statement:

```yaoxiang
# Method 1: bind directly inside the type definition body
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # bind to position 0
}

# Method 2: anonymous function + position binding
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
```

**Currying semantics**:
- When binding `distance = distance[0]`, the original function signature is `(a: Point, b: Point) -> Float`
- The generated method signature: `b: Point -> Float` (position 0 is filled by the caller)

### Usage Examples

```yaoxiang
# === Complete examples ===

Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

# 1. Basic distance calculation
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

# Binding: Point.distance = distance[1]
# Call: p1.distance(p2) → distance(p2, p1)
# But we want p1.distance(p2) → distance(p1, p2), so:
Point.distance = distance[0]

# 2. Transform operations (multi-position binding)
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# Binding Point.transform = transform[1]
# Call: p.transform(v) → transform(v, p) ❌
# Binding Point.transform = transform[0]
# Call: p.transform(v) → transform(p, v) ✓

# 3. Complex multi-parameter functions
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# Bind only the 1st argument (Point type), keep the 3rd argument
Point.scale = multiply[0, _]
# Call: p.scale(2.0) → multiply(p, 2.0)

# 4. Cross-type binding
Circle: Type = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# Bind the distance method to the Circle type
Circle.distance = distance[0, 1]
# Call: c1.distance(c2) → distance(c1, c2)
```

### Tuple Destructuring Support

```yaoxiang
# === Tuple destructuring binding ===

# Function that receives a tuple argument
process_coordinates: (coord: (Float, Float)) -> String = {
    return match coord {
        (0.0, 0.0) -> "origin"
        (x, 0.0) -> "on x-axis at ${x}"
        (0.0, y) -> "on y-axis at ${y}"
        (x, y) -> "point at (${x}, ${y})"
    }
}

Coord: Type = { x: Float, y: Float }

# Automatic destructuring binding: Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# Usage: coord.describe() → process_coordinates((coord.x, coord.y))
```

### Multi-Return Value Binding

```yaoxiang
# === Multi-return value binding ===

min_max: (list: List(Int)) -> (Int, Int) = {
    min = list.reduce(Int.MAX, (a, b) => if a < b then a else b)
    max = list.reduce(Int.MIN, (a, b) => if a > b then a else b)
    return (min, max)
}

List.range: (T:Type)->((self: List(T)) -> (T, T)) = min_max[1]
# Usage: (min_val, max_val) = list.range()
```

## Detailed Design

### Compiler Implementation

### Type Checking Rules

```rust
fn check_binding_type_compatibility(
    binding: &Binding,
    func: &Function
) -> Result<(), TypeError> {
    // 1. If positions are auto-searched (not explicitly specified), check whether a match is found
    if binding.positions.is_empty() {
        return Err(TypeError::NoMatchingParameter(
            binding.type_name.clone(),
            func.name.clone()
        ));
    }

    // 2. Verify that all position indices are valid
    for pos in &binding.positions {
        if *pos >= func.params.len() {
            return Err(TypeError::InvalidBindingPosition(*pos));
        }
    }

    // 3. Check type compatibility at binding positions
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. Check that method call arguments match the remaining parameters
    Ok(())
}
```

### Runtime Behavior

| Scenario | Binding Syntax | Invocation | Converts To |
|----------|---------------|-----------|-------------|
| No binding | `Point.distance = distance` | `Point.distance(p1, p2)` | `distance(p1, p2)` |
| Single position | `Point.distance = distance[0]` | `p1.distance(p2)` | `distance(p1, p2)` |
| Single position | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| Negative index | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| Multi-position (currying) | `Point.scale = scale[0]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| Placeholder | `Type.method = func[1]` | `obj.method(arg)` | `func(arg, obj)` |

**Notes**:
- **No binding**: `Point.name = func` is merely a namespace alias, with no `.` call syntax
- `[0]`: the caller binds to position 0 (the first argument)
- `[1]`: the caller binds to position 1 (the second argument)
- `[-1]`: the caller binds to the last position (counted from the end)

## Trade-offs

### Advantages

- **Explicit binding**: `[n]` is the only binding mechanism; omitting it means no binding, with no implicit behavior
- **Precise control**: can bind to any parameter position, highly flexible
- **Type safety**: full type checking at compile time; binding only occurs when types match
- **Concise syntax**: the `[position]` syntax is intuitive and easy to understand
- **No `self` keyword**: keeps the language concise
- **Currying friendly**: naturally supports partial application and chained calls
- **OOP friendly**: automatic currying lets OOP programmers migrate without thinking

### Disadvantages

- **Learning cost**: requires understanding the position index concept
- **Compilation complexity**: binding resolution and type checking increase compiler complexity
- **Debugging difficulty**: error messages must clearly point out binding position issues

## Alternatives

| Approach | Description | Why Not Chosen |
|----------|-------------|----------------|
| `self` keyword | Introduce Python/Rust-style `self` | Violates YaoXiang's design philosophy of no implicit `self` |
| Named-argument binding | Use named arguments like `func(a=obj)` | Requires modifying function signature definitions, increasing complexity |
| Macro system | Implement binding via macros | Large runtime overhead, reduced type safety |
| Operator overloading | Restrict `self` to specific positions | Inconsistent syntax, confusing semantics |

## Implementation Strategy

### Phased Plan

1. **Phase 1: Basic Binding** (v0.3)
   - Implement the single-position `[n]` binding syntax (n starts at 0, supports negative numbers)
   - Basic type checking and code generation
   - Unit test coverage

2. **Phase 2: Advanced Features** (v0.5)
   - Support range syntax `[n..m]`
   - Compile-time position computation optimization

### Dependencies

- No external dependencies
- No direct relationship with RFC-001 (error handling)
- Can be implemented independently

### Risks

- Compatibility handling with existing binding syntax
- Performance optimization strategy (compile-time expansion vs runtime lookup)

## Open Questions

The following questions have been resolved in the design and are recorded in Appendix A:

- ~~Position indices start at 0~~ → Decided: start at 0
- ~~Negative indices~~ → Decided: supported
- ~~Placeholder~~ → Decided: use `_`
- ~~Range syntax~~ → Decided: to be implemented

**Remaining open questions**:

- [ ] Compatibility handling with existing binding syntax
- [ ] Performance optimization strategy (compile-time expansion vs runtime lookup)

---

## Appendix

### Appendix A: Design Decision Records

| Decision | Resolution | Reason |
|----------|-----------|--------|
| Index base | Start at 0 | Consistent with tuple/parameter list indexing |
| Negative indices | Supported | Flexible, counted from the end |
| Placeholder | `_` | Concise, universal symbol |
| Range syntax | Implemented | Batch binding, e.g., `[0..2]` |
| Syntax style | Infix `Type.method = func[positions]` | Unified with RFC-010 |
| **Binding rule** | **Explicit `[n]` is required to bind; omitting it means no binding** | **No implicit behavior; function definitions and bindings are orthogonal** |
| **Namespace** | **`Type.name` is merely namespace ownership, does not trigger binding** | **Definition and binding are separated** |
| **Function syntax** | **Parameter names declared in the signature `name: (params) -> Return`** | **Unified with RFC-010** |

### Appendix B: Glossary

| Term | Definition |
|------|-----------|
| Binding position | An index position in the function parameter list |
| Joint binding | Binding a type to multiple parameter positions |
| Partial application | Providing only some arguments, returning a function awaiting completion |
| **Unified syntax** | **`name: (params) -> Return = body`, parameter names declared in the signature** |
| **Namespace function** | **`Type.name` syntax; the function belongs to Type's namespace, no implicit binding** |
| **Explicit binding** | **`Type.name = func[n]`, the only method binding mechanism** |

---

## References

- [Rust impl syntax](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell type classes](https://wiki.haskell.org/Type_class)
- [Kotlin extension functions](https://kotlinlang.org/docs/extensions.html)