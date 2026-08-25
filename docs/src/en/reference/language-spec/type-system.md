# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
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
| Truth \(\top\)                                 | `Void` (Unit, has default value)                |
| Falsity \(\bot\)                               | `Never` (zero constructor, no inhabitant)       |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| Case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (e.g., `Add`'s case analysis + recursive call on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checks.
- **Type checking is proof verification**. When a program passes type checking, the corresponding
  logical proposition is constructively proven.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): type-level case analysis + recursive call of natural number
   `Nat(Zero/Succ)` corresponds to Peano axioms—provided the compiler performs termination checks
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to finite quantification
   of "for each integer n there exists a type"

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
> (`name: type = value`), types and values still need to be distinguished at the syntactic level. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`); `TypeExpr` is a BNF placeholder corresponding to the `Type` enum in the
> implementation, meaning "this position expects a type."

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                                   | Default Size |
| -------- | ---------------------- | --------------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                     | 0 bytes      |
| `Never`  | ⊥ (falsity/empty type) | Zero constructor, no value at all. Divergent/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (truth/Unit)         | Has default void value, zero-field product type. `x: Void = <default>` is legal.              | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                               | 1 byte       |
| `Int`    | —                      | Signed integer                                                                                | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                              | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                         | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                                  | Variable     |
| `Char`   | —                      | Unicode character                                                                             | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                     | Variable     |

Bit-width integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Bit-width floats: `Float32`,
`Float64`.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to falsity (⊥) and
truth (⊤) respectively.

**Never (⊥, falsity/empty type)** — three non-negotiable properties:

1. **Zero constructor**: no literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing on the right side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which code passes type checking (though it never executes).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis and `match` branch confluence.

`Never` is a builtin type name (same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, truth/Unit)** — has exactly one inhabitant (default void value). `Void` is the identity
element of zero-field product types. `x: Void = <default>` is legal; a function with no explicit
`return` returns `Void`.

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
- The field name is followed by a colon and a type
- Interface names listed in the type body indicate implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) means the function belongs to
> `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call syntax like
> `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and RFC-010
> for details.

#### 3.1.1 Field Default Values

Type fields can specify default values and are optional during construction:

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
Point2(x=1, y=2) // Correct
Point2()          // Error
```

**Rules**:

- `field: Type = expression` -> has default value, optional during construction
- `field: Type` -> no default value, required during construction

#### 3.1.2 Builtin Binding

Methods can be bound directly inside a type definition body:

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
// Type implementing interfaces
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
d.draw(screen)        // After compilation: directly calls circle_draw, no vtable

// Function return value (indeterminate at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategies**:

| Scenario                           | Inference Result           | Call Method                 |
| ---------------------------------- | -------------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value              | Unknown                    | vtable                      |
| Heterogeneous collection           | Multiple types             | vtable                      |

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

