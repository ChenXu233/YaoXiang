# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, compound types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
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
| False \(\bot\)                                 | `Never` (zero constructors, no value can inhabit it) |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell's paradox) |
| Case analysis                                  | Type-level `match`                                   |

> **Note**: A type-level `match` is case analysis, not mathematical induction. Induction requires a
> type-level recursive function and compiler termination checking.

### 0.2 Types Are Propositions, Programs Are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to a correct constructive proof**. YaoXiang type
  families, such as case analysis on `Nat` in `Add` followed by recursive calls, are fundamentally
  type-level encodings of mathematical induction—provided that the compiler can perform termination
  checking.
- **Type checking is proof verification**. When a program passes type checking, a logical
  proposition has been constructively proved.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard correspondence in YaoXiang are:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox
   caused by `Type: Type` (Girard's paradox).
2. **Type families** (RFC-011): type-level case analysis and recursive calls over the natural-number
   type `Nat(Zero/Succ)` correspond to the Peano axioms—provided that the compiler performs
   termination checking.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to the finite
   quantification “for every integer N, there exists a type.”

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

> **Design note**: Although RFC-010 proposed a unified “everything is an assignment” model
> (`name: type = value`), types and values must still be distinguished at the syntax level. In the
> compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`). As a BNF placeholder, `TypeExpr` corresponds to the `Type` enum in the
> implementation and means “a type is expected in this position.”

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

**There are 15 built-in type names; this section is the authoritative list**: `Never` / `Void` /
`()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` /
`Char` / `String` / `Bytes`. The single source of truth in the implementation is
`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`). This table matches it **name for
name**—the T2 gate in `tools/type-tables --report` compares this table, `CONST_PARAM_TYPES`, and the
LSP type-name list in all three directions.

| Type      | Logical Correspondence | Description                                                                               | Default Size |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| `Type`    | —                      | Meta type (not a value type name; see below)                                              | 0 bytes      |
| `Never`   | ⊥ (false/empty type)   | Zero constructors; no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`    | ⊤ (true/Unit)          | Has a default void value; a zero-field product type. `x: Void = <默认>` is valid.         | 0 bytes      |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                           | 1 byte       |
| `Int`     | —                      | Signed integer (= `Int64`)                                                                | 8 bytes      |
| `Int8`    | —                      | 8-bit signed integer                                                                      | 1 byte       |
| `Int16`   | —                      | 16-bit signed integer                                                                     | 2 bytes      |
| `Int32`   | —                      | 32-bit signed integer                                                                     | 4 bytes      |
| `Int64`   | —                      | 64-bit signed integer                                                                     | 8 bytes      |
| `Float`   | —                      | Floating-point number (= `Float64`)                                                       | 8 bytes      |
| `Float32` | —                      | 32-bit floating-point number                                                              | 4 bytes      |
| `Float64` | —                      | 64-bit floating-point number                                                              | 8 bytes      |
| `String`  | —                      | UTF-8 string                                                                              | Variable     |
| `Char`    | —                      | Unicode character                                                                         | 4 bytes      |
| `Bytes`   | —                      | Raw bytes                                                                                 | Variable     |

`()` is the **type notation** for `Void` (a zero-element tuple = the unit type) and is synonymous
with `Void`: it is written as `() -> Void` in standard-library signature strings, and the parser
table places `()` and `Void` in the same arm.

**There is only one spelling**. `Int` is not `int`; `Int64` is not `int64`/`i64`; `String` is not
`string`/`str`; `Bool` is not `bool`; `Void` is not `void`; `Never` is not `never`; `Float` is not
`float`/`f64`; `Char` is not `char`; and `Bytes` is not `bytes`. The implementation once coexisted
with 22 such lowercase or abbreviated spellings, all of which were removed in **WBS 6.1.2 (finalized
2026-10-27)**: using a noncanonical spelling in a type position always reports an unknown type and
is never silently resolved to a user-defined type.

> **`DateTime` is not a built-in name**. It is a **semantic alias in the `std` layer**: the `now` /
> `parse_time` signature strings in `src/std/time.rs` and `src/std/interfaces/time.yx` still use
> `DateTime`, but at runtime it is `Int` (fixed as part of #338). The parser table retains it only
> as a **temporary compatibility alias**, and this table does not include it. Migrating it out of
> the built-in name table and letting the `std` layer provide it belongs to **P10 S11** (issue
> [#513](https://github.com/ChenXu233/YaoXiang/issues/513)).

> **There are no unsigned integer types**. `Uint` (and `Int128`) are not valid type names—`MonoType`
> has no unsigned variant. The LSP type-name list and the name table used by `sizeof` each once
> contained a residual entry, both of which were removed with WBS 6.1.2. If unsigned semantics are
> required, use `Int` with a lower-bound convention.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system, corresponding respectively to
false (⊥) and true (⊤).

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. There is no
   right-hand side to write for `x: Never = ...`.
2. **Principle of explosion**: `Never <: T` holds for every type `T`. `assert(false)` returns
   `Never`, after which the code can pass type checking, although that point is never reached.
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead-code analysis and `match` branch confluence.

`Never` is a built-in type name, registered through the same path as `Int`/`Bool`; it is not a
keyword.

**Void (⊤, true/Unit)** — exactly one inhabitant (the default void value). `Void` is the identity
element of the zero-field product type. `x: Void = <默认>` is valid. The value of a block is given
by its **tail expression** (an empty block `{}` is `Void`). See
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
- A field name is followed directly by a colon and a type.
- An interface name written in the type body indicates that the type implements that interface.

> **Namespace ownership**: A `Type.name` prefix, such as `Point.draw`, indicates that the function
> belongs to the `Point` namespace. It does not trigger any implicit binding. For `.`-style call
> syntax such as `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See
> RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, which may optionally be supplied during construction:

```yaoxiang
// Field with a default value - optional during construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Use
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without a default value - required during construction
Point2: Type = {
    x: Float,
    y: Float
}

// Use
Point2(x=1, y=2) // Correct
Point2()          // Error
```

**Rules**:

- `field: Type = expression` -> has a default value and is optional during construction.
- `field: Type` -> has no default value and is required during construction.

#### 3.1.2 Builtin Bindings

Methods may be bound directly within a type definition:

```yaoxiang
// Method 1: Bind by referencing an external function
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

**Direct interface assignment**: A concrete type can be assigned directly to an interface-typed
variable (structural subtyping).

```yaoxiang
// Direct assignment (concrete type is known at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be known at compile time -> vtable call)
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

**Coherence and the orphan rule (not applicable; closure explanation)**: YaoXiang interfaces are
structural types (an interface is a record whose fields are all function types), not nominal traits.
There is therefore no cross-crate or cross-module ownership question of “who may implement what for
whom.” Rust-style orphan rules and coherence checks have no applicable target (see RFC-011 §2.1 for
the decision record). The corresponding safeguard in a structural world is
**duplicate-implementation rejection**: defining the same method signature more than once on a type
causes a compilation error (RFC-011a §3; overriding is forbidden, while overloading is legal).

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

**Two parameter forms, with different ownership of names** (the type side of the RFC-007 shorthand
rules):

- **Named** (`a: Int`): The name enters the contract and can be referenced by named arguments
  (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)). The lambda header may also be omitted when
  the signature already declares parameter names and types (`add: (a: Int, b: Int) -> Int = a + b`,
  RFC-007 §Shorthand Rules).
- **Unnamed** (a bare type such as `(Int, Int) -> Int`): Type constraints apply **positionally**,
  and the name belongs to the implementation. The lambda header must provide its own parameter names
  (`mk: (Int, Int) -> Int = (x, y) => x + y`; RFC-007 allows the lambda header to be omitted if and
  only if the signature already declares parameter names). Such parameters cannot be referenced by
  named arguments and can only be called positionally. Bare identifiers are resolved in the type
  namespace; if no declared type can be resolved, an error is reported—there are no bare identifiers
  in parameter positions that are neither types nor type parameters.

**Binding semantics of annotations**: An annotation is a declaration, not a comment. Binding
statements check the implementation against the annotation: parameter types drive the lambda header
positionally, while the function body's exits (the tail expression and `return`) are uniformly
checked against the annotated return type. The type registered for the caller is the annotated form.
Unannotated bindings use HM inference (RFC-007). `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are rejected under the same rule—binding semantics do not diverge
according to how the value is written (lambda / block / expression / named function reference).

**Implementation-header parameters are positional binders** (`f: (a: Int) -> Int = (b) => b + 1`):

- **Names and types follow the declaration**: The i-th implementation-header parameter corresponds
  to the i-th position in the **first N positions of the value-level signature** (N = the number of
  implementation-header parameters; the value-level signature = the parameter sequence after
  compile-time positions—`T: Type`, type-level function positions, and const positions—are removed
  from the declaration). The implementation header only declares “what the i-th parameter is called
  inside the body”; renaming it has no effect on the call site. The available name in `f(a = 7)`
  comes from the declaration, not from the implementation header.
- **Annotations in the implementation header are used only for comparison**: A type mismatch at the
  same position reports **E1011**. The declaration is authoritative, and an author's annotation must
  not be silently overridden.
- **More implementation-header parameters than the declaration's runtime parameter positions**
  report **E1112**. The extra positions have no values to bind; type positions (`T: Type`),
  type-level function positions, and const positions do not occupy runtime parameter positions.
- **The reverse case—fewer parameters in the implementation—is not an error** (it occupied the
  “pass” cell of the t1 behavior matrix before D54 and must not be reclassified): The type level
  still checks arity according to the **declaration** (`f(1)` is legal, while `f(1, 2)` reports
  E1010). Missing positions are supplied positionally by the declaration into the **type-checking
  scope**. However, the implementation header has no corresponding runtime slots, so arguments at
  those positions are discarded on the IR side and do not participate in computation in the body (in
  `f: (a: Int) -> Int = () => 1`, `f(1)` is unrelated to the argument). **This has been recorded as
  a gap**—making the runtime also accept the declared arity would be a D-class behavior change and
  requires a separate decision.
- **Unified exits**: Whenever the tail expression or `return` (the arrow sugar `=> x` and explicit
  `return x` are two AST shapes) differs from the declared return position, **E1012** is reported at
  the **annotation**.

> **Implementation status**: The binding-period comparison for `Fn` annotations has been implemented
> (WBS 6.8.2, 2026-11-02): parameter positions drive alignment (the parser and type checker use
> **the same alignment rule**: the first N positions of the value-level signature) /
> implementation-header annotations are compared (E1011; the author's annotation is retained, and
> any mismatch with the declaration is an error) / exits are unified (E1012) / the registered form
> follows the declaration (the available names for named arguments).

---

## Chapter 4: Generics

### 4.1 Generic Parameter Syntax

Generic parameters are part of a function type and uniformly use the `()` syntax:

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

Container types are generic type constructors, not built-in primitives. They receive the same
treatment as user-defined generics and are handled through a unified generic-instantiation path.
Ownership of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                                  | Foundation                               |
| ------------- | ------------- | ------------------------------------------ | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)       | Core primitive (stack/inline first)      |
| `Vec(T)`      | Runtime value | Raw buffer with a runtime length; growable | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard-library type (growable list)      | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                          | `HeapValue::Dict`                        |

> `List(T)` is a **standard-library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list` and receives the same treatment as a user-defined generic record. All
> growable-semantics policies—when to expand, by how much, and whether sharing is possible—live in
> the library; the compiler does not participate. `Vec(T)` is the minimal foundational primitive on
> which it depends.
>
> Set(T) has been retired: it has no literal, runtime representation, or `std.set`. If the
> requirement arises, complete it following the Dict pattern.

Key rules:

- **Literal placement is determined by context**: A bare `[...]` literal with a `List(T)` annotation
  becomes a growable list; an `Array(T, N)` annotation applied directly to a literal produces a
  fixed-length array. Placement validation requires that the number of elements equal N and that
  their types be compatible with T; otherwise, compile-time E1002 is reported. When N is a symbolic
  constant (a const parameter), count validation is deferred to the refinement-type phase.
- **Implicit List→Array conversion is forbidden**: Fixed length is guaranteed by the type
  layer—`push` accepts only a `List(A)` receiver.
- **Performance hierarchy**: From bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index-failure contract** (runtime errors are transitional; the target state is compile-time
  refinement through value-dependent types, see §8.4):
  - Out-of-bounds index, including a negative index → `E6003`
  - Missing Dict key → `E6008`
- **String indexing semantics** (#385): `s[i]` indexes characters by **Unicode scalar value (code
  point)** and produces a single-character `String`. This is the same unit used by `std.string`'s
  `len` / `index_of` / `substring` / `chars` / `char_code`, so combinations do not become
  misaligned. Out-of-bounds access, including negative indices, follows the index-failure contract
  above: `E6003`.
- **The membership `in` predicate**: Returns `Bool` without reporting an error. Its right operand
  covers List/Array/Dict(keys)/Tuple/String/Range. It is a first-class Horn predicate and the basis
  for propositions provable at compile time in refinement types.`

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
`residual` / `from_error`). On failure, the current function returns early with the result of
`from_error(residual(t))`; on success, the expression's value is `success(t)`. The enclosing
function's return type must also implement `Try`, and its failure residual type must agree with the
receiver's (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` receive `Try`
implementations from `std.result` / `std.option` (`Option`'s failure residual type is `Void`).
User-defined union types can opt in by writing `Try(Self, T, E)` in the type body. The lowering of
`?` does not distinguish built-in types from user-defined types—the same interface and the same path
are used.

A sum type's **variant set enters the checker through its definition or a `use` import**:
`Result`/`Option` must be imported with `use std.result` / `use std.option` before use (whole-module
and grouped `use std.{...}` forms have equal authority). Variant construction (in its type-qualified
form), match-variant destructuring, and exhaustiveness checking all use this registration as their
sole criterion. A native function signature returning `Result(Float, Error)` mints a `Generic` with
the same name; it has the same identity as the `std.result` sum type by name and can be destructured
with `match` after the import.

### 4.3 Generic Constructor Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: Each field
corresponds to a constructor parameter, and the field name is the parameter name. Fields with
default values may be omitted during construction; fields without default values are required.
Function-typed fields (methods) do not generate constructor parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value -> constructor parameter is required
    extra: T,
}
// Fully expanded form generated automatically (compiler-internal view; users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invoke the automatically generated constructor
c  = Container(42, 43)            // Constructor parameters are filled in field order; T is inferred from the element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type argument + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // Field-name form; any order
c5 = Container(Int)()             // Empty construction: fields take their default/zero values (data assigned later)

// Field default values -> constructor parameters may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Calling rules** (single parentheses; match the declared parameters positionally from left to
right):

1. Try to match arguments positionally against the declared type parameters: A `Type` position
   accepts a type argument, while a compile-time value-parameter position such as `Int` accepts a
   compile-time constant.
2. If a compile-time value-parameter position matches successfully (a partial match), process the
   call as a type constructor: Check every parameter position and, on error, report **the first
   mismatched or missing parameter in declaration order**.
3. If the arguments do not correspond to the declared parameters at all (all are values, and no
   compile-time value-parameter position can match), process the call as a constructor: Fill
   positional arguments in field order and infer type parameters automatically from the element
   types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two levels: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern; data assigned later)

