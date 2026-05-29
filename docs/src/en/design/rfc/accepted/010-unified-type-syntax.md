---
title: RFC-010: Unified Type Syntax
---

# RFC-010: Unified Type Syntax - the name: type = value Model

> **Status**: Accepted
>
> **Author**: Chen Xu
>
> **Created**: 2025-01-20
>
> **Last Updated**: 2026-03-21 (Phases 1-4 implementation complete, unified Fn/TypeDef/MethodBind into Binding)

## Summary

This RFC proposes an extremely minimal unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression.
**No `fn`, no `struct`, no `trait`, no `impl`, no lowercase `type` keyword (but there is `Type` as a meta type keyword)**.

> **Core Design**: `Type` itself is a generic type. `(T: Type) -> Type` represents "a type that accepts a type parameter T".

| Concept | Code | Description |
|---------|------|-------------|
| Variable | `x: Int = 42` | |
| Function | `add: (a: Int, b: Int) -> Int = a + b` | |
| Record type | `Point: Type = { x: Float, y: Float }` | |
| Interface | `Drawable: Type = { draw: (Surface) -> Void }` | |
| Generic type | `List: (T: Type) -> Type = { data: Array(T), length: Int }` | |
| Generic type | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` | |
| Method | `Point.draw: (self: Point, s: Surface) -> Void = ...` | |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` | |

**`Type` is the only meta type keyword in the language**.
It is used to annotate the type hierarchy. The compiler automatically handles the distinction between Type0, Type1, Type2... transparently for users.

```yaoxiang
// Core syntax: unified + distinguished

// Variable
x: Int = 42

// Function (parameter names in signature)
add: (a: Int, b: Int) -> Int = a + b

// Record type
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// Interface (essentially a record type with all function fields)
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Method definition (using Type.method syntax)
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

// Generic type ((T: Type) -> Type = generic type that accepts type parameters)
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int
}

Map: (K: Type, V: Type) -> Type = {
    keys: Array(K),
    values: Array(V)
}

// Usage
p: Point = Point(1.0, 2.0)
p.draw(screen)           // Syntactic sugar → Point.draw(p, screen)
s: Drawable = p           // Structural subtyping: Point implements Drawable
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## Motivation

### Why is this feature needed?

The current type system has multiple separate concepts:
- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack unity, leading to syntax fragmentation and high learning costs.

### Design Goals

1. **Extreme unification**: One syntax rule covers all cases
2. **Concise elegance**: Symmetrical beauty of `name: type = value`
3. **No new keywords**: Reuse existing syntactic elements
4. **Theoretical elegance**: Types are also values of Type type
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with Generics System

RFC-010's unified syntax model **naturally aligns** with RFC-011's generics system design. Generic parameters integrate seamlessly into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type position in signature can be omitted, auto-inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraints (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:
- RFC-011 Phase 1 (Basic generics) is a **hard dependency** of RFC-010
- Without basic generics, RFC-010's generic examples cannot compile
- Recommendation: Implement RFC-011 Phase 1 and RFC-010 in sync

## Proposal

### Core Principle: Type Constructors vs Functions/Variables

**This is a key design choice that determines the syntax disambiguation rules:**

| Syntax | Meaning | Rule | Description |
|--------|---------|------|-------------|
| **`x: Type = ...`** | Type constructor | `: Type` explicit declaration → forced to type | |
| **`f = ...`** | Function or variable | No `: Type` → HM actively infers as function/variable | |

**Why this design?**

The `{ ... }` syntax itself has ambiguity:
- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executes statements, returns Void)

**Disambiguation rules**:
- **Has** `: Type` → force parse as type constructor, `{ ... }` is a type literal
- **No** `: Type` → HM actively parses `{ ... }` as code block, infers as function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main = { println("Hello") }

# ❌ Error: no : Type, compiler cannot parse { ... } as type
Point = { x: Float, y: Float }  // HM infers as function, not type!
```

---

**Unified model: identifier : type = expression**

