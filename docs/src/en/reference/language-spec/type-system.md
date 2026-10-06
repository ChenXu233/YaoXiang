# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and trait.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between a programming language's type system and mathematical logic:

| Logic                                      | Programming Language                            |
| ------------------------------------------ | ----------------------------------------------- |
| Proposition \(P\)                          | Type `Type`                                     |
| Proof \(p: P\)                             | Program `x: T = ...`                            |
| Implication \(P \rightarrow Q\)            | Function type `(P) -> Q`                        |
| Conjunction \(P \wedge Q\)                 | Product type `{ a: P, b: Q }`                   |
| Disjunction \(P \vee Q\)                   | Sum type `{ a(P) \| b(Q) }`                     |
| Universal quantifier \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                     |
| True \(\top\)                              | `Void` (Unit, has default value)                |
| False \(\bot\)                             | `Never` (zero constructors, uninhabited)        |
| Type universe \(Type_n : Type_{n+1}\)      | Universe hierarchy (prevents Russell's paradox) |
| Case analysis                              | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions plus compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (e.g., case analysis + recursive calls for `Add` on `Nat`) are essentially the type-level
  encoding of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proven.

### 0.3 Impact on Language Design

Concrete embodiments of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive calls on the natural number
   type `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination
   checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
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

> **Design note**: Although RFC-010 proposes the unified "everything is assignment" model
> (`name: type = value`), at the syntax level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`); `TypeExpr` is a BNF placeholder corresponding to the `Type` enum in the
> implementation, meaning "a type is expected at this position".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical counterpart | Description                                                                               | Default size |
| -------- | ------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                   | Meta type                                                                                 | 0 bytes      |
| `Never`  | ⊥ (False/empty)     | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (True/Unit)       | Has a default void value, zero-field product type. `x: Void = <default>` is legal.        | 0 bytes      |
| `Bool`   | —                   | Boolean value: `true` / `false`                                                           | 1 byte       |
| `Int`    | —                   | Signed integer                                                                            | 8 bytes      |
| `Float`  | —                   | Floating-point number                                                                     | 8 bytes      |
| `String` | —                   | UTF-8 string                                                                              | variable     |
| `Char`   | —                   | Unicode character                                                                         | 4 bytes      |
| `Bytes`  | —                   | Raw bytes                                                                                 | variable     |

Width-suffixed integers: `Int8`, `Int16`, `Int32`, `Int64`; width-suffixed floats: `Float32`,
`Float64`. The complete built-in type name list is in `src/frontend/core/types/mono.rs:618-640`
under `MonoType::from_builtin_name`.

> **No unsigned integer types**. `Uint` (as well as `Int128`) is not a usable type name—`Uint` only
> appears in the LSP completion candidate list at `src/lsp/world.rs:176` and the `sizeof` fallback
> branch in `src/frontend/core/types/eval/const_eval.rs:503`; neither is a type registration. When
> you need unsigned semantics, use `Int` with explicit lower/upper bound conventions.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to False (⊥) and True
(⊤) respectively.

**Never (⊥, False/empty type)** — three non-negotiable properties:

1. **Zero constructors**: no literal or expression can produce a value of type `Never`. There is no
   right-hand side to write for `x: Never = ...`.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which the code can pass type checking (though it is never actually reached).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch merging.

`Never` is a built-in type name (same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, True/Unit)** — has exactly one inhabitant (the default void value). `Void` is the unit of
the zero-field product type. `x: Void = <default>` is legal. A block's value is given by its **tail
expression** (the empty block `{}` is `Void`); see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for details.

---

## Chapter 3: Composite Types

### 3.1 Record Type

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

// Record type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Rules**:

- Record types are defined using curly braces `{}`
- Field name is followed directly by colon and type
- Interface names written inside the type body indicate that the interface is implemented

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that a function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call
> syntax (e.g., `p.draw()`) work, an explicit binding is required: `Point.draw = draw[0]`. See
> RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values; the constructor call can optionally provide them:

```yaoxiang
// Field with a default value - optional in construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without a default value - required in construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // OK
Point2()          // Error
```

**Rules**:

- `field: Type = expression` -> has default value, optional in construction
- `field: Type` -> no default value, required in construction

#### 3.1.2 Built-in Bindings

Methods can be bound directly inside the type definition body:

```yaoxiang
// Method 1: reference an external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bind to position 0
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

### 3.2 Interface Type

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
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // implements Drawable interface
    Serializable     // implements Serializable interface
}
```

**Direct interface assignment**: A concrete type can be directly assigned to a variable of an
interface type (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: calls circle_draw directly, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Look up the method through the vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                             | Inferred result            | Call mechanism              |
| ------------------------------------ | -------------------------- | --------------------------- |
| Direct assignment of a concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value                | Unknown                    | vtable                      |
| Heterogeneous collection             | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, closure note)**: YaoXiang's interfaces are structural
types (interface = a record whose fields are all function types), not nominal traits—there is no
cross-crate/module "who can implement for whom" ownership problem; Rust-style orphan rules and
coherence checks have no subject of application (see the ruling in RFC-011 §2.1). The corresponding
guarantee in the structural world is **rejection of duplicate implementations**: defining the same
method signature twice on a type causes a compile error (RFC-011a §3, overriding is forbidden;
overloading is allowed).

### 3.4 Tuple Type

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 Function Type

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= Param (',' Param)*
Param       ::= Identifier ':' TypeExpr    // named parameter
              | TypeExpr                   // unnamed parameter (bare type)
```

**Two parameter forms, with different ownership of the name** (the type side of RFC-007 shorthand
rules):

- **Named** (`a: Int`): the name enters the contract—it can be referenced by named arguments
  (`f(a = 3)`, see [syntax §2.3](syntax.md#23-函数调用)), and when the signature has already
  declared parameter names and types the lambda header can be omitted
  (`add: (a: Int, b: Int) -> Int = a + b`, RFC-007 §Shorthand Rules).
- **Unnamed** (bare type, e.g., `(Int, Int) -> Int`): the type constraint is enforced **by
  position**; the name belongs to the implementation—the lambda header must supply its own parameter
  names (`mk: (Int, Int) -> Int = (x, y) => x + y`, RFC-007: the lambda header can be omitted if and
  only if the signature has already declared parameter names); cannot be referenced by named
  arguments, only positional calls. A bare identifier is resolved in the type namespace; if no
  declared type is found, an error is reported—there is no bare identifier that is "neither a type
  nor a type parameter" at a parameter position.

**Binding semantics of annotations**: annotations are declarations, not comments. Binding statements
check the implementation against the annotation—parameter types drive the lambda header by position
(if the implementation supplies its own explicit types, they must match the annotation), and the
function body's exits (tail expression and `return`) are uniformly checked against the annotation's
return type; the type registered with the caller from the binding is exactly the annotation form.
Bindings without annotations use HM inference (RFC-007). `x: Int = "hello"` and
`f: () -> Int = () => "hello"` are rejected by the same rule—binding semantics do not fork based on
the value's written form (lambda / block / expression).

> **Implementation status**: Binding-time checking for non-`Fn` annotations is already in effect;
> binding-time checking for `Fn` annotations and type resolution for bare identifiers are landed
> with [RFC-039](../../rfc/accepted/039-compiler-architecture.md) D53/D54 (P6/P8).

---

## Chapter 4: Generics

### 4.1 Generic Parameter Syntax

Generic parameters are part of the function type and use the same `()` syntax as ordinary
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` is the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they are treated the same as
user-defined generics and go through the unified generic instantiation path. Where the length
information lives is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                             | Foundation                               |
| ------------- | ------------- | ------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)  | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Runtime-length raw buffer, growable   | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list) | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value map                         | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list`, treated the same as a user-defined generic record. All growable-semantics
> strategies (when to grow, by how much, whether sharing is allowed) live in the library; the
> compiler is not involved. `Vec(T)` is the minimal foundation primitive it depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When the need arises,
> complete it following the Dict pattern.

Key rules:

- **Literal destination is determined by context**: a bare `[...]` literal paired with a `List(T)`
  annotation lands as a growable list; an `Array(T, N)` annotation acting directly on a literal
  lands as a fixed-length array. Destination check: element count == N, element type compatible with
  T; otherwise compile-time E1002; when N is a symbolic constant (const parameter), the count check
  is deferred to the refinement-type phase.
- **Implicit List→Array conversion is forbidden**: fixed-length is guaranteed at the type
  level—`push` only accepts a `List(A)` receiver.
- **Performance hierarchy**: from bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index-failure contract** (runtime errors are transitional; the target state is compile-time
  refinement, via value-dependent types, see §8.4):
  - Out-of-bounds index (including negative index) → `E6003`
  - Missing key in Dict → `E6008`
- **Membership `in` predicate**: returns `Bool` without erroring; the right operand covers
  List/Array/Dict(key)/Tuple/String/Range. A first-class Hoare predicate, the foundation of
  propositions provable at compile time in refinement types.`

