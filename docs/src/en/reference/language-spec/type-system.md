# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals a deep correspondence between the type system of a programming language and mathematical
logic:

| Logic                                          | Programming Language                                 |
| ---------------------------------------------- | ---------------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                          |
| Proof \(p: P\)                                 | Program `x: T = ...`                                 |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                             |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                        |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                          |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                          |
| True \(\top\)                                  | `Void` (Unit, with default value)                    |
| False \(\bot\)                                 | `Never` (zero constructors, uninhabited)             |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                                   |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (e.g., `Add` with case analysis + recursive calls on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, the equivalent
  logical proposition has been constructively proven.

### 0.3 Impact on Language Design

Concrete embodiments of the Curry-Howard correspondence in YaoXiang:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradoxes
   (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): type-level case analysis + recursive calls on natural numbers
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to finite quantification
   of "for every integer n there exists a type"

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
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`); `TypeExpr` is a BNF placeholder corresponding to the `Type` enum in the
> implementation, meaning "a type is expected at this position".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical correspondence | Description                                                                                         | Default size |
| -------- | ---------------------- | --------------------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                           | 0 bytes      |
| `Never`  | ⊥ (False/empty type)   | Zero constructors, no value exists. Return type for divergence/panic. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (True/Unit)          | Has default void value, zero-field product type. `x: Void = <default>` is valid.                    | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                                     | 1 byte       |
| `Int`    | —                      | Signed integer                                                                                      | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                                    | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                               | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                                        | variable     |
| `Char`   | —                      | Unicode character                                                                                   | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                           | variable     |

Integers with explicit bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Floats with explicit
bit width: `Float32`, `Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to False (⊥) and True
(⊤), respectively.

**Never (⊥, False/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing to write on the right-hand side.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   and subsequent code passes type checking (though it never executes).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (with the same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, True/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity element of zero-field product types. `x: Void = <default>` is valid; functions without an
explicit `return` return `Void`.

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

- Record types are defined using curly braces `{}`
- A field name is followed directly by a colon and a type
- An interface name written inside the type body indicates implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) denotes that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call
> syntax like `p.draw()` work, you must explicitly bind: `Point.draw = draw[0]`. See RFC-004 and
> RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, making them optional during construction:

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

- `field: Type = expression` -> has default value, optional during construction
- `field: Type` -> no default value, required during construction

#### 3.1.2 Builtin Bindings

Methods can be bound directly inside a type definition body:

```yaoxiang
// Method 1: reference an external function and bind it
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
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // implements Drawable
    Serializable     // implements Serializable
}
```

**Direct interface assignment**: A concrete type can be directly assigned to an interface type
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type is known at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // method lookup via vtable

// Interface as a function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario                           | Inferred result     | Call method                 |
| ---------------------------------- | ------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type known | Direct call (zero overhead) |
| Function return value              | Unknown             | vtable                      |
| Heterogeneous collection           | Multiple types      | vtable                      |

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

Generic parameters are part of the function type signature, sharing the `()` syntax with ordinary
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` denotes the return type:

````yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
``

### 4.1.1 Container Types (#299)

Container types are generic type constructors, not built-in primitives—they receive the same treatment as user-defined generics and are processed via the unified generic instantiation path:

| Type | Semantics | Underlying representation |
| --- | --- | --- |
| `List(T)` | growable list | `HeapValue::List` |
| `Array(T, N)` | fixed-length array (const generic N) | `HeapValue::Array` |
| `Dict(K, V)` | key-value map | `HeapValue::Dict` |

> `Set(T)` has been removed (decision 4 of #300): no literal, no runtime representation, no `std.set`. When the need arises, complete it following the `Dict` pattern.

Key rules:

- **The destination of a literal is determined by context**: a bare `[...]` literal and a `List(T)` annotation land in a growable list; an `Array(T, N)` annotation applied directly to a literal lands in a fixed-length array. Landing-point validation (#300): element count == N, element type compatible with T; mismatches are compile-time E1002; when N is a symbolic constant (const parameter), count validation is deferred to the refinement type stage.
- **Implicit List→Array conversion is forbidden**: fixed-length-ness is guaranteed at the type level—`push` only accepts `List(A)` receivers.
- **Indexing failure contract** (runtime errors are transitional; the target state is compile-time refinement coverage via value-dependent types):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **`in` membership predicate**: returns `Bool` and does not error; the right-hand side covers `List`/`Array`/`Dict` (keys)/`Tuple`/`String`/`Range`. A first-class Hoare predicate, serving as the basis for propositions provable at compile time via refinement types.`

In a generic function, type parameters are likewise declared in the signature, and the compiler infers them automatically from arguments:

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
````

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

### 4.3 Generic Construction Call and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each field
corresponds to a construction parameter, and the field name is the parameter name; fields with
default values may be omitted during construction, fields without default values are required.
Function-type fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> construction parameter required
    extra: T,
}
// Automatically expanded full form (compiler's internal view, users need not write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Calls: call the auto-generated constructor
c  = Container(42, 43)            // construction parameters filled by field order; T auto-unpacked from elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type argument + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // field-name form, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default value -> construction parameter may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, matching each declared parameter position from left to right):

1. Each argument attempts to match the corresponding declared parameter position: a `Type` position
   accepts a type argument; a compile-time value position (e.g., `Int`) accepts a compile-time
   constant.
2. If a compile-time value position matches successfully (partial match), treat it as a type
   construction: check all parameter positions in order; on error, report the **first
   mismatched/missing parameter in declaration order** first.
3. If the arguments do not correspond to declared parameters at all (all are values, no compile-time
   value position matches), treat as construction parameters: positional form fills by field order;
   type arguments are auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one-level type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two levels: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ pos 0: T←42 doesn't match (42 is not a type); pos 1: Rows←42 matches;
              //    pos 2: Cols missing -> report first error: T expected Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: Type parameters of a generic type constructor are auto-unpacked from
construction parameter elements (`Container(42, 43)` → T=Int); type parameters of a generic function
are auto-unpacked from argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When
unpacking is impossible, the arguments must be supplied explicitly.

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
// More complex associated types
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // associated type can also be generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-time Generics

### 7.1 Compile-time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // compile-time constant (candidate)
```

> **Erratum (#296)**: The original text stated that compile-time value parameters are "determined at
> compile time by default"—this statement is **strictly wrong**—in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters. Only concrete type
> parameters **referenced at the type position** are compile-time value parameters. The correct
> definition is below.

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (e.g., `Int`)
is called a **compile-time value parameter candidate**; whether it becomes a compile-time value
parameter depends on whether its value is referenced at the type position (value dependency). **No
`const` keyword is needed** (the implementation internally once used "const generics"; the
documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Shape-based coarse filtering**: The parameter is annotated with a concrete type other than
   `Type` (`Int`/`Bool`/`Float`) → candidate.
2. **Use-based fine filtering**: The candidate name appears in a **type position** (a type body
   field type, an inner `Fn` parameter type, an `Assert` predicate, an `Array(T, N)` type
   construction argument position) → true compile-time value parameter; otherwise, **runtime value
   parameter**.

| Writing                                                    | Determination                             | Reason                                               |
| ---------------------------------------------------------- | ----------------------------------------- | ---------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | only appear at value position                        |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N appears at the type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N serves as the type of the inner parameter k        |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in the type body                 |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. A candidate that falls through (passes shape
filtering but misses use filtering) degrades to a runtime value parameter—function-level cases
already follow this; the type constructor path is tracked in
[issue #297](https://github.com/ChenXu233/YaoXiang/issues/297).

```yaoxiang
// Compile-time value parameter: N is referenced at the type position (Array length slot)
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears at the type construction argument position → compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at the type position (compile time), result 120 is embedded in the type
arr: StaticArray(Int, factorial(5))  // compiler computes factorial(5) = 120 at compile time

// Value dependency: N serves as the type of the inner parameter k
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
// IsTrue bridging and Assert refinement type (see §8.3 for details)
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

`assert` and `Assert` are two sides of the same refinement primitive—automatically chosen by the
dispatch pipeline based on "whether the predicate's free variables are reachable at compile time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                    |
| ------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erase to Void, false → compile error (Never is uninhabited)    |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refinement facts into the flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable is reassigned, all assumptions involving that variable are removed (kill
set). When branches merge, Γ takes the intersection of each branch.

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
// Basic specialization: function overloading (compiler auto-selects)
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

// P is a predefined generic parameter name representing the current compilation platform
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 11: Type Properties

YaoXiang has only one type property to distinguish: linear vs copyable. It is automatically inferred
by the compiler.

### 11.1 Move (default ownership transfer)

All types follow Move semantics by default. Assignment, parameter passing, and return = ownership
transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (shallow copy: copy the handle, share the data)

**The Dup property is used for reference/token types**. Assignment of a Dup type = shallow copy—copy
the handle/token, the underlying data is shared. Multiple holders point to the same block of data.

| Type            | Property | Description                                                                         |
| --------------- | -------- | ----------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token, copying the token = multiple views pointing to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, shared heap data                                  |
| `&mut T`        | Linear   | Zero-sized write token, exclusive, cannot be copied                                 |
| All other types | Move     | Default ownership transfer                                                          |

**Primitive value types** (Int, Float, Bool, Char) are a special built-in compiler treatment:
assignment automatically copies the value, and the two values are completely independent. This is a
native compiler behavior, not part of the Dup type property.

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

### 11.3 Clone (explicit deep copy) and its relation to Dup

**Clone** is an explicit deep-copy interface. All types can implement Clone, providing a `.clone()`
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

|                         | Dup                                                    | Clone                                      |
| ----------------------- | ------------------------------------------------------ | ------------------------------------------ |
| **Semantics**           | Shallow copy: copy handle/token, share underlying data | Deep copy: create a fully independent copy |
| **Invocation**          | Implicit (automatic on assignment/parameter passing)   | Explicit (`.clone()`)                      |
| **Modification effect** | Affect each other (shared underlying data)             | Independent (separate copies)              |
| **Applicable types**    | `&T` token, `ref T`                                    | Any type implementing Clone                |
| **Cost**                | Zero overhead (tokens are zero-sized types)            | Depends on the type                        |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // available
print(view2.x)      // available, viewing the same data

// Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // available

// Clone: explicit deep copy, creating an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still available
r = p               // Move: ownership transfer, because Point is not Dup and not a primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views on the same data"
- Clone is used for scenarios requiring independent copies, with explicit calls making the cost
  visible
- The copying of primitive value types (Int/Float/Bool/Char) is a compiler built-in behavior, not
  part of Dup
- Most user-defined types default to Move, zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references", but
"type-level proofs of access permission".

```
&T      →  zero-sized, freezes the source data (forbids WriteToken from being acquired during this time),
          multiple read-only tokens are safe under the freeze guarantee → Dup (copyable)
&mut T  →  zero-sized, exclusive read-write (forbids any other token),
          copying is meaningless under exclusive access → Linear (not Dup)
```

**Key properties**:

- Tokens are **ordinary types**, following the same scoping rules as all other types
- No lifetime annotation `'a` needed
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally infer permissions
- Completely disappear after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare the parameter type, which determines the required permission
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
p.print()                       // compiler automatically creates &Point token
p.shift(1.0, 1.0)               // compiler automatically creates &mut Point token
p.print()                       // OK, the previous token was released when the shift call ended

// Multiple &T tokens coexisting — Dup types allow free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations of ordinary types:

**Return a token** — the token propagates along with the return value:

```yaoxiang
// ✅ Sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to the caller
print(px_ref)                    // OK, token is still in scope
```

**Store in a struct** — a struct can carry token fields:

```yaoxiang
// ✅ Struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field — holding a read-only view of target
}
```

**Closures do not capture; context is fixed at the creation point** — a closure only eats its own
parameters; when it needs outer data, the value is fixed into the closure via currying at the
creation point:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter; gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: When a closure (function value) escapes, the scope at its definition site may be dead, so it
> must not implicitly capture outer variables; however, the scope at the call site (creation point)
> is necessarily alive, and it is safe to fix the context as a value into the closure at that point.

### 12.4 Automatic Borrow Selection

The compiler at the call site automatically selects according to the following priority:

```
1. If the argument is used later → prefer creating a token (&T or &mut T, based on method signature)
2. If the argument is not used later → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // not used later → Move
```

**Method receiver follows signature semantics** (erratum 2026-08-30, same as RFC-011a receiver
spelling convention): the receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow
token; by value → Move (consume the receiver). The borrow token created at the call site is released
when the call ends (transient, see §12.5 interval semantics); the interface's borrow receiver is
explicitly declared as `&Self` by the interface author, and the impl signature, after
`Self ↦ impl type` substitution, must match the interface exactly (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and feeds them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and the derived &T cannot be live at the same time
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ Token is automatically released after its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // uses &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken still available
}

// ❌ The same argument cannot simultaneously create an &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Brand Mechanism

Users never touch brands. The compiler internally assigns a unique compile-time identifier to each
token:

```
User-visible         Compiler-internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: tokens can only be obtained from the owner's capsule, not constructed out of
  thin air
- **Association tracking**: a field-access-derived `&Float` carries the derived brand
  (`#N.field_x`), allowing the compiler to trace back to the parent token
- **Conflict detection**: a WriteToken and a derived ReadToken from the same source cannot be live
  at the same time

Brands disappear completely after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read-write → Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                               | `ref`                               |
| --------------- | ------------------------------------------------------------- | ----------------------------------- |
| What it does    | Look at / modify in place                                     | Shared ownership                    |
| Scope           | Follows the token value's scope                               | Cross-scope                         |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler auto-selects)   |
| Escape          | Yes (token propagates via return value / struct)              | Designed to escape                  |
| Cross-task      | No (tokens do not implement cross-task passing)               | Yes (compiler auto-selects Arc)     |
| Cycle detection | Not involved                                                  | Silent within task, cross-task lint |

> Note (undefined): how to read the content after a `ref` is created (dereference/method/auto) is
> not yet defined in the specification; the current implementation reports E1052 for `*a`. To be
> added to this section once defined.

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

// Type implementing interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // implements Serializable interface
}

// === Function type ===

Adder: Type = (Int, Int) -> Int
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

// Compile-time generic: N is referenced at the type position (k: N) → compile-time value parameter
factorial: (N: Int)(k: N) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Property Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, parameter passing, and return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // auto value copy on assignment, the two values are completely independent
Bool, Char      // not Dup; this is the compiler's built-in handling of primitives

// === Dup (shallow copy: copy the handle, share underlying data) ===
&T              // zero-sized read token, copying the token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count +1, shared heap data

// === Linear ===
&mut T          // zero-sized write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent copy, modifications do not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // zero-sized compile-time read token, freezes source data → Dup (copyable)
&mut T          // zero-sized compile-time write token, exclusive read-write → Linear (cannot be copied)

// Call site auto-selection
// 1. Argument used later → create a token
// 2. Argument not used later → Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in a struct, captured by a closure
// ❌ Cannot cross tasks (tokens do not implement cross-task passing)
```
