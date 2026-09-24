# Type System Specification

This document defines the type system specification of the YaoXiang programming language, including
primitive types, composite types, generics, and trait.

---

## Chapter 0: Theoretical Foundations

### 0.1 Curry-Howard Correspondence

The Curry-Howard correspondence is the theoretical foundation of the YaoXiang type system. It
reveals the deep correspondence between a programming language's type system and mathematical logic:

| Logic                                          | Programming Language                            |
| ---------------------------------------------- | ----------------------------------------------- |
| Proposition \(P\)                              | Type `Type`                                     |
| Proof \(p: P\)                                 | Program `x: T = ...`                            |
| Implication \(P \rightarrow Q\)                | Function type `(P) -> Q`                        |
| Conjunction \(P \wedge Q\)                     | Product type `{ a: P, b: Q }`                   |
| Disjunction \(P \vee Q\)                       | Sum type `{ a(P) \| b(Q) }`                     |
| Universal quantification \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...`                     |
| Verum \(\top\)                                 | `Void` (Unit, with a default value)             |
| Falsum \(\bot\)                                | `Never` (zero constructors, uninhabited)        |
| Type universe \(Type_n : Type_{n+1}\)          | Universe hierarchy (prevents Russell's paradox) |
| case analysis                                  | Type-level `match`                              |

> **Note**: Type-level `match` is case analysis, not mathematical induction. Induction requires
> type-level recursive functions + compiler termination checking.

### 0.2 Types as Propositions, Programs as Proofs

In YaoXiang, this correspondence is a first-class design principle:

- **Terminating type-level computation corresponds to correct constructive proofs**. YaoXiang's type
  family (e.g., `Add`'s case analysis + recursive calls on `Nat`) is essentially the type-level
  encoding of mathematical induction—provided the compiler can perform termination checks.
- **Type checking is proof verification**. When a program passes type checking, it is as if a
  logical proposition has been constructively proven.

### 0.3 Impact on Language Design

Concrete manifestations of the Curry-Howard correspondence in YaoXiang:

1. **Universe hierarchy** (RFC-010): `Type₀ : Type₁ : Type₂ ...` avoids the logical paradox
   (Girard's paradox) caused by `Type: Type`
2. **Type family** (RFC-011): The type-level case analysis + recursive calls of natural numbers
   `Nat(Zero/Succ)` correspond to the Peano axioms—provided the compiler performs termination checks
3. **Conditional type** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` corresponds to case
   disjunction in logic
4. **Value-dependent type** (RFC-011): `Array: (T: Type, N: Int) -> Type` corresponds to bounded
   quantification of "for every integer N, there exists a type"

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

> **Design Note**: Although RFC-010 proposes a unified model of "everything is assignment"
> (`name: type = value`), at the syntactic level, types and values still need to be distinguished.
> In the compiler implementation, `Type` and `Expr` are two independent AST enums (`ast.rs:406` and
> `ast.rs:25`). `TypeExpr`, as a BNF placeholder, corresponds to the `Type` enum in the
> implementation, indicating "this position expects a type".

---

## Chapter 2: Primitive Types

### 2.1 Primitive Types

| Type     | Logical Counterpart | Description                                                                               | Default Size |
| -------- | ------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| `Type`   | —                   | Meta type                                                                                 | 0 bytes      |
| `Never`  | ⊥ (falsum/empty)    | Zero constructors, no values. Divergence/panic return type. `Never <: T` holds for any T. | 0 bytes      |
| `Void`   | ⊤ (verum/Unit)      | Has a default void value, zero-field product type. `x: Void = <default>` is legal.        | 0 bytes      |
| `Bool`   | —                   | Boolean value: `true` / `false`                                                           | 1 byte       |
| `Int`    | —                   | Signed integer                                                                            | 8 bytes      |
| `Uint`   | —                   | Unsigned integer                                                                          | 8 bytes      |
| `Float`  | —                   | Floating-point number                                                                     | 8 bytes      |
| `String` | —                   | UTF-8 string                                                                              | variable     |
| `Char`   | —                   | Unicode character                                                                         | 4 bytes      |
| `Bytes`  | —                   | Raw bytes                                                                                 | variable     |

