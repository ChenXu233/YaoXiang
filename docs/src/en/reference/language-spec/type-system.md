# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter Zero: Theoretical Foundations

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
| False \(\bot\)                                 | `Never` (zero constructor, no inhabitant)            |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                                   |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires a
> type-level recursive function plus a compiler termination check.

### 0.2 Types Are Propositions, Programs Are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to a correct constructive proof**. YaoXiang type
  families (such as case analysis over `Nat` for `Add`, followed by a recursive call) are
  fundamentally type-level encodings of mathematical induction—provided that the compiler can
  perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, a logical
  proposition has been constructively proved.

### 0.3 Impact on Language Design

The Curry-Howard correspondence is reflected in YaoXiang as follows:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox
   caused by `Type: Type` (Girard's paradox).
2. **Type families** (RFC-011): type-level case analysis plus recursive calls over the
   natural-number type `Nat(Zero/Succ)` corresponds to the Peano axioms—provided that the compiler
   performs termination checking.
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic.
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to the finite
   quantification “for every integer N, there exists a type.”

---

## Chapter One: Type Categories

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
> `ast.rs:25`). `TypeExpr` serves as a BNF placeholder corresponding to the `Type` enum in the
> implementation and means “the type expected in this position.”

---

## Chapter Two: Primitive Types

### 2.1 Primitive Types

**There are 15 built-in type names; this section is the authoritative table**: `Never` / `Void` /
`()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` /
`Char` / `String` / `Bytes`. The single source of truth in the implementation is
`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`). This table matches it **name for
name**—the T2 gate in `tools/type-tables --report` compares this table, `CONST_PARAM_TYPES`, and the
LSP type-name list in all three directions.

| Type      | Logical Correspondence | Description                                                                                   | Default Size |
| --------- | ---------------------- | --------------------------------------------------------------------------------------------- | ------------ |
| `Type`    | —                      | Meta type (not a value type name; see below)                                                  | 0 bytes      |
| `Never`   | ⊥ (false/empty type)   | Zero constructor, with no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
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

`()` is the **type syntax** for `Void` (a zero-element tuple = the unit type) and is synonymous with
`Void`: standard-library signature strings write `() -> Void`, and the parser table places `()` and
`Void` in the same arm.

**There is only one valid spelling**. `Int` is not `int`; `Int64` is not `int64`/`i64`; `String` is
not `string`/`str`; `Bool` is not `bool`; `Void` is not `void`; `Never` is not `never`; `Float` is
not `float`/`f64`; `Char` is not `char`; and `Bytes` is not `bytes`. The implementation once
contained 22 such lowercase or abbreviated spellings, all of which were removed in **WBS 6.1.2
(finalized on 2026-10-27)**: using a noncanonical spelling in a type position always produces an
unknown-type error and is never silently accepted as a user type.

