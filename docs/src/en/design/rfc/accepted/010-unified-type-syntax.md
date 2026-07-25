---
title: "RFC-010: Unified Type Syntax - name: type = value Model"
status: "Accepted"
author: "Chenxu"
updated: "2026-07-14 (Never builtin type implemented, #157 closed)"
issue: "#127"
---
# RFC-010: Unified Type Syntax - name: type = value Model


## Summary

This RFC proposes an extremely minimal and unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

where `type` can be any type expression, and `expression` can be any value expression.
**There is no `fn`, no `struct`, no `trait`, no `impl`, and no lowercase `type` keyword (but there is `Type` as the meta type keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that accepts a type parameter T".

| Concept | Code |
|---------|------|
| Variable | `x: Int = 42` |
| Function | `add: (a: Int, b: Int) -> Int = a + b` |
| Record type | `Point: Type = { x: Float, y: Float }` |
| Interface | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic type | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| Generic type | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| Method | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` is the only meta type keyword in the language**.

> **Namespace vs. Method Binding**: The `Type.name` prefix indicates **namespace ownership**—nothing more.
> It does not trigger any implicit binding. For the `.` call syntax like `p.draw(screen)` to work,
> an explicit binding is required: `Point.draw = draw[0]`.
> See the "Namespace and Method Binding" section below for details.
It is used to mark the type hierarchy; the compiler automatically handles the distinction between Type0, Type1, Type2..., transparently to the user.

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

1. **Ultimate unity**: One syntax rule covers all cases
2. **Concise and elegant**: The symmetric aesthetics of `name: type = value`
3. **No new keywords**: Reuse existing syntax elements
4. **Theoretical elegance**: Types themselves are values of type Type
5. **Generics-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 and the generics system design of RFC-011 are **naturally compatible**, and generics parameters can be seamlessly integrated into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: the Type position in the signature can be omitted, automatically inferred at the call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraint (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by the parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:
- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generics examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 and RFC-010 simultaneously

## Proposal

### Core Principle: Type Constructors vs. Functions/Variables

**This is a key design choice that determines the disambiguation rules for the syntax:**

| Form | Meaning | Rule |
|------|---------|------|
| **`x: Type = ...`** | Type constructor | Explicit `: Type` declaration → forced to be a type |
| **`f = ...`** | Function or variable | No `: Type` → HM actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:
- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executable statement, returning Void)

**Disambiguation rules**:
- **With** `: Type` → forced to be parsed as a type constructor, `{ ... }` is a type literal
- **Without** `: Type` → HM actively parses `{ ... }` as a code block, inferred as a function type

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
│       Point.draw = draw[0]  # Only after explicit binding does the dot-call syntax become available
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Meta Type Hierarchy (Compiler Internal)

The **compiler internally** maintains a universe hierarchy `level: selfpointnum` (stored as a string, theoretically extendable infinitely).

| Level | Description |
|-------|-------------|
| `Type0` | Everyday types (`Int`, `Float`, `Point`) |
| `Type1` | Type constructors (`List`, `Maybe`) |
| `Type2+` | Higher-order constructors |

**Users never see these numbers**, only `: Type`.

### Curry-Howard Correspondence: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not chosen arbitrarily—it is a direct mapping of the Curry-Howard correspondence. This correspondence reveals a profound fact: **type systems and logic systems are two sides of the same thing**.

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
// "x: Int = 42" reads as: "there exists a proof of Int, named x, with value 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "there exists an implication proof: given proofs a and b of Int, we can construct a proof of Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If a type system allows constructing a value of type `T` with no valid runtime representation, that is like allowing a proof of a false proposition in logic—the system breaks. Curry-Howard tells us: **a type-safe language is naturally a logically consistent system**.

2. **The universe hierarchy is a necessary condition**: As detailed below, if `Type: Type` were allowed (i.e., "the type of types is itself a type"), it would yield Russell's paradox (manifested as Girard's paradox in type theory). YaoXiang's `Type₀ : Type₁ : Type₂ : ...` stratification ensures each type belongs to exactly one level, forming an ever-rising chain that never closes, fundamentally avoiding paradoxes. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

3. **The theoretical foundation of the unified syntax**: The reason `name: type = value` can cover variables, functions, types, interfaces, and generics with a single syntax is precisely because they are all the same thing under Curry-Howard—**providing proofs for propositions**. Variables are evidence of propositions, functions are evidence of implications, records are evidence of conjunctions, generics are evidence of universal quantification. The unified syntax is not a coincidental design choice, but a natural consequence of the Curry-Howard correspondence.

> **Further reading**: Wadler, P. (2015). *"Propositions as Types."* Communications of the ACM, 58(12), 75–84. This article explains the history and significance of the Curry-Howard correspondence in an accessible way.

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

The return value depends on the form to the right of `=`:

| Form | Return Value |
|------|--------------|
| `= expr` (no braces) | Returns `expr` directly |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` |

```yaoxiang
# Single expression: returns directly, no return needed
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

> **Design rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its return semantics differ from single expressions. Braces introduce a multi-statement context, so an explicit `return` is needed to eliminate the ambiguity of "whether the last expression is the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

In YaoXiang, `{ ... }` is not just a code block—it is a **dependency-driven computation unit**. This semantics is consistent across function bodies, variable initializations, and `spawn`:

**Core rules**:
- Assignment statements within `{}` are automatically sorted by dependency, not by writing order
- Execution begins immediately when dependencies are satisfied; blocked when missing
- Use `return` to explicitly return a value (see return rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler automatically orders
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependencies → can execute first
    return b       # returns 11
}
```

> **Difference from single expressions**: `= expr` (no braces) is a simple binding that directly returns the value; `= { ... }` (with braces) introduces a dependency-driven computation context, allowing multiple statements and explicit `return`.

#### `spawn` Block

`spawn { ... }` is YaoXiang's only parallel primitive. It leverages the dependency-driven semantics of `{}` to achieve automatic parallelization:

- Direct child assignments within `spawn { ... }` automatically create parallel tasks
- Tasks with ready dependencies execute concurrently
- The caller blocks until all child tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # depends on a, b → executes after both complete
    return c
}
// The caller blocks here until all tasks within the spawn block complete
```

> **Detailed definition**: The complete semantics of `spawn`, task creation rules, and blocking model are detailed in `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used to define opaque types and operate on raw pointers. It uses the return semantics of `{}` to return type definitions to the enclosing scope:

**Core rules**:
- Types can be defined and raw pointers operated on within `unsafe {}`
- Use `return` to return type definitions to the enclosing scope
- Returned types are available outside `unsafe {}`
- Accessing a type's fields requires unsafe permission

```yaoxiang
# Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw pointer
    }
    return SqliteDb
}

# SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: the handle field requires unsafe permission
handle = db.handle

# ✅ Access via method calls
db.close()
```

> **Detailed definition**: The complete semantics of `unsafe`, FFI type definitions, and method binding are detailed in `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, encompassing fields, default values, bound methods, and interface implementations:

##### Basic Types

**Record type**: a list of fields whose types can be any type expression.

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

##### Builtin Types

YaoXiang's identifier system is divided into three layers, recognized by different compiler phases:

1. **Keywords** (parser-independent tokens) — control structures and declaration keywords, e.g. `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser-independent tokens) — `true`, `false`, `void`, `Type`; cannot be used as ordinary identifiers
3. **Builtin type names** (pre-registered by the type checker) — the parser treats them as ordinary identifiers, and the type checker is responsible for parsing. **Not reserved words, can be shadowed (not recommended)**

The difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, builtin type name): `void` is a value literal (equal to the sole value of Unit), and `Void` is a type name (equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Predefined builtin type names:

| Type | Logical Correspondence | Description |
|------|------------------------|-------------|
| `Never` | ⊥ (false/empty type) | Zero constructors; no value can inhabit this type. Represents "impossibility"—divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` indicates it never returns normally. **Not a keyword, but a builtin type name.** |
| `Void` | ⊤ (true/Unit) | Exactly one inhabitant (the default `void` value). `x: Void = <default>` is legal. The identity element of sum types corresponds to the identity element of product types—`Void` is the zero-field product type (Unit), `Never` is the zero-variant sum type. |
| `Int` | — | Signed integer |
| `Float` | — | Floating-point number |
| `Bool` | — | Boolean: `true` / `false` |
| `Char` | — | Unicode character |
| `String` | — | String |

##### Bound Methods

**Method 1: Bind an external function directly inside the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // bound to position 0, curried into method: (b: Point) -> Float
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

**Interface names are written in the type body; the compiler automatically checks their implementation**

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

**The `Type.name` prefix indicates namespace ownership**, nothing more. It does not trigger any implicit binding.

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

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing it as `p`, `this`, or `x` has exactly the same effect.
> The compiler does not look at parameter names, but at types.

##### Method Binding (The Only Way)

For the `.` method call syntax like `p.draw(screen)` to work, **an explicit binding is required**.
The `[position]` syntax is the only mechanism for binding a function as a "method" (see RFC-004 for detailed syntax).

```yaoxiang
// Define a function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does the p.draw(screen) syntax become available
Point.draw = draw[0]   // the parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // both call forms are equivalent

// Omitting [0] = no binding. Point.draw is just an ordinary function alias, with no . syntax
Point.draw = draw       // not bound: only callable as Point.draw(p, screen)
```

**Default behavior**: omitting `[n]` = no parameter is bound. The user must explicitly decide which parameters are filled by the caller.

**Multi-position binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method to ordinary function):

```yaoxiang
// Extract the function from the binding
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. Interface Composition

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using an intersection type
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

// Generic method (RFC-023 syntax: type parameters automatically inferred at the call site)
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

Generic types and generic functions use the unified `()` syntax. `[]` is not used in any generic context.

**Core rules**:

1. **`()` does everything**: type application, function call, value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function call—types flow automatically from arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value`—Type parameters are declared on the left, the right side is always a concrete value. The `T` of an empty container `List()` must be obtained from the left-side type annotation.

3. **Type information only needs to be written once**—in the parameter declaration, the compiler carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String automatically from numbers and f's types
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

> **Comparison with the old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`, `List[Int](1,2,3)` → `List(1,2,3)`.
> The old `[]` generics syntax has been completely removed. `[]` is used only for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========
// Interface = a record type whose fields are all function types
// Interfaces don't need a self parameter — interfaces only define "the function signature with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // returns the interface type; concrete implementations return their own type
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

// Define a function (self is just a conventional name, not a keyword)
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

// Explicit binding — dot-call syntax only becomes available after binding
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

// Generic function (RFC-023 syntax: omit type parameters at call site, automatically inferred)
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
        // Check whether the type has a method of the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check whether the method signature is compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Compare: after removing the self parameter they should match
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

Interface types support direct assignment; the compiler automatically chooses the optimal call strategy based on the right-hand side's type:

```yaoxiang
// Direct assignment of a concrete type → concrete type determinable at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → concrete type cannot be determined at compile time, use vtable
d: Drawable = get_shape()
d.draw(screen)  // Method lookup via vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Method lookup via vtable
}
```

**Compile-time optimization strategy**:

| Scenario | Inferred Result | Call Method |
|----------|-----------------|-------------|
| `d: Drawable = Circle(1)` | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()` | Unknown | vtable |
| `shapes: List(Drawable) = [...]` | Heterogeneous | vtable |

**Rules**:
1. When the right-hand side is a concrete type constructor and determinable at compile time, generate direct-call IR
2. When the right-hand side's type cannot be determined at compile time, fall back to the vtable mechanism
3. vtable serves as a fallback to ensure the correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as the same method exists, it can be assigned to the interface type
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
| Requires `impl` keyword | No keyword needed; interface names are written after the type body |
### Deprecated: `|` Variant Syntax

> **Deprecation announcement (2026-07-25, issue #203)**: The `|` variant syntax is officially deprecated and removed from the implementation.

The following forms are **no longer supported**:

```
type Color = red | green | blue                # ❌ Deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ Deprecated
type Option(T) = some(T) | none                # ❌ Deprecated
```

Unified use of record types to express sum types. When a record type's fields are all functions, all returning the type itself, it is a sum type:

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

1. **Eliminate special cases**: `|` is the only non-`name: type = value` form in the BNF. After its removal, the `type_expr` production is fully unified, and the parser no longer needs to maintain a separate path with lookahead/backtracking for variant types.
2. **Mathematical equivalence**: Under the Curry-Howard correspondence, the sum type corresponding to disjunction P ⊕ Q is equivalent to a record type "whose fields are all functions returning the type itself". They express the same semantics and require no two separate syntaxes.
3. **Zero destructiveness**: Before removal, the `|` syntax was only half-supported in the parser (parameterless variants could be parsed but parameter types were lost during monomorphization), and no user code depended on it.
4. **AST simplification**: The `Type::Variant(Vec<VariantDef>)` node is removed; all variant types uniformly go through the `Type::Struct` path, eliminating all special-case branches in downstream typecheck/mono/formatter.

> **Note**: The semantic properties of sum types (such as `match` exhaustiveness checking, tagged union memory layout) are derived by the typecheck layer from the `Type::Struct` structure and do not depend on a dedicated AST node.

## Syntax Design Explanation: Named Functions Are Syntactic Sugar for Lambdas

### Core Insight

**Named functions and lambda expressions are the same thing!** The only difference is that a named function gives a lambda a name.

```yaoxiang
// These two are essentially completely identical
add: (a: Int, b: Int) -> Int = a + b           // Named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda form (fully equivalent)
```

### Syntactic Sugar Model

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially
name: (Params) -> ReturnType = (params) => body
```

**Key point**: When the signature fully declares parameter types, the parameter names in the lambda header become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: The parameter scope in the signature overrides the function body, and the inner scope takes precedence.

```yaoxiang
x = 10  // outer variable

double: (x: Int) -> Int = x * 2  // ✅ parameter x overrides outer x, result is 20
```

### Flexible Annotation Position

Type annotations can appear in any of the following positions; **at least one annotation is required**:

| Annotation Position | Form | Description |
|---------------------|------|-------------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda header only | `double = (x: Int) => x * 2` | ✅ Legal |
| Both sides | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete Example

```yaoxiang
// ✅ Recommended: signature complete, lambda header omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: type annotations in the lambda header
double = (x: Int) => x * 2

// ✅ Legal: both sides annotated
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature | Advantage |
|---------|-----------|
| **Concise** | No need to repeat parameter names when the signature is complete |
| **Flexible** | Lambda form preserved; use whichever you prefer |
| **Consistent** | Maintains the unified pattern with variable declaration `x: Int = 42` |
| **Intuitive** | `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Trade-offs

### Advantages

| Advantage | Description |
|-----------|-------------|
| Ultimate unity | One syntax rule covers all cases |
| Theoretical elegance | Perfectly symmetric `name: type = value` |
| No new keywords | Reuses existing syntax elements |
| Easy to implement | The compiler only needs to handle one declaration form |
| Easy to learn | Remember one pattern and you can write all code |
| Easy to extend | New features can fit naturally into this model |

### Disadvantages

| Disadvantage | Description |
|--------------|-------------|
| Naming convention | Methods must follow the `Type.method` naming |
| Verbosity | Full syntax is longer than simplified syntax, but inference is available |
| Learning curve | Requires understanding the unified model |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compile error example:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Types can be omitted; inferred by the compiler
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE automatically suggests missing methods
```

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Parsing complexity | Unified syntax may increase parsing complexity | Use a recursive descent parser |
| Performance overhead | vtable lookup may have additional overhead | Compile-time monomorphization optimization |

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
║   The Yi has the Supreme Ultimate, which gives birth to the Two Forms. ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of the language. ║
║   The compiler falls silent here; philosophy pauses here.     ║
║                                                              ║
║   Thank you for touching the philosophical boundary of the language. ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot correctly handle `Type: Type = Type` (it would cause the Type0/Type1 universe paradox), but we deliberately preserve this "easter egg"—when you try to compile it, you will receive a Zen message from the language's founder. This is not only a technical boundary, but also YaoXiang's tribute to the philosophy of types.

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

# Generic parameters: as part of a function type, e.g. (T: Type, R: Type) -> (...)
# No dedicated BNF rule needed—the : Type parameters are just ordinary function parameters

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
| Declaration | An assignment statement of the form `name: type = value` |
| Record type | A `{ ... }` type containing named fields |
| Interface | A record type whose fields are all function types |
| Generic type | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters |
| Namespace function | A function of the form `Type.name`, belonging to the Type namespace. Implies no binding |
| Method binding | `Type.name = func[n]`, binding the position n of func as the caller, making the `obj.name(args)` syntax available |
| Generic function | A function using the `(T: Type)` syntax, with type parameters as the first parameter group |
| Meta type | `Type`, the only type hierarchy marker in the language |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
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
│ (Formal design) │ │ (Kept in place) │
└─────────────┘    └─────────────┘
```