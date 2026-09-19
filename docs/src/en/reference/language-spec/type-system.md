# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, compound types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

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
| True \(\top\)                                  | `Void` (Unit, with default value)               |
| False \(\bot\)                                 | `Never` (zero constructor, no inhabitant)       |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is classification (case analysis), not mathematical induction.
> Induction requires type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (e.g., `Add`'s case analysis + recursive calls on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proved.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): The type-level case analysis + recursive calls of natural number
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checking
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

> **Design Note**: Although RFC-010 proposes a unified model of "everything is assignment"
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), where `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the
> implementation, meaning "this position expects a type."

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Counterpart | Description                                                                              | Default Size |
| -------- | ------------------- | ---------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                   | Meta type                                                                                | 0 bytes      |
| `Never`  | ⊥ (false/empty)     | Zero constructor, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)       | Has default void value, zero-field product type. `x: Void = <default>` is valid.         | 0 bytes      |
| `Bool`   | —                   | Boolean value: `true` / `false`                                                          | 1 byte       |
| `Int`    | —                   | Signed integer                                                                           | 8 bytes      |
| `Uint`   | —                   | Unsigned integer                                                                         | 8 bytes      |
| `Float`  | —                   | Floating point                                                                           | 8 bytes      |
| `String` | —                   | UTF-8 string                                                                             | variable     |
| `Char`   | —                   | Unicode character                                                                        | 4 bytes      |
| `Bytes`  | —                   | Raw bytes                                                                                | variable     |

Bit-width integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`; bit-width floats: `Float32`,
`Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤) respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructor**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which code can pass type checking (though it will never actually be executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered on the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity element of the zero-field product type. `x: Void = <default>` is valid. The value of a
block is given by the **tail expression** (empty block `{}` is `Void`), see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for details.

---

## Chapter 3: Compound Types

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
- Field name is followed directly by colon and type
- Interface name written in the type body indicates implementation of that interface

> **Namespace attribution**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. For `.` call syntax like
> `p.draw()` to work, explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and RFC-010
> for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, optional at construction:

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

- `field: Type = expression` -> has default value, optional at construction
- `field: Type` -> no default value, required at construction

#### 3.1.2 Built-in Bindings

Methods can be bound directly in the type definition body:

```yaoxiang
// Method 1: Bind external function reference
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

**Syntax**: An interface is a record type where all fields are function types

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

**Direct interface assignment**: A concrete type can be directly assigned to an interface type
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // after compilation: directly calls circle_draw, no vtable

// Function return value (undeterminable at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // looks up method via vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                   | Inference Result           | Call Method                 |
| -------------------------- | -------------------------- | --------------------------- |
| Direct concrete assignment | Concrete type determinable | Direct call (zero overhead) |
| Function return value      | Unknown                    | vtable                      |
| Heterogeneous collection   | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, closure statement)**: YaoXiang's interfaces are
structural types (interface = record with all function-type fields), not nominal traits—there is no
"who can implement for whom" attribution issue across crates/modules, and Rust-style orphan rules
and coherence checks have no applicable object (ruling record in RFC-011 §2.1). The corresponding
guarantee in the structural world is **duplicate implementation rejection**: defining the same
method signature on a type repeatedly is a compile error (RFC-011a §3, override prohibited;
overloading allowed).

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

Generic parameters are part of the function type, unified with regular parameters using `()` syntax:

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
treatment as user-defined generics, going through the unified generic instantiation path. The
attribution of length information is the fundamental difference among three container concepts:

| Type          | Length        | Semantics                             | Foundation                               |
| ------------- | ------------- | ------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)  | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Runtime-length raw buffer, growable   | Core primitive (heap contiguous buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list) | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value map                         | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: defined by YaoXiang itself in
> `std.list`, receiving the same treatment as user-defined generic records. All growable semantic
> strategies (when to expand, by how much, whether sharing is allowed) are in the library, and the
> compiler does not participate. `Vec(T)` is the minimal foundation primitive it depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When needs arise,
> complete it following the Dict pattern.

