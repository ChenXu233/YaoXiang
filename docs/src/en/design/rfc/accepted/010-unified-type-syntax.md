```yaml
---
title: "RFC-010: Unified Type Syntax"
---

# RFC-010: Unified Type Syntax - The name: type = value Model

> **Status**: Accepted
>
> **Author**: Chen Xu (晨煦)
>
> **Created**: 2025-01-20
>
> **Last Updated**: 2026-06-05 (Updated return rules and `{}` semantics)

## Abstract

This RFC proposes a minimalist, unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression.
**No `fn`, no `struct`, no `trait`, no `impl`, no lowercase `type` keyword (but `Type` exists as a meta-type keyword)**.

> **Core Design**: `Type` itself is a generic type. `(T: Type) -> Type` represents "a type that accepts a type parameter T".

| Concept       | Code                                       |
|---------------|--------------------------------------------|
| Variable      | `x: Int = 42`                              |
| Function      | `add: (a: Int, b: Int) -> Int = a + b`     |
| Record type   | `Point: Type = { x: Float, y: Float }`     |
| Interface     | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic type  | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| Generic type  | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| Method        | `Point.draw: (self: Point, s: Surface) -> Void = ...` |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` is the only meta-type keyword in the language**.
It is used to annotate the type level; the compiler automatically handles the distinction between Type0, Type1, Type2..., which is transparent to users.

```yaoxiang
// Core syntax: Unified + Differentiated

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
2. **Clean and elegant**: Symmetrical aesthetics of `name: type = value`
3. **No new keywords**: Reuse existing syntactic elements
4. **Theoretically elegant**: Types themselves are values of Type type
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 **naturally aligns** with RFC-011's generics system design. Generic parameters seamlessly integrate into the unified model:

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
- RFC-011 Phase 1 (basic generics) is a **hard dependency** of RFC-010
- Without basic generics, RFC-010's generic examples cannot compile
- Recommendation: RFC-011 Phase 1 and RFC-010 be implemented together

## Proposal

### Core Principle: Type Constructor vs Function/Variable

**This is a key design choice that determines the disambiguation rules for syntax:**

| Syntax | Meaning | Rule |
|--------|---------|------|
| **`x: Type = ...`** | Type constructor | `: Type` explicit declaration → forced as type |
| **`f = ...`** | Function or variable | No `: Type` → HM actively infers as function/variable |

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
├── Generic type (multi-parameter)
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # Must return: Type
│
├── Method
│   └── Point.draw: (self: Point, surface: Surface) -> Void = ...
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Universe Levels (Compiler Internally)

**Internally, the compiler maintains a universe level `level: selfpointnum`** (stored as a string, theoretically infinitely extendable).

| Level | Description |
|-------|-------------|
| `Type0` | Everyday types (`Int`, `Float`, `Point`) |
| `Type1` | Type constructors (`List`, `Maybe`) |
| `Type2+` | Higher-order constructors |

**Users never see these numbers**, only `: Type`.

> **Curry-Howard Isomorphism**: The existence of universe levels is not an engineering implementation detail, but a necessary condition for logical consistency. The Curry-Howard isomorphism equates types with propositions. If we allowed `Type: Type` (i.e., "the type of types is also a type"), it would create a Russell's paradox like "this sentence is false" — manifesting in the type system as Girard's paradox. YaoXiang's `Type0 / Type1 / Type2…` stratification (i.e., cumulative universes in Martin-Löf type theory) ensures each type belongs to exactly one level, with `Typeₙ : Typeₙ₊₁` forming an ever-ascending chain that never closes, fundamentally avoiding paradox. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

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
// Single expression form (returns value directly, no return needed)
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// Code block form (must use return to return value)
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Multi-line code block
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }
}

// Void function (no return needed in code block)
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### Return Rules

The return value depends on the form on the right side of `=`:

| Syntax | Return value |
|--------|--------------|
| `= expr` (no curly braces) | Returns `expr` directly |
| `= { ... }` (with curly braces) | Must use `return`, otherwise returns `Void` |

```yaoxiang
# Single expression: returns value directly, no return needed
add: (a: Int, b: Int) -> Int = a + b

# Code block: must use return to return value
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

