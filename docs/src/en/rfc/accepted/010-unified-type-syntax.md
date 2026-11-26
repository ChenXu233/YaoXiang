---
title: 'RFC-010: Unified Type Syntax - `name: type = value` Model'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-14'
updated: '2026-09-25'
issue: '#127'
---

# RFC-010: Unified Type Syntax - `name: type = value` Model

## Abstract

This RFC proposes a minimalist, unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Here, `type` may be any type expression, and `expression` may be any value expression. **There is no
`fn`, no `struct`, no `trait`, no `impl`, and no lowercase `type` keyword (although `Type` exists as
the metatype keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that accepts
> the type parameter T."

| Concept          | Syntax                                                                       |
| ---------------- | ---------------------------------------------------------------------------- |
| Variable         | `x: Int = 42`                                                                |
| Function         | `add: (a: Int, b: Int) -> Int = a + b`                                       |
| Record type      | `Point: Type = { x: Float, y: Float }`                                       |
| Interface        | `Drawable: Type = { draw: (Surface) -> Void }`                               |
| Generic type     | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                  |
| Generic type     | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`     |
| Method           | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`    |

**`Type` is the only metatype keyword in the language.**

> **Namespace vs. method binding**: The `Type.name` prefix indicates **namespace membership**, and
> nothing more. It does not trigger any implicit binding. For `.` call syntax such as
> `p.draw(screen)` to work, an explicit binding is required: `Point.draw = draw[0]`. See the
> "Namespaces and Method Binding" section below. It is used to annotate the type hierarchy; the
> compiler automatically handles distinctions such as Type0, Type1, Type2, and so on, transparently
> to users.

```yaoxiang
// Core syntax: unified + distinction

// Variable
x: Int = 42

// Function (parameter names are in the signature)
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
p.draw(screen)           // Syntax sugar → Point.draw(p, screen)
s: Drawable = p           // Structural subtyping: Point implements Drawable
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## Motivation

### Why is this feature needed?

The current type system has several separate concepts:

- Syntax for variable declarations
- Syntax for function definitions
- Syntax for type definitions (different syntaxes)
- Syntax for interface definitions
- Syntax for method bindings

These concepts lack unification, resulting in fragmented syntax and a high learning cost.

### Design goals

1. **Maximum unification**: one syntax rule covers every case
2. **Concise elegance**: the symmetric aesthetics of `name: type = value`
3. **No new keywords**: reuse existing syntactic elements
4. **Theoretical elegance**: types themselves are values of type Type
5. **Generics-friendly**: integrates seamlessly with the generics system (RFC-011)

### Integration with the generics system

RFC-010's unified syntax model naturally fits RFC-011's generics system design. Generic parameters
can be incorporated into the unified model seamlessly:

```yaoxiang
// Basic generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// Generic function (RFC-023 syntax: Type positions in the signature may be omitted and are inferred automatically at the call site)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Type constraint (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // The T: Clone constraint is carried by the parameter type

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Dependencies**:

- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, RFC-010's generics examples cannot compile
- Recommendation: implement RFC-011 Phase 1 and RFC-010 in parallel

## Proposal

### Core principle: type constructors vs. functions/variables

**This is a critical design choice that determines the syntax's disambiguation rules:**

| Syntax              | Meaning              | Rule                                                    |
| ------------------- | -------------------- | ------------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | Explicit `: Type` declaration → forced to be a type     |
| **`f = ...`**       | Function or variable | No `: Type` → HM proactively infers a function/variable |

**Why is it designed this way?**

The `{ ... }` syntax is inherently ambiguous:

- `{ x: Float, y: Float }` may be a **type literal** (a record type)
- `{ a = 1 + 1 }` may be a **code block** (executing statements and returning Void)

**Rules for resolving the ambiguity**:

- **With** `: Type` → forced to parse as a type constructor; `{ ... }` is a type literal
- **Without** `: Type` → HM proactively parses `{ ... }` as a code block and infers a function type

```yaoxiang
# ✅ Type constructor: has : Type
Point: Type = { x: Float, y: Float }

# ✅ Function: no : Type; HM infers () -> Void
main: () -> Void = { println("Hello") }

# ❌ Error: without : Type, the compiler cannot parse { ... } as a type
Point = { x: Float, y: Float }  // HM infers a function, not a type!
```

---

**Unified model: identifier : type = expression**

```
├── 变量
│   └── x: Int = 42
│
├── 函数
│   └── add: (a: Int, b: Int) -> Int = a + b  # No : Type; HM infers a function
│
├── 记录类型
│   └── Point: Type = { x: Float, y: Float }  # Must return: Type
│
├── 接口
│   └── Drawable: Type = { draw: (Surface) -> Void }  # Must return: Type
│
├── 泛型类型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # Must return: Type
│
├── 泛型类型（多参数）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # Must return: Type
│
├── 命名空间函数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # Dot-call syntax is available only after an explicit binding
│
└── 泛型函数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Does not return Type; HM infers a function
```

### Metatype hierarchy (compiler internals)

Internally, the **compiler** maintains a universe hierarchy `level: selfpointnum` (stored as a
string and theoretically extendable without limit).

| Level    | Description                              |
| -------- | ---------------------------------------- |
| `Type0`  | Everyday types (`Int`, `Float`, `Point`) |
| `Type1`  | Type constructors (`List`, `Maybe`)      |
| `Type2+` | Higher-order constructors                |

**Users never see these numbers**; they see only `: Type`.

### Curry-Howard correspondence: types are propositions, programs are proofs

YaoXiang's unified syntax `name: type = value` was not chosen arbitrarily—it is a direct mapping of
the Curry-Howard correspondence. This correspondence reveals a profound fact: **the type system and
the logical system are two sides of the same thing**.

| Logic (proposition)                | Type system (YaoXiang)              | Example                              |
| ---------------------------------- | ----------------------------------- | ------------------------------------ |
| Proposition P                      | Type T                              | `Int`, `Bool`                        |
| Proof that P is true               | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication)                | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)                | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal quantification) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)                | Enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**The meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads as: "there exists a proof of type Int named x whose value is 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads as:
// "there exists an implication proof: given proofs a and b of type Int, an Int proof can be constructed"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads as:
// "Point is a proposition whose proof requires simultaneously providing a Float proof x and a Float proof y"
Point: Type = { x: Float, y: Float }
```

**Why is this important?**

1. **Logical consistency = type safety**: if the type system permits constructing a value of type
   `T` with no valid runtime representation, that is like allowing logic to prove a false
   proposition—the system collapses. Curry-Howard tells us: **a type-safe language is naturally a
   logically consistent system**.

2. **A universe hierarchy is necessary**: as detailed below, allowing `Type: Type` (that is, "the
   type of types is itself a type") would produce Russell's paradox (manifested as Girard's paradox
   in type theory). YaoXiang's `Type₀ : Type₁ : Type₂ : ...` layering ensures that every type
   belongs to exactly one level, forming an upward chain that never closes and thereby fundamentally
   avoiding paradoxes. This means that YaoXiang's type system is **logically consistent** in the
   Curry-Howard sense.

3. **The theoretical foundation of unified syntax**: the fact that `name: type = value` can use one
   syntax to cover variables, functions, types, interfaces, and generics stems from their being the
   same thing under Curry-Howard—**the provision of proofs for propositions**. Variables are
   evidence of propositions, functions are evidence of implications, records are evidence of
   conjunctions, and generics are evidence of universal quantification. Unified syntax is not an
   arbitrary coincidence, but a natural consequence of the Curry-Howard correspondence.

> **Further reading**: Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM,
> 58(12), 75–84. This article explains the history and significance of the Curry-Howard
> correspondence in accessible terms.

### Syntax definition

#### 1. Variable declarations

```yaoxiang
// Basic syntax
x: Int = 42
name: String = "Alice"
flag: Bool = true

