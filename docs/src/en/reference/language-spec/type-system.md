# Type System Specification

This document defines the type system specification for the YaoXiang programming language, including primitive types, composite types, generics, and traits.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Isomorphism

The Curry-Howard correspondence is the theoretical foundation of YaoXiang's type system. It reveals the deep correspondence between type systems in programming languages and mathematical logic:

| Logic                             | Programming Language                 |
| --------------------------------- | ------------------------------------ |
| Proposition \(P\)                 | Type `Type`                          |
| Proof \(p: P\)                    | Program `x: T = ...`                 |
| Implication \(P \rightarrow Q\)   | Function type `(P) -> Q`             |
| Conjunction \(P \wedge Q\)        | Product type `{ a: P, b: Q }`        |
| Disjunction \(P \vee Q\)          | Sum type `{ a(P) \| b(Q) }`          |
| Universal \(\forall x:T. P(x)\)   | Generics `(T: Type) -> ...`          |
| True \(\top\)                     | `Void` (Unit, has default value)    |
| False \(\bot\)                    | `Never` (zero constructors, no value)|
| Type universe \(Type_n : Type_{n+1}\) | Universe stratification (prevents Russell's paradox) |
| Case analysis                    | Type-level `match`                   |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires type-level recursive functions + compiler termination checks.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type families (e.g., `Add` on `Nat`'s case analysis + recursive calls) are essentially the type-level encoding of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to a constructive proof of a logical proposition.

### 0.3 Impact on Language Design

The concrete manifestations of the Curry-Howard isomorphism in YaoXiang:

1. **Universe stratification** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox from `Type: Type` (Girard paradox)
2. **Type families** (RFC-011): Natural number `Nat(Zero/Succ)` type-level case analysis + recursive calls correspond to Peano axioms—provided the compiler performs termination checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case disjunction in logic
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite quantification of "for each integer N there exists a type"

---

## Chapter 1: Type Classification

### 1.1 Type Expression

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

> **Design Note**: Although RFC-010 proposes a unified model of "everything is assignment" (`name: type = value`), at the syntactic level, types and values still need to be distinguished. In the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and `ast.rs:25`), and `TypeExpr` as a BNF placeholder corresponds to the `Type` enum in the implementation, representing "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type      | Logical Correspondence | Description                                                                    | Default Size |
| --------- | ---------------------- | ------------------------------------------------------------------------------ | ------------ |
| `Type`    | —                      | Meta type                                                                      | 0 bytes      |
| `Never`   | ⊥ (False/Empty type)  | Zero constructors, no values. Divergence/panic return type. `Never <: T` for any T. | 0 bytes      |
| `Void`    | ⊤ (True/Unit)          | Has default void value, zero-field product type. `x: Void = <default>` is legal.| 0 bytes      |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                | 1 byte       |
| `Int`     | —                      | Signed integer                                                                 | 8 bytes      |
| `Uint`    | —                      | Unsigned integer                                                               | 8 bytes      |
| `Float`   | —                      | Floating point number                                                          | 8 bytes      |
| `String`  | —                      | UTF-8 string                                                                   | Variable     |
| `Char`    | —                      | Unicode character                                                              | 4 bytes      |
| `Bytes`   | —                      | Raw bytes                                                                      | Variable     |

Width-specified integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128` Width-specified floats: `Float32`, `Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to False (⊥) and True (⊤) respectively.

**Never (⊥, False/Empty type)** — Three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. `x: Never = ...` has no right-hand side to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`, and subsequent code passes type checking (though never actually executes).
3. **Divergence marker**: `f: (...) -> Never` indicates `f` is guaranteed not to return. The compiler uses this for dead code analysis and `match` branch merging.

`Never` is a built-in type name (same registration path as `Int`/`Bool`), not a keyword.

**Void (⊤, True/Unit)** — Exactly one inhabitant (default void value). `Void` is the identity element of zero-field product types. `x: Void = <default>` is legal. The value of a block is given by the **tail expression** (empty block `{}` is `Void`), see [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) for details.

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
// 简单记录类型
Point: Type = { x: Float, y: Float }

// 空记录类型
Empty: Type = {}

// 带泛型的记录类型
Pair: (T: Type) -> Type = { first: T, second: T }

// 实现接口的记录类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Rules**:

- Record types are defined using curly braces `{}`
- Field name is followed directly by colon and type
- Interface names written in the type body indicate implementation of that interface

> **Namespace attribution**: The `Type.name` prefix (e.g., `Point.draw`) indicates the function belongs to `Point`'s namespace. It does not trigger any implicit binding. For `.` call syntax like `p.draw()` to work, explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, which are optional during construction:

```yaoxiang
// Field with default value - optional during construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Field without default value - required during construction
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

Methods can be bound directly within a type definition body:

```yaoxiang
// Method 1: Reference external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // Bind to position 0
}
// 调用：p1.distance(p2) -> distance(p1, p2)

// 方式2：匿名函数 + 位置绑定
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// 语法：((params) => body)[position]
// 调用：p1.distance(p2) -> distance(p1, p2)
```

### 3.2 Interface Type

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Syntax**: Interfaces are record types with all fields being function types

```yaoxiang
// 接口定义
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空接口
EmptyInterface: Type = {}
```

**Interface implementation**: A type implements interfaces by listing the interface names at the end of its definition

```yaoxiang
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Implements Drawable interface
    Serializable     // Implements Serializable interface
}
```

**Interface direct assignment**: Concrete types can be directly assigned to interface type variables (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determined at compile time -> zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        // 编译后：直接调用 circle_draw，无 vtable

// Function return value (concrete type unknown at compile time -> vtable call)
d: Drawable = get_shape()
d.draw(screen)        // Method lookup through vtable

// 接口作为函数参数
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario             | Inference Result      | Call Method          |
| -------------------- | --------------------- | -------------------- |
| Direct assignment of concrete type | Concrete type determined | Direct call (zero overhead) |
| Function return value       | Unknown              | vtable               |
| Heterogeneous collection     | Multiple types       | vtable               |

**Coherence and Orphan Rules (Not applicable, closing note)**: YaoXiang's interfaces are structural types (interface = record with all fields being function types), not nominal traits—there is no "who can implement what for whom" issue across crates/modules, and Rust-style orphan rules and coherence checks have no subjects to apply to (resolution recorded in RFC-011 §2.1). The corresponding guarantee in the structural world is **duplicate implementation rejection**: redefining the same method signature on a type results in a compilation error (RFC-011a §3, overriding prohibited; overloading allowed).

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

Generic parameters are part of function types, unified with regular parameters using `()` syntax:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In generic type definitions, `(T: Type)` is the type constructor's parameter signature, and `-> Type` represents the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they receive the same treatment as user-defined generics, processed through the unified generic instantiation path. The ownership of length information is the fundamental distinction among the three container concepts:

| Type         | Length      | Semantics                                | Backing                        |
| ------------ | ----------- | ---------------------------------------- | ------------------------------ |
| `Array(T, N)` | Type level  | Fixed-size array (const generic N)       | Core primitive (stack/inline first) |
| `Vec(T)`     | Runtime value | Runtime-length primitive buffer, growable | Core primitive (heap continuous buffer) |
| `List(T)`    | Runtime value | Standard library type (growable list)   | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)` | Runtime value | Key-value mapping                        | `HeapValue::Dict`              |

> `List(T)` is a **standard library type, not a compiler primitive**: defined by YaoXiang itself in `std.list`, treated the same as user-defined generic records. All growable semantics strategies (when to expand, by how much, whether sharing is possible) are in the library; the compiler does not participate. `Vec(T)` is the minimal foundational primitive it depends on.
>
> Set(T) has been removed: no literals, no runtime representation, no std.set. When the need arises, it can be completed following the Dict pattern.

Key rules:

- **Literal destination determined by context**: Bare `[...]` literals with `List(T)` annotation land in growable lists; when `Array(T, N)` annotation directly applies to a literal, it lands in fixed-size arrays. Destination validation: element count == N, element type compatible with T, otherwise compile-time E1002; when N is a symbolic constant (const parameter), count validation is deferred to the refinement type stage.
- **No implicit List→Array conversion**: Fixed-size is guaranteed at the type level—push only accepts `List(A)` receiver.
- **Performance hierarchy**: From bottom to top: decreasing performance, increasing flexibility: `Array` > `Vec` > `List`.
- **Index failure contract** (runtime error is transitional; target state compile-time refinement covers this via value-dependent types, see §8.4):
  - Index out of bounds (including negative indices) → `E6003`
  - Dict missing key → `E6008`
- **Membership `in` predicate**: Returns `Bool`, does not error. Right operand covers List/Array/Dict(keys)/Tuple/String/Range. First-class Hoare predicate, foundation for compile-time provable propositions in refinement types.`

In generic functions, type parameters are also declared in the signature, and the compiler infers them automatically from actual arguments:

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 Generic Type Definitions

```yaoxiang
// 基础泛型类型
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

The field list of a generic type definition **automatically generates constructors**: each field corresponds to a construction parameter, field name is the parameter name; fields with default values can be omitted during construction, fields without defaults are required. Function type fields (methods) do not generate construction parameters.

```yaoxiang
// 类型定义
Container: (T: Type) -> Type = {
    value: T,        // No default -> construction parameter required
    extra: T,
}
// Auto-expanded complete form (compiler internal view, not required to be written manually):
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// Call: invoke the auto-generated constructor
c  = Container(42, 43)            // Construction parameters filled in field order; T unpacked from elements = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // Explicit type parameter + positional construction parameters
c4 = Container(Int)(extra=43, value=42)  // Named field style, order arbitrary
c5 = Container(Int)()             // Empty construction: fields take default/zero values (data assigned later)

// Field default value -> construction parameter can be omitted
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float，x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Invocation rules** (single parentheses, positional matching against declared parameters, left to right):

1. Arguments are tried positionally against type declaration parameters: `Type` position accepts type arguments, compile-time value parameter positions (e.g., `Int`) accept compile-time constants.
2. If a compile-time value parameter position matches successfully (partial match), treat as type construction: check all parameter positions sequentially, when reporting errors report the **first mismatched/missing parameter** according to declaration order.
3. If arguments completely don't match declaration parameters (all are values, no compile-time value parameter positions to match), treat as construction parameters: fill positionally by field order, type parameters unpacked from element types automatically.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 类型位置：一层类型构造
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 两层：类型 + 构造参数
m3 = Matrix(Int, 3, 4)()          // 空构造（RFC-011 §9.3 模式，数据事后赋值）

Matrix(42)    // ❌ Position 0: T←42 doesn't match (42 is not a type); Position 1: Rows←42 matches;
              //    Position 2: Cols missing -> Report first error: T expects Type, found 42
Container(42) // ❌ Missing construction parameter extra
Container(42, 43, 44)  // ❌ Too many construction parameters
```

**Type inference**: Type parameters of generic type constructors are automatically unpacked from construction parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions are automatically unpacked from argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). Must be explicitly filled when unpacking is not possible.

---

## Chapter 5: Type Constraints

### 5.1 Single Constraint

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// 接口类型定义（作为约束）
Clone: Type = {
    clone: () -> Clone
}

// Using constraints
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

> **Constraint resolution source** (RFC-011b): Resolution of operator constraint names (`Add` / `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) = lookup in the interface implementation registry—
> `T: Add` ≜ there is a registered `Add(T, T, T)` instantiation; `Equal` additionally has structural derivation (records with all comparable fields are automatically comparable). `Zero` / `One` / `PartialOrd` etc. have no definition source yet, they are dangling constraint names.

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
// 高阶函数约束
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
// Iterator trait（使用记录类型语法）
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

