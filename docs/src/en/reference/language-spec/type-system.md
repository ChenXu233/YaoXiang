# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter Zero: Theoretical Foundations

### 0.1 Curry–Howard Correspondence

The Curry–Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between the type system of a programming language and mathematical
logic:

| Logic                                          | Programming Language                                    |
| ---------------------------------------------- | ------------------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                             |
| Proof \(p: P\)                                 | Program `x: T = ...`                                    |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                                |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                           |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                             |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                             |
| True \(\top\)                                  | `Void` (Unit, with a default value)                     |
| False \(\bot\)                                 | `Never` (nullary constructor, uninhabited by any value) |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell's paradox)    |
| Case analysis                                  | Type-level `match`                                      |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires a
> type-level recursive function plus a compiler termination check.

### 0.2 Types Are Propositions; Programs Are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to a correct constructive proof**. YaoXiang type
  families (such as case analysis over `Add` on `Nat` plus recursive calls) are fundamentally
  type-level encodings of mathematical induction—provided that the compiler can perform a
  termination check.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition having been constructively proved.

### 0.3 Impact on Language Design

The Curry–Howard correspondence is concretely reflected in YaoXiang as follows:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` prevents the logical paradox
   caused by `Type: Type` (Girard's paradox).
2. **Type families** (RFC-011): type-level case analysis and recursive calls over the natural-number
   type family `Nat(Zero/Succ)` correspond to Peano's axioms—provided that the compiler performs a
   termination check.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to the finite
   quantification “for every integer N, there exists a type.”

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

> **Design note**: Although RFC-010 proposes a unified “everything is assignment” model
> (`name: type = value`), types and values must still be distinguished at the syntactic level. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`). As a BNF placeholder, `TypeExpr` corresponds to the `Type` enum in the
> implementation and means “a type is expected in this position.”

---

## Chapter Two: Primitive Types

### 2.1 Primitive Types

**There are 15 built-in type names; this section is the authoritative table**: `Never` / `Void` /
`()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` /
`Char` / `String` / `Bytes`. The single source of truth in the implementation is
`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`). This table is **identical to it
name for name**—the T2 gate in `tools/type-tables --report` compares this table,
`CONST_PARAM_TYPES`, and the LSP type-name list across all three sources.

| Type      | Logical Correspondence | Description                                                                                   | Default Size |
| --------- | ---------------------- | --------------------------------------------------------------------------------------------- | ------------ |
| `Type`    | —                      | Meta type (not a value-type name; see below)                                                  | 0 bytes      |
| `Never`   | ⊥ (false/empty type)   | Nullary constructor; no values. Divergence/panic return type. `Never <: T` holds for every T. | 0 bytes      |
| `Void`    | ⊤ (true/Unit)          | Has a default void value; a zero-field product type. `x: Void = <default>` is valid.          | 0 bytes      |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                               | 1 byte       |
| `Int`     | —                      | Signed integer (= `Int64`)                                                                    | 8 bytes      |
| `Int8`    | —                      | 8-bit signed integer                                                                          | 1 byte       |
| `Int16`   | —                      | 16-bit signed integer                                                                         | 2 bytes      |
| `Int32`   | —                      | 32-bit signed integer                                                                         | 4 bytes      |
| `Int64`   | —                      | 64-bit signed integer                                                                         | 8 bytes      |
| `Float`   | —                      | Floating-point number (= `Float64`)                                                           | 8 bytes      |
| `Float32` | —                      | 32-bit floating-point number                                                                  | 4 bytes      |
| `Float64` | —                      | 64-bit floating-point number                                                                  | 8 bytes      |
| `String`  | —                      | UTF-8 string                                                                                  | Variable     |
| `Char`    | —                      | Unicode character                                                                             | 4 bytes      |
| `Bytes`   | —                      | Raw bytes                                                                                     | Variable     |

`()` is the **type notation** for `Void` (a zero-element tuple = the unit type) and is synonymous
with `Void`: standard-library signature strings write `() -> Void`, and the parser table places `()`
and `Void` in the same arm.

**There is only one spelling for each type**. `Int` is not `int`; `Int64` is not `int64`/`i64`;
`String` is not `string`/`str`; `Bool` is not `bool`; `Void` is not `void`; `Never` is not `never`;
`Float` is not `float`/`f64`; `Char` is not `char`; and `Bytes` is not `bytes`. The implementation
previously contained 22 such lowercase or abbreviated spellings, all of which were removed in **WBS
6.1.2 (finalized 2026-10-27)**: using a noncanonical spelling in a type position always reports an
unknown type and is never silently coerced to a user-defined type.

