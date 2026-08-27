# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and trait.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between the type systems of programming languages and mathematical
logic:

| Logic                                          | Programming Language                            |
| ---------------------------------------------- | ----------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                     |
| Proof \(p: P\)                                 | Program `x: T = ...`                            |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                        |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                   |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                     |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                     |
| True \(\top\)                                  | `Void` (Unit, has a default value)              |
| False \(\bot\)                                 | `Never` (zero constructors, uninhabited)        |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checks.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computations correspond to correct constructive proofs**. YaoXiang's type
  families (e.g., case analysis + recursive calls of `Add` over `Nat`) are essentially the
  type-level encoding of mathematical induction—provided the compiler can perform termination
  checks.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proved.

### 0.3 Impact on Language Design

Concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): The case analysis + recursive calls at the type level for the
   natural number `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs
   termination checks
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to a finite
   quantification of "for each integer n, there exists a type"

---

## Chapter 1: Type Classification

### 1.1 Type Expression

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
> (`name: type = value`), at the syntax level, types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), and `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the
> implementation, indicating "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Type

| Type     | Logic Correspondence | Description                                                                        | Default Size |
| -------- | -------------------- | ---------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                    | Meta type                                                                          | 0 bytes      |
| `Never`  | ⊥ (False/Empty)      | Zero constructors, no values. Diverging/panic return type. `Never <: T` for any T. | 0 bytes      |
| `Void`   | ⊤ (True/Unit)        | Has a default void value, zero-field product type. `x: Void = <default>` is valid. | 0 bytes      |
| `Bool`   | —                    | Boolean values: `true` / `false`                                                   | 1 byte       |
| `Int`    | —                    | Signed integer                                                                     | 8 bytes      |
| `Uint`   | —                    | Unsigned integer                                                                   | 8 bytes      |
| `Float`  | —                    | Floating point                                                                     | 8 bytes      |
| `String` | —                    | UTF-8 string                                                                       | Variable     |
| `Char`   | —                    | Unicode character                                                                  | 4 bytes      |
| `Bytes`  | —                    | Raw bytes                                                                          | Variable     |

Width-suffixed integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128` Width-suffixed floats:
`Float32`, `Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to False (⊥) and True
(⊤) respectively.

**Never (⊥, False/Empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing to write on the right.
2. **Principle of explosion**: `Never <: T` for any type `T`. `assert(false)` returns `Never`, and
   subsequent code passes type checking (although it will never execute).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered with the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, True/Unit)** — has exactly one inhabitant (the default void value). `Void` is the unit of
zero-field product types. `x: Void = <default>` is valid, and a function returns `Void` by default
when it has no `return`.

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
- Field names are followed directly by a colon and the type
- Interface names written in the type body indicate that the interface is implemented

> **Namespace affiliation**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. For `.` call syntax like
> `p.draw()` to take effect, you must explicitly bind: `Point.draw = draw[0]`. See RFC-004 and
> RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, which are optional when constructing:

```yaoxiang
// Field with a default value - optional at construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without a default value - required at construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // Correct
Point2()          // Error
```

**Rules**:

- `field: Type = expression` -> has a default value, optional at construction
- `field: Type` -> no default value, required at construction

#### 3.1.2 Builtin Binding

Methods can be bound directly within the type definition body:

```yaoxiang
// Method 1: Reference an external function binding
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
    Drawable,        // Implement Drawable interface
    Serializable     // Implement Serializable interface
}
```

**Direct interface assignment**: Concrete types can be directly assigned to interface type variables
(structural subtyping)

```yaoxiang
// Direct assignment (compile-time determinable concrete type -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (compile-time undeterminable -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

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

Generic parameters are part of the function type, using the unified `()` syntax with regular
parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` indicates the return type:

````yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
``

### 4.1.1 Container Types (#299)

Container types are generic type constructors, not built-in primitives—they are treated the same as user-defined generics, processed via the unified generic instantiation path:

| Type | Semantics | Underlying |
| --- | --- | --- |
| `List(T)` | Growable list | `HeapValue::List` |
| `Array(T, N)` | Fixed-length array (const generic N) | `HeapValue::Array` |
| `Dict(K, V)` | Key-value mapping | `HeapValue::Dict` |

