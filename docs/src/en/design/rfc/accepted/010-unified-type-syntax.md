---
title: 'RFC-010: Unified Type Syntax - name: type = value Model'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-14 (Never builtin type implemented, #157 closed)'
issue: '#127'
---

# RFC-010: Unified Type Syntax - name: type = value Model

## Summary

This RFC proposes an extremely minimal unified type syntax model: **everything is
`name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

where `type` can be any type expression and `expression` can be any value expression. **There is no
`fn`, no `struct`, no `trait`, no `impl`, and no lowercase `type` keyword (but there is `Type` as a
meta-type keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that accepts
> type parameter T".

| Concept          | Code Form                                                                    |
| ---------------- | ---------------------------------------------------------------------------- |
| Variable         | `x: Int = 42`                                                                |
| Function         | `add: (a: Int, b: Int) -> Int = a + b`                                       |
| Record type      | `Point: Type = { x: Float, y: Float }`                                       |
| Interface        | `Drawable: Type = { draw: (Surface) -> Void }`                               |
| Generic type     | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                  |
| Generic type     | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`     |
| Method           | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`    |

**`Type` is the only meta-type keyword in the language**.

> **Namespace vs. method binding**: The `Type.name` prefix denotes **namespace ownership**, nothing
> more. It does not trigger any implicit binding. For the `.` call syntax such as `p.draw(screen)`
> to work, an explicit binding is required: `Point.draw = draw[0]`. See the "Namespace and Method
> Binding" section below for details. It is used to mark type-level structure, with the compiler
> automatically handling the distinction of Type0, Type1, Type2... transparently to the user.

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
    "Point(${self.x}, ${self.y})"
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
p.draw(screen)           // syntactic sugar → Point.draw(p, screen)
s: Drawable = p           // structural subtyping: Point implements Drawable
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## Motivation

### Why is this feature needed?

The current type system has several disjoint concepts:

- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack uniformity, leading to fragmented syntax and a high learning cost.

### Design Goals

1. **Maximum uniformity**: One syntactic rule covers all cases
2. **Concise and elegant**: The symmetric aesthetics of `name: type = value`
3. **No new keywords**: Reuse existing syntactic elements
4. **Theoretically elegant**: Types themselves are values of type Type
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 fits the generics system design of RFC-011 **naturally**, and
generic parameters blend into the unified model seamlessly:

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

- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: RFC-011 Phase 1 should be implemented together with RFC-010

## Proposal

### Core Principle: Type Constructors vs. Functions/Variables

**This is a key design choice that determines the disambiguation rules of the syntax:**

| Form                | Meaning              | Rule                                                  |
| ------------------- | -------------------- | ----------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | `: Type` explicit declaration → forced to be a type   |
| **`f = ...`**       | Function or variable | No `: Type` → HM actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax is inherently ambiguous:

- `{ x: Float, y: Float }` could be a **type literal** (record type)
- `{ a = 1 + 1 }` could be a **code block** (executable statement, returning Void)

**Disambiguation rule**:

- **Has** `: Type` → forced to be parsed as a type constructor, `{ ... }` is a type literal
- **No** `: Type` → HM actively parses `{ ... }` as a code block, inferring a function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main: () -> Void = { println("Hello") }

# ❌ Error: no : Type, compiler cannot parse { ... } as a type
Point = { x: Float, y: Float }  // HM infers as a function, not a type!
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
├── Namespace function
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # Explicit binding enables dot call syntax
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Meta-type Hierarchy (Compiler Internal)

**The compiler internally** maintains a universe hierarchy `level: selfpointnum` (stored as a
string, theoretically infinitely extensible).

| Level    | Description                              |
| -------- | ---------------------------------------- |
| `Type0`  | Everyday types (`Int`, `Float`, `Point`) |
| `Type1`  | Type constructors (`List`, `Maybe`)      |
| `Type2+` | Higher-order constructors                |

**Users never see these numbers**, only `: Type`.

### Curry-Howard Isomorphism: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not arbitrarily chosen—it is a direct mapping of
the Curry-Howard correspondence. This correspondence reveals a profound fact: **the type system and
the logic system are two sides of the same thing**.

| Logic (Proposition)                | Type System (YaoXiang)              | Example                              |
| ---------------------------------- | ----------------------------------- | ------------------------------------ |
| Proposition P                      | Type T                              | `Int`, `Bool`                        |
| Proof of P                         | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication)                | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)                | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal quantification) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)                | Enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads: "there exists a proof of type Int, named x, whose value is 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads:
// "there exists an implication proof: given proofs a and b of type Int, one can construct a proof of type Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If the type system allows constructing a value of type `T`
   with no legal runtime representation, it is like allowing a proof of a false proposition in
   logic—the system collapses. Curry-Howard tells us: **a type-safe language is inherently a
   consistent logic system**.

2. **The universe hierarchy is necessary**: As detailed below, if `Type: Type` were allowed (i.e.,
   "the type of types is also a type"), it would produce Russell's paradox (manifested as Girard's
   paradox in type theory). YaoXiang's layering of `Type₀ : Type₁ : Type₂ : ...` ensures that each
   type belongs to only one level, forming a never-closing ascending chain that fundamentally avoids
   paradoxes. This means YaoXiang's type system is **logically consistent** in the Curry-Howard
   sense.

3. **Theoretical basis for unified syntax**: The reason `name: type = value` can use one syntax to
   cover variables, functions, types, interfaces, and generics is that they are all the same thing
   under Curry-Howard—**providing proofs for propositions**. Variables are evidence of propositions,
   functions are evidence of implications, records are evidence of conjunctions, and generics are
   evidence of universal quantifications. Unified syntax is not an arbitrary design coincidence, but
   a natural consequence of the Curry-Howard isomorphism.

> **Further reading**: Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM,
> 58(12), 75–84. This article explains the history and significance of the Curry-Howard isomorphism
> in accessible language.

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

**The value of a block = the tail expression; `return` exits the function (type `Never`)**—see
[RFC-010a](010a-tail-expression-and-return.md) for details.

```yaoxiang
// Single-expression form
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// Code block form: value is the tail expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b                    // tail expression → value of the block
}

