---
title: 'RFC-010: Unified Type Syntax - The name: type = value Model'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-09-25'
issue: '#127'
---

# RFC-010: Unified Type Syntax - The name: type = value Model

## Summary

This RFC proposes an extremely minimal and unified type syntax model: **everything is
`name: type = value`**.

YaoXiang has only one form of declaration:

```
identifier : type = expression
```

where `type` can be any type expression, and `expression` can be any value expression. **No `fn`, no
`struct`, no `trait`, no `impl`, no lowercase `type` keyword (but there is `Type` as the meta-type
keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that takes a
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

**`Type` is the only meta-type keyword in the language**.

> **Namespace vs Method Binding**: The prefix `Type.name` denotes **namespace ownership**, nothing
> more. It does not trigger any implicit binding. For `.`-call syntax like `p.draw(screen)` to work,
> an explicit binding is required: `Point.draw = draw[0]`. See the "Namespace and Method Binding"
> section below. It is used to mark the type hierarchy; the compiler automatically handles the
> distinction between Type0, Type1, Type2..., transparent to the user.

```yaoxiang
// Core syntax: unification + distinction

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

The current type system has several separate concepts:

- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack unity, leading to fragmented syntax and a high learning cost.

### Design Goals

1. **Extreme unification**: A single syntax rule covers all cases
2. **Concise and elegant**: Symmetric aesthetic of `name: type = value`
3. **No new keywords**: Reuse existing syntax elements
4. **Theoretical elegance**: Types themselves are also values of the type Type
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 fits **naturally** with the generics system design of RFC-011.
Generic parameters can be seamlessly integrated into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type position in the signature can be omitted, automatically inferred at the call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraints (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by the parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:

- RFC-011 Phase 1 (Basic Generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 in sync with RFC-010

## Proposal

### Core Principle: Type Constructors vs Functions/Variables

**This is a key design decision that determines the disambiguation rules of the syntax:**

| Syntax              | Meaning              | Rule                                                    |
| ------------------- | -------------------- | ------------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | Explicit `: Type` declaration → forced to be a type     |
| **`f = ...`**       | Function or variable | No `: Type` → HM actively infers as a function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:

- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executed statement, returning Void)

**Disambiguation rules**:

- **With** `: Type` → forced to resolve as a type constructor, `{ ... }` is a type literal
- **Without** `: Type` → HM actively resolves `{ ... }` as a code block, inferred as a function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main: () -> Void = { println("Hello") }

# ❌ Error: no : Type, the compiler cannot resolve { ... } as a type
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
│       Point.draw = draw[0]  # Only after explicit binding does the dot-call syntax work
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as a function
```

### Meta-type Hierarchy (Compiler Internals)

**The compiler internally** maintains a universe hierarchy `level: selfpointnum` (stored as a
string, theoretically infinitely extensible).

| Level    | Description                              |
| -------- | ---------------------------------------- |
| `Type0`  | Everyday types (`Int`, `Float`, `Point`) |
| `Type1`  | Type constructors (`List`, `Maybe`)      |
| `Type2+` | Higher-order constructors                |

**Users never see these numbers**, only `: Type`.

### Curry-Howard Correspondence: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not a random choice—it is the direct mapping of
the Curry-Howard correspondence. This correspondence reveals a profound fact: **the type system and
the logical system are two sides of the same coin**.

| Logic (Proposition)            | Type System (YaoXiang)              | Example                              |
| ------------------------------ | ----------------------------------- | ------------------------------------ |
| Proposition P                  | Type T                              | `Int`, `Bool`                        |
| Proof that P holds             | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication)            | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)            | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal quantifier) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)            | Enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "There exists a proof of type Int, named x, whose value is 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "There exists an implication proof: given proofs a and b of type Int, we can construct a proof of type Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If the type system allows constructing a value of type `T`
   without any legitimate runtime representation, that is like allowing a proof of a false
   proposition in logic—the system collapses. Curry-Howard tells us: **a type-safe language is
   naturally a consistent logical system**.

