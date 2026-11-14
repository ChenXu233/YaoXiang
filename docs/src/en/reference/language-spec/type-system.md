# Type System Specification

This document defines the type system specification for the YaoXiang programming language, including
primitive types, compound types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the profound correspondence between the type system of a programming language and
mathematical logic:

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
| Case analysis                                  | Type-level `match`                                   |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires a
> type-level recursive function and compiler termination checking.

### 0.2 Types Are Propositions, Programs Are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computations correspond to correct constructive proofs**. YaoXiang type
  families (such as case analysis on `Add` over `Nat` plus recursive calls) are fundamentally
  type-level encodings of mathematical induction—provided that the compiler can perform termination
  checking.
- **Type checking is proof verification**. When a program passes type checking, this is equivalent
  to a logical proposition having been constructively proved.

### 0.3 Impact on Language Design

The specific manifestations of the Curry-Howard correspondence in YaoXiang are:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox
   caused by `Type: Type` (Girard's paradox).
2. **Type families** (RFC-011): type-level case analysis and recursive calls for the natural-number
   type `Nat(Zero/Succ)` correspond to the Peano axioms—provided that the compiler performs
   termination checking.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
   quantification of the form “for every integer N, there exists a type.”

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

> **Design note**: Although RFC-010 proposes a unified “everything is an assignment” model
> (`name: type = value`), types and values must still be distinguished at the syntactic level. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`). As a BNF placeholder, `TypeExpr` corresponds to the `Type` enum in the
> implementation and indicates “a type is expected at this position.”

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

**There are 15 built-in type names; this section is the authoritative table**: `Never` / `Void` /
`()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` /
`Char` / `String` / `Bytes`. The single source of truth on the implementation side is
`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`). This table matches it **name for
name**—the T2 gate of `tools/type-tables --report` compares this table, `CONST_PARAM_TYPES`, and the
LSP type-name list.

| Type      | Logical Correspondence | Description                                                                                     | Default Size |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------- | ------------ |
| `Type`    | —                      | Meta-type (not a value type name; see below)                                                    | 0 bytes      |
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
with `Void`: standard-library signature strings write `() -> Void`, and the parsing table places
`()` and `Void` in the same arm.

**There is only one correct spelling**. `Int` is not `int`, `Int64` is not `int64`/`i64`, `String`
is not `string`/`str`, `Bool` is not `bool`, `Void` is not `void`, `Never` is not `never`, `Float`
is not `float`/`f64`, `Char` is not `char`, and `Bytes` is not `bytes`. The implementation
previously supported 22 such lowercase or abbreviated spellings in parallel, but all of them were
removed in **WBS 6.1.2 (finalized on 2026-10-27)**: writing a noncanonical spelling in a type
position always produces an unknown-type error and is never silently accepted as a user-defined
type.

> **`DateTime` is not a built-in name**. It is a **semantic alias in the std layer**: the `now` /
> `parse_time` signature strings in `src/std/time.rs` and `src/std/interfaces/time.yx` still use
> `DateTime`, but at runtime it is `Int` (fixed together with #338). The parsing table retains it
> only as a **temporary compatibility alias**, and this table does not include it. Moving it out of
> the built-in name table and making it a responsibility of the std layer belongs to **P10 S11**
> (issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)).

> **There are no unsigned integer types**. `Uint` (and `Int128`) are not usable type
> names—`MonoType` has no unsigned variant. The LSP type-name list and the `sizeof` name table each
> once contained a residual entry, but both were removed with WBS 6.1.2. When unsigned semantics are
> needed, use `Int` with a lower-bound convention.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤), respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. There is
   nothing to place on the right-hand side of `x: Never = ...`.
2. **Explosion principle**: `Never <: T` holds for every type `T`. `assert(false)` returns `Never`,
   after which subsequent code can pass type checking (although it is never reached).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead-code analysis and `match` branch confluence.

`Never` is a built-in type name (registered through the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — exactly one inhabitant (the default void value). `Void` is the identity
element of zero-field product types. `x: Void = <default>` is valid. A block's value is provided by
its **tail expression** (an empty block `{}` has type `Void`); see
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

// Record type with generics
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
- An interface name written in the type body indicates that the interface is implemented.
- Fields are divided into data fields and evidence fields according to the **proof-space criterion**
  (§8.3): a field whose normalized type lies in proof space (`Proof`) is an evidence field; it does
  not occupy a constructor argument position or a runtime data slot. A field lying in data space
  (`Data`) is always a data field.

> **Namespace ownership**: A `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to the `Point` namespace. It does not trigger any implicit binding. For `.`-call syntax
> such as `p.draw()` to take effect, an explicit binding is required: `Point.draw = draw[0]`. See
> RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values that may be supplied optionally during construction:

```yaoxiang
// Fields with default values - optional during construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Fields without default values - required during construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // Correct
Point2()          // Error
```

**Rules**:

- `field: Type = expression` -> has a default value and is optional during construction
- `field: Type` -> has no default value and is required during construction
- Evidence fields have no runtime value and therefore have no default value (§8.3)

#### 3.1.2 Built-in Bindings

Methods can be bound directly within a type definition:

```yaoxiang
// Method 1: reference an external function binding
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

**Direct interface assignment**: A concrete type can be assigned directly to a variable of interface
type (structural subtyping).

```yaoxiang
// Direct assignment (the concrete type is known at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through the vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inference Result       | Calling Method              |
| ------------------------------------ | ---------------------- | --------------------------- |
| Direct assignment of a concrete type | Concrete type is known | Direct call (zero overhead) |
| Function return value                | Unknown                | vtable                      |
| Heterogeneous collection             | Multiple types         | vtable                      |

**Coherence and orphan rules (not applicable; closure explanation)**: YaoXiang interfaces are
structural types (an interface is a record whose fields are all functions), not nominal traits.
Consequently, there is no cross-crate/module “who may implement what” ownership problem, and
Rust-style orphan rules and coherence checks have no applicable target (the decision record is in
RFC-011 §2.1). The corresponding safeguard in a structural world is **duplicate-implementation
rejection**: defining the same method signature more than once on a type is a compile error
(RFC-011a §3, overriding is prohibited; overloading is permitted).

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
| Data field                   | Occupies one position, in field declaration order | Occupies one slot      |
| Evidence field               | Does not occupy a position                        | Does not occupy a slot |
| Function-type field (method) | Does not occupy a position                        | Does not occupy a slot |

**Compile-time arity = runtime arity = number of data fields.** Evidence is not a constructor
argument: it comes from proof obligations automatically generated at the construction site (§8.3.2),
which the compiler synthesizes itself or verifies using an independent proof function supplied by
the programmer. There is no source-level “evidence argument position.” Passing one extra argument
does not match any data parameter slot and produces an argument error in the E1010 / E1014 family;
it is not silently ignored.

Fields with default values may be omitted during construction; fields without default values are
required (§3.1.1).

**Construction sites carry proof obligations**. Each evidence field creates a proof obligation at
the construction site: substitute the compile-time values of the strictly preceding data fields into
the field's proposition, yielding a closed proposition, and have the proof kernel verify it under
the current logical signature (§8.3.2). If the obligation is disproved, compilation fails and a
counterexample is provided. If it cannot be proved, supply an independent proof function or a named
proof from a proof library (§8.3.2), rather than adding a constructor argument—evidence fields have
no argument position at construction sites.

**Two views of the same construction**:

- **Compile-time view**: Data arguments are bound in declaration order; evidence fields do not
  occupy argument positions, but each carries a proof obligation awaiting verification.
- **Runtime view**: Only data fields enter the runtime layout; evidence fields and their
  certificates are completely erased at runtime (§8.3.6).

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Box: Type = {
    v: Int,          // Data field: constructor argument 0, runtime data slot 0
    w: Pos(v),       // Evidence field: Pos is a proof-space family, and Pos(v) is a proposition
    z: Int,          // Data field: constructor argument 1, runtime data slot 1
}

b = Box(3, 5)       // arity = 2 (v, z); only v=3 and z=5 exist at runtime, while w is compile-time evidence
```

> **Design basis**: [RFC-027 §14.1–§14.2](../../rfc/accepted/027-compile-time-evaluation-types.md)
> (field-position rules and construction views),
> [RFC-027 §4](../../rfc/accepted/027-compile-time-evaluation-types.md) (proof kernel and logical
> signatures).
>
> **Implementation status**: Field quantity representations, runtime data-index mapping, and
> immutable witness-bearing records have been implemented (WBS 6.9.5 B.1). Evidence-field
> classification and construction-site proof obligations are not yet user-accessible— until
> classification is implemented, proposition-shaped fields still participate in constructor argument
> counting as ordinary data fields (6.9.5 B.2–B.4).

### 3.5 Function Types

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= Param (',' Param)*
Param       ::= Identifier ':' TypeExpr    // Named parameter
              | TypeExpr                   // Unnamed parameter (bare type)
```

**Two parameter forms, with different ownership of names** (the type-level side of RFC-007's
shorthand rules):

- **Named** (`a: Int`): The name enters the contract—it can be referenced by a named argument
  (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)), and when the signature declares parameter
  names and types, the lambda header may be omitted (`add: (a: Int, b: Int) -> Int = a + b`, RFC-007
  §Shorthand Rules).