### 6.2 Generic Associated Type (GAT)

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

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // Compile-time constant (candidate)
```

> The determining factor is **being referenced in a type position**, not "having a concrete type annotation": in `add: (a: Int, b: Int) -> Int = a + b` the `a`/`b` are runtime value parameters (both don't appear in any type position).

**Terminology**: Generic parameters annotated with non-`Type` concrete types (such as `Int`) are called **compile-time value parameter candidates**. Whether they become compile-time value parameters depends on whether their values are referenced in type positions (value-dependent). **No `const` keyword needed** (the implementation once used "const generics" internally; documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Shape coarse filter**: Parameter annotated with non-`Type` concrete type (`Int`/`Bool`/`Float`) → candidate.
2. **Usage fine filter**: Candidate name appears in **type position** (field types in type body, inner `Fn` parameter types, `Assert` predicates, `Array(T, N)` type constructor argument position) → true compile-time value parameter; otherwise **runtime value parameter**.

| Syntax                                                      | Determination                    | Reason                      |
| ----------------------------------------------------------- | -------------------------------- | --------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                      | a/b are runtime value parameters | Only appear in value positions |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is compile-time value parameter| N in type constructor argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                      | N is compile-time value parameter| N as the type of inner parameter k |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                  | N falls through → runtime value parameter | N not referenced in type body |

**Core design**: Using `(N: Int)` compile-time value parameter + `(k: N)` value parameter distinguishes compile-time constants from runtime values. Falling-through candidates (shape is candidate, usage not hit) degrade to runtime value parameters—both the function-level and type constructor paths are handled this way.

```yaoxiang
// Compile-time value parameter: N is referenced in type position (Array length slot)
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N appears in type constructor argument position → compile-time value parameter
    length: N
}

