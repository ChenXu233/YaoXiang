# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between programming language type systems and mathematical logic:

| Logic                                      | Programming Language                            |
| ------------------------------------------ | ----------------------------------------------- |
| Proposition \(P\)                          | Type `Type`                                     |
| Proof \(p: P\)                             | Program `x: T = ...`                            |
| Implication \(P \rightarrow Q\)            | Function type `(P) -> Q`                        |
| Conjunction \(P \wedge Q\)                 | Product type `{ a: P, b: Q }`                   |
| Disjunction \(P \vee Q\)                   | Sum type `{ a(P) \| b(Q) }`                     |
| Universal quantifier \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                     |
| True \(\top\)                              | `Void` (Unit, with default value)               |
| False \(\bot\)                             | `Never` (zero constructors, uninhabitable)      |
| Type universe \(Type_n : Type_{n+1}\)      | Universe hierarchy (prevents Russell's paradox) |
| case analysis                              | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as `Add` with case analysis + recursive call on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is verifying proofs**. When a program passes type checking, it is equivalent to a
  logical proposition being constructively proved.

### 0.3 Impact on Language Design

Specific manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradoxes
   (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive call on natural numbers
   `Nat(Zero/Succ)` corresponds to Peano axioms—provided the compiler performs termination checking
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

> **Design Note**: Although RFC-010 proposes a unified model of "everything is assignment"
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), and `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the
> implementation, indicating "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                                   | Default Size |
| -------- | ---------------------- | --------------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                     | 0 bytes      |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no values. Return type for divergence/panic. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)          | Has a default void value, zero-field product type. `x: Void = <default>` is legal.            | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                               | 1 byte       |
| `Int`    | —                      | Signed integer                                                                                | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                              | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                         | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                                  | Variable     |
| `Char`   | —                      | Unicode character                                                                             | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                     | Variable     |

Integers with bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Floats with bit width:
`Float32`, `Float64`.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤) respectively.

**Never (⊥, false/empty type)** — Three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side that can be written.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which the code can pass type checking (though it will never be executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered on the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — Has exactly one inhabitant (the default void value). `Void` is the
identity element of zero-field product types. `x: Void = <default>` is legal; functions without an
explicit `return` return `Void` by default.

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
- Field names are followed directly by a colon and the type
- Interface names written in the type body indicate implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (such as `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call
> syntax like `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004
> and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, which are optional during construction:

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

Methods can be bound directly within a type definition body:

```yaoxiang
// Method 1: Reference to external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bound to position 0
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

**Interface implementation**: A type implements interfaces by listing the interface names at the end
of its definition

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
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategies**:

| Scenario                           | Inferred Result            | Call Method                 |
| ---------------------------------- | -------------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value              | Unknown                    | vtable                      |
| Heterogeneous collection           | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, final statement)**: YaoXiang's interfaces are
structural types (interface = record with all function-type fields), not nominal traits—there is no
"who can implement for whom" attribution issue across crates/modules, so Rust-style orphan rules and
coherence checking have no applicable targets (ruling record in RFC-011 §2.1). The corresponding
guarantee in the structural world is **duplicate implementation rejection**: defining the same
method signature repeatedly on a type causes a compile error (RFC-011a §3, no override; overloading
is legal).

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

In generic type definitions, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` indicates the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics, processed through the unified generic instantiation path:

| Type          | Semantics                            | Underlying         |
| ------------- | ------------------------------------ | ------------------ |
| `List(T)`     | Growable list                        | `HeapValue::List`  |
| `Array(T, N)` | Fixed-length array (const generic N) | `HeapValue::Array` |
| `Dict(K, V)`  | Key-value mapping                    | `HeapValue::Dict`  |

> `Set(T)` has been removed: no literal, no runtime representation, no std.set. When needed,
> complete it following the Dict pattern.

Key rules:

- **Literal destination determined by context**: The bare literal `[...]` combined with a `List(T)`
  annotation falls into the growable list; an `Array(T, N)` annotation applied directly to a literal
  falls into a fixed-length array. Destination validation: number of elements == N, element type
  compatible with T, otherwise compile-time E1002; when N is a symbolic constant (const parameter),
  the count check is deferred to the refined-type phase.
- **Implicit List→Array conversion is forbidden**: Fixed-lengthness is guaranteed at the type
  layer—`push` only accepts `List(A)` receivers.
- **Index failure contract** (runtime error is a transitional state, target state is compile-time
  refinement coverage, via value-dependent types):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **`in` membership predicate**: Returns `Bool` without error; the right operand covers
  List/Array/Dict(key)/Tuple/String/Range. A first-class Hoare predicate, the basis for compile-time
  provable propositions in refined types.`

In generic functions, type parameters are similarly declared in the signature, and the compiler
automatically infers them from arguments:

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

