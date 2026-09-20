# Type System Specification

This document defines the type system specification for the YaoXiang programming language, including
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
| Truth \(\top\)                                 | `Void` (Unit, with default value)               |
| Falsity \(\bot\)                               | `Never` (zero constructors, uninhabitable)      |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computations correspond to correct constructive proofs**. YaoXiang's type
  families (such as `Add`'s case analysis + recursive call on `Nat`) are essentially the type-level
  encoding of mathematical induction—provided the compiler can perform termination checking.
- **Type checking is proof verification**. When a program passes type checking, it is equivalent to
  a logical proposition being constructively proven.

### 0.3 Impact on Language Design

Specific manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ …` avoids the logical paradox (Girard's
   paradox) caused by `Type: Type`
2. **Type families** (RFC-011): Type-level case analysis + recursive call on natural number
   `Nat(Zero/Succ)` corresponds to Peano axioms—provided the compiler performs termination checking
3. **Conditional types** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent types** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to finite
   quantification of "for every integer N there exists a type"

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
> `ast.rs:25`), where `TypeExpr` serves as a BNF placeholder corresponding to the `Type` enum in the
> implementation, indicating "a type is expected at this position".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Correspondence | Description                                                                                    | Default Size |
| -------- | ---------------------- | ---------------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                      | Meta type                                                                                      | 0 bytes      |
| `Never`  | ⊥ (False/Empty type)   | Zero constructors, no value exists. Diverging/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (True/Unit)          | Has default void value, zero-field product type. `x: Void = <default>` is legal.               | 0 bytes      |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                                | 1 byte       |
| `Int`    | —                      | Signed integer                                                                                 | 8 bytes      |
| `Uint`   | —                      | Unsigned integer                                                                               | 8 bytes      |
| `Float`  | —                      | Floating-point number                                                                          | 8 bytes      |
| `String` | —                      | UTF-8 string                                                                                   | Variable     |
| `Char`   | —                      | Unicode character                                                                              | 4 bytes      |
| `Bytes`  | —                      | Raw bytes                                                                                      | Variable     |

Integers with bit widths: `Int8`, `Int16`, `Int32`, `Int64`, `Int128` Floats with bit widths:
`Float32`, `Float64`

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to False (⊥) and True
(⊤) respectively.

**Never (⊥, False/Empty type)** — Three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing to write on the right side.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   after which the code can pass type checking (though it will never actually execute).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis and `match` branch merging.

`Never` is a built-in type name (registered with the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, True/Unit)** — Has exactly one inhabitant (the default void value). `Void` is the
identity element of zero-field product types. `x: Void = <default>` is legal. The value of a block
is given by the **tail expression** (an empty block `{}` is `Void`); see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for details.

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
- Field names are followed directly by a colon and the type
- Interface names written in the type body indicate implementation of that interface

> **Namespace Ownership**: The `Type.name` prefix (e.g., `Point.draw`) indicates that the function
> belongs to `Point`'s namespace. It does not trigger any implicit binding. To make the `.` call
> syntax like `p.draw()` work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004
> and RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, and may be optionally provided during construction:

```yaoxiang
// 有默认值的字段 - 构造时可选
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// 无默认值的字段 - 构造时必填
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正确
Point2()          // 错误
```

**Rules**:

- `field: Type = expression` -> has default value, optional during construction
- `field: Type` -> no default value, required during construction

#### 3.1.2 Built-in Binding

Methods can be bound directly within a type definition body:

```yaoxiang
// 方式1：引用外部函数绑定
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 绑定到位置0
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

### 3.2 Interface Types

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Syntax**: An interface is a record type whose fields are all function types

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

**Interface Implementation**: A type implements an interface by listing the interface name at the
end of its definition

```yaoxiang
// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // 实现 Drawable 接口
    Serializable     // 实现 Serializable 接口
}
```

**Direct Interface Assignment**: A concrete type can be directly assigned to an interface type
variable (structural subtyping)

```yaoxiang
// 直接赋值（编译期可确定具体类型 -> 零开销调用）
d: Drawable = Circle(1)
d.draw(screen)        // 编译后：直接调用 circle_draw，无 vtable

// 函数返回值（编译期无法确定 -> vtable 调用）
d: Drawable = get_shape()
d.draw(screen)        // 通过 vtable 查找方法

// 接口作为函数参数
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time Optimization Strategies**:

| Scenario                           | Inference Result           | Call Method                 |
| ---------------------------------- | -------------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value              | Unknown                    | vtable                      |
| Heterogeneous collection           | Multiple types             | vtable                      |

**Coherence and Orphan Rules (N/A, Concluding Note)**: YaoXiang's interfaces are structural types
(interface = record with all function fields), not nominal traits—there is no "who can implement for
whom" ownership issue across crates/modules, and Rust-style orphan rules and coherence checks have
no applicable object (ruling recorded in RFC-011 §2.1). The corresponding guarantee in the
structural world is **duplicate implementation rejection**: repeated definitions of the same method
signature on a type result in a compile error (RFC-011a §3, overriding prohibited; overloading is
legal).

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

Generic parameters are part of the function type, using the same `()` syntax as ordinary parameters:

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

### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they are treated the same as
user-defined generics and processed through the unified generic instantiation path. The placement of
length information is the fundamental distinction among the three container concepts:

| Type          | Length        | Semantics                             | Foundation                               |
| ------------- | ------------- | ------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)  | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Runtime-length raw buffer, growable   | Core primitive (continuous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list) | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                     | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: it is defined by YaoXiang
> itself in `std.list`, treated the same as user-defined generic records. All growable semantics
> strategies (when to expand, how much, whether it can be shared) are in the library; the compiler
> does not participate. `Vec(T)` is the minimal foundational primitive it depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When needs arise,
> complete it following the Dict pattern.

Key rules:

- **Literal destination determined by context**: A bare `[...]` literal and the `List(T)` annotation
  land in the growable list; the `Array(T, N)` annotation directly applied to a literal lands in a
  fixed-length array. Landing validation: number of elements == N, element types compatible with T,
  otherwise compile-time E1002; when N is a symbolic constant (const parameter), the count
  validation is deferred to the refinement type phase.
- **Implicit List→Array conversion prohibited**: Fixed-length property is guaranteed by the type
  layer—push only accepts `List(A)` receivers.
- **Performance hierarchy**: From bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index failure contract** (runtime error as transitional state, target state is compile-time
  refinement coverage, via value-dependent types):
  - Index out of bounds (including negative indices) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: returns `Bool` without erroring, the right operand covers
  List/Array/Dict(keys)/Tuple/String/Range. First-class Hoare predicate, the foundation of
  compile-time provable propositions in refinement types.

In generic functions, type parameters are also declared in the signature, and the compiler
automatically infers them from the actual arguments:

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
    push: (self: List(T), item: T) -> Void,   // self 只是约定名，不是关键字
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Generic Construction Calls and Type Inference

The field list of a generic type definition **automatically generates a constructor function**: each
field corresponds to a construction parameter, with the field name as the parameter name; fields
with default values can be omitted during construction, while fields without default values are
required. Function-type fields (methods) do not generate construction parameters.

```yaoxiang
// 类型定义
Container: (T: Type) -> Type = {
    value: T,        // 无默认值 → 构造参数必填
    extra: T,
}
// 自动展开的完整形式（编译器内部视图，不要求用户手写）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 调用：调用自动生成的构造函数
c  = Container(42, 43)            // 构造参数按字段顺序填；T 从元素自动解包 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 显式类型参数 + 位置式构造参数
c4 = Container(Int)(extra=43, value=42)  // 字段名式，顺序任意
c5 = Container(Int)()             // 空构造：字段取默认值/零值（数据事后赋值）

// 字段默认值 → 构造参数可省略
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float，x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**Call rules** (single parentheses, matching by declared parameters position-by-position, left to
right):

1. Actual arguments attempt to match declared type parameters position-by-position: the `Type`
   position accepts type arguments, and compile-time value parameter positions (e.g., `Int`) accept
   compile-time constants.
2. If some compile-time value parameter positions match successfully (partial match), process as
   type construction: check all parameter positions in order, reporting errors by declaration
   order—**report the first mismatching/missing parameter first**.
3. If actual arguments do not correspond to declared parameters at all (all are values, no
   compile-time value parameter positions match), process as construction parameters: positional
   style fills by field order, type parameters are auto-unpacked from element types.

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 类型位置：一层类型构造
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 两层：类型 + 构造参数
m3 = Matrix(Int, 3, 4)()          // 空构造（RFC-011 §9.3 模式，数据事后赋值）

Matrix(42)    // ❌ 位0: T←42 不匹配（42 不是类型）；位1: Rows←42 匹配；
              //    位2: Cols 缺失 → 先报第一个错误：T 期望 Type，找到 42
Container(42) // ❌ 缺构造参数 extra
Container(42, 43, 44)  // ❌ 构造参数超数
```

**Type inference**: Type parameters of a generic type constructor are auto-unpacked from
construction parameter elements (`Container(42, 43)` → T=Int); type parameters of generic functions
are auto-unpacked from actual argument types (`map(numbers, f)` → T=Int, R=String, see §4.1).
Explicit filling is required when unpacking is impossible.

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

// 使用约束
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple Constraints

```yaoxiang
// 多重约束语法
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// 泛型容器的排序
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
    Item: T,                    // 关联类型
    next: () -> Option(T),
    has_next: () -> Bool
}

