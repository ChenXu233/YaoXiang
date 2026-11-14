# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, compound types, generics, and traits.

---

## Chapter Zero: Theoretical Foundations

### 0.1 Curry–Howard Correspondence

The Curry–Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between the type system of a programming language and mathematical
logic:

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
| Case analysis                                  | Type-level `match`                                   |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires a
> type-level recursive function and compiler termination checking.

### 0.2 Types Are Propositions, Programs Are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to a correct constructive proof**. YaoXiang type
  families (such as case analysis on `Nat` using `Add` plus recursive calls) are fundamentally
  type-level encodings of mathematical induction—provided that the compiler can perform termination
  checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition having been constructively proved.

### 0.3 Impact on Language Design

The Curry–Howard correspondence is reflected in YaoXiang as follows:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox
   caused by `Type: Type` (Girard's paradox).
2. **Type families** (RFC-011): Type-level case analysis and recursive calls over the natural-number
   type family `Nat(Zero/Succ)` correspond to the Peano axioms—provided that the compiler performs
   termination checking.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
   quantification of the form “for every integer N, there exists a type.”

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

> **Design note**: Although RFC-010 proposes a unified “everything is an assignment” model
> (`name: type = value`), types and values must still be distinguished at the syntactic level. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`). `TypeExpr`, as a BNF placeholder, corresponds to the implementation's `Type` enum
> and means “a type is expected in this position.”

---

## Chapter Two: Primitive Types

### 2.1 Primitive Types

**There are 15 built-in type names; this section is the authoritative table**: `Never` / `Void` /
`()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` /
`Char` / `String` / `Bytes`. The single source of truth in the implementation is
`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`). This table matches it **name for
name**—the T2 gate of `tools/type-tables --report` compares this table, `CONST_PARAM_TYPES`, and the
LSP type-name list.

| Type      | Logical Correspondence | Description                                                                                 | Default Size |
| --------- | ---------------------- | ------------------------------------------------------------------------------------------- | ------------ |
| `Type`    | —                      | Meta type (not a value type name; see below)                                                | 0 bytes      |
| `Never`   | ⊥ (false/empty type)   | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for every T. | 0 bytes      |
| `Void`    | ⊤ (true/Unit)          | Has a default void value; a zero-field product type. `x: Void = <default>` is legal.        | 0 bytes      |
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

`()` is the **type notation** for `Void` (a zero-element tuple = the unit type) and is synonymous
with `Void`: standard-library signatures write `() -> Void`, and the parser table places `()` and
`Void` in the same arm.

**There is only one valid spelling**. `Int` is not `int`; `Int64` is not `int64`/`i64`; `String` is
not `string`/`str`; `Bool` is not `bool`; `Void` is not `void`; `Never` is not `never`; `Float` is
not `float`/`f64`; `Char` is not `char`; and `Bytes` is not `bytes`. The implementation once
coexisted with 22 such lowercase or abbreviated spellings, all of which were removed in **WBS 6.1.2
(finalized on 2026-10-27)**: using a non-canonical spelling in a type position always reports an
unknown type and is never silently resolved as a user-defined type.

> **`DateTime` is not a built-in name**. It is a **semantic alias in the std layer**: the `now` /
> `parse_time` signatures in `src/std/time.rs` and `src/std/interfaces/time.yx` still use
> `DateTime`, but at runtime it is simply `Int` (fixed together with #338). The parser table retains
> it only as a **temporary compatibility alias**, and this table does not include it. Moving it out
> of the built-in-name table so that it is provided by the std layer belongs to **P10 S11** (issue
> [#513](https://github.com/ChenXu233/YaoXiang/issues/513)).

> **There are no unsigned integer types**. `Uint` (and `Int128`) are not available type
> names—`MonoType` has no unsigned variant. One residual occurrence each remained in the LSP
> type-name list and the `sizeof` name table, but both were removed with WBS 6.1.2. When unsigned
> semantics are needed, use `Int` together with a lower-bound convention.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system, corresponding respectively to
false (⊥) and true (⊤).

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. There is no
   right-hand side that can be written for `x: Never = ...`.
2. **Ex falso**: `Never <: T` holds for every type `T`. `assert(false)` returns `Never`, after which
   subsequent code can pass type checking (although it is never reached).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead-code analysis and `match` branch convergence.

`Never` is a built-in type name registered through the same path as `Int`/`Bool`; it is not a
keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity of the zero-field product type. `x: Void = <default>` is legal. A block's value is given by
its **tail expression** (an empty block `{}` has type `Void`). See
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for details.

---

## Chapter Three: Compound Types

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
- A field name is directly followed by a colon and a type.
- An interface name inside the type body indicates that the type implements that interface.
- Fields are divided into data fields and evidence fields according to the **proof-space criterion**
  (§8.3): if a field's normalized type lies in the proof space (`Proof`), it is an evidence field.
  It does not occupy a constructor argument position or a runtime data slot. If it lies in the data
  space (`Data`), it is always a data field.

> **Namespace ownership**: A `Type.name` prefix (such as `Point.draw`) indicates that a function
> belongs to the `Point` namespace. It does not trigger any implicit binding. For `.` call syntax
> such as `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004
> and RFC-010 for details.

#### 3.1.1 Field Default Values

A type field may specify a default value, which may optionally be supplied during construction:

```yaoxiang
// Field with a default value - optional during construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without a default value - required during construction
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
- Evidence fields have no runtime value and therefore cannot have default values (§8.3).

#### 3.1.2 Built-in Bindings

Methods may be bound directly inside a type definition:

```yaoxiang
// Method 1: Reference an external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // Bind to position 0
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
// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implements the Drawable interface
    Serializable     // Implements the Serializable interface
}
```

**Direct interface assignment**: A concrete type may be assigned directly to a variable of an
interface type (structural subtyping).

```yaoxiang
// Direct assignment (the concrete type is known at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // Compiled to: direct call to circle_draw, no vtable

// Function return value (the concrete type is unknown -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through the vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inferred Result            | Call Method                 |
| ------------------------------------ | -------------------------- | --------------------------- |
| Direct assignment of a concrete type | The concrete type is known | Direct call (zero overhead) |
| Function return value                | Unknown                    | vtable                      |
| Heterogeneous collection             | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable; closure rationale)**: Interfaces in YaoXiang are
structural types (an interface is a record whose fields are all function types), not nominal traits.
Therefore, there is no cross-crate or cross-module ownership question of “who may implement what for
whom,” and Rust-style orphan rules and coherence checks have no applicable subject (see the ruling
in RFC-011 §2.1). The corresponding guarantee in the structural world is **duplicate-implementation
rejection**: defining the same method signature more than once on a type produces a compilation
error (RFC-011a §3; overriding is prohibited, while overloading is legal).

### 3.3 Tuple Types

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.4 Constructors

Record types (§3.1) automatically generate constructors. **Constructor arguments correspond only to
data fields**—compile-time evidence fields (evidence fields, §8.3) do not occupy any constructor
argument position.

