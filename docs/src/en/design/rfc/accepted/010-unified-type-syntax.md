---
title: "RFC-010: Unified Type Syntax - The name: type = value Model"
status: "Accepted"
author: "Chenxu"
created: "2025-01-20"
updated: "2026-06-05 (Updated return rules and {} semantics)"
issue: "#127"
---
# RFC-010: Unified Type Syntax - The name: type = value Model


## Summary

This RFC proposes a minimalist, unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

where `type` can be any type expression and `expression` can be any value expression.
**There is no `fn`, no `struct`, no `trait`, no `impl`, and no lowercase `type` keyword (but `Type` is used as a meta type keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that accepts type parameter T".

| Concept           | Code Form                                                                |
|-------------------|--------------------------------------------------------------------------|
| Variable          | `x: Int = 42`                                                           |
| Function          | `add: (a: Int, b: Int) -> Int = a + b`                                  |
| Record type       | `Point: Type = { x: Float, y: Float }`                                  |
| Interface         | `Drawable: Type = { draw: (Surface) -> Void }`                          |
| Generic type      | `List: (T: Type) -> Type = { data: Array(T), length: Int }`             |
| Generic type      | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`|
| Method            | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic function  | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` is the only meta type keyword in the language**.

> **Namespace vs Method binding**: The `Type.name` prefix indicates **namespace membership**, nothing more.
> It triggers no implicit binding. For the `.` call syntax like `p.draw(screen)` to work,
> an explicit binding is required: `Point.draw = draw[0]`.
> See the "Namespace and Method Binding" section below for details.

It is used to mark the type hierarchy, and the compiler automatically handles the distinction of Type0, Type1, Type2..., which is transparent to the user.

```yaoxiang
// Core syntax: unified + distinguishable

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

// Generic type ((T: Type) -> Type = a generic type that accepts type parameters)
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
p.draw(screen)           // Syntax sugar → Point.draw(p, screen)
s: Drawable = p           // Structural subtyping: Point implements Drawable
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## Motivation

### Why is this feature needed?

The current type system has several disconnected concepts:
- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack unity, leading to fragmented syntax and a high learning cost.

### Design Goals

1. **Ultimate unity**: A single syntax rule covers all cases
2. **Concise and elegant**: Symmetric aesthetics of `name: type = value`
3. **No new keywords**: Reuses existing syntax elements
4. **Theoretically elegant**: Types themselves are values of type Type
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 fits **naturally** with the generics system design of RFC-011, where generic parameters can integrate seamlessly into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type position in signature can be omitted, automatically inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraint (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:
- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 and RFC-010 synchronously

## Proposal

### Core Principle: Type Constructors vs Functions/Variables

**This is a key design choice that determines the disambiguation rules for syntax:**

| Form | Meaning | Rule |
|------|---------|------|
| **`x: Type = ...`** | Type constructor | `: Type` explicit declaration → forced to be a type |
| **`f = ...`** | Function or variable | No `: Type` → HM actively infers as a function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:
- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executed statement, returns void)

**Disambiguation rules**:
- **Has** `: Type` → forced to parse as a type constructor, `{ ... }` is a type literal
- **No** `: Type` → HM actively parses `{ ... }` as a code block, infers as function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main = { println("Hello") }

# ❌ Error: no : Type, compiler cannot parse { ... } as a type
Point = { x: Float, y: Float }  // HM infers as a function, not a type!
```

---

**Unified model: identifier : type = expression**

```
├── Variables
│   └── x: Int = 42
│
├── Functions
│   └── add: (a: Int, b: Int) -> Int = a + b  # No : Type, HM infers as function
│
├── Record types
│   └── Point: Type = { x: Float, y: Float }  # Must return: Type
│
├── Interfaces
│   └── Drawable: Type = { draw: (Surface) -> Void }  # Must return: Type
│
├── Generic types
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # Must return: Type
│
├── Generic types (multiple parameters)
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # Must return: Type
│
├── Namespace functions
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # Only after explicit binding is dot-call syntax available
│
└── Generic functions
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Meta Type Hierarchy (Compiler Internal)

**The compiler internally** maintains a universe hierarchy `level: selfpointnum` (stored as a string, theoretically infinitely extensible).

| Level | Description |
|-------|-------------|
| `Type0` | Everyday types (`Int`, `Float`, `Point`) |
| `Type1` | Type constructors (`List`, `Maybe`) |
| `Type2+` | Higher-order constructors |

**Users never see these numbers**, only `: Type`.

### Curry-Howard Isomorphism: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not a random choice—it is a direct mapping of the Curry-Howard correspondence. This correspondence reveals a profound fact: **the type system and the logic system are two sides of the same coin**.

| Logic (Proposition) | Type System (YaoXiang) | Example |
|---|---|---|
| Proposition P | Type T | `Int`, `Bool` |
| Proof that P is true | A value of type T | `42: Int`, `true: Bool` |
| P → Q (implication) | Function type `(P) -> Q` | `(x: Int) -> Bool` |
| P ∧ Q (conjunction) | Record type `{ p: P, q: Q }` | `{ x: Int, y: Bool }` |
| ∀x.P(x) (universal quantification) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...` |
| P ⊕ Q (disjunction) | Enum / tagged union | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "there exists a proof of type Int, named x, with value 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "there exists an implication proof: given proofs a and b of type Int, we can construct a proof of type Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires simultaneously providing Float proof x and Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If the type system allows constructing a value of type `T` without any legal runtime representation, it's like allowing a false proposition to be proven in logic—the system collapses. Curry-Howard tells us: **a type-safe language is naturally a logically consistent system**.

2. **Universe hierarchy is a necessary condition**: As detailed below, if `Type: Type` were allowed (i.e., "the type of types is also a type"), it would produce Russell's paradox (manifested as Girard's paradox in type theory). YaoXiang's `Type₀ : Type₁ : Type₂ : ...` stratification ensures each type belongs to a specific level, forming an unending ascending chain that fundamentally avoids paradox. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

