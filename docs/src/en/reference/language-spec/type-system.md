# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, compound types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence (Curry-Howard correspondence) is the theoretical foundation of the
YaoXiang type system. It reveals the deep correspondence between the type system of a programming
language and mathematical logic:

| Logic                                          | Programming Language                                 |
| ---------------------------------------------- | ---------------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                          |
| Proof \(p: P\)                                 | Program `x: T = ...`                                 |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                             |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                        |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                          |
| Universal quantification \(\forall x:T. P(x)\) | Generic `(T: Type) -> ...`                           |
| True \(\top\)                                  | `Void` (Unit, with a default value)                  |
| False \(\bot\)                                 | `Never` (zero constructors, no inhabitant)           |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                                   |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions and compiler termination checking.

### 0.2 Types Are Propositions, Programs Are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computations correspond to correct constructive proofs**. YaoXiang type
  families (such as `Add` implemented through case analysis on `Nat` plus recursive calls) are
  fundamentally type-level encodings of mathematical induction—provided that the compiler can
  perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition having been constructively proved.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard correspondence in YaoXiang are:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox
   caused by `Type: Type` (Girard's paradox).
2. **Type families** (RFC-011): type-level case analysis and recursive calls over the natural-number
   type `Nat(Zero/Succ)` correspond to Peano's axioms—provided that the compiler performs
   termination checking.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to
   case-based disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to bounded
   quantification of the form “for every integer N, there exists a type.”

---

## Chapter 1: Type Categories

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
> `ast.rs:25`). `TypeExpr` is a BNF placeholder corresponding to the `Type` enum in the
> implementation and indicates “a type is expected in this position.”

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

**There are 15 built-in type names; this section is the authoritative table:** `Never` / `Void` /
`()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` /
`Char` / `String` / `Bytes`. The single source of truth in the implementation is
`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`), and this table matches it **name
for name**—the T2 gate in `tools/type-tables --report` compares this table, `CONST_PARAM_TYPES`, and
the LSP type-name list.

| Type      | Logical Correspondence | Description                                                                                     | Default Size |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------- | ------------ |
| `Type`    | —                      | Meta type (not a value type name; see below)                                                    | 0 bytes      |
| `Never`   | ⊥ (false/empty type)   | Zero constructors, no values. Return type for divergence/panic. `Never <: T` holds for every T. | 0 bytes      |
| `Void`    | ⊤ (true/Unit)          | Has a default void value; a zero-field product type. `x: Void = <default>` is valid.            | 0 bytes      |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                 | 1 byte       |
| `Int`     | —                      | Signed integer (= `Int64`)                                                                      | 8 bytes      |
| `Int8`    | —                      | 8-bit signed integer                                                                            | 1 byte       |
| `Int16`   | —                      | 16-bit signed integer                                                                           | 2 bytes      |
| `Int32`   | —                      | 32-bit signed integer                                                                           | 4 bytes      |
| `Int64`   | —                      | 64-bit signed integer                                                                           | 8 bytes      |
| `Float`   | —                      | Floating-point number (= `Float64`)                                                             | 8 bytes      |
| `Float32` | —                      | 32-bit floating-point number                                                                    | 4 bytes      |
| `Float64` | —                      | 64-bit floating-point number                                                                    | 8 bytes      |
| `String`  | —                      | UTF-8 string                                                                                    | Variable     |
| `Char`    | —                      | Unicode character                                                                               | 4 bytes      |
| `Bytes`   | —                      | Raw bytes                                                                                       | Variable     |

`()` is the **type notation** for `Void` (a zero-element tuple = the unit type) and is synonymous
with `Void`: standard-library signature strings write `() -> Void`, and the parser table places `()`
and `Void` in the same arm.

**There is only one spelling**. `Int` is not `int`; `Int64` is not `int64`/`i64`; `String` is not
`string`/`str`; `Bool` is not `bool`; `Void` is not `void`; `Never` is not `never`; `Float` is not
`float`/`f64`; `Char` is not `char`; and `Bytes` is not `bytes`. The implementation once contained
22 such lowercase or abbreviated spellings, all of which were removed in **WBS 6.1.2 (finalized
2026-10-27)**: using a noncanonical spelling in a type position always produces an unknown-type
error and is never silently resolved as a user-defined type.

