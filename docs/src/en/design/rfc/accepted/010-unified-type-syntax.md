---
title: 'RFC-010: Unified Type Syntax - name: type = value Model'
status: 'Accepted'
author: '晨煦'
updated: '2026-07-14 (Never built-in type implemented, #157 closed)'
issue: '#127'
---

# RFC-010: Unified Type Syntax - name: type = value Model

## Summary

This RFC proposes a minimalist unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression and `expression` can be any value expression. **No `fn`, no `struct`, no
`trait`, no `impl`, no lowercase `type` keyword (but there is `Type` as a meta type keyword)**.

> **Core Design**: `Type` itself is a generic type. `(T: Type) -> Type` represents "a type that accepts a type parameter T".

| Concept     | Code写法                                                                        |
| ----------- | ------------------------------------------------------------------------------- |
| Variable    | `x: Int = 42`                                                                   |
| Function    | `add: (a: Int, b: Int) -> Int = a + b`                                          |
| Record type | `Point: Type = { x: Float, y: Float }`                                          |
| Interface   | `Drawable: Type = { draw: (Surface) -> Void }`                                  |
| Generic type| `List: (T: Type) -> Type = { data: Array(T), length: Int }`                     |
| Generic type| `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`         |
| Method      | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]`    |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`  |

**`Type` is the only meta type keyword in the language**.

> **Namespace vs Method Binding**: `Type.name`
> The prefix indicates **namespace ownership**, nothing more. It does not trigger any implicit binding. To make `.`
> call syntax like `p.draw(screen)` work, you must explicitly bind: `Point.draw = draw[0]`. See the "Namespace and Method Binding" section below for details. It is used to annotate type hierarchy; the compiler automatically handles Type0, Type1, Type2... distinctions, which is transparent to users.

```yaoxiang
// Core syntax: unified + differentiated

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
p.draw(screen)           // Syntactic sugar → Point.draw(p, screen)
s: Drawable = p           // Structural subtyping: Point implements Drawable
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

These concepts lack uniformity, leading to syntactic fragmentation and high learning costs.

### Design Goals

1. **Extreme unification**: One syntactic rule covers all cases
2. **Concise and elegant**: Symmetric aesthetics of `name: type = value`
3. **No new keywords**: Reuse existing syntactic elements
4. **Theoretically elegant**: Types are also values of Type type
5. **Generic-friendly**: Seamless integration with the generics system (RFC-011)

### Integration with the Generics System

The unified syntax model of RFC-010 **naturally aligns** with the generics system design of RFC-011. Generic parameters can seamlessly integrate into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type position in signature can be omitted, automatically inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraint (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:

- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: Implement RFC-011 Phase 1 and RFC-010 together

## Proposal

### Core Principle: Type Constructor vs Function/Variable

**This is a key design choice that determines the syntax disambiguation rules:**

|写法                  | Meaning          | Rule                                   |
|---------------------|------------------|----------------------------------------|
| **`x: Type = ...`** | Type constructor | `: Type` explicitly declared → forced to be a type |
| **`f = ...`**       | Function or variable | No `: Type` → HM actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax itself has ambiguity:

- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executes statements, returns Void)

**Disambiguation rules**:

- **Has** `: Type` → Force parse as type constructor, `{ ... }` is a type literal
- **No** `: Type` → HM actively parses `{ ... }` as a code block, infers as function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type, HM infers as () -> Void
main = { println("Hello") }

# ❌ Error: no : Type, compiler cannot parse { ... } as type
Point = { x: Float, y: Float }  // HM infers as function, not type!
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
│       Point.draw = draw[0]  # Explicit binding required before dot call syntax
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Universe Levels (Compiler Internals)

**Internally**, the compiler maintains a universe level `level: selfpointnum` (stored as string, theoretically infinitely extensible).

| Level   | Description                                  |
| ------- | -------------------------------------------- |
| `Type0` | Everyday types (`Int`, `Float`, `Point`)     |
| `Type1` | Type constructors (`List`, `Maybe`)          |
| `Type2+`| Higher-order constructors                   |

**Users never see these numbers**, only `: Type`.

### Curry-Howard Isomorphism: Types as Propositions, Programs as Proofs

YaoXiang's unified syntax `name: type = value` is not a random choice—it is a direct mapping of the Curry-Howard
correspondence. This isomorphism reveals a profound truth: **the type system and the logical system are two sides of the same coin**.

| Logic (Proposition)     | Type System (YaoXiang)        | Example                                 |
|------------------------|-------------------------------|----------------------------------------|
| Proposition P          | Type T                        | `Int`, `Bool`                          |
| Proof that P is true   | A value of type T             | `42: Int`, `true: Bool`                |
| P → Q (implication)    | Function type `(P) -> Q`       | `(x: Int) -> Bool`                     |
| P ∧ Q (conjunction)    | Record type `{ p: P, q: Q }`  | `{ x: Int, y: Bool }`                  |
| ∀x.P(x) (universal)    | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...` |
| P ⊕ Q (disjunction)    | Enum / tagged union           | `Maybe: (T: Type) -> Type = { ... }`   |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "There exists a proof of type Int, named x, with value 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "There exists an implication proof: given proof a of Int and proof b of Int, construct a proof of Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires simultaneously providing proof x of Float and proof y of Float"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: If the type system allows constructing a value of type `T`
   without any legitimate runtime representation, it is like allowing proof of a false proposition in logic—the system breaks down. Curry-Howard tells us: **a type-safe language is naturally a consistent logical system**.