> **`DateTime` is not a built-in name**. It is a **semantic alias in the std layer**: the `now` /
> `parse_time` signature strings in `src/std/time.rs` and `src/std/interfaces/time.yx` still use
> `DateTime`, but at runtime it is `Int` (fixed as part of #338). The parser table retains it only
> as a **temporary compatibility alias**, and this table does not include it. Moving it out of the
> built-in type table and having std provide it belongs to **P10 S11** (issue
> [#513](https://github.com/ChenXu233/YaoXiang/issues/513)).

> **There are no unsigned integer types**. `Uint` (and `Int128`) are not available type
> names—`MonoType` has no unsigned variant. The LSP type-name list and the `sizeof` name table each
> contained one residual entry, both of which were removed with WBS 6.1.2. When unsigned semantics
> are required, use `Int` with an explicit lower-bound convention.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤), respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructor**: No literal or expression can produce a value of type `Never`. There is no
   right-hand side that can be written for `x: Never = ...`.
2. **Explosion principle**: `Never <: T` holds for every type `T`. `assert(false)` returns `Never`,
   after which the code passes type checking (although that point is never reached).
3. **Divergence marker**: `f: (...) -> Never` means that `f` is guaranteed not to return. The
   compiler uses this for dead-code analysis and `match` branch confluence.

`Never` is a built-in type name registered through the same path as `Int`/`Bool`; it is not a
keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity of the zero-field product type. `x: Void = <default>` is valid. A block's value is provided
by its **tail expression** (an empty block `{}` is `Void`); see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for details.

---

## Chapter Three: Composite Types

### 3.1 Record Types

**Unified syntax**: `Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // 接口约束
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
- A field name is followed directly by a colon and its type.
- An interface name in the type body indicates that the type implements that interface.

> **Namespace ownership**: A `Type.name` prefix (such as `Point.draw`) indicates that a function
> belongs to the namespace of `Point`. It does not create any implicit binding. For `.` call syntax
> such as `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004
> and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, which become optional when constructing a value:

```yaoxiang
// Field with a default value — optional at construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without a default value — required at construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // Correct
Point2()          // Incorrect
```

**Rules**:

- `field: Type = expression` -> Has a default value and is optional at construction.
- `field: Type` -> Has no default value and is required at construction.

#### 3.1.2 Builtin Bindings

Methods may be bound directly inside a type definition:

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
// Type implementing interfaces
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

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through the vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inference Result                | Invocation Method           |
| ------------------------------------ | ------------------------------- | --------------------------- |
| Direct assignment of a concrete type | Concrete type can be determined | Direct call (zero overhead) |
| Function return value                | Unknown                         | vtable                      |
| Heterogeneous collection             | Multiple types                  | vtable                      |

**Coherence and the orphan rule (not applicable; closure rationale)**: YaoXiang interfaces are
structural types (an interface is a record whose fields are all function types), not nominal traits.
Therefore, there is no cross-crate/module ownership question of “who may implement what,” and
Rust-style orphan rules and coherence checks have no applicable target (see RFC-011 §2.1 for the
decision record). The corresponding guarantee in a structural world is **duplicate-implementation
rejection**: defining the same method signature more than once on a type is a compile error
(RFC-011a §3 prohibits overriding; overloading is valid).

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

**Two parameter forms, with different ownership of names** (the type side of RFC-007's shorthand
rules):

- **Named** (`a: Int`): The name enters the contract and may be referenced by a named argument
  (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)). When the signature already declares
  parameter names and types, the lambda header may be omitted
  (`add: (a: Int, b: Int) -> Int = a + b`, RFC-007 §Shorthand Rules).
- **Unnamed** (a bare type such as `(Int, Int) -> Int`): Type constraints apply **positionally**,
  and the name belongs to the implementation. The lambda header must provide its own parameter names
  (`mk: (Int, Int) -> Int = (x, y) => x + y`; RFC-007 allows the lambda header to be omitted if and
  only if the signature declares parameter names). Such parameters cannot be referenced by named
  arguments and can only be called positionally. A bare identifier is resolved in the type
  namespace; an error is reported if no declared type can be found. There is no bare identifier in a
  parameter position that is “neither a type nor a type parameter.”

**Binding semantics of annotations**: An annotation is a declaration, not a comment. Binding
statements are checked against their annotations: parameter types drive the lambda header
positionally, and function-body exits (the tail expression and `return`) are checked against the
annotated return type. The type registered for callers is the annotated form. Unannotated bindings
use HM inference (RFC-007). Under the same rules, `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are both rejected—binding semantics do not vary according to how the
value is written (lambda / block / expression / named function reference).

**Implementation-header formals are positional binders** (`f: (a: Int) -> Int = (b) => b + 1`):

- **Both the declared name and type are authoritative**: The i-th implementation-header formal
  corresponds to the **first N slots of the declaration's value-level signature** (N = the number of
  implementation-header formals; the value-level signature = the declaration's formal sequence after
  removing compile-time slots—`T: Type`, type-level function slots, and const slots). The
  implementation header only declares “what the i-th slot is called in the body”; changing its name
  does not affect callers—the available name in `f(a = 7)` comes from the declaration and is
  independent of the implementation header's name.
- **Annotations in the implementation header serve only as cross-checks**: A type mismatch at the
  same position reports **E1011** (the declaration is authoritative, and an author's annotation must
  not be silently overridden).
- **More implementation-header formals than the declaration's runtime formal slots** reports
  **E1112**: The extra slots have no values to bind (type slots such as `T: Type`, type-level
  function slots, and const slots do not occupy runtime formal slots).
- **The reverse case (the implementation specifies fewer formals) is not an error** (the “pass” cell
  in the t1 behavior matrix before D54 must not be reclassified): The type layer still checks arity
  according to the **declaration** (`f(1)` is valid and `f(1, 2)` reports E1010), and missing slots
  are filled positionally by the declaration into the **type-checking scope**. However, the
  implementation header has no corresponding runtime slots, so arguments falling into those slots
  are discarded on the IR side and do not participate in computation in the body (`f(1)` is
  unrelated to the argument in `f: (a: Int) -> Int = () => 1`). This has been **recorded as a
  gap**—making the runtime also receive the declaration's arity would be a class D behavior change
  and would require a separate decision.
- **Uniform exit checking**: Whenever the tail expression or `return` (the arrow sugar `=> x` and
  the explicit `return x` are two AST forms) does not match the declared return slot, **E1012** is
  reported, with the location pointing to the **annotation**.

> **Implementation status**: Positional checking of `Fn` annotations during binding has been
> implemented (WBS 6.8.2, 2026-11-02): positional formals drive the lambda header (parser and
> typecheck use **the same alignment rule**: the first N slots of the value-level signature) /
> implementation-header annotation cross-checking (E1011; the author's annotation is retained and a
> mismatch is reported) / uniform exit checking (E1012) / the declaration determines the registered
> form (available names for named arguments).

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

In a generic type definition, `(T: Type)` is the type constructor's parameter signature, and
`-> Type` specifies the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives. They receive the same
treatment as user-defined generics and are handled through a unified generic instantiation path.
Ownership of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                                | Foundation                               |
| ------------- | ------------- | ---------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)     | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Raw buffer with runtime length; growable | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard-library type (growable list)    | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                        | `HeapValue::Dict`                        |

> `List(T)` is a **standard-library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list` and receives the same treatment as a user-defined generic record. All
> growable-list policies (when to expand, by how much, and whether sharing is possible) reside in
> the library; the compiler is not involved. `Vec(T)` is the minimal foundational primitive on which
> it depends.
>
> Set(T) has been removed: it has no literal syntax, no runtime representation, and no std.set. When
> the need arises, implement it following the Dict pattern.

Key rules:

- **Literal placement is determined by context**: A bare `[...]` literal together with a `List(T)`
  annotation produces a growable list; an `Array(T, N)` annotation applied directly to a literal
  produces a fixed-length array. Placement validation requires the element count to equal N and the
  element types to be compatible with T; otherwise E1002 is reported at compile time. When N is a
  symbolic constant (a const parameter), count validation is deferred to the refinement-type phase.
- **Implicit List→Array conversion is forbidden**: Fixed-length semantics are guaranteed by the type
  layer—push accepts only a `List(A)` receiver.
- **Performance hierarchy**: Performance decreases and flexibility increases from bottom to top:
  `Array` > `Vec` > `List`.
- **Index-failure contract** (runtime errors are a transitional state; the target state is
  compile-time refinement through value-dependent types; see §8.4):
  - Index out of bounds, including a negative index → `E6003`
  - Missing Dict key → `E6008`
- **String indexing semantics** (#385): `s[i]` indexes characters by **Unicode scalar value (code
  point)** and produces a single-character `String`. This uses the same unit as `len` / `index_of` /
  `substring` / `chars` / `char_code` in `std.string`, so combinations do not shift units. An
  out-of-bounds index, including a negative index, follows the index-failure contract above:
  `E6003`.
- **Membership `in` predicate**: Returns `Bool` without reporting an error. Its right operand covers
  List/Array/Dict(keys)/Tuple/String/Range. It is a first-class Hore predicate and forms the basis
  of propositions provable at compile time through refinement types.`

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
receiver's type must implement the `Try` interface (with the four members `is_failure` / `success` /
`residual` / `from_error`). On failure, the result of `from_error(residual(t))` causes an early
return from the current function; on success, the expression's value is `success(t)`. The enclosing
function's return type must also implement `Try`, and its failure-residual type must match the
receiver's (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try`
implementations through `std.result` / `std.option` (`Option`'s failure-residual type is `Void`). A
user-defined sum type can participate in `?` by writing `Try(Self, T, E)` in its type body. Lowering
`?` makes no distinction between built-in and user-defined types—the same interface follows the same
path.

A sum type's **variant set enters the checker when it is defined or imported with `use`**:
`Result`/`Option` must be imported with `use std.result` / `use std.option` before use (whole-module
and grouped `use std.{...}` forms have equal authority). Variant construction (in its type-qualified
form), match-variant destructuring, and exhaustiveness checking all use this registry as their sole
criterion; a native function signature returning `Result(Float, Error)` creates a `Generic` with the
same name, which has the same identity by name as the `std.result` sum type and can be destructured
in a match after `use`.

### 4.3 Generic Constructor Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: Each field
corresponds to a constructor argument, and the field name is the argument name. Fields with default
values may be omitted during construction, while fields without default values are required.
Function-typed fields (methods) do not generate constructor arguments.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value → constructor argument is required
    extra: T,
}
// Fully expanded form generated automatically (the compiler's internal view; users need not write it manually):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invoke the automatically generated constructor
c  = Container(42, 43)            // Constructor arguments are filled in field order; T is automatically unpacked from the element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type argument + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // Named fields, in any order
c5 = Container(Int)()             // Empty construction: fields take their default/zero values (data is assigned later)

// Field default values → constructor arguments may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Calling rules** (single parentheses, matching declared parameters positionally from left to
right):