### 4.3 Generic Construction Call and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a construction parameter, with the field name as the parameter name; fields
with default values can be omitted during construction, and fields without default values are
required. Function-type fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> construction parameter required
    extra: T,
}
// Automatically expanded full form (compiler's internal view, not required for users to write manually):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: call the automatically generated constructor
c  = Container(42, 43)            // construction parameters filled in field order; T auto-unpacked from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type argument + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // field-name style, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default value -> construction parameter can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, position-by-position matching against declared parameters,
left-to-right):

1. Arguments attempt to match declared parameter positions in order: the `Type` position accepts
   type arguments, and compile-time value parameter positions (e.g., `Int`) accept compile-time
   constants.
2. If a compile-time value parameter position matches successfully (partial match), process as type
   construction: check all parameter positions in order, reporting errors **starting with the first
   mismatched/missing parameter** in declaration order.
3. If the arguments do not correspond to declared parameters at all (all are values, no compile-time
   value parameter position matches), process as construction parameters: fill in field order
   positionally, and type parameters are auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one-level type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two levels: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 doesn't match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols missing -> report the first error: T expected Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: Type parameters of generic type constructors are auto-unpacked from the elements
of construction parameters (`Container(42, 43)` → T=Int); type parameters of generic functions are
auto-unpacked from argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When unpacking is
impossible, explicit filling is required.

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

// Using constraints
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

```yaoxiang
// Multiple constraint syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// Sorting of generic containers
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

> **Erratum**: The original text stated that compile-time value parameters are "determined at
> compile time by default"—this statement is **strictly wrong**. In
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters. Only concrete type
> parameters **referenced in type position** are compile-time value parameters. The correct
> definition is below.

**Terminology**: Generic parameters annotated with concrete types other than `Type` (such as `Int`)
are called **compile-time value parameter candidates**; whether they become compile-time value
parameters depends on whether their values are referenced in type position (value-dependence). **No
`const` keyword is needed** (the implementation once used "const generics" internally; the
documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Shape screening**: Parameters annotated with concrete types other than `Type`
   (`Int`/`Bool`/`Float`) → candidates.
2. **Usage screening**: Candidate names appearing in **type position** (type body field types, inner
   `Fn` parameter types, `Assert` predicates, `Array(T, N)` type construction argument positions) →
   true compile-time value parameters; otherwise **runtime value parameters**.

| Writing                                                    | Determination                             | Reason                                           |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear in value position                    |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N appears in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N serves as the type of inner parameter k        |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N not referenced in type body                    |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Candidates that fall through (shape is a
candidate, usage misses) degrade to runtime value parameters—both function-level and
type-constructor paths are handled this way.

```yaoxiang
// Compile-time value parameter: N is referenced in type position (Array length slot)
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at type position (compile-time), the result 120 is embedded in the type
arr: StaticArray(Int, factorial(5))  // the compiler computes factorial(5) = 120 at compile time

// Value-dependence: N as the type of inner parameter k
// N is a compile-time value parameter (appears in the type position of (k: N));
// k is a runtime value parameter whose type is the literal type N (single-value type).
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
// IsTrue bridging and Assert refined types (see §8.3)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, diverges/compile error
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

### 8.3 Assert Refined Types and assert Assertions

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch pipeline based on "whether predicate free variables are compile-time accessible".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch routing rules**:

| Criterion                                                                              | Mode        | Behavior                                                                                 |
| -------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------- |
| All free variables are compile-time known (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never uninhabitable) |
| Runtime free variables exist (function parameters, external input)                     | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ  |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control flow point:

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

**Syntax**: The type intersection `A & B` represents a type that satisfies both A and B

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using intersection types
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

YaoXiang has only one type property that needs to be distinguished: linear vs copyable.
Automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership
transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p cannot be read anymore
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assignment of Dup types = shallow copy—copy
the handle/token, the underlying data is shared. Multiple holders point to the same data.

| Type            | Property | Description                                                                        |
| --------------- | -------- | ---------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-size read token, copying the token = multiple views pointing to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count + 1, shared heap data                                |
| `&mut T`        | Linear   | Zero-size write token, exclusive, cannot be copied                                 |
| All other types | Move     | Default ownership transfer                                                         |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: they are
automatically value-copied on assignment, with the two values being completely independent. This is
the compiler's native behavior, not part of the Dup type property.

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

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is the explicit deep-copy interface. All types can implement Clone, providing the
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

**The difference between Dup and Clone**:

|                         | Dup                                                     | Clone                                         |
| ----------------------- | ------------------------------------------------------- | --------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create a complete independent copy |
| **Call method**         | Implicit (auto on assignment/parameter passing)         | Explicit (`.clone()`)                         |
| **Modification effect** | Mutually affecting (shared underlying data)             | Mutually independent (independent copy)       |
| **Applicable types**    | `&T` tokens, `ref T`                                    | Any type implementing the Clone interface     |
| **Cost**                | Zero overhead (tokens are zero-size types)              | Depends on the type                           |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

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