# Void function: no return needed
print: (msg: String) -> Void = {
    console.write(msg)
}
```

> **Design rationale**: `{ ... }` is a dependency-driven computation unit (see below). Its return semantics differ from single expressions. Curly braces introduce a multi-statement context, so explicit `return` is needed to disambiguate whether "the last expression is the return value."

#### `{}` Semantics: Dependency-Driven Computation Unit

`{ ... }` in YaoXiang is not just a code block — it is a **dependency-driven computation unit**. This semantics remains consistent across function bodies, variable initialization, and `spawn`:

**Core rules**:
- Assignment statements inside `{}` are automatically ordered by dependency, not by write order
- Statements with complete dependencies execute immediately; those with missing dependencies block and wait
- Use `return` to explicitly return a value (see return rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler auto-orders
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependencies → can execute first
    return b       # returns 11
}
```

> **Difference from single expressions**: `= expr` (no curly braces) is a simple binding that returns a value directly; `= { ... }` (with curly braces) introduces a dependency-driven computation context, allowing multiple statements and explicit `return`.

#### `spawn` Block

`spawn { ... }` is YaoXiang's only parallel primitive. It leverages `{}`'s dependency-driven semantics for automatic parallelization:

- Direct child assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks with complete dependencies execute concurrently immediately
- The caller blocks until all subtasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # depends on a, b → waits for both to complete
    return c
}
// Caller blocks here until all tasks in the spawn block complete
```

> **Full definition**: For `spawn`'s complete semantics, task creation rules, and blocking model, see `008-runtime-concurrency-model.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound methods, and interface implementations:

##### Basic Types

**Record type**: List of fields, field types can be any type expression.

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

**Method 1: Bind external function directly in type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # Bind to position 0, curried → method: (b: Point) -> Float
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

**Interface name written in type body, compiler automatically checks its implementation**

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
    Drawable,          # Implements Drawable interface
    Serializable       # Implements Serializable interface
}
```

##### Interface Definition

**Interface = Record type with all function fields**

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

**Type method**: Associated with a specific type (using Type.method syntax)

```yaoxiang
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}
```

##### Method Binding (External)

Regular methods can be bound to types using `[position]` syntax (detailed syntax in RFC-004).

**Manual binding**:

```yaoxiang
// Explicit binding
Point.distance = distance[0]

// Specify binding position
Point.transform = transform[1]  # this bound to position 1
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

// Using intersection type
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

IntList.push(Int)(self, item)  // Usage example

// Generic method (RFC-023 syntax: type parameters auto-inferred at call site)
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

Generic types and generic functions use unified `()` syntax for invocation. `[]` is not used in any generic context.

**Core rules**:

1. **`()` does all application**: Type application, function call, value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function call — type flows automatically from arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on left, value on right**: `name: type = value` — Type parameters declared on left, right side is always a concrete value. For empty container `List()`, `T` must come from left-side type annotation.

3. **Type information written once** — at parameter declaration, compiler carries it along:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int written once on left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String auto from numbers and f's types
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       // Inferred as List(Int)
y = List("a", "b")      // Inferred as List(String)
z = List()              // ❌ Compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int from left-side annotation
```

5. **Type alias**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`, `List[Int](1,2,3)` → `List(1,2,3)`.
> The old `[]` generic syntax has been completely removed. `[]` is used only for array/list literals and index access.

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

// Methods for Point
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

// Regular method (pub, auto-bound to Point.distance)
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// Methods for Rect
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

Point.distance = distance[0]  // Bind to position 0, curried → method: (p2: Point) -> Float
Point.transform = transform[1]  // Bind to position 1, curried → method: (dx: Float, dy: Float) -> Point
Rect.transform = transform[1]  // Bind to position 1, curried → method: (dx: Float, dy: Float) -> Rect

// ...and so on, bind other methods...

// ======== 5. Usage ========

// Create instances
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// Method call (syntactic sugar)
p.draw(screen)
r.draw(screen)

// Regular method call (direct call)
d: Float = distance(p, Point(0.0, 0.0))