In generic functions, the type parameters are likewise declared in the signature, and the compiler
automatically infers them from the actual arguments:

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 Generic Type Definition

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
    push: (self: List(T), item: T) -> Void,   // self is just a convention name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation—the
receiver type must implement the `Try` interface (four members `is_failure` / `success` / `residual`
/ `from_error`); on failure the function returns early with `from_error(residual(t))`; on success
the expression's value is `success(t)`. The outer function's return type must likewise implement
`Try`, and its failure residual type must match the receiver's (`E1081` / `E1082` / `E1083`).
`Result(T, E)` and `Option(T)` are supplied with `Try` implementations by `std.result` /
`std.option` (the failure residual type of `Option` is `Void`); user-defined sum types can hook into
`?` by writing `Try(Self, T, E)` in the type body. The lowering of `?` does not distinguish between
built-in and user types—same interface, same path.

The **variant set of a sum type enters the checker via its definition or `use` import**:
`Result`/`Option` must be `use std.result` / `use std.option` before use (whole-module and grouped
`use std.{...}` are equivalent); variant construction (type-qualified form), match-variant
deconstruction, and exhaustiveness checks all use this registration as their sole basis; native
function signatures that return `Result(Float, Error)` mint a `Generic` of the same name, which is
the same identity as the sum type in `std.result` by name and can be match-deconstructed after
`use`.

### 4.3 Generic Construction Call and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each field
corresponds to a constructor parameter, and the field name is the parameter name; fields with
default values can be omitted at construction time, fields without default values are required.
Function-type fields (methods) do not generate constructor parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default -> constructor parameter required
    extra: T,
}
// The auto-expanded full form (compiler's internal view, users do not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: the auto-generated constructor
c  = Container(42, 43)            // fill constructor parameters by field order; T unpacked from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type parameter + positional constructor parameters
c4 = Container(Int)(extra=43, value=42)  // field-name form, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default values -> constructor parameters can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, matching declared parameters position by position, left to
right):

1. Try to match each actual argument against the declared parameter type by position: a `Type`
   position accepts a type argument; a compile-time value position (e.g., `Int`) accepts a
   compile-time constant.
2. If some compile-time value position matches successfully (partial match), treat as type
   construction: check every parameter position; on error, **report the first mismatching/missing
   parameter** in declaration order.
3. If the actual arguments do not correspond to the declared parameters at all (all values, no
   compile-time value position matched), treat as constructor parameters: fill by field order
   positionally; type parameters are unpacked automatically from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one-level type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two levels: type + constructor parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ pos 0: T←42 mismatch (42 is not a type); pos 1: Rows←42 matches;
              //    pos 2: Cols missing -> report first error first: T expected Type, found 42
