---
title: 'RFC-004: Multi-Position Joint Binding Design for Curried Methods'
status: 'Accepted'
author: 'Chenxu'
created: '2025-01-05'
updated: '2026-09-16 (Added § Curried Chain Termination and Parenthesis Semantics)'
issue: '#132'
---

# RFC-004: Multi-Position Joint Binding Design for Curried Methods

> **Addendum (2026-09-16, Parenthesis Semantics)**: The curried chain in this document has a
> termination rule not previously written. **Parentheses** at the return position declare "this is a
> complete type (value)", at which point the chain terminates:
>
> ```yaoxiang
> f: (a: Int) -> (b: Int) -> Int       # Curried: two parameter segments, f(1)(2)
> g: (a: Int) -> ((b: Int) -> Int)    # Returns a function: one parameter segment, g(1) gives a function
> ```
>
> Rationale: annotations are types. `g` declares that `g(1) : (b: Int) -> Int`, so `g(1)` must
> **be** that function (rather than "the next parameter segment"). This is also the intent of the
> notation in RFC-010 and the language specification,
> `map: (T: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`, consistent with the method
> signature `-> ((self: ...) -> ...)` in `stdlib.md`.
>
> Reflexively, all nested `Fn` without parentheses are curried (including `(T: Type) -> (x: T) -> T`
> from RFC-011). The two behaviors are distinguishable and both usable.
>
> Implementation: `Type::Paren` is only produced when "parentheses do not form a parameter group"
> (i.e., the parentheses are not followed by `->`), and is only perceived by two places: chain
> splitting and the formatter. In all other stages it is treated as transparent.

## Summary

This RFC proposes a brand-new **multi-position joint binding** syntax, allowing functions to be
precisely bound to any parameter position of a type, supporting single-position binding and
multi-position joint binding, fundamentally solving the problem of "who is the caller" in curried
binding, without needing to introduce the `self` keyword.

## Motivation

### Why is this feature needed?

In the current language design, binding independent functions as type methods faces the following
issues:

1. **Inflexible caller position**: Traditional binding can only fix `obj` in `obj.method(args)` as
   the first parameter
2. **Difficulty with multi-parameter binding**: When a method needs to receive multiple parameters
   of the same type, there is no elegant way to express it
3. **Ambiguity in curried semantics**: During partial application, it's hard to distinguish "which
   position to bind to"

### Design Goal: Unifying Two Programming Perspectives

This design aims to **unify the functional and OOP programming perspectives**:

```yaoxiang
# Functional perspective: explicitly pass all arguments
distance(p1, p2)

# OOP perspective: implicit this
p1.distance(p2)

# The [positions] syntax sugar makes the two equivalent; both are essentially function calls
Point.distance = distance[0]   # this binds to position 0
```

**Core values**:

- The bottom layer is functions, the top layer is method syntax
- No `self` keyword is introduced, keeping the language concise
- Fully functional: method calls are essentially argument passing
- `[0]`, `[1]`, `[-1]` flexibly control where this is bound
- **Unified syntax**: function definitions use the `name: (params) -> Return = body` format

### Current Issues

```yaoxiang
# Problems with the current design:
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# Can only bind to the first parameter
Point.distance = distance  # equivalent to distance[0]
# p1.distance(p2) → distance(p1, p2) ✓

# But what if the signature of transform is transform(Vector, Point)?
# The semantics of p1.transform(v1) → transform(v1, p1) cannot be expressed
```

## Proposal

### Core Design: Explicit Position Specification

**Core rule: omitting `[n]` = no binding.** `Point.name = func` is merely a namespace alias and does
not trigger any implicit binding. To enable the `.` call syntax like `p.name(args)`, you must
explicitly specify: `Point.name = func[n]`.

#### Single-Position Binding

