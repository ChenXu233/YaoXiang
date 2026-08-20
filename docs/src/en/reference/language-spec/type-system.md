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
| True \(\top\)                                  | `Void` (Unit, with default value)               |
| False \(\bot\)                                 | `Never` (zero constructors, no inhabitant)      |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checks.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (e.g., `Add` with case analysis + recursive calls on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checks.
- **Type checking is verifying proofs**. When a program passes type checking, it is equivalent to a
  logical proposition being constructively proven.

### 0.3 Impact on Language Design

Specific manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type family** (RFC-011): Type-level case analysis + recursive calls on natural numbers
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checks
3. **Conditional type** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent type** (RFC-011): `Vec: (n: Int) -> Type` corresponds to "for each integer n
   there exists a type" finite quantification

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

> **Design Note**: Although RFC-010 proposes a unified "everything is assignment" model
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), and `TypeExpr` serves as a BNF placeholder corresponding to the `Type` enum in the
> implementation, indicating "a type is expected at this position."

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                              | Default Size |
| -------- | ---------------------- | ---------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                | 0 bytes      |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no values. Diverging/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)          | Has a default void value, zero-field product type. `x: Void = <default>` is valid.       | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                          | 1 byte       |
| `Int`    | —                      | Signed integer                                                                           | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                         | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                    | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                             | Variable     |
| `Char`   | —                      | Unicode character                                                                        | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                | Variable     |

Integers with bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Floats with bit width:
`Float32`, `Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤) respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing to write on the right side.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which code passes type checking (although it will never be executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch conflation.

`Never` is a built-in type name (registered with the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity element of zero-field product types. `x: Void = <default>` is valid, and functions without
an explicit `return` return `Void` by default.

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
- Field name is followed directly by a colon and the type
- Interface names written inside the type body indicate implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call
> syntax like `p.draw()` work, explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and
> RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, which are optional when constructing:

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

#### 3.1.2 Builtin Binding

Methods can be bound directly inside the type definition body:

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

**Interface Implementation**: A type implements an interface by listing the interface name at the
end of its definition

```yaoxiang
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implements Drawable interface
    Serializable     // Implements Serializable interface
}
```

**Direct Interface Assignment**: Concrete types can be directly assigned to interface type variables
(structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile-time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile-time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Look up method through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time Optimization Strategy**:

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

Generic parameters are part of the function type and use the same `()` syntax as regular parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` indicates the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

In generic functions, type parameters are also declared in the signature, and the compiler
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
    push: (self: List(T), item: T) -> Void,   // self is just a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a constructor parameter, the field name is the parameter name; fields with
default values can be omitted at construction, fields without default values are required.
Function-type fields (methods) do not generate constructor parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // No default value → constructor parameter required
    extra: T,
}
// Auto-expanded full form (compiler's internal view, users don't need to write this):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invoke the auto-generated constructor
c  = Container(42, 43)            // Constructor parameters filled in field order; T auto-unpacked from element = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type parameters + positional constructor parameters
c4 = Container(Int)(extra=43, value=42)  // Field name form, order arbitrary
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned later)

// Field default values → constructor parameters can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Calling rules** (single parentheses, positional matching by declared parameters, left to right):

1. Actual arguments attempt to match declared parameters positionally: `Type` positions accept type
   arguments, compile-time value parameter positions (e.g., `Int`) accept compile-time constants.
2. If any compile-time value parameter position matches successfully (partial match), treat as type
   construction: check all parameter positions, report the first mismatch/missing parameter in
   declaration order when an error occurs.
3. If actual arguments do not correspond to declared parameters at all (all are values, no
   compile-time value parameter positions matchable), treat as constructor parameters: fill by field
   order positionally, type parameters auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one layer of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two layers: type + constructor parameters
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ Position 0: T←42 mismatch (42 is not a type); Position 1: Rows←42 matches;
              //    Position 2: Cols missing → report first error: T expects Type, found 42