Matrix(42)    // ❌ Position 0: T←42 does not match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols is missing -> report the first error first: T expected Type, found 42
Container(42) // ❌ Missing constructor argument extra
Container(42, 43, 44)  // ❌ Too many constructor arguments
```

**Type inference**: The type parameters of a generic type constructor are inferred automatically
from the elements of the constructor arguments (`Container(42, 43)` → T=Int); the type parameters of
a generic function are inferred automatically from argument types (`map(numbers, f)` → T=Int,
R=String; see §4.1). If inference is impossible, explicit arguments must be supplied.

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

> **Source of constraint resolution (RFC-011b)**: Operator constraint names (`Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) are resolved by consulting the
> interface-implementation registry—`T: Add` ≜ a registered `Add(T, T, T)` instance. `Equal` also
> supports structural inference (a record whose fields are all comparable is automatically
> comparable). Names such as `Zero` / `One` / `PartialOrd` do not yet have a definition source and
> remain dangling constraint names.

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
// More complex associated type
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

The criterion is **being referenced in a type position**, rather than “being annotated with a
concrete type”: in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters
because they do not appear in any type position.

**Terminology**: A generic parameter annotated with a concrete type other than `Type`, such as
`Int`, is called a **compile-time value-parameter candidate**. Whether it becomes a compile-time
value parameter depends on whether its value is referenced in a type position (value dependence).
**No `const` keyword is required** (the implementation once referred to these as “const generics,”
but the documentation consistently uses “compile-time value parameter”).