Generic parameters are part of the function type and use the unified `()` syntax with normal
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In generic type definitions, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` denotes the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types (#299)

Container types are generic type constructors, not builtin primitives—treated the same as
user-defined generics, processed via the unified generic instantiation path:

| Type          | Semantics                                   | Backing            |
| ------------- | ------------------------------------------- | ------------------ |
| `List(T)`     | Growable list                               | `HeapValue::List`  |
| `Array(T, N)` | Fixed-length array (const generic N)        | `HeapValue::Array` |
| `Dict(K, V)`  | Key-value mapping                           | `HeapValue::Dict`  |
| `Set(T)`      | Set (no literal, constructed via `std.set`) | —                  |

Key rules:

- **Literal destination is determined by context**: bare literal `[...]` together with `List(T)`
  annotation falls into a growable list; `Array(T, N)` annotation on a literal directly falls into a
  fixed-length array.
- **Implicit List→Array conversion is prohibited**: fixed-length property is guaranteed at the type
  level—push only accepts a `List(A)` receiver.
- **Index failure contract** (runtime error is transitional, target state is compile-time
  refinement, via value-dependent types):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: returns `Bool` without error, right operand covers
  List/Array/Dict(key)/Set/Tuple/String/Range. First-class Hoare predicate, base of propositions
  provable at compile time by refined types.

In generic functions, type parameters are likewise declared in the signature, and the compiler
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
    push: (self: List(T), item: T) -> Void,   // self is just a convention, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a construction parameter, field name is the parameter name; fields with default
values can be omitted during construction, fields without default values are required.
Function-typed fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> construction parameter required
    extra: T,
}
// Automatically expanded full form (compiler internal view, not required to be written by user):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: call the auto-generated constructor
c  = Container(42, 43)            // construction parameters filled by field order; T auto-unpacked from elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type parameters + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // field-name style, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default values -> construction parameters can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, matching declared parameters position by position, left to
right):

1. Try to match actual arguments against declared parameters position by position: `Type` positions
   accept type arguments, compile-time value parameter positions (e.g., `Int`) accept compile-time
   constants.
2. If a partial match exists for a compile-time value parameter position, treat as type
   construction: check all parameter positions in order, on error report the first
   mismatched/missing parameter in declaration order.
3. If actual arguments do not match the declared parameters at all (all are values, no compile-time
   value parameter position matches), treat as construction: positional fill by field order, type
   parameters auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one layer of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two layers: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 doesn't match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols missing -> report first error: T expected Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: type parameters of generic type constructors are auto-unpacked from construction
parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions are
auto-unpacked from actual argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When
auto-unpacking fails, parameters must be supplied explicitly.

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
// Multiple constraint syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// Sorting of generic container
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

> **Erratum (#296)**: The original text called compile-time value parameters "determined at compile
> time by default"—this is **strictly incorrect**. In `add: (a: Int, b: Int) -> Int = a + b`,
> `a`/`b` are runtime value parameters. Only specific type parameters **referenced in type
> positions** are compile-time value parameters. See the correct definition below.

**Terminology**: Generic parameters annotated with specific types other than `Type` (e.g., `Int`)
are called **compile-time value parameter candidates**; whether they become compile-time value
parameters depends on whether their value is referenced in a type position (value dependence). **No
`const` keyword required** (the implementation used "const generic" internally, but the
documentation uniformly uses "compile-time value parameter").

**Decision rules (two steps)**:

1. **Shape coarse filtering**: parameter annotated with a specific type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use precise filtering**: the candidate name appears in a **type position** (type body field
   type, inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction argument
   position) → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing                                                    | Decision                                  | Reason                                      |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | only appear in value positions              |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is compile-time value parameter         | N is in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is compile-time value parameter         | N serves as the type of inner parameter k   |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in the type body        |

**Core design**: use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish between compile-time constants and runtime values. Fall-through candidates (shape is
candidate, use is not hit) degrade to runtime value parameters—function-level has been handled this
way, see the type constructor path in
[issue #297](https://github.com/ChenXu233/YaoXiang/issues/297).

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (Array length slot)
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at compile time in type position, result 120 is embedded in the type
arr: StaticArray(Int, factorial(5))  // compiler computes factorial(5) = 120 at compile time

// Value dependence: N as the type of inner parameter k
// N is a compile-time value parameter (appears in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (single-value type).
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
// IsTrue bridging and Assert refined type (see §8.3)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, divergent/compile error
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
dispatch pipeline based on "whether predicate free variables are reachable at compile time."

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                  |
| ------------------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enters proof pipeline: true → erased to Void, false → compile error (Never uninhabitable) |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Inserts runtime Bool check, injects refinement fact into flow-sensitive assumption set Γ  |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After assigning to a `mut` variable, all assumptions involving that variable are removed (kill set).
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

**Syntax**: The type intersection `A & B` represents a type that satisfies both A and B

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
// Basic specialization: using function overloading (automatically selected by compiler)
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

YaoXiang has only one type property that needs to be distinguished: linear vs copyable. It is
automatically inferred by the compiler.

### 11.1 Move (default ownership transfer)

All types follow Move semantics by default. Assignment, parameter passing, and return all equal
ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (shallow copy: copy handle, share data)

**The Dup property is used for reference/token types**. Assigning a Dup type = shallow copy—copy the
handle/token, the underlying data is shared. Multiple holders point to the same data.

| Type            | Property | Description                                                                        |
| --------------- | -------- | ---------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-size read token, copying the token = multiple views pointing to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, shared heap data                                 |
| `&mut T`        | Linear   | Zero-size write token, exclusive, not copyable                                     |
| All other types | Move     | Default ownership transfer                                                         |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: on assignment,
value is automatically copied, the two values are completely independent. This is a native compiler
behavior and does not belong to the Dup type property.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy token, both are valid
print(view.x)    // usable
print(view2.x)   // usable

// &mut T: Linear, not copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (explicit deep copy) and its relation to Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing the
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

|                         | Dup                                                     | Clone                                       |
| ----------------------- | ------------------------------------------------------- | ------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create complete independent copy |
| **Call style**          | Implicit (automatic on assignment/parameter passing)    | Explicit (`.clone()`)                       |
| **Modification impact** | Affect each other (shared underlying data)              | Do not affect each other (independent copy) |
| **Applicable types**    | `&T` tokens, `ref T`                                    | Any type implementing Clone interface       |
| **Cost**                | Zero overhead (tokens are zero-size types)              | Depends on type                             |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy token, both point to the same p
print(view.x)       // usable
print(view2.x)      // usable, see the same data

// Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // usable

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor primitive value type
```