Container(42) // ❌ missing constructor parameter extra
Container(42, 43, 44)  // ❌ too many constructor parameters
```

**Type inference**: type parameters of a generic type constructor are unpacked automatically from
constructor parameter elements (`Container(42, 43)` → T=Int); type parameters of a generic function
are unpacked automatically from actual argument types (`map(numbers, f)` → T=Int, R=String, see
§4.1). When unpacking is impossible, they must be supplied explicitly.

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

> **Constraint resolution source (RFC-011b)**: the resolution of operator constraint names (`Add` /
> `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) = look up the interface
> implementation registry—`T: Add` ≜ the instantiation `Add(T, T, T)` has been registered; `Equal`
> additionally has structural derivation (a record whose fields are all comparable is automatically
> comparable). Names like `Zero` / `One` / `PartialOrd` have no defined source yet and are dangling
> constraint names.

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
// Iterator trait (using record-type syntax)
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

## Chapter 7: Compile-time Generics

### 7.1 Compile-time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // compile-time constant (candidate)
```

> The criterion is **being referenced in a type position**, not "being annotated with a concrete
> type": in `add: (a: Int, b: Int) -> Int = a + b`, `a` and `b` are runtime value parameters
> (neither appears in any type position).

**Terminology**: a generic parameter annotated with a concrete type other than `Type` (e.g., `Int`)
is called a **compile-time value parameter candidate**; whether it becomes a compile-time value
parameter depends on whether its value is referenced in a type position (value-dependence). **No
`const` keyword is needed** (the implementation used to call these "const generics" internally; the
documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Shape pre-screening**: the parameter is annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use refined screening**: the candidate name appears in a **type position** (field type in a
   type body, inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type-construction
   argument position) → true compile-time value parameter; otherwise **runtime value parameter**.

| Form                                                       | Determination                             | Reason                                             |
| ---------------------------------------------------------- | ----------------------------------------- | -------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear in value positions                     |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N appears in a type-construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N acts as the type of inner parameter k            |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in the type body               |

**Core design**: use a compile-time value parameter `(N: Int)` plus a value parameter `(k: N)` to
distinguish compile-time constants from runtime values. A fallen-through candidate (shape is a
candidate, use did not hit) degrades to a runtime value parameter—this is how both the
function-level and the type-constructor paths handle it.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in a type-construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated in a type position (compile time), result 120 embedded in the type
arr: Measure(Int, factorial(5))  // the compiler computes factorial(5) = 120 at compile time

// Value dependence: N as the type of the inner parameter k
// N is a compile-time value parameter (appears in the type position of (k: N));
// k is a runtime value parameter, its type is the literal type N (a single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 Compile-time Constant Arrays

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
// IsTrue bridging and Assert refinement types (see §8.3 for details)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, divergence/compile error
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 Type Families

```yaoxiang
// Compile-time type transformation
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 8.3 Assert Refinement Types and the `assert` Statement

`assert` and `Assert` are two sides of the same refinement primitive—the dispatch pipeline
automatically chooses based on "whether the predicate's free variables are compile-time reachable".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                       |
| ----------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enter the proof pipeline: true → erase to Void, false → compile error (Never is uninhabitable) |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Insert a runtime Bool check; inject refinement facts into the flow-sensitive assumption set Γ  |

**Flow-sensitive assumption set Γ**:

The compiler maintains, at each control-flow point, the set of known propositions:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable is reassigned, all assumptions involving that variable are removed (kill
set). At branch merge, Γ is the intersection of each branch's Γ.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, on par with `Int` and `Never` as core primitives (a
built-in name, not a keyword). It binds a **measure** to a piece of computation, declaring that the
computation terminates and providing a witness for termination.

**Shape**: two arities, one predicate:

| Shape                   | Anchor                 | Purpose                                                                                                                             |
| ----------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The binding's name     | Default form — self-recursive functions, loops                                                                                      |
| `Terminates(FnType, m)` | Explicit function type | When the measure's ownership must be stated explicitly (the measure is defined elsewhere, one measure serves multiple computations) |