| Field                        | Constructor Argument Position                     | Runtime Data Slot      |
| ---------------------------- | ------------------------------------------------- | ---------------------- |
| Data field                   | Occupies one position, in field declaration order | Occupies one position  |
| Evidence field               | Does not occupy a position                        | Does not occupy a slot |
| Function-type field (method) | Does not occupy a position                        | Does not occupy a slot |

**Compile-time arity = runtime arity = number of data fields.** Evidence is not a constructor
argument: it comes from proof obligations automatically generated at the construction site (§8.3.2),
which the compiler synthesizes itself or verifies using an independently supplied proof function.
There is no source-level “evidence argument position.” Supplying one extra argument cannot map to
any data-parameter slot and results in an E1010/E1014-family argument error; it is never silently
discarded.

Fields with default values may be omitted during construction, while fields without default values
are required (§3.1.1).

**Construction sites carry proof obligations**. Each evidence field produces a proof obligation at
the construction site: the compile-time values of strictly preceding data fields are substituted
into the field's proposition, yielding a closed proposition that is verified by the proof kernel
under the current logical signature (§8.3.2). If an obligation is disproved, compilation fails and a
counterexample is provided. If it cannot be proved, supply an independent proof function or a named
proof from the proof library (§8.3.2); do not add a constructor argument. Evidence fields have no
argument positions at construction sites.

**Two views of the same construction**:

- **Compile-time view**: Data arguments are bound in declaration order. Evidence fields do not
  occupy argument positions, but each carries a proof obligation awaiting verification.
- **Runtime view**: Only data fields enter the runtime layout. Evidence fields and their
  certificates are completely erased at runtime (§8.3.6).

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Box: Type = {
    v: Int,          // Data field: constructor argument position 0, runtime data slot 0
    w: Pos(v),       // Evidence field: Pos is a proof-space family, and Pos(v) is a proposition
    z: Int,          // Data field: constructor argument position 1, runtime data slot 1
}

b = Box(3, 5)       // Arity = 2 (v, z); at runtime only v=3 and z=5 exist; w is compile-time evidence
```

> **Design basis**: [RFC-027 §14.1–§14.2](../../rfc/accepted/027-compile-time-evaluation-types.md)
> (field-position rules and construction views),
> [RFC-027 §4](../../rfc/accepted/027-compile-time-evaluation-types.md) (proof kernel and logical
> signatures).
>
> **Implementation status**: Evidence determination for non-generic records (proof space → position
> → category), construction-site proof obligations and data-field arity, and propagation of
> quantifiers through the record model to the IR have landed (WBS 6.9.5 B.2a). Instantiation
> determination for generic records, const parameters, and proof families has also landed (B.2b):
> constructor arguments count only data fields, and evidence-field evidence is synthesized and
> determined automatically at the construction site—a false determination reports E4018, and an
> unproven one reports E2031. Value-level reads report E1114, and mutable writes report E1115.
> Type-level `b.w` (B.3) and cross-module export (B.4) have not yet landed.

### 3.5 Function Types

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= Param (',' Param)*
Param       ::= Identifier ':' TypeExpr    // Named parameter
              | TypeExpr                   // Unnamed parameter (bare type)
```

**Two parameter forms, with different ownership of names** (the type side of RFC-007's shorthand
rules):

- **Named** (`a: Int`): The name enters the contract and may be referenced by a named argument
  (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)). When the signature already declares
  parameter names and types, the lambda header may be omitted
  (`add: (a: Int, b: Int) -> Int = a + b`, RFC-007 §Shorthand Rules).
- **Unnamed** (a bare type such as `(Int, Int) -> Int`): Type constraints apply **by position**,
  while the name belongs to the implementation. The lambda header must provide parameter names
  (`mk: (Int, Int) -> Int = (x, y) => x + y`; under RFC-007, the lambda header may be omitted if and
  only if the signature declares parameter names). Such parameters cannot be referenced by named
  arguments and can only be called positionally. A bare identifier is resolved in the type
  namespace; an error is reported if it does not resolve to a declared type—there is no bare
  identifier in a parameter position that is “neither a type nor a type parameter.”

**Annotated binding semantics**: An annotation is a declaration, not a comment. A binding statement
checks its implementation against the annotation: parameter types drive the lambda header
positionally, the function body's exits (the tail expression and `return`) are uniformly checked
against the annotated return type, and the type registered for callers is the annotated form. An
unannotated binding uses HM inference (RFC-007). `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are rejected under the same rule—binding semantics do not diverge
according to how the value is written (lambda / block / expression / named function reference).

**The implementation header's parameters are positional binders**
(`f: (a: Int) -> Int = (b) => b + 1`):

- **Names and types follow the declaration**: The i-th implementation-header parameter corresponds
  to the i-th parameter of the declaration's **value-level signature** (N = the number of
  implementation-header parameters; the value-level signature is the declaration's parameter
  sequence after removing compile-time positions—`T: Type`, type-level function positions, and const
  positions). The implementation header only declares “what the i-th parameter is called in the
  body”; renaming it does not affect the caller. The names available to `f(a = 7)` come from the
  declaration and are unrelated to the implementation header's names.
- **Annotations in the implementation header are used only for comparison**: If the type at the same
  position differs from the declaration, **E1011** is reported (the declaration is the authoritative
  source, and the author's annotation must not be silently overridden).
- **If the implementation header has more parameters than the declaration's runtime parameter
  positions**, **E1112** is reported: the extra positions have no values to bind (type positions
  such as `T: Type`, type-level function positions, and const positions do not occupy runtime
  parameter positions).
- **The reverse case (the implementation omits parameters) is not an error** (this was the “pass”
  cell in D54's pre-D54 behavior matrix and must not be reclassified): the type level continues to
  check arity according to the **declaration** (`f(1)` is legal, while `f(1, 2)` reports E1010), and
  missing positions are filled into the **type-checking scope** according to the declaration.
  However, the implementation header has no corresponding runtime slots, so arguments at those
  positions are discarded on the IR side and do not participate in computation in the body (in
  `f: (a: Int) -> Int = () => 1`, `f(1)` is unrelated to the argument). **This has been registered
  as a gap**—making the runtime also receive the declared arity would be a D-class behavior change
  and requires a separate ruling.
- **Unified exits**: Whenever the tail expression or `return` (the arrow shorthand `=> x` and
  explicit `return x` are two AST forms) does not match the declared return position, **E1012** is
  reported, with the location pointing to the **annotation**.

> **Implementation status**: Alignment of the binding-period checks for `Fn` annotations has landed
> (WBS 6.8.2, 2026-11-02): parameters are driven positionally (the parser and typechecker use **the
> same alignment rule**: the first N positions of the value-level signature) / implementation-header
> annotation comparison (E1011; the author's annotation is retained and any mismatch with the
> declaration is an error) / unified exits (E1012) / the registered form follows the declaration
> (the names available for named arguments).

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

In a generic type definition, `(T: Type)` is the parameter signature of a type constructor, and
`-> Type` denotes the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives. They are treated in the same
way as user-defined generics and handled through a unified generic-instantiation path. The ownership
of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                                | Foundation                               |
| ------------- | ------------- | ---------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type-level    | Fixed-length array (const generic N)     | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Raw buffer with runtime length; growable | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard-library type (growable list)    | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                        | `HeapValue::Dict`                        |

> `List(T)` is a **standard-library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list` and receives the same treatment as user-defined generic records. All
> growable-list semantics (when to grow, how much to grow, and whether sharing is possible) live in
> the library; the compiler does not participate. `Vec(T)` is the minimal foundational primitive on
> which it depends.
>
> Set(T) has been retired: it has no literal, no runtime representation, and no `std.set`. When the
> requirement arises, complete it following the Dict pattern.