Integers with bit width: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`. Floats with bit width:
`Float32`, `Float64`.

### 2.2 Never and Void: ⊥ and ⊤

`Never` and `Void` are the logical primitives of the type system—corresponding to falsum (⊥) and
verum (⊤) respectively.

**Never (⊥, falsum/empty type)** — three non-negotiable properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`.
   `x: Never = ...` has nothing on the right to write.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. `assert(false)` returns `Never`,
   and subsequent code passes type checking (though it never actually executes).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis and `match` branch confluence.

`Never` is a built-in type name (registered through the same path as `Int`/`Bool`), not a keyword.

**Void (⊤, verum/Unit)** — exactly one inhabitant (the default void value). `Void` is the identity
element of the zero-field product type. `x: Void = <default>` is legal. A block's value is given by
the **tail expression** (an empty block `{}` is `Void`), see
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) for details.

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
- An interface name written in the type body indicates implementation of that interface

> **Namespace ownership**: The `Type.name` prefix (e.g., `Point.draw`) means the function belongs to
> `Point`'s namespace. It does not trigger any implicit binding. For the `.` call syntax like
> `p.draw()` to work, an explicit binding is required: `Point.draw = draw[0]`. See RFC-004 and
> RFC-010 for details.

#### 3.1.1 Field Default Values

Type fields can specify default values, which are optional during construction:

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

- `field: Type = expression` -> Has a default value, optional during construction
- `field: Type` -> No default value, required during construction

#### 3.1.2 Built-in Bindings

Methods can be bound directly within a type's body:

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

### 3.2 Interface Type

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Syntax**: An interface is a record type whose fields are all function types.

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

**Interface implementation**: A type implements interfaces by listing the interface names at the end
of its definition.

```yaoxiang
// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // 实现 Drawable 接口
    Serializable     // 实现 Serializable 接口
}
```

**Direct interface assignment**: A concrete type can be directly assigned to an interface type
variable (structural subtyping).

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

**Compile-time optimization strategy**:

| Scenario                           | Inference Result           | Call Method                 |
| ---------------------------------- | -------------------------- | --------------------------- |
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value              | Unknown                    | vtable                      |
| Heterogeneous collection           | Multiple types             | vtable                      |

**Coherence and orphan rules (not applicable, closing note)**: YaoXiang's interfaces are structural
types (interface = record whose fields are all function types), not nominal trait—there is no
cross-crate/module "who can implement for whom" ownership question, and Rust-style orphan rules and
coherence checks have no applicable targets (see ruling record in RFC-011 §2.1). The corresponding
guarantee in the structural world is **duplicate implementation rejection**: duplicate definitions
of the same method signature on a type produce a compile error (RFC-011a §3, override prohibited;
overloading legal).

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

Generic parameters are part of the function type, using the same `()` syntax as regular parameters:

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

In a generic type definition, `(T: Type)` is the type constructor's parameter signature, and
`-> Type` is the return type:

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

#### 4.1.1 Container Types

Container types are generic type constructors, not built-in primitives—they receive the same
treatment as user-defined generics and are processed through the unified generic instantiation path.
The ownership of length information is the fundamental distinction among the three container
concepts:

| Type          | Length        | Semantics                             | Foundation                               |
| ------------- | ------------- | ------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | Fixed-length array (const generic N)  | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | Runtime-length raw buffer, growable   | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type (growable list) | Library: `{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | Runtime value | Key-value mapping                     | `HeapValue::Dict`                        |

> `List(T)` is a **standard library type, not a compiler primitive**: defined by YaoXiang itself in
> `std.list`, receiving the same treatment as user-defined generic records. All growable-semantics
> strategies (when to expand, by how much, whether to share) live in the library; the compiler is
> not involved. `Vec(T)` is the minimal foundation primitive it depends on.
>
> Set(T) has been removed: no literal, no runtime representation, no std.set. When the need arises,
> complete it following the Dict pattern.

Key rules:

- **Literal target is determined by context**: A bare `[...]` literal annotated with `List(T)` lands
  as a growable list; an `Array(T, N)` annotation applied directly to a literal lands as a
  fixed-length array. Target validation: element count == N, element type compatible with T;
  otherwise compile-time E1002; when N is a symbolic constant (const parameter), count validation is
  deferred to the refinement type phase.
- **Implicit List→Array conversion is prohibited**: fixed-length property is guaranteed by the type
  layer—push only accepts `List(A)` receivers.
- **Performance hierarchy**: from bottom to top, performance decreases and flexibility increases:
  `Array` > `Vec` > `List`.
- **Index failure contract** (runtime error is a transitional state, target state is compile-time
  refinement coverage via value-dependent types, see §8.4):
  - Index out of bounds (including negative index) → `E6003`
  - Dict missing key → `E6008`
- **membership `in` predicate**: returns `Bool` without error, the right operand covers
  List/Array/Dict(keys)/Tuple/String/Range. A first-class Hoare predicate, serving as the foundation
  for propositions provable at compile time by refinement types.

In generic functions, type parameters are also declared in the signature, and the compiler
automatically infers them from arguments:

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

The field list of a generic type definition **automatically generates a constructor**: each field
corresponds to a construction parameter, with the field name as the parameter name; fields with
default values can be omitted during construction, fields without default values are required.
Function-typed fields (methods) do not generate construction parameters.

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

**Calling rules** (single parentheses, position-by-position match against declared parameters, left
to right):

1. Arguments attempt to match the type-declared parameters position by position: the `Type` position
   accepts type arguments, compile-time value parameter positions (e.g., `Int`) accept compile-time
   constants.
2. If there exists a successful match at a compile-time value parameter position (partial match),
   process as type construction: check all parameter positions one by one; when reporting an error,
   report the **first mismatched/missing parameter** in declaration order.
3. If the arguments do not correspond to the declared parameters at all (all are values, no
   compile-time value parameter position can match), process as construction parameters: fill
   positionally by field order, and type parameters are auto-unpacked from element types.

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

**Type inference**: The type parameters of a generic type constructor are auto-unpacked from
construction parameter elements (`Container(42, 43)` → T=Int); the type parameters of a generic
function are auto-unpacked from argument types (`map(numbers, f)` → T=Int, R=String, see §4.1). When
unpacking fails, explicit filling is required.

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

> **Constraint resolution source (RFC-011b)**: Operator constraint names (`Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`) are resolved by querying the interface
> implementation registry—`T: Add` ≜ registered `Add(T, T, T)` instantiation; `Equal` additionally
> has structural derivation (records with all fields comparable are automatically comparable). Names
> like `Zero` / `One` / `PartialOrd` have no defined source yet and are dangling constraint names.

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

### 6.1 Associated Type Definition

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

### 6.2 Generic Associated Type (GAT)

```yaoxiang
// 更复杂的关联类型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 关联类型也是泛型的
    iter: () -> IteratorType
}
```

---

## Chapter 7: Compile-Time Generics

### 7.1 Compile-Time Value Parameters

```
LiteralType   ::= Identifier ':' Int          // 编译期常量（候选）
```

> The criterion is **being referenced at a type position**, not "annotated with a concrete type": in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters (neither appears at
> any type position).

**Terminology**: A generic parameter annotated with a concrete type other than `Type` (e.g., `Int`)
is called a **compile-time value parameter candidate**; whether it becomes a compile-time value
parameter depends on whether its value is referenced at a type position (value-dependence). **No
`const` keyword is needed** (the implementation internally used "const generic" to refer to it, but
documentation uniformly uses "compile-time value parameter").

**Determination rules (two steps)**:

1. **Form coarse filter**: The parameter is annotated with a concrete type other than `Type`
   (`Int`/`Bool`/`Float`) → candidate.
2. **Usage precise filter**: The candidate name appears at a **type position** (type body field
   types, inner `Fn` parameter types, `Assert` predicates, `Array(T, N)` type construction argument
   positions) → true compile-time value parameter; otherwise **runtime value parameter**.

| Form                                                       | Determination                             | Reason                                      |
| ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters          | Only appears at value position              |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter       | N is at type construction argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter       | N is the type of inner parameter k          |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through → runtime value parameter | N is not referenced in the type body        |

**Core design**: Use the compile-time value parameter `(N: Int)` together with the value parameter
`(k: N)` to distinguish compile-time constants from runtime values. A falling-through candidate
(form is a candidate, but usage does not hit) degrades to a runtime value parameter—this applies to
both the function-level and type-constructor paths.

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

### 7.2 Compile-Time Constant Arrays

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

### 8.3 Assert Refinement Type and assert Statement

`assert` and `Assert` are two sides of the same refinement primitive—selected automatically by the
dispatch pipeline based on "whether the predicate's free variables are reachable at compile time".

**Core signature**: `assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**Dispatch rules**:

| Criterion                                                                             | Mode        | Behavior                                                                                   |
| ------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------ |
| All free variables known at compile time (generic parameters, compile-time constants) | CompileTime | Enter proof pipeline: true → erased to Void, false → compile error (Never uninhabitable)   |
| Runtime free variables exist (function parameters, external inputs)                   | Runtime     | Insert runtime Bool check, inject refinement fact into the flow-sensitive assumption set Γ |

**Flow-sensitive assumption set Γ**:

The compiler maintains a set of known propositions for each control flow point:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 传播
mut x = x - 5       // Γ = {}  ← mut kill set：旧假设失效
```

After a `mut` variable is assigned, all assumptions involving that variable are removed (kill set).
When branches merge, Γ is the intersection of each branch's Γ.

### 8.4 Terminates: Termination Measure Predicate

`Terminates` is a **built-in predicate**, belonging to the core primitives alongside `Int` and
`Never` (built-in name, not a keyword). It binds a **measure** to a piece of computation, declaring
that the computation terminates, and provides the witness of termination.

**Forms**: Two arities, the same predicate:

| Form                    | Anchor                            | Purpose                                                                                                             |
| ----------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The name of the enclosing binding | Default form—self-recursive functions, loops                                                                        |
| `Terminates(FnType, m)` | Explicit function type            | When measure ownership needs to be explicit (measure defined elsewhere, same measure serving multiple computations) |

```yaoxiang
// 测度：普通函数，可单测、可复用，不参与运行时
gcd_measure: (a: Int, b: Int) -> Int = { b }

// 二元形态：测度定义在别处，显式指明归属
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// 一元形态：锚点即绑定名，测度是作用域内的表达式
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**Semantics**: What `Terminates(m)` refines is the value type of the computation annotated at its
type position. The obligation falls on that computation—on a function, on each recursive call site;
on a loop, on the back edge—in both cases, "the measure of the next state is strictly less than the
measure of the current state", and is judged under the path guards at that point.

The call-site obligation is `m(callee_args) < m(caller_args)`; the back-edge obligation is
`m(next iteration) < m(current iteration)`. The two are identical in form.

> **Why loops are covered**: Loops are anonymous constructs, usually not referable. The binding name
> is the name—the `acc` in `acc: Terminates(n - i) = while ...` provides the anchor, so the loop
> becomes referable. This is the reason the unary form of `Terminates` acts on loops.

