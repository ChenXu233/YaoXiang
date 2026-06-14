# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including primitive types, compound types, generics, and traits.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It reveals the deep correspondence between programming language type systems and mathematical logic:

| Logic | Programming Language |
|--------|----------|
| Proposition \(P\) | Type `Type` |
| Proof \(p: P\) | Program `x: T = ...` |
| Implication \(P \rightarrow Q\) | Function type `(P) -> Q` |
| Conjunction \(P \wedge Q\) | Product type `{ a: P, b: Q }` |
| Disjunction \(P \vee Q\) | Sum type `{ a(P) \| b(Q) }` |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...` |
| Truth \(\top\) | Empty type `{}` |
| Falsity \(\bot\) | `Void` / `Never` |
| Type universe \(Type_n : Type_{n+1}\) | Universe hierarchy (prevents Russell's paradox) |
| Mathematical induction | Type-level `match` |

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **A type is a logical proposition**. `Int` is the proposition "an integer exists", `fn(a: Int, b: Int) -> Int` is the proposition "given two integers, an integer exists".
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to a logical proposition being constructively proved.
- **Terminating type-level computation corresponds to correct inductive reasoning**. YaoXiang's type families (such as `match` on `Add` over `Nat`) are essentially type-level encodings of mathematical induction under the Peano axioms.

### 0.3 Impact on Language Design

Concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level pattern matching on natural numbers `Nat(Zero/Succ)` corresponds to inductive proofs under the Peano axioms
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to finite quantification "for each integer n there exists a type"

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

> **Design Note**: Although RFC-010 proposes a unified model of "everything is assignment" (`name: type = value`), at the syntax level types and values still need to be distinguished. In the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and `ast.rs:25`), and `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the implementation, indicating "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type | Description | Default Size |
|------|------|----------|
| `Type` | Meta type | 0 bytes |
| `Void` | Void | 0 bytes |
| `Bool` | Boolean | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Float | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | Raw bytes | Variable |

Bit-width integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Bit-width floats: `Float32`, `Float64`

---

## Chapter 3: Compound Types

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
- A field name is followed directly by a colon and a type
- An interface name written in the type body indicates implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function belongs to the `Point` namespace. It does not trigger any implicit binding. For the `.` call syntax like `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, and construction may optionally provide them:

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

Methods can be bound directly within the type definition body:

```yaoxiang
// Method 1: Reference an external function for binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // bind to position 0
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

**Interface implementation**: A type implements an interface by listing the interface name at the end of its definition

```yaoxiang
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // implements Drawable interface
    Serializable     // implements Serializable interface
}
```

