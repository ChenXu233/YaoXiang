---
title: 'RFC-010: Unified Type Syntax - name: type = value Model'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-09-25'
issue: '#127'
---

# RFC-010: Unified Type Syntax - name: type = value Model

## Summary

This RFC proposes a minimalist unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression. **No `fn`, no
`struct`, no `trait`, no `impl`, no lowercase `type` keyword (but `Type` exists as the meta-type
keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` represents "a type that
> accepts a type parameter T".

| Concept          | Code                                                                         |
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

> **Namespace vs. method binding**: The `Type.name` prefix denotes **namespace membership**, nothing
> more. It does not trigger any implicit binding. To enable `.` call syntax like `p.draw(screen)`,
> you must explicitly bind: `Point.draw = draw[0]`. See the "Namespace and Method Binding" section
> below. It is used to label type levels, and the compiler automatically handles the distinction of
> Type0, Type1, Type2..., which is transparent to users.

```yaoxiang
// Core syntax: unified + distinguished

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

1. **Extreme unification**: One syntactic rule covers all cases
2. **Concise and elegant**: The symmetric aesthetics of `name: type = value`
3. **No new keywords**: Reuse existing syntactic elements
4. **Theoretically elegant**: Types themselves are values of type Type
5. **Generics-friendly**: Seamlessly integrates with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 and the generics system design of RFC-011 are **naturally
compatible**, and generic parameters can be seamlessly integrated into the unified model:

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
- Recommendation: Implement RFC-011 Phase 1 in sync with RFC-010

## Proposal

### Core Principle: Type Constructor vs. Function/Variable

**This is a key design choice that determines the disambiguation rules for the syntax:**

| Syntax              | Meaning              | Rule                                                |
| ------------------- | -------------------- | --------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | `: Type` explicit declaration → forced to be a type |
| **`f = ...`**       | Function or variable | No `: Type` → HM actively infers function/variable  |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:

- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executed statement, returns Void)

**Disambiguation rules**:

- **Has** `: Type` → forced to parse as type constructor, `{ ... }` is a type literal
- **Does not have** `: Type` → HM actively parses `{ ... }` as a code block, infers function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers () -> Void
main: () -> Void = { println("Hello") }

# ❌ Error: no : Type, compiler cannot parse { ... } as a type
Point = { x: Float, y: Float }  // HM infers as function, not type!
```

---

**Unified model: identifier : type = expression**

```
├── Variable
│   └── x: Int = 42
│
├── Function
│   └── add: (a: Int, b: Int) -> Int = a + b  # No : Type, HM infers function
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
│       Point.draw = draw[0]  # Explicit binding enables dot call syntax
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers function
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

YaoXiang's unified syntax `name: type = value` is not an arbitrary choice—it is a direct mapping of
the Curry-Howard correspondence. This correspondence reveals a profound fact: **the type system and
the logical system are two sides of the same coin**.

| Logic (proposition)  | Type system (YaoXiang)              | Example                              |
| -------------------- | ----------------------------------- | ------------------------------------ |
| Proposition P        | Type T                              | `Int`, `Bool`                        |
| Proof that P is true | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication)  | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)  | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal)  | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)  | Enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**Meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "there exists a proof of type Int, named x, with value 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "there exists an implication proof: given proofs a and b of Int, one can construct a proof of Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If the type system allows constructing a value of type `T`
   without any legal runtime representation, that is like allowing a proof of a false proposition in
   logic—the system crashes. Curry-Howard tells us: **a type-safe language is naturally a consistent
   logical system**.

2. **Universe hierarchy is a necessary condition**: As detailed below, allowing `Type: Type` (i.e.,
   "the type of types is also a type") would produce the Russell paradox (manifested as Girard's
   paradox in type theory). YaoXiang's stratification `Type₀ : Type₁ : Type₂ : ...` ensures that
   each type belongs to only one level, forming an ever-rising chain that never closes,
   fundamentally avoiding paradoxes. This means that YaoXiang's type system is **logically
   consistent** in the Curry-Howard sense.

