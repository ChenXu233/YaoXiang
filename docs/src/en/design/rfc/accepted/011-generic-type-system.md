---
title: 'RFC-011: Generics System Design - Zero-Cost Abstractions and Macro Replacement'
status: 'Accepted'
author: 'Chen Xu'
updated:
  '2026-07-15 (Type body code blocks + compile-time specifications + effect seeds implemented)'
issue: '#128'
issues_impl:
  - '#45'
  - '#46'
  - '#73'
  - '#90'
  - '#96'
  - '#40'
  - '#151'
pr_impl:
  - '#122'
---

# RFC-011: Generics System Design - Zero-Cost Abstractions and Macro Replacement

## Abstract

This document defines the **generics system design** of the YaoXiang language, achieving zero-cost
abstractions through powerful generic capabilities, reducing dependence on macros via compile-time
optimizations, and providing a dead code elimination mechanism.

**Core Design**:

- **Unified signature syntax**: `(T: Type, R: Type) -> ...` unifies generic parameters and regular
  parameters
- **Type self-description mechanism**: `Type` is a language-level special entity; `Type` positions
  in signatures can be automatically inferred and filled
- **Type constraints**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated types**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time generics**: `N: Int` generic value parameters, compile-time constant instantiation
- **Conditional types**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type
  families

**Value**:

- Zero-cost abstractions: Compile-time monomorphization, no runtime overhead
- Dead code elimination: Instantiation graph analysis + LLVM optimization
- Macro replacement: Generics replace 90% of macro usage scenarios
- Type safety: Compile-time checks, IDE-friendly
- **Explicit is better than implicit**: `Type` self-description, automatic compiler inference

## Reference Documents

The design of this document is based on the following documents:

| Document                                                                                                   | Relationship              | Description                                                                      |
| ---------------------------------------------------------------------------------------------------------- | ------------------------- | -------------------------------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax foundation**     | Generic syntax integrated with the unified `name: type = value` model            |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call syntax**           | Section 6: Generic call syntax—unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md)                                              | **Type system**           | Natural combination of Move semantics and generics                               |
| [RFC-024: spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution model**       | DAG analysis and generic type checking                                           |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)                                      | **Compiler architecture** | Generic monomorphization and compile-time optimization strategies                |
| [Type Universe Thought](../reference/plan/ongoing/type-universe-thought.md)                                | **Theoretical core**      | Type universe hierarchy model and value-dependent type design                    |
| [RFC-027: Compile-time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination checking**  | Automatic metric synthesis and compile-time evaluation safety guarantees         |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generics system is built upon the **type universe thought**, a mental model that unifies
all concepts in the language into a layered structure. The core innovation is elevating
**value-dependent types** to first-class citizens at the Type2 layer.

### What are value-dependent types?

**Value-dependent types** are types that depend on one or more **values** (rather than only on other
types). These values can be evaluated at compile time, thereby providing type safety guarantees at
the compilation stage.

```yaoxiang
# 传统泛型：类型参数
List: (T: Type) -> Type

# 值依赖类型：值参数
Vec: (n: Int) -> Type  # 向量类型依赖于长度值 n
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 矩阵类型依赖于行数和列数
```

### Core advantages of value-dependent types

Compared to traditional generics, YaoXiang's value-dependent types have the following core
advantages:

| Feature                 | Traditional generics (C++/Rust)               | YaoXiang value-dependent types                            |
| ----------------------- | --------------------------------------------- | --------------------------------------------------------- |
| Values types depend on  | Only type parameters                          | Any value, including function call results                |
| Compile-time evaluation | C++ template manual specialization, Rust none | Automatic compile-time evaluation, termination guaranteed |
| Type-level computation  | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                     |
| Type safety             | C++ none, Rust limited                        | Complete type safety, compile-time checks                 |
| Dimension verification  | Runtime check or manual specialization        | Compile-time dimension verification, no runtime overhead  |

### Type universe hierarchy and value-dependent types

The type universe thought divides language concepts into different layers based on semantic roles,
with value-dependent types located at the **Type2 layer**:

| Layer     | Role                                                  | Examples                                                                                             |
| --------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Type-1    | Value                                                 | `42`, `factorial(5)`, functions themselves                                                           |
| Type0     | Meta-type keyword                                     | `Type`                                                                                               |
| Type1     | Concrete type                                         | `Int`, `String`, `Vec(3)`                                                                            |
| **Type2** | **Functions/Type constructors/Value-dependent types** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key design**: Functions, type constructors, and value-dependent types at the Type2 layer use
**unified syntax**, all in the form of `(params) -> result`:

- Regular functions: `(Int, Int) -> Int` → return value is a value
- Type constructors: `(T: Type) -> Type` → return value is a type
- Value-dependent types: `(n: Int) -> Type` → return value is a type, but depends on value
  parameters

> **Curry-Howard Isomorphism**: This unification is not a coincidence. The Curry-Howard isomorphism
> states "types are propositions, programs are proofs"—function type `A → B` corresponds to the
> logical implication "if A then B", generics `(T: Type) -> Type` corresponds to universal
> quantification "for all types T", and value-dependent types `(n: Int) -> Type` corresponds to "for
> each integer n there exists a type". YaoXiang unifies functions, type constructors, and
> value-dependent types at the Type2 layer, essentially unifying "proof" and "computation" as the
> same concept—**constructive proof**. This is the direct embodiment of the Curry-Howard isomorphism
> in language design: one form (`(params) -> result`) simultaneously carries logical propositions
> and computational processes.

### Compile-time Determinism Guarantee

YaoXiang's type universe thought requires: **Everything at the Type level is determined at compile
time**.

```yaoxiang
# 编译期维度验证示例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # 编译期检查：维度必须为正
    _assert: Assert(Rows > 0),
    _assert: Assert(Cols > 0),
}

# 创建 3x3 单位矩阵 - 编译期完成
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# 编译期计算：factorial(3) = 6，向量大小在编译期确定
vec: Vec(factorial(3)) = Vec(6)()
```

The compiler automatically:

1. Detects function calls in type positions
2. Performs compile-time termination checks on functions (see the termination check mechanism below)
3. Performs evaluation at compile time
4. Embeds results into the generated type

### Application scenarios for value-dependent types

#### Compile-time dimension verification

```yaoxiang
# 矩阵乘法：编译期验证维度匹配
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # 编译期检查：a.Cols == b.Rows，否则编译错误
    result = Matrix(T, Rows, M)()
    # ...
}

# 错误在编译期捕获：
# multiply(matrix_2x3, matrix_4x2)  # 编译错误：2 != 4
```

#### Type-safe array sizes

```yaoxiang
# 数组大小是编译期常量
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N 是编译期常量，可以用于类型级计算
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3（编译期已知）
```

#### Conditional types

```yaoxiang
# 类型级If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 类型族
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,
}
```

#### Generic functions

```yaoxiang
# map: 泛型函数，类型参数 T, R 在编译期确定
map: (T: Type, R: Type) -> (
    (list: List(T), f: (x: T) -> R) -> List(R)
) = (list, f) => {
    result = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用时完全透明，类型自动推导
numbers = List(Int)()   # 勘误：值构造两层形式（见 §9.1）；元素用 push 填充
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # 推导为 map[Int, Int]
```

### Comparison with other languages

| Feature                                                     | C++ Templates          | Rust Generics | Haskell GADT   | **YaoXiang**                              |
| ----------------------------------------------------------- | ---------------------- | ------------- | -------------- | ----------------------------------------- |
| Type parameters                                             | ✅                     | ✅            | ✅             | ✅                                        |
| Value-dependent types                                       | ❌                     | ❌            | ✅             | ✅                                        |
| Compile-time evaluation                                     | Template instantiation | ❌            | ✅             | ✅                                        |
| Termination guarantee                                       | ❌                     | ❌            | ❌ (dangerous) | ✅ (automatic metric synthesis, RFC-027)  |
| Type safety                                                 | ❌ (macro expansion)   | ✅            | ✅             | ✅                                        |
| Unified syntax                                              | ❌                     | ❌            | ❌             | ✅                                        |
| Compile-time dimension verification                         | Manual specialization  | Runtime check | Type families  | Compile-time automatic verification       |
| Semi-automatic termination annotation (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (only fully automatic at compile time) |

### Termination check mechanism (unified with RFC-027)

The compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the
type system will fall into infinite loops. Termination checks are **fully automatically** completed
by RFC-027's compile-time proof pipeline—the compiler automatically synthesizes metrics; recursive
calls/loops that can be proven pass, those that cannot be proven directly report compile errors.
**No room for semi-automatic annotations**: RFC-022's `//! decreases`, `/*! invariant !*/` have been
deprecated along with RFC-022; specifications are the type annotations themselves.

#### Termination check for recursive functions

Before compile-time evaluation, the compiler checks whether the parameters of recursive calls
strictly decrease on each recursive path (RFC-027 §6.7). No specification comments are required:

```yaoxiang
# 编译期阶乘：无 //! requires/ensures/decreases，编译器自动分析
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # 编译器分析：n-1 < n → 递减 → 终止
}

# 使用：在类型位置调用，编译器先验证终止再求值
vec: Vec(factorial(5)) = Vec(120)()  # 编译期求值 factorial(5) = 120
```

| Scenario                                              | Behavior                      |
| ----------------------------------------------------- | ----------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation       |
| Not decreasing / cannot determine decrease            | Compile error                 |
| Runtime calls (non-type position)                     | No termination check required |

#### Termination check for loops

Loops do not require `: Invariant(...)` or `: decreases(...)` annotations. Refined type annotations
on variables (such as `UpTo(n)`) simultaneously provide loop invariants and metric bounds. The
compiler tries four metric synthesis strategies in priority order, stopping when one is found
(RFC-027 §7):

1. **Linear rank function automatic synthesis**—extract variable bounds from type annotations,
   enumerate linear combinations, SMT verifies m ≥ 0 and all paths m' < m
2. **Predicate violation counting** (experimental)—extract violation_count from target type
   definitions (such as `Sorted`), covering adjacent swaps/moves
3. **Bounded increment/decrement patterns**—`v += const` → metric `upper - v` (degenerate form of
   strategy 1, fastest path)
4. **Multiplicative scaling metric template**—`v *= const` (const > 1) → metric
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # 类型标注给出上界 arr.len 与下界 0
    while i < arr.len {
        # 编译器自动推导：度量 arr.len - i，每次迭代严格递减 1 → 终止得证
        s += arr[i]; i += 1
    }
    return s
}
```

#### Termination check workflow

```
┌─────────────────────────────────────────────────────────────┐
│  Type checking phase                                          │
│  Encounter function call in type position (e.g., Vec(factorial(5))) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination check (RFC-027 proof pipeline, fully automatic) │
│     - Recursive functions: check parameters strictly decrease on each recursive path │
│     - Loops: four metric synthesis strategies (linear rank/violation count/bounded pattern/  │
│       multiplicative scaling), SMT verifies decrease                                │
│     - Cannot prove → Compile error (hard boundary, no semi-automatic annotation fallback)        │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time evaluation (executed by built-in interpreter)                           │
│     - Pure functions: directly evaluate                                       │
│     - Side effects: Compile error (type positions must be side-effect free)                │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result embedded into type                                            │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → concrete type                        │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensures compile-time evaluation necessarily terminates, avoiding the type system
  falling into infinite loops
- **Unification**: Termination checks and correctness verification (VC generation) share the same
  compile-time proof pipeline (RFC-027), no independent specification syntax
- **Fully automatic**: The compiler automatically synthesizes metrics from type annotations, passes
  if provable, errors if not—does not rely on programmers manually writing `decreases`

## Motivation

### Why do we need a strong generics system?

Current mainstream languages have limitations in their generics:

| Language     | Generics capability       | Problem                                                                                     |
| ------------ | ------------------------- | ------------------------------------------------------------------------------------------- |
| Java         | Bounded types             | Compile-time monomorphization, no generic specialization                                    |
| C#           | Generic constraints       | Runtime type checking, performance overhead                                                 |
| Rust         | Generics + Trait          | Complex Trait system, steep learning curve                                                  |
| C++          | Templates                 | Complex template specialization, poor compilation error messages                            |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, compile-time dimension verification, termination guaranteed** |

### Core contradictions

1. **Performance vs flexibility**: Runtime flexibility vs compile-time optimization
2. **Complex vs simple**: Powerful type system vs usability
3. **Macros vs generics**: Macro code generation vs generic type safety
4. **Value dependency vs type safety**: Traditional generics cannot verify dimensions at compile
   time

### Core advantages of value-dependent types

YaoXiang's **value-dependent types** are a core advantage over traditional generics:

| Advantage                   | Description                                                                                                        |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Types depend on values**  | `Vec: (n: Int) -> Type` makes types depend on specific values                                                      |
| **Compile-time evaluation** | Function calls in type positions are evaluated at compile time, results directly embedded into types               |
| **Dimension verification**  | `Matrix(Float, 3, 3)` verifies matrix dimensions at compile time                                                   |
| **Type-level computation**  | Conditional types like `If`, `Match` support type-level computation                                                |
| **Termination guarantee**   | Compile-time termination check (automatic metric synthesis) ensures compile-time evaluation necessarily terminates |

```yaoxiang
# C++/Rust 无法做到的编译期验证
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# 编译期计算：factorial(3) = 6, factorial(2) = 2
# 类型为 Matrix(Float, 6, 2)

# 维度不匹配在编译期捕获
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # 编译错误：2 != 3
```

### Value of the generics system

```yaoxiang
# 示例：统一API设计
# 不同容器类型的map操作

# 传统方案：每个类型单独实现
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# 泛型方案：一个泛型函数覆盖所有类型
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## Design Goals

### Core goals

1. **Zero-cost abstraction** - Generic calls equivalent to concrete type calls
2. **Dead code elimination** - Compile-time analysis, only instantiate used generics
3. **Macro replacement** - Generics replace 90% of macro usage scenarios
4. **Type safety** - Compile-time checks, no runtime type overhead
5. **IDE-friendly** - Smart hints, clear error messages
6. **Value-dependent types** - Types can depend on values, supporting compile-time dimension
   verification
7. **Compile-time evaluation safety** - Ensures compile-time evaluation terminates through
   compile-time termination checks (RFC-027 automatic metric synthesis)

### Design principles

- **Compile-time determination**: Generic parameters determined at compile time
- **Monomorphization first**: Generate concrete code, avoid virtual function calls
- **Constraint-driven**: Type constraints guide instantiation
- **Platform optimization**: Specialization supports platform-specific optimization
- **Type universe unification**: Functions/type constructors/value-dependent types unified as Type2
  layer
- **Termination guarantee**: Function calls in type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic type parameters

> **Key rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise they will
> be inferred by HM as functions.
>
> | Notation                          | Meaning                            |
> | --------------------------------- | ---------------------------------- |
> | `List: (T: Type) -> Type = {...}` | ✅ Type constructor                |
> | `List = {...}`                    | ❌ HM infers as function, not type |

```yaoxiang
# 泛型类型定义（必须有 : Type）
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
    push: (self: List(T), item: T) -> Void,   # self 只是约定名，不是关键字
    get: (self: List(T), index: Int) -> Option(T),
}

