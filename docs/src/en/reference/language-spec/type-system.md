# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between programming language type systems and mathematical logic:

| Logic                                          | Programming Language                            |
| ---------------------------------------------- | ----------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                     |
| Proof \(p: P\)                                 | Program `x: T = ...`                            |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                        |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                   |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                     |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                     |
| True \(\top\)                                  | `Void` (Unit, has a default value)              |
| False \(\bot\)                                 | `Never` (zero constructors, no inhabitant)      |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| Case analysis                                  | Type-level `match`                              |

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

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive calls on natural numbers
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checking
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

> **Design Note**: Although RFC-010 proposes the unified model of "everything is assignment"
> (`name: type = value`), at the syntax level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`); `TypeExpr`, as a BNF placeholder, corresponds to the `Type` enum in the
> implementation, indicating "this position expects a type."

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

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing on the right side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which code passes type checking (though it is never actually executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch convergence.

`Never` is a built-in type name (with the same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity element of the zero-field product type. `x: Void = <default>` is legal; a function with no
`return` by default returns `Void`.

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

// Record type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Rules**:

- Record types use curly braces `{}` for definition
- A field name is directly followed by a colon and a type
- An interface name written inside the type body indicates implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that a function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call
> syntax like `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004
> and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, which are optional during construction:

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

#### 3.1.2 Built-in Bindings

Methods can be bound directly inside the type definition body:

```yaoxiang
// Method 1: Reference an external function
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bind to position 0
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

**Interface Implementation**: A type implements an interface by listing the interface name at the
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

**Direct Interface Assignment**: A concrete type can be directly assigned to an interface type
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-cost call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup via vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time Optimization Strategy**:

| Scenario                   | Inference Result           | Call Method             |
| -------------------------- | -------------------------- | ----------------------- |
| Direct concrete assignment | Concrete type determinable | Direct call (zero cost) |
| Function return value      | Unknown                    | vtable                  |
| Heterogeneous collection   | Multiple types             | vtable                  |

**Coherence and Orphan Rules (Not Applicable, Closure Note)**: YaoXiang's interfaces are structural
types (interface = record with all function-type fields), not nominal traits—there is no "who can
implement for whom" attribution issue across crates/modules, so Rust-style orphan rules and
coherence checks have no applicable subjects (ruling recorded in #46, RFC-011 §2.1). The
corresponding guarantee in the structural world is **duplicate implementation rejection**: duplicate
definitions of the same method signature on a type cause a compile error (RFC-011a §3, no
overriding; overloading is legal). The nominal resolution mechanism of TraitResolver has been
removed along with #46.

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

Generic parameters are part of the function type signature, unified with ordinary parameters using
`()` syntax:

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

Container types are generic type constructors, not built-in primitives—treated on equal footing with user-defined generics, processed through the unified generic instantiation path:

| Type | Semantics | Underlying Representation |
| --- | --- | --- |
| `List(T)` | Growable list | `HeapValue::List` |
| `Array(T, N)` | Fixed-length array (const generic N) | `HeapValue::Array` |
| `Dict(K, V)` | Key-value map | `HeapValue::Dict` |

> `Set(T)` has been removed (decision 4 of #300): no literal, no runtime representation, no std.set. When the need arises, complete it following the Dict pattern.

Key rules:

- **Literal landing point is context-determined**: A bare `[...]` literal with a `List(T)` annotation lands in the growable list; an `Array(T, N)` annotation acting directly on a literal lands in the fixed-length array. Landing-point validation (#300): number of elements == N, element types compatible with T; otherwise compile-time E1002. When N is a symbolic constant (const parameter), element count validation is deferred to the refinement type phase.
- **No implicit List→Array conversion**: Fixed-length property is guaranteed at the type level—push only accepts `List(A)` receiver.
- **Indexing failure contract** (runtime error is a transitional state; target state is compile-time refinement coverage via value-dependent types):
  - Out-of-bounds indexing (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: Returns `Bool` and does not error; the right operand covers List/Array/Dict(keys)/Tuple/String/Range. A first-class Hoare predicate, serving as the foundation for compile-time provable propositions in refinement types.`

In generic functions, type parameters are likewise declared in the signature, and the compiler automatically infers them from the actual arguments:

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
````

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
    push: (self: List(T), item: T) -> Void,   // self is only a conventional name, not a keyword
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
    value: T,        // no default -> required construction parameter
    extra: T,
}
// Automatically expanded complete form (compiler's internal view, users need not write):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Invocation: call the auto-generated constructor
c  = Container(42, 43)            // construction parameters filled by field order; T auto-unpacked from elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type parameter + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // by field name, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default value -> construction parameter can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Invocation rules** (single parentheses, position-by-position matching against declared parameters,
left to right):

1. Actual arguments try to match declared parameters position by position: a `Type` position accepts
   type arguments, while compile-time value parameter positions (e.g., `Int`) accept compile-time
   constants.
2. If a partial match succeeds at a compile-time value parameter position, treat as type
   construction: check all parameter positions; on error, **report the first non-matching/missing
   parameter in declaration order**.
