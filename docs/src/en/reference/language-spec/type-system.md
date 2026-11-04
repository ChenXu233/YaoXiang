# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Isomorphism

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
| Truth \(\top\)                                 | `Void` (Unit, with a default value)                  |
| Falsehood \(\bot\)                             | `Never` (zero constructors; no value can inhabit it) |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell's paradox) |
| Case analysis                                  | Type-level `match`                                   |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions and compiler termination checking.

### 0.2 Types Are Propositions, Programs Are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computations correspond to correct constructive proofs**. YaoXiang type
  families (such as case analysis on `Add` over `Nat` plus recursive calls) are essentially
  type-level encodings of mathematical induction—provided that the compiler can perform termination
  checking.
- **Type checking is proof verification**. When a program passes type checking, this is equivalent
  to a logical proposition having been constructively proved.

### 0.3 Impact on Language Design

The Curry-Howard correspondence is reflected in YaoXiang as follows:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox
   caused by `Type: Type` (Girard's paradox).
2. **Type families** (RFC-011): Type-level case analysis plus recursive calls over the
   natural-number type `Nat(Zero/Succ)` correspond to the Peano axioms—provided that the compiler
   performs termination checking.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
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
> implementation and means “the type expected in this position.”

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

**There are 15 built-in type names; this section is the authoritative table**: `Never` / `Void` /
`()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` /
`Char` / `String` / `Bytes`. The implementation's single source of truth is
`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`), and this table matches it **name
for name**—the T2 gate in `tools/type-tables --report` compares this table, `CONST_PARAM_TYPES`, and
the LSP type-name list in all three directions.

| Type      | Logical Correspondence | Description                                                                                   | Default Size |
| --------- | ---------------------- | --------------------------------------------------------------------------------------------- | ------------ |
| `Type`    | —                      | Meta type (not a value type name; see below)                                                  | 0 bytes      |
| `Never`   | ⊥ (false/empty type)   | Zero constructors; no values. Return type for divergence/panic. `Never <: T` holds for any T. | 0 bytes      |
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

`()` is the **type notation** for `Void` (the zero-element tuple = the unit type) and is synonymous
with `Void`: standard-library signature strings write `() -> Void`, and the parser table places `()`
and `Void` in the same arm.

**There is only one valid spelling**. `Int` is not `int`; `Int64` is not `int64`/`i64`; `String` is
not `string`/`str`; `Bool` is not `bool`; `Void` is not `void`; `Never` is not `never`; `Float` is
not `float`/`f64`; `Char` is not `char`; and `Bytes` is not `bytes`. The implementation once
supported 22 such lowercase or abbreviated spellings, but all of them were removed under **WBS 6.1.2
(finalized on 2026-10-27)**: using a non-canonical spelling in a type position always produces an
unknown-type error and is never silently accepted as a user-defined type.

> **`DateTime` is not a built-in name**. It is a **semantic alias in the std layer**: the `now` /
> `parse_time` signature strings in `src/std/time.rs` and `src/std/interfaces/time.yx` still use
> `DateTime`, but at runtime it is `Int` (fixed in connection with #338). The parser table retains
> it only as a **temporary compatibility alias**, and it is not included in this table; migrating it
> out of the built-in-name table and having the std layer carry it instead belongs to **P10 S11**
> (issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)).

> **There are no unsigned integer types**. `Uint` (and `Int128`) are not usable type
> names—`MonoType` has no unsigned variant, and the remnants formerly present in the LSP type-name
> list and the `sizeof` name table were both removed under WBS 6.1.2. When unsigned semantics are
> required, use `Int` together with a lower-bound convention.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to falsehood (⊥) and
truth (⊤), respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. There is
   nothing to write on the right-hand side of `x: Never = ...`.
2. **Explosion principle**: `Never <: T` holds for every type `T`. `assert(false)` returns `Never`,
   after which subsequent code can pass type checking (although it is never reached).