3. **The theoretical foundation of unified syntax**: The reason `name: type = value` can use one syntax to cover variables, functions, types, interfaces, and generics is precisely because they are all the same thing under Curry-Howard—**providing proofs for propositions**. Variables are evidence for propositions, functions are evidence for implications, records are evidence for conjunctions, generics are evidence for universal quantification. Unified syntax is not a coincidental human design, but a natural consequence of the Curry-Howard isomorphism.

> **Further reading**: Wadler, P. (2015). *"Propositions as Types."* Communications of the ACM, 58(12), 75–84. This article explains the history and significance of the Curry-Howard correspondence in accessible language.

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
// Single expression form (returns the value directly, no return needed)
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// Code block form (must use return to return a value)
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

| Form | Return Value |
|------|--------------|
| `= expr` (no curly braces) | Returns `expr` directly |
| `= { ... }` (with curly braces) | Must use `return`, otherwise returns `Void` |

```yaoxiang
# Single expression: returns the value directly, no return needed
add: (a: Int, b: Int) -> Int = a + b

# Code block: must use return to return a value
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

> **Design rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its return semantics differ from a single expression. Curly braces introduce a multi-statement context, so an explicit `return` is needed to eliminate the ambiguity of "is the last expression the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

`{ ... }` in YaoXiang is not merely a code block—it is a **dependency-driven computation unit**. This semantics remains consistent across function bodies, variable initialization, and `spawn`:

**Core rules**:
- Assignment statements within `{}` are automatically sorted by dependency, not by written order
- When dependencies are ready, execution proceeds immediately; when missing, it blocks and waits
- Use `return` to explicitly return a value (see return rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler automatically sorts
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependency → can execute first
    return b       # returns 11
}
```

> **Difference from single expression**: `= expr` (no curly braces) is a simple binding that returns the value directly; `= { ... }` (with curly braces) introduces a dependency-driven computation context, allowing multiple statements and explicit `return`.

#### `spawn` Block

`spawn { ... }` is the only parallel primitive in YaoXiang. It leverages the dependency-driven semantics of `{}` to achieve automatic parallelization:

- Direct child assignments within `spawn { ... }` automatically create parallel tasks
- Tasks with ready dependencies execute concurrently immediately
- The caller blocks until all child tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # Depends on a, b → waits for both to complete
    return c
}
// Caller blocks here until all tasks within the spawn block complete
```

> **Detailed definition**: For the complete semantics of `spawn`, task creation rules, and blocking model, see `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and manipulate raw pointers. It leverages the `{}` return semantics to return type definitions to the parent scope:

**Core rules**:
- Types can be defined and raw pointers manipulated within `unsafe {}`
- Use `return` to return type definitions to the parent scope
- Returned types are available outside `unsafe {}`
- Field access of types requires unsafe permission

