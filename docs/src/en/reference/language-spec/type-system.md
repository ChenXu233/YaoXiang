# Type System Specification

This document defines the type system specification for the YaoXiang programming language, including primitive types, composite types, generics, and traits.

---

## Chapter Zero: Theoretical Foundations

### 0.1 Curry-Howard Isomorphism

The Curry-Howard isomorphism (Curry-Howard correspondence) is the theoretical foundation of YaoXiang's type system. It reveals the deep correspondence between type systems in programming languages and mathematical logic:

| Logic | Programming Language |
|-------|---------------------|
| Proposition \(P\) | Type `Type` |
| Proof \(p: P\) | Program `x: T = ...` |
| Implication \(P \rightarrow Q\) | Function type `(P) -> Q` |
| Conjunction \(P \wedge Q\) | Product type `{ a: P, b: Q }` |
| Disjunction \(P \vee Q\) | Sum type `{ a(P) | b(Q) }` |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...` |
| Truth \(\top\) | Empty type `{}` |
| Falsity \(\bot\) | `Void` / `Never` |
| Type universe \(Type_n : Type_{n+1}\) | Universe stratification (preventing Russell's paradox) |
| Mathematical induction | Type-level `match` |

### 0.2 Types are Propositions, Programs are Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **A type is a logical proposition**. `Int` is the proposition "an integer exists", and `fn(a: Int, b: Int) -> Int` is the proposition "given two integers, an integer exists".
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to a constructive proof of a logical proposition.
- **Terminating type-level computation corresponds to correct inductive reasoning**. YaoXiang's type families (such as pattern matching on `Add` over `Nat`) are essentially type-level encodings of mathematical induction.

### 0.3 Impact on Language Design

The manifestations of Curry-Howard isomorphism in YaoXiang:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids logical paradoxes (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level pattern matching on natural numbers `Nat(Zero/Succ)` corresponds to inductive proofs under Peano axioms
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to finite quantification of "for each integer n there exists a type"

---

## Chapter One: Type Classification

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

---

## Chapter Two: Primitive Types

### 2.1 Primitive Types

| Type | Description | Default Size |
|------|-------------|--------------|
| `Type` | Meta type | 0 bytes |
| `Void` | Void | 0 bytes |
| `Bool` | Boolean | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating-point | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | Raw bytes | Variable |

Width-specified integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Width-specified floats: `Float32`, `Float64`

---

## Chapter Three: Composite Types

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
- Field names are directly followed by a colon and type
- Interface names written inside the type body indicate implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function belongs to `Point`'s namespace.
> It does not trigger any implicit binding. For `.` call syntax like `p.draw()` to work, explicit binding is required:
> `Point.draw = draw[0]`.
> See RFC-004 and RFC-010 for details.

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
Point2(x=1, y=2) // OK
Point2()          // Error
```

**Rules**:
- `field: Type = expression` -> has default, optional during construction
- `field: Type` -> no default, required during construction

#### 3.1.2 Builtin Bindings

Methods can be bound directly within a type definition body:

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

**Interface implementation**: A type implements interfaces by listing interface names at the end of its definition

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
// Direct assignment (concrete type determinable at compile-time -> zero-cost call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (concrete type unknown at compile-time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup via vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario | Inference Result | Call Method |
|----------|-----------------|-------------|
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

## Chapter Four: Generics

### 4.1 Generic Parameter Syntax

Generic parameters are part of function types, unified with regular parameters using `()` syntax:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In generic type definitions, `(T: Type)` is the parameter signature of the type constructor, and `-> Type` represents the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

In generic functions, type parameters are also declared in the signature, and the compiler infers them automatically from actual arguments:

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
    push: (self: List(T), item: T) -> Void,   # self is just a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Type Inference

```yaoxiang
// Compiler infers generic parameters automatically
numbers: List(Int) = List(1, 2, 3)  // Compiler infers List(Int)
```

---

## Chapter Five: Type Constraints

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

## Chapter Six: Associated Types

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
    IteratorType: Iterator(T),  // Associated type is also generic
    iter: () -> IteratorType
}
```

---

## Chapter Seven: Compile-Time Generics

### 7.1 Compile-Time Constant Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**Core design**: Use `(n: Int)` generic parameter + `(n: n)` value parameter to distinguish compile-time constants from runtime values.

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

## Chapter Eight: Conditional Types

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

## Chapter Nine: Type Union and Intersection

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

## Chapter Ten: Function Overloading and Specialization

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
// Platform type enum (stdlib definition)
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P is a predefined generic parameter name, representing current compilation platform
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter Eleven: Type Properties

YaoXiang has only one type property to distinguish: linear vs. copyable. It is automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types default to Move semantics. Assignment, parameter passing, and return = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p cannot be read again
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assignment of Dup types = shallow copy—copy the handle/token, share the underlying data. Multiple owners point to the same piece of data.

| Type | Property | Description |
|------|----------|-------------|
| `&T` | Dup | Zero-size read token, copying token = multiple views pointing to same data |
| `ref T` | Dup | Rc/Arc copy = ref count +1, shared heap data |
| `&mut T` | Linear | Zero-size write token, exclusive, non-copyable |
| All other types | Move | Default ownership transfer |

**Primitive value types** (Int, Float, Bool, Char) have special built-in compiler handling: automatic value copy on assignment, two values are completely independent. This is the compiler's native behavior and does not belong to the Dup type property.

```yaoxiang
// &T: Dup, can be freely aliased
view: &Point = &p
view2 = view     // Dup: copy token, both are valid
print(view.x)    // OK
print(view2.x)   // OK