Key rules:

- **Literal placement is determined by context**: A bare `[...]` literal and a `List(T)` annotation
  resolve to a growable list; an `Array(T, N)` annotation applied directly to a literal resolves to
  a fixed-length array. Placement validation requires the number of elements to equal N and their
  types to be compatible with T; otherwise E1002 is reported at compile time. When N is a symbolic
  constant (a const parameter), the count check is deferred to the refinement-type phase.
- **Implicit List→Array conversion is prohibited**: Fixed-lengthness is guaranteed by the type
  layer—`push` accepts only a `List(A)` receiver.
- **Performance hierarchy**: Performance decreases and flexibility increases from bottom to top:
  `Array` > `Vec` > `List`.
- **Index-failure contract** (runtime errors are a transitional state; the target state is
  compile-time refinement coverage through value-dependent types; see §8.5):
  - Out-of-bounds index, including a negative index → `E6003`
  - Missing Dict key → `E6008`
- **String indexing semantics** (#385): `s[i]` uses the character at the **Unicode scalar value
  (code point)** index and produces a single-character `String`. This is the same unit used by `len`
  / `index_of` / `substring` / `chars` / `char_code` in `std.string`, so combining them does not
  introduce a domain mismatch. An out-of-bounds index, including a negative index, follows the
  index-failure contract above and reports `E6003`.
- **Membership `in` predicate**: Returns `Bool` without reporting an error. Its right operand covers
  List/Array/Dict(keys)/Tuple/String/Range. It is a first-class Horn predicate and the foundation of
  propositions that refinement types can prove at compile time.`

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
receiver type must implement the `Try` interface (four members: `is_failure` / `success` /
`residual` / `from_error`). On failure, the result of `from_error(residual(t))` is returned early
from the current function; on success, the expression's value is `success(t)`. The enclosing
function's return type must also implement `Try`, and its failure-residual type must match the
receiver's (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try`
implementations through `std.result` / `std.option` (`Option`'s failure-residual type is `Void`). A
user-defined type can participate in `?` by writing `Try(自身, T, E)` in its type body. Lowering of
`?` does not distinguish built-in and user-defined types—the same interface follows the same path.

A sum type's **variant set enters the checker when the definition or a `use` import is processed**:
`Result`/`Option` require `use std.result` / `use std.option` before use (whole-module and grouped
`use std.{...}` forms have equal authority). Variant construction (in type-qualified form),
match-variant destructuring, and exhaustiveness determination all use this registration as their
sole criterion. A native function signature returning `Result(Float, Error)` mints a `Generic` with
the same name; it has the same identity as the sum type from `std.result` by name and can be
match-destructured after `use`.

### 4.3 Generic Constructor Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each **data
field** corresponds to one constructor argument, and the field name is the argument name. Fields
with default values may be omitted during construction, while fields without default values are
required. Function-type fields (methods) do not generate constructor arguments. Evidence fields
(§8.3) do not generate constructor arguments—their evidence is verified by the compiler at the
construction site and is not passed through an argument position.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value -> constructor argument is required
    extra: T,
}
// Automatically expanded full form (the compiler's internal view; users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }
// The expansion lists only data fields; evidence fields do not generate constructor arguments (§3.4)

// Calls: invoke the automatically generated constructor
c  = Container(42, 43)            // Constructor arguments are filled in field order; T is automatically unpacked from the element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type argument + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // Field-name form, in any order
c5 = Container(Int)()             // Empty construction: fields take their default/zero values (data is assigned later)

// Field default values -> constructor arguments may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses; positions are matched against declared parameters from left to
right):

1. Arguments are matched positionally against the type-declaration parameters: a `Type` position
   accepts a type argument, while a compile-time value parameter position (such as `Int`) accepts a
   compile-time constant.
2. If a compile-time value parameter position matches successfully (a partial match), construction
   is handled as type construction: all parameter positions are checked in order, and on failure the
   **first mismatched or missing parameter in declaration order** is reported.
3. If the arguments do not correspond to the declaration parameters at all (all are values and no
   compile-time value parameter position matches), construction is handled through constructor
   arguments: positional arguments fill fields in order, and type parameters are automatically
   unpacked from the element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),       // Evidence field: Assert is a proof-space family (§8.3)
    data: Array(Array(T, Cols), Rows),    // Data field: Array is a data type family
}

m: Matrix(Int, 3, 4)              // Type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two levels: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern; data is assigned later)

Matrix(42)    // ❌ Position 0: T←42 does not match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols is missing -> report the first error: T expects Type, but 42 was found
Container(42) // ❌ Missing constructor argument extra
Container(42, 43, 44)  // ❌ Too many constructor arguments
```

**Type inference**: A generic type constructor's type parameters are automatically unpacked from
constructor-argument elements (`Container(42, 43)` → T=Int); a generic function's type parameters
are automatically unpacked from argument types (`map(numbers, f)` → T=Int, R=String; see §4.1).
Explicit arguments are required when automatic unpacking is impossible.

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

// Use of a constraint
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

> **Constraint-solving source (RFC-011b)**: Solving an operator constraint name (`Add` / `Subtract`
> / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) means consulting the
> interface-implementation registry—`T: Add` ≜ a registered `Add(T, T, T)` instantiation. `Equal`
> additionally supports structural inference (records whose fields are all comparable are
> automatically comparable). Names such as `Zero` / `One` / `PartialOrd` do not yet have a
> definition source and are dangling constraint names.

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

// Use of an associated type
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
// More complex associated types
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // An associated type may itself be generic
    iter: () -> IteratorType
}
```

---