# 泛型函数（无 : Type，HM 推断为函数）
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# 泛型约束（直接表达式，单行可省略 return）
clone: (T: Clone)(value: T) -> T = value.clone()

# 多类型参数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### Generic function call syntax

#### 1.1 Unified signature syntax

```yaoxiang
# 泛型函数使用统一的 (T: Type, R: Type) 签名语法
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 多类型参数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type self-description mechanism

`Type` is a language-level special entity; the compiler naturally recognizes `Type` positions in
signatures and automatically infers and fills them from actual argument types.

```yaoxiang
# 编译器自动推断泛型参数
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         类型声明   构造调用：Int 填充 T，() 值构造

# 函数调用推断
numbers: List(Int) = List(Int)()
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# 编译器推断：T=Int, R=String
```

#### 1.3 Monomorphization

```yaoxiang
# 源代码
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = {
    result: List(R) = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用点
int_list: List(Int) = List(Int)()
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # 实例化 map[Int, Int]

string_list: List(String) = List(String)()
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # 实例化 map[String, String]

# 编译后（等价代码）
map_Int_Int: (list: List(Int), f: (Int) -> Int) -> List(Int) = {
    result: List(Int) = List(Int)()
    for x in list {
        result.push(f(x))
    }
    return result
}

map_String_String: (list: List(String), f: (String) -> String) -> List(String) = {
    result: List(String) = List(String)
    for s in list {
        result.push(f(s))
    }
    return result
}
```

#### 1.4 Explicit filling (when inference fails)

````yaoxiang
# 可推断时省略 Type 参数
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 无法推断时必须显式填充
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R

### 2. Type Constraint System

#### 2.1 Single constraint

```yaoxiang
# 基本trait定义（接口类型）
Clone: Type = {
    clone: (Self) -> Self,
}

Display: Type = {
    fmt: (Self, Formatter) -> Result,
}

Debug: Type = {
    fmt: (Self, Formatter) -> Result,
}

# 使用约束：在签名中直接声明类型约束
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
````

#### 2.2 Multiple constraints

```yaoxiang
# 多重约束语法
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# 泛型容器的排序
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # 实现排序算法
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# 函数类型约束
map: (T: Type, R: FnMut(T))(array: Array(T), f: R) -> Array(R) = {
    result: Array(R) = Array()
    for item in array {
        result.push(f(item))
    }
    return result
}

# 使用
doubled: Array(Int) = map(Array(1, 2, 3), (x: Int) => x * 2)  # 编译器推断
```

#### 2.3 Function type constraints

```yaoxiang
# 高阶函数约束
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

call_with_arg: (T: Type, U: Type, F: Fn(T) -> U)(arg: T, f: F) -> U = f(arg)

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))

# 使用示例
result: Int = call_with_arg(42, (x: Int) => x * 2)  # result = 84
composed: String = compose(
    "hello",
    (s: String) => s.to_uppercase(),
    (s: String) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

#### 2.4 Built-in marker traits: Dup and Clone

**Three types of copy semantics**:

| Type                     | Meaning                                                               | Trigger                                | Use case                          |
| ------------------------ | --------------------------------------------------------------------- | -------------------------------------- | --------------------------------- |
| **Primitive value copy** | Automatic value copy on assignment, two values completely independent | Assignment/parameter passing automatic | Int, Float, Bool, Char            |
| **Dup**                  | Shallow copy: copy handle/token, underlying data shared               | Assignment/parameter passing automatic | `&T` token, `ref T`, String/Bytes |
| **Clone**                | Deep copy: create completely independent copy                         | `value.clone()`                        | Any type implementing Clone       |

**Dup semantics**: Types implementing Dup do not transfer ownership on assignment/parameter
passing—the compiler copies the handle/token, with multiple holders pointing to the same underlying
data. This is complementary to the default Move semantics in the RFC-009 ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = Copy handle, share data (modifications affect each other)
Clone = Copy data, independent copies (modifications don't affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — Compiler built-in value copy, not part of Dup
2. Dup — Only applicable to reference/token types and internally reference-counted types
3. Clone — Explicit deep copy, any type can implement
4. Default Move — Other types maintain default Move semantics
```

**Which types are Dup**:

| Type                     | Dup     | Reason                                                            |
| ------------------------ | ------- | ----------------------------------------------------------------- |
| `&T` (borrowed token)    | ✅      | Zero-size token, copy token = multiple views point to same data   |
| `ref T`                  | ✅      | Rc/Arc copy = reference count +1, share heap data                 |
| String, Bytes            | ✅      | Internal reference counting, copy handle shares underlying buffer |
| `&mut T` (mutable token) | ❌      | Linearly exclusive, cannot copy                                   |
| struct                   | Derived | All fields Dup → struct Dup                                       |
| enum                     | Derived | All fields of all variants Dup → enum Dup                         |
| tuple                    | Derived | All elements Dup → tuple Dup                                      |
| Fn (closure)             | ❌      | Captured environment may not be Dup                               |
| `*T` (raw pointer)       | ❌      | unsafe, not participating in ownership system                     |

**Int/Float/Bool/Char are not Dup**—they are value types, and the compiler automatically copies
values on assignment (two values completely independent). This is not "shallow copy", but the
compiler's built-in handling of primitives, and does not need and should not be expressed through
the Dup type property.

```yaoxiang
# 原语值类型：编译器自动值复制（不是 Dup）
x: Int = 42
y = x          # 值复制，x 和 y 完全独立
print(x)       # ✅

# Dup：浅拷贝，复制句柄共享数据
view: &Point = &point
view2 = view    # ✅ Dup：复制令牌，两者指向同一个 point
print(view.x)   # ✅

# Clone：显式深拷贝，创建独立副本
backup = big_struct.clone()  # 显式调用

# 泛型约束
dup_use: (T: Dup) -> T = x         # T: Dup → 可以浅拷贝
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → 可以深拷贝
```

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety guarantees are fully
> automatically handled by the `ref` keyword and compiler—`ref` automatically chooses Rc or Arc,
> users do not need to understand Send/Sync.

### 3. Associated Types

#### 3.1 Associated type definitions

```yaoxiang
# Iterator trait（使用 (Item: Type) -> Type 语法）
Iterator: (Item: Type) -> Type = {
    next: (Self) -> Option(Item),
    has_next: (Self) -> Bool,
    collect: (T: Type)(Self) -> List(T),
}

# 使用
collect_all: (T: Type, I: Iterator(T))(iter: I) -> List(T) = {
    result: List(T) = List(T)
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}

# Array的Iterator实现
# 使用方法语法糖：Array.Item, Array.next, Array.has_next
Array.has_next: (T: Type)(self: Array(T)) -> Bool = {
    return self.index < self.length
}

Array.next: (T: Type)(self: Array(T)) -> Option(T) = {
    if has_next(self) {
        item = self.data[self.index]
        self.index = self.index + 1
        return Option.some(item)
    } else {
        return Option.none()
    }
}

Array.Item: (T: Type)(arr: Array(T)) -> T = {
    return arr.data[0]
}
```

#### 3.2 Generic associated types (GAT)

```yaoxiang
# 更复杂的关联类型
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# 关联类型可以是泛型的
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # 关联类型也是泛型的
    iter: (Self) -> IteratorType,
}

# 使用
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. Compile-time Generics

#### 4.1 Compile-time value parameters

> **Erratum (#296)**: The original text said "`Int` and other value parameters are compile-time
> determinable by default in generic contexts", This statement is **strictly incorrect**—in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters. Only specific type
> parameters **referenced in type positions** are compile-time value parameters. The correct
> definition is below.

**Core design**: `Type` in generic signatures marks type parameters; parameters annotated with
concrete types (`Int`/`Bool`/`Float`, etc.) are listed as **compile-time value parameter
candidates**, and whether they become compile-time value parameters depends on whether their values
are **referenced in type positions** (value dependency). No `const` keyword required.

**Determination rules (two steps)**:

1. **Form coarse filtering**: Parameters annotated with concrete types other than `Type` (e.g.,
   `Int`) → listed as candidates.
2. **Usage fine filtering**: Candidate names appearing in **type positions** (type body field types,
   inner `Fn` parameter types, `Assert` predicates, `Array(T, N)` and other type constructor
   argument positions) → confirmed as compile-time value parameters; otherwise treated as **runtime
   value parameters**.

| Notation                                                   | Determination                  | Reason                                                                 |
| ---------------------------------------------------------- | ------------------------------ | ---------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters   | Only appear in value positions, not participating in type construction |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter | N appears in `Array(T, N)` type constructor argument position          |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter | N as the type of inner parameter `k`                                   |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N drops (see below)            | N not referenced in type body, degenerates to runtime value parameter  |

> **Value dependency essence**: Compile-time value parameters are value-dependent types—only when
> values are used to **construct types**, do they need to be determined at compile time. Form
> (`: Int`) only determines candidate qualification, usage (appearing in type positions) determines
> whether they are compile-time value parameters. This is the same criterion as "function calls in
> type positions are evaluated at compile time" in the §"Compile-time Determinism Guarantee".

```yaoxiang
# ════════════════════════════════════════════════════════
# 编译期值参数：N 在类型位置（Array 长度槽）被引用
# ════════════════════════════════════════════════════════
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N 出现在类型构造实参位 → 编译期值参数
    length: N,
}

