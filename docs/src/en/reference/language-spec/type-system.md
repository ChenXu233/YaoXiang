# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundation

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
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as `Add` doing case analysis + recursive calls on `Nat`) are essentially type-level
  encodings of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is verifying proofs**. When a program passes type checking, it is equivalent to a
  logical proposition being constructively proven.

### 0.3 Impact on Language Design

Concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradoxes
   (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive calls on the natural number
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to bounded
   quantification "for every integer n there exists a type"

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

> **Design note**: Although RFC-010 proposes the "everything is assignment" unified model
> (`name: type = value`), at the syntactic level types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), and `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the
> implementation, indicating "a type is expected at this position."

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                                  | Default Size |
| -------- | ---------------------- | -------------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                    | 0 bytes      |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no values. Return type of divergence/panic. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)          | Has a default void value, zero-field product type. `x: Void = <default>` is legal.           | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                              | 1 byte       |
| `Int`    | —                      | Signed integer                                                                               | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                             | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                        | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                                 | variable     |
| `Char`   | —                      | Unicode character                                                                            | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                    | variable     |

Integers with bit widths: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Floats with bit widths:
`Float32`, `Float64`.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤) respectively.

**Never (⊥, false/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing to write on the right side.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which subsequent code passes type checking (although it is never actually executed).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis and `match` branch merging.

`Never` is a built-in type name (registered on the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — has exactly one inhabitant (the default void value). `Void` is the
identity element of zero-field product types. `x: Void = <default>` is legal, and functions default
to returning `Void` when there is no `return`.

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
- Interface names written inside the type body indicate that the interface is implemented

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. For the `.` call syntax
> such as `p.draw()` to take effect, an explicit binding is required: `Point.draw = draw[0]`. See
> RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values and become optional when constructing:

```yaoxiang
// Fields with default values - optional when constructing
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Fields without default values - required when constructing
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) // correct
Point2()          // error
```

**Rules**:

- `field: Type = expression` -> has default value, optional when constructing
- `field: Type` -> no default value, required when constructing

#### 3.1.2 Builtin Binding

Methods can be bound directly inside the type definition body:

```yaoxiang
// Method 1: reference an external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bind to position 0
}
// Call: p1.distance(p2) -> distance(p1, p2)

// Method 2: anonymous function + positional binding
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

**Direct interface assignment**: A concrete type can be directly assigned to a variable of an
interface type (structural subtyping)

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

| Scenario                        | Inference Result           | Call Method                 |
| ------------------------------- | -------------------------- | --------------------------- |
| Direct concrete type assignment | Concrete type determinable | Direct call (zero overhead) |
| Function return value           | Unknown                    | vtable                      |
| Heterogeneous collection        | Multiple types             | vtable                      |

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

Generic parameters are part of the function type, unified with ordinary parameters using the `()`
syntax:

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

In generic functions, type parameters are also declared in the signature, and the compiler
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
    push: (self: List(T), item: T) -> Void,   // self is just a convention name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor**: each field
corresponds to a construction parameter, and the field name is the parameter name; fields with
default values can be omitted when constructing, while fields without default values are required.
Function-type fields (methods) do not generate construction parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> construction parameter required
    extra: T,
}
// Automatically expanded full form (compiler's internal view, users are not required to write it):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invoke the automatically generated constructor
c  = Container(42, 43)            // construction parameters filled in field order; T automatically unwrapped from elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type parameter + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // field name style, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default values -> construction parameters can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, matching declared parameters position by position, left to
right):

1. The actual arguments try to match the declared parameters position by position: the `Type`
   position accepts type arguments, and compile-time value parameter positions (such as `Int`)
   accept compile-time constants.
2. If some compile-time value parameter positions match successfully (partial match), treat it as
   type construction: check all parameter positions one by one, and **report the first
   mismatching/missing parameter in declaration order** on error.
3. If the actual arguments do not correspond to the declared parameters at all (all are values, no
   compile-time value parameter positions to match), treat as construction parameters: fill in field
   order positionally, and type parameters are automatically unwrapped from the element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // Type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // Two levels: type + construction parameters