- **Unnamed** (a bare type, such as `(Int, Int) -> Int`): Type constraints apply **by position**,
  while the names belong to the implementation— the lambda header must provide its own parameter
  names (`mk: (Int, Int) -> Int = (x, y) => x + y`, RFC-007: a lambda header may be omitted if and
  only if the signature already declares parameter names). Such parameters cannot be referenced by
  named arguments and can only be called positionally. A bare identifier is resolved in the type
  namespace; an error is reported if no declared type can be resolved—there is no bare identifier in
  a parameter position that is “neither a type nor a type parameter.”

**Binding semantics of annotations**: An annotation is a declaration, not a comment. Binding
statements are checked against their annotations—parameter types drive the lambda header position by
position, and the function body's exits (the tail expression and `return`) are uniformly checked
against the annotated return type. The type registered for callers is the annotated form.
Unannotated bindings use HM inference (RFC-007). `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are rejected under the same rule—binding semantics do not diverge
according to the syntactic form of the value (lambda / block / expression / named function
reference).

**The parameters in an implementation header are positional binders**
(`f: (a: Int) -> Int = (b) => b + 1`):

- **Names and types follow the declaration**: The implementation header's i-th position corresponds
  to the first N positions of the declaration's **value-level signature** (N = the number of
  parameters in the implementation header; the value-level signature = the parameter sequence
  remaining after removing compile-time positions—`T: Type`, type-level function positions, and
  const positions). The implementation header only declares “what the i-th position is called in the
  body”; renaming it does not affect the call site—the available name for `f(a = 7)` comes from the
  declaration, independently of the implementation header's name.
- **Annotations in the implementation header are used only for comparison**: If the type at the same
  position differs from the declaration, **E1011** is reported (the declaration is the authoritative
  source, and the author's annotation must not be silently overridden).
- **If the implementation header has more parameters than the declaration's runtime parameter
  positions**, **E1112** is reported: the extra positions have no values to bind (type positions
  `T: Type`, type-level function positions, and const positions do not occupy runtime parameter
  positions).
- **The reverse case (the implementation omits parameters) is not an error** (the “pass” cell in the
  t1 behavior matrix before D54 must not be reclassified): at the type level, arity is still checked
  according to the **declaration** (`f(1)` is valid and `f(1, 2)` reports E1010), and missing
  positions are supplied positionally by the declaration into the **type-checking scope**. However,
  the implementation header has no corresponding runtime slots, so arguments at those positions are
  discarded on the IR side and do not participate in computations in the body (`f(1)` is unrelated
  to the argument in `f: (a: Int) -> Int = () => 1`). **This is registered as a gap**—making the
  runtime receive the declared arity would be a class-D behavior change and requires a separate
  decision.
- **Uniform exit handling**: Whenever the tail expression or `return` (the arrow-sugar form `=> x`
  and the explicit form `return x` are two AST shapes) does not match the declared return position,
  **E1012** is reported, with the location pointing to the **annotation**.

> **Implementation status**: The binding-period comparison for `Fn` annotations has been implemented
> (WBS 6.8.2, 2026-11-02): parameters are driven positionally (the parser and typechecker use the
> **same alignment rule**: the first N positions of the value-level signature) /
> implementation-header annotation comparison (E1011, the author's annotation is retained and a
> mismatch with the declaration is an error) / uniform exit handling (E1012) / the declaration
> determines the registered form (available names for named arguments).

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

In a generic type definition, `(T: Type)` is the type constructor's parameter signature, and
`-> Type` indicates the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics and are handled through a unified generic-instantiation path. The
ownership of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                                | Foundation                               |
| ------------- | ------------- | ---------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type-level    | Fixed-length array (const generic `N`)   | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Raw buffer with runtime length; can grow | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard-library type (growable list)    | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                        | `HeapValue::Dict`                        |

> `List(T)` is a **standard-library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list` and receives the same treatment as user-defined generic records. All
> strategies for growable semantics (when to grow, how much to grow, and whether sharing is
> possible) live in the library; the compiler does not participate. `Vec(T)` is the minimal
> foundational primitive on which it depends.
>
> Set(T) has been removed: it has no literal, no runtime representation, and no std.set. When the
> need arises, complete it following the Dict pattern.

