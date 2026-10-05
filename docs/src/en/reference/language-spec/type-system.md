# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals a deep correspondence between a programming language's type system and mathematical logic:

| Logic                                          | Programming Language                            |
| ---------------------------------------------- | ----------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                     |
| Proof \(p: P\)                                 | Program `x: T = ...`                            |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                        |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                   |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                     |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                     |
| True \(\top\)                                  | `Void` (Unit, has a default value)              |
| False \(\bot\)                                 | `Never` (zero constructors, uninhabitable)      |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (e.g., case analysis + recursive calls of `Add` over `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, the equivalent
  logical proposition is constructively proved.

### 0.3 Impact on Language Design

Concrete embodiments of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`.
2. **Type families** (RFC-011): Type-level case analysis + recursive calls on natural numbers
   `Nat(Zero/Succ)` correspond to the Peano axioms—provided the compiler performs termination
   checking.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
   quantification "for each integer N there exists a type."

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

> **Design note**: Although RFC-010 proposes a unified "everything is an assignment" model
> (`name: type = value`), at the syntactic level, types and values still need to be distinguished.
> In the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`); `TypeExpr` is a BNF placeholder corresponding to the `Type` enum in the
> implementation, indicating "a type is expected at this position."

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                                   | Default Size |
| -------- | ---------------------- | --------------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                     | 0 bytes      |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no values. Return type for divergence/panic. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)          | Has a default void value, zero-field product type. `x: Void = <default>` is valid.            | 0 bytes      |
| `Bool`   | —                      | Boolean: `true` / `false`                                                                     | 1 byte       |
| `Int`    | —                      | Signed integer                                                                                | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                         | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                                  | variable     |
| `Char`   | —                      | Unicode character                                                                             | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                     | variable     |

Fixed-width integers: `Int8`, `Int16`, `Int32`, `Int64`; fixed-width floats: `Float32`, `Float64`.
The complete list of built-in type names is in `MonoType::from_builtin_name` at
`src/frontend/core/types/mono.rs:618-640`.

> **There is no unsigned integer type**. `Uint` (as well as `Int128`) is not an available type
> name—`Uint` only appears in the LSP completion candidate table at `src/lsp/world.rs:176` and in
> the `sizeof` fallback branch at `src/frontend/core/types/eval/const_eval.rs:503`; neither is a
> type registration. When you need unsigned semantics, use `Int` with lower/upper-bound conventions
> yourself.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤), respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   and subsequent code passes type checking (though it is never reached).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis and `match` branch conflation.

`Never` is a built-in type name (registered via the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — exactly one inhabitant (the default void value). `Void` is the identity
element of the zero-field product type. `x: Void = <default>` is valid. The value of a block is
given by the **tail expression** (an empty block `{}` is `Void`); see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for details.

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

// Record type with generics
Pair: (T: Type) -> Type = { first: T, second: T }

// Record type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Rules**:

- Record types are defined using curly braces `{}`.
- A field name is followed directly by a colon and the type.
- An interface name written inside the type body indicates that the interface is implemented.

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) means the function belongs to
> the `Point` namespace. It does not trigger any implicit binding. For the `.` call syntax like
> `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and
> RFC-010 for details.

#### 3.1.1 Field Default Values

A type field can specify a default value and is optional at construction:

```yaoxiang
// Field with default value - optional at construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without default value - required at construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // correct
Point2()          // error
```

**Rules**:

- `field: Type = expression` -> has a default value, optional at construction.
- `field: Type` -> no default value, required at construction.

#### 3.1.2 Builtin Bindings

Methods can be bound directly inside a type definition body:

```yaoxiang
// Method 1: Reference external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bind to position 0
}
// Call: p1.distance(p2) -> distance(p1, p2)

// Method 2: Anonymous function + position binding
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

**Syntax**: An interface is a record type whose fields are all function types.

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

**Interface implementation**: A type implements interfaces by listing the interface names at the end
of its definition.

```yaoxiang
// Type that implements interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // implement Drawable interface
    Serializable     // implement Serializable interface
}
```

**Direct interface assignment**: A concrete type can be assigned directly to a variable of an
interface type (structural subtyping).

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Look up method through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategies**:

| Scenario                           | Inferred Result            | Call Method                 |
| ---------------------------------- | -------------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value              | Unknown                    | vtable                      |
| Heterogeneous collection           | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, finalizing note)**: YaoXiang's interfaces are
structural types (an interface is a record whose fields are all function types), not nominal
traits—there is no cross-crate/module "who can implement for whom" ownership question; Rust-style
orphan rules and coherence checks have no applicable target (see RFC-011 §2.1 for the ruling). The
corresponding guarantee in the structural world is **duplicate implementation rejection**: defining
the same method signature twice on a type is a compile-time error (RFC-011a §3, override forbidden;
overloading allowed).

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

Generic parameters are part of the function type, sharing the same `()` syntax as ordinary
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` is the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics and are processed through the unified generic instantiation path.
The ownership of length information is the fundamental difference between the three container
concepts:

| Type          | Length        | Semantics                             | Foundation                                        |
| ------------- | ------------- | ------------------------------------- | ------------------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)  | Core primitive (stack/inline preferred)           |
| `Vec(T)`      | Runtime value | Raw runtime-length buffer, growable   | Core primitive (heap-allocated contiguous buffer) |
| `List(T)`     | Runtime value | Standard library type (growable list) | Library: `{ data: Vec(T), length: Int }`          |
| `Dict(K, V)`  | Runtime value | Key-value map                         | `HeapValue::Dict`                                 |

> `List(T)` is a **standard library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list` and receives the same treatment as user-defined generic records. All
> growable-semantics strategies (when to grow, by how much, whether to share) live in the library;
> the compiler is not involved. `Vec(T)` is the minimal substrate primitive it depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When the need arises,
> follow the Dict pattern.

Key rules:

- **The literal's landing point is determined by context**: a bare `[...]` literal with a `List(T)`
  annotation lands as a growable list; an `Array(T, N)` annotation applied directly to a literal
  lands as a fixed-length array. Landing-point validation: element count == N, element type
  compatible with T; otherwise compile-time E1002. When N is a symbolic constant (const parameter),
  the count check is deferred to the refinement-type phase.
- **Implicit List→Array conversion is forbidden**: fixed-length-ness is guaranteed by the type
  layer—`push` only accepts a `List(A)` receiver.
- **Performance hierarchy**: from bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index-failure contract** (runtime errors are a transitional state; the target state is
  compile-time refinement coverage via value-dependent types, see §8.4):
  - Index out of bounds (including negative indices) → `E6003`
  - Dict missing key → `E6008`
- **`in` membership predicate**: returns `Bool` without erroring; the right operand covers
  List/Array/Dict(key)/Tuple/String/Range. A first-class Hoare predicate, serving as the foundation
  of compile-time provable propositions for refinement types.`

In generic functions, type parameters are likewise declared in the signature, and the compiler
infers them from the arguments automatically:

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 Generic Type Definitions

```yaoxiang
// Basic generic types
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
`residual` / `from_error`). On failure, it returns early from the current function with the result
of `from_error(residual(t))`; on success, the expression's value is `success(t)`. The outer
function's return type must also implement `Try`, and its failure-residual type must match the
receiver's (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try`
implementations from `std.result` / `std.option` (the failure-residual type of `Option` is `Void`);
user-defined sum types plug into `?` by writing `Try(Self, T, E)` inside the type body. The lowering
of `?` does not distinguish built-in from user types—same interface, same path.

A sum type's **variant set enters the checker via its definition or a `use` import**:
`Result`/`Option` require `use std.result` / `use std.option` before use (full-module import and
grouped `use std.{...}` are equally valid). Variant construction (type-qualified form), `match`
variant deconstruction, and exhaustiveness checking all use this registration as the sole criterion.
A native function signature's returned `Result(Float, Error)` casts a same-name `Generic`, which is
by-name identical to `std.result`'s sum type; once `use`d, it can be `match`-deconstructed.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each field
corresponds to a construction parameter, and the field name is the parameter name. Fields with
default values may be omitted at construction; fields without default values are required.
Function-typed fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value → construction parameter required
    extra: T,
}
// Auto-expanded full form (compiler's internal view, users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: call the auto-generated constructor
c  = Container(42, 43)            // construction parameters filled by field order; T auto-unpacked from elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type argument + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // field-name form, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default value → construction parameter may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, match declared parameters position by position, left to right):