```
├── Variable
│   └── x: Int = 42
│
├── Function
│   └── add: (a: Int, b: Int) -> Int = a + b  # No : Type, HM infers as function
│
├── Record type
│   └── Point: Type = { x: Float, y: Float }  # Must return: Type
│
├── Interface
│   └── Drawable: Type = { draw: (Surface) -> Void }  # Must return: Type
│
├── Generic type
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # Must return: Type
│
├── Generic type (multiple parameters)
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # Must return: Type
│
├── Method
│   └── Point.draw: (self: Point, surface: Surface) -> Void = ...
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Meta Type Hierarchy (Compiler Internal)

**The compiler internally** maintains a universe level `level: selfpointnum` (stored as string, theoretically infinitely extensible).

| Level | Description | Description |
|-------|-------------|-------------|
| `Type0` | Everyday types (`Int`, `Float`, `Point`) | |
| `Type1` | Type constructors (`List`, `Maybe`) | |
| `Type2+` | Higher-order constructors | |

**Users never see these numbers**, only `: Type`.

> **Curry-Howard Isomorphism**: The existence of universe levels is not an engineering implementation detail, but a necessary condition for logical consistency. The Curry-Howard isomorphism equates types with propositions. If `Type: Type` were allowed (i.e., "the type of types is also a type"), it would create a Russell's paradox similar to "this sentence is false" — manifested in the type system as Girard's paradox. YaoXiang's `Type0 / Type1 / Type2…` stratification (i.e., cumulative universes in Martin-Löf type theory) ensures each type belongs to exactly one level, with `Typeₙ : Typeₙ₊₁` forming an ever-ascending chain that never closes, fundamentally avoiding paradox. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

### Syntax Definitions

#### 1. Variable Declaration

```yaoxiang
// Basic syntax
x: Int = 42
name: String = "Alice"
flag: Bool = true

// Type inference (can be omitted)
y = 100  // Inferred as Int
```

#### 2. Function Definition

```yaoxiang
// Complete syntax (parameter names declared in signature)
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// With parameter names
greet: (name: String) -> String = {
    return "Hello, ${name}!"
}

// Multiple parameters
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }
}

// Multi-line function body
calc2: (x: Float, y: Float) -> Float = {
    if x > y {
        return x
    }
    return y
}
```

#### Return Rules

All functions must explicitly use the `return` keyword to return values (except functions returning `()`):

```yaoxiang
// Non-Void return type - must use return
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Void return type - return optional (usually omitted)
print: (msg: String) -> Void = {
    // No return needed
}

// Single-line expression (directly return value, no return needed)
greet: (name: String) -> String = "Hello, ${name}!"

// Multi-line function body - must use return
max: (a: Int, b: Int) -> Int = {
    if a > b {
        return a
    } else {
        return b
    }
}
```

#### 3. Type Definition

Type definitions are the core of YaoXiang's unified syntax, containing fields, default values, bound methods, and interface implementations:


##### Basic Types

**Record type**: Field list, field types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: Fields can have default values, optional at construction.

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

Usage:

```yaoxiang
Point() → Point(x=0, y=0)
Point(x=1) → Point(x=1, y=0)
Point(x=1, y=2) → Point(x=1, y=2)
```

**Fields without default values**: Must be provided at construction.

```yaoxiang
Point2: Type = {
    x: Float,
    y: Float
}
```

Usage:
```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### Bound Methods

**Method 1: Directly bind external function inside type definition**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // Bind to position 0, curried to method: (b: Point) -> Float
}
// Call: p1.distance(p2) → distance(p1, p2)
```

**Method 2: Anonymous function + position binding**

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// Syntax: ((params) => body)[position]
// Call: p1.distance(p2) → distance(p1, p2)
```

##### Interface Implementation

**Interface name written inside type body, compiler automatically checks implementation**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Point: Type = {
    x: Float,
    y: Float,
    Drawable,          // Implements Drawable interface
    Serializable      // Implements Serializable interface
}
```

##### Interface Definition

**Interface = record type with all function fields**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Empty type/empty interface
EmptyType: Type = {}
Empty: Type = {}
```

##### Method Definition (External)

**Type methods**: Associated with a specific type (using Type.method syntax)

```yaoxiang
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}
```

##### Method Binding (External)

Regular methods can be bound to a type using the `[position]` syntax (detailed syntax in RFC-004).

