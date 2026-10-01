# Type System Specification

This document defines the type system specification for the YaoXiang programming language, including
primitive types, composite types, generics, and trait.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between a programming language's type system and mathematical logic:

| Logic                                          | Programming Language                            |
| ---------------------------------------------- | ----------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                     |
| Proof \(p: P\)                                 | Program `x: T = ...`                            |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                        |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                   |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                     |
| Universal quantification \(\forall x:T. P(x)\) | generics `(T: Type) -> ...`                     |
| True \(\top\)                                  | `Void` (Unit, has a default value)              |
| False \(\bot\)                                 | `Never` (zero constructors, uninhabited)        |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires a
> type-level recursive function plus a compiler termination check.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as `Add` doing case analysis + recursive calls on `Nat`) are essentially the
  type-level encoding of mathematical induction—provided the compiler can perform termination
  checks.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proved.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type family** (RFC-011): The type-level case analysis + recursive calls of natural number
   `Nat(Zero/Succ)` corresponds to the Peano axioms—provided the compiler performs termination
   checks
3. **Conditional type** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent type** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
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

> **Design note**: Although RFC-010 proposes a unified model of "everything is assignment"
> (`name: type = value`), at the syntactic level, types and values still need to be distinguished.
> In the compiler implementation, `Type` and `Expr` are two independent AST enumerations
> (`ast.rs:406` and `ast.rs:25`), and `TypeExpr` serves as a BNF placeholder corresponding to the
> `Type` enumeration in the implementation, indicating "a type is expected at this position".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                               | Default Size |
| -------- | ---------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta-type                                                                                 | 0 bytes      |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)          | Has a default void value, zero-field product type. `x: Void = <default>` is legal.        | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                           | 1 byte       |
| `Int`    | —                      | Signed integer                                                                            | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                          | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                     | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                              | variable     |
| `Char`   | —                      | Unicode character                                                                         | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                 | variable     |

Integers with bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`; Floats with bit width:
`Float32`, `Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤), respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which code can pass type checking (although it will never execute).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — exactly one inhabitant (the default void value). `Void` is the unit of the
zero-field product type. `x: Void = <default>` is legal. The value of a block is given by the **tail
expression** (an empty block `{}` is `Void`); see
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) for details.

---

## Chapter 3: Composite Types

### 3.1 Record Type

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

- Record types are defined using curly braces `{}`
- A field name is directly followed by a colon and a type
- An interface name written inside the type body indicates that the interface is implemented

> **Namespace ownership**: The `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to the namespace of `Point`. It does not trigger any implicit binding. To make the `.`
> call syntax like `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See
> RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, and are optional during construction:

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
Point2(x=1, y=2) // correct
Point2()          // error
```

**Rules**:

- `field: Type = expression` -> has a default value, optional during construction
- `field: Type` -> no default value, required during construction

#### 3.1.2 Built-in Bindings

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
// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // implements Drawable interface
    Serializable     // implements Serializable interface
}
```

**Direct interface assignment**: A concrete type can be directly assigned to an interface-typed
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Look up method through vtable

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
structural types (an interface is a record whose fields are all function types), not nominal
traits—there is no "who can implement for whom" ownership issue across crates/modules, and the
Rust-style orphan rules and coherence checks have no applicable subject (see the ruling in RFC-011
§2.1). The corresponding guarantee in the structural world is **duplicate implementation
rejection**: repeated definitions of the same method signature on a type cause a compile error
(RFC-011a §3, override forbidden; overloading is legal).

### 3.4 Tuple Type

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 Function Type

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

---

## Chapter 4: Generics

### 4.1 Generic Parameter Syntax

