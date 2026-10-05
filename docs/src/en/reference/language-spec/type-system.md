# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between a programming language's type system and mathematical logic:

| Logic                                          | Programming Language                        |
| ---------------------------------------------- | ------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                 |
| Proof \(p: P\)                                 | Program `x: T = ...`                        |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                    |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`               |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                 |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                 |
| True \(\top\)                                  | `Void` (Unit, has default)                  |
| False \(\bot\)                                 | `Never` (zero constructors, no inhabitant)  |
| Type universe \(Type_n : Type_{n+1}\)          | Universe levels (prevent Russell's paradox) |
| case analysis                                  | Type-level `match`                          |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checks.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as case analysis + recursive calls on `Add` over `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checks.
- **Type checking is verifying proofs**. When a program passes type checking, it is equivalent to a
  logical proposition being constructively proven.

### 0.3 Impact on Language Design

The concrete embodiment of the Curry-Howard correspondence in YaoXiang:

1. **Universe levels** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive calls on the natural number
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checks
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to bounded
   quantification of "for every integer N there exists a type"

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

> **Design note**: Although RFC-010 proposes the unified model of "everything is an assignment"
> (`name: type = value`), at the syntax level, types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), and `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the
> implementation, indicating "this position expects a type."

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logic Correspondence | Description                                                                                         | Default Size |
| -------- | -------------------- | --------------------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                    | Meta type                                                                                           | 0 bytes      |
| `Never`  | ⊥ (false/empty type) | Zero constructors, no value at all. Return type for divergence/panic. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)        | Has a default void value, zero-field product type. `x: Void = <default>` is valid.                  | 0 bytes      |
| `Bool`   | —                    | Boolean value: `true` / `false`                                                                     | 1 byte       |
| `Int`    | —                    | Signed integer                                                                                      | 8 bytes      |
| `Float`  | —                    | Floating point                                                                                      | 8 bytes      |
| `String` | —                    | UTF-8 string                                                                                        | Variable     |
| `Char`   | —                    | Unicode character                                                                                   | 4 bytes      |
| `Bytes`  | —                    | Raw bytes                                                                                           | Variable     |

Width-specified integers: `Int8`, `Int16`, `Int32`, `Int64`; width-specified floats: `Float32`,
`Float64`. The complete built-in type name table is in `MonoType::from_builtin_name` at
`src/frontend/core/types/mono.rs:618-640`.

> **There is no unsigned integer type**. `Uint` (and `Int128`) are not usable type names—`Uint` only
> appears in the LSP completion candidate list at `src/lsp/world.rs:176` and the `sizeof` fallback
> branch at `src/frontend/core/types/eval/const_eval.rs:503`, neither of which is a type
> registration. When you need unsigned semantics, use `Int` with upper/lower bound conventions
> yourself.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤) respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which code can pass type checking (although it will never be executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered via the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — exactly one inhabitant (the default void value). `Void` is the identity of
the zero-field product type. `x: Void = <default>` is valid. The value of a block is given by the
**tail expression** (an empty block `{}` is `Void`), see
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
// simple record type
Point: Type = { x: Float, y: Float }

// empty record type
Empty: Type = {}

// record type with generics
Pair: (T: Type) -> Type = { first: T, second: T }

// record type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Rules**:

- Record types are defined using curly braces `{}`
- The field name is directly followed by a colon and the type
- Interface names written in the type body indicate implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to the namespace of `Point`. It does not trigger any implicit binding. To make the `.`
> call syntax like `p.draw()` work, you must explicitly bind: `Point.draw = draw[0]`. See RFC-004
> and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, optional when constructing:

```yaoxiang
// field with default value - optional when constructing
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// fields without default value - required when constructing
Point2: Type = {
    x: Float,
    y: Float
}

// usage
Point2(x=1, y=2) // correct
Point2()          // error
```

**Rules**:

- `field: Type = expression` -> has a default value, optional when constructing
- `field: Type` -> no default value, required when constructing

#### 3.1.2 Built-in Bindings

Methods can be bound directly in the type definition body:

```yaoxiang
// method 1: reference an external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bind to position 0
}
// call: p1.distance(p2) -> distance(p1, p2)

