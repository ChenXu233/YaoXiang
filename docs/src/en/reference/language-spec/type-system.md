# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and trait.

---

## Chapter Zero: Theoretical Foundations

### 0.1 Curry-Howard Isomorphism

The Curry-Howard isomorphism (Curry-Howard correspondence) is the theoretical foundation of the
YaoXiang type system. It reveals the deep correspondence between the type system of a programming
language and mathematical logic:

| Logic                                          | Programming Language                                 |
| ---------------------------------------------- | ---------------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                          |
| Proof \(p: P\)                                 | Program `x: T = ...`                                 |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                             |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                        |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                          |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                          |
| True \(\top\)                                  | `Void` (Unit, with a default value)                  |
| False \(\bot\)                                 | `Never` (zero constructors, no inhabitant)           |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                                   |

> **Note**: Type-level `match` is classification by cases (case analysis), not mathematical
> induction. Induction requires type-level recursive functions and compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to a correct constructive proof**. YaoXiang type
  families, such as type-level case analysis over `Add` on `Nat` plus recursive calls, are
  fundamentally type-level encodings of mathematical induction—provided that the compiler can
  perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, this is equivalent
  to a logical proposition having been constructively proved.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard isomorphism in YaoXiang are:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox
   caused by `Type: Type` (Girard's paradox).
2. **Type families** (RFC-011): type-level case analysis and recursive calls for the natural-number
   type `Nat(Zero/Succ)` correspond to the Peano axioms—provided that the compiler performs
   termination checking.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to the
   bounded quantification “for every integer N, there exists one type.”

---

## Chapter One: Type Classification

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

> **Design note**: Although RFC-010 proposed a unified “everything is an assignment” model
> (`name: type = value`), types and values must still be distinguished at the syntax level. In the
> compiler implementation, `Type` and `Expr` are two separate AST enums (`ast.rs:406` and
> `ast.rs:25`). As a BNF placeholder, `TypeExpr` corresponds to the `Type` enum in the
> implementation and indicates “the type expected at this position.”

---

## Chapter Two: Primitive Types

### 2.1 Primitive Types

**There are 15 built-in type names; this section is the authoritative table**: `Never` / `Void` /
`()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` /
`Char` / `String` / `Bytes`. The single source of truth in the implementation is
`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`), and this table matches it **name
for name**—the T2 gate in `tools/type-tables --report` compares this table, `CONST_PARAM_TYPES`, and
the LSP type-name list.

| Type      | Logical Correspondence | Description                                                                                 | Default Size |
| --------- | ---------------------- | ------------------------------------------------------------------------------------------- | ------------ |
| `Type`    | —                      | Meta type (not a value-type name; see below)                                                | 0 bytes      |
| `Never`   | ⊥ (false/empty type)   | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for every T. | 0 bytes      |
| `Void`    | ⊤ (true/Unit)          | Has a default void value; zero-field product type. `x: Void = <default>` is legal.          | 0 bytes      |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                             | 1 byte       |
| `Int`     | —                      | Signed integer (= `Int64`)                                                                  | 8 bytes      |
| `Int8`    | —                      | 8-bit signed integer                                                                        | 1 byte       |
| `Int16`   | —                      | 16-bit signed integer                                                                       | 2 bytes      |
| `Int32`   | —                      | 32-bit signed integer                                                                       | 4 bytes      |
| `Int64`   | —                      | 64-bit signed integer                                                                       | 8 bytes      |
| `Float`   | —                      | Floating-point number (= `Float64`)                                                         | 8 bytes      |
| `Float32` | —                      | 32-bit floating-point number                                                                | 4 bytes      |
| `Float64` | —                      | 64-bit floating-point number                                                                | 8 bytes      |
| `String`  | —                      | UTF-8 string                                                                                | Variable     |
| `Char`    | —                      | Unicode character                                                                           | 4 bytes      |
| `Bytes`   | —                      | Raw bytes                                                                                   | Variable     |

`()` is the **type notation** for `Void` (zero-element tuple = unit type) and is synonymous with
`Void`: standard-library signature strings write it as `() -> Void`, and the parser table places
`()` and `Void` in the same arm.

**There is only one valid spelling**. `Int` is not `int`; `Int64` is not `int64`/`i64`; `String` is
not `string`/`str`; `Bool` is not `bool`; `Void` is not `void`; `Never` is not `never`; `Float` is
not `float`/`f64`; `Char` is not `char`; and `Bytes` is not `bytes`. The implementation once
coexisted with 22 such lowercase or abbreviated spellings, all of which were removed in **WBS 6.1.2
(finalized on 2026-10-27)**: writing a noncanonical spelling in a type position always produces an
unknown-type error and is never silently resolved as a user-defined type.

