# Type System Specification

This document defines the type system specification for the YaoXiang programming language, including
primitive types, compound types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Isomorphism

The Curry-Howard correspondence is the theoretical foundation of YaoXiang's type system. It reveals
the deep correspondence between type systems in programming languages and mathematical logic:

| Logic                                          | Programming Language                               |
| ---------------------------------------------- | -------------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                        |
| Proof \(p: P\)                                 | Program `x: T = ...`                               |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                           |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                      |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                        |
| Universal quantification \(\forall x:T. P(x)\) | Generic `(T: Type) -> ...`                         |
| Truth \(\top\)                                 | `Void` (Unit, has default value)                   |
| Falsehood \(\bot\)                             | `Never` (zero constructors, no value can inhabit)  |
| Type universe \(Type_n : Type_{n+1}\)          | Universe stratification (prevents Russell paradox) |
| Case analysis                                  | Type-level `match`                                 |

> **Note**: Type-level `match` is case analysis (classification discussion), not mathematical
> induction. Induction requires type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proof**. YaoXiang's type
  families (such as `Add` on `Nat` with case analysis + recursive calls) are essentially the
  type-level encoding of mathematical induction—provided the compiler performs termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proven.

### 0.3 Impact on Language Design

The specific manifestations of the Curry-Howard isomorphism in YaoXiang:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradoxes
   (Girard paradox) caused by `Type: Type`
2. **Type families** (RFC-011): The type-level case analysis + recursive calls of natural numbers
   `Nat(Zero/Succ)` correspond to Peano axioms—provided the compiler performs termination checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to finite quantification
   of "for each integer n, there exists a type"

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
> (`name: type = value`), at the syntax level, types and values still need to be distinguished. In
> the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`), with `TypeExpr` serving as a BNF placeholder corresponding to the `Type` enum in the
> implementation, indicating "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                               | Default Size |
| -------- | ---------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                 | 0 bytes      |
| `Never`  | ⊥ (false/empty type)   | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (true/Unit)          | Has default void value, zero-field product type. `x: Void = <default>` is legal.          | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                           | 1 byte       |
| `Int`    | —                      | Signed integer                                                                            | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                          | 8 bytes      |
| `Float`  | —                      | Floating point number                                                                     | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                              | Variable     |
| `Char`   | —                      | Unicode character                                                                         | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                 | Variable     |

Integers with bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128` Floating point with bit width:
`Float32`, `Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to false (⊥) and true
(⊤) respectively.

**Never (⊥, false/empty type)** — Three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   and code after it passes type checking (though never actually executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch merging.

`Never` is a built-in type name (same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — Exactly one inhabitant (the default void value). `Void` is the identity
element for zero-field product types. `x: Void = <default>` is legal, and functions without an
explicit `return` default to returning `Void`.

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

- Record types use curly braces `{}` for definition
- Field names are followed directly by a colon and type
- Interface names written in the type body indicate implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. For `.` call syntax like
> `p.draw()` to work, explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and RFC-010
> for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, which are optional when constructing:

```yaoxiang
// Fields with defaults - optional during construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Fields without defaults - required during construction
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

Methods can be directly bound within type definition bodies:

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

### 3.2 Interface Types

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Syntax**: Interfaces are record types where all fields are function types

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

**Interface implementation**: Types implement interfaces by listing interface names at the end of
their definition

```yaoxiang
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implements Drawable interface
    Serializable     // Implements Serializable interface
}
```

