# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

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
| True \(\top\)                                  | `Void` (Unit, with default value)            |
| False \(\bot\)                                 | `Never` (zero constructors, no inhabitant)   |
| Type universe \(Type_n : Type_{n+1}\)          | Universe levels (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                           |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as `Add`'s case analysis + recursive calls on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proved.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe levels** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoid the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive calls on natural numbers
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checking
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

> **Design Note**: Although RFC-010 proposes a unified model where "everything is assignment"
> (`name: type = value`), at the syntactic level, types and values still need to be distinguished.
> In the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`); `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the
> implementation, meaning "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logic Correspondence | Description                                                                                   | Default Size |
| -------- | -------------------- | --------------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                    | Meta type                                                                                     | 0 bytes      |
| `Never`  | ⊥ (false/empty type) | Zero constructors, no values. Return type for divergence/panic. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)        | Has a default void value, zero-field product type. `x: Void = <default>` is legal.            | 0 bytes      |
| `Bool`   | —                    | Boolean value: `true` / `false`                                                               | 1 byte       |
| `Int`    | —                    | Signed integer                                                                                | 8 bytes      |
| `Uint`   | —                    | Unsigned integer                                                                              | 8 bytes      |
| `Float`  | —                    | Floating-point number                                                                         | 8 bytes      |
| `String` | —                    | UTF-8 string                                                                                  | variable     |
| `Char`   | —                    | Unicode character                                                                             | 4 bytes      |
| `Bytes`  | —                    | Raw bytes                                                                                     | variable     |

Bit-width integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Bit-width floats: `Float32`,
`Float64`.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤) respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which code passes type checking (though it will never execute).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis and `match` branch merging.

`Never` is a built-in type name (with the same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the unit
element of zero-field product types. `x: Void = <default>` is legal. A block's value is given by the
**tail expression** (empty block `{}` is `Void`); see
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
- Field names are followed directly by a colon and a type
- Interface names written inside the type body indicate implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make `.` call syntax
> like `p.draw()` work, you must explicitly bind: `Point.draw = draw[0]`. See RFC-004 and RFC-010
> for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, optionally provided at construction:

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
Point2(x=1, y=2) // Correct
Point2()          // Error
```

**Rules**:

- `field: Type = expression` -> has default value, optional at construction
- `field: Type` -> no default value, required at construction

#### 3.1.2 Built-in Bindings

Methods can be bound directly inside a type definition body:

```yaoxiang
// Method 1: Reference external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // Bind to position 0
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

**Interface implementation**: A type implements interfaces by listing interface names at the end of
its definition

```yaoxiang
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implement Drawable interface
    Serializable     // Implement Serializable interface
}
```

**Direct interface assignment**: A concrete type can be directly assigned to an interface type
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile-time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: directly calls circle_draw, no vtable

// Function return value (cannot determine at compile-time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method looked up via vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                   | Inference Result           | Call Method                 |
| -------------------------- | -------------------------- | --------------------------- |
| Direct concrete assignment | Concrete type determinable | Direct call (zero overhead) |
| Function return value      | Unknown                    | vtable                      |
| Heterogeneous collection   | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, closing note)**: YaoXiang's interfaces are structural
types (interface = record whose fields are all function types), not nominal traits—there is no "who
can implement for whom" ownership issue across crates/modules, and Rust-style orphan rules and
coherence checks have no applicable object (decision record in RFC-011 §2.1). The corresponding
safeguard in the structural world is **duplicate implementation rejection**: defining the same
method signature multiple times on a type causes a compile error (RFC-011a §3, overriding
prohibited; overloading allowed).

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

Generic parameters are part of the function type, using the unified `()` syntax with regular
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In generic type definitions, `(T: Type)` is the type constructor's parameter signature, and
`-> Type` indicates the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics, processed through the unified generic instantiation path. Where
the length information lives is the fundamental difference between three container concepts:

| Type          | Length        | Semantics                              | Foundation                               |
| ------------- | ------------- | -------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)   | Core primitive (stack/inline first)      |
| `Vec(T)`      | Runtime value | Raw buffer of runtime length, growable | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list)  | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                      | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: defined by YaoXiang itself in
> `std.list`, receiving the same treatment as user-defined generic records. All growable semantics
> strategies (when to grow, by how much, whether to share) live in the library; the compiler does
> not participate. `Vec(T)` is the minimal underlying primitive it depends on.
>
> Set(T) has been removed: no literals, no runtime representation, no std.set. When the need arises,
> it will be completed following the Dict pattern.

Key rules:

- **Literal landing is determined by context**: a bare `[...]` literal with `List(T)` annotation
  lands as a growable list; an `Array(T, N)` annotation acting directly on a literal lands as a
  fixed-length array. Landing verification: element count == N, element types compatible with T,
  otherwise compile-time E1002; when N is a symbolic constant (const parameter), the count check is
  deferred to the refinement type phase.
- **Implicit List→Array conversion is forbidden**: fixed-length property is guaranteed at the type
  layer—push only accepts `List(A)` receivers.
- **Performance hierarchy**: from bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index failure contract** (runtime errors are a transitional state, target state is compile-time
  refinement coverage via value-dependent types, see §8.4):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: returns `Bool` without error; right operand covers List/Array/Dict
  (key)/Tuple/String/Range. First-class Hoare predicate, the foundation for compile-time provable
  propositions in refinement types.

In generic functions, type parameters are likewise declared in the signature, and the compiler
automatically infers them from actual arguments:

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
    push: (self: List(T), item: T) -> Void,   // self is just a convention name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

**`?` propagation and the `Try` interface**: `expr?` is interface-driven error propagation—the
receiver type must implement the `Try` interface (four members: `is_failure` / `success` /
`residual` / `from_error`); on failure, it returns early from the current function with
`from_error(residual(t))`; on success, the expression's value is `success(t)`. The outer function's
return type must also implement `Try`, and its failure residual type must match the receiver's
(`E1081` / `E1082` / `E1083`). `Result(T, E)` and `Option(T)` have `Try` implementations provided by
`std.result` / `std.option` (the failure residual type of `Option` is `Void`); user-defined sum
types plug into `?` by writing `Try(self, T, E)` inside the type body. The lowering of `?` does not
distinguish between built-in and user types—same interface, same path.

### 4.3 Generic Construction Call and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a constructor parameter, the field name is the parameter name; fields with
default values can be omitted at construction, fields without default values are required.
Function-type fields (methods) do not generate constructor parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value -> constructor parameter required
    extra: T,
}
// Automatically expanded full form (compiler internal view, users are not required to write):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: call the automatically generated constructor
c  = Container(42, 43)            // Constructor parameters filled in field order; T auto-unpacked from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type parameter + positional constructor parameters
c4 = Container(Int)(extra=43, value=42)  // Field-name style, order arbitrary
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned later)

// Field default value -> constructor parameter can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, match declared parameters position by position, left to right):

1. Actual arguments try to match declared parameters position by position: the `Type` position
   accepts type arguments, compile-time value parameter positions (e.g., `Int`) accept compile-time
   constants.
2. If a compile-time value parameter position matches successfully (partial match), treat as type
   construction: check all parameter positions in order; on error, report the **first
   mismatching/missing parameter in declared order**.
3. If actual arguments do not correspond to declared parameters at all (all are values, no
   compile-time value parameter positions match), treat as constructor parameters: positional style
   fills in field order, type parameters auto-unpack from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two levels: type + constructor parameters
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ Position 0: T←42 doesn't match (42 is not a type); Position 1: Rows←42 matches;
              //    Position 2: Cols missing → first error reported: T expected Type, found 42