**Decision rules (two steps)**:

1. **Coarse shape filter**: A parameter annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) is a candidate.
2. **Use-based refinement**: A candidate name appears in a **type position** (a field type in a type
   body, an inner `Fn` parameter type, an `Assert` predicate, or an `Array(T, N)` type-constructor
   argument position) and is therefore a true compile-time value parameter; otherwise, it is a
   **runtime value parameter**.

| Form                                                       | Decision                                         | Reason                                            |
| ---------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters                 | They appear only in value positions               |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter              | N appears in a type-constructor argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter              | N is the type of the inner parameter k            |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through and is a runtime value parameter | N is not referenced in the type body              |

**Core design**: Use the compile-time value parameter `(N: Int)` and the value parameter `(k: N)` to
distinguish compile-time constants from runtime values. A candidate that falls through—its shape
qualifies, but its use does not—degrades to a runtime value parameter. Both function-level and
type-constructor paths follow this rule.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (the Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in a type-constructor argument position -> compile-time value parameter
    length: N
}

// Use: factorial(5) is evaluated in a type position (at compile time), and the result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // The compiler computes factorial(5) = 120 at compile time

// Value dependence: N acts as the type of the inner parameter k
// N is a compile-time value parameter (it appears in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (a single-value type).
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
// IsTrue bridge and Assert refinement type (see §8.3 for details)
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