3. **Divergence marker**: `f: (...) -> Never` means that `f` is guaranteed not to return. The
   compiler uses this for dead-code analysis and branch confluence in `match`.

`Never` is a built-in type name (registered through the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — exactly one inhabitant (the default void value). `Void` is the identity of
the zero-field product type. `x: Void = <default>` is valid. A block's value is determined by its
**tail expression** (an empty block `{}` is `Void`); see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

---

## Chapter 3: Composite Types

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
- An interface name written in the type body indicates that the type implements that interface.

> **Namespace ownership**: A `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to the namespace of `Point`. It does not trigger any implicit binding. For `.` call syntax
> such as `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004
> and RFC-010.

#### 3.1.1 Field Default Values

Type fields may specify default values, which may optionally be supplied during construction:

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

#### 3.1.2 Built-in Bindings

Methods may be bound directly inside a type definition:

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

**Direct interface assignment**: A concrete type may be assigned directly to an interface-typed
variable (structural subtyping).

```yaoxiang
// Direct assignment (the concrete type is known at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw; no vtable

// Function return value (not known at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method is looked up through the vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inference Result       | Invocation Method           |
| ------------------------------------ | ---------------------- | --------------------------- |
| Direct assignment of a concrete type | Concrete type is known | Direct call (zero overhead) |
| Function return value                | Unknown                | vtable                      |
| Heterogeneous collection             | Multiple types         | vtable                      |

**Coherence and orphan rules (not applicable; closure note)**: YaoXiang interfaces are structural
types (an interface is a record whose fields are all function types), not nominal traits. There is
therefore no cross-crate/module ownership question of “who may implement what for whom”; Rust-style
orphan rules and coherence checks have no applicable target (see RFC-011 §2.1 for the decision
record). The corresponding safeguard in the structural world is **rejection of duplicate
implementations**: defining the same method signature more than once on a type produces a
compile-time error (RFC-011a §3, which prohibits overriding; overloading is legal).

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

- **Named** (`a: Int`): The name enters the contract—it may be referenced by a named argument
  (`f(a = 3)`, see [Syntax §2.3](syntax.md#23-函数调用)), and the lambda header may be omitted when
  the signature already declares parameter names and types (`add: (a: Int, b: Int) -> Int = a + b`,
  RFC-007 §Shorthand Rules).
- **Unnamed** (a bare type such as `(Int, Int) -> Int`): Type constraints apply **by position**, and
  the names belong to the implementation—the lambda header must provide its own parameter names
  (`mk: (Int, Int) -> Int = (x, y) => x + y`; under RFC-007, the lambda header may be omitted if and
  only if the signature has declared parameter names). Such parameters cannot be referenced by named
  arguments and may only be called positionally. Bare identifiers are resolved in the type
  namespace; if no declared type is found, an error is reported—there is no bare identifier in a
  parameter position that is “neither a type nor a type parameter.”

**Annotated binding semantics**: An annotation is a declaration, not a comment. Binding statements
are checked against their annotations—the parameter types drive the lambda header positionally, the
function body's exits (the tail expression and `return`) are uniformly checked against the annotated
return type, and the type registered for the caller is the annotated form. Unannotated bindings use
HM inference (RFC-007). `x: Int = "hello"` and `f: () -> Int = () => "hello"` are rejected under the
same rule—the binding semantics do not diverge according to how the value is written (lambda / block
/ expression / named-function reference).

**The parameters in an implementation header are positional binders**
(`f: (a: Int) -> Int = (b) => b + 1`):

- **Names and types are governed by the declaration**: The i-th position in the implementation
  header corresponds to the **first N positions of the declaration's value-level signature** (N =
  the number of parameters in the implementation header; the value-level signature = the parameter
  sequence after removing compile-time positions—`T: Type`, type-level function positions, and const
  positions). The implementation header only declares “what the i-th parameter is called inside the
  body”; changing its name does not affect the caller—the names available to `f(a = 7)` come from
  the declaration and are independent of the implementation header's names.
- **Annotations in the implementation header are used only for comparison**: If the type at the same
  position differs from the declaration, **E1011** is reported (the declaration is authoritative,
  and the author's annotation must not be silently overridden).
- **If the implementation header has more runtime parameter positions than the declaration**,
  **E1112** is reported: the extra positions have no values to bind (type positions such as
  `T: Type`, type-level function positions, and const positions do not occupy runtime parameter
  positions).
- **The reverse case (the implementation omits parameters) is not an error** (this was the “pass”
  cell in the t1 behavior matrix before D54 and must not be reclassified): Type-level checking still
  uses the **declaration's** arity (`f(1)` is valid and `f(1, 2)` reports E1010), and missing
  positions are filled positionally by the declaration into the **type-checking scope**; however,
  the implementation header has no corresponding runtime slots, so arguments at those positions are
  discarded on the IR side and do not participate in computation inside the body (in
  `f: (a: Int) -> Int = () => 1`, `f(1)` is unrelated to the argument). **This has been recorded as
  a gap**—making the runtime also receive the declaration's arity would be a class-D behavioral
  change and requires a separate decision.
- **Uniform exit checking**: Whenever the tail expression or `return` (the arrow sugar `=> x` and
  explicit `return x` are two AST shapes) does not match the declared return position, **E1012** is
  reported, with the location pointing to the **annotation**.

> **Implementation status**: Positional checking of `Fn` annotation bindings has been implemented
> (WBS 6.8.2, 2026-11-02): parameters are checked positionally (the parser and typechecker use **the
> same alignment rule**: the first N positions of the value-level signature) / implementation-header
> annotations are compared (E1011; the author's annotation is retained and reported if it differs
> from the declaration) / exits are checked uniformly (E1012) / the registered form comes from the
> declaration (the available names for named arguments).

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
`-> Type` denotes the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives; they receive the same
treatment as user-defined generics and are handled through a unified generic-instantiation path. The
ownership of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                                           | Foundation                               |
| ------------- | ------------- | --------------------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (compile-time value parameter N) | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Raw buffer with runtime length; growable            | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard-library type (growable list)               | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                                   | `HeapValue::Dict`                        |

> `List(T)` is a **standard-library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list` and receives the same treatment as a user-defined generic record. Every
> growability strategy (when to expand, by how much, and whether sharing is possible) lives in the
> library; the compiler is not involved. `Vec(T)` is the minimal foundational primitive on which it
> depends.
>
> Set(T) has been retired: it has no literal, no runtime representation, and no `std.set`. When the
> requirement arises, implement it following the Dict pattern.

Key rules:

- **Literal placement is determined by context**: A bare `[...]` literal is placed in a growable
  list when annotated as `List(T)`; when an `Array(T, N)` annotation is applied directly to a
  literal, it is placed in a fixed-length array. Placement validation: the number of elements must
  equal N, and the element types must be compatible with T; otherwise, compile-time error E1002 is
  reported. When N is a symbolic constant (a const parameter), the count check is deferred to the
  refinement-type phase.
- **Implicit List→Array conversion is forbidden**: Fixed length is guaranteed by the type layer—push
  accepts only a `List(A)` receiver.
- **Performance hierarchy**: Performance decreases and flexibility increases from bottom to top:
  `Array` > `Vec` > `List`.
- **Index-failure contract** (runtime errors are a transitional state; the target state is coverage
  through compile-time refinement types, discussed in §8.4):
  - Index out of bounds (including a negative index) → `E6003`
  - Missing Dict key → `E6008`
- **String indexing semantics** (#385): `s[i]` indexes a character by **Unicode scalar value (code
  point)** and produces a single-character `String`. This uses the same unit as `len` / `index_of` /
  `substring` / `chars` / `char_code` in `std.string`, so combinations do not get out of alignment;
  an out-of-bounds index (including a negative index) follows the index-failure contract above,
  `E6003`.
- **The membership `in` predicate**: Returns `Bool` without erroring, and its right operand covers
  List/Array/Dict(keys)/Tuple/String/Range. It is a first-class HOL predicate and forms the basis of
  propositions provable at compile time by refinement types.`

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
`residual` / `from_error`). On failure, the function returns early with the result of
`from_error(residual(t))`; on success, the expression's value is `success(t)`. The enclosing
function's return type must also implement `Try`, and its failure-residual type must match the
receiver's (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try`
implementations through `std.result` / `std.option` (the failure-residual type of `Option` is
`Void`). A user-defined type may participate in `?` simply by writing `Try(Self, T, E)` in its type
body. The lowering of `?` does not distinguish built-in types from user-defined types—the same
interface follows the same path.

The **variant set** of a sum type enters the checker when it is defined or imported with `use`:
`Result`/`Option` must `use std.result` / `use std.option` before use (whole-module and grouped
`use std.{...}` forms have equal authority). Variant construction (in its type-qualified form),
match-variant destructuring, and exhaustiveness checking all use this registry as their sole
criterion; a native function signature returning `Result(Float, Error)` mints a `Generic` with the
same name, which has the same identity by name as the sum type in `std.result` and can therefore be
destructured through match after `use`.

### 4.3 Generic Constructor Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: Each field
corresponds to one constructor argument, and the field name becomes the argument name; fields with
default values may be omitted during construction, while fields without default values are required.
Function-typed fields (methods) do not generate constructor arguments.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value → constructor argument required
    extra: T,
}
// Fully expanded form (the compiler's internal view; users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invoke the automatically generated constructor
c  = Container(42, 43)            // Constructor arguments are filled in field order; T is automatically unpacked from the elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type argument + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // Named by field; any order
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data is assigned later)

