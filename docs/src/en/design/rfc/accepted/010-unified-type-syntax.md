---
title: "RFC-010: Unified Type Syntax - name: type = value Model"
status: "Accepted"
author: "Chenxu"
created: "2025-01-20"
updated: "2026-06-05 (Updated return rules and {} semantics)"
issue: "#127"
---

# RFC-010: Unified Type Syntax - name: type = value Model

## Summary

This RFC proposes an extremely minimal, unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

where `type` can be any type expression, and `expression` can be any value expression.
**No `fn`, no `struct`, no `trait`, no `impl`, no lowercase `type` keyword (but there is `Type` as the metatype keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that accepts a type parameter T".

| Concept            | Code                                                                                |
|--------------------|-------------------------------------------------------------------------------------|
| Variable           | `x: Int = 42`                                                                       |
| Function           | `add: (a: Int, b: Int) -> Int = a + b`                                              |
| Record type        | `Point: Type = { x: Float, y: Float }`                                              |
| Interface          | `Drawable: Type = { draw: (Surface) -> Void }`                                      |
| Generic type       | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                         |
| Generic type       | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`            |
| Method             | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]`        |
| Generic function   | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`           |

**`Type` is the only metatype keyword in the language**.

> **Namespace vs. method binding**: The `Type.name` prefix denotes **namespace membership**, nothing more.
> It does not trigger any implicit binding. For the `.` call syntax like `p.draw(screen)` to take effect,
> you must explicitly bind: `Point.draw = draw[0]`.
> See the "Namespace and Method Binding" section below for details.

It is used to mark the type hierarchy, and the compiler automatically handles the distinction among Type0, Type1, Type2..., which is transparent to the user.

```yaoxiang
// Core syntax: unified + distinguished

// Variable
x: Int = 42

// Function (parameter names appear in the signature)
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
p.draw(screen)           // Syntactic sugar → Point.draw(p, screen)
s: Drawable = p           // Structural subtyping: Point implements Drawable
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

These concepts lack unity, leading to fragmented syntax and high learning cost.

### Design goals

1. **Extreme unification**: one syntax rule covers all cases
2. **Concise and elegant**: symmetric aesthetic of `name: type = value`
3. **No new keywords**: reuse existing syntax elements
4. **Theoretically elegant**: types themselves are values of the Type type
5. **Generics-friendly**: seamlessly integrates with the generics system (RFC-011)

### Integration with the generics system

The unified syntax model of RFC-010 fits **naturally** with the generics system design of RFC-011; generic parameters can be seamlessly incorporated into the unified model:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type positions in signature can be omitted, inferred at call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraints (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // T: Clone constraint carried by parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:
- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generic examples in RFC-010 cannot compile
- Recommendation: implement RFC-011 Phase 1 and RFC-010 in sync

## Proposal

### Core principle: type constructor vs. function/variable

**This is a key design choice that determines the disambiguation rules of the syntax:**

| Syntax                       | Meaning              | Rule                                                   |
|------------------------------|----------------------|--------------------------------------------------------|
| **`x: Type = ...`**          | Type constructor     | Explicit `: Type` declaration → forced to be a type    |
| **`f = ...`**                | Function or variable | No `: Type` → HM actively infers it as function/variable |

**Why is it designed this way?**

The `{ ... }` syntax is itself ambiguous:
- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executed statement, returns Void)

**Disambiguation rules**:
- **Has** `: Type` → forced to be parsed as a type constructor; `{ ... }` is a type literal
- **Has no** `: Type` → HM actively parses `{ ... }` as a code block and infers a function type

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
│       Point.draw = draw[0]  # Only after explicit binding does dot call syntax work
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type, HM infers as function
```

### Metatype hierarchy (compiler internals)

The **compiler internals** maintain a universe hierarchy `level: selfpointnum` (stored as a string, theoretically infinitely extendable).

| Level     | Description                                |
|-----------|--------------------------------------------|
| `Type0`   | Everyday types (`Int`, `Float`, `Point`)   |
| `Type1`   | Type constructors (`List`, `Maybe`)        |
| `Type2+`  | Higher-order constructors                  |

**Users never see these numbers**, only `: Type`.

### Curry-Howard correspondence: types as propositions, programs as proofs

YaoXiang's unified syntax `name: type = value` is not chosen arbitrarily—it is a direct mapping of the Curry-Howard correspondence. This correspondence reveals a profound fact: **the type system and the logic system are two sides of the same coin**.

| Logic (proposition)        | Type system (YaoXiang)                | Example                              |
|----------------------------|---------------------------------------|--------------------------------------|
| Proposition P              | Type T                                | `Int`, `Bool`                        |
| Proof that P holds         | A value of type T                     | `42: Int`, `true: Bool`              |
| P → Q (implication)        | Function type `(P) -> Q`              | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)        | Record type `{ p: P, q: Q }`          | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal quant.) | Generic function `(T: Type) -> ...`   | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)        | Enum / tagged union                   | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads: "there exists a proof of Int, named x, whose value is 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads:
// "there exists an implication proof: given proofs a and b of Int, one can construct a proof of Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads:
// "Point is a proposition whose proof requires both a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: if the type system allows constructing a value of type `T` with no valid runtime representation, that is like allowing a proof of a false proposition in logic—the system breaks down. Curry-Howard tells us: **a type-safe language is inherently a logically consistent system**.

2. **Universe hierarchy is a necessary condition**: as detailed below, allowing `Type: Type` (i.e., "the type of types is also a type") would produce Russell's paradox (manifested as Girard's paradox in type theory). YaoXiang's stratified `Type₀ : Type₁ : Type₂ : ...` ensures each type belongs to exactly one level, forming an ever-rising chain that never closes, fundamentally avoiding paradox. This means YaoXiang's type system is **logically consistent** in the Curry-Howard sense.

3. **Theoretical foundation of the unified syntax**: the reason `name: type = value` can cover variables, functions, types, interfaces, and generics with a single syntax is precisely because, under Curry-Howard, they are all the same thing—**providing a proof for a proposition**. A variable is evidence of a proposition; a function is evidence of an implication; a record is evidence of a conjunction; generics are evidence of a universal quantification. The unified syntax is not an arbitrary design coincidence, but a natural consequence of the Curry-Howard correspondence.

> **Further reading**: Wadler, P. (2015). *"Propositions as Types."* Communications of the ACM, 58(12), 75–84. This article explains the history and significance of the Curry-Howard correspondence in accessible language.

### Syntax definition

#### 1. Variable declaration

```yaoxiang
// Basic syntax
x: Int = 42
name: String = "Alice"
flag: Bool = true

// Type inference (can be omitted)
y = 100  // Inferred as Int
```

#### 2. Function definition

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

// Void function (no return needed inside the block)
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### Return rules

The return value depends on the form on the right-hand side of `=`:

| Syntax                    | Return value                            |
|---------------------------|-----------------------------------------|
| `= expr` (no braces)      | Returns `expr` directly                 |
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

> **Design rationale**: `{ ... }` is a dependency-driven computation unit (see below); its return semantics differ from a single expression. Braces introduce a multi-statement context, so an explicit `return` is required to eliminate the ambiguity of "whether the last expression is the return value".

#### `{}` semantics: dependency-driven computation unit

In YaoXiang, `{ ... }` is not just a code block—it is a **dependency-driven computation unit**. This semantics is consistent across function bodies, variable initialization, and `spawn`:

**Core rules**:
- Assignment statements inside `{}` are automatically ordered by dependency, not by written order
- A statement executes immediately when its dependencies are ready; otherwise it blocks and waits
- Use `return` to explicitly return a value (see return rules)

```yaoxiang
# Dependency-driven: b depends on a, compiler reorders automatically
result: Int = {
    b = a + 1      # depends on a → automatically placed after a
    a = 10         # no dependency → can execute first
    return b       # returns 11
}
```

> **Difference from a single expression**: `= expr` (no braces) is a simple binding that directly returns the value; `= { ... }` (with braces) introduces a dependency-driven computation context, allowing multiple statements and explicit `return`.

#### `spawn` block

`spawn { ... }` is the only parallel primitive in YaoXiang. It leverages the dependency-driven semantics of `{}` to enable automatic parallelization:

- Direct child assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks execute concurrently as soon as their dependencies are ready
- The caller blocks until all child tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (no dependency on a, runs in parallel)
    c = process(a, b)         # depends on a, b → waits for both to complete
    return c
}
// The caller blocks here until all tasks inside the spawn block complete
```

> **Detailed definition**: the full semantics of `spawn`, task creation rules, and blocking model are detailed in `008-runtime-concurrency-model.md`.

#### `unsafe` block

`unsafe { ... }` is used to define opaque types and to operate on raw pointers. It leverages the return semantics of `{}` to surface type definitions to the enclosing scope:

**Core rules**:
- Types can be defined and raw pointers operated on inside `unsafe {}`
- Use `return` to surface type definitions to the enclosing scope
- Returned types are usable outside of `unsafe {}`
- Field access on such types requires `unsafe` permission

```yaoxiang
# Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw pointer
    }
    return SqliteDb
}

# SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: the handle field requires unsafe permission
handle = db.handle

# ✅ Through a method call
db.close()
```

> **Detailed definition**: the full semantics of `unsafe`, FFI type definition, and method binding are detailed in `ffi.md`.

#### 3. Type definition

Type definition is the core of YaoXiang's unified syntax; it includes fields, default values, bound methods, and interface implementation:

##### Basic types

**Record type**: a list of fields, whose types can be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: fields may have default values and become optional at construction.

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

1. **Keywords** (parser-specific tokens) — control structures and declaration keywords, such as `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser-specific tokens) — `true`, `false`, `void`, `Type`; cannot be used as ordinary identifiers
3. **Built-in type names** (pre-registered in the type checker) — the parser treats them as ordinary identifiers, and the type checker resolves them. **Not reserved words, can be shadowed (not recommended)**

The difference between `void` (lowercase, literal reserved word) and `Void` (uppercase, built-in type name): `void` is a value literal (equal to the single inhabitant of Unit), `Void` is a type name (equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Predefined built-in type names:

| Type     | Logical counterpart | Description                                                                                                                                                                                                                                                                                                       |
|----------|---------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Never`  | ⊥ (false/empty)     | Zero constructors; no value can inhabit this type. Represents "impossible" — divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` indicates it never returns normally. **Not a keyword, but a built-in type name.**                                     |
| `Void`   | ⊤ (true/Unit)       | Exactly one inhabitant (the default `void` value). `x: Void = <default>` is legal. The unit of sum types corresponds to the unit of product types — `Void` is the zero-field product type (Unit), and `Never` is the zero-variant sum type.                                                                      |
| `Int`    | —                   | Signed integer                                                                                                                                                                                                                                                                                                    |
| `Float`  | —                   | Floating-point number                                                                                                                                                                                                                                                                                            |
| `Bool`   | —                   | Boolean: `true` / `false`                                                                                                                                                                                                                                                                                         |
| `Char`   | —                   | Unicode character                                                                                                                                                                                                                                                                                                 |
| `String` | —                   | String                                                                                                                                                                                                                                                                                                            |

##### Bound methods

**Way 1: Bind an external function directly inside the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // Bound to position 0; after currying the method is: (b: Point) -> Float
}
// Call: p1.distance(p2) → distance(p1, p2)
```

**Way 2: Anonymous function + positional binding**

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

##### Interface implementation

**The interface name is written inside the type body, and the compiler automatically checks its implementation**

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

**The `Type.name` prefix denotes namespace membership**, nothing more. It does not trigger any implicit binding.

```yaoxiang
// Namespace function: an ordinary function under the Point namespace
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

> **Note**: `self` is not a keyword, only a conventional parameter name. Naming it `p`, `this`, or `x` works exactly the same.
> The compiler does not look at the parameter name, only the type.

##### Method binding (the only way)

For the `.` method-call syntax like `p.draw(screen)` to take effect, **you must explicitly bind**.
The `[position]` syntax is the only mechanism to bind a function as a "method" (see RFC-004 for detailed syntax).

```yaoxiang
// Define the function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — only after this does the p.draw(screen) syntax become available
Point.draw = draw[0]   // The parameter at position 0 (&Point) is filled by the caller

// Usage
p.draw(screen)          // Syntactic sugar → draw(&p, screen)
Point.draw(p, screen)   // Both call forms are equivalent

// Without [0] = no binding. Point.draw is just an ordinary function alias with no . syntax
Point.draw = draw       // No binding: only Point.draw(p, screen) works
```

**Default behavior**: omitting `[n]` = no parameter is bound. The user must explicitly decide which parameters are filled by the caller.

**Multi-position binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Call: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method back to ordinary function):

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
    return item.serialize()
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
    return (self: List(type), item: type) -> {
        self.data.append(item)
        self.length = self.length + 1
    }
}

IntList.push(Int)(self, item)  // Call example

// Generic methods (RFC-023 syntax: type parameters inferred automatically at the call site)
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

#### 6. Generic call syntax

Both generic types and generic functions uniformly use `()` syntax. `[]` is never used in any generic context.

**Core rules**:

1. **`()` does it all**: type application, function call, and value construction all use `()`

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

2. **Type on the left, value on the right**: `name: type = value` — Type parameters are declared on the left; the right side is always a concrete value. The `T` of an empty container `List()` must come from the type annotation on the left.

3. **Type information only needs to be written once** — when declared in parameters, the compiler carries it along:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String flow automatically from the types of numbers and f
```

4. **Value construction infers the type from the elements**:

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

> **Comparison with old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`, `List[Int](1,2,3)` → `List(1,2,3)`.
> The old `[]` generic syntax has been completely removed. `[]` is used only for array/list literals and index access.

### Examples

#### Complete example

```yaoxiang
// ======== 1. Interface definition ========
// Interface = a record type whose fields are all function types
// Interfaces do not need a self parameter — an interface only defines "the function signature with the caller position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // Returns the interface type; concrete implementations return their own type
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

// ======== 3. Method implementation (ordinary function + explicit binding) ========

// Define functions (self is just a conventional name, not a keyword)
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

// Explicit binding — only after binding does the dot-call syntax work
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

// Generic function (RFC-023 syntax: omit type parameters at call site, inferred automatically)
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

Interface types support direct assignment, and the compiler automatically selects the optimal call strategy based on the right-hand side type of the assignment:

```yaoxiang
// Direct assignment of a concrete type → type is known at compile time, zero-overhead call
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: directly calls circle_draw(screen), no vtable

// Function return value → type cannot be determined at compile time, uses vtable
d: Drawable = get_shape()
d.draw(screen)  // Method looked up through vtable

// Heterogeneous collection → uses vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Method looked up through vtable
}
```

**Compile-time optimization strategy**:

| Scenario                                      | Inferred result     | Call method     |
|-----------------------------------------------|---------------------|-----------------|
| `d: Drawable = Circle(1)`                     | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()`                   | Unknown              | vtable          |
| `shapes: List(Drawable) = [...]`              | Heterogeneous        | vtable          |

**Rules**:
1. When the right-hand side is a concrete type constructor and can be determined at compile time, generate direct-call IR
2. When the right-hand side type cannot be determined at compile time, fall back to the vtable mechanism
3. The vtable fallback guarantees correctness of runtime polymorphism

### Duck typing support

```yaoxiang
// As long as the same methods exist, the value can be assigned to the interface type
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

| Before                                                                  | After                                                                                                                                  |
|-------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------|
| `type Point = Point(x: Float, y: Float)`                               | `Point: Type = { x: Float, y: Float }`                                                                                                |
| `type Result(T, E) = ok(T) \| err(E)`                                  | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }`                                        |
| Requires the `impl` keyword                                             | No keyword required; interface name is written inside the type body                                                                  |

## Syntax design note: named functions are essentially syntactic sugar over lambdas

### Core understanding

**Named functions and lambda expressions are the same thing!** The only difference is: a named function gives a lambda a name.

```yaoxiang
// These two are essentially identical
add: (a: Int, b: Int) -> Int = a + b           // Named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda form (fully equivalent)
```

### Syntactic sugar model

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially
name: (Params) -> ReturnType = (params) => body
```

