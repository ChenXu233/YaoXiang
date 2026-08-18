# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between the type system of a programming language and mathematical
logic:

| Logic                                          | Programming Language                              |
| ---------------------------------------------- | ------------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                       |
| Proof \(p: P\)                                 | Program `x: T = ...`                              |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                          |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                     |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                       |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                       |
| True \(\top\)                                  | `Void` (Unit, with default)                       |
| False \(\bot\)                                 | `Never` (zero constructors, no value can inhabit) |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox)   |
| case analysis                                  | Type-level `match`                                |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  families (such as case analysis + recursive calls on `Nat`) are essentially a type-level encoding
  of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proven.

### 0.3 Impact on Language Design

Concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type family** (RFC-011): Type-level case analysis + recursive calls on the natural number
   `Nat(Zero/Succ)` corresponds to Peano axioms—provided the compiler performs termination checking
3. **Conditional type** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent type** (RFC-011): `Vec: (n: Int) -> Type` corresponds to finite quantification
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
> `ast.rs:25`), where `TypeExpr` serves as a BNF placeholder corresponding to the `Type` enum in the
> implementation, indicating "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Type

| Type     | Logical Correspondence | Description                                                                               | Default Size |
| -------- | ---------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                 | 0 bytes      |
| `Never`  | ⊥ (False/empty type)   | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (True/Unit)          | Has default void value, zero-field product type. `x: Void = <default>` is valid.          | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                           | 1 byte       |
| `Int`    | —                      | Signed integer                                                                            | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                          | 8 bytes      |
| `Float`  | —                      | Floating point number                                                                     | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                              | variable     |
| `Char`   | —                      | Unicode character                                                                         | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                 | variable     |

Integers with bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Floats with bit width:
`Float32`, `Float64`.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to False (⊥) and True
(⊤) respectively.

**Never (⊥, False/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing to write on the right.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which the code can pass type checking (though it will never actually be executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered on the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, True/Unit)** — exactly one inhabitant (default void value). `Void` is the identity
element of the zero-field product type. `x: Void = <default>` is valid, and a function with no
`return` returns `Void` by default.

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
- A field name is directly followed by a colon and a type
- An interface name written inside the type body indicates that the interface is implemented

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function
> belongs to the namespace of `Point`. It does not trigger any implicit binding. To make the `.`
> call syntax such as `p.draw()` effective, an explicit binding is required: `Point.draw = draw[0]`.
> See RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, which are optional when constructing:

```yaoxiang
// Field with default value - optional at construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without default value - required at construction
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
// Method 1: reference external function binding
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
    Drawable,        // implements Drawable interface
    Serializable     // implements Serializable interface
}
```

**Direct interface assignment**: A concrete type can be assigned directly to an interface-typed
variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile-time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: directly calls circle_draw, no vtable

// Function return value (cannot be determined at compile-time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup via vtable

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

Generic parameters are part of the function type, using the same `()` syntax as ordinary parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` denotes the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

In generic functions, type parameters are likewise declared in the signature, and the compiler
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
    push: (self: List(T), item: T) -> Void,   // self is just a convention name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Generic Construction Call and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a constructor parameter, with the field name serving as the parameter name;
fields with default values may be omitted at construction time, while fields without default values
are required. Function-typed fields (methods) do not generate constructor parameters.

```yaoxiang
// Type definition
Container: (T: Type) -> Type = {
    value: T,        // no default value -> constructor parameter required
    extra: T,
}
// The fully expanded form (compiler's internal view, users are not required to write this manually):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invokes the automatically generated constructor
c  = Container(42, 43)            // constructor parameters filled in field order; T is automatically unwrapped from elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // explicit type argument + positional constructor parameters
c4 = Container(Int)(extra=43, value=42)  // by field name, order arbitrary
c5 = Container(Int)()             // empty construction: fields take default/zero values (data assigned later)

// Field default values -> constructor parameters may be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x<-1.5, y<-2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, positional matching of declared parameters, left-to-right):

1. Actual arguments are matched against declared parameters position by position: `Type` positions
   accept type arguments, and compile-time value parameter positions (such as `Int`) accept
   compile-time constants.
2. If a compile-time value parameter position matches successfully (partial match), it is processed
   as a type construction: each parameter position is checked in turn, and on error, the **first
   mismatching/missing parameter in declaration order** is reported.
3. If the actual arguments do not correspond to the declared parameters at all (all values, no
   compile-time value parameter positions matchable), it is processed as construction parameters:
   positional filling follows field order, and type parameters are automatically unwrapped from
   element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // type position: one level of type construction
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // two levels: type + constructor parameters
m3 = Matrix(Int, 3, 4)()          // empty construction (RFC-011 §9.3 pattern, data assigned later)

Matrix(42)    // ❌ pos 0: T<-42 does not match (42 is not a type); pos 1: Rows<-42 matches;
              //    pos 2: Cols missing -> first error reported: T expected Type, found 42
Container(42) // ❌ missing constructor parameter extra
Container(42, 43, 44)  // ❌ too many constructor parameters
```

**Type inference**: Type parameters of generic type constructors are automatically unwrapped from
constructor parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions
are automatically unwrapped from actual argument types (`map(numbers, f)` → T=Int, R=String, see
§4.1). When unwrapping is not possible, explicit filling is required.

---

## Chapter 5: Type Constraint

### 5.1 Single Constraint

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// Interface type definition (as constraint)
Clone: Type = {
    clone: () -> Clone
}

// Using constraints
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraint

```yaoxiang
// Multiple constraint syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// Sorting with a generic container
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

## Chapter 6: Associated Type

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

### 6.2 Generic Associated Type (GAT)

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

### 7.1 Compile-Time Constant Parameter

```
LiteralType   ::= Identifier ':' Int          // compile-time constant
```

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (such as
`Int`) is called a **compile-time value parameter**, determined by default at compile time,
**without requiring the `const` keyword** (the implementation internally once used "const generics",
but the documentation uniformly uses "compile-time value parameter").

**Core design**: Use `(n: Int)` compile-time value parameters + `(n: n)` value parameters to
distinguish compile-time constants from runtime values.

```yaoxiang
// Compile-time factorial: parameters must be literals known at compile time
factorial: (n: Int) -> (n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// Compile-time constant array
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // array size known at compile time
    length: N
}

// Usage
arr: StaticArray(Int, factorial(5))  // compiler computes factorial(5) = 120 at compile time
```

### 7.2 Compile-Time Constant Array

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

## Chapter 8: Conditional Type

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
// IsTrue bridging with Assert refined type (see §8.3 for details)
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

### 8.3 Assert Refined Type and assert Assertion

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch pipeline based on "whether the free variables of the predicate are reachable at compile
time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                 |
| ------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never uninhabitable) |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refined facts into flow-sensitive assumption set Γ     |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (kill set).
At branch confluence, Γ takes the intersection of each branch.

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

