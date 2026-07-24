# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundation

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It reveals the deep correspondence between the type systems of programming languages and mathematical logic:

| Logic | Programming Language |
|--------|----------|
| Proposition \(P\) | Type `Type` |
| Proof \(p: P\) | Program `x: T = ...` |
| Implication \(P \rightarrow Q\) | Function type `(P) -> Q` |
| Conjunction \(P \wedge Q\) | Product type `{ a: P, b: Q }` |
| Disjunction \(P \vee Q\) | Sum type `{ a(P) \| b(Q) }` |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...` |
| True \(\top\) | `Void` (Unit, has a default value) |
| False \(\bot\) | `Never` (zero constructors, no inhabitable values) |
| Type universe \(Type_n : Type_{n+1}\) | Universe hierarchy (prevents Russell's paradox) |
| Case analysis | Type-level `match` |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type families (e.g., case analysis + recursive calls of `Add` on `Nat`) are essentially type-level encodings of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to a logical proposition being constructively proven.

### 0.3 Impact on Language Design

Concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Case analysis + recursive calls of `Nat(Zero/Succ)` at the type level correspond to Peano axioms—provided the compiler performs termination checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case disjunction in logic
4. **Value-dependent types** (RFC-011): `Vec: (n: Int) -> Type` corresponds to bounded quantification of "for each integer n there exists a type"

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

> **Design Note**: Although RFC-010 proposes a unified "everything is assignment" model (`name: type = value`), at the syntactic level types and values still need to be distinguished. In the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and `ast.rs:25`), and `TypeExpr` is a BNF placeholder corresponding to the `Type` enum in the implementation, indicating "a type is expected at this position."

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types
| Type | Logical Correspondence | Description | Default Size |
|------|---------|------|----------|
| `Type` | — | Meta type | 0 bytes |
| `Never` | ⊥ (false/empty type) | Zero constructors, no inhabitable values. Return type for divergence/panic. `Never <: T` holds for any T. | 0 bytes |
| `Void` | ⊤ (true/Unit) | Has a default void value, a zero-field product type. `x: Void = <default>` is legal. | 0 bytes |
| `Bool` | — | Boolean values: `true` / `false` | 1 byte |
| `Int` | — | Signed integer | 8 bytes |
| `Uint` | — | Unsigned integer | 8 bytes |
| `Float` | — | Floating-point number | 8 bytes |
| `String` | — | UTF-8 string | Variable |
| `Char` | — | Unicode character | 4 bytes |
| `Bytes` | — | Raw bytes | Variable |

Bit-width-specified integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Bit-width-specified floats: `Float32`, `Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are logical primitives of the type system—corresponding to false (⊥) and true (⊤), respectively.

**Never (⊥, false/empty type)** — Three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`, after which code passes type checking (though it is never actually executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The compiler performs dead code analysis and `match` branch merging based on this.

`Never` is a built-in type name (registered with the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, true/Unit)** — Has exactly one inhabitant (the default void value). `Void` is the identity element of zero-field product types. `x: Void = <default>` is legal; functions with no explicit `return` return `Void` by default.

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

// Record type with generics
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
- Record types are defined using curly braces `{}`
- A field name is followed directly by a colon and a type
- An interface name written in the type body indicates implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function belongs to `Point`'s namespace. It does not trigger any implicit binding. For `.` call syntax like `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields may specify default values, making them optional during construction:

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
- `field: Type = expression` -> has a default value, optional during construction
- `field: Type` -> no default value, required during construction

#### 3.1.2 Builtin Bindings

Methods can be bound directly within a type definition body:

```yaoxiang
// Method 1: Reference an external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // Bind to position 0
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

**Interface implementation**: A type implements an interface by listing the interface name at the end of its definition

```yaoxiang
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implement the Drawable interface
    Serializable     // Implement the Serializable interface
}
```

**Direct interface assignment**: A concrete type can be assigned directly to an interface type variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // After compilation: direct call to circle_draw, no vtable

// Function return value (indeterminate at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup via vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario | Inference Result | Call Method |
|------|----------|----------|
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

Generic parameters are part of the function type signature and use the same `()` syntax as ordinary parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the parameter signature of the type constructor, and `-> Type` denotes the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

In generic functions, type parameters are likewise declared in the signature, and the compiler automatically infers them from the actual arguments:

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

// Using an associated type
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

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Constant Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**Core design**: Use `(n: Int)` generic parameters + `(n: n)` value parameters to distinguish compile-time constants from runtime values.

```yaoxiang
// Compile-time factorial: the argument must be a compile-time-known literal
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// Compile-time constant array
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // Array with compile-time-known size
    length: N
}

