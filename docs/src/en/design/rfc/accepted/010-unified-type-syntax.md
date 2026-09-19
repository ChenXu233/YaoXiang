---
title: 'RFC-010: Unified Type Syntax - The name: type = value Model'
status: 'Accepted'
author: '晨煦 (Chenxu)'
updated: '2026-07-14 (Never builtin type implemented, #157 closed)'
issue: '#127'
---

# RFC-010: Unified Type Syntax - The `name: type = value` Model

## Summary

This RFC proposes an extremely minimalist, unified type syntax model: **everything is
`name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression. **There is no
`fn`, no `struct`, no `trait`, no `impl`, no lowercase `type` keyword (but `Type` exists as the
meta-type keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that accepts a
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

> **Namespace vs Method binding**: The `Type.name` prefix denotes **namespace membership**, nothing
> more. It does not trigger any implicit binding. For the `.` call syntax like `p.draw(screen)` to
> take effect, an explicit binding is required: `Point.draw = draw[0]`. See the "Namespace and
> Method Binding" section below. It is used to mark the type level; the compiler automatically
> handles the distinction between Type0, Type1, Type2..., which is transparent to the user.

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

// Generic type ((T: Type) -> Type = generic type accepting a type parameter)
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

These concepts lack unity, leading to fragmented syntax and a steep learning curve.

### Design Goals

1. **Extreme unification**: A single syntax rule covers all cases
2. **Concise and elegant**: Symmetric `name: type = value` aesthetics
3. **No new keywords**: Reuse existing syntax elements
4. **Theoretical elegance**: Types themselves are values of type `Type`
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 **naturally fits** the generics system design of RFC-011, and
generic parameters can be seamlessly incorporated into the unified model:

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

- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 and RFC-010 together

## Proposal

### Core Principle: Type Constructors vs. Functions/Variables

**This is a key design choice that determines the disambiguation rules for the syntax:**

| Form                | Meaning              | Rule                                                  |
| ------------------- | -------------------- | ----------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | Explicit `: Type` declaration → forced as type        |
| **`f = ...`**       | Function or variable | No `: Type` → HM actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:

- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executed statement, returns Void)

**Disambiguation rules**:

- **With** `: Type` → forced to parse as a type constructor, `{ ... }` is a type literal
- **Without** `: Type` → HM actively parses `{ ... }` as a code block, infers as a function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main: () -> Void = { println("Hello") }

# ❌ Error: no : Type, the compiler cannot parse { ... } as a type
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
├── Generic type (multi-parameter)
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # Must return: Type
│
├── Namespace function
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # Only after explicit binding does the dot call syntax work
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Meta-type Hierarchy (Compiler Internal)

**The compiler internally** maintains a universe hierarchy `level: selfpointnum` (stored as a
string, theoretically infinite).

| Level    | Description                              |
| -------- | ---------------------------------------- |
| `Type0`  | Everyday types (`Int`, `Float`, `Point`) |
| `Type1`  | Type constructors (`List`, `Maybe`)      |
| `Type2+` | Higher-order constructors                |

**Users never see these numbers**, only `: Type`.

### Curry-Howard Isomorphism: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not an arbitrary choice—it is a direct mapping of
the Curry-Howard correspondence. This isomorphism reveals a profound fact: **the type system and the
logic system are two sides of the same coin**.

| Logic (proposition)  | Type system (YaoXiang)              | Example                              |
| -------------------- | ----------------------------------- | ------------------------------------ |
| Proposition P        | Type T                              | `Int`, `Bool`                        |
| Proof that P is true | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication)  | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)  | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal)  | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)  | Enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "There exists a proof of type Int, named x, whose value is 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "There exists an implication proof: given proofs a and b of Int, one can construct a proof of Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If the type system allows constructing a value of type `T`
   without any legal runtime representation, it is like allowing a proof of a false proposition in
   logic—the system collapses. Curry-Howard tells us: **a type-safe language is naturally a
   consistent logic system**.

2. **Universe hierarchy is a necessary condition**: As detailed below, allowing `Type: Type` (i.e.,
   "the type of types is also a type") would produce Russell's paradox (which manifests as Girard's
   paradox in type theory). YaoXiang's `Type₀ : Type₁ : Type₂ : ...` stratification ensures that
   each type belongs to a specific level, forming an ever-rising chain that never closes,
   fundamentally avoiding paradoxes. This means YaoXiang's type system is **logically consistent**
   in the Curry-Howard sense.

3. **Theoretical foundation of the unified syntax**: The reason `name: type = value` can cover
   variables, functions, types, interfaces, and generics with a single syntax is precisely because
   they are all the same thing under Curry-Howard—**providing proofs for propositions**. A variable
   is evidence for a proposition, a function is evidence for an implication, a record is evidence
   for a conjunction, and generics is evidence for a universal quantification. The unified syntax is
   not an artificially designed coincidence, but a natural consequence of the Curry-Howard
   isomorphism.

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
y = 100  // Inferred as Int
```