> **`DateTime` is not a built-in name**. It is a **semantic alias in the std layer**: the `now` /
> `parse_time` signature strings in `src/std/time.rs` and `src/std/interfaces/time.yx` still use
> `DateTime`, but at runtime it is `Int` (fixed together with #338). The parser table retains it
> only as a **temporary compatibility alias**, and this table does not include it; migrating it out
> of the built-in-name table and letting the std layer carry it belong to **P10 S11** (issue
> [#513](https://github.com/ChenXu233/YaoXiang/issues/513)).

> **There are no unsigned integer types**. `Uint` (and `Int128`) are not usable type
> names—`MonoType` has no unsigned variant. The LSP type-name list and the `sizeof` name table each
> previously contained one residual entry, and both were removed with WBS 6.1.2. If unsigned
> semantics are needed, use `Int` with an explicit lower-bound convention.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤), respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Nullary constructor**: no literal or expression can produce a value of type `Never`. There is
   no right-hand side to write for `x: Never = ...`.
2. **Explosion principle**: `Never <: T` holds for every type `T`. `assert(false)` returns `Never`,
   after which subsequent code can pass type checking (although it is never reached).
3. **Divergence marker**: `f: (...) -> Never` means that `f` is guaranteed not to return. The
   compiler uses this for dead-code analysis and `match` branch confluence.

`Never` is a built-in type name, registered through the same path as `Int`/`Bool`; it is not a
keyword.

**Void (⊤, true/Unit)** — exactly one inhabitant (the default void value). `Void` is the identity
element of zero-field product types. `x: Void = <default>` is valid. A block's value is provided by
its **tail expression** (an empty block `{}` has type `Void`); see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

---

## Chapter Three: Composite Types

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
- A field name is followed directly by a colon and a type.
- An interface name in the type body indicates that the type implements that interface.
- Fields are divided into data fields and evidence fields according to the **proof-space criterion**
  (§8.3): after type normalization, a field whose type falls in the proof space (`Proof`) is an
  evidence field. It does not occupy a constructor argument position or a runtime data slot. A field
  that falls in the data space (`Data`) is always a data field.

> **Namespace ownership**: A `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to the namespace of `Point`. It does not trigger any implicit binding. For `.` call syntax
> such as `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004
> and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, which may be omitted during construction:

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
- Evidence fields have no runtime value and therefore have no default value (§8.3).

#### 3.1.2 Builtin Bindings

Methods may be bound directly within a type definition:

```yaoxiang
// Method 1: bind by referencing an external function
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // Bind to position 0
}
// Call: p1.distance(p2) -> distance(p1, p2)

// Method 2: anonymous function + positional binding
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

**Direct interface assignment**: A concrete type may be assigned directly to an interface-typed
variable (structural subtyping).

```yaoxiang
// Direct assignment (the concrete type is known at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (the concrete type cannot be determined -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Look up the method through the vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inferred Result                 | Call Method                 |
| ------------------------------------ | ------------------------------- | --------------------------- |
| Direct assignment of a concrete type | Concrete type can be determined | Direct call (zero overhead) |
| Function return value                | Unknown                         | vtable                      |
| Heterogeneous collection             | Multiple types                  | vtable                      |

**Coherence and orphan rules (not applicable; closure note)**: YaoXiang interfaces are structural
types (an interface is a record whose fields are all function types), not nominal traits. Therefore,
there is no cross-crate/module “who may implement what for whom” ownership issue, and Rust-style
orphan rules and coherence checks have no applicable subject (the ruling is recorded in RFC-011
§2.1). The corresponding guarantee in a structural world is **duplicate-implementation rejection**:
defining the same method signature more than once on a type is a compile-time error (RFC-011a §3;
overriding is prohibited, while overloading is permitted).

### 3.3 Tuple Types

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.4 Constructors

Record types (§3.1) automatically generate constructors. **Constructor arguments correspond only to
data fields**—compile-time evidence fields (evidence fields, §8.3) do not occupy any constructor
argument position.

| Field                        | Constructor Argument Position                     | Runtime Data Slot |
| ---------------------------- | ------------------------------------------------- | ----------------- |
| Data field                   | Occupies one position, in field declaration order | Occupies one slot |
| evidence field               | Occupies no position                              | Occupies no slot  |
| Function-type field (method) | Occupies no position                              | Occupies no slot  |

**Compile-time arity = runtime arity = number of data fields.** Evidence is not a constructor
argument: it comes from a proof obligation automatically generated at the construction site
(§8.3.2), which the compiler synthesizes itself or verifies using an independently supplied proof
function. There is no source-level “evidence argument position.” Passing one extra argument cannot
land in any data-parameter slot and results in an E1010/E1014-family argument error; it is never
silently ignored.

Fields with default values may be omitted during construction; fields without default values are
required (§3.1.1).

**Construction sites carry proof obligations**. Each evidence field creates a proof obligation at
the construction site: substitute the compile-time values of strictly preceding data fields into the
field's proposition to obtain a closed proposition, then have the proof kernel verify it under the
current logical signature (§8.3.2). If the obligation is disproved, compilation fails and reports a
counterexample. If it cannot be proved, supply an independent proof function or a named proof from a
proof library (§8.3.2), rather than adding a constructor argument—evidence fields have no argument
positions at construction sites.

**Two views of the same construction**:

- **Compile-time view**: Data arguments are bound in declaration order; evidence fields occupy no
  argument positions, but each carries a proof obligation awaiting verification.
- **Runtime view**: Only data fields enter the runtime layout; evidence fields and their
  certificates are erased entirely (§8.3.6).

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Box: Type = {
    v: Int,          // Data field: constructor argument position 0, runtime data slot 0
    w: Pos(v),       // evidence field: Pos is a proof-space family, Pos(v) is a proposition
    z: Int,          // Data field: constructor argument position 1, runtime data slot 1
}

b = Box(3, 5)       // arity = 2 (v, z); at runtime only v=3 and z=5 exist; w is compile-time evidence
```

> **Design basis**: [RFC-027 §14.1–§14.2](../../rfc/accepted/027-compile-time-evaluation-types.md)
> (field-position rules and construction views),
> [RFC-027 §4](../../rfc/accepted/027-compile-time-evaluation-types.md) (proof kernel and logical
> signatures).
>
> **Implementation status**: Evidence classification for non-generic records (proof space → position
> → category), construction-site proof obligations, and data-field arity and quantification
> propagation through the record model to IR have been implemented (WBS 6.9.5 B.2a). Instantiation
> classification for generic records, const parameters, and proof families has also been implemented
> (B.2b): constructor arguments count only data fields, while evidence-field evidence is
> automatically synthesized and adjudicated at the construction site—a false proposition reports
> E4018, an unproven proposition reports E2031, value-level reads report E1114, and mutable writes
> report E1115. Type-level `b.w` (B.3) has been implemented (admission reading form, D2 ruling A);
> cross-module export (B.4) has been implemented (V1 re-verification only: per-library
> source-fingerprint comparison + importer-side re-verification; translation/bridging awaits the
> §4.1 kernel; the `evidence_ref`/`certificate_ref` slots are not materialized, D1 ruling A).

### 3.5 Function Types

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= Param (',' Param)*
Param       ::= Identifier ':' TypeExpr    // Named parameter
              | TypeExpr                   // Unnamed parameter (bare type)
```

**Two parameter forms, with different ownership of names** (the type-level side of RFC-007's
shorthand rules):

- **Named** (`a: Int`): The name enters the contract—it may be referenced by a named argument
  (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)), and when the signature declares parameter
  names and types, the lambda header may be omitted (`add: (a: Int, b: Int) -> Int = a + b`, RFC-007
  §Shorthand Rules).
- **Unnamed** (a bare type, such as `(Int, Int) -> Int`): Type constraints apply **by position**,
  while the names belong to the implementation—the lambda header must provide its own parameter
  names (`mk: (Int, Int) -> Int = (x, y) => x + y`, RFC-007: a lambda header may be omitted if and
  only if the signature declares parameter names). Such parameters cannot be referenced by named
  arguments and can only be called positionally. A bare identifier is resolved in the type
  namespace; if it does not resolve to a declared type, an error is reported—there is no bare
  identifier in a parameter position that is “neither a type nor a type parameter.”

**Annotation binding semantics**: An annotation is a declaration, not a comment. Binding statements
check their implementation against the annotation—the parameter types drive the lambda header
positionally, the exits of the function body (tail expression and `return`) are uniformly checked
against the annotated return type, and the type registered for callers is exactly the annotated
form. An unannotated binding uses HM inference (RFC-007). `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are rejected under the same rule—binding semantics do not diverge
according to the syntactic form of the value (lambda / block / expression / named function
reference).

**Implementation-header parameters are positional binders** (`f: (a: Int) -> Int = (b) => b + 1`):

- **Names and types follow the declaration**: The i-th implementation-header parameter corresponds
  to the i-th position in the declaration's **value-level signature's first N positions** (N = the
  number of implementation-header parameters; the value-level signature = the parameter sequence
  after removing compile-time positions from the declaration—`T: Type`, type-level function
  positions, and const positions). The implementation header only declares “what the i-th parameter
  is called in the body”; renaming it does not affect the caller—`f(a = 7)` uses a name available
  from the declaration, independently of the implementation header's name.
- **Annotations on the implementation header are used only for comparison**: A type mismatch at the
  same position as the declaration reports **E1011** (the declaration is authoritative, and the
  author's annotation must not be silently overridden).
- **An implementation header with more parameters than the declaration's runtime parameter
  positions** reports **E1112**: the extra positions have no values to bind (type positions
  `T: Type`, type-level function positions, and const positions do not occupy runtime parameter
  positions).
- **The reverse case (the implementation specifies too few parameters) is not an error** (this was
  the “pass” cell in D54's earlier t1 behavior matrix and must not be reclassified): at the type
  level, arity is still checked according to the **declaration** (`f(1)` is legal and `f(1, 2)`
  reports E1010), and missing positions are supplied positionally by the declaration into the
  **type-checking scope**. However, the implementation header has no corresponding runtime slots, so
  arguments in those positions are discarded on the IR side and do not participate in computation in
  the body (in `f: (a: Int) -> Int = () => 1`, `f(1)` is unrelated to the argument). **This has been
  registered as a gap**—making the runtime also receive the declared arity requires a D-class
  behavior change and a separate ruling.
- **Uniform exits**: Whenever the tail expression or `return` (the arrow shorthand `=> x` and
  explicit `return x` are two AST forms) does not match the declared return position, **E1012** is
  reported, with the location pointing to the **annotation**.

> **Implementation status**: Alignment of `Fn` annotations with their binding phases has been
> implemented (WBS 6.8.2, 2026-11-02): positional parameter driving (the parser and typechecker use
> **the same alignment rule**: the first N positions of the value-level signature) / comparison
> against implementation-header annotations (E1011; the author's annotation is retained and a
> mismatch with the declaration is an error) / uniform exits (E1012) / registration in the
> declaration's form (names available to named arguments).

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

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics and are handled through a unified generic-instantiation path.
Ownership of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                                         | Backing Implementation                               |
| ------------- | ------------- | ------------------------------------------------- | ---------------------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)              | Core primitive (stack/inline preferred)              |
| `Vec(T)`      | Runtime value | Runtime-length raw buffer; growable               | Core primitive (contiguous heap buffer)              |
| `List(T)`     | Runtime value | Growable-list annotation under source name `List` | Same as `Vec(T)` (source-name landing is normalized) |
| `Dict(K, V)`  | Runtime value | Key-value mapping                                 | `HeapValue::Dict`                                    |

> `List(T)` is not a type independent of `Vec(T)`: since commit 22a67d93, `std/list.yx` as a whole
> uses `Vec(T)` as its backing implementation and no longer defines a `List` record; the source name
> `List` is normalized to the `Vec` surface in the type domain. All growable-semantics policies
> (when to grow, how much to grow, and whether sharing is possible) reside in the library; the
> compiler does not participate. `Vec(T)` is the minimal foundational primitive.
>
> Set(T) has been retired: it has no literal syntax, no runtime representation, and no std.set
> implementation. When the requirement arises, it should be completed following the Dict pattern.

Key rules:

- **Literal landing is determined by context**: bare `[...]` literals and `List(T)` annotations land
  on the growable-list surface—the growable-list type surface has the single name `Vec(T)`, and
  `List(T)` annotations are normalized by source name and land on the same type surface as `Vec(T)`.
  An `Array(T, N)` annotation applied directly to a literal lands on a fixed-length array. Landing
  validation requires the element count to equal N and the element types to be compatible with T;
  nonconformance reports compile-time E1002. When N is a symbolic constant (a const parameter),
  count validation is deferred to the refinement-type phase. A user-defined record named `List` is
  no longer expanded through a `List(T)` annotation—the annotation uniformly lands on the `Vec`
  surface, and accessing the record's fields reports the formal diagnostic E1053.
- **Implicit List→Array conversion is forbidden**: fixed length is guaranteed by the type
  system—push accepts only a `Vec(A)` receiver.
- **Performance hierarchy**: performance decreases and flexibility increases from bottom to top:
  `Array` > `Vec`.
- **Index-failure contract** (runtime errors are a transitional state; the target state is
  compile-time refinement, as discussed in §8.5):
  - Index out of bounds (including a negative index) → `E6003`
  - Missing Dict key → `E6008`
- **String indexing semantics** (#385): `s[i]` indexes characters by **Unicode scalar value (code
  point)** and produces a single-character `String`. This is the same unit used by `len` /
  `index_of` / `substring` / `chars` / `char_code` in `std.string`, so combinations do not drift out
  of alignment; an out-of-bounds index (including a negative index) follows the index-failure
  contract above, `E6003`.
- **Membership `in` predicate**: returns `Bool` without error. The right operand covers
  List/Array/Dict(keys)/Tuple/String/Range. It is a first-class Horner predicate and forms the basis
  of propositions that refinement types can prove at compile time.`

In a generic function, type parameters are likewise declared in the signature, and the compiler
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
    push: (self: List(T), item: T) -> Void,   // self is merely a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation—the
receiver type must implement the `Try` interface (with the four members `is_failure` / `success` /
`residual` / `from_error`). On failure, the result of `from_error(residual(t))` returns early from
the current function; on success, the expression's value is `success(t)`. The enclosing function's
return type must also implement `Try`, and its failure-residual type must match the receiver's
(`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try` implementations through
`std.result` / `std.option` (`Option`'s failure-residual type is `Void`). A user-defined sum type
can opt into `?` by writing `Try(自身, T, E)` in its type body. The lowering of `?` makes no
distinction between built-in and user-defined types—the same interface and the same path are used.

The **variant set of a sum type enters the checker through its definition or a `use` import**:
`Result`/`Option` must `use std.result` / `use std.option` before use (whole-module and grouped
`use std.{...}` imports have equal authority). Variant construction (in type-qualified form),
match-variant destructuring, and exhaustiveness checking all use this registry as their sole
criterion; a native function signature returning `Result(Float, Error)` mints a `Generic` with the
same name, which has the same identity by name as the sum type in `std.result` and can be
destructured by match after `use`.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each **data
field** corresponds to one constructor argument, and the field name is the argument name. Fields
with default values may be omitted during construction, while fields without default values are
required. Function-type fields (methods) do not generate constructor arguments. Evidence fields
(§8.3) do not generate constructor arguments—their evidence is verified by the compiler at the
construction site and is not passed through an argument position.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default -> constructor argument is required
    extra: T,
}
// Fully expanded form (compiler's internal view; users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }
// Expansion lists data fields only; evidence fields do not generate constructor arguments (§3.4)

// Call: invoke the automatically generated constructor
c  = Container(42, 43)            // Constructor arguments follow field order; T is inferred from the elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type argument + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // Named-field form, arbitrary order
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned later)

// Field defaults -> constructor arguments may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Invocation rules** (single parentheses, matched positionally against the declared parameters from
left to right):

