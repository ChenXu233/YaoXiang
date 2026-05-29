---
title: RFC-004: Multi-Position Union Binding Design for Curried Methods
---

# RFC-004: Multi-Position Union Binding Design for Curried Methods

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2026-02-18 (Added builtin binding and postfix binding syntax)

## Abstract

This RFC proposes a brand-new **multi-position union binding** syntax that allows precise binding of functions to any parameter position of a type, supporting both single-position binding and multi-position union binding. It fundamentally solves the "who is the caller" problem in curried binding without introducing a `self` keyword.

## Motivation

### Why is this feature needed?

The current language design faces the following problems when binding standalone functions as type methods:

1. **Inflexible caller position**: Traditional binding can only fix `obj` in `obj.method(args)` as the first parameter
2. **Difficulty with multi-parameter binding**: When a method needs to receive multiple parameters of the same type, there is no elegant way to express this
3. **Currying semantic ambiguity**: During partial application, it's difficult to distinguish "which position is being bound to"

### Design Goal: Unify Two Programming Perspectives

This design aims to **unify functional and OOP programming perspectives**:

```yaoxiang
# Functional perspective: explicitly pass all parameters
distance(p1, p2)

# OOP perspective: implicit this
p1.distance(p2)

# [positions] syntax sugar makes both equivalent; both are function calls at their core
Point.distance = distance[0]   # this bound to position 0
```

**Core Values**:
- Underlying is functions, upper layer is method syntax
- No `self` keyword introduced, keeping the language concise
- Fully functional: method calls are essentially parameter passing
- `[0]`, `[1]`, `[-1]` flexibly control this binding position
- **Unified syntax**: Function definitions use `name: (params) -> Return = body` format

### Current Problems

```yaoxiang
# Problems with the current design:
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# Can only bind to the first parameter
Point.distance = distance  # equivalent to distance[0]
# p1.distance(p2) → distance(p1, p2) ✓

# But what if transform's signature is transform(Vector, Point)?
# Cannot express the semantics of p1.transform(v1) → transform(v1, p1)
```

## Proposal

### Core Design: Default Binding + Optional Position Specification

#### Default Binding to First Type-Matching Position

**Default behavior**: `Type.method = function` automatically finds the first position that matches that type and binds to it

```yaoxiang
# Default bind to first type-matching position
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

# Only explicitly specify when you need a special position (not the first match)
Point.compare = distance[1]        # Bind to second Point parameter
p1.compare(p2)                    # → distance(p2, p1)
```

**Binding failure handling**:
- **No matching type found**: If the function has no parameters of that type, report error or warning
- **Factory function pattern**: If no parameters match, may be used as a factory function

```yaoxiang
# Case 1: No matching type found
create_point: () -> Point = { ... }
Point.create = create_point        # Error: no Point type parameter

# Case 2: Factory function pattern (optional)
Point.create = create_point        # As factory function, call: Point.create()
```

**Benefits**:
- Smart binding: automatic matching by type, intuitive
- Type safety: only binds when types match, preventing errors
- Flexible control: explicitly specify position when default binding is not the desired behavior

#### Automatic Currying Binding

When the number of function parameters > number of binding positions, automatically generate a curried function:

```yaoxiang
Point: Type = { x: Float, y: Float }

# Base function: 3 parameters
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# Auto-curried on binding
Point.scale = scale[0, 1]   # Point bound to positions 0, 1; position 2 retained

# Auto partial application on call
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0) direct call
result = scaled              # → Point(4.0, 6.0)

# Chained calls are more elegant
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### Position Index Binding Syntax

Introduce `[position]` syntax for precise control of the binding relationship between function parameters and types:

```yaoxiang
# Syntax format: Type.method = function[positions]

# === Basic Binding ===

# Single position binding
Point.distance = distance[1]           # Bind to parameter 1 (index starts at 0)
# Usage: p1.distance(p2) → distance(p2, p1)

# Multi-position union binding (tuple destructuring)
Point.transform = transform[1, 2]      # Bind to parameters 1, 2
# Usage: p1.transform(v1) → transform(v1, p1)
# Original function signature: transform(Point, Vector) → Point
# After binding: Point.transform(Vector) → Point
```

### Detailed Syntax Definition

```
Binding Declaration ::= Type '.' Identifier '=' Function '[' Position List ']'

Position List ::= Position (',' Position)*
Position     ::= Integer                    # Placeholder
              | '_'                        # Skip this position (placeholder)
              | Integer '..' Integer       # Position range (future extension)

Function     ::= Identifier
Type         ::= Identifier (Generic Parameters)?
```

### Builtin Binding

Bindings can be written directly inside the type definition body without separate binding statements:

```yaoxiang
# Method 1: Direct binding inside type definition
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # Bind to position 0
}

# Method 2: Anonymous function + position binding
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
- The generated method signature: `b: Point -> Float` (position 0 filled by caller)

### Usage Examples

```yaoxiang
# === Complete Examples ===

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

# 2. Transform operation (multi-position binding)
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# Binding Point.transform = transform[1]
# Call: p.transform(v) → transform(v, p) ❌
# Binding Point.transform = transform[0]
# Call: p.transform(v) → transform(p, v) ✓

# 3. Complex multi-parameter function
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# Bind only parameter 1 (Point type), retain parameter 3
Point.scale = multiply[0, _]
# Call: p.scale(2.0) → multiply(p, 2.0)

# 4. Cross-type binding
Circle: Type = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# Bind distance method to Circle type
Circle.distance = distance[0, 1]
# Call: c1.distance(c2) → distance(c1, c2)
```

### Tuple Destructuring Support