// Type inference (can be omitted)
y = 100  // Inferred as Int
```

#### 2. Function definitions

**The value of a block is its trailing expression; `return` exits the function (type `Never`)**—see
[RFC-010a](010a-tail-expression-and-return.md) for details.

```yaoxiang
// Single-expression form
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// Code block form: the value is the trailing expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b                    // Trailing expression → value of the block
}

// Multi-line code block
calc: (x: Float, y: Float, op: String) -> Float = {
    match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }                    // match is the trailing expression
}

// Void function: the trailing expression is Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### Return rules

**The value of a block is its trailing expression (the sole exit)**:

| Form                                                  | Value                                         |
| ----------------------------------------------------- | --------------------------------------------- |
| `= expr` (without braces)                             | `expr`                                        |
| `= { ...; e }` (with braces)                          | Trailing expression `e`                       |
| `= { ...; s }` (final item is a statement/assignment) | `Void` (the value of an assignment is `Void`) |
| `= {}` (empty block)                                  | `Void`                                        |

**Semantics of `return`**: non-local exit that **exits the nearest function boundary** (rather than
"returning to the block"), with type `Never`. `Never <: T` holds for every type (the principle of
explosion), so `return` may appear anywhere a return type is expected.

```yaoxiang
# Single expression: directly returns the value
add: (a: Int, b: Int) -> Int = a + b

# Code block: the value is the trailing expression
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# Early return: return penetrates the block and exits the function (type Never)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # Trailing expression
}
```