```yaoxiang
// Measure: an ordinary function, unit-testable, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Two-arity form: the measure is defined elsewhere, ownership is stated explicitly
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// One-arity form: the anchor is the binding's name, the measure is an expression in scope
// The loop body is a `{}` block; its value is given by the tail expression (spec §2.9): the body is not `Void` only if a tail expression is written
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

**Semantics**: what `Terminates(m)` refines is the value type of the computation annotated at that
type position. The obligation falls on that computation—on every recursive call site for functions,
on the back edge for loops—both stating "the measure of the next state is strictly less than the
measure of the current state", verified under the path guard at that point.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back edge is
`m(next round) < m(current round)`. The two are the same form.

> **Why this covers loops**: loops are anonymous constructs and usually cannot be referred to. The
> binding's name is the name—in `acc: Terminates(n - i) = while ...` the `acc` provides the anchor,
> so the loop becomes referable. This is why the one-arity form of `Terminates` applies to loops.

**Measure**: no restriction on the return type (not required to be a natural number); the "strictly
decreasing" over it is given by whatever well-founded order is available on that type. Whether the
measure is well-founded (e.g., whether it is `>= 0` when it returns `Int`) is an **independent
obligation**, also determined by the compile-time proof pipeline, on equal footing with the decrease
obligation.

**Trigger**: termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. Unrefined ordinary types (e.g., a bare `while` loop, a function without a
refined signature) do not enter verification mode and incur no termination obligation.

**Automatic exploration preferred**: the compiler first tries to auto-explore a measure (four
templates: linear rank function, predicate-violation count, bounded increase/decrease,
multiplicative scaling); an explicit `Terminates` is only needed when exploration fails.

**Runtime representation**: a pure compile-time entity, erased along with the witness, not present
in the runtime binary.

> See the full design in [RFC-027 §6.9](../../rfc/accepted/027-compile-time-evaluation-types.md)
> (semantics) and [RFC-027a](../../rfc/review/027a-termination-explicit-measure.md) (landing
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

**Syntax**: a type intersection `A & B` denotes the type that satisfies both A and B

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using the intersection type
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## Chapter 10: Function Overloading and Specialization

### 10.1 Function Overloading

```yaoxiang
// Basic specialization: using function overloading (the compiler selects automatically)
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
// Platform type enum (defined in the standard library)
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P is a predefined generic parameter name, representing the current compile target platform
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 11: Type Properties

YaoXiang has only one type property to distinguish: linear vs. copyable. It is derived automatically
by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership
transfer.

```yaoxiang
// A struct with heap-buffer fields does not derive Dup (the field falls into Move) -> default Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move, b cannot be read again
```

### 11.2 Dup (Shallow Copy: Copy the Handle, Share the Data)

**The Dup property is used for reference/token types and handle value types with internal reference
counting**. Assignment of a Dup type = shallow copy—the handle/token is copied and the underlying
data is shared. Multiple holders point to the same piece of data.