// method 2: anonymous function + position binding
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = b.y - a.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// syntax: ((params) => body)[position]
// call: p1.distance(p2) -> distance(p1, p2)
```

### 3.2 Interface Types

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Syntax**: An interface is a record type whose fields are all function types

```yaoxiang
// interface definition
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// empty interface
EmptyInterface: Type = {}
```

**Interface implementation**: A type implements an interface by listing the interface name at the
end of its definition

```yaoxiang
// type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // implements Drawable interface
    Serializable     // implements Serializable interface
}
```

**Direct interface assignment**: A concrete type can be directly assigned to an interface type
variable (structural subtyping)

```yaoxiang
// direct assignment (concrete type determinable at compile time -> zero-cost call)
d: Drawable = Circle(1)
d.draw(screen)        // after compilation: direct call to circle_draw, no vtable

// function return value (cannot determine at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // look up method via vtable

// interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategies**:

| Scenario                           | Inference Result           | Call Method                 |
| ---------------------------------- | -------------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value              | Unknown                    | vtable                      |
| Heterogeneous collection           | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, closing statement)**: YaoXiang's interfaces are
structural types (interface = record with all function-typed fields), not nominal traits—there is no
"who can implement for whom" ownership issue across crates/modules, and Rust-style orphan rules and
coherence checks have no applicable target (see RFC-011 §2.1 for the ruling record). The
corresponding guarantee in the structural world is **duplicate implementation rejection**: duplicate
definitions of the same method signature on a type are compile errors (RFC-011a §3, overriding is
prohibited; overloading is allowed).

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

Generic parameters are part of the function type, using the same `()` syntax as ordinary parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In generic type definitions, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` represents the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they are treated the same as
user-defined generics and processed through the unified generic instantiation path. The ownership of
length information is the fundamental difference between the three container concepts:

| Type          | Length        | Semantics                             | Foundation                               |
| ------------- | ------------- | ------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)  | Core primitive (stack/inline priority)   |
| `Vec(T)`      | Runtime value | Runtime-length raw buffer, growable   | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list) | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                     | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: defined by YaoXiang itself in
> `std.list`, treated the same as user-defined generic records. All growable semantics strategies
> (when to grow, by how much, whether sharing is allowed) are in the library, and the compiler does
> not participate. `Vec(T)` is the minimal foundational primitive it depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When the need arises,
> complete it following the Dict pattern.

Key rules:

- **Literal landing point is determined by context**: The bare literal `[...]` with a `List(T)`
  annotation lands on the growable list; when an `Array(T, N)` annotation directly applies to the
  literal, it lands on a fixed-length array. Landing point validation: element count == N, element
  type compatible with T, otherwise compile-time E1002; when N is a symbolic constant (const
  parameter), the count validation is deferred to the refined type phase.
- **Implicit List→Array conversion is prohibited**: Fixed-length is guaranteed by the type
  layer—`push` only accepts a `List(A)` receiver.
- **Performance hierarchy**: From bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index failure contract** (runtime errors are a transitional state; the target state is covered
  by compile-time refinement, using value-dependent types, see §8.4):
  - Index out of bounds (including negative indices) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: Returns `Bool` without error, the right operand covers
  List/Array/Dict(key)/Tuple/String/Range. A first-class Hoare predicate, the basis for propositions
  provable at compile time in refinement types.

In generic functions, type parameters are also declared in the signature, and the compiler
automatically infers from the actual arguments:

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 Generic Type Definitions

```yaoxiang
// basic generic type
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
`residual` / `from_error`). On failure, the result of `from_error(residual(t))` is returned early
from the current function; on success, the expression value is `success(t)`. The return type of the
outer function must also implement `Try`, and its failure residual type must be consistent with the
receiver (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` provide `Try` implementations
via `std.result` / `std.option` (the failure residual type of `Option` is `Void`); user-defined sum
types write `Try(Self, T, E)` in the type body to plug into `?`. The lowering of `?` does not
distinguish between built-in and user types—same interface, same path.

The **variant set of a sum type enters the checker with the definition or `use` import**:
`Result`/`Option` must be preceded by `use std.result` / `use std.option` (whole module and group
`use std.{...}` have equal authority); variant construction (type-qualified form), match variant
deconstruction, and exhaustiveness determination all use this registration as the sole criterion;
the `Result(Float, Error)` returned by native function signatures casts a same-name `Generic`, which
is identical by name to the sum type in `std.result`, and can be match-deconstructed after `use`.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a constructor parameter, the field name is the parameter name; fields with
default values can be omitted when constructing, fields without default values are required.
Function-typed fields (methods) do not generate constructor parameters.

```yaoxiang
// type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> constructor parameter required
    extra: T,
}
// automatically expanded full form (compiler internal view, not required for users to write by hand):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// call: call the automatically generated constructor
c  = Container(42, 43)            // constructor parameters filled in field order; T auto-unpacked from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type parameter + positional constructor parameters
c4 = Container(Int)(extra=43, value=42)  // by field name, any order
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// field default values -> constructor parameters can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Calling rules** (single parenthesis, match declared parameters position by position, from left to
right):