// 使用关联类型
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
// 更复杂的关联类型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 关联类型也是泛型的
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-time Generics

### 7.1 Compile-time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // 编译期常量（候选）
```

> The judgment criterion is **being referenced in type position**, not "annotated with a specific
> type": in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters (neither
> appears in any type position).

**Terminology**: Generic parameters annotated with concrete types other than `Type` (such as `Int`)
are called **compile-time value parameter candidates**; whether they become compile-time value
parameters depends on whether their values are referenced in type positions (value-dependent). **No
`const` keyword is needed** (the implementation internally once used "const generics" as a term; the
documentation uniformly uses "compile-time value parameters").

**Determination rules (two steps)**:

1. **Form coarse screening**: Parameters annotated with concrete types other than `Type`
   (`Int`/`Bool`/`Float`) → candidates.
2. **Usage fine screening**: Candidate names appear in **type positions** (type body field types,
   inner `Fn` parameter types, `Assert` predicates, `Array(T, N)` type construction argument
   positions) → true compile-time value parameters; otherwise **runtime value parameters**.

| Syntax                                                     | Determination                             | Reason                                           |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appear in value positions                   |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N appears in type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N is used as the type of inner parameter k       |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N not referenced in type body                    |

**Core design**: Use `(N: Int)` compile-time value parameter + `(k: N)` value parameter to
distinguish compile-time constants from runtime values. Candidates that fall through (form is
candidate, usage does not hit) degenerate to runtime value parameters—both function-level and
type-constructor paths handle this uniformly.

```yaoxiang
// 编译期值参数：N 在类型位置（Array 长度槽）被引用
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N 出现在类型构造实参位 → 编译期值参数
    length: N
}