// Usage: factorial(5) evaluates at type position (compile-time), result 120 embedded in type
arr: Measure(Int, factorial(5))  // Compiler computes factorial(5) = 120 at compile time

// Value dependency: N as the type of inner parameter k
// N is a compile-time value parameter (appears in type position of (k: N));
// k is a runtime value parameter, its type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 Compile-Time Constant Arrays

```yaoxiang
// 矩阵类型使用
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
// 类型级 If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// Example: compile-time branching
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue bridging with Assert refinement types (details in §8.3)
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤，程序继续
    false => Never,    // ⊥，发散/编译错误
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 Type Family

```yaoxiang
// 编译期类型转换
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 8.3 Assert Refinement Types and assert Assertions

`assert` and `Assert` are two facets of the same refinement primitive—dispatched automatically based on whether predicate free variables are accessible at compile time.

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                             | Mode         | Behavior                                                               |
| ----------------------------------------------------- | ------------ | ---------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never cannot be inhabited) |
| Runtime free variables exist (function parameters, external input)       | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions for each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 传播
mut x = x - 5       // Γ = {}  ← mut kill set：旧假设失效
```

After `mut` variable assignment, all assumptions involving that variable are removed (kill set). When branches merge, Γ takes the intersection of each branch.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **builtin predicate**, alongside `Int` and `Never` as core primitives (builtin name, not a keyword). It binds a **measure** to a computation, declaring that the computation terminates, and provides a witness of termination.

**Forms**: Two arities, the same predicate:

| Form                       | Anchor              | Purpose                                                              |
| -------------------------- | ------------------- | -------------------------------------------------------------------- |
| `Terminates(m)`            | Name of the binding | Default form—self-recursive functions, loops                         |
| `Terminates(FnType, m)`    | Explicit function type | When measure attribution needs explicit indication (measure defined elsewhere, same measure serves multiple computations) |

```yaoxiang
// Measure: ordinary function, testable individually, reusable, does not participate in runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Binary form: measure defined elsewhere, explicit attribution
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Unary form: anchor is the binding name, measure is an expression in scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**Semantics**: `Terminates(m)` refines the value type of the computation annotated at its type position. The obligation falls on that computation—functions at each recursive call site, loops at the back edge—all being "the measure of the next state is strictly less than the measure of the current state", judged under the path guard at that point.