- Dup is used for token/reference types, solving the problem of "multiple views looking at the same
  data"
- Clone is used in scenarios that need an independent copy, with explicit calls making the cost
  visible
- The copying of primitive value types (Int/Float/Bool/Char) is the compiler's built-in behavior,
  not part of Dup
- Most custom types default to Move, with zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references", but
"type-level proofs of access permission".

```
&T      →  zero size, freezes the source data (prohibits WriteToken from being obtained during this period),
          under the freeze guarantee multiple read-only accesses are safe -> Dup (copyable)
&mut T  →  zero size, exclusive read/write (prohibits any other token),
          copying is meaningless under exclusive access -> Linear (non-Dup)
```

**Key properties**:

- Tokens are **ordinary types**, following the same scoping rules as all other types
- No lifetime annotation `'a` is needed
- No dedicated borrow checker is needed—the type property (Dup/Linear) naturally derives permissions
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

// Call side: compiler automatically chooses borrow or Move
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

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations on ordinary types:

**Returning tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-token and parent token are returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // tokens returned to the caller
print(px_ref)                    // OK, token still in scope
```

**Stored in structs**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field—holding a read-only view of target
}
```

**Closures do not capture; context is fixed at the creation point**—closures only eat their own
parameters; when outer data is needed, the value is fixed into the closure through currying at the
creation point:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter, gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition may already be dead,
> so it must not implicitly capture outer variables; but the scope at the call point (creation
> point) is guaranteed to be alive, and it is safe to fix the context as a value into the closure at
> that point.

### 12.4 Automatic Borrow Selection

The compiler on the call side automatically selects according to the following priority:

```
1. If the argument is still used afterwards -> prefer to create a token (&T or &mut T, according to the method signature)
2. If the argument is not used afterwards -> Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point -> compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point -> compiler creates &mut Point token
p2 = p             // not used afterwards -> Move
```

**Method receiver follows signature semantics** (erratum 2026-08-30, same as RFC-011a receiver
spelling convention): the receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow
token; by value → Move (consumes the receiver). The borrow token generated at the call point is
released when the call ends (transient, §12.5 interval semantics); the interface's borrow receiver
is explicitly declared by the interface author as `&Self`, and the impl signature must be completely
consistent with the interface after `Self ↦ impl type` substitution (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and feeds them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be active simultaneously
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

// ❌ The same argument cannot simultaneously create &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Brand Mechanism

Users never come into contact with brands. The compiler internally assigns a unique compile-time
identifier to each token:

```
User sees                Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Purpose of brands:

- **Anti-forgery**: tokens can only be obtained from the owner capsule, not constructed out of thin
  air
- **Correlation tracking**: field-access-derived `&Float` carries a derived brand (`#N.field_x`),
  the compiler can track it to the parent token
- **Conflict detection**: same-source WriteToken and derived ReadToken cannot be active
  simultaneously

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken (freezes source data -> Dup safe)
               | &mut T      // WriteToken (exclusive read/write -> Linear)
```

### 12.8 Borrow Tokens vs ref

|                 | `&T` / `&mut T`                                              | `ref`                                    |
| --------------- | ------------------------------------------------------------ | ---------------------------------------- |
| What it does    | Take a look/modify in place                                  | Shared ownership                         |
| Scope           | Follows the scope of the token value                         | Cross-scope                              |
| Cost            | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler selects)             |
| Escape          | Yes (token propagates with return value/struct)              | Originally for escaping                  |
| Cross-task      | No (tokens do not implement cross-task passing)              | Yes (compiler automatically selects Arc) |
| Cycle detection | Not involved                                                 | Silent within task, cross-task lint      |

> Note (undefined): How to read content (dereference/method/auto) after `ref` creation has not yet
> been defined in the specification, the current implementation reports `*a` as E1052. To be added
> to this section after definition.

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

// === Function types ===

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

### A.3 Type Properties Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // auto value copy on assignment, the two values are completely independent
Bool, Char      // not Dup, this is the compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // zero-size read token, copy token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count + 1, shared heap data

// === Linear ===
&mut T          // zero-size write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent copy, modifications do not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // zero-size compile-time read token, freezes source data -> Dup (copyable)
&mut T          // zero-size compile-time write token, exclusive read/write -> Linear (cannot be copied)

// Call-side auto-selection
// 1. Argument still used afterwards -> create token
// 2. Argument not used afterwards -> Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in structs, captured by closures
// ❌ Cannot cross tasks (tokens do not implement cross-task passing)
```