1. Actual arguments try to match the type-declared parameters position by position: the `Type`
   position accepts type arguments, the compile-time value parameter position (such as `Int`)
   accepts compile-time constants.
2. If a compile-time value parameter position matches successfully (partial match), process as type
   construction: check all parameter positions one by one, and **report the first
   mismatching/missing parameter first** in declaration order when reporting errors.
3. If the actual arguments completely do not correspond to the declared parameters (all are values,
   no compile-time value parameter position matches), process as constructor parameters:
   positionally fill by field order, and type parameters are automatically unpacked from element
   types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two levels: type + constructor parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 doesn't match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols missing -> report first error first: T expected Type, found 42
Container(42) // ❌ missing constructor parameter extra
Container(42, 43, 44)  // ❌ constructor parameters overflow
```

**Type inference**: Type parameters of generic type constructors are automatically unpacked from
constructor parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions
are automatically unpacked from actual argument types (`map(numbers, f)` → T=Int, R=String, see
§4.1). When unpacking is not possible, you must explicitly fill them in.

---

## Chapter 5: Type Constraints

### 5.1 Single Constraint

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// interface type definition (as constraint)
Clone: Type = {
    clone: () -> Clone
}

// using constraint
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

> **Sources of constraint resolution (RFC-011b)**: Resolution of operator constraint names (`Add` /
> `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) = look up the interface
> implementation registry—`T: Add` ≜ registered `Add(T, T, T)` instantiation; `Equal` has additional
> structural inference (records with all comparable fields are automatically comparable). Names like
> `Zero` / `One` / `PartialOrd` do not yet have a definition source, and are dangling constraint
> names.

```yaoxiang
// multiple constraint syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// sorting generic containers
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 Function Type Constraints

```yaoxiang
// higher-order function constraints
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

// using associated type
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
// more complex associated type
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // associated type is also generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-time Generics

### 7.1 Compile-time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // compile-time constant (candidate)
```

> The basis for judgment is **being referenced in a type position**, not "annotated with a specific
> type": in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters (neither
> appears in any type position).

**Terminology**: Generic parameters annotated with a concrete type other than `Type` (such as `Int`)
are called **compile-time value parameter candidates**. Whether they become compile-time value
parameters depends on whether their value is referenced in a type position (value-dependent). **No
`const` keyword is needed** (the implementation internally used "const generic" to refer to it; the
documentation uniformly uses "compile-time value parameter").

**Decision rules (two steps)**:

1. **Form coarse filtering**: Parameter annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use fine filtering**: The candidate name appears in a **type position** (type body field type,
   inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction argument position)
   → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing                                                    | Decision                                  | Reason                                          |
| ---------------------------------------------------------- | ----------------------------------------- | ----------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters              | Only appear in value position                   |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter            | N is in the type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter            | N serves as the type of inner parameter k       |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N not referenced in type body                   |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish between compile-time constants and runtime values. Fall-through candidates (form is a
candidate, but use does not hit) degenerate to runtime value parameters—both function-level and
type-constructor paths handle this way.

```yaoxiang
// compile-time value parameter: N referenced in type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in the type construction argument position -> compile-time value parameter
    length: N
}

// usage: factorial(5) evaluated in type position (compile-time), result 120 embedded in type
arr: Measure(Int, factorial(5))  // the compiler calculates factorial(5) = 120 at compile time

// value-dependent: N as the type of inner parameter k
// N is the compile-time value parameter (appears in the type position of (k: N));
// k is the runtime value parameter, its type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 Compile-time Constant Arrays