Key rules:

- **The destination of a literal is determined by context**: A bare `[...]` literal and a `List(T)`
  annotation produce a growable list; an `Array(T, N)` annotation applied directly to a literal
  produces a fixed-length array. Destination validation: the number of elements must equal N, and
  the element types must be compatible with T; otherwise E1002 is reported at compile time. When N
  is a symbolic constant (a const parameter), count validation is deferred to the refinement-type
  stage.
- **Implicit List→Array conversion is prohibited**: Fixed-length semantics are guaranteed by the
  type system—push accepts only a `List(A)` receiver.
- **Performance hierarchy**: Performance decreases and flexibility increases from bottom to top:
  `Array` > `Vec` > `List`.
- **Index-failure contract** (runtime errors are transitional; the target is compile-time refinement
  coverage through value-dependent types; see §8.5):
  - Index out of bounds (including a negative index) → `E6003`
  - Missing Dict key → `E6008`
- **String indexing semantics** (#385): `s[i]` indexes characters by **Unicode scalar value (code
  point)** and produces a single-character `String`—the same unit used by `len` / `index_of` /
  `substring` / `chars` / `char_code` in `std.string`, so these operations compose without unit
  mismatches; an out-of-bounds index (including a negative index) follows the index-failure contract
  above, `E6003`.
- **Membership `in` predicate**: Returns `Bool` without raising an error, and supports
  List/Array/Dict (keys)/Tuple/String/Range as its right operand. As a first-class higher-order
  predicate, it forms the basis of propositions provable at compile time by refinement types.`

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

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation—the
receiver type must implement the `Try` interface (four members: `is_failure` / `success` /
`residual` / `from_error`). On failure, the result of `from_error(residual(t))` is returned early
from the current function; on success, the expression's value is `success(t)`. The enclosing
function's return type must also implement `Try`, and its failure-residual type must match the
receiver's (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` receive `Try`
implementations from `std.result` / `std.option` (the failure-residual type of `Option` is `Void`).
A user-defined type can participate in `?` by writing `Try(Self, T, E)` in its type body. Lowering
for `?` does not distinguish built-in and user-defined types—the same interface follows the same
path.

The **variant set of a sum type enters the checker through its definition or a `use` import**:
`Result`/`Option` must be imported with `use std.result` / `use std.option` before use (whole-module
imports and grouped `use std.{...}` imports have equal authority). Variant construction (in
type-qualified form), match-variant destructuring, and exhaustiveness checking all use this registry
as their sole criterion; a native function signature returning `Result(Float, Error)` creates a
`Generic` with the same name and identity as the sum type in `std.result`, and it can be
destructured with match after `use`.

### 4.3 Generic Constructor Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each **data
field** corresponds to one constructor argument, and the field name is the argument name; fields
with default values may be omitted during construction, while fields without default values are
required. Function-type fields (methods) do not generate constructor arguments. Evidence fields
(§8.3) do not generate constructor arguments—their evidence is verified by the compiler at the
construction site and is not passed through argument positions.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value -> constructor argument is required
    extra: T,
}
// Automatically expanded complete form (compiler's internal view; users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }
// The expansion lists only data fields; evidence fields do not generate constructor arguments (§3.4)

// Call: call the automatically generated constructor
c  = Container(42, 43)            // Constructor arguments are filled in field order; T is automatically unpacked as Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type arguments + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // Named fields, in any order
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned afterward)

// Field default values -> constructor arguments may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single-parenthesized form, matched positionally against declared parameters from
left to right):

1. Arguments are matched positionally against the type declaration's parameters: a `Type` position
   accepts a type argument, while a compile-time value parameter position (such as `Int`) accepts a
   compile-time constant.
2. If a compile-time value parameter position matches successfully (partial match), processing is
   handled as type construction: all parameter positions are checked in order, and on failure the
   **first mismatched or missing parameter in declaration order** is reported.
3. If the arguments do not correspond at all to the declared parameters (all are values and no
   compile-time value parameter position can match), processing is handled as constructor
   invocation: positional arguments fill fields in order, and type parameters are automatically
   unpacked from the element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),       // Evidence field: Assert is a proof-space family (§8.3)
    data: Array(Array(T, Cols), Rows),    // Data field: Array is a data type family
}

m: Matrix(Int, 3, 4)              // Type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two levels: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern; data assigned afterward)

