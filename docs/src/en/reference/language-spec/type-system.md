# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and trait.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between a programming language's type system and mathematical logic:

| Logic                                          | Programming Language                                 |
| ---------------------------------------------- | ---------------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                          |
| Proof \(p: P\)                                 | Program `x: T = ...`                                 |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                             |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                        |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                          |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                          |
| Truth \(\top\)                                 | `Void` (Unit, with a default value)                  |
| Falsehood \(\bot\)                             | `Never` (zero constructor, no inhabitant)            |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                                   |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as case analysis + recursive calls of `Add` on `Nat`) are essentially a type-level
  encoding of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, the equivalent is a
  constructive proof of a logical proposition.

### 0.3 Impact on Language Design

Concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox
   (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): type-level case analysis + recursive calls of the natural number
   `Nat(Zero/Succ)` correspond to the Peano axioms—provided the compiler performs termination
   checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
   quantification of "for each integer N, there exists a type"

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

> **Design Note**: Although RFC-010 proposes the unified "everything is assignment" model
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`); `TypeExpr`, as a BNF placeholder, corresponds to the `Type` enum in the
> implementation, meaning "a type is expected at this position".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                              | Default Size |
| -------- | ---------------------- | ---------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                | 0 bytes      |
| `Never`  | ⊥ (false/empty)        | Zero constructor, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)          | Has a default void value, zero-field product type. `x: Void = <default>` is legal.       | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                          | 1 byte       |
| `Int`    | —                      | Signed integer                                                                           | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                         | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                    | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                             | Variable     |
| `Char`   | —                      | Unicode character                                                                        | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                | Variable     |

Integers with explicit bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Floats with explicit
bit width: `Float32`, `Float64`.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding respectively to
falsehood (⊥) and truth (⊤).

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructor**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing writable on the right.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which subsequent code passes type checking (though it will never be executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered with the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity element of the zero-field product type. `x: Void = <default>` is legal. The value of a
block is given by its **tail expression** (empty block `{}` is `Void`); see
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
- A field name is directly followed by a colon and a type
- An interface name written inside the type body indicates implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call
> syntax `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and
> RFC-010 for details.

#### 3.1.1 Field Default Values

A type field may specify a default value, which is optional at construction time:

```yaoxiang
// Field with default value - optional at construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without default value - required at construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // correct
Point2()          // error
```

**Rules**:

- `field: Type = expression` -> has default value, optional at construction
- `field: Type` -> no default value, required at construction

#### 3.1.2 Built-in Binding

Methods can be bound directly inside a type definition body:

```yaoxiang
// Method 1: Reference an external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bound to position 0
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

**Direct interface assignment**: A concrete type can be assigned directly to an interface-typed
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategies**:

| Scenario                        | Inference Result           | Call Method                 |
| ------------------------------- | -------------------------- | --------------------------- |
| Direct concrete-type assignment | Concrete type determinable | Direct call (zero overhead) |
| Function return value           | Unknown                    | vtable                      |
| Heterogeneous collection        | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, conclusion statement)**: YaoXiang's interfaces are
structural types (interface = record whose fields are all function types), not nominal traits—there
is no "who can implement for whom" ownership issue across crates/modules; Rust-style orphan rules
and coherence checks have no applicable target (see RFC-011 §2.1 for the ruling). The corresponding
guarantee in the structural world is **duplicate implementation rejection**: defining the same
method signature twice on a type is a compile-time error (RFC-011a §3, override prohibited;
overloading is legal).

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

Generic parameters are part of the function type, sharing the unified `()` syntax with regular
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

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics and go through the unified generic instantiation path. The
placement of length information is the fundamental difference among the three container concepts:

| Type          | Length        | Semantics                             | Foundation                               |
| ------------- | ------------- | ------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type-level    | Fixed-length array (const generic N)  | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Runtime-length raw buffer, growable   | Core primitive (heap contiguous buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list) | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                     | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: defined by YaoXiang itself in
> `std.list`, receiving the same treatment as user-defined generic records. All growable semantics
> strategies (when to expand, how much, whether sharing is allowed) live in the library; the
> compiler does not participate. `Vec(T)` is the minimal underlying primitive on which it depends.
>
> `Set(T)` has been removed: no literal, no runtime representation, no `std.set`. When a need
> arises, it will be completed following the `Dict` pattern.

Key rules:

- **Literal landing point is determined by context**: the bare literal `[...]` together with a
  `List(T)` annotation lands in the growable list; an `Array(T, N)` annotation acting directly on a
  literal lands in the fixed-length array. Landing-point check: number of elements == N, element
  type compatible with T; mismatch is a compile-time E1002; when N is a symbolic constant (const
  parameter), the count check is deferred to the refined-type phase.
- **Implicit List→Array conversion is forbidden**: fixed-length is guaranteed at the type
  level—`push` only accepts a `List(A)` receiver.
- **Performance hierarchy**: from bottom to top, performance decreases while flexibility increases:
  `Array` > `Vec` > `List`.
- **Index failure contract** (runtime error is a transitional state; the target state is covered by
  compile-time refinement via value-dependent types, see §8.4):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: returns `Bool` without error; the right operand covers
  List/Array/Dict(key)/Tuple/String/Range. A first-class Hoare predicate, the substrate of
  propositions provable at compile time via refined types.`

