---
title: "RFC-010: Unified Type Syntax - name: type = value Model"
status: "Accepted"
author: "Chenxu"
updated: "2026-07-14 (Never builtin type implemented, #157 closed)"
issue: "#127"
---
# RFC-010: Unified Type Syntax - name: type = value Model

## Summary

This RFC proposes an extremely simple, unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression.
**No `fn`, no `struct`, no `trait`, no `impl`, no lowercase `type` keyword (but `Type` exists as the meta-type keyword)**.

> **Core Design**: `Type` is itself a generic type. `(T: Type) -> Type` represents "a type that accepts type parameter T".

| Concept       | Code Form                                          |
|---------------|---------------------------------------------------|
| Variable      | `x: Int = 42`                                |
| Function      | `add: (a: Int, b: Int) -> Int = a + b`       |
| Record Type   | `Point: Type = { x: Float, y: Float }`       |
| Interface     | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic Type  | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| Generic Type  | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| Method        | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic Function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` is the only meta-type keyword in the language**.

> **Namespace vs Method Binding**: The `Type.name` prefix denotes **namespace ownership**, and nothing more.
> It triggers no implicit binding. To make the `.` call syntax like `p.draw(screen)` work,
> you must explicitly bind: `Point.draw = draw[0]`.
> See the "Namespace and Method Binding" section below for details.

It is used to mark the type level; the compiler automatically handles the distinction of Type0, Type1, Type2..., which is transparent to the user.

```yaoxiang
// Core syntax: unify and distinguish

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

The current type system has several separate concepts:
- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack unification, leading to fragmented syntax and high learning cost.

### Design Goals

1. **Extreme Unification**: One syntax rule covers all cases
2. **Concise and Elegant**: Symmetric aesthetics of `name: type = value`
3. **No New Keywords**: Reuse existing syntax elements
4. **Theoretically Elegant**: Types themselves are values of type Type
5. **Generics-Friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 and the generics system design of RFC-011 are **naturally compatible**—generic parameters can seamlessly blend into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type position in signature can be omitted, auto-inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraints (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint is carried by the parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:
- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generics examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 in sync with RFC-010

## Proposal

### Core Principle: Type Constructors vs Functions/Variables

**This is a key design choice that determines the disambiguation rules for the syntax:**

| Syntax | Meaning | Rule |
|------|------|------|
| **`x: Type = ...`** | Type constructor | Explicit `: Type` declaration → forced to be a type |
| **`f = ...`** | Function or variable | No `: Type` → HM actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:
- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (execution statement, returns Void)

**Disambiguation rules**:
- **With** `: Type` → forced to parse as type constructor, `{ ... }` is a type literal
- **Without** `: Type` → HM actively parses `{ ... }` as a code block, inferred as a function type

```yaoxiang
# ✅ Type constructor: with : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: without : Type, HM infers () -> Void
main = { println("Hello") }

# ❌ Error: without : Type, compiler cannot parse { ... } as a type
Point = { x: Float, y: Float }  // HM infers as a function, not a type!
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
├── Generic type (multi-parameter)
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # Must return: Type
│
├── Namespace function
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # Point-call syntax only available after explicit binding
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Meta-Type Hierarchy (Compiler Internals)

**The compiler internally** maintains a universe hierarchy `level: selfpointnum` (stored as a string, theoretically infinitely extensible).

| Level | Description |
|-------|------|
| `Type0` | Everyday types (`Int`, `Float`, `Point`) |
| `Type1` | Type constructors (`List`, `Maybe`) |
| `Type2+` | Higher-order constructors |

**Users never see these numbers**, only `: Type`.

### Curry-Howard Isomorphism: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not chosen arbitrarily—it is precisely a direct mapping of the Curry-Howard correspondence. This isomorphism reveals a profound truth: **the type system and the logical system are two sides of the same thing**.

| Logic (Proposition) | Type System (YaoXiang) | Example |
|---|---|---|
| Proposition P | Type T | `Int`, `Bool` |
| Proof of P | A value of type T | `42: Int`, `true: Bool` |
| P → Q (implication) | Function type `(P) -> Q` | `(x: Int) -> Bool` |
| P ∧ Q (conjunction) | Record type `{ p: P, q: Q }` | `{ x: Int, y: Bool }` |
| ∀x.P(x) (universal quantification) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...` |
| P ⊕ Q (disjunction) | Enum / tagged union | `Maybe: (T: Type) -> Type = { ... }` |

**What `name: type = value` means under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads: "There exists a proof of type Int, named x, whose value is 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads:
// "There exists an implication proof: given proofs a and b of Int, we can construct a proof of Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical Consistency = Type Safety**: If a type system allows constructing a value of type `T` with no valid runtime representation, it's like allowing a proof of a false proposition in logic—the system collapses. Curry-Howard tells us: **a type-safe language is naturally a consistent logical system**.

2. **Universe Hierarchy is a Necessary Condition**: As detailed below, allowing `Type: Type` (i.e., "the type of types is also a type") would produce Russell's paradox (manifested as Girard's paradox in type theory). YaoXiang's `Type₀ : Type₁ : Type₂ : ...` stratification ensures each type belongs to exactly one level, forming a never-closing ascending chain that fundamentally avoids paradox. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