**Design rationale**: `{ ... }` is a dependency-driven computation unit (see below), and its
evaluation semantics differ from those of a single expression—braces introduce a multi-statement
context, and **its value is given by the trailing expression**, eliminating the ambiguity of whether
"the last expression is the return value."

#### `{}` semantics: dependency-driven computation units

In YaoXiang, `{ ... }` is not merely a code block—it is a **dependency-driven computation unit**.
This semantics remains consistent in function bodies, variable initializers, and `spawn`:

**Core rules**:

- Assignment statements inside `{}` are ordered automatically according to dependencies, rather than
  source order
- A statement executes immediately once its dependencies are satisfied; otherwise, it blocks and
  waits
- **The value of a block is its trailing expression** (see the return rules); `return` is a
  non-local exit of type `Never` that exits the function

```yaoxiang
# Dependency-driven execution: b depends on a, so the compiler orders it automatically
result: Int = {
    b = a + 1      # Depends on a → automatically ordered after a
    a = 10         # No dependencies → may execute first
    b              # Trailing expression → value of the block is 11
}
```

> **Difference from a single expression**: `= expr` (without braces) is a simple binding that
> directly returns a value; `= { ... }` (with braces) introduces a dependency-driven computation
> context, permits multiple statements, and has a value given by its trailing expression.

#### `spawn` blocks

`spawn { ... }` is YaoXiang's only parallel primitive. It uses the dependency-driven semantics of
`{}` to implement automatic parallelization:

- Direct child assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks execute concurrently as soon as their dependencies are satisfied
- The caller blocks until all child tasks complete

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # Task 1
    b = fetch_data("url2")    # Task 2 (independent of a, so it runs in parallel)
    c = process(a, b)         # Depends on a and b → waits for both to complete
    c                         # Trailing expression → value of spawn
}
// The caller blocks here until all tasks in the spawn block have completed
```

> **Detailed definition**: the complete semantics of `spawn`, task creation rules, and the blocking
> model are described in `008-runtime-concurrency-model.md`.

#### `unsafe` blocks

`unsafe { ... }` is used to define opaque types and manipulate raw pointers. It uses the evaluation
semantics of `{}` to delegate type definitions to the enclosing scope (the value exits through the
trailing expression):

**Core rules**:

- Types may be defined and raw pointers manipulated inside `unsafe {}`
- The **trailing expression** gives the value of `unsafe {}` (the type definition is delegated to
  the enclosing scope)
- Returned types are available outside `unsafe {}`
- Accessing a type's fields requires `unsafe` permission

```yaoxiang
# Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # Raw pointer
    }
    SqliteDb           # Trailing expression → value of the unsafe block
}

# SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

# ❌ Compile error: accessing the handle field requires unsafe permission
handle = db.handle