## Chapter Seven: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant (candidate)
```

> The criterion is **being referenced in a type position**, not “having a concrete type annotation.”
> In `add: (a: Int, b: Int) -> Int = a + b`, `a` and `b` are runtime value parameters because
> neither appears in any type position.

**Terminology**: Whether a generic parameter annotated with a concrete type other than `Type` (such
as `Int`) enters compile time depends on whether its value is referenced in a type position (value
dependence). **No `const` keyword is required** (the implementation once used “const generics,” but
the documentation uniformly uses “compile-time value parameter”).

> **Design basis**:
> [RFC-011 §4.1 Compile-Time Value Parameters](../../rfc/accepted/011-generic-type-system.md).

The decision is driven by **strength declarations that can be read locally within the signature**:
when the author writes a reference to a value parameter in a type position (`(k: N)`, `Array(T, N)`,
`Assert(N > 0)`), that is a strength declaration; its **quantity** is the mechanical reading. Each
binding infers a quantity `q ∈ {0, 1, ω}`: q=0 is computed at compile time, erased at runtime, and
occupies no slot; q=1 is linear (Move); q=ω is unrestricted (Dup). See Chapter Eleven for 1/ω. This
is not implicit: the appearance of a value name in a type position is a declaration written directly
by the author and is the same act as writing `b: Positive(b)` (§8.4).

A **“type position” means every `Type` subtree in a declaration**—annotated parameter types,
function return types, field types in a type body, inner `Fn` types, `Assert` predicates, and
argument positions in type constructions such as `Array(T, N)`. The decision traverses **all**
`Type` subtrees without case-by-case treatment by position; missing even one subtree—for example,
the return type—would misclassify a compile-time value parameter as a runtime value parameter.

Whether something is **“referenced”** is determined through namespace resolution rather than literal
name comparison:

1. The identifier is first resolved in the **type namespace**—declared type names and type
   parameters (`T: Type`).
2. If it resolves to a declared type, it is unrelated to the formal parameter. A formal parameter
   with the same name as a type does **not** constitute shadowing: the value and type namespaces do
   not consult the same table (see the scope and shadowing rules in [modules](modules.md)). The two
   `Int` occurrences in `(Int: Int) -> Int` belong to different namespaces.
3. If it does not resolve, but a formal parameter with the same name exists, the strength
   declaration is matched and that parameter has q=0.
4. If neither resolves, an error is reported: a bare identifier in a type position cannot be
   “neither a type nor a type parameter.”

Self-reference is handled through a monotone fixed point (the set only grows, for at most
|candidates| rounds).

**Slot allocation uses q as its sole input**. The call-site argument sequence and
implementation-header binding sequence are two **derived views** of the same function:

- **Call-site argument view** = formal parameters − type parameters (in declaration order). A q=0
  parameter still occupies an argument position: in `f(42)(5)`, 42 supplies N and 5 supplies n.
  Argument positions represent “the values the author supplied in declaration order.”
- **Implementation-header binding view** = formal parameters − type parameters − q=0 parameters (in
  declaration order). The implementation header binds only names for which runtime values are
  available: `= (n) => n` is legal, while `= (N, n) => n` reports E1112.

Both E1011 and E1112 count according to the **implementation-header binding view**.

| Form                                                       | Determination                       | Reason                                                                    |
| ---------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | They occur only in value positions; there is no strength declaration      |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N occurs in a type-construction argument position                         |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N serves as the type of the inner parameter k                             |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N is an ordinary parameter          | N is not referenced in the type body, so there is no strength declaration |

**Core design**: A compile-time value parameter `(N: Int)` and a value parameter `(k: N)`
distinguish compile-time constants from runtime values. **The caller is checked according to the
declaration's strength**: the legality of `f(y)` depends on whether the argument y carries
compile-time evidence (a literal, a value environment, a single-value pinning, a Γ equation), not on
a global analysis of f's declaration. When there is no evidence, the error is reported at the call
site and points to the q=0 slot visible in f's type. A candidate that is not selected (its form is a
candidate, but it is not referenced in a type position) is an ordinary parameter without a strength
declaration and is handled normally as q=1/ω, with no separate diagnostic code.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (the Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N occurs in a type-construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated in a type position (compile time), and the result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // The compiler computes factorial(5) = 120 at compile time

// Value dependence: N serves as the type of the inner parameter k
// N is a compile-time value parameter (it occurs in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (a single-value type).
// Single-value pinning: k's value is fixed by its single-value type and is known at compile time,
// so k - 1 carries compile-time evidence and can fill the N position of the recursive call.
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)(k - 1)
    }
}
```

### 7.2 Compile-Time Constant Arrays