2. **The universe hierarchy is a necessary condition**: As detailed below, if `Type: Type` were
   allowed (i.e., "the type of types is also a type"), it would lead to Russell's paradox
   (manifested as Girard's paradox in type theory). YaoXiang's `Type₀ : Type₁ : Type₂ : ...`
   stratification ensures that each type belongs to only one level, forming a never-closing
   ascending chain, fundamentally avoiding paradoxes. This means YaoXiang's type system is
   **logically consistent** in the Curry-Howard sense.

3. **The theoretical foundation of the unified syntax**: The reason `name: type = value` can use a
   single syntax to cover variables, functions, types, interfaces, and generics is precisely because
   under Curry-Howard, they are all the same thing—**providing a proof for a proposition**.
   Variables are evidence of a proposition, functions are evidence of implication, records are
   evidence of conjunction, generics are evidence of universal quantification. The unified syntax is
   not a human-designed coincidence, but a natural consequence of the Curry-Howard correspondence.

> **Further reading**: Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM,
> 58(12), 75–84. This article explains the history and significance of the Curry-Howard
> correspondence in accessible language.

### Syntax Definition

#### 1. Variable Declaration

```yaoxiang
// Basic syntax
x: Int = 42
name: String = "Alice"
flag: Bool = true

// Type inference (optional)
y = 100  // Inferred as Int
```

#### 2. Function Definition

**The value of a block is the tail expression, `return` exits the function (type `Never`)**—see
[RFC-010a](010a-tail-expression-and-return.md) for details.

```yaoxiang
// Single expression form
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// Code block form: the value is the tail expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b                    // Tail expression → value of the block
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

#### Return Rules

**The value of a block = the tail expression (the only exit point)**:

| Syntax                                                   | Value                                         |
| -------------------------------------------------------- | --------------------------------------------- |
| `= expr` (no braces)                                     | `expr`                                        |
| `= { ...; e }` (with braces)                             | Tail expression `e`                           |
| `= { ...; s }` (last position is a statement/assignment) | `Void` (the value of an assignment is `Void`) |
| `= {}` (empty block)                                     | `Void`                                        |

**The semantics of `return`**: Non-local exit, **exits the nearest function boundary** (does not
"return to the block"), with type `Never`. `Never <: T` holds for any type (principle of explosion),
so `return` can appear at any return-type position.

```yaoxiang
# Single expression: directly returns the value
add: (a: Int, b: Int) -> Int = a + b

# Code block: the value is the tail expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# Early return: return passes through the block and exits the function (type Never)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # Tail expression
}
```

**Design rationale**: `{ ... }` is a dependency-driven computation unit (see below); its evaluation
semantics differ from a single expression—braces introduce a multi-statement context, **its value is
given by the tail expression**, with no ambiguity about whether "the last expression is the return
value".

#### `{}` Semantics: Dependency-Driven Computation Unit

In YaoXiang, `{ ... }` is not merely a code block—it is a **dependency-driven computation unit**.
This semantics is consistent in function bodies, variable initialization, and `spawn`:

**Core rules**:

- Assignment statements inside `{}` are automatically ordered by dependency, not by written order
- When dependencies are ready, execute immediately; otherwise, block and wait
- **The value of the block = the tail expression** (see Return Rules); `return` is a non-local exit
  of type `Never`, exiting the function

```yaoxiang
# Dependency-driven: b depends on a, the compiler automatically orders them
result: Int = {
    b = a + 1      # Depends on a → automatically placed after a
    a = 10         # No dependency → can execute first
    b              # Tail expression → block value 11
}
```

> **Difference from single expression**: `= expr` (no braces) is a simple binding that directly
> returns the value; `= { ... }` (with braces) introduces a dependency-driven computation context,
> allowing multiple statements, and its value is given by the tail expression.

#### `spawn` Block

`spawn { ... }` is the only parallel primitive in YaoXiang. It leverages the dependency-driven
semantics of `{}` for automatic parallelization:

- Direct sub-assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks with ready dependencies execute concurrently immediately
- The caller blocks until all sub-tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # Depends on a, b → waits for both to complete
    c                         # Tail expression → value of spawn
}
// The caller blocks here until all tasks inside the spawn block complete
```