1. Arguments are first matched positionally against the type declaration's parameters: `Type`
   positions accept type arguments, while compile-time value parameter positions (such as `Int`)
   accept compile-time constants.
2. If a compile-time value parameter position matches successfully (a partial match), construction
   is treated as type construction: all parameter positions are checked, and on failure the **first
   mismatched or missing parameter is reported in declaration order**.
3. If the arguments do not correspond at all to the declared parameters (all are values and no
   compile-time value parameter position can match), construction is treated as a constructor
   invocation: positional arguments fill fields in field order, and type parameters are inferred
   automatically from the element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),       // evidence field: Assert is a proof-space family (§8.3)
    data: Array(Array(T, Cols), Rows),    // Data field: Array is a data-type family
}

m: Matrix(Int, 3, 4)              // Type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two levels: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ Position 0: T←42 does not match (42 is not a type); Position 1: Rows←42 matches;
              //    Position 2: Cols is missing -> report the first error first: T expects Type, found 42
Container(42) // ❌ Missing constructor argument extra
Container(42, 43, 44)  // ❌ Too many constructor arguments
```

**Type inference**: A generic type constructor's type parameters are inferred automatically from the
elements of its constructor arguments (`Container(42, 43)` → T=Int); a generic function's type
parameters are inferred automatically from the types of its arguments (`map(numbers, f)` → T=Int,
R=String; see §4.1). Explicit arguments are required when they cannot be inferred.

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

> **Constraint resolution sources (RFC-011b)**: Resolution of operator constraint names (`Add` /
> `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) means consulting the
> interface-implementation registry— `T: Add` ≜ an `Add(T, T, T)` instance has been registered.
> `Equal` additionally has structural inference (records whose fields are all comparable are
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
// Iterator trait (using record type syntax)
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

## Chapter Seven: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant (candidate)
```

> The criterion is **being referenced in a type position**, rather than “having a concrete type
> annotation”: in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters
> (neither appears in any type position).

**Terminology**: Whether a generic parameter annotated with a concrete type other than `Type` (such
as `Int`) enters the compile-time phase depends on whether its value is referenced in a type
position (value dependency). **No `const` keyword is required** (the implementation once referred to
these as “const generics,” but the documentation consistently uses “compile-time value parameter”).

> **Design basis**:
> [RFC-011 §4.1 Compile-Time Value Parameters](../../rfc/accepted/011-generic-type-system.md).

Classification is driven by **strength declarations that can be read locally within a signature**:
when the author writes a value-parameter reference in a type position (`(k: N)`, `Array(T, N)`,
`Assert(N > 0)`), that constitutes a strength declaration; the **quantity is its mechanical
reading**. Each binding infers a quantity `q ∈ {0, 1, ω}`: q=0 is compile-time computed, erased at
runtime, and occupies no slot; q=1 is linear (Move); q=ω is free (Dup)—see Chapter Eleven for 1/ω.
This is not implicit: the appearance of a value name in a type position is a declaration made
explicitly by the author, just like writing `b: Positive(b)` (§8.4).

The **“type position”** means all `Type` subtrees in a declaration—parameter annotation types,
function return types, field types in a type body, the types of inner `Fn`s, `Assert` predicates,
and argument positions in type constructions such as `Array(T, N)`. Classification traverses **all**
`Type` subtrees without case distinctions by position; missing even one subtree (for example, a
return type) would misclassify a compile-time value parameter as a runtime value parameter.

Whether a parameter is **“referenced”** is determined by namespace resolution rather than literal
name comparison:

1. The identifier is first resolved in the **type namespace**—declared type names and type
   parameters (`T: Type`);
2. If it resolves to a declared type, it is unrelated to the formal parameter. A formal-parameter
   name and type name having the same spelling **does not constitute shadowing**: the value
   namespace and type namespace do not consult the same table (see the scope and shadowing rules in
   [modules](modules.md)); the two `Int` occurrences in `(Int: Int) -> Int` belong to different
   namespaces;
3. If it does not resolve, but a formal parameter with the same name exists, the strength
   declaration is matched and that parameter has q=0;
4. If neither resolves, an error is reported: there is no bare identifier in a type position that is
   “neither a type nor a type parameter.”

Self-reference is handled by a monotone fixed point (the set only grows, with at most |candidates|
iterations).

**Slot allocation uses q as its sole input**. The call-site argument sequence and the
implementation-header binding sequence are two **derived views** of the same function:

- **Call-site argument view** = formals − type parameters (in declaration order). A q=0 parameter
  still occupies one argument position: in `f(42)(5)`, 42 is supplied to N and 5 to n. The argument
  position is the carrier of “how many values the author supplied in declaration order.”
- **Implementation-header binding view** = formals − type parameters − q=0 parameters (in
  declaration order). The implementation header binds only names for which a runtime value is
  available: `= (n) => n` is legal, whereas `= (N, n) => n` reports E1112.

Both E1011 and E1112 are counted according to the **implementation-header binding view**.

| Form                                                       | Classification                      | Reason                                                                    |
| ---------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | They appear only in value positions; no strength declaration              |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N appears in a type-construction argument position                        |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N serves as the type of the inner parameter k                             |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N is an ordinary parameter          | N is not referenced in the type body, so there is no strength declaration |

**Core design**: Use a `(N: Int)` compile-time value parameter and a `(k: N)` value parameter to
distinguish compile-time constants from runtime values. **The caller's obligation is checked
according to declaration strength**: the legality of `f(y)` depends on whether the argument y
carries compile-time evidence (a literal, a value environment, singleton pinning, a Γ equation), not
on a global analysis of f's declaration. When there is no evidence, the error is reported at the
call site and points to the q=0 slot visible in f's type. A candidate that falls through (its form
makes it a candidate, but it was not referenced in a type position) is an ordinary parameter with no
strength declaration and is handled normally as q=1/ω, with no separate diagnostic code.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in a type-construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated in a type position (compile time), and the result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // The compiler computes factorial(5) = 120 at compile time

// Value dependency: N serves as the type of the inner parameter k
// N is a compile-time value parameter (it appears in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (singleton type).
// Singleton pinning: k's value is pinned by its singleton type and is known at compile time,
// so k - 1 carries compile-time evidence and can be supplied in the N position of the recursive call.
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

## Chapter Eight: Conditional Types and Dependent Records

### 8.1 The If Conditional Type

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
    true => Void,      // ⊤, program continues
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

A **dependent record Σ** is a record type in which “later fields may reference earlier fields.” Σ's
fields fall into two categories:

- **Data fields**: Fields whose types fall in the data space. They occupy runtime data slots and
  participate in construction and access like ordinary fields.
- **evidence fields**: Fields whose types fall in the **proof space** (propositions)—in other words,
  “an assertion awaiting proof.” They are not runtime data but compile-time evidence: when
  constructing the record, the compiler must supply a certificate verified by the proof kernel.

The **sole criterion** for this division is the **proof-space criterion**: normalize the field type,
take the space registered for its **result type family** under the current logical signature, and
use `Proof` (the result of a proposition / refinement / proof family) or `Data` (runtime data). A
result in `Proof` ⇒ enter evidence classification; a result in `Data` ⇒ data field, **regardless of
how many preceding fields it references**. The space is supplied by the logical signature's sort
registry and the library manifest, not guessed from field names, argument forms, or
“predicate-looking” syntax.

This section calls them evidence fields; diagnostic messages call them witness fields (E1114 /
E1115). Both terms refer to the same category of field.

> **Design basis**: [RFC-027 §14](../../rfc/accepted/027-compile-time-evaluation-types.md) (complete
> semantics of dependent record Σ),
> [RFC-027 §4.1](../../rfc/accepted/027-compile-time-evaluation-types.md) (logical-signature sort
> registry).
>
> **Implementation status**: Field classification for non-generic records (proof space → position →
> category), construction-site proof obligations (evidence automatically synthesized and sent to the
> sole adjudication unit; the current adjudication tiers are direct evaluation / exact Γ / proof
> functions, with SMT not serving as a proof source), quantity propagation through the record model
> to IR, and user-reachable E4018 / E2031 / E1114 / E1115 diagnostics have been implemented (WBS
> 6.9.5 B.2a). Instantiation classification for generic records, const parameters, and proof
> families has also been implemented (B.2b). Quantity representation, runtime erasure, and the 5A
> criterion itself were implemented in an earlier round (WBS 6.9.5 B.1). Type-level `b.w` (B.3) has
> been implemented (admission reading form, D2 ruling A); cross-module export (B.4) has been
> implemented (V1 re-verification + per-library fingerprint comparison; translation/bridging awaits
> the §4.1 kernel). The `evidence_ref` / `certificate_ref` slots are **not added with B.3**; under
> B.4, D1 ruling A **still does not materialize the two slots** (there is no producer yet; it awaits
> the §4.1 kernel batch). The explicit evidence entry point in B.2 is a proof function in a
> constraint, not a constructor argument. Illegal declaration positions reuse E1092 / E1042 (D1
> ruling).

#### 8.3.1 Field-Position Rules: The Proof-Space Criterion

Let the fields of a record `R` be `f₀, f₁, …, fₙ₋₁` in source order, with the type of field `fᵢ`
being `Tᵢ`. Classification has three steps: **first determine the space (proposition family or data
family), then determine the argument position (preceding / self / forward), and finally determine
the field category**.

**Step 1: Determine the space.** Normalize `Tᵢ`, then take the space registered for its result type
family under the current logical signature:

- If the space is `Proof` (proposition / refinement type / `Assert` / `Terminates` / the result of a
  proof family), enter evidence classification;
- If the space is `Data` (`Int`, `Array`, `Vec`, an ordinary record, or the **application of a
  data-type family**), it is a **data field**, regardless of how many preceding fields it
  references.

The `n` in `data: Array(Int, n)` references a preceding data field and changes the “length of the
data slot”; it does not “generate a proof.” Classifying it as evidence would erase the buffer from
the runtime layout—this misclassification is precisely what the rule is intended to eliminate.

**Step 2: Determine the position** (this applies only to fields whose space is `Proof`). There are
three cases based on the positions referenced by the arguments:

- If all arguments are **strictly preceding data fields** `fₕ (h < i)`, compile-time constants,
  compile-time value parameters, or type parameters, the field is classified as evidence in Step 3
  (a compile-time value parameter takes a concrete constant after instantiation and is counted as a
  compile-time constant; see §8.3.3);
- If an argument **is `fᵢ` itself** (a self-referential refinement such as `v: Pos(v)`), the field
  is classified as a **data field (admission)** in Step 3: the field is the variable carrying the
  value, and the constraint merely checks that value rather than proving a fact about something
  else;
- A forward reference to a later field, a reference to another evidence field, or treating `fᵢ`
  itself as an evidence argument while mixing it with other fields (SelfMixed) ⇒ rejected at
  declaration time.

**Step 3: Determine the category.**

| #   | Form                                                                                                                                                                                                       | Classification                                                      | Runtime Layout                                                                                                               |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| 1   | Space `Proof`, with all arguments being strictly preceding data fields / compile-time constants / compile-time value parameters / type parameters (positive example `w: Pos(v)`)                           | **evidence field**                                                  | Occupies no slot; `base` does not enter the runtime layout                                                                   |
| 2   | Space `Data`: ordinary value types (`n: Int`), data-type-family applications (positive example `data: Array(Int, n)`), ordinary records                                                                    | **Data field**                                                      | Occupies a data slot                                                                                                         |
| 3   | Space `Proof`, but an argument is `fᵢ` itself (self-referential refinement `v: Pos(v)`)                                                                                                                    | **Data field (admission)**                                          | Occupies a data slot; the constraint is adjudicated at the construction site under the existing refinement obligation (§8.4) |
| 4   | Space `Proof`, but there is a forward reference to a later field, a reference to another evidence field, or `fᵢ` itself is treated as an evidence argument while being mixed with other fields (SelfMixed) | **Illegal**: rejected at declaration time, with no silent downgrade | Produces no field                                                                                                            |

The four rows do not overlap: Row 2 means that “ordinary value type / data family” is always a data
field; Row 3 means that a “proposition family with self-reference” is an admission data field; and
Row 4 means that a “proposition family with an illegal position” is always rejected at declaration
time. **There is no path under which “a forward reference also counts as a data field”**—a forward
reference neither occupies a data slot nor enters the runtime data index.

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }     // Space Proof

Box: Type = {                         // Positive example: w is evidence
    v: Int,          // Data field
    w: Pos(v),       // evidence: Pos is a Proof family, and argument v is a strictly preceding data field
    z: Int,          // Data field; runtime index = 1 (skipping w)
}

Buf: Type = {                         // Negative example: data is still a data field
    n: Int,                 // Data field
    data: Array(Int, n),    // Data field: Array is a Data family, and n only determines the length; it does not generate a proof
}

Admission: Type = {                   // Self-referential refinement: data field, not evidence
    v: Pos(v),           // Proof family, but the argument is itself => admission (Row 3 of the table)
}
```

**Evidence fields may depend only on strictly preceding data fields.** Their arguments must not
reference another evidence field, a later field, or the field itself. This constraint guarantees
that, when construction reaches the field in declaration order, all arguments already have
compile-time values, making the proposition a closed term that can be verified by the proof kernel.

**`base` does not enter the runtime layout.** The base type of an evidence field (the `base` of a
refinement type) is type information used only for kernel equality checking and normalization; a
runtime slot is never allocated for the field. Evidence fields also have no default value—they have
no runtime value to supply.

#### 8.3.2 Construction-Site Obligations and Proofs

**There is only one source-level construction form: data arguments.** Evidence fields **occupy no
constructor argument position**—there is neither a form that “passes a witness as a value” nor an
“evidence argument position.” Constructor arity counts only data fields: **compile-time arity =
runtime arity = number of data fields**.

**Evidence is not a constructor argument; it comes from a proof obligation automatically generated
at the construction site.** When constructing a record, the compiler generates one proof obligation
for each evidence field:

1. Take the compile-time values of the strictly preceding data fields referenced by that field's
   arguments;
2. Substitute them into the field's proposition to obtain a closed proposition (`Pos(3)` reduces to
   `3 > 0`);
