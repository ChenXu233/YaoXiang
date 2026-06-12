---
title: "RFC-010: Unified Type Syntax - The name: type = value Model"
status: "Accepted"
author: "Chenxu"
created: "2025-01-20"
updated: "2026-06-05 (Updated return rules and {} semantics)"
---

# RFC-010: Unified Type Syntax - The `name: type = value` Model


## Summary

This RFC proposes an extremely minimal and unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression.
**There is no `fn`, no `struct`, no `trait`, no `impl`, and no lowercase `type` keyword (but there is `Type` as the meta type keyword)**.

> **Core Design**: `Type` itself is a generic type. `(T: Type) -> Type` represents "a type that accepts the type parameter T".

| Concept       | Code Form                                      |
|---------------|-----------------------------------------------|
| Variable      | `x: Int = 42`                                |
| Function      | `add: (a: Int, b: Int) -> Int = a + b`       |
| Record type   | `Point: Type = { x: Float, y: Float }`       |
| Interface     | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic type  | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| Generic type  | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| Method        | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` is the only meta type keyword in the language**.

> **Namespace vs Method Binding**: The `Type.name` prefix indicates **namespace ownership**, nothing more.
> It does not trigger any implicit binding. For the `.` call syntax like `p.draw(screen)` to work,
> an explicit binding is required: `Point.draw = draw[0]`.
> See the "Namespace and Method Binding" section below for details.
It is used to mark the type hierarchy, and the compiler automatically handles the distinction between Type0, Type1, Type2, ..., which is transparent to the user.

```yaoxiang
// Core syntax: unified + distinct

// Variable
x: Int = 42

// Function (parameter names in the signature)
add: (a: Int, b: Int) -> Int = a + b

// Record type
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// Interface (essentially a record type whose fields are all functions)
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

// Generic type ((T: Type) -> Type = generic type that accepts a type parameter)
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
p.draw(screen)           // syntactic sugar → Point.draw(p, screen)
s: Drawable = p           // structural subtyping: Point implements Drawable
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## Motivation

### Why is this feature needed?

The current type system has several separate concepts:
- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack unity, leading to fragmented syntax and a high learning cost.

### Design Goals