#### 2. Function Definition

> **Erratum (2026-09-15, RFC-010a)**: This section and the "Return Rules" below originally used
> `return` as the value exit of a block, conflicting with RFC-007's early-return semantics (see
> [RFC-010a](010a-tail-expression-and-return.md) for details). It is now changed to: **the value of
> a block = the tail expression; `return` exits the function (type `Never`)**. The original
> erroneous example of "must use `return` to return a value" has been removed and is not preserved
> in this erratum.

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
    }                    // match as the tail expression
}

// Void function: tail expression is Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### Return Rules

> **Erratum (2026-09-15, RFC-010a)**: The original rule "in `= { ... }` you must use `return`,
> otherwise it returns `Void`" and its rationale "explicit `return` is needed to eliminate the
> ambiguity of 'whether the last expression is the return value'" are **abolished**. The tail
> expression does not produce this ambiguity, and `return` does not interfere with it (Rust has
> validated this isomorphic design).

**The value of a block = the tail expression (the only exit)**:

| Form                                            | Value                                      |
| ----------------------------------------------- | ------------------------------------------ |
| `= expr` (no braces)                            | `expr`                                     |
| `= { ...; e }` (with braces)                    | Tail expression `e`                        |
| `= { ...; s }` (last is a statement/assignment) | `Void` (the value of assignment is `Void`) |
| `= {}` (empty block)                            | `Void`                                     |

**Semantics of `return`**: Non-local exit, **exits the nearest function boundary** (does not "return
to the block"), of type `Never`. `Never <: T` holds for any type (principle of explosion), so
`return` can appear in any return-type position.

```yaoxiang
# Single expression: returns the value directly
add: (a: Int, b: Int) -> Int = a + b

# Code block: value is the tail expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# Early return: return passes through the block, exiting the function (type Never)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # tail expression
}
```

> **Design rationale (Erratum 2026-09-15, RFC-010a)**: `{ ... }` is a dependency-driven computation
> unit (see below), and its evaluation semantics differ from a single expression—braces introduce a
> multi-statement context, **its value is given by the tail expression**. The original argument that
> "explicit `return` is needed to eliminate ambiguity" is abolished: the tail expression does not
> produce that ambiguity.

#### `{}` Semantics: Dependency-Driven Computation Unit

In YaoXiang, `{ ... }` is not merely a code block—it is a **dependency-driven computation unit**.
This semantics is consistent in function bodies, variable initialization, and `spawn`:

**Core rules**:

- Assignment statements within `{}` are automatically sorted by dependency relationship, not by
  writing order
- Execute immediately when dependencies are ready; block and wait when missing
- **The value of a block = the tail expression** (see Return Rules); `return` is a non-local exit of
  type `Never`, exiting the function

```yaoxiang
# Dependency-driven: b depends on a, the compiler automatically sorts
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependencies → can execute first
    b              # tail expression → value of the block is 11
}
```

> **Difference from a single expression**: `= expr` (no braces) is a simple binding that directly
> returns the value; `= { ... }` (with braces) introduces a dependency-driven computation context,
> allowing multiple statements, with its value given by the tail expression.

#### `spawn` Block

`spawn { ... }` is the only parallel primitive in YaoXiang. It leverages the dependency-driven
semantics of `{}` to achieve automatic parallelization:

- Direct child assignments within `spawn { ... }` automatically create parallel tasks
- Tasks with satisfied dependencies execute concurrently immediately
- The caller blocks until all child tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # depends on a, b → waits for both to complete
    return c                  # value of the block (tail expression exit not yet implemented, see #365)
}
// The caller blocks here until all tasks in the spawn block complete
```

> **Detailed definition**: The complete semantics of `spawn`, task creation rules, and blocking
> model are detailed in `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and to operate on raw pointers. It leverages the
evaluation semantics of `{}` to hand the type definition to the outer scope (the value exit is the
tail expression):