**Manual binding**:

```yaoxiang
// Explicit binding
Point.distance = distance[0]

// Specify binding position
Point.transform = transform[1]  // this bound to position 1
```

**Multi-position binding**:

```yaoxiang
// Bind multiple positions (auto-curried)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse binding** (type method to regular function):

```yaoxiang
// Type method to regular function
draw_point: (p: Point, surface: Surface) -> Void = Point.draw
```

#### 4. Interface Composition

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Use intersection type
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    return item.serialize()
}
```

#### 5. Generic Types

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T:Type)-((self: List(T), item: T) -> Void),
    get: (T:Type)->((self: List(T), index: Int) -> Maybe(T))
}

// Concrete instantiation (RFC-023 syntax)
IntList: Type = List(Int)

IntList.push = {
    self.data.append(item)
    self.length = self.length + 1
}

List.push = (type: Type) -> {
    return (self: List(type), item: type) -> {
        self.data.append(item)
        self.length = self.length + 1
    }
}

IntList.push(Int)(self, item)  // Call example

// Generic methods (RFC-023 syntax: type parameters auto-inferred from call site)
List.push: (self: List(T), item: T) -> Void = {
    self.data.append(item)
    self.length = self.length + 1
}

List.get: (self: List(T), index: Int) -> Maybe(T) = {
    if index >= 0 && index < self.length {
        return Maybe.Just(self.data[index])
    } else {
        return Maybe.Nothing
    }
}
```

#### 6. Generic Call Syntax

Generic types and generic functions use a unified `()` syntax for calls. `[]` is not used in any generic context.

**Core rules**:

1. **`()` does all application**: Type application, function call, value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function call - type flows automatically from arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on left, value on right**: `name: type = value` — Type parameters declared on the left, right side is always a concrete value. For empty container `List()`, `T` must come from the left-side type annotation.

3. **Type information written once** — At parameter declaration, the compiler carries it along:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String auto from numbers and f's types
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       // Inferred as List(Int)
y = List("a", "b")      // Inferred as List(String)
z = List()              // ❌ Compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int from left annotation
```

5. **Type aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`, `List[Int](1,2,3)` → `List(1,2,3)`.
> The old `[]` generic syntax has been completely removed. `[]` is only used for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========

Drawable: Type = {
    draw: (self: Self, surface: Surface) -> Void,
    bounding_box: (self: Self) -> Rect
}

Serializable: Type = {
    serialize: (self: Self) -> String
}

Transformable: Type = {
    translate: (self: Self, dx: Float, dy: Float) -> Self,
    scale: (self: Self, factor: Float) -> Self
}

// ======== 2. Type Definition ========

Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable,
    Transformable
}

Rect: Type = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable,
    Serializable,
    Transformable
}

// ======== 3. Method Definition ========

// Point's methods
draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

bounding_box: (self: Point) -> Rect = {
    return Rect(self.x - 1, self.y - 1, 2, 2)
}

serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

translate: (self: Point, dx: Float, dy: Float) -> Point = {
    return Point(self.x + dx, self.y + dy)
}

scale: (self: Point, factor: Float) -> Point = {
    return Point(self.x * factor, self.y * factor)
}

// Regular methods (pub, auto-bound to Point.distance)
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// Rect's methods
draw: (self: Rect, surface: Surface) -> Void = {
    surface.draw_rect(self.x, self.y, self.width, self.height)
}

bounding_box: (self: Rect) -> Rect = self

serialize: (self: Rect) -> String = {
    return "Rect(${self.x}, ${self.y}, ${self.width}, ${self.height})"
}

translate: (self: Rect, dx: Float, dy: Float) -> Rect = {
    return Rect(self.x + dx, self.y + dy, self.width, self.height)
}

scale: (self: Rect, factor: Float) -> Rect = {
    return Rect(self.x * factor, self.y * factor, self.width * factor, self.height * factor)
}

// ======== 4. Method Binding ========

Point.distance = distance[0]  // Bind to position 0, curried to method: (p2: Point) -> Float
Point.transform = transform[1]  // Bind to position 1, curried to method: (dx: Float, dy: Float) -> Point
Rect.transform = transform[1]  // Bind to position 1, curried to method: (dx: Float, dy: Float) -> Rect

// ... and so on, binding other methods ...

// ======== 5. Usage ========

// Create instances
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// Method calls (syntactic sugar)
p.draw(screen)
r.draw(screen)

// Regular method calls (direct call)
d: Float = distance(p, Point(0.0, 0.0))

// Chained calls
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// Interface assignment
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// Generic functions (RFC-023 syntax: type parameters omitted at call, auto-inferred)
process_all: (items: List(T)) -> Void = {
    for item in items {
        print(item.serialize())
    }
}

process_all([p, r])
```