> **`DateTime` is not a built-in name**. The `now` / `parse_time` functions in `std.time` return an
> `Int` timestamp (fixed together with #338); the parser table's **temporary compatibility alias**
> was moved out in **P10 S11** (issue
> [#513](https://github.com/ChenXu233/YaoXiang/issues/513))—stdlib signature strings now directly
> use `Int`, and a bare `DateTime` annotation produces the same unknown-type error as any other
> unknown type name.

> **There are no unsigned integer types**. `Uint` (and `Int128`) are not available type
> names—`MonoType` has no unsigned variant, and the LSP type-name list and the `sizeof` name table
> each previously contained one residual entry, both removed in WBS 6.1.2. When unsigned semantics
> are needed, use `Int` together with a lower-bound convention.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system, corresponding respectively to
false (⊥) and true (⊤).

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. There is no
   right-hand side that can be written for `x: Never = ...`.
2. **Principle of explosion**: `Never <: T` holds for every type `T`. `assert(false)` returns
   `Never`, after which the code passes type checking (although that point is never reached).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead-code analysis and `match` branch confluence.

`Never` is a built-in type name (registered through the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity element of zero-field product types. `x: Void = <default>` is valid. A block's value is
determined by its **tail expression** (an empty block `{}` has type `Void`); see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for details.

---

## Chapter 3: Compound Types

### 3.1 Record Types

**Unified syntax**: `Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // Interface constraint
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
- A field name is followed directly by a colon and its type.
- An interface name inside the type body indicates that the type implements that interface.
- Fields are divided into data fields and evidence fields according to the **proof-space criterion**
  (§8.3): a field whose normalized type falls in the proof space (`Proof`) is an evidence field. It
  occupies neither a constructor argument position nor a runtime data slot; any field falling in the
  data space (`Data`) is a data field.

> **Namespace ownership**: A `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to the `Point` namespace. It does not create any implicit binding. To make `.` call syntax
> such as `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and
> RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, which are optional during construction:

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

- `field: Type = expression` -> Has a default value; optional during construction.
- `field: Type` -> Has no default value; required during construction.
- Evidence fields have no runtime value and therefore cannot have default values (§8.3).

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
d.draw(screen)        // After compilation: directly calls circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Looks up the method through the vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inference Result              | Call Method                 |
| ------------------------------------ | ----------------------------- | --------------------------- |
| Direct assignment of a concrete type | Concrete type is determinable | Direct call (zero overhead) |
| Function return value                | Unknown                       | vtable                      |
| Heterogeneous collection             | Multiple types                | vtable                      |

**Coherence and orphan rules (not applicable; closure note)**: YaoXiang interfaces are structural
types (an interface is a record whose fields are all function types), not nominal traits.
Consequently, there is no cross-crate/module “who may implement what” ownership issue, and
Rust-style orphan rules and coherence checks have no applicable targets (see RFC-011 §2.1 for the
decision record). The corresponding guarantee in a structural world is **duplicate-implementation
rejection**: defining the same method signature more than once on a type produces a compilation
error (RFC-011a §3, overriding is prohibited; overloading is permitted).

### 3.3 Tuple Types

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.4 Constructors

Record types (§3.1) automatically generate constructors. **Constructor arguments correspond only to
data fields**—compile-time evidence fields (evidence fields, §8.3) occupy no constructor argument
position.

| Field                        | Constructor Argument Position                     | Runtime Data Slot |
| ---------------------------- | ------------------------------------------------- | ----------------- |
| Data field                   | Occupies one position, in field declaration order | Occupies one slot |
| Evidence field               | Occupies no position                              | Occupies no slot  |
| Function-type field (method) | Occupies no position                              | Occupies no slot  |

**Compile-time arity = runtime arity = number of data fields.** Evidence is not a constructor
argument: it comes from proof obligations automatically generated at the construction site (§8.3.2),
which the compiler synthesizes itself or verifies using an independently supplied proof function.
There is no source-level “evidence argument position.” Passing one extra argument cannot reach any
data-parameter slot and produces an E1010 / E1014-family argument error; it is not silently ignored.

Fields with default values may be omitted during construction, whereas fields without default values
are required (§3.1.1).

**Construction sites carry proof obligations**. Each evidence field creates a proof obligation at
the construction site: substitute the compile-time values of strictly preceding data fields into the
field's proposition to obtain a closed proposition, then have the proof kernel verify it under the
current logical signature (§8.3.2). If an obligation is disproved, compilation fails and a
counterexample is provided. If it cannot be proved, supply an independent proof function or a named
proof from a proof library (§8.3.2)—not an additional constructor argument—because evidence fields
have no argument position at construction sites.

**Two views of the same construction**:

- **Compile-time view**: Data arguments are bound in declaration order; evidence fields occupy no
  argument positions, but each carries a proof obligation awaiting verification.
- **Runtime view**: Only data fields enter the runtime layout; evidence fields and their
  certificates are completely erased at runtime (§8.3.6).

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Box: Type = {
    v: Int,          // Data field: constructor argument position 0, runtime data slot 0
    w: Pos(v),       // Evidence field: Pos is a proof-space family, and Pos(v) is a proposition
    z: Int,          // Data field: constructor argument position 1, runtime data slot 1
}

b = Box(3, 5)       // arity = 2 (v, z); at runtime only v=3 and z=5 exist; w is compile-time evidence
```

> **Design basis**: [RFC-027 §14.1–§14.2](../../rfc/accepted/027-compile-time-evaluation-types.md)
> (field-position rules and constructor views),
> [RFC-027 §4](../../rfc/accepted/027-compile-time-evaluation-types.md) (proof kernel and logical
> signatures).
>
> **Implementation status**: Evidence classification for non-generic records (proof space → position
> → category), construction-site proof obligations and data-field arity, and propagation of
> quantifiers to IR through the record model have been implemented (WBS 6.9.5 B.2a); generic
> records, const parameters, and instantiation classification for proof families have also been
> implemented (B.2b): constructor arguments count only data fields, while evidence for evidence
> fields is synthesized and classified automatically at the construction site—disproof reports
> E4018, failure to prove reports E2031, value-level reads report E1114, and mutable writes report
> E1115. Type-level `b.w` (B.3) has been implemented (admission form, D2 decision A); cross-module
> export (B.4) has been implemented (V1 re-verification only: per-library source-fingerprint
> comparison plus re-verification by the importer; translation/bridging awaits the §4.1 kernel; the
> `evidence_ref`/`certificate_ref` slots are not materialized, D1 decision A).

### 3.5 Function Types

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= Param (',' Param)*
Param       ::= Identifier ':' TypeExpr    // Named parameter
              | TypeExpr                   // Unnamed parameter (bare type)
```

**Two parameter forms, with different ownership of names** (the type-level aspect of RFC-007's
shorthand rules):

- **Named** (`a: Int`): The name becomes part of the contract—it may be referenced by a named
  argument (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)), and when the signature already
  declares parameter names and types, the lambda header may be omitted
  (`add: (a: Int, b: Int) -> Int = a + b`, RFC-007 §Shorthand Rules).
- **Unnamed** (a bare type such as `(Int, Int) -> Int`): Type constraints apply **positionally**,
  and the names belong to the implementation— the lambda header must provide its own parameter names
  (`mk: (Int, Int) -> Int = (x, y) => x + y`, RFC-007: a lambda header may be omitted if and only if
  the signature has declared parameter names). Such parameters cannot be referenced by named
  arguments and may only be called positionally. Bare identifiers are resolved in the type
  namespace; an unresolved identifier is an error because declared types cannot be found. There is
  no bare identifier in a parameter position that is “neither a type nor a type parameter.”

**Annotation binding semantics**: An annotation is a declaration, not a comment. Binding statements
check their implementation against the annotation—parameter types drive the lambda header
positionally, and the function body's exits (tail expression and `return`) are uniformly checked
against the annotated return type. The type registered for callers is the annotated form.
Unannotated bindings use HM inference (RFC-007). `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are rejected under the same rule—the binding semantics do not diverge
based on the syntactic form of the value (lambda / block / expression / named-function reference).

**The implementation header's parameters are positional binders**
(`f: (a: Int) -> Int = (b) => b + 1`):

- **Both names and types follow the declaration**: The i-th implementation-header parameter
  corresponds to the i-th parameter of the declaration's **value-level signature** (N = number of
  implementation-header parameters; value-level signature = the parameter sequence after removing
  compile-time positions from the declaration—`T: Type`, type-level function positions, and const
  positions). The implementation header declares only “what the i-th parameter is called inside the
  body”; renaming it does not affect callers—the names available to `f(a = 7)` come from the
  declaration, independently of the implementation header's names.
- **Annotations in the implementation header are used only for comparison**: A type at the same
  position that differs from the declaration reports **E1011** (the declaration is authoritative;
  the author's annotation must not be silently overridden).
- **More implementation-header parameters than runtime parameter positions in the declaration**
  report **E1112**: The extra positions have no values to bind (type positions `T: Type`, type-level
  function positions, and const positions do not occupy runtime parameter positions).
- **The reverse case (the implementation omits parameters) is not an error** (this was the “pass”
  cell in the t1 behavior matrix before D54 and must not be reclassified): at the type level, arity
  is still checked according to the **declaration** (`f(1)` is valid and `f(1, 2)` reports E1010),
  and missing positions are filled positionally by the declaration into the **type-checking scope**;
  however, the implementation header has no corresponding runtime slots, and arguments at those
  positions are discarded on the IR side and do not participate in computation inside the body
  (`f(1)` for `f: (a: Int) -> Int = () => 1` is unrelated to the argument). This has been **recorded
  as a gap**—making the runtime also receive the declared arity is a D-class behavior change and
  requires a separate decision.
- **Uniform exits**: Whenever the tail expression or `return` (the arrow sugar `=> x` and explicit
  `return x` are two AST forms) does not match the declared return position, **E1012** is reported,
  with the location pointing to the **annotation**.

> **Implementation status**: The binding-period comparison for `Fn` annotations has been implemented
> (WBS 6.8.2, 2026-11-02): positional parameter alignment (parser and typecheck use **the same
> alignment rule**: the first N positions of the value-level signature) / implementation-header
> annotation comparison (E1011, the author's annotation is retained and an inconsistency with the
> declaration is an error) / uniform exit checking (E1012) / registration form taken from the
> declaration (named-argument names).

---

## Chapter 4: Generics

### 4.1 Generic Parameter Syntax

Generic parameters are part of a function type and use the same `()` syntax as ordinary parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` indicates the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives. They receive the same
treatment as user-defined generics and are handled through a unified generic-instantiation path. The
ownership of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                                             | Backing Implementation                       |
| ------------- | ------------- | ----------------------------------------------------- | -------------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)                  | Core primitive (stack/inline preferred)      |
| `Vec(T)`      | Runtime value | Growable raw buffer with runtime length               | Core primitive (contiguous heap buffer)      |
| `List(T)`     | Runtime value | Growable-list annotation under the source name `List` | Same as `Vec(T)` (source-name normalization) |
| `Dict(K, V)`  | Runtime value | Key-value mapping                                     | `HeapValue::Dict`                            |

> `List(T)` is not a type independent of `Vec(T)`: since commit 22a67d93, `std/list.yx` is
> implemented entirely on `Vec(T)` and no longer defines a `List` record; the source name `List` is
> normalized into the type domain and lands on the same `Vec` type. All growable-semantics policies
> (when to grow, by how much, and whether sharing is possible) live in the library, not in the
> compiler. `Vec(T)` is the minimal foundational primitive.
>
> Set(T) has been removed: there is no literal, no runtime representation, and no std.set. If the
> requirement arises, implement it following the Dict pattern.

Key rules:

- **The landing point of a literal is determined by context**: a bare `[...]` literal and a
  `List(T)` annotation land on the growable-list type—the growable-list type surface has the single
  name `Vec(T)`, and a `List(T)` annotation reaches the same type surface through source-name
  normalization; an `Array(T, N)` annotation applied directly to a literal lands on a fixed-length
  array. Landing validation: the element count must equal N and the element types must be compatible
  with T; otherwise E1002 is reported at compile time. When N is a symbolic constant (a const
  parameter), count validation is deferred to the refined-type stage. User-defined records named
  `List` are no longer expanded through a `List(T)` annotation—the annotation always lands on the
  `Vec` surface, and field access on such a record produces the formal diagnostic (E1053).
- **Implicit List→Array conversion is prohibited**: Fixed length is guaranteed by the type
  layer—push accepts only a `Vec(A)` receiver.
- **Performance hierarchy**: From strongest to weakest performance and from least to most
  flexibility: `Array` > `Vec`.
- **Index-failure contract** (runtime errors are a transitional state; the target state is
  compile-time refinement through value-dependent types, see §8.5):
  - Out-of-bounds index (including a negative index) → `E6003`
  - Missing Dict key → `E6008`
- **String indexing convention** (#385): `s[i]` indexes a character by **Unicode scalar value (code
  point)** and produces a single-character `String`—the same unit used by `len` / `index_of` /
  `substring` / `chars` / `char_code` in `std.string`, so combined operations do not drift out of
  alignment; out-of-bounds indices (including negative indices) follow the index-failure contract
  above, `E6003`.
- **Membership `in` predicate**: Returns `Bool` without error, and its right operand may be a
  List/Array/Dict(key)/Tuple/String/Range. It is a first-class HORN predicate and forms the basis of
  propositions provable at compile time by refined types.`

In generic functions, type parameters are likewise declared in the signature, and the compiler
infers them automatically from arguments:

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

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation—the
receiver type must implement the `Try` interface (four members: `is_failure` / `success` /
`residual` / `from_error`). On failure, the current function returns early with the result of
`from_error(residual(t))`; on success, the expression's value is `success(t)`. The enclosing
function's return type must also implement `Try`, and its failure-residual type must match the
receiver's (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try`
implementations through `std.result` / `std.option` (`Option`'s failure-residual type is `Void`); a
user-defined type can opt into `?` by writing `Try(Self, T, E)` in its type body. The lowering of
`?` makes no distinction between built-in and user-defined types—the same interface and the same
path are used.

The **variant set of a sum type enters the checker through its definition or a `use` import**:
`Result`/`Option` require `use std.result` / `use std.option` before use (whole-module and grouped
`use std.{...}` imports have equal authority), and variant construction (in type-qualified form),
match-variant destructuring, and exhaustiveness checking all use this registration as their sole
criterion; a native function signature returning `Result(Float, Error)` creates a `Generic` with the
same name and identity as the sum type in `std.result`, and after `use` it can be matched and
destructured.

### 4.3 Generic Constructor Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each **data
field** corresponds to one constructor argument, and the field name is the argument name; fields
with default values may be omitted during construction, while fields without default values are
required. Function-type fields (methods) do not generate constructor arguments. Evidence fields
(§8.3) do not generate constructor arguments—their evidence is verified by the compiler at the
construction site and is not passed through an argument position.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value → constructor argument is required
    extra: T,
}
// Fully expanded form generated automatically (the compiler's internal view; users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }
// The expansion lists only data fields; evidence fields do not generate constructor arguments (§3.4)

// Calls: invoke the automatically generated constructor
c  = Container(42, 43)            // Constructor arguments are filled in field order; T is inferred from the element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type arguments + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // Named fields, in any order
c5 = Container(Int)()             // Empty construction: fields use default/zero values (data assigned afterward)

// Field default values → constructor arguments may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses; matching proceeds positionally against the declared parameters,
from left to right):

1. Arguments are matched positionally against the type declaration's parameters: a `Type` position
   accepts a type argument, while a compile-time value parameter position (such as `Int`) accepts a
   compile-time constant.
2. If a compile-time value parameter position matches successfully (partial match), the call is
   processed as a type construction: every parameter position is checked, and errors are reported in
   declaration order, **with the first missing or mismatched parameter reported first**.
3. If the arguments do not correspond at all to the declared parameters (all are values and no
   compile-time value parameter position can match), the call is processed as a constructor call:
   positional arguments fill fields in order, and type parameters are inferred automatically from
   element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),       // Evidence field: Assert is a proof-space family (§8.3)
    data: Array(Array(T, Cols), Rows),    // Data field: Array is a data-type family
}

m: Matrix(Int, 3, 4)              // Type position: one layer of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two layers: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned afterward)

Matrix(42)    // ❌ Position 0: T←42 does not match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols is missing → report the first error first: T expects Type, found 42
Container(42) // ❌ Missing constructor argument extra
Container(42, 43, 44)  // ❌ Too many constructor arguments
```

**Type inference**: The type parameters of a generic type constructor are inferred automatically
from constructor-argument elements (`Container(42, 43)` → T=Int); the type parameters of a generic
function are inferred automatically from argument types (`map(numbers, f)` → T=Int, R=String, see
§4.1). Explicit arguments are required when inference is impossible.

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

> **Constraint-solving sources (RFC-011b)**: Operator constraint names (`Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) are solved by consulting the
> interface-implementation registry— `T: Add` ≜ a registered `Add(T, T, T)` instance; `Equal`
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