> **`DateTime` is not a built-in name**. It is a **semantic alias in the std layer**: the `now` /
> `parse_time` signature strings in `src/std/time.rs` and `src/std/interfaces/time.yx` still spell
> it `DateTime`, while at runtime it is exactly `Int` (fixed together with #338). The parser table
> retains it only as a **temporary compatibility alias**, and this table does not include it;
> migrating it out of the built-in-name table so that it is carried by std belongs to **P10 S11**
> (issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)).

> **There are no unsigned integer types**. `Uint` (and `Int128`) are not usable type
> names—`MonoType` has no unsigned variant. One residual entry each remained in the LSP type-name
> list and the `sizeof` name table, but both were removed with WBS 6.1.2. When unsigned semantics
> are needed, use `Int` with a lower-bound convention.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system, corresponding respectively to
false (⊥) and true (⊤).

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. There is no
   right-hand side to write for `x: Never = ...`.
2. **Explosion principle**: `Never <: T` holds for every type `T`. `assert(false)` returns `Never`,
   after which subsequent code can pass type checking, although execution never reaches it.
3. **Divergence marker**: `f: (...) -> Never` means that `f` is guaranteed not to return. The
   compiler uses this for dead-code analysis and branch confluence.

`Never` is a built-in type name, registered through the same path as `Int`/`Bool`; it is not a
keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant, the default void value. `Void` is the identity
of the zero-field product type. `x: Void = <default>` is legal. A block's value is provided by its
**tail expression** (an empty block `{}` is `Void`). See
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for details.

---

## Chapter Three: Composite Types

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

- Record types are defined using curly braces `{}`.
- A field name is followed directly by a colon and a type.
- An interface name is written in the type body to indicate that the interface is implemented.

> **Namespace ownership**: A `Type.name` prefix, such as `Point.draw`, indicates that the function
> belongs to the namespace of `Point`. It does not trigger any implicit binding. To make `.`-call
> syntax such as `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See
> RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, which may be optionally provided during construction:

```yaoxiang
// A field with a default value - optional during construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// A field without a default value - required during construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // Correct
Point2()          // Error
```

**Rules**:

- `field: Type = expression` -> has a default value and is optional during construction.
- `field: Type` -> has no default value and is required during construction.

#### 3.1.2 Built-in Bindings

Methods may be bound directly within a type definition:

```yaoxiang
// Method 1: Bind by referencing an external function
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // Bound to position 0
}
// Call: p1.distance(p2) -> distance(p1, p2)

// Method 2: Anonymous function + positional binding
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

**Interface implementation**: A type implements an interface by listing the interface name at the
end of its definition.

```yaoxiang
// A type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implements the Drawable interface
    Serializable     // Implements the Serializable interface
}
```

**Direct interface assignment**: A concrete type may be assigned directly to an interface-type
variable (structural subtyping).

```yaoxiang
// Direct assignment (the concrete type can be determined at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Looks up the method through the vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inference Result                | Invocation Method           |
| ------------------------------------ | ------------------------------- | --------------------------- |
| Direct assignment of a concrete type | Concrete type can be determined | Direct call (zero overhead) |
| Function return value                | Unknown                         | vtable                      |
| Heterogeneous collection             | Multiple types                  | vtable                      |

**Coherence and the orphan rule (not applicable; closure note)**: YaoXiang interfaces are structural
types (an interface is a record whose fields are all functions), not nominal traits. Therefore,
there is no cross-crate or cross-module ownership question about “who may implement what.”
Rust-style orphan rules and coherence checks have no applicable subject (see RFC-011 §2.1 for the
ruling). The corresponding guarantee in the structural world is **duplicate-implementation
rejection**: defining the same method signature more than once on a type produces a compile-time
error (RFC-011a §3; overriding is forbidden, while overloading is legal).

### 3.4 Tuple Types

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 Function Types

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= Param (',' Param)*
Param       ::= Identifier ':' TypeExpr    // Named parameter
              | TypeExpr                   // Unnamed parameter (bare type)
```

**The two parameter forms have different ownership of names** (the type side of RFC-007's shorthand
rules):

- **Named** (`a: Int`): The name enters the contract and may be referenced by named arguments
  (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)). When a signature already declares
  parameter names and types, the lambda head may be omitted (`add: (a: Int, b: Int) -> Int = a + b`,
  RFC-007 §Shorthand Rules).
- **Unnamed** (a bare type such as `(Int, Int) -> Int`): Type constraints apply **positionally**,
  while names belong to the implementation. The lambda head must provide its own parameter names
  (`mk: (Int, Int) -> Int = (x, y) => x + y`, RFC-007: the lambda head may be omitted if and only if
  the signature already declares parameter names). Such parameters cannot be referenced by named
  arguments and can only be called positionally. A bare identifier is resolved in the type
  namespace; if no declared type can be resolved, an error is reported—there is no bare identifier
  in a parameter position that is “neither a type nor a type parameter.”