// Multi-line code block
calc: (x: Float, y: Float, op: String) -> Float = {
    match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }                    // match as tail expression
}

// Void function: tail expression is Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### Return Rules

**The value of a block = the tail expression (sole exit point)**:

| Form                                            | Value                                  |
| ----------------------------------------------- | -------------------------------------- |
| `= expr` (no braces)                            | `expr`                                 |
| `= { ...; e }` (with braces)                    | tail expression `e`                    |
| `= { ...; s }` (last is a statement/assignment) | `Void` (value of assignment is `Void`) |
| `= {}` (empty block)                            | `Void`                                 |

**Semantics of `return`**: Non-local exit, **exits the nearest function boundary** (does not "return
to the block"), with type `Never`. `Never <: T` holds for any type (principle of explosion), so
`return` can appear in any return type position.

```yaoxiang
# Single expression: directly returns the value
add: (a: Int, b: Int) -> Int = a + b

# Code block: value is the tail expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# Early return: return pierces the block, exits the function (type Never)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # tail expression
}
```

**Design rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its
evaluation semantics differ from a single expression—braces introduce a multi-statement context,
**whose value is given by the tail expression**, with no ambiguity of "whether the last expression
is the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

`{ ... }` in YaoXiang is not just a code block—it is a **dependency-driven computation unit**. This
semantics is consistent in function bodies, variable initialization, and `spawn`:

**Core rules**:

- Assignment statements within `{}` are automatically ordered by dependency, not by written order
- Execution begins immediately when dependencies are ready; otherwise it blocks and waits
- **The value of a block = the tail expression** (see return rules); `return` is a non-local exit of
  type `Never`, exiting the function

```yaoxiang
# Dependency-driven: b depends on a, compiler automatically orders
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependency → can execute first
    b              # tail expression → block value 11
}
```

> **Difference from single expressions**: `= expr` (no braces) is a simple binding that directly
> returns a value; `= { ... }` (with braces) introduces a dependency-driven computation context that
> allows multiple statements, whose value is given by the tail expression.

#### `spawn` Block

`spawn { ... }` is the only parallel primitive in YaoXiang. It leverages the dependency-driven
semantics of `{}` to achieve automatic parallelization:

- Direct child assignments within `spawn { ... }` automatically create parallel tasks
- Tasks with ready dependencies execute concurrently immediately
- The caller blocks until all child tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # task 1
    b = fetch_data("url2")    # task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # depends on a, b → executes after both complete
    c                         # tail expression → value of spawn
}
// caller blocks here until all tasks in the spawn block complete
```

> **Detailed definition**: The complete semantics of `spawn`, task creation rules, and blocking
> model are detailed in `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and operate on raw pointers. It leverages the
evaluation semantics of `{}` to delegate type definitions to the enclosing scope (value exit is the
tail expression):

**Core rules**:

- Types can be defined and raw pointers can be operated within `unsafe {}`
- The **tail expression** gives the value of `unsafe {}` (type definitions are delegated to the
  enclosing scope)
- The returned type is available outside `unsafe {}`
- Field access on such types requires unsafe permission

```yaoxiang
# Define an opaque type within an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw pointer
    }
    SqliteDb           # tail expression → value of the unsafe block
}

# SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: handle field requires unsafe permission
handle = db.handle

# ✅ Through a method call
db.close()
```

> **Detailed definition**: The complete semantics of `unsafe`, FFI type definition, and method
> binding are detailed in `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound
methods, and interface implementation:

##### Basic Types

**Record type**: a list of fields, whose types can be any type expression.

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

**Fields without default values**: must be provided at construction.

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

YaoXiang's identifier system is layered into three tiers, recognized by different compiler phases:

1. **Keywords** (parser-level distinct tokens) — control structures and declaration keywords, such
   as `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser-level distinct tokens) — `true`, `false`, `void`, `Type`;
   cannot be used as ordinary identifiers
3. **Builtin type names** (registered by type checker) — the parser treats them as ordinary
   identifiers, and the type checker is responsible for resolution. **They are not reserved words
   and can be shadowed (not recommended)**

Difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, builtin type
name): `void` is a value literal (equal to the unique value of Unit), while `Void` is a type name
(equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Predefined builtin type names:

| Type     | Logical Equivalent   | Description                                                                                                                                                                                                                                                                    |
| -------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Never`  | ⊥ (false/empty type) | Zero constructor, no value can inhabit this type. Represents "impossible"—divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` indicates it never returns normally. **Not a keyword, but a builtin type name.** |
| `Void`   | ⊤ (true/Unit)        | Has exactly one inhabitant (default `void` value). `x: Void = <default>` is legal. Corresponds to the unit element of the sum type and the unit element of the product type—`Void` is the zero-field product type (Unit), `Never` is the zero-variant sum type.                |
| `Int`    | —                    | Signed integer                                                                                                                                                                                                                                                                 |
| `Float`  | —                    | Floating-point number                                                                                                                                                                                                                                                          |
| `Bool`   | —                    | Boolean: `true` / `false`                                                                                                                                                                                                                                                      |
| `Char`   | —                    | Unicode character                                                                                                                                                                                                                                                              |
| `String` | —                    | String                                                                                                                                                                                                                                                                         |

##### Bound Methods

**Method 1: Directly bind external functions within the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // bind to position 0; after currying, method: (b: Point) -> Float
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
        (dx * dx + dy * dy).sqrt()      # tail expression
    })
}
// Syntax: ((params) => body)[position]
// Call: p1.distance(p2) → distance(p1, p2)
```

##### Interface Implementation

**Interface names are written within the type body, and the compiler automatically checks their
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

// Empty type / empty interface
EmptyType: Type = {}
Empty: Type = {}
```

##### Namespace Function Definition

**The `Type.name` prefix denotes namespace ownership**, nothing more. It does not trigger any
implicit binding.

```yaoxiang
// Namespace function: an ordinary function under the Point namespace
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

// Call: just an ordinary function call
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing `p`, `this`, or `x`
> has exactly the same effect. The compiler does not look at the parameter name, it looks at the
> type.

##### Method Binding (the only way)

For the `.` method call syntax such as `p.draw(screen)` to work, **an explicit binding is
required**. The `[position]` syntax is the only mechanism to bind a function as a "method" (see
RFC-004 for detailed syntax).

```yaoxiang
// Define a function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does p.draw(screen) syntax work
Point.draw = draw[0]   // the parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // both call forms are equivalent

// Without [0] = no binding. Point.draw is just an ordinary function alias, no . syntax
Point.draw = draw       // not bound: only Point.draw(p, screen) is available
```

**Default behavior**: Not writing `[n]` = not binding any parameter. The user must explicitly decide
which parameters are filled by the caller.

**Multi-position binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method back to ordinary function):

```yaoxiang
// Extract a function from a binding
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. Interface Composition

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using the intersection type
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    item.serialize()
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
    (self: List(type), item: type) -> {
        self.data.append(item)
        self.length = self.length + 1
    }
}

IntList.push(Int)(self, item)  // call example

// Generic method (RFC-023 syntax: type parameter inferred automatically at the call site)
List.push: (self: List(T), item: T) -> Void = {
    self.data.append(item)
    self.length = self.length + 1
}