```yaoxiang
// Use in a matrix type
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// Compile-time dimension validation
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## Chapter Eight: Conditional Types and Dependent Records

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
// IsTrue bridge and Assert refinement type (see §8.4 for details)
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

### 8.3 Dependent Record Σ: Evidence Fields

A **dependent record Σ** is a record type in which later fields may reference earlier fields. Σ's
fields fall into two categories:

- **Data fields**: Fields whose types lie in the data space. They occupy runtime data slots and
  participate in construction and access like ordinary fields.
- **Evidence fields**: Fields whose types lie in the **proof space** (propositions)—that is, “an
  assertion awaiting proof.” They are not runtime data but compile-time evidence: when the record is
  constructed, the compiler must supply a certificate verified by the proof kernel.

The sole criterion for this division is the **proof-space criterion**: normalize the field type and
take the space registered for its **result type family** under the current logical signature—`Proof`
(the result of a proposition / refinement / proof family) or `Data` (runtime data). A result in
`Proof` ⇒ enters evidence determination; a result in `Data` ⇒ a data field, **regardless of how many
preceding fields it references**. The space is provided by the logical signature's sort registration
and library manifest, not inferred from field names, argument forms, or syntax that merely “looks
like a predicate.”

This section calls these **evidence fields**; diagnostic messages call them **witness fields**
(E1114 / E1115). Both terms refer to the same category of field.

> **Design basis**: [RFC-027 §14](../../rfc/accepted/027-compile-time-evaluation-types.md) (complete
> semantics of dependent record Σ),
> [RFC-027 §4.1](../../rfc/accepted/027-compile-time-evaluation-types.md) (sort registration for
> logical signatures).
>
> **Implementation status**: Field classification for non-generic records (proof space → position →
> category), construction-site proof obligations (evidence is synthesized automatically and
> submitted to the sole determination unit; the current determination modes are direct evaluation /
> exact Γ / proof functions, with SMT not used as a proof source), propagation of quantifiers
> through the record model to the IR, and user-reachable E4018 / E2031 / E1114 / E1115 have landed
> (WBS 6.9.5 B.2a). Instantiation determination for generic records, const parameters, and proof
> families has also landed (B.2b). Quantity representation, runtime erasure, and the 5A criterion
> proper belong to an earlier round (WBS 6.9.5 B.1). Type-level `b.w` (B.3) and cross-module export
> (B.4) have not yet landed. `evidence_ref` / `certificate_ref` slots will be added with B.3 / B.4;
> B.2's explicit evidence entry point is a proof function in a constraint, not a constructor
> argument. The diagnostic code for illegal declaration positions is pending a ruling (currently a
> borrowed-code fallback).

#### 8.3.1 Field-Position Rules: Proof-Space Criterion

Suppose the fields of record `R` are `f₀, f₁, …, fₙ₋₁` in source order, and field `fᵢ` has type
`Tᵢ`. Determination has three steps: **first determine the space (proposition family or data
family), then determine the argument position (preceding / self / forward), and finally determine
the field category**.

**Step 1: Determine the space.** Normalize `Tᵢ` and take the space registered for its result type
family under the current logical signature:

- If the space is `Proof` (a proposition / refinement type / `Assert` / `Terminates` / the result of
  a proof family) ⇒ enter evidence determination.
- If the space is `Data` (`Int`, `Array`, `Vec`, an ordinary record, or a **data type family
  application**) ⇒ a **data field**, regardless of how many preceding fields it references.

The `n` in `data: Array(Int, n)` references a preceding data field and changes the “data slot
length”; it does not “generate a proof.” Classifying it as evidence would erase the buffer from the
runtime layout—this is the misclassification that this rule is intended to eliminate.

**Step 2: Determine the position** (this applies only to fields whose space is `Proof`). There are
three cases based on the positions of the arguments:

- All arguments are **strictly preceding data fields** `fₕ (h < i)`, compile-time constants,
  compile-time value parameters, or type parameters ⇒ Step 3 determines an evidence field (a
  compile-time value parameter is treated as a compile-time constant after instantiation; see
  §8.3.3).
- An argument **is `fᵢ` itself** (a self-referential refinement such as `v: Pos(v)`) ⇒ Step 3
  determines a **data field (admission)**: the field is a variable that carries a value, and its
  constraint checks its own value rather than proving a fact about something else.
- A forward reference to a later field, a reference to another evidence field, or using `fᵢ` itself
  as an evidence argument while mixing it with other fields (SelfMixed) ⇒ rejected at declaration
  time.

**Step 3: Determine the category.**

| #   | Form                                                                                                                                                                                                    | Determination                                                         | Runtime Layout                                                                                                              |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| 1   | Space `Proof`, with all arguments being strictly preceding data fields / compile-time constants / compile-time value parameters / type parameters (positive example `w: Pos(v)`)                        | **Evidence field**                                                    | Occupies no slot; `base` does not enter the runtime layout                                                                  |
| 2   | Space `Data`: ordinary value type (`n: Int`), data type family application (positive example `data: Array(Int, n)`), or ordinary record                                                                 | **Data field**                                                        | Occupies a data slot                                                                                                        |
| 3   | Space `Proof`, but an argument is `fᵢ` itself (self-referential refinement `v: Pos(v)`)                                                                                                                 | **Data field (admission)**                                            | Occupies a data slot; the constraint is determined at the construction site under the existing refinement obligation (§8.4) |
| 4   | Space `Proof`, but there is a forward reference to a later field, a reference to another evidence field, or `fᵢ` itself is used as an evidence argument while being mixed with other fields (SelfMixed) | **Illegal**: rejected at declaration time, without silent degradation | No field is produced                                                                                                        |

The four rows do not overlap: Row 2 means “ordinary value type / data family” is always a data
field; Row 3 is the admission data field for “proposition family with a self reference”; and Row 4
means “proposition family with an illegal position” is always rejected at declaration time. **There
is no path under which “a forward reference also counts as a data field”**—a forward reference
occupies neither a data slot nor a runtime data index.

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }     // Proof space

Box: Type = {                         // Positive example: w is evidence
    v: Int,          // Data field
    w: Pos(v),       // Evidence: Pos is a Proof family, and v is a strictly preceding data field
    z: Int,          // Data field; runtime index = 1 (skipping w)
}

Buf: Type = {                         // Negative example: data is still a data field
    n: Int,                 // Data field
    data: Array(Int, n),    // Data field: Array is a Data family; n only determines the length and does not generate proof
}

Admission: Type = {                   // Self-referential refinement: a data field, not evidence
    v: Pos(v),           // Proof family, but the argument is itself => admission (Row 3 of the table)
}
```

**Evidence fields may depend only on strictly preceding data fields.** Their arguments must not
reference another evidence field, a later field, or themselves. This constraint guarantees that,
when the construction site reaches such a field in declaration order, all arguments already have
compile-time values and the proposition is a closed term that can be verified by the proof kernel.

**`base` does not enter the runtime layout.** The base type of an evidence field (the `base` of a
refinement type) is type information used only for kernel equality and normalization. A runtime slot
is never allocated for that field. Evidence fields also have no default values, because they have no
runtime value to supply.

#### 8.3.2 Construction-Site Obligations and Proofs

**There is only one source-level construction form: data arguments.** Evidence fields **do not
occupy any constructor argument position**—there is neither a way to “pass a witness as a value” nor
an “evidence argument position.” Construction arity counts only data fields: **compile-time arity =
runtime arity = number of data fields**.

**Evidence is not a constructor argument; it comes from a proof obligation automatically generated
at the construction site.** When constructing a record, the compiler generates one proof obligation
for each evidence field:

1. Take the compile-time values of the strictly preceding data fields referenced by that field's
   arguments.
2. Substitute them into the field's proposition, obtaining a closed proposition (`Pos(3)` reduces to
   `3 > 0`).
3. Submit it to the proof kernel for verification under the current logical signature (RFC-027 §4).

Determination has three possible outcomes:

| Result    | Compile-Time Behavior                                                                               |
| --------- | --------------------------------------------------------------------------------------------------- |
| Proved    | Passes; the certificate is retained at compile time and erased at runtime                           |
| Disproved | Compilation error + counterexample (**E4018**)                                                      |
| Unproven  | Compilation error + unresolved proposition + reason + suggestion for supplying evidence (**E2031**) |

**Unproven does not mean that the proposition is false**, nor does it allow the program to pass.
Evidence is supplied through an **independent declaration**, not by rewriting the construction
expression: write an **independent proof function** whose return type equals the proposition, or
register a **named proof** in the proof library and reference it from the construction obligation
(RFC-027 §4.3). There is no “fill in an incorrect witness value” form—evidence fields have no
argument positions at construction sites. Supplying an extra source-level evidence argument
(`Box(3, 5, evidence)`) cannot map to any data-parameter slot and reports an E1010 / E1014-family
argument error; it is never silently discarded.

A self-referential refinement field (`v: Pos(v)`) is a data field: its constraint is determined at
the construction site under the existing refinement obligation (§8.4). It does not generate an
evidence obligation and does not change the field's runtime index.

#### 8.3.3 Generic Records: Proof Family Parameter `P: T -> Proposition`

