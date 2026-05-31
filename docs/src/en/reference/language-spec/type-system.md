# Type System Specification

This document defines the type system specification for the YaoXiang programming language, covering primitive types, composite types, generics, and traits.

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
| Disjunction \(P \vee Q\) | Sum type `{ a(P) \| b(Q) }` |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...` |
| Truth \(\top\) | Empty type `{}` |
| Falsity \(\bot\) | `Void` / `Never` |
| Type universe \(Type_n : Type_{n+1}\) | Universe stratification (preventing Russell's paradox) |
| Mathematical induction | Type-level `match` |

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **A type is a logical proposition**. `Int` is the proposition "an integer exists", `fn(a: Int, b: Int) -> Int` is the proposition "given two integers, an integer exists".
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to a constructive proof of a logical proposition.
- **Terminating type-level computation corresponds to correct inductive reasoning**. YaoXiang's type families (such as pattern matching on `Add` over `Nat`) are essentially type-level encodings of mathematical induction.

### 0.3 Impact on Language Design

The specific manifestations of the Curry-Howard isomorphism in YaoXiang:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids logical paradoxes (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level pattern matching on natural numbers `Nat(Zero/Succ)` corresponds to inductive proofs under Peano axioms
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to finite quantification of "for each integer n there exists a type"

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
| `Void` | empty value | 0 bytes |
| `Bool` | Boolean | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating-point | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | Raw bytes | Variable |

Integer types with bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Floating-point types with bit width: `Float32`, `Float64`

---

## Chapter 3: Composite Types

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
- Interface names written within the type body indicate implementation of those interfaces

#### 3.1.1 Field Default Values

Type fields can specify default values, which are optional during construction:

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

**Interface implementation**: Types implement interfaces by listing interface names at the end of their definition

```yaoxiang
// Types implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implement Drawable interface
    Serializable     // Implement Serializable interface
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

| Scenario | Inference Result | Call Method |
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
// Basic generic types
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

### 5.1 Single Constraints

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
// Compile-time factorial: parameter must be a compile-time known literal
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
// Platform type enum (stdlib definition)
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

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

YaoXiang marks types' copy and concurrency semantics through type properties. Type properties are automatically inferred by the compiler and not directly annotated by users.

### 11.1 Dup (Implicit Shallow Copy)

**Dup** (Duplicable) is an implicit shallow copy marker. Types implementing Dup automatically perform shallow copying (bitwise copy) during assignment and parameter passing, where the original and new values are completely independent.

**Dup types**:
| Type | Description |
|------|-------------|
| `Int`, `Int8`..`Int128` | All integer types |
| `Float`, `Float32`, `Float64` | All floating-point types |
| `Bool` | Boolean value |
| `Char` | Unicode character |
| `String` | UTF-8 string (shallow copy) |
| `Bytes` | Raw bytes (shallow copy) |
| `&T` | Read token (see Chapter 12) |

**Non-Dup types** (default Move):
| Type | Description |
|------|-------------|
| `&mut T` | Write token, linear type |
| Most structs | Default Move, unless all fields are Dup |
| Enums (parameterized variants) | If carried data is not Dup, the entire variant is non-Dup |

```yaoxiang
// Dup types: free to copy
a: Int = 42
b = a           // Shallow copy, a is still usable
c = a           // Can copy multiple times

// Non-Dup types (Move default)
p: Point = Point(1.0, 2.0)
q = p           // Move, p is no longer readable
// r = p        // ❌ Compile error: p has been moved
```

**Automatic Dup inference**:
- Primitive types (Int, Float, Bool, Char, String, Bytes) automatically impl Dup
- Structs: automatically impl Dup iff all field types are Dup
- Enum variants: automatically impl Dup iff all carried types in all variants are Dup

### 11.2 Clone (Explicit Deep Copy)

**Clone** is an explicit deep copy trait. All types can impl Clone, providing a `.clone()` method.

```yaoxiang
// Clone trait definition (stdlib)
Clone: Type = {
    clone: (Self) -> Self
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // Deep copy, p is still usable
p2 = p.clone()        // Can clone multiple times
```

