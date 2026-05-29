# Type System Specification

This document defines the type system specification for the YaoXiang programming language, including primitive types, compound types, generics, and traits.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Isomorphism

The Curry-Howard correspondence is the theoretical foundation of YaoXiang's type system. It reveals the deep correspondence between type systems in programming languages and mathematical logic:

| Logic | Programming Language |
|-------|----------------------|
| Proposition \(P\) | Type `Type` |
| Proof \(p: P\) | Program `x: T = ...` |
| Implication \(P \rightarrow Q\) | Function type `(P) -> Q` |
| Conjunction \(P \wedge Q\) | Product type `{ a: P, b: Q }` |
| Disjunction \(P \vee Q\) | Sum type `{ a(P) | b(Q) }` |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...` |
| Truth \(\top\) | Void type `{}` |
| Falsity \(\bot\) | `Void` / `Never` |
| Type universe \(Type_n : Type_{n+1}\) | Universe stratification (preventing Russell's paradox) |
| Mathematical induction | Type-level `match` |

### 0.2 Types are Propositions, Programs are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **A type is a logical proposition**. `Int` is the proposition "an integer exists", and `fn(a: Int, b: Int) -> Int` is the proposition "given two integers, an integer exists".
- **Type checking is proof verification**. When a program passes type checking, it is as if a constructive proof of a logical proposition has been constructed.
- **Terminating type-level computation corresponds to sound inductive reasoning**. YaoXiang's type families (such as pattern matching on `Add` over `Nat`) are essentially type-level encodings of mathematical induction.

### 0.3 Impact on Language Design

The manifestations of Curry-Howard isomorphism in YaoXiang:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids logical paradoxes (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Pattern matching at the type level on natural numbers `Nat(Zero/Succ)` corresponds to inductive proofs under Peano axioms
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to finite quantification over "for each integer n, a type exists"

---

## Chapter 1: Type Classification

### 1.1 Type Expressions

```
TypeExpr    ::= PrimitiveType
              | StructType
              | EnumType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
              | ConstrainedType
              | AssociatedType
```

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type | Description | Default Size |
|------|-------------|--------------|
| `Type` | Meta type | 0 bytes |
| `Void` | void | 0 bytes |
| `Bool` | boolean type | 1 byte |
| `Int` | signed integer type | 8 bytes |
| `Uint` | unsigned integer type | 8 bytes |
| `Float` | float type | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | raw bytes | Variable |

Width-specified integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Width-specified floats: `Float32`, `Float64`

---

## Chapter 3: Compound Types

### 3.1 Record Types

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
- Field names are followed directly by a colon and type
- Interface names written within the type body indicate implementation of that interface

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
Point2(x=1, y=2) // Correct
Point2()          // Error
```

**Rules**:
- `field: Type = expression` -> has default value, optional during construction
- `field: Type` -> no default value, required during construction

#### 3.1.2 Builtin Bindings

Methods can be bound directly within type definition bodies:

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

### 3.2 Enum Types (Variant Types)

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**Syntax**: `Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// Variants without parameters
Color: Type = { red | green | blue }

// Variants with parameters
Option: (T: Type) -> Type = { some(T) | none }

// Mixed
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Variants without parameters are equivalent to parameterless constructors
Bool: Type = { true | false }
```

### 3.3 Interface Types

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

**Interface Implementation**: A type implements an interface by listing the interface name at the end of its definition

```yaoxiang
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implements Drawable interface
    Serializable     // Implements Serializable interface
}
```

**Direct interface assignment**: Concrete types can be directly assigned to interface type variables (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile-time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (concrete type not determinable at compile-time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario | Inference result | Call method |
|----------|------------------|-------------|
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value | Unknown | vtable |
| Heterogeneous collection | Multiple types | vtable |

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

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

### 4.2 Generic Type Definitions

```yaoxiang
// Basic generic type
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Type Inference

```yaoxiang
// Compiler automatically infers generic parameters
numbers: List(Int) = List(1, 2, 3)  // Compiler infers List(Int)
```

---

## Chapter 5: Type Constraints

### 5.1 Single Constraint

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// Interface type definition (as constraint)
Clone: Type = {
    clone: (Self) -> Self
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

// Sorting generic containers
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 Function Type Constraints

```yaoxiang
// Higher-order function constraints
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## Chapter 6: Associated Types

### 6.1 Associated Type Definitions

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (using record type syntax)
Iterator: (T: Type) -> Type = {
    Item: T,                    // Associated type
    next: (Self) -> Option(T),
    has_next: (Self) -> Bool
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
    iter: (Self) -> IteratorType
}
```

---

## Chapter 7: Compile-Time Generics

### 7.1 Literal Type Constraints

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**Core design**: Using `(n: Int)` generic parameter + `(n: n)` value parameter distinguishes compile-time constants from runtime values.

```yaoxiang
// Compile-time factorial: parameter must be a literal known at compile-time
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// Compile-time constant array
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // Array with compile-time known size
    length: N
}

// Usage
arr: StaticArray(Int, factorial(5))  // Compiler computes factorial(5) = 120 at compile-time
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

## Chapter 8: Conditional Types

### 8.1 If Conditional Types

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
// Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// Example: Compile-time branching
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

// Compile-time verification
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
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

**Syntax**: Type intersection `A & B` represents types that satisfy both A and B

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
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P is a predefined generic parameter name, representing the current compilation platform
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definitions

```
// === Record types (curly braces) ===

// Struct
Point: Type = { x: Float, y: Float }

// Enum (variant types)
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === Interface types (curly braces, all fields are functions) ===

// Interface definition
Serializable: Type = { serialize: () -> String }

// Type implementing interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implements Serializable interface
}

// === Function types ===

Adder: Type = (Int, Int) -> Int
```

### A.2 Generic Syntax

```
// Generic types
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Generic functions
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = { ... }

// Type constraints
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated types
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Compile-time generics
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: T(N), length: N }

// Conditional types
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```