# 使用方式：factorial(5) 在类型位置求值（编译期），结果 120 嵌入类型
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120)

# ════════════════════════════════════════════════════════
# 值依赖：N 作为内层参数 k 的类型
# ════════════════════════════════════════════════════════
# N 是编译期值参数（出现在 (k: N) 的类型位）；
# k 是运行时值参数，其类型为字面量类型 N（单值类型）。
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling of dropping candidates**: Candidates annotated with concrete types but not referenced
> in type positions (such as `N` of `Foo` in the table above) degenerate to runtime value parameters
> (function-level path). Dropping candidates on the type constructor path cannot occupy runtime slot
> positions (type constructors are evaluated at compile time), and the declaration side directly
> reports error [E1094]: "N is declared as a compile-time value parameter but not referenced in the
> type body"—previously silently discarded leading to instantiation arity inconsistency, see issue
> [#297](https://github.com/ChenXu233/YaoXiang/issues/297).

#### 4.2 Compile-time computation

```yaoxiang
# ════════════════════════════════════════════════════════
# 编译期计算示例
# ════════════════════════════════════════════════════════

# 编译器在编译期计算字面量类型的函数调用
SIZE: Int = factorial(5)  # 编译期为 120

# 矩阵类型使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# 编译期维度验证
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    matrix: Matrix(T, N, N) = Matrix(T, N, N)()
    for i in 0..size {
        for j in 0..size {
            if i == j {
                matrix.data[i][j] = One::one()
            } else {
                matrix.data[i][j] = Zero::zero()
            }
        }
    }
    matrix
}

# 使用：编译期计算，生成 Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never and Void: ⊥ and ⊤ in the type system

YaoXiang's type system in the Curry-Howard isomorphism simultaneously has ⊥ (false/empty type) and ⊤
(true/Unit), carried by the two built-in type names `Never` and `Void`:

**Never (⊥)** — Three non-negotiable core properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. This is a
   meta-level property and must be built in.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. A `Never` value can be used as
   any type—this is exactly why code after `assert(false)` still passes type checking (although it
   never executes).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis.

`Never` is a built-in type name, not a keyword, the parser is insensitive to it. No empty sum and
type literal syntax is opened.

**Void (⊤, i.e., Unit)** — Has exactly one inhabitant (default void value), and is the carrier of
the true proposition "always true". `Void` is the identity of zero-field product types, `Never` is
the identity of zero-variant sum types—the two are dual. `x: Void = <default>` is legal,
`x: Never = ...` has no right side to write.

#### 4.3 Compile-time verification (standard library implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# 标准库实现：利用条件类型
# ════════════════════════════════════════════════════════

# 标准库定义
# IsTrue：值宇宙到类型宇宙的桥——Bool 真值映射为类型
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤，有值，程序继续
    false => Never,    # ⊥，无值，发散
}

# Assert：编译期精化类型原语——对 Bool 命题的类型级表述
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond 为 true  → Assert(true)  = Void    （恒真，擦除）
# cond 为 false → Assert(false) = Never   （恒假，编译错误/发散）
# cond 判不了   → 由证明管道按 dispatch 模式决定：
#                  CompileTime → Unknown，要求 prove
#                  Runtime     → 插入 check，注入 Γ 假设

# 使用方式1：在类型定义中作为约束
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # 编译期检查：N 必须大于 0（Assert 在类型位置）
    length: Assert(N > 0),
}

# 使用方式2：在表达式中使用
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# 验证：IntArray(10) 的大小等于 sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-time generic specialization

```yaoxiang
# 小数组优化：使用函数重载实现编译期泛型特化

# 通用实现
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    result = Zero::zero()
    for item in arr.data {
        result = result + item
    }
    return result
}