Matrix(42)    // ❌ Position 0: T←42 does not match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols is missing → report the first error: T expects Type, but found 42
Container(42) // ❌ Missing constructor argument extra
Container(42, 43, 44)  // ❌ Too many constructor arguments
```

**Type inference**: Type parameters of a generic type constructor are automatically unpacked from
constructor-argument elements (`Container(42, 43)` → T=Int); type parameters of a generic function
are automatically unpacked from argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). If
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

> **Constraint-solving sources (RFC-011b)**: Operator constraint names (`Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) are solved by consulting the interface
> implementation registry— `T: Add` ≜ a registered `Add(T, T, T)` instantiation; `Equal`
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
// More complex associated type
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // An associated type may itself be generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant (candidate)
```

> The criterion is **being referenced in a type position**, rather than “being annotated with a
> concrete type”: in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters
> (neither occurs in any type position).

**Terminology**: Whether a generic parameter annotated with a concrete type other than `Type` (such
as `Int`) enters the compile-time realm depends on whether its value is referenced in a type
position (value dependence). **No `const` keyword is needed** (the implementation once used “const
generic,” but the documentation consistently uses “compile-time value parameter”).

> **Design basis**:
> [RFC-011 §4.1 Compile-Time Value Parameters](../../rfc/accepted/011-generic-type-system.md).

Classification is driven by **strength declarations that can be read locally within a signature**:
when the author references a value parameter in a type position (`(k: N)`, `Array(T, N)`,
`Assert(N > 0)`), that reference is a strength declaration; the **quantity is its mechanical
reading**. Each binding infers a quantity `q ∈ {0, 1, ω}`: q=0 means compile-time evaluation,
runtime erasure, and no slot occupancy; q=1 is linear (Move); q=ω is free (Dup)—see the type
attributes in Chapter 11. This is not implicit: the appearance of a value name in a type position is
a declaration explicitly written by the author, and is the same act as writing `b: Positive(b)`
(§8.4).

A **“type position” means every `Type` subtree in a declaration**—the annotated types of parameters,
the function's return type, field types in type bodies, the types of nested `Fn`s, `Assert`
predicates, and argument positions of type constructions such as `Array(T, N)`. Classification
traverses **all** `Type` subtrees without case-by-case treatment by position; missing even one
subtree—for example, the return type—would misclassify a compile-time value parameter as a runtime
value parameter.

Whether a name is **“referenced”** is determined by namespace resolution, not by literal name
comparison:

1. The identifier is first resolved in the **type namespace**—declared type names and type
   parameters (`T: Type`);
2. If it resolves to a declared type, it is unrelated to the parameter. A parameter name and type
   name having the same spelling **does not constitute shadowing**: the value and type namespaces do
   not consult the same table (see the scope and shadowing rules in [modules](modules.md)); the two
   `Int`s in `(Int: Int) -> Int` belong to different namespaces;
3. If it does not resolve and a parameter with the same name exists among the declared parameters,
   the strength declaration is matched and that parameter has q=0;
4. If neither resolution succeeds, an error is reported: a type position cannot contain a bare
   identifier that is “neither a type nor a type parameter.”

Self-reference is handled with a monotone fixed point (the set only grows, with at most |candidates|
iterations).

**Slot allocation uses q as its sole input**. The argument sequence at a call site and the binding
sequence in the implementation header are two **derived views** of the same function:

- **Call-site argument view** = parameters − type parameters (in declaration order). A q=0 parameter
  still occupies an argument position: in `f(42)(5)`, 42 is supplied to N and 5 to n; argument
  positions carry “how many values the author filled in declaration order.”
- **Implementation-header binding view** = parameters − type parameters − q=0 parameters (in
  declaration order). The implementation header binds only names for which runtime values exist:
  `= (n) => n` is valid, while `= (N, n) => n` reports E1112.

E1011 and E1112 both count according to the **implementation-header binding view**.

| Form                                                       | Classification                      | Reason                                                                    |
| ---------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | They occur only in value positions, with no strength declaration          |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N occurs in a type-construction argument position                         |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N is the type of the inner parameter k                                    |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N is an ordinary parameter          | N is not referenced in the type body, so there is no strength declaration |

**Core design**: A compile-time value parameter `(N: Int)` is distinguished from a value parameter
`(k: N)`, separating compile-time constants from runtime values. **Obligations on the call side are
checked according to declared strength**: the validity of `f(y)` depends on whether the argument y
carries compile-time evidence (a literal, a value environment, a single-value pin, a Γ equation),
not on a global analysis of f's declaration. Without evidence, the error is reported at the call
site and points to the q=0 slot visible in f's type. A rejected candidate (a candidate shape that is
not referenced in a type position) is an ordinary parameter without a strength declaration and is
handled normally as q=1/ω, with no separate routing code.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (the Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N occurs in a type-construction argument position → compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated in a type position (at compile time), and the result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // The compiler evaluates factorial(5) = 120 at compile time

// Value dependence: N is the type of the inner parameter k
// N is a compile-time value parameter (it occurs in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (a single-value type).
// Single-value pinning: k's value is fixed by its single-value type and is known at compile time,
// so k - 1 carries compile-time evidence and can fill the recursive call's N position.
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

## Chapter 8: Conditional Types and Dependent Records

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
// IsTrue bridging and Assert refinement types (see §8.4 for details)
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

A **dependent record Σ** is a record type in which “later fields may reference earlier fields.” The
fields of Σ fall into two categories:

- **Data fields**: Fields whose types lie in data space. They occupy runtime data slots and
  participate normally in construction and access.
- **Evidence fields**: Fields whose types lie in **proof space** (propositions)—in other words, “an
  assertion awaiting proof.” They are not runtime data but compile-time evidence: when the record is
  constructed, the compiler must provide a certificate verified by the proof kernel.

The sole criterion for this division is the **proof-space criterion**: normalize the field type and
take the space registered for its **result type family** under the current logical signature—`Proof`
(the result of a proposition/refinement/proof family) or `Data` (runtime data). A result in `Proof`
⇒ enter evidence classification; a result in `Data` ⇒ data field, **regardless of how many preceding
fields it references**. The space is determined by the logical signature's sort registration and the
library manifest, not inferred from the field name, argument shape, or predicate-like syntax.

This section calls them evidence fields; diagnostic messages call them witness fields (E1114 /
E1115). Both terms refer to the same kind of field.

> **Design basis**: [RFC-027 §14](../../rfc/accepted/027-compile-time-evaluation-types.md) (complete
> semantics of dependent record Σ),
> [RFC-027 §4.1](../../rfc/accepted/027-compile-time-evaluation-types.md) (sort registration for
> logical signatures).
>
> **Implementation status**: Field quantity representations, runtime erasure, and immutable
> witness-bearing records (5A) have been implemented (WBS 6.9.5 B.1). Evidence classification,
> construction-site proof obligations, type-level field paths, and cross-module export will be
> implemented in phases under 6.9.5 B.2–B.4. Before classification is implemented,
> proposition-shaped fields are handled as ordinary data fields—they occupy data slots and
> participate in constructor argument counting.

#### 8.3.1 Field-Position Rules: Proof-Space Criterion

Let the fields of record `R` be `f₀, f₁, …, fₙ₋₁` in source order, with the type of field `fᵢ` being
`Tᵢ`. Classification has three steps: **first determine the space (proposition family or data
family), then determine argument position (preceding / self / forward), and finally determine the
field category**.

**First step: Determine the space.** Normalize `Tᵢ` and take the space registered for its result
type family under the current logical signature:

- If the space is `Proof` (a proposition/refinement type/`Assert`/`Terminates`/the result of a proof
  family), enter evidence classification;
- If the space is `Data` (`Int`, `Array`, `Vec`, an ordinary record, or an **application of a data
  type family**), it is a **data field**, regardless of how many preceding fields it references.

The `n` in `data: Array(Int, n)` references a preceding data field and changes the “length of the
data slot”; it does not “generate a proof.” Classifying it as evidence would erase the buffer from
the runtime layout—this is the misclassification that this rule is intended to eliminate.

**Second step: Determine the position** (applies only to fields whose space is `Proof`). There are
three cases according to the positions referenced by the arguments:

- All arguments are **strictly preceding data fields** `fₕ (h < i)`, compile-time constants,
  compile-time value parameters, or type parameters ⇒ the third step classifies it as evidence (a
  compile-time value parameter takes a concrete constant after instantiation and is counted as a
  compile-time constant; §8.3.3);
- The argument **is `fᵢ` itself** (a self-referential refinement such as `v: Pos(v)`) ⇒ the third
  step classifies it as a **data field (admission)**: the field carries a value, and its constraint
  only checks the value itself rather than proving a fact elsewhere;
- A forward reference to a later field, a reference to another evidence field, or using `fᵢ` itself
  as an evidence argument ⇒ rejected at declaration time.

**Third step: Determine the category.**

| #   | Shape                                                                                                                                                                     | Classification                                                      | Runtime Layout                                                                                                           |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| 1   | Space `Proof`; all arguments are strictly preceding data fields / compile-time constants / compile-time value parameters / type parameters (positive example `w: Pos(v)`) | **Evidence field**                                                  | Occupies no slot; `base` does not enter the runtime layout                                                               |
| 2   | Space `Data`: ordinary value types (`n: Int`), data type family applications (positive example `data: Array(Int, n)`), and ordinary records                               | **Data field**                                                      | Occupies a data slot                                                                                                     |
| 3   | Space `Proof`, but an argument is `fᵢ` itself (self-referential refinement `v: Pos(v)`)                                                                                   | **Data field (admission)**                                          | Occupies a data slot; the constraint is checked at the construction site under the existing refinement obligation (§8.4) |
| 4   | Space `Proof`, but there is a forward reference to a later field, a reference to another evidence field, or `fᵢ` itself is used as an evidence argument                   | **Illegal**: rejected at declaration time, with no silent downgrade | Produces no field                                                                                                        |

The four rows are not conflated: row 2 is “ordinary value type/data family always means data field”;
row 3 is “proposition family with a self-reference is an admission data field”; and row 4 is
“proposition family with an invalid position is always rejected at declaration time.” **There is no
path in which “a forward reference also counts as a data field”**—a forward reference neither
occupies a data slot nor enters the runtime data index.

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }     // Space Proof

Box: Type = {                         // Positive example: w is evidence
    v: Int,          // Data field
    w: Pos(v),       // Evidence: Pos is a Proof family, and argument v is a strictly preceding data field
    z: Int,          // Data field; runtime index = 1 (skipping w)
}

Buf: Type = {                         // Negative example: data is still a data field
    n: Int,                 // Data field
    data: Array(Int, n),    // Data field: Array is a Data family, and n only determines length; it does not generate proof
}

Admission: Type = {                   // Self-referential refinement: data field, not evidence
    v: Pos(v),           // Proof family, but the argument is itself ⇒ admission (row 3 of the table)
}
```