3. Have the proof kernel verify it under the current logical signature (RFC-027 §4).

Adjudication is three-valued:

| Result    | Compile-Time Behavior                                                            |
| --------- | -------------------------------------------------------------------------------- |
| Proved    | Passes; the certificate is retained at compile time and erased at runtime        |
| Disproved | Compile error + counterexample (**E4018**)                                       |
| Unproven  | Compile error + unproven proposition + reason + evidence-supply hint (**E2031**) |

**Unproven does not mean that the proposition is false**, nor does it permit the code through.
Evidence is supplied by an **independent declaration**, not by rewriting the construction
expression: write an **independent proof function** whose return type equals the proposition, or
register a **named proof** in a proof library and reference it from the construction obligation
(RFC-027 §4.3). There is no form that “fills in an incorrect witness value”—evidence fields have no
argument position at construction sites. Supplying one extra evidence argument at source level
(`Box(3, 5, evidence)`) cannot land in any data-parameter slot and reports an E1010/E1014-family
argument error rather than being silently ignored.

A self-referential refinement field (`v: Pos(v)`) is a data field: its constraint is adjudicated at
the construction site under the existing refinement obligation (§8.4), and it creates no evidence
obligation or change to the field's runtime index.

#### 8.3.3 Generic Records: Proof Family Parameter `P: T -> Proposition`