```yaoxiang
// matrix type usage
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// compile-time dimension validation
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
// type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// example: compile-time branch
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue bridging and Assert refinement type (see §8.3 for details)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, divergence/compile error
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 Type Families

```yaoxiang
// compile-time type conversion
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 8.3 Assert Refinement Types and assert Assertions

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch routing pipeline based on "whether the predicate's free variables are accessible at compile
time."

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch routing rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                    |
| ------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never uninhabitable)    |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refinement facts into the flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (kill set).
At branch confluence, Γ takes the intersection of each branch.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, belonging to the same core primitives as `Int`, `Never`
(built-in name, not a keyword). It binds a **measure** to a computation, declaring that the
computation terminates and providing a witness of termination.

**Forms**: two arities, same predicate:

| Form                    | Anchor                                      | Purpose                                                                                                                                |
| ----------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The name of the binding where it is located | Default form—self-recursive functions, loops                                                                                           |
| `Terminates(FnType, m)` | Explicit function type                      | When the measure ownership needs to be explicitly specified (measure defined elsewhere, the same measure serves multiple computations) |

```yaoxiang
// measure: ordinary function, unit-testable, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// binary form: measure defined elsewhere, explicitly specify ownership
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// unary form: anchor is the binding name, measure is an expression in the scope
// the loop body is a `{}` block, its value is given by the tail expression (spec §2.9): only when an expression is written at the end of the body is it not `Void`
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
on back-edges for loops—each being "the measure of the next state is strictly less than the measure
of the current state," judged under the path guard of that point.

The call-site obligation is `m(callee_args) < m(caller_args)`; the back-edge obligation is
`m(next_round) < m(this_round)`. The two are formally identical.

> **Why it covers loops**: Loops are anonymous constructs, usually not referable. The binding name
> is the name—the `acc` in `acc: Terminates(n - i) = while ...` provides the anchor, so the loop is
> referable. This is why the unary form of `Terminates` applies to loops.

**Measure**: No restriction on return type (not forced to be a natural number); "strictly
decreasing" on it is given by the well-order available on that type. Whether the measure is
well-founded (such as whether returning `Int` is `>= 0`) is an **independent obligation**, also
judged by the compile-time proof pipeline, as is the decreasing obligation.

**Trigger**: Termination checks are triggered by **refinement types**—once a type is refined, it
enters verification mode. Plain types that are not refined (such as bare `while` loops, functions
without refined signatures) do not enter verification mode and do not generate termination
obligations.

**Automatic exploration first**: The compiler first automatically explores measures (four templates:
linear rank function, predicate violation count, bounded increase/decrease, multiplicative scaling),
and only when exploration fails does an explicit `Terminates` need to be given.

**Runtime representation**: A pure compile-time entity, erased with the witness, does not enter the
runtime binary.

> The complete design is in [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md)
> (semantics) and [RFC-027a](../../rfc/review/027a-termination-explicit-measure.md) (implementation
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

**Syntax**: The type intersection `A & B` represents a type that satisfies both A and B

```yaoxiang
// interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// using intersection type
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## Chapter 10: Function Overloading and Specialization

### 10.1 Function Overloading

```yaoxiang
// basic specialization: use function overloading (compiler automatically selects)
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// general implementation
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
// platform type enum (defined by the standard library)
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P is a predefined generic parameter name, representing the current compilation platform
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 11: Type Attributes

YaoXiang has only one type attribute that needs to be distinguished: linear vs copyable.
Automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, passing arguments, returning = ownership
transfer.

```yaoxiang
// struct with heap buffer field does not derive Dup (field falls into Move) -> default Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move, b cannot be read again
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup attribute is used for reference/token types and handle value types with internal reference
counting**. Assignment of a Dup type = shallow copy—copy the handle/token, the underlying data is
shared. Multiple holders point to the same block of data.

