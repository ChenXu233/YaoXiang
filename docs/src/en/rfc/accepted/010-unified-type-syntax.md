---
title: 'RFC-010: Unified Type Syntax - The name: type = value Model'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-14'
updated: '2026-09-25'
issue: '#127'
---

# RFC-010: Unified Type Syntax - The name: type = value Model

## Summary

This RFC proposes an extremely minimal and unified type syntax model: **everything is
`name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

where `type` can be any type expression and `expression` can be any value expression. **No `fn`, no
`struct`, no `trait`, no `impl`, no lowercase `type` keyword (but `Type` exists as the meta type
keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that accepts a
> type parameter T".

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

**`Type` is the only meta type keyword in the language**.

> **Namespace vs method binding**: The `Type.name` prefix indicates **namespace membership**—nothing
> more. It triggers no implicit binding. To make the `.` call syntax like `p.draw(screen)` work, you
> must bind explicitly: `Point.draw = draw[0]`. See the "Namespace and Method Binding" section below
> for details. It is used to label the type level; the compiler automatically distinguishes Type0,
> Type1, Type2..., transparent to the user.

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

// Generic type ((T: Type) -> Type = a generic type that accepts a type parameter)
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

The current type system has multiple disjoint concepts:

- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack unity, leading to fragmented syntax and a high learning cost.

### Design goals

1. **Extreme unity**: One syntactic rule covers all cases
2. **Concise and elegant**: The symmetric aesthetics of `name: type = value`
3. **No new keywords**: Reuse existing syntactic elements
4. **Theoretically elegant**: Types themselves are values of type Type
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the generics system

The unified syntax model of RFC-010 and the generics system design of RFC-011 are **a natural
fit**—generic parameters slot seamlessly into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type positions in the signature may be omitted, inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraint (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:

- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 together with RFC-010

## Proposal

### Core principle: type constructor vs function/variable

**This is a key design choice that determines the disambiguation rules of the syntax:**

| Notation            | Meaning              | Rule                                                       |
| ------------------- | -------------------- | ---------------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | Explicit `: Type` declaration → forced to be a type        |
| **`f = ...`**       | Function or variable | No `: Type` → HM actively infers it as a function/variable |

**Why this design?**

The `{ ... }` syntax is itself ambiguous:

- `{ x: Float, y: Float }` could be a **type literal** (record type)
- `{ a = 1 + 1 }` could be a **code block** (executed statement, returning Void)

**Disambiguation rule**:

- **With** `: Type` → forced to parse as a type constructor; `{ ... }` is a type literal
- **Without** `: Type` → HM actively parses `{ ... }` as a code block, inferring a function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers () -> Void
main: () -> Void = { println("Hello") }

# ❌ Error: no : Type, compiler cannot parse { ... } as a type
Point = { x: Float, y: Float }  // HM infers a function, not a type!
```

---

**Unified model: identifier : type = expression**

```
├── Variable
│   └── x: Int = 42
│
├── Function
│   └── add: (a: Int, b: Int) -> Int = a + b  # No : Type, HM infers a function
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
│       Point.draw = draw[0]  # Explicit binding enables the dot-call syntax
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers a function
```

### Meta type hierarchy (compiler-internal)

**Internally** the compiler maintains a universe hierarchy `level: selfpointnum` (stored as a
string, theoretically extendable without limit).

| Level    | Description                              |
| -------- | ---------------------------------------- |
| `Type0`  | Everyday types (`Int`, `Float`, `Point`) |
| `Type1`  | Type constructors (`List`, `Maybe`)      |
| `Type2+` | Higher-order constructors                |

**Users never see these numbers**, only `: Type`.

### Curry-Howard correspondence: types as propositions, programs as proofs

YaoXiang's unified syntax `name: type = value` is not chosen arbitrarily—it is a direct mapping of
the Curry-Howard correspondence. This correspondence reveals a profound fact: **a type system and a
logic system are two sides of the same coin**.

