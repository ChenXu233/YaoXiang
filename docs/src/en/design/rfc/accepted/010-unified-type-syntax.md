---
title: 'RFC-010: Unified Type Syntax - name: type = value Model'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-14 (Never builtin type implemented, #157 closed)'
issue: '#127'
---

# RFC-010: Unified Type Syntax - name: type = value Model

## Summary

This RFC proposes an extremely minimalist unified type syntax model: **everything is
`name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression and `expression` can be any value expression. **There is no
`fn`, no `struct`, no `trait`, no `impl`, and no lowercase `type` keyword (though there is `Type` as
a meta type keyword)**.

> **Core Design**: `Type` itself is a generic type. `(T: Type) -> Type` represents "a type that
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

**`Type` is the only meta type keyword in the language**.

> **Namespace vs Method Binding**: The prefix `Type.name` indicates **namespace ownership**—nothing
> more. It does not trigger any implicit binding. To make `.` call syntax like `p.draw(screen)`
> work, you must explicitly bind: `Point.draw = draw[0]`. See the "Namespace and Method Binding"
> section below for details. It is used to mark type hierarchy, and the compiler automatically
> handles the distinction between Type0, Type1, Type2..., which is transparent to users.

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
    return "Point(${self.x}, ${self.y})"
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
p.draw(screen)           // Syntactic sugar → Point.draw(p, screen)
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

These concepts lack unity among each other, leading to fragmented syntax and a high learning cost.

### Design Goals

1. **Extreme unity**: One syntax rule covers all cases
2. **Concise and elegant**: Symmetric aesthetics of `name: type = value`
3. **No new keywords**: Reuse existing syntax elements
4. **Theoretically elegant**: Types themselves are values of type Type
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 is **naturally compatible** with the generics system design of
RFC-011, allowing generic parameters to seamlessly integrate into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type positions in signature can be omitted, auto-inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraint (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:

- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 synchronously with RFC-010

## Proposal

### Core Principle: Type Constructor vs Function/Variable

**This is a key design choice that determines the disambiguation rules of the syntax:**

| Form                | Meaning              | Rule                                                  |
| ------------------- | -------------------- | ----------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | `: Type` explicit declaration → forced to be a type   |
| **`f = ...`**       | Function or variable | No `: Type` → HM actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:

- `{ x: Float, y: Float }` could be a **type literal** (record type)
- `{ a = 1 + 1 }` could be a **code block** (executed statement, returns Void)

**Disambiguation rules**:

- **Has** `: Type` → forced to resolve as a type constructor, `{ ... }` is a type literal
- **No** `: Type` → HM actively resolves `{ ... }` as a code block, infers as a function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main = { println("Hello") }

# ❌ Error: no : Type, compiler cannot resolve { ... } as a type
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
│       Point.draw = draw[0]  # Dot call syntax only available after explicit binding
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Meta Type Hierarchy (Compiler Internal)

**The compiler internally** maintains a universe hierarchy `level: selfpointnum` (stored as a
string, theoretically infinitely extensible).

| Level    | Description                           |
| -------- | ------------------------------------- |
| `Type0`  | Daily types (`Int`, `Float`, `Point`) |
| `Type1`  | Type constructors (`List`, `Maybe`)   |
| `Type2+` | Higher-order constructors             |

**Users never see these numbers**, they only see `: Type`.

### Curry-Howard Isomorphism: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not arbitrarily chosen—it is a direct mapping of
the Curry-Howard correspondence. This isomorphism reveals a profound fact: **the type system and the
logical system are two sides of the same coin**.

| Logic (Proposition)                | Type System (YaoXiang)              | Example                              |
| ---------------------------------- | ----------------------------------- | ------------------------------------ |
| Proposition P                      | Type T                              | `Int`, `Bool`                        |
| Proof that P holds                 | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication)                | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)                | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal quantification) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)                | Enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "There exists a proof of type Int, named x, with value 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "There exists an implication proof: given proofs a and b of type Int, a proof of type Int can be constructed"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If the type system allows constructing a value of type `T`
   without any legal runtime representation, it is like allowing a proof of a false proposition in
   logic—the system collapses. Curry-Howard tells us: **a type-safe language is naturally a
   logically consistent system**.

2. **Universe hierarchy is a necessary condition**: As detailed below, allowing `Type: Type` (i.e.,
   "the type of types is also a type") would produce the Russell paradox (manifested as Girard's
   paradox in type theory). YaoXiang's stratification of `Type₀ : Type₁ : Type₂ : ...` ensures that
   each type belongs to only one level, forming an upward chain that never closes, fundamentally
   avoiding paradoxes. This means YaoXiang's type system is **logically consistent** in the
   Curry-Howard sense.

3. **Theoretical foundation of unified syntax**: The reason why `name: type = value` can use one
   syntax to cover all concepts—variables, functions, types, interfaces, generics—is that under
   Curry-Howard they are all the same thing—**providing proofs for propositions**. Variables are
   evidence of propositions, functions are evidence of implications, records are evidence of
   conjunctions, generics are evidence of universal quantification. Unified syntax is not a
   coincidental human design, but a natural corollary of the Curry-Howard isomorphism.

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

```yaoxiang
// Single expression form (directly returns the value, no return needed)
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

The return value depends on the form to the right of `=`:

| Form                      | Return Value                                |
| ------------------------- | ------------------------------------------- |
| `= expr` (no braces)      | Directly returns `expr`                     |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` |

```yaoxiang
# Single expression: directly returns the value, no return needed
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

> **Design rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its
> return semantics differ from a single expression. Braces introduce a multi-statement context, so
> an explicit `return` is needed to eliminate the ambiguity of "whether the last expression is the
> return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

In YaoXiang, `{ ... }` is more than just a code block—it is a **dependency-driven computation
unit**. This semantics is consistent in function bodies, variable initialization, and `spawn`:

**Core rules**:

- Assignment statements inside `{}` are automatically ordered by dependency, not by writing order
- Execution happens immediately once dependencies are met; otherwise blocked waiting
- Use `return` to explicitly return a value (see Return Rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler automatically orders
result: Int = {
    b = a + 1      # Depends on a → automatically placed after a
    a = 10         # No dependency → can be executed first
    return b       # Returns 11
}
```

> **Difference from single expression**: `= expr` (no braces) is a simple binding that directly
> returns the value; `= { ... }` (with braces) introduces a dependency-driven computation context,
> allowing multi-statement and explicit `return`.

#### `spawn` Block

`spawn { ... }` is YaoXiang's sole parallel primitive. It leverages the dependency-driven semantics
of `{}` to implement automatic parallelization:

- Direct child assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks with satisfied dependencies execute concurrently immediately
- The caller blocks waiting for all child tasks to complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # Depends on a, b → executes after both complete
    return c
}
// Caller blocks here until all tasks in the spawn block complete
```

> **Detailed definition**: For the complete semantics of `spawn`, task creation rules, and blocking
> model, see `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and operate on raw pointers. It leverages the return
semantics of `{}` to return type definitions to the enclosing scope:

**Core rules**:

- Types can be defined and raw pointers operated on inside `unsafe {}`
- Use `return` to return the type definition to the enclosing scope
- The returned type is available outside `unsafe {}`
- Field access of the type requires unsafe permission

```yaoxiang
# Define an opaque type in an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # Raw pointer
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

> **Detailed definition**: For the complete semantics of `unsafe`, FFI type definitions, and method
> binding, see `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound
methods, and interface implementation:

##### Basic Types

**Record type**: A field list, where field types can be any type expression.

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

YaoXiang's identifier system has three layers, recognized by different compiler phases in sequence:

1. **Keywords** (parser independent tokens) — control structures and declaration keywords, such as
   `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser independent tokens) — `true`, `false`, `void`, `Type`, cannot
   be used as ordinary identifiers
3. **Built-in type names** (pre-registered by the type checker) — the parser treats them as ordinary
   identifiers, the type checker is responsible for parsing. **Not reserved words, can be shadowed
   (not recommended)**

The difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, built-in
type name): `void` is a value literal (equal to the unique value of Unit), and `Void` is a type name
(equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Predefined built-in type names:

| Type     | Logical Correspondence | Description                                                                                                                                                                                                                                                                 |
| -------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥ (false/empty type)   | Zero constructor, no value can inhabit this type. Represents "impossible"—divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` means it never returns normally. **Not a keyword, but a built-in type name.** |
| `Void`   | ⊤ (true/Unit)          | Exactly one inhabitant (default void value). `x: Void = <default>` is legal. Corresponds to the identity element of sum type and product type—`Void` is a zero-field product type (Unit), `Never` is a zero-variant sum type.                                               |
| `Int`    | —                      | Signed integer                                                                                                                                                                                                                                                              |
| `Float`  | —                      | Floating point number                                                                                                                                                                                                                                                       |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                                                                                                                                                                                                             |
| `Char`   | —                      | Unicode character                                                                                                                                                                                                                                                           |
| `String` | —                      | String                                                                                                                                                                                                                                                                      |

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

**Interface names are written in the type body, and the compiler automatically checks its
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
    Drawable,          // Implements Drawable interface
    Serializable      // Implements Serializable interface
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

**The `Type.name` prefix indicates namespace ownership**—nothing more. It does not trigger any
implicit binding.

```yaoxiang
// Namespace function: an ordinary function in the Point namespace
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

// Call: just an ordinary function call
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing it as `p`, `this`,
> `x` works exactly the same. The compiler doesn't look at parameter names, it looks at types.

##### Method Binding (The Only Way)

To make `.` method call syntax like `p.draw(screen)` work, **explicit binding is required**. The
`[position]` syntax is the only mechanism to bind a function as a "method" (see RFC-004 for detailed
syntax).

```yaoxiang
// Define function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this is p.draw(screen) syntax available
Point.draw = draw[0]   // The parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // Syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // Both calling styles are equivalent

// Without [0] = no binding. Point.draw is just an ordinary function alias, no . syntax
Point.draw = draw       // No binding: only Point.draw(p, screen) works
```

**Default behavior**: No `[n]` = no parameter is bound. The user must explicitly decide which
parameters are filled by the caller.

**Multi-positional binding**:

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

// Generic method (RFC-023 syntax: type parameters are auto-inferred at call site)
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

Generic types and generic function calls uniformly use the `()` syntax. `[]` is not used in any
generic context.

**Core rules**:

1. **`()` does everything**: Type application, function calls, value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function call—type flows automatically from arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value`—Type parameters are declared on
   the left, the right side is always concrete values. The `T` of an empty container `List()` must
   be obtained from the left-side type annotation.

3. **Type information only needs to be written once**—declared at the parameter declaration, the
   compiler carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically come from numbers and f's types
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       // Inferred as List(Int)
y = List("a", "b")      // Inferred as List(String)
z = List()              // ❌ Compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int comes from the left annotation
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
// Interfaces don't need self parameters — they only define "function signatures with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // Returns interface type, concrete implementation returns its own type
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

// Explicit binding — only after binding is dot call syntax available
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

// Ordinary method call (direct call)
d: Float = distance(p, Point(0.0, 0.0))

// Chained call
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// Interface assignment
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// Generic function (RFC-023 syntax: type parameters omitted at call site, auto-inferred)
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
        // Check if the type has a method of the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check if method signatures are compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Compare: after removing the self parameter, they should match
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
// Direct assignment of concrete type → compile-time deterministic concrete type, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → compile-time cannot determine concrete type, use vtable
d: Drawable = get_shape()
d.draw(screen)  // Look up method through vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Look up method through vtable
}
```

**Compile-time optimization strategy**:

| Scenario                         | Inference Result     | Call Method                 |
| -------------------------------- | -------------------- | --------------------------- |
| `d: Drawable = Circle(1)`        | Concrete type Circle | Direct call (zero-overhead) |
| `d: Drawable = get_shape()`      | Unknown              | vtable                      |
| `shapes: List(Drawable) = [...]` | Heterogeneous        | vtable                      |

**Rules**:

1. When the right-hand side is a concrete type constructor and can be determined at compile time,
   generate direct call IR
2. When the right-hand side type cannot be determined at compile time, fall back to the vtable
   mechanism
3. The vtable fallback ensures the correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as there are the same methods, it can be assigned to an interface type
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
| Requires `impl` keyword                  | No keyword needed, interface name written after the type body                                |

### Deprecated: `|` Variant Syntax

> **Deprecation announcement (2026-07-25)**: The `|` variant syntax is officially deprecated and
> removed from the implementation.

The following forms are **no longer supported**:

```
type Color = red | green | blue                # ❌ Deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ Deprecated
type Option(T) = some(T) | none                # ❌ Deprecated
```

Uniformly use record types to express sum types. When the fields of a record type are all functions
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

1. **Eliminate special cases**: `|` is the only non-`name: type = value` form syntax in BNF. After
   removal, the `type_expr` production is completely unified, and the parser no longer needs to
   maintain independent paths and lookahead fallbacks for variant types.
2. **Mathematical equivalence**: Under the Curry-Howard isomorphism, the sum type corresponding to
   disjunction P ⊕ Q is equivalent to a record type whose fields are all "functions returning the
   type itself". Both express the same semantics, no need for two syntaxes.
3. **Zero destructiveness**: Before removal, the `|` syntax was half-supported in the parser
   (parameterless variants could be parsed but parameter types were lost during monomorphization),
   and no user code depended on it.
4. **AST simplification**: The `Type::Variant(Vec<VariantDef>)` node is removed, all variant types
   uniformly go through the `Type::Struct` path, and the special branches in downstream
   typecheck/mono/formatter are all eliminated.

> **Note**: The semantic properties of sum types (such as match exhaustiveness checking, tagged
> union memory layout) are derived by the typecheck layer from the `Type::Struct` structure, not
> relying on independent AST nodes.

### Logical Operators: `and` / `or` / `!` (Authoritative Definition, Zig-style)

> **Definition announcement (2026-08-03)**: The authoritative form of logical operators is the
> keywords `and` / `or` + symbol unary `!` (consistent with SPEC `syntax.md` §2.2 priority table).
> This design aligns with Zig: **short-circuit control flow uses keywords, pure unary operations use
> symbols**. The early implementation that drifted toward C with `&&` / `||` and the intermediate
> keyword `not` have all been removed.

**Semantics**:

| Operator | Priority (SPEC §2.2)            | Associativity | Semantics                                    |
| -------- | ------------------------------- | ------------- | -------------------------------------------- |
| `!`      | 3 (unary prefix, tight binding) | Right-to-left | Logical NOT (pure function, no control flow) |
| `and`    | 10                              | Left-to-right | Short-circuit logical AND                    |
| `or`     | 10                              | Left-to-right | Short-circuit logical OR                     |

```yaoxiang
# Short-circuit evaluation: when left of and is false / left of or is true, right side is not executed
if x != 0 and y / x > 1 { ... }   # When x == 0, no division by zero

# Tight binding: !a == b ≡ (!a) == b (Zig-style; opposite to Python's not a == b ≡ not (a == b))
!3 == 4          # false: (!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs)), negated after the call
```

The following forms are **no longer supported** (lexer reports an error and suggests the
corresponding form):

```
x && y     # ❌ Removed, use x and y
x || y     # ❌ Removed, use x or y
not x      # ❌ Removed, use !x (not is restored as an ordinary identifier; != is unaffected)
```

**Design rationale** (aligned with Zig, ziglang/zig#272 / #6625):

1. **Short-circuit is control flow → keywords; pure function is operation → symbols**. `and` / `or`
   change the evaluation order (right side is skipped on demand), same nature as `if`, so use
   keywords; `!` performs pure negation on an already-evaluated operand, same nature as `-` `+`, so
   use symbols. YaoXiang uses `?` for error propagation (§2.11), so `!` has no conflict.
2. **Tight binding eliminates ambiguity**: `!` visually "sticks" to the operand, and the high
   priority is clear at a glance; the keyword `not` is forced to leave a space with the operand,
   making it easy to cause mental ambiguity about which side it binds to (`not a == b`).
3. **Disambiguation**: `&` serves two roles—the borrow token (`&p` / `&mut p`, RFC-009) and bitwise
   AND (SPEC §2.2 priority 8). Introducing `&&` would let one symbol carry three meanings. `and` /
   `or` / `!` visually separate the three concepts of borrow, bitwise operation, and logic
   completely.
4. **Precedent**: Zig (a modern system language in the same ecological niche) is exactly the
   combination of `and` / `or` keywords + `!` symbols; Python / Lua / Ada / SQL use all keywords
   (including `not`); the C family uses all symbols—YaoXiang takes Zig's hybrid, getting the best of
   both.
5. **Curry-Howard consistency**: Types are propositions (see the isomorphism section above), logical
   conjunctions in refined types are written as `and` / `or` (e.g.,
   `{ 0 <= idx and idx < arr.len }`) is the natural expression of propositions; `!` as a unary
   negation symbol corresponds to ¬.

> **Implementation**: `and` / `or` are expanded into short-circuit jump sequences at the IR level
> (`a and b ≡ if a { b } else { false }`), `!` is parsed as a unary tight binding (operand takes
> `BP_UNARY + 1`). Regression tests: `tests/yaoxiang/01-syntax/basics/logical_ops.yx`,
> `logical_not.yx`.

## Syntax Design Notes: Named Functions are Essentially Syntactic Sugar for Lambdas

### Core Understanding

**Named functions and Lambda expressions are the same thing!** The only difference is that named
functions give a Lambda a name.

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

**Key point**: When the signature completely declares parameter types, the parameter names in the
Lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: The parameter scope in the signature overrides the function
body, and the inner scope has higher priority.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x overrides outer x, result is 20
```

### Flexible Annotation Position

Type annotations can be in any of the following positions, **at least one must be annotated**:

| Annotation Position | Form                                     | Description              |
| ------------------- | ---------------------------------------- | ------------------------ |
| Signature only      | `double: (x: Int) -> Int = x * 2`        | ✅ Recommended           |
| Lambda head only    | `double = (x: Int) => x * 2`             | ✅ Legal                 |
| Both annotated      | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Example

```yaoxiang
// ✅ Recommended: signature complete, Lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: type annotated in Lambda head
double = (x: Int) => x * 2

// ✅ Legal: annotated on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature        | Advantage                                                                       |
| -------------- | ------------------------------------------------------------------------------- |
| **Concise**    | No need to repeat parameter names when the signature is complete                |
| **Flexible**   | Keep the Lambda form, use whichever you prefer                                  |
| **Consistent** | Stays in the unified pattern of `x: Int = 42`                                   |
| **Intuitive**  | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage            | Description                                        |
| -------------------- | -------------------------------------------------- |
| Extreme unity        | One syntax rule covers all cases                   |
| Theoretical elegance | Perfectly symmetric `name: type = value`           |
| No new keywords      | Reuse existing syntax elements                     |
| Easy to implement    | Compiler only needs to handle one declaration form |
| Easy to learn        | Remember one pattern and you can write all code    |
| Easy to extend       | New features can naturally fit into this model     |

### Disadvantages

| Disadvantage      | Description                                                       |
| ----------------- | ----------------------------------------------------------------- |
| Naming convention | Methods must follow the `Type.method` naming convention           |
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
// Type can be omitted and inferred by the compiler
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

**Warning**: This is the **unspeakable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One gives birth to two, two gives birth to three, three gives birth to the myriad things.    ║
║   Change has the supreme ultimate, which gives birth to the two forms.                        ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of language.  ║
║   The compiler falls silent here, philosophy pauses here.    ║
║                                                              ║
║   Thank you for reaching the philosophical boundary of the language.                          ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause a Type0/Type1
> universe paradox), but we intentionally keep this "easter egg"—when you try to compile it, you
> will receive a Zen message from the language's founder. This is not only a technical boundary, but
> also YaoXiang's tribute to type philosophy.

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

# Generic parameter: as part of function type, e.g., (T: Type, R: Type) -> (...)
# No independent BNF rule needed—: Type parameter is an ordinary function parameter

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

| Term               | Definition                                                                                              |
| ------------------ | ------------------------------------------------------------------------------------------------------- |
| Declaration        | An assignment statement in the form `name: type = value`                                                |
| Record type        | A `{ ... }` type containing named fields                                                                |
| Interface          | A record type whose fields are all function types                                                       |
| Generic type       | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters                        |
| Namespace function | A function in the form `Type.name`, belonging to the Type namespace. Implies no binding                 |
| Method binding     | `Type.name = func[n]`, binds position n of func as the caller, making `obj.name(args)` syntax available |
| Generic function   | A function using the `(T: Type)` syntax, with type parameters as the first parameter group              |
| Meta type          | `Type`, the only type hierarchy marker in the language                                                  |

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
│ (Official design) │ (Kept in place) │
└─────────────┘    └─────────────┘
```