```yaoxiang
# Define opaque type in unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # Raw pointer
    }
    return SqliteDb
}

# SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compilation error: handle field requires unsafe permission
handle = db.handle

# ✅ Through method call
db.close()
```

> **Detailed definition**: For the complete semantics of `unsafe`, FFI type definitions, and method binding, see `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound methods, and interface implementations:

##### Basic Types

**Record type**: a list of fields whose field types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: fields can have default values and are optional during construction.

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

**Fields without default values**: must be provided during construction.

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

##### Method Binding

**Method 1: Bind external functions directly within the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // Bind to position 0, after currying method: (b: Point) -> Float
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

**Interface names are written inside the type body, and the compiler automatically checks the implementation**

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

**Interface = a record type with all function fields**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Empty type / empty interface
EmptyType: Type = {}
Empty: Type = {}
```

##### Namespace Function Definition

**The `Type.name` prefix indicates namespace membership**, nothing more. It triggers no implicit binding.

```yaoxiang
// Namespace function: a regular function under the Point namespace
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

// Call: just a regular function call
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword, merely a conventional parameter name. Writing `p`, `this`, or `x` works exactly the same.
> The compiler does not look at parameter names, but at types.

##### Method Binding (The Only Way)

For the `.` method call syntax like `p.draw(screen)` to work, **an explicit binding is required**.
The `[position]` syntax is the only mechanism to bind a function as a "method" (see RFC-004 for detailed syntax).

```yaoxiang
// Define function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this is the p.draw(screen) syntax available
Point.draw = draw[0]   // The parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // Syntax sugar → draw(&p, screen)
Point.draw(p, screen)   // Both call forms are equivalent

// Without [0] = not bound. Point.draw is a regular function alias, no . syntax
Point.draw = draw       // Not bound: only Point.draw(p, screen)
```

**Default behavior**: omitting `[n]` = binding no parameters. Users must explicitly decide which parameters are filled by the caller.

**Multi-position binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method to regular function):

```yaoxiang
// Extract the function from a binding
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
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

IntList.push(Int)(self, item)  // Call example

// Generic method (RFC-023 syntax: type parameters automatically inferred at call site)
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

Generic types and generic functions uniformly use the `()` syntax for calls. `[]` is not used in any generic context.

**Core rules**:

1. **`()` does everything for application**: type application, function calls, and value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function call — types flow automatically from parameters
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value` — Type parameters are declared on the left, the right side is always concrete values. The `T` of an empty container `List()` must be obtained from the left-side type annotation.

3. **Type information only needs to be written once** — at parameter declaration, the compiler carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  # Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   # T=Int, R=String automatically derived from numbers and f's types
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       # Inferred as List(Int)
y = List("a", "b")      # Inferred as List(String)
z = List()              # ❌ Compilation error: cannot infer T
z: List(Int) = List()   # ✅ T=Int comes from the left-side annotation
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
// ======== 1. Interface Definitions ========
// Interface = a record type with all function fields
// Interfaces do not need self parameter — interfaces only define "function signatures with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // Return interface type, specific implementation returns its own type
    scale: (factor: Float) -> Transformable
}

// ======== 2. Type Definitions ========

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

// ======== 3. Method Implementations (regular functions + explicit binding) ========

// Define functions (self is just a conventional name, not a keyword)
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

// Explicit binding — dot-call syntax is available only after binding
Point.draw = draw[0]
Point.bounding_box = bounding_box[0]
Point.serialize = serialize[0]
Point.translate = translate[0]
Point.scale = scale[0]
Point.distance = distance[0]

// Methods for Rect are similar
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

// Method call (syntax sugar)
p.draw(screen)
r.draw(screen)

// Direct method call
d: Float = distance(p, Point(0.0, 0.0))

// Chained calls
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// Interface assignment
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// Generic function (RFC-023 syntax: type parameters omitted at call site, automatically inferred)
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
    // For each field (function field) of the interface
    for (field_name, iface_field) in &iface.fields {
        // Check whether the type has a method with the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check whether the method signature is compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Compare: should match after removing the self parameter
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

Interface types support direct assignment, and the compiler automatically selects the optimal calling strategy based on the right-hand side type of the assignment:

```yaoxiang
// Direct assignment of concrete type → compile-time can determine concrete type, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → compile-time cannot determine concrete type, uses vtable
d: Drawable = get_shape()
d.draw(screen)  // Look up method through vtable