3. **Theoretical Foundation of Unified Syntax**: The reason `name: type = value` can cover all concepts—variables, functions, types, interfaces, generics—with a single syntax is precisely because under Curry-Howard they are all the same thing—**providing proofs for propositions**. Variables are evidence of propositions, functions are evidence of implications, records are evidence of conjunctions, generics are evidence of universal quantification. Unified syntax is not an arbitrary design coincidence, but a natural consequence of the Curry-Howard isomorphism.

> **Further Reading**: Wadler, P. (2015). *"Propositions as Types."* Communications of the ACM, 58(12), 75–84. This article explains the history and significance of the Curry-Howard isomorphism in accessible language.

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

// Void function (no return needed inside the code block)
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### Return Rules

The return value depends on the form on the right side of `=`:

| Form | Return Value |
|------|--------|
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

> **Design Rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its return semantics differ from a single expression. Braces introduce a multi-statement context, so an explicit `return` is needed to eliminate the ambiguity of "whether the last expression is the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

In YaoXiang, `{ ... }` is not merely a code block—it is a **dependency-driven computation unit**. This semantics is consistent across function bodies, variable initialization, and `spawn`:

**Core Rules**:
- Assignment statements inside `{}` are automatically ordered by dependency, not by writing order
- Execute immediately when dependencies are ready, block and wait if missing
- Use `return` to explicitly return a value (see return rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler auto-orders
result: Int = {
    b = a + 1      # Depends on a → auto-ordered after a
    a = 10         # No dependency → can execute first
    return b       # Returns 11
}
```

> **Difference from Single Expression**: `= expr` (without braces) is a simple binding that directly returns a value; `= { ... }` (with braces) introduces a dependency-driven computation context, allowing multi-statements and explicit `return`.

#### `spawn` Block

`spawn { ... }` is the sole parallel primitive in YaoXiang. It leverages the dependency-driven semantics of `{}` to achieve automatic parallelization:

- Direct sub-assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks with ready dependencies execute concurrently immediately
- The caller blocks until all sub-tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, runs in parallel)
    c = process(a, b)         # Depends on a, b → executes after both complete
    return c
}
// Caller blocks here until all tasks in the spawn block complete
```

> **Detailed Definition**: See `008-runtime-concurrency-model.md` for the complete semantics, task creation rules, and blocking model of `spawn`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and operate on raw pointers. It uses the `return` semantics of `{}` to return type definitions to the enclosing scope:

**Core Rules**:
- Inside `unsafe {}`, you can define types and operate on raw pointers
- Use `return` to return type definitions to the enclosing scope
- Returned types are available outside `unsafe {}`
- Field access on the type requires unsafe permission

```yaoxiang
# Define opaque type inside an unsafe block
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

> **Detailed Definition**: See `ffi.md` for the complete semantics of `unsafe`, FFI type definition, and method binding.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound methods, and interface implementations:

##### Basic Types

**Record Type**: A list of fields; field types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with Default Values**: Fields can have default values, optional at construction time.

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

**Fields without Default Values**: Must be provided at construction time.

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

##### Builtin Types

YaoXiang's identifier system has three layers, recognized by different compiler stages in sequence:

1. **Keywords** (parser independent tokens) — Control structures and declaration keywords, such as `if`, `match`, `pub`, `return`
2. **Literal Reserved Words** (parser independent tokens) — `true`, `false`, `void`, `Type`, cannot be used as regular identifiers
3. **Builtin Type Names** (preregistered in the type checker) — Parser treats them as regular identifiers; the type checker is responsible for parsing. **Not reserved words, can be shadowed (not recommended)**

The difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, builtin type name): `void` is a value literal (equal to the only value of Unit), `Void` is a type name (equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Predefined builtin type names:

| Type | Logical Correspondence | Description |
|------|---------|------|
| `Never` | ⊥ (false/empty type) | Zero constructors, no value can inhabit this type. Represents "impossibility"—divergence, panic, dead code. `Never <: T` holds for any `T` (ex falso quodlibet). A function returning `Never` never returns normally. **Not a keyword, but a builtin type name.** |
| `Void` | ⊤ (true/Unit) | Exactly one inhabitant (default void value). `x: Void = <default>` is legal. The identity of sum types corresponds to the identity of product types—`Void` is the zero-field product type (Unit), `Never` is the zero-variant sum type. |
| `Int` | — | Signed integer |
| `Float` | — | Floating-point number |
| `Bool` | — | Boolean value: `true` / `false` |
| `Char` | — | Unicode character |
| `String` | — | String |

##### Bound Methods

**Method 1: Directly bind an external function inside the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // Bind to position 0; after currying, method: (b: Point) -> Float
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

**Interface name is written inside the type body; the compiler automatically checks its implementation**

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
    Drawable,          // Implement Drawable interface
    Serializable      // Implement Serializable interface
}
```

##### Interface Definition

**Interface = a record type whose fields are all functions**

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

**The `Type.name` prefix denotes namespace ownership**, and nothing more. It triggers no implicit binding.

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

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing it as `p`, `this`, or `x` has exactly the same effect.
> The compiler looks at types, not parameter names.

##### Method Binding (The Only Way)

To make the `.` method call syntax like `p.draw(screen)` work, **explicit binding is required**.
The `[position]` syntax is the only mechanism to bind a function as a "method" (see RFC-004 for detailed syntax).

```yaoxiang
// Define the function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does the p.draw(screen) syntax become available
Point.draw = draw[0]   // The argument at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // Syntax sugar → draw(&p, screen)
Point.draw(p, screen)   // The two call styles are equivalent

// Not writing [0] = not binding. Point.draw is just a regular function alias, no . syntax
Point.draw = draw       // Not binding: only Point.draw(p, screen) works
```

**Default Behavior**: Not writing `[n]` = don't bind any argument. The user must explicitly decide which arguments are filled by the caller.

**Multi-position Binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse Operation** (method to regular function):

```yaoxiang
// Extract the function from the binding
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

// Generic method (RFC-023 syntax: type parameter auto-inferred at call site)
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

Generic types and generic functions are uniformly called using the `()` syntax. `[]` is not used in any generic context.

**Core Rules**:

1. **`()` does all application**: type application, function calls, and value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left side
empty: List(Int) = List()

# Generic function call—types flow automatically from arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value`—Type parameters are declared on the left, the right side is always a concrete value. The `T` of an empty container `List()` must come from the left-side type annotation.

3. **Type information only needs to be written once**—when declaring a parameter, the compiler carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String auto-derived from the types of numbers and f
```

4. **Value construction infers the type from elements**:

```yaoxiang
x = List(1, 2, 3)       // Inferred as List(Int)
y = List("a", "b")      // Inferred as List(String)
z = List()              // ❌ Compilation error: cannot infer T
z: List(Int) = List()   // ✅ T=Int comes from the left-side annotation
```

5. **Type Aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with Old Syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`, `List[Int](1,2,3)` → `List(1,2,3)`.
> The old `[]` generics syntax has been completely removed. `[]` is only used for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========
// Interface = a record type whose fields are all function types
// Interfaces don't need self parameters—they only define "function signatures with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // Returns interface type; concrete implementation returns its own type
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

// ======== 3. Method Implementation (regular function + explicit binding) ========

// Define function (self is just a conventional name, not a keyword)
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

// Explicit binding—only after binding does the dot-call syntax work
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

// Method call (syntax sugar)
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

// Generic function (RFC-023 syntax: omit type parameters at call site, auto-inferred)
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
        // Check if the type has a same-named method
        if let Some(method) = typ.methods.get(field_name) {
            // Check if the method signature is compatible
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

Interface types support direct assignment; the compiler automatically selects the optimal call strategy based on the right-hand value's type:

```yaoxiang
// Direct assignment of concrete type → concrete type can be determined at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: directly calls circle_draw(screen), no vtable

// Function return value → concrete type cannot be determined at compile time, use vtable
d: Drawable = get_shape()
d.draw(screen)  // Look up method via vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Look up method via vtable
}
```

**Compile-Time Optimization Strategy**:

| Scenario | Inferred Result | Call Method |
|------|----------|----------|
| `d: Drawable = Circle(1)` | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()` | Unknown | vtable |
| `shapes: List(Drawable) = [...]` | Heterogeneous | vtable |

**Rules**:
1. When the right-hand value is a concrete type constructor and can be determined at compile time, generate direct-call IR
2. When the right-hand value's type cannot be determined at compile time, fall back to the vtable mechanism
3. vtable serves as the fallback to ensure the correctness of runtime polymorphism

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

| Before | After |
|------|------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| Requires the `impl` keyword | No keyword needed; interface names are written after the type body |

## Syntax Design Notes: Named Functions Are Syntactic Sugar for Lambdas

### Core Understanding

**Named functions and lambda expressions are the same thing!** The only difference is that a named function gives the lambda a name.

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

**Key Point**: When the signature fully declares parameter types, the parameter names in the lambda header become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: The parameter scope in the signature takes precedence over the function body; the inner scope has higher priority.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x overrides outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be in any of the following positions; **annotating at least one location suffices**:

| Annotation Position | Form | Description |
|----------|------|------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda header only | `double = (x: Int) => x * 2` | ✅ Legal |
| Both sides | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Example

```yaoxiang
// ✅ Recommended: signature complete, lambda header omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: annotate types in the lambda header
double = (x: Int) => x * 2

// ✅ Legal: annotate both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature | Advantage |
|------|------|
| **Concise** | No need to repeat parameter names when the signature is complete |
| **Flexible** | Keeps the lambda form; use whichever you prefer |
| **Consistent** | Maintains a unified pattern with variable declaration `x: Int = 42` |
| **Intuitive** | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage | Description |
|------|------|
| Extreme Unification | One syntax rule covers all cases |
| Theoretically Elegant | Perfectly symmetric `name: type = value` |
| No New Keywords | Reuses existing syntax elements |
| Easy to Implement | The compiler only needs to handle one declaration form |
| Easy to Learn | Remember one pattern and you can write all code |
| Easy to Extend | New features naturally fit into this model |

### Disadvantages

| Disadvantage | Description |
|------|------|
| Naming Convention | Methods need to follow the `Type.method` naming |
| Verbosity | The full syntax is longer than simplified syntax, but can be inferred |
| Learning Curve | Need to understand the unified model |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Example compilation error:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Types can be omitted and inferred by the compiler
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE auto-suggests missing methods
```

### Risks

| Risk | Impact | Mitigation |
|------|------|----------|
| Parsing Complexity | Unified syntax may increase parsing complexity | Use a recursive descent parser |
| Performance Overhead | vtable lookups may add extra overhead | Compile-time monomorphization optimization |

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
║   One begets two, two beget three, three beget all things.    ║
║   The Yi has the Supreme Ultimate, which begets the Two Modes.║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.   ║
║   The compiler falls silent here, philosophy pauses here.     ║
║                                                              ║
║   Thank you for reaching the philosophical boundary           ║
║   of the language.                                           ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause a Type0/Type1 universe paradox), but we deliberately keep this "easter egg"—when you try to compile it, you will receive a zen message from the language's creator. This is not just a technical boundary, but also YaoXiang's tribute to type philosophy.

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
       | 'Type'                                    # Meta-type

type_field ::= identifier ':' type_expr
             | identifier                           # Interface constraint

# Generic parameters: as part of function type, e.g., (T: Type, R: Type) -> (...)
# No separate BNF rule needed—: Type parameters are regular function parameters

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
|------|------|
| Declaration | An assignment statement in the form `name: type = value` |
| Record Type | A `{ ... }` type containing named fields |
| Interface | A record type whose fields are all function types |
| Generic Type | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters |
| Namespace Function | A function in the form `Type.name`, belonging to the Type namespace. Implies no binding |
| Method Binding | `Type.name = func[n]`, binds position n of func as the caller, enabling the `obj.name(args)` syntax |
| Generic Function | A function using the `(T: Type)` syntax, with type parameters as the first parameter group |
| Meta-Type | `Type`, the only type-level marker in the language |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review│  ← Open community discussion and feedback
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
│(Formal des.)│    │(Remains here)│
└─────────────┘    └─────────────┘
```