`assert` and `Assert` are two sides of the same refinement primitive. The dispatch pipeline selects
between them automatically according to “whether the predicate's free variables are available at
compile time.”

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                           |
| ----------------------------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enter the proof pipeline: true → erased to Void; false → compile error (Never cannot be inhabited) |
| A runtime free variable exists (function parameter, external input)                       | Runtime     | Insert a runtime Bool check and inject a refinement fact into the flow-sensitive assumption set Γ  |

**Flow-sensitive assumption set Γ**:

The compiler maintains the set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: the old assumption is invalidated
```

After a `mut` variable is assigned, every assumption involving that variable is removed (kill set).
When branches merge, Γ is the intersection of the branch assumption sets.

### 8.4 Terminates: Termination-Measure Predicate

`Terminates` is a **built-in predicate** and, like `Int` and `Never`, a core primitive (a built-in
name, not a keyword). It binds a **measure** to a computation, declares that the computation
terminates, and provides a witness of termination.

**Forms**: Two arities using the same predicate:

| Form                    | Anchor                         | Use Case                                                                                                                     |
| ----------------------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the containing binding | Default form—self-recursive functions and loops                                                                              |
| `Terminates(FnType, m)` | Explicit function type         | When the measure's ownership must be specified explicitly (the measure is defined elsewhere or serves multiple computations) |

```yaoxiang
// Measure: an ordinary function that can be unit-tested and reused; it does not participate at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere, with explicit ownership
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, and the measure is an expression in scope
// The loop body is a `{}` block whose value is given by the tail expression (spec §2.9): it is not `Void` unless the body ends with an expression
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
position. The obligation falls on that computation—for a function, at every recursive call site; for
a loop, at the back edge. In both cases, the requirement is that “the measure of the next state is
strictly less than the measure of the current state,” evaluated under the path guard at that point.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back edge is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: A loop is an anonymous construct and normally cannot be referred to.
> The binding name is its name—`acc` in `acc: Terminates(n - i) = while ...` provides the anchor,
> making the loop referable. This is why the unary form of `Terminates` can apply to a loop.