3. **Theoretical foundation of unified syntax**: The reason `name: type = value` can cover
   variables, functions, types, interfaces, and generics with one syntax is precisely because they
   are all the same thing under Curry-Howard—**providing proofs for propositions**. Variables are
   evidence of propositions, functions are evidence of implications, records are evidence of
   conjunctions, generics are evidence of universal quantifications. Unified syntax is not an
   arbitrary design coincidence, but a natural corollary of the Curry-Howard isomorphism.

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

**The value of a block is the trailing expression, `return` exits the function (type `Never`)**—see
[RFC-010a](010a-tail-expression-and-return.md) for details.

```yaoxiang
// Single expression form
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// Code block form: the value is the trailing expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b                    // trailing expression → value of the block
}

// Multi-line code block
calc: (x: Float, y: Float, op: String) -> Float = {
    match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }                    // match as trailing expression
}

// Void function: trailing expression is Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### Return Rules

**The value of a block = trailing expression (the sole exit)**:

| Syntax                                          | Value                                      |
| ----------------------------------------------- | ------------------------------------------ |
| `= expr` (no braces)                            | `expr`                                     |
| `= { ...; e }` (with braces)                    | trailing expression `e`                    |
| `= { ...; s }` (ending in statement/assignment) | `Void` (the value of assignment is `Void`) |
| `= {}` (empty block)                            | `Void`                                     |

**Semantics of `return`**: non-local exit, **exits the nearest function boundary** (does not "return
to the block"), type is `Never`. `Never <: T` holds for any type (principle of explosion), so
`return` can appear in any position with any return type.

```yaoxiang
# Single expression: directly returns the value
add: (a: Int, b: Int) -> Int = a + b

# Code block: value is the trailing expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# Early return: return passes through the block, exits the function (type Never)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # trailing expression
}
```

**Design rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its
evaluation semantics differ from a single expression—braces introduce a multi-statement context,
**whose value is given by the trailing expression**, eliminating the ambiguity of "is the last
expression the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

In YaoXiang, `{ ... }` is not merely a code block—it is a **dependency-driven computation unit**.
This semantics is consistent across function bodies, variable initializations, and `spawn`:

**Core rules**:

- Assignment statements within `{}` are automatically ordered by dependency rather than by writing
  order
- Once dependencies are satisfied, execution happens immediately; if missing, it blocks and waits
- **The value of the block = trailing expression** (see return rules); `return` is a non-local exit
  of type `Never`, exiting the function

```yaoxiang
# Dependency-driven: b depends on a, compiler automatically orders
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependencies → can execute first
    b              # trailing expression → value of block is 11
}
```

> **Difference from a single expression**: `= expr` (no braces) is a simple binding that directly
> returns the value; `= { ... }` (with braces) introduces a dependency-driven computation context,
> allowing multiple statements, whose value is given by the trailing expression.

#### `spawn` Block

`spawn { ... }` is YaoXiang's sole parallelism primitive. It leverages the dependency-driven
semantics of `{}` to achieve automatic parallelization:

- Direct child assignments within `spawn { ... }` automatically create parallel tasks
- Tasks with satisfied dependencies execute concurrently immediately
- The caller blocks until all child tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # task 1
    b = fetch_data("url2")    # task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # depends on a, b → waits for both to complete
    c                         # trailing expression → value of spawn
}
// caller blocks here until all tasks within the spawn block complete
```

> **Detailed definition**: The complete semantics of `spawn`, task creation rules, and blocking
> model are detailed in `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and operate on raw pointers. It leverages the
evaluation semantics of `{}` to hand over the type definition to the outer scope (the value exit is
the trailing expression):

**Core rules**:

- Types can be defined and raw pointers operated on within `unsafe {}`
- The **trailing expression** gives the value of `unsafe {}` (type definitions are handed over to
  the outer scope)
- Returned types are available outside the `unsafe {}` block
- Field access of types requires unsafe permission

```yaoxiang
# Define an opaque type within an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw pointer
    }
    SqliteDb           # trailing expression → value of unsafe block
}

# SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: the handle field requires unsafe permission
handle = db.handle

# ✅ Access via method call
db.close()
```

> **Detailed definition**: The complete semantics of `unsafe`, FFI type definition, and method
> binding are detailed in `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, encompassing fields, default values, bound
methods, and interface implementation:

##### Basic Types

**Record type**: a list of fields, where field types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: fields can have default values, optional at construction.

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

##### Built-in Types

YaoXiang's identifier system is divided into three layers, recognized by different compiler phases:

1. **Keywords** (parser independent tokens) — control structures and declaration keywords, such as
   `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser independent tokens) — `true`, `false`, `void`, `Type`, cannot
   be used as ordinary identifiers
3. **Built-in type names** (registered by the type checker) — the parser treats them as ordinary
   identifiers, and the type checker is responsible for resolving them. **Not reserved words, can be
   shadowed (not recommended)**

The distinction between `void` (lowercase, literal reserved word) and `Void` (uppercase, built-in
type name): `void` is a value literal (equal to the sole value of Unit), and `Void` is a type name
(equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Predefined built-in type names:

| Type     | Logical Correspondence | Description                                                                                                                                                                                                                                                                      |
| -------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no value can inhabit this type. Represents "impossible"—divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` indicates it never returns normally. **Not a keyword, but a built-in type name.** |
| `Void`   | ⊤ (true/Unit)          | Has exactly one inhabitant (the default `void` value). `x: Void = <default>` is legal. The unit of sums and the unit of products—`Void` is the zero-field product type (Unit), `Never` is the zero-variant sum type.                                                             |
| `Int`    | —                      | Signed integer                                                                                                                                                                                                                                                                   |
| `Float`  | —                      | Floating point number                                                                                                                                                                                                                                                            |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                                                                                                                                                                                                                  |
| `Char`   | —                      | Unicode character                                                                                                                                                                                                                                                                |
| `String` | —                      | String                                                                                                                                                                                                                                                                           |

##### Bound Methods

**Method 1: Bind external functions directly within the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // bound to position 0, after currying method: (b: Point) -> Float
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
        (dx * dx + dy * dy).sqrt()      # trailing expression
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

**Interface = record type whose fields are all functions**

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

**The `Type.name` prefix denotes namespace membership**, nothing more. It does not trigger any
implicit binding.

```yaoxiang
// Namespace function: ordinary function under the Point namespace
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

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing it as `p`, `this`,
> `x` is completely equivalent. The compiler does not look at the parameter name, it looks at the
> type.

##### Method Binding (the only way)

To enable `.` method call syntax like `p.draw(screen)`, **you must explicitly bind**. The
`[position]` syntax is the only mechanism for binding a function as a "method" (see RFC-004 for
detailed syntax).

```yaoxiang
// Define function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does p.draw(screen) syntax become available
Point.draw = draw[0]   // The parameter at position 0 (&Point) is filled in by the caller

// Usage
p.draw(screen)          // syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // both call forms are equivalent

// Not writing [0] = not bound. Point.draw is just an ordinary function alias, no . syntax
Point.draw = draw       // not bound: only Point.draw(p, screen) works
```

**Default behavior**: not writing `[n]` = do not bind any parameter. Users must explicitly decide
which parameters are filled in by the caller.

**Multi-position binding**:

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

// Using intersection type
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

// Generic method (RFC-023 syntax: type parameters inferred at call site)
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

Generic type and generic function calls uniformly use the `()` syntax. `[]` is not used in any
generic context.

**Core rules**:

1. **`()` does everything**: type application, function calls, and value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left side
empty: List(Int) = List()

# Generic function call — types flow automatically from arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value` — type parameters are declared on
   the left, the right side is always concrete values. The `T` of an empty container `List()` must
   be obtained from the type annotation on the left.