**Annotated binding semantics**: An annotation is a declaration, not a comment. A binding statement
checks the implementation against the annotation: parameter types drive the lambda head
positionally, and the function body's exits (the tail expression and `return`) are uniformly checked
against the annotated return type. The type registered for callers is the annotated form.
Unannotated bindings use HM inference (RFC-007). `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are rejected under the same rule—binding semantics do not diverge
according to how the value is written (lambda / block / expression / named-function reference).

**The implementation head's parameters are positional binders**
(`f: (a: Int) -> Int = (b) => b + 1`):

- **Both names and types follow the declaration**: The i-th position of the implementation head
  corresponds to the **first N positions of the declaration's value-level signature** (N = the
  number of implementation-head parameters; the value-level signature is the parameter sequence
  after removing compile-time positions—`T: Type`, type-level function positions, and const
  positions). The implementation head declares only “what the i-th parameter is called inside the
  body”; changing its name does not affect the call site. The available name in `f(a = 7)` comes
  from the declaration and is independent of the implementation head's name.
- **Annotations in the implementation head serve only as a cross-check**: If the type at the same
  position differs from the declaration, **E1011** is reported. The declaration is the authoritative
  source, and the author's annotation must not be silently overridden.
- **More implementation-head parameters than the declaration's runtime parameter positions** produce
  **E1112**. There are no values to bind to the extra positions (type positions such as `T: Type`,
  type-level function positions, and const positions do not occupy runtime parameter positions).
- **The reverse case—omitting parameters in the implementation—does not produce an error** (before
  D54, this was the “pass” cell in the t1 behavior matrix and must not be reclassified). At the type
  level, arity is still checked against the **declaration** (`f(1)` is legal, while `f(1, 2)`
  reports E1010). Missing positions are filled positionally by the declaration into the
  **type-checking scope**; however, the implementation head has no corresponding runtime slots, so
  arguments at those positions are discarded on the IR side and do not participate in computation
  (`f(1)` in `f: (a: Int) -> Int = () => 1` is unrelated to the implementation). This has been
  **recorded as a gap**—making runtime receipt follow the declared arity would be a class-D
  behavioral change and requires a separate ruling.
- **Uniform exit checking**: Whenever the tail expression or `return` (the arrow sugar `=> x` and
  explicit `return x` are two AST shapes) does not match the declared return position, **E1012** is
  reported, with the location pointing to the **annotation**.

> **Implementation status**: The binding-period cross-check for `Fn` annotations has been
> implemented (WBS 6.8.2, 2026-11-02): positional parameter alignment (the parser and typechecker
> use **the same alignment rule**: the first N positions of the value-level signature),
> implementation-head annotation checking (E1011; the author's annotation is retained and a mismatch
> is reported), uniform exit checking (E1012), and declaration-shaped registration (available
> named-argument names).

---

## Chapter Four: Generics

### 4.1 Generic Parameter Syntax

Generic parameters are part of a function type and use the same `()` syntax as ordinary parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of a type constructor, while
`-> Type` denotes the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives. They receive the same
treatment as user-defined generics and are handled through a unified generic-instantiation path.
Ownership of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                               | Foundation                               |
| ------------- | ------------- | --------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type-level    | Fixed-length array (const generic N)    | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Growable raw buffer with runtime length | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard-library type (growable list)   | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                       | `HeapValue::Dict`                        |

> `List(T)` is a **standard-library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list` and receives the same treatment as a user-defined generic record. All
> growable semantics—when to expand, by how much, and whether sharing is possible—are implemented in
> the library and do not involve the compiler. `Vec(T)` is the minimal foundational primitive on
> which it depends.
>
> Set(T) has been removed: it has no literal syntax, no runtime representation, and no `std.set`.
> When the requirement arises, it should be completed following the Dict pattern.

Key rules:

- **Literal placement is determined by context**: A bare `[...]` literal annotated as `List(T)`
  becomes a growable list; an `Array(T, N)` annotation applied directly to a literal produces a
  fixed-length array. Placement validation requires the element count to equal N and the element
  types to be compatible with T; otherwise, compile-time E1002 is reported. When N is a symbolic
  constant (a const parameter), count validation is deferred to the refinement-type stage.
- **Implicit List→Array conversion is forbidden**: Fixed-lengthness is guaranteed by the type
  layer—push accepts only a `List(A)` receiver.
- **Performance hierarchy**: Performance decreases and flexibility increases from bottom to top:
  `Array` > `Vec` > `List`.
- **Index-failure contract** (runtime errors are a transitional state; the target state is
  compile-time refinement coverage through value-dependent types; see §8.4):
  - Index out of bounds, including a negative index → `E6003`
  - Missing Dict key → `E6008`
- **String indexing semantics** (#385): `s[i]` indexes characters by **Unicode scalar value (code
  point)** and produces a single-character `String`. This uses the same unit as `len` / `index_of` /
  `substring` / `chars` / `char_code` in `std.string`, so combined use does not introduce a
  mismatch. An out-of-bounds index, including a negative index, follows the index-failure contract
  above: `E6003`.