**Direct assignment to interface type**: Concrete types can be directly assigned to interface type
variables (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile-time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (concrete type not determinable at compile-time -> vtable call)
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

Generic parameters are part of function types, using the same `()` syntax as regular parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In generic type definitions, `(T: Type)` is the parameter signature of the type constructor, and
`-> Type` represents the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

In generic functions, type parameters are also declared in the signature, and the compiler infers
them automatically from actual arguments:

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 Generic Type Definitions

```yaoxiang
// Basic generic types
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

### 4.3 Type Inference

```yaoxiang
// Compiler automatically infers generic parameters
numbers: List(Int) = List(1, 2, 3)  // Compiler infers List(Int)
```

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
call_twice: (T: Type, F: () -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: (A) -> B, G: (B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
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
    IteratorType: Iterator(T),  // Associated types can also be generic
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Constant Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**Core design**: Use `(n: Int)` generic parameter + `(n: n)` value parameter to distinguish
compile-time constants from runtime values.

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
arr: StaticArray(Int, factorial(5))  // Compiler evaluates factorial(5) = 120 at compile-time
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

// Example: Compile-time branching
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue bridges to Assert for type refinement (see §8.3)
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

### 8.3 Assert Refinement Types and assert Assertions

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch pipeline based on whether "predicate free variables are accessible at compile-time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                       |
| ------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------- |
| All free variables known at compile-time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never cannot be inhabited) |
| Runtime free variables exist (function parameters, external input)                    | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ        |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After `mut` variable assignment, all assumptions involving that variable are removed (kill set).
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

**Syntax**: Type intersection `A & B` represents types that simultaneously satisfy both A and B

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
// Basic specialization: using function overloading (compiler auto-selects)
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

// P is a predefined generic parameter name, representing the current compilation target
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 11: Type Properties

YaoXiang has only one type property that needs distinction: Linear vs Copyable. It is automatically
inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types default to Move semantics. Assignment, passing arguments, returning = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p cannot be read again
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is for reference/token types**. Assignment of Dup types = shallow copy—copy the
handle/token, with underlying data shared. Multiple holders point to the same data.

| Type            | Property | Description                                                                |
| --------------- | -------- | -------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-size read token, copying token = multiple views pointing to same data |
| `ref T`         | Dup      | Rc/Arc copy = refcount increment, shared heap data                         |
| `&mut T`        | Linear   | Zero-size write token, exclusive, non-copyable                             |
| All other types | Move     | Default ownership transfer                                                 |

**Primitive value types** (Int, Float, Bool, Char) are special compiler-builtin handling: assignment
automatically performs value copy, resulting in two fully independent values. This is
compiler-native behavior, not a Dup type property.

```yaoxiang
// &T: Dup, can be freely aliased
view: &Point = &p
view2 = view     // Dup: copy token, both are valid
print(view.x)    // Available
print(view2.x)   // Available

// &mut T: Linear, not copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Relationship with Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing a `.clone()`
method.

```yaoxiang
// Clone interface definition (standard library)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // Deep copy, p still usable
p2 = p.clone()        // Can clone multiple times
```

**Differences between Dup and Clone**:

|                         | Dup                                                     | Clone                                        |
| ----------------------- | ------------------------------------------------------- | -------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, underlying data shared | Deep copy: create complete independent copy  |
| **Invocation**          | Implicit (automatic on assignment/passing)              | Explicit (`.clone()`)                        |
| **Modification effect** | Affects each other (shared underlying data)             | Independent (copies don't affect each other) |
| **Applicable types**    | `&T` tokens, `ref T`                                    | Any type implementing Clone interface        |
| **Cost**                | Zero overhead (tokens are zero-size types)              | Depends on type                              |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy token, both point to same p
print(view.x)       // Available
print(view2.x)      // Available, seeing the same data

// Primitive value types: compiler auto value-copy (not Dup)
x: Int = 42
y = x               // Value copy, x and y are fully independent
print(x)            // Available

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor primitive value type
```

**Design Intent**:

- Dup is for token/reference types, solving the problem of "multiple views of the same data"
- Clone is for scenarios requiring independent copies, explicit invocation makes cost visible
- Primitive value types (Int/Float/Bool/Char) copying is compiler-builtin behavior, not Dup
- Most user-defined types default to Move for zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references" but
"type-level proofs of access permissions".

```
&T      →  Zero size, freezes source data (prevents WriteToken acquisition during this time),
          multi-copy read-safe under freeze guarantee → Dup (copyable)
&mut T  →  Zero size, exclusive read-write (prevents any other token),
          copying is meaningless under exclusive access → Linear (not Dup)
```

**Key characteristics**:

- Tokens are **ordinary types**, following the same scope rules as all other types
- No lifetime annotations `'a` needed
- No dedicated borrow checker needed—the type property (Dup/Linear) naturally derives permissions
- Completely disappear after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare parameter types, determining required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point token grants write permission
    self.y = self.y + dy
}

// Call side: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler auto-creates &Point token
p.shift(1.0, 1.0)               // Compiler auto-creates &mut Point token
p.print()                       // OK, previous token released after shift call ends

// Multiple &T tokens coexisting—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, thus supporting all ordinary type operations:

**Returning tokens**—tokens propagate with return values:

```yaoxiang
// ✅ Child token and parent token return together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Token returned to caller
print(px_ref)                    // OK, token still in scope
```

**Storing in structs**—structs can carry token fields:

```yaoxiang
// ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,              // Token field—holds read-only view of target
}
```

**Closure capture**—closures capture tokens like any value:

```yaoxiang
// ✅ Closure captures &Float token (Dup type, freely copied into closure)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 Auto Borrow Selection

Compiler auto-selects at call site according to the following priority:

```
1. If actual argument still used later → prefer creating token (&T or &mut T, according to method signature)
2. If actual argument not used later → Move
3. Match priority: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // not used later → Move
```

### 12.5 Token Conflict Detection

The compiler performs **flow-sensitive liveness analysis** on token values, tracking each token's
state (live/moved):

```yaoxiang
// ❌ &mut and derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal WriteToken usage
    print(p.y)
}

