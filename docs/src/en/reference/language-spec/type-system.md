# Type System Specification

This file defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals a deep correspondence between programming language type systems and mathematical logic:

| Logic                                          | Programming Language                            |
| ---------------------------------------------- | ----------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                     |
| Proof \(p: P\)                                 | Program `x: T = ...`                            |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                        |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                   |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                     |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                     |
| Truth \(\top\)                                 | `Void` (Unit, with a default value)             |
| Falsity \(\bot\)                               | `Never` (zero constructors, no inhabitant)      |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as `Add` doing case analysis + recursive calls over `Nat`) are essentially a
  type-level encoding of mathematical induction — provided the compiler can perform termination
  checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proved.

### 0.3 Impact on Language Design

The Curry-Howard correspondence is concretely manifested in YaoXiang as:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive calls on the natural number
   `Nat(Zero/Succ)` correspond to Peano axioms — provided the compiler performs termination checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
   quantification "for every integer N there exists a type"

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

> **Design note**: Although RFC-010 proposes the unified model of "everything is assignment"
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), and `TypeExpr`, as a BNF placeholder, corresponds to the `Type` enum in the
> implementation, meaning "a type is expected at this position."

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Counterpart  | Description                                                                               | Default Size |
| -------- | -------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                    | Meta type                                                                                 | 0 bytes      |
| `Never`  | ⊥ (false/empty type) | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)        | Has a default void value, a zero-field product type. `x: Void = <default>` is valid.      | 0 bytes      |
| `Bool`   | —                    | Boolean values: `true` / `false`                                                          | 1 byte       |
| `Int`    | —                    | Signed integers                                                                           | 8 bytes      |
| `Float`  | —                    | Floating-point numbers                                                                    | 8 bytes      |
| `String` | —                    | UTF-8 strings                                                                             | variable     |
| `Char`   | —                    | Unicode characters                                                                        | 4 bytes      |
| `Bytes`  | —                    | Raw bytes                                                                                 | variable     |

Integers with explicit bit width: `Int8`, `Int16`, `Int32`, `Int64`; floats with explicit bit width:
`Float32`, `Float64`. The complete list of built-in type names is in `MonoType::from_builtin_name`
at `src/frontend/core/types/mono.rs:618-640`.

> **No unsigned integer types**. `Uint` (and `Int128`) are not usable type names — `Uint` only
> appears in the LSP completion candidate list at `src/lsp/world.rs:176` and the `sizeof` fallback
> branch at `src/frontend/core/types/eval/const_eval.rs:503`; neither is a type registration. When
> unsigned semantics are needed, use `Int` with explicit lower/upper bounds.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system — corresponding to falsity (⊥) and
truth (⊤), respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: no literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side that can be written.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   and subsequent code passes type checking (even though it is never executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered through the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity element of zero-field product types. `x: Void = <default>` is valid. The value of a block
is given by its **tail expression** (empty block `{}` has type `Void`); see
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
- A field name is directly followed by a colon and a type
- An interface name written inside the type body means the interface is implemented

> **Namespace ownership**: The `Type.name` prefix (such as `Point.draw`) means the function belongs
> to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call syntax such
> as `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and
> RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, which are optional when constructing:

```yaoxiang
// Fields with default values - optional at construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Fields without default values - required at construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // correct
Point2()          // error
```

**Rules**:

- `field: Type = expression` -> has a default value, optional at construction
- `field: Type` -> no default value, required at construction

#### 3.1.2 Built-in Bindings

Methods can be bound directly inside a type definition body:

```yaoxiang
// Method 1: reference an external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bind to position 0
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
    Drawable,        // implements the Drawable interface
    Serializable     // implements the Serializable interface
}
```

**Direct interface assignment**: A concrete type can be directly assigned to an interface-typed
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type known at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: directly call circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Look up method through vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inference Result           | Call Method                 |
| ------------------------------------ | -------------------------- | --------------------------- |
| Direct assignment of a concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value                | Unknown                    | vtable                      |
| Heterogeneous collection             | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable; closure note)**: YaoXiang's interfaces are structural
types (an interface = a record whose fields are all function types), not nominal traits — there is
no "who can implement for whom" attribution problem across crates/modules, and Rust-style orphan
rules and coherence checks have no applicable subject (see RFC-011 §2.1 for the ruling record). The
corresponding guarantee in the structural world is **duplicate implementation rejection**: defining
a method with the same signature on a type twice is a compile error (RFC-011a §3, overriding is
forbidden; overloading is allowed).

### 3.4 Tuple Types

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 Function Types

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= Param (',' Param)*
Param       ::= Identifier ':' TypeExpr    // named parameter
              | TypeExpr                   // unnamed parameter (bare type)
```

