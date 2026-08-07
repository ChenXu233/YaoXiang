---
title: 'RFC-010: Unified Type Syntax - The name: type = value Model'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-14 (Never built-in type implemented, #157 closed)'
issue: '#127'
---

# RFC-010: Unified Type Syntax - The name: type = value Model

## Summary

This RFC proposes a minimalist unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression. **There is no
`fn`, no `struct`, no `trait`, no `impl`, no lowercase `type` keyword (but there is `Type` as the
meta-type keyword)**.

> **Core Design**: `Type` itself is a generic type. `(T: Type) -> Type` represents "a type that
> accepts the type parameter T".

| Concept          | Code Form                                                                    |
| ---------------- | ---------------------------------------------------------------------------- |
| Variable         | `x: Int = 42`                                                                |
| Function         | `add: (a: Int, b: Int) -> Int = a + b`                                       |
| Record Type      | `Point: Type = { x: Float, y: Float }`                                       |
| Interface        | `Drawable: Type = { draw: (Surface) -> Void }`                               |
| Generic Type     | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                  |
| Generic Type     | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`     |
| Method           | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic Function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`    |

**`Type` is the only meta-type keyword in the language**.

> **Namespace vs Method Binding**: The `Type.name` prefix denotes **namespace membership**, nothing
> more. It does not trigger any implicit binding. To make the `.` call syntax like `p.draw(screen)`
> work, an explicit binding is required: `Point.draw = draw[0]`. See the "Namespace and Method
> Binding" section below. It is used to mark type hierarchies; the compiler automatically handles
> the distinction between Type0, Type1, Type2..., transparent to the user.

```yaoxiang
// Core syntax: unified + distinct

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

// Generic type ((T: Type) -> Type = a generic type accepting type parameters)
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

The current type system has multiple separate concepts:

- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

There is a lack of unity between these concepts, leading to syntax fragmentation and a high learning
cost.

### Design Goals

1. **Extreme Unification**: One syntax rule covers all cases
2. **Concise and Elegant**: The symmetric aesthetic of `name: type = value`
3. **No New Keywords**: Reuse existing syntactic elements
4. **Theoretical Elegance**: Types themselves are values of type Type
5. **Generics-Friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 fits **naturally** with the generics system design of RFC-011,
and generic parameters can blend seamlessly into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic functions (RFC-023 syntax: Type position in signature can be omitted, inferred automatically at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraints (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:

- RFC-011 Phase 1 (Basic Generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 together with RFC-010

## Proposal

### Core Principle: Type Constructors vs Functions/Variables

**This is a key design choice that determines the disambiguation rules for syntax:**

| Syntax              | Meaning              | Rule                                                  |
| ------------------- | -------------------- | ----------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | `: Type` explicit declaration → forced to be a type   |
| **`f = ...`**       | Function or variable | No `: Type` → HM actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:

- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executed statement, returning Void)

**Disambiguation rules**:

- **With** `: Type` → forced to parse as type constructor, `{ ... }` is a type literal
- **Without** `: Type` → HM actively parses `{ ... }` as a code block, inferred as function type

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
│   └── add: (a: Int, b: Int) -> Int = a + b  # no : Type, HM infers as function
│
├── Record Type
│   └── Point: Type = { x: Float, y: Float }  # must return: Type
│
├── Interface
│   └── Drawable: Type = { draw: (Surface) -> Void }  # must return: Type
│
├── Generic Type
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # must return: Type
│
├── Generic Type (multi-parameter)
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # must return: Type
│
├── Namespace Function
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # only after explicit binding does the dot-call syntax work
│
└── Generic Function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # does not return Type, HM infers as function
```

### Meta-Type Hierarchy (Compiler Internal)

**The compiler internally** maintains a universe hierarchy `level: selfpointnum` (stored as a
string, theoretically extensible infinitely).

| Level    | Description                              |
| -------- | ---------------------------------------- |
| `Type0`  | Everyday types (`Int`, `Float`, `Point`) |
| `Type1`  | Type constructors (`List`, `Maybe`)      |
| `Type2+` | Higher-order constructors                |

**Users never see these numbers**, only `: Type`.

### Curry-Howard Correspondence: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not an arbitrary choice—it happens to be a direct
mapping of the Curry-Howard correspondence. This correspondence reveals a profound fact: **type
systems and logic systems are two sides of the same coin**.

| Logic (Proposition)                | Type System (YaoXiang)              | Example                              |
| ---------------------------------- | ----------------------------------- | ------------------------------------ |
| Proposition P                      | Type T                              | `Int`, `Bool`                        |
| Proof that P is true               | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication)                | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)                | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal quantification) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)                | Enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**What `name: type = value` means under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads: "there exists a proof of type Int, named x, with value 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads:
// "there exists an implication proof: given proofs a and b of type Int, one can construct a proof of type Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical Consistency = Type Safety**: If the type system allows constructing a value of type `T`
   without any legal runtime representation, that is like allowing a proof of a false proposition in
   logic—the system collapses. Curry-Howard tells us: **a type-safe language is naturally a
   logically consistent system**.