A generic record may have a **proof family parameter** `P` whose kind is `T -> Proposition`: it
accepts a type parameter and returns a proposition. `Proposition` is a sort alias for the `Proof`
space in the logical signature (RFC-027 §4.1), not a source-level type name. Its source-level form
is the same as that of an ordinary type-level function parameter, and its kind constraint is checked
at declaration and instantiation:

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Cell: (T: Type, P: (T: Type) -> Type) -> Type = {
    v: T,        // Data field
    w: P(v),     // Evidence: P is a Proof family, and v is a strictly preceding data field
}
```

- `P`'s kind has the same source as the criterion in §8.3.1: its **result space must be `Proof`**.
  Passing a data type family such as `Array` to `P` violates its kind and is rejected directly at
  declaration or instantiation; this is a type error, not a reason for `w` to “degrade into a data
  field.”
- `P` is a compile-time parameter position and occupies no runtime slot. When `Cell(Int, Pos)` is
  instantiated, `P` is bound to the proof family `Pos`, the proposition of `w` becomes `Pos(v)`, and
  evidence determination is exactly the same as in §8.3.1.
- Determination is rerun **after instantiation**.
- When `P` is unbound (the generic type is not instantiated), the result space of `w` is still
  `Proof` (its kind has been declared), but the proposition is an open term. The construction site
  returns Unproven—it is neither silently accepted nor degraded into a data field.

#### 8.3.4 Cross-Module Export

When a record containing evidence fields is used across modules, its propositions and evidence are
exported together with the type definition, and the importing module does not re-infer them from
field names. The export carries: the kinds of type-family parameters (including proof family
parameters), the proof obligation for each evidence field, and the evidence slot for each evidence
field—the proposition, its explicit evidence source or verified certificate, and the hash of the
logical signature that produced it.

The importing module uses **its own current logical signature** to verify these propositions. When
the signature hashes differ, RFC-027 §4.1's three explicit paths apply—translation items, bridge
axioms, or reproof under the importing module's signature. The importing module must not directly
reuse the exporting module's conclusions.

A referenced proof-library entry must appear in the importing module's library list. Otherwise, the
conclusion is Unproven, and the diagnostic suggests registering the library or writing a bridge
axiom.

#### 8.3.5 Type-Level Use and Value-Level Rejection

Evidence fields **may only be used at the type level**. A `b.w` in a type position is
**user-writable type-level syntax** referring to that field's **evidence item** (proof item), not a
runtime value. It points to the evidence reference registered for the field, may serve as a premise
for another proposition, and is replayed by the proof kernel under the current logical signature.

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Box: Type = {
    v: Int,
    w: Pos(v),
}

// Type-level use: treat b.w as an evidence premise, not as a value read
// `Proved` here denotes proposition construction "with this evidence item as a premise"
useWitness: (b: Box) -> Type = { Proved(b.w) }
```

**Value-level reads of evidence fields are always rejected**: reading `b.w` in a value expression
reports **E1114** (`witness_field_not_readable`). A field path is not a value term, and neither the
evaluator nor the runtime representation materializes a value for it. If runtime data is required,
define the field as an ordinary data field.

```yaoxiang
b = Box(3)
x = b.w        // ❌ Value-level read of a witness field -> E1114
```

> **Design basis**: [RFC-027 §14.5](../../rfc/accepted/027-compile-time-evaluation-types.md) (the
> term form for type-level use and its replacement of the “no user syntax at the type level” rule in
> 4A). The syntax and parsing of type-level field paths have not yet landed—the parser currently
> does not accept a field path in a proof position; this form will be connected with 6.9.5 B.3.

#### 8.3.6 Immutability, Erasure, and Diagnostic Codes

**5A: A record containing evidence fields is immutable as a whole.** Data fields must not declare
`mut`, undergo field assignment, be borrowed through `&mut`, or be used as the receiver of a
mutating method (including automatic write borrowing). Otherwise, the evidence's proposition could
become invalid because a field was rewritten. A violation reports **E1115**
(`witness_record_immutable`). Records without evidence fields are not subject to this restriction.

**Erasure.** Evidence fields do not occupy runtime slots, and the runtime layout contains only data
fields. Proof certificates are retained at compile time and completely erased at runtime (RFC-027
§13). Runtime data indices count only data fields—the data index of `z` in record `{ v, w, z }` is
1, skipping witness field `w`. Runtime-mode boolean checks (§8.4) are unrelated to evidence: they
check runtime values, not certificates.

**Division of diagnostic codes** (each code handles one concern and does not replace another):

| Code      | Trigger                                                                                   | Semantics                                                                  |
| --------- | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| **E4018** | An evidence proposition at a construction site is disproved                               | A refinement predicate is violated; a counterexample is attached           |
| **E2031** | An evidence proposition at a construction site is Unproven through the refinement channel | The refinement constraint cannot be proved; explicit evidence is suggested |
| **E1114** | Value-level read of an evidence field                                                     | Evidence fields cannot be read at the value level                          |
| **E1115** | Mutable write to an evidence record                                                       | The record is immutable as a whole                                         |
| **E4023** | General Unproven outcome outside the refinement channel                                   | General proof failure, with proposition, reason, and location              |

**Declaration-time rejection list** (consistent item by item with the four-category table in
[RFC-027 §14.6](../../rfc/accepted/027-compile-time-evaluation-types.md)): when a field whose space
is `Proof` has one of the following forms, it is rejected directly at its declaration. It does not
proceed to the construction site and is not silently degraded into a data field:

| #   | Rejected Form                                                                                                | Description                                                                                                        |
| --- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| 1   | Forward reference to a later field `fₕ (h > i)`                                                              | Illegal position                                                                                                   |
| 2   | Reference to another evidence field as an argument                                                           | Illegal position                                                                                                   |
| 3   | Using `fᵢ` itself as an evidence argument while mixing it with other fields (SelfMixed, such as `P(fᵢ, fₕ)`) | Illegal position; pure self-reference `P(fᵢ)` is an admission data field under Row 3 of §8.3.1 and is not rejected |
| 4   | The kind of proof family parameter `P` is a data type family (§8.3.3)                                        | Kind mismatch; a type error at declaration/instantiation                                                           |

**Evidence fields must not have default values** (`w: Pos(v) = expr`, DefaultOnEvidence): an
evidence field has no runtime value to supply, and the construction view has no argument position
for it, so specifying a default is rejected at declaration time. This is an independent
field-declaration check. It is not merged with the four categories above and is not silently treated
as “a data field with a default value”; `StructFieldInfo.has_default` is always `false` for a
witness. The default-value rules in §3.1.1 (optional when a default exists, required otherwise)
apply only to data fields.

Data type family applications whose space is `Data` and pure self-referential refinements whose
space is `Proof` (`v: Pos(v)`, admission) are both legal data fields and do not constitute illegal
positions. When a generic type is not instantiated, its proposition is an open term, and the
construction site returns Unproven (§8.3.3), not a declaration-time rejection.

### 8.4 Assert Refinement Types and assert Assertions

`assert` and `Assert` are two sides of the same refinement primitive. A dispatch pipeline
automatically selects between them according to "whether the predicate's free variables are
accessible at compile time."

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                            |
| ----------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enters the proof pipeline: true → erased to Void; false → compilation error (Never is uninhabited)  |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Inserts a runtime Bool check and injects a refinement fact into the flow-sensitive assumption set Γ |

Dispatch is an instance of **checking according to declaration strength** (§7.1). The same principle
covers the entire strength ladder: classical types without a strength declaration do not enter the
proof pipeline and do not undergo quantity inference; predicate annotations (this section), q=0
slots (§7.1), dependent-record fields (§8.3), and relational predicates each bear the corresponding
checks according to their declaration form. Cost follows declaration strength rather than being
determined by global analysis.

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: the old assumption is invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (the kill
set). When branches merge, Γ is the intersection of the branch sets.

### 8.5 Terminates: Termination Measure Predicates