Generic parameters are part of the function type, and use the unified `()` syntax with regular
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In generic type definitions, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` indicates the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics, processed via the unified generic instantiation path. The
ownership of length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                              | Foundation                                        |
| ------------- | ------------- | -------------------------------------- | ------------------------------------------------- |
| `Array(T, N)` | Type-level    | Fixed-length array (const generic N)   | Core primitive (stack/inline preferred)           |
| `Vec(T)`      | Runtime value | Raw buffer of runtime length, growable | Core primitive (heap-allocated contiguous buffer) |
| `List(T)`     | Runtime value | Standard library type (growable list)  | Library: `{ data: Vec(T), length: Int }`          |
| `Dict(K, V)`  | Runtime value | Key-value mapping                      | `HeapValue::Dict`                                 |

> `List(T)` is a **standard library type, not a compiler primitive**: it is defined in `std.list` by
> YaoXiang itself, and receives the same treatment as user-defined generic records. All growable
> semantics (when to expand, by how much, whether sharing is possible) live in the library; the
> compiler does not participate. `Vec(T)` is the minimal underlying primitive it depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When the need arises,
> fill in according to the Dict pattern.

Key rules:

- **Literal landing point is determined by context**: A bare `[...]` literal and the `List(T)`
  annotation land in a growable list; the `Array(T, N)` annotation directly applied to a literal
  lands in a fixed-length array. Landing-point check: number of elements == N, element types
  compatible with T, otherwise compile-time E1002; when N is a symbolic constant (const parameter),
  the count check is deferred to the refined-type phase.
- **No implicit List→Array conversion**: Fixed-length nature is guaranteed at the type level—push
  only accepts a `List(A)` receiver.
- **Performance hierarchy**: From bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index failure contract** (runtime errors are a transitional state, target state is compile-time
  refinement coverage, via value-dependent types, see §8.4):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: returns `Bool` without error, the right operand covers
  List/Array/Dict(keys)/Tuple/String/Range. A first-class Hoare predicate, the foundation of
  propositions provable at compile time via refined types.`

In generic functions, the type parameters are also declared in the signature, and the compiler
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
/ `from_error`). On failure, the function returns early from the current function with the result of
`from_error(residual(t))`; on success, the expression value is `success(t)`. The return type of the
outer function must also implement `Try`, and its failure residual type must match the receiver's
(`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` have `Try` implementations provided by
`std.result` / `std.option` (the failure residual type of `Option` is `Void`); user-defined sum
types can hook into `?` by writing `Try(Self, T, E)` in the type body. The lowering of `?` does not
distinguish between built-in and user types—the same interface, the same path.

The **variant set of a sum type enters the checker through the definition or `use` import**:
`Result`/`Option` must be preceded by `use std.result` / `use std.option` (full-module and grouped
`use std.{...}` are equivalent), and variant construction (type-qualified form), match variant
deconstruction, and exhaustiveness checks all use this registration as the sole criterion. The
`Result(Float, Error)` cast from a native function signature produces a `Generic` of the same name,
which has the same identity as the sum type in `std.result` by name, and can be match-deconstructed
after `use`.

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a construction parameter, the field name is the parameter name; fields with
default values can be omitted during construction, fields without default values are required.
Function-typed fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> construction parameter required
    extra: T,
}
// Automatically expanded full form (compiler internal view, user is not required to write it by hand):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: call the auto-generated constructor
c  = Container(42, 43)            // construction parameters filled in field order; T auto-unwrapped from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type parameter + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // field name form, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default values -> construction parameters can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, matching by declared parameters in order, left to right):

1. Actual arguments try to match declared parameters position by position: the `Type` position
   accepts type arguments, compile-time value parameter positions (e.g. `Int`) accept compile-time
   constants.
2. If some compile-time value parameter positions match successfully (partial match), proceed as
   type construction: check all parameter positions, and when reporting an error, **report the first
   mismatched/missing parameter in declaration order**.
3. If the actual arguments do not match the declared parameters at all (all are values, no
   compile-time value parameter position matches), proceed as construction parameters: fill in field
   order positionally, type parameters are auto-unwrapped from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one-level type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two levels: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ pos 0: T←42 mismatched (42 is not a type); pos 1: Rows←42 matched;
              //    pos 2: Cols missing → first error reported: T expects Type, got 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: The type parameters of a generic type constructor are auto-unwrapped from
construction parameter elements (`Container(42, 43)` → T=Int); the type parameters of a generic
function are auto-unwrapped from actual argument types (`map(numbers, f)` → T=Int, R=String, see
§4.1). When unwrapping is not possible, they must be filled in explicitly.

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