2. **Universe Hierarchy is a Necessary Condition**: As detailed below, allowing `Type: Type` (i.e.,
   "the type of types is also a type") leads to Russell's paradox (expressed as Girard's paradox in
   type theory). YaoXiang's `Type₀ : Type₁ : Type₂ : ...` stratification ensures each type belongs
   to only one level, forming an ever-rising chain that never closes, fundamentally avoiding
   paradox. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

3. **Theoretical Foundation of Unified Syntax**: The reason `name: type = value` can cover
   variables, functions, types, interfaces, and generics with a single syntax is precisely because
   under Curry-Howard they are all the same thing—**providing proofs for propositions**. Variables
   are evidence for propositions, functions are evidence for implications, records are evidence for
   conjunctions, generics are evidence for universal quantification. Unified syntax is not an
   artificial coincidence, but a natural consequence of the Curry-Howard correspondence.

> **Further Reading**: Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM,
> 58(12), 75–84. This article explains the history and significance of the Curry-Howard
> correspondence in accessible language.

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
// Single-expression form (returns value directly, no return needed)
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

// Void function (no return needed inside code block)
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### Return Rules

The return value depends on the form of the right side of `=`:

| Syntax                    | Return Value                                |
| ------------------------- | ------------------------------------------- |
| `= expr` (no braces)      | Returns `expr` directly                     |
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

> **Design Rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its
> return semantics differ from single expressions. Braces introduce a multi-statement context, so an
> explicit `return` is needed to disambiguate "whether the last expression is the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

In YaoXiang, `{ ... }` is not merely a code block—it is a **dependency-driven computation unit**.
This semantics is consistent across function bodies, variable initialization, and `spawn`:

**Core Rules**:

- Assignment statements inside `{}` are automatically ordered by dependencies, not by writing order
- When dependencies are ready, execution begins immediately; if missing, it blocks and waits
- Use `return` to explicitly return a value (see return rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler automatically orders
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependency → can execute first
    return b       # returns 11
}
```

> **Difference from Single Expression**: `= expr` (no braces) is a simple binding that returns the
> value directly; `= { ... }` (with braces) introduces a dependency-driven computation context,
> allowing multiple statements and explicit `return`.

#### `spawn` Block

`spawn { ... }` is YaoXiang's only parallel primitive. It leverages the dependency-driven semantics
of `{}` to achieve automatic parallelization:

- Direct sub-assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks with ready dependencies execute concurrently immediately
- The caller blocks until all sub-tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # task 1
    b = fetch_data("url2")    # task 2 (no dependency on a, runs in parallel)
    c = process(a, b)         # depends on a, b → waits for both to complete
    return c
}
// caller blocks here until all tasks inside the spawn block complete
```

> **Detailed Definition**: For the complete semantics of `spawn`, task creation rules, and blocking
> model, see `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and manipulate raw pointers. It leverages the return
semantics of `{}` to return type definitions to the enclosing scope:

**Core Rules**:

- Types and raw pointer operations can be defined inside `unsafe {}`
- Use `return` to return type definitions to the enclosing scope
- Returned types are available outside `unsafe {}`
- Accessing type fields requires unsafe permission

```yaoxiang
# Define opaque type inside unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw pointer
    }
    return SqliteDb
}

# SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: handle field requires unsafe permission
handle = db.handle

# ✅ Through method call
db.close()
```

> **Detailed Definition**: For the complete semantics of `unsafe`, FFI type definition, and method
> binding, see `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound
methods, and interface implementations:

##### Basic Types

**Record Type**: a list of fields, where field types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with Default Values**: fields can have default values, which are optional during
construction.

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

**Fields without Default Values**: must be provided during construction.

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

##### Built-in Types

YaoXiang's identifier system has three layers, recognized by different compiler phases in turn:

1. **Keywords** (parser independent tokens) — control structures and declaration keywords, such as
   `if`, `match`, `pub`, `return`
2. **Literal Reserved Words** (parser independent tokens) — `true`, `false`, `void`, `Type`, cannot
   be used as ordinary identifiers
3. **Built-in Type Names** (type checker pre-registered) — the parser treats them as ordinary
   identifiers, the type checker is responsible for parsing. **Not reserved words, can be shadowed
   (not recommended)**

The difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, built-in
type name): `void` is a value literal (equal to the only value of Unit), `Void` is a type name
(equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Pre-set built-in type names:

| Type     | Logical Correspondence | Description                                                                                                                                                                                                                                                                       |
| -------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no value can inhabit this type. Represents "impossible" — divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` indicates it never returns normally. **Not a keyword, is a built-in type name.** |
| `Void`   | ⊤ (true/Unit)          | Has exactly one inhabitant (default void value). `x: Void = <default>` is legal. Corresponds to the unit of sum type and the unit of product type — `Void` is a zero-field product type (Unit), `Never` is a zero-variant sum type.                                               |
| `Int`    | —                      | Signed integer                                                                                                                                                                                                                                                                    |
| `Float`  | —                      | Floating-point number                                                                                                                                                                                                                                                             |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                                                                                                                                                                                                                   |
| `Char`   | —                      | Unicode character                                                                                                                                                                                                                                                                 |
| `String` | —                      | String                                                                                                                                                                                                                                                                            |

##### Bound Methods

**Method 1: Bind external functions directly inside the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // bind to position 0, after currying method: (b: Point) -> Float
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

**Interface names are written inside the type body, and the compiler automatically checks its
implementation**

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

**The `Type.name` prefix denotes namespace membership**, nothing more. It does not trigger any
implicit binding.

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

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing it as `p`, `this`,
> `x` has exactly the same effect. The compiler does not look at parameter names, but at types.

##### Method Binding (The Only Way)

To make `.` method call syntax like `p.draw(screen)` work, **an explicit binding is required**. The
`[position]` syntax is the only mechanism for binding a function as a "method" (detailed syntax see
RFC-004).

```yaoxiang
// Define function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does p.draw(screen) syntax work
Point.draw = draw[0]   // parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // the two call styles are equivalent

// Not writing [0] = no binding. Point.draw is just a regular function alias, without . syntax
Point.draw = draw       // no binding: only Point.draw(p, screen) can be used
```

**Default Behavior**: Not writing `[n]` = no parameters are bound. The user must explicitly decide
which parameters are filled by the caller.

**Multi-Position Binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse Operation** (method to regular function):

```yaoxiang
// Extract function from binding
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

IntList.push(Int)(self, item)  // call example

// Generic method (RFC-023 syntax: type parameters are inferred automatically at the call site)
List.push: (self: List(T), item: T) -> Void = {
    self.data.append(item)
    self.length = self.length + 1
}

List.get: (self: List(T), index: Int) -> Maybe(T) = {
    if index >= 0 and index < self.length {
        return Maybe.Just(self.data[index])
    } else {
        return Maybe.Nothing
    }
}
```

#### 6. Generic Call Syntax

Generic types and generic functions use the `()` syntax uniformly. `[]` is not used in any generic
context.

**Core Rules**:

1. **`()` does all applications**: type application, function call, value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function call — type flows automatically from parameters
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value` — Type parameters are declared on
   the left, the right side is always concrete values. The `T` of an empty container `List()` must
   be obtained from the left type annotation.

3. **Type information only needs to be written once** — when declaring parameters, the compiler
   carries it along:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically comes from the types of numbers and f
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       // inferred as List(Int)
y = List("a", "b")      // inferred as List(String)
z = List()              // ❌ compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int comes from the left annotation
```

5. **Type Aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with Old Syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`,
> `List[Int](1,2,3)` → `List(1,2,3)`. The old `[]` generic syntax has been completely removed. `[]`
> is only used for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========
// Interface = a record type whose fields are all function types
// Interfaces don't need self parameters — interfaces only define "function signatures with the caller position removed"

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

// ======== 3. Method Implementation (Regular Function + Explicit Binding) ========

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

// Explicit binding — dot-call syntax only works after binding
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

// Generic function (RFC-023 syntax: type parameters omitted at call site, inferred automatically)
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
        // Check whether the type has a method with the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check whether the method signature is compatible
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

### Direct Interface Assignment and Compile-Time Optimization

Interface types support direct assignment, and the compiler automatically selects the optimal call
strategy based on the right-hand side type of the assignment:

```yaoxiang
// Direct assignment of concrete type → concrete type can be determined at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // after compilation: direct call to circle_draw(screen), no vtable

// Function return value → concrete type cannot be determined at compile time, use vtable
d: Drawable = get_shape()
d.draw(screen)  // look up method through vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // look up method through vtable
}
```

**Compile-Time Optimization Strategy**:

| Scenario                         | Inference Result     | Call Style                  |
| -------------------------------- | -------------------- | --------------------------- |
| `d: Drawable = Circle(1)`        | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()`      | Unknown              | vtable                      |
| `shapes: List(Drawable) = [...]` | Heterogeneous        | vtable                      |

**Rules**:

1. When the right-hand side is a concrete type constructor and can be determined at compile time,
   generate direct call IR
2. When the right-hand side type cannot be determined at compile time, fall back to vtable mechanism
3. vtable fallback guarantees correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as the same method exists, it can be assigned to an interface type
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

| Before                                   | After                                                                                        |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)`    | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| Requires `impl` keyword                  | No keyword needed, interface names are written after the type body                           |

### Deprecated: `|` Variant Syntax

> **Deprecation Notice (2026-07-25, issue #203)**: The `|` variant syntax is officially deprecated
> and removed from the implementation.

The following syntax is **no longer supported**:

```
type Color = red | green | blue                # ❌ deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ deprecated
type Option(T) = some(T) | none                # ❌ deprecated
```

Unified use of record type expressions and sum types. When a record type's fields are all functions
and all return the type itself, it is a sum type:

```yaoxiang
Color: Type = {
    red: () -> Color,
    green: () -> Color,
    blue: () -> Color
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E)
}

Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T)
}
```

**Design Rationale**:

1. **Eliminate Special Cases**: `|` is the only non-`name: type = value` formal syntax in the BNF.
   After removal, the `type_expr` production is completely unified, and the parser no longer needs
   to maintain independent paths and lookahead fallbacks for variant types.
2. **Mathematical Equivalence**: Under the Curry-Howard correspondence, the disjunction P ⊕ Q
   corresponding to a sum type is equivalent to "a record type whose fields are all functions
   returning the type itself". The two express the same semantics, no need for two sets of syntax.
3. **Zero Destructiveness**: Before removal, the `|` syntax was only half-supported in the parser
   (parameterless variants could be parsed, but parameter types were lost during monomorphization),
   and no user code depended on it.
4. **AST Simplification**: The `Type::Variant(Vec<VariantDef>)` node is deleted, all variant types
   uniformly go through the `Type::Struct` path, and all special branches in downstream
   typecheck/mono/formatter are eliminated.

> **Note**: The semantic properties of sum types (such as match exhaustiveness checks, tagged union
> memory layout) are inferred by the typecheck layer from the `Type::Struct` structure, without
> depending on independent AST nodes.

### Logical Operators: `and` / `or` / `!` (Authoritative Definition, Zig-style)

> **Definition Statement (2026-08-03, issue #251 revision)**: The authoritative form of logical
> operators is the keyword `and` / `or` + symbolic unary `!` (consistent with SPEC `syntax.md` §2.2
> priority table). This design aligns with Zig: **short-circuit control flow uses keywords, pure
> unary operations use symbols**. The early implementation's drift toward C-style `&&` / `||` and
> the intermediate state's keyword `not` have all been removed.

**Semantics**:

| Operator | Priority (SPEC §2.2)            | Associativity | Semantics                                    |
| -------- | ------------------------------- | ------------- | -------------------------------------------- |
| `!`      | 3 (unary prefix, tightly bound) | right to left | logical NOT (pure function, no control flow) |
| `and`    | 10                              | left to right | short-circuit logical AND                    |
| `or`     | 10                              | left to right | short-circuit logical OR                     |

```yaoxiang
# Short-circuit evaluation: when left side of and is false / left side of or is true, right side is not executed
if x != 0 and y / x > 1 { ... }   # no division by zero when x == 0

# Tight binding: !a == b ≡ (!a) == b (Zig-style; opposite to Python's not a == b ≡ not (a == b))
!3 == 4          # false: (!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs)), call then negate
```

The following syntax is **no longer supported** (lexer reports an error and suggests the
corresponding syntax):

```
x && y     # ❌ removed, use x and y
x || y     # ❌ removed, use x or y
not x      # ❌ removed, use !x (not is restored as a regular identifier; != is unaffected)
```

**Design Rationale** (aligned with Zig, ziglang/zig#272 / #6625):

1. **Short-circuit is control flow → keyword; pure function is operation → symbol**. `and` / `or`
   change the order of evaluation (right side is skipped as needed), same nature as `if`, use
   keywords; `!` does pure negation on already-evaluated operands, same nature as `-` `+`, use
   symbols. YaoXiang uses `?` for error propagation (§2.11), no conflict with `!`.
2. **Tight binding eliminates ambiguity**: `!` visually "sticks" to the operand, high priority is
   immediately apparent; the keyword `not` is forced to leave a space from the operand, and what it
   binds to (`not a == b`) easily causes mental ambiguity.
3. **Disambiguation**: `&` has two roles — borrow token (`&p` / `&mut p`, RFC-009) and bitwise AND
   (SPEC §2.2 priority 8). Adding `&&` would let one symbol carry three meanings. `and` / `or` / `!`
   make borrowing, bitwise operations, and logic visually completely separate.
4. **Precedent**: Zig (same ecological niche modern system language) is exactly the combination of
   `and` / `or` keywords + `!` symbol; Python / Lua / Ada / SQL use all keywords (including `not`);
   the C family uses all symbols — YaoXiang takes Zig's mix, getting the best of both.
5. **Curry-Howard Consistency**: Types as propositions (see the correspondence section above),
   logical connections in refinement types are written as `and` / `or` (such as
   `{ 0 <= idx and idx < arr.len }`), which is a natural expression of propositions; `!` as a unary
   negation symbol corresponds to ¬.

> **Implementation**: `and` / `or` are expanded at the IR layer into short-circuit jump sequences
> (`a and b ≡ if a { b } else { false }`), `!` is parsed as unary tight binding (operands according
> to `BP_UNARY + 1`). Regression tests: `tests/yaoxiang/01-syntax/basics/logical_ops.yx`,
> `logical_not.yx`.

## Syntax Design Note: Named Functions Are Syntactic Sugar for Lambdas

### Core Understanding

**Named functions and lambda expressions are the same thing!** The only difference is that a named
function gives a lambda a name.

```yaoxiang
// These two are essentially completely the same
add: (a: Int, b: Int) -> Int = a + b           // named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // lambda form (completely equivalent)
```

### Syntactic Sugar Model

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially
name: (Params) -> ReturnType = (params) => body
```

**Key Point**: When the signature completely declares the parameter types, the parameter names in
the lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: the parameter scope in the signature overrides the function
body, and the inner scope has higher priority.

```yaoxiang
x = 10  // outer variable

double: (x: Int) -> Int = x * 2  // ✅ parameter x overrides outer x, result is 20
```

### Flexible Annotation Location

Type annotations can be in any of the following positions, **annotating at least one place is
sufficient**:

| Annotation Location  | Form                                     | Description              |
| -------------------- | ---------------------------------------- | ------------------------ |
| Signature only       | `double: (x: Int) -> Int = x * 2`        | ✅ recommended           |
| Lambda head only     | `double = (x: Int) => x * 2`             | ✅ legal                 |
| Both sides annotated | `double: (x: Int) -> Int = (x) => x * 2` | ✅ redundant but allowed |

### Complete Example

```yaoxiang
// ✅ Recommended: signature complete, lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: type annotated in lambda head
double = (x: Int) => x * 2

// ✅ Legal: annotated on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature        | Advantage                                                                       |
| -------------- | ------------------------------------------------------------------------------- |
| **Concise**    | No need to repeat parameter names when signature is complete                    |
| **Flexible**   | Lambda form is preserved, use whichever you prefer                              |
| **Consistent** | Consistent with variable declaration `x: Int = 42`                              |
| **Intuitive**  | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage            | Description                                         |
| -------------------- | --------------------------------------------------- |
| Extreme Unification  | One syntax rule covers all cases                    |
| Theoretical Elegance | Perfectly symmetric `name: type = value`            |
| No New Keywords      | Reuse existing syntactic elements                   |
| Easy to Implement    | Compiler only needs to handle one declaration form  |
| Easy to Learn        | Remembering one pattern is enough to write all code |
| Easy to Extend       | New features can naturally fit into this model      |

### Disadvantages

| Disadvantage      | Description                                                     |
| ----------------- | --------------------------------------------------------------- |
| Naming Convention | Methods need to follow `Type.method` naming                     |
| Verbosity         | Complete syntax is longer than simplified syntax, but inferable |
| Learning Curve    | Need to understand the unified model                            |

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
// IDE automatically hints missing methods
```

### Risks

| Risk                 | Impact                                         | Mitigation                                 |
| -------------------- | ---------------------------------------------- | ------------------------------------------ |
| Parsing Complexity   | Unified syntax may increase parsing complexity | Use recursive descent parser               |
| Performance Overhead | vtable lookup may have additional overhead     | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: The Source of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: This is an **unspeakable** entity!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One gives birth to two, two gives birth to three, three gives birth to the myriad things.  ║
║   The Yi has the Supreme Ultimate, which gives birth to the Two Modes.                  ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.   ║
║   The compiler falls silent here, philosophy pauses here.     ║
║                                                              ║
║   Thank you for reaching the philosophical boundary of the language.  ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would lead to the
> Type0/Type1 universe paradox), but we deliberately keep this "easter egg" — when you try to
> compile it, you will receive a Zen message from the language's founder. This is not only a
> technical boundary, but also a tribute from YaoXiang to type philosophy.

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
       | 'Type'                                    # meta-type

type_field ::= identifier ':' type_expr
             | identifier                           # interface constraint

# Generic parameters: as part of a function type, e.g., (T: Type, R: Type) -> (...)
# No independent BNF rules needed — : Type parameters are ordinary function parameters

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

| Term               | Definition                                                                                      |
| ------------------ | ----------------------------------------------------------------------------------------------- |
| Declaration        | An assignment statement in the form `name: type = value`                                        |
| Record Type        | A `{ ... }` type containing named fields                                                        |
| Interface          | A record type whose fields are all function types                                               |
| Generic Type       | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters                |
| Namespace Function | A function in the form `Type.name`, belonging to the Type namespace. Implies no binding         |
| Method Binding     | `Type.name = func[n]`, binds position n of func as the caller, enabling `obj.name(args)` syntax |
| Generic Function   | A function using the `(T: Type)` syntax, with type parameters as the first parameter group      |
| Meta-Type          | `Type`, the only type hierarchy marker in the language                                          |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← open community discussion and feedback
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
│  design)    │    │  place)     │
└─────────────┘    └─────────────┘
```