3. If actual arguments do not correspond to declared parameters at all (all are values, no
   compile-time value parameter position matches), treat as construction parameter handling:
   positional filling by field order, with type parameters auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two levels: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 doesn't match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols missing -> first error: T expected Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: For generic type constructors, type parameters are auto-unpacked from
construction parameter elements (`Container(42, 43)` → T=Int); for generic functions, type
parameters are auto-unpacked from actual argument types (`map(numbers, f)` → T=Int, R=String, see
§4.1). When unpacking is impossible, explicit specification is required.

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

> **Erratum (#296)**: The original text stated that compile-time value parameters are "determined at
> compile time by default." This statement is **strictly incorrect**—in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters. Only specific type
> parameters **referenced in a type position** are compile-time value parameters. The correct
> definition is below.

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (such as
`Int`) is called a **compile-time value parameter candidate**; whether it becomes a compile-time
value parameter depends on whether its value is referenced in a type position (value-dependent).
**No `const` keyword is required** (the implementation internally used "const generics" to refer to
this, but the documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Form coarse filtering**: A parameter annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Use fine filtering**: If the candidate name appears in a **type position** (type body field
   type, inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction argument
   position) → true compile-time value parameter; otherwise **runtime value parameter**.

| Notation                                                   | Determination                             | Reason                                             |
| ---------------------------------------------------------- | ----------------------------------------- | -------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear in value positions                     |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N appears in a type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N serves as the type of inner parameter k          |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in the type body               |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Candidates that fall through (form is a
candidate, use does not match) degrade to runtime value parameters—the function level already
follows this, and the type constructor path is tracked in
[issue #297](https://github.com/ChenXu233/YaoXiang/issues/297).

```yaoxiang
// Compile-time value parameter: N is referenced in a type position (Array length slot)
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in a type construction argument position -> compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated in a type position (compile time), the result 120 is embedded in the type
arr: StaticArray(Int, factorial(5))  // compiler computes factorial(5) = 120 at compile time

// Value-dependent: N serves as the type of inner parameter k
// N is a compile-time value parameter (appears in the type position of (k: N));
// k is a runtime value parameter, its type being the literal type N (a single-value type).
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

### 8.3 Assert Refinement Type and assert Assertion

`assert` and `Assert` are two sides of the same refinement primitive—automatically chosen by the
dispatch pipeline based on whether "predicate free variables are reachable at compile time."

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                                 | Mode        | Behavior                                                                                |
| ----------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erase to Void, false → compile error (Never uninhibitable) |
| Runtime free variables exist (function parameters, external input)                        | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains the set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After assignment to a `mut` variable, all assumptions involving that variable are removed (kill
set). On branch convergence, Γ takes the intersection of all branches.

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

## Chapter 11: Type Properties

YaoXiang has only one type property to distinguish: linear vs. copyable. It is automatically
inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types default to Move semantics. Assignment, parameter passing, and return = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is for reference/token types**. Assignment of a Dup type = shallow copy—copying
the handle/token, with the underlying data shared. Multiple holders point to the same piece of data.

| Type            | Property | Description                                                                |
| --------------- | -------- | -------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token; copying the token = multiple views on the same data |
| `ref T`         | Dup      | Rc/Arc copy = ref count + 1, share heap data                               |
| `&mut T`        | Linear   | Zero-sized write token, exclusive, not copyable                            |
| All other types | Move     | Default ownership transfer                                                 |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: assignment
automatically performs a value copy, with the two values being completely independent. This is the
compiler's native behavior and does not belong to the Dup type property.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy the token, both remain valid
print(view.x)    // usable
print(view2.x)   // usable

// &mut T: Linear, not copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relation to Dup

**Clone** is the explicit deep-copy interface. Any type may implement Clone, providing a `.clone()`
method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // deep copy, p remains usable
p2 = p.clone()        // can be cloned multiple times
```

**Difference between Dup and Clone**:

|                      | Dup                                                    | Clone                                         |
| -------------------- | ------------------------------------------------------ | --------------------------------------------- |
| **Semantics**        | Shallow copy: copy handle/token, share underlying data | Deep copy: create a full independent replica  |
| **Invocation**       | Implicit (automatic on assignment/parameter passing)   | Explicit (`.clone()`)                         |
| **Mutation effect**  | Mutually affect each other (share underlying data)     | Do not affect each other (independent copies) |
| **Applicable types** | `&T` tokens, `ref T`                                   | Any type implementing the Clone interface     |
| **Cost**             | Zero overhead (tokens are zero-sized types)            | Depends on the type                           |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // usable
print(view2.x)      // usable, viewing the same data

// Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // usable

// Clone: explicit deep copy, create an independent replica
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p remains usable
r = p               // Move: ownership transfer, because Point is neither Dup nor a primitive value type
```

**Design intent**:

- Dup is for token/reference types, solving the problem of "multiple views on the same data"
- Clone is for scenarios requiring independent copies; explicit invocation makes the cost visible
- The copy behavior of primitive value types (Int/Float/Bool/Char) is the compiler's built-in
  behavior and does not belong to Dup
- Most custom types default to Move, achieving zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references" but
"type-level proofs of access rights."

```
&T      →  zero-sized, freezes source data (prohibits WriteToken acquisition during this period),
          multi-read safety guaranteed under freezing -> Dup (copyable)
&mut T  →  zero-sized, exclusive read-write (prohibits any other token),
          copying is meaningless under exclusive access -> Linear (not Dup)
```

**Key properties**:

- A token is an **ordinary type**, following the same scope rules as all other types
- No lifetime annotation `'a` needed
- No dedicated borrow checker needed—permissions are naturally inferred from type properties
  (Dup/Linear)
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

// Caller side: compiler automatically chooses between borrow and Move
p = Point(1.0, 2.0)
p.print()                       // compiler automatically creates an &Point token
p.shift(1.0, 1.0)               // compiler automatically creates an &mut Point token
p.print()                       // OK, the previous token was released when the shift call ended

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations of ordinary types:

**Returning tokens**—tokens propagate along with return values:

```yaoxiang
// ✅ Sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to the caller
print(px_ref)                    // OK, token still in scope
```

**Storing in structs**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field—holds a read view of target
}
```

**Closures do not capture, context is fixed at the creation point**—a closure only eats its own
parameters; when outer-scope data is needed, curry the value into the closure at the creation point:

```yaoxiang
// ✅ Context fixed via currying: threshold is a parameter; gt_point(threshold) fixes the value into the closure at the creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: A closure (function value) may escape into a context where its definition site is already
> dead, so it may not implicitly capture outer variables; however, the call site (creation point) is
> guaranteed to be live, so fixing context as a value into the closure at that point is safe.

### 12.4 Automatic Borrow Selection

The caller compiler automatically selects by the following priority:

```
1. If the actual argument is used later -> prefer creating a token (&T or &mut T, according to the method signature)
2. If the actual argument is not used later -> Move
3. Priority order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // parameter type of print is &Point -> compiler creates an &Point token
p.shift(1.0, 1.0)  // parameter type of shift is &mut Point -> compiler creates an &mut Point token
p2 = p             // not used later -> Move
```

**Method receiver follows signature semantics** (erratum 2026-08-30, same convention as RFC-011a
receiver spelling convention): receiver is `&T` → read borrow token; `&mut T` → mutable borrow
token; by value → Move (consume the receiver). Borrow tokens generated at the call site are released
when the call ends (transient, see interval semantics in §12.5); an interface's borrow receiver is
explicitly declared as `&Self` by the interface author, and the impl signature must be exactly
consistent with the interface after `Self ↦ impl type` substitution (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and feeds them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot both be live
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ After the token scope ends, it is automatically released
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // uses &mut Point
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

Users never encounter brands. The compiler internally assigns each token a compile-time unique
identifier:

```
User-visible           Compiler-internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: A token can only be obtained from the owner's capsule, not constructed out of
  thin air
- **Association tracking**: Field-access-derived `&Float` carries a derived brand (`#N.field_x`);
  the compiler can trace it to the parent token
- **Conflict detection**: WriteToken and derived ReadToken of the same source cannot both be live

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freeze source data -> Dup safe)
               | &mut T      // WriteToken (exclusive read-write -> Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                             | `ref`                                 |
| --------------- | ----------------------------------------------------------- | ------------------------------------- |
| What it does    | Glance at / modify in place                                 | Shared ownership                      |
| Scope           | With the token value's scope                                | Cross-scope                           |
| Cost            | Zero overhead (zero-sized type, vanishes after compilation) | Rc or Arc (compiler chooses)          |
| Escape          | Allowed (token propagates via return value/struct)          | Designed for escape                   |
| Cross-task      | Not allowed (tokens do not implement cross-task passing)    | Allowed (compiler auto-selects Arc)   |
| Cycle detection | N/A                                                         | Silent within task, lint across tasks |

> Note (undefined): How to read content after `ref` is created (dereference/method/auto) is not yet
> defined in the specification; current implementation reports `*a` as E1052. Will be added to this
> section after definition.

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
    Serializable    // implements Serializable
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

// Compile-time generics: N is referenced in a type position (k: N) -> compile-time value parameter
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
Bool, Char      // not Dup, but the compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // zero-sized read token, copying the token = multiple views on the same data
ref T           // Rc/Arc copy = ref count + 1, share heap data

// === Linear ===
&mut T          // zero-sized write token, Linear (exclusive, not copyable)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent replica, modifications do not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // zero-sized compile-time read token, freezes source data -> Dup (copyable)
&mut T          // zero-sized compile-time write token, exclusive read-write -> Linear (not copyable)

// Caller-side automatic selection
// 1. Actual argument used later -> create token
// 2. Actual argument not used later -> Move
// 3. Priority: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in structs, captured by closures
// ❌ Cannot cross tasks (tokens do not implement cross-task passing)
```