m3 = Matrix(Int, 3, 4)()          // Empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ position 0: T←42 doesn't match (42 is not a type); position 1: Rows←42 matches;
              //    position 2: Cols missing -> report the first error: T expected Type, found 42
Container(42) // ❌ missing construction parameter extra
Container(42, 43, 44)  // ❌ too many construction parameters
```

**Type inference**: The type parameters of a generic type constructor are automatically unwrapped
from the construction parameter elements (`Container(42, 43)` → T=Int); the type parameters of a
generic function are automatically unwrapped from the actual argument types (`map(numbers, f)` →
T=Int, R=String, see §4.1). Explicit filling is required when unwrapping is not possible.

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

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Constant Parameters

```
LiteralType   ::= Identifier ':' Int          // compile-time constant
```

**Terminology**: A generic parameter annotated with a concrete non-`Type` type (such as `Int`) is
called a **compile-time value parameter**, determined at compile time by default, **without the need
for a `const` keyword** (the implementation internally used "const generics" to refer to it; the
documentation uniformly uses "compile-time value parameter").

**Core design**: Use `(n: Int)` compile-time value parameter + `(n: n)` value parameter to
distinguish compile-time constants from runtime values.

```yaoxiang
// Compile-time factorial: the parameter must be a literal known at compile time
factorial: (n: Int) -> (n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// Compile-time constant array
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // size known at compile time
    length: N
}

// Usage
arr: StaticArray(Int, factorial(5))  // The compiler computes factorial(5) = 120 at compile time
```

### 7.2 Compile-Time Constant Arrays

```yaoxiang
// Used in matrix types
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

### 8.3 Assert Refinement Type and the assert Statement

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch pipeline based on "whether the free variables of the predicate are reachable at compile
time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                         |
| ------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------ |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter the proof pipeline: true → erased as Void, false → compile error (Never has no inhabitant) |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refinement facts into the flow-sensitive assumption set Γ      |

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

YaoXiang has only one type property to distinguish: linear vs copyable. It is automatically inferred
by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership
transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assigning a Dup type = shallow copy—copy the
handle/token, the underlying data is shared. Multiple holders point to the same data.

| Type            | Property | Description                                                                         |
| --------------- | -------- | ----------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token, copying the token = multiple views pointing to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, sharing heap data                                 |
| `&mut T`        | Linear   | Zero-sized write token, exclusive, non-copyable                                     |
| All other types | Move     | Default ownership transfer                                                          |

**Primitive value types** (Int, Float, Bool, Char) are a special compiler built-in handling: values
are automatically copied on assignment, and the two values are completely independent. This is the
compiler's native behavior and is not a Dup type property.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // available
print(view2.x)   // available

// &mut T: Linear, non-copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and its Relationship to Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing a `.clone()`
method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // Deep copy, p is still available
p2 = p.clone()        // Can be cloned multiple times
```

**The difference between Dup and Clone**:

|                         | Dup                                                     | Clone                                         |
| ----------------------- | ------------------------------------------------------- | --------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create a complete independent copy |
| **Call method**         | Implicit (automatic on assignment/parameter passing)    | Explicit (`.clone()`)                         |
| **Modification impact** | Affects each other (shared underlying data)             | Does not affect each other (independent copy) |
| **Applicable types**    | `&T` tokens, `ref T`                                    | Any type implementing the Clone interface     |
| **Cost**                | Zero overhead (tokens are zero-sized types)             | Depends on the type                           |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy token, both point to the same p
print(view.x)       // available
print(view2.x)      // available, seeing the same data

// Primitive value type: compiler automatically copies the value (not Dup)
x: Int = 42
y = x               // Value copy, x and y are completely independent
print(x)            // available

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still available
r = p               // Move: ownership transfer, because Point is neither Dup nor a primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views of the same data"
- Clone is used in scenarios that require independent copies, making the cost visible through
  explicit calls
- The copy of primitive value types (Int/Float/Bool/Char) is a compiler built-in behavior and is not
  part of Dup
- Most custom types default to Move, with zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references", but
"type-level proofs of access permission".

```
&T      →  Zero-sized, freezes the source data (forbids WriteToken acquisition during this period),
          under freezing guarantee multiple read-only accesses are safe → Dup (copyable)