The call site's obligation is `m(callee_args) < m(caller_args)`; the back edge's obligation is `m(next_round) < m(this_round)`. Both have the same form.

> **Why cover loops**: Loops are anonymous constructs, typically not referable. The binding name provides the anchor—`acc` in `acc: Terminates(n - i) = while ...` makes the loop referable. This is why the unary form of `Terminates` applies to loops.

**Measure**: Return type is unrestricted (natural numbers not enforced); the "strictly decreasing" relationship is given by a well-founded order available on that type. Whether the measure is well-founded (e.g., when returning `Int`, whether it's `>= 0`) is an **independent obligation**, judged by the compile-time proof pipeline alongside the decreasing obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it enters verification mode. Ordinary types not refined (bare `while` loops, functions without refinement signatures) do not enter verification mode and do not generate termination obligations.

**Automatic exploration priority**: The compiler first automatically explores measures (four templates: linear rank functions, predicate violation counts, bounded increment/decrement, multiplicative scaling), only requiring explicit `Terminates` when exploration fails.

**Runtime representation**: Pure compile-time entity, erased with witness, does not appear in runtime binary.

> Full design in [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md) (implementation mechanism).

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

**Syntax**: Type intersection `A & B` represents a type satisfying both A and B

```yaoxiang
// Interface combination = type intersection
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

// 通用实现
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

YaoXiang has only one type property to distinguish: linear vs. copyable. Derived automatically by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types default to Move semantics. Assignment, passing arguments, returning = ownership transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move, p cannot be read again
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**Dup property is for reference/handle types**. Dup type assignment = shallow copy—copy handle/handle, underlying data shared. Multiple holders point to the same data.

| Type        | Property | Description                                              |
| ----------- | -------- | -------------------------------------------------------- |
| `&T`        | Dup      | Zero-size read handle, copying handle = multiple views to same data |
| `ref T`     | Dup      | Rc/Arc copy = reference count +1, shared heap data     |
| `&mut T`    | Linear   | Zero-size write handle, exclusive, non-copyable          |
| All others  | Move     | Default ownership transfer                               |

**Primitive value types** (Int, Float, Bool, Char) are specially handled by the compiler: automatic value copy on assignment, two values completely independent. This is compiler-native behavior, not a Dup type property.

```yaoxiang
// &T: Dup, can alias freely
view: &Point = &p
view2 = view     // Dup: copy handle, both valid
print(view.x)    // usable
print(view2.x)   // usable

// &mut T: Linear，不可复制
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T 不是 Dup，不能复制
```

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing a `.clone()` method.

```yaoxiang
// Clone 接口定义（标准库）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // Deep copy, p still usable
p2 = p.clone()        // Can clone multiple times
```

**Difference between Dup and Clone**:

|              | Dup                                 | Clone                     |
| ------------ | ----------------------------------- | ------------------------- |
| **Semantics**| Shallow copy: copy handle/handle, underlying data shared | Deep copy: create complete independent copy |
| **Invocation** | Implicit (automatic on assignment/passing) | Explicit (`.clone()`)    |
| **Modification effect** | Affect each other (shared underlying data) | Independent (separate copies) |
| **Applicable types** | `&T` handles, `ref T`            | Any type implementing Clone interface |
| **Cost**     | Zero overhead (handles are zero-size types) | Varies by type            |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup type: copy handle, underlying data shared
view: &Point = &p
view2 = view        // Dup: copy handle, both point to same p
print(view.x)       // usable
print(view2.x)      // usable, sees the same data

// Primitive value types: compiler automatic value copy (not Dup)
x: Int = 42
y = x               // Value copy, x and y completely independent
print(x)            // usable

// Clone: explicit deep copy, create independent copy
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone: deep copy, p still usable
r = p               // Move: ownership transfer, because Point is neither Dup nor primitive value type
```

**Design intent**:

- Dup is for handle/reference types, solving "multiple views of the same data" problem
- Clone is for scenarios requiring independent copies, explicit call makes cost visible
- Primitive value types (Int/Float/Bool/Char) copy is compiler built-in behavior, not Dup
- Most custom types default to Move, zero-copy high performance

## Chapter 12: Borrow Handle Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time handle types**. They are not "references" but "type-level proofs of access permission".

```
&T      →  zero size, freezes source data (prevents WriteHandle acquisition during this time),
          multiple read-only views safe under freeze guarantee → Dup (copyable)
&mut T  →  zero size, exclusive read/write (prevents any other handles),
          copying meaningless under exclusive access → Linear (non-Dup)
```

**Key characteristics**:

- Handles are **ordinary types**, following the same scope rules as all other types
- No lifetime annotations `'a` needed
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally derive permissions
- Completely disappear after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// Method side: declare parameter types, determine required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point handle grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point handle grants write permission
    self.y = self.y + dy
}