// Field default values → constructor arguments may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, matched positionally against the declared parameters from left
to right):

1. Arguments are matched positionally against the parameters in the type declaration: a `Type`
   position accepts a type argument, and a compile-time value-parameter position (such as `Int`)
   accepts a compile-time constant.
2. If a compile-time value-parameter position matches successfully (a partial match), construction
   is processed as a type construction: all parameter positions are checked in order, and on an
   error the **first mismatching or missing parameter is reported in declaration order**.
3. If the arguments do not correspond at all to the declared parameters (all are values and no
   compile-time value-parameter position can match), construction is processed as a constructor
   call: positional arguments fill fields in order, and type parameters are automatically unpacked
   from the element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one layer of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two layers: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern; data is assigned later)

Matrix(42)    // ❌ Position 0: T←42 does not match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols is missing → report the first error first: T expected Type, found 42
Container(42) // ❌ Missing constructor argument extra
Container(42, 43, 44)  // ❌ Too many constructor arguments
```

**Type inference**: The type parameters of a generic type constructor are automatically unpacked
from the constructor arguments' element types (`Container(42, 43)` → T=Int); the type parameters of
a generic function are automatically unpacked from the argument types (`map(numbers, f)` → T=Int,
R=String, see §4.1). Explicit arguments are required when automatic unpacking is impossible.

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

> **Constraint-solving source (RFC-011b)**: Solving operator constraint names (`Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) means consulting the
> interface-implementation registry— `T: Add` ≜ a registered `Add(T, T, T)` instantiation; `Equal`
> additionally supports structural inference (records whose fields are all comparable are
> automatically comparable). Names such as `Zero` / `One` / `PartialOrd` currently have no
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