Container(42) // ❌ Missing constructor parameter extra
Container(42, 43, 44)  // ❌ Too many constructor parameters
```

**Type inference**: Type parameters of generic type constructors are auto-unpacked from constructor
parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions are
auto-unpacked from actual argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). Must be
specified explicitly when unpacking is not possible.

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

> **Erratum (#296)**: The original text stated that compile-time value parameters are "default
> compile-time determined", which is **strictly incorrect**— In
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters. Only **concrete type
> parameters referenced at type positions** are compile-time value parameters. See the correct
> definition below.

**Terminology**: Generic parameters annotated with concrete types other than `Type` (e.g., `Int`)
are called **compile-time value parameter candidates**; whether they become compile-time value
parameters depends on whether their values are referenced at type positions (value dependency). **No
`const` keyword needed** (the implementation internally used "const generics" to refer to this, but
the documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Form coarse filtering**: Parameters annotated with concrete types other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Usage fine filtering**: The candidate name appears at a **type position** (field types in type
   body, inner `Fn` parameter types, `Assert` predicates, `Array(T, N)` type construction argument
   positions) → true compile-time value parameter; otherwise **runtime value parameter**.

| Writing                                                    | Determination                             | Reason                                      |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear at value positions              |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is compile-time value parameter         | N is in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is compile-time value parameter         | N serves as type of inner parameter k       |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in type body            |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Falling-through candidates (form is
candidate, usage not matched) degrade to runtime value parameters—function-level handling is in
place, type constructor path see [issue #297](https://github.com/ChenXu233/YaoXiang/issues/297).

```yaoxiang
// Compile-time value parameter: N is referenced at type position (Array length slot)
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears at type construction argument position → compile-time value parameter
    length: N
}

// Usage: factorial(5) is evaluated at type position (compile-time), result 120 is embedded in type
arr: StaticArray(Int, factorial(5))  // Compiler computes factorial(5) = 120 at compile-time

// Value dependency: N serves as type of inner parameter k
// N is compile-time value parameter (appears at type position of (k: N));
// k is runtime value parameter, whose type is literal type N (single-value type).
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
    false => Never,    // ⊥, diverge/compile error
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

### 8.3 Assert Refinement Type and assert Assertion

`assert` and `Assert` are two sides of the same refinement primitive—automatically chosen by the
dispatch pipeline based on whether the predicate's free variables are compile-time accessible.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                      |
| ------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------- |
| All free variables known at compile-time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erase to Void, false → compile error (Never cannot be inhabited) |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ       |

**Flow-sensitive Assumption Set Γ**:

The compiler maintains a set of known propositions at each control flow point:

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

**Syntax**: Type intersection `A & B` represents the type that satisfies both A and B

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

YaoXiang has only one type property to distinguish: linear vs copyable. Automatically inferred by
the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership
transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assignment of a Dup type = shallow copy—copy
the handle/token, the underlying data is shared. Multiple holders point to the same data block.

| Type            | Property | Description                                                                        |
| --------------- | -------- | ---------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-size read token, copying the token = multiple views pointing to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count + 1, shared heap data                                |
| `&mut T`        | Linear   | Zero-size write token, exclusive, cannot be copied                                 |
| All other types | Move     | Default ownership transfer                                                         |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: automatically
value-copied on assignment, the two values are completely independent. This is the compiler's native
behavior, not a Dup type property.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy token, both are valid
print(view.x)    // Usable
print(view2.x)   // Usable

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and its Relationship to Dup

**Clone** is the explicit deep copy interface. All types can implement Clone, providing the
`.clone()` method.

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

**Differences between Dup and Clone**:

|                         | Dup                                                     | Clone                                       |
| ----------------------- | ------------------------------------------------------- | ------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create complete independent copy |
| **Invocation**          | Implicit (auto on assignment/parameter passing)         | Explicit (`.clone()`)                       |
| **Modification effect** | Mutually affect (share underlying data)                 | Independent (separate copies)               |
| **Applicable types**    | `&T` token, `ref T`                                     | Any type implementing Clone interface       |
| **Cost**                | Zero overhead (token is zero-size type)                 | Depends on type                             |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy token, both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable, seeing the same data

// Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x               // Value copy, x and y are completely independent
print(x)            // Usable

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the "multiple views of the same data" problem
- Clone is used for scenarios that need independent copies, explicit invocation makes cost visible
- Primitive value types (Int/Float/Bool/Char) copying is compiler built-in behavior, not Dup
- Most custom types default to Move, zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references", but
"type-level proof of access permission".

```
&T      →  Zero size, freezes source data (prevents WriteToken acquisition during this period),
          multiple read-only accesses are safe under freeze guarantee → Dup (copyable)
&mut T  →  Zero size, exclusive read/write (prohibits any other token),
          copying is meaningless under exclusive access → Linear (non-Dup)
```

**Key characteristics**:

- Tokens are **ordinary types**, following the same scoping rules as all other types
- No lifetime annotation `'a` required
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally derive permissions
- Completely disappear after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method end: declare parameter types, determine required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call end: compiler automatically selects borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates &Point token
p.shift(1.0, 1.0)               // Compiler automatically creates &mut Point token
p.print()                       // OK, the previous token was released when shift call ended

// Multiple &T tokens coexist—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all ordinary type operations:

**Return tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-token and parent token return together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Token returns to caller
print(px_ref)                    // OK, token is still in scope
```

**Store in struct**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,              // Token field—holds read-only view of target
}
```

**Closures do not capture, context is fixed at creation point**—closures only consume their own
parameters; when outer data is needed, the value is fixed into the closure at creation point through
currying:

```yaoxiang
// ✅ Context fixed through currying: threshold is parameter, gt_point(threshold) fixes the value into the closure at creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition location may have
> died, so it must not implicitly capture outer variables; However, the call point (creation point)
> scope is guaranteed to be alive, so fixing context into the closure as a value at that point is
> safe.

### 12.4 Automatic Borrow Selection

The compiler at the call end automatically selects based on the following priority:

```
1. If the actual argument is used afterward → prefer to create token (&T or &mut T, depending on method signature)
2. If the actual argument is not used afterward → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // Not used afterward → Move
```

### 12.5 Token Conflict Detection

Token conflict detection is the **Borrow Hoare Proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) into the proof pipeline for
verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a §Reverse BFS
Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be alive simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ Token is automatically released after scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Use &mut Point
    }
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken is still available
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
User sees              Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand purposes:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Association tracking**: Field-access-derived `&Float` carries a derived brand (`#N.field_x`),
  the compiler can track it to the parent token
- **Conflict detection**: Same-source WriteToken and derived ReadToken cannot be alive
  simultaneously

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freeze source data → Dup safe)
               | &mut T      // WriteToken (exclusive read/write → Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                              | `ref`                                |
| --------------- | ------------------------------------------------------------ | ------------------------------------ |
| What it does    | Take a look / modify in place                                | Shared ownership                     |
| Range           | Follows token value scope                                    | Cross-scope                          |
| Cost            | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler selects)         |
| Escape          | Possible (token propagates with return value/struct)         | Designed for escape                  |
| Cross-task      | Not possible (tokens not implemented for cross-task passing) | Possible (compiler auto-selects Arc) |
| Cycle detection | Not involved                                                 | Silent within task, lint cross-task  |

> Note (undefined): How to read content (dereference/method/auto) after ref creation is not yet
> defined in the spec, Current implementation `*a` reports E1052. To be added to this section after
> definition.

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

// Type implementing interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implements Serializable interface
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

// Compile-time generic: N is referenced at type position (k: N) → compile-time value parameter
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
Int, Float,     // Auto value copy on assignment, two values are completely independent
Bool, Char      // Not Dup, but compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-size read token, copy token = multiple views point to the same data
ref T           // Rc/Arc copy = reference count + 1, shared heap data

// === Linear ===
&mut T          // Zero-size write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // Create independent copy, modifications do not affect original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow token ===
&T              // Zero-size compile-time read token, freezes source data → Dup (copyable)
&mut T          // Zero-size compile-time write token, exclusive read/write → Linear (not copyable)

// Call end auto-selection
// 1. Actual argument used afterward → create token
// 2. Actual argument not used afterward → Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in struct, captured by closure
// ❌ Cannot cross task (tokens not implemented for cross-task passing)
```