Key rules:

- **Literal destination determined by context**: bare `[...]` literals and `List(T)` annotations
  fall into growable lists; `Array(T, N)` annotations directly applied to literals fall into
  fixed-length arrays. Destination validation: element count == N, element type compatible with T,
  mismatch is compile-time E1002; when N is a symbolic constant (const parameter), count validation
  is deferred to the refinement type stage.
- **Implicit List→Array conversion prohibited**: fixed-length property is guaranteed at the type
  level—push only accepts `List(A)` receiver.
- **Performance hierarchy**: performance decreases and flexibility increases from bottom to top:
  `Array` > `Vec` > `List`.
- **Indexing failure contract** (runtime error is transitional, target state covered by compile-time
  refinement, using value-dependent types):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: returns `Bool` without error, right operand covers
  List/Array/Dict(key)/Tuple/String/Range. First-class Hoare predicate, the foundation of refinement
  type compile-time provable propositions.`

In generic functions, type parameters are likewise declared in the signature, and the compiler
automatically infers from actual arguments:

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

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a construction parameter, with the field name as the parameter name; fields
with default values can be omitted at construction, fields without default values are required.
Function-type fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> construction parameter required
    extra: T,
}
// Auto-expanded complete form (compiler's internal view, users don't need to write):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Calls: calling the auto-generated constructor
c  = Container(42, 43)            // construction parameters filled by field order; T auto-unpacked from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type parameter + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // field-name style, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default value -> construction parameter can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Calling rules** (single parenthesis, matching declared parameters position by position, left to
right):

1. Actual arguments attempt to match declared parameter positions one by one: `Type` position
   accepts type arguments, compile-time value parameter positions (e.g., `Int`) accept compile-time
   constants.
2. If a partial match succeeds with a compile-time value parameter position, process as type
   construction: check all parameter positions, when reporting errors, **report the first
   mismatched/missing parameter** in declaration order.
3. If actual arguments do not correspond to declared parameters at all (all are values, no
   compile-time value parameter position matches), process as construction parameters: positional
   filling by field order, type parameters auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one-level type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two levels: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 doesn't match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols missing -> report first error: T expects Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: Type parameters of generic type constructors are auto-unpacked from construction
parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions are
auto-unpacked from actual argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When
unpacking fails, explicit filling is required.

---

## Chapter 5: Type Constraints

### 5.1 Single Constraint

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// Interface type definition (as constraint)
Clone: Type = {
    clone: () -> Clone
}

// Using a constraint
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

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

// Using associated types
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

> **Erratum**: The original text states that compile-time value parameters are "defaulted to
> compile-time determined", which is **strictly incorrect**— in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters. Only specific type
> parameters **referenced in type position** are compile-time value parameters. See below for the
> correct definition.

**Terminology**: Generic parameters annotated with concrete types other than `Type` (such as `Int`)
are called **compile-time value parameter candidates**, and whether they become compile-time value
parameters depends on whether their value is referenced in type position (value-dependent). **No
`const` keyword required** (the implementation internally used "const generics" to refer to these,
but documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Form coarse screening**: parameter annotated with concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Usage fine screening**: candidate name appears in **type position** (field types in type body,
   inner `Fn` parameter types, `Assert` predicates, `Array(T, N)` type construction argument
   positions) → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing                                                    | Determination                             | Reason                                           |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters              | only appears in value position                   |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter            | N appears in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter            | N serves as inner parameter k's type             |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N not referenced in type body                    |

**Core design**: use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Fallen-through candidates (form is
candidate, usage not hit) degrade to runtime value parameters—both function-level and type
constructor paths follow this treatment.

```yaoxiang
// Compile-time value parameter: N is referenced in type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction argument position → compile-time value parameter
    length: N
}