// Chained calls
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// Interface assignment
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// Generic function (RFC-023 syntax: type parameters omitted at call site, auto-inferred)
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
    // For each field in the interface (function field)
    for (field_name, iface_field) in &iface.fields {
        // Check if the type has a method with the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check if method signature is compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Comparison: should match after removing self parameter
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

### Direct Interface Assignment and Compile-Time Optimization

Interface types support direct assignment, and the compiler automatically selects the optimal calling strategy based on the right-hand side value type:

```yaoxiang
// Direct assignment of concrete type → concrete type determinable at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → concrete type not determinable at compile time, use vtable
d: Drawable = get_shape()
d.draw(screen)  // Method lookup via vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Method lookup via vtable
}
```

**Compile-time optimization strategy**:

| Scenario | Inference Result | Calling Method |
|----------|------------------|----------------|
| `d: Drawable = Circle(1)` | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()` | Unknown | vtable |
| `shapes: List(Drawable) = [...]` | Heterogeneous | vtable |

**Rules**:
1. When the right-hand side is a concrete type constructor and determinable at compile time, generate direct call IR
2. When the right-hand side type cannot be determined at compile time, fall back to vtable mechanism
3. vtable fallback guarantees correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as it has the same methods, it can be assigned to interface type
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

| Before | After |
|--------|-------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Self, err: (E) -> Self }` |
| Requires `impl` keyword | No keyword needed, interface name written after type body |

## Syntax Design Note: Named Functions Are Syntactic Sugar for Lambda

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

**Key point**: When the signature fully declares parameter types, parameter names in the Lambda header become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: Parameters in signature have scope that covers function body, inner scope has higher priority.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x overrides outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be in any of the following positions, **at least one annotation is required**:

| Annotation Position | Form | Description |
|---------------------|------|-------------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda header only | `double = (x: Int) => x * 2` | ✅ Valid |
| Both sides | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Examples

```yaoxiang
// ✅ Recommended: signature complete, Lambda header omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Valid: type annotation in Lambda header
double = (x: Int) => x * 2

// ✅ Valid: both sides annotated
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature | Advantage |
|---------|-----------|
| **Concise** | No need to repeat parameter names when signature is complete |
| **Flexible** | Lambda form preserved, use whichever you prefer |
| **Consistent** | Unified pattern with variable declaration `x: Int = 42` |
| **Intuitive** | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Tradeoffs

### Advantages

| Advantage | Description |
|-----------|-------------|
| Extreme unification | One syntax rule covers all cases |
| Theoretically elegant | Perfectly symmetrical `name: type = value` |
| No new keywords | Reuse existing syntactic elements |
| Easy to implement | Compiler only needs to handle one declaration form |
| Easy to learn | Remember one pattern to write all code |
| Easy to extend | New features can naturally integrate into this model |

### Disadvantages

| Disadvantage | Description |
|--------------|-------------|
| Naming conventions | Methods need to follow `Type.method` naming |
| Verbosity | Complete syntax is longer than simplified syntax, but can be inferred |
| Learning curve | Need to understand the unified model |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compile error example:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Type can be omitted, compiler infers it
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE auto-suggests missing methods
```

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Parsing complexity | Unified syntax may increase parsing complexity | Use recursive descent parser |
| Performance overhead | vtable lookup may have extra overhead | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: Origin of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Attempt to define the type of types...
Type: Type = Type
```

**Warning**: This is the **ineffable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One produces two, two produces three, three produces all. ║
║   In the I Ching, there is the Great Ultimate, generating    ║
║   yin and yang.                                              ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language. ║
║   The compiler falls silent here; philosophy dwells.         ║
║                                                              ║
║   Thank you for reaching the philosophical boundary          ║
║   of this language.                                          ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause a Type0/Type1 universe paradox), but we intentionally keep this "easter egg" — when you try to compile it, you receive a Zen message from the language founder. This is not just a technical boundary, but a tribute from YaoXiang to the philosophy of types.

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
       | '{' type_field* '}'                       # Record/interface type
       | 'Type'                                    # Meta type

type_field ::= identifier ':' type_expr
             | identifier                           # Interface constraint

# Generic parameters: part of function type, e.g., (T: Type, R: Type) -> (...)
# No separate BNF rule needed — : Type parameters are ordinary function parameters

# Expression
expression ::= literal
              | identifier
              | identifier '(' expression (',' expression)* ')'  # Function call / constructor call
              | '(' expression (',' expression)* ')'              # Tuple
              | expression '.' identifier '(' arguments? ')'    # Method call
              | lambda
              | '{' field ':' expression (',' field ':' expression)* '}'

arguments ::= expression (',' expression)*

lambda ::= '(' parameter_list? ')' '=>' block

block ::= expression | '{' expression* '}'
```

### Glossary

| Term | Definition |
|------|------------|
| Declaration | Assignment statement of form `name: type = value` |
| Record type | A `{ ... }` type containing named fields |
| Interface | A record type with all function fields |
| Generic type | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters |
| Type method | A method in `Type.method` form, associated with a specific type |
| Generic function | A function using `(T: Type)` syntax, with type parameters as the first parameter group |
| Meta type | `Type`, the only type-level marker in the language |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← Open for community discussion and feedback
│  Review     │
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
│ (official   │    │ (preserved  │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```