// 使用方式：factorial(5) 在类型位置求值（编译期），结果 120 嵌入类型
arr: Measure(Int, factorial(5))  // 编译器在编译期计算 factorial(5) = 120

// 值依赖：N 作为内层参数 k 的类型
// N 是编译期值参数（出现在 (k: N) 的类型位）；
// k 是运行时值参数，其类型为字面量类型 N（单值类型）。
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 Compile-time Constant Arrays

```yaoxiang
// 矩阵类型使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// 编译期维度验证
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

// 示例：编译期分支
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue 桥接与 Assert 精化类型（详见 §8.3）
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤，程序继续
    false => Never,    // ⊥，发散/编译错误
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 Type Families

```yaoxiang
// 编译期类型转换
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 8.3 Assert Refinement Types and assert Statement

`assert` and `Assert` are two sides of the same refinement primitive—automatically selected by the
dispatch pipeline based on "whether predicate free variables are compile-time reachable".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                              | Mode        | Behavior                                                                                 |
| -------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------- |
| All free variables are compile-time known (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never uninhabitable) |
| Runtime free variables exist (function parameters, external inputs)                    | Runtime     | Insert runtime Bool check, inject refinement facts into flow-sensitive assumption set Γ  |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions at each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
mut x = x - 5       // Γ = {}  ← mut kill set：旧假设失效
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