**Core rules**:

- Types can be defined and raw pointers operated on within `unsafe {}`
- The **tail expression** gives the value of `unsafe {}` (type definition handed to the outer scope)
- Returned types are usable outside `unsafe {}`
- Field access on the type requires `unsafe` permission

```yaoxiang
# Define an opaque type in an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw pointer
    }
    SqliteDb           # tail expression → value of the unsafe block
}

# SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: the handle field requires unsafe permission
handle = db.handle

# ✅ Through method call
db.close()
```

> **Detailed definition**: The complete semantics of `unsafe`, FFI type definition, and method
> binding are detailed in `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound
methods, and interface implementations:

##### Basic Types

**Record type**: A list of fields; field types can be any type expression.

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

##### Builtin Types

YaoXiang's identifier system has three layers, recognized by different compiler phases:

1. **Keywords** (parser-independent tokens) — Control structures and declaration keywords, such as
   `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser-independent tokens) — `true`, `false`, `void`, `Type`, cannot
   be used as ordinary identifiers
3. **Builtin type names** (pre-registered by the type checker) — The parser treats them as ordinary
   identifiers; the type checker is responsible for parsing. **Not reserved words, can be shadowed
   (not recommended)**

The difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, builtin type
name): `void` is a value literal (equal to the unique value of Unit), `Void` is a type name (equal
to the Unit type, logical ⊤). `let x: Void = void` is legal.

Pre-registered builtin type names:

| Type     | Logical equivalent   | Description                                                                                                                                                                                                                                                                |
| -------- | -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥ (false/empty type) | Zero constructor; no value can inhabit this type. Represents "impossible"—divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` means it never returns normally. **Not a keyword, but a builtin type name.** |
| `Void`   | ⊤ (true/Unit)        | Exactly one inhabitant (default `void` value). `x: Void = <default>` is legal. Corresponds to the identity element of sum types and the identity element of product types—`Void` is a zero-field product type (Unit), `Never` is a zero-variant sum type.                  |
| `Int`    | —                    | Signed integer                                                                                                                                                                                                                                                             |
| `Float`  | —                    | Floating-point number                                                                                                                                                                                                                                                      |
| `Bool`   | —                    | Boolean value: `true` / `false`                                                                                                                                                                                                                                            |
| `Char`   | —                    | Unicode character                                                                                                                                                                                                                                                          |
| `String` | —                    | String                                                                                                                                                                                                                                                                     |

##### Bound Methods

**Method 1: Bind external functions directly within the type definition body**

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
        (dx * dx + dy * dy).sqrt()      # tail expression
    })
}
// Syntax: ((params) => body)[position]
// Call: p1.distance(p2) → distance(p1, p2)
```

##### Interface Implementation

**The interface name is written within the type body, and the compiler automatically checks its
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
    "Point(${p.x}, ${p.y})"
}

// Call: just a regular function call
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing it as `p`, `this`,
> or `x` has exactly the same effect. The compiler does not look at the parameter name, but at the
> type.

##### Method Binding (The Only Way)

For the `.` method call syntax like `p.draw(screen)` to take effect, **explicit binding is
required**. The `[position]` syntax is the only mechanism for binding a function as a "method" (see
RFC-004 for detailed syntax).