// Call side: compiler automatically selects borrow or Move
p = Point(1.0, 2.0)
p.print()                       // Compiler automatically creates &Point handle
p.shift(1.0, 1.0)               // Compiler automatically creates &mut Point handle
p.print()                       // OK, previous handle released after shift call ends

// Multiple &T handles coexist—Dup type allows free copying
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Handle Scope and Propagation

Handles are ordinary types, thus supporting all ordinary type operations:

**Returning handles**—handles propagate with return values:

```yaoxiang
// ✅ Sub-handles and parent handles returned together
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // Handle returned to caller
print(px_ref)                    // OK, handle still in scope
```

**Storing in structs**—structs can carry handle fields:

```yaoxiang
// ✅ Structs carry handles as fields
Window: Type = {
    target: Point,
    view: &Point,              // Handle field—holds read-only view of target
}
```

**Closures don't capture; context solidifies at creation point**—closures only take their own parameters; when outer data is needed, currying solidifies the value into the closure at the creation point:

```yaoxiang
// ✅ Context solidified via currying: threshold is a parameter, gt_point(threshold) solidifies the value into the closure at creation point
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After closures (function values) escape, their definition scope may be dead, so implicit outer variable capture is not allowed; but the call site (creation point) scope is guaranteed alive, solidifying context as a value into the closure at that point is safe.

### 12.4 Automatic Borrow Selection

The call-side compiler auto-selects according to priority:

```
1. If actual argument is used later → prefer creating handle (&T or &mut T, according to method signature)
2. If actual argument is not used later → Move
3. Match priority: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print's parameter type is &Point → compiler creates &Point handle
p.shift(1.0, 1.0)  // shift's parameter type is &mut Point → compiler creates &mut Point handle
p2 = p             // not used later → Move
```

**Method receiver follows signature semantics** (RFC-011a receiver spelling convention same flavor): receiver is `&T` → read-only borrow handle; `&mut T` → mutable borrow handle; by value → Move (consumes receiver). Borrow handles created at call sites are released when the call ends (transient, §12.5 interval semantics); interface borrow receivers are explicitly declared `&Self` by the interface author, impl signatures after `Self ↦ impl type` substitution must exactly match the interface (RFC-011a §3).

### 12.5 Handle Conflict Detection

Handle conflict detection is the **borrow Hoare proposition** (RFC-009a), not an independent flow-sensitive analysis. The compiler auto-generates borrow propositions (`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) sent to the proof pipeline for verification; handle liveness is the interval `[created_at, last_use]` (see RFC-009a §reverse BFS liveness analysis):