// Heterogeneous collection → uses vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Look up method through vtable
}
```

**Compile-time optimization strategy**:

| Scenario | Inference Result | Call Method |
|----------|-----------------|-------------|
| `d: Drawable = Circle(1)` | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()` | Unknown | vtable |
| `shapes: List(Drawable) = [...]` | Heterogeneous | vtable |

**Rules**:
1. When the right-hand side is a concrete type constructor and is determinable at compile time, generate direct call IR
2. When the right-hand side type cannot be determined at compile time, fall back to the vtable mechanism
3. The vtable fallback ensures correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as it has the same methods, it can be assigned to the interface type
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
| Requires `impl` keyword | No keyword needed, interface names are written after the type body |

## Syntax Design Notes: Named Functions Are Essentially Syntax Sugar for Lambdas

### Core Understanding

**Named functions and lambda expressions are the same thing!** The only difference is that named functions give the lambda a name.

```yaoxiang
// These two are essentially completely identical
add: (a: Int, b: Int) -> Int = a + b           // Named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda form (completely equivalent)
```

### Syntax Sugar Model

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially
name: (Params) -> ReturnType = (params) => body
```

**Key point**: when the signature fully declares parameter types, the parameter names in the lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer scope variables**: the parameter scope in the signature overrides the function body, and the inner scope has higher priority.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x overrides outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be in any of the following positions, **at least one annotation is required**:

| Annotation Position | Form | Description |
|---------------------|------|-------------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda head only | `double = (x: Int) => x * 2` | ✅ Valid |
| Both sides | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Examples

```yaoxiang
// ✅ Recommended: signature is complete, lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Valid: annotate type in lambda head
double = (x: Int) => x * 2

// ✅ Valid: annotate on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature | Advantage |
|---------|-----------|
| **Concise** | No need to repeat parameter names when signature is complete |
| **Flexible** | Lambda form is preserved, use whichever you like |
| **Consistent** | Maintains unified pattern with variable declaration `x: Int = 42` |
| **Intuitive** | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage | Description |
|-----------|-------------|
| Ultimate unity | A single syntax rule covers all cases |
| Theoretically elegant | Perfectly symmetric `name: type = value` |
| No new keywords | Reuses existing syntax elements |
| Easy to implement | Compiler only needs to handle one declaration form |
| Easy to learn | Remember one pattern to write all code |
| Easy to extend | New features can naturally fit into this model |

### Disadvantages

| Disadvantage | Description |
|--------------|-------------|
| Naming convention | Methods need to follow the `Type.method` naming |
| Verbosity | Complete syntax is longer than simplified syntax, but inferable |
| Learning curve | Requires understanding the unified model |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compilation error example:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Types can be omitted and inferred by the compiler
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE automatically hints at missing methods
```

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Parsing complexity | Unified syntax may increase parsing complexity | Use recursive descent parser |
| Performance overhead | vtable lookup may have additional overhead | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: The Origin of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: This is the **unspeakable** thing!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One gives birth to two, two to three, three to all things.   ║
║   The Yi has the Supreme Ultimate, which gives birth to the Two Forms.║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.   ║
║   The compiler is silent here, philosophy pauses here.        ║
║                                                              ║
║   Thank you for reaching the philosophical boundary of the language.║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would lead to the Type0/Type1 universe paradox), but we deliberately preserve this "easter egg"—when you try to compile it, you will receive a Zen message from the language's founder. This is not only a technical boundary, but also YaoXiang's tribute to type philosophy.

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

# Generic parameters: as part of the function type, e.g. (T: Type, R: Type) -> (...)
# No independent BNF rules needed — : Type parameters are ordinary function parameters

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
| Declaration | Assignment statement in the form `name: type = value` |
| Record type | A `{ ... }` type containing named fields |
| Interface | A record type with all function fields |
| Generic type | A type defined as `Name: (T: Type) -> Type = { ... }` that accepts type parameters |
| Namespace function | A function in the form `Type.name` that belongs to the Type namespace. Implies no binding |
| Method binding | `Type.name = func[n]`, binds position n of func as the caller, enabling the `obj.name(args)` syntax |
| Generic function | A function using the `(T: Type)` syntax, where type parameters are the first parameter group |
| Meta type | `Type`, the only type hierarchy marker in the language |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Review     │  ← Open community discussion and feedback
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
│ (Formal design)│  │ (Remain in place)│
└─────────────┘    └─────────────┘
```