| Type            | Property | Description                                                                             |
| --------------- | -------- | --------------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token; copying the token = multiple views of the same data              |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, shared heap data                                      |
| String, Bytes   | Dup      | Internal reference count; assignment copies the handle and shares the underlying buffer |
| `&mut T`        | Linear   | Zero-sized write token; exclusive, cannot be copied                                     |
| struct          | derived  | All fields are copyable (primitive value types ∪ Dup) → Dup, otherwise Move (#398)      |
| tuple           | derived  | Per-element check, same rules as struct (#398)                                          |
| All other types | Move     | Default ownership transfer                                                              |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: assignment
auto-copies the value, and the two values are completely independent. This is the compiler's native
behavior, not part of the Dup type property.

**Derivation rules** (#398 finalized) — "automatically derive when all fields are Dup" cannot be
executed literally: primitive fields (Int, etc.) are themselves not Dup, so `{ x: Int, y: Int }`
would be misjudged as Move. The executable form:

1. **Copyable-field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), already-Dup composite types);
2. **struct**: all fields are in the copyable-field set → derive Dup; **any** field falls into
   Linear (`&mut T`) or Move (a nested Move struct / Vec, Dict and other containers / resources) →
   the whole type remains Move (no "partially copyable" intermediate state);
3. **tuple**: same rules as struct, per-element check; the empty tuple (unit) is `Void`;
4. **Derivation is recursive**: when a field is a named type (e.g., `target: Point`), expand its
   definition and check again; `A = { b: B }` follows B's derived result; cyclic aliases are
   conservatively placed in Move by a depth limit;
5. **Not in derivation scope** (remain Move, separate case): containers (Vec / Dict / Set / Option /
   Result / Array) and enum.

```yaoxiang
// &T: Dup, free to alias
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // OK
print(view2.x)   // OK

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is the explicit deep-copy interface. All types can implement Clone, providing a `.clone()`
method.

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

|                         | Dup                                                         | Clone                                            |
| ----------------------- | ----------------------------------------------------------- | ------------------------------------------------ |
| **Semantics**           | Shallow copy: copy the handle/token, underlying data shared | Deep copy: create a complete independent replica |
| **Call style**          | Implicit (automatic on assignment/passing)                  | Explicit (`.clone()`)                            |
| **Modification impact** | Affect each other (shared underlying data)                  | Independent (separate copies)                    |
| **Applicable types**    | `&T` tokens, `ref T`                                        | Any type that implements the Clone interface     |
| **Cost**                | Zero overhead (tokens are zero-sized types)                 | Depends on the type                              |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // OK
print(view2.x)      // OK, seeing the same data

// Primitive value type: compiler auto-copies the value (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // OK

// Clone: explicit deep copy, create an independent replica
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable

// Non-Dup type (fields fall into Move, not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf has a Vec field, not derived to Dup (see §11.2 derivation rules)
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views of the same data"
- Clone is for scenarios requiring an independent replica; the explicit call makes the cost visible
- Copying of primitive value types (Int/Float/Bool/Char) is a compiler built-in behavior, not part
  of Dup
- Custom types default to Move (zero-copy, high performance); when all fields are copyable, Dup is
  auto-derived (§11.2 derivation rules)

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references" but
"type-level proofs of access rights".

```
&T      →  zero-sized, freezes the source data (forbids WriteToken from being acquired meanwhile),
          under the freeze guarantee, multiple read-only views are safe → Dup (copyable)
&mut T  →  zero-sized, exclusive read/write (forbids any other token),
          under exclusive access, copying is meaningless → Linear (not Dup)
```

**Key properties**:

- Tokens are **ordinary types**, following the same scoping rules as any other type
- No lifetime annotations like `'a` are needed
- No dedicated borrow checker is needed—the type property (Dup/Linear) naturally derives the
  permission
- Completely disappears after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// On the method side: declare the parameter type, which determines the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// On the call side: the compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // the compiler automatically creates a &Point token
p.shift(1.0, 1.0)               // the compiler automatically creates a &mut Point token
p.print()                       // OK, the previous token was released when the shift call ended

// Multiple &T tokens coexisting — Dup types allow free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all the operations of ordinary types:

**Returning tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ Child token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to the caller
print(px_ref)                    // OK, the token is still in scope
```

**Stored in a struct**—a struct can carry token fields:

```yaoxiang
// ✅ A struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field — holds a read-only view of target
}
```

**Closures do not capture; context is fixed at the creation point**—a closure only takes its own
parameters; when outer data is needed, the value is fixed into the closure at the creation point via
currying:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter; gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: after a closure (function value) escapes, the scope at its definition site may already be
> dead, so it must not implicitly capture outer variables; however, the scope at the call site
> (creation point) is guaranteed to be alive, and fixing the context into the closure as a value at
> that point is safe.

### 12.4 Automatic Borrow Selection

The call-side compiler automatically chooses by the following priority:

```
1. If the actual argument is used afterward -> prefer to create a token (&T or &mut T, according to the method signature)
2. If the actual argument is not used afterward -> Move
3. Match-priority order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point -> the compiler creates a &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point -> the compiler creates a &mut Point token
p2 = p             // no later use -> Move
```

**Method receiver follows the signature semantics** (same as the receiver-spelling convention in
RFC-011a): the receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value
→ Move (consume the receiver). The borrow token created at the call site is released when the call
ends (transient, see §12.5 interval semantics); an interface's borrow receiver is explicitly
declared as `&Self` by the interface author, and the impl signature, after `Self ↦ impl-type`
substitution, must match the interface exactly (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions (`borrow_conflict`
/ `use_after_move` / `use_after_drop` / `mut_violation`) and feeds them into the proof pipeline for
verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a §Reverse-BFS
Liveness Analysis):

```yaoxiang
// ❌ &mut and the derived &T cannot be live at the same time
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of the WriteToken
    print(p.y)
}