## Chapter 6: Associated Types

### 6.1 Associated Type Definitions

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (using record-type syntax)
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
// More complex associated types
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // Associated types may also be generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant (candidate)
```

> The criterion is **being referenced in a type position**, rather than “having a concrete type
> annotation”: in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters
> (neither appears in any type position).

**Terminology**: Whether a generic parameter annotated with a concrete non-`Type` type (such as
`Int`) is compile-time depends on whether its value is referenced in a type position (value
dependency). **No `const` keyword is required** (the implementation internally used to call these
“const generics,” but the documentation consistently uses “compile-time value parameter”).

> **Design basis**:
> [RFC-011 §4.1 Compile-Time Value Parameters](../../rfc/accepted/011-generic-type-system.md).

Classification is driven by **strength declarations locally readable within a signature**: when the
author writes a reference to a value parameter in a type position (`(k: N)`, `Array(T, N)`,
`Assert(N > 0)`), that reference is a strength declaration; the **quantity (q) is its mechanical
reading**. Each binding infers a quantity `q ∈ {0, 1, ω}`: q=0 means compile-time computation,
runtime erasure, and no slot occupation; q=1 means linear (Move); q=ω means unrestricted (Dup)—see
Chapter 11 for 1/ω. This is not implicit: a value name appearing in a type position is a declaration
explicitly written by the author, and is the same action as writing `b: Positive(b)` (§8.4).

A **“type position”** means all `Type` subtrees in a declaration—the annotated types of parameters,
the function's return type, field types in type bodies, the types of inner `Fn`s, `Assert`
predicates, and argument positions of type constructions such as `Array(T, N)`. Classification
traverses **all** `Type` subtrees without making distinctions by position; missing even one
subtree—for example, the return type—causes a compile-time value parameter to be misclassified as a
runtime value parameter.

Whether something is **“referenced”** is determined through namespace resolution, not literal name
comparison:

1. The identifier is first resolved in the **type namespace**—declared type names and type
   parameters (`T: Type`).
2. If it resolves to a declared type, it is unrelated to the parameter. A parameter name identical
   to a type name **does not constitute shadowing**: the value and type namespaces do not consult
   the same table (see the scope and shadowing rules in [modules](modules.md)); the two `Int` values
   in `(Int: Int) -> Int` belong to two different namespaces.
3. If it does not resolve and the declaration has a parameter with the same name, the strength
   declaration matches and that parameter has q=0.
4. If neither resolution succeeds, an error is reported: there is no bare identifier in a type
   position that is “neither a type nor a type parameter.”

Self-reference is handled with a monotone fixed point (the set only grows, for at most |candidates|
rounds).

**Slot allocation uses q as its sole input**, and the call-site argument sequence and
implementation-header binding sequence are two **derived views** of the same function:

- **Call-site argument view** = parameters − type parameters (in declaration order). A q=0 parameter
  still occupies one argument position: in `f(42)(5)`, 42 is supplied for N and 5 for n; argument
  positions carry “how many values the author supplied in declaration order.”
- **Implementation-header binding view** = parameters − type parameters − q=0 parameters (in
  declaration order). The implementation header binds only names for which runtime values exist:
  `= (n) => n` is valid, whereas `= (N, n) => n` reports E1112.

Both E1011 and E1112 count according to the **implementation-header binding view**.

| Form                                                       | Classification                      | Reason                                                                    |
| ---------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | They appear only in value positions, with no strength declaration         |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N appears in a type-construction argument position                        |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N serves as the type of the inner parameter k                             |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N is an ordinary parameter          | N is not referenced in the type body, so there is no strength declaration |

**Core design**: `(N: Int)` compile-time value parameters and `(k: N)` value parameters distinguish
compile-time constants from runtime values. **The caller's obligations are checked according to the
declared strength**: the validity of `f(y)` depends on whether the argument y carries compile-time
evidence (a literal, value environment, single-value pinning, a Γ equation), not on a global
analysis of f's declaration. If there is no evidence, the error is reported at the call site, with
the location pointing to the q=0 slot visible in f's type. An unmatched candidate (one whose form
makes it a candidate but which is not referenced in a type position) is an ordinary parameter
without a strength declaration and is handled normally as q=1/ω, without a separate diagnostic code.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (the Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in a type-construction argument position → compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated in a type position (at compile time), and the result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // The compiler computes factorial(5) = 120 at compile time

// Value dependency: N serves as the type of the inner parameter k
// N is a compile-time value parameter (it appears in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (a single-value type).
// Single-value pinning: k's value is fixed by its single-value type and is known at compile time,
// so k - 1 carries compile-time evidence and can be supplied in the recursive call's N position.
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 -> 1,
        _ -> k * factorial(k - 1)(k - 1)
    }
}
```

