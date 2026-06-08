```markdown
---
title: "RFC-010: Unified Type Syntax - name: type = value Model"
status: "Accepted"
author: "Morning Dawn"
created: "2025-01-20"
updated: "2026-06-05 (Updated return rules and {} semantics)"
---

# RFC-010: Unified Type Syntax - name: type = value Model

## Summary

This RFC proposes an extremely minimalist unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression.
**No `fn`, no `struct`, no `trait`, no `impl`, no lowercase `type` keyword (but there is `Type` as a meta type keyword)**.

> **Core Design**: `Type` itself is a generic type. `(T: Type) -> Type` represents "a type that accepts a type parameter T".

| Concept | Code Syntax |
|---------|-------------|
| Variable | `x: Int = 42` |
| Function | `add: (a: Int, b: Int) -> Int = a + b` |
| Record type | `Point: Type = { x: Float, y: Float }` |
| Interface | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic type | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| Generic type | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| Method | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` is the only meta type keyword in the language**.

> **Namespace vs Method Binding**: The `Type.name` prefix indicates **namespace ownership** and nothing more.
> It does not trigger any implicit binding. For `p.draw(screen)` dot call syntax to work,
> explicit binding is required: `Point.draw = draw[0]`.
> See the "Namespace and Method Binding" section below for details.
It is used to annotate type hierarchies; the compiler automatically handles the distinction between Type0, Type1, Type2..., which is transparent to users.

```yaoxiang
// Core syntax: unified + differentiated

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

// Interface (essentially a record type where all fields are functions)
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

// Generic type ((T: Type) -> Type = generic type accepting type parameters)
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
s: Drawable = p           // Structural subtype: Point implements Drawable
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

There is a lack of unity between these concepts, leading to syntactic fragmentation and a high learning curve.

### Design Goals

1. **Extreme unification**: One syntactic rule covers all cases
2. **Concise elegance**: Symmetric aesthetics of `name: type = value`
3. **No new keywords**: Reuse existing syntactic elements
4. **Theoretically elegant**: Types themselves are values of Type type
5. **Generic-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 **naturally aligns** with the generics system design of RFC-011. Generic parameters integrate seamlessly into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type position in signature can be omitted, inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraints (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:
- RFC-011 Phase 1 (Basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 together with RFC-010

## Proposal

### Core Principle: Type Constructor vs Function/Variable

**This is a key design choice that determines the syntax disambiguation rules:**

| Syntax | Meaning | Rule |
|--------|---------|------|
| **`x: Type = ...`** | Type constructor | `: Type` explicitly declared → forced to be a type |
| **`f = ...`** | Function or variable | No `: Type` → HM actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:
- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executes statements, returns Void)

**Disambiguation rules**:
- **Has** `: Type` → force parse as type constructor, `{ ... }` is a type literal
- **No** `: Type` → HM actively parses `{ ... }` as code block, inferring as function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main = { println("Hello") }

# ❌ Error: no : Type, compiler cannot parse { ... } as type
Point = { x: Float, y: Float }  // HM infers as function, not type!
```

---

**Unified Model: identifier : type = expression**

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
├── Namespace function
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # Dot call syntax only available after explicit binding
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Universe Hierarchy (Compiler Internals)

**The compiler internally maintains a universe hierarchy `level: selfpointnum`** (stored as string, theoretically extendable indefinitely).

| Level | Description |
|-------|-------------|
| `Type0` | Everyday types (`Int`, `Float`, `Point`) |
| `Type1` | Type constructors (`List`, `Maybe`) |
| `Type2+` | Higher-order constructors |

**Users never see these numbers**, only `: Type`.

> **Curry-Howard Isomorphism**: The existence of universe levels is not an engineering implementation detail but a necessary condition for logical consistency. The Curry-Howard isomorphism equates types with propositions. If `Type: Type` were allowed (i.e., "the type of types is also a type"), it would create a Russell's paradox-like situation—"this sentence is false"—which manifests in the type system as Girard's paradox. YaoXiang's `Type0 / Type1 / Type2…` stratification (i.e., cumulative universes in Martin-Löf type theory) ensures each type belongs to exactly one level, with `Typeₙ : Typeₙ₊₁` forming an ever-ascending chain that never closes, fundamentally avoiding paradox. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

### Syntax Definition

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

| Syntax | Return Value |
|--------|--------------|
| `= expr` (no braces) | Returns `expr` directly |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` |

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

> **Design Rationale**: `{ ... }` is a dependency-driven computation unit (see below). Its return semantics differ from single expressions. Braces introduce a multi-statement context, so explicit `return` is needed to disambiguate whether "the last expression is the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

`{ ... }` in YaoXiang is not just a code block—it is a **dependency-driven computation unit**. This semantics remains consistent across function bodies, variable initialization, and `spawn`:

**Core Rules**:
- Assignment statements inside `{}` are automatically ordered by dependency, not by written order
- Tasks with all dependencies ready execute immediately; missing dependencies cause blocking
- Use `return` to explicitly return a value (see return rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler automatically orders
result: Int = {
    b = a + 1      # Depends on a → automatically placed after a
    a = 10         # No dependencies → can execute first
    return b       # Returns 11
}
```