# ✅ Use a method call instead
db.close()
```

> **Detailed definition**: the complete semantics of `unsafe`, FFI type definitions, and method
> bindings are described in `ffi.md`.

#### 3. Type definitions

Type definitions are the core of YaoXiang's unified syntax. They include fields, default values,
method bindings, and interface implementations:

##### Basic types

**Record type**: a list of fields, whose field types may be arbitrary type expressions.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: fields may have default values and are optional during construction.

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

**Fields without default values**: these must be supplied during construction.

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

YaoXiang's identifier system has three layers, recognized successively by different compiler stages:

1. **Keywords** (independent parser tokens)—control-flow and declaration keywords such as `if`,
   `match`, `pub`, and `return`
2. **Literal reserved words** (independent parser tokens)—`true`, `false`, `void`, and `Type`; these
   cannot be used as ordinary identifiers
3. **Built-in type names** (preregistered by the type checker)—the parser treats them as ordinary
   identifiers, while the type checker resolves them. **They are not reserved words and may be
   shadowed (though this is not recommended)**

The distinction between `void` (lowercase, a literal reserved word) and `Void` (uppercase, a
built-in type name): `void` is a value literal (the sole value of Unit), whereas `Void` is a type
name (the Unit type, logically ⊤). `let x: Void = void` is valid.

Predefined built-in type names:

| Type     | Logical correspondence | Description                                                                                                                                                                                                                                                                       |
| -------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥ (false/empty type)   | Zero constructor; no value can inhabit this type. It represents "impossible"—divergence, panic, and dead code. `Never <: T` holds for every `T` (principle of explosion). A function returning `Never` never returns normally. **It is not a keyword, but a built-in type name.** |
| `Void`   | ⊤ (true/Unit)          | Exactly one inhabitant (the default `void` value). `x: Void = <default>` is valid. The identity of a sum type corresponds to the identity of a product type—`Void` is the zero-field product type (Unit), while `Never` is the zero-variant sum type.                             |
| `Int`    | —                      | Signed integer                                                                                                                                                                                                                                                                    |
| `Float`  | —                      | Floating-point number                                                                                                                                                                                                                                                             |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                                                                                                                                                                                                                   |
| `Char`   | —                      | Unicode character                                                                                                                                                                                                                                                                 |
| `String` | —                      | String                                                                                                                                                                                                                                                                            |

##### Method bindings

**Method 1: bind an external function directly within the type definition**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // Bound to position 0; after currying, method: (b: Point) -> Float
}
// Invocation: p1.distance(p2) → distance(p1, p2)
```

**Method 2: anonymous function + positional binding**

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        (dx * dx + dy * dy).sqrt()      # Trailing expression
    })
}
// Syntax: ((params) => body)[position]
// Invocation: p1.distance(p2) → distance(p1, p2)
```

##### Interface implementations

**The interface name is written in the type body, and the compiler automatically checks the
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

##### Interface definitions

**An interface is a record type whose fields are all functions**

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

##### Namespace function definitions

**The `Type.name` prefix indicates namespace membership**, and nothing more. It does not trigger any
implicit binding.

```yaoxiang
// Namespace function: an ordinary function in the Point namespace
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

// Invocation: an ordinary function call
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword; it is merely a conventional parameter name. Writing `p`,
> `this`, or `x` has exactly the same effect. The compiler examines types, not parameter names.

##### Method binding (the only mechanism)

For `.` method-call syntax such as `p.draw(screen)` to work, an **explicit binding is mandatory**.
The `[position]` syntax is the only mechanism for binding a function as a "method" (see RFC-004 for
the detailed syntax).

```yaoxiang
// Define a function
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// Explicit binding — p.draw(screen) syntax is available only after this
Point.draw = draw[0]   // The parameter at position 0 (&Point) is supplied by the caller

// Usage
p.draw(screen)          // Syntax sugar → draw(&p, screen)
Point.draw(p, screen)   // The two invocation forms are equivalent

// Without [0] = no binding. Point.draw is merely an alias for an ordinary function and has no . syntax
Point.draw = draw       // No binding: only Point.draw(p, screen) is possible
```

**Default behavior**: omitting `[n]` binds no arguments. The user must explicitly determine which
arguments are supplied by the caller.

**Multi-positional binding**:

```yaoxiang
// Bind multiple positions (automatic currying)
Point.transform = transform_points[0, 1]
// Invocation: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (converting a method into an ordinary function):

```yaoxiang
// Extract the function from a binding
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. Interface composition

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Use an intersection type
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

IntList.push(Int)(self, item)  // Example invocation

// Generic method (RFC-023 syntax: type parameters are inferred automatically at the call site)
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

#### 6. Generic invocation syntax

Generic types and generic functions uniformly use `()` for invocation. `[]` is not used in any
generic context.

**Core rules**:

1. **`()` does everything**: type application, function calls, and value construction all use `()`

```yaoxiang
# Type annotation
numbers: List(Int) = List(1, 2, 3)

# Empty container: T comes from the left
empty: List(Int) = List()

# Generic function invocation—types flow automatically from the arguments
strings = map(numbers, f)
// T=Int comes from numbers: List(Int)
// R=String comes from f: (Int) -> String
```

2. **Type on the left, value on the right**: `name: type = value`—Type parameters are declared on
   the left, and the right-hand side is always a concrete value. The `T` of an empty container
   `List()` must be obtained from the type annotation on the left.