In generic functions, the type parameters are likewise declared in the signature, and the compiler
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
receiver type must implement the `Try` interface (four members `is_failure` / `success` / `residual`
/ `from_error`); on failure, the function returns early with `from_error(residual(t))`; on success,
the expression value is `success(t)`. The outer function's return type must likewise implement
`Try`, and its failure residual type must match that of the receiver (`E1081` / `E1082` / `E1083`).
`Result(T, E)` and `Option(T)` provide `Try` implementations via `std.result` / `std.option` (the
failure residual type of `Option` is `Void`); user-defined sum types can hook into `?` by writing
`Try(Self, T, E)` in the type body. The lowering of `?` does not distinguish between built-in and
user types—same interface, same path.

The **variant set of a sum type enters the checker with its definition or via `use` import**:
`Result`/`Option` must be imported with `use std.result` / `use std.option` before use (whole-module
and grouped `use std.{...}` are equivalent); variant construction (type-qualified form), `match`
variant deconstruction, and exhaustiveness checks all use this registration as the sole criterion; a
`Result(Float, Error)` returned from a native function signature casts out a same-named `Generic`,
which by name shares identity with the sum type in `std.result`, and can be `match`-deconstructed
after `use`.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a construction parameter, the field name is the parameter name; fields with
default values may be omitted at construction, fields without default values are required.
Function-typed fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default -> construction parameter required
    extra: T,
}
// The automatically expanded full form (compiler-internal view, users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Calls: invoking the auto-generated constructor
c  = Container(42, 43)            // construction parameters filled in field order; T auto-unwrapped from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type argument + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // field-name form, order is free
c5 = Container(Int)()             // empty construction: fields take defaults/zero values (data assigned later)

// Field default value -> construction parameter may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, match declared parameters position by position, left to right):

1. Each actual argument attempts to match a declared parameter position: a `Type` position accepts a
   type argument, a compile-time value parameter position (e.g., `Int`) accepts a compile-time
   constant.
2. If some compile-time value parameter position is matched (partial match), treat as type
   construction: check all parameter positions; on error, report the **first mismatched/missing
   parameter in declaration order** first.
3. If actual arguments do not correspond to any declared parameter (all are values, no compile-time
   value parameter position can be matched), treat as construction parameter handling: positional
   filling in field order, type arguments auto-unwrapped from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: a one-layer type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two layers: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 mismatched (42 is not a type); position 1: Rows←42 matched;
              //    position 2: Cols missing -> report first error: T expected Type, found 42
Container(42) // ❌ missing construction parameter `extra`
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: Type parameters of a generic type constructor are auto-unwrapped from the
construction parameter elements (`Container(42, 43)` → T=Int); type parameters of a generic function
are auto-unwrapped from actual argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When
unwrapping fails, type arguments must be provided explicitly.

---

## Chapter 5: Type Constraint

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

### 5.2 Multiple Constraint