```yaoxiang
# Explicitly bind to the first Point parameter position (indices start from 0)
Point.distance = distance[0]
p1.distance(p2)                     # → distance(p1, p2)

# Bind to the second Point parameter position
Point.compare = distance[1]         # bind to the second Point parameter
p1.compare(p2)                      # → distance(p2, p1)
```

**Omitting `[n]` = no binding**:

```yaoxiang
# No [n] → pure namespace alias, no . call syntax
Point.distance = distance            # only Point.distance(p1, p2)
# p1.distance(p2)  ❌  no binding

# Factory functions are naturally legal, no special handling needed
create_point: () -> Point = { ... }
Point.create = create_point          # Point.create()   ✅
```

- Type safety: only binds when types match, avoiding errors
- Flexible control: precisely control the binding position via `[n]`

#### Curried Binding

When the number of function parameters exceeds the number of binding positions, a curried function
is automatically generated. **Binding is always an explicit operation.**

```yaoxiang
Point: Type = { x: Float, y: Float }

# Basic function
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# Explicitly bind to position 0 → curried: the remaining parameter factor is provided by the caller
Point.scale = scale[0]

# Call
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0)

# Chained calls are more elegant
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### Position Index Binding Syntax

Introduce the `[position]` syntax to precisely control the binding relationship between function
parameters and types:

```yaoxiang
# Syntax format: Type.method = function[positions]

# === Basic Binding ===

# Single-position binding
Point.distance = distance[1]           # bind to parameter 1 (indices start from 0)
# Usage: p1.distance(p2) → distance(p2, p1)

# Multi-position joint binding (tuple destructuring)
Point.transform = transform[1, 2]      # bind to parameters 1 and 2
# Usage: p1.transform(v1) → transform(v1, p1)
# Original function signature: transform(Point, Vector) → Point
# After binding: Point.transform(Vector) → Point
```

### Detailed Syntax Definition

```
BindingDecl ::= Type '.' Identifier '=' FunctionName '[' PositionList ']'

PositionList ::= Position (',' Position)*
Position     ::= Integer                # placeholder
              | '_'                     # skip this position (placeholder)
              | Integer '..' Integer    # position range (future extension)