**Measure**: Does not restrict the return type (does not force natural numbers); the "strictly
decreasing" on it is given by the well-order available on that type. Whether the measure is
well-founded (e.g., whether the returned `Int` is `>= 0`) is an **independent obligation**, judged
by the compile-time proof pipeline in the same way as the decrease obligation.

**Trigger**: Termination checking is triggered by **refinement types**—once a type is refined, it
enters verification mode. Plain types that are not refined (e.g., bare `while` loops, functions
without refinement signatures) do not enter verification mode and generate no termination
obligations.

**Automatic exploration first**: The compiler first automatically explores measures (four templates:
linear rank function, predicate violation count, bounded increase/decrease, multiplicative scaling),
and only when exploration fails does it require an explicit `Terminates`.

**Runtime representation**: A pure compile-time entity, erased along with the witness, not present
in the runtime binary.

> See the complete design in
> [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md) (semantics) and
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md) (landing mechanism).

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

**Syntax**: A type intersection `A & B` represents a type that satisfies both A and B.

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

YaoXiang only has one type property to distinguish: linear vs copyable. It is automatically inferred
by the compiler.

### 11.1 Move (Default Ownership Transfer)

All types follow Move semantics by default. Assignment, parameter passing, return = ownership
transfer.

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move，p 不可再读
```

### 11.2 Dup (Shallow Copy: Copy Handle, Share Data)

**The Dup property is used for reference/token types**. Assignment of a Dup type = shallow
copy—copies the handle/token, the underlying data is shared. Multiple holders point to the same
block of data.

| Type            | Property | Description                                                                        |
| --------------- | -------- | ---------------------------------------------------------------------------------- |
| `&T`            | Dup      | Zero-size read token, copying the token = multiple views pointing to the same data |
| `ref T`         | Dup      | Rc/Arc copy = reference count +1, sharing heap data                                |
| `&mut T`        | Linear   | Zero-size write token, exclusive, non-copyable                                     |
| All other types | Move     | Default ownership transfer                                                         |

**Primitive value types** (Int, Float, Bool, Char) are special handling built into the compiler:
automatic value copy on assignment, two values are completely independent. This is the compiler's
native behavior, not part of the Dup type property.

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

### 11.3 Clone (Explicit Deep Copy) and Its Relationship with Dup

**Clone** is an explicit deep copy interface. All types can implement Clone, providing a `.clone()`
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

|                         | Dup                                                        | Clone                                            |
| ----------------------- | ---------------------------------------------------------- | ------------------------------------------------ |
| **Semantics**           | Shallow copy: copy handle/token, underlying data is shared | Deep copy: create a complete independent replica |
| **Invocation**          | Implicit (automatic on assignment/parameter passing)       | Explicit (`.clone()`)                            |
| **Modification effect** | Mutually affect each other (share underlying data)         | No mutual influence (independent replicas)       |
| **Applicable types**    | `&T` token, `ref T`                                        | Any type implementing the Clone interface        |
| **Cost**                | Zero overhead (tokens are zero-size types)                 | Depends on the type                              |

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

- Dup is used for token/reference types, solving the problem of "multiple views on the same data".
- Clone is used for scenarios requiring an independent copy; explicit invocation keeps the cost
  visible.
- The copy of primitive value types (Int/Float/Bool/Char) is the compiler's built-in behavior, not
  part of Dup.
- Most custom types default to Move, zero-copy and high-performance.

## Chapter 12: Borrow Token Types

### 12.1 Core Concepts

`&T` and `&mut T` are **zero-size compile-time token types**. They are not "references" but
"type-level proofs of access permission".

```
&T      →  Zero size, freezes source data (prohibits WriteToken acquisition during this period),
          under freeze guarantee multiple read-only accesses are safe → Dup (copyable)
&mut T  →  Zero size, exclusive read-write (prohibits any other token),
          under exclusive access copying is meaningless → Linear (not Dup)
```

**Key properties**:

- Tokens are **ordinary types**, following the same scoping rules as all other types.
- No lifetime annotation `'a` is required.
- No dedicated borrow checker is required—type properties (Dup/Linear) naturally derive permissions.
- Completely disappear after compilation, zero runtime overhead.

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