YaoXiang has only one type property that needs to be distinguished: linear vs copyable. This is
automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types default to Move semantics. Assignment, parameter passing, and return = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assignment of a Dup type = shallow
copy—copying the handle/token, with the underlying data shared. Multiple holders point to the same
data block.

| Type            | Property | Description                                                                        |
| --------------- | -------- | ---------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-size read token, copying the token = multiple views pointing to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, shared heap data                                 |
| `&mut T`        | Linear   | Zero-size write token, exclusive, cannot be copied                                 |
| All other types | Move     | Default ownership transfer                                                         |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: assignments
automatically perform value copying, with the two values being completely independent. This is
native compiler behavior, not part of the Dup type property.

```yaoxiang
// &T: Dup, free aliasing
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // available
print(view2.x)   // available

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is an explicit deep-copy interface. All types can implement Clone, providing the
`.clone()` method.

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

**Difference between Dup and Clone**:

|                         | Dup                                                            | Clone                                           |
| ----------------------- | -------------------------------------------------------------- | ----------------------------------------------- |
| **Semantics**           | Shallow copy: copy the handle/token, underlying data is shared | Deep copy: create a complete independent copy   |
| **Call method**         | Implicit (automatic on assignment/parameter passing)           | Explicit (`.clone()`)                           |
| **Modification impact** | Affects each other (shared underlying data)                    | Does not affect each other (independent copies) |
| **Applicable types**    | `&T` tokens, `ref T`                                           | Any type that implements the Clone interface    |
| **Cost**                | Zero overhead (tokens are zero-size types)                     | Depends on the type                             |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data is shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // available
print(view2.x)      // available, they see the same data

// Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x               // value copy, x and y are completely independent
print(x)            // available

// Clone: explicit deep copy, create an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still available
r = p               // Move: ownership transfer, because Point is neither Dup nor a primitive value type
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views of the same data"
- Clone is used for scenarios requiring independent copies; explicit calls make the cost visible
- The copying of primitive value types (Int/Float/Bool/Char) is a built-in compiler behavior, not
  part of Dup
- Most custom types default to Move, with zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references" but
"type-level proofs of access permissions".

```
&T      →  Zero-size, freezes the source data (preventing WriteToken from being acquired in the meantime),
          multiple read-only tokens are safe under the freeze guarantee → Dup (copyable)