FunctionName ::= Identifier
Type         ::= Identifier (GenericParams)?
```

### Built-in Binding

Binding can be written directly inside the type definition body, without needing a separate binding
statement:

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

**Curried Semantics**:

- When binding `distance = distance[0]`, the original function signature is
  `(a: Point, b: Point) -> Float`
- The generated method signature: `b: Point -> Float` (position 0 is filled by the caller)

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

# Bind only the first parameter (Point type), keep the third parameter
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
# === Tuple Destructuring Binding ===

# Function takes a tuple parameter
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

### Multi-Return-Value Binding

```yaoxiang
# === Multi-Return-Value Binding ===

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
    // 1. If automatically finding a position (not explicitly specified), check whether a match is found
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

    // 4. Check that method call arguments match the remaining parameters
    Ok(())
}
```

### Runtime Behavior

| Scenario               | Binding Syntax                 | Call                     | Translates To      |
| ---------------------- | ------------------------------ | ------------------------ | ------------------ |
| No binding             | `Point.distance = distance`    | `Point.distance(p1, p2)` | `distance(p1, p2)` |
| Single position        | `Point.distance = distance[0]` | `p1.distance(p2)`        | `distance(p1, p2)` |
| Single position        | `Point.distance = distance[1]` | `p1.distance(p2)`        | `distance(p2, p1)` |
| Negative index         | `Point.test = func[-1]`        | `p.test(a, b)`           | `func(a, b, p)`    |
| Multi-position (curry) | `Point.scale = scale[0]`       | `p.scale(2.0)`           | `scale(p, 2.0)`    |
| Placeholder            | `Type.method = func[1]`        | `obj.method(arg)`        | `func(arg, obj)`   |

**Notes**:

- **No binding**: `Point.name = func` is merely a namespace alias, with no `.` call syntax
- `[0]`: caller is bound to position 0 (the first parameter)
- `[1]`: caller is bound to position 1 (the second parameter)
- `[-1]`: caller is bound to the last position (counting from the end)

## Trade-offs

### Advantages

- **Explicit binding**: `[n]` is the only binding mechanism; omitting it means no binding, with no
  implicit behavior
- **Precise control**: can bind to any parameter position, highly flexible
- **Type safety**: fully type-checked at compile-time, only binds when types match
- **Concise syntax**: the `[position]` syntax is intuitive and easy to understand
- **No `self` keyword**: keeps the language concise
- **Curry-friendly**: naturally supports partial application and chained calls
- **OOP-friendly**: automatic currying makes migration seamless for OOP programmers

### Disadvantages

- **Learning cost**: requires understanding the concept of position indices
- **Compile complexity**: binding resolution and type checking increase compiler complexity
- **Debugging difficulty**: error messages need to clearly point out binding position issues

## Alternatives

| Option                  | Description                             | Why Not Chosen                                                      |
| ----------------------- | --------------------------------------- | ------------------------------------------------------------------- |
| `self` keyword          | Introduce Python/Rust-style `self`      | Violates YaoXiang's design philosophy of no implicit `self`         |
| Named parameter binding | Use named parameters like `func(a=obj)` | Requires modifying function signature definition, adding complexity |
| Macro system            | Use macros to implement binding         | High runtime overhead, reduced type safety                          |
| Operator overloading    | Restrict `self` to specific positions   | Inconsistent syntax, confusing semantics                            |

## Implementation Strategy

### Phasing

1. **Phase 1: Basic Binding** (v0.3)
   - Implement single-position `[n]` binding syntax (n starts from 0, negative numbers supported)
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

## Open Questions

The following questions have been resolved in the design, recorded in Appendix A:

- ~~Position indices start from 0~~ → Decided: start from 0
- ~~Negative indices~~ → Decided: supported
- ~~Placeholders~~ → Decided: use `_`
- ~~Range syntax~~ → Decided: implement

**Remaining Open Questions**:

- [ ] Compatibility handling with existing binding syntax
- [ ] Performance optimization strategy (compile-time expansion vs. runtime lookup)

---

## Appendix

### Appendix A: Design Decision Records

| Decision            | Decision                                                               | Rationale                                                                |
| ------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Index base          | Start from 0                                                           | Consistent with tuple/parameter list indexing                            |
| Negative indices    | Supported                                                              | Flexible, counts from the end                                            |
| Placeholder         | `_`                                                                    | Concise, universal symbol                                                |
| Range syntax        | Implement                                                              | Batch binding, e.g., `[0..2]`                                            |
| Syntax style        | Infix `Type.method = func[positions]`                                  | Unifies with RFC-010                                                     |
| **Binding rule**    | **Explicit `[n]` to bind; omitting means no binding**                  | **No implicit behavior; function definition and binding are orthogonal** |
| **Namespace**       | **`Type.name` is only namespace membership, does not trigger binding** | **Separation of definition and binding**                                 |
| **Function syntax** | **Parameter names declared in signatures `name: (params) -> Return`**  | **Unifies with RFC-010**                                                 |

### Appendix B: Glossary

| Term                   | Definition                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------ |
| Binding position       | The index position in a function's parameter list                                          |
| Joint binding          | Binding a type to multiple parameter positions                                             |
| Partial application    | Providing only some parameters, returning a function awaiting completion of the call       |
| **Unified syntax**     | **`name: (params) -> Return = body`, parameter names declared in the signature**           |
| **Namespace function** | **`Type.name` syntax: the function belongs to Type's namespace, with no implicit binding** |
| **Explicit binding**   | **`Type.name = func[n]`, the only method binding mechanism**                               |

---

## References

- [Rust impl syntax](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell type classes](https://wiki.haskell.org/Type_class)
- [Kotlin extension functions](https://kotlinlang.org/docs/extensions.html)