> **Difference from Single Expression**: `= expr` (no braces) is a simple binding that returns the value directly; `= { ... }` (with braces) introduces a dependency-driven computation context, allowing multiple statements and explicit `return`.

#### `spawn` Block

`spawn { ... }` is YaoXiang's only parallel primitive. It leverages `{}`'s dependency-driven semantics for automatic parallelization:

- Direct child assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks with all dependencies ready execute concurrently
- The caller blocks waiting for all subtasks to complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # Depends on a, b → waits for both, then executes
    return c
}
// Caller blocks here until all tasks in spawn block complete
```

> **Full Definition**: For `spawn`'s complete semantics, task creation rules, and blocking model, see `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used for defining opaque types and operating on raw pointers. It leverages `{}`'s return semantics to expose type definitions to the outer scope:

**Core Rules**:
- `unsafe {}` can define types and operate on raw pointers
- Use `return` to expose type definitions to the outer scope
- The returned type is usable outside `unsafe {}`
- Accessing a type's fields requires unsafe permission

```yaoxiang
# Define opaque type inside unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # Raw pointer
    }
    return SqliteDb
}

# SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: handle field requires unsafe permission
handle = db.handle

# ✅ Through method call
db.close()
```

> **Full Definition**: For `unsafe`'s complete semantics, FFI type definitions, and method binding, see `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound methods, and interface implementations:

##### Basic Types

**Record type**: Field list, where field types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: Fields can have default values, optional at construction time.

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

**Fields without default values**: Must be provided at construction time.

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

##### Binding Methods

**Method 1: Directly bind external function inside type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # Bind to position 0, curried: method: (b: Point) -> Float
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

**Interface name written inside type body; compiler automatically checks its implementation**

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
    Serializable      # Implements Serializable interface
}
```

##### Interface Definition

**Interface = record type where all fields are function types**

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

##### Namespace Function Definition

**`Type.name` prefix indicates namespace ownership** and nothing more. It does not trigger any implicit binding.

```yaoxiang
// Namespace function: ordinary function in Point namespace
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

// Call: just ordinary function call
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing `p`, `this`, or `x` has exactly the same effect.
> The compiler doesn't look at parameter names, it looks at types.

##### Method Binding (The Only Way)

For `p.draw(screen)` dot method call syntax to work, **explicit binding is required**.
The `[position]` syntax is the only mechanism for binding a function as a "method" (full syntax in RFC-004).

```yaoxiang
// Define function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does p.draw(screen) syntax work
Point.draw = draw[0]   # Position 0's parameter (&Point) is filled by caller

// Usage
p.draw(screen)          // Syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // Both calling styles are equivalent

// No [0] = no binding. Point.draw is just a regular function alias, no . syntax
Point.draw = draw       # Not bound: can only call Point.draw(p, screen)
```

**Default behavior**: Not writing `[n]` = no parameter bound. Users must explicitly decide which parameters are filled by the caller.

**Multiple position binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method to ordinary function):

```yaoxiang
// Extract function from binding
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
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

// Generic methods (RFC-023 syntax: type parameters inferred automatically at call site)
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

Generic types and generic function calls uniformly use `()` syntax. `[]` is not used in any generic context.

**Core Rules**:

1. **`()` does all application**: Type application, function call, value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T flows from the left side
empty: List(Int) = List()

# Generic function call — types flow automatically from arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on left, value on right**: `name: type = value`—Type parameters are declared on the left, right side is always a concrete value. For empty container `List()`, `T` must be obtained from the left-side type annotation.

3. **Type information written only once**—at parameter declaration, compiler carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically from numbers and f's types
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       // Inferred as List(Int)
y = List("a", "b")      // Inferred as List(String)
z = List()              // ❌ Compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int comes from left-side annotation
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
// Interface = record type where all fields are function types
// Interface doesn't need self parameter — interface only defines "function signature after removing caller position"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // Returns interface type, concrete implementation returns own type
    scale: (factor: Float) -> Transformable
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

// ======== 3. Method Implementation (ordinary function + explicit binding) ========

// Define function (self is just conventional name, not a keyword)
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

bounding_box: (p: &Point) -> Rect = {
    return Rect(p.x - 1, p.y - 1, 2, 2)
}

serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

translate: (p: &Point, dx: Float, dy: Float) -> Point = {
    return Point(p.x + dx, p.y + dy)
}

scale: (p: &Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

distance: (p1: &Point, p2: &Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// Explicit binding — dot call syntax only after binding
Point.draw = draw[0]
Point.bounding_box = bounding_box[0]
Point.serialize = serialize[0]
Point.translate = translate[0]
Point.scale = scale[0]
Point.distance = distance[0]

// Rect's methods are similar
draw: (r: &Rect, surface: Surface) -> Void = {
    surface.draw_rect(r.x, r.y, r.width, r.height)
}
Rect.draw = draw[0]

bounding_box: (r: &Rect) -> Rect = r
Rect.bounding_box = bounding_box[0]

serialize: (r: &Rect) -> String = {
    return "Rect(${r.x}, ${r.y}, ${r.width}, ${r.height})"
}
Rect.serialize = serialize[0]

translate: (r: &Rect, dx: Float, dy: Float) -> Rect = {
    return Rect(r.x + dx, r.y + dy, r.width, r.height)
}
Rect.translate = translate[0]

scale: (r: &Rect, factor: Float) -> Rect = {
    return Rect(r.x * factor, r.y * factor, r.width * factor, r.height * factor)
}
Rect.scale = scale[0]

// ======== 4. Usage ========

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

// Generic functions (RFC-023 syntax: type parameters omitted at call site, inferred automatically)
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
    // For each field of the interface (function field)
    for (field_name, iface_field) in &iface.fields {
        // Check if type has method with same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check if method signature is compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Comparison: after removing self parameter, they should match
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

Interface types support direct assignment, and the compiler automatically selects the optimal calling strategy based on the right-hand side type:

```yaoxiang
// Direct assignment of concrete type → concrete type determined at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: directly call circle_draw(screen), no vtable