## Detailed Design

### Interface Checking Algorithm

```rust
fn check_type_implements_interface(
    typ: &Type,
    iface: &Type
) -> Result<(), TypeError> {
    // For each field of the interface (function fields)
    for (field_name, iface_field) in &iface.fields {
        // Check if type has a method with the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check if method signature is compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Compare: should match after removing self parameter
            if !method_signature_matches(method, iface_field.type_) {
                return Err(TypeError::MethodSignatureMismatch {
                    type_name: typ.name,
                    interface_name: iface.name,
                    method_name: field_name,
                });
            }
        } else {
            return Err(TypeError::MissingMethod {
                type_name: typ.name,
                interface_name: iface.name,
                method_name: field_name,
            });
        }
    }
    Ok(())
}
```

### Interface Direct Assignment and Compile-Time Optimization

Interface types support direct assignment. The compiler automatically selects the optimal calling strategy based on the right-hand value's type:

```yaoxiang
// Direct assignment of concrete type → concrete type determinable at compile-time, zero-cost call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: directly call circle_draw(screen), no vtable

// Function return value → concrete type not determinable at compile-time, use vtable
d: Drawable = get_shape()
d.draw(screen)  // Method lookup through vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Method lookup through vtable
}
```

**Compile-time optimization strategy**:

| Scenario | Inferred Result | Calling Method | Description |
|----------|-----------------|----------------|-------------|
| `d: Drawable = Circle(1)` | Concrete type Circle | Direct call (zero overhead) | |
| `d: Drawable = get_shape()` | Unknown | vtable | |
| `shapes: List(Drawable) = [...]` | Heterogeneous | vtable | |

**Rules**:
1. When the right-hand side is a concrete type constructor and determinable at compile-time, generate direct call IR
2. When the right-hand side type cannot be determined at compile-time, fall back to vtable mechanism
3. vtable fallback guarantees correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as it has the same methods, it can be assigned to an interface type
CustomPoint: Type = {
    draw: (self: CustomPoint, surface: Surface) -> Void,
    x: Float,
    y: Float
}

custom: CustomPoint = CustomPoint(
    (self: CustomPoint, surface: Surface) => surface.plot(self.x, self.y),
    1.0,
    2.0
)
```

### Syntax Changes

| Before | After | Description |
|--------|-------|-------------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` | |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Self, err: (E) -> Self }` | |
| Requires `impl` keyword | No keyword needed, interface name written after type body | |

## Syntax Design Note: Named Functions are Syntactic Sugar for Lambda

### Core Understanding

**Named functions and Lambda expressions are the same thing!** The only difference is that named functions give a Lambda a name.

```yaoxiang
// These two are essentially identical
add: (a: Int, b: Int) -> Int = a + b           // Named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda form (completely equivalent)
```

### Syntactic Sugar Model

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially
name: (Params) -> ReturnType = (params) => body
```

**Key point**: When the signature fully declares parameter types, parameter names in the Lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: Parameters in the signature have scope that covers the function body, with inner scope having higher priority.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x overrides outer x, result is 20
```

### Annotation Position Flexibility

Type annotations can be in any of the following positions, **at least one position is required**:

| Annotation Position | Form | Description |
|---------------------|------|-------------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda head only | `double = (x: Int) => x * 2` | ✅ Valid |
| Both sides | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Examples

```yaoxiang
// ✅ Recommended: complete signature, Lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Valid: type annotation in Lambda head
double = (x: Int) => x * 2