- **Membership `in` predicate**: Returns `Bool` without raising an error; the right-hand operand may
  be List/Array/Dict (keys)/Tuple/String/Range. It is a first-class Horn predicate and forms the
  basis of propositions that can be proved at compile time in refinement types.`

In generic functions, type parameters are likewise declared in the signature, and the compiler
infers them automatically from the arguments:

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
    push: (self: List(T), item: T) -> Void,   // self is only a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation. The
receiver's type must implement the `Try` interface, whose four members are `is_failure` / `success`
/ `residual` / `from_error`. On failure, the current function returns early with the result of
`from_error(residual(t))`; on success, the expression's value is `success(t)`. The enclosing
function's return type must also implement `Try`, and its failure-residual type must match the
receiver's (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try`
implementations through `std.result` / `std.option` (the failure-residual type of `Option` is
`Void`). A user-defined type may opt in to `?` by writing `Try(Self, T, E)` in its type body. The
lowering of `?` does not distinguish built-in and user-defined types—the same interface and the same
path are used.

The **variant set of a sum type enters the checker through its definition or a `use` import**:
`Result`/`Option` must be imported with `use std.result` / `use std.option` before use (whole-module
and grouped `use std.{...}` imports have equal authority). Variant construction in its
type-qualified form, match-variant destructuring, and exhaustiveness checking all use this
registration as their sole criterion. A native function signature returning `Result(Float, Error)`
mints a `Generic` with the same name, which has the same identity as the sum type in `std.result` by
name and may be match-destructured after `use`.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: Each field
corresponds to a constructor argument, and the field name becomes the argument name. Fields with
default values may be omitted during construction, while fields without default values are required.
Function-type fields (methods) do not generate constructor arguments.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value -> constructor argument is required
    extra: T,
}
// Fully expanded form generated automatically (compiler's internal view; users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invoke the automatically generated constructor
c  = Container(42, 43)            // Constructor arguments are filled in field order; T is automatically unpacked from the element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type argument + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // Field-name form, arbitrary order
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned later)

// Field default value -> constructor argument may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses; match declared parameters positionally from left to right):

1. Arguments are matched positionally against the declared type parameters: a `Type` position
   accepts a type argument, and a compile-time value-parameter position (such as `Int`) accepts a
   compile-time constant.
2. If a compile-time value-parameter position matches (a partial match), construction is handled as
   a type construction: all parameter positions are checked positionally, and the **first
   mismatching or missing parameter in declaration order** is reported.
3. If the arguments do not correspond to the declared parameters at all (all are values and no
   compile-time value position can match), construction is handled as a constructor call: positional
   arguments fill fields in order, and type parameters are automatically unpacked from the element
   types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one layer of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two layers: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern; data assigned later)

Matrix(42)    // ❌ Position 0: T←42 does not match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols is missing → report the first error: T expects Type, found 42
Container(42) // ❌ Missing constructor argument extra
Container(42, 43, 44)  // ❌ Too many constructor arguments
```

**Type inference**: The type parameters of a generic type constructor are automatically unpacked
from the constructor arguments' element types (`Container(42, 43)` → T=Int); the type parameters of
a generic function are automatically unpacked from the argument types (`map(numbers, f)` → T=Int,
R=String; see §4.1). If they cannot be unpacked, they must be supplied explicitly.

---

## Chapter Five: Type Constraints

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

> **Constraint resolution sources (RFC-011b)**: Operator constraint names (`Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) are resolved by looking up the
> interface-implementation registry—`T: Add` ≜ a registered `Add(T, T, T)` instantiation. `Equal`
> additionally has structural inference (records whose fields are all comparable are automatically
> comparable). Names such as `Zero` / `One` / `PartialOrd` do not yet have a definition source and
> are dangling constraint names.

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

### 5.3 Function-Type Constraints

```yaoxiang
// Higher-order function constraint
call_twice: (T: Type, F: () -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: (A) -> B, G: (B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## Chapter Six: Associated Types

### 6.1 Associated Type Definitions

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (uses record-type syntax)
Iterator: (T: Type) -> Type = {
    Item: T,                    // Associated type
    next: () -> Option(T),
    has_next: () -> Bool
}

// Using an associated type
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

### 6.2 Generic Associated Types (GATs)

```yaoxiang
// A more complex associated type
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // An associated type may also be generic
    iter: () -> IteratorType
}
```

---

## Chapter Seven: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant (candidate)
```

> The criterion is **being referenced in a type position**, not having a concrete type annotation:
> in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters because neither
> appears in any type position.

**Terminology**: Whether a generic parameter whose annotation is a concrete type other than `Type`
(such as `Int`) enters compile-time evaluation depends on whether its value is referenced in a type
position, that is, whether it is value-dependent. **No `const` keyword is required** (the
implementation once referred to these as “const generics,” but the documentation consistently uses
“compile-time value parameter”).