### 7.2 Compile-Time Constant Arrays

```yaoxiang
// Use with matrix types
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// Compile-time dimension validation
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## Chapter 8: Conditional Types and Dependent Records

### 8.1 If Conditional Type

```
IfType        ::= 'If' '(' BoolExpr ',' TypeExpr ',' TypeExpr ')'
```

```yaoxiang
// Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True -> T,
    False -> E
}

// Example: compile-time branching
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue bridge and Assert refined type (see §8.4 for details)
IsTrue: (b: Bool) -> Type = match b {
    true -> Void,       // ⊤, execution continues
    false -> Never,     // ⊥, divergence/compilation error
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 Type Families

```yaoxiang
// Compile-time type conversion
AsString: (T: Type) -> Type = match T {
    Int -> String,
    Float -> String,
    Bool -> String,
    _ -> String
}
```

### 8.3 Dependent Record Σ: Evidence Fields

A **dependent record Σ** is a record type in which “later fields may reference earlier fields.” Σ's
fields fall into two categories:

- **Data fields**: Fields whose types land in the data space. They occupy runtime data slots and
  participate in construction and access like ordinary fields.
- **Evidence fields**: Fields whose types land in the **proof space** (propositions)—in other words,
  “an assertion awaiting proof.” They are not runtime data but compile-time evidence: when
  constructing the record, the compiler must provide a certificate verified by the proof kernel.

The sole classification criterion is the **proof-space criterion**: normalize the field type and
take the space registered for its **result type family** under the current logical signature—`Proof`
(proposition / refinement / proof-family result) or `Data` (runtime data). Landing in `Proof` ⇒
enter evidence classification; landing in `Data` ⇒ data field, **regardless of how many preceding
fields it references**. The space is provided by the logical signature's sort registration and the
library manifest, not guessed from field names, argument forms, or predicate-like syntax.

This section calls them evidence fields; diagnostic messages call them witness fields (E1114 /
E1115). Both terms refer to the same category.

> **Design basis**: [RFC-027 §14](../../rfc/accepted/027-compile-time-evaluation-types.md) (complete
> semantics of dependent record Σ),
> [RFC-027 §4.1](../../rfc/accepted/027-compile-time-evaluation-types.md) (sort registration for
> logical signatures).
>
> **Implementation status**: Field classification for non-generic records (proof space → position →
> category), construction-site proof obligations (evidence synthesized automatically and submitted
> to the sole decision unit; the current decision tiers are direct evaluation / exact Γ / proof
> functions, with SMT not serving as a proof source), propagation of quantifiers to IR through the
> record model, and user-reachable E4018 / E2031 / E1114 / E1115 have been implemented (WBS 6.9.5
> B.2a); generic records, const parameters, and instantiation classification for proof families have
> also been implemented (B.2b). Quantity representation, runtime erasure, and the 5A criterion
> itself belong to an earlier round (WBS 6.9.5 B.1). Type-level `b.w` (B.3) has been implemented
> (admission form, D2 decision A); cross-module export (B.4) has been implemented (V1
> re-verification only + per-library fingerprint comparison; translation/bridging awaits the §4.1
> kernel). The `evidence_ref` / `certificate_ref` slots **are not added with B.3**; B.4 follows D1
> decision A and **still does not materialize the two slots** (there is no producer; awaiting the
> §4.1 kernel round); B.2's explicit evidence entry point is a proof function in a constraint, not a
> constructor argument. Illegal declaration positions reuse E1092 / E1042 (D1 decision).

#### 8.3.1 Field-Position Rules: Proof-Space Criterion

Suppose the fields of record `R`, in source order, are `f₀, f₁, …, fₙ₋₁`, and field `fᵢ` has type
`Tᵢ`. Classification has three steps: **first determine the space (proposition family or data
family), then determine the argument position (preceding / self / forward), and finally determine
the field category**.

**Step 1: Determine the space.** Normalize `Tᵢ` and take the space registered for its result type
family under the current logical signature:

- If the space is `Proof` (proposition / refined type / `Assert` / `Terminates` / proof-family
  result), enter evidence classification.
- If the space is `Data` (`Int`, `Array`, `Vec`, an ordinary record, or the **application of a
  data-type family**), it is a **data field**, regardless of how many preceding fields it
  references.

The `n` in `data: Array(Int, n)` references a preceding data field and changes the “data slot's
length”; it does not “generate a proof.” Classifying it as evidence would erase the buffer from the
runtime layout—the misclassification this rule is intended to eliminate.

**Step 2: Determine the position** (this applies only when the space is `Proof`). There are three
cases based on where the arguments point:

- All arguments are **strictly preceding data fields** `fₕ (h < i)`, compile-time constants,
  compile-time value parameters, or type parameters ⇒ Step 3 classifies the field as evidence (a
  compile-time value parameter obtains a concrete constant after instantiation and is counted as a
  compile-time constant; §8.3.3).
- The argument **is `fᵢ` itself** (a self-referential refinement such as `v: Pos(v)`) ⇒ Step 3
  classifies it as a **data field (admission)**: the field is the variable carrying the value, and
  the constraint merely checks that value rather than proving a fact elsewhere.
- A forward reference to a later field, a reference to another evidence field, or using `fᵢ` itself
  as an evidence argument while mixing it with other fields (SelfMixed) ⇒ rejected at declaration
  time.

**Step 3: Determine the category.**

| #   | Form                                                                                                                                                                                              | Classification                                                      | Runtime Layout                                                                                                           |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| 1   | Space `Proof`; all arguments are strictly preceding data fields / compile-time constants / compile-time value parameters / type parameters (positive example `w: Pos(v)`)                         | **Evidence field**                                                  | Occupies no slot; `base` does not enter the runtime layout                                                               |
| 2   | Space `Data`: ordinary value type (such as `n: Int`), data-type-family application (positive example `data: Array(Int, n)`), or ordinary record                                                   | **Data field**                                                      | Occupies a data slot                                                                                                     |
| 3   | Space `Proof`, but the argument is `fᵢ` itself (self-referential refinement `v: Pos(v)`)                                                                                                          | **Data field (admission)**                                          | Occupies a data slot; the constraint is checked at the construction site under the existing refinement obligation (§8.4) |
| 4   | Space `Proof`, but there is a forward reference to a later field, a reference to another evidence field, or `fᵢ` itself is used as an evidence argument while mixed with other fields (SelfMixed) | **Illegal**: rejected at declaration time, with no silent downgrade | Produces no field                                                                                                        |

The four rows do not overlap: row 2 means “ordinary value type / data family” is always a data
field; row 3 means “proposition family but self-referential” is an admission data field; row 4 means
“proposition family but illegal position” is always rejected at declaration time. **There is no path
in which “a forward reference also counts as a data field”**—a forward reference occupies neither a
data slot nor a runtime data index.

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }     // Space Proof