> The criterion is **being referenced in a type position**, rather than being annotated with a
> concrete type: in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters
> (neither appears in any type position).

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (such as
`Int`) is called a **compile-time value parameter candidate**. Whether it becomes a compile-time
value parameter depends on whether its value is referenced in a type position (value dependency).
**No `const` keyword is required** (the implementation internally used the term “const generic,” but
the documentation consistently uses “compile-time value parameter”).

> **Design basis**:
> [RFC-011 §4.1 Compile-Time Value Parameters](../../rfc/accepted/011-generic-type-system.md).

**Decision rules (two steps)**:

1. **Coarse shape filter**: A parameter annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Fine-grained usage filter**: The candidate name is referenced in a **type position** → a true
   compile-time value parameter; otherwise, it is a **runtime value parameter**.

**A “type position” means every `Type` subtree in a declaration**, not a fixed list of locations—the
annotation type of a parameter, a function's return type, a type body's field types, the types of
inner `Fn`s, `Assert` predicates, and the argument positions of type constructions such as
`Array(T, N)` are merely instances of a `Type` subtree in different syntactic locations. During the
decision, traverse **all** `Type` subtrees rather than handling locations separately: missing even
one subtree (for example, the return type) causes a compile-time value parameter to be misclassified
as a runtime value parameter.