3. **Type information only needs to be written once** — at parameter declaration, the compiler
   carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically from numbers and f's types
```

4. **Value construction infers type from elements**:

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

> **Comparison with old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`,
> `List[Int](1,2,3)` → `List(1,2,3)`. The old `[]` generic syntax has been completely removed. `[]`
> is only used for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========
// Interface = record type whose fields are all function types
// Interfaces do not need the self parameter — interfaces only define "function signatures with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // returns interface type, specific implementations return their own type
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

// ======== 3. Method Implementation (ordinary functions + explicit binding) ========

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

// Ordinary function call (direct call)
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
        // Check whether the type has a method of the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check whether the method signature is compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Comparison: after removing the self parameter, they should match
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

### Direct Interface Assignment and Compile-time Optimization

Interface types support direct assignment, and the compiler automatically selects the optimal call
strategy based on the type of the right-hand side of the assignment:

```yaoxiang
// Direct assignment of concrete type → compile-time can determine concrete type, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // after compilation: direct call to circle_draw(screen), no vtable

// Function return value → compile-time cannot determine concrete type, uses vtable
d: Drawable = get_shape()
d.draw(screen)  // look up method via vtable

// Heterogeneous collection → uses vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // look up method via vtable
}
```

**Compile-time optimization strategies**:

| Scenario                         | Inference result     | Call method                 |
| -------------------------------- | -------------------- | --------------------------- |
| `d: Drawable = Circle(1)`        | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()`      | Unknown              | vtable                      |
| `shapes: List(Drawable) = [...]` | Heterogeneous        | vtable                      |

**Rules**:

1. When the right-hand side is a concrete type constructor and can be determined at compile time,
   generate direct call IR
2. When the right-hand side type cannot be determined at compile time, fall back to the vtable
   mechanism
3. vtable fallback guarantees the correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as it has the same method, it can be assigned to the interface type
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
| Requires `impl` keyword                  | No keyword needed, interface names written after the type body                               |

### Deprecated: `|` Variant Syntax

> **Deprecation announcement (2026-07-25)**: The `|` variant syntax is officially deprecated and
> removed from the implementation.

The following forms are **no longer supported**:

```
type Color = red | green | blue                # ❌ deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ deprecated
type Option(T) = some(T) | none                # ❌ deprecated
```

Unified use of record types to express sum types. When all fields of a record type are functions,
and they all return the type itself, it is a sum type:

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

1. **Eliminate special cases**: `|` is the only non-`name: type = value` form in the BNF. After
   removal, the `type_expr` production is completely unified, and the parser no longer needs to
   maintain independent paths and lookahead fallback for variant types.
2. **Mathematical equivalence**: Under the Curry-Howard isomorphism, the sum type corresponding to
   disjunction P ⊕ Q is equivalent to a record type "whose fields are all functions returning the
   type itself". Both express the same semantics, no need for two syntaxes.
3. **Zero destructiveness**: Before removal, the `|` syntax was only half-supported in the parser
   (parameterless variants could be parsed, but parameter types were lost during monomorphization),
   with no user code depending on it.
4. **AST simplification**: The `Type::Variant(Vec<VariantDef>)` node is deleted, all variant types
   uniformly go through the `Type::Struct` path, and special branches in downstream
   typecheck/mono/formatter are all eliminated.

> **Note**: The semantic properties of sum types (variant construction, match exhaustiveness check,
> tagged union memory layout) are derived by the typecheck layer from the `Type::Struct` structure,
> without depending on independent AST nodes. The complete semantics of variant construction are
> described in the next section [Record-style Sum Type Variant Construction (Authoritative
> Definition)].

### Record-style Sum Type Variant Construction (Authoritative Definition)

> This section expands the recognition criteria from [Deprecated: `|` Variant Syntax] into complete
> semantics (finalized 2026-09-25, prerequisite for issue #341 RFC-011b Phase 2). Four decisions:
> type-qualified calls, zero-payload variants in function call form, type-self-sufficient inference,
> all-or-nothing variant name promotion.

#### Decision Rule: All or Nothing

A record type is determined to be a **sum type** when the following two conditions are met:

1. All field types in the type body are function types;
2. The return type of each field, after type parameter substitution (`Self`/generic arguments), is
   the record type itself.

The decision is **all-or-nothing**: when the decision holds, all function fields are simultaneously
promoted to variant constructors; when not satisfied, the entire type is an ordinary record
(function fields are data fields that store function values), and there is no mixed form of "partial
variants, partial data". When it is necessary to carry both data and variant semantics, model as two
types: a data record and a sum type, without merging declarations.

#### Call Form: Type-qualified, No Bare Name

The **only** call form for a variant constructor is the type-qualified call—accessing the variant
member on the type value and calling it:

```yaoxiang
r1 = Result(Int, String).ok(5)   // Result(Int, String), payload 5
c  = Color.red()                 // zero-payload variant: function call form consistent with the field signature
```

**The bare-name form is not provided** (`ok(5)` is not a constructor call). Rationale: the bare name
relies on the expected type context to determine which sum type and type arguments it belongs to,
whereas types in this language must be explicitly writable, and inference flows unidirectionally
from expected positions (same discipline as RFC-011a "Self is an explicit type parameter, no
magic"). Under the qualified-name form, the type is completely self-sufficient and requires no
context. If a bare name is introduced in the future, it can only be as an expansion sugar "when the
context is uniquely determinable", and the semantic baseline remains the qualified name in this
section.

#### Inference: Type Self-sufficiency

After the qualified name provides complete type arguments, the signature of the variant constructor
is **the result of field signature substitution with type arguments**:

```yaoxiang
Result(Int, String).ok   // : (Int) -> Result(Int, String)
```

Payload type checking is ordinary function call argument checking (mismatch reports E1002), with no
new inference rules. Constructors of generic sum types are naturally monomorphized with type
instantiation, without separate mechanisms.

#### Field Promotion: Variant Names Are Not Data Fields

After being determined as a sum type, variant names are removed from the data field space—field
access to variant names of sum type **values** is rejected at compile time:

```yaoxiang
r1.ok    // compile error: ok is a variant constructor of Result, not a data field
```

Rationale: runtime sum type values are tagged unions (see below) and do not carry a variant
declaration table; allowing field access would inevitably silently mistranslate. This is an
extension of the same discipline as RFC-011a §1.2 (unified field/method namespace, conflict error
reporting). Construction uses type qualification (`Result.ok`), and extraction uses match
destructuring (RFC-010b).

#### Runtime Representation and Equality

The runtime representation of a sum type value is a tagged union:
`Enum { type identity, variant_id, payload }`.

- **Type identity** carries the concrete sum type (not a global placeholder)—values across sum types
  are not comparable;
- **variant_id** is numbered according to the **declaration order** of variants in the type body,
  and this order is simultaneously the variant set input for RFC-010b's exhaustiveness check;
- **Equality** (`==`/`!=`): same type identity, same variant_id, and payloads are equal value by
  value (recursive).

#### std Migration

`std.result` switches to defining `ok` / `err` using the mechanism in this section (native
`result_ok` / `result_err` are retired), and the std's variant construction and user sum types go
through the same set of rules, with no privileged channels. The parser special case for `Result` /
`Option` in type positions and the dismantling of `make_result` name-based special cases are
undertaken by [RFC-011b](./011b-operator-overloading.md) Phase 2—after the `?` interfaceization,
`Result` becomes an ordinary std sum type.

### Logical Operators: `and` / `or` / `!` (Authoritative Definition, Zig-style)

> **Definition announcement (2026-08-03)**: The authoritative form of logical operators is the
> keywords `and` / `or` + the symbolic unary `!` (consistent with SPEC `syntax.md` §2.2 priority
> table). This design aligns with Zig: **short-circuit control flow uses keywords, pure unary
> operations use symbols**. Early implementations that drifted toward C with `&&` / `||` and the
> intermediate-state keyword `not` have all been removed.

**Semantics**:

| Operator | Priority (SPEC §2.2)            | Associativity | Semantics                                    |
| -------- | ------------------------------- | ------------- | -------------------------------------------- |
| `!`      | 3 (unary prefix, tight binding) | right-to-left | logical NOT (pure function, no control flow) |
| `and`    | 10                              | left-to-right | short-circuit logical AND                    |
| `or`     | 10                              | left-to-right | short-circuit logical OR                     |

```yaoxiang
# Short-circuit evaluation: when the left side of and is false / left side of or is true, the right side is not executed
if x != 0 and y / x > 1 { ... }   # when x == 0, no division by zero occurs