A generic record may carry a **proof family parameter** `P` whose kind is `T -> Proposition`: it
accepts a type parameter and returns a proposition. `Proposition` is the sort alias for the `Proof`
space in the logical signature (RFC-027 §4.1), not a source-level type name. Its source-level form
is identical to an ordinary type-level function parameter, and its kind constraint is checked at
declaration and instantiation:

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Cell: (T: Type, P: (T: Type) -> Type) -> Type = {
    v: T,        // Data field
    w: P(v),     // evidence: P is a Proof family, and argument v is a strictly preceding data field
}
```

- The kind of `P` comes from the same source as the criterion in §8.3.1: **the result space must be
  `Proof`**. Passing a data-type family (such as `Array`) as `P` violates the kind and is rejected
  directly at declaration or instantiation—this is a type error, not a case where `w` “degenerates
  into a data field.”
- `P` is a compile-time parameter position and occupies no runtime slot. When instantiating
  `Cell(Int, Pos)`, `P` is bound to the proof family `Pos`, the proposition of `w` becomes `Pos(v)`,
  and evidence classification is word-for-word identical to §8.3.1.
- Classification is rerun **after instantiation**.
- When `P` is unbound (the generic has not been instantiated), the result space of `w` remains
  `Proof` (its kind has been declared), but the proposition is an open term, and the construction
  site returns Unproven—it is neither silently accepted nor downgraded to a data field.

#### 8.3.4 Cross-Module Export

When a record containing evidence fields is used across modules, its propositions and evidence are
exported together with the type definition, and the importer does not re-infer them from field
names. The exported payload includes: the kind of type-family parameters (including proof family
parameters), the proof obligation for each evidence field, and each evidence field's evidence
slot—the proposition, an explicit evidence source or verified certificate, and the hash of the
logical signature that produced it.

The importer uses **its own current logical signature** to verify these propositions: when the
signature hash differs, RFC-027 §4.1's three explicit paths apply—a translation term, a bridging
axiom, or re-proving under the importer's signature. The importer must not directly reuse the
exporter's conclusion.

A referenced proof-library entry must appear in the importer's library list; otherwise, the
conclusion is Unproven, and the diagnostic recommends registering the library or writing a bridging
axiom.

> **Implementation status (V1)**: The current implementation performs importer-side re-verification
> and per-library source-fingerprint comparison only; translation terms and bridging axioms await
> RFC-027 §4.1's kernel; the `evidence_ref`/`certificate_ref` slots are not materialized (D1 A), and
> `signature_hash` is the exporter-side evidence-payload fingerprint.

#### 8.3.5 Type-Level Use and Value-Level Rejection

Evidence fields **may only be used at the type level**. A `b.w` in a type position is
**user-writable type-level syntax** referring to the field's **evidence item** (proof item), not a
runtime value: it points to the field's registered evidence reference and may serve as a premise for
another proposition. V1's semantics are **admission** (the evidence slot's proposition was proved at
the construction point; see 09a §P6 6.9.5-B.3 §五), rather than replay under the logical signature
(§4.2 is not implemented).

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Box: Type = {
    v: Int,
    w: Pos(v),
}

// Type-level use: treat bb.w as an evidence premise, not as a value read
Holds: (bb: Box) -> Type = { bb.w }
needW: (b: Box, h: Holds(b)) -> Int = { b.v }
```