```yaoxiang
// ❌ &mut and derived &T cannot be simultaneously alive
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ Normal WriteHandle usage
    print(p.y)
}

// ✅ Handle scope ends, automatically released
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部作用域
        print(p.x)               // 使用 &mut Point
    }
    // inner scope ends
    p.x = 10.0                   // ✅ WriteHandle still usable
}

// ❌ Same actual argument cannot simultaneously create &mut handle and other handles
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p simultaneously derives &mut and & handles
```

### 12.6 Compiler Internals: Branding Mechanism

Users never touch brands. The compiler internally assigns each handle a compile-time unique identifier:

```
User-visible           Compiler internal representation
────────────────────────────────────────
&Point         →  ReadHandle(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteHandle(Point, #M)   // #M is a compile-time unique integer
```

Brand uses:

- **Anti-forgery**: Handles can only be obtained from owning capsules, cannot be conjured
- **Derivation tracking**: Field access-derived `&Float` carries derived brand (`#N.field_x`), compiler can track back to parent handle
- **Conflict detection**: Same-origin WriteHandle and derived ReadHandle cannot be simultaneously alive

Brands completely disappear after monomorphization and inlining; generated machine code contains no trace. **Zero runtime overhead.**

### 12.7 Handle Sum Types

```
&BorrowHandle ::= &T          // ReadHandle (freezes source data → Dup safe)
               | &mut T      // WriteHandle (exclusive read/write → Linear)
```