| Type            | Attribute | Description                                                                             |
| --------------- | --------- | --------------------------------------------------------------------------------------- |
| `&T`            | Dup       | Zero-size read token, copying the token = multiple views pointing to the same data      |
| `ref T`         | Dup       | Rc/Arc copy = reference count +1, share heap data                                       |
| String, Bytes   | Dup       | Internal reference counting, assignment copies the handle sharing the underlying buffer |
| `&mut T`        | Linear    | Zero-size write token, exclusive, non-copyable                                          |
| struct          | Derived   | All fields copyable (primitive value types ∪ Dup) → Dup, otherwise Move (#398)          |
| tuple           | Derived   | Per-element judgment, same as struct rule (#398)                                        |
| All other types | Move      | Default ownership transfer                                                              |

**Primitive value types** (Int, Float, Bool, Char) are special processing built into the compiler:
automatic value copy on assignment, the two values are completely independent. This is the
compiler's native behavior and does not belong to the Dup type attribute.

**Derivation rules** (#398 final)—"automatically derived when all fields are Dup" cannot be executed
literally: primitive fields (Int etc.) are not Dup themselves, `{ x: Int, y: Int }` would be
misjudged as Move. Executable form:

1. **Copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), already-Dup composite types);
2. **struct**: All fields are in the copyable field set → derived Dup; **any** field falls into
   Linear (`&mut T`) or Move (nested Move struct / containers like Vec, Dict / resources) → overall
   remains Move (no intermediate state of "partially copyable" is introduced);
3. **tuple**: Same rule as struct, per-element judgment; empty tuple (unit) is `Void`;
4. **Derivation is recursive**: When a field is a named type (such as `target: Point`), expand its
   definition before judgment, `A = { b: B }` follows B's derivation result; circular aliases fall
   conservatively to Move by depth limit;
5. **Not in derivation range** (still Move, separate case): containers (Vec / Dict / Set / Option /
   Result / Array) and enum.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy token, both are valid
print(view.x)    // available
print(view2.x)   // available

// &mut T: Linear, non-copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot copy
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing the
`.clone()` method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // deep copy, p still available
p2 = p.clone()        // can be cloned multiple times
```

**Differences between Dup and Clone**:

|                         | Dup                                                    | Clone                                         |
| ----------------------- | ------------------------------------------------------ | --------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, share underlying data | Deep copy: create a complete independent copy |
| **Call method**         | Implicit (automatic on assignment/argument passing)    | Explicit (`.clone()`)                         |
| **Modification impact** | Affect each other (share underlying data)              | Do not affect each other (independent copies) |
| **Applicable types**    | `&T` token, `ref T`                                    | Any type that implements the Clone interface  |
| **Cost**                | Zero overhead (token is zero-size type)                | Depends on the type                           |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, share underlying data
view: &Point = &p
view2 = view        // Dup: copy token, both point to the same p
print(view.x)       // available
print(view2.x)      // available, seeing the same data

// primitive value type: compiler automatic value copy (not Dup)
x: Int = 42
y = x               // value copy, x and y completely independent
print(x)            // available

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p still available

// non-Dup type (field falls into Move, not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains Vec field, not derived Dup (see §11.2 derivation rules)
```

**Design intent**:

- Dup is used for token/reference types to solve the problem of "multiple views seeing the same
  data"
- Clone is used for scenarios that need independent copies, and explicit calls make the cost visible
- The copying of primitive value types (Int/Float/Bool/Char) is the compiler's built-in behavior,
  not Dup
- User-defined types default to Move (zero-copy high performance); when all fields are copyable, Dup
  is automatically derived (§11.2 derivation rules)

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references" but
"type-level proof of access permission."

```
&T      →  zero-size, freezes source data (forbids WriteToken acquisition during this period),
          multi-read safety under freeze guarantee -> Dup (copyable)
&mut T  →  zero-size, exclusive read/write (forbids any other token),
          copying is meaningless under exclusive access -> Linear (non-Dup)
```

**Key features**:

- Tokens are **ordinary types**, following the same scope rules as all other types
- No lifetime annotation `'a` is needed
- No dedicated borrow checker is needed—type attributes (Dup/Linear) naturally derive permissions
- Disappears completely after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// method end: declare parameter type, decide the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// call site: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // compiler automatically creates &Point token
p.shift(1.0, 1.0)               // compiler automatically creates &mut Point token
p.print()                       // OK, the previous token has been released as the shift call ended

// multiple &T tokens coexist — Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations of ordinary types:

**Return token**—the token propagates with the return value:

```yaoxiang
// ✅ sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to caller
print(px_ref)                    // OK, token still in scope
```

**Store in struct**—a struct can carry token fields:

```yaoxiang
// ✅ struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,              // token field — holds a read-only view of target
}
```

**Closures do not capture, context is fixed at the creation point**—closures only eat their own
parameters; when external data is needed, the value is fixed into the closure at the creation point
through currying:

```yaoxiang
// ✅ context is fixed through currying: threshold is the parameter, gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, its definition-site scope may be dead, so it must
> not implicitly capture outer variables; but the call site (creation point) scope is necessarily
> alive, and it is safe to fix the context as a value into the closure at that point.

### 12.4 Automatic Borrow Selection

The call-site compiler automatically selects according to the following priority:

```
1. If the actual argument is still used later -> prefer creating a token (&T or &mut T, depending on the method signature)
2. If the actual argument is not used later -> Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point -> compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point -> compiler creates &mut Point token
p2 = p             // not used later -> Move
```

**Method receiver follows signature semantics** (same as the RFC-011a receiver spelling convention):
receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value → Move (consume
receiver). The borrow token generated at the call site is released when the call ends (transient,
§12.5 interval semantics); the interface's borrow receiver is explicitly declared by the interface
author as `&Self`, and the impl signature must be exactly consistent with the interface after
`Self ↦ impl type` substitution (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§reverse BFS liveness analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be active simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ automatically released after the token scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // use &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken still available
}

// ❌ the same actual argument cannot simultaneously create &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Brand Mechanism

Users never touch brands. The compiler internally assigns a compile-time unique identifier to each
token:

```
User sees               Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

The uses of brands:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Association tracking**: The `&Float` derived from field access carries the derived brand
  (`#N.field_x`), and the compiler can track it to the parent token
- **Conflict detection**: WriteToken and derived ReadToken from the same source cannot be active
  simultaneously

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken (freeze source data -> Dup safe)
               | &mut T      // WriteToken (exclusive read/write -> Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                              | `ref`                                    |
| --------------- | ------------------------------------------------------------ | ---------------------------------------- |
| What it does    | Take a look / modify in place                                | Shared ownership                         |
| Scope           | Follows the scope of the token value                         | Cross-scope                              |
| Cost            | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler selects)             |
| Escape          | Yes (token propagates with return value/struct)              | Originally for escaping                  |
| Cross-task      | Not possible (tokens not implemented for cross-task passing) | Yes (compiler automatically selects Arc) |
| Cycle detection | Not involved                                                 | Silent within task, cross-task lint      |