**Two parameter forms, with different name ownership** (the type side of RFC-007 shorthand rules):

- **Named** (`a: Int`): the name enters the contract — it can be referenced by named arguments
  (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)), and when the signature has already
  declared parameter names and types, the lambda head can be omitted
  (`add: (a: Int, b: Int) -> Int = a + b`, RFC-007 §Shorthand Rules).
- **Unnamed** (bare type, e.g. `(Int, Int) -> Int`): type constraints apply **by position**, and the
  name belongs to the implementation — the lambda head must supply its own parameter names
  (`mk: (Int, Int) -> Int = (x, y) => x + y`, RFC-007: the lambda head can be omitted if and only if
  the signature has declared parameter names); it cannot be referenced by named arguments and can
  only be called positionally. A bare identifier resolves in the type namespace; if no declared type
  matches, an error is reported — there is no "neither type nor type parameter" bare identifier in a
  parameter position.

**Annotation binding semantics**: An annotation is a declaration, not a comment. A binding statement
checks the implementation against the annotation — the parameter types drive the lambda head by
position (when the implementation supplies explicit types, they must match the annotation), the
function body's exits (tail expression and `return`) are uniformly checked against the annotated
return type, and the type registered to the caller for the binding is the annotated form. Bindings
without annotations use HM inference (RFC-007). `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are rejected under the same rule — binding semantics do not fork
based on the syntactic form of the value (lambda / block / expression).

> **Implementation status**: Bindings with non-`Fn` annotations are already checked; bindings with
> `Fn` annotations and the type resolution of bare identifiers will land with D53/D54 of
> [RFC-039](../../rfc/accepted/039-compiler-architecture.md) (P6/P8).

---

## Chapter 4: Generics

### 4.1 Generic Parameter Syntax

Generic parameters are part of the function type and uniformly use the `()` syntax with regular
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

Container types are generic type constructors, not built-in primitives — they are treated the same
as user-defined generics and handled through the unified generic instantiation path. The attribution
of length information is the fundamental difference among the three container concepts:

| Type          | Length        | Semantics                                | Foundation                               |
| ------------- | ------------- | ---------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type-level    | Fixed-length array (const generic N)     | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Raw buffer with runtime length, growable | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list)    | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                        | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list`, treated the same as a user-defined generic record. All growable semantics
> (when to grow, by how much, whether sharing is allowed) live in the library — the compiler does
> not participate. `Vec(T)` is the minimal ground primitive it depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When needed, fill it
> in following the Dict pattern.

Key rules:

- **Literal target is determined by context**: a bare `[...]` literal combined with a `List(T)`
  annotation lands in a growable list; an `Array(T, N)` annotation applied directly to a literal
  lands in a fixed-length array. Landing validation: element count == N, element type compatible
  with T; mismatches yield compile-time E1002. When N is a symbolic constant (const parameter), the
  count check is deferred to the refinement typing phase.
- **No implicit List→Array conversion**: fixed-length is guaranteed at the type level — `push` only
  accepts `List(A)` receivers.
- **Performance hierarchy**: from bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index failure contract** (runtime error is a transitional state; the target state is
  compile-time refinement coverage via value-dependent types, see §8.4):
  - Index out of bounds (including negative indices) → `E6003`
  - Dict missing key → `E6008`