1. Each actual argument tries to match a declared parameter position: the `Type` position accepts a
   type argument; a compile-time value-parameter position (e.g., `Int`) accepts a compile-time
   constant.
2. If a compile-time value-parameter position matches successfully (partial match), proceed as a
   type construction: check every parameter position in order, and on error report the first
   mismatched/missing parameter in declaration order.
3. If the actual arguments do not correspond to the declared parameters at all (all values, no
   compile-time value-parameter position to match), proceed as construction parameters: position
   form fills by field order, and type parameters are auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one-level type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two levels: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 mode, data assigned later)

Matrix(42)    // ❌ pos 0: T←42 mismatch (42 is not a type); pos 1: Rows←42 matches;
              //    pos 2: Cols missing → first error reported: T expected Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: A generic type constructor's type parameters are auto-unpacked from
construction-parameter elements (`Container(42, 43)` → T=Int); a generic function's type parameters
are auto-unpacked from argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When
unpacking is impossible, they must be supplied explicitly.

---

## Chapter 5: Type Constraints

### 5.1 Single Constraint

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// Interface type definition (used as a constraint)
Clone: Type = {
    clone: () -> Clone
}

// Using a constraint
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

> **Resolution source for constraints (RFC-011b)**: The resolution of operator constraint names
> (`Add` / `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) = look up the
> interface implementation registry—`T: Add` ≜ an `Add(T, T, T)` instantiation is registered.
> `Equal` also has structural inference (a record whose every field is comparable is
> auto-comparable). Names like `Zero` / `One` / `PartialOrd` have no defined source yet and are
> dangling constraint names.

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
// Higher-order function constraint
call_twice: (T: Type, F: () -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: (A) -> B, G: (B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## Chapter 6: Associated Types

### 6.1 Associated Type Definitions

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (using record type syntax)
Iterator: (T: Type) -> Type = {
    Item: T,                    // associated type
    next: () -> Option(T),
    has_next: () -> Bool
}

// Using the associated type
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
// More complex associated type
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // the associated type is also generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // compile-time constant (candidate)
```

> The criterion is **being referenced at a type position**, not "annotated with a concrete type": in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters (neither appears at
> any type position).

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (e.g., `Int`)
is called a **compile-time value-parameter candidate**; whether it becomes a compile-time value
parameter depends on whether its value is referenced at a type position (value dependency). **No
`const` keyword required** (the implementation used to call these "const generics" internally; this
document uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Shape coarse filter**: the parameter is annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use fine filter**: the candidate name appears at a **type position** (a type-body field type,
   an inner `Fn` parameter type, an `Assert` predicate, an `Array(T, N)` type-construction argument
   position) → real compile-time value parameter; otherwise a **runtime value parameter**.

| Form                                                       | Determination                             | Reason                                               |
| ---------------------------------------------------------- | ----------------------------------------- | ---------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear at value positions                       |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N appears at the type-construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N is the type of inner parameter k                   |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in the type body                 |

**Core design**: Use `(N: Int)` compile-time value parameters + `(k: N)` value parameters to
distinguish compile-time constants from runtime values. Candidates that fall through (candidate by
shape but not by use) degrade to runtime value parameters—both function-level and type-constructor
paths are handled this way.

```yaoxiang
// Compile-time value parameter: N is referenced at a type position (the Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears at the type-construction argument position → compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at a type position (compile time), the result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // the compiler computes factorial(5) = 120 at compile time

// Value dependency: N as the type of inner parameter k
// N is a compile-time value parameter (appears at the type position of (k: N));
// k is a runtime value parameter, its type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 Compile-Time Constant Arrays

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

### 8.1 If Conditional Types

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

### 8.3 Assert Refinement Type and `assert` Assertion

`assert` and `Assert` are two sides of the same refinement primitive—chosen automatically by the
dispatch routing pipeline based on "whether the predicate's free variables are reachable at compile
time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch routing rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                       |
| ------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter the proof pipeline: true → erase to Void, false → compile error (Never is uninhabitable) |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert a runtime Bool check, inject refinement facts into the flow-sensitive assumption set Γ  |

**Flow-sensitive assumption set Γ**:

The compiler maintains the known proposition set at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (kill set).
When branches merge, Γ is the intersection of each branch.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **builtin predicate**, in the same family of core primitives as `Int` and `Never`
(built-in name, not a keyword). It binds a **measure** to a piece of computation, declares that the
computation terminates, and provides a witness of termination.

**Forms**: two arities, the same predicate:

| Form                    | Anchor                            | Use                                                                                                                            |
| ----------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `Terminates(m)`         | The name of the enclosing binding | Default form—self-recursive functions, loops                                                                                   |
| `Terminates(FnType, m)` | Explicit function type            | When the measure's attachment must be stated explicitly (measure defined elsewhere, same measure serves multiple computations) |

```yaoxiang
// Measure: an ordinary function, unit-testable, reusable, not part of runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Two-argument form: measure defined elsewhere, attachment stated explicitly
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// One-argument form: the anchor is the binding name, the measure is an expression in scope
// The loop body is a `{}` block whose value is given by its tail expression (spec §2.9): only with a tail expression is it not `Void`
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

**Semantics**: What `Terminates(m)` refines is the value type of the computation annotated at its
type position. The obligation falls on that computation—on every recursive call site for functions,
on the back edge for loops—and is in both cases "the measure of the next state is strictly less than
the measure of the current state", judged under the path guard at that point.

The call-site obligation is `m(callee_args) < m(caller_args)`; the back-edge obligation is
`m(next iteration) < m(this iteration)`. The two are the same form.

> **Why it covers loops**: A loop is an anonymous construct, usually cannot be referred to. A
> binding name is a name—the `acc` in `acc: Terminates(n - i) = while ...` provides the anchor, and
> the loop thereby becomes referable. This is the reason `Terminates`'s one-argument form applies to
> loops.

**Measure**: The return type is not restricted (not forced to be `Nat`); the "strictly decreasing"
relation is given by a well-order available on that type. Whether the measure is well-founded (e.g.,
when it returns `Int`, whether it is `>= 0`) is an **independent obligation**, judged by the
compile-time proof pipeline just like the decreasing obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. Unrefined ordinary types (e.g., bare `while` loops, functions without
refinement signatures) do not enter verification mode and generate no termination obligations.

**Automatic exploration first**: The compiler first tries to auto-explore a measure (four templates:
linear rank function, predicate-violation counter, bounded increase/decrease, multiplicative
scaling); only when exploration fails must `Terminates` be given explicitly.

**Runtime representation**: A purely compile-time entity; erased with its witness; never enters the
runtime binary.

> See the full design in
> [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../rfc/review/027a-termination-explicit-measure.md) (implementation
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

**Syntax**: The type intersection `A & B` denotes the type that satisfies both A and B.

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using an intersection type
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## Chapter 10: Function Overloading and Specialization

### 10.1 Function Overloading

```yaoxiang
// Basic specialization: function overloading (chosen automatically by the compiler)
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

YaoXiang has only one type property to distinguish: linear vs copyable. It is inferred automatically
by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership
transfer.

```yaoxiang
// A struct with heap-buffer fields does not derive Dup (fields fall to Move) → default Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move, b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property applies to reference/token types and to handle value types with internal
reference counting**. Assignment of a Dup type = shallow copy—copy the handle/token, the underlying
data is shared. Multiple holders point to the same data.

| Type            | Property | Description                                                                              |
| --------------- | -------- | ---------------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token; copying the token = multiple views on the same data               |
| `ref T`         | Dup      | Rc/Arc copy = refcount+1, share heap data                                                |
| String, Bytes   | Dup      | Internal reference counting; assignment copies the handle, sharing the underlying buffer |
| `&mut T`        | Linear   | Zero-sized write token, exclusive, non-copyable                                          |
| struct          | derived  | All fields copyable (primitive value types ∪ Dup) → Dup, otherwise Move (#398)           |
| tuple           | derived  | Element-wise determination, same rules as struct (#398)                                  |
| All other types | Move     | Default ownership transfer                                                               |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: assignments
automatically copy the value, the two values are completely independent. This is the compiler's
native behavior, not a Dup type property.

**Derivation rules** (finalized in #398) — "automatically derive when all fields are Dup" cannot be
executed literally: primitive fields (e.g., Int) are not themselves Dup, and `{ x: Int, y: Int }`
would be misclassified as Move. Executable form:

1. **Copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), composite types already at Dup);
2. **struct**: every field is in the copyable field set → derive Dup; **any** field falls to Linear
   (`&mut T`) or Move (nested Move struct / containers like Vec, Dict / resources) → the whole
   struct stays Move (no intermediate "partially copyable" state is introduced);
3. **tuple**: same rule as struct, element-wise determination; the empty tuple (unit) is `Void`;
4. **Derivation is recursive**: when a field has a named type (e.g., `target: Point`), expand its
   definition and judge; `A = { b: B }` follows B's derived result; circular aliases fall back
   conservatively to Move by depth limit;
5. **Not in the derivation scope** (still Move, separate concern): containers (Vec / Dict / Set /
   Option / Result / Array) and enum.

```yaoxiang
// &T: Dup, free aliasing
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // available
print(view2.x)   // available

// &mut T: Linear, non-copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot copy
```

### 11.3 Clone (Explicit Deep Copy) and Its Relation to Dup

**Clone** is the explicit deep-copy interface. Any type can implement Clone, providing a `.clone()`
method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // deep copy, p is still available
p2 = p.clone()        // can be cloned multiple times
```

**Differences between Dup and Clone**:

|                         | Dup                                                                | Clone                                        |
| ----------------------- | ------------------------------------------------------------------ | -------------------------------------------- |
| **Semantics**           | Shallow copy: copy the handle/token, the underlying data is shared | Deep copy: create a full independent replica |
| **Call method**         | Implicit (auto on assignment/parameter passing)                    | Explicit (`.clone()`)                        |
| **Modification impact** | Mutually affected (shared underlying data)                         | Mutually unaffected (independent replicas)   |
| **Applicable types**    | `&T` token, `ref T`                                                | Any type that implements the Clone interface |
| **Cost**                | Zero overhead (token is a zero-sized type)                         | Depends on the type                          |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, the underlying data is shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // available
print(view2.x)      // available, viewing the same data

// Primitive value type: compiler auto-copies the value (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // available

// Clone: explicit deep copy, create an independent replica
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still available

// Non-Dup type (fields fall to Move, not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field, does not derive Dup (see §11.2 derivation rules)
```

**Design intent**:

- Dup is for token/reference types, solving the "multiple views on the same data" problem.
- Clone is for scenarios that need an independent replica; the explicit call makes the cost visible.
- The copying of primitive value types (Int/Float/Bool/Char) is the compiler's built-in behavior,
  not a Dup property.
- User-defined types default to Move (zero-copy, high performance); when all fields are copyable,
  Dup is auto-derived (§11.2 derivation rules).

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references" but
"type-level proofs of access permission".

```
&T      →  zero-sized, freezes the source data (forbids a WriteToken being acquired meanwhile),
          multiple read-only copies are safe under the freeze guarantee → Dup (copyable)
&mut T  →  zero-sized, exclusive read/write (forbids any other token),
          copying is meaningless under exclusive access → Linear (non-Dup)
```

**Key properties**:

- A token is an **ordinary type**, following the same scoping rules as any other type.
- No lifetime annotation `'a` is needed.
- No dedicated borrow checker is required—type properties (Dup/Linear) naturally derive the
  permissions.
- It disappears completely after compilation, with zero runtime overhead.

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

// Caller side: the compiler automatically chooses between borrow and Move
p = Point(1.0, 2.0)
p.print()                       // the compiler auto-creates an &Point token
p.shift(1.0, 1.0)               // the compiler auto-creates an &mut Point token
p.print()                       // OK, the previous token was released when shift returned

// Multiple &T tokens coexist — Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

A token is an ordinary type, so it supports all ordinary type operations:

**Returning a token**—the token propagates with the return value:

```yaoxiang
// ✅ The child token and the parent token are returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // tokens returned to the caller
print(px_ref)                    // OK, the token is still in scope
```

**Storing in a struct**—a struct can carry token fields:

```yaoxiang
// ✅ The struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field — holds a read-only view of target
}
```

**Closures do not capture; the context is fixed at the creation point**—a closure only takes its own
parameters; when it needs outer data, the value is fixed into the closure via currying at the
creation point:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter, and gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition site may already be
> dead, so it must not implicitly capture outer variables; but the scope at the call site (creation
> point) is necessarily live, so fixing the context as a value into the closure at that point is
> safe.

### 12.4 Automatic Borrow Selection

The caller-side compiler automatically chooses by the following priority:

```
1. If the actual argument is still used afterwards → prefer to create a token (&T or &mut T, per the method signature)
2. If the actual argument is not used afterwards → Move
3. Preference order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates an &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates an &mut Point token
p2 = p             // not used afterwards → Move
```

**Method receiver follows signature semantics** (same convention as RFC-011a receiver spelling):
receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value → Move (consume
the receiver). The borrow token created at the call site is released when the call ends (transient,
§12.5 interval semantics); an interface's borrow receiver is explicitly declared by the interface
author as `&Self`, and the impl signature after substituting `Self ↦ impl type` must match the
interface exactly (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler auto-generates borrow propositions (`borrow_conflict` /
`use_after_move` / `use_after_drop` / `mut_violation`) and feeds them into the proof pipeline for
verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a § reverse-BFS
liveness analysis):

```yaoxiang
// ❌ &mut and a derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ The token is auto-released after its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // uses &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken still available
}

// ❌ The same actual argument cannot simultaneously create an &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives &mut and & tokens simultaneously
```

### 12.6 Compiler Internals: Brand Mechanism

Users never touch brands. The compiler internally assigns each token a compile-time unique
identifier:

```
What users see          Compiler's internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: a token can only be obtained from the owner's capsule; it cannot be constructed
  out of thin air.
- **Relation tracking**: a field-access-derived `&Float` carries a derived brand (`#N.field_x`),
  which the compiler can trace back to the parent token.
- **Conflict detection**: a same-source WriteToken and a derived ReadToken cannot be live
  simultaneously.

Brands disappear completely after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source → Dup safe)
               | &mut T      // WriteToken (exclusive read/write → Linear)
```

### 12.8 Borrow Token vs `ref`

|                 | `&T` / `&mut T`                                               | `ref`                                   |
| --------------- | ------------------------------------------------------------- | --------------------------------------- |
| What it does    | Take a peek / mutate in place                                 | Shared ownership                        |
| Range           | Follows the scope of the token value                          | Cross-scope                             |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (chosen by the compiler)      |
| Escape          | Possible (token propagates with return value / struct)        | Designed to escape                      |
| Cross-task      | No (tokens are not implemented for cross-task passing)        | Yes (compiler auto-selects Arc)         |
| Cycle detection | Not involved                                                  | Silently within a task, cross-task lint |

> Note (undefined): how to read content after `ref` is created (dereference / method / automatic) is
> not yet defined in the spec; the current implementation's `*a` reports E1052. To be added to this
> section once defined.

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

// Type that implements the interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // implement Serializable interface
}

// === Function type ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (builtin predicate, see §8.4) ===

// One-argument: the anchor is the binding name (self-recursive functions, loops)
// The body tail must provide a value — the value of `while` = the value of the loop body block = the tail expression (spec §2.9)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}

// Two-argument: explicitly state the measure's attachment (measure defined elsewhere)
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

// Type constraint
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated type
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Compile-time generic: N is referenced at the type position (k: N) → compile-time value parameter
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
Int, Float,     // assignment auto-copies the value; the two values are completely independent
Bool, Char      // not Dup; this is the compiler's built-in handling of primitives

// === Dup (shallow copy: copy the handle, share the underlying data) ===
&T              // zero-sized read token; copying the token = multiple views on the same data
ref T           // Rc/Arc copy = refcount+1, share heap data
String, Bytes   // internal reference counting; assignment copies the handle, sharing the underlying buffer
struct / tuple  // derived: all fields ∈ (primitive value types ∪ Dup) → Dup (#398)

// === Not derived (stays Move) ===
Vec/Dict/Set    // containers and enums are not in the derivation scope (#398)

// === Linear ===
&mut T          // zero-sized write token, Linear (exclusive, non-copyable)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent replica; modifications do not affect the original
```

### A.4 Borrow Token Cheat Sheet

```
// === Borrow tokens ===
&T              // zero-sized compile-time read token, freezes source → Dup (copyable)
&mut T          // zero-sized compile-time write token, exclusive read/write → Linear (non-copyable)

// Caller-side auto-selection
// 1. Actual argument still used afterwards → create a token
// 2. Actual argument not used afterwards → Move
// 3. Preference: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in a struct, captured by a closure
// ❌ Cannot cross tasks (tokens are not implemented for cross-task passing)
```