> **Constraint resolution source (RFC-011b)**: Operator constraint names (`Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) resolve via lookup in the interface
> implementation registry—`T: Add` ≜ an instantiated `Add(T, T, T)` is registered; `Equal` has an
> additional structural derivation (all-field-comparable records are auto-comparable). Names like
> `Zero` / `One` / `PartialOrd` have no defined source yet and are dangling constraint names.

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

## Chapter 6: Associated Type

### 6.1 Associated Type Definition

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (uses record type syntax)
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

### 6.2 Generic Associated Type (GAT)

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

> The basis for judgment is **being referenced in a type position**, not "annotated with a concrete
> type": in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters (neither
> appears in any type position).

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (such as
`Int`) is called a **compile-time value parameter candidate**; whether it becomes a compile-time
value parameter depends on whether its value is referenced in a type position (value dependency).
**No `const` keyword is needed** (the implementation once used "const generic" internally; the
documentation uniformly uses "compile-time value parameter").

**Determination rule (two steps)**:

1. **Form coarse filter**: A parameter annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use fine filter**: The candidate name appears in a **type position** (a field type in a type
   body, an inner `Fn` parameter type, an `Assert` predicate, an `Array(T, N)` type construction
   argument position) → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing Style                                              | Judgment                                  | Reason                                           |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters              | only appear in value positions                   |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter            | N appears in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter            | N is the type of inner parameter k               |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N not referenced in type body                    |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Candidates that fall through (form is a
candidate, use does not match) degrade to runtime value parameters—both function-level and
type-constructor paths follow this treatment.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at the type position (compile time), result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // compiler computes factorial(5) = 120 at compile time

// Value dependency: N as the type of inner parameter k
// N is a compile-time value parameter (appears in the type position of (k: N));
// k is a runtime value parameter, its type is the literal type N (single-value type).
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

## Chapter 8: Conditional Type

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
// IsTrue bridging and Assert refined types (see §8.3 for details)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, divergence/compile error
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 Type Family

```yaoxiang
// Compile-time type conversion
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 8.3 Assert Refined Type and assert Statement

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch pipeline based on "whether the free variables of the predicate are reachable at compile
time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                 |
| ------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never uninhabitable) |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ  |

**Flow-sensitive assumption set Γ**:

The compiler maintains the set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After assigning a `mut` variable, all assumptions involving that variable are removed (kill set). At
branch confluence, Γ takes the intersection of each branch.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, belonging to the same core primitives as `Int` and `Never`
(built-in name, not a keyword). It binds a **measure** to a piece of computation, declaring that the
computation terminates and providing a witness of termination.

**Forms**: two arities, the same predicate:

| Form                    | Anchor                        | Use                                                                                                                            |
| ----------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `Terminates(m)`         | name of the enclosing binding | Default form—self-recursive functions, loops                                                                                   |
| `Terminates(FnType, m)` | explicit function type        | When the measure's ownership needs to be made explicit (measure defined elsewhere, same measure serving multiple computations) |

