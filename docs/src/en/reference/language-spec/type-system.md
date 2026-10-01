# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between programming language type systems and mathematical logic:

| Logic                                          | Programming Language                         |
| ---------------------------------------------- | -------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                  |
| Proof \(p: P\)                                 | Program `x: T = ...`                         |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                     |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                  |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                  |
| True \(\top\)                                  | `Void` (Unit, with a default value)          |
| False \(\bot\)                                 | `Never` (zero constructors, uninhabited)     |
| Type universe \(Type_n : Type_{n+1}\)          | Universe levels (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                           |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as `Add` with case analysis + recursive calls on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proven.

### 0.3 Impact on Language Design

Concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe levels** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive calls on natural numbers
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to bounded
   quantification "for each integer N there exists a type"

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

> **Design Note**: Although RFC-010 proposes a unified model where "everything is assignment"
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`); `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the
> implementation, indicating "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                                                  | Default Size |
| -------- | ---------------------- | ------------------------------------------------------------------------------------------------------------ | ------------ |
| `Type`   | —                      | Meta type                                                                                                    | 0 bytes      |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no value of this type exists. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)          | Has a default void value, zero-field product type. `x: Void = <default>` is legal.                           | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                                              | 1 byte       |
| `Int`    | —                      | Signed integer                                                                                               | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                                             | 8 bytes      |
| `Float`  | —                      | Floating point number                                                                                        | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                                                 | Variable     |
| `Char`   | —                      | Unicode character                                                                                            | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                                    | Variable     |

Integers with bit widths: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Floats with bit widths:
`Float32`, `Float64`.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤) respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which the code can pass type checking (though it will never be executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (with the same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — exactly one inhabitant (the default void value). `Void` is the identity
element of the zero-field product type. `x: Void = <default>` is legal. The value of a block is
given by its **tail expression** (empty block `{}` is `Void`), see
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) for details.

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
- Field name is directly followed by a colon and a type
- An interface name written in the type body indicates implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call
> syntax such as `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See
> RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, optional at construction time:

```yaoxiang
// Fields with default values - optional at construction time
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Fields without default values - required at construction time
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // correct
Point2()          // error
```

**Rules**:

- `field: Type = expression` -> has default value, optional at construction time
- `field: Type` -> no default value, required at construction time

#### 3.1.2 Builtin Bindings

Methods can be bound directly within a type definition body:

```yaoxiang
// Method 1: Reference external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bind to position 0
}
// Call: p1.distance(p2) -> distance(p1, p2)

// Method 2: Anonymous function + position binding
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
// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // implements Drawable interface
    Serializable     // implements Serializable interface
}
```

**Direct assignment to interface**: A concrete type can be directly assigned to an interface type
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: directly calls circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup via vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                           | Inference Result           | Call Method                 |
| ---------------------------------- | -------------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value              | Unknown                    | vtable                      |
| Heterogeneous collection           | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, closing statement)**: YaoXiang's interfaces are
structural types (interface = record whose fields are all function types), not nominal traits—there
is no "who can implement for whom" ownership issue across crates/modules, and Rust-style orphan
rules and coherence checks have no applicable object (see ruling in RFC-011 §2.1). The corresponding
guarantee in the structural world is **duplicate implementation rejection**: repeated definition of
the same method signature on a type is a compile error (RFC-011a §3, no overriding; overloading is
legal).

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

Generic parameters are part of the function type and use the unified `()` syntax with regular
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` represents the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics, processed through the unified generic instantiation path. The
assignment of length information is the fundamental distinction between the three container
concepts:

| Type          | Length        | Semantics                                | Foundation                               |
| ------------- | ------------- | ---------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)     | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Raw buffer with runtime length, growable | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list)    | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                        | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list`, receiving the same treatment as user-defined generic records. All the
> strategies for growable semantics (when to expand, by how much, whether sharing is allowed) live
> in the library; the compiler does not participate. `Vec(T)` is the minimal underlying primitive it
> depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When the need arises,
> follow the Dict pattern to complete it.

Key rules:

- **Literal destination is determined by context**: A bare `[...]` literal combined with a `List(T)`
  annotation lands as a growable list; an `Array(T, N)` annotation directly applied to a literal
  lands as a fixed-length array. Destination check: number of elements == N, element type compatible
  with T; mismatch is compile-time E1002; when N is a symbolic constant (const parameter), the count
  check is deferred to the refined-type phase.
- **No implicit List→Array conversion**: Fixed-length-ness is guaranteed at the type level—push only
  accepts a `List(A)` receiver.
- **Performance hierarchy**: From bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index failure contract** (runtime error is a transitional state, target state is compile-time
  refinement coverage, via value-dependent types, see §8.4):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: returns `Bool` and does not error; the right operand covers
  List/Array/Dict(key)/Tuple/String/Range. First-class Hoare predicate, the foundation of
  propositions provable at compile time via refined types.`