- **String indexing convention** (#385): `s[i]` indexes by **Unicode scalar value (code point)** to
  fetch a character, producing a single-character `String` — same unit as `std.string`'s `len` /
  `index_of` / `substring` / `chars` / `char_code`, consistent when combined; out-of-bounds
  (including negative indices) follows the index failure contract `E6003` above.
- **`in` membership predicate**: returns `Bool` without erroring; the right operand covers
  List/Array/Dict(key)/Tuple/String/Range. A first-class Hoare predicate, the basis of propositions
  provable at compile time by refinement types.`

In generic functions, type parameters are likewise declared in the signature, and the compiler
automatically infers them from arguments:

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

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation — the
receiver type must implement the `Try` interface (four members `is_failure` / `success` / `residual`
/ `from_error`); on failure, the result of `from_error(residual(t))` is returned early from the
current function, and on success the expression's value is `success(t)`. The outer function's return
type must also implement `Try`, and its failure residual type must match the receiver's (`E1081` /
`E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try` implementations through
`std.result` / `std.option` (`Option`'s failure residual type is `Void`); a user-defined sum type
opts into `?` by writing `Try(Self, T, E)` inside its type body. `?`'s lowering does not distinguish
built-in from user types — same interface, same path.

A sum type's **variant set enters the checker via definition or `use` import**: `Result` / `Option`
must be preceded by `use std.result` / `use std.option` before use (whole-module and grouped
`use std.{...}` are equivalent), and variant construction (in type-qualified form), match variant
destructuring, and exhaustiveness checking all use this registration as the sole basis; a native
function signature returning `Result(Float, Error)` forges a `Generic` of the same name, which is
identical by name to the `std.result` sum type and can be `use`d and then match-destructured.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each field
corresponds to a constructor argument, the field name is the parameter name; fields with default
values may be omitted at construction, fields without default values are required. Function-typed
fields (methods) do not generate constructor arguments.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> constructor argument required
    extra: T,
}
// Auto-expanded full form (compiler's internal view; users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Calls: calling the auto-generated constructor
c  = Container(42, 43)            // constructor arguments filled in field order; T auto-unpacked from elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type argument + positional constructor arguments
c4 = Container(Int)(extra=43, value=42)  // field-name form, order free
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default values -> constructor arguments may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x=1.5, y=2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, positional matching against declared parameters, left to right):

1. Arguments are tried positionally against the declared type parameters: a `Type` slot accepts a
   type argument; a compile-time value slot (e.g. `Int`) accepts a compile-time constant.
2. If some compile-time value slots match (partial match), proceed as type construction: check every
   parameter slot positionally, and on error report the **first mismatched/missing parameter** in
   declaration order.
3. If no argument matches the declared parameters (all are values, no compile-time value slot
   matches), proceed as constructor arguments: positional in field order, type arguments
   auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one-level type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two levels: type + constructor arguments
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ slot 0: T←42 doesn't match (42 is not a type); slot 1: Rows←42 matches;
              //    slot 2: Cols missing -> report the first error: T expected Type, found 42
Container(42) // ❌ missing constructor argument extra
Container(42, 43, 44)  // ❌ too many constructor arguments
```

**Type inference**: a generic type constructor's type arguments are auto-unpacked from constructor
argument elements (`Container(42, 43)` → T=Int); a generic function's type arguments are
auto-unpacked from argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When unpacking
fails, explicit specification is required.

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

// Using a constraint
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

> **Constraint resolution source (RFC-011b)**: resolving operator constraint names (`Add` /
> `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) means looking up the interface
> implementation registry — `T: Add` ≜ the instantiation `Add(T, T, T)` is registered; `Equal`
> additionally has structural derivation (any record whose fields are all comparable is
> automatically comparable). Names like `Zero` / `One` / `PartialOrd` have no defined source yet,
> and are dangling constraint names.

```yaoxiang
// Multiple constraints syntax
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
    Item: T,                    // associated type
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

### 6.2 Generic Associated Types (GAT)

```yaoxiang
// More complex associated types
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // associated type is also generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // compile-time constant (candidate)
```

> The criterion is **being referenced in a type position**, not "annotated with a concrete type": in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters (neither appears in
> any type position).

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (such as
`Int`) is called a **compile-time value parameter candidate**; whether it becomes a compile-time
value parameter depends on whether its value is referenced in a type position (value dependency).
**No `const` keyword is needed** (the implementation internally once used the term "const generics";
the docs uniformly use "compile-time value parameters").

**Decision rules (two steps)**:

1. **Form coarse filter**: the parameter is annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use fine filter**: the candidate name appears in a **type position** (type body field type,
   inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction argument slot) →
   true compile-time value parameter; otherwise **runtime value parameter**.

| Form                                                       | Determination                             | Reason                                         |
| ---------------------------------------------------------- | ----------------------------------------- | ---------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear in value positions                 |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N appears in a type construction argument slot |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N serves as the type of inner parameter k      |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in the type body           |

**Core design**: use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Candidates that fall through (form is a
candidate, use does not hit) degrade to runtime value parameters — both function-level and
type-constructor paths are handled this way.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in a type construction argument slot -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at the type position (compile time), result 120 embedded in the type
arr: Measure(Int, factorial(5))  // compiler computes factorial(5) = 120 at compile time

// Value dependency: N serves as the type of inner parameter k
// N is a compile-time value parameter (appears in the type position of (k: N));
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
// IsTrue bridging and Assert refinement types (see §8.3)
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

### 8.3 Assert Refinement Types and `assert` Statements

`assert` and `Assert` are two sides of the same refinement primitive — the dispatch pipeline
automatically chooses based on "whether the predicate's free variables are reachable at compile
time."

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                      |
| ------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter the proof pipeline: true → erase to Void, false → compile error (Never uninhabitable)   |
| Runtime free variables exist (function arguments, external input)                     | Runtime     | Insert a runtime Bool check, inject refinement facts into the flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (kill set).
At branch confluence, Γ takes the intersection of the branches.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, belonging to the core primitives alongside `Int` and
`Never` (built-in name, not a keyword). It binds a **measure** to a piece of computation, declaring
that the computation terminates, and provides a witness of termination.

**Form**: two arities, one predicate:

| Form                    | Anchor                  | Use                                                                                                                       |
| ----------------------- | ----------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The host binding's name | Default form — self-recursive functions, loops                                                                            |
| `Terminates(FnType, m)` | Explicit function type  | When the measure's owner must be named explicitly (measure defined elsewhere, same measure serving multiple computations) |

```yaoxiang
// Measure: a regular function, unit-testable, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: the measure is defined elsewhere, attribute specified explicitly
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: the anchor is the binding name, the measure is an in-scope expression
// The loop body is a `{}` block, whose value comes from the tail expression (spec §2.9): only when the body tail has an expression is the value not `Void`
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
position. The obligation falls on that computation — for a function, at each recursive call site;
for a loop, on the back edge — in both cases "the next state's measure is strictly less than the
current state's measure," judged under that point's path guard.

The call-site obligation is `m(callee_args) < m(caller_args)`; the back-edge obligation is
`m(next iteration) < m(this iteration)`. Both take the same form.

> **Why it covers loops**: A loop is an anonymous construct and usually cannot be referred to. The
> binding name is the name — `acc: Terminates(n - i) = while ...` supplies the anchor through `acc`,
> so the loop becomes referable. This is the reason `Terminates`'s unary form works on loops.

**Measure**: no restriction on the return type (not forced to be a natural number); the available
well-order on that type provides the "strict decrease" relation. Whether the measure is well-founded
(e.g. when returning `Int`, whether `>= 0`) is an **independent obligation**, also judged by the
compile-time proof pipeline, on par with the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types** — once a type is refined, it
enters verification mode. Ordinary types that are not refined (such as a bare `while` loop, a
function with no refinement signature) do not enter verification mode and generate no termination
obligation.

**Auto-exploration first**: the compiler first auto-explores the measure (linear rank functions,
predicate-violation counting, bounded increase/decrease, multiplicative scaling — four templates);
only when exploration fails is an explicit `Terminates` required.

**Runtime representation**: a pure compile-time entity, erased together with the witness, never
enters the runtime binary.

> See [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../rfc/accepted/027a-termination-explicit-measure.md) (landing mechanism) for the
> full design.

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

**Syntax**: the type intersection `A & B` represents the type that satisfies both A and B

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
// Basic specialization: function overloading (compiler chooses automatically)
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

## Chapter 11: Type Attributes

YaoXiang has only one type attribute to distinguish: linear vs. copyable. It is inferred
automatically by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, passing arguments, returning = ownership
transfer.

```yaoxiang
// A struct containing a heap buffer field does not derive Dup (the field falls to Move) -> defaults to Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move, b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle, Share the Data)

**The Dup attribute is used for reference/token types and handle value types with internal reference
counting**. Assignment of a Dup type = shallow copy — the handle/token is copied, and the underlying
data is shared. Multiple holders point to the same block of data.

| Type            | Attribute | Description                                                                          |
| --------------- | --------- | ------------------------------------------------------------------------------------ |
| `&T`            | Dup       | Zero-sized read token, copying the token = multiple views pointing to the same data  |
| `ref T`         | Dup       | Rc/Arc copy = refcount + 1, share heap data                                          |
| String, Bytes   | Dup       | Internal reference count, assignment copies the handle sharing the underlying buffer |
| `&mut T`        | Linear    | Zero-sized write token, exclusive, cannot be copied                                  |
| struct          | Derived   | All fields are copyable (primitive value types ∪ Dup) → Dup, otherwise Move (#398)   |
| tuple           | Derived   | Determined element by element, same as struct rules (#398)                           |
| All other types | Move      | Default ownership transfer                                                           |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: assignment
automatically copies the value, and the two values are completely independent. This is native
compiler behavior, not a property of the Dup type.

**Derivation rules** (#398 final) — "automatically derive when all fields are Dup" cannot be
executed literally: primitive fields (Int, etc.) are themselves not Dup, and `{ x: Int, y: Int }`
would be misjudged as Move. Executable form:

1. **Copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), already-Dup composite types);
2. **struct**: all fields fall in the copyable field set → derive Dup; **any** field falls to Linear
   (`&mut T`) or Move (nested Move struct / Vec, Dict and other containers / resources) → the whole
   stays Move (no "partially copyable" intermediate state);
3. **tuple**: same rule as struct, determined element by element; empty tuple (unit) is `Void`;
4. **Derivation is recursive**: when a field is a named type (e.g. `target: Point`), expand its
   definition to determine, and `A = { b: B }` follows B's derivation; cyclic aliases conservatively
   fall to Move by depth cap;
5. **Not in the derivation scope** (still Move, separate case): containers (Vec / Dict / Set /
   Option / Result / Array) and enum.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // available
print(view2.x)   // available

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is the explicit deep-copy interface. Every type can implement Clone, providing a
`.clone()` method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // deep copy, p is still usable
p2 = p.clone()        // can be cloned multiple times
```

**Difference between Dup and Clone**:

|                      | Dup                                                        | Clone                                           |
| -------------------- | ---------------------------------------------------------- | ----------------------------------------------- |
| **Semantics**        | Shallow copy: copy the handle/token, share underlying data | Deep copy: create a complete independent copy   |
| **Call style**       | Implicit (automatic on assignment / passing)               | Explicit (`.clone()`)                           |
| **Mutation effect**  | Affects each other (shared underlying data)                | Does not affect each other (independent copies) |
| **Applicable types** | `&T` tokens, `ref T`                                       | Any type implementing the Clone interface       |
| **Cost**             | Zero overhead (tokens are zero-sized types)                | Depends on the type                             |

**Dup does not imply Clone, and Clone does not imply Dup** — they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, share the underlying data
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // available
print(view2.x)      // available, seeing the same data

// Primitive value types: compiler auto value-copy (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // available

// Clone: explicit deep copy, create an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable

// Non-Dup type (field falls to Move, no derivation): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains a Vec field, does not derive Dup (see §11.2 derivation rules)
```

**Design intent**:

- Dup is used for token/reference types, solving the "multiple views of the same data" problem
- Clone is used in scenarios requiring independent copies; making the cost visible through explicit
  calls
- Primitive value types (Int/Float/Bool/Char) are copied via compiler-built-in behavior, not Dup
- Custom types default to Move (zero-copy, high performance); when all fields are copyable, Dup is
  derived automatically (§11.2 derivation rules)

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references" but
"type-level proofs of access permission."

```
&T      →  zero-sized, freezes the source data (forbids WriteToken acquisition during this period),
          under the freeze guarantee multiple read-only is safe -> Dup (copyable)
&mut T  →  zero-sized, exclusive read/write (forbids any other token),
          exclusive access makes copying meaningless -> Linear (non-Dup)
```

**Key properties**:

- The token is a **regular type**, following the same scoping rules as all other types
- No lifetime annotations `'a` are needed
- No dedicated borrow checker is required — type attributes (Dup/Linear) naturally infer permissions
- Completely disappears after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare the parameter type, determining the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call side: the compiler automatically chooses to borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates an &Point token
p.shift(1.0, 1.0)               // Compiler automatically creates an &mut Point token
p.print()                       // OK, the previous token has been released as the shift call ends

// Multiple &T tokens coexisting — Dup types allow free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are regular types, so they support all operations of regular types:

**Returning tokens** — the token propagates along with the return value:

```yaoxiang
// ✅ Sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to the caller
print(px_ref)                    // OK, the token is still in scope
```

**Storing in structs** — a struct may carry token fields:

```yaoxiang
// ✅ Struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field — holds a read-only view of target
}
```

**Closures do not capture; context is fixed at the creation site** — a closure only consumes its own
parameters; when outer data is needed, curry at the creation site to fix the value into the closure:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter; gt_point(threshold) fixes the value into the closure at the creation site
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: after a closure (function value) escapes, the scope at its definition site may be dead, so
> implicit capture of outer variables is not allowed; but the call site (creation site) scope is
> necessarily alive, so fixing context as a value into the closure at that point is safe.

### 12.4 Automatic Borrow Selection

The call-side compiler automatically selects by the following priorities:

```
1. If the argument is used later -> prefer creating a token (&T or &mut T, according to the method signature)
2. If the argument is not used again later -> Move
3. Priority of matching: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // parameter type of print is &Point -> compiler creates an &Point token
p.shift(1.0, 1.0)  // parameter type of shift is &mut Point -> compiler creates an &mut Point token
p2 = p             // no further use -> Move
```

**Method receiver follows signature semantics** (same as the receiver-spelling convention in
RFC-011a): the receiver is `&T` → a read-only borrow token; `&mut T` → a mutable borrow token; by
value → Move (consuming the receiver). The borrow token generated at the call site is released when
the call ends (transient, see interval semantics in §12.5); an interface's borrow receiver is
declared explicitly by the interface author as `&Self`, and the impl signature, after substituting
`Self ↦ impl type`, must exactly match the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrowing Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions (`borrow_conflict`
/ `use_after_move` / `use_after_drop` / `mut_violation`) and feeds them into the proof pipeline for
verification; token liveness uses the interval `[created_at, last_use]` (see RFC-009a §Reverse-BFS
Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ Token automatically released when its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // uses &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken is still available
}

// ❌ The same argument cannot create &mut and other tokens simultaneously
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives both &mut and & tokens simultaneously
```

### 12.6 Compiler Internals: Brand Mechanism

The user never touches brands. The compiler internally assigns a compile-time-unique identifier to
each token:

```
What the user sees         The compiler's internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time-unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time-unique integer
```

Uses of brands:

- **Anti-forgery**: a token can only be obtained from the owner's capsule, not constructed out of
  thin air
- **Origin tracking**: a field-access-derived `&Float` carries a derived brand (`#N.field_x`), and
  the compiler can trace it back to the parent token
- **Conflict detection**: the same-origin WriteToken and derived ReadToken cannot be live
  simultaneously

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes the source -> Dup is safe)
               | &mut T      // WriteToken (exclusive read/write -> Linear)
```

### 12.8 Borrow Tokens vs. ref

|                 | `&T` / `&mut T`                                               | `ref`                                   |
| --------------- | ------------------------------------------------------------- | --------------------------------------- |
| What it does    | A look / an in-place edit                                     | Shared ownership                        |
| Scope           | Follows the token value's scope                               | Crosses scopes                          |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler chooses)            |
| Escape          | Yes (token propagates via return value / struct)              | Designed to escape                      |
| Cross-task      | No (tokens do not implement cross-task passing)               | Yes (compiler auto-chooses Arc)         |
| Cycle detection | Not involved                                                  | Silent within a task, lint across tasks |

> Note (undefined): how to read content (dereference / method / auto) after a ref is created has not
> yet been defined in the spec; the current implementation reports E1052 for `*a`. This will be
> added once defined.

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definitions

```
// === Record types (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Interface types (curly braces, fields all functions) ===

// Interface definition
Serializable: Type = { serialize: () -> String }

// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // implements the Serializable interface
}

// === Function type ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (built-in predicate, see §8.4) ===

// Unary: anchor is the binding name (self-recursive functions, loops)
// The body tail must yield a value — `while`'s value = the loop body block's value = tail expression (spec §2.9)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}

// Binary: explicitly specify the measure's owner (measure defined elsewhere)
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 Generics Syntax

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

// Compile-time generics: N is referenced in type position (k: N) -> compile-time value parameter
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
// All types default to Move. Assignment, passing arguments, returning = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // assignment auto value-copies, the two values are completely independent
Bool, Char      // not Dup, this is the compiler's built-in handling of primitives

// === Dup (shallow copy: copy the handle, share the underlying data) ===
&T              // zero-sized read token, copying the token = multiple views pointing to the same data
ref T           // Rc/Arc copy = refcount + 1, share heap data
String, Bytes   // internal reference count, assignment copies the handle sharing the underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) -> Dup (#398)

// === Not derived (stays Move) ===
Vec/Dict/Set    // containers and enum are not in the derivation scope (#398)

// === Linear ===
&mut T          // zero-sized write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent copy, mutations do not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // zero-sized compile-time read token, freezes the source -> Dup (copyable)
&mut T          // zero-sized compile-time write token, exclusive read/write -> Linear (cannot be copied)

// Call-side auto-selection
// 1. The argument is used later -> create a token
// 2. The argument is not used again later -> Move
// 3. Priority of matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in a struct, captured by a closure
// ❌ Cannot cross tasks (tokens do not implement cross-task passing)
```