List.get: (self: List(T), index: Int) -> Maybe(T) = {
    if index >= 0 and index < self.length {
        Maybe.Just(self.data[index])
    } else {
        Maybe.Nothing
    }
}
```

#### 6. Generic Call Syntax

Generic types and generic function calls uniformly use the `()` syntax. `[]` is not used in any
generic context.

**Core rules**:

1. **`()` does everything**: type application, function calls, and value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left side
empty: List(Int) = List()

# Generic function call — types flow automatically from parameters
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value`—Type parameters are declared on
   the left side, and the right side is always a concrete value. The `T` of an empty container
   `List()` must be obtained from the left-hand type annotation.

3. **Type information is written only once**—at the parameter declaration, the compiler carries it
   through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically from numbers and f
```

4. **Value construction infers types from elements**:

```yaoxiang
x = List(1, 2, 3)       // inferred as List(Int)
y = List("a", "b")      // inferred as List(String)
z = List()              // ❌ compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int from the left-hand annotation
```

5. **Type aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`,
> `List[Int](1,2,3)` → `List(1,2,3)`. The old `[]` generic syntax has been completely removed. `[]`
> is used only for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========
// Interface = record type whose fields are all function types
// No self parameter is required in an interface — an interface only defines "function signatures with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // returns interface type; concrete impl returns its own type
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

// Define functions (self is just a conventional name, not a keyword)
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

bounding_box: (p: &Point) -> Rect = {
    Rect(p.x - 1, p.y - 1, 2, 2)
}

serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

translate: (p: &Point, dx: Float, dy: Float) -> Point = {
    Point(p.x + dx, p.y + dy)
}

scale: (p: &Point, factor: Float) -> Point = {
    Point(p.x * factor, p.y * factor)
}

distance: (p1: &Point, p2: &Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Explicit binding — only after binding does the dot call syntax work
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
    "Rect(${r.x}, ${r.y}, ${r.width}, ${r.height})"
}
Rect.serialize = serialize[0]

translate: (r: &Rect, dx: Float, dy: Float) -> Rect = {
    Rect(r.x + dx, r.y + dy, r.width, r.height)
}
Rect.translate = translate[0]

scale: (r: &Rect, factor: Float) -> Rect = {
    Rect(r.x * factor, r.y * factor, r.width * factor, r.height * factor)
}
Rect.scale = scale[0]

// ======== 4. Usage ========

// Create instances
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// Method call (syntactic sugar)
p.draw(screen)
r.draw(screen)

// Ordinary method call (direct call)
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

### Interface Checking Algorithm

```rust
fn check_type_implements_interface(
    typ: &Type,
    iface: &Type
) -> Result<(), TypeError> {
    // For each field of the interface (function field)
    for (field_name, iface_field) in &iface.fields {
        // Check if the type has a method with the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check if the method signature is compatible
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

Interface types support direct assignment, and the compiler automatically selects the optimal call
strategy based on the type of the right-hand value:

```yaoxiang
// Direct assignment of a concrete type → compile-time determines the concrete type, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → compile-time cannot determine the concrete type, uses vtable
d: Drawable = get_shape()
d.draw(screen)  // method lookup via vtable

// Heterogeneous collection → uses vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // method lookup via vtable
}
```

**Compile-time optimization strategy**:

| Scenario                         | Inferred Result      | Call Method                 |
| -------------------------------- | -------------------- | --------------------------- |
| `d: Drawable = Circle(1)`        | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()`      | Unknown              | vtable                      |
| `shapes: List(Drawable) = [...]` | Heterogeneous        | vtable                      |

**Rules**:

1. When the right-hand value is a concrete type constructor and can be determined at compile time,
   generate direct call IR
2. When the right-hand value type cannot be determined at compile time, fall back to the vtable
   mechanism
3. The vtable fallback ensures correctness of runtime polymorphism

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