> **Detailed definition**: For the complete semantics of `spawn`, task creation rules, and blocking
> model, see `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and operate on raw pointers. It leverages the
evaluation semantics of `{}` to hand over the type definition to the outer scope (the value exit is
the tail expression):

**Core rules**:

- Types can be defined and raw pointers operated on inside `unsafe {}`
- The **tail expression** gives the value of `unsafe {}` (type definitions are handed over to the
  outer scope)
- Returned types are usable outside `unsafe {}`
- Field access of the types requires unsafe permission

```yaoxiang
# Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # Raw pointer
    }
    SqliteDb           # Tail expression → value of the unsafe block
}

# SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: the handle field requires unsafe permission
handle = db.handle

# ✅ Via method call
db.close()
```

> **Detailed definition**: For the complete semantics of `unsafe`, FFI type definitions, and method
> binding, see `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound
methods, and interface implementations:

##### Basic Types

**Record type**: A list of fields, where field types can be any type expression.

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

##### Built-in Types

YaoXiang's identifier system is divided into three layers, recognized by different compiler phases
in sequence:

1. **Keywords** (parser-specific tokens) — Control structures and declaration keywords, such as
   `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser-specific tokens) — `true`, `false`, `void`, `Type`, cannot be
   used as ordinary identifiers
3. **Built-in type names** (pre-registered in the type checker) — The parser treats them as ordinary
   identifiers, and the type checker handles them. **They are not reserved words and can be shadowed
   (not recommended)**

The distinction between `void` (lowercase, literal reserved word) and `Void` (uppercase, built-in
type name): `void` is a value literal (equal to the single value of Unit), and `Void` is a type name
(equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Predefined built-in type names:

| Type     | Logical counterpart  | Description                                                                                                                                                                                                                                                                      |
| -------- | -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥ (false/empty type) | Zero constructors, no value can inhabit this type. Represents "impossible"—divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` indicates it never returns normally. **Not a keyword, but a built-in type name.** |
| `Void`   | ⊤ (true/Unit)        | Exactly one inhabitant (default void value). `x: Void = <default>` is legal. The identity element of the sum type corresponds to the identity element of the product type—`Void` is the zero-field product type (Unit), and `Never` is the zero-variant sum type.                |
| `Int`    | —                    | Signed integer                                                                                                                                                                                                                                                                   |
| `Float`  | —                    | Floating-point number                                                                                                                                                                                                                                                            |
| `Bool`   | —                    | Boolean: `true` / `false`                                                                                                                                                                                                                                                        |
| `Char`   | —                    | Unicode character                                                                                                                                                                                                                                                                |
| `String` | —                    | String                                                                                                                                                                                                                                                                           |

##### Binding Methods

**Method 1: Bind external functions directly inside the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // Bind to position 0, after currying, method: (b: Point) -> Float
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
        (dx * dx + dy * dy).sqrt()      # Tail expression
    })
}
// Syntax: ((params) => body)[position]
// Call: p1.distance(p2) → distance(p1, p2)
```

##### Interface Implementation

**Interface names are written inside the type body, and the compiler automatically checks the
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
    Drawable,          // Implements the Drawable interface
    Serializable      // Implements the Serializable interface
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

> **Note**: `self` is not a keyword, just a conventional name for the parameter. Writing `p`,
> `this`, or `x` has exactly the same effect. The compiler does not look at parameter names, but at
> types.

##### Method Binding (the only way)

For the `.` method-call syntax like `p.draw(screen)` to work, **an explicit binding is required**.
The `[position]` syntax is the only mechanism for binding a function as a "method" (see RFC-004 for
detailed syntax).

```yaoxiang
// Define the function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does the p.draw(screen) syntax work
Point.draw = draw[0]   // The parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // Syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // Both calling forms are equivalent