`Terminates` is a **built-in predicate** and, like `Int` and `Never`, belongs to the core primitives
(a built-in name, not a keyword). It binds a **measure** to a computation, declares that the
computation terminates, and provides a witness of termination.

**Forms**: Two arities using the same predicate:

| Form                    | Anchor                             | Use                                                                                                                                       |
| ----------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The name of the containing binding | Default form—self-recursive functions, loops                                                                                              |
| `Terminates(FnType, m)` | Explicit function type             | When the measure's ownership must be specified explicitly (the measure is defined elsewhere, or one measure serves multiple computations) |

```yaoxiang
// Measure: an ordinary function; unit-testable and reusable; does not participate at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere and its ownership is specified explicitly
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, and the measure is an expression in scope
// The loop body is a `{}` block whose value is given by the tail expression (spec §2.9): it is not `Void` only when the body ends with an expression
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines the value type of the computation annotated in its type
position. The obligation falls on that computation—for a function, at every recursive call site; for
a loop, at the back edge—in both cases taking the form “the next state's measure is strictly less
than the current state's measure,” determined under the path guard at that point.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back edge is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: A loop is an anonymous construct and usually cannot be referred to. Its
> binding name is its name—the `acc` in `acc: Terminates(n - i) = while ...` provides the anchor,
> allowing the loop to be referred to. This is why the unary form of `Terminates` applies to loops.

**Measure**: The return type is not restricted (it need not be a natural number); “strict decrease”
is determined by a well-founded order available on that type. Whether the measure is well
founded—for example, whether an `Int` measure is `>= 0`—is an **independent obligation**, determined
by the compile-time proof pipeline just like the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. Ordinary types that have not been refined (such as a bare `while` loop or
a function without a refined signature) do not enter verification mode and do not generate
termination obligations.

**Automatic exploration takes priority**: The compiler first attempts to explore a measure
automatically, using four templates—linear ranking functions, predicate-violation counts, bounded
increment/decrement, and multiplicative scaling. An explicit `Terminates` is needed only when
automatic exploration fails.

**Runtime representation**: This is a purely compile-time entity and is erased with its witness; it
does not enter the runtime binary.

> See [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md) for the complete
> semantics and [RFC-027a](../../rfc/accepted/027a-termination-explicit-measure.md) for the
> implementation mechanism.

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

**Syntax**: A type intersection `A & B` denotes a type that satisfies both A and B.

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Use of an intersection type
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

YaoXiang distinguishes only one kind of type attribute: linear versus copyable. It is inferred
automatically by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, argument passing, and return transfer
ownership.

```yaoxiang
// A struct containing a heap-buffer field does not derive Dup (the field falls under Move) -> Move by default
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move; b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle, Share the Data)

**The Dup attribute is used for reference/token types and internally reference-counted handle value
types**. Assigning a Dup type performs a shallow copy—the handle or token is copied, while the
underlying data is shared. Multiple holders refer to the same data.