Container(42) // ❌ Missing constructor parameter extra
Container(42, 43, 44)  // ❌ Too many constructor parameters
```

**Type inference**: Type parameters of generic type constructors are auto-unpacked from constructor
parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions are
auto-unpacked from actual argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When
unpacking is impossible, explicit filling is required.

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

> **Resolution source for constraints (RFC-011b)**: Resolution of operator constraint names (`Add` /
> `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) = look up the interface
> implementation registry—`T: Add` ≜ an `Add(T, T, T)` instantiation is registered; `Equal`
> additionally has structural derivation (records with all comparable fields are automatically
> comparable). Names like `Zero` / `One` / `PartialOrd` have no defined source yet, and are dangling
> constraint names.

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

### 6.1 Associated Type Definition

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

### 6.2 Generic Associated Types (GAT)

```yaoxiang
// More complex associated types
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // Associated type is also generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-time Generics

### 7.1 Compile-time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant (candidate)
```

> The determining criterion is **being referenced in a type position**, not "annotated with a
> concrete type": in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters
> (neither appears in any type position).

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (such as
`Int`) is called a **compile-time value parameter candidate**; whether it becomes a compile-time
value parameter depends on whether its value is referenced in a type position (value-dependent).
**No `const` keyword is required** (the implementation internally used "const generics";
documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Form coarse screening**: parameter annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use fine screening**: the candidate name appears in a **type position** (type body field type,
   inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction argument position)
   → true compile-time value parameter; otherwise **runtime value parameter**.

| Syntax                                                     | Determination                             | Reason                                   |
| ---------------------------------------------------------- | ----------------------------------------- | ---------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters              | Appear only in value positions           |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter            | N in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter            | N acts as type of inner parameter k      |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N not referenced in type body            |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Fall-through candidates (form is a
candidate, use does not hit) degrade to runtime value parameters—both function-level and
type-constructor paths follow this.

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) evaluates at type position (compile-time), result 120 embedded in type
arr: Measure(Int, factorial(5))  // Compiler computes factorial(5) = 120 at compile-time

// Value-dependent: N as type of inner parameter k
// N is a compile-time value parameter (appears in the type position of (k: N));
// k is a runtime value parameter, its type is the literal type N (singleton type).
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
// IsTrue bridging and Assert refinement types (see §8.3)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
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

### 8.3 Assert Refinement Types and the assert Statement

`assert` and `Assert` are two faces of the same refinement primitive—automatically chosen by the
dispatch pipeline based on "whether the predicate's free variables are compile-time accessible".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                |
| ------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------- |
| All free variables known at compile-time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erase as Void, false → compile error (Never uninhabitable) |
| Free runtime variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains the set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable assignment, all assumptions involving that variable are removed (kill set).
When branches merge, Γ takes the intersection of each branch.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, belonging to the core primitives alongside `Int` and
`Never` (built-in name, not a keyword). It binds a **measure** to a piece of computation, declaring
that the computation terminates, and provides a witness of termination.

**Form**: two arities, same predicate:

| Form                    | Anchor                        | Use Case                                                                                                           |
| ----------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `Terminates(m)`         | Name of the enclosing binding | Default form—self-recursive functions, loops                                                                       |
| `Terminates(FnType, m)` | Explicit function type        | When the measure needs explicit attribution (measure defined elsewhere, same measure serves multiple computations) |