# N=1 特化
sum: (T: Type) -> ((arr: Array(T, 1)) -> T) = arr.data[0]

# N=2 特化
sum: (T: Type) -> ((arr: Array(T, 2)) -> T) = arr.data[0] + arr.data[1]

# 小数组循环展开（N <= 4）
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    # 编译器优化：展开循环
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. Conditional Types

> **Curry-Howard Isomorphism**: Conditional types from the Curry-Howard perspective are **case
> analysis** in logic. `Bool` type corresponds to a proposition with two possible values
> (True/False), `If` chooses different results based on the truth value of that proposition—this is
> exactly case disjunction in logic. `match C { True => T, False => E }` is actually expressing: "if
> proposition C is known to be True, the conclusion is T; if C is False, the conclusion is E".

#### 5.1 If conditional type

```yaoxiang
# 类型级If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 示例：编译期分支
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# 编译期验证（统一到 §4.3 的 Assert 定义）
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# 使用
# 类型计算：If(True, Int, String) => Int
# 类型计算：If(False, Int, String) => String
```

#### 5.2 Type families

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but **constructing a proposition about natural number addition**. `(Zero, B) => B` says
> "proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` says "if
> Add(A', B) holds, then Add(Succ(A'), B) also holds". This is the addition definition itself in
> Peano axioms. The type checker verifying this match expression passes is equivalent to verifying
> the logical consistency of this definition.

