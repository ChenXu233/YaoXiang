# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals a deep correspondence between the type systems of programming languages and mathematical
logic:

| Logic                                          | Programming Language                        |
| ---------------------------------------------- | ------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                 |
| Proof \(p: P\)                                 | Program `x: T = ...`                        |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                    |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`               |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                 |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                 |
| Truth \(\top\)                                 | `Void` (Unit, with default)                 |
| Falsehood \(\bot\)                             | `Never` (zero constructors, no inhabitant)  |
| Type universe \(Type_n : Type_{n+1}\)          | Universe levels (prevent Russell's paradox) |
| case analysis                                  | Type-level `match`                          |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + a compiler termination check.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (e.g., `Add` case analysis + recursive call on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checks.
- **Type checking is proof verification**. When a program passes type checking, the equivalent of a
  logical proposition has been constructively proved.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe levels** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradoxes (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive call on the natural number
   `Nat(Zero/Succ)` corresponds to Peano axioms—provided the compiler performs termination checks
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
   quantification: "for every integer N there exists a type"

---

## Chapter 1: Type Classification

### 1.1 Type Expressions

```
TypeExpr    ::= PrimitiveType
              | RecordType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
```

> **Design Note**: Although RFC-010 proposes a unified "everything is assignment" model
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), and `TypeExpr` is a BNF placeholder corresponding to the `Type` enum in the
> implementation, indicating "a type is expected at this position".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Counterpart | Description                                                                               | Default Size |
| -------- | ------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                   | Meta type                                                                                 | 0 bytes      |
| `Never`  | ⊥ (False/Empty)     | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (True/Unit)       | Has a default void value, zero-field product type. `x: Void = <default>` is legal.        | 0 bytes      |
| `Bool`   | —                   | Boolean value: `true` / `false`                                                           | 1 byte       |
| `Int`    | —                   | Signed integer                                                                            | 8 bytes      |
| `Float`  | —                   | Floating-point number                                                                     | 8 bytes      |
| `String` | —                   | UTF-8 string                                                                              | variable     |
| `Char`   | —                   | Unicode character                                                                         | 4 bytes      |
| `Bytes`  | —                   | Raw bytes                                                                                 | variable     |

Integer types with explicit widths: `Int8`, `Int16`, `Int32`, `Int64`; floating-point with explicit
widths: `Float32`, `Float64`. The complete table of built-in type names is in
`MonoType::from_builtin_name` at `src/frontend/core/types/mono.rs:618-640`.

> **There is no unsigned integer type**. `Uint` (and `Int128`) are not usable type names—`Uint` only
> appears in the LSP completion candidate list at `src/lsp/world.rs:176` and the `sizeof` fallback
> branch at `src/frontend/core/types/eval/const_eval.rs:503`; neither is a type registration. When
> you need unsigned semantics, use `Int` with explicit lower/upper bounds.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to False (⊥) and
Truth (⊤) respectively.

**Never (⊥, False/Empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing writable on the right-hand side.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   and subsequent code passes type checking (although it is never actually executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered the same way as `Int`/`Bool`), not a keyword.

**Void (⊤, True/Unit)** — has exactly one inhabitant (the default void value). `Void` is the unit of
zero-field product types. `x: Void = <default>` is legal. A block's value is given by its **tail
expression** (an empty block `{}` is `Void`); see
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md).

---

## Chapter 3: Composite Types

### 3.1 Record Types

**Unified syntax**: `Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // interface constraint
```

```yaoxiang
// Simple record type
Point: Type = { x: Float, y: Float }

// Empty record type
Empty: Type = {}

// Generic record type
Pair: (T: Type) -> Type = { first: T, second: T }

// Record type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Rules**:

- Record types are defined using curly braces `{}`
- The field name is directly followed by a colon and a type
- Interface names written in the type body indicate the interface is implemented

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. For the `.` call syntax
> such as `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004
> and RFC-010.

#### 3.1.1 Field Default Values

Type fields may specify default values; providing them during construction is optional:

```yaoxiang
// Field with default value - optional during construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without default value - required during construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // Correct
Point2()          // Error
```

**Rules**:

- `field: Type = expression` -> has a default value, optional during construction
- `field: Type` -> no default value, required during construction

#### 3.1.2 Built-in Bindings

Methods can be bound directly inside a type definition body:

```yaoxiang
// Method 1: reference an external function via binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // Bind to position 0
}
// Call: p1.distance(p2) -> distance(p1, p2)

// Method 2: anonymous function + position binding
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
// Call: p1.distance(p2) -> distance(p1, p2)
```

### 3.2 Interface Types

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Syntax**: An interface is a record type whose fields are all function types

```yaoxiang
// Interface definition
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Empty interface
EmptyInterface: Type = {}
```

**Interface implementation**: A type implements an interface by listing the interface name at the
end of its definition

```yaoxiang
// A type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implements the Drawable interface
    Serializable     // Implements the Serializable interface
}
```

**Direct interface assignment**: A concrete type can be assigned directly to an interface-typed
variable (structural subtyping)

```yaoxiang
// Direct assignment (compile-time known concrete type -> zero-cost call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: directly calls circle_draw, no vtable

// Function return (compile-time unknown -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Looks up the method through the vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                           | Inferred Result     | Call Method                 |
| ---------------------------------- | ------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type known | Direct call (zero overhead) |
| Function return value              | Unknown             | vtable                      |
| Heterogeneous collection           | Multiple types      | vtable                      |

**Coherence and orphan rules (not applicable, closed explanation)**: YaoXiang's interfaces are
structural types (interface = record with all-function fields), not nominal traits—there is no "who
can implement for whom" attribution problem across crates/modules, so the Rust-style orphan rule and
coherence check have no applicable target (decision recorded in RFC-011 §2.1). The corresponding
guarantee in the structural world is the **duplicate-implementation rejection**: re-defining the
same method signature on a type is a compile error (RFC-011a §3, overriding forbidden; overloading
is legal).

### 3.4 Tuple Types

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 Function Types

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

---

## Chapter 4: Generics

### 4.1 Generic Parameter Syntax

Generic parameters are part of the function type and use the same `()` syntax as ordinary
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` denotes the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they are treated the same as
user-defined generics, processed via the unified generic instantiation path. The ownership of length
information is the fundamental distinction between the three container concepts:

| Type          | Length        | Semantics                                | Foundation                                        |
| ------------- | ------------- | ---------------------------------------- | ------------------------------------------------- |
| `Array(T, N)` | type-level    | Fixed-length array (const generic N)     | Core primitive (stack/inline preferred)           |
| `Vec(T)`      | runtime value | Raw buffer with runtime length, growable | Core primitive (heap-allocated contiguous buffer) |
| `List(T)`     | runtime value | Standard library type (growable list)    | Library: `{ data: Vec(T), length: Int }`          |
| `Dict(K, V)`  | runtime value | Key-value mapping                        | `HeapValue::Dict`                                 |

> `List(T)` is a **standard library type, not a compiler primitive**: it is defined in YaoXiang
> itself in `std.list`, treated the same as user-defined generic records. All growable semantics
> strategies (when to expand, by how much, sharing eligibility) live in the library; the compiler is
> not involved. `Vec(T)` is the minimal ground primitive on which it depends.
>
> `Set(T)` has been removed: no literal, no runtime representation, no `std.set`. When a need
> arises, fill it in following the `Dict` pattern.

Key rules:

- **Literal landing point is determined by context**: a bare `[...]` literal combined with a
  `List(T)` annotation lands on a growable list; an `Array(T, N)` annotation applied directly to a
  literal lands on a fixed-length array. Landing-point check: element count == N, element type
  compatible with T; mismatch is compile-time `E1002`; when N is a symbolic constant (const
  parameter), the count check is deferred to the refinement-typing phase.
- **No implicit `List`→`Array` conversion**: fixed-length property is guaranteed at the type
  level—`push` only accepts a `List(A)` receiver.
- **Performance tiers**: from bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Indexing failure contract** (runtime error is transitional, target state is compile-time
  refinement coverage, via value-dependent types, see §8.4):
  - Index out of bounds (including negative indices) → `E6003`
  - Dict missing key → `E6008`
- **`in` membership predicate**: returns `Bool` without erroring; the right-hand operand covers
  `List`/`Array`/`Dict` (keys)/`Tuple`/`String`/`Range`. A first-class Hoare predicate, the
  foundation of compile-time provable propositions in refinement types.`