In generic functions, type parameters are also declared in the signature, and the compiler
automatically infers them from the actual arguments:

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

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation—the
receiver type must implement the `Try` interface (four members: `is_failure` / `success` /
`residual` / `from_error`). On failure, it returns early from the current function with the result
of `from_error(residual(t))`; on success, the expression's value is `success(t)`. The outer
function's return type must also implement `Try`, and its failure residual type must be consistent
with the receiver (`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` are provided with
`Try` implementations by `std.result` / `std.option` (the failure residual type of `Option` is
`Void`); user-defined sum types can be hooked into `?` by writing `Try(Self, T, E)` in the type
body. The lowering of `?` makes no distinction between built-in and user types—same interface, same
path.

The **variant set of a sum type enters the checker via the definition or `use` import**:
`Result`/`Option` require `use std.result` / `use std.option` before use (full-module and grouped
`use std.{...}` are equivalent); variant construction (in type-qualified form), match variant
deconstruction, and exhaustiveness determination all use this registration as the sole criterion.
The `Result(Float, Error)` cast from a native function signature produces a same-named `Generic`,
which shares identity with the sum type in `std.result` by name and can be match-deconstructed after
`use`.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each field
corresponds to a construction parameter, the field name is the parameter name; fields with default
values can be omitted at construction time, fields without default values are required.
Function-typed fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> construction parameter required
    extra: T,
}
// Automatically expanded full form (compiler's internal view, users are not required to write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: call the automatically generated constructor
c  = Container(42, 43)            // Construction parameters filled by field order; T is auto-unwrapped from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type argument + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // Field-name style, any order
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned later)

// Field default values -> construction parameters can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, position-by-position matching against declared parameters, left
to right):

1. Try to match actual arguments position-by-position against declared type parameters: a `Type`
   position accepts a type argument, a compile-time value parameter position (such as `Int`) accepts
   a compile-time constant.
2. If a compile-time value parameter position match succeeds (partial match), treat as type
   construction: check all parameter positions, when erroring, **report the first mismatched/missing
   parameter in declaration order**.
3. If the actual arguments do not correspond to declared parameters at all (all are values, no
   compile-time value parameter position matches), treat as construction parameters: positional
   style filled by field order, type arguments auto-unwrapped from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one layer of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two layers: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 does not match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols missing -> first error reported: T expected Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: Type parameters of a generic type constructor are auto-unwrapped from the
elements of the construction parameters (`Container(42, 43)` → T=Int); type parameters of generic
functions are auto-unwrapped from the actual argument types (`map(numbers, f)` → T=Int, R=String,
see §4.1). When unwrapping is impossible, they must be explicitly filled.

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

> **Resolution source for constraints (RFC-011b)**: The resolution of operator constraint names
> (`Add` / `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) = look up the
> interface implementation registry—`T: Add` ≜ registered `Add(T, T, T)` instantiation; `Equal`
> additionally has structural inference (records whose fields are all comparable are automatically
> comparable). Names like `Zero` / `One` / `PartialOrd` do not yet have a defined source, and are
> dangling constraint names.

```yaoxiang
// Multiple constraint syntax
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
// More complex associated type
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

**Terminology**: A generic parameter annotated with a specific non-`Type` type (such as `Int`) is
called a **compile-time value parameter candidate**; whether it becomes a compile-time value
parameter depends on whether its value is referenced in a type position (value-dependent). **No
`const` keyword needed** (the implementation internally once used "const generics" to refer to this;
the documentation uniformly uses "compile-time value parameters").

**Judgment rules (two steps)**:

1. **Shape coarse filtering**: Parameters annotated with a specific non-`Type` type
   (`Int`/`Bool`/`Float`) → candidate.
2. **Usage fine filtering**: The candidate name appears in a **type position** (type body field
   type, inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction argument
   position) → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing Style                                              | Judgment                                  | Reason                                           |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear in value positions                   |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is compile-time value parameter         | N appears in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is compile-time value parameter         | N serves as type of inner parameter k            |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N not referenced in type body                    |

**Core design**: Use `(N: Int)` compile-time value parameters + `(k: N)` value parameters to
distinguish compile-time constants from runtime values. Fallthrough candidates (shape is candidate,
usage not hit) degrade to runtime value parameters—this applies to both function-level and
type-constructor paths.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) evaluates in type position (compile time), result 120 embedded in type
arr: Measure(Int, factorial(5))  // Compiler computes factorial(5) = 120 at compile time