**Value-level reads of evidence fields are always rejected**: reading `b.w` in a value expression
reports **E1114** (`witness_field_not_readable`). A field path is not a value term, and neither the
evaluator nor the runtime representation materializes a value for it. If runtime data is required,
define the field as an ordinary data field.

```yaoxiang
b = Box(3)
x = b.w        // ❌ Value-level read of a witness field -> E1114
```

> **Design basis**: [RFC-027 §14.5](../../rfc/accepted/027-compile-time-evaluation-types.md) (item
> forms for type-level use, replacing 4A's “no user syntax at the type level”). The syntax and
> parsing of type-level field paths have been implemented (WBS 6.9.5 B.3): type positions accept a
> single-level `ident.ident`; value-level reads still report E1114.

#### 8.3.6 Immutability, Erasure, and Diagnostic Codes

**5A: A record containing evidence fields is immutable as a whole.** Data fields must not declare
`mut`, undergo field assignment, be borrowed through `&mut`, or serve as the receiver of a write
method (including automatic write borrows). Otherwise, an evidence proposition could become invalid
after a field is rewritten. A violation reports **E1115** (`witness_record_immutable`). Records
without evidence fields are not subject to this restriction.

**Erasure.** Evidence fields occupy no runtime slots; the runtime layout contains only data fields.
Proof certificates are retained at compile time and erased entirely at runtime (RFC-027 §13).
Runtime data indexes count only data fields—the data index of `z` in a record `{ v, w, z }` is 1,
skipping the witness field `w`. Runtime-mode boolean checks (§8.4) are unrelated to evidence—they
inspect runtime values, not certificates.

**Division of diagnostic responsibilities** (each code covers one aspect and does not replace
another):

| Code      | Trigger                                                                         | Semantics                                                                  |
| --------- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| **E4018** | An evidence proposition at a construction site is disproved                     | A refinement predicate is violated; a counterexample is attached           |
| **E2031** | An evidence proposition at a construction site is Unproven (refinement channel) | The refinement constraint cannot be proved; explicit evidence is requested |
| **E1114** | Value-level read of an evidence field                                           | Evidence fields cannot be read at the value level                          |
| **E1115** | Mutable write to an evidence record                                             | The record is immutable as a whole                                         |
| **E4023** | General Unproven outside the refinement channel                                 | A general inability to prove, with the proposition, reason, and location   |

**Declaration-time rejection list** (consistent item by item with the four-category table in
[RFC-027 §14.6](../../rfc/accepted/027-compile-time-evaluation-types.md)): when a field whose space
is `Proof` has one of the following forms, it is rejected directly at its declaration, does not
proceed to the construction site, and is not silently downgraded to a data field:

| #   | Rejected Form                                                                                                   | Explanation                                                                                                        |
| --- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| 1   | Forward reference to a later field `fₕ (h > i)`                                                                 | Illegal position                                                                                                   |
| 2   | Reference to another evidence field as an argument                                                              | Illegal position                                                                                                   |
| 3   | Treating `fᵢ` itself as an evidence argument while mixing it with other fields (SelfMixed, such as `P(fᵢ, fₕ)`) | Illegal position; pure self-reference `P(fᵢ)` is an admission data field under Row 3 of §8.3.1 and is not rejected |
| 4   | A proof family parameter `P` whose kind is a data-type family (§8.3.3)                                          | Kind mismatch; a type error at declaration/instantiation                                                           |

**Evidence fields must not have default values** (`w: Pos(v) = expr`, DefaultOnEvidence): evidence
fields have no runtime value to supply, and the construction view has no argument position for them,
so a default value is rejected at declaration time. This is an independent field-declaration check;
it is not merged with the four categories above or silently handled as “a data field with a default
value”; `StructFieldInfo.has_default` is always `false` for a witness. The default-value rules in
§3.1.1 (fields with defaults may be omitted during construction; fields without defaults are
required) apply only to data fields.

Applications of data-type families whose space is `Data` and pure self-referential refinements whose
space is `Proof` (`v: Pos(v)`, admission) are both legal data fields and do not belong to the
illegal-position categories. When a generic has not been instantiated, its proposition is an open
term, and the construction site returns Unproven (§8.3.3); this is not a declaration-time rejection.

### 8.4 Assert Refinement Types and assert Assertions

`assert` and `Assert` are two sides of the same refinement primitive, automatically selected by the
dispatch pipeline according to whether the predicate's free variables are accessible at compile
time.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                          |
| ----------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enter the proof pipeline: true → erase to Void; false → compile error (Never is uninhabited)      |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Insert a runtime Bool check and inject a refinement fact into the flow-sensitive assumption set Γ |

Dispatch is an instance of **checking by declaration strength** (§7.1) at the predicate position.
The same principle covers the entire strength ladder: classic types without a strength declaration
do not enter the proof pipeline or undergo quantity inference; predicate annotations (this section),
q=0 slots (§7.1), dependent-record fields (§8.3), and relational predicates each perform the
corresponding checks according to their declaration form. Cost follows declaration strength rather
than global analysis.

**Flow-sensitive assumption set Γ**:

The compiler maintains the set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: the old assumption is invalidated
```

After an assignment to a `mut` variable, all assumptions involving that variable are removed (kill
set). At a branch confluence, Γ is the intersection of the branches.

### 8.5 Terminates: Termination-Measure Predicates

`Terminates` is a **built-in predicate**, belonging to the core primitives alongside `Int` and
`Never` (a built-in name, not a keyword). It binds a **measure** to a computation, declares that the
computation terminates, and supplies a witness of termination.

**Forms**: two arities using the same predicate:

| Form                    | Anchor                         | Use                                                                                                                                       |
| ----------------------- | ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the containing binding | Default form—self-recursive functions and loops                                                                                           |
| `Terminates(FnType, m)` | Explicit function type         | When the measure's ownership must be specified explicitly (the measure is defined elsewhere, or one measure serves multiple computations) |

```yaoxiang
// Measure: an ordinary function that can be unit-tested and reused; it does not participate at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere, with explicit ownership
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, and the measure is an expression in scope
// The loop body is a `{}` block whose value is provided by its tail expression (spec §2.9): it is not `Void` unless the body tail contains an expression
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

**Semantics**: The `Terminates(m)` refinement applies to the value type of the computation marked in
its type position. The obligation is placed on that computation—for a function, at every recursive
call site; for a loop, at the back edge—in both cases requiring that “the next state's measure is
strictly less than the current state's measure,” adjudicated under the path guard at that point.

The call-site obligation is `m(callee_args) < m(caller_args)`; the back-edge obligation is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: A loop is an anonymous construct and usually cannot be referred to by
> name. The binding name is the name—in `acc: Terminates(n - i) = while ...`, `acc` provides the
> anchor, making the loop referable. This is why the unary form of `Terminates` applies to loops.

**Measure**: The return type is not restricted (natural numbers are not required); its “strict
decrease” is determined by a well-founded order available on that type. Whether the measure is
well-founded (for example, whether an `Int` measure is `>= 0`) is an **independent obligation**,
adjudicated by the compile-time proof pipeline just like the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. An ordinary unrefined type (such as a bare `while` loop or a function
without a refined signature) does not enter verification mode and creates no termination obligation.

**Automatic exploration first**: The compiler first attempts to discover a measure automatically
(using four templates: linear ranking functions, predicate-violation counts, bounded
increment/decrement, and multiplicative scaling). An explicit `Terminates` is needed only when
automatic exploration fails.

**Runtime representation**: This is a purely compile-time entity and is erased with its witness; it
does not enter the runtime binary.

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

## Chapter Eleven: Type Attributes

YaoXiang has only one type attribute that needs to be distinguished: linear versus copyable. It is
inferred automatically by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, argument passing, and return all mean
ownership transfer.

```yaoxiang
// A struct containing a heap-buffer field does not derive Dup (the field falls under Move) -> default Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move; b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle and Share the Data)

**The Dup attribute is used for reference/token types and internally reference-counted handle-value
types**. Assigning a Dup type performs a shallow copy—it copies the handle/token while the
underlying data remains shared. Multiple holders refer to the same data.

| Type            | Attribute | Description                                                                                |
| --------------- | --------- | ------------------------------------------------------------------------------------------ |
| `&T`            | Dup       | Zero-sized read token; copying the token gives multiple views of the same data             |
| `ref T`         | Dup       | Copying an Rc/Arc increments the reference count and shares heap data                      |
| String, Bytes   | Dup       | Internal reference counting; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear    | Zero-sized write token; exclusive and non-copyable                                         |
| struct          | Derived   | Dup if all fields are copyable (primitive value types ∪ Dup); otherwise Move (#398)        |
| tuple           | Derived   | Determined element by element, using the same rule as struct (#398)                        |
| All other types | Move      | Default ownership transfer                                                                 |

**Primitive value types** (`Int`, `Float`, `Bool`, `Char`) receive special treatment from the
compiler: assignment automatically copies the value, and the two values become fully independent.
This is native compiler behavior and is not part of the Dup type attribute.

**Derivation rules** (finalized in #398)—the proposed rule “automatically derive Dup when all fields
are Dup” cannot be applied literally: primitive fields such as `Int` are not themselves Dup, so
`{ x: Int, y: Int }` would be misclassified as Move. The executable form is:

1. **Copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), and composite types that have already derived
   Dup);
2. **struct**: If all fields are in the copyable field set, derive Dup; if **any** field is Linear
   (`&mut T`) or Move (a nested Move struct / Vec, Dict, or another container / resource), the
   struct as a whole remains Move—no intermediate “partially copyable” state is introduced. Evidence
   fields have no runtime values and do not participate in derivation (§8.3);
3. **tuple**: Uses the same rule as a struct and is determined element by element; an empty tuple
   (unit) is `Void`;
4. **Derivation is recursive**: When a field has a named type (such as `target: Point`), its
   definition is expanded before classification; `A = { b: B }` follows B's derivation result.
   Cyclic aliases conservatively fall under Move at the depth limit;
5. **Outside the derivation scope** (still Move, handled separately): containers (Vec / Dict / Set /
   Option / Result / Array) and enums.

```yaoxiang
// &T: Dup, aliases may be created freely
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

**Difference between Dup and Clone**:

|                        | Dup                                                                     | Clone                                          |
| ---------------------- | ----------------------------------------------------------------------- | ---------------------------------------------- |
| **Semantics**          | Shallow copy: copies the handle/token while sharing the underlying data | Deep copy: creates a complete independent copy |
| **Invocation**         | Implicit (automatic during assignment/argument passing)                 | Explicit (`.clone()`)                          |
| **Effect of mutation** | Changes are mutually visible (the underlying data is shared)            | Changes are independent (separate copies)      |
| **Applicable types**   | `&T` tokens, `ref T`                                                    | Any type implementing the Clone interface      |
| **Cost**               | Zero overhead (a token is a zero-sized type)                            | Depends on the type                            |

**Dup does not imply Clone, and Clone does not imply Dup**—they are orthogonal concepts:

```yaoxiang
// Dup type: copies the token while the underlying data is shared
view: &Point = &p
view2 = view        // Dup: copies the token; both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable and sees the same data

// Primitive value type: the compiler automatically copies the value (not Dup)
x: Int = 42
y = x               // Value copy; x and y are fully independent
print(x)            // Usable

// Clone: explicit deep copy that creates an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy; p remains usable

// Non-Dup type (a field falls under Move, so Dup is not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field and does not derive Dup (see the derivation rules in §11.2)
```

**Design intent**:

- Dup is used for tokens/reference types and solves the problem of “multiple views of the same
  data.”
- Clone is used where an independent copy is required; making the cost explicit through an explicit
  invocation is intentional.
- Copying primitive value types (`Int`/`Float`/`Bool`/`Char`) is built-in compiler behavior and is
  not part of Dup.
- Custom types are Move by default (zero-copy and high performance); Dup is derived automatically
  when all fields are copyable (§11.2 derivation rules).

## Chapter Twelve: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not “references,” but
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
- No dedicated borrow checker is needed—type attributes (Dup/Linear) naturally infer permissions.
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

Tokens are ordinary types and therefore support all operations available to ordinary types:

**Returning a token**—the token propagates with the return value:

```yaoxiang
// ✅ The child token is returned together with the parent token
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // The token is returned to the caller
print(px_ref)                    // OK; the token remains in scope
```

**Storing it in a struct**—a struct may carry token fields:

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

> Note: After a closure (function value) escapes, the scope in which it was defined may have ended,
> so it must not implicitly capture outer variables. The call site (creation point) is necessarily
> still alive, and fixing the context there as a value inside the closure is safe.

### 12.4 Automatic Borrow Selection

At the call site, the compiler selects automatically according to the following priority:

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
RFC-011a): a receiver of `&T` means a read-only borrow token; `&mut T` means a mutable borrow token;
by-value means Move (consuming the receiver). A borrow token created at the call site is released
when the call ends (transient; interval semantics in §12.5). An interface's borrow receiver must be
declared explicitly by the interface author as `&Self`, and the impl signature, after replacing
`Self ↦ impl 类型`, must match the interface exactly (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrowing Horner proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrowing propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification. Token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and a derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ The token is released automatically after its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Uses &mut Point
    }
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken remains usable
}