**Direct interface assignment**: A concrete type can be directly assigned to an interface-typed variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (cannot be determined at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // method lookup via vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario | Inferred Result | Call Method |
|------|----------|----------|
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value | Unknown | vtable |
| Heterogeneous collection | Multiple types | vtable |

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

In generic type definitions, `(T: Type)` is the parameter signature of the type constructor, and `-> Type` represents the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

In generic functions, type parameters are also declared in the signature, and the compiler automatically infers from the actual arguments:

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
    push: (self: List(T), item: T) -> Void,   # self is just a convention name, not a keyword
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
// Interface type definition (as a constraint)
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
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**Core design**: Use `(n: Int)` generic parameters + `(n: n)` value parameters to distinguish compile-time constants from runtime values.

```yaoxiang
// Compile-time factorial: the parameter must be a literal known at compile time
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// Compile-time constant array
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // array with size known at compile time
    length: N
}

// Usage
arr: StaticArray(Int, factorial(5))  // Compiler computes factorial(5) = 120 at compile time
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

// Compile-time validation
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
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

**Syntax**: The type intersection `A & B` represents the type that satisfies both A and B

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
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P is a predefined generic parameter name representing the current compilation platform
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 11: Type Attributes

YaoXiang has only one type attribute that needs to be distinguished: linear vs. copyable. This is automatically inferred by the compiler.

### 11.1 Move (default ownership transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p cannot be read again
```

### 11.2 Dup (shallow copy: copy the handle, share the data)

**The Dup attribute is used for reference/token types**. Assignment of Dup types = shallow copy — copy the handle/token, the underlying data is shared. Multiple holders point to the same piece of data.

| Type | Attribute | Description |
|------|------|------|
| `&T` | Dup | Zero-sized read token; copying the token = multiple views pointing to the same data |
| `ref T` | Dup | Rc/Arc copy = reference count + 1, sharing heap data |
| `&mut T` | Linear | Zero-sized write token, exclusive, cannot be copied |
| All other types | Move | Default ownership transfer |

**Primitive value types** (Int, Float, Bool, Char) are special-cases handled by the compiler: value copy happens automatically on assignment, and the two values are completely independent. This is the compiler's native behavior and does not belong to the Dup type attribute.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // usable
print(view2.x)   // usable

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is the explicit deep copy interface. All types can implement Clone, providing the `.clone()` method.

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

| | Dup | Clone |
|---|---|---|
| **Semantics** | Shallow copy: copy the handle/token, underlying data is shared | Deep copy: create a complete independent replica |
| **Call method** | Implicit (automatic on assignment/parameter passing) | Explicit (`.clone()`) |
| **Modification impact** | Mutually affecting (shared underlying data) | Mutually non-affecting (independent replicas) |
| **Applicable types** | `&T` tokens, `ref T` | Any type implementing the Clone interface |
| **Cost** | Zero overhead (token is a zero-sized type) | Depends on the type |

**Dup does not imply Clone, Clone does not imply Dup** — they are two orthogonal concepts:

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

// Clone: explicit deep copy, create an independent replica
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor a primitive value type
```

**Design intent**:
- Dup is used for token/reference types, solving the problem of "multiple views on the same data"
- Clone is used for scenarios requiring independent replicas; explicit calls keep costs visible
- Primitive value types (Int/Float/Bool/Char) copy via compiler built-in behavior, not belonging to Dup
- Most user-defined types default to Move, zero-copy and high-performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references", but "type-level proofs of access permission".

```
&T      →  Zero-sized, freezes the source data (prohibits WriteToken from being obtained during this period),
          freezing guarantees multiple read-only access is safe → Dup (copyable)
&mut T  →  Zero-sized, exclusive read-write (prohibits any other token),
          under exclusive access copying is meaningless → Linear (non-Dup)
```

**Key properties**:
- The token is an **ordinary type** and follows the same scope rules as all other types
- No lifetime annotation `'a` required
- No dedicated borrow checker required — type attributes (Dup/Linear) naturally derive permissions
- Completely disappears after compilation, zero runtime overhead

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

// Call end: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates &Point token
p.shift(1.0, 1.0)               // Compiler automatically creates &mut Point token
p.print()                       // OK, the previous token has been released as the shift call ended

// Multiple &T tokens coexisting — Dup types allow free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations of ordinary types:

**Returning tokens** — tokens propagate with the return value:

```yaoxiang
// ✅ Sub-token and parent token returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // token returned to the caller
print(px_ref)                    // OK, the token is still in scope
```

**Stored in struct** — structs can carry token fields:

```yaoxiang
// ✅ Struct carrying a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // token field — holding a read-only view of target
}
```

**Closure capture** — closures capture tokens just like capturing any value:

```yaoxiang
// ✅ Closure captures &Float token (Dup type, freely copy into the closure)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 Automatic Borrow Selection

The compiler on the call end automatically selects by the following priority:

```
1. If the actual argument is still used later → prefer creating a token (&T or &mut T, based on method signature)
2. If the actual argument is no longer used → Move
3. Priority match order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point token
p2 = p             // not used later → Move
```

### 12.5 Token Conflict Detection

The compiler performs **flow-sensitive liveness analysis** on token values, tracking the state of each token (active/moved):

```yaoxiang
// ❌ &mut and derived &T cannot be simultaneously active
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ Token scope automatically released after end
good_seq: (p: &mut Point) -> Void = {
    {
        // inner scope
        print(p.x)               // use &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteToken still available
}

// ❌ The same actual argument cannot simultaneously create &mut token and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p derives &mut and & tokens at the same time
```

### 12.6 Compiler Internals: Brand Mechanism

Users never come into contact with brands. The compiler internally assigns a unique compile-time identifier to each token:

```
User sees            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:
- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of thin air
- **Association tracking**: `&Float` derived from field access carries the derived brand (`#N.field_x`), and the compiler can trace back to the parent token
- **Conflict detection**: Same-source WriteToken and derived ReadToken cannot be simultaneously active

Brands completely disappear after monomorphization and inlining, and do not exist in the generated machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freezes source data → Dup safe)
               | &mut T      // WriteToken (exclusive read-write → Linear)
```

### 12.8 Borrow Token vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Take a look / modify in place | Shared ownership |
| Range | Follows the scope of the token value | Cross-scope |
| Cost | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler selects) |
| Escape | Yes (token propagates with return value/struct/closure) | Designed to escape |
| Cross-task | No (tokens have not implemented cross-task passing) | Yes (compiler auto-selects Arc) |
| Cycle detection | Not involved | Silent within task, cross-task lint |

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definitions

```
// === Record type (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Interface type (curly braces, all fields are functions) ===

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

// Type constraints
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

### A.3 Type Attributes Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // value copy happens automatically on assignment, the two values are completely independent
Bool, Char      // not Dup, but the compiler's built-in handling of primitives

// === Dup (shallow copy: copy the handle, share the underlying data) ===
&T              // zero-sized read token, copying the token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count + 1, sharing heap data

// === Linear ===
&mut T          // zero-sized write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // create an independent replica, modifications do not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow token ===
&T              // zero-sized compile-time read token, freezes source data → Dup (copyable)
&mut T          // zero-sized compile-time write token, exclusive read-write → Linear (non-copyable)

// Call end auto-selection
// 1. Actual argument is still used later → create token
// 2. Actual argument is no longer used → Move
// 3. Priority match: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in structs, captured by closures
// ❌ Cannot cross tasks (tokens have not implemented cross-task passing)
```