**“Referenced” is determined by namespace resolution**, not by literal comparison of names:

1. The identifier is first resolved in the **type namespace**—declared type names and type
   parameters (`T: Type`);
2. If it resolves to a declared type, it is unrelated to the parameter. A parameter name having the
   same spelling as a type name **does not constitute shadowing**: the value namespace and type
   namespace do not consult the same table (see the scope and shadowing rules in
   [modules](modules.md)); the two `Int`s in `(Int: Int) -> Int` belong to different namespaces;
3. If it does not resolve but a declared parameter has the same name as a candidate, that candidate
   is a compile-time value parameter (value dependency);
4. If neither resolves, an error is reported: a bare identifier in a type position may be “neither a
   type nor a type parameter.”

**Self-reference is handled with a monotone fixed point**: When resolving
`factorial: (N: Int) -> (k: N) -> Int`, resolving `(k: N)` first requires knowing whether `N` is a
type, but classifying `N` is precisely what this rule computes. The decision therefore uses a fixed
point: the initial type namespace contains only declared types and type parameters; each round
confirms names that cannot be resolved as types but match a candidate as compile-time value
parameters and adds them to the type namespace; the process terminates when the set no longer grows
(at most |candidates| rounds). The declaration's own name (such as `Array` in `Array: ... -> Type`)
enters the type namespace before its type body is resolved.

**Corollary (placeholder rule)**: Compile-time value parameters **do not occupy runtime parameter
positions**. Runtime parameter positions = parameters − type parameters − compile-time value
parameters, arranged in declaration order; call-site arguments, the available names for named
arguments, and positional bindings in implementation headers all use this sequence.

| Form                                                       | Decision                                       | Reason                                             |
| ---------------------------------------------------------- | ---------------------------------------------- | -------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters               | They appear only in value positions                |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter            | N appears in a type-construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter            | N serves as the type of the inner parameter k      |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N misses and becomes a runtime value parameter | N is not referenced in the type body               |

**Core design**: Use `(N: Int)` compile-time value parameters together with `(k: N)` value
parameters to distinguish compile-time constants from runtime values. Candidates that miss remain
runtime value parameters—this applies to both function-level and type-constructor paths.

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
// k is a runtime value parameter whose type is the literal type N (a singleton type).
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 Compile-Time Constant Arrays