### 11.3 Relationship Between Dup and Clone

**Dup implies Clone, but Clone does not imply Dup**:

```
Dup ⇒ Clone (field-wise bitwise copy can implement .clone())
Clone ⇏ Dup (explicit deep copy does not prevent default Move semantics)
```

```yaoxiang
// Dup ⇒ Clone: Int is both Dup and Clone
x: Int = 42
y = x              // Dup: implicit shallow copy
z = x.clone()      // Clone: explicit deep copy (same effect)

// Clone ⇏ Dup: Point can Clone, but defaults to Move
p: Point = Point(1.0, 2.0)
q = p.clone()      // Clone: explicit deep copy, p is still usable
r = p              // Move: ownership transfer, because Point is not Dup
```

**Design intent**:
- Dup is a promise that "this type is cheap/natural to copy"
- Clone is an ability that "I can give you an independent copy"
- Most structs do not automatically impl Dup, keeping Move as default — zero-copy high performance

### 11.4 Send / Sync (Not User-Visible)

**Send** and **Sync** are not user-visible type properties. They are automatically handled by the compiler and the `ref` keyword.

| Property | Meaning | How users trigger it |
|----------|---------|----------------------|
| **Send** | Safe to pass across tasks | Compiler automatically selects Arc when `ref` crosses tasks |
| **Sync** | Safe to share across tasks | Compiler automatically selects Arc when `ref` crosses tasks |

**Automatic inference rules (compiler internals)**:

| Type | Send | Sync | Description |
|------|------|------|-------------|
| Value types (Int, Float, Point...) | Yes | Yes | Value passing is naturally safe |
| `ref T` | Yes | Yes | Compiler automatically selects Rc (single-task) / Arc (cross-task) |
| `&T` / `&mut T` | No | No | Tokens cannot cross task boundaries |
| `*T` | No | No | Raw pointers are single-threaded |

```yaoxiang
// Cross-task sharing: ref automatically handles Send/Sync
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }    // Compiler: cross-task → Arc (Send + Sync)
    spawn { use(data) }    // Compiler: cross-task → Arc (Send + Sync)
}

// Tokens cannot cross tasks (non-Send)
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }   // ❌ Compile error: &T does not impl Send
}
```

**Users don't need to worry about Send/Sync**: The `ref` keyword encapsulates all concurrency safety logic.

---

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references" but "type-level proofs of access permission".

```
&T      →  Zero-sized, Dup (copyable), grants read-only permission
&mut T  →  Zero-sized, Linear (non-Dup), grants exclusive read-write permission
```

**Key characteristics**:
- Tokens are **ordinary types**, following the same scoping rules as all other types
- No lifetime annotations `'a` needed
- No dedicated borrow checker needed — type properties (Dup/Linear) naturally derive permissions
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

// Call site: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates &Point token
p.shift(1.0, 1.0)              // Compiler automatically creates &mut Point token
p.print()                      // OK, previous token has ended with the shift call

// Multiple &T tokens coexist — Dup types allow free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, thus supporting all operations of ordinary types:

**Returning tokens** — tokens propagate along with return values:

```yaoxiang
// ✅ Child tokens and parent tokens return together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Token returned to caller
print(px_ref)                    // OK, token still in scope
```

**Storing in structs** — structs can carry token fields:

```yaoxiang
// ✅ Structs can carry tokens as fields
Window: Type = {
    target: Point,
    view: &Point,              // Token field — holds read-only view of target
}
```

**Closure capture** — closures capture tokens just like any value:

```yaoxiang
// ✅ Closure captures &Float token (Dup type, freely copied into closure)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 Automatic Borrow Selection

The compiler at call sites automatically selects according to the following priority:

```
1. If the actual argument is used later → prefer creating a token (&T or &mut T, based on method signature)
2. If the actual argument is not used later → Move
3. Match priority: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print declares &self → compiler creates &Point token
p.shift(1.0, 1.0)  // shift declares &mut self → compiler creates &mut Point token
p2 = p             // not used later → Move
```

### 12.5 Freeze Mechanism

`&mut T` tokens can be temporarily "frozen" to produce `&T` tokens:

```yaoxiang
modify_and_read: (p: &mut Point) -> Void = {
    p.x = 10.0                   // Use &mut Point to modify
    
    // Freeze &mut, obtain read-only view
    view: &Point = freeze(p)     // p is frozen here
    print(view.x)                // Read through &Point
    print(view.y)
    // view goes out of scope, freeze lifted
    
    p.y = 20.0                   // &mut Point resumes
}
```

`freeze` semantics:
- Accepts `&mut T`, returns `&T`
- During `&T`'s lifetime, the original `&mut T` is unavailable
- After `&T` goes out of scope, `&mut T` automatically resumes
- This is **flow-sensitive liveness analysis** — the compiler tracks token state within function bodies

### 12.6 Token Conflict Detection

The compiler performs **flow-sensitive liveness analysis** on token values, tracking each token's state (active/frozen/moved):

```yaoxiang
// ❌ &mut and derived &T cannot both be active
bad_alias: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)     // p is frozen
    p.x = 10.0                   // ❌ Compile error: WriteToken in frozen state
    print(view.x)
}

// ✅ After freeze lifted, can continue using &mut
good_seq: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)     // p is frozen
    print(view.x)                // Use &T
    // view goes out of scope, freeze lifted
    p.x = 10.0                   // ✅ WriteToken has resumed
}

// ❌ Same actual argument cannot simultaneously create &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.7 Compiler Internals: The Branding Mechanism

Users never see brands. The compiler internally assigns a compile-time unique identifier to each token:

```
User-visible           Compiler internal representation
────────────────────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand uses:
- **Anti-forgery**: Tokens can only be obtained from owner capsules or freeze operations, cannot be constructed out of thin air
- **Provenance tracking**: Field access derives `&Float` carrying derived brand (`#N.field_x`), allowing compiler to trace back to parent token
- **Conflict detection**: Same-origin WriteToken and derived ReadToken cannot both be active

Brands completely disappear after monomorphization and inlining; no tokens exist in the generated machine code. **Zero runtime overhead.**

### 12.8 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken (Dup, copyable)
               | &mut T      // WriteToken (Linear, exclusive)
```

### 12.9 Borrow Tokens vs. ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Glance/modify in place | Shared ownership |
| Scope | Follows token value's scope | Across scopes |
| Cost | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler selects) |
| Escape | Can (token propagates via return/struct/closure) | Designed for escaping anyway |
| Cross-task | Cannot (token does not impl Send) | Can (compiler automatically selects Arc) |
| Cycle detection | Not applicable | Silent within task, lint across tasks |

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
    Serializable    // Implement Serializable interface
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

### A.3 Type Properties Quick Reference

```
// === Dup (implicit shallow copy) ===
// Primitive types are automatically Dup
Int, Float, Bool, Char, String, Bytes   // Dup
&T                                      // Dup (shared read token)

// Non-Dup (default Move)
&mut T                                  // Linear (exclusive write token)
Most structs                            // Move default

// Dup implies Clone (field-wise bitwise copy), but Clone does not imply Dup

// === Clone (explicit deep copy) ===
value.clone()                           // Explicit deep copy

// === Send / Sync (not user-visible) ===
// Automatically handled by ref keyword and compiler
// Value types: Send + Sync
// ref T: Send + Sync (compiler automatically selects Rc/Arc)
// &T / &mut T: Non-Send (cannot cross tasks)
// *T: Non-Send (raw pointers are single-threaded)
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-sized compile-time read token, Dup (copyable)
&mut T          // Zero-sized compile-time write token, Linear (non-copyable)

// Automatic call-site selection
// 1. Actual argument is used later → create token
// 2. Actual argument is not used later → Move
// 3. Match priority: &T < &mut T < Move

// Token propagation
// ✅ Can return, store in structs, capture in closures
// ❌ Cannot cross tasks (does not impl Send)

// Freeze
view: &T = freeze(mut_ref)   // &mut T → &T (&mut T unavailable during freeze)
```