**Design intent**:

- Dup is for token/reference types, solving the problem of "multiple views on the same data"
- Clone is for scenarios requiring independent copies, explicit call makes cost visible
- Primitive value types (Int/Float/Bool/Char) copying is a compiler builtin behavior, not belonging
  to Dup
- Most custom types default to Move, zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references", but
"type-level proofs of access permission".

```
&T      →  zero size, freezes source data (prohibits WriteToken acquisition during this period),
          freezing guarantees multi-read safety -> Dup (copyable)
&mut T  →  zero size, exclusive read-write (prohibits any other token),
          under exclusive access copying is meaningless -> Linear (non-Dup)
```

**Key features**:

- The token is a **normal type**, following the same scoping rules as all other types
- No lifetime annotation `'a` required
- No dedicated borrow checker required—type properties (Dup/Linear) naturally infer permissions
- Completely disappears after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method end: declare parameter type, determining required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Caller end: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // compiler automatically creates &Point token
p.shift(1.0, 1.0)               // compiler automatically creates &mut Point token
p.print()                       // OK, previous token has been released with shift call end

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are normal types, so they support all normal type operations:

**Return token**—token propagates with the return value:

```yaoxiang
// ✅ Child token and parent token are returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to caller
print(px_ref)                    // OK, token still in scope
```

**Store in struct**—struct can carry token fields:

```yaoxiang
// ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,              // token field—holds read-only view of target
}
```

**Closures do not capture, context is fixed at creation point**—closures only consume their own
parameters; when external data is needed, the value is fixed into the closure through currying at
the creation point:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter, gt_point(threshold) fixes value into closure at creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: after a closure (function value) escapes, the scope at its definition point may be dead, so
> it must not implicitly capture outer variables; but the scope at the call point (creation point)
> is necessarily alive, fixing the context as a value into the closure at that point is safe.

### 12.4 Automatic Borrow Selection

The caller's compiler automatically selects according to the following priority:

```
1. If the actual argument is used later -> prefer to create a token (&T or &mut T, based on method signature)
2. If the actual argument is not used later -> Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point -> compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point -> compiler creates &mut Point token
p2 = p             // not used later -> Move
```

### 12.5 Token Conflict Detection

Token conflict detection is a **borrowing Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them to the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be simultaneously alive
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ Token scope automatically releases after ending
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // uses &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken still available
}

// ❌ Same actual argument cannot simultaneously create &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internal: Brand Mechanism

Users never encounter brands. The compiler internally assigns a compile-time unique identifier to
each token:

```
User sees            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Association tracking**: field access-derived `&Float` carries the derived brand (`#N.field_x`),
  compiler can trace back to the parent token
- **Conflict detection**: same-source WriteToken and derived ReadToken cannot be simultaneously
  alive

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data -> Dup safe)
               | &mut T      // WriteToken (exclusive read-write -> Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                              | `ref`                                       |
| --------------- | ------------------------------------------------------------ | ------------------------------------------- |
| What it does    | Glance at / modify in place                                  | Shared ownership                            |
| Scope           | Follows the token value's scope                              | Cross-scope                                 |
| Cost            | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler chooses)                |
| Escape          | Possible (token propagates with return value/struct)         | Designed for escape                         |
| Cross-task      | Not possible (tokens don't support cross-task transfer)      | Possible (compiler automatically picks Arc) |
| Cycle detection | Not involved                                                 | Silent in task, cross-task lint             |

> Note (undefined): how to read content after ref is created (dereference/method/auto) has not been
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

// Type implementing interface
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

// Compile-time generic: N referenced in type position (k: N) -> compile-time value parameter
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
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler builtin) ===
Int, Float,     // auto value copy on assignment, two values completely independent
Bool, Char      // not Dup, but compiler's builtin handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // zero-size read token, copy token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count +1, share heap data

// === Linear ===
&mut T          // zero-size write token, Linear (exclusive, not copyable)

// === Clone (explicit deep copy) ===
value.clone()   // create independent copy, modifications don't affect original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow token ===
&T              // zero-size compile-time read token, freezes source data -> Dup (copyable)
&mut T          // zero-size compile-time write token, exclusive read-write -> Linear (not copyable)

// Caller automatic selection
// 1. If actual argument is used later -> create token
// 2. If actual argument is not used later -> Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ can be returned, stored in struct, captured by closure
// ❌ cannot cross tasks (tokens don't support cross-task transfer)
```