```yaoxiang
// Used by matrix types
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
// IsTrue bridge and Assert refinement types (see §8.3)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, execution continues
    false => Never,    // ⊥, divergence/compile-time error
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

`assert` and `Assert` are two sides of the same refinement primitive—the dispatch pipeline
automatically chooses between them according to whether the predicate's free variables are available
at compile time.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                                 |
| ----------------------------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enters the proof pipeline: true → erased to Void; false → compile-time error (Never cannot be inhabited) |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Inserts a runtime Bool check and injects refinement facts into the flow-sensitive assumption set Γ       |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions are invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (the kill
set). At a branch confluence point, Γ is the intersection of the branches' sets.

### 8.4 Terminates: Termination-Measure Predicate

`Terminates` is a **built-in predicate** and, like `Int` and `Never`, belongs to the core primitives
(a built-in name, not a keyword). It binds a **measure** to a computation, declares that the
computation terminates, and provides a witness for termination.

**Forms**: Two arities using the same predicate:

| Form                    | Anchor                         | Use                                                                                                                               |
| ----------------------- | ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the containing binding | Default form—self-recursive functions, loops                                                                                      |
| `Terminates(FnType, m)` | Explicit function type         | Used when the measure's ownership must be specified explicitly (the measure is defined elsewhere or serves multiple computations) |

```yaoxiang
// Measure: an ordinary function; unit-testable, reusable, and not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere, with ownership specified explicitly
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, and the measure is an expression in scope
// The loop body is a `{}` block whose value is given by its tail expression (spec §2.9): the body is not `Void` only when it ends with an expression
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
type position. The obligation falls on that computation—for a function, it falls on each recursive
call site; for a loop, it falls on the back edge—in both cases requiring that “the measure of the
next state is strictly less than the measure of the current state,” evaluated under the path guard
at that point.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back edge is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: A loop is an anonymous construction and usually cannot be referred to
> by name. The binding name is the name—in `acc: Terminates(n - i) = while ...`, `acc` provides the
> anchor, making the loop referable. This is why the unary form of `Terminates` applies to loops.

**Measure**: The return type is not restricted (it need not be a natural number); “strict decrease”
is determined by the well-founded order available on that type. Whether the measure is well-founded
(for example, whether an `Int` result is `>= 0`) is an **independent obligation**, determined by the
compiler's proof pipeline just like the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. Ordinary unrefined types (such as a bare `while` loop or a function
without a refined signature) do not enter verification mode and do not generate termination
obligations.

**Automatic exploration takes priority**: The compiler first attempts to explore a measure
automatically (using four templates: linear ranking functions, predicate-violation counts, bounded
increment/decrement, and multiplicative scaling); an explicit `Terminates` is needed only when
automatic exploration fails.

**Runtime representation**: It is a purely compile-time entity, erased along with the witness, and
does not enter the runtime binary.

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

YaoXiang distinguishes only one kind of type attribute: linear vs. copyable. The compiler infers it
automatically.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, argument passing, and returning transfer
ownership.

```yaoxiang
// A struct containing a heap-buffer field does not derive Dup (the field falls under Move) → default Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move; b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle, Share the Data)

The **Dup attribute is used for reference/token types and internally reference-counted handle value
types**. Assigning a Dup type performs a shallow copy—the handle/token is copied while the
underlying data is shared. Multiple holders refer to the same data.

| Type            | Attribute | Description                                                                                |
| --------------- | --------- | ------------------------------------------------------------------------------------------ |
| `&T`            | Dup       | Zero-sized read token; copying the token gives multiple views of the same data             |
| `ref T`         | Dup       | Rc/Arc copy = reference count + 1; heap data is shared                                     |
| String, Bytes   | Dup       | Internal reference counting; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear    | Zero-sized write token; exclusive and cannot be copied                                     |
| struct          | Derived   | All fields are copyable (primitive value types ∪ Dup) → Dup; otherwise Move (#398)         |
| tuple           | Derived   | Determined element by element, using the same rule as structs (#398)                       |
| All other types | Move      | Default ownership transfer                                                                 |

**Primitive value types** (Int, Float, Bool, Char) receive special handling from the compiler:
assignment performs an automatic value copy, and the two values are completely independent. This is
native compiler behavior and is not part of the Dup type attribute.

**Derivation rules** (finalized in #398)—the phrase “automatically derive Dup when all fields are
Dup” cannot be applied literally: primitive fields such as Int are not themselves Dup, so
`{ x: Int, y: Int }` would be misclassified as Move. The executable form is:

1. The **copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup
   (`&T`, `ref T`, String / Bytes, function values (#352), and composite types already derived as
   Dup);
2. **struct**: All fields are in the copyable field set → derive Dup; if **any** field falls under
   Linear (`&mut T`) or Move (a nested Move struct / containers such as Vec and Dict / resources),
   the whole struct remains Move (there is no intermediate “partially copyable” state);
3. **tuple**: The same rule as for structs, determined element by element; an empty tuple (the unit)
   is `Void`;
4. **Derivation is recursive**: When a field has a named type (such as `target: Point`), its
   definition is expanded before the decision; `A = { b: B }` follows B's derivation result. Cyclic
   aliases conservatively fall under Move at the depth limit;
5. **Outside the derivation scope** (and therefore still Move, pending a separate decision):
   containers (Vec / Dict / Set / Option / Result / Array) and enums.

```yaoxiang
// &T: Dup; free aliasing is allowed
view: &Point = &p
view2 = view     // Dup: copies the token; both remain valid
print(view.x)    // Usable
print(view2.x)   // Usable