```yaoxiang
# 编译期类型转换
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,  # 默认
}

# 类型级计算
Length: (T: Type) -> Type = match T.length {
    0 => Zero,
    1 => Succ(Zero),
    2 => Succ(Succ(Zero)),
    _ => TooLong,
}

# 类型级加法（Curry-Howard：case analysis + 递归调用，需要终止性检查才是完整归纳）
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 示例：编译期计算 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. Function Overload Specialization

#### 6.1 Basic specialization

```yaoxiang
# 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array(Int)) -> Int = {
    # 编译为更高效的代码
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    # 使用SIMD指令
    return simd_sum_float(arr.data, arr.length)
}

# 通用实现
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

#### 6.2 Conditional specialization

```yaoxiang
# 完全符合RFC-010语法的特化方式：函数重载

# 具体类型特化
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# 泛型实现（编译器自动选择最优）
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用时完全透明
int_arr = Array(Int)(1, 2, 3)
float_arr = Array(Float)(1.0, 2.0, 3.0)

# 编译器自动选择最优特化
sum(int_arr)     # 选择 sum: (Array(Int)) -> Int
sum(float_arr)    # 选择 sum: (Array(Float)) -> Float
```

#### 6.3 Perfect combination of function overloading and inlining

**Key feature**: Function overloading and inlining optimization naturally combine to achieve
zero-cost abstraction.

```yaoxiang
# ======== 源代码 ========
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用
int_arr = Array(Int)(1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== 编译后（等价代码）=======
# 编译器自动选择最优特化，然后内联
result = native_sum_int(int_arr.data, int_arr.length)

# 完全等价于手写优化代码，无函数调用开销！
```

**Core advantages**:

1. **Compiler intelligent selection**

   ```yaoxiang
   sum(int_arr)      # 自动选择 sum: (Array(Int)) -> Int
   sum(float_arr)    # 自动选择 sum: (Array(Float)) -> Float
   sum(custom_arr)  # 自动选择 sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **Inlining optimization**
   - Small functions automatically inlined to call sites
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function table required

4. **Perfect fit with RFC-010**

   ```yaoxiang
   # 完全使用统一语法
   name: type = value
   # 无需impl、where等新关键字
   ```

**Practical application example**:

```yaoxiang
# 性能敏感的数值计算
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # 使用Binet公式
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# 编译器自动选择并内联
fibonacci(10)      # 选择 Int 版本，完全内联
fibonacci(10.5)    # 选择 Float 版本，使用Binet公式
```

**What does this mean?**

- ✅ **Generic specialization** → Function overloading naturally solves it
- ✅ **Performance optimization** → Inlining automatically completed
- ✅ **Code reuse** → One function name, multiple implementations
- ✅ **Zero-cost abstraction** → Compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords** → Perfectly conforms to RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation graph analysis

```rust
// 编译器内部：构建泛型实例化依赖图
struct InstantiationGraph {
    // 节点：泛型实例化
    nodes: HashMap<InstanceKey, InstanceNode>,

    // 边：使用关系
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // 泛型函数ID
    type_args: Vec<TypeId>,  // 类型参数
    const_args: Vec<ConstId>,  // Const参数
}

// 算法：可达性分析
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // 从入口点开始（main、导出函数等）
    let entry_points = find_entry_points();
    for entry in entry_points {
        dfs_visit(entry, &graph, &mut reachable);
    }

    // 未访问的实例化就是死代码
    for node in &graph.nodes {
        if !reachable.contains(node.key) {
            eliminate(node);
        }
    }
}
````

#### 7.2 Use point analysis

```yaoxiang
# 源代码分析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用点1：实例化 map(Int, Int)
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # 需要 map[Int, Int]

# 使用点2：实例化 map(String, String)
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 需要 map[String, String]

# 未使用：map[Float, Float] 等
# 这些泛型实例不会被生成

# 编译后只包含被使用的实例
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 Compile-time generic DCE

```yaoxiang
# 编译期分析：编译期泛型使用情况
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 实际使用情况
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # 两层：类型参数 + 构造参数
# 勘误：早期版本写作 Array(Int, 10)(1, 2, 3, ...)（元素直接铺开），
# 与 §9.3 的权威模式（Type(参数)(字段构造参数)/空构造）不一致，
# 统一为字段名式构造参数，见 SPEC type-system.md §4.3。
arr_100_int = Array(Int, 100)()   # 空构造，数据事后赋值

# 编译后只生成被使用的Size
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用的Size不会生成
# Array(Int, 50) 不会生成
```

#### 7.4 Cross-module DCE

```yaoxiang
# 模块A
# A.yx
pub map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 模块B
# B.yx
use A.{map}
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # 实例化 map(Int, Int)