**Evidence fields may depend only on strictly preceding data fields.** Their arguments must not
reference another evidence field, a later field through a forward reference, or the field itself.
This constraint guarantees that, when construction reaches such a field in declaration order, all
arguments already have compile-time values and the proposition is closed and can be handed to the
proof kernel for verification.

**`base` does not enter the runtime layout.** The base type of an evidence field (the `base` of a
refinement type) is type information used only for kernel equality checking and normalization; a
runtime slot is never allocated for such a field. Evidence fields also have no default values—they
have no runtime value to supply.

#### 8.3.2 Construction-Site Obligations and Proofs

**There is only one source-level construction form: data arguments.** Evidence fields **do not
occupy any constructor argument position**—there is neither a form that “passes a witness as a
value” nor an “evidence argument position.” Constructor arity counts only data fields:
**compile-time arity = runtime arity = number of data fields**.

**Evidence is not a constructor argument; it comes from proof obligations automatically generated at
construction sites.** When constructing a record, the compiler generates one proof obligation for
each evidence field:

1. Take the compile-time values of the strictly preceding data fields referenced by the field's
   arguments;
2. Substitute them into the field's proposition to obtain a closed proposition (`Pos(3)` reduces to
   `3 > 0`);
3. Have the proof kernel verify it under the current logical signature (RFC-027 §4).

The result is three-valued:

| Result    | Compile-Time Behavior                                                                           |
| --------- | ----------------------------------------------------------------------------------------------- |
| Proved    | Passes; the certificate is retained at compile time and erased at runtime                       |
| Disproved | Compile error + counterexample (**E4018**)                                                      |
| Unproven  | Compile error + unresolved proposition + reason + suggestion for supplying evidence (**E2031**) |

**Unproven does not mean that the proposition is false**, nor does it permit the program to pass.
Evidence is supplied through an **independent declaration**, not by rewriting the construction
expression: write an **independent proof function** whose return type equals the proposition, or
register a **named proof** in a proof library and reference it from the construction obligation
(RFC-027 §4.3). There is no form that “fills in an incorrect witness value”—evidence fields have no
argument position at construction sites. Passing an extra evidence argument at source level
(`Box(3, 5, evidence)`) does not match any data parameter slot and reports an argument error in the
E1010 / E1014 family; it is not silently ignored.

A self-referential refinement field (`v: Pos(v)`) is a data field: its constraint is checked at the
construction site under the existing refinement obligation (§8.4), and it produces no evidence
obligation or change to the field's runtime index.

#### 8.3.3 Generic Records: Proof Family Parameter `P: T -> Proposition`