| Logic (proposition) | Type system (YaoXiang)              | Example                              |
| ------------------- | ----------------------------------- | ------------------------------------ |
| Proposition P       | Type T                              | `Int`, `Bool`                        |
| Proof of P          | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication) | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction) | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction) | Enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "there exists a proof of type Int, named x, with value 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "there exists an implication proof: given proofs a and b of Int, we can construct a proof of Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof must simultaneously provide a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If a type system allowed constructing a value of type `T`
   with no legitimate runtime representation, it would be like logic allowing proof of a false
   proposition—the system collapses. Curry-Howard tells us: **a type-safe language is naturally a
   consistent logic system**.

2. **The universe hierarchy is a necessary condition**: As detailed below, allowing `Type: Type`
   (i.e., "the type of types is also a type") would yield the Russell paradox (manifested as
   Girard's paradox in type theory). YaoXiang's stratification `Type₀ : Type₁ : Type₂ : ...` ensures
   every type belongs to exactly one level, forming an ever-rising chain that never closes,
   fundamentally avoiding the paradox. This means YaoXiang's type system is **logically consistent**
   in the Curry-Howard sense.

3. **Theoretical foundation of the unified syntax**: The reason `name: type = value` can use one
   syntax to cover all the concepts of variables, functions, types, interfaces, and generics is
   precisely that they are all the same thing under Curry-Howard—**providing a proof for a
   proposition**. A variable is evidence of a proposition, a function is evidence of an implication,
   a record is evidence of a conjunction, a generic is evidence of a universal quantification. The
   unified syntax is not an arbitrary design choice, but a natural consequence of the Curry-Howard
   correspondence.

> **Further reading**: Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM,
> 58(12), 75–84. This article explains the history and significance of the Curry-Howard
> correspondence in accessible language.

### Syntax definition

#### 1. Variable declaration

```yaoxiang
// Basic syntax
x: Int = 42
name: String = "Alice"
flag: Bool = true

// Type inference (may be omitted)
y = 100  // inferred as Int
```

#### 2. Function definition

**The value of a block = the tail expression; `return` exits the function (type `Never`)**—see
[RFC-010a](010a-tail-expression-and-return.md) for details.

```yaoxiang
// Single-expression form
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// Code-block form: value is the tail expression
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
    }                    // match as the tail expression
}

// Void function: the tail expression is Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### Return rules

**The value of a block = the tail expression (the only exit)**:

| Notation                             | Value                                    |
| ------------------------------------ | ---------------------------------------- |
| `= expr` (no braces)                 | `expr`                                   |
| `= { ...; e }` (with braces)         | tail expression `e`                      |
| `= { ...; s }` (last is stmt/assign) | `Void` (an assignment's value is `Void`) |
| `= {}` (empty block)                 | `Void`                                   |

**Semantics of `return`**: non-local exit, **exits the nearest function boundary** (does not "return
to the block"), with type `Never`. `Never <: T` holds for any type (principle of explosion), so
`return` may appear at any return-type position.

```yaoxiang
# Single expression: return value directly
add: (a: Int, b: Int) -> Int = a + b

# Code block: value is the tail expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# Early return: return pierces the block and exits the function (type Never)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # tail expression
}
```

**Design rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its
evaluation semantics differ from a single expression—braces introduce a multi-statement context,
**whose value is given by the tail expression**, removing the ambiguity of "is the last expression
the return value?".

#### `{}` semantics: dependency-driven computation unit

In YaoXiang, `{ ... }` is not just a code block—it is a **dependency-driven computation unit**. This
semantics is consistent in function bodies, variable initialization, and `spawn`:

**Core rules**:

- Assignment statements within `{}` are auto-sorted by dependency relation, not by source order
- Tasks execute immediately when their dependencies are ready; otherwise they block and wait
- **The value of the block = the tail expression** (see return rules); `return` is a `Never`-typed
  non-local exit that exits the function

```yaoxiang
# Dependency-driven: b depends on a, the compiler auto-sorts
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependencies → may run first
    b              # tail expression → block value 11
}
```

> **Difference from a single expression**: `= expr` (no braces) is a simple binding that returns the
> value directly; `= { ... }` (with braces) introduces a dependency-driven computation context,
> allowing multiple statements, with its value given by the tail expression.

#### `spawn` block

`spawn { ... }` is YaoXiang's sole parallel primitive. It leverages the dependency-driven semantics
of `{}` to achieve automatic parallelization:

- Direct sub-assignments within `spawn { ... }` automatically create parallel tasks
- Tasks with ready dependencies execute concurrently
- The caller blocks until all child tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # task 1
    b = fetch_data("url2")    # task 2 (no dependency on a, runs in parallel)
    c = process(a, b)         # depends on a, b → waits for both to complete
    c                         # tail expression → value of spawn
}
// Caller blocks here until all tasks within the spawn block complete
```