```yaoxiang
// Measure: ordinary function, unit-testable, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Two-argument form: measure defined elsewhere, explicitly stating ownership
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// One-argument form: anchor is the binding name, measure is an in-scope expression
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines exactly the value type of the computation annotated at its
type position. The obligation falls on that computation—for a function, on each recursive call site;
for a loop, on the back edge; both are "the measure of the next state is strictly less than the
measure of the current state", decided under the path guard at that point.

The obligation at a call site is `m(callee_args) < m(caller_args)`; the obligation at a back edge is
`m(next iteration) < m(current iteration)`. Both are formally identical.

> **Why it covers loops**: A loop is an anonymous construct, usually not denotable. The binding name
> is the name—in `acc: Terminates(n - i) = while ...`, `acc` provides the anchor, so the loop
> becomes denotable. This is the reason the one-argument form of `Terminates` acts on loops.

**Measure**: no restriction on the return type (not forced to be a natural number); the available
well-order on that type supplies the "strictly decreasing" relation. Whether the measure is
well-founded (e.g., whether it returns `Int` and satisfies `>= 0`) is an **independent obligation**,
decided by the compile-time proof pipeline just like the decreasing obligation.

**Trigger**: termination checking is triggered by **refined types**—once a type is refined, it
enters verification mode. Types that are not refined (such as bare `while` loops, functions without
refinement signatures) do not enter verification mode and generate no termination obligations.

**Automatic exploration priority**: The compiler first automatically explores measures (four
templates: linear rank function, predicate-violation counter, bounded increase/decrease,
multiplicative scaling); only when exploration fails is an explicit `Terminates` required.

**Runtime representation**: a pure compile-time entity, erased along with the witness, not present
in the runtime binary.

> See [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md) (landing mechanism) for
> the full design.

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

**Syntax**: A type intersection `A & B` denotes the type that satisfies both A and B

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

YaoXiang has only one type attribute that needs to be distinguished: linear vs. copyable. It is
automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types default to Move semantics. Assignment, parameter passing, return = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy the Handle, Share the Data)

**The Dup attribute is used for reference/token types**. Assignment of a Dup type = shallow
copy—copy the handle/token, the underlying data is shared. Multiple holders point to the same block
of data.

| Type            | Attribute | Description                                                                |
| --------------- | --------- | -------------------------------------------------------------------------- |
| `&T`            | Dup       | Zero-sized read token, copying the token = multiple views on the same data |
| `ref T`         | Dup       | Rc/Arc copy = reference count +1, shared heap data                         |
| `&mut T`        | Linear    | Zero-sized write token, exclusive, cannot be copied                        |
| All other types | Move      | Default ownership transfer                                                 |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: assignment
triggers automatic value copy, with the two values being completely independent. This is a native
compiler behavior and is not part of the Dup type attribute.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy the token, both remain valid
print(view.x)    // usable
print(view2.x)   // usable

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship to Dup

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

**Differences between Dup and Clone**:

|                         | Dup                                                         | Clone                                         |
| ----------------------- | ----------------------------------------------------------- | --------------------------------------------- |
| **Semantics**           | Shallow copy: copy the handle/token, underlying data shared | Deep copy: create a complete independent copy |
| **Invocation**          | Implicit (automatic on assignment/parameter passing)        | Explicit (`.clone()`)                         |
| **Modification effect** | Mutually affected (shared underlying data)                  | Mutually unaffected (independent copies)      |
| **Applicable types**    | `&T` tokens, `ref T`                                        | Any type implementing the Clone interface     |
| **Cost**                | Zero overhead (tokens are zero-sized)                       | Depends on the type                           |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // usable
print(view2.x)      // usable, viewing the same data

// Primitive value type: compiler automatic value copy (not Dup)
x: Int = 42
y = x               // value copy, x and y completely independent
print(x)            // usable

// Clone: explicit deep copy, create an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor a primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views on the same data"
- Clone is used in scenarios that need an independent copy; explicit invocation makes the cost
  visible
- The copying of primitive value types (Int/Float/Bool/Char) is built-in compiler behavior, not part
  of Dup
- Most user-defined types default to Move, achieving zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references", but
"type-level proof of access permission".

```
&T      →  zero-sized, freezes the source data (prohibits obtaining a WriteToken in the meantime),
          under the freezing guarantee multiple read-only accesses are safe -> Dup (copyable)
&mut T  →  zero-sized, exclusive read/write (prohibits any other token),
          exclusive access makes copying meaningless -> Linear (not Dup)
```

**Key properties**:

- A token is a **regular type**, following the same scoping rules as all other types
- No lifetime annotation `'a` is needed
- No dedicated borrow checker is required—type attributes (Dup/Linear) naturally infer permissions
- Completely disappears after compilation, with zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declares parameter types, determines required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call side: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // compiler automatically creates a &Point token
p.shift(1.0, 1.0)               // compiler automatically creates a &mut Point token
p.print()                       // OK, the previous token has been released along with the end of the shift call

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are regular types, so they support all operations on regular types:

**Returning tokens**—tokens propagate together with the return value:

```yaoxiang
// ✅ Sub-token and parent token are returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to the caller
print(px_ref)                    // OK, token is still in scope
```