1. Arguments are matched positionally against the declared type parameters: a `Type` slot accepts a
   type argument, and a compile-time value-parameter slot (such as `Int`) accepts a compile-time
   constant.
2. If a compile-time value-parameter slot matches successfully (a partial match), construction is
   handled as a type construction: all parameter slots are checked in order, and the **first
   mismatched or missing parameter in declaration order** is reported.
3. If the arguments do not correspond to the declared parameters at all (all are values and no
   compile-time value-parameter slot can match), construction is handled through constructor
   arguments: positional arguments fill fields in field order, and type parameters are automatically
   unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one layer of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two layers: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern; data is assigned later)

Matrix(42)    // ❌ Slot 0: T←42 does not match (42 is not a type); slot 1: Rows←42 matches;
              //    slot 2: Cols is missing → report the first error first: T expects Type, but 42 was found
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

// Using a constraint
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

> **Constraint resolution sources (RFC-011b)**: Operator constraint names (`Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) are resolved by consulting the
> interface-implementation registry— `T: Add` ≜ a registered `Add(T, T, T)` instantiation. `Equal`
> additionally has structural inference (records whose fields are all comparable are automatically
> comparable). Names such as `Zero` / `One` / `PartialOrd` currently have no definition source and
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

### 5.3 Function Type Constraints

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
// More complex associated type
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
> because neither appears in any type position.

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (such as
`Int`) is called a **compile-time value-parameter candidate**. Whether it becomes a compile-time
value parameter depends on whether its value is referenced in a type position (value dependency).
**No `const` keyword is required** (the implementation once used “const generic” for this concept,
but the documentation consistently uses “compile-time value parameter”).

> **Design basis**:
> [RFC-011 §4.1 Compile-Time Value Parameters](../../rfc/accepted/011-generic-type-system.md).

**Decision rules (two steps)**:

1. **Syntactic screening**: A parameter annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Usage screening**: The candidate name is referenced in a **type position** → a true
   compile-time value parameter; otherwise → a **runtime value parameter**.

**A “type position” means every `Type` subtree in a declaration**, not a fixed list of positions.
The annotation type of a formal, a function's return type, a type body's field type, the type of an
inner `Fn`, an `Assert` predicate, and the argument positions of type constructions such as
`Array(T, N)` are merely instances of a “`Type` subtree” in different syntactic positions. The
decision traverses **all** `Type` subtrees and does not branch by position: missing even one
subtree—for example, by overlooking the return type—causes a compile-time value parameter to be
misclassified as a runtime value parameter.

**“Referenced” is determined through namespace resolution**, not by literal comparison of names:

1. The identifier is first resolved in the **type namespace**—declared type names and type
   parameters (`T: Type`);
2. If it resolves to a declared type, it is unrelated to the formal. A formal name and a type name
   having the same spelling **does not constitute shadowing**: the value and type namespaces do not
   use the same table (see the scope and shadowing rules in [modules](modules.md)); the two `Int`
   occurrences in `(Int: Int) -> Int` belong to different namespaces;
3. If it cannot be resolved but a formal with the same name exists among the candidates, that
   candidate is a compile-time value parameter (value dependency);
4. If neither can be resolved, an error is reported: a bare identifier in a type position may be
   “neither a type nor a type parameter.”

**Self-reference is handled through a monotone fixed point**: Resolving `(k: N)` in
`factorial: (N: Int) -> (k: N) -> Int` requires first knowing whether `N` is a type, but classifying
`N` is precisely what this rule computes. The decision therefore uses a fixed-point procedure: the
initial type namespace contains only declared types and type parameters; each round confirms names
that cannot be resolved as types but match a candidate and adds them to the type namespace; the
process terminates when the set no longer grows (at most |candidates| rounds). A declaration's own
name (such as `Array` in `Array: ... -> Type`) enters the type namespace before its type body is
resolved.

**Implications (two separate sequences)**: Compile-time value parameters occupy different positions
in two **different** sequences and must not be merged.

- **Call-site argument sequence** = formals − type parameters (in declaration order). A compile-time
  value parameter **still occupies** one argument slot: a call to `f: (N: Int) -> (n: N) -> Int` is
  written `f(42)(5)`, with 42 supplied for N and 5 for n. The reason is that argument slots carry
  “how many values the author supplied in declaration order,” independently of whether those values
  are bound at runtime; RFC-011 §4.1's `factorial(k - 1)` precisely supplies a runtime-computed
  value in N's position.
- **Implementation-header binding sequence** = formals − type parameters − compile-time value
  parameters (in declaration order). The implementation header binds only this sequence of names:
  `= (n) => n` is valid, while `= (N, n) => n` reports E1112. The reason is that the implementation
  header binds **runtime values**; a compile-time value parameter has no runtime value to bind.

E1011 and E1112 both count according to the **implementation-header binding sequence**.

| Form                                                       | Classification                      | Reason                                             |
| ---------------------------------------------------------- | ----------------------------------- | -------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | They appear only in value positions                |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N appears in a type-construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N serves as the type of inner parameter k          |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N is unused → E1094                 | N is not referenced in the type body               |

**Core design**: Using the compile-time value parameter `(N: Int)` together with the runtime value
parameter `(k: N)` distinguishes compile-time constants from runtime values. An unused candidate
(syntactically a candidate but not matched by usage) is routed according to its **declared form**:
on a function-level path it degenerates into a runtime value parameter (the unused candidate
occupies one runtime value slot); on a type-constructor path it reports **E1094** (a type
constructor is evaluated at compile time, and an unused candidate has no runtime slot to occupy).

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (the Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in a type-construction argument position → compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at compile time, and the result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // The compiler computes factorial(5) = 120 at compile time

// Value dependency: N serves as the type of the inner parameter k
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

### 8.3 Assert Refinement Types and the `assert` Assertion

`assert` and `Assert` are two sides of the same refinement primitive. The dispatch pipeline selects
between them automatically according to whether the predicate's free variables are available at
compile time.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                            |
| ----------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enters the proof pipeline: true → erased to Void; false → compile error (Never is uninhabited)      |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Inserts a runtime Bool check and injects a refinement fact into the flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: the old assumption is invalidated
```

After a `mut` variable is assigned, every assumption involving that variable is removed (kill set).
When branches are merged, Γ is the intersection of the branches' sets.

### 8.4 `Terminates`: Termination Measure Predicates

`Terminates` is a **built-in predicate** and, like `Int` and `Never`, belongs to the core primitives
(a built-in name, not a keyword). It binds a **measure** to a computation, declares that the
computation terminates, and provides a witness of termination.

**Forms**: Two arities of the same predicate:

| Form                    | Anchor                         | Use Case                                                                                                                          |
| ----------------------- | ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the containing binding | Default form—self-recursive functions, loops                                                                                      |
| `Terminates(FnType, m)` | Explicit function type         | Used when the measure's ownership must be specified explicitly (the measure is defined elsewhere or serves multiple computations) |

```yaoxiang
// Measure: an ordinary function that can be unit-tested and reused; it does not participate at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere, and its ownership is specified explicitly
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, and the measure is an expression in scope
// The loop body is a `{}` block whose value is provided by its tail expression (spec §2.9): the body is not `Void` unless it ends with an expression
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines the value type of the computation annotated in the containing
type slot. The obligation falls on that computation—for functions, at every recursive call site; for
loops, at the back edge—in each case requiring that “the next state's measure is strictly less than
the current state's measure,” evaluated under that point's path guards.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back edge is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: A loop is an anonymous construct and usually cannot be referred to by
> name. The binding name provides that name—the `acc` in `acc: Terminates(n - i) = while ...`
> provides an anchor, making the loop referable. This is why the unary form of `Terminates` applies
> to loops.

**Measure**: The return type is not restricted (natural numbers are not required); strict decrease
is determined by the well-founded ordering available on that type. Whether the measure is
well-founded—for example, whether an `Int` return value is `>= 0`—is an **independent obligation**,
evaluated by the compile-time proof pipeline just like the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types**—a type enters verification
mode as soon as it is refined. Ordinary unrefined types (such as a bare `while` loop or a function
without a refined signature) do not enter verification mode and generate no termination obligations.

**Automatic exploration first**: The compiler first attempts to discover a measure automatically
using four templates—linear ranking functions, predicate-violation counts, bounded
increment/decrement, and multiplicative scaling. An explicit `Terminates` is required only when
automatic exploration fails.

**Runtime representation**: It is a purely compile-time entity and is erased with its witness; it
does not enter the runtime binary.

> For the complete design, see
> [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md) for the semantics and
> [RFC-027a](../../rfc/accepted/027a-termination-explicit-measure.md) for the implementation
> mechanism.

---

## Chapter Nine: Type Unions and Intersections

### 9.1 Type Union

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 Type Intersection

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**Syntax**: The type intersection `A & B` represents a type that satisfies both A and B.

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

## Chapter Eleven: Type Properties

YaoXiang has only one type-property distinction: linear versus copyable. The compiler infers it
automatically.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, argument passing, and return transfer
ownership.

```yaoxiang
// A struct containing a heap-buffer field does not derive Dup (the field falls under Move) → Move by default
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move; b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle, Share the Data)