# Tight binding: !a == b ≡ (!a) == b (Zig-style; opposite to Python's not a == b ≡ not (a == b))
!3 == 4          # false: (!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs)), invert after call
```

The following forms are **no longer supported** (lexer reports error and suggests the corresponding
form):

```
x && y     # ❌ removed, use x and y
x || y     # ❌ removed, use x or y
not x      # ❌ removed, use !x (not reverts to ordinary identifier; != is not affected)
```

**Design rationale** (aligned with Zig, ziglang/zig#272 / #6625):

1. **Short-circuit is control flow → keyword; pure function is operation → symbol**. `and` / `or`
   change the evaluation order (the right side is skipped as needed), of the same nature as `if`,
   and use keywords; `!` performs pure inversion on an already-evaluated operand, of the same nature
   as `-` `+`, and uses a symbol. YaoXiang uses `?` for error propagation (§2.11), so `!` has no
   conflict.
2. **Tight binding eliminates ambiguity**: `!` is visually "close" to the operand, and the high
   priority is immediately obvious; the keyword `not` is forced to have a space with the operand,
   and where it binds (`not a == b`) is prone to mental ambiguity.
3. **Disambiguation**: `&` already serves two roles—borrow token (`&p` / `&mut p`, RFC-009) and
   bitwise AND (SPEC §2.2 priority 8). Adding `&&` would let one symbol carry three meanings. `and`
   / `or` / `!` visually completely separates the three concepts of borrow, bitwise operation, and
   logic.
4. **Precedent**: Zig (a modern system language in the same ecological niche) is precisely the
   combination of `and` / `or` keywords + `!` symbol; Python / Lua / Ada / SQL use all keywords
   (including `not`), and the C family uses all symbols—YaoXiang takes Zig's mix, getting the best
   of both.
5. **Curry-Howard consistency**: types are propositions (see the isomorphism section above), and
   logical connectives in refinement types are written as `and` / `or` (such as
   `{ 0 <= idx and idx < arr.len }`), which is a natural expression of propositions; `!` as the
   unary negation symbol corresponds to ¬.

> **Implementation**: `and` / `or` expand in the IR layer into short-circuit jump sequences
> (`a and b ≡ if a { b } else { false }`), and `!` is parsed as a unary tight binding (the operand
> follows `BP_UNARY + 1`). Regression tests: `tests/yaoxiang/01-syntax/basics/logical_ops.yx`,
> `logical_not.yx`.

## Syntax Design Note: Named Functions are Essentially Syntactic Sugar for Lambdas

### Core Understanding

**Named functions and lambda expressions are the same thing!** The only difference is that named
functions give a lambda a name.

```yaoxiang
// These two are essentially completely identical
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