// Omitting [0] = no binding. Point.draw is just an ordinary function alias, without the . syntax
Point.draw = draw       // No binding: only Point.draw(p, screen) is possible
```

**Default behavior**: Not writing `[n]` = not binding any parameter. The user must explicitly decide
which parameters are filled by the caller.

**Multiple positional bindings**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method to ordinary function):

```yaoxiang
// Take the function out of a binding
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

IntList.push(Int)(self, item)  // Call example

// Generic method (RFC-023 syntax: type parameters are automatically inferred at the call site)
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

1. **`()` does everything for application**: Type application, function calls, and value
   construction all use `()`

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

2. **Type on the left, value on the right**: `name: type = value` — Type parameters are declared on
   the left, and the right side is always concrete values. The `T` in an empty container `List()`
   must be obtained from the left-side type annotation.

3. **Type information only needs to be written once**—in the parameter declaration, the compiler
   carries it along:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically comes from the types of numbers and f
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       // Inferred as List(Int)
y = List("a", "b")      // Inferred as List(String)
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
> `List[Int](1,2,3)` → `List(1,2,3)`. The old `[]` generic syntax has been completely removed. `[]`
> is only used for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========
// Interface = a record type whose fields are all function types
// No self parameter is needed in the interface — the interface only defines "the function signature with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // Returns the interface type, the concrete implementation returns its own type
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

// ======== 3. Method Implementation (Ordinary function + explicit binding) ========

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

// Explicit binding — the dot-call syntax works only after binding
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

// Generic function (RFC-023 syntax: type parameters omitted at call site, automatically inferred)
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
            // Check if the method signature is compatible
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

### Direct Interface Assignment and Compile-Time Optimization

Interface types support direct assignment, and the compiler automatically chooses the optimal
calling strategy based on the right-hand-side type of the assignment:

```yaoxiang
// Direct assignment of a concrete type → the concrete type can be determined at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → the concrete type cannot be determined at compile time, vtable is used
d: Drawable = get_shape()
d.draw(screen)  // Method looked up via vtable

// Heterogeneous collection → vtable is used
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Method looked up via vtable
}
```

**Compile-time optimization strategy**:

| Scenario                         | Inference result     | Call method                 |
| -------------------------------- | -------------------- | --------------------------- |
| `d: Drawable = Circle(1)`        | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()`      | Unknown              | vtable                      |
| `shapes: List(Drawable) = [...]` | Heterogeneous        | vtable                      |

**Rules**:

1. When the right-hand side is a concrete type constructor and can be determined at compile time,
   generate a direct call IR
2. When the right-hand side type cannot be determined at compile time, fall back to the vtable
   mechanism
3. The vtable fallback guarantees the correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as the same method is present, it can be assigned to the interface type
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
| Requires the `impl` keyword              | No keyword required, interface name written after the type body                              |

### Deprecated: `|` Variant Syntax

> **Deprecation announcement (2026-07-25)**: The `|` variant syntax is officially deprecated and
> removed from the implementation.

The following writing styles are **no longer supported**:

```
type Color = red | green | blue                # ❌ Deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ Deprecated
type Option(T) = some(T) | none                # ❌ Deprecated
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

1. **Eliminate special cases**: `|` is the only syntax form in the BNF that is not
   `name: type = value`. After its removal, the `type_expr` production is completely unified, and
   the parser no longer needs to maintain independent paths and lookahead fallbacks for variant
   types.
2. **Mathematical equivalence**: Under the Curry-Howard correspondence, the sum type corresponding
   to disjunction P ⊕ Q is equivalent to a record type "whose fields are all functions returning the
   type itself". They express the same semantics, and two sets of syntax are not required.
3. **Zero destructiveness**: Before removal, the `|` syntax was only half-supported in the parser
   (parameterless variants could be parsed but parameter types were lost during monomorphization),
   and no user code depended on it.
4. **AST simplification**: The `Type::Variant(Vec<VariantDef>)` node is removed; all variant types
   uniformly take the `Type::Struct` path, and the special branches in the downstream
   typecheck/mono/formatter are all eliminated.