// Value-dependent: N serves as type of inner parameter k
// N is compile-time value parameter (appears in the type position of (k: N));
// k is runtime value parameter, its type is the literal type N (single-value type).
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
// IsTrue bridge and Assert refined type (see §8.3 for details)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, diverge/compile error
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

### 8.3 Assert Refined Types and assert Statements

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch pipeline based on "whether the predicate's free variables are reachable at compile time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                               |
| ------------------------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased as Void, false → compile error (Never uninhabited) |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refined facts into flow-sensitive assumption set Γ   |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (kill set).
When branches merge, Γ takes the intersection of each branch.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, belonging to the same core primitives as `Int` and `Never`
(built-in name, not a keyword). It binds a **measure** to a piece of computation, declares that the
computation terminates, and provides a witness of termination.

**Form**: Two arities, same predicate:

| Form                    | Anchor                            | Purpose                                                                                                                                 |
| ----------------------- | --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The name of the enclosing binding | Default form—self-recursive functions, loops                                                                                            |
| `Terminates(FnType, m)` | Explicit function type            | When the measure's attribution needs to be explicitly indicated (measure defined elsewhere, same measure serving multiple computations) |

```yaoxiang
// Measure: an ordinary function, unit-testable, reusable, does not participate at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Two-arity form: measure defined elsewhere, explicitly indicate attribution
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// One-arity form: anchor is the binding name, measure is an expression in scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines exactly the value type of the computation annotated at its
type position. The obligation falls on that computation—the function falls on each recursive call
site, the loop falls on the back edge—both being "the next state's measure is strictly smaller than
the current state's measure", judged under the path guard at that point.

The call-site obligation is `m(callee_args) < m(caller_args)`; the back-edge obligation is
`m(next iteration) < m(this iteration)`. Both are formally identical.

> **Why loops are covered**: Loops are anonymous constructs, usually not referable. The binding name
> is the name—`acc: Terminates(n - i) = while ...` provides the anchor via `acc`, so the loop
> becomes referable. This is the reason the one-arity form of `Terminates` applies to loops.

**Measure**: No restriction on return type (not forced to natural numbers); the "strictly
decreasing" on it is given by a well-defined order available on that type. Whether the measure is
well-founded (e.g., when returning `Int` whether it is `>= 0`) is an **independent obligation**,
also determined by the compile-time proof pipeline, same as the decreasing obligation.

**Trigger**: Termination checking is triggered by **refined types**—once a type is refined, it
enters verification mode. Unrefined ordinary types (such as bare `while` loops, functions without
refined signatures) do not enter verification mode and generate no termination obligation.

**Automatic exploration first**: The compiler first automatically explores measures (four templates:
linear rank function, predicate violation counting, bounded increase/decrease, multiplicative
scaling); only when exploration fails is an explicit `Terminates` required.

**Runtime representation**: Pure compile-time entity, erased along with witnesses, not present in
the runtime binary.

> See [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md) (implementation
> mechanism) for the complete design.

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

**Syntax**: Type intersection `A & B` represents the type that satisfies both A and B

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
// Basic specialization: use function overloading (compiler selects automatically)
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
// Platform type enum (defined in standard library)
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

## Chapter 11: Type Properties

YaoXiang has only one type property that needs to be distinguished: linear vs. copyable. It is
automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership
transfer.

```yaoxiang
// Structs containing heap buffer fields do not derive Dup (fields fall to Move) -> default Move
Buf: Type = { data: Vec(Float) }
b: Buf = Buf([1.0, 2.0])
q = b           // Move, b can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types and handle value types with internal reference
counting**. Assignment of a Dup type = shallow copy—copy the handle/token, the underlying data is
shared. Multiple holders point to the same block of data.