// ✅ Valid: both sides annotated
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature | Advantage | Description |
|---------|-----------|-------------|
| **Concise** | No need to repeat parameter names when signature is complete | |
| **Flexible** | Keep Lambda form available, use whichever you prefer | |
| **Consistent** | Unified pattern with variable declaration `x: Int = 42` | |
| **Intuitive** | `name: Type = body` directly corresponds to "named name, type Type, value body" | |

## Trade-offs

### Advantages

| Advantage | Description | Description |
|-----------|-------------|-------------|
| Extreme unification | One syntax rule covers all cases | |
| Theoretical elegance | Perfectly symmetric `name: type = value` | |
| No new keywords | Reuse existing syntactic elements | |
| Easy to implement | Compiler only needs to handle one declaration form | |
| Easy to learn | Remember one pattern to write all code | |
| Easy to extend | New features naturally fit into this model | |

### Disadvantages

| Disadvantage | Description | Description |
|--------------|-------------|-------------|
| Naming convention | Methods must follow `Type.method` naming | |
| Verbosity | Complete syntax is longer than simplified syntax, but can be inferred | |
| Learning curve | Need to understand the unified model | |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compile error example:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Type can be omitted, compiler infers
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE auto-suggests missing methods
```


### Risks

| Risk | Impact | Mitigation | Description |
|------|--------|------------|-------------|
| Parsing complexity | Unified syntax may increase parsing complexity | Use recursive descent parser | |
| Performance overhead | vtable lookup may have extra overhead | Compile-time monomorphization optimization | |

---

## Easter Egg 🎮: Origin of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Trying to define the type of types...
Type: Type = Type
```

**Warning**: This is an **ineffable** thing!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二，二生三，三生万物。                                   ║
║   易有太极，是生两仪。                                         ║
║                                                              ║
║   One produces two, two produces three,                      ║
║   three produces all things.                                 ║
║   In the Yijing, there is the Great Ultimate,                ║
║   which gives rise to the Two Principles.                    ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language. ║
║   The compiler falls silent here, philosophy dwells.         ║
║                                                              ║
║   Thanks for reaching the philosophical boundary of language.║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause a Type0/Type1 universe paradox), but we deliberately keep this "Easter egg" — when you try to compile it, you receive a Zen-like message from the language's founder. This is not only a technical boundary, but also a tribute to YaoXiang's type philosophy.

---

## Appendix

### Syntax BNF

```bnf
program ::= statement*

statement ::= declaration | expression

# Unified declaration: name: Type = expression
declaration ::= identifier ':' type_expr '=' expression

# Type expression
type_expr ::= identifier
       | identifier '(' type_expr (',' type_expr)* ')'      # Type application
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # Function type
       | '{' type_field* '}'                       # Record/Interface type
       | 'Type'                                    # Meta type

type_field ::= identifier ':' type_expr
             | identifier                           # Interface constraint

# Generic parameters: as part of function type, e.g., (T: Type, R: Type) -> (...)
# No separate BNF rule needed — : Type parameters are ordinary function parameters

# Expression
expression ::= literal
              | identifier
              | identifier '(' expression (',' expression)* ')'  # Function call / Constructor call
              | '(' expression (',' expression)* ')'              # Tuple
              | expression '.' identifier '(' arguments? ')'    # Method call
              | lambda
              | '{' field ':' expression (',' field ':' expression)* '}'

arguments ::= expression (',' expression)*

lambda ::= '(' parameter_list? ')' '=>' block

block ::= expression | '{' expression* '}'
```

### Glossary

| Term | Definition | Description |
|------|------------|-------------|
| Declaration | Assignment statement in the form `name: type = value` | |
| Record type | A `{ ... }` type containing named fields | |
| Interface | A record type with all function fields | |
| Generic type | A type defined as `Name: (T: Type) -> Type = { ... }` that accepts type parameters | |
| Type method | A method in the form `Type.method`, associated with a specific type | |
| Generic function | A function using `(T: Type)` syntax, with type parameters as the first parameter group | |
| Meta type | `Type`, the only type-level keyword in the language | |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review│  ← Open for community discussion and feedback
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│(final design)│    │ (kept in   │
│             │    │ original   │
│             │    │ position)  │
└─────────────┘    └─────────────┘
```