**The Dup property applies to reference/token types and internally reference-counted handle value
types**. Assigning a Dup type performs a shallow copy—it copies the handle/token while sharing the
underlying data. Multiple holders refer to the same data.

| Type            | Property | Description                                                                                |
| --------------- | -------- | ------------------------------------------------------------------------------------------ |
| `&T`            | Dup      | Zero-sized read token; copying the token provides multiple views of the same data          |
| `ref T`         | Dup      | Copying an Rc/Arc increments the reference count and shares the heap data                  |
| String, Bytes   | Dup      | Internal reference counting; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear   | Zero-sized write token; exclusive access and cannot be copied                              |
| struct          | Derived  | All fields are copyable (primitive value types ∪ Dup) → Dup; otherwise Move (#398)         |
| tuple           | Derived  | Determined element by element, using the same rule as structs (#398)                       |
| All other types | Move     | Ownership transfer by default                                                              |

**Primitive value types** (Int, Float, Bool, Char) receive special handling from the compiler:
assignment automatically copies the value, and the two values are completely independent. This is
native compiler behavior and is not part of the Dup property.

**Derivation rules** (finalized in #398)—the rule “derive Dup automatically when all fields are Dup”
cannot be executed literally: primitive fields such as Int are not themselves Dup, so
`{ x: Int, y: Int }` would be misclassified as Move. The executable form is:

1. The **copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup
   (`&T`, `ref T`, String / Bytes, function values (#352), and composite types already classified as
   Dup);
2. **struct**: If all fields belong to the copyable field set → derive Dup. If **any** field falls
   under Linear (`&mut T`) or Move (a nested Move struct / containers such as Vec or Dict /
   resources) → the whole type remains Move (there is no intermediate “partially copyable” state);
3. **tuple**: Follows the same rule as structs, determined element by element; an empty tuple (the
   unit) is `Void`;
4. **Derivation is recursive**: When a field has a named type (such as `target: Point`), its
   definition is expanded before classification; `A = { b: B }` follows B's derived result. Cyclic
   aliases conservatively fall under Move at the depth limit;
5. **Outside the scope of derivation** (and therefore still Move, handled separately): containers
   (Vec / Dict / Set / Option / Result / Array) and enums.

```yaoxiang
// &T: Dup, aliases may be created freely
view: &Point = &p
view2 = view     // Dup: copies the token; both remain valid
print(view.x)    // Valid
print(view2.x)   // Valid

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup and cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship to Dup

**Clone** is the explicit deep-copy interface. Every type may implement Clone by providing a
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

|                        | Dup                                                                  | Clone                                                  |
| ---------------------- | -------------------------------------------------------------------- | ------------------------------------------------------ |
| **Semantics**          | Shallow copy: copies the handle/token and shares the underlying data | Deep copy: creates a complete, independent copy        |
| **Invocation**         | Implicit (automatic during assignment/argument passing)              | Explicit (`.clone()`)                                  |
| **Effect of mutation** | Changes affect one another (shared underlying data)                  | Changes do not affect one another (independent copies) |
| **Applicable types**   | `&T` tokens, `ref T`                                                 | Any type implementing the Clone interface              |
| **Cost**               | Zero overhead (tokens are zero-sized types)                          | Depends on the type                                    |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copies the token and shares the underlying data
view: &Point = &p
view2 = view        // Dup: copies the token; both refer to the same p
print(view.x)       // Valid
print(view2.x)      // Valid; both observe the same data

// Primitive value type: the compiler automatically copies the value (not Dup)
x: Int = 42
y = x               // Value copy; x and y are completely independent
print(x)            // Valid

// Clone: explicit deep copy that creates an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy; p remains usable

// Non-Dup type (a field falls under Move, so Dup is not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field and does not derive Dup (see the derivation rules in §11.2)
```

**Design intent**:

- Dup is used for tokens/references and solves the problem of “multiple views of the same data.”
- Clone is used where an independent copy is required; making the call explicit makes its cost
  visible.
- Copying primitive value types (Int/Float/Bool/Char) is built-in compiler behavior and is not part
  of Dup.
- User-defined types are Move by default (zero-copy, high performance); Dup is derived automatically
  when all fields are copyable (see the derivation rules in §11.2).

## Chapter Twelve: Borrow Token Types

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

- A token is an **ordinary type** and follows the same scope rules as every other type.
- No lifetime annotation `'a` is needed.
- No dedicated borrow checker is needed—the type properties (Dup/Linear) naturally determine
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

// Call side: the compiler automatically selects a borrow or Move
p = Point(1.0, 2.0)
p.print()                       // The compiler automatically creates an &Point token
p.shift(1.0, 1.0)               // The compiler automatically creates an &mut Point token
p.print()                       // OK; the previous token was released when the shift call ended

// Multiple &T tokens may coexist—Dup types may be copied freely
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
print(px_ref)                    // OK; the token remains in scope
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

> Note: After a closure (function value) escapes, the scope in which it was defined may no longer be
> alive, so it must not implicitly capture outer variables. However, the scope at the call site (the
> creation point) is necessarily alive, so fixing the context there as a value captured by the
> closure is safe.

### 12.4 Automatic Borrow Selection

At the call site, the compiler selects automatically according to the following priority:

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

**Method receivers follow signature semantics** (the same convention as RFC-011a's receiver
spelling): a receiver of `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value →
Move (consumes the receiver). Borrow tokens created at a call site are released when the call ends
(transient; interval semantics in §12.5). An interface's borrow receiver must be explicitly declared
by the interface author as `&Self`, and after replacing `Self ↦ impl type` in the impl signature, it
must exactly match the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is the **borrow Hore proposition** (RFC-009a), not a separate
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification. Token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and a derived &T cannot be live at the same time
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ A token is released automatically when its scope ends
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

### 12.6 Compiler Internals: Brand Mechanism

Users never interact with brands directly. Internally, the compiler assigns each token a unique
compile-time identifier:

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N 是编译期唯一整数
&mut Point     →  WriteToken(Point, #M)   // #M 是编译期唯一整数
```

The purposes of brands are:

- **Forgery prevention**: A token may only be obtained from its owner's capsule and cannot be
  constructed from nothing.
- **Association tracking**: A derived `&Float` for field access carries a derived brand
  (`#N.field_x`), allowing the compiler to trace it back to the parent token.
- **Conflict detection**: A WriteToken and a derived ReadToken from the same source cannot be live
  simultaneously.

Brands disappear completely after monomorphization and inlining and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken（冻结源数据 → Dup 安全）
               | &mut T      // WriteToken（独占读写 → Linear）
```

### 12.8 Borrow Tokens vs. `ref`

|                 | `&T` / `&mut T`                                               | `ref`                                              |
| --------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Purpose         | Inspect/modify in place                                       | Shared ownership                                   |
| Scope           | Follows the token value's scope                               | Crosses scopes                                     |
| Cost            | Zero overhead (zero-sized type; disappears after compilation) | Rc or Arc (selected by the compiler)               |
| Escape          | Possible (tokens propagate through return values/structs)     | Intended for escaping                              |
| Cross-task      | Not supported (tokens do not implement cross-task transfer)   | Supported (the compiler automatically selects Arc) |
| Cycle detection | Not applicable                                                | Silent within a task; a lint across tasks          |

> Note (undefined): How to read the contents of a `ref` after creation (dereference/method/automatic
> behavior) has not yet been defined in the specification; the current implementation reports E1052
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

// === Termination measure (built-in predicate, see §8.4) ===

// Unary: the anchor is the binding name (self-recursive functions, loops)
// The body must end with a value—the value of `while` = the value of the loop-body block = the tail expression (spec §2.9)
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

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Properties Quick Reference

```
// === Move (default) ===
// All types are Move by default. Assignment, argument passing, and return transfer ownership

// === Primitive value types (compiler built-ins) ===
Int, Float,     // The value is copied automatically on assignment; the two values are completely independent
Bool, Char      // Not Dup; built-in compiler handling of primitives

// === Dup (shallow copy: copies the handle and shares the underlying data) ===
&T              // Zero-sized read token; copying the token provides multiple views of the same data
ref T           // Copying an Rc/Arc increments the reference count and shares the heap data
String, Bytes   // Internal reference counting; assignment copies the handle and shares the underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) → Dup (#398)

// === No derivation (remain Move) ===
Vec/Dict/Set    // Containers and enums are outside the scope of derivation (#398)

// === Linear ===
&mut T          // Zero-sized write token; Linear (exclusive access, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // Creates an independent copy; modifying it does not affect the original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token; freezes source data → Dup (copyable)
&mut T          // Zero-sized compile-time write token; exclusive read/write access → Linear (cannot be copied)

// Automatic call-site selection
// 1. The argument is used afterward → create a token
// 2. The argument is not used afterward → Move
// 3. Priority: &T < &mut T < Move

// Token propagation
// ✅ May be returned, stored in a struct, or captured by a closure
// ❌ May not cross tasks (tokens do not implement cross-task transfer)
```