| Before                                   | After                                                                                        |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)`    | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| Requires the `impl` keyword              | No keyword needed; interface name is written after the type body                             |

### Deprecated: `|` Variant Syntax

> **Deprecation notice (2026-07-25)**: The `|` variant syntax is officially deprecated and removed
> from the implementation.

The following forms are **no longer supported**:

```
type Color = red | green | blue                # ❌ deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ deprecated
type Option(T) = some(T) | none                # ❌ deprecated
```

Record types are uniformly used to express sum types. When all fields of a record type are functions
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

**Design rationale**:

1. **Eliminate special cases**: `|` is the only non-`name: type = value` form in the BNF. After its
   removal, the `type_expr` production is completely unified, and the parser no longer needs to
   maintain a separate path with lookahead fallback for variant types.
2. **Mathematical equivalence**: Under the Curry-Howard isomorphism, the disjunction P ⊕ Q
   corresponds to a sum type, which is equivalent to a record type "whose fields are all functions
   returning the type itself". They express the same semantics, so two syntactic forms are
   unnecessary.
3. **Zero destructiveness**: Before removal, the `|` syntax was only half-supported in the parser
   (parameterless variants could be parsed but parameter types were lost during monomorphization),
   and no user code depended on it.
4. **AST simplification**: The `Type::Variant(Vec<VariantDef>)` node is deleted, and all variant
   types uniformly go through the `Type::Struct` path; the special branches in downstream
   typecheck/mono/formatter are all eliminated.

> **Note**: The semantic properties of sum types (such as match exhaustiveness checking and tagged
> union memory layout) are inferred by the typecheck layer from the `Type::Struct` structure, not
> relying on independent AST nodes.

### Logical Operators: `and` / `or` / `!` (Authoritative Definition, Zig-style)

> **Definition notice (2026-08-03)**: The authoritative form of logical operators is the keywords
> `and` / `or` plus the symbolic unary `!` (consistent with SPEC `syntax.md` §2.2 precedence table).
> This design aligns with Zig: **short-circuit control flow uses keywords, pure unary operations use
> symbols**. The early C-drifting implementation of `&&` / `||` and the intermediate keyword `not`
> have all been removed.

**Semantics**:

| Operator | Precedence (SPEC §2.2)          | Associativity | Semantics                                    |
| -------- | ------------------------------- | ------------- | -------------------------------------------- |
| `!`      | 3 (unary prefix, tight binding) | right-to-left | logical NOT (pure function, no control flow) |
| `and`    | 10                              | left-to-right | short-circuit logical AND                    |
| `or`     | 10                              | left-to-right | short-circuit logical OR                     |

```yaoxiang
# Short-circuit evaluation: when left of and is false / left of or is true, the right is not executed
if x != 0 and y / x > 1 { ... }   # when x == 0, no division by zero

# Tight binding: !a == b ≡ (!a) == b (Zig-style; opposite to Python's not a == b ≡ not (a == b))
!3 == 4          # false: (!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs)), invert after call
```

The following forms are **no longer supported** (lexer reports an error with a hint to the
corresponding form):

```
x && y     # ❌ removed, use x and y
x || y     # ❌ removed, use x or y
not x      # ❌ removed, use !x (not reverts to a regular identifier; != is unaffected)
```

**Design rationale** (aligned with Zig, ziglang/zig#272 / #6625):

1. **Short-circuit is control flow → keyword; pure function is operation → symbol**. `and` / `or`
   change the order of evaluation (the right side is skipped on demand), which is of the same nature
   as `if`, so they use keywords; `!` performs pure negation on an already-evaluated operand, which
   is of the same nature as `-` `+`, so it uses a symbol. YaoXiang uses `?` for error propagation
   (§2.11), so there is no conflict for `!`.
2. **Tight binding eliminates ambiguity**: `!` visually "sticks" to the operand, making high
   precedence immediately clear; the keyword `not` is forced to leave a space with the operand,
   making it easy to cause mental ambiguity about which side it binds to (`not a == b`).
3. **Disambiguation**: `&` already plays two roles—the borrow token (`&p` / `&mut p`, RFC-009) and
   bitwise AND (SPEC §2.2 precedence 8). Introducing `&&` would make one symbol carry three
   meanings. `and` / `or` / `!` visually separate borrow, bitwise operation, and logic into three
   distinct concepts.
4. **Precedent**: Zig (a modern system language in the same ecosystem niche) is exactly the
   combination of `and` / `or` keywords plus `!` symbol; Python / Lua / Ada / SQL use all keywords
   (including `not`), the C family uses all symbols—YaoXiang takes Zig's hybrid, getting the best of
   both.
5. **Curry-Howard consistency**: Types are propositions (see the isomorphism section above), and the
   logical connectives in refined types are written as `and` / `or` (e.g.,
   `{ 0 <= idx and idx < arr.len }`), which is the natural expression of propositions; `!` as a
   unary negation symbol corresponds to ¬.

> **Implementation**: `and` / `or` are expanded at the IR level into short-circuit jump sequences
> (`a and b ≡ if a { b } else { false }`); `!` is parsed with unary tight binding (operand is taken
> at `BP_UNARY + 1`). Regression tests: `tests/yaoxiang/01-syntax/basics/logical_ops.yx`,
> `logical_not.yx`.

## Syntax Design Note: Named Functions Are Essentially Syntactic Sugar for Lambdas

### Core Understanding

**Named functions and lambda expressions are the same thing!** The only difference is that a named
function gives a lambda a name.

```yaoxiang
// These two are essentially identical
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