// &mut T: Linear; cannot be copied
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

|                            | Dup                                                                  | Clone                                                 |
| -------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------- |
| **Semantics**              | Shallow copy: copies the handle/token and shares the underlying data | Deep copy: creates a complete independent copy        |
| **Invocation**             | Implicit (automatic during assignment/argument passing)              | Explicit (`.clone()`)                                 |
| **Effect of modification** | Changes are mutually visible (shared underlying data)                | Changes do not affect each other (independent copies) |
| **Applicable types**       | `&T` tokens, `ref T`                                                 | Any type implementing the Clone interface             |
| **Cost**                   | Zero overhead (tokens are zero-sized types)                          | Depends on the type                                   |

**Dup does not imply Clone, and Clone does not imply Dup**—they are orthogonal concepts:

```yaoxiang
// Dup type: copies the token and shares the underlying data
view: &Point = &p
view2 = view        // Dup: copies the token; both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable; sees the same data

// Primitive value type: the compiler performs an automatic value copy (not Dup)
x: Int = 42
y = x               // Value copy; x and y are completely independent
print(x)            // Usable

// Clone: explicit deep copy that creates an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy; p remains usable

// Non-Dup type (a field falls under Move, so Dup is not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field and does not derive Dup (see the derivation rules in §11.2)
```

**Design intent**:

- Dup is used for token/reference types and solves the problem of viewing the same data from
  multiple perspectives.
- Clone is used where an independent copy is required; the explicit call makes the cost visible.
- Copying primitive value types (Int/Float/Bool/Char) is built-in compiler behavior and is not part
  of Dup.
- Custom types default to Move (zero-copy, high performance); Dup is derived automatically when all
  fields are copyable (see the derivation rules in §11.2).

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not “references”; they are
“type-level proofs of access permission.”

```
&T      →  零大小，冻结源数据（禁止 WriteToken 在此期间获取），
          冻结保证下多份只读安全 → Dup（可复制）
&mut T  →  零大小，独占读写（禁止任何其他令牌），
          独占访问下复制无意义 → Linear（非 Dup）
```

**Key properties**:

- Tokens are **ordinary types** and follow the same scope rules as all other types.
- No lifetime annotations such as `'a` are needed.
- No dedicated borrow checker is needed—the type attributes (Dup/Linear) naturally derive the
  permissions.
- They disappear completely after compilation and have zero runtime overhead.

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

// Multiple &T tokens may coexist—Dup types allow free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types and therefore support all operations available to ordinary types:

**Returning tokens**—the token propagates together with the return value:

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

> Note: After a closure (function value) escapes, the scope in which it was defined may no longer be
> live, so it must not implicitly capture outer variables. However, the call site (creation point)
> is necessarily live, so fixing the context there as a value inside the closure is safe.

### 12.4 Automatic Borrow Selection

At the call site, the compiler automatically selects in the following order of priority:

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

