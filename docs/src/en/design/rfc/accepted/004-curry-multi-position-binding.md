---
title: 'RFC-004: Multi-Position Union Binding Design for Curried Methods'
status: 'Accepted'
author: 'Chen Xu'
created: '2025-01-05'
updated: '2026-02-18 (Added builtin binding and postfix binding syntax)'
issue: '#132'
---

# RFC-004: Multi-Position Union Binding Design for Curried Methods

## Summary

This RFC proposes a new **multi-position union binding** syntax that allows precisely binding
functions to any parameter position of a type, supporting both single-position binding and
multi-position union binding. It fundamentally solves the "who is the caller" problem in curried
bindings without introducing a `self` keyword.

## Motivation

### Why is this feature needed?

In the current language design, binding standalone functions as type methods faces the following
issues:

1. **Inflexible caller position**: Traditional binding can only fix `obj` in `obj.method(args)` as
   the first parameter
2. **Difficult multi-parameter binding**: When a method needs to receive multiple parameters of the
   same type, there is no elegant way to express it
3. **Currying semantic ambiguity**: During partial application, it is difficult to distinguish
   "which position is being bound"

### Design Goal: Unify Two Programming Perspectives

This design aims to **unify functional and OOP programming perspectives**:

```yaoxiang
# Functional perspective: explicitly pass all arguments
distance(p1, p2)

# OOP perspective: implicit this
p1.distance(p2)

# [positions] syntax makes both equivalent; both are essentially function calls
Point.distance = distance[0]   # this bound to position 0
```

**Core Value**:

- The底层是函数，上层是方法语法
- Does not introduce `self` keyword, keeping the language simple
- Fully functional: method calls are essentially parameter passing
- `[0]`, `[1]`, `[-1]` flexibly control this binding position
- **Syntax unification**: function definitions use `name: (params) -> Return = body` format

### Current Problems

```yaoxiang
# Problems with existing design:
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# Can only bind to the first parameter
Point.distance = distance  # equivalent to distance[0]
# p1.distance(p2) → distance(p1, p2) ✓

# But what if transform's signature is transform(Vector, Point)?
# Cannot express p1.transform(v1) → transform(v1, p1) semantics
```

## Proposal

### Core Design: Explicit Position Specification

**Core Rule: Not writing `[n]` = No binding.** `Point.name = func` is only a namespace alias and
does not trigger any implicit binding. For `.` call syntax like `p.name(args)` to work, you must
explicitly specify: `Point.name = func[n]`.

#### Single-Position Binding

```yaoxiang
# Explicitly bind to the first Point parameter position (index starts at 0)
Point.distance = distance[0]
p1.distance(p2)                     # → distance(p1, p2)

# Bind to the second Point parameter position
Point.compare = distance[1]         # bind to second Point parameter
p1.compare(p2)                      # → distance(p2, p1)
```

**Not writing `[n]` = No binding**:

```yaoxiang
# No [n] → pure namespace alias, no . call syntax
Point.distance = distance            # only Point.distance(p1, p2)
# p1.distance(p2)  ❌  no binding

# Factory functions are naturally valid, no special handling needed
create_point: () -> Point = { ... }
Point.create = create_point          # Point.create()   ✅
```

- Type safe: only binds when types match, preventing errors
- Flexible control: precisely control binding position via `[n]`

#### Curried Binding

When the number of function parameters > number of binding positions, a curried function is
automatically generated. **Binding is always an explicit operation.**

```yaoxiang
Point: Type = { x: Float, y: Float }

# Base function
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# Explicitly bind to position 0 → currying: remaining parameter factor provided by caller
Point.scale = scale[0]

# Call
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0)

# Chained calls are more elegant
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### Position Index Binding Syntax

Introduce `[position]` syntax to precisely control the binding relationship between function
parameters and types:

```yaoxiang
# Syntax format: Type.method = function[positions]

# === Basic Binding ===

# Single-position binding
Point.distance = distance[1]           # bind to parameter 1 (index starts at 0)
# Usage: p1.distance(p2) → distance(p2, p1)

# Multi-position union binding (tuple destructuring)
Point.transform = transform[1, 2]      # bind to parameters 1, 2
# Usage: p1.transform(v1) → transform(v1, p1)
# Original function signature: transform(Point, Vector) → Point
# After binding: Point.transform(Vector) → Point
```

### Detailed Syntax Definition

```
Binding Declaration ::= Type '.' Identifier '=' FunctionName '[' PositionList ']'

PositionList ::= Position (',' Position)*
Position     ::= Integer                    # placeholder
           | '_'                    # skip this position (placeholder)
           | Integer '..' Integer         # position range (future extension)

FunctionName ::= Identifier
Type     ::= Identifier (GenericParameters)?
```

### Builtin Binding

Bindings can be written directly inside the type definition body without a separate binding
statement:

```yaoxiang
# Method 1: bind directly inside type definition
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

**Currying Semantics**:

- When binding `distance = distance[0]`, the original function signature is
  `(a: Point, b: Point) -> Float`
- The generated method signature is: `b: Point -> Float` (position 0 filled by caller)

### Usage Examples

```yaoxiang
# === Complete Example ===

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

# Bind only parameter 1 (Point type), keep parameter 3
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
    // 1. If auto-finding position (not explicitly specified), check if match found
    if binding.positions.is_empty() {
        return Err(TypeError::NoMatchingParameter(
            binding.type_name.clone(),
            func.name.clone()
        ));
    }

    // 2. Verify all position indices are valid
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

    // 4. Check method call arguments match remaining parameters
    Ok(())
}
```