&mut T  →  Zero-size, exclusive read/write (prohibits any other tokens),
          copying is meaningless under exclusive access → Linear (non-Dup)
```

**Key features**:

- Tokens are **ordinary types**, following the same scoping rules as all other types
- No lifetime annotations `'a` required
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally infer permissions
- Completely disappear after compilation, with zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare parameter types, determining the required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call side: compiler automatically chooses between borrow and Move
p = Point(1.0, 2.0)
p.print()                       // compiler automatically creates &Point token
p.shift(1.0, 1.0)               // compiler automatically creates &mut Point token
p.print()                       // OK, the previous token was released at the end of the shift call

// Multiple &T tokens coexist——Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all ordinary type operations:

**Returning tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-tokens and parent tokens are returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // tokens are returned to the caller
print(px_ref)                    // OK, the token is still in scope
```

**Storing in structs**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries tokens as fields
Window: Type = {
    target: Point,
    view: &Point,              // token field——holds a read-only view of target
}
```

**Closure capture**—closures capture tokens just like any other value:

```yaoxiang
// ✅ Closure captures &Float token (Dup type, freely copyable into the closure)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 Automatic Borrow Selection

The compiler automatically chooses on the call side by the following priority:

```
1. If the actual argument is used subsequently → prefer creating a token (&T or &mut T, based on method signature)
2. If the actual argument is not used subsequently → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // parameter type of print is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // parameter type of shift is &mut Point → compiler creates &mut Point token
p2 = p             // not used subsequently → Move
```

### 12.5 Token Conflict Detection

The compiler performs **flow-sensitive liveness analysis** on token values, tracking the state of
each token (live/moved):

```yaoxiang
// ❌ &mut and derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ normal use of WriteToken
    print(p.y)
}

// ✅ Automatically released after token scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // use &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken still available
}

// ❌ The same actual argument cannot simultaneously create &mut tokens and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives &mut and & tokens simultaneously
```

### 12.6 Compiler Internals: Brand Mechanism

Users never interact with brands. The compiler internally assigns a compile-time unique identifier
to each token:

```
User sees            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Purpose of brands:

- **Anti-counterfeiting**: Tokens can only be obtained from the owner capsule, not constructed out
  of thin air
- **Association tracking**: Field-access-derived `&Float` carries the derived brand (`#N.field_x`),
  allowing the compiler to trace back to the parent token
- **Conflict detection**: A source WriteToken and a derived ReadToken cannot be live simultaneously

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → safe for Dup)
               | &mut T      // WriteToken (exclusive read/write → Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                              | `ref`                                   |
| --------------- | ------------------------------------------------------------ | --------------------------------------- |
| What it does    | Take a look / modify in place                                | Shared ownership                        |
| Scope           | Follows the scope of the token value                         | Across scopes                           |
| Cost            | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler selects)            |
| Escape          | Yes (token propagates via return value/struct/closure)       | Designed for escape                     |
| Cross-task      | No (tokens do not implement cross-task passing)              | Yes (compiler auto-selects Arc)         |
| Cycle detection | Not involved                                                 | Silent within a task, lint across tasks |

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definition

```
// === Record type (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Interface type (curly braces, fields are all functions) ===

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

// Compile-time generics
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
// All types default to Move. Assignment, parameter passing, and return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // assignments automatically perform value copying, with the two values being completely independent
Bool, Char      // not Dup, but built-in compiler handling of primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // zero-size read token, copy token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count +1, share heap data

// === Linear ===
&mut T          // zero-size write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent copy, modifications do not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow token ===
&T              // zero-size compile-time read token, freezes source data → Dup (copyable)
&mut T          // zero-size compile-time write token, exclusive read/write → Linear (cannot be copied)

// Call-side automatic selection
// 1. Actual argument used subsequently → create token
// 2. Actual argument not used subsequently → Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in structs, captured by closures
// ❌ Cannot cross tasks (tokens do not implement cross-task passing)
```