Box: Type = {                         // Positive example: w is evidence
    v: Int,          // Data field
    w: Pos(v),       // Evidence: Pos is a Proof family, and v is a strictly preceding data field
    z: Int,          // Data field; runtime index = 1 (skipping w)
}

Buf: Type = {                         // Negative example: data is still a data field
    n: Int,                 // Data field
    data: Array(Int, n),    // Data field: Array is a Data family; n only determines the length and does not generate a proof
}

Admission: Type = {                   // Self-referential refinement: data field, not evidence
    v: Pos(v),           // Proof family, but the argument is itself ⇒ admission (row 3 of the table)
}
```

**Evidence fields may depend only on strictly preceding data fields.** Their arguments must not
reference another evidence field, a later field, or themselves. This constraint guarantees that,
when the construction site reaches the field in declaration order, all arguments already have
compile-time values, making the proposition a closed term that can be verified by the proof kernel.

**`base` does not enter the runtime layout.** The base type of an evidence field (the `base` of a
refined type) is type information used only for kernel equality checks and normalization, and no
runtime slot is ever allocated for that field. Evidence fields also have no default values—they have
no runtime value to supply.

#### 8.3.2 Construction-Site Obligations and Proofs

**There is only one source-level construction form: data arguments.** Evidence fields **occupy no
constructor argument position**—there is neither a form that “passes a witness as a value” nor an
“evidence argument position.” Constructor arity counts only data fields: **compile-time arity =
runtime arity = number of data fields**.

**Evidence is not a constructor argument; it comes from a proof obligation automatically generated
at the construction site.** When constructing a record, the compiler generates one proof obligation
for each evidence field:

1. Take the compile-time value of the strictly preceding data fields referenced by that field's
   arguments.
2. Substitute those values into the field's proposition to obtain a closed proposition (`Pos(3)`
   reduces to `3 > 0`).
3. Submit it to the proof kernel for verification under the current logical signature (RFC-027 §4).

The decision has three outcomes:

| Decision Result | Compile-Time Behavior                                                                      |
| --------------- | ------------------------------------------------------------------------------------------ |
| Proved          | Passes; the certificate is retained at compile time and erased at runtime                  |
| Disproved       | Compilation error + counterexample (**E4018**)                                             |
| Unproven        | Compilation error + unproved proposition + reason + evidence-supplement prompt (**E2031**) |

**Unproven does not mean that the proposition is false**, nor does it mean that the program is
allowed to proceed. Evidence is supplied through an **independent declaration**, not by rewriting
the construction expression: write an **independent proof function** whose return type equals the
proposition, or register a **named proof** in a proof library, and reference it from the
construction obligation (RFC-027 §4.3). There is no form that “fills in an incorrect witness
value”—evidence fields have no argument positions at construction sites. Supplying one extra
evidence argument at source level (`Box(3, 5, evidence)`) cannot reach any data-parameter slot and
produces an E1010 / E1014-family argument error; it is not silently ignored.

A self-referential refinement field (`v: Pos(v)`) is a data field: its constraint is checked at the
construction site under the existing refinement obligation (§8.4), produces no evidence obligation,
and does not change the field's runtime index.

#### 8.3.3 Generic Records: Proof-Family Parameter `P: T -> Proposition`

A generic record may carry a **proof-family parameter** `P`, whose kind is `T -> Proposition`: it
accepts a type parameter and returns a proposition. `Proposition` is an alias for the `Proof` sort
in the logical signature (RFC-027 §4.1), not a source-level type name; its source-level form is the
same as an ordinary type-level function parameter, and the kind constraint is checked at declaration
and instantiation:

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Cell: (T: Type, P: (T: Type) -> Type) -> Type = {
    v: T,        // Data field
    w: P(v),     // Evidence: P is a Proof family, and v is a strictly preceding data field
}
```