> **Source of constraint resolution (RFC-011b)**: The resolution of operator constraint names (`Add`
> / `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) = looking up the interface
> implementation registry—`T: Add` ≜ an `Add(T, T, T)` instantiation has been registered; `Equal`
> additionally has structural inference (records whose all fields are comparable are automatically
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

### 5.3 Function Type Constraint

```yaoxiang
// Higher-order function constraint
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
// Iterator trait (using record type syntax)
Iterator: (T: Type) -> Type = {
    Item: T,                    // associated type
    next: () -> Option(T),
    has_next: () -> Bool
}

// Using the associated type
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

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // compile-time constant (candidate)
```

> The criterion is **being referenced at a type position**, not "being annotated with a concrete
> type": in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters (they do
> not appear in any type position).

**Terminology**: A generic parameter annotated with a non-`Type` concrete type (such as `Int`) is
called a **compile-time value parameter candidate**; whether it becomes a compile-time value
parameter depends on whether its value is referenced at a type position (value dependency). **No
`const` keyword needed** (the implementation internally used the term "const generics", but the
documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Form coarse filter**: The parameter is annotated with a non-`Type` concrete type
   (`Int`/`Bool`/`Float`) → candidate.
2. **Usage fine filter**: The candidate name appears at a **type position** (type body field type,
   inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction argument position)
   → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing Style                                              | Determination                             | Reason                                           |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters              | Only appear at value positions                   |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter            | N appears in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter            | N acts as the type of inner parameter k          |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in the type body             |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Candidates that fall through (form is a
candidate, usage does not hit) degrade to runtime value parameters—both the function-level and
type-constructor paths are handled this way.

```yaoxiang
// Compile-time value parameter: N is referenced at a type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at the type position (compile time), result 120 is embedded in the type
arr: Measure(Int, factorial(5))  // compiler computes factorial(5) = 120 at compile time

// Value dependency: N acts as the type of inner parameter k
// N is a compile-time value parameter (appears at the type position of (k: N));
// k is a runtime value parameter, its type is the literal type N (single-value type).
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

// Compile-time dimension verification
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
// IsTrue bridge and Assert refined type (see §8.3 for details)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, diverges/compile error
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

### 8.3 Assert Refined Type and assert Assertion

`assert` and `Assert` are the two sides of the same refinement primitive—automatically selected by
the dispatch routing pipeline based on "whether the free variables of the predicate are reachable at
compile time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch routing rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                |
| ----------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never uninhabited)  |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains the set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After assignment to a `mut` variable, all assumptions involving that variable are removed (kill
set). At branch confluence, Γ takes the intersection of each branch.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, belonging to the core primitives along with `Int`, `Never`
(built-in name, not a keyword). It binds a **measure** to a piece of computation, declaring that the
computation terminates and providing a witness of termination.

**Form**: Two arities, the same predicate:

| Form                    | Anchor                           | Use                                                                                                                                  |
| ----------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `Terminates(m)`         | The name of the binding it is in | Default form—self-recursive functions, loops                                                                                         |
| `Terminates(FnType, m)` | Explicit function type           | When the measure needs to be explicitly attributed (the measure is defined elsewhere, the same measure serves multiple computations) |