**Stored in structs**—structs may carry token fields:

```yaoxiang
// ✅ Struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field—holds a read-only view of target
}
```

**Closures do not capture, context is frozen at the creation point**—a closure only consumes its own
parameters; when external data is needed, the value is frozen into the closure at the creation point
via currying:

```yaoxiang
// ✅ Context frozen via currying: threshold is a parameter; gt_point(threshold) freezes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition site may already be
> dead, so it must not implicitly capture outer variables; however, the call site (creation point)
> scope is necessarily alive, so freezing the context into the closure as a value at that point is
> safe.

### 12.4 Automatic Borrow Selection

The call-side compiler automatically selects according to the following priority:

```
1. If the actual argument is used afterwards -> prefer creating a token (&T or &mut T, according to method signature)
2. If the actual argument is not used afterwards -> Move
3. Priority order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point -> compiler creates a &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point -> compiler creates a &mut Point token
p2 = p             // not used afterwards -> Move
```

**Method receiver follows signature semantics** (same as the receiver spelling convention of
RFC-011a): receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value →
Move (consumes the receiver). The borrow token produced at the call site is released when the call
ends (transient, see §12.5 interval semantics); an interface's borrow receiver is explicitly
declared by the interface author as `&Self`; the impl signature, after `Self ↦ impl type`
substitution, must exactly match the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a §
reverse BFS liveness analysis):

```yaoxiang
// ❌ &mut and the derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ Token is released automatically after the scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // uses &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken is still usable
}

// ❌ The same actual argument cannot simultaneously create an &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives an &mut token and an & token
```

### 12.6 Compiler Internals: Brand Mechanism

The user never encounters brands. Internally the compiler assigns a compile-time unique identifier
to each token:

```
What the user sees        Compiler's internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: A token can only be obtained from the owner capsule, not constructed out of thin
  air
- **Association tracking**: A `&Float` derived from field access carries the derived brand
  (`#N.field_x`); the compiler can track it back to the parent token
- **Conflict detection**: A WriteToken and a derived ReadToken of the same origin cannot be live
  simultaneously

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data -> Dup safe)
               | &mut T      // WriteToken (exclusive read/write -> Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                               | `ref`                                   |
| --------------- | ------------------------------------------------------------- | --------------------------------------- |
| What it does    | Glance at / modify in place                                   | Shared ownership                        |
| Range           | With the token value's scope                                  | Across scopes                           |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler chooses)            |
| Escape          | Yes (token propagates with return value/struct)               | Designed to escape                      |
| Cross-task      | No (tokens do not implement cross-task transfer)              | Yes (compiler auto-selects Arc)         |
| Cycle detection | Not involved                                                  | Silent within a task, lint across tasks |

> Note (undefined): how to read content after `ref` is created (dereference/method/auto) has not yet
> been defined in the spec; the current implementation reports E1052 for `*a`. This section will be
> completed once defined.

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

// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // implements Serializable interface
}

// === Function types ===

Adder: Type = (Int, Int) -> Int

// === Termination measures (built-in predicate, see §8.4) ===

// One-argument: anchor is the binding name (self-recursive functions, loops)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// Two-argument: explicitly state measure ownership (measure defined elsewhere)
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
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // assignment triggers automatic value copy, two values completely independent
Bool, Char      // not Dup, but the compiler's built-in handling of primitives

// === Dup (shallow copy: copy the handle, share the underlying data) ===
&T              // zero-sized read token, copy token = multiple views on the same data
ref T           // Rc/Arc copy = reference count +1, shared heap data

// === Linear ===
&mut T          // zero-sized write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent copy, modification does not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow token ===
&T              // zero-sized compile-time read token, freezes source data -> Dup (copyable)
&mut T          // zero-sized compile-time write token, exclusive read/write -> Linear (not copyable)

// Call-side automatic selection
// 1. Actual argument used afterwards -> create a token
// 2. Actual argument not used afterwards -> Move
// 3. Priority: &T < &mut T < Move

// Token propagation
// ✅ Returnable, storable in structs, capturable by closures
// ❌ Not cross-task (tokens do not implement cross-task transfer)
```