// ✅ Token is automatically released after its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // use the &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken is still available
}

// ❌ The same actual argument cannot create both an &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives both an &mut and an & token simultaneously
```

### 12.6 Compiler Internals: Brand Mechanism

Users never see brands. Internally, the compiler assigns a compile-time unique identifier to each
token:

```
User sees             Compiler-internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

The uses of the brand:

- **Anti-forgery**: a token can only be obtained from its owner capsule, not constructed out of thin
  air
- **Derivation tracking**: a derived `&Float` from a field access carries a derived brand
  (`#N.field_x`), and the compiler can trace it back to the parent token
- **Conflict detection**: same-source WriteToken and derived ReadToken cannot be live at the same
  time

The brand completely disappears after monomorphization and inlining; nothing of it exists in the
generated machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes the source data -> Dup-safe)
               | &mut T      // WriteToken (exclusive read/write -> Linear)
```

### 12.8 Borrow Token vs `ref`

|                 | `&T` / `&mut T`                                           | `ref`                                      |
| --------------- | --------------------------------------------------------- | ------------------------------------------ |
| What it does    | Peek / modify in place                                    | Shared ownership                           |
| Scope           | Follows the token value's scope                           | Crosses scopes                             |
| Cost            | Zero overhead (zero-sized type, erased after compilation) | Rc or Arc (compiler chooses)               |
| Escape          | Allowed (tokens propagate with return values / structs)   | Designed to escape                         |
| Cross-task      | Not allowed (tokens do not support cross-task passing)    | Allowed (compiler picks Arc automatically) |
| Cycle detection | N/A                                                       | Silent within a task, lint across tasks    |

> Note (undefined): how to read the contents after creating a `ref` (dereference / method /
> automatic) has not yet been defined in the specification; in the current implementation, `*a`
> reports E1052. This section will be completed once it is defined.

---

## Appendix: Type Definition Cheat Sheet

### A.1 Type Definitions

```
// === Record type (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Interface type (curly braces, fields all functions) ===

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

// One-arity: anchor is the binding's name (self-recursive functions, loops)
// The body tail must yield a value — the value of `while` = value of the loop body block = tail expression (spec §2.9)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}

// Two-arity: state the measure's ownership explicitly (the measure is defined elsewhere)
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

### A.3 Type Property Cheat Sheet

```
// === Move (default) ===
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // auto value-copy on assignment, two values completely independent
Bool, Char      // not Dup; this is the compiler's built-in handling of primitives

// === Dup (shallow copy: copy the handle, share the underlying data) ===
&T              // zero-sized read token, copy the token = multiple views of the same data
ref T           // Rc/Arc copy = reference count +1, shared heap data
String, Bytes   // internal reference count, copy the handle and share the underlying buffer
struct / tuple  // derived: all fields ∈ (primitive value types ∪ Dup) → Dup (#398)

// === Not derived (remain Move) ===
Vec/Dict/Set    // containers and enum are not in derivation scope (#398)

// === Linear ===
&mut T          // zero-sized write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent replica; modifications do not affect the original
```

### A.4 Borrow Token Cheat Sheet

```
// === Borrow tokens ===
&T              // zero-sized compile-time read token, freezes the source data -> Dup (copyable)
&mut T          // zero-sized compile-time write token, exclusive read/write -> Linear (not copyable)

// Automatic selection on the call side
// 1. The actual argument is used afterward -> create a token
// 2. The actual argument is not used afterward -> Move
// 3. Priority match: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in a struct, captured by a closure
// ❌ Cannot cross tasks (tokens do not support cross-task passing)
```