### 12.8 Borrow Handles vs. ref

|        | `&T` / `&mut T`                  | `ref`                   |
| ------ | -------------------------------- | ----------------------- |
| Purpose| Glance/modify in place           | Shared ownership        |
| Scope  | Follows handle value's scope      | Cross-scope             |
| Cost   | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler selects)   |
| Escape | Can (handle propagates with returns/structs)    | Originally for escaping      |
| Cross-task | Cannot (handles not implemented for cross-task transfer) | Can (compiler auto-selects Arc)  |
| Cycle detection | Not involved                           | Silent within task, lint across tasks |

> Note (undefined): How to read content after ref creation (dereference/method/auto) is not yet defined in the spec; current implementation `*a` reports E1052. To be added after definition.

---

## Appendix: Type Definition Quick Reference

### A.1 Type Definitions

```
// === 记录类型（花括号） ===

// 记录类型
Point: Type = { x: Float, y: Float }

// 带变体的记录类型（使用函数字段）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === 接口类型（花括号，字段全为函数） ===

// 接口定义
Serializable: Type = { serialize: () -> String }

// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Implements Serializable interface
}

// === 函数类型 ===

Adder: Type = (Int, Int) -> Int

// === Termination measure (builtin predicate, see §8.4) ===

// Unary: anchor is binding name (self-recursive functions, loops)
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// Binary: explicit measure attribution (measure defined elsewhere)
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 Generic Syntax

```
// 泛型类型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 泛型函数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// Type constraints
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated types
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Compile-time generics: N referenced in type position (k: N) → compile-time value parameter
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// Conditional types
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 函数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Properties Quick Reference

```
// === Move (default) ===
// All types default to Move. Assignment, argument passing, returning = ownership transfer

// === Primitive value types (compiler built-in) ===
Int, Float,     // Automatic value copy on assignment, two values completely independent
Bool, Char      // Not Dup, compiler built-in handling for primitives

// === Dup (shallow copy: copy handle, share underlying data) ===
&T              // Zero-size read handle, copying handle = multiple views to same data
ref T           // Rc/Arc copy = reference count +1, shared heap data

// === Linear ===
&mut T          // Zero-size write handle, Linear (exclusive, non-copyable)

// === Clone (explicit deep copy) ===
value.clone()   // Create independent copy, modification doesn't affect original
```

### A.4 Borrow Handle Quick Reference

```
// === Borrow handles ===
&T              // Zero-size compile-time read handle, freezes source data → Dup (copyable)
&mut T          // Zero-size compile-time write handle, exclusive read/write → Linear (non-copyable)

// Call-side auto-selection
// 1. Actual argument used later → create handle
// 2. Actual argument not used later → Move
// 3. Match priority: &T < &mut T < Move

// Handle propagation
// ✅ Can return, store in structs, captured by closures
// ❌ Cannot cross tasks (handles not implemented for cross-task transfer)
```