In a generic function, type parameters are similarly declared in the signature; the compiler
automatically infers them from the actual arguments:

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 Generic Type Definitions

```yaoxiang
// Basic generic type
Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T)
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E)
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,   // self is just a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation—the
receiver type must implement the `Try` interface (four members: `is_failure` / `success` /
`residual` / `from_error`); on failure it returns early from the current function with
`from_error(residual(t))`; on success the expression's value is `success(t)`. The outer function's
return type must also implement `Try`, and its failure residual type must match the receiver's
(`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try` implementations from
`std.result` / `std.option` (the failure residual type of `Option` is `Void`); a user-defined sum
type opts in to `?` by writing `Try(Self, T, E)` inside its type body. The lowering of `?` does not
distinguish built-in from user types—the same interface, the same path.

The **variant set of a sum type enters the checker via the definition or `use` import**:
`Result`/`Option` must be preceded by `use std.result` / `use std.option` (whole-module and grouped
`use std.{...}` are equivalent); variant construction (type-qualified form), `match` variant
deconstruction, and exhaustiveness checks all rely on this registration as their sole basis; native
function signatures returning `Result(Float, Error)` mint a same-named `Generic`, which is
identity-equal by name to `std.result`'s sum type, and is `match`-deconstructable after `use`.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to one constructor parameter, and the field name is the parameter name; fields
with default values can be omitted during construction, fields without default values are required.
Function-typed fields (methods) do not generate constructor parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value -> constructor parameter required
    extra: T,
}
// Auto-expanded full form (compiler's internal view; users don't need to write this):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invoke the auto-generated constructor
c  = Container(42, 43)            // Constructor parameters filled by field order; T auto-unpacked from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type argument + positional constructor parameters
c4 = Container(Int)(extra=43, value=42)  // Field-name style, order arbitrary
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned later)

// Field default value -> constructor parameter can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Calling rules** (single parentheses, match position by position against declared parameters, left
to right):

1. Actual arguments attempt to match the declared parameters position by position: a `Type` position
   accepts a type argument; a compile-time value position (e.g., `Int`) accepts a compile-time
   constant.
2. If a compile-time value position matches successfully (partial match), treat it as a type
   construction: check every parameter position, and on error report **the first mismatched/missing
   parameter** in declaration order.
3. If the actual arguments do not correspond to the declared parameters at all (all are values, no
   compile-time value position matches), treat as constructor parameters: fill by field order
   positionally; type parameters are auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one layer of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two layers: type + constructor parameters
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ pos 0: T←42 doesn't match (42 is not a type); pos 1: Rows←42 matches;
              //    pos 2: Cols missing -> report first error: T expected Type, found 42
Container(42) // ❌ Missing constructor parameter extra
Container(42, 43, 44)  // ❌ Too many constructor parameters
```

**Type inference**: Type parameters of a generic type constructor are auto-unpacked from the
elements of constructor arguments (`Container(42, 43)` → T=Int); type parameters of a generic
function are auto-unpacked from the types of actual arguments (`map(numbers, f)` → T=Int, R=String,
see §4.1). When unpacking fails, explicit specification is required.

---

## Chapter 5: Type Constraints

### 5.1 Single Constraint

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// Interface type definition (as a constraint)
Clone: Type = {
    clone: () -> Clone
}

// Using constraints
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