```yaoxiang
# === Tuple Destructuring Binding ===

# Function receiving tuple parameter
process_coordinates: (coord: (Float, Float)) -> String = {
    return match coord {
        (0.0, 0.0) -> "origin"
        (x, 0.0) -> "on x-axis at ${x}"
        (0.0, y) -> "on y-axis at ${y}"
        (x, y) -> "point at (${x}, ${y})"
    }
}

Coord: Type = { x: Float, y: Float }

# Auto-destructuring binding: Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# Usage: coord.describe() → process_coordinates((coord.x, coord.y))
```

### Multi-Return Value Binding

```yaoxiang
# === Multi-Return Value Binding ===

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
    // 1. If automatic position lookup (not explicitly specified), check if match found
    if binding.positions.is_empty() {
        return Err(TypeError::NoMatchingParameter(
            binding.type_name.clone(),
            func.name.clone()
        ));
    }

    // 2. Validate all position indices are valid
    for pos in &binding.positions {
        if *pos >= func.params.len() {
            return Err(TypeError::InvalidBindingPosition(*pos));
        }
    }

    // 3. Check type compatibility of binding positions
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. Check method call arguments match remaining parameters
    Ok(())
}
```

### Runtime Behavior

| Scenario | Binding Syntax | Call | Transforms To |
|----------|----------------|------|---------------|
| Default binding | `Point.distance = distance` | `p1.distance(p2)` | `distance(p1, p2)` |
| Auto-match | `Point.transform = transform` | `p.transform(v)` | `transform(p, v)` |
| Single position | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| Single position | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| Auto-currying | `Point.scale = scale[0, _]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| Placeholder | `Type.method = func[1, _]` | `obj.method(arg)` | `func(arg, obj)` |

**Explanation**:
- **Default binding**: automatically finds the first type-matching position
- `[0]`: this bound to position 0 (first parameter)
- `[1]`: this bound to position 1 (second parameter)
- `[-1]`: this bound to last position (counted from end)

## Tradeoffs

### Advantages

- **Smart default binding**: defaults to first type-matching position, no explicit `[positions]` needed
- **Precise control**: can bind to any parameter position, high flexibility
- **Type safety**: full compile-time type checking, only binds when types match
- **Concise syntax**: `[position]` syntax is intuitive and easy to understand
- **No `self` keyword**: keeps the language concise
- **Currying-friendly**: naturally supports partial application and method chaining
- **OOP-friendly**: auto-currying allows OOP programmers to migrate effortlessly

### Disadvantages

- **Learning curve**: need to understand the concept of position indices
- **Compilation complexity**: binding resolution and type checking increase compiler complexity
- **Debugging difficulty**: error messages need to clearly indicate binding position issues

## Alternative Solutions

| Solution | Description | Why Not Chosen |
|----------|-------------|----------------|
| `self` keyword | Introduce Python/Rust-style `self` | Violates YaoXiang's design philosophy of no implicit `self` |
| Named parameter binding | Use named parameters `func(a=obj)` | Requires modifying function signature definitions, adds complexity |
| Macro system | Implement binding with macros | Runtime overhead, reduced type safety |
| Operator overloading | Restrict `self` to specific positions | Inconsistent syntax, confusing semantics |

## Implementation Strategy

### Phase Division

1. **Phase 1: Basic Binding** (v0.3)
   - Implement single-position `[n]` binding syntax (n starts at 0, supports negative numbers)
   - Basic type checking and code generation
   - Unit test coverage

2. **Phase 2: Advanced Features** (v0.5)
   - Support range syntax `[n..m]`
   - Compile-time position calculation optimization

### Dependencies

- No external dependencies
- No direct relation to RFC-001 (error handling)
- Can be implemented independently

### Risks

- Compatibility handling with existing binding syntax
- Performance optimization strategy (compile-time expansion vs. runtime lookup)

## Open Issues

The following issues have been resolved in the design, recorded in Appendix A:

- ~~Position index starting at 0~~ → Decided: starts at 0
- ~~Negative indices~~ → Decided: supported
- ~~Placeholder~~ → Decided: use `_`
- ~~Range syntax~~ → Decided: implement

**Remaining Open Issues**:

- [ ] Compatibility handling with existing binding syntax
- [ ] Performance optimization strategy (compile-time expansion vs. runtime lookup)

---

## Appendix

### Appendix A: Design Decision Record

| Decision | Decision Made | Reason |
|----------|---------------|--------|
| Index base | Starts at 0 | Consistent with tuple/parameter list indexing |
| Negative indices | Supported | Flexible, counted from end |
| Placeholder | `_` | Concise, universal symbol |
| Range syntax | Implement | Batch binding, e.g., `[0..2]` |
| Syntax style | Infix `Type.method = func[positions]` | Unified with RFC-010 |
| **Default binding logic** | **Bind to first type-matching position** | **Smarter, safer, intuitive** |
| **Binding failure handling** | **Error/warning/factory function when no match found** | **Flexible handling based on context** |
| **Function syntax** | **Parameter names in signature `name: (params) -> Return`** | **Unified with RFC-010** |

### Appendix B: Glossary

| Term | Definition |
|------|------------|
| Binding position | Index position in the function parameter list |
| Union binding | Binding a type to multiple parameter positions |
| Partial application | Providing only some parameters, returning a function waiting for the rest |
| **Unified syntax** | **`name: (params) -> Return = body`, parameter names declared in signature** |
| **Type-matching binding** | **Default binding logic: automatically finds first position matching caller's type** |
| **Factory function binding** | **When no function parameters match, used as a constructor** |

---

## References

- [Rust impl syntax](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell Type Classes](https://wiki.haskell.org/Type_class)
- [Kotlin Extension Functions](https://kotlinlang.org/docs/extensions.html)