&mut T  →  Zero-sized, exclusive read/write (forbids any other tokens),
          under exclusive access copying is meaningless → Linear (non-Dup)
```

**Key properties**:

- Tokens are **ordinary types** and follow the same scope rules as all other types
- No lifetime annotation `'a` is required
- No dedicated borrow checker is needed—type properties (Dup/Linear) naturally derive permissions
- They completely disappear after compilation, with zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// On the method side: declare the parameter type to determine the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// On the call side: the compiler automatically chooses to borrow or Move
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

Tokens are ordinary types, so they support all ordinary type operations:

**Returning tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-tokens and parent tokens returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Tokens returned to the caller
print(px_ref)                    // OK, token is still in scope
```

**Storing in struct**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries token as a field
Window: Type = {
    target: Point,
    view: &Point,              // Token field—holds a read-only view of target
}
```

**Closures do not capture, context is fixed at creation point**—closures only consume their own
parameters; when external data is needed, it is fixed into the closure through currying at the
creation point:

```yaoxiang
// ✅ Context fixed through currying: threshold is a parameter, gt_point(threshold) fixes the value into the closure at creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition site may already be
> dead, so it must not implicitly capture outer variables; but the call site (creation point) scope
> is guaranteed to be alive, and it is safe for the context to be fixed into the closure as a value
> at that point.

### 12.4 Automatic Borrow Selection

The compiler on the call side automatically selects based on the following priority:

```
1. If the actual argument is used later → prefer to create a token (&T or &mut T, according to method signature)
2. If the actual argument is not used later → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // Not used later → Move
```

### 12.5 Token Conflict Detection

Token conflict detection is the **Borrowing Hoare proposition** (RFC-009a), not a separate
flow-sensitive analysis. The compiler automatically generates borrowing propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and sends them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be active at the same time
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ Token automatically released after scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // uses &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken still available
}

// ❌ The same actual argument cannot create &mut and other tokens at the same time
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives both &mut and & tokens simultaneously
```

### 12.6 Compiler Internals: Brand Mechanism

Users never come into contact with brands. The compiler internally assigns a compile-time unique
identifier to each token:

```
What the user sees       Compiler's internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, not constructed out of thin
  air
- **Association tracking**: `&Float` derived from field access carries the derived brand
  (`#N.field_x`), and the compiler can trace it to the parent token
- **Conflict detection**: Same-source WriteToken and derived ReadToken cannot be active at the same
  time

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read/write → Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                               | `ref`                                    |
| --------------- | ------------------------------------------------------------- | ---------------------------------------- |
| What it does    | Take a look / modify in place                                 | Shared ownership                         |
| Scope           | With the scope of the token value                             | Across scopes                            |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler choice)              |
| Escape          | Yes (token propagates through return value/struct)            | Designed to escape                       |
| Cross-task      | No (tokens do not implement cross-task passing)               | Yes (compiler automatically chooses Arc) |
| Cycle detection | Not involved                                                  | Silent within task, lint across tasks    |

> Note (undefined): How to read the content after `ref` is created (dereference/method/auto) has not
> been defined in the specification, and the current implementation `*a` reports E1052. To be added
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

// === Interface types (curly braces, fields all functions) ===

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

// Compile-time generic
factorial: (n: Int)(n: n) -> Int = { ... }
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

// === Primitive value types (compiler built-in) ===
Int, Float,     // Values are automatically copied on assignment, the two values are completely independent
Bool, Char      // Not Dup, but the compiler's built-in handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-sized read token, copying token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count +1, sharing heap data

// === Linear ===
&mut T          // Zero-sized write token, Linear (exclusive, non-copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Create an independent copy, modifications do not affect the original value
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token, freezes source data → Dup (copyable)
&mut T          // Zero-sized compile-time write token, exclusive read/write → Linear (non-copyable)

// Automatic selection on the call side
// 1. Actual argument used later → create token
// 2. Actual argument not used later → Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in structs, captured by closures
// ❌ Cannot cross tasks (tokens do not implement cross-task passing)
```