1. **Extreme Unity**: One syntax rule covers all cases
2. **Concise and Elegant**: The symmetric aesthetic of `name: type = value`
3. **No New Keywords**: Reuse existing syntactic elements
4. **Theoretically Elegant**: Types themselves are values of type Type
5. **Generics-Friendly**: Seamlessly integrated with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 is **naturally compatible** with the generics system design of RFC-011, and generic parameters can seamlessly integrate into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type position in signature can be omitted, inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraint (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:
- RFC-011 Phase 1 (Basic Generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 and RFC-010 simultaneously

## Proposal

### Core Principle: Type Constructor vs Function/Variable

**This is a critical design choice that determines the disambiguation rules for the syntax:**

| Form | Meaning | Rule |
|------|---------|------|
| **`x: Type = ...`** | Type constructor | `: Type` explicit declaration → forced to be a type |
| **`f = ...`** | Function or variable | No `: Type` → HM actively infers function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:
- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executed statement, returns Void)

**Disambiguation rules**:
- **With** `: Type` → forced to parse as a type constructor, `{ ... }` is a type literal
- **Without** `: Type` → HM actively parses `{ ... }` as a code block, infers a function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main = { println("Hello") }

# ❌ Error: no : Type, compiler cannot parse { ... } as a type
Point = { x: Float, y: Float }  // HM infers as a function, not a type!
```

---

**Unified Model: identifier : type = expression**

```
├── Variable
│   └── x: Int = 42
│
├── Function
│   └── add: (a: Int, b: Int) -> Int = a + b  # No : Type, HM infers as a function
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
│       Point.draw = draw[0]  # Explicit binding enables the dot call syntax
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as a function
```

### Meta Type Hierarchy (Compiler Internal)

The compiler internally maintains a universe hierarchy `level: selfpointnum` (stored as a string, theoretically infinitely extensible).

| Level | Description |
|-------|-------------|
| `Type0` | Everyday types (`Int`, `Float`, `Point`) |
| `Type1` | Type constructors (`List`, `Maybe`) |
| `Type2+` | Higher-order constructors |

**Users never see these numbers**, they only see `: Type`.

### Curry-Howard Isomorphism: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not an arbitrary choice—it is a direct mapping of the Curry-Howard correspondence. This correspondence reveals a profound fact: **the type system and the logic system are two sides of the same coin**.

| Logic (Proposition) | Type System (YaoXiang) | Example |
|---|---|---|
| Proposition P | Type T | `Int`, `Bool` |
| Proof that P holds | A value of type T | `42: Int`, `true: Bool` |
| P → Q (Implication) | Function type `(P) -> Q` | `(x: Int) -> Bool` |
| P ∧ Q (Conjunction) | Record type `{ p: P, q: Q }` | `{ x: Int, y: Bool }` |
| ∀x.P(x) (Universal quantification) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...` |
| P ⊕ Q (Disjunction) | Enum / tagged union | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "There exists a proof of type Int, named x, whose value is 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "There exists an implication proof: given proofs a and b of Int, we can construct a proof of Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires both a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical Consistency = Type Safety**: If the type system allows constructing a value of type `T` with no legitimate runtime representation, that is like allowing a proof of a false proposition in logic—the system collapses. Curry-Howard tells us: **a type-safe language is naturally a logically consistent system**.

2. **Universe Hierarchy is a Necessary Condition**: As detailed below, allowing `Type: Type` (i.e., "the type of types is also a type") would produce the Russell paradox (which manifests as Girard's paradox in type theory). YaoXiang's `Type₀ : Type₁ : Type₂ : ...` stratification ensures that each type belongs to a single level, forming a never-closing ascending chain, fundamentally avoiding the paradox. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

3. **Theoretical Foundation of Unified Syntax**: The reason `name: type = value` can use one syntax to cover all concepts of variables, functions, types, interfaces, and generics is that they are all the same thing under Curry-Howard—**providing proofs for propositions**. Variables are evidence for propositions, functions are evidence for implications, records are evidence for conjunctions, and generics are evidence for universal quantification. The unified syntax is not a human-designed coincidence, but a natural consequence of the Curry-Howard correspondence.

> **Further Reading**: Wadler, P. (2015). *"Propositions as Types."* Communications of the ACM, 58(12), 75–84. This article explains the history and significance of the Curry-Howard correspondence in accessible language.

### Syntax Definition

#### 1. Variable Declaration

```yaoxiang
// Basic syntax
x: Int = 42
name: String = "Alice"
flag: Bool = true

// Type inference (can be omitted)
y = 100  // inferred as Int
```

#### 2. Function Definition

```yaoxiang
// Single-expression form (returns the value directly, no return needed)
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

// Void function (no return needed in the code block)
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### Return Rules

The return value depends on the form on the right side of `=`:

| Form | Return Value |
|------|--------------|
| `= expr` (no braces) | Returns `expr` directly |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` |

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

> **Design Rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its return semantics differ from a single expression. Braces introduce a multi-statement context, so an explicit `return` is required to disambiguate "whether the last expression is the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

In YaoXiang, `{ ... }` is not just a code block—it is a **dependency-driven computation unit**. This semantics is consistent across function bodies, variable initialization, and `spawn`:

**Core Rules**:
- Assignment statements within `{}` are automatically ordered by their dependency relationships, not by their written order
- Execution proceeds immediately when dependencies are satisfied; otherwise, execution is blocked waiting
- Use `return` to explicitly return a value (see Return Rules)

```yaoxiang
# Dependency-driven: b depends on a, the compiler automatically orders them
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependencies → can execute first
    return b       # returns 11
}
```

> **Difference from Single Expression**: `= expr` (no braces) is a simple binding that directly returns a value; `= { ... }` (with braces) introduces a dependency-driven computation context, allowing multiple statements and explicit `return`.

#### `spawn` Block

`spawn { ... }` is the sole parallel primitive in YaoXiang. It leverages the dependency-driven semantics of `{}` to achieve automatic parallelization:

- Direct sub-assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks with satisfied dependencies execute concurrently immediately
- The caller blocks until all sub-tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # task 1
    b = fetch_data("url2")    # task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # depends on a, b → executes after both complete
    return c
}
// The caller blocks here until all tasks inside the spawn block complete
```

> **Detailed Definition**: See `008-runtime-concurrency-model.md` for the complete semantics of `spawn`, task creation rules, and blocking model.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and operate on raw pointers. It leverages the return semantics of `{}` to pass type definitions up to the enclosing scope:

**Core Rules**:
- Types can be defined and raw pointers operated within `unsafe {}`
- Use `return` to pass type definitions up to the enclosing scope
- Returned types are usable outside of `unsafe {}`
- Access to type fields requires unsafe permissions

```yaoxiang
# Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw pointer
    }
    return SqliteDb
}

# SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: handle field requires unsafe permission
handle = db.handle

# ✅ Access through method calls
db.close()
```

> **Detailed Definition**: See `ffi.md` for the complete semantics of `unsafe`, FFI type definitions, and method binding.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, encompassing fields, default values, bound methods, and interface implementations:

##### Basic Types

**Record type**: a list of fields, where field types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: fields can have default values, optional at construction time.

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

**Fields without default values**: must be provided at construction time.

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

**Method 1: Bind external functions directly within the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // bound to position 0, after currying: method: (b: Point) -> Float
}
// Call: p1.distance(p2) → distance(p1, p2)
```