**Key point**: When the signature fully declares the parameter types, the parameter names in the
lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters shadow outer variables**: the parameter scope in the signature shadows outer-scope
variables, and the inner scope has higher priority.

```yaoxiang
x = 10  // outer-scope variable

double: (x: Int) -> Int = x * 2  // ✅ parameter x shadows outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be placed in any of the following positions, **at least one annotation is
required**:

| Annotation Position | Form                                     | Description              |
| ------------------- | ---------------------------------------- | ------------------------ |
| Signature only      | `double: (x: Int) -> Int = x * 2`        | ✅ recommended           |
| Lambda head only    | `double = (x: Int) => x * 2`             | ✅ legal                 |
| Both sides          | `double: (x: Int) -> Int = (x) => x * 2` | ✅ redundant but allowed |

### Complete Examples

```yaoxiang
// ✅ Recommended: signature complete, lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: types annotated in the lambda head
double = (x: Int) => x * 2

// ✅ Legal: both sides annotated
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature        | Advantage                                                                       |
| -------------- | ------------------------------------------------------------------------------- |
| **Concise**    | No need to repeat parameter names when the signature is complete                |
| **Flexible**   | Lambda form is preserved; use whichever you prefer                              |
| **Consistent** | Maintains the unified pattern with variable declaration `x: Int = 42`           |
| **Intuitive**  | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage             | Description                                            |
| --------------------- | ------------------------------------------------------ |
| Maximum uniformity    | One syntactic rule covers all cases                    |
| Theoretically elegant | Perfectly symmetric `name: type = value`               |
| No new keywords       | Reuse existing syntactic elements                      |
| Easy to implement     | The compiler only needs to handle one declaration form |
| Easy to learn         | Remember one pattern and you can write all code        |
| Easy to extend        | New features can naturally fit into this model         |

### Disadvantages

| Disadvantage      | Description                                                             |
| ----------------- | ----------------------------------------------------------------------- |
| Naming convention | Methods must follow the `Type.method` naming convention                 |
| Verbosity         | The full syntax is longer than a simplified syntax, but can be inferred |
| Learning curve    | Need to understand the unified model                                    |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Example of compile error:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Types can be omitted and inferred by the compiler
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE automatically suggests missing methods
```

### Risks

| Risk                 | Impact                                         | Mitigation                                 |
| -------------------- | ---------------------------------------------- | ------------------------------------------ |
| Parsing complexity   | Unified syntax may increase parsing complexity | Use a recursive descent parser             |
| Performance overhead | vtable lookup may have extra overhead          | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: The Source of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: This is **unspeakable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One begets two, two beget three, three beget all things.   ║
║   The Yi has the Supreme Ultimate, which begets the Two Forms.║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.  ║
║   The compiler is silent here, philosophy pauses here.       ║
║                                                              ║
║   Thank you for reaching the philosophical boundary of the language.║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause the Type0/Type1
> universe paradox), but we deliberately keep this "easter egg"—when you try to compile it, you will
> receive a zen message from the language's founder. This is not only a technical boundary, but also
> a tribute from YaoXiang to type philosophy.

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

# Generic parameter: as part of a function type, e.g., (T: Type, R: Type) -> (...)
# No independent BNF rule needed — the : Type parameter is just a normal function parameter

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

| Term               | Definition                                                                                                |
| ------------------ | --------------------------------------------------------------------------------------------------------- |
| Declaration        | An assignment statement of the form `name: type = value`                                                  |
| Record type        | A `{ ... }` type containing named fields                                                                  |
| Interface          | A record type whose fields are all function types                                                         |
| Generic type       | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters                          |
| Namespace function | A function of the form `Type.name`, belonging to the Type namespace; implies no binding                   |
| Method binding     | `Type.name = func[n]`, binding position n of func as the caller, making `obj.name(args)` syntax available |
| Generic function   | A function using the `(T: Type)` syntax, with type parameters as the first parameter group                |
| Meta-type          | `Type`, the only type-level marker in the language                                                        |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review │  ← open for community discussion and feedback
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
│ (formal design) │  │ (kept in place) │
└─────────────┘    └──────┬──────┘
                           │
                           ▼
                      (rejected)
```