> **Detailed definition**: The complete semantics of `spawn`, task creation rules, and blocking
> model are detailed in `008-runtime-concurrency-model.md`.

#### `unsafe` block

`unsafe { ... }` is used to define opaque types and operate on raw pointers. It leverages the
evaluation semantics of `{}` to hand the type definition to the enclosing scope (value exit is the
tail expression):

**Core rules**:

- `unsafe {}` may define types and operate on raw pointers
- The **tail expression** gives the value of `unsafe {}` (type definition handed to enclosing scope)
- Returned types are usable outside `unsafe {}`
- Field access on those types requires unsafe permission

```yaoxiang
# Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw pointer
    }
    SqliteDb           # tail expression → value of the unsafe block
}

# SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: handle field requires unsafe permission
handle = db.handle

# ✅ Via method call
db.close()
```

> **Detailed definition**: The complete semantics of `unsafe`, FFI type definitions, and method
> binding are detailed in `ffi.md`.

#### 3. Type definition

Type definition is the core of YaoXiang's unified syntax, covering fields, default values, bound
methods, and interface implementations:

##### Basic types

**Record type**: a list of fields; the field types may be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: fields may have default values, optional at construction time.

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

##### Built-in types

YaoXiang's identifier system has three layers, recognized by different compiler phases in turn:

1. **Keywords** (parser-distinct tokens) — control structures and declaration keywords, such as
   `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser-distinct tokens) — `true`, `false`, `void`, `Type`; cannot be
   used as ordinary identifiers
3. **Built-in type names** (preregistered in the type checker) — parsed as ordinary identifiers by
   the parser, resolved by the type checker. **Not reserved words; can be shadowed (not
   recommended)**

The difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, built-in
type name): `void` is a value literal (equal to the sole value of Unit), and `Void` is a type name
(equal to the Unit type, logically ⊤). `let x: Void = void` is legal.

Preset built-in type names:

| Type     | Logical counterpart    | Description                                                                                                                                                                                                                                                          |
| -------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥ (false / empty type) | Zero constructors; no value can inhabit this type. Represents "impossible"—divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` never returns normally. **Not a keyword; it's a built-in type name.** |
| `Void`   | ⊤ (true / Unit)        | Exactly one inhabitant (default `void` value). `x: Void = <default>` is legal. The unit element of the product type—`Void` is the zero-field product type (Unit), `Never` is the zero-variant sum type.                                                              |
| `Int`    | —                      | Signed integer                                                                                                                                                                                                                                                       |
| `Float`  | —                      | Floating-point number                                                                                                                                                                                                                                                |
| `Bool`   | —                      | Boolean: `true` / `false`                                                                                                                                                                                                                                            |
| `Char`   | —                      | Unicode character                                                                                                                                                                                                                                                    |
| `String` | —                      | String                                                                                                                                                                                                                                                               |

##### Bound methods

**Approach 1: bind an external function directly inside the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // bound to position 0, after currying: method: (b: Point) -> Float
}
// Call: p1.distance(p2) → distance(p1, p2)
```

**Approach 2: anonymous function + positional binding**

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

##### Interface implementation

**Interface names are written inside the type body; the compiler automatically checks their
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
    Drawable,          // implements the Drawable interface
    Serializable      // implements the Serializable interface
}
```

##### Interface definition

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

##### Namespace function definition

**The `Type.name` prefix indicates namespace membership**—nothing more. It triggers no implicit
binding.

```yaoxiang
// Namespace function: an ordinary function under the Point namespace
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

// Call: just a normal function call
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword, only a conventional parameter name. Writing `p`, `this`, `x`
> has exactly the same effect. The compiler does not look at the parameter name; it looks at the
> type.

##### Method binding (the only way)

To make the `.` method call syntax like `p.draw(screen)` work, **explicit binding is required**. The
`[position]` syntax is the only mechanism to bind a function as a "method" (see RFC-004 for the
detailed syntax).

```yaoxiang
// Define a function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this is the p.draw(screen) syntax available
Point.draw = draw[0]   // The parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // the two call forms are equivalent