// ✅ Token auto-released after scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Uses &mut Point
    }
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken still usable
}

// ❌ Cannot create &mut token and other tokens from same argument simultaneously
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Brand Mechanism

Users never interact with brands. The compiler internally assigns a compile-time unique identifier
to each token:

```
User-visible           Compiler internal representation
────────────────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Purpose of brands:

- **Anti-forgery**: Tokens can only be obtained from owner capsules, cannot be凭空 constructed
- **Tracing derivation**: Field access-derived `&Float` carries derived brand (`#N.field_x`),
  compiler can trace back to parent token
- **Conflict detection**: Same-source WriteToken and derived ReadToken cannot be live simultaneously

Brands completely disappear after monomorphization and inlining. The generated machine code contains
no trace of them. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read-write → Linear)
```

### 12.8 Borrow Tokens vs ref

|                 | `&T` / `&mut T`                                              | `ref`                                 |
| --------------- | ------------------------------------------------------------ | ------------------------------------- |
| Purpose         | Glance/modify in place                                       | Shared ownership                      |
| Scope           | Follows token value's scope                                  | Cross scopes                          |
| Cost            | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler chooses)          |
| Escape          | Can (tokens propagate via returns/structs/closures)          | Designed for escaping by default      |
| Cross-task      | Cannot (tokens not implemented for cross-task passing)       | Can (compiler auto-chooses Arc)       |
| Cycle detection | Not applicable                                               | Silent within task, lint across tasks |

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
    Serializable    // Implements Serializable interface
}

// === Function types ===

Adder: Type = (Int, Int) -> Int
```

### A.2 Generic Syntax

```
// Generic types
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Generic functions
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// Type constraints
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated types
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Compile-time generics
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Conditional types
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Property Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, passing, returning = ownership transfer

// === Primitive value types (compiler-builtin) ===
Int, Float,     // Auto value-copy on assignment, two values fully independent
Bool, Char      // Not Dup, compiler builtin handling for primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-size read token, copying token = multiple views pointing to same data
ref T           // Rc/Arc copy = refcount+1, shared heap data

// === Linear ===
&mut T          // Zero-size write token, Linear (exclusive, non-copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Create independent copy, modifications don't affect original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-size compile-time read token, freezes source data → Dup (copyable)
&mut T          // Zero-size compile-time write token, exclusive read-write → Linear (non-copyable)

// Call-side auto selection
// 1. Actual argument still used later → create token
// 2. Actual argument not used later → Move
// 3. Match priority: &T < &mut T < Move

// Token propagation
// ✅ Can return, store in struct, capture in closure
// ❌ Cannot cross tasks (tokens not implemented for cross-task passing)
```