```yaoxiang
// Measure: a regular function, unit-testable, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Two-argument form: the measure is defined elsewhere, explicitly attributed
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// One-argument form: the anchor is the binding name, the measure is an expression in scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**Semantics**: What `Terminates(m)` refines is the value type of the computation annotated at its
type position. The obligation falls on that computation—the function falls on every recursive call
site, the loop falls on the back edge—both are "the measure of the next state is strictly less than
the measure of the current state", and are judged under the path guard at that point.

The obligation at the call site is `m(callee_args) < m(caller_args)`; the obligation at the back
edge is `m(next iteration) < m(this iteration)`. Both have the same form.

> **Why loops are covered**: A loop is an anonymous construct and usually cannot be referred to. The
> binding name is the name—`acc: Terminates(n - i) = while ...` provides an anchor via `acc`, so the
> loop becomes referable. This is the reason the one-argument form of `Terminates` works on loops.

**Measure**: The return type is not restricted (not forced to be a natural number); the "strict
decrease" is given by the well-founded order available on that type. Whether the measure is
well-founded (e.g. whether it is `>= 0` when returning `Int`) is an **independent obligation**,
judged by the compile-time proof pipeline just like the decrease obligation.

**Trigger**: Termination checks are triggered by **refined types**—once a type is refined, the
verification mode is entered. Unrefined ordinary types (such as bare `while` loops, functions
without refined signatures) do not enter verification mode and do not generate termination
obligations.

**Automatic exploration first**: The compiler first automatically explores the measure (four
templates: linear rank function, predicate violation count, bounded increase/decrease,
multiplicative scaling); an explicit `Terminates` is needed only when exploration fails.

**Runtime representation**: A pure compile-time entity, erased along with the witness, does not
enter the runtime binary.

> See the full design in
> [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md) (implementation
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

**Syntax**: A type intersection `A & B` represents the type that satisfies both A and B

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
// Basic specialization: using function overloading (compiler automatically selects)
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
// Platform type enumeration (defined in the standard library)
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

YaoXiang has only one type property to distinguish: linear vs. copyable. It is automatically
inferred by the compiler.

### 11.1 Move (default ownership transfer)

All types follow Move semantics by default. Assignment, parameter passing, and return = ownership
transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (shallow copy: copy handle, share data)

**The Dup property is used for reference/token types and handle value types with internal reference
counting**. Assignment of a Dup type = shallow copy—copy the handle/token, the underlying data is
shared. Multiple holders point to the same block of data.

| Type            | Property | Description                                                                            |
| --------------- | -------- | -------------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-size read token, copy token = multiple views point to the same data               |
| `ref T`         | Dup      | Rc/Arc copy = reference count+1, share heap data                                       |
| String, Bytes   | Dup      | Internal reference counting, assignment copies handle and shares the underlying buffer |
| `&mut T`        | Linear   | Zero-size write token, exclusive, cannot be copied                                     |
| All other types | Move     | Default ownership transfer                                                             |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: they are
automatically value-copied on assignment, and the two values are completely independent. This is the
compiler's native behavior, not a Dup type property.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // usable
print(view2.x)   // usable

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

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

**Differences between Dup and Clone**:

|                         | Dup                                                     | Clone                                       |
| ----------------------- | ------------------------------------------------------- | ------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create complete independent copy |
| **Call method**         | Implicit (automatic on assignment/parameter passing)    | Explicit (`.clone()`)                       |
| **Modification effect** | Mutually affect (share underlying data)                 | Mutually independent (independent copies)   |
| **Applicable types**    | `&T` token, `ref T`                                     | Any type implementing Clone interface       |
| **Cost**                | Zero overhead (token is a zero-size type)               | Depends on the type                         |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data is shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // usable
print(view2.x)      // usable, seeing the same data

// Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // usable

// Clone: explicit deep copy, create an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor a primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views of the same data"
- Clone is used in scenarios requiring independent copies, the explicit call makes the cost visible
- The copying of primitive value types (Int/Float/Bool/Char) is a compiler built-in behavior, not
  Dup
- Most user-defined types default to Move, zero-copy and high-performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references", but
"type-level proof of access permission".

```
&T      →  zero-size, freezes source data (forbids WriteToken from being obtained during this period),
          under the freeze guarantee, multiple read-only accesses are safe → Dup (copyable)
&mut T  →  zero-size, exclusive read and write (forbids any other token),
          under exclusive access, copying is meaningless → Linear (not Dup)
```

**Key features**:

- Tokens are **ordinary types**, following the same scope rules as all other types
- No lifetime annotation `'a` needed
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally derive permissions
- Completely disappears after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare parameter types, determine required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call side: compiler automatically selects borrow or Move
p = Point(1.0, 2.0)
p.print()                       // compiler automatically creates &Point token
p.shift(1.0, 1.0)               // compiler automatically creates &mut Point token
p.print()                       // OK, the previous token was released when the shift call ended

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Scope and Propagation of Tokens