**Key point**: when the signature fully declares the parameter types, the parameter names in the lambda head become redundant and can be omitted.

### Parameter scoping rules

**Parameters shadow outer variables**: parameters in the signature shadow outer variables, with the inner scope taking precedence.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x shadows outer x, result is 20
```

### Flexible annotation position

Type annotations can be at any of the following positions; **at least one annotation is required**:

| Annotation position    | Form                                    | Description                                |
|------------------------|-----------------------------------------|--------------------------------------------|
| Signature only         | `double: (x: Int) -> Int = x * 2`       | ✅ Recommended                             |
| Lambda head only       | `double = (x: Int) => x * 2`            | ✅ Legal                                   |
| Both sides             | `double: (x: Int) -> Int = (x) => x * 2`| ✅ Redundant but allowed                   |

### Complete examples

```yaoxiang
// ✅ Recommended: signature is complete, lambda head is omitted
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Legal: type annotation in the lambda head
double = (x: Int) => x * 2

// ✅ Legal: both sides annotated
double: (x: Int) -> Int = (x) => x * 2
```

### Design advantages

| Feature        | Advantage                                                                                                |
|----------------|----------------------------------------------------------------------------------------------------------|
| **Concise**    | No need to repeat parameter names when the signature is complete                                        |
| **Flexible**   | Keep the lambda form; use whichever you prefer                                                           |
| **Consistent** | Matches the unified pattern of variable declaration `x: Int = 42`                                        |
| **Intuitive**  | `name: Type = body` directly corresponds to "named name, of type Type, with value body"                 |

## Trade-offs

### Advantages

| Advantage          | Description                                                                |
|--------------------|----------------------------------------------------------------------------|
| Extreme unity      | One syntax rule covers all cases                                           |
| Theoretically elegant | Perfectly symmetric `name: type = value`                                |
| No new keywords    | Reuses existing syntax elements                                             |
| Easy to implement  | The compiler only needs to handle one declaration form                     |
| Easy to learn      | Remember one pattern and you can write all code                            |
| Easy to extend     | New features can be naturally incorporated into this model                 |

### Disadvantages

| Disadvantage       | Description                                                                |
|--------------------|----------------------------------------------------------------------------|
| Naming convention  | Methods must follow the `Type.method` naming                               |
| Verbosity          | The full syntax is longer than simplified syntax, but inference is possible |
| Learning curve     | Requires understanding the unified model                                   |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Compile error example:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// The type can be omitted and inferred by the compiler
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// IDE automatically suggests missing methods
```