> Set(T) has been removed (decision 4 of #300): no literals, no runtime representation, no std.set. When requirements arise, complete it following the Dict pattern.

Key rules:

- **Literal landing point is determined by context**: The bare literal `[...]` and `List(T)` annotation land on the growable list; the `Array(T, N)` annotation directly applied to a literal lands on the fixed-length array. Landing-point validation (#300): number of elements == N, element type compatible with T; otherwise compile-time E1002. When N is a symbolic constant (const parameter), the element-count check is deferred to the refinement type stage.
- **Implicit List→Array conversion is forbidden**: Fixed-length property is guaranteed at the type level—push only accepts `List(A)` receiver.
- **Indexing failure contract** (runtime errors as a transitional state, target state is compile-time refinement coverage, going through value-dependent types):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: Returns `Bool` without error, right operand covers List/Array/Dict(key)/Tuple/String/Range. First-class Hoare predicate, the basis of propositions that the refinement type system can prove at compile time.`

In generic functions, type parameters are also declared in the signature, and the compiler automatically infers them from actual arguments:

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
    push: (self: List(T), item: T) -> Void,   // self is just a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a construction parameter, the field name is the parameter name; fields with
default values can be omitted at construction time, fields without default values are required.
Function-type fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value -> construction parameter is required
    extra: T,
}
// Automatically expanded full form (compiler's internal view, not required to write by hand):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Calls: Calling the auto-generated constructor
c  = Container(42, 43)            // Construction parameters filled by field order; T auto-unwrapped from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type parameter + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // Field-name style, any order
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned later)

// Field default value -> construction parameter can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, matching by declared parameters position by position, from left
to right):

1. Actual arguments attempt to match declared parameter positions one by one: `Type` positions
   accept type arguments, compile-time value parameter positions (e.g., `Int`) accept compile-time
   constants.
2. If some compile-time value parameter positions match successfully (partial match), proceed as
   type construction: check all parameter positions one by one, when reporting errors, report **the
   first mismatched/missing parameter in declaration order**.
3. If actual arguments do not correspond to declared parameters at all (all are values, no
   compile-time value parameter positions to match), proceed as construction parameters: positional
   style fills by field order, type parameters are auto-unwrapped from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one-layer type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two layers: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 doesn't match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols missing -> first error reported first: T expects Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: Type parameters of a generic type constructor are auto-unwrapped from
construction parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions
are auto-unwrapped from actual argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When
unwrapping is not possible, they must be explicitly filled.

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

### 5.2 Multiple Constraint

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

// Using associated type
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

> **Erratum (#296)**: The original text states that compile-time value parameters are "determined at
> compile time by default"; this statement is **strictly incorrect**——in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters. Only **specific type
> parameters referenced in type position** are compile-time value parameters. The correct definition
> is below.

**Terminology**: Generic parameters annotated with a specific type other than `Type` (such as `Int`)
are called **compile-time value parameter candidates**; whether they become compile-time value
parameters depends on whether their values are referenced in type position (value dependency). **No
`const` keyword needed** (the implementation internally used the term "const generic" to refer to
this, but the documentation uniformly uses "compile-time value parameter").

**Determination rules (two steps)**:

1. **Shape pre-screening**: Parameters annotated with specific types other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Usage refinement**: The candidate name appears in a **type position** (type body field type,
   inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction actual argument
   position) → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing                                                    | Determination                             | Reason                                                 |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear in value position                          |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N is in the type construction actual argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N acts as the type of inner parameter k                |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in type body                       |

**Core design**: Using `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Fall-through candidates (shape is candidate,
usage does not match) degrade to runtime value parameters—the function level has already been
handled this way, the type constructor path is in
[issue #297](https://github.com/ChenXu233/YaoXiang/issues/297).

```yaoxiang
// Compile-time value parameter: N is referenced in type position (Array length slot)
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction actual argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at type position (compile time), result 120 is embedded in type
arr: StaticArray(Int, factorial(5))  // Compiler evaluates factorial(5) = 120 at compile time

// Value dependency: N acts as the type of inner parameter k
// N is a compile-time value parameter (appears in (k: N)'s type position);
// k is a runtime value parameter, whose type is literal type N (single-value type).
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
// IsTrue bridge and Assert refinement type (see §8.3 for details)
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

### 8.3 Assert Refinement Type and assert Statement

`assert` and `Assert` are two sides of the same refinement primitive—automatically chosen by the
dispatch pipeline based on "whether the predicate's free variables are compile-time reachable".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                              | Mode        | Behavior                                                                                  |
| -------------------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------------- |
| All free variables are compile-time known (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never is uninhabited) |
| Runtime free variables exist (function parameters, external inputs)                    | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ   |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions for each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (kill set).
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
// Platform type enum (standard library definition)
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

YaoXiang has only one type property that needs to be distinguished: linear vs. copyable.
Automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, passing parameters, and returning =
ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assignment of Dup types = shallow copy—copy
the handle/token, the underlying data is shared. Multiple holders point to the same block of data.

| Type            | Property | Description                                                                      |
| --------------- | -------- | -------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token, copying the token = multiple views point to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, share heap data                                |
| `&mut T`        | Linear   | Zero-sized write token, exclusive, cannot be copied                              |
| All other types | Move     | Default ownership transfer                                                       |

**Primitive value types** (Int, Float, Bool, Char) are special handling built into the compiler:
when assigned, the value is automatically copied, and the two values are completely independent.
This is a native behavior of the compiler and does not belong to the Dup type property.

```yaoxiang
// &T: Dup, can freely alias
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // Usable
print(view2.x)   // Usable

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing a `.clone()`
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

|                         | Dup                                                    | Clone                                         |
| ----------------------- | ------------------------------------------------------ | --------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, share underlying data | Deep copy: create a complete independent copy |
| **Call method**         | Implicit (automatic on assignment/passing parameters)  | Explicit (`.clone()`)                         |
| **Modification effect** | Affects each other (share underlying data)             | Does not affect each other (independent copy) |
| **Applicable types**    | `&T` token, `ref T`                                    | Any type that implements the Clone interface  |
| **Cost**                | Zero overhead (tokens are zero-sized types)            | Depends on the type                           |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data is shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable, seeing the same data

// Primitive value type: compiler automatically copies the value (not Dup)
x: Int = 42
y = x               // Value copy, x and y are completely independent
print(x)            // Usable

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views of the same data"
- Clone is used for scenarios requiring independent copies, with the explicit call making the cost
  visible
- The copy of primitive value types (Int/Float/Bool/Char) is a compiler built-in behavior and does
  not belong to Dup
- Most custom types are Move by default, with zero-copy high performance

## Chapter 12: Borrow Token Type

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references", but
"type-level proofs of access permission".

```
&T      →  Zero-sized, freezes the source data (forbids WriteToken from being obtained during this period),
          multiple copies are safe to read-only under the freezing guarantee → Dup (copyable)
&mut T  →  Zero-sized, exclusive read and write (forbids any other token),
          copying is meaningless under exclusive access → Linear (non-Dup)
```

**Key features**:

- Tokens are **ordinary types**, following the same scope rules as all other types
- No lifetime annotation `'a` needed
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally derive permissions
- Completely disappear after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method end: declare parameter types, determine the required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call end: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates &Point token
p.shift(1.0, 1.0)               // Compiler automatically creates &mut Point token
p.print()                       // OK, the previous token has been released as the shift call ends

// Multiple &T tokens coexist——Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations of ordinary types:

**Return tokens**——tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-token and parent token are returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Token returned to the caller
print(px_ref)                    // OK, token is still in scope
```

**Stored in struct**——structs can carry token fields:

```yaoxiang
// ✅ Struct carries token as a field
Window: Type = {
    target: Point,
    view: &Point,              // Token field——holds a read-only view of target
}
```

**Closures do not capture, context is fixed at creation point**——closures only eat their own
parameters; when outer data is needed, the value is fixed into the closure at the creation point
through currying:

```yaoxiang
// ✅ Context fixed through currying: threshold is a parameter, gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition point may have died,
> so it must not implicitly capture outer variables; but the call point (creation point) scope is
> necessarily alive, and it is safe for the context to be fixed as a value into the closure at this
> point.

### 12.4 Automatic Borrow Selection

The call-end compiler automatically selects according to the following priorities:

```
1. If the actual argument is used later → prefer creating a token (&T or &mut T, depending on the method signature)
2. If the actual argument is no longer used later → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // No further use → Move
```

### 12.5 Token Conflict Detection

Token conflict detection is a **borrowing Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrowing propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be active simultaneously
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
    p.x = 10.0                   // ✅ WriteToken is still available
}

// ❌ The same actual argument cannot create &mut token and other tokens simultaneously
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives &mut and & tokens simultaneously
```

### 12.6 Compiler Internals: Brand Mechanism

Users never touch brands. The compiler internally assigns a compile-time unique identifier to each
token:

```
User sees              Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-counterfeiting**: Tokens can only be obtained from the owner's capsule, and cannot be
  constructed out of thin air
- **Association tracking**: `&Float` derived from field access carries a derived brand
  (`#N.field_x`), the compiler can trace it back to the parent token
- **Conflict detection**: Source WriteToken and derived ReadToken cannot be active simultaneously

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read and write → Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                               | `ref`                                    |
| --------------- | ------------------------------------------------------------- | ---------------------------------------- |
| What it does    | Take a look / modify in place                                 | Shared ownership                         |
| Range           | With the scope of the token value                             | Cross-scope                              |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler chooses)             |
| Escape          | Yes (token propagates with return value/struct)               | Originally for escaping                  |
| Cross-task      | No (tokens not implemented for cross-task passing)            | Yes (compiler automatically chooses Arc) |
| Cycle detection | Not involved                                                  | Silent within task, lint across tasks    |

> Note (undefined): After `ref` is created, how to read the content (dereference/method/automatic)
> is not yet defined in the specification; the current implementation reports E1052 for `*a`. To be
> added after definition.

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

// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implement Serializable interface
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

// Compile-time generics: N is referenced in type position (k: N) -> compile-time value parameter
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
// All types default to Move. Assignment, passing parameters, and returning = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // When assigned, the value is automatically copied, and the two values are completely independent
Bool, Char      // Not Dup, but the compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-sized read token, copying the token = multiple views point to the same data
ref T           // Rc/Arc copy = reference count +1, share heap data

// === Linear ===
&mut T          // Zero-sized write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // Create an independent copy, modifications do not affect the original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow token ===
&T              // Zero-sized compile-time read token, freezes source data → Dup (copyable)
&mut T          // Zero-sized compile-time write token, exclusive read and write → Linear (cannot be copied)

// Call-end automatic selection
// 1. Actual argument used later → create token
// 2. Actual argument no longer used later → Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in struct, captured by closure
// ❌ Cannot cross tasks (tokens not implemented for cross-task passing)
```