3. **Type information need only be written once**—the compiler carries it along when processing
   parameter declarations:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int is written once on the left
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int and R=String are inferred automatically from numbers and f
```

4. **Value construction infers its type from its elements**:

```yaoxiang
x = List(1, 2, 3)       // Inferred as List(Int)
y = List("a", "b")      // Inferred as List(String)
z = List()              // ❌ Compile error: cannot infer T
z: List(Int) = List()   // ✅ T=Int comes from the annotation on the left
```

5. **Type aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with the old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`,
> `List[Int](1,2,3)` → `List(1,2,3)`. The old `[]` generics syntax has been completely removed. `[]`
> is now used only for array/list literals and index access.

### Examples

#### Complete example

```yaoxiang
// ======== 1. Interface definitions ========
// An interface is a record type whose fields are all function types
// Interfaces do not need a self parameter—an interface defines only the function signature "with the caller's position removed"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // Returns the interface type; a concrete implementation returns its own type
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

// ======== 3. Method implementations (ordinary functions + explicit bindings)========

// Define a function (self is merely a conventional name, not a keyword)
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

// Explicit binding—dot-call syntax is available only after binding
Point.draw = draw[0]
Point.bounding_box = bounding_box[0]
Point.serialize = serialize[0]
Point.translate = translate[0]
Point.scale = scale[0]
Point.distance = distance[0]

// Rect's methods are defined similarly
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

// Method invocation (syntax sugar)
p.draw(screen)
r.draw(screen)

// Ordinary method invocation (direct call)
d: Float = distance(p, Point(0.0, 0.0))

// Chained invocation
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// Interface assignment
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// Generic function (RFC-023 syntax: type parameters are omitted at the call site and inferred automatically)
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
    // For each field of the interface (function field)
    for (field_name, iface_field) in &iface.fields {
        // Check whether the type has a method with the same name
        if let Some(method) = typ.methods.get(field_name) {
            // Check whether the method signature is compatible
            // Interface field: (Surface) -> Void
            // Method signature: (Point, Surface) -> Void
            // Comparison: they should match after removing the self parameter
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

Interface types support direct assignment. The compiler automatically selects the optimal dispatch
strategy based on the type of the right-hand value:

```yaoxiang
// Direct assignment of a concrete type → the concrete type can be determined at compile time, with zero-overhead calls
d: Drawable = Circle(1)
d.draw(screen)  // After compilation: direct call to circle_draw(screen), with no vtable

// Function return value → the concrete type cannot be determined at compile time, so a vtable is used
d: Drawable = get_shape()
d.draw(screen)  // The method is looked up through a vtable

// Heterogeneous collection → a vtable is used
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // The method is looked up through a vtable
}
```

**Compile-time optimization strategy**:

| Scenario                         | Inferred result        | Dispatch method             |
| -------------------------------- | ---------------------- | --------------------------- |
| `d: Drawable = Circle(1)`        | Concrete type `Circle` | Direct call (zero overhead) |
| `d: Drawable = get_shape()`      | Unknown                | vtable                      |
| `shapes: List(Drawable) = [...]` | Heterogeneous          | vtable                      |

**Rules**:

1. When the right-hand value is a concrete type constructor that can be determined at compile time,
   generate direct-call IR
2. When the right-hand type cannot be determined at compile time, fall back to the vtable mechanism
3. The vtable fallback ensures the correctness of runtime polymorphism

### Duck typing support

```yaoxiang
// A value may be assigned to an interface type as long as it has the same methods
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
| Requires the `impl` keyword              | No keyword required; write the interface name in the type body                               |

### Deprecated: `|` variant syntax

> **Deprecation announcement (2026-07-25)**: the `|` variant syntax is officially deprecated and
> removed from the implementation.

The following forms are **no longer supported**:

```
type Color = red | green | blue                # ❌ Deprecated
type Result(T, E) = ok(T) | err(E)             # ❌ Deprecated
type Option(T) = some(T) | none                # ❌ Deprecated
```

Use record types uniformly to express sum types. When every field of a record type is a function and
every function returns the record type itself, the record type is a sum type:

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

1. **Eliminating special cases**: `|` is the only grammatical form in the BNF that does not follow
   `name: type = value`. After its removal, the `type_expr` production is completely unified, and
   the parser no longer needs a separate path and lookahead backtracking for variant types.
2. **Mathematical equivalence**: under the Curry-Howard correspondence, the sum type corresponding
   to the disjunction P ⊕ Q is equivalent to a record type "whose fields are all functions returning
   the type itself." Both express the same semantics and require no two separate syntaxes.
3. **Zero breaking changes**: before removal, the `|` syntax was only partially supported by the
   parser (parameterless variants could be parsed, but parameter types were lost during
   monomorphization), and no user code depended on it.