# 模块C
# C.yx
use A.{map}
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 实例化 map(String, String)

# 编译分析：
# - 模块B使用 map[Int, Int]
# - 模块C使用 map[String, String]
# - 编译后二进制只包含这两个实例
```

#### 7.5 LLVM-level DCE

```rust
// 编译流水线
fn optimize_ir(ir: &mut IR) {
    // 1. 单态化（YaoXiang编译器）
    ir.monomorphize();

    // 2. 内联优化
    ir.inline_small_functions();

    // 3. 常量传播
    ir.constant_propagation();

    // 4. 生成LLVM IR
    let llvm_ir = ir.to_llvm();

    // 5. LLVM优化pass
    llvm_ir.add_pass(Passes::DEAD_CODE_ELIMINATION);
    llvm_ir.add_pass(Passes::INLINE_FUNCTION);
    llvm_ir.add_pass(Passes::GLOBAL_DCE);
    llvm_ir.add_pass(Passes::MERGE_FUNC);

    // 6. 运行优化
    llvm_ir.run_optimization_passes();
}
```

### 8. Macro Replacement Strategy

#### 8.1 Code generation replacement

```yaoxiang
# ❌ 宏方案：代码生成
macro_rules! impl_debug {
    ($($t:ty),*) => {
        $(impl Debug for $t {
            fn fmt(&self, f: &mut Formatter) -> Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

# ✅ 泛型方案：自动派生
# 使用函数重载方式自动派生
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# 使用
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # 自动生成调用
```

#### 8.2 DSL replacement

```yaoxiang
# ❌ 宏方案：HTML DSL
html! {
    <div class="container">
        <h1> { title } </h1>
        <ul>
            { for item in items {
                <li> { item } </li>
            }}
        </ul>
    </div>
}

# ✅ 泛型方案：类型安全构建器
Element: Type = {
    tag: String,
    attrs: HashMap(String, String),
    children: List(Element),
    text: Option(String),
}

create_element: (tag: String) -> Element = {
    return Element(tag, HashMap::new(), List::new(), None)
}

with_class: [E: Element](elem: E, class: String) -> E = {
    elem.attrs.insert("class", class)
    return elem
}

with_text: [E: Element](elem: E, text: String) -> E = {
    return E { text: Some(text), ..elem }
}

# 构建DOM
container = create_element("div")
    |> with_class("container")
    |> with_children(List::new())

title_elem = create_element("h1") |> with_text(title)
items_li = items.map((item) =>
    create_element("li") |> with_text(item)
)
root = container |> with_children(List::new() + [title_elem, ul_elem])
```

#### 8.3 Type-level programming replacement

```yaoxiang
# ❌ 宏方案：类型级计算
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ 泛型方案：条件类型
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Int, Int) => Int,
    (Float, Float) => Float,
    (Int, Float) => Float,
    (Float, Int) => Float,
    _ => TypeError,
}

# 编译期验证
AssertAddable: (A: Type, B: Type) -> Type = If(Add(A, B) != TypeError, (A, B), compile_error("Cannot add"))

# 使用
result_type = Add[Int, Float]  # 推导为 Float
```

### 9. Examples

#### 9.1 Complete generic container example

```yaoxiang
# ======== 1. 定义泛型容器 ========
# 使用 (T: Type) -> Type 语法
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,

    # 泛型方法（T 由外层 List(T) 自动带入作用域）
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. 实现泛型方法 ========
# 函数定义在 List 命名空间下（List. 前缀 = 命名空间归属）
# 要让 list.push(item) 这种 . 调用语法生效，需要显式绑定：List.push = push[0]
# self 只是约定参数名，编译器不看名字看类型

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # 扩容
        new_data = Array(T)(self.data.length * 2)
        for i in 0..self.length {
            new_data[i] = self.data[i]
        }
        self.data = new_data
    }
    self.data[self.length] = item
    self.length = self.length + 1
}

List.pop: (T: Type) -> ((self: List(T)) -> Option(T)) = {
    if self.length > 0 {
        self.length = self.length - 1
        return Option.some(self.data[self.length])
    } else {
        return Option.none()
    }
}

List.map: (T: Type, R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)) = {
    result = List(R)()
    for i in 0..self.length {
        result.push(f(self.data[i]))
    }
    return result
}

List.filter: (T: Type) -> ((self: List(T), predicate: (T) -> Bool) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        if predicate(self.data[i]) {
            result.push(self.data[i])
        }
    }
    return result
}

List.fold: (T: Type, U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U) = {
    result = initial
    for i in 0..self.length {
        result = f(result, self.data[i])
    }
    return result
}

# ======== 3. 类型约束使用 ========
# 实现 Clone for List
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. 使用示例 ========
# 创建泛型List
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# 使用泛型方法
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# 使用fold计算
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# 泛型组合
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 Generic algorithm example

```yaoxiang
# ======== 1. 泛型排序算法 ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# 泛型quicksort
quicksort: (T: Clone) -> ((array: Array(T), cmp: Comparator(T)) -> Array(T)) = {
    if array.length <= 1 {
        return array.clone()
    }

    pivot = array[array.length / 2]
    left = Array(T)()
    right = Array(T)()

    for i in 0..array.length {
        if i == array.length / 2 {
            continue
        }
        item = array[i]
        comparison = cmp.compare(item, pivot)
        if comparison < 0 {
            left.push(item)
        } else {
            right.push(item)
        }
    }

    sorted_left = quicksort(left, cmp)
    sorted_right = quicksort(right, cmp)

    result = sorted_left.clone()
    result.push(pivot)
    result.extend(sorted_right)
    return result
}

# ======== 2. IntComparator实现 ========
# 使用函数重载实现
compare: (a: Int, b: Int) -> Int = {
    if a < b {
        return -1
    } else if a > b {
        return 1
    } else {
        return 0
    }
}

# ======== 3. 使用示例 ========
# 排序Int数组
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# 排序String数组（需要StringComparator）
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-time generic example

```yaoxiang
# ======== 1. 编译期矩阵类型 ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # 编译期维度验证：利用 Assert 标准库类型
    _assert: Assert(Rows > 0),  # Rows > 0，否则编译错误
    _assert: Assert(Cols > 0),  # Cols > 0，否则编译错误

    # 矩阵运算
    multiply: (M: Int) -> ((self: Matrix(T, Rows, Cols), other: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)) = {
        result = Matrix(T, Rows, M)()
        for i in 0..Rows {
            for j in 0..M {
                sum = Zero::zero()
                for k in 0..Cols {
                    sum = sum + self.data[i][k] * other.data[k][j]
                }
                result.data[i][j] = sum
            }
        }
        return result
    }
}