**Syntax**: Type intersection `A & B` represents a type that satisfies both A and B

```yaoxiang
// 接口组合 = 类型交集
DrawableSerializable: Type = Drawable & Serializable

// 使用交集类型
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## Chapter 10: Function Overloading and Specialization

### 10.1 Function Overloading

```yaoxiang
// 基本特化：使用函数重载（编译器自动选择）
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
// 平台类型枚举（标准库定义）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P 是预定义泛型参数名，代表当前编译平台
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 11: Type Properties

YaoXiang has only one type property that needs to be distinguished: linear vs. copyable. This is
automatically inferred by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, returning = ownership
transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move，p 不可再读
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assignment of a Dup type = shallow copy—copy
the handle/token, share the underlying data. Multiple holders point to the same block of data.

| Type            | Property | Description                                                                         |
| --------------- | -------- | ----------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-sized read token, copying the token = multiple views pointing to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, share heap data                                   |
| `&mut T`        | Linear   | Zero-sized write token, exclusive, non-copyable                                     |
| All other types | Move     | Default ownership transfer                                                          |

**Primitive value types** (Int, Float, Bool, Char) have special handling built into the compiler:
assignments automatically value-copy, the two values are completely independent. This is a native
compiler behavior, not part of the Dup type property.

```yaoxiang
// &T: Dup，可自由别名
view: &Point = &p
view2 = view     // Dup：复制令牌，两者均有效
print(view.x)    // 可用
print(view2.x)   // 可用

// &mut T: Linear，不可复制
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T 不是 Dup，不能复制
```

### 11.3 Clone (Explicit Deep Copy) and its Relationship with Dup

**Clone** is the explicit deep copy interface. All types can implement Clone, providing a `.clone()`
method.

```yaoxiang
// Clone 接口定义（标准库）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深复制，p 仍然可用
p2 = p.clone()        // 可多次克隆
```

**Differences between Dup and Clone**:

|                         | Dup                                                    | Clone                                       |
| ----------------------- | ------------------------------------------------------ | ------------------------------------------- |
| **Semantics**           | Shallow copy: copy handle/token, share underlying data | Deep copy: create complete independent copy |
| **Call method**         | Implicit (auto on assignment/parameter passing)        | Explicit (`.clone()`)                       |
| **Modification impact** | Mutually affect (share underlying data)                | Mutually independent (independent copies)   |
| **Applicable types**    | `&T` tokens, `ref T`                                   | Any type implementing the Clone interface   |
| **Cost**                | Zero overhead (tokens are zero-sized types)            | Depends on the type                         |

**Dup does not imply Clone, Clone does not imply Dup**—they are two orthogonal concepts:

```yaoxiang
// Dup 类型：复制令牌，底层数据共享
view: &Point = &p
view2 = view        // Dup：复制令牌，两者指向同一个 p
print(view.x)       // 可用
print(view2.x)      // 可用，看到的是同一份数据

// 原语值类型：编译器自动值复制（不是 Dup）
x: Int = 42
y = x               // 值复制，x 和 y 完全独立
print(x)            // 可用

// Clone：显式深拷贝，创建独立副本
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深复制，p 仍然可用
r = p               // Move：所有权转移，因为 Point 不是 Dup 也不是原语值类型
```

**Design intent**:

- Dup is used for token/reference types, solving the problem of "multiple views of the same data"
- Clone is used for scenarios requiring independent copies, with explicit calls making the cost
  visible
- Primitive value types (Int/Float/Bool/Char) copying is built-in compiler behavior, not part of Dup
- Most user-defined types are Move by default, with zero-copy high performance

## Chapter 12: Borrow Token Types

### 12.1 Core Concept

`&T` and `&mut T` are **zero-sized compile-time token types**. They are not "references", but
"type-level proofs of access permission".

```
&T      →  Zero-sized, freezes source data (prevents WriteToken acquisition during this period),
          under the freeze guarantee, multiple read-only copies are safe → Dup (copyable)
&mut T  →  Zero-sized, exclusive read-write (prevents any other tokens),
          under exclusive access, copying is meaningless → Linear (non-Dup)
```

**Key features**:

- Tokens are **ordinary types**, following the same scope rules as all other types
- No lifetime annotation `'a` required
- No dedicated borrow checker needed—type properties (Dup/Linear) naturally infer permissions
- Completely disappear after compilation, zero runtime overhead

### 12.2 Basic Usage

```yaoxiang
// 方法端：声明参数类型，决定需要的权限
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point 令牌授予读权限
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point 令牌授予写权限
    self.y = self.y + dy
}

// 调用端：编译器自动选择借用或 Move
p = Point(1.0, 2.0)
p.print()                       // 编译器自动创建 &Point 令牌
p.shift(1.0, 1.0)               // 编译器自动创建 &mut Point 令牌
p.print()                       // OK，上一个令牌已随 shift 调用结束而释放

// 多个 &T 令牌共存——Dup 类型允许自由复制
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 Token Scope and Propagation

Tokens are ordinary types, so they support all operations of ordinary types:

**Returning tokens**—tokens propagate along with the return value:

```yaoxiang
// ✅ 子令牌和父令牌一起返回
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // 令牌返回给调用者
print(px_ref)                    // OK，令牌仍在作用域
```

**Stored in struct**—structs can carry token fields:

```yaoxiang
// ✅ 结构体携带令牌作为字段
Window: Type = {
    target: Point,
    view: &Point,              // 令牌字段——持有对 target 的只读视图
}
```

**Closures do not capture, context is fixed at creation point**—closures only take their own
parameters; when they need outer data, they fix the value into the closure at the creation point
through currying:

```yaoxiang
// ✅ 上下文经柯里化固化：threshold 是参数，gt_point(threshold) 在创建点把值固化进闭包
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition may be dead, so
> implicit capture of outer variables is not allowed; but the call point (creation point) scope is
> necessarily alive, and it is safe for context to be fixed as a value into the closure at that
> point.

### 12.4 Automatic Borrow Selection

The call-side compiler auto-selects based on the following priority:

```
1. If the actual argument is used afterwards → prioritize creating a token (&T or &mut T, based on method signature)
2. If the actual argument is not used afterwards → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print 的参数类型为 &Point → 编译器创建 &Point 令牌
p.shift(1.0, 1.0)  // shift 的参数类型为 &mut Point → 编译器创建 &mut Point 令牌
p2 = p             // 后续不再使用 → Move
```

**Method receiver follows signature semantics** (same as RFC-011a receiver spelling convention):
receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value → Move (consume
receiver). The borrow token generated at the call point is released when the call ends (transient,
§12.5 interval semantics); interface borrow receivers are explicitly declared by the interface
author as `&Self`, and the impl signature after `Self ↦ impl type` substitution must exactly match
the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is **borrowing Hoare propositions** (RFC-009a), not independent
flow-sensitive analysis. The compiler automatically generates borrowing propositions
(`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`) and feeds them into the proof
pipeline for verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a
§Reverse BFS Liveness Analysis):