> **Design basis**:
> [RFC-011 §4.1 compile-time value parameters](../../rfc/accepted/011-generic-type-system.md).

Determination is driven by **strength declarations that are locally readable within a signature**:
when the author writes a reference to a value parameter in a type position (`(k: N)`, `Array(T, N)`,
`Assert(N > 0)`), that reference is a strength declaration; the **quantity** is its mechanical
reading. Each binding infers a quantity `q ∈ {0, 1, ω}`: q=0 is computed at compile time, erased at
runtime, and occupies no slot; q=1 is linear (Move); q=ω is free (Dup)—see Chapter Eleven for 1/ω.
This is not implicit: the appearance of a value name in a type position is a declaration written
explicitly by the author, just like writing `b: Positive(b)` (§8.3).

**“Type position” refers to every `Type` subtree in a declaration**—the annotated types of
parameters, the function's return type, field types in a type body, the types of inner `Fn`s,
`Assert` predicates, and argument positions in type constructions such as `Array(T, N)`.
Determination traverses **all** `Type` subtrees without case distinctions by position; missing even
one subtree, such as the return type, would cause a compile-time value parameter to be misclassified
as a runtime value parameter.

**“Referenced” is determined through namespace resolution**, not by literal name comparison:

1. The identifier is first resolved in the **type namespace**—declared type names and type
   parameters (`T: Type`).
2. If it resolves to a declared type, it is unrelated to the parameter. A parameter name and type
   name having the same spelling **does not constitute shadowing**: the value and type namespaces do
   not consult the same table (see the scope and shadowing rules in [modules](modules.md)). The two
   `Int`s in `(Int: Int) -> Int` belong to different namespaces.
3. If it cannot be resolved, but a parameter with the same name exists in the declaration, the
   strength declaration is matched and that parameter receives q=0.
4. If neither applies, an error is reported: there is no bare identifier in a type position that is
   “neither a type nor a type parameter.”

Self-reference is handled with a monotone fixed point (sets only grow, with at most |candidates|
rounds).

**Slot allocation uses q as its sole input**; the call-site argument sequence and the
implementation-head binding sequence are two **derived views** of the same function:

- **Call-site argument view** = parameters − type parameters, in declaration order. A q=0 parameter
  still occupies an argument position: in `f(42)(5)`, 42 supplies N and 5 supplies n; the argument
  positions record how many values the author supplied in declaration order.
- **Implementation-head binding view** = parameters − type parameters − q=0 parameters, in
  declaration order. The implementation head binds only names for which runtime values exist:
  `= (n) => n` is legal, while `= (N, n) => n` reports E1112.

Both E1011 and E1112 are counted according to the **implementation-head binding view**.

| Formulation                                                | Determination                       | Reason                                                        |
| ---------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | They appear only in value positions; no strength declaration  |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N appears in a type-construction argument position            |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N acts as the type of the inner parameter k                   |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N is an ordinary parameter          | N is not referenced in the type body; no strength declaration |

**Core design**: Use a `(N: Int)` compile-time value parameter and a `(k: N)` value parameter to
distinguish compile-time constants from runtime values. **The call site's obligation is checked
according to declaration strength**: whether `f(y)` is legal depends on whether the argument y
carries compile-time evidence (a literal, a value environment, singleton pinning, a Γ equation),
rather than on a global analysis of f's declaration. When there is no evidence, the error is
reported at the call site and points to the q=0 slot visible in f's type. A candidate that falls
through (its shape is a candidate, but it is not referenced in a type position) is an ordinary
parameter without a strength declaration and is handled normally as q=1/ω, with no separate
diagnostic code.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (the Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in a type-construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated in a type position (at compile time), and the result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // The compiler computes factorial(5) = 120 at compile time

// Value dependence: N acts as the type of the inner parameter k
// N is a compile-time value parameter (it appears in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (a singleton type).
// Singleton pinning: k's value is fixed by its singleton type and is known at compile time,
// so k - 1 carries compile-time evidence and may fill the N position of the recursive call.
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)(k - 1)
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

## Chapter Eight: Conditional Types

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
// IsTrue bridge and Assert refinement types (see §8.3 for details)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, execution continues
    false => Never,    // ⊥, divergence/compile error
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

### 8.3 Assert Refinement Types and the assert Assertion

`assert` and `Assert` are two sides of the same refinement primitive. The dispatch pipeline
automatically selects between them based on whether the predicate's free variables are available at
compile time.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                            |
| ----------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enters the proof pipeline: true → erased to Void; false → compile error (Never cannot be inhabited) |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Inserts a runtime Bool check and injects refinement facts into the flow-sensitive assumption set Γ  |