4. **AST simplification**: the `Type::Variant(Vec<VariantDef>)` node is removed, all variant types
   follow the `Type::Struct` path, and all special-case branches in downstream
   typecheck/mono/formatter components are eliminated.

> **Note**: the semantic properties of sum types (variant construction, match exhaustiveness
> checking, and tagged union memory layout) are inferred by the typecheck layer from the
> `Type::Struct` structure and do not depend on a separate AST node. The complete semantics of
> variant construction is described in the following section, [Record-Style Sum Type Variant
> Construction (Authoritative Definition)].

### Record-style sum type variant construction (authoritative definition)

> This section expands the recognition criteria from the [Deprecated: `|` Variant Syntax] section
> into a complete semantic definition (finalized on 2026-09-25 as a prerequisite for issue #341
> RFC-011b Phase 2). Four decisions: type-qualified invocation, zero-payload variants taking the
> form of function calls, self-contained type inference, and all-or-nothing promotion of variant
> names.

#### Decision rule: all or nothing

A record type is classified as a **sum type** when it satisfies both of the following conditions:

1. Every field type in the type body is a function type;
2. After type-parameter substitution (`Self`/generic arguments), every field's return type is the
   record type itself.

This classification is **all or nothing**: when it succeeds, all function fields are simultaneously
promoted to variant constructors; when it fails, the entire type is an ordinary record (whose
function fields are data fields containing function values), and no hybrid form of "some variants
and some data" exists. When data and variant semantics must be carried simultaneously, model them as
two separate types—a data record and a sum type—and do not combine their declarations.

#### Invocation form: type-qualified, with no bare names

The **only** invocation form for a variant constructor is a type-qualified invocation: obtain the
variant member from the type value and call it:

```yaoxiang
r1 = Result(Int, String).ok(5)   // Result(Int, String), payload 5
c  = Color.red()                 // Zero-payload variant: takes the form of a function call matching the field signature
```

**No bare-name form is provided** (`ok(5)` is not a constructor invocation). The reason is that a
bare name depends on backward inference from the contextually expected type to determine its owning
sum type and type arguments, whereas types in this language must be explicitly writable and
inference must flow in one direction from the expected position (the same discipline as RFC-011a
["Self is an explicit type parameter; no magic"]). The qualified-name form is self-contained and
requires no context. If bare names are introduced in the future, they may only be expansion sugar
for cases in which "context uniquely determines the answer"; the qualified-name form remains the
semantic baseline.

#### Inference: self-contained types

Once the qualified name provides the complete type arguments, the signature of a variant constructor
is the **result of substituting the type arguments into the field signature**:

```yaoxiang
Result(Int, String).ok   // : (Int) -> Result(Int, String)
```

Payload type checking is ordinary function-call argument checking (a mismatch reports E1002); there
are no new inference rules. Generics and type constructors are naturally monomorphized as type
instances are created, with no separate mechanism.

#### Field promotion: variant names are not data fields

After a type is classified as a sum type, variant names are removed from the data-field
namespace—accessing a variant-name field on a **value** of the sum type is rejected at compile time:

```yaoxiang
r1.ok    // Compile error: ok is Result's variant constructor, not a data field
```

The reason is that a sum-type value is represented at runtime as a tagged union (see below) and does
not carry a variant declaration table. Allowing field access would inevitably silently misinterpret
the value. This extends the same discipline as RFC-011a §1.2 (a unified field/method namespace and
conflict errors). Construction uses type qualification (`Result.ok`), while consumption uses match
destructuring (RFC-010b).

#### Runtime representation and equality

The runtime representation of a sum-type value is a tagged union:
`Enum { type identity, variant_id, payload }`.

- The **type identity** carries the concrete sum type—not a global placeholder—so values of
  different sum types are incomparable;
- The **variant_id** is assigned according to the **declaration order** of the variants in the type
  body; this order also provides the variant-set input for RFC-010b exhaustiveness checking;
- **Equality** (`==`/`!=`): the type identities are identical, the variant_ids are identical, and
  the payloads are equal recursively.

#### std migration

`std.result` uses this mechanism to define `ok` / `err` (the native `result_ok` / `result_err`
functions are retired), and variant construction in `std` follows the same rules as user-defined sum
types, with no privileged path. Removal of the parser's special handling for `Result` / `Option` in
type positions and for `make_result` by name is handled by Phase 2 of
[RFC-011b](011b-operator-overloading.md)—once `?` is interface-based, `Result` becomes an ordinary
std sum type.