A generic record may have a **proof family parameter** `P`—its kind is `T -> Proposition`: it
accepts a type parameter and returns a proposition. `Proposition` is a sort alias for the `Proof`
space in the logical signature (RFC-027 §4.1), not a source-level type name. Its source-level form
is identical to an ordinary type-level function parameter, and kind constraints are checked at
declaration and instantiation:

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Cell: (T: Type, P: (T) -> Type) -> Type = {
    v: T,        // Data field
    w: P(v),     // Evidence: P is a Proof family, and argument v is a strictly preceding data field
}
```

- The kind of `P` comes from the same source as the criterion in §8.3.1: **the result space must be
  `Proof`**. Passing a data type family (such as `Array`) to `P` violates the kind and is rejected
  directly at declaration or instantiation—this is a type error, not a case where “`w` degenerates
  into a data field.”
- `P` is a compile-time parameter position and does not occupy a runtime slot. When `Cell(Int, Pos)`
  is instantiated, `P` is bound to the proof family `Pos`, the proposition for `w` becomes `Pos(v)`,
  and evidence classification proceeds exactly as in §8.3.1.
- Classification is rerun **after instantiation**.
- When `P` is unbound (the generic has not been instantiated), the result space for `w` remains
  `Proof` (the kind has been declared), but the proposition is open, so the construction site
  returns Unproven—it is neither silently permitted nor downgraded to a data field.

#### 8.3.4 Cross-Module Export

When a record containing evidence fields is used across modules, its propositions and evidence are
exported together with the type definition, and the importer does not re-infer them from field
names. The exported artifact carries: the kinds of type family parameters (including proof family
parameters), the proof obligation for each evidence field, and an evidence slot for each evidence
field—the proposition, its explicit evidence source or verified certificate, and the hash of the
logical signature that produced it.

The importer verifies these propositions using **its own current logical signature**: when signature
hashes differ, RFC-027 §4.1's three explicit paths apply—translation items, bridging axioms, or
reproof under the importer's signature. The importer must not directly reuse the exporter's
conclusions.

A referenced proof-library entry must appear in the importer's library list; otherwise the
conclusion is Unproven, and the diagnostic suggests registering the library or writing a bridging
axiom.

#### 8.3.5 Type-Level Use and Value-Level Rejection

Evidence fields **may only be used at the type level**. A `b.w` in a type position is
**user-writable type-level syntax** referring to the field's **evidence item** (proof item), not a
runtime value: it points to the evidence reference registered for the field and may serve as a
premise for another proposition, which the proof kernel replays under the current logical signature.

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

Box: Type = {
    v: Int,
    w: Pos(v),
}

// Type-level use: treat b.w as an evidence premise, not as a value read
// `Proved` here denotes a proposition constructed "with this evidence item as a premise"
useWitness: (b: Box) -> Type = { Proved(b.w) }
```

**Value-level reads of evidence fields are always rejected**: reading `b.w` in a value expression
reports **E1114** (`witness_field_not_readable`). A field path is not a value term, and neither the
evaluator nor the runtime representation materializes a value for it. When runtime data is needed,
define the field as an ordinary data field.

```yaoxiang
b = Box(3)
x = b.w        // ❌ Value-level read of a witness field → E1114
```