**Measure**: The return type is not restricted (it is not required to be a natural number). Its
strict decrease is determined by the well-founded order available on that type. Whether the measure
itself is well-founded—for example, whether it is `>= 0` when returning `Int`—is an **independent
obligation**, determined by the compile-time proof pipeline just like the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. Ordinary types that have not been refined, such as bare `while` loops or
functions without refined signatures, do not enter verification mode and do not generate termination
obligations.

**Automatic exploration first**: The compiler first attempts to discover a measure automatically
(four templates: linear ranking functions, predicate-violation counts, bounded increases/decreases,
and multiplicative scaling). An explicit `Terminates` is needed only when automatic exploration
fails.

**Runtime representation**: A purely compile-time entity that is erased with its witness and does
not enter the runtime binary.

> For the complete design, see
> [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md) for the semantics and
> [RFC-027a](../../rfc/accepted/027a-termination-explicit-measure.md) for the implementation
> mechanism.

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
// Platform type enum (defined by the standard library)
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

YaoXiang has only one type property that needs to be distinguished: linear versus copyable. It is
inferred automatically by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, argument passing, and returning transfer
ownership.

```yaoxiang
// A struct containing a heap-buffer field does not derive Dup (the field falls under Move) -> defaults to Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move; b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle and Share the Data)

**The Dup property is intended for reference/token types and internally reference-counted handle
value types**. Assigning a Dup type performs a shallow copy—the handle/token is copied, while the
underlying data is shared. Multiple holders point to the same data.

| Type            | Property | Description                                                                                |
| --------------- | -------- | ------------------------------------------------------------------------------------------ |
| `&T`            | Dup      | Zero-sized read token; copying the token gives multiple views of the same data             |
| `ref T`         | Dup      | An Rc/Arc copy increments the reference count and shares heap data                         |
| String, Bytes   | Dup      | Internal reference counting; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear   | Zero-sized write token; exclusive ownership and cannot be copied                           |
| struct          | Derived  | All fields are copyable (primitive value types ∪ Dup) → Dup; otherwise Move (#398)         |
| tuple           | Derived  | Determined element by element using the same rule as structs (#398)                        |
| All other types | Move     | Default ownership transfer                                                                 |

**Primitive value types** (Int, Float, Bool, Char) receive special compiler handling: assignment
automatically copies the value, producing two completely independent values. This is native compiler
behavior and is not part of the Dup type property.

**Derivation rules** (finalized in #398)—the formulation “automatically derive Dup when all fields
are Dup” cannot be executed literally: primitive fields such as Int are not themselves Dup, so
`{ x: Int, y: Int }` would be incorrectly classified as Move. The executable form is:

1. **Copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), and composite types already classified as Dup);
2. **struct**: If every field belongs to the copyable field set, derive Dup; if **any** field falls
   under Linear (`&mut T`) or Move (a nested Move struct / containers such as Vec and Dict /
   resources), the whole struct remains Move. No intermediate “partially copyable” state is
   introduced;
3. **tuple**: Uses the same rule as a struct and is determined element by element; an empty tuple
   (the unit) is `Void`;
4. **Derivation is recursive**: When a field has a named type, such as `target: Point`, its
   definition is expanded before classification. Thus, `A = { b: B }` follows B's derivation result.
   Cyclic aliases conservatively remain Move subject to a depth limit;
5. **Outside the derivation scope** (and therefore still Move, handled separately): containers (Vec
   / Dict / Set / Option / Result / Array) and enums.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copies the token; both remain valid
print(view.x)    // Available
print(view2.x)   // Available

// &mut T: Linear, not copyable
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

// Use
p: Point = Point(1.0, 2.0)
backup = p.clone()    // Deep copy; p remains usable
p2 = p.clone()        // May be cloned multiple times
```