- `P`'s kind comes from the same source as the criterion in §8.3.1: **the result space must be
  `Proof`**. Passing a data-type family (such as `Array`) to `P` violates the kind and is rejected
  directly at declaration or instantiation—this is a type error, not a case where “`w` degenerates
  into a data field.”
- `P` is a compile-time parameter position and occupies no runtime slot. When instantiating
  `Cell(Int, Pos)`, `P` is bound to the proof family `Pos`, the proposition for `w` becomes
  `Pos(v)`, and evidence classification is exactly the same as in §8.3.1.
- Classification is rerun **after instantiation**.
- When `P` is unbound (the generic is not instantiated), the result space for `w` is still `Proof`
  (the kind has been declared), but the proposition is an open term, and the construction site
  returns Unproven—it is neither silently allowed nor downgraded to a data field.

#### 8.3.4 Cross-Module Export

When a record containing evidence fields is used across modules, its propositions and evidence are
exported together with the type definition, and the importer does not re-infer them from field
names. The export payload includes: the kinds of type-family parameters (including proof-family
parameters), the proof obligation for each evidence field, and the evidence slot for each evidence
field—the proposition, explicit evidence source or verified certificate, and the hash of the logical
signature that produced it.

The importer verifies these propositions using **its own current logical signature**: if the
signature hashes differ, RFC-027 §4.1's three explicit paths apply—translation items, bridge axioms,
or re-proving under the importer's signature. The importer must not directly reuse the exporter's
conclusions.

Referenced proof-library entries must appear in the importer's library list; otherwise the
conclusion is Unproven, and the diagnostic prompts the user to register the library or write a
bridge axiom.

> **Implementation status (V1)**: The current implementation performs only importer-side
> re-verification and per-library source-fingerprint comparison; translation items and bridge axioms
> await RFC-027 §4.1's kernel; the `evidence_ref`/`certificate_ref` slots are not materialized (D1
> A), and `signature_hash` is the export-side evidence-payload fingerprint.

#### 8.3.5 Type-Level Use and Value-Level Rejection

Evidence fields **may be used only at the type level**. A `b.w` in a type position is
**user-writable type-level syntax** that denotes the field's **evidence item** (proof item), not a
runtime value: it points to the field's registered evidence reference and may serve as a premise for
another proposition. V1's semantics are **admission** (the evidence slot's proposition was proved at
the construction site; see 6.9.5-B.3 §5), not replay under the logical signature (§4.2 is not
implemented).

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Box: Type = {
    v: Int,
    w: Pos(v),
}

// Type-level use: treat bb.w as an evidence premise, not as a value
Holds: (bb: Box) -> Type = { bb.w }
needW: (b: Box, h: Holds(b)) -> Int = { b.v }
```

**Value-level reads of evidence fields are always rejected**: reading `b.w` in a value expression
reports **E1114** (`witness_field_not_readable`). The field path is not a value term, and neither
the evaluator nor the runtime representation materializes a value for it. If runtime data is needed,
define the field as an ordinary data field.

```yaoxiang
b = Box(3)
x = b.w        // ❌ Value-level read of a witness field → E1114
```

> **Design basis**: [RFC-027 §14.5](../../rfc/accepted/027-compile-time-evaluation-types.md) (term
> form for type-level use, and replacement of the 4A “no user syntax at the type level” provision).
> The syntax and parsing of type-level field paths have been implemented (WBS 6.9.5 B.3): type
> positions accept a single-level `ident.ident`; value-level reads still report E1114.

#### 8.3.6 Immutability, Erasure, and Diagnostic Codes

**5A: A record containing evidence fields is immutable as a whole.** Data fields must not declare
`mut`, must not be assigned, must not be borrowed through `&mut`, and must not be used as the
receiver of a mutating method (including automatic mutable borrows). Otherwise, the evidence's
proposition might become invalid after a field is rewritten. Violations report **E1115**
(`witness_record_immutable`). Records without evidence fields are not subject to this restriction.

**Erasure.** Evidence fields occupy no runtime slots, and the runtime layout contains only data
fields; proof certificates are retained at compile time and completely erased at runtime (RFC-027
§13). Runtime data indices count only data fields—the `z` in record `{ v, w, z }` has data index 1,
skipping witness field `w`. Runtime-mode Boolean checks (§8.4) are unrelated to evidence—they
inspect runtime values, not certificates.

**Division of diagnostic responsibilities** (each code handles a separate aspect and does not
replace another):

| Code      | Trigger                                                                                     | Semantics                                                                                    |
| --------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| **E4018** | An evidence proposition at a construction site is disproved                                 | A refinement predicate is violated; a counterexample is attached                             |
| **E2031** | An evidence proposition at a construction site is Unproven (through the refinement channel) | The refinement constraint cannot be proved; the user is prompted to supply explicit evidence |
| **E1114** | Value-level read of an evidence field                                                       | Evidence fields cannot be read at the value level                                            |
| **E1115** | Mutable write to an evidence record                                                         | The record is immutable as a whole                                                           |
| **E4023** | General Unproven outside the refinement channel                                             | A general inability to prove; the proposition, reason, and location are attached             |

**Declaration-time rejection list** (consistent item by item with the four-category table in
[RFC-027 §14.6](../../rfc/accepted/027-compile-time-evaluation-types.md)): when a field whose space
is `Proof` has one of the following forms, it is rejected directly at its declaration. It does not
proceed to the construction site and is not silently downgraded to a data field:

| #   | Rejected Form                                                                                                 | Description                                                                                                      |
| --- | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| 1   | Forward reference to a later field `fₕ (h > i)`                                                               | Illegal position                                                                                                 |
| 2   | Reference to another evidence field as an argument                                                            | Illegal position                                                                                                 |
| 3   | Use of `fᵢ` itself as an evidence argument while mixing it with other fields (SelfMixed, such as `P(fᵢ, fₕ)`) | Illegal position; pure self-reference `P(fᵢ)` is the admission data field in row 3 of §8.3.1 and is not rejected |
| 4   | The kind of proof-family parameter `P` is a data-type family (§8.3.3)                                         | Kind mismatch; a type error at declaration/instantiation                                                         |

**Evidence fields must not have default values** (`w: Pos(v) = expr`, DefaultOnEvidence): evidence
fields have no runtime value to supply, and the constructor view has no argument position for them,
so specifying a default is rejected at declaration time. This is an independent field-declaration
check, is not merged with the four categories above, and is not silently treated as “a data field
with a default value”; `StructFieldInfo.has_default` is always `false` for witnesses. The
default-value rules in §3.1.1 (fields with defaults may be omitted during construction; fields
without defaults are required) apply only to data fields.

Applications of data-type families whose space is `Data`, and pure self-referential refinements
whose space is `Proof` (`v: Pos(v)`, admission), are legal data fields and do not constitute illegal
positions. When a generic has not been instantiated, the proposition is an open term and the
construction site returns Unproven (§8.3.3), not a declaration-time rejection.

### 8.4 Assert Refined Types and assert Assertions

`assert` and `Assert` are two sides of the same refinement primitive—the dispatch pipeline
automatically selects one according to whether the predicate's free variables are accessible at
compile time.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                            |
| ----------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enters the proof pipeline: true → erased to Void; false → compilation error (Never is uninhabited)  |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Inserts a runtime Bool check and injects a refinement fact into the flow-sensitive assumption set Γ |

Dispatch is an instance of **checking according to declared strength** (§7.1) at the predicate
position. The same principle covers the entire strength ladder: classic types without strength
declarations do not enter the proof pipeline and do not undergo quantity inference; predicate
annotations (this section), q=0 slots (§7.1), dependent-record fields (§8.3), and relational
predicates each perform the corresponding checks according to their declared form. Cost follows
declaration strength rather than being determined by global analysis.

**Flow-sensitive assumption set Γ**:

The compiler maintains the set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: the old assumption is invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (kill set).
At branch confluence, Γ is the intersection of the branch sets.

### 8.5 Terminates: Termination-Measure Predicates

`Terminates` is a **built-in predicate**, belonging to the same core primitives as `Int` and `Never`
(a built-in name, not a keyword). It binds a **measure** to a computation, declares that the
computation terminates, and supplies a witness of termination.

**Forms**: two arities using the same predicate:

| Form                    | Anchor                             | Use                                                                                                                                            |
| ----------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The name of the containing binding | Default form—self-recursive functions and loops                                                                                                |
| `Terminates(FnType, m)` | Explicit function type             | Used when the measure's ownership must be specified explicitly (the measure is defined elsewhere, or one measure serves multiple computations) |

```yaoxiang
// Measure: an ordinary function; independently testable and reusable; not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere, with explicit ownership
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, and the measure is an expression in scope
// A loop body is a `{}` block whose value is given by its tail expression (spec §2.9): the body has a value only if its tail contains an expression
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