// Without [0] = no binding. Point.draw is just a regular function alias, no . syntax
Point.draw = draw       // no binding: only Point.draw(p, screen)
```

**Default behavior**: omitting `[n]` = no parameter bound. The user must explicitly decide which
parameters the caller fills.

**Multi-position binding**:

```yaoxiang
// Bind multiple positions (auto-currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method to ordinary function):

```yaoxiang
// Extract the function from a binding
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. Interface composition

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Use the intersection type
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    item.serialize()
}
```

#### 5. Generic types

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

// Generic method (RFC-023 syntax: type parameters are inferred at the call site)
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

#### 6. Generic call syntax

Generic types and generic functions uniformly use the `()` syntax. `[]` is never used in generic
contexts.

**Core rules**:

1. **`()` does everything**: type application, function calls, and value construction all use `()`

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

2. **Type on the left, value on the right**: `name: type = value`—Type parameters are declared on
   the left, while the right side is always concrete values. The `T` of an empty container `List()`
   must come from the left-side type annotation.

3. **Type information is written only once**—in the parameter declaration, the compiler carries it
   as it flows:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String flow in automatically from numbers and f
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       // inferred as List(Int)
y = List("a", "b")      // inferred as List(String)
z = List()              # ❌ compile error: cannot infer T
z: List(Int) = List()   # ✅ T=Int comes from the left-side annotation
```

5. **Type aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Compared to the old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`,
> `List[Int](1,2,3)` → `List(1,2,3)`. The old `[]` generic syntax is completely removed. `[]` is
> only used for array/list literals and index access.

### Examples

#### Complete example

```yaoxiang
// ======== 1. Interface definitions ========
// Interface = a record type whose fields are all function types
// Interfaces do not need a self parameter — interfaces only define the function signature with the caller position removed

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

// ======== 2. Type definitions ========

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

// ======== 3. Method implementation (ordinary functions + explicit binding) ========

// Define functions (self is only a conventional name, not a keyword)
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

// Explicit binding — only after binding is the dot-call syntax available
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

## Detailed design

### Interface checking algorithm

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

### Direct interface assignment and compile-time optimization

Interface types support direct assignment; the compiler automatically picks the optimal call
strategy based on the right-hand-side type:

```yaoxiang
// Direct assignment of a concrete type → the concrete type is known at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → concrete type is not known at compile time, uses vtable
d: Drawable = get_shape()
d.draw(screen)  // looks up the method through the vtable

// Heterogeneous collection → uses vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // looks up the method through the vtable
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
   generate direct-call IR
2. When the right-hand side type cannot be determined at compile time, fall back to the vtable
   mechanism
3. The vtable fallback guarantees correctness of runtime polymorphism

### Duck-typing support

```yaoxiang
// As long as the same methods exist, it can be assigned to the interface type
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

### Syntax changes

| Before                                   | After                                                                                        |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)`    | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| Requires the `impl` keyword              | No keyword needed; interface names are written inside the type body                          |

### Deprecated: `|` variant syntax

> **Deprecation announcement (2026-07-25)**: The `|` variant syntax is officially deprecated and
> removed from the implementation.

The following forms are **no longer supported**:

```
type Color = red | green | blue                # ❌ deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ deprecated
type Option(T) = some(T) | none                # ❌ deprecated
```

Record types are used uniformly to express sum types. When all fields of a record type are
functions, and they all return the type itself, it is a sum type:

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
   maintain a separate path with lookahead/backtracking for variant types.
2. **Mathematical equivalence**: Under the Curry-Howard correspondence, the disjunction P ⊕ Q
   corresponds to a sum type, which is equivalent to a record type "whose fields are all functions
   returning the type itself". They express the same semantics; two syntaxes are unnecessary.
3. **Zero destructive change**: Before removal, the `|` syntax was only half-supported in the parser
   (parameterless variants parsed correctly, but argument types were lost during monomorphization),
   and no user code depended on it.
4. **AST simplification**: The `Type::Variant(Vec<VariantDef>)` node is removed; all variant types
   go through the `Type::Struct` path, and special branches in the downstream
   typecheck/mono/formatter are all eliminated.