**Differences between Dup and Clone**:

|                        | Dup                                                                  | Clone                                                  |
| ---------------------- | -------------------------------------------------------------------- | ------------------------------------------------------ |
| **Semantics**          | Shallow copy: copies the handle/token and shares the underlying data | Deep copy: creates a complete, independent copy        |
| **Invocation**         | Implicit (automatic on assignment/argument passing)                  | Explicit (`.clone()`)                                  |
| **Effect of mutation** | Changes affect one another (shared underlying data)                  | Changes do not affect one another (independent copies) |
| **Applicable types**   | `&T` tokens, `ref T`                                                 | Any type implementing the Clone interface              |
| **Cost**               | Zero overhead (a token is a zero-sized type)                         | Depends on the type                                    |

**Dup does not imply Clone, and Clone does not imply Dup**—they are orthogonal concepts:

```yaoxiang
// Dup type: copies the token and shares the underlying data
view: &Point = &p
view2 = view        // Dup: copies the token; both point to the same p
print(view.x)       // Available
print(view2.x)      // Available; both observe the same data

// Primitive value type: the compiler automatically copies the value (not Dup)
x: Int = 42
y = x               // Value copy; x and y are completely independent
print(x)            // Available

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
- Clone is used when an independent copy is required; explicit invocation makes the cost visible.
- Copying primitive value types (Int/Float/Bool/Char) is built-in compiler behavior and is not part
  of Dup.
- Custom types default to Move (zero-copy, high performance); Dup is derived automatically when all
  fields are copyable (see the derivation rules in §11.2).

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

- Tokens are **ordinary types** and follow the same scoping rules as all other types.
- No lifetime annotation `'a` is required.
- No dedicated borrow checker is needed—the type properties (Dup/Linear) naturally derive
  permissions.