**Semantics**: The `Terminates(m)` refinement applies to the value type of the computation annotated
at that type position. The obligation falls on that computation—for functions, on every recursive
call site; for loops, on the back edge—in both cases requiring that “the next state's measure is
strictly less than the current state's measure,” checked under that point's path guard.

The call-site obligation is `m(callee_args) < m(caller_args)`; the back-edge obligation is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: A loop is an anonymous construct and usually cannot be referred to. Its
> binding name is its name—the `acc` in `acc: Terminates(n - i) = while ...` provides the anchor,
> making the loop referable. This is why `Terminates`' unary form applies to loops.

**Measure**: The return type is unrestricted (it need not be a natural number); “strict decrease” is
determined by a well-founded order available on that type. Whether the measure is well-founded (for
example, whether a returned `Int` is always `>= 0`) is an **independent obligation**, determined by
the compile-time proof pipeline just like the decrease obligation.

**Trigger**: Termination checking is triggered by **refined types**—once a type is refined, it
enters verification mode. Ordinary types that have not been refined (such as a bare `while` loop or
a function without a refined signature) do not enter verification mode and generate no termination
obligation.

**Automatic exploration first**: The compiler first attempts to discover a measure automatically
(linear rank functions, predicate-violation counts, bounded increment/decrement, and multiplicative
scaling—four templates); an explicit `Terminates` is needed only when automatic exploration fails.

**Runtime representation**: A purely compile-time entity, erased along with the witness and never
included in the runtime binary.

> See [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../rfc/accepted/027a-termination-explicit-measure.md) (implementation mechanism) for
> the complete design.

---

## Chapter 9: Type Unions and Intersections

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
// Basic specialization: use function overloading (the compiler selects automatically)
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

## Chapter 11: Type Attributes

YaoXiang distinguishes only one kind of type attribute: linear versus duplicable. It is inferred
automatically by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, argument passing, and returning transfer
ownership.

```yaoxiang
// A struct containing a heap-buffer field does not derive Dup (the field lands in Move) → Move by default
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move; b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle, Share the Data)

**The Dup attribute is used for reference/token types and handle-like value types with internal
reference counting.** Assigning a Dup type performs a shallow copy—copying the handle/token while
sharing the underlying data. Multiple holders point to the same data.

| Type            | Attribute | Description                                                                                |
| --------------- | --------- | ------------------------------------------------------------------------------------------ |
| `&T`            | Dup       | Zero-sized read token; copying the token creates multiple views of the same data           |
| `ref T`         | Dup       | Rc/Arc copy = reference count + 1, sharing heap data                                       |
| String, Bytes   | Dup       | Internal reference counting; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear    | Zero-sized write token; exclusive and non-copyable                                         |
| struct          | Derived   | Dup if all fields are copyable (primitive value types ∪ Dup); otherwise Move (#398)        |
| tuple           | Derived   | Element-by-element decision, following the struct rule (#398)                              |
| All other types | Move      | Default ownership transfer                                                                 |

**Primitive value types** (Int, Float, Bool, Char) receive special treatment from the compiler:
assignment automatically copies the value, producing two fully independent values. This is native
compiler behavior and is not part of the Dup type attribute.

**Derivation rules** (finalized in #398)—the formulation “automatically derive Dup when all fields
are Dup” cannot be applied literally: primitive fields (such as Int) are not themselves Dup, so
`{ x: Int, y: Int }` would be misclassified as Move. The executable form is:

1. **Copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), and composite types already classified as Dup);
2. **struct**: If all fields belong to the copyable field set, derive Dup; if **any** field lands in
   Linear (`&mut T`) or Move (a nested Move struct / containers such as Vec and Dict / resources),
   the whole value remains Move (no intermediate “partially copyable” state); evidence fields have
   no runtime value and do not participate in derivation (§8.3);
3. **tuple**: Follows the same rule as struct, deciding element by element; an empty tuple (the
   unit) is `Void`;
4. **Derivation is recursive**: When a field has a named type (such as `target: Point`), its
   definition is expanded before classification; `A = { b: B }` follows B's derived result.
   Recursive aliases conservatively land in Move at the depth limit;
5. **Outside the derivation scope** (remain Move, handled separately): containers (Vec / Dict / Set
   / Option / Result / Array) and enums.

```yaoxiang
// &T: Dup; aliases may be created freely
view: &Point = &p
view2 = view     // Dup: the token is copied, and both remain valid
print(view.x)    // Available
print(view2.x)   // Available

// &mut T: Linear; non-copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup and cannot be copied
```

### 11.3 The Relationship Between Clone (Explicit Deep Copy) and Dup

**Clone** is the explicit deep-copy interface. All types may implement Clone and provide a
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

|                      | Dup                                                                  | Clone                                                    |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------- |
| **Semantics**        | Shallow copy: copies the handle/token and shares the underlying data | Deep copy: creates a complete independent copy           |
| **Invocation**       | Implicit (automatic during assignment/argument passing)              | Explicit (`.clone()`)                                    |
| **Mutation impact**  | Mutations affect one another (shared underlying data)                | Mutations do not affect one another (independent copies) |
| **Applicable types** | `&T` tokens, `ref T`                                                 | Any type implementing the Clone interface                |
| **Cost**             | Zero overhead (tokens are zero-sized types)                          | Depends on the type                                      |

**Dup does not imply Clone, and Clone does not imply Dup**—they are orthogonal concepts:

```yaoxiang
// Dup type: the token is copied and the underlying data is shared
view: &Point = &p
view2 = view        // Dup: the token is copied, and both point to the same p
print(view.x)       // Available
print(view2.x)      // Available; both observe the same data