**Key point**: when the signature fully declares the parameter types, the parameter names in the
lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: the parameter scope in the signature overrides the function
body, with the inner scope taking higher priority.

```yaoxiang
x = 10  // outer variable

double: (x: Int) -> Int = x * 2  // ✅ parameter x overrides outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be in any of the following positions, **at least one annotation is required**:

| Annotation position  | Form                                     | Description              |
| -------------------- | ---------------------------------------- | ------------------------ |
| Signature only       | `double: (x: Int) -> Int = x * 2`        | ✅ recommended           |
| Lambda head only     | `double = (x: Int) => x * 2`             | ✅ legal                 |
| Both sides annotated | `double: (x: Int) -> Int = (x) => x * 2` | ✅ redundant but allowed |

### Complete Example

```yaoxiang
// ✅ Recommended: complete signature, lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: type annotated in lambda head
double = (x: Int) => x * 2

// ✅ Legal: annotated on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature        | Advantage                                                                         |
| -------------- | --------------------------------------------------------------------------------- |
| **Concise**    | No need to repeat parameter names when the signature is complete                  |
| **Flexible**   | Retain the lambda form, use whichever you like                                    |
| **Consistent** | Maintains a unified pattern with variable declaration `x: Int = 42`               |
| **Intuitive**  | `name: Type = body` directly corresponds to "named name, typed Type, valued body" |

## Trade-offs

### Advantages

| Advantage            | Description                                        |
| -------------------- | -------------------------------------------------- |
| Extreme unification  | One syntactic rule covers all cases                |
| Theoretical elegance | Perfectly symmetric `name: type = value`           |
| No new keywords      | Reuse existing syntactic elements                  |
| Easy to implement    | Compiler only needs to handle one declaration form |
| Easy to learn        | Remember one pattern to write all code             |
| Easy to extend       | New features can naturally fit into this model     |

### Disadvantages

| Disadvantage      | Description                                                                   |
| ----------------- | ----------------------------------------------------------------------------- |
| Naming convention | Methods need to follow the `Type.method` naming                               |
| Verbose           | The complete syntax is longer than the simplified syntax, but can be inferred |
| Learning curve    | Need to understand the unified model                                          |

### Mitigation Measures

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

| Risk                 | Impact                                         | Mitigation                                 |
| -------------------- | ---------------------------------------------- | ------------------------------------------ |
| Parsing complexity   | Unified syntax may increase parsing complexity | Use recursive descent parser               |
| Performance overhead | vtable lookup may have additional overhead     | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: The Source of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: this is an **unspeakable** thing!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One gives birth to two, two gives birth to three, three gives birth to all things. ║
║   Change has the Supreme Ultimate, which gives birth to the Two Forms.               ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of the language.  ║
║   The compiler falls silent here, philosophy pauses here.    ║
║                                                              ║
║   Thank you for reaching the philosophical boundary of the language. ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause a Type0/Type1
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

# Generic parameters: as part of the function type, e.g. (T: Type, R: Type) -> (...)
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

| Term               | Definition                                                                                                    |
| ------------------ | ------------------------------------------------------------------------------------------------------------- |
| Declaration        | Assignment statement in the form `name: type = value`                                                         |
| Record type        | `{ ... }` type containing named fields                                                                        |
| Interface          | Record type whose fields are all function types                                                               |
| Generic type       | Type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters                                |
| Namespace function | Function in the form `Type.name`, belonging to the Type namespace. Does not imply any binding                 |
| Method binding     | `Type.name = func[n]`, binding position n of func as the caller, making the `obj.name(args)` syntax available |
| Generic function   | Function using the `(T: Type)` syntax, with type parameters as the first parameter group                      |
| Meta-type          | `Type`, the only type hierarchy marker in the language                                                        |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review │  ← open community discussion and feedback
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
│ (formal design)│  │ (kept in place) │
└─────────────┘    └─────────────┘
```