- They disappear completely after compilation, with zero runtime overhead.

### 12.2 Basic Usage

```yaoxiang
// Method side: declaring the parameter type determines the required permission
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

// Multiple &T tokens coexist—Dup types may be copied freely
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types and therefore support all operations available to ordinary types:

**Returning tokens**—a token propagates together with the return value:

```yaoxiang
// ✅ The child token is returned together with the parent token
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // The token is returned to the caller
print(px_ref)                    // OK; the token remains in scope
```

**Storing tokens in structs**—a struct may carry token fields:

```yaoxiang
// ✅ A struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // Token field—holds a read-only view of target
}
```

**Closures do not capture; context is fixed at the creation point**—a closure accepts only its own
parameters. When outer data is needed, use currying to fix the value into the closure at the
creation point:

```yaoxiang
// ✅ Context is fixed through currying: threshold is a parameter, and gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope in which it was defined may no longer be
> alive, so it must not implicitly capture outer variables. The scope at the call site (the creation
> point) is necessarily alive, and fixing the context there as a value captured by the closure is
> safe.

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

**Method receivers follow signature semantics** (as with the RFC-011a receiver-spelling convention):
A receiver of `&T` is a read-only borrow token; `&mut T` is a mutable borrow token; and by-value
means Move (consuming the receiver). A borrow token produced at the call site is released when the
call ends (transient; interval semantics in §12.5). An interface's borrowed receiver must be
explicitly declared as `&Self` by the interface author, and after replacing `Self ↦ impl type` in
the impl signature, the signature must exactly match the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrowing Horn proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrowing propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification. Token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse-BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and a derived &T cannot be live at the same time
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
    // The inner scope ends
    p.x = 10.0                   // ✅ WriteToken is still available
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
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