### Logical operators: `and` / `or` / `!` (authoritative definition, Zig-style)

> **Definition announcement (2026-08-03)**: the authoritative forms of the logical operators are the
> keywords `and` / `or` plus the symbolic unary operator `!` (consistent with the priority table in
> SPEC `syntax.md` §2.2). This design follows Zig: **short-circuit control flow uses keywords, while
> pure unary operations use symbols**. Earlier implementations that drifted toward C's `&&` / `||`,
> as well as the intermediate keyword `not`, have been removed.

**Semantics**:

| Operator | Priority (SPEC §2.2)              | Associativity | Semantics                                         |
| -------- | --------------------------------- | ------------- | ------------------------------------------------- |
| `!`      | 3 (unary prefix, tightly binding) | Right to left | Logical negation (pure function, no control flow) |
| `and`    | 10                                | Left to right | Short-circuit logical AND                         |
| `or`     | 11                                | Left to right | Short-circuit logical OR                          |

```yaoxiang
# Short-circuit evaluation: when the left side of and is false or the left side of or is true, the right side is not evaluated
if x != 0 and y / x > 1 { ... }   # No division by zero occurs when x == 0

# Tight binding: !a == b ≡ (!a) == b (Zig-style; opposite of Python's not a == b ≡ not (a == b))
!3 == 4          # false: (!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs)); negate the result of the call
```

The following forms are **no longer supported** (the lexer reports an error and suggests the
corresponding alternative):

```
x && y     # ❌ Removed; use x and y
x || y     # ❌ Removed; use x or y
not x      # ❌ Removed; use !x (not is restored as an ordinary identifier; != is unaffected)
```