> **Design basis**: [RFC-027 §14.5](../../rfc/accepted/027-compile-time-evaluation-types.md) (term
> shape for type-level use and replacement of 4A's “no user syntax at the type level”). The syntax
> and parsing of type-level field paths have not yet been implemented—the current parser does not
> accept field paths in proof positions, and this form will be connected in 6.9.5 B.2–B.3.

#### 8.3.6 Immutability, Erasure, and Diagnostic Codes

**5A: A record containing evidence fields is wholly immutable.** Data fields must not declare `mut`,
must not be assigned, must not be borrowed with `&mut`, and must not be receivers of mutating
methods (including automatic mutable borrows). Otherwise an evidence proposition could become
invalid because a field was rewritten. Violations report **E1115** (`witness_record_immutable`).
Records without evidence fields are not subject to this restriction.

**Erasure.** Evidence fields do not occupy runtime slots, and the runtime layout contains only data
fields; proof certificates are retained at compile time and completely erased at runtime (RFC-027
§13). Runtime data indices count only data fields—in the record `{ v, w, z }`, the data index of `z`
is 1 (skipping witness field `w`). Runtime-mode boolean checks (§8.4) are unrelated to evidence—they
check runtime values, not certificates.

**Division of diagnostic responsibilities** (each code handles one aspect and does not replace
another):

| Code      | Trigger                                                                         | Semantics                                                                  |
| --------- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| **E4018** | An evidence proposition at a construction site is disproved                     | A refinement predicate is violated; a counterexample is attached           |
| **E2031** | An evidence proposition at a construction site is Unproven (refinement channel) | The refinement constraint cannot be proved; explicit evidence is suggested |
| **E1114** | Value-level read of an evidence field                                           | Evidence fields cannot be read at the value level                          |
| **E1115** | Mutable use of an evidence record                                               | The entire record is immutable                                             |
| **E4023** | General Unproven outside the refinement channel                                 | General proof failure; the proposition, reason, and location are included  |

Invalid positions at declaration time—space `Proof` combined with a forward reference to a later
field, a reference to another evidence field, or a proof family parameter `P` whose kind is a data
type family—are rejected directly at the declaration. They do not proceed to the construction site
and are not silently downgraded to data fields. A data type family application in space `Data` and a
self-referential refinement in space `Proof` are both valid data fields and are not invalid
positions. When a generic remains uninstantiated, the proposition is open and the construction site
returns Unproven (§8.3.3).

### 8.4 Assert Refinement Types and assert Assertions

`assert` and `Assert` are two sides of the same refinement primitive—the dispatch routing pipeline
automatically chooses between them based on whether the predicate's free variables are accessible at
compile time.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                           |
| ----------------------------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enters the proof pipeline: true → erased to Void; false → compile error (Never is uninhabited)     |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Inserts a runtime Bool check and injects refinement facts into the flow-sensitive assumption set Γ |

Dispatch is **checked according to declared strength** (§7.1) at the predicate position. The same
principle covers the entire strength ladder: classical types without a strength declaration do not
enter the proof pipeline and receive no quantity inference; predicate annotations (this section),
q=0 slots (§7.1), dependent record fields (§8.3), and relational predicates each perform the checks
appropriate to their declared form. Cost follows declared strength rather than global analysis.

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: the old assumption is invalidated
```

After assigning to a `mut` variable, all assumptions involving that variable are removed (kill set).
When branches are joined, Γ is the intersection of the branches.

### 8.5 Terminates: Termination-Measure Predicates

`Terminates` is a **built-in predicate** at the same level as `Int` and `Never` as a core primitive
(a built-in name, not a keyword). It binds a **measure** to a computation, declares that the
computation terminates, and provides a witness of termination.

**Forms**: two arities for the same predicate:

| Form                    | Anchor                 | Use                                                                                                                       |
| ----------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Binding name           | Default form—self-recursive functions, loops                                                                              |
| `Terminates(FnType, m)` | Explicit function type | When the measure's ownership must be stated explicitly (the measure is defined elsewhere or serves multiple computations) |

```yaoxiang
// Measure: an ordinary function; independently testable and reusable; does not participate at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere, with explicit ownership
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, and the measure is an expression in scope
// The loop body is a `{}` block whose value is given by its tail expression (spec §2.9): the body has a value only if its tail contains an expression
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines the value type of the computation annotated at its type
position. The obligation falls on that computation—for functions, on each recursive call site; for
loops, on each back edge—and requires that “the next state's measure is strictly less than the
current state's measure,” evaluated under that point's path guard.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back edge is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: A loop is an anonymous construction and usually cannot be referred to
> by name. The binding name is the name—in `acc: Terminates(n - i) = while ...`, `acc` provides the
> anchor, making the loop referable. This is why the unary form of `Terminates` applies to loops.

**Measure**: The return type is unrestricted (a natural number is not required); “strict decrease”
over it is determined by a well-founded order available on that type. Whether the measure is
well-founded (for example, whether it is `>= 0` when its type is `Int`) is an **independent
obligation**, evaluated by the compile-time proof pipeline just like the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types**—as soon as a type is refined,
verification mode is entered. Ordinary types that have not been refined (such as a bare `while` loop
or a function without a refined signature) do not enter verification mode and generate no
termination obligation.

**Automatic exploration first**: The compiler first explores measures automatically (four templates:
linear ranking functions, predicate-violation counts, bounded increment/decrement, and
multiplicative scaling), and an explicit `Terminates` is needed only when automatic exploration
fails.

**Runtime representation**: A purely compile-time entity, erased along with its witness and absent
from the runtime binary.

> For the complete design, see
> [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../rfc/accepted/027a-termination-explicit-measure.md) (implementation mechanism).

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

**Syntax**: The type intersection `A & B` denotes a type that satisfies both A and B.

```yaoxiang
// Interface combination = type intersection
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
// A struct containing a heap-buffer field does not derive Dup (the field falls under Move) → Move by default
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move; b cannot be read again
```

### 11.2 Dup (Shallow Copy: Copy the Handle, Share the Data)

**The Dup attribute is used for reference/token types and internally reference-counted handle value
types**. Assignment of a Dup type is a shallow copy—the handle/token is copied while the underlying
data is shared. Multiple holders refer to the same data.

| Type            | Attribute | Description                                                                                |
| --------------- | --------- | ------------------------------------------------------------------------------------------ |
| `&T`            | Dup       | Zero-sized read token; copying the token provides multiple views of the same data          |
| `ref T`         | Dup       | Rc/Arc copy = increment the reference count and share heap data                            |
| String, Bytes   | Dup       | Internal reference counting; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear    | Zero-sized write token; exclusive and non-duplicable                                       |
| struct          | Derived   | All fields are duplicable (primitive value types ∪ Dup) → Dup; otherwise Move (#398)       |
| tuple           | Derived   | Determined element by element, following the same rule as structs (#398)                   |
| All other types | Move      | Default ownership transfer                                                                 |

**Primitive value types** (Int, Float, Bool, Char) receive special treatment from the compiler:
values are copied automatically on assignment, and the two values are completely independent. This
is native compiler behavior and is not part of the Dup type attribute.

**Derivation rules** (finalized in #398)—the rule “automatically derive Dup when all fields are Dup”
cannot be applied literally: primitive fields such as Int are not themselves Dup, so
`{ x: Int, y: Int }` would be misclassified as Move. The executable form is:

1. The **duplicable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup
   (`&T`, `ref T`, String / Bytes, function values (#352), and composite types that already derive
   Dup);
2. **struct**: If all fields belong to the duplicable field set, derive Dup; if **any** field falls
   under Linear (`&mut T`) or Move (a nested Move struct / containers such as Vec and Dict /
   resources), the entire struct remains Move (no intermediate “partially duplicable” state);
   evidence fields have no runtime value and do not participate in derivation (§8.3);
3. **tuple**: Follows the same rule as structs, determined element by element; an empty tuple (unit)
   is `Void`;
4. **Derivation is recursive**: When a field has a named type (such as `target: Point`), its
   definition is expanded before classification, and `A = { b: B }` follows B's derivation result;
   cyclic aliases conservatively fall under Move at the depth limit;
5. **Outside the derivation scope** (still Move, handled separately): containers (Vec / Dict / Set /
   Option / Result / Array) and enums.

```yaoxiang
// &T: Dup; arbitrary aliases are permitted
view: &Point = &p
view2 = view     // Dup: copies the token; both remain valid
print(view.x)    // Usable
print(view2.x)   // Usable

// &mut T: Linear; cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup and cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship to Dup

**Clone** is an explicit deep-copy interface. All types can implement Clone, providing a `.clone()`
method.

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

|                        | Dup                                                               | Clone                                         |
| ---------------------- | ----------------------------------------------------------------- | --------------------------------------------- |
| **Semantics**          | Shallow copy: copy the handle/token and share the underlying data | Deep copy: create a complete independent copy |
| **Invocation**         | Implicit (automatic on assignment/argument passing)               | Explicit (`.clone()`)                         |
| **Effect of mutation** | Changes are mutually visible (shared underlying data)             | Changes are independent (separate copies)     |
| **Applicable types**   | `&T` tokens, `ref T`                                              | Any type implementing the Clone interface     |
| **Cost**               | Zero overhead (tokens are zero-sized types)                       | Depends on the type                           |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copies the token and shares the underlying data
view: &Point = &p
view2 = view        // Dup: copies the token; both refer to the same p
print(view.x)       // Usable
print(view2.x)      // Usable; sees the same data

// Primitive value type: the compiler performs value copying automatically (not Dup)
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

- Dup is used for tokens/reference types, solving the problem of “multiple views of the same data.”
- Clone is used when an independent copy is required; an explicit call makes the cost visible.
- Copying primitive value types (Int/Float/Bool/Char) is built-in compiler behavior and is not part
  of Dup.
- User-defined types default to Move (zero-copy, high performance); Dup is derived automatically
  when all fields are duplicable (§11.2 derivation rules).

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
- No dedicated borrow checker is needed—the type attributes (Dup/Linear) naturally infer
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

// Call side: the compiler automatically selects borrowing or Move
p = Point(1.0, 2.0)
p.print()                       // The compiler automatically creates an &Point token
p.shift(1.0, 1.0)               // The compiler automatically creates an &mut Point token
p.print()                       // OK; the previous token was released when the shift call ended

// Multiple &T tokens coexist—Dup types allow unrestricted copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types and therefore support all operations available to ordinary types:

**Returning tokens**—tokens propagate together with return values:

```yaoxiang
// ✅ The child token is returned together with the parent token
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // The tokens are returned to the caller
print(px_ref)                    // OK; the tokens remain in scope
```

**Storing tokens in structs**—a struct may carry token fields:

```yaoxiang
// ✅ A struct carries tokens as fields
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

> Note: After a closure (function value) escapes, the scope of its definition may no longer be
> alive, so it must not implicitly capture outer variables. However, the scope at the call site
> (creation point) necessarily remains alive, so fixing the context there as a value inside the
> closure is safe.

### 12.4 Automatic Borrowing Selection

At the call site, the compiler automatically selects according to the following priorities:

```
1. 如果实参后续还有使用 → 优先创建令牌（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // The parameter type of print is &Point → the compiler creates an &Point token
p.shift(1.0, 1.0)  // The parameter type of shift is &mut Point → the compiler creates an &mut Point token
p2 = p             // Not used afterward → Move
```

**Method receivers follow signature semantics** (following the receiver-spelling convention of
RFC-011a): a receiver of `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value →
Move (consumes the receiver). Borrow tokens created at a call site are released when the call ends
(transient; interval semantics in §12.5). An interface's borrow receiver must be explicitly declared
by the interface author as `&Self`, and after the impl signature is substituted with
`Self ↦ impl type`, it must exactly match the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is the **borrowing higher-order proposition** (RFC-009a), not an
independent flow-sensitive analysis. The compiler automatically generates borrowing propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and a derived &T cannot be live simultaneously
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
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken is still usable
}

// ❌ The same argument cannot simultaneously create an &mut token and another token
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives both an &mut token and an & token
```

### 12.6 Compiler Internals: The Branding Mechanism

Users never interact with brands. Internally, the compiler assigns each token a compile-time unique
identifier:

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N 是编译期唯一整数
&mut Point     →  WriteToken(Point, #M)   // #M 是编译期唯一整数
```

Purposes of brands:

- **Anti-forgery**: Tokens can be obtained only from an owner capsule and cannot be fabricated from
  nothing.
- **Association tracking**: A derived `&Float` from a field access carries a derived brand
  (`#N.field_x`), allowing the compiler to trace it to the parent token.
- **Conflict detection**: A WriteToken and a derived ReadToken from the same source cannot be live
  simultaneously.

Brands disappear completely after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken (freezes source data → safe to Dup)
               | &mut T      // WriteToken (exclusive read/write → Linear)
```

### 12.8 Borrow Tokens vs ref

|                 | `&T` / `&mut T`                                               | `ref`                                              |
| --------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Purpose         | Look at / modify in place                                     | Shared ownership                                   |
| Scope           | Follows the scope of the token value                          | Crosses scopes                                     |
| Cost            | Zero overhead (zero-sized type; disappears after compilation) | Rc or Arc (selected by the compiler)               |
| Escaping        | Permitted (tokens propagate through return values/structs)    | Intended for escaping                              |
| Cross-task      | Not permitted (tokens do not implement cross-task transfer)   | Permitted (the compiler automatically selects Arc) |
| Cycle detection | Not applicable                                                | Silent within a task; a lint across tasks          |

> Note (undefined): How to read the contents after creating a ref (dereference/method/automatic
> behavior) has not yet been defined in the specification. The current implementation reports E1052
> for `*a`. This will be added to the section once defined.

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definitions

```
// === 记录类型（花括号） ===

// 记录类型
Point: Type = { x: Float, y: Float }

// 带变体的记录类型（使用函数字段）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === 依赖记录 Σ（evidence 字段，见 §8.3） ===

// 编译期谓词 + evidence 字段：w 不占构造参数位与运行期数据槽
Pos: (v: Int) -> Type = { v > 0 }
Box: Type = { v: Int, w: Pos(v) }

// === 接口类型（花括号，字段全为函数） ===

// 接口定义
Serializable: Type = { serialize: () -> String }

// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // 实现 Serializable 接口
}

// === 函数类型 ===

Adder: Type = (Int, Int) -> Int

// === 终止测度（内置谓词，见 §8.5） ===

// 一元：锚点即绑定名（自递归函数、循环）
// 体尾须给出值——`while` 的值 = 循环体块的值 = 尾表达式（spec §2.9）
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}

// 二元：显式指明测度归属（测度定义在别处）
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 Generic Syntax

```
// 泛型类型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 泛型函数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 类型约束
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 关联类型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// 编译期泛型：N 在类型位置 (k: N) 被引用 → 编译期值参数
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// proof family 参数：kind 是 T -> Proposition（结果空间必须是 Proof）
Cell: (T: Type, P: (T) -> Type) -> Type = { v: T, w: P(v) }

// 条件类型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 函数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Attribute Quick Reference

```
// === Move（默认） ===
// 所有类型默认 Move。赋值、传参、返回 = 所有权转移

// === 原语值类型（编译器内置） ===
Int, Float,     // 赋值时自动值复制，两个值完全独立
Bool, Char      // 不是 Dup，是编译器对原语的内置处理

// === Dup（浅拷贝：复制句柄，共享底层数据） ===
&T              // 零大小读取令牌，复制令牌 = 多个视角指向同一数据
ref T           // Rc/Arc 复制 = 引用计数+1，共享堆数据
String, Bytes   // 内部引用计数，复制句柄共享底层 buffer
struct / tuple  // 派生：所有字段 ∈（原语值类型 ∪ Dup）→ Dup（#398）

// === 不派生（保持 Move） ===
Vec/Dict/Set    // 容器与 enum 不在派生范围（#398）

// === Linear ===
&mut T          // 零大小写入令牌，Linear（独占，不可复制）

// === Clone（显式深复制） ===
value.clone()   // 创建独立副本，修改不影响原值
```

### A.4 Borrow Token Quick Reference

```
// === 借用令牌 ===
&T              // 零大小编译期读令牌，冻结源数据 → Dup（可复制）
&mut T          // 零大小编译期写令牌，独占读写 → Linear（不可复制）

// 调用端自动选择
// 1. 实参后续还有使用 → 创建令牌
// 2. 实参后续不再使用 → Move
// 3. 优先匹配：&T < &mut T < Move

// 令牌传播
// ✅ 可返回、可存结构体、可被闭包捕获
// ❌ 不可跨任务（令牌未实现跨任务传递）
```