> **Resolution source of constraints (RFC-011b)**: Resolution of operator constraint names (`Add` /
> `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) = lookup in the interface
> implementation registry—`T: Add` ≜ an `Add(T, T, T)` instantiation is registered; `Equal`
> additionally has structural inference (records whose fields are all comparable are automatically
> comparable). Names like `Zero` / `One` / `PartialOrd` have no defined source yet and are dangling
> constraint names.

```yaoxiang
// Multiple-constraint syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// Sorting a generic container
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 Function Type Constraints

```yaoxiang
// Higher-order function constraints
call_twice: (T: Type, F: () -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: (A) -> B, G: (B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## Chapter 6: Associated Types

### 6.1 Associated Type Definition

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (using record type syntax)
Iterator: (T: Type) -> Type = {
    Item: T,                    // Associated type
    next: () -> Option(T),
    has_next: () -> Bool
}

// Using associated types
collect: (T: Type, I: Iterator(T))(iter: I) -> List(T) = {
    result = List(T)()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}
```

### 6.2 Generic Associated Types (GAT)

```yaoxiang
// More complex associated types
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // Associated type that is also generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-time Generics

### 7.1 Compile-time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant (candidate)
```

> The criterion is **being referenced at a type position**, not "annotated with a specific type": in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters (neither appears at
> any type position).

**Terminology**: a generic parameter annotated with a concrete type other than `Type` (e.g., `Int`)
is called a **compile-time value parameter candidate**; whether it becomes a compile-time value
parameter depends on whether its value is referenced at a type position (value-dependence). **No
`const` keyword is needed** (the implementation internally used to call this "const generics";
documentation uniformly uses "compile-time value parameters").

**Determination rule (two steps)**:

1. **Shape coarse filter**: the parameter is annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use refinement filter**: the candidate name appears at a **type position** (field type in a
   type body, inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction
   argument) → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing                                                    | Determination                             | Reason                                           |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear at value positions                   |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is compile-time value parameter         | N appears in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is compile-time value parameter         | N serves as the type of inner parameter k        |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N not referenced in type body                    |

**Core design**: use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. A fall-through candidate (shape is a
candidate, but usage doesn't match) degrades to a runtime value parameter—this applies to both the
function-level and type-constructor paths.

```yaoxiang
// Compile-time value parameter: N is referenced at a type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at a type position (compile-time), result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // Compiler evaluates factorial(5) = 120 at compile time

// Value-dependence: N as the type of inner parameter k
// N is a compile-time value parameter (appears in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 Compile-time Constant Arrays

```yaoxiang
// Matrix type usage
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// Compile-time dimension validation
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## Chapter 8: Conditional Types

### 8.1 If Conditional Type

```
IfType        ::= 'If' '(' BoolExpr ',' TypeExpr ',' TypeExpr ')'
```

```yaoxiang
// Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// Example: compile-time branch
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue bridging and Assert refinement type (see §8.3)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, divergence / compile error
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 Type Families

```yaoxiang
// Compile-time type conversion
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 8.3 Assert Refinement Types and the `assert` Assertion

`assert` and `Assert` are two sides of the same refinement primitive—chosen automatically by the
dispatch pipeline based on whether the predicate's free variables are compile-time reachable.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rule**:

| Criterion                                                                              | Mode        | Behavior                                                                                          |
| -------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------- |
| All free variables are compile-time known (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erase to `Void`; false → compile error (`Never` cannot be inhabited) |
| Runtime free variables exist (function parameters, external inputs)                    | Runtime     | Insert runtime `Bool` check; inject refinement fact into flow-sensitive assumption set Γ          |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After assignment to a `mut` variable, all assumptions involving that variable are removed (kill
set). At branch join points, Γ is the intersection of each branch's Γ.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, belonging to the core primitives alongside `Int` and
`Never` (built-in name, not a keyword). It binds a **measure** to a piece of computation, declaring
that the computation terminates and providing a witness of termination.

**Forms**: two arities, same predicate:

| Form                    | Anchor                 | Use                                                                                                                  |
| ----------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | the binding's name     | Default form—self-recursive functions, loops                                                                         |
| `Terminates(FnType, m)` | explicit function type | When the measure must be specified explicitly (measure defined elsewhere, same measure serves multiple computations) |

```yaoxiang
// Measure: a regular function, unit-testable, reusable, not part of runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Two-argument form: measure defined elsewhere, explicitly state ownership
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// One-argument form: the anchor is the binding name; the measure is an expression in scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines the value type of the computation annotated at its type
position. The obligation falls on that computation—on every recursive call site for a function, on
the back-edge for a loop—each being "the measure of the next state is strictly less than the measure
of the current state", judged under the path guard at that point.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back-edge is
`m(next_iteration) < m(this_iteration)`. The two forms are identical.

> **Why it covers loops**: a loop is an anonymous construct, usually not nameable. The binding name
> is the name—the `acc` in `acc: Terminates(n - i) = while ...` provides the anchor, so the loop
> becomes nameable. This is the rationale for the one-argument form of `Terminates` applying to
> loops.

**Measure**: the return type is unrestricted (not forced to be a natural number); the "strictly
decreasing" order is given by the well-founded order available on that type. Whether the measure is
well-founded (e.g., whether it is `>= 0` when returning `Int`) is an **independent obligation**,
equally judged by the compile-time proof pipeline as the decreasing obligation.

**Trigger**: termination checking is triggered by **refinement types**—once a type is refined, the
validation mode is entered. Unrefined ordinary types (such as a bare `while` loop, a function with
no refinement signature) do not enter validation mode and do not generate termination obligations.

**Automatic exploration first**: the compiler first automatically explores measures (four templates:
linear rank function, predicate violation counter, bounded increase/decrease, multiplicative
scaling); only when exploration fails is an explicit `Terminates` required.

**Runtime representation**: a pure compile-time entity, erased along with the witness, not present
in the runtime binary.

> See the full design in
> [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md) (implementation
> mechanism).

---

## Chapter 9: Type Union and Intersection

### 9.1 Type Union

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 Type Intersection

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**Syntax**: The type intersection `A & B` represents the type satisfying both A and B

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using the intersection type
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## Chapter 10: Function Overloading and Specialization

### 10.1 Function Overloading

```yaoxiang
// Basic specialization: function overloading (compiler chooses automatically)
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// Generic implementation
sum: (T: Add)(arr: Array(T)) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

### 10.2 Platform Specialization

```yaoxiang
// Platform type enum (defined in the standard library)
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P is a predefined generic parameter name representing the current compilation platform
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 11: Type Properties

YaoXiang has only one type property to distinguish: linear vs. copyable. It is inferred
automatically by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership
transfer.

```yaoxiang
// A struct containing a heap-buffer field does not derive Dup (field lands on Move) -> default Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move, b cannot be read again
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types and reference-counted handle value types**.
Assignment of a Dup type = shallow copy—copy the handle/token, the underlying data is shared.
Multiple holders point to the same data block.

| Type            | Property | Description                                                                              |
| --------------- | -------- | ---------------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token, copying a token = multiple views pointing to the same data        |
| `ref T`         | Dup      | Rc/Arc copy = reference count + 1, sharing heap data                                     |
| String, Bytes   | Dup      | Internal reference counting, assignment copies the handle to share the underlying buffer |
| `&mut T`        | Linear   | Zero-sized write token, exclusive, not copyable                                          |
| struct          | derived  | All fields are copyable (primitive value types ∪ Dup) → Dup; otherwise Move (#398)       |
| tuple           | derived  | Element-by-element, same rule as struct (#398)                                           |
| All other types | Move     | Default ownership transfer                                                               |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: assignment
automatically performs value copying, the two values are completely independent. This is the
compiler's native behavior and does not belong to the Dup type property.

**Derivation rule** (decided in #398) — "all fields are Dup implies auto-derived Dup" cannot be
executed literally: primitive fields (Int etc.) are themselves not Dup, so `{ x: Int, y: Int }`
would be misjudged as Move. Executable form:

1. **Copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), already-Dup composite types);
2. **struct**: all fields are in the copyable field set → derives Dup; **any** field lands on Linear
   (`&mut T`) or Move (nested Move struct / Vec, Dict, etc. containers / resources) → the whole
   remains Move (no "partially copyable" intermediate state);
3. **tuple**: same rule as struct, judged element by element; empty tuple (unit) is `Void`;
4. **Derivation is recursive**: when a field is a named type (e.g., `target: Point`), expand its
   definition and judge; `A = { b: B }` follows B's derivation result; cyclic aliases conservatively
   land on Move by a depth limit;
5. **Not in derivation scope** (remain Move, separate case): containers (Vec / Dict / Set / Option /
   Result / Array) and enum.

```yaoxiang
// &T: Dup, free to alias
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // Usable
print(view2.x)   // Usable

// &mut T: Linear, not copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot copy
```

### 11.3 Clone (Explicit Deep Copy) and Its Relation to Dup

**Clone** is the explicit deep-copy interface. All types can implement Clone, providing a `.clone()`
method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // Deep copy, p is still usable
p2 = p.clone()        // Can be cloned multiple times
```

**Difference between Dup and Clone**:

|                      | Dup                                                     | Clone                                            |
| -------------------- | ------------------------------------------------------- | ------------------------------------------------ |
| **Semantics**        | Shallow copy: copy handle/token, underlying data shared | Deep copy: create a complete independent replica |
| **Invocation**       | Implicit (automatic on assignment/parameter passing)    | Explicit (`.clone()`)                            |
| **Mutation effect**  | Mutually affect each other (sharing underlying data)    | Mutually independent (independent replicas)      |
| **Applicable types** | `&T` tokens, `ref T`                                    | Any type implementing the Clone interface        |
| **Cost**             | Zero overhead (tokens are zero-sized types)             | Depends on the type                              |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data is shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable, looking at the same data

// Primitive value type: compiler auto-copies the value (not Dup)
x: Int = 42
y = x               // Value copy, x and y are completely independent
print(x)            // Usable

// Clone: explicit deep copy, create an independent replica
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable

// Non-Dup type (field lands on Move, not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field, does not derive Dup (see §11.2 derivation rule)
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views on the same data"
- Clone is used when an independent replica is needed, making the cost visible through explicit
  invocation
- Copying of primitive value types (Int/Float/Bool/Char) is a compiler built-in behavior, not Dup
- Custom types default to Move (zero-copy high performance); Dup is auto-derived when all fields are
  copyable (§11.2 derivation rule)

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references", but
"type-level proofs of access permission".

```
&T      →  Zero-sized, freezes the source data (prohibits WriteToken acquisition during this period),
          safe to have multiple read-only under the freeze guarantee → Dup (copyable)
&mut T  →  Zero-sized, exclusive read-write (prohibits any other token),
          copying is meaningless under exclusive access → Linear (non-Dup)
```

**Key properties**:

- A token is an **ordinary type**, following the same scoping rules as any other type
- No lifetime annotation `'a` is needed
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally derive permissions
- Completely disappears after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare the parameter type, which determines the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call site: the compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates a &Point token
p.shift(1.0, 1.0)               // Compiler automatically creates a &mut Point token
p.print()                       // OK, the previous token was released when shift's call ended

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations of ordinary types:

**Returning tokens**—tokens propagate together with the return value:

```yaoxiang
// ✅ Sub-token and parent token are returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Token returned to the caller
print(px_ref)                    // OK, the token is still in scope
```

**Storing in a struct**—a struct can carry token fields:

```yaoxiang
// ✅ Struct carrying a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // Token field—holding a read-only view of target
}
```

**Closures don't capture, context is fixed at creation site**—a closure only consumes its own
parameters; when external data is needed, the value is fixed into the closure at creation through
currying:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter; gt_point(threshold) fixes the value into the closure at creation
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: once a closure (function value) escapes, the scope at its definition site may already be
> dead, so it must not implicitly capture outer variables; but the call site (creation site) scope
> is necessarily alive, and fixing the context into the closure as a value at that point is safe.

### 12.4 Automatic Borrow Selection

The compiler at the call site automatically selects according to the following priority:

```
1. If the actual argument is used later → prefer to create a token (&T or &mut T, depending on the method signature)
2. If the actual argument is not used later → Move
3. Priority order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates a &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates a &mut Point token
p2 = p             // Not used afterward → Move
```

**Method receiver follows the signature semantics** (same as the receiver spelling convention in
RFC-011a): receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value →
Move (consume the receiver). The borrow token created at the call site is released when the call
ends (transient, see §12.5 interval semantics); an interface's borrow receiver is explicitly
declared by the interface author as `&Self`, and the impl signature, after substituting
`Self ↦ impl type`, must exactly match the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrowing Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and feeds them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse-BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be simultaneously live
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ The token is automatically released after its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Use &mut Point
    }
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken still usable
}

// ❌ The same actual argument cannot simultaneously create &mut and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Brand Mechanism

Users never come into contact with brands. The compiler internally assigns a compile-time unique
identifier to each token:

```
What users see           Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: a token can only be obtained from the owner's capsule, it cannot be fabricated
  out of thin air
- **Correlation tracking**: a derived `&Float` from a field access carries a derived brand
  (`#N.field_x`), which the compiler can trace back to the parent token
- **Conflict detection**: a same-source WriteToken and a derived ReadToken cannot be simultaneously
  live

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead**.

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read-write → Linear)
```

### 12.8 Borrow Tokens vs `ref`

|                 | `&T` / `&mut T`                                                 | `ref`                                 |
| --------------- | --------------------------------------------------------------- | ------------------------------------- |
| What            | Look once / modify in place                                     | Shared ownership                      |
| Scope           | Follows the token value's scope                                 | Cross-scope                           |
| Cost            | Zero overhead (zero-sized type, disappears after compilation)   | Rc or Arc (compiler chooses)          |
| Escape          | Allowed (token propagates with return value/struct)             | Designed to escape                    |
| Cross-task      | Not allowed (tokens are not implemented for cross-task passing) | Allowed (compiler auto-selects Arc)   |
| Cycle detection | Not involved                                                    | Silent within a task, cross-task lint |

> Note (undefined): how to read the contents of a `ref` after creation (dereference/method/auto) has
> not yet been defined in the spec; the current implementation has `*a` reporting `E1052`. To be
> filled into this section after definition.

---

## Appendix: Type Definition Cheat Sheet

### A.1 Type Definitions

```
// === Record types (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Interface types (curly braces, all fields are functions) ===

// Interface definition
Serializable: Type = { serialize: () -> String }

// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implements the Serializable interface
}