```yaoxiang
// ❌ &mut 和派生的 &T 不能同时活跃
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ 正常使用 WriteToken
    print(p.y)
}

// ✅ 令牌作用域结束后自动释放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部作用域
        print(p.x)               // 使用 &mut Point
    }
    // 内部作用域结束
    p.x = 10.0                   // ✅ WriteToken 仍可用
}

// ❌ 同一实参不能同时创建 &mut 令牌和其他令牌
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p 同时派生 &mut 和 & 令牌
```

### 12.6 Compiler Internals: Brand Mechanism

Users never encounter brands. The compiler internally assigns a unique compile-time identifier to
each token:

```
User sees            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Purpose of brands:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Association tracking**: Field-access-derived `&Float` carries the derived brand (`#N.field_x`),
  the compiler can trace back to the parent token
- **Conflict detection**: Same-source WriteToken and derived ReadToken cannot be active
  simultaneously

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freeze source data → Dup safe)
               | &mut T      // WriteToken (exclusive read-write → Linear)
```

### 12.8 Borrow Tokens vs ref

|                 | `&T` / `&mut T`                                               | `ref`                               |
| --------------- | ------------------------------------------------------------- | ----------------------------------- |
| What it does    | Take a look / modify in place                                 | Shared ownership                    |
| Scope           | Follows token value's scope                                   | Cross-scope                         |
| Cost            | Zero overhead (zero-sized type, disappears after compilation) | Rc or Arc (compiler chooses)        |
| Escape          | Yes (token propagates via return value/struct)                | Designed for escape                 |
| Cross-task      | No (tokens do not implement cross-task passing)               | Yes (compiler auto-selects Arc)     |
| Cycle detection | Not involved                                                  | Silent within task, cross-task lint |

> Note (undefined): How to read content after ref creation (dereference/method/auto) is not yet
> defined in the spec, current implementation has `*a` reporting E1052. To be added to this section
> after definition.

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

// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // 实现 Serializable 接口
}

// === 函数类型 ===

Adder: Type = (Int, Int) -> Int
```

### A.2 Generic Syntax

```
// 泛型类型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 泛型函数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 类型约束
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 关联类型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// 编译期泛型：N 在类型位置 (k: N) 被引用 → 编译期值参数
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// 条件类型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 函数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 Type Properties Quick Reference

```
// === Move（默认） ===
// 所有类型默认 Move。赋值、传参、返回 = 所有权转移

// === 原语值类型（编译器内置） ===
Int, Float,     // 赋值时自动值复制，两个值完全独立
Bool, Char      // 不是 Dup，是编译器对原语的内置处理

// === Dup（浅拷贝：复制句柄，共享底层数据） ===
&T              // 零大小读取令牌，复制令牌 = 多个视角指向同一数据
ref T           // Rc/Arc 复制 = 引用计数+1，共享堆数据

// === Linear ===
&mut T          // 零大小写入令牌，Linear（独占，不可复制）

// === Clone（显式深复制） ===
value.clone()   // 创建独立副本，修改不影响原值
```

### A.4 Borrow Token Quick Reference

```
// === 借用令牌 ===
&T              // 零大小编译期读令牌，冻结源数据 → Dup（可复制）
&mut T          // 零大小编译期写令牌，独占读写 → Linear（不可复制）

// 调用端自动选择
// 1. 实参后续还有使用 → 创建令牌
// 2. 实参后续不再使用 → Move
// 3. 优先匹配：&T < &mut T < Move

// 令牌传播
// ✅ 可返回、可存结构体、可被闭包捕获
// ❌ 不可跨任务（令牌未实现跨任务传递）
```