> **Note**: The semantic properties of sum types (variant construction, match exhaustiveness checks,
> tagged-union memory layout) are derived by the typecheck layer from the `Type::Struct` structure,
> not relying on a dedicated AST node. The complete semantics of variant construction are described
> in the next section [Record-style sum type variant construction (authoritative definition)].

### Record-style sum type variant construction (authoritative definition)

> This section expands the recognition criteria from the [Deprecated: `|` variant syntax] section
> into a complete semantics (finalized 2026-09-25, prerequisite for issue #341 RFC-011b phase 2).
> Four decisions: type-qualified calls, zero-payload variants as function-call form, type
> self-sufficient inference, all-or-nothing promotion of variant names.

#### Recognition rule: all-or-nothing

A record type is recognized as a **sum type** when it satisfies both of the following:

1. All field types in the type body are function types;
2. The return type of every field, after type-parameter substitution (`Self` / generic arguments),
   is the record type itself.

Recognition is **all-or-nothing**: when recognition succeeds, all function fields are promoted to
variant constructors simultaneously; if it does not hold, the entire type is a normal record (the
function fields are data fields holding function values), and there is no hybrid form of "partial
variant, partial data". When both data and variant semantics need to be carried, model them as a
data record and a sum type as two types, not as one combined declaration.

#### Call form: type-qualified, no bare name

The **only** call form for a variant constructor is a type-qualified call—taking a variant member on
a type value and invoking it:

```yaoxiang
r1 = Result(Int, String).ok(5)   // Result(Int, String), payload 5
c  = Color.red()                 // zero-payload variant: function-call form matching the field signature
```

**No bare-name form is provided** (`ok(5)` is not a constructor call). Reason: the bare name depends
on the contextual expected type to backtrack the owning sum type and its type arguments, while this
language requires types to be explicitly writable and inference to flow unidirectionally from the
expected position (same discipline as RFC-011a's "Self is an explicit type parameter, no magic").
With the qualified-name form, the type is fully self-sufficient and no context is required. If a
bare name is introduced in the future, it can only be a desugaring when "the context makes it
uniquely determined", and the semantic baseline is still the qualified name defined in this section.

#### Inference: type self-sufficient

Once the qualified name gives the complete type arguments, the variant constructor's signature is
the result of **substituting the type arguments into the field signature**:

```yaoxiang
Result(Int, String).ok   // : (Int) -> Result(Int, String)
```

Payload type checking is ordinary argument checking for function calls (a mismatch reports E1002),
with no new inference rules. Generic and type constructors naturally monomorphize with the type
instantiation; no separate mechanism is needed.

#### Field promotion: variant names are not data fields

Once recognized as a sum type, the variant names are removed from the data field space—accessing a
variant name as a field on a **value** of the sum type is rejected at compile time:

```yaoxiang
r1.ok    // compile error: ok is a variant constructor of Result, not a data field
```

Reason: at runtime, a sum-type value is a tagged union (see below), and does not carry a variant
declaration table; allowing field access would necessarily mis-translate silently. This is an
extension of the same discipline as RFC-011a §1.2 (unified namespace of fields/methods, conflicts
report errors). Construction uses type qualification (`Result.ok`); extraction uses match
destructuring (RFC-010b).

#### Runtime representation and equality

The runtime representation of a sum-type value is a tagged union:
`Enum { type_identity, variant_id, payload }`.

- **Type identity** carries the concrete sum type (not a global placeholder)—values across sum types
  are not comparable;
- **variant_id** is numbered according to the **declaration order** of the variants in the type
  body, which is also the variant set input for RFC-010b's exhaustiveness check;
- **Equality** (`==` / `!=`): same type identity, same variant_id, payloads are equal value by value
  (recursively).

#### std migration

`std.result` is redefined using the mechanism in this section for `ok` / `err` (the native
`result_ok` / `result_err` retire), and the std variant construction follows the same rules as user
sum types, with no privileged channel. The parser special cases for `Result` / `Option` in type
positions and the by-name special case of `make_result` are taken over by phase 2 of
[RFC-011b](011b-operator-overloading.md)—once `?` becomes interface-based, `Result` becomes an
ordinary std sum type.

### Logical operators: `and` / `or` / `!` (authoritative definition, Zig style)

> **Definition announcement (2026-08-03)**: The authoritative form of logical operators is the
> keywords `and` / `or` plus the symbol unary `!` (consistent with SPEC `syntax.md` §2.2 precedence
> table). This design aligns with Zig: **short-circuit control flow uses keywords, pure unary
> operations use symbols**. The C-drift `&&` / `||` from early implementations and the intermediate
> keyword `not` have both been removed.

**Semantics**:

| Operator | Precedence (SPEC §2.2)            | Associativity | Semantics                                    |
| -------- | --------------------------------- | ------------- | -------------------------------------------- |
| `!`      | 3 (unary prefix, tightly binding) | Right-to-left | Logical NOT (pure function, no control flow) |
| `and`    | 10                                | Left-to-right | Short-circuit logical AND                    |
| `or`     | 10                                | Left-to-right | Short-circuit logical OR                     |

```yaoxiang
# Short-circuit evaluation: the right side of `and` does not run when the left is false, nor `or` when the left is true
if x != 0 and y / x > 1 { ... }   # no division by zero when x == 0

# Tight binding: !a == b ≡ (!a) == b (Zig style; opposite to Python's not a == b ≡ not (a == b))
!3 == 4          # false: (!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs)), call then negate
```

The following forms are **no longer supported** (the lexer errors out and points to the
corresponding form):

```
x && y     # ❌ removed, use x and y
x || y     # ❌ removed, use x or y
not x      # ❌ removed, use !x (not reverts to a normal identifier; != is unaffected)
```

**Design rationale** (aligned with Zig, ziglang/zig#272 / #6625):

1. **Short-circuiting is control flow → keyword; pure function is an operation → symbol**. `and` /
   `or` change evaluation order (right side skipped as needed), sharing nature with `if`, hence use
   keywords; `!` performs a pure negation on an already-evaluated operand, sharing nature with `-`
   `+`, hence uses a symbol. YaoXiang uses `?` for error propagation (§2.11), so `!` has no
   conflict.
2. **Tight binding eliminates ambiguity**: `!` visually "sticks" to the operand, with high
   precedence made obvious at a glance; the keyword `not` is forced to leave a space, making the
   binding target (`not a == b`) mentally ambiguous.
3. **Disambiguation**: `&` has two jobs—borrow tokens (`&p` / `&mut p`, RFC-009) and bitwise AND
   (SPEC §2.2 precedence 8). Adding `&&` would make one symbol carry three meanings. `and` / `or` /
   `!` make borrow, bitwise, and logical three concepts visually completely separate.
4. **Precedent**: Zig (a modern system language in the same ecological niche) is exactly the
   combination of `and` / `or` keywords + `!` symbol; Python / Lua / Ada / SQL use all keywords
   (including `not`), and the C family uses all symbols—YaoXiang takes Zig's mix and gets the best
   of both.
5. **Curry-Howard consistency**: Types are propositions (see the correspondence section above), and
   the logical connectives in refinement types are written as `and` / `or` (e.g.
   `{ 0 <= idx and idx < arr.len }`), which is the natural way to express propositions; `!` as a
   unary negation symbol corresponds to ¬.

> **Implementation**: `and` / `or` expand at the IR level into short-circuit jump sequences
> (`a and b ≡ if a { b } else { false }`); `!` is parsed as a unary tight-binding (operand is taken
> at `BP_UNARY + 1`). Regression tests: `tests/yaoxiang/01-syntax/basics/logical_ops.yx`,
> `logical_not.yx`.

## Syntax design notes: named functions are essentially syntactic sugar for lambdas

### Core understanding

**Named functions and lambda expressions are the same thing!** The only difference is that a named
function gives the lambda a name.

```yaoxiang
// These two are essentially identical
add: (a: Int, b: Int) -> Int = a + b           // named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // lambda form (completely equivalent)
```

### Syntactic-sugar model

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Is essentially
name: (Params) -> ReturnType = (params) => body
```

**Key point**: when the signature fully declares the parameter types, the parameter names in the
lambda head become redundant and can be omitted.

### Parameter scope rules

**Parameters shadow outer variables**: parameters in the signature take precedence over the function
body, with the inner scope having higher priority.

```yaoxiang
x = 10  // outer variable

double: (x: Int) -> Int = x * 2  // ✅ parameter x shadows outer x, result is 20
```

### Annotation position is flexible

Type annotations can appear in any of the following positions—**at least one annotation is
required**:

| Annotation position | Form                                     | Note                     |
| ------------------- | ---------------------------------------- | ------------------------ |
| Signature only      | `double: (x: Int) -> Int = x * 2`        | ✅ recommended           |
| Lambda head only    | `double = (x: Int) => x * 2`             | ✅ legal                 |
| Both sides          | `double: (x: Int) -> Int = (x) => x * 2` | ✅ redundant but allowed |

### Complete example

```yaoxiang
// ✅ Recommended: signature is complete, lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: types annotated in the lambda head
double = (x: Int) => x * 2

// ✅ Legal: both sides annotated
double: (x: Int) -> Int = (x) => x * 2
```

### Design advantages

| Property       | Advantage                                                                       |
| -------------- | ------------------------------------------------------------------------------- |
| **Concise**    | No need to repeat parameter names when the signature is complete                |
| **Flexible**   | The lambda form is preserved; use whichever you like                            |
| **Consistent** | Same pattern as variable declarations `x: Int = 42`                             |
| **Intuitive**  | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Tradeoffs

### Advantages

| Advantage            | Description                                            |
| -------------------- | ------------------------------------------------------ |
| Extreme unity        | One syntactic rule covers all cases                    |
| Theoretical elegance | Perfectly symmetric `name: type = value`               |
| No new keywords      | Reuse existing syntactic elements                      |
| Easy to implement    | The compiler only needs to handle one declaration form |
| Easy to learn        | Remember one pattern and you can write all code        |
| Easy to extend       | New features can naturally fit this model              |

### Disadvantages

| Disadvantage      | Description                                                       |
| ----------------- | ----------------------------------------------------------------- |
| Naming convention | Methods must follow the `Type.method` naming                      |
| Verbosity         | Full syntax is longer than simplified syntax, but inference helps |
| Learning curve    | Need to understand the unified model                              |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compile-error example:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// The type may be omitted and inferred by the compiler
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE automatically hints at missing methods
```

### Risks

| Risk                 | Impact                                         | Mitigation                     |
| -------------------- | ---------------------------------------------- | ------------------------------ |
| Parsing complexity   | Unified syntax may increase parsing complexity | Use a recursive-descent parser |
| Performance overhead | vtable lookup may add overhead                 | Compile-time monomorphization  |

---

## Easter egg 🎮: the source of the language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: this is the **unspeakable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One begets two, two begets three, three begets the myriad things.║
║   The Yi has the Supreme Ultimate, which begets the Two Modes.   ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.  ║
║   The compiler falls silent here; philosophy pauses here.    ║
║                                                              ║
║   Thank you for reaching the philosophical boundary of the language.║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would lead to a Type0/Type1
> universe paradox), but we deliberately keep this "easter egg"—when you try to compile it, you will
> receive a zen message from the language's founder. This is not only a technical boundary, but also
> YaoXiang's tribute to type philosophy.

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
       | '{' type_field* '}'                       # record / interface type
       | 'Type'                                    # meta type

type_field ::= identifier ':' type_expr
             | identifier                           # interface constraint

# Generic parameters: as part of the function type, e.g. (T: Type, R: Type) -> (...)
# No dedicated BNF rule needed—: Type parameters are ordinary function parameters

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

| Term               | Definition                                                                                          |
| ------------------ | --------------------------------------------------------------------------------------------------- |
| Declaration        | An assignment in the form `name: type = value`                                                      |
| Record type        | A `{ ... }` type with named fields                                                                  |
| Interface          | A record type whose fields are all function types                                                   |
| Generic type       | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters                    |
| Namespace function | A function in the form `Type.name`, belonging to the Type namespace. Implies no binding             |
| Method binding     | `Type.name = func[n]`, binds position n of func as the caller, enabling the `obj.name(args)` syntax |
| Generic function   | A function using the `(T: Type)` syntax, with type parameters as the first parameter group          |
| Meta type          | `Type`, the only type-level marker in the language                                                  |

---

## Lifecycle and destination

```
┌─────────────┐
│   Draft     │  ← current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Reviewing  │  ← open community discussion and feedback
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
└─────────────┘    └─────────────┘
```