**Method 2: Anonymous function + positional binding**

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

**Interface names are written inside the type body; the compiler automatically checks the implementation**

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
    Drawable,          // implements Drawable interface
    Serializable      // implements Serializable interface
}
```

##### Interface Definition

**An interface = a record type whose fields are all functions**

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

**The `Type.name` prefix indicates namespace ownership**, nothing more. It does not trigger any implicit binding.

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

> **Note**: `self` is not a keyword, it is merely a conventional parameter name. Writing `p`, `this`, or `x` has exactly the same effect.
> The compiler does not look at parameter names, it looks at types.

##### Method Binding (The Only Way)

For the `.` method call syntax like `p.draw(screen)` to work, **an explicit binding is required**.
The `[position]` syntax is the only mechanism for binding a function as a "method" (see RFC-004 for detailed syntax).

```yaoxiang
// Define a function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this is the p.draw(screen) syntax available
Point.draw = draw[0]   // the parameter at position 0 (&Point) is filled in by the caller

// Usage
p.draw(screen)          // syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // both call forms are equivalent

// Not writing [0] = no binding. Point.draw is just a regular function alias, with no . syntax
Point.draw = draw       // no binding: only Point.draw(p, screen) is possible
```

**Default Behavior**: Omitting `[n]` = no parameter bound. The user must explicitly decide which parameters are filled in by the caller.

**Multi-position Binding**:

```yaoxiang
// Bind multiple positions (auto-currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse Operation** (method to regular function):

```yaoxiang
// Extract the function from a binding
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. Interface Composition

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using intersection types
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

IntList.push(Int)(self, item)  // call example

// Generic methods (RFC-023 syntax: type parameters automatically inferred at call site)
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

1. **`()` does everything**: type application, function calls, and value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function call—types flow automatically from parameters
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value`—Type parameters are declared on the left, the right side is always a concrete value. The `T` of an empty container `List()` must be obtained from the left-side type annotation.

3. **Type information only needs to be written once**—in the parameter declaration, the compiler carries it along:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically come from the types of numbers and f
```

4. **Value construction infers the type from elements**:

```yaoxiang
x = List(1, 2, 3)       // inferred as List(Int)
y = List("a", "b")      // inferred as List(String)
z = List()              // ❌ compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int comes from the left-side annotation
```

5. **Type aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with Old Syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`, `List[Int](1,2,3)` → `List(1,2,3)`.
> The old `[]` generic syntax has been completely removed. `[]` is now used only for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========
// An interface = a record type whose fields are all function types
// The interface does not need a self parameter — the interface only defines the "function signature with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // returns interface type, concrete implementation returns its own type
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

// ======== 3. Method Implementation (regular functions + explicit binding) ========

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

// Explicit binding — only after binding is the dot call syntax available
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

// Method call (syntactic sugar)
p.draw(screen)
r.draw(screen)

// Regular method call (direct call)
d: Float = distance(p, Point(0.0, 0.0))

// Chained call
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// Interface assignment
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// Generic function (RFC-023 syntax: omit type parameters at call site, inferred automatically)
process_all: (items: List(T)) -> Void = {
    for item in items {
        print(item.serialize())
    }
}