> **Note**: The semantic properties of sum types (variant construction, match exhaustiveness check,
> tagged union memory layout) are derived by the typecheck layer from the `Type::Struct` structure,
> without relying on an independent AST node. The complete semantics of variant construction are
> described in the next section [Record-Style Sum Type Variant Construction (Authoritative
> Definition)].

### Record-Style Sum Type Variant Construction (Authoritative Definition)

> This section expands the recognition criteria from the [Deprecated: `|` Variant Syntax] section
> into a complete semantics (finalized on 2026-09-25, a prerequisite for issue #341 RFC-011b Phase
> 2). Four decisions: type-qualified call, zero-payload variant as function call form,
> type-self-sufficient inference, all-or-nothing promotion of variant names.

#### Decision Rule: All or Nothing

A record type is determined to be a **sum type** when the following two conditions are met:

1. All field types in the type body are function types;
2. The return type of each field, after type parameter substitution (`Self`/generic arguments), is
   the record type itself.

The decision is **all or nothing**: when the decision holds, all function fields are simultaneously
promoted to variant constructors; if not, the entire type is an ordinary record (function fields are
data fields storing function values), and there is no mixed form of "partial variants, partial
data". When data and variant semantics need to be carried simultaneously, model them as two types—a
data record and a sum type—without merging declarations.

#### Call Form: Type-Qualified, No Bare Name

The **only** call form of a variant constructor is a type-qualified call—take the variant member on
the type value and call it:

```yaoxiang
r1 = Result(Int, String).ok(5)   // Result(Int, String), payload 5
c  = Color.red()                 // Zero-payload variant: function call form consistent with the field signature
```

**No bare-name form is provided** (`ok(5)` is not a constructor call). Reason: the bare name depends
on the expected context type to retroactively determine the belonging sum type and type arguments,
while in this language types must be explicitly writable, and inference flows unidirectionally from
the expected position (same discipline as RFC-011a "Self is an explicit type parameter, no magic").
Under the qualified-name form, the type is completely self-sufficient and does not require context.
If a bare name is introduced in the future, it can only be used as a "context-uniquely-determined"
expansion sugar, and the semantic baseline remains the qualified name in this section.

#### Inference: Type Self-Sufficiency

After the qualified name gives the complete type arguments, the signature of the variant constructor
is the result of **the field signature substituted with type arguments**:

```yaoxiang
Result(Int, String).ok   // : (Int) -> Result(Int, String)
```

The payload type check is just the ordinary function call argument check (mismatch reports E1002),
with no new inference rules. Constructors of generic sum types are naturally monomorphized with type
instantiation, without a separate mechanism.

#### Field Promotion: Variant Names Are Not Data Fields

After being determined as a sum type, the variant names are removed from the data field
space—accessing the variant-name field of a **value** of the sum type is rejected at compile time:

```yaoxiang
r1.ok    // Compile error: ok is a variant constructor of Result, not a data field
```

Reason: A sum type value is a tagged union at runtime (see below), and does not carry a variant
declaration table; allowing field access would inevitably be silently mis-translated. This is an
extension of the same discipline as RFC-011a §1.2 (unified namespace for fields/methods, conflict
errors). Use type qualification for construction (`Result.ok`), and use match for deconstruction
(RFC-010b).

#### Runtime Representation and Equality

The runtime representation of a sum type value is a tagged union:
`Enum { type identity, variant_id, payload }`.

- **Type identity** carries the concrete sum type (not a global placeholder)—values across sum types
  are not comparable;
- **variant_id** is numbered according to the **declaration order** of the variants in the type
  body, which is also the variant set input for the RFC-010b exhaustiveness check;
- **Equality** (`==`/`!=`): Same type identity, same variant_id, and value-by-value equality of
  payloads (recursive).

#### std Migration

`std.result` is changed to use the mechanism in this section to define `ok` / `err` (native
`result_ok` / `result_err` are demoted), and `std`'s variant construction goes through the same set
of rules as user sum types, with no privileged channel. The parser special-cases of `Result` /
`Option` at type positions and the removal of name-based special-cases of `make_result` are
undertaken by [RFC-011b](./011b-operator-overloading.md) Phase 2—after the `?` interface-ization,
`Result` becomes an ordinary std sum type.