### Runtime Behavior

| Scenario        | Binding Syntax                 | Call                     | Transforms to      |
| --------------- | ------------------------------ | ------------------------ | ------------------ |
| No binding      | `Point.distance = distance`    | `Point.distance(p1, p2)` | `distance(p1, p2)` |
| Single position | `Point.distance = distance[0]` | `p1.distance(p2)`        | `distance(p1, p2)` |
| Single position | `Point.distance = distance[1]` | `p1.distance(p2)`        | `distance(p2, p1)` |
| Negative index  | `Point.test = func[-1]`        | `p.test(a, b)`           | `func(a, b, p)`    |
| Multi-position  | `Point.scale = scale[0]`       | `p.scale(2.0)`           | `scale(p, 2.0)`    |
| (curried)       |                                |                          |                    |
| Placeholder     | `Type.method = func[1]`        | `obj.method(arg)`        | `func(arg, obj)`   |

**Explanation**:

- **No binding**: `Point.name = func` is only a namespace alias, no `.` call syntax
- `[0]`: caller bound to position 0 (first parameter)
- `[1]`: caller bound to position 1 (second parameter)
- `[-1]`: caller bound to last position (counting from end)

## Trade-offs

### Advantages

- **Explicit binding**: `[n]` is the sole binding mechanism; no binding without it, no implicit
  behavior
- **Precise control**: can bind to any parameter position, high flexibility
- **Type safe**: complete compile-time type checking, only binds when types match
- **Concise syntax**: `[position]` syntax is intuitive and easy to understand
- **No `self` keyword**: keeps the language simple
- **Currying friendly**: naturally supports partial application and chained calls
- **OOP friendly**: automatic currying allows OOP programmers to migrate effortlessly

### Disadvantages

- **Learning curve**: requires understanding position indexing concept
- **Compiler complexity**: binding resolution and type checking increase compiler complexity
- **Debugging difficulty**: error messages need to clearly indicate binding position issues

## Alternative Approaches

| Approach             | Description                           | Why Not Chosen                                                    |
| -------------------- | ------------------------------------- | ----------------------------------------------------------------- |
| `self` keyword       | Introduce Python/Rust-style `self`    | Violates YaoXiang's design philosophy of no implicit `self`       |
| Named parameter      | Use named parameters `func(a=obj)`    | Requires modifying function signature definition, adds complexity |
| binding              |                                       |                                                                   |
| Macro system         | Implement binding via macros          | High runtime overhead, reduced type safety                        |
| Operator overloading | Restrict `self` to specific positions | Inconsistent syntax, confusing semantics                          |

## Implementation Strategy

### Phase Breakdown

1. **Phase 1: Basic Binding** (v0.3)
   - Implement single-position `[n]` binding syntax (n starts at 0, supports negative)
   - Basic type checking and code generation
   - Unit test coverage

2. **Phase 2: Advanced Features** (v0.5)
   - Support range syntax `[n..m]`
   - Compile-time position calculation optimization

### Dependencies

- No external dependencies
- No direct relationship with RFC-001 (error handling)
- Can be implemented independently

### Risks

- Compatibility handling with existing binding syntax
- Performance optimization strategy (compile-time expansion vs runtime lookup)

## Open Questions

The following issues have been resolved in the design, recorded in Appendix A:

- ~~Position index starts at 0~~ → Decided: starts at 0
- ~~Negative indices~~ → Decided: supported
- ~~Placeholder~~ → Decided: use `_`
- ~~Range syntax~~ → Decided: implement

**Remaining Open Questions**:

- [ ] Compatibility handling with existing binding syntax
- [ ] Performance optimization strategy (compile-time expansion vs runtime lookup)

---

## Appendices

### Appendix A: Design Decision Record

| Decision            | Decision                                                              | Reason                                                                   |
| ------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Index base          | Starts at 0                                                           | Consistent with tuple/parameter list indexing                            |
| Negative indices    | Supported                                                             | Flexible, counting from end                                              |
| Placeholder         | `_`                                                                   | Concise, universal symbol                                                |
| Range syntax        | Implement                                                             | Batch binding, e.g., `[0..2]`                                            |
| Syntax style        | Infix `Type.method = func[positions]`                                 | Unified with RFC-010                                                     |
| **Binding rule**    | **Explicit `[n]` for binding, none = no binding**                     | **No implicit behavior, function definition and binding are orthogonal** |
| **Namespace**       | **`Type.name` is only namespace ownership, does not trigger binding** | **Definition and binding are separate**                                  |
| **Function syntax** | **Parameter names in signature `name: (params) -> Return`**           | **Unified with RFC-010**                                                 |

### Appendix B: Glossary

| Term                   | Definition                                                                         |
| ---------------------- | ---------------------------------------------------------------------------------- |
| Binding position       | Index position in function parameter list                                          |
| Union binding          | Binding a type to multiple parameter positions                                     |
| Partial application    | Providing only part of the parameters, returning a function for the remaining call |
| **Unified syntax**     | **`name: (params) -> Return = body`, parameter names declared in signature**       |
| **Namespace function** | **`Type.name` syntax, function belongs to Type's namespace, no implicit binding**  |
| **Explicit binding**   | **`Type.name = func[n]`, the sole method binding mechanism**                       |

---

## References

- [Rust impl Syntax](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell Type Classes](https://wiki.haskell.org/Type_class)
- [Kotlin Extension Functions](https://kotlinlang.org/docs/extensions.html)