> Note (undefined): How to read content after ref is created (dereference/method/automatic) has not
> been defined in the specification, and the current implementation `*a` reports E1052. To be
> supplemented in this section after definition.

---

## Appendix: Type Definition Cheat Sheet

### A.1 Type Definitions

```
// === Record types (curly braces) ===

// record type
Point: Type = { x: Float, y: Float }

// record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Interface types (curly braces, all fields are functions) ===

// interface definition
Serializable: Type = { serialize: () -> String }

// type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // implements Serializable interface
}

// === Function types ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (built-in predicate, see §8.4) ===

// unary: anchor is the binding name (self-recursive functions, loops)
// the body tail must give a value — `while` value = loop body block value = tail expression (spec §2.9)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}

// binary: explicitly specify measure ownership (measure defined elsewhere)
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 Generic Syntax

```
// generic type
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// generic function
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// type constraint
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// associated type
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// compile-time generic: N referenced in type position (k: N) -> compile-time value parameter
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Attribute Cheat Sheet

```
// === Move (default) ===
// All types default to Move. Assignment, passing arguments, returning = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // automatic value copy on assignment, the two values are completely independent
Bool, Char      // not Dup, is the compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // zero-size read token, copying the token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count +1, share heap data
String, Bytes   // internal reference counting, copy handle shares the underlying buffer
struct / tuple  // derived: all fields ∈ (primitive value types ∪ Dup) -> Dup (#398)

// === Not derived (remain Move) ===
Vec/Dict/Set    // containers and enum are not in the derivation range (#398)

// === Linear ===
&mut T          // zero-size write token, Linear (exclusive, non-copyable)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent copy, modifications do not affect the original value
```

### A.4 Borrow Token Cheat Sheet

```
// === Borrow tokens ===
&T              // zero-size compile-time read token, freezes source data -> Dup (copyable)
&mut T          // zero-size compile-time write token, exclusive read/write -> Linear (non-copyable)

// automatic selection at the call site
// 1. actual argument still used later -> create a token
// 2. actual argument not used later -> Move
// 3. priority matching: &T < &mut T < Move

// token propagation
// ✅ can be returned, stored in struct, captured by closure
// ❌ cannot cross tasks (tokens not implemented for cross-task passing)
```