```yaoxiang
// Define the function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does p.draw(screen) syntax become available
Point.draw = draw[0]   // The parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // both call forms are equivalent

// Not writing [0] = not binding. Point.draw is just an ordinary function alias, no . syntax
Point.draw = draw       // not bound: only Point.draw(p, screen) works
```

**Default behavior**: Not writing `[n]` = do not bind any parameters. The user must explicitly
decide which parameters are filled by the caller.

**Multi-position binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method to ordinary function):

```yaoxiang
// Extract the function from a binding
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. Interface Composition

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using an intersection type
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

// Generic method (RFC-023 syntax: type parameter inferred automatically at call site)
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

1. **`()` does everything**: type application, function call, value construction—all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function call — types flow automatically from arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value`—Type parameters are declared on
   the left side, and the right side is always a concrete value. The `T` in empty container `List()`
   must be obtained from the left-side type annotation.

3. **Type information only needs to be written once**—in the parameter declaration, the compiler
   carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically come from the types of numbers and f
```

4. **Value construction infers the type from elements**:

```yaoxiang
x = List(1, 2, 3)       // inferred as List(Int)
y = List("a", "b")      // inferred as List(String)
z = List()              // ❌ Compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int comes from the left-side annotation
```

5. **Type aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`,
> `List[Int](1,2,3)` → `List(1,2,3)`. The old `[]` generic syntax is completely removed. `[]` is
> only used for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface definition ========
// Interface = a record type whose fields are all function types
// Interfaces do not need a self parameter — interfaces only define "function signatures with the caller position removed"

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

// ======== 2. Type definition ========

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

// ======== 3. Method implementation (regular function + explicit binding) ========

// Define the function (self is just a conventional name, not a keyword)
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