### Logical Operators: `and` / `or` / `!` (Authoritative Definition, Zig-Style)

> **Definition announcement (2026-08-03)**: The authoritative form of the logical operators is the
> keywords `and` / `or` plus the symbol unary `!` (consistent with SPEC `syntax.md` §2.2 priority
> table). This design aligns with Zig: **short-circuit control flow uses keywords, pure unary
> operations use symbols**. The early implementation's drift toward C-style `&&` / `||` and the
> intermediate state's keyword `not` have all been removed.

**Semantics**:

| Operator | Priority (SPEC §2.2)            | Associativity | Semantics                                    |
| -------- | ------------------------------- | ------------- | -------------------------------------------- |
| `!`      | 3 (unary prefix, tight binding) | Right to left | Logical NOT (pure function, no control flow) |
| `and`    | 10                              | Left to right | Short-circuit logical AND                    |
| `or`     | 10                              | Left to right | Short-circuit logical OR                     |

```yaoxiang
# Short-circuit evaluation: when the left side of `and` is false / the left side of `or` is true, the right side is not executed
if x != 0 and y / x > 1 { ... }   # No division by zero when x == 0

# Tight binding: !a == b ≡ (!a) == b (Zig-style; opposite to Python's not a == b ≡ not (a == b))
!3 == 4          # false: (!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs)), negate after the call
```

The following writing styles are **no longer supported** (lexer reports an error and suggests the
corresponding style):

```
x && y     # ❌ Removed, use x and y
x || y     # ❌ Removed, use x or y
not x      # ❌ Removed, use !x (not is restored as an ordinary identifier; != is unaffected)
```