// Usage
arr: StaticArray(Int, factorial(5))  // Compiler computes factorial(5) = 120 at compile time
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

// Example: compile-time branch
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue bridging and Assert refinement types (see §8.3 for details)
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

### 8.3 Assert Refinement Types and assert Statements

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the dispatch routing pipeline based on whether the free variables of the predicate are reachable at compile time.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch routing rules**:

| Criterion | Mode | Behavior |
|------|------|------|
| All free variables are compile-time known (generic parameters, compile-time constants) | CompileTime | Enters the proof pipeline: true → erased to Void, false → compile error (Never cannot be inhabited) |
| Runtime free variables exist (function parameters, external inputs) | Runtime | Inserts a runtime Bool check, injects refinement facts into the flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control-flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set: old assumptions invalidated
```

After assigning to a `mut` variable, all assumptions involving that variable are removed (kill set). When branches merge, Γ is the intersection of each branch.

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

**Syntax**: A type intersection `A & B` denotes a type that satisfies both A and B

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
// Basic specialization: use function overloading (compiler auto-selects)
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

YaoXiang has only one kind of type attribute that needs to be distinguished: Linear vs. Copyable. This is automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p can no longer be read
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup attribute is for reference/token types**. Assignment of a Dup type = shallow copy—the handle/token is copied, and the underlying data is shared. Multiple holders point to the same block of data.

| Type | Attribute | Description |
|------|------|------|
| `&T` | Dup | Zero-size read token; copying the token = multiple views pointing to the same data |
| `ref T` | Dup | Rc/Arc copy = reference count +1, shared heap data |
| `&mut T` | Linear | Zero-size write token, exclusive, cannot be copied |
| All other types | Move | Default ownership transfer |

**Primitive value types** (Int, Float, Bool, Char) are special-cased by the compiler: assignment automatically performs value copy, and the two values are completely independent. This is a native compiler behavior and does not belong to the Dup type attribute.

```yaoxiang
// &T: Dup, freely aliasable
view: &Point = &p
view2 = view     // Dup: copy the token, both are valid
print(view.x)    // Usable
print(view2.x)   // Usable

// &mut T: Linear, cannot be copied
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T is not Dup, cannot be copied
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship to Dup

**Clone** is an explicit deep-copy interface. All types can implement Clone, providing a `.clone()` method.

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

**Difference between Dup and Clone**:

| | Dup | Clone |
|---|---|---|
| **Semantics** | Shallow copy: copy the handle/token, underlying data is shared | Deep copy: create a complete independent copy |
| **Invocation** | Implicit (automatic on assignment/parameter passing) | Explicit (`.clone()`) |
| **Modification Impact** | Mutually affecting (shared underlying data) | Mutually independent (separate copies) |
| **Applicable Types** | `&T` tokens, `ref T` | Any type implementing the Clone interface |
| **Cost** | Zero overhead (tokens are zero-size types) | Depends on the type |

**Dup does not imply Clone, and Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy the token, underlying data is shared
view: &Point = &p
view2 = view        // Dup: copy the token, both point to the same p
print(view.x)       // Usable
print(view2.x)      // Usable, looking at the same data

// Primitive value type: compiler automatic value copy (not Dup)
x: Int = 42
y = x               // Value copy, x and y are completely independent
print(x)            // Usable

// Clone: explicit deep copy, create an independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p is still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor a primitive value type
```

**Design intent**:
- Dup is for token/reference types, solving the problem of "multiple views looking at the same data"
- Clone is for scenarios requiring independent copies; explicit invocation makes the cost visible
- The copying of primitive value types (Int/Float/Bool/Char) is a built-in compiler behavior and does not belong to Dup
- Most user-defined types default to Move, with zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references," but "type-level proofs of access permission."

```
&T      →  Zero-size, freezes source data (prohibits acquiring WriteToken during this period),
          multi-party read-only is safe under the freeze guarantee → Dup (copyable)
&mut T  →  Zero-size, exclusive read-write (prohibits any other tokens),
          copying is meaningless under exclusive access → Linear (not Dup)