// Chained calls
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
    // For each field of the interface (function field)
    for (field_name, iface_field) in &iface.fields {
        // Check if the type has a method with the same name
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

### Interface Direct Assignment and Compile-Time Optimization

Interface types support direct assignment, and the compiler will automatically choose the optimal
calling strategy based on the type of the right-hand side value:

```yaoxiang
// Direct assignment of a concrete type → specific type is known at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → specific type cannot be determined at compile time, use vtable
d: Drawable = get_shape()
d.draw(screen)  // method lookup through vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // method lookup through vtable
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
| Requires `impl` keyword                  | No keyword required, interface name written after the type body                              |

### Deprecated: `|` Variant Syntax

> **Deprecation announcement (2026-07-25)**: The `|` variant syntax is officially deprecated and
> removed from the implementation.

The following forms are **no longer supported**:

```
type Color = red | green | blue                # ❌ Deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ Deprecated
type Option(T) = some(T) | none                # ❌ Deprecated
```

Sum types are now uniformly expressed using record types. When all fields of a record type are
functions that return the type itself, it is a sum type:

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

1. **Eliminate special cases**: `|` is the only non-`name: type = value` form syntax in BNF. After
   removal, the `type_expr` production is completely unified, and the parser no longer needs to
   maintain separate paths and lookahead backtracking for variant types.
2. **Mathematical equivalence**: Under the Curry-Howard isomorphism, the sum type corresponding to
   disjunction P ⊕ Q is equivalent to "a record type whose fields are all functions returning the
   type itself". Both express the same semantics, and two syntaxes are not needed.
3. **Zero destructiveness**: Before removal, the `|` syntax was only half-supported in the parser
   (no-argument variants could be parsed, but argument types were lost during monomorphization), and
   no user code depended on it.
4. **AST simplification**: The `Type::Variant(Vec<VariantDef>)` node is removed, and all variant
   types uniformly take the `Type::Struct` path. The special branches in downstream
   typecheck/mono/formatter are all eliminated.

> **Note**: The semantic properties of sum types (such as match exhaustiveness checking, tagged
> union memory layout) are inferred by the typecheck layer from the `Type::Struct` structure, and do
> not depend on independent AST nodes.

### Logical Operators: `and` / `or` / `!` (Authoritative Definition, Zig-style)

> **Definition announcement (2026-08-03)**: The authoritative form of logical operators is the
> keywords `and` / `or` plus the symbolic unary `!` (consistent with the priority table in SPEC
> `syntax.md` §2.2). This design aligns with Zig: **short-circuit control flow uses keywords, pure
> unary operations use symbols**. The early C-style implementation of `&&` / `||` and the
> intermediate keyword `not` have been removed.

**Semantics**:

| Operator | Precedence (SPEC §2.2)          | Associativity | Semantics                                    |
| -------- | ------------------------------- | ------------- | -------------------------------------------- |
| `!`      | 3 (unary prefix, tight binding) | Right-to-left | Logical NOT (pure function, no control flow) |
| `and`    | 10                              | Left-to-right | Short-circuit logical AND                    |
| `or`     | 10                              | Left-to-right | Short-circuit logical OR                     |

```yaoxiang
# Short-circuit evaluation: when left of and is false / left of or is true, right side does not execute
if x != 0 and y / x > 1 { ... }   # no division by zero when x == 0

# Tight binding: !a == b ≡ (!a) == b (Zig-style; opposite to Python's not a == b ≡ not (a == b))
!3 == 4          # false: (!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs)), negate after call
```

The following forms are **no longer supported** (the lexer reports an error and suggests the
corresponding form):

```
x && y     # ❌ Removed, use x and y
x || y     # ❌ Removed, use x or y
not x      # ❌ Removed, use !x (not reverts to a regular identifier; != is not affected)
```

**Design rationale** (aligned with Zig, ziglang/zig#272 / #6625):

1. **Short-circuit is control flow → keyword; pure function is operation → symbol**. `and` / `or`
   change the order of evaluation (right side skipped as needed), the same nature as `if`, so they
   use keywords; `!` performs a pure negation on an already-evaluated operand, the same nature as
   `-` `+`, so it uses a symbol. YaoXiang uses `?` for error propagation (§2.11), no conflict with
   `!`.
2. **Tight binding eliminates ambiguity**: Visually, `!` "sticks tightly" to the operand, and its
   high precedence is clear at a glance; the keyword `not` is forced to leave a space from the
   operand, and binding to which side (`not a == b`) is prone to mental ambiguity.
3. **Disambiguation**: `&` plays two roles—borrow token (`&p` / `&mut p`, RFC-009) and bitwise AND
   (SPEC §2.2 priority 8). Introducing `&&` would make one symbol carry three meanings. `and` / `or`
   / `!` make the three concepts of borrow, bitwise, and logical visually completely separated.
4. **Precedent**: Zig (a modern systems language in the same ecosystem niche) is exactly the
   combination of `and` / `or` keywords + `!` symbol; Python / Lua / Ada / SQL use all keywords
   (including `not`); the C family uses all symbols—YaoXiang takes Zig's hybrid, getting the best of
   both.
5. **Curry-Howard consistency**: Types are propositions (see the isomorphism section above), and the
   logical connectives in refinement types are written as `and` / `or` (such as
   `{ 0 <= idx and idx < arr.len }`), which is a natural expression of propositions; `!` as a unary
   negation symbol corresponds to ¬.

> **Implementation**: `and` / `or` are expanded at the IR level into short-circuit jump sequences
> (`a and b ≡ if a { b } else { false }`); `!` is parsed as a unary with tight binding (operand is
> parsed according to `BP_UNARY + 1`). Regression tests:
> `tests/yaoxiang/01-syntax/basics/logical_ops.yx`, `logical_not.yx`.

## Syntax Design Notes: Named Functions Are Essentially Syntactic Sugar for Lambdas

### Core Understanding

**Named functions and lambda expressions are the same thing!** The only difference is that a named
function gives the lambda a name.

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

**Key point**: When the signature fully declares the parameter types, the parameter names in the
lambda header become redundant and can be omitted.

### Parameter Scope Rules

**Parameters shadow outer variables**: The parameter scope in the signature shadows the function
body, with the inner scope having higher priority.

```yaoxiang
x = 10  // outer variable

double: (x: Int) -> Int = x * 2  // ✅ parameter x shadows outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be in any of the following positions, **at least one is required**:

| Annotation position | Form                                     | Description              |
| ------------------- | ---------------------------------------- | ------------------------ |
| Signature only      | `double: (x: Int) -> Int = x * 2`        | ✅ Recommended           |
| Lambda header only  | `double = (x: Int) => x * 2`             | ✅ Legal                 |
| Both sides          | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Example

```yaoxiang
// ✅ Recommended: signature complete, lambda header omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: types annotated in lambda header
double = (x: Int) => x * 2

// ✅ Legal: annotated on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature        | Advantage                                                                       |
| -------------- | ------------------------------------------------------------------------------- |
| **Concise**    | No need to repeat parameter names when signature is complete                    |
| **Flexible**   | Lambda form is preserved, use whichever you prefer                              |
| **Consistent** | Maintains the unified pattern with variable declaration `x: Int = 42`           |
| **Intuitive**  | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage            | Description                                              |
| -------------------- | -------------------------------------------------------- |
| Extreme unification  | A single syntax rule covers all cases                    |
| Theoretical elegance | Perfectly symmetric `name: type = value`                 |
| No new keywords      | Reuses existing syntax elements                          |
| Easy to implement    | The compiler only needs to handle one declaration form   |
| Easy to learn        | Remember one pattern to write all code                   |
| Easy to extend       | New features can be naturally integrated into this model |

### Disadvantages

| Disadvantage      | Description                                                       |
| ----------------- | ----------------------------------------------------------------- |
| Naming convention | Methods need to follow the `Type.method` naming convention        |
| Verbose           | Full syntax is longer than simplified syntax, but can be inferred |
| Learning curve    | Need to understand the unified model                              |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compile error example:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Type can be omitted, inferred by the compiler
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

**Warning**: This is an **unspeakable** thing!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One gives birth to two, two to three, three to all things.  ║
║   Change has the Supreme Ultimate, which gives birth to the two modes. ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.   ║
║   The compiler falls silent here, philosophy pauses here.     ║
║                                                              ║
║   Thank you for reaching the philosophical boundary of the language. ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause the Type0/Type1
> universe paradox), but we deliberately preserve this "easter egg"—when you try to compile it, you
> will receive a Zen message from the language's founder. This is not only a technical boundary, but
> also a tribute from YaoXiang to the philosophy of types.

---

## Appendix

### Syntax BNF

```bnf
program ::= statement*

statement ::= declaration | expression

# Unified declaration: name: Type = expression
declaration ::= identifier ':' type_expr '=' expression

# Type expressions
type_expr ::= identifier
       | identifier '(' type_expr (',' type_expr)* ')'      # type application
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # function type
       | '{' type_field* '}'                       # record/interface type
       | 'Type'                                    # meta-type

type_field ::= identifier ':' type_expr
             | identifier                           # interface constraint

# Generic parameters: as part of a function type, e.g., (T: Type, R: Type) -> (...)
# No separate BNF rule needed — : Type parameters are ordinary function parameters

# Expressions
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

| Term               | Definition                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------------- |
| Declaration        | An assignment statement in the form `name: type = value`                                          |
| Record type        | A `{ ... }` type containing named fields                                                          |
| Interface          | A record type whose fields are all function types                                                 |
| Generic type       | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters                  |
| Namespace function | A function in the form `Type.name`, belonging to the Type namespace. Does not imply any binding   |
| Method binding     | `Type.name = func[n]`, binding position n of func as the caller, enabling `obj.name(args)` syntax |
| Generic function   | A function using `(T: Type)` syntax, with type parameters as the first parameter group            |
| Meta-type          | `Type`, the only type-level marker in the language                                                |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review│  ← Open community discussion and feedback
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
│ (Official design) │ (Stays in place) │
└─────────────┘    └─────────────┘
```