| Type            | Attribute | Description                                                                                |
| --------------- | --------- | ------------------------------------------------------------------------------------------ |
| `&T`            | Dup       | Zero-sized read token; copying the token gives multiple views of the same data             |
| `ref T`         | Dup       | Rc/Arc copy = reference count + 1, with shared heap data                                   |
| String, Bytes   | Dup       | Internal reference counting; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear    | Zero-sized write token; exclusive access and not copyable                                  |
| struct          | Derived   | All fields are copyable (primitive value types ∪ Dup) → Dup; otherwise Move (#398)         |
| tuple           | Derived   | Determined element by element, using the same rules as structs (#398)                      |
| All other types | Move      | Default ownership transfer                                                                 |

**Primitive value types** (Int, Float, Bool, Char) receive special built-in handling by the
compiler: assignment automatically copies the value, and the two values are completely independent.
This is native compiler behavior and is not part of the Dup type attribute.

**Derivation rules** (finalized in #398)—the formulation “derive Dup automatically when all fields
are Dup” cannot be applied literally: primitive fields such as Int are not themselves Dup, so
`{ x: Int, y: Int }` would be misclassified as Move. The executable formulation is:

1. The **copyable-field set** consists of primitive value types (Int / Float / Bool / Char / Range)
   ∪ Dup (`&T`, `ref T`, String / Bytes, function values (#352), and composite types that have
   already been derived as Dup).
2. **struct**: If all fields belong to the copyable-field set, Dup is derived. If **any** field
   falls under Linear (`&mut T`) or Move (a nested Move struct / containers such as Vec and Dict /
   resources), the struct as a whole remains Move; no intermediate “partially copyable” state is
   introduced. Evidence fields have no runtime value and do not participate in derivation (§8.3).
3. **tuple**: The same rules as structs, determined element by element; an empty tuple (the unit) is
   `Void`.
4. **Derivation is recursive**: When a field has a named type (such as `target: Point`), that type's
   definition is expanded before determination. `A = { b: B }` follows B's derived result. Cyclic
   aliases conservatively fall under Move at the depth limit.
5. **Outside the derivation scope** (still Move, handled separately): containers (Vec / Dict / Set /
   Option / Result / Array) and enums.

```yaoxiang
// &T: Dup; it may be freely aliased
view: &Point = &p
view2 = view     // Dup: the token is copied, and both remain valid
print(view.x)    // Usable
print(view2.x)   // Usable

// &mut T: Linear and not copyable
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
p2 = p.clone()        // It may be cloned multiple times
```

**Difference between Dup and Clone**:

|                        | Dup                                                                         | Clone                                                  |
| ---------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------ |
| **Semantics**          | Shallow copy: the handle/token is copied, and the underlying data is shared | Deep copy: creates a complete, independent copy        |
| **Invocation**         | Implicit (automatic on assignment/argument passing)                         | Explicit (`.clone()`)                                  |
| **Effect of mutation** | Changes are mutually visible (the underlying data is shared)                | Changes do not affect one another (independent copies) |
| **Applicable types**   | `&T` tokens, `ref T`                                                        | Any type implementing the Clone interface              |
| **Cost**               | Zero overhead (a token is a zero-sized type)                                | Depends on the type                                    |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: the token is copied, and the underlying data is shared
view: &Point = &p
view2 = view        // Dup: the token is copied, and both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable; both see the same data

// Primitive value type: the compiler performs an automatic value copy (not Dup)
x: Int = 42
y = x               // Value copy; x and y are completely independent
print(x)            // Usable

// Clone: explicit deep copy that creates an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy; p remains usable

// Non-Dup type (a field falls under Move and Dup is not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field and does not derive Dup (see the derivation rules in §11.2)
```

**Design intent**:

- Dup is used for tokens and reference types, solving the problem of “multiple views of the same
  data.”
- Clone is used when an independent copy is required, and explicit invocation makes the cost
  visible.
- Copying primitive value types (Int/Float/Bool/Char) is built-in compiler behavior and is not part
  of Dup.
- User-defined types use Move by default (zero-copy, high performance); Dup is derived automatically
  when all fields are copyable (see the derivation rules in §11.2).

## Chapter Twelve: Borrowing Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not “references,” but
“type-level proofs of access permission.”

```
&T      →  零大小，冻结源数据（禁止 WriteToken 在此期间获取），
          冻结保证下多份只读安全 → Dup（可复制）
&mut T  →  零大小，独占读写（禁止任何其他令牌），
          独占访问下复制无意义 → Linear（非 Dup）
```

**Key properties**:

- A token is an **ordinary type** and follows the same scoping rules as every other type.
- No lifetime annotation such as `'a` is required.
- No dedicated borrow checker is needed—the type attributes (Dup/Linear) naturally infer
  permissions.
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

// Call site: the compiler automatically selects a borrow or Move
p = Point(1.0, 2.0)
p.print()                       // The compiler automatically creates an &Point token
p.shift(1.0, 1.0)               // The compiler automatically creates an &mut Point token
p.print()                       // OK; the previous token was released when the shift call ended

// Multiple &T tokens coexist—the Dup type permits free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types and therefore support all operations available to ordinary types:

**Returning tokens**—tokens propagate together with the return value:

```yaoxiang
// ✅ The child token is returned together with the parent token
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // The token is returned to the caller
print(px_ref)                    // OK; the token is still in scope
```

**Storing tokens in structs**—structs may carry token fields:

```yaoxiang
// ✅ A struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // Token field—holds a read-only view of target
}
```

**Closures do not capture; context is fixed at the creation point**—a closure accepts only its own
parameters. When outer data is needed, currying fixes the value into the closure at the creation
point:

```yaoxiang
// ✅ Context is fixed through currying: threshold is a parameter, and gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope in which it was defined may no longer be
> live, so it must not implicitly capture outer variables. The scope at the call site (the creation
> point) is necessarily live, so fixing the context there as a value inside the closure is safe.

### 12.4 Automatic Borrow Selection

At the call site, the compiler selects automatically according to the following priorities:

```
1. 如果实参后续还有使用 → 优先创建令牌（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point -> the compiler creates an &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point -> the compiler creates an &mut Point token
p2 = p             // No subsequent use -> Move
```

**Method receivers follow signature semantics** (following RFC-011a's receiver-spelling convention):
a receiver of `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value → Move
(consumes the receiver). Borrow tokens created at the call site are released when the call ends
(transient; interval semantics in §12.5). An interface's borrowing receiver must be explicitly
declared as `&Self` by the interface author, and after replacing `Self ↦ impl type` in the impl
signature, it must exactly match the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is the **borrowing Horn proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrowing propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and submits them to the proof
pipeline for verification. Token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and a derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of a WriteToken
    print(p.y)
}

// ✅ The token is automatically released when its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Uses &mut Point
    }
    // The inner scope ends
    p.x = 10.0                   // ✅ The WriteToken is still usable
}

// ❌ The same argument cannot simultaneously create an &mut token and another token
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Branding Mechanism

Users never interact with brands. Internally, the compiler assigns a unique compile-time identifier
to every token:

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N 是编译期唯一整数
&mut Point     →  WriteToken(Point, #M)   // #M 是编译期唯一整数
```

Brands serve the following purposes:

- **Prevent forgery**: A token may only be obtained from its owner's capsule and cannot be
  fabricated from nothing.
- **Track associations**: A derived `&Float` carries its derived brand (`#N.field_x`), allowing the
  compiler to trace it back to the parent token.
- **Detect conflicts**: A WriteToken and a derived ReadToken from the same source cannot be live
  simultaneously.

Brands disappear completely after monomorphization and inlining and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data -> safe to Dup)
               | &mut T      // WriteToken (exclusive read/write -> Linear)
```

### 12.8 Borrowing Tokens vs. ref

|                 | `&T` / `&mut T`                                               | `ref`                                              |
| --------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Purpose         | Take a look / modify in place                                 | Shared ownership                                   |
| Scope           | Follows the scope of the token value                          | May cross scopes                                   |
| Cost            | Zero overhead (zero-sized type; disappears after compilation) | Rc or Arc (selected by the compiler)               |
| Escape          | Possible (tokens propagate through return values/structs)     | Intended for escaping                              |
| Cross-task      | Not supported (tokens do not yet support cross-task transfer) | Supported (the compiler automatically selects Arc) |
| Cycle detection | Not involved                                                  | Silent within a task; lint across tasks            |

> Note (undefined): How to read the contents of a `ref` after creation (dereference/method/automatic
> behavior) has not yet been defined in the specification. The current implementation reports E1052
> for `*a`. This section will be updated once it is defined.

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definitions

```
// === Record type (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Dependent record Σ (evidence fields; see §8.3) ===

// Compile-time predicate + evidence field: w does not occupy a constructor argument position or runtime data slot
Pos: (v: Int) -> Type = { v > 0 }
Box: Type = { v: Int, w: Pos(v) }

// === Interface type (curly braces, all fields are functions) ===

// Interface definition
Serializable: Type = { serialize: () -> String }

// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implements the Serializable interface
}

// === Function type ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (built-in predicate; see §8.5) ===

// Unary: the anchor is the binding name (self-recursive functions, loops)
// The body tail must provide a value—the value of `while` = the value of the loop-body block = the tail expression (spec §2.9)
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

// Compile-time generic: N is referenced in the type position (k: N) -> compile-time value parameter
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Proof family parameter: its kind is T -> Proposition (the result space must be Proof)
Cell: (T: Type, P: (T: Type) -> Type) -> Type = { v: T, w: P(v) }

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Attribute Quick Reference

```
// === Move (default) ===
// All types use Move by default. Assignment, argument passing, and return transfer ownership

// === Primitive value types (built into the compiler) ===
Int, Float,     // Assignment automatically copies the value, and the two values are completely independent
Bool, Char      // Not Dup; built-in compiler handling for primitives

// === Dup (shallow copy: copy the handle and share the underlying data) ===
&T              // Zero-sized read token; copying the token gives multiple views of the same data
ref T           // Rc/Arc copy = reference count + 1, sharing heap data
String, Bytes   // Internal reference counting; assignment copies the handle and shares the underlying buffer
struct / tuple  // Derived: all fields ∈（primitive value types ∪ Dup）→ Dup (#398)

// === Not derived (remain Move) ===
Vec/Dict/Set    // Containers and enums are outside the derivation scope (#398)

// === Linear ===
&mut T          // Zero-sized write token; Linear (exclusive and not copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Creates an independent copy; modifications do not affect the original value
```

### A.4 Borrowing Token Quick Reference

```
// === Borrowing tokens ===
&T              // Zero-sized compile-time read token; freezes source data -> Dup (copyable)
&mut T          // Zero-sized compile-time write token; exclusive read/write -> Linear (not copyable)

// Automatic selection at the call site
// 1. The argument is used subsequently -> create a token
// 2. The argument is not used subsequently -> Move
// 3. Priority: &T < &mut T < Move

// Token propagation
// ✅ May be returned, stored in a struct, or captured by a closure
// ❌ May not cross tasks (tokens do not yet support cross-task transfer)
```