Dispatch is an instance of **checking according to declaration strength** (§7.1) at the predicate
position. The same principle covers the entire strength ladder: classic types without a strength
declaration do not enter the proof pipeline and receive no quantity inference; predicate annotations
(this section), q=0 slots (§7.1), dependent record fields, and relation predicates each undertake
the checks appropriate to their declaration form. Cost follows declaration strength and is not
determined by global analysis.

**Flow-sensitive assumption set Γ**:

The compiler maintains the set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: the old assumption is invalidated
```

After a `mut` variable is assigned, every assumption involving that variable is removed (the kill
set). When branches are merged, Γ is the intersection of the branches' sets.

### 8.4 Terminates: Termination-Measure Predicate

`Terminates` is a **built-in predicate** and, like `Int` and `Never`, belongs to the core primitives
(a built-in name, not a keyword). It binds a **measure** to a computation, declares that the
computation terminates, and provides a witness of termination.

**Form**: Two arities over the same predicate:

| Form                    | Anchor                            | Use                                                                                                                               |
| ----------------------- | --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The name of the enclosing binding | Default form—self-recursive functions and loops                                                                                   |
| `Terminates(FnType, m)` | Explicit function type            | Used when the measure's ownership must be specified explicitly (the measure is defined elsewhere or serves multiple computations) |

```yaoxiang
// Measure: an ordinary function that can be unit-tested and reused; it does not participate at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere, with its ownership specified explicitly
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, and the measure is an expression in scope
// A loop body is a `{}` block whose value is provided by its tail expression (spec §2.9): it is not `Void` unless the body ends in an expression
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines exactly the value type of the computation annotated at its
type position. The obligation falls on that computation—for a function, at every recursive call
site; for a loop, at the back edge—and requires that “the measure of the next state is strictly less
than the measure of the current state,” evaluated under the path guard at that point.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back edge is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: A loop is an anonymous construct and usually cannot be referred to. Its
> binding name is its name—the `acc` in `acc: Terminates(n - i) = while ...` provides an anchor,
> making the loop referable. This is why the unary form of `Terminates` applies to loops.

**Measure**: The return type is not restricted (natural numbers are not required); strict decrease
is determined by a well-founded order available on that type. Whether the measure is
well-founded—for example, whether an `Int` result is `>= 0`—is an **independent obligation**,
determined by the compiler's proof pipeline just like the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. Ordinary types that have not been refined, such as a bare `while` loop or
a function without a refinement signature, do not enter verification mode and do not generate
termination obligations.

**Automatic exploration first**: The compiler first attempts to explore measures automatically using
four templates: linear ranking functions, predicate-violation counts, bounded increment/decrement,
and multiplicative scaling. An explicit `Terminates` is needed only when automatic exploration
fails.

**Runtime representation**: A purely compile-time entity, erased with the witness and not included
in the runtime binary.

> For the complete design, see
> [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../rfc/accepted/027a-termination-explicit-measure.md) (implementation mechanism).

---

## Chapter Nine: Type Unions and Intersections

### 9.1 Type Unions

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 Type Intersections

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**Syntax**: The type intersection `A & B` denotes a type that satisfies both A and B.

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

## Chapter Ten: Function Overloading and Specialization

### 10.1 Function Overloading

```yaoxiang
// Basic specialization: use function overloading (the compiler selects automatically)
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// General implementation
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
// Platform type enumeration (defined by the standard library)
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

## Chapter Eleven: Type Attributes

YaoXiang has only one type attribute that needs to be distinguished: linear versus duplicable. It is
inferred automatically by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, argument passing, and returning equal an
ownership transfer.

```yaoxiang
// A struct containing a heap-buffer field does not derive Dup (the field falls under Move) -> defaults to Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move; b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle and Share the Data)

**The Dup attribute is intended for reference/token types and internally reference-counted handle
value types**. Assigning a Dup type performs a shallow copy—the handle/token is copied while the
underlying data is shared. Multiple holders point to the same data.