**Closures do not capture, context is fixed at the creation point**—closures only eat their own
parameters; when external data is needed, the value is fixed into the closure at the creation point
via currying:

```yaoxiang
// ✅ 上下文经柯里化固化：threshold 是参数，gt_point(threshold) 在创建点把值固化进闭包
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> Note: After a closure (function value) escapes, the scope at its definition may already be dead,
> so it must not implicitly capture outer variables; but the call-site (creation point) scope is
> guaranteed to be alive, so fixing context as values into the closure at that point is safe.

### 12.4 Automatic Borrow Selection

The call-side compiler automatically selects by the following priority:

```
1. If the argument is still used later → Prefer to create a token (&T or &mut T, according to method signature)
2. If the argument is not used later → Move
3. Priority match order: &T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print 的参数类型为 &Point → 编译器创建 &Point 令牌
p.shift(1.0, 1.0)  // shift 的参数类型为 &mut Point → 编译器创建 &mut Point 令牌
p2 = p             // 后续不再使用 → Move
```

**Method receiver follows signature semantics** (same as RFC-011a receiver spelling convention):
receiver is `&T` → read-only borrow token; `&mut T` → mutable borrow token; by value → Move
(consuming receiver). Borrow tokens produced at call sites are released when the call ends
(transient, §12.5 interval semantics); the interface's borrow receiver is explicitly declared as
`&Self` by the interface author, and after `Self ↦ impl type` substitution, the impl signature must
be completely consistent with the interface (RFC-011a §3).

### 12.5 Token Conflict Detection

Token conflict detection is a **borrow Hoare proposition** (RFC-009a), not an independent
flow-sensitive analysis. The compiler automatically generates borrow propositions (`borrow_conflict`
/ `use_after_move` / `use_after_drop` / `mut_violation`) and feeds them into the proof pipeline for
verification; token liveness is the interval `[created_at, last_use]` (see RFC-009a § Reverse BFS
Liveness Analysis):

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

Users never come into contact with brands. The compiler internally assigns each token a compile-time
unique identifier:

```
What the user sees         Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-counterfeiting**: Tokens can only be obtained from the owner capsule, not constructed out
  of thin air.
- **Association tracking**: The `&Float` derived from field access carries a derived brand
  (`#N.field_x`), and the compiler can trace it back to the parent token.
- **Conflict detection**: WriteToken and derived ReadToken from the same source cannot be active
  simultaneously.

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

### 12.7 Token Sum Type

```
&BorrowToken ::= &T          // ReadToken (freeze source data → Dup safe)
               | &mut T      // WriteToken (exclusive read-write → Linear)
```

### 12.8 Borrow Token vs ref

|                 | `&T` / `&mut T`                                              | `ref`                                    |
| --------------- | ------------------------------------------------------------ | ---------------------------------------- |
| What it does    | Take a look / modify in place                                | Shared holding                           |
| Range           | Follows the scope of the token value                         | Cross-scope                              |
| Cost            | Zero overhead (zero-size type, disappears after compilation) | Rc or Arc (compiler selects)             |
| Escape          | Yes (tokens propagate with return values/structs)            | Originally meant for escaping            |
| Cross-task      | No (tokens have not implemented cross-task passing)          | Yes (compiler automatically selects Arc) |
| Cycle detection | Not involved                                                 | Silent within task, lint across tasks    |

> Note (undefined): How to read content after ref is created (dereference/method/automatic) has not
> yet been defined in the specification; the current implementation reports E1052 for `*a`. To be
> added to this section once defined.

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

// === 终止测度（内置谓词，见 §8.4） ===

// 一元：锚点即绑定名（自递归函数、循环）
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// 二元：显式指明测度归属（测度定义在别处）
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

### A.3 Type Property Quick Reference

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