// Function return value → concrete type cannot be determined at compile time, use vtable
d: Drawable = get_shape()
d.draw(screen)  // Method lookup through vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Method lookup through vtable
}
```

**Compile-time optimization strategy**:

| Scenario | Inference Result | Calling Method |
|----------|------------------|----------------|
| `d: Drawable = Circle(1)` | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()` | Unknown | vtable |
| `shapes: List(Drawable) = [...]` | Heterogeneous | vtable |

**Rules**:
1. When the right-hand side is a concrete type constructor and can be determined at compile time, generate direct call IR
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
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| Needs `impl` keyword | No keyword needed, interface name written after type body |

## Syntax Design Note: Named Functions Are Syntactic Sugar for Lambda

### Core Understanding

**Named functions and lambda expressions are the same thing!** The only difference is: a named function gives the lambda a name.

```yaoxiang
// These two are essentially identical
add: (a: Int, b: Int) -> Int = a + b           // Named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda form (completely equivalent)
```

### Syntactic Sugar Model

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially is
name: (Params) -> ReturnType = (params) => body
```

**Key point**: When the signature fully declares parameter types, the parameter names in the lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters shadow outer variables**: Parameters in the signature have scope that covers the function body; inner scope has higher priority.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x shadows outer x, result is 20
```

### Flexible Annotation Placement

Type annotations can be in any of the following positions, **at least one annotation is required**:

| Annotation Position | Form | Description |
|--------------------|------|-------------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda head only | `double = (x: Int) => x * 2` | ✅ Valid |
| Both sides | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Examples

```yaoxiang
// ✅ Recommended: signature complete, lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Valid: type annotated in lambda head
double = (x: Int) => x * 2

// ✅ Valid: annotated on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature | Advantage |
|---------|-----------|
| **Concise** | No need to repeat parameter names when signature is complete |
| **Flexible** | Lambda form preserved, use whichever you prefer |
| **Consistent** | Maintains unified pattern with variable declaration `x: Int = 42` |
| **Intuitive** | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage | Description |
|-----------|-------------|
| Extreme unification | One syntactic rule covers all cases |
| Theoretically elegant | Perfectly symmetric `name: type = value` |
| No new keywords | Reuses existing syntactic elements |
| Easy to implement | Compiler only needs to handle one declaration form |
| Easy to learn | Remember one pattern to write all code |
| Easy to extend | New features can naturally integrate into this model |

### Disadvantages

| Disadvantage | Description |
|--------------|-------------|
| Naming convention | Methods need to follow `Type.method` naming |
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
// Type can be omitted, compiler will infer
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE automatically hints missing methods
```

### Risks

| Risk | Impact | Mitigations |
|------|--------|-------------|
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
║   One produces two, two produces three,                      ║
║   three produces all things.                                 ║
║   In the Yi Jing, there is the Great Ultimate,               ║
║   which gives rise to the two principles.                    ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the origin of YaoXiang, the boundary of language.  ║
║   The compiler falls silent here, philosophy dwells.          ║
║                                                              ║
║   Thank you for reaching the philosophical boundary          ║
║   of the language.                                           ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause a Type0/Type1 universe paradox), but we deliberately keep this "easter egg"—when you try to compile it, you receive a Zen-like message from the language creator. This is not just a technical boundary, but also a tribute to YaoXiang's philosophy of types.

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

| Term | Definition |
|------|------------|
| Declaration | Assignment statement of form `name: type = value` |
| Record type | `{ ... }` type containing named fields |
| Interface | Record type where all fields are function types |
| Generic type | Type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters |
| Namespace function | Function of form `Type.name`, belonging to Type namespace. Implies no binding |
| Method binding | `Type.name = func[n]`, binding position n of func as caller, enabling `obj.name(args)` syntax |
| Generic function | Function using `(T: Type)` syntax, with type parameters as first parameter group |
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
│ (official  │    │ (preserved  │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```