// ❌ The same argument cannot simultaneously create an &mut token and another token
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives an &mut token and an & token
```

### 12.6 Compiler Internals: The Brand Mechanism

Users never interact with brands. Internally, the compiler assigns each token a unique compile-time
identifier:

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

The purposes of brands are:

- **Anti-forgery**: A token may only be obtained from its owner's capsule and cannot be fabricated
  from nothing.
- **Association tracking**: A derived `&Float` carries its parent's derived brand (`#N.field_x`),
  allowing the compiler to trace it back to the parent token.
- **Conflict detection**: A WriteToken and a derived ReadToken from the same source cannot be live
  simultaneously.

Brands disappear completely after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data -> Dup-safe)
               | &mut T      // WriteToken (exclusive read/write -> Linear)
```

### 12.8 Borrow Tokens vs ref

|                 | `&T` / `&mut T`                                               | `ref`                                             |
| --------------- | ------------------------------------------------------------- | ------------------------------------------------- |
| Purpose         | Look at it / modify it in place                               | Shared ownership                                  |
| Scope           | Follows the token value's scope                               | Crosses scopes                                    |
| Cost            | Zero overhead (zero-sized type; disappears after compilation) | Rc or Arc (selected by the compiler)              |
| Escape          | Possible (the token propagates through return values/structs) | Intended for escaping                             |
| Cross-task      | Not possible (tokens do not implement cross-task transfer)    | Possible (the compiler automatically selects Arc) |
| Cycle detection | Not applicable                                                | Silent within a task; cross-task lint             |

> Note (undefined): How content is read after creating a `ref` (dereference / method / automatic
> behavior) has not yet been defined in the specification. The current implementation reports E1052
> for `*a`. This section will be completed once the behavior is defined.

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

// Compile-time predicate + evidence field: w occupies neither a constructor argument position nor a runtime data slot
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

// Unary form: the anchor is the binding name (self-recursive functions and loops)
// The body tail must provide a value—the value of `while` = the value of the loop body block = the tail expression (spec §2.9)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}

// Binary form: explicitly specify measure ownership (the measure is defined elsewhere)
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

// proof family parameter: kind is T -> Proposition (the result space must be Proof)
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
// All types are Move by default. Assignment, argument passing, and return = ownership transfer

// === Primitive value types (compiler built-ins) ===
Int, Float,     // Automatically copied by value during assignment; the two values are fully independent
Bool, Char      // Not Dup; built-in compiler handling of primitives

// === Dup (shallow copy: copies the handle and shares the underlying data) ===
&T              // Zero-sized read token; copying the token = multiple views of the same data
ref T           // Copying an Rc/Arc = reference count + 1; heap data is shared
String, Bytes   // Internal reference counting; assignment copies the handle and shares the underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) -> Dup (#398)

// === No derivation (remain Move) ===
Vec/Dict/Set    // Containers and enums are outside the derivation scope (#398)

// === Linear ===
&mut T          // Zero-sized write token; Linear (exclusive, non-copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Creates an independent copy; mutation does not affect the original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token; freezes source data -> Dup (copyable)
&mut T          // Zero-sized compile-time write token; exclusive read/write -> Linear (non-copyable)

// Automatic selection at call sites
// 1. If the argument is used later -> create a token
// 2. If the argument is not used later -> Move
// 3. Priority: &T < &mut T < Move

// Token propagation
// ✅ May be returned, stored in a struct, or captured by a closure
// ❌ May not cross tasks (tokens do not implement cross-task transfer)
```