2. **Universe levels are necessary**: As detailed below, allowing
   `Type: Type` (i.e., "the type of types is also a type") leads to Russell's paradox (manifesting as Girard's paradox in type theory). YaoXiang's
   `Type₀ : Type₁ : Type₂ : ...`
   stratification ensures each type belongs to exactly one level, forming an ever-ascending chain that never closes, fundamentally avoiding paradoxes. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

3. **Theoretical foundation of unified syntax**: The reason `name: type = value`
   can cover variables, functions, types, interfaces, and generics with one syntax is precisely because they are all the same thing under Curry-Howard—**providing proofs for propositions**. Variables are evidence of propositions, functions are evidence of implications, records are evidence of conjunctions, generics are evidence of universal quantification. The unified syntax is not a coincidental artificial design but a natural consequence of the Curry-Howard isomorphism.

> **Further reading**: Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM, 58(12),
> 75–84. This article explains the history and significance of the Curry-Howard isomorphism in accessible language.

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

```yaoxiang
// Single expression form (directly returns value, no return needed)
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

// Void function (no return needed in code block)
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### Return Rules

The return value depends on the form on the right side of `=`:

|写法                     | Return value                           |
|------------------------|----------------------------------------|
| `= expr` (no braces)   | Directly return `expr`                 |
| `= { ... }` (with braces)| Must use `return`, otherwise returns `Void` |

```yaoxiang
# Single expression: directly returns value, no return needed
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

> **Design rationale**: `{ ... }`
> is a dependency-driven computation unit (see below), and its return semantics differ from single expressions. Braces introduce a multi-statement context, so explicit `return` is needed to disambiguate "whether the last expression is the return value".

#### `{}` Semantics: Dependency-Driven Computation Unit

`{ ... }` in YaoXiang is not just a code block—it is a **dependency-driven computation unit**. This semantics remains consistent across function bodies, variable initialization, and
`spawn`:

**Core rules**:

- Assignment statements inside `{}` are automatically sorted by dependency, not writing order
- Tasks with complete dependencies execute immediately; those missing dependencies block and wait
- Use `return` to explicitly return a value (see return rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler auto-sorts
result: Int = {
    b = a + 1      # depends on a → auto-sorted after a
    a = 10         # no dependencies → can execute first
    return b       # returns 11
}
```

> **Difference from single expressions**: `= expr` (no braces) is a simple binding that directly returns a value; `= { ... }` (with braces) introduces a dependency-driven computation context allowing multiple statements and explicit `return`.

#### `spawn` Block

`spawn { ... }` is YaoXiang's only parallelism primitive. It uses `{}`'s dependency-driven semantics for automatic parallelization:

- Direct child assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks with complete dependencies execute concurrently immediately
- Caller blocks waiting for all child tasks to complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, executes in parallel)
    c = process(a, b)         # depends on a, b → executes after both complete
    return c
}
// Caller blocks here until all tasks in spawn block complete
```

> **Full definition**: Complete semantics of `spawn`, task creation rules, and blocking model are detailed in `008-runtime-concurrency-model.md`.

#### `unsafe` Block

`unsafe { ... }` is used for defining opaque types and operating on raw pointers. It uses `{}`'s return semantics to bring type definitions back to the outer scope:

**Core rules**:

- Types and raw pointer operations can be defined in `unsafe {}`
- Use `return` to bring type definitions back to the outer scope
- Returned types are usable outside `unsafe {}`
- Field access on types requires unsafe permission

```yaoxiang
# Define opaque type inside unsafe block
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

# ✅ Through method call
db.close()
```

> **Full definition**: Complete semantics of `unsafe`, FFI type definitions, and method binding are detailed in `ffi.md`.

#### 3. Type Definition

Type definition is the core of YaoXiang's unified syntax, including fields, default values, bound methods, and interface implementations:

##### Basic Types

**Record type**: List of fields, field types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: Fields can have default values, optional during construction.

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

**Fields without default values**: Must be provided during construction.

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

YaoXiang's identifier system has three layers, recognized by different compiler stages in order:

1. **Keywords** (parser standalone token) — Control structure and declaration keywords like `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser standalone token) — `true`, `false`, `void`, `Type`, cannot be used as regular identifiers
3. **Built-in type names** (type
   checker pre-registered) — Parser treats them as regular identifiers, type checker handles resolution. **Not reserved words, can be shadowed (not recommended)**

The difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, built-in type name): `void`
is a value literal (equals the unique value of Unit), `Void` is a type name (equals the Unit type, logical ⊤). `let x: Void = void`
is valid.

Pre-configured built-in type names:

| Type    | Logical correspondence | Description                                                                                                                                                                                                               |
|---------|----------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Never` | ⊥ (false/empty type) | Zero constructors, no value can inhabit this type. Represents "impossible"—divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` means it never returns normally. **Not a keyword, is a built-in type name.** |
| `Void`  | ⊤ (true/Unit)        | Exactly one inhabitant (default void value). `x: Void = <default>` is valid. Corresponds to the identity of sum types—the identity of product types—`Void` is a zero-field product type (Unit), `Never` is a zero-variant sum type.        |
| `Int`   | —                    | Signed integer                                                                                                                                                                                                           |
| `Float` | —                    | Floating point number                                                                                                                                                                                                    |
| `Bool`  | —                    | Boolean value: `true` / `false`                                                                                                                                                                                          |
| `Char`  | —                    | Unicode character                                                                                                                                                                                                        |
| `String`| —                    | String                                                                                                                                                                                                                   |

##### Method Binding

**Method 1: Directly bind external function in type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // Bind to position 0, after currying method: (b: Point) -> Float
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

**Interface name written in type body, compiler automatically checks implementation**

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

**Interface = Record type with all function fields**

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

**`Type.name` prefix indicates namespace ownership**, nothing more. It does not trigger any implicit binding.

```yaoxiang
// Namespace function: regular function in Point namespace
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

// Call: just regular function call
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword, just a conventional parameter name. Writing `p`, `this`, `x`
> works exactly the same. The compiler looks at types, not parameter names.

##### Method Binding (The Only Way)

To make `.` method call syntax like `p.draw(screen)` work, **explicit binding is required**. The `[position]`
syntax is the only mechanism to bind a function as a "method" (full syntax in RFC-004).

```yaoxiang
// Define function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does p.draw(screen) syntax work
Point.draw = draw[0]   // Position 0 parameter (&Point) filled by caller

// Usage
p.draw(screen)          // Syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // Both call styles are equivalent

// Without [0] = no binding. Point.draw is just a regular function alias, no . syntax
Point.draw = draw       // No binding: can only call Point.draw(p, screen)
```

**Default behavior**: No `[n]` = no parameter bound. User must explicitly decide which parameters are filled by caller.

**Multi-position binding**:

```yaoxiang
// Bind multiple positions (auto-curried)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method to regular function):

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

// Generic methods (RFC-023 syntax: type parameters auto-inferred at call site)
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

Generic types and generic functions use unified `()` syntax for calls. `[]` is not used in any generic context.

**Core rules**:

1. **`()` does all applications**: Type application, function call, value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T flows from left side
empty: List(Int) = List()

# Generic function call — type flows automatically from parameters
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on left, value on right**: `name: type = value`—Type parameters declared on left, right side is always concrete value. For empty container
   `List()`, `T` must come from left-side type annotation.

3. **Type information written once**—at parameter declaration, compiler carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int written once on left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String auto from numbers and f types
```

4. **Value construction infers type from elements**:

```yaoxiang
x = List(1, 2, 3)       // Inferred as List(Int)
y = List("a", "b")      // Inferred as List(String)
z = List()              // ❌ Compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int from left-side annotation
```

5. **Type aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`, `List[Int](1,2,3)` →
> `List(1,2,3)`. Old `[]` generic syntax completely removed. `[]` only used for array/list literals and index access.

### Examples

#### Complete Example

```yaoxiang
// ======== 1. Interface Definition ========
// Interface = record type with all function fields
// Interface doesn't need self parameter — interface only defines "function signature after removing caller position"

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

// ======== 3. Method Implementation (regular function + explicit binding) ========

// Define function (self is just conventional name, not a keyword)
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

// Explicit binding — only after this does dot call syntax work
Point.draw = draw[0]
Point.bounding_box = bounding_box[0]
Point.serialize = serialize[0]
Point.translate = translate[0]
Point.scale = scale[0]
Point.distance = distance[0]

// Rect methods similar
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

// Chained calls
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
        // Check if type has method with same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check if method signature is compatible
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

### Interface Direct Assignment and Compile-Time Optimization

Interface types support direct assignment; the compiler automatically selects the optimal call strategy based on the right-hand value type:

```yaoxiang
// Direct assignment of concrete type → concrete type determinable at compile time, zero-cost call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), no vtable

// Function return value → concrete type not determinable at compile time, use vtable
d: Drawable = get_shape()
d.draw(screen)  // Method lookup through vtable

// Heterogeneous collection → use vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Method lookup through vtable
}
```

**Compile-time optimization strategy**:

| Scenario                            | Inference result      | Call method            |
|-------------------------------------|-----------------------|------------------------|
| `d: Drawable = Circle(1)`           | Concrete type Circle | Direct call (zero cost)|
| `d: Drawable = get_shape()`         | Unknown              | vtable                 |
| `shapes: List(Drawable) = [...]`   | Heterogeneous         | vtable                 |

**Rules**:

1. When right-hand side is a concrete type constructor and determinable at compile time, generate direct call IR
2. When right-hand side type cannot be determined at compile time, fall back to vtable mechanism
3. vtable fallback guarantees correctness of runtime polymorphism

### Duck Typing Support

```yaoxiang
// As long as it has the same methods, can be assigned to interface type
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

| Before                                  | After                                                                                        |
|----------------------------------------|---------------------------------------------------------------------------------------------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| Needed `impl` keyword                  | No keyword needed, interface name written after type body                                   |

### Deprecated: `|` Variant Syntax

> **Deprecation notice (2026-07-25, issue #203)**: The `|` variant syntax is officially deprecated and removed from implementation.

The following写法 **is no longer supported**:

```
type Color = red | green | blue                # ❌ Deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ Deprecated
type Option(T) = some(T) | none                # ❌ Deprecated
```

Unified use of record types to express sum types. When all fields of a record type are functions and all return the type itself, it is a sum type:

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

1. **Eliminate special cases**: `|` is the only non-`name: type = value` form in the BNF. After removal, the `type_expr`
   production is completely unified; the parser no longer needs to maintain separate paths and lookahead fallback for variant types.
2. **Mathematical equivalence**: Under Curry-Howard isomorphism, disjunction P ⊕
   Q's corresponding sum type is equivalent to a record type with all fields being functions that return the type itself. Both express the same semantics; no need for two syntaxes.
3. **Zero breaking**: Before removal, `|`
   syntax was half-supported in the parser (parameterless variants could parse but parameter types were lost during monomorphization), with no user code depending on it.
4. **AST simplification**: `Type::Variant(Vec<VariantDef>)` node deleted; all variant types unified to `Type::Struct`
   path; special branches in downstream typecheck/mono/formatter completely eliminated.

> **Note**: Semantic properties of sum types (such as match exhaustiveness checking, tagged union memory layout) are derived by typecheck layer from
> `Type::Struct` structure, not depending on a separate AST node.

## Syntax Design Note: Named Functions Are Syntactic Sugar for Lambdas

### Core Understanding

**Named functions and Lambda expressions are the same thing!** The only difference is: named functions give a Lambda a name.

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

**Key point**: When the signature fully declares parameter types, the parameter names in the Lambda head become redundant and can be omitted.

### Parameter Scope Rules

**Parameters override outer variables**: Parameters in the signature have scope that overrides the function body, inner scope has higher priority.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x overrides outer x, result is 20
```

### Annotation Position Flexibility

Type annotations can be in any of the following positions; **annotating at least one place is sufficient**:

| Annotation position | Form                                     | Note          |
|---------------------|------------------------------------------|---------------|
| Signature only      | `double: (x: Int) -> Int = x * 2`        | ✅ Recommended|
| Lambda head only    | `double = (x: Int) => x * 2`             | ✅ Valid      |
| Both sides          | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed|

### Complete Examples

```yaoxiang
// ✅ Recommended: full signature, Lambda head omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Valid: type annotation in Lambda head
double = (x: Int) => x * 2

// ✅ Valid: annotated on both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design Advantages

| Feature      | Advantage                                              |
|-------------|-------------------------------------------------------|
| **Concise** | No need to repeat parameter names when signature is complete |
| **Flexible**| Preserve Lambda form, use whichever you prefer        |
| **Consistent**| Maintains unified pattern with variable declaration `x: Int = 42` |
| **Intuitive**| `name: Type = body` directly corresponds to "named name, type Type, value body" |

## Tradeoffs

### Advantages

| Advantage       | Description                              |
|-----------------|------------------------------------------|
| Extreme unification | One syntax rule covers all cases     |
| Theoretically elegant | Perfectly symmetric `name: type = value` |
| No new keywords | Reuses existing syntactic elements       |
| Easy to implement | Compiler only needs to handle one declaration form |
| Easy to learn   | Remember one pattern to write all code   |
| Easy to extend  | New features can naturally integrate into this model |

### Disadvantages

| Disadvantage   | Description                                |
|---------------|--------------------------------------------|
| Naming convention | Methods need to follow `Type.method` naming |
| Verbosity     | Full syntax is longer than simplified syntax, but can be inferred |
| Learning curve | Need to understand the unified model       |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compile error example:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Can omit type, compiler infers
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE auto-suggests missing methods
```

### Risks

| Risk          | Impact                          | Mitigation            |
|--------------|--------------------------------|-----------------------|
| Parsing complexity | Unified syntax may increase parsing complexity | Use recursive descent parser |
| Performance overhead | vtable lookup may have extra cost | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: Origin of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Attempt to define the type of types...
Type: Type = Type
```

**Warning**: This is **the unnameable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二，二生三，三生万物。                                   ║
║   易有太极，是生两仪。                                         ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the origin of YaoXiang, the boundary of language. ║
║   The compiler falls silent here, philosophy dwells.         ║
║                                                              ║
║   Thank you for reaching the philosophical boundary          ║
║   of the language.                                           ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: The compiler cannot properly handle
> `Type: Type = Type` (would cause Type0/Type1 universe paradox), but we deliberately keep this "easter egg"—when you try to compile it, you receive a Zen message from the language's creator. This is not just a technical boundary, but YaoXiang's tribute to the philosophy of types.

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
       | '{' type_field* '}'                       # Record/Interface type
       | 'Type'                                    # Meta type

type_field ::= identifier ':' type_expr
             | identifier                           # Interface constraint

# Generic parameters: part of function type, e.g., (T: Type, R: Type) -> (...)
# No separate BNF rule needed — : Type parameters are just regular function parameters

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

| Term           | Definition                                                                              |
|----------------|-----------------------------------------------------------------------------------------|
| Declaration    | Assignment statement in the form `name: type = value`                                   |
| Record type    | `{ ... }` type containing named fields                                                   |
| Interface      | Record type with all function fields                                                     |
| Generic type   | Type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters          |
| Namespace function | Function in the form `Type.name`, belonging to Type namespace. Does not imply any binding |
| Method binding | `Type.name = func[n]`, binds position n of func to caller, enabling `obj.name(args)` syntax |
| Generic function| Function using `(T: Type)` syntax, type parameters as first parameter group              |
| Meta type      | `Type`, the only type-level marker in the language                                       |

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review│  ← Open for community discussion and feedback
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │   Rejected  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │     rfc/    │
│ (official)  │    │ (preserved) │
└─────────────┘    └─────────────┘
```