```

**Key properties**:
- Tokens are **ordinary types** that follow the same scoping rules as all other types
- No lifetime annotations `'a` are required
- No dedicated borrow checker is needed—type attributes (Dup/Linear) naturally derive permissions
- Completely disappear after compilation, with zero runtime overhead

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

// Caller side: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates an &Point token
p.shift(1.0, 1.0)               // Compiler automatically creates an &mut Point token
p.print()                       // OK, the previous token was released when shift returned

// Multiple &T tokens coexist — Dup types allow free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations of ordinary types:

**Returning tokens** — tokens propagate along with the return value:

```yaoxiang
// ✅ Sub-token and parent token are returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Token returned to the caller
print(px_ref)                    // OK, token is still in scope
```

**Stored in structs** — structs can carry token fields:

```yaoxiang
// ✅ Struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,              // Token field — holds a read-only view of target
}
```

**Closure capture** — closures capture tokens just like any value:

```yaoxiang
// ✅ Closure captures &Float token (Dup type, freely copied into the closure)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 Automatic Borrow Selection

The caller-side compiler automatically chooses based on the following priority:

```
1. If the actual argument is still used afterward → prioritize creating a token (&T or &mut T, according to the method signature)
2. If the actual argument is not used afterward → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates an &Point token
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates an &mut Point token
p2 = p             // Not used afterward → Move
```

### 12.5 Token Conflict Detection

The compiler performs **flow-sensitive liveness analysis** on token values, tracking the state of each token (active/moved):

```yaoxiang
// ❌ &mut and derived &T cannot both be active
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal use of WriteToken
    print(p.y)
}

// ✅ Token is automatically released after its scope ends
good_seq: (p: &mut Point) -> Void = {
    {
        // Inner scope
        print(p.x)               // Use &mut Point
    }
    // Inner scope ends
    p.x = 10.0                   // ✅ WriteToken is still available
}

// ❌ The same actual argument cannot simultaneously create &mut and other tokens
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & tokens
```

### 12.6 Compiler Internals: Brand Mechanism

Users never interact with brands. The compiler internally assigns each token a compile-time-unique identifier:

```
What users see           Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time-unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time-unique integer
```

Purposes of brands:
- **Anti-forgery**: Tokens can only be obtained from the owner capsule, not constructed out of thin air
- **Association tracking**: A derived `&Float` from a field access carries the derived brand (`#N.field_x`), which the compiler can trace back to the parent token
- **Conflict detection**: Same-source WriteToken and derived ReadToken cannot both be active

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
| Scope | Follows the scope of the token value | Cross-scope |
| Cost | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler-chosen) |
| Escape | Possible (token propagates via return value/struct/closure) | Specifically designed to escape |
| Cross-task | Not supported (tokens are not implemented for cross-task passing) | Supported (compiler auto-selects Arc) |
| Cycle Detection | Not involved | Silent within a task, lint across tasks |

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definitions

```
// === Record types (curly braces) ===

// Record type
Point: Type = { x: Float, y: Float }

// Record type with variants (using function fields)
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === Interface types (curly braces, fields are all functions) ===

// Interface definition
Serializable: Type = { serialize: () -> String }

// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implement the Serializable interface
}

// === Function types ===

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

// Compile-time generics
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Function specialization
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Attribute Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, parameter passing, return = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // Automatic value copy on assignment, two values are completely independent
Bool, Char      // Not Dup; this is the compiler's built-in handling of primitives

// === Dup (shallow copy: copy the handle, share underlying data) ===
&T              // Zero-size read token; copying the token = multiple views pointing to the same data
ref T           // Rc/Arc copy = reference count +1, shared heap data

// === Linear ===
&mut T          // Zero-size write token, Linear (exclusive, cannot be copied)

// === Clone (explicit deep copy) ===
value.clone()   // Create an independent copy; modifications do not affect the original
```

### A.4 Borrow Token Quick Reference

```
// === Borrow tokens ===
&T              // Zero-size compile-time read token, freezes source data → Dup (copyable)
&mut T          // Zero-size compile-time write token, exclusive read-write → Linear (not copyable)

// Caller-side automatic selection
// 1. Actual argument is still used afterward → create a token
// 2. Actual argument is not used afterward → Move
// 3. Priority matching: &T < &mut T < Move

// Token propagation
// ✅ Can be returned, stored in structs, captured by closures
// ❌ Cannot cross tasks (tokens are not implemented for cross-task passing)
```