| Type            | Attribute | Description                                                                                |
| --------------- | --------- | ------------------------------------------------------------------------------------------ |
| `&T`            | Dup       | Zero-sized read token; copying the token gives multiple views onto the same data           |
| `ref T`         | Dup       | Copying an Rc/Arc increments the reference count and shares the heap data                  |
| String, Bytes   | Dup       | Internal reference counting; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear    | Zero-sized write token; exclusive access and cannot be copied                              |
| struct          | Derived   | Dup if all fields are duplicable (primitive value types ∪ Dup); otherwise Move (#398)      |
| tuple           | Derived   | Determined element by element, using the same rule as struct (#398)                        |
| All other types | Move      | Default ownership transfer                                                                 |

**Primitive value types** (Int, Float, Bool, Char) receive special handling from the compiler:
assignment automatically copies the value, and the two values are completely independent. This is
native compiler behavior and is not part of the Dup type attribute.

**Derivation rules** (finalized in #398)—the rule “automatically derive Dup when all fields are Dup”
cannot be applied literally: primitive fields such as Int are not themselves Dup, so
`{ x: Int, y: Int }` would be misclassified as Move. The executable form is:

1. **Duplicable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), and composite types already known to be Dup);
2. **struct**: If all fields belong to the duplicable field set, Dup is derived. If **any** field
   falls under Linear (`&mut T`) or Move (a nested Move struct / containers such as Vec and Dict /
   resources), the whole type remains Move—there is no intermediate “partially duplicable” state;
3. **tuple**: The same rule as struct, determined element by element; an empty tuple (the unit) is
   `Void`;
4. **Derivation is recursive**: When a field has a named type, such as `target: Point`, its
   definition is expanded before determination. `A = { b: B }` follows B's derivation result; cyclic
   aliases conservatively fall under Move at the depth limit;
5. **Outside the derivation scope** (still Move, handled separately): containers (Vec / Dict / Set /
   Option / Result / Array) and enums.

```yaoxiang
// &T: Dup, may be aliased freely
view: &Point = &p
view2 = view     // Dup: copies the token; both remain valid
print(view.x)    // Usable
print(view2.x)   // Usable

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup and cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship to Dup

**Clone** is an explicit deep-copy interface. Every type may implement Clone and provide a
`.clone()` method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // Deep copy; p remains usable
p2 = p.clone()        // May be cloned multiple times
```

**The difference between Dup and Clone**:

|                            | Dup                                                                  | Clone                                                  |
| -------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------ |
| **Semantics**              | Shallow copy: copies the handle/token and shares the underlying data | Deep copy: creates a complete, independent copy        |
| **Invocation**             | Implicit (automatic during assignment/argument passing)              | Explicit (`.clone()`)                                  |
| **Effect of modification** | Changes affect one another (the underlying data is shared)           | Changes do not affect one another (independent copies) |
| **Applicable types**       | `&T` tokens, `ref T`                                                 | Any type implementing the Clone interface              |
| **Cost**                   | Zero overhead (a token is a zero-sized type)                         | Depends on the type                                    |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copies the token while the underlying data is shared
view: &Point = &p
view2 = view        // Dup: copies the token; both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable; both see the same data

// Primitive value type: the compiler automatically copies the value (not Dup)
x: Int = 42
y = x               // Value copy; x and y are completely independent
print(x)            // Usable

// Clone: explicit deep copy that creates an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy; p remains usable

// Non-Dup type (a field falls under Move and is not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field and does not derive Dup (see the derivation rules in §11.2)
```

**Design intent**:

- Dup is intended for tokens and reference types and solves the problem of “multiple views onto the
  same data.”
- Clone is intended for scenarios that require an independent copy; an explicit call makes the cost
  visible.
- Copying primitive value types (Int/Float/Bool/Char) is built-in compiler behavior and is not part
  of Dup.
- Custom types default to Move (zero-copy and high performance); Dup is derived automatically when
  all fields are duplicable (see the derivation rules in §11.2).

## Chapter Twelve: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not “references,” but rather
type-level proofs of access permission.

```
&T      →  零大小，冻结源数据（禁止 WriteToken 在此期间获取），
          冻结保证下多份只读安全 → Dup（可复制）
&mut T  →  零大小，独占读写（禁止任何其他令牌），
          独占访问下复制无意义 → Linear（非 Dup）
```

**Key properties**:

- A token is an **ordinary type** and follows the same scope rules as every other type.
- No lifetime annotation `'a` is required.
- No dedicated borrow checker is needed—type attributes (Dup/Linear) naturally derive permissions.
- It disappears completely after compilation, with zero runtime overhead.

### 12.2 Basic Usage

```yaoxiang
// Method side: declare the parameter type to determine the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)               // The &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // The &mut Point token grants write permission
    self.y = self.y + dy
}

// Call side: the compiler automatically chooses a borrow or Move
p = Point(1.0, 2.0)
p.print()                       // The compiler automatically creates an &Point token
p.shift(1.0, 1.0)               // The compiler automatically creates an &mut Point token
p.print()                       // OK; the previous token was released when the shift call ended

// Multiple &T tokens coexist—Dup types may be copied freely
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Because tokens are ordinary types, they support all operations available to ordinary types:

**Returning a token**—the token propagates together with the return value:

```yaoxiang
// ✅ The child token is returned together with the parent token
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // The token is returned to the caller
print(px_ref)                    // OK; the token remains in scope
```

**Storing in a struct**—a struct may carry token fields:

```yaoxiang
// ✅ A struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // Token field—holds a read-only view of target
}
```

**Closures do not capture; context is fixed at the creation point**—a closure consumes only its own
parameters. When outer data is needed, currying is used at the creation point to fix the value into
the closure:

```yaoxiang
// ✅ Context is fixed through currying: threshold is a parameter, and gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure escapes, the scope in which it was defined may have ended, so it must not
> implicitly capture outer variables. However, the call site—the creation point—is necessarily still
> alive, and fixing the context there as a value that enters the closure is safe.