process_all([p, r])
```

## Detailed Design

### Interface Check Algorithm

```rust
fn check_type_implements_interface(
    typ: &Type,
    iface: &Type
) -> Result<(), TypeError> {
    // For each field (function field) of the interface
    for (field_name, iface_field) in &iface.fields {
        // Check if the type has a method with the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check whether the method signature is compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Comparison: should match after removing the self parameter
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

Interface types support direct assignment, and the compiler automatically selects the optimal call strategy based on the right-hand side type of the assignment:

```yaoxiang
// Direct assignment of a concrete type → concrete type is determinable at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: directly calls circle_draw(screen), no vtable

// Function return value → concrete type cannot be determined at compile time, vtable is used
d: Drawable = get_shape()
d.draw(screen)  // Method is looked up through the vtable

// Heterogeneous collection → vtable is used
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Method is looked up through the vtable
}
```

**Compile-Time Optimization Strategies**:

| Scenario | Inference Result | Call Method |
|----------|------------------|-------------|
| `d: Drawable = Circle(1)` | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()` | Unknown | vtable |
| `shapes: List(Drawable) = [...]` | Heterogeneous | vtable |

**Rules**:
1. When the right-hand side is a concrete type constructor determinable at compile time, generate a direct call IR
2. When the right-hand side type cannot be determined at compile time, fall back to the vtable mechanism
3. The vtable is the fallback to ensure correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as the same methods exist, it can be assigned to an interface type
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
| Requires the `impl` keyword | No keyword required, interface names are written after the type body |

## Syntax Design Note: Named Functions are Essentially Syntactic Sugar for Lambdas

### Core Understanding

**Named functions and Lambda expressions are the same thing!** The only difference is that a named function gives a Lambda a name.

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

**Key Point**: When the signature fully declares the parameter types, the parameter names in the Lambda header become redundant and can be omitted.

### Parameter Scope Rules

**Parameters shadow outer variables**: parameters in the signature shadow the outer scope, the inner scope has higher priority.

```yaoxiang
x = 10  // outer variable

double: (x: Int) -> Int = x * 2  // ✅ parameter x shadows outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be placed in any of the following positions, **at least one is required**:

| Annotation Position | Form | Description |
|---------------------|------|-------------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda header only | `double = (x: Int) => x * 2` | ✅ Valid |
| Both sides | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Example

```yaoxiang
// ✅ Recommended: signature is complete, Lambda header is omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Valid: type annotations in Lambda header
double = (x: Int) => x * 2

// ✅ Valid: annotations on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature | Advantage |
|---------|-----------|
| **Concise** | No need to repeat parameter names when the signature is complete |
| **Flexible** | Lambda form is preserved, use whichever you prefer |
| **Consistent** | Maintains the unified pattern with variable declaration `x: Int = 42` |
| **Intuitive** | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage | Description |
|-----------|-------------|
| Extreme Unity | One syntax rule covers all cases |
| Theoretically Elegant | Perfectly symmetric `name: type = value` |
| No New Keywords | Reuses existing syntactic elements |
| Easy to Implement | The compiler only needs to handle one declaration form |
| Easy to Learn | Remember one pattern and you can write all code |
| Easy to Extend | New features can naturally integrate into this model |

### Disadvantages

| Disadvantage | Description |
|--------------|-------------|
| Naming Convention | Methods need to follow the `Type.method` naming convention |
| Verbosity | The full syntax is longer than the simplified syntax, but inference is available |
| Learning Curve | Need to understand the unified model |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compile error example:
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
| Parsing Complexity | Unified syntax may increase parsing complexity | Use a recursive descent parser |
| Performance Overhead | vtable lookup may incur extra overhead | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: The Source of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: This is the **unspeakable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One gives birth to two, two gives birth to three,           ║
║   three gives birth to the myriad things.                      ║
║   The Yi has the Supreme Ultimate, which begets the Two Modes.║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.  ║
║   The compiler falls silent here, philosophy pauses here.      ║
║                                                              ║
║   Thank you for touching the philosophical boundary           ║
║   of the language.                                           ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would lead to a Type0/Type1 universe paradox), but we deliberately keep this "easter egg"—when you try to compile it, you will receive a Zen message from the language's founder. This is not only a technical boundary, but also YaoXiang's tribute to the philosophy of types.

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
       | identifier '(' type_expr (',' type_expr)* ')'      # type application
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # function type
       | '{' type_field* '}'                       # record/interface type
       | 'Type'                                    # meta type

type_field ::= identifier ':' type_expr
             | identifier                           # interface constraint

# Generic parameter: as part of a function type, e.g., (T: Type, R: Type) -> (...)
# No independent BNF rule needed — : Type parameters are ordinary function parameters

# Expression
expression ::= literal
              | identifier
              | identifier '(' expression (',' expression)* ')'  # function call / constructor call
              | '(' expression (',' expression)* ')'              # tuple
              | expression '.' identifier '(' arguments? ')'    # method call
              | lambda
              | '{' field ':' expression (',' field ':' expression)* '}'

arguments ::= expression (',' expression)*

lambda ::= '(' parameter_list? ')' '=>' block

block ::= expression | '{' expression* '}'
```

### Glossary

| Term | Definition |
|------|------------|
| Declaration | An assignment statement in the form `name: type = value` |
| Record type | A `{ ... }` type containing named fields |
| Interface | A record type whose fields are all function types |
| Generic type | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters |
| Namespace function | A function in `Type.name` form, belonging to the Type namespace. Does not imply any binding |
| Method binding | `Type.name = func[n]`, binds position n of func as the caller, enabling the `obj.name(args)` syntax |
| Generic function | A function using `(T: Type)` syntax, with type parameters as the first parameter group |
| Meta type | `Type`, the sole type hierarchy marker in the language |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← Open community discussion and feedback
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
│ (formal     │    │ (kept in    │
│  design)    │    │   place)    │
└─────────────┘    └─────────────┘
```