// Primitive value type: the compiler automatically copies the value (not Dup)
x: Int = 42
y = x               // Value copy; x and y are fully independent
print(x)            // Available

// Clone: explicit deep copy that creates an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy; p remains usable

// Non-Dup type (a field lands in Move and Dup is not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field, so Dup is not derived (see the derivation rules in §11.2)
```

**Design intent**:

- Dup is used for tokens and reference types, solving the problem of “multiple views of the same
  data.”
- Clone is used where an independent copy is needed; making the invocation explicit makes the cost
  visible.
- Copying primitive value types (Int/Float/Bool/Char) is built-in compiler behavior and is not part
  of Dup.
- Custom types default to Move (zero-copy, high performance); Dup is derived automatically when all
  fields are copyable (§11.2 derivation rules).

## Chapter 12: Borrow Token Types

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

- Tokens are **ordinary types** and follow the same scope rules as all other types.
- No lifetime annotation `'a` is required.
- No dedicated borrow checker is needed—the type attributes (Dup/Linear) naturally derive
  permissions.
- They disappear completely after compilation, with zero runtime overhead.

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

// Call side: the compiler automatically chooses borrowing or Move
p = Point(1.0, 2.0)
p.print()                       // The compiler automatically creates an &Point token
p.shift(1.0, 1.0)               // The compiler automatically creates an &mut Point token
p.print()                       // OK; the previous token was released when the shift call ended

// Multiple &T tokens may coexist—Dup types allow unrestricted copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types and therefore support all ordinary type operations:

**Returning tokens**—tokens propagate along with the return value:

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

**Closures do not capture; context is fixed at the creation point**—a closure consumes only its own
parameters. When outer data is needed, currying fixes the value into the closure at the creation
point:

```yaoxiang
// ✅ Context is fixed through currying: threshold is a parameter, and gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope of its definition may have ended, so it
> must not implicitly capture outer variables. However, the scope at the call site (creation point)
> is necessarily alive, and fixing the context there as a value captured by the closure is safe.

### 12.4 Automatic Borrow Selection

At the call site, the compiler selects automatically according to the following priorities:

```
1. 如果实参后续还有使用 → 优先创建令牌（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // The parameter type of print is &Point → the compiler creates an &Point token
p.shift(1.0, 1.0)  // The parameter type of shift is &mut Point → the compiler creates an &mut Point token
p2 = p             // No subsequent use → Move
```

**Method receiver semantics follow the signature** (the same convention as RFC-011a's receiver
spelling rules): an `&T` receiver means a read-only borrow token; `&mut T` means a mutable borrow
token; by-value means Move (consumes the receiver). Borrow tokens created at the call site are
released when the call ends (transient, interval semantics in §12.5); an interface's borrow receiver
must be explicitly declared by the interface author as `&Self`, and after replacing
`Self ↦ impl type` in the impl signature, it must exactly match the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is the **borrow HORN proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and submits them to the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and a derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ The token is automatically released when its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Uses &mut Point
    }
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken remains usable
}

// ❌ The same argument cannot simultaneously produce an &mut token and another token
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Branding Mechanism

Users never interact with brands. Internally, the compiler assigns each token a unique compile-time
identifier:

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N 是编译期唯一整数
&mut Point     →  WriteToken(Point, #M)   // #M 是编译期唯一整数
```

The purposes of brands are:

- **Anti-forgery**: A token may only be obtained from its owner's capsule and cannot be fabricated
  from nothing.
- **Association tracking**: A derived `&Float` produced by field access carries a derived brand
  (`#N.field_x`), allowing the compiler to trace it back to the parent token.
- **Conflict detection**: A WriteToken and a derived ReadToken from the same source cannot be live
  simultaneously.

Brands disappear completely after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken（冻结源数据 → Dup 安全）
               | &mut T      // WriteToken（独占读写 → Linear）
```

### 12.8 Borrow Tokens vs ref

|                 | `&T` / `&mut T`                                               | `ref`                                             |
| --------------- | ------------------------------------------------------------- | ------------------------------------------------- |
| What it does    | Look at / modify in place                                     | Shared ownership                                  |
| Scope           | Follows the token value's scope                               | Crosses scopes                                    |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (selected by the compiler)              |
| Escape          | Possible (the token propagates through return values/structs) | Intended for escaping                             |
| Cross-task      | Not possible (tokens do not implement cross-task transfer)    | Possible (the compiler automatically selects Arc) |
| Cycle detection | Not involved                                                  | Silent within a task; cross-task lint             |

> Note (undefined): How to read the contents after creating a ref (dereference/method/automatic
> behavior) has not yet been defined in the specification; the current implementation reports E1052
> for `*a`. It is to be added to this section once defined.

---

## Appendix: Quick Reference for Type Definitions

### A.1 Type Definitions

```
// === Record types (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Dependent record Σ (evidence fields, see §8.3) ===

// Compile-time predicate + evidence field: w occupies neither a constructor argument position nor a runtime data slot
Pos: (v: Int) -> Type = { v > 0 }
Box: Type = { v: Int, w: Pos(v) }

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

// === Termination measure (built-in predicate, see §8.5) ===

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

// Compile-time generic: N is referenced in a type position (k: N) → compile-time value parameter
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Proof-family parameter: kind is T -> Proposition (the result space must be Proof)
Cell: (T: Type, P: (T: Type) -> Type) -> Type = { v: T, w: P(v) }

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True -> T, False -> E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Quick Reference for Type Attributes

```
// === Move (default) ===
// All types default to Move. Assignment, argument passing, and returning transfer ownership

// === Primitive value types (compiler built-ins) ===
Int, Float,     // The value is copied automatically on assignment, and the two values are fully independent
Bool, Char      // Not Dup; built-in compiler handling for primitives

// === Dup (shallow copy: copies the handle and shares the underlying data) ===
&T              // Zero-sized read token; copying the token = multiple views of the same data
ref T           // Rc/Arc copy = reference count + 1, sharing heap data
String, Bytes   // Internal reference counting; copying the handle shares the underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) → Dup (#398)

// === Not derived (remain Move) ===
Vec/Dict/Set    // Containers and enums are outside the derivation scope (#398)

// === Linear ===
&mut T          // Zero-sized write token; Linear (exclusive and non-copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Creates an independent copy; mutations do not affect the original value
```

### A.4 Quick Reference for Borrow Tokens

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token; freezes source data → Dup (copyable)
&mut T          // Zero-sized compile-time write token; exclusive read/write → Linear (non-copyable)

// Automatic selection at the call site
// 1. The argument has subsequent uses → create a token
// 2. The argument has no subsequent uses → Move
// 3. Preference order: &T < &mut T < Move

// Token propagation
// ✅ May be returned, stored in structs, or captured by closures
// ❌ May not cross tasks (tokens do not implement cross-task transfer)
```