### Risks

| Risk                | Impact                                          | Mitigation                                  |
|---------------------|-------------------------------------------------|---------------------------------------------|
| Parsing complexity  | The unified syntax may increase parsing complexity | Use a recursive descent parser            |
| Performance overhead | vtable lookup may incur extra overhead         | Compile-time monomorphization optimization   |

---

## Easter egg 🎮: The source of the language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: this is **unnameable**!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   One begets two, two begets three, three begets all things.   ║
║   Change has the supreme ultimate; it begot the two forms.    ║
║                                                              ║
║   Type: Type = Type                                          ║
║   This is the source of YaoXiang, the boundary of the language. ║
║   The compiler falls silent here; philosophy pauses here.     ║
║                                                              ║
║   Thank you for touching the philosophical frontier of the language. ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: the compiler cannot correctly process `Type: Type = Type` (it would lead to a Type0/Type1 universe paradox), but we deliberately keep this "easter egg"—when you try to compile it, you will receive a Zen message from the language's founder. This is not only a technical boundary, but also YaoXiang's tribute to the philosophy of types.

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
       | 'Type'                                    # Metatype

type_field ::= identifier ':' type_expr
             | identifier                           # Interface constraint

# Generic parameters: as part of a function type, e.g. (T: Type, R: Type) -> (...)
# No dedicated BNF rule needed — : Type parameters are ordinary function parameters

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

| Term                 | Definition                                                                                                    |
|----------------------|---------------------------------------------------------------------------------------------------------------|
| Declaration          | An assignment statement of the form `name: type = value`                                                     |
| Record type          | A `{ ... }` type containing named fields                                                                      |
| Interface            | A record type whose fields are all function types                                                            |
| Generic type         | A type defined as `Name: (T: Type) -> Type = { ... }` that accepts type parameters                          |
| Namespace function   | A function of the form `Type.name` that belongs to the Type namespace. Implies no binding of any kind         |
| Method binding       | `Type.name = func[n]`, binds position n of `func` to be filled by the caller, enabling the `obj.name(args)` syntax |
| Generic function     | A function using the `(T: Type)` syntax; type parameters form the first parameter group                       |
| Metatype             | `Type`, the only type-level marker in the language                                                            |

---

## Lifecycle and fate

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
│  (Official design)│ (Original location retained) │
└─────────────┘    └─────────────┘
```