// &mut T: Linear, non-copyable
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and its Relationship with Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing a `.clone()` method.

```yaoxiang
// Clone interface definition (stdlib)
Clone: Type = {
    clone: () -> Clone
}

// Usage
p: Point = Point(1.0, 2.0)
backup = p.clone()    // Deep copy, p is still usable
p2 = p.clone()        // Can clone multiple times
```

**Differences between Dup and Clone**:

| | Dup | Clone |
|---|---|---|
| **Semantics** | Shallow copy: copy handle/token, share underlying data | Deep copy: create complete independent copy |
| **Call method** | Implicit (automatic on assignment/param passing) | Explicit (`.clone()`) |
| **Modification effect** | Mutually affect each other (shared underlying data) | Mutually independent (independent copy) |
| **Applicable types** | `&T` tokens, `ref T` | Any type implementing Clone interface |
| **Cost** | Zero overhead (tokens are zero-size types) | Varies by type |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy token, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy token, both point to the same p
print(view.x)       // OK
print(view2.x)      // OK, seeing the same data

// Primitive value types: compiler automatic value copy (not Dup)
x: Int = 42
y = x               // Value copy, x and y are completely independent
print(x)            // OK

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is not Dup nor a primitive value type
```

**Design intent**:
- Dup is for token/reference types, solving the problem of "multiple views of the same data"
- Clone is for scenarios requiring independent copies, explicit calling makes cost visible
- Primitive value types (Int/Float/Bool/Char) copy is compiler built-in behavior, not Dup
- Most user-defined types default to Move, zero-copy for high performance

## Chapter Twelve: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references" but "type-level proofs of access permissions".

```
&T      →  Zero size, Dup (copyable), grants read-only permission
&mut T  →  Zero size, Linear (non-Dup), grants exclusive read-write permission
```

**Key characteristics**:
- Tokens are **ordinary types**, following the same scope rules as all other types
- No lifetime annotations `'a` needed
- No dedicated borrow checker needed—the type property (Dup/Linear) naturally derives permissions
- Completely disappear after compilation, zero runtime overhead

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

// Caller side: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler auto-creates &Point token
p.shift(1.0, 1.0)               // Compiler auto-creates &mut Point token
p.print()                       // OK, previous token released after shift call ends

// Multiple &T tokens coexist—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, thus supporting all operations of ordinary types:

**Returning tokens**—tokens propagate along with return values:

```yaoxiang
// ✅ Sub-token and parent token return together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Token returned to caller
print(px_ref)                    // OK, token still in scope
```

**Stored in structs**—structs can carry token fields:

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

### 12.4 Automatic Borrow Selection

The caller-side compiler auto-selects based on the following priority:

```
1. If actual argument is used later → Prefer creating token (&T or &mut T, based on method signature)
2. If actual argument is not used later → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → Compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → Compiler creates &mut Point token
p2 = p             // Not used later → Move
```

### 12.5 Token Conflict Detection

The compiler performs **flow-sensitive liveness analysis** on token values, tracking each token's state (live/moved):

```yaoxiang
// ❌ &mut and derived &T cannot be live simultaneously
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ Token auto-releases after scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Uses &mut Point
    }
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken still usable
}

// ❌ Cannot create &mut token and other tokens from the same argument simultaneously
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives both &mut and & tokens simultaneously
```

### 12.6 Compiler Internals: Branding Mechanism

Users never interact with brands. The compiler internally assigns a compile-time unique identifier to each token:

```
What user sees           Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Purpose of brands:
- **Anti-counterfeiting**: Tokens can only be obtained from owner capsules, cannot be凭空 constructed
- **Provenance tracking**: `&Float` derived from field access carries derived brand (`#N.field_x`), compiler can trace back to parent token
- **Conflict detection**: Same-origin WriteToken and derived ReadToken cannot be live simultaneously

Brands completely disappear after monomorphization and inlining; they do not exist in generated machine code. **Zero runtime overhead.**

### 12.7 Token Sum Types

```
&BorrowToken ::= &T          // ReadToken (Dup, copyable)
               | &mut T      // WriteToken (Linear, exclusive)
```

### 12.8 Borrow Tokens vs. ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Glance / mutate in place | Shared ownership |
| Scope | Follows token value's scope | Crosses scopes |
| Cost | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler-selected) |
| Escape | Can (token propagates via return/struct/closure) | Designed to escape |
| Cross-task | Cannot (token does not implement Send) | Can (compiler auto-selects Arc) |
| Cycle detection | Not applicable | Silent within task, lint across tasks |

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
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // Automatic value copy on assignment, two values completely independent
Bool, Char      // Not Dup, compiler built-in handling for primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-size read token, copying token = multiple views pointing to same data
ref T           // Rc/Arc copy = ref count +1, shared heap data

// === Linear ===
&mut T          // Zero-size write token, Linear (exclusive, non-copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Create independent copy, modifications don't affect original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-size compile-time read token, Dup (copyable)
&mut T          // Zero-size compile-time write token, Linear (non-copyable)

// Caller-side auto-selection
// 1. Actual argument used later → Create token
// 2. Actual argument not used later → Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can return, can be stored in structs, can be captured by closures
// ❌ Cannot cross tasks (not implemented Send)
```