**Design rationale** (aligned with Zig, ziglang/zig#272 / #6625):

1. **Short-circuiting is control flow → keywords; pure functions are operations → symbols**. `and` /
   `or` change evaluation order by skipping the right-hand side on demand, and are therefore
   analogous to `if`; `!` applies pure negation to an already evaluated operand and is therefore
   analogous to `-` and `+`. YaoXiang uses `?` for error propagation (§2.11), so `!` creates no
   conflict.
2. **Tight binding eliminates ambiguity**: `!` visually “hugs” its operand, making its high
   precedence immediately apparent. The keyword `not` requires whitespace before its operand, making
   it ambiguous which expression it binds to (such as `not a == b`).
3. **Disambiguation**: `&` already has two roles—the borrow token (`&p` / `&mut p`, RFC-009) and
   bitwise AND (priority 8 in SPEC §2.2). Introducing `&&` would give one symbol three meanings.
   `and` / `or` / `!` completely separate borrowing, bitwise operations, and logic visually.
4. **Precedent**: Zig (a modern systems language in a comparable ecosystem) combines the `and` /
   `or` keywords with the `!` symbol; Python / Lua / Ada / SQL use keywords exclusively (including
   `not`), while the C family uses symbols exclusively—YaoXiang takes Zig's hybrid approach and
   gains the benefits of both.
5. **Curry-Howard consistency**: types are propositions (see the correspondence section above), and
   logical conjunctions in refined types are naturally written using `and` / `or` (such as
   `{ 0 <= idx and idx < arr.len }`); `!` serves as the unary negation symbol corresponding to ¬.

> **Implementation**: `and` / `or` are expanded at the IR level into short-circuiting jump sequences
> (`a and b ≡ if a { b } else { false }`), and `!` is parsed as a tightly binding unary operator
> (with the operand parsed according to `BP_UNARY + 1`). Regression tests:
> `tests/yaoxiang/01-syntax/basics/logical_ops.yx`, `logical_not.yx`.

## Syntax design rationale: named functions are syntactic sugar for lambdas

### Core understanding

**Named functions and lambda expressions are the same thing!** The only difference is that a named
function gives a lambda a name.

```yaoxiang
// These two are essentially completely identical
add: (a: Int, b: Int) -> Int = a + b           // Named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda form (completely equivalent)
```

### Syntactic-sugar model

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially
name: (Params) -> ReturnType = (params) => body
```

**Key point**: when a signature fully declares parameter types, the parameter names in the lambda
header become redundant and may be omitted.

### Parameter scope rules

**Parameters shadow outer variables**: the scope of a parameter in the signature covers the function
body, and the inner scope has higher precedence.

```yaoxiang
x = 10  // Outer variable

double: (x: Int) -> Int = x * 2  // ✅ Parameter x shadows outer x; the result is 20
```

### Flexible annotation placement

Type annotations may be placed in any of the following locations; **at least one annotation is
required**:

| Annotation location | Form                                     | Description            |
| ------------------- | ---------------------------------------- | ---------------------- |
| Signature only      | `double: (x: Int) -> Int = x * 2`        | ✅ Recommended         |
| Lambda header only  | `double = (x: Int) => x * 2`             | ✅ Valid               |
| Both                | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but valid |

### Complete example

```yaoxiang
// ✅ Recommended: complete signature; omit the lambda header
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Valid: annotate the type in the lambda header
double = (x: Int) => x * 2

// ✅ Valid: annotate both sides
double: (x: Int) -> Int = (x) => x * 2
```

### Design advantages

| Feature        | Advantage                                                                                         |
| -------------- | ------------------------------------------------------------------------------------------------- |
| **Concise**    | Parameter names need not be repeated when the signature is complete                               |
| **Flexible**   | Supports the lambda form; use whichever is preferred                                              |
| **Consistent** | Uses the same pattern as variable declaration `x: Int = 42`                                       |
| **Intuitive**  | `name: Type = body` directly expresses "a value named name, of type Type, with body as its value" |

## Trade-offs

### Advantages

| Advantage            | Description                                            |
| -------------------- | ------------------------------------------------------ |
| Maximum unification  | One syntax rule covers every case                      |
| Theoretical elegance | Perfect symmetry of `name: type = value`               |
| No new keywords      | Reuses existing syntactic elements                     |
| Easy to implement    | The compiler only needs to handle one declaration form |
| Easy to learn        | Remember one pattern to write all code                 |
| Easy to extend       | New features fit naturally into this model             |

### Disadvantages

| Disadvantage       | Description                                                                           |
| ------------------ | ------------------------------------------------------------------------------------- |
| Naming conventions | Methods must follow the `Type.method` naming convention                               |
| Verbosity          | The complete syntax is longer than the simplified syntax, although it can be inferred |
| Learning curve     | The unified model must be understood                                                  |

### Mitigations

```yaoxiang
// 1. Clear error messages
// Example compilation error:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Type annotations may be omitted and inferred by the compiler
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE hints
// The IDE automatically suggests missing methods
```

### Risks

| Risk                 | Impact                                        | Mitigation                                 |
| -------------------- | --------------------------------------------- | ------------------------------------------ |
| Parser complexity    | Unified syntax may increase parser complexity | Use a recursive-descent parser             |
| Performance overhead | vtable lookups may incur additional overhead  | Compile-time monomorphization optimization |

---

## Easter Egg 🎮: The Source of the Language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Try to define the type of types...
Type: Type = Type
```

**Warning**: this is an **ineffable** thing!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二，二生三，三生万物。                                   ║
║   易有太极，是生两仪。                                         ║
║                                                              ║
║   Type: Type = Type                                          ║
║   此乃爻象之源，语言之边界。                                   ║
║   编译器在此沉默，哲学在此驻足。                               ║
║                                                              ║
║   感谢你触达语言的哲学边界。                                   ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Note**: the compiler cannot correctly process `Type: Type = Type` (it causes a Type0/Type1
> universe paradox), but we have deliberately preserved this "Easter egg"—when you try to compile
> it, you receive a zen message from the language's creator. This is not merely a technical
> boundary, but also YaoXiang's tribute to the philosophy of types.

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

# Generic parameters: part of a function type, e.g. (T: Type, R: Type) -> (...)
# No separate BNF rule is needed—the : Type parameters are ordinary function parameters

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

| Term               | Definition                                                                                            |
| ------------------ | ----------------------------------------------------------------------------------------------------- |
| Declaration        | An assignment statement of the form `name: type = value`                                              |
| Record type        | A `{ ... }` type containing named fields                                                              |
| Interface          | A record type whose fields are all function types                                                     |
| Generic type       | A type defined as `Name: (T: Type) -> Type = { ... }` that accepts type parameters                    |
| Namespace function | A function of the form `Type.name` that belongs to the Type namespace; it implies no binding          |
| Method binding     | `Type.name = func[n]`, binding position n of func as the caller so that `obj.name(args)` is available |
| Generic function   | A function using `(T: Type)` syntax, with type parameters as its first parameter group                |
| Metatype           | `Type`, the language's only type-level marker                                                         |

---

## Lifecycle and destination

```
┌─────────────┐
│   草案      │  ← 当前状态
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 开放社区讨论和反馈
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```