Tokens are ordinary types, so they support all operations of ordinary types:

**Returning tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to the caller
print(px_ref)                    // OK, the token is still in scope
```

**Storing in struct**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field—holds a read-only view of target
}
```

**Closures don't capture, context is fixed at creation point**—closures only eat their own
parameters; when external data is needed, the value is fixed into the closure at the creation point
via currying:

```yaoxiang
// ✅ Context is fixed via currying: threshold is a parameter, gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition may be dead, so it
> must not implicitly capture outer variables; but the scope at the call site (creation point) is
> necessarily alive, and it is safe for the context to be fixed as a value into the closure at that
> point.

### 12.4 Automatic Borrow Selection

The call-side compiler automatically selects by the following priority:

```
1. If the actual argument is still used later → prefer to create a token (&T or &mut T, depending on the method signature)
2. If the actual argument is no longer used → Move
3. Priority match order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // not used later → Move
```

**Method receiver follows signature semantics** (same as the receiver spelling convention in
RFC-011a): the receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value
→ Move (consume the receiver). The borrow token generated at the call site is released when the call
ends (transient, §12.5 interval semantics); the interface's borrow receiver is explicitly declared
by the interface author as `&Self`, and the impl signature must be exactly consistent with the
interface after the `Self ↦ impl type` substitution (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and the derived &T cannot be alive at the same time
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ Token is automatically released after the scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // use &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken is still available
}

// ❌ The same actual argument cannot create both &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives &mut and & tokens simultaneously
```

### 12.6 Compiler Internals: Brand Mechanism

Users never come into contact with brands. The compiler internally assigns a unique compile-time
identifier to each token:

```
User sees              Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-counterfeiting**: A token can only be obtained from the owner's capsule, not constructed
  out of thin air
- **Association tracking**: The `&Float` derived from field access carries the derived brand
  (`#N.field_x`), which the compiler can trace back to the parent token
- **Conflict detection**: A WriteToken of the same source and the derived ReadToken cannot be alive
  at the same time

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read and write → Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                              | `ref`                                 |
| --------------- | ------------------------------------------------------------ | ------------------------------------- |
| What it does    | Look / modify in place                                       | Shared ownership                      |
| Scope           | With the scope of the token value                            | Cross-scope                           |
| Cost            | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler selects)          |
| Escape          | Yes (token propagates with return value/struct)              | Is meant to escape                    |
| Cross-task      | No (tokens do not implement cross-task passing)              | Yes (compiler auto-selects Arc)       |
| Cycle detection | Not involved                                                 | Silently within task, cross-task lint |

> Note (undefined): How to read content after ref is created (dereference/method/auto) is not yet
> defined in the specification; in the current implementation, `*a` reports E1052. To be added to
> this section after definition.

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definition

```
// === Record type (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Interface type (curly braces, all fields are functions) ===

// Interface definition
Serializable: Type = { serialize: () -> String }

// Type implementing the interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // implements Serializable interface
}

// === Function type ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (built-in predicate, see §8.4) ===

// One-argument: anchor is the binding name (self-recursive function, loop)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// Two-argument: explicitly attribute the measure (measure is defined elsewhere)
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

// Type constraint
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated type
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Compile-time generic: N is referenced at the type position (k: N) → compile-time value parameter
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
// All types default to Move. Assignment, parameter passing, and return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // automatically value-copied on assignment, the two values are completely independent
Bool, Char      // not Dup, but the compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // zero-size read token, copy token = multiple views point to the same data
ref T           // Rc/Arc copy = reference count+1, share heap data

// === Linear ===
&mut T          // zero-size write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent copy, modifications do not affect the original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow token ===
&T              // zero-size compile-time read token, freezes source data → Dup (copyable)
&mut T          // zero-size compile-time write token, exclusive read and write → Linear (cannot be copied)

// Call-side automatic selection
// 1. Actual argument is still used later → create token
// 2. Actual argument is no longer used → Move
// 3. Priority match: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in structs, captured by closures
// ❌ Cannot cross tasks (tokens do not implement cross-task passing)
```