**Design rationale** (aligning with Zig, ziglang/zig#272 / #6625):

1. **Short-circuiting is control flow → keyword; pure function is operation → symbol**. `and` / `or`
   change the order of evaluation (the right side is skipped on demand), which is of the same nature
   as `if`, so keywords are used; `!` performs pure negation on an already-evaluated operand, which
   is of the same nature as `-` `+`, so a symbol is used. YaoXiang uses `?` for error propagation
   (§2.11), so `!` has no conflict.
2. **Tight binding eliminates ambiguity**: `!` visually "sticks" to the operand, and the high
   priority is clear at a glance; the keyword `not` is forced to leave a space from the operand, and
   which side it binds to (`not a == b`) easily causes mental ambiguity.
3. **Disambiguation**: `&` serves two purposes—the borrow token (`&p` / `&mut p`, RFC-009) and
   bitwise AND (SPEC §2.2 priority 8). Introducing `&&` as well would make one symbol carry three
   meanings. `and` / `or` / `!` make borrowing, bitwise operations, and logic completely separated
   visually.
4. **Precedent**: Zig (a modern systems language in the same ecological niche) is exactly the
   combination of `and` / `or` keywords + `!` symbol; Python / Lua / Ada / SQL use all keywords
   (including `not`), and the C family uses all symbols—YaoXiang adopts Zig's mix, getting the best
   of both.
5. **Curry-Howard consistency**: Types as propositions (see the corresponding section above),
   writing the logical conjunction in refinement types as `and` / `or` (e.g.,
   `{ 0 <= idx and idx < arr.len }`) is a natural expression of the proposition; `!` as a unary
   negation symbol corresponds to ¬.

> **Implementation**: `and` / `or` is expanded into short-circuit jump sequences at the IR layer
> (`a and b ≡ if a { b } else { false }`), and `!` is parsed according to unary tight binding
> (operand according to `BP_UNARY + 1`). Regression tests:
> `tests/yaoxiang/01-syntax/basics/logical_ops.yx`, `logical_not.yx`.

## Syntax Design Note: Named Functions Are Essentially Syntax Sugar for Lambdas

### Core Understanding

**Named functions and lambda expressions are the same thing!** The only difference is that a named
function gives the lambda a name.

```yaoxiang
// These two are essentially identical
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

**Key point**: When the signature fully declares the parameter types, the parameter names in the
lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: The parameters in the signature have a scope that overrides
the function body, and the inner scope has higher priority.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x overrides the outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be placed at any of the following positions, **at least one annotation is
required**:

| Annotation position  | Form                                     | Description              |
| -------------------- | ---------------------------------------- | ------------------------ |
| Signature only       | `double: (x: Int) -> Int = x * 2`        | ✅ Recommended           |
| Lambda head only     | `double = (x: Int) => x * 2`             | ✅ Legal                 |
| Both sides annotated | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Example

```yaoxiang
// ✅ Recommended: signature complete, lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: type annotations in the lambda head
double = (x: Int) => x * 2

// ✅ Legal: annotations on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature        | Advantage                                                                       |
| -------------- | ------------------------------------------------------------------------------- |
| **Concise**    | No need to repeat parameter names when the signature is complete                |
| **Flexible**   | Lambda form is preserved, use whichever you like                                |
| **Consistent** | Maintains a unified pattern with variable declaration `x: Int = 42`             |
| **Intuitive**  | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage            | Description                                               |
| -------------------- | --------------------------------------------------------- |
| Extreme unification  | A single syntax rule covers all cases                     |
| Theoretical elegance | Perfectly symmetric `name: type = value`                  |
| No new keywords      | Reuse existing syntax elements                            |
| Easy to implement    | The compiler only needs to handle one form of declaration |
| Easy to learn        | Remember one pattern and you can write all code           |
| Easy to extend       | New features can fit naturally into this model            |

### Disadvantages

| Disadvantage      | Description                                                                      |
| ----------------- | -------------------------------------------------------------------------------- |
| Naming convention | Methods must follow the `Type.method` naming convention                          |
| Verbosity         | The full syntax is longer than the simplified syntax, but inference is available |
| Learning curve    | The unified model needs to be understood                                         |

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

| Risk                 | Impact                                             | Mitigation                                 |
| -------------------- | -------------------------------------------------- | ------------------------------------------ |
| Parsing complexity   | The unified syntax may increase parsing complexity | Use a recursive descent parser             |
| Performance overhead | vtable lookup may have additional overhead         | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: The Source of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: This is the **ineffable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One begets two, two beget three, three beget all things.    ║
║   The Yi has the Supreme Ultimate, which begets the Two Modes.║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.  ║
║   The compiler falls silent here, philosophy pauses here.     ║
║                                                              ║
║   Thank you for reaching the philosophical boundary of the language.║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would lead to the
> Type0/Type1 universe paradox), but we specifically keep this "easter egg"—when you try to compile
> it, you will receive a Zen message from the language's founder. This is not only a technical
> boundary, but also a tribute from YaoXiang to the philosophy of types.

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
       | identifier '(' type_expr (',' type_expr)* ')'      # Type application
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # Function type
       | '{' type_field* '}'                       # Record/interface type
       | 'Type'                                    # Meta-type

type_field ::= identifier ':' type_expr
             | identifier                           # Interface constraint

# Generic parameters: as part of the function type, e.g. (T: Type, R: Type) -> (...)
# No independent BNF rule needed — : Type parameters are ordinary function parameters

# Expressions
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

| Term               | Definition                                                                                                  |
| ------------------ | ----------------------------------------------------------------------------------------------------------- |
| Declaration        | An assignment statement in the form `name: type = value`                                                    |
| Record type        | A `{ ... }` type containing named fields                                                                    |
| Interface          | A record type whose fields are all function types                                                           |
| Generic type       | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters                            |
| Namespace function | A function in the form `Type.name`, belonging to the Type namespace. Implies no binding                     |
| Method binding     | `Type.name = func[n]`, binds position n of func as the caller, making the `obj.name(args)` syntax available |
| Generic function   | A function using the `(T: Type)` syntax, with type parameters as the first parameter group                  |
| Meta-type          | `Type`, the only type-hierarchy marker in the language                                                      |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Reviewing  │  ← Open community discussion and feedback
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
│ (Formal design) │  │ (Kept in place) │
└─────────────┘    └─────────────┘
```