| Type            | Property | Description                                                                        |
| --------------- | -------- | ---------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token, copy token = multiple views point to the same data          |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, share heap data                                  |
| String, Bytes   | Dup      | Internal reference count, copy handle shares underlying buffer                     |
| `&mut T`        | Linear   | Zero-sized write token, exclusive, cannot be copied                                |
| struct          | Derived  | All fields are copyable (primitive value types ∪ Dup) → Dup, otherwise Move (#398) |
| tuple           | Derived  | Element-by-element judgment, same as struct rule (#398)                            |
| All other types | Move     | Default ownership transfer                                                         |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: value copy is
automatic on assignment, the two values are completely independent. This is the compiler's native
behavior, not belonging to the Dup type property.

**Derivation rules** (#398 final) — "automatically derive when all fields are Dup" cannot be
literally executed: primitive fields (Int, etc.) themselves do not belong to Dup,
`{ x: Int, y: Int }` would be misjudged as Move. Executable form:

1. **Copyable field set** = primitive value types (Int / Float / Bool / Char / Range) ∪ Dup (`&T`,
   `ref T`, String / Bytes, function values (#352), already-Dup composite types);
2. **struct**: all fields are within the copyable field set → derive Dup; **any** field falls to
   Linear (`&mut T`) or Move (nested Move struct / Vec, Dict and other containers / resources) → the
   whole remains Move (no "partially copyable" intermediate state introduced);
3. **tuple**: same rule as struct, element-by-element judgment; the empty tuple (unit) is `Void`;
4. **Derivation is recursive**: when a field is a named type (e.g., `target: Point`), expand its
   definition before judgment, `A = { b: B }` follows B's derivation result; circular aliases
   conservatively fall to Move according to a depth limit;
5. **Not in derivation range** (still Move, separate case): containers (Vec / Dict / Set / Option /
   Result / Array) and enum.

```yaoxiang
// &T: Dup, free aliasing
view: &Point = &p
view2 = view     // Dup: copy token, both are valid
print(view.x)    // usable
print(view2.x)   // usable

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship to Dup

**Clone** is the explicit deep copy interface. All types can implement Clone, providing a `.clone()`
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

**The difference between Dup and Clone**:

|                         | Dup                                                     | Clone                                         |
| ----------------------- | ------------------------------------------------------- | --------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create complete independent copy   |
| **Call method**         | Implicit (automatic on assignment/parameter passing)    | Explicit (`.clone()`)                         |
| **Modification effect** | Affect each other (share underlying data)               | Do not affect each other (independent copies) |
| **Applicable types**    | `&T` tokens, `ref T`                                    | Any type implementing the Clone interface     |
| **Cost**                | Zero overhead (tokens are zero-sized types)             | Depends on the type                           |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy token, both point to the same p
print(view.x)       // usable
print(view2.x)      // usable, seeing the same data

// Primitive value type: compiler automatically value-copies (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // usable

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable

// Non-Dup type (fields fall to Move, not derived): Move transfers ownership
buf: Buf = Buf([1.0, 2.0])
buf2 = buf          // Move: Buf contains Vec field, does not derive Dup (see §11.2 derivation rules)
```

**Design intent**:

- Dup is used for token/reference types, solving the "multiple views of the same data" problem
- Clone is used for scenarios requiring independent copies; explicit calls make the cost visible
- The copying of primitive value types (Int/Float/Bool/Char) is the compiler's built-in behavior,
  not belonging to Dup
- Custom types default to Move (zero-copy high performance); Dup is automatically derived when all
  fields are copyable (§11.2 derivation rules)

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references", but
"type-level proof of access permission".

```
&T      →  zero-sized, freezes source data (prohibits WriteToken acquisition during this period),
          under freezing guarantee multiple read-only are safe -> Dup (copyable)
&mut T  →  zero-sized, exclusive read-write (prohibits any other token),
          under exclusive access copying is meaningless -> Linear (non-Dup)
```

**Key features**:

- Tokens are **ordinary types**, following the same scope rules as all other types
- No lifetime annotation `'a` needed
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally infer permissions
- Completely disappear after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare parameter type, determining the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call side: compiler automatically chooses to borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates &Point token
p.shift(1.0, 1.0)               // Compiler automatically creates &mut Point token
p.print()                       // OK, the previous token was released when shift call ended

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all ordinary type operations:

**Returning tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to caller
print(px_ref)                    // OK, token is still in scope
```

**Storing in structs**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,              // token field—holds a read-only view of target
}
```

**Closures do not capture; context is fixed at the creation point**—closures only consume their own
parameters; when external data is needed, the value is fixed into the closure at the creation point
via currying:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter, gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, its definition's scope may be dead, so it must not
> implicitly capture outer variables; but the call site (creation point) scope is necessarily alive,
> and it is safe for the context to be fixed as a value into the closure at that point.

### 12.4 Automatic Borrow Selection

The compiler automatically selects at the call site according to the following priority:

```
1. If the actual argument is used later -> prefer to create a token (&T or &mut T, based on the method signature)
2. If the actual argument is not used later -> Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point -> compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point -> compiler creates &mut Point token
p2 = p             // not used later -> Move
```

**Method receiver follows signature semantics** (same as RFC-011a receiver spelling convention):
receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value → Move (consume
receiver). The borrow token generated at the call site is released when the call ends (transient,
§12.5 interval semantics); interface borrow receivers are explicitly declared `&Self` by the
interface author, and the impl signature after `Self ↦ impl type` substitution must be completely
consistent with the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is **borrow Hoare propositions** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be alive simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ Token is automatically released after its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // use &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken still available
}

// ❌ The same actual argument cannot simultaneously create &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Brand Mechanism

Users never encounter brands. The compiler internally assigns a compile-time unique identifier to
each token:

```
User sees               Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Association tracking**: `&Float` derived from field access carries the derived brand
  (`#N.field_x`), the compiler can trace it back to the parent token
- **Conflict detection**: A WriteToken and its derived ReadToken from the same source cannot be
  alive simultaneously

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data -> Dup safe)
               | &mut T      // WriteToken (exclusive read-write -> Linear)
```

### 12.8 Borrow Tokens vs. ref

|                 | `&T` / `&mut T`                                               | `ref`                                        |
| --------------- | ------------------------------------------------------------- | -------------------------------------------- |
| What it does    | Glance at / modify in place                                   | Shared ownership                             |
| Range           | With the scope of the token value                             | Across scopes                                |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler chooses)                 |
| Escape          | Allowed (token propagates with return value/struct)           | Designed to escape                           |
| Cross-task      | Not allowed (tokens do not support cross-task passing)        | Allowed (compiler automatically chooses Arc) |
| Cycle detection | Not involved                                                  | Silent within task, lint across tasks        |

> Note (undefined): After `ref` is created, how to read its contents (dereference/method/auto) is
> not yet defined in the specification; the current implementation reports `*a` as E1052. To be
> added to this section after definition.

---

## Appendix: Type Definition Quick Reference

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

// Type implementing the interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // implements Serializable interface
}

// === Function types ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (built-in predicate, see §8.4) ===

// One-arity: anchor is the binding name (self-recursive functions, loops)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// Two-arity: explicitly indicate measure attribution (measure defined elsewhere)
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

// Type constraints
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated type
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

### A.3 Type Properties Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // Automatic value copy on assignment, the two values are completely independent
Bool, Char      // Not Dup, but the compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-sized read token, copy token = multiple views point to the same data
ref T           // Rc/Arc copy = reference count +1, share heap data
String, Bytes   // Internal reference count, copy handle shares underlying buffer
struct / tuple  // Derived: all fields ∈ (primitive value types ∪ Dup) -> Dup (#398)

// === Not derived (stays Move) ===
Vec/Dict/Set    // Containers and enum not in derivation range (#398)

// === Linear ===
&mut T          // Zero-sized write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // Create independent copy, modification does not affect original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token, freezes source data -> Dup (copyable)
&mut T          // Zero-sized compile-time write token, exclusive read-write -> Linear (not copyable)

// Call-side automatic selection
// 1. Actual argument is used later -> create token
// 2. Actual argument is not used later -> Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in struct, captured by closure
// ❌ Cannot cross tasks (tokens do not support cross-task passing)
```