```yaoxiang
// Measure: a regular function, can be unit-tested, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: measure defined elsewhere, explicitly indicating attribution
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: anchor is the binding name, measure is an expression in scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines the value type of the computation annotated at its type
position. The obligation falls on that computation—on a function, it falls on every recursive call
site; on a loop, it falls on the back edge—both are "the measure of the next state is strictly less
than the measure of the current state", judged under the path guard at that point.

The call site obligation is `m(callee_args) < m(caller_args)`; the back-edge obligation is
`m(next iteration) < m(current iteration)`. The two have the same form.

> **Why loops are covered**: Loops are anonymous constructs, usually not referable. The binding name
> is the name—the `acc` in `acc: Terminates(n - i) = while ...` provides the anchor, so the loop
> becomes referable. This is the reason for the unary form of `Terminates` acting on loops.

**Measure**: No restriction on the return type (not forced to be natural number); the "strictly
decreasing" on it is given by the well-order available on that type. Whether the measure is
well-founded (e.g., whether `>= 0` when returning `Int`) is an **independent obligation**, judged by
the compile-time proof pipeline just like the decreasing obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. Non-refined ordinary types (such as bare `while` loops, functions without
refinement signatures) do not enter verification mode and generate no termination obligations.

**Automatic exploration first**: The compiler first automatically explores measures (four templates:
linear rank function, predicate violation count, bounded increase/decrease, multiplicative scaling).
An explicit `Terminates` is required only when exploration fails.

**Runtime representation**: A pure compile-time entity, erased along with the witness, not present
in the runtime binary.

> See [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md) (landing mechanism) for
> the complete design.

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

**Syntax**: Type intersection `A & B` represents a type that satisfies both A and B

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using intersection type
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

// P is a predefined generic parameter name representing the current compile target platform
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 11: Type Properties

YaoXiang has only one type property to distinguish: linear vs. copyable. Automatically derived by
the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, returning = ownership
transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assignment of Dup type = shallow copy—copy
the handle/token, the underlying data is shared. Multiple holders point to the same data block.

| Type            | Property | Description                                                                     |
| --------------- | -------- | ------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-size read token, copying the token = multiple views point to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count + 1, share heap data                              |
| `&mut T`        | Linear   | Zero-size write token, exclusive, cannot be copied                              |
| All other types | Move     | Default ownership transfer                                                      |

**Primitive value types** (Int, Float, Bool, Char) are special handling built into the compiler: on
assignment, they automatically value-copy, and the two values are completely independent. This is
the compiler's native behavior, not part of the Dup type property.

```yaoxiang
// &T: Dup, free aliasing
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // Usable
print(view2.x)   // Usable

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
backup = p.clone()    // Deep copy, p is still usable
p2 = p.clone()        // Can be cloned multiple times
```

**Difference between Dup and Clone**:

|                         | Dup                                                     | Clone                                         |
| ----------------------- | ------------------------------------------------------- | --------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create complete independent copy   |
| **Call method**         | Implicit (automatic on assignment/parameter passing)    | Explicit (`.clone()`)                         |
| **Modification impact** | Affects each other (shared underlying data)             | Does not affect each other (independent copy) |
| **Applicable types**    | `&T` token, `ref T`                                     | Any type implementing the Clone interface     |
| **Cost**                | Zero overhead (token is zero-size type)                 | Depends on type                               |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy token, both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable, see the same data

// Primitive value type: compiler automatic value copy (not Dup)
x: Int = 42
y = x               // Value copy, x and y are completely independent
print(x)            // Usable

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor a primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views of the same data"
- Clone is used for scenarios that need independent copies; explicit call makes the cost visible
- The copy of primitive value types (Int/Float/Bool/Char) is the compiler's built-in behavior, not
  part of Dup
- Most user-defined types default to Move, zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references", but
"type-level proofs of access permission".

```
&T      →  Zero size, freezes source data (forbids WriteToken acquisition during this period),
          multiple read accesses safe under freeze guarantee → Dup (copyable)
&mut T  →  Zero size, exclusive read-write (forbids any other token),
          copying meaningless under exclusive access → Linear (not Dup)
```

**Key features**:

- Tokens are **ordinary types**, following the same scoping rules as all other types
- No lifetime annotation `'a` is required
- No dedicated borrow checker is needed—type properties (Dup/Linear) naturally derive permissions
- Completely disappear after compilation, zero runtime overhead

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

// Call site: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates &Point token
p.shift(1.0, 1.0)               // Compiler automatically creates &mut Point token
p.print()                       // OK, the previous token was released when the shift call ended

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, therefore all ordinary type operations are supported:

**Returning tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Token returned to caller
print(px_ref)                    // OK, token still in scope
```

**Storing in struct**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries token as a field
Window: Type = {
    target: Point,
    view: &Point,              // Token field—holds a read-only view of target
}
```

**Closures do not capture; context is fixed at creation site**—closures only consume their own
parameters; when outer data is needed, the value is fixed into the closure via currying at the
creation site:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter, gt_point(threshold) fixes the value into the closure at creation site
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: Once a closure (function value) escapes, the scope at its definition site may already be
> dead, so implicit capture of outer variables is not allowed; but the scope at the call site
> (creation site) is guaranteed alive, so fixing context as a value into the closure at that point
> is safe.

### 12.4 Automatic Borrow Selection

The call site compiler automatically selects by the following priority:

```
1. If the actual argument is used later → prefer to create a token (&T or &mut T, according to the method signature)
2. If the actual argument is not used later → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // Not used later → Move
```

**Method receiver follows signature semantics** (same as the receiver spelling convention in
RFC-011a): receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value →
Move (consume receiver). The borrow token produced at the call site is released when the call ends
(transient, §12.5 interval semantics); the interface's borrow receiver is explicitly declared as
`&Self` by the interface author, and the impl signature, after substitution `Self ↦ impl type`, must
match the interface exactly (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ Token automatically released after scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Use &mut Point
    }
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken still usable
}

// ❌ The same actual argument cannot simultaneously create an &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Brand Mechanism

Users never encounter brands. The compiler internally assigns a compile-time unique identifier to
each token:

```
User sees             Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Purposes of brands:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Association tracking**: `&Float` derived from field access carries the derived brand
  (`#N.field_x`), which the compiler can trace back to the parent token
- **Conflict detection**: Same-origin WriteToken and derived ReadToken cannot be live simultaneously

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read-write → Linear)
```

### 12.8 Borrow Tokens vs ref

|                 | `&T` / `&mut T`                                          | `ref`                                 |
| --------------- | -------------------------------------------------------- | ------------------------------------- |
| What            | Peek / modify in place                                   | Shared ownership                      |
| Range           | Follows token value's scope                              | Cross-scope                           |
| Cost            | Zero overhead (zero-size type, erased after compilation) | Rc or Arc (compiler-chosen)           |
| Escape          | Possible (token propagates with return value/struct)     | Designed to escape                    |
| Cross-task      | Not allowed (tokens do not implement cross-task passing) | Possible (compiler auto-selects Arc)  |
| Cycle detection | Not involved                                             | Silent within task, lint across tasks |

> Note (undefined): How to read content after `ref` is created (dereference/method/auto) is not yet
> defined in the spec; the current implementation reports E1052 for `*a`. To be added to this
> section when defined.

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

// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implement Serializable interface
}

// === Function types ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (built-in predicate, see §8.4) ===

// Unary: anchor is the binding name (self-recursive functions, loops)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// Binary: explicitly indicate measure attribution (measure defined elsewhere)
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 Generic Syntax

```
// Generic types
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Generic function
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// Type constraints
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated type
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Compile-time generics: N referenced in type position (k: N) → compile-time value parameter
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
// All types default to Move. Assignment, parameter passing, returning = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // Automatic value copy on assignment, the two values are completely independent
Bool, Char      // Not Dup, but the compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-size read token, copying the token = multiple views point to the same data
ref T           // Rc/Arc copy = reference count + 1, share heap data

// === Linear ===
&mut T          // Zero-size write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // Create an independent copy, modifications do not affect the original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-size compile-time read token, freezes source data → Dup (copyable)
&mut T          // Zero-size compile-time write token, exclusive read-write → Linear (not copyable)

// Call site automatic selection
// 1. Actual argument used later → create a token
// 2. Actual argument not used later → Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in structs, captured by closures
// ❌ Cannot cross tasks (tokens do not implement cross-task passing)
```