The purposes of brands are:

- **Forgery prevention**: A token can only be obtained from its owner capsule and cannot be
  constructed from nothing.
- **Association tracking**: A derived `&Float` from a field access carries a derived brand
  (`#N.field_x`), allowing the compiler to trace it back to the parent token.
- **Conflict detection**: A WriteToken and a derived ReadToken from the same source cannot be live
  at the same time.

Brands disappear completely after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data -> safely Dup)
               | &mut T      // WriteToken (exclusive read/write -> Linear)
```

### 12.8 Borrow Tokens vs. `ref`

|                 | `&T` / `&mut T`                                               | `ref`                                             |
| --------------- | ------------------------------------------------------------- | ------------------------------------------------- |
| Purpose         | Inspect or modify in place                                    | Shared ownership                                  |
| Scope           | Follows the scope of the token value                          | May cross scopes                                  |
| Cost            | Zero overhead (zero-sized type; disappears after compilation) | Rc or Arc (selected by the compiler)              |
| Escape          | Possible (the token propagates through return values/structs) | Intended for escaping                             |
| Cross-task      | Not possible (tokens do not yet support cross-task transfer)  | Possible (the compiler automatically selects Arc) |
| Cycle detection | Not applicable                                                | Silent within a task; a cross-task lint           |

> Note (undefined): How to read the contents after creating a `ref` (dereference/method/automatic
> behavior) has not yet been defined in the specification. The current implementation reports E1052
> for `*a`. This section will be completed once the behavior is defined.

---

## Appendix: Quick Reference for Type Definitions

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

// === Termination measure (built-in predicate; see §8.4) ===

// Unary form: the anchor is the binding name (self-recursive functions and loops)
// The body must end with a value—the value of `while` = the value of the loop-body block = the tail expression (spec §2.9)
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

// Compile-time generic: N is referenced in a type position (k: N) -> compile-time value parameter
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Property Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, argument passing, and returning = ownership transfer

// === Primitive value types (compiler built-ins) ===
Int, Float,     // Automatically copied by value on assignment; the two values are completely independent
Bool, Char      // Not Dup; built-in compiler handling for primitives

// === Dup (shallow copy: copies the handle and shares the underlying data) ===
&T              // Zero-sized read token; copying the token = multiple views of the same data
ref T           // An Rc/Arc copy = reference count +1; shares heap data
String, Bytes   // Internal reference counting; copying the handle shares the underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) → Dup (#398)

// === Not derived (remain Move) ===
Vec/Dict/Set    // Containers and enums are outside the derivation scope (#398)

// === Linear ===
&mut T          // Zero-sized write token; Linear (exclusive and not copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Creates an independent copy; mutations do not affect the original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token; freezes source data → Dup (copyable)
&mut T          // Zero-sized compile-time write token; exclusive read/write → Linear (not copyable)

// Automatic selection at the call site
// 1. The argument is used later -> create a token
// 2. The argument is not used later -> Move
// 3. Preference order: &T < &mut T < Move

// Token propagation
// ✅ May be returned, stored in a struct, or captured by a closure
// ❌ May not cross tasks (tokens do not yet support cross-task transfer)
```