**Method receivers follow signature semantics** (as in RFC-011a's receiver spelling convention): A
receiver of `&T` → a read-only borrow token; `&mut T` → a mutable borrow token; by value → Move
(consumes the receiver). Borrow tokens created at a call site are released when the call ends
(transient; interval semantics in §12.5); an interface's borrowed receiver must be explicitly
declared by the interface author as `&Self`, and after the impl signature is replaced using
`Self ↦ impl type`, it must match the interface exactly (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is the **borrowing HOL proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrowing propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification. Token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse-BFS Liveness Analysis):

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

// ❌ The same argument cannot simultaneously produce an &mut token and another token
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives an &mut token and an & token
```

### 12.6 Compiler Internals: Branding Mechanism

Users never interact with brands. Internally, the compiler assigns every token a unique compile-time
identifier:

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a unique compile-time integer
&mut Point     →  WriteToken(Point, #M)   // #M is a unique compile-time integer
```

The purposes of brands are:

- **Preventing forgery**: A token may only be obtained from its owner's capsule and cannot be
  constructed from nothing.
- **Association tracking**: A derived `&Float` obtained through field access carries a derived brand
  (`#N.field_x`), allowing the compiler to trace it back to the parent token.
- **Conflict detection**: A WriteToken and a derived ReadToken from the same source cannot be live
  simultaneously.

Brands disappear completely after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken (freezes source data → safe to Dup)
               | &mut T      // WriteToken (exclusive read/write → Linear)
```

### 12.8 Borrow Tokens vs. ref

|                 | `&T` / `&mut T`                                               | `ref`                                            |
| --------------- | ------------------------------------------------------------- | ------------------------------------------------ |
| Purpose         | Read briefly / modify in place                                | Shared ownership                                 |
| Scope           | Follows the token value's scope                               | Crosses scopes                                   |
| Cost            | Zero overhead (zero-sized type; disappears after compilation) | Rc or Arc (selected by the compiler)             |
| Escape          | Allowed (the token propagates through return values/structs)  | Intended for escaping                            |
| Cross-task      | Not allowed (tokens do not implement cross-task transfer)     | Allowed (the compiler automatically selects Arc) |
| Cycle detection | Not applicable                                                | Silent within a task; a lint across tasks        |

> Note (undefined): How to read the contents of a `ref` after creation (dereference/method/automatic
> behavior) has not yet been defined in the specification; the current implementation reports E1052
> for `*a`. This section will be updated once the behavior is defined.

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definitions

```
// === Record type (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

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

// === Termination measure (built-in predicate; see §8.4) ===

// Unary form: the anchor is the binding name (self-recursive functions, loops)
// The body must yield a value—the value of `while` = the value of the loop-body block = the tail expression (spec §2.9)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}

// Binary form: measure ownership is specified explicitly (the measure is defined elsewhere)
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

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Attribute Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, argument passing, and returning = ownership transfer

// === Primitive value types (compiler built-ins) ===
Int, Float,     // Assignment automatically performs a value copy; the two values are completely independent
Bool, Char      // Not Dup; built-in compiler handling for primitives

// === Dup (shallow copy: copy the handle and share the underlying data) ===
&T              // Zero-sized read token; copying the token = multiple views of the same data
ref T           // Rc/Arc copy = reference count + 1; shares heap data
String, Bytes   // Internal reference counting; copies the handle and shares the underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) → Dup (#398)

// === Not derived (remains Move) ===
Vec/Dict/Set    // Containers and enums are outside the derivation scope (#398)

// === Linear ===
&mut T          // Zero-sized write token; Linear (exclusive and cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // Creates an independent copy; modifications do not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token; freezes source data → Dup (copyable)
&mut T          // Zero-sized compile-time write token; exclusive read/write → Linear (cannot be copied)

// Automatic selection at the call site
// 1. The argument is used later → create a token
// 2. The argument is not used later → Move
// 3. Priority: &T < &mut T < Move

// Token propagation
// ✅ May be returned, stored in a struct, or captured by a closure
// ❌ May not cross tasks (tokens do not implement cross-task transfer)
```