// Usage: factorial(5) evaluates at compile time in type position, result 120 embedded in type
arr: Measure(Int, factorial(5))  // compiler computes factorial(5) = 120 at compile time

// Value-dependent: N as inner parameter k's type
// N is a compile-time value parameter (appears in (k: N)'s type position);
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

// Example: compile-time branching
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

### 8.3 Assert Refinement Types and assert Statement

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch pipeline based on "whether the predicate's free variables are reachable at compile time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch selection rules**:

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

After a `mut` variable assignment, all assumptions involving that variable are removed (kill set).
When branches merge, Γ takes the intersection of each branch.

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
// Basic specialization: using function overloading (compiler selects automatically)
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
// Platform type enum (standard library defined)
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

YaoXiang has only one type attribute that needs to be distinguished: linear vs copyable.
Automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types default to Move semantics. Assignment, parameter passing, return = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p cannot be read again
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup attribute is used for reference/token types**. Assignment of Dup types = shallow copy—copy
the handle/token, underlying data is shared. Multiple holders point to the same block of data.

| Type            | Attribute | Description                                                                 |
| --------------- | --------- | --------------------------------------------------------------------------- |
| `&T`            | Dup       | Zero-sized read token, copying token = multiple views pointing to same data |
| `ref T`         | Dup       | Rc/Arc copy = reference count+1, share heap data                            |
| `&mut T`        | Linear    | Zero-sized write token, exclusive, cannot copy                              |
| All other types | Move      | Default ownership transfer                                                  |

**Primitive value types** (Int, Float, Bool, Char) are specially handled by the compiler: assignment
automatically value-copies, two values are completely independent. This is the compiler's native
behavior, not a Dup type attribute.

```yaoxiang
// &T: Dup, can freely alias
view: &Point = &p
view2 = view     // Dup: copy token, both are valid
print(view.x)    // available
print(view2.x)   // available

// &mut T: Linear, cannot copy
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot copy
```

### 11.3 Clone (Explicit Deep Copy) and its Relationship with Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing a `.clone()`
method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // deep copy, p is still available
p2 = p.clone()        // can be cloned multiple times
```

**Differences between Dup and Clone**:

|                         | Dup                                                     | Clone                                          |
| ----------------------- | ------------------------------------------------------- | ---------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create complete independent replica |
| **Call method**         | Implicit (automatic on assignment/parameter passing)    | Explicit (`.clone()`)                          |
| **Modification effect** | Mutually affect (share underlying data)                 | Mutually unaffected (independent replicas)     |
| **Applicable types**    | `&T` token, `ref T`                                     | Any type implementing Clone interface          |
| **Cost**                | Zero overhead (token is zero-sized type)                | Depends on type                                |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy token, both point to same p
print(view.x)       // available
print(view2.x)      // available, seeing the same data

// Primitive value type: compiler auto value-copies (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // available

// Clone: explicit deep copy, create independent replica
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still available
r = p               // Move: ownership transfer, because Point is neither Dup nor primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views of the same data"
- Clone is used for scenarios requiring independent replicas, making cost visible through explicit
  calls
- Copying of primitive value types (Int/Float/Bool/Char) is the compiler's built-in behavior, not
  Dup
- Most custom types default to Move, zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references" but
"type-level proofs of access permission".

```
&T      →  zero-sized, freezes source data (prohibits WriteToken acquisition during this period),
          under freeze guarantee, multiple read-only are safe → Dup (copyable)
&mut T  →  zero-sized, exclusive read-write (prohibits any other tokens),
          copying under exclusive access is meaningless → Linear (non-Dup)
```

**Key characteristics**:

- Tokens are **ordinary types**, following the same scoping rules as all other types
- No lifetime annotation `'a` required
- No dedicated borrow checker needed—type attributes (Dup/Linear) naturally derive permissions
- Completely disappear after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare parameter type, determine required permission
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
p.print()                       // compiler auto-creates &Point token
p.shift(1.0, 1.0)               // compiler auto-creates &mut Point token
p.print()                       // OK, previous token released after shift call ends

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all ordinary type operations:

**Return token**—token propagates along with return value:

```yaoxiang
// ✅ Sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to caller
print(px_ref)                    // OK, token still in scope
```

**Store in struct**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,              // token field—holds read-only view of target
}
```

**Closures don't capture, context solidified at creation point**—closures only eat their own
parameters; when needing outer data, values are solidified into the closure through currying at the
creation point:

```yaoxiang
// ✅ Context solidified via currying: threshold is parameter, gt_point(threshold) solidifies value into closure at creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: after a closure (function value) escapes, the scope at its definition may already be dead,
> so it must not implicitly capture outer variables; but the call site (creation point) scope is
> guaranteed alive, and solidifying the context at that point as a value entering the closure is
> safe.

### 12.4 Automatic Borrow Selection

The call-side compiler automatically selects by the following priority:

```
1. If the actual argument is used afterwards → prefer creating a token (&T or &mut T, based on method signature)
2. If the actual argument is not used afterwards → Move
3. Preference matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // not used afterwards → Move
```

**Method receiver follows signature semantics** (erratum 2026-08-30, same as RFC-011a receiver
spelling convention): receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by
value → Move (consume receiver). Borrow tokens generated at the call site are released when the call
ends (transient, §12.5 interval semantics); interface borrow receivers are explicitly declared as
`&Self` by the interface author, and impl signature must be completely consistent with the interface
after `Self ↦ impl type` substitution (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is the **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and feeds them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot both be active
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ Token automatically released after scope ends
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

Users never touch brands. The compiler internally assigns a unique compile-time identifier to each
token:

```
User sees              Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is compile-time unique integer
```

Brand uses:

- **Anti-counterfeiting**: tokens can only be obtained from the owner capsule, cannot be constructed
  out of thin air
- **Correlation tracking**: `&Float` derived from field access carries the derived brand
  (`#N.field_x`), compiler can trace back to the parent token
- **Conflict detection**: same-source WriteToken and derived ReadToken cannot be active
  simultaneously

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read-write → Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                               | `ref`                                |
| --------------- | ------------------------------------------------------------- | ------------------------------------ |
| What it does    | Glance/modify in place                                        | Shared ownership                     |
| Scope           | Follows the token value's scope                               | Cross-scope                          |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler selects)         |
| Escape          | Possible (token propagates with return value/struct)          | Designed to escape                   |
| Cross-task      | Not possible (tokens not implemented for cross-task passing)  | Possible (compiler auto-selects Arc) |
| Cycle detection | Not involved                                                  | Silent in task, cross-task lint      |

> Note (undefined): how to read content after ref creation (dereference/method/auto) is not yet
> defined in the spec, current implementation `*a` reports E1052. To be added to this section after
> definition.

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
    Serializable    // implements Serializable interface
}

// === Function type ===

Adder: Type = (Int, Int) -> Int
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

// Compile-time generics: N referenced in type position (k: N) → compile-time value parameter
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
Int, Float,     // auto value-copy on assignment, two values completely independent
Bool, Char      // not Dup, is compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // zero-sized read token, copying token = multiple views pointing to same data
ref T           // Rc/Arc copy = reference count+1, share heap data

// === Linear ===
&mut T          // zero-sized write token, Linear (exclusive, cannot copy)

// === Clone (explicit deep copy) ===
value.clone()   // create independent replica, modification doesn't affect original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // zero-sized compile-time read token, freezes source data → Dup (copyable)
&mut T          // zero-sized compile-time write token, exclusive read-write → Linear (cannot copy)

// Call-side auto selection
// 1. Actual argument used afterwards → create token
// 2. Actual argument not used afterwards → Move
// 3. Preference matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in struct, captured by closure
// ❌ Cannot cross task (tokens not implemented for cross-task passing)
```