// === Function types ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (built-in predicate, see §8.4) ===

// One-argument: anchor is the binding name (self-recursive functions, loops)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// Two-argument: explicitly state measure ownership (measure defined elsewhere)
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 Generic Syntax

```
// Generic type
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Generic function
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// Type constraints
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated types
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Compile-time generics: N referenced at type position (k: N) -> compile-time value parameter
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Property Cheat Sheet

```
// === Move (default) ===
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // Auto value copy on assignment, the two values are completely independent
Bool, Char      // Not Dup; compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-sized read token, copy the token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count + 1, share heap data
String, Bytes   // Internal reference counting, copy the handle to share the underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) → Dup (#398)

// === Not derived (remains Move) ===
Vec/Dict/Set    // Containers and enum are not in the derivation scope (#398)

// === Linear ===
&mut T          // Zero-sized write token, Linear (exclusive, not copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Create an independent replica, modification does not affect the original
```

### A.4 Borrow Token Cheat Sheet

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token, freezes source data → Dup (copyable)
&mut T          // Zero-sized compile-time write token, exclusive read-write → Linear (not copyable)

// Call site auto-selection
// 1. Actual argument is used later → create a token
// 2. Actual argument not used later → Move
// 3. Priority: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in struct, captured by closure
// ❌ Cannot cross tasks (tokens are not implemented for cross-task passing)
```