### 12.4 Automatic Borrow Selection

At the call site, the compiler selects automatically according to the following priorities:

```
1. 如果实参后续还有使用 → 优先创建令牌（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // The parameter type of print is &Point -> the compiler creates an &Point token
p.shift(1.0, 1.0)  // The parameter type of shift is &mut Point -> the compiler creates an &mut Point token
p2 = p             // No subsequent use -> Move
```

**Method receivers follow signature semantics** (following the receiver-spelling convention of
RFC-011a): A receiver of `&T` is a read-only borrow token; a receiver of `&mut T` is a mutable
borrow token; a receiver by value is a Move that consumes the receiver. Borrow tokens created at a
call site are released when the call ends (transient; interval semantics in §12.5). A borrowed
receiver of an interface must be declared explicitly as `&Self` by the interface author, and after
replacing `Self ↦ impl type` in the impl signature, it must exactly match the interface (RFC-011a
§3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrowing Horn proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification. Token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse-BFS Liveness Analysis):

```yaoxiang
// ❌ An &mut and a derived &T cannot be live at the same time
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ The token is released automatically when its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Uses &mut Point
    }
    // The inner scope ends
    p.x = 10.0                   // ✅ WriteToken remains usable
}

// ❌ The same argument cannot simultaneously create an &mut token and another token
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: The Branding Mechanism

Users never encounter brands. Internally, the compiler assigns every token a unique compile-time
identifier:

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

The purposes of a brand are:

- **Anti-forgery**: A token may only be obtained from its owner's capsule and cannot be fabricated
  from nothing.
- **Association tracking**: A derived `&Float` obtained through field access carries a derived brand
  (`#N.field_x`), allowing the compiler to trace it back to the parent token.
- **Conflict detection**: A WriteToken and a derived ReadToken from the same source cannot be live
  at the same time.

Brands disappear completely after monomorphization and inlining and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup is safe)
               | &mut T      // WriteToken (exclusive read/write → Linear)
```

### 12.8 Borrow Tokens vs. ref

|                 | `&T` / `&mut T`                                                   | `ref`                                             |
| --------------- | ----------------------------------------------------------------- | ------------------------------------------------- |
| Purpose         | Take a look / modify in place                                     | Shared ownership                                  |
| Scope           | Follows the scope of the token value                              | Across scopes                                     |
| Cost            | Zero overhead (zero-sized type; disappears after compilation)     | Rc or Arc (chosen by the compiler)                |
| Escape          | Possible (the token propagates through return values and structs) | Intended for escape                               |
| Cross-task      | Not possible (tokens do not implement cross-task passing)         | Possible (the compiler automatically chooses Arc) |
| Cycle detection | Not applicable                                                    | Silent within a task; lint across tasks           |

> Note (undefined): How the contents are read after creating a `ref`
> (dereferencing/method/automatically) has not yet been defined in the specification. The current
> implementation reports E1052 for `*a`. This section will be completed once the behavior is
> defined.

---

## Appendix: Type Definition Quick Reference

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

// A type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implements the Serializable interface
}

// === Function types ===

Adder: Type = (Int, Int) -> Int

// === Termination measures (built-in predicates; see §8.4) ===

// Unary: the anchor is the binding name (self-recursive functions and loops)
// The body must end in a value—the value of `while` = the value of the loop body block = the tail expression (spec §2.9)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}

// Binary: explicitly specify measure ownership (the measure is defined elsewhere)
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

// Compile-time generics: N is referenced in the type position (k: N) -> compile-time value parameter
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Attribute Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, argument passing, and return = ownership transfer

// === Primitive value types (compiler built-ins) ===
Int, Float,     // Automatically value-copied on assignment; the two values are completely independent
Bool, Char      // Not Dup; built-in compiler handling for primitives

// === Dup (shallow copy: copy the handle/token and share the underlying data) ===
&T              // Zero-sized read token; copying the token gives multiple views onto the same data
ref T           // Copying an Rc/Arc increments the reference count and shares the heap data
String, Bytes   // Internal reference counting; assignment copies the handle and shares the underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) → Dup (#398)

// === Not derived (remain Move) ===
Vec/Dict/Set    // Containers and enums are outside the derivation scope (#398)

// === Linear ===
&mut T          // Zero-sized write token; Linear (exclusive and cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // Creates an independent copy; modifications do not affect the original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token; freezes source data → Dup (copyable)
&mut T          // Zero-sized compile-time write token; exclusive read/write → Linear (not copyable)

// Automatic selection at the call site
// 1. The argument is used subsequently -> create a token
// 2. The argument is not used subsequently -> Move
// 3. Preference order: &T < &mut T < Move

// Token propagation
// ✅ May be returned, stored in a struct, or captured by a closure
// ❌ May not cross tasks (tokens do not implement cross-task passing)
```