# ======== 2. 编译期矩阵创建 ========
identity: (T: Add + Multiply + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    for i in 0..N {
        for j in 0..N {
            if i == j {
                matrix.data[i][j] = One::one()
            } else {
                matrix.data[i][j] = Zero::zero()
            }
        }
    }
    return matrix
}

# ======== 3. 使用示例 ========
# 创建编译期已知大小的矩阵
# 2x3 矩阵
matrix_2x3 = Matrix(Float, 2, 3)()
matrix_2x3.data[0][0] = 1.0
matrix_2x3.data[0][1] = 2.0
matrix_2x3.data[0][2] = 3.0
matrix_2x3.data[1][0] = 4.0
matrix_2x3.data[1][1] = 5.0
matrix_2x3.data[1][2] = 6.0

# 3x2 矩阵
matrix_3x2 = Matrix(Float, 3, 2)()
matrix_3x2.data[0][0] = 7.0
matrix_3x2.data[0][1] = 8.0
matrix_3x2.data[1][0] = 9.0
matrix_3x2.data[1][1] = 10.0
matrix_3x2.data[2][0] = 11.0
matrix_3x2.data[2][1] = 12.0

# 矩阵乘法：2x3 * 3x2 = 2x2
result = matrix_2x3.multiply(matrix_3x2)

# 编译期验证：result类型为 Matrix(Float, 2, 2)
# 2x2 单位矩阵
identity_3x3 = identity(Float, 3)()

# 维度不匹配：编译错误
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # 编译错误：3x3 != 2x3
```

## Trade-offs

### Advantages

1. **Zero-cost abstraction**
   - Compile-time monomorphization, no runtime overhead
   - No virtual functions, no RTTI

2. **Dead code elimination**
   - Compile-time analysis, only instantiate used generics
   - Controllable code bloat

3. **Macro replacement**
   - Type-safe code generation
   - IDE-friendly, clear error messages

4. **Compile-time computation**
   - Compile-time generics support compile-time computation
   - Dimension verification and other features
   - No `const` keyword required, pure type constraints

### Disadvantages

1. **Compile time**
   - Generic instantiation increases compile time
   - Constraint solving may be slow

2. **Memory usage**
   - Compiler memory usage increases
   - Caching mechanism requires memory

3. **Implementation complexity**
   - Complex constraint solver
   - Complex type-level computation engine

4. **Error diagnosis**
   - Generic errors may be complex
   - Need clear error messages

### Mitigation measures

1. **Caching strategy**
   - Instantiation result cache
   - LRU cache to limit memory

2. **Incremental compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error messages**
   - Clear error messages
   - Generic parameter inference hints

4. **Parallel compilation**
   - Parallel instantiation of generics
   - Multi-threaded constraint solving

## Alternative Solutions

| Solution                   | Why not chosen                      |
| -------------------------- | ----------------------------------- |
| Only basic generics        | Cannot replace complex macros       |
| Pure macro system          | No type safety, poor error messages |
| Only dependent constraints | Insufficient flexibility            |
| Runtime generics           | Performance overhead                |

### Risks

| Risk                          | Impact                     | Mitigation                  |
| ----------------------------- | -------------------------- | --------------------------- |
| Constraint solving complexity | Compile time too long      | Incremental solving + cache |
| Code bloat                    | Binary file too large      | DCE + threshold control     |
| Implementation complexity     | Extended development cycle | Phased implementation       |
| Error diagnosis               | Poor user experience       | Detailed error messages     |

## Open Questions

### Issues to be decided

| Topic                  | Description                         | Status |
| ---------------------- | ----------------------------------- | ------ |
| Instantiation strategy | Eager vs Lazy vs Threshold          | TBD    |
| Cache size             | LRU cache capacity setting          | TBD    |
| Error diagnosis        | Generics error message detail level | TBD    |

### Future optimizations

| Optimization item              | Value  | Implementation difficulty |
| ------------------------------ | ------ | ------------------------- |
| Instantiation graph analysis   | High   | Medium                    |
| Type-level programming DSL     | Medium | High                      |
| Generics performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# 泛型参数使用统一 () 语法，作为函数类型的一部分
# 如 map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 类型约束（在泛型参数中）
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# 参数声明（类型 + 名字）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 函数声明：name: type = expression
# 泛型参数是函数类型中的第一个参数组：(T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# 方法声明：Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 类型定义（统一 Binding 语法）
# 泛型类型如 List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# 泛型参数中的 Type 由编译器自动从实参类型填充
# 如 map(numbers, f)，T 从 numbers: List(Int) 提取，R 从 f: (Int) -> String 提取
```

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review │  ← Open community discussion and feedback
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (Official design) │ (Kept in place)  │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documentation

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./accepted/009-ownership-model.md)
- [RFC-001: spawn Model](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)
- [tutorial/ Tutorial](../../../../../tutorial/)

### External References

- [Rust Generics System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
