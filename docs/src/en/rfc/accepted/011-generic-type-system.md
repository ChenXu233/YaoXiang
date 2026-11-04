---
title: 'RFC-011: Generic System Design - Zero-Cost Abstractions and Macro Replacement'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-14'
updated: '2026-07-15 (type-body code blocks + compile-time reductions + effect seeds implemented)'
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

# RFC-011: Generic System Design - Zero-Cost Abstractions and Macro Replacement

## Abstract

This document defines the **generic system design** of the YaoXiang language. It achieves zero-cost
abstractions through powerful generic capabilities, reduces reliance on macros through compile-time
optimization, and provides a dead-code elimination mechanism.

**Core design**:

- **Unified signature syntax**: `(T: Type, R: Type) -> ...` unifies generic parameters and ordinary
  parameters
- **Type self-description mechanism**: `Type` is a language-level special entity, and `Type`
  positions in signatures can be automatically inferred and filled
- **Type constraints**: `T: Dup + Add` multiple constraints and function-type constraints
- **Associated types**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time generics**: `N: Int` generic value parameters with compile-time constant
  instantiation
- **Conditional types**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation and type
  families

**Value**:

- Zero-cost abstractions: compile-time monomorphization with no runtime overhead
- Dead-code elimination: instantiation-graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro use cases
- Type safety: compile-time checking and IDE-friendly diagnostics
- **Explicit is better than implicit**: `Type` is self-describing, and the compiler performs
  automatic inference

## Reference Documents

The design of this document is based on the following documents:

| Document                                                                                                 | Relationship              | Description                                                                    |
| -------------------------------------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------ |
| [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)                                               | **Syntax basis**          | Integrates generic syntax with the unified `name: type = value` model          |
| [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)                                               | **Call syntax**           | Section 6: generic call syntax—unified `()` application; `[]` is fully removed |
| [RFC-009: Ownership Model](009-ownership-model.md)                                                       | **Type system**           | A natural combination of move semantics and generics                           |
| [RFC-024: spawn-Based Concurrency Runtime Semantics](024-concurrency-model.md)                           | **Execution model**       | DAG analysis and generic type checking                                         |
| [RFC-008: Runtime Model](008-runtime-concurrency-model.md)                                               | **Compiler architecture** | Generic monomorphization and compile-time optimization strategies              |
| Type-universe ideas (see the identically named section below)                                            | **Theoretical core**      | Type-universe hierarchy model and value-dependent type design                  |
| [RFC-027: Compile-Time Predicates and Unified Static Verification](027-compile-time-evaluation-types.md) | **Termination checking**  | Automatic measure synthesis and compile-time evaluation safety guarantees      |

## Type-Universe Ideas and Value-Dependent Types

YaoXiang's generic system is built on **type-universe ideas**. This mental model unifies all
concepts in the language into a layered structure, with its core innovation being the promotion of
**value-dependent types** to first-class citizens at the Type2 layer.

### What Are Value-Dependent Types?

A **value-dependent type** is a type that depends on one or more **values**, rather than only on
other types. These values can be evaluated at compile time, thereby providing type-safety guarantees
during compilation.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Array: (T: Type, N: Int) -> Type  # The array type depends on the length value N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # The matrix type depends on the row and column counts
```

### Layering of Container-Type Names

The language layer has three container concepts. The ownership of length information is their
fundamental distinction:

| Type          | Length        | Semantics                                    | Underlying implementation                |
| ------------- | ------------- | -------------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | **Fixed-length** array, with N in the type   | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | **Raw buffer with runtime length**, growable | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard-library type                        | Library: `{ data: Vec(T), length: Int }` |

The division of responsibility among the three is as follows:

- **`Array(T, N)` is the only form that places length in the type**—length is a compile-time
  constant, so boundary failures can be rejected at compile time (`a[5]` directly produces a
  compile-time error when `a: Array(Int, 3)`; see “Compile-Time Dimensional Validation” below).
- **`Vec(T)` is the minimal foundation for runtime-length containers**—it provides only four
  capabilities: allocation, length retrieval, reading and writing, and growth. Capacity policy,
  growth factor, and shrinking are deliberately left unspecified. It is the raw material for
  building other containers.
- **`List(T)` is a library type, not a primitive**—it is defined in `std.list` using YaoXiang itself
  (`{ data: Vec(T), length: Int }`) and receives the same treatment as user-defined generic records.
  All policies for growable semantics—including when to grow, how much to grow, and whether sharing
  is possible—live in the library, with no compiler involvement.

The construction form of `Vec(T)` has two layers: type parameters first, then constructor arguments:

```yaoxiang
# Empty construction—length 0, with elements appended later
v = Vec(Int)()

# Element construction—the length is determined by the number of elements
w = Vec(Int)(1, 2, 3)          # Length 3

# Slot allocation—allocate n zero-valued slots
buf = Vec(Int)(len=64)         # Length 64, with all elements initialized to zero
```

> Slot allocation uses the **named-field form** (`len=`) rather than the positional form: a single
> positional integer would be ambiguous with a “single-element vector” (`Vec(Int)(64)` cannot
> distinguish “length 64” from “one element whose value is 64”). This is consistent with the unified
> rules for generic construction: named-field arguments bind by name and are unaffected by
> positional inference.
>
> This is the only primitive required for `List` growth—`List` allocates new slots and moves
> elements when necessary:
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> All decisions about when to grow, how much to grow, and whether to shrink are made by `List`.
> `Vec` implements no capacity policy.

From bottom to top, performance decreases while flexibility increases: `Array` > `Vec` > `List`.

> Naming rationale: `Vec`/`vector` denotes a growable sequence with runtime length in mainstream
> languages such as Rust and C++; `Array` specifies a fixed length.

### Core Advantages of Value-Dependent Types

Compared with traditional generics, YaoXiang's value-dependent types offer the following core
advantages:

| Feature                      | Traditional Generics (C++/Rust)                            | YaoXiang Value-Dependent Types                                |
| ---------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------- |
| Values on which types depend | Depend only on type parameters                             | May depend on any value, including function-call results      |
| Compile-time evaluation      | C++ templates require manual specialization; Rust has none | Automatic compile-time evaluation with guaranteed termination |
| Type-level computation       | Template metaprogramming (complex/dangerous)               | Unified type-level computation engine                         |
| Type safety                  | None in C++; limited in Rust                               | Complete type safety with compile-time checks                 |
| Dimensional validation       | Runtime checks or manual specialization                    | Compile-time dimensional validation with no runtime overhead  |

### Type-Universe Hierarchy and Value-Dependent Types

The type-universe idea divides language concepts into different layers according to their semantic
roles. Value-dependent types reside at the **Type2 layer**:

| Layer     | Role                                                  | Examples                                                                                                        |
| --------- | ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | Values                                                | `42`, `factorial(5)`, the function itself                                                                       |
| Type0     | Metatype keyword                                      | `Type`                                                                                                          |
| Type1     | Concrete types                                        | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **Functions/type constructors/value-dependent types** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key design**: functions, type constructors, and value-dependent types at the Type2 layer all use
**unified syntax**, taking the form `(params) -> result`:

- Ordinary function: `(Int, Int) -> Int` → returns a value
- Type constructor: `(T: Type) -> Type` → returns a type
- Value-dependent type: `(T: Type, N: Int) -> Type` → returns a type that depends on value parameter
  N

> **Curry-Howard correspondence**: this unification is not an accident. The Curry-Howard
> correspondence states that “types are propositions, and programs are proofs”—the function type
> `A → B` corresponds to the logical implication “if A, then B”; the generic type
> `(T: Type) -> Type` corresponds to universal quantification “for every type T”; and the
> value-dependent type `(n: Int) -> Type` corresponds to “for every integer n, there exists a type.”
> YaoXiang unifies functions, type constructors, and value-dependent types at the Type2 layer,
> essentially unifying “proof” and “computation” as the same concept—**constructive proof**. This is
> the direct expression of the Curry-Howard correspondence in language design: one form,
> `(params) -> result`, simultaneously carries logical propositions and computational processes.

### Compile-Time Determinism Guarantee

YaoXiang's type-universe idea requires that **everything at the Type layer be determined at compile
time**.

```yaoxiang
# Example of compile-time dimensional validation
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # Compile-time check: dimensions must be positive
    _assert: Assert(Rows > 0),
    _assert: Assert(Cols > 0),
}

# Create a 3x3 identity matrix—completed at compile time
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# Compile-time computation: factorial(3) = 6; the array size is determined at compile time
arr: Array(Int, factorial(3)) = Array(Int, 6)()
```

The compiler automatically:

1. Detects function calls in type positions
2. Performs compile-time termination checking on the functions (see the termination-checking
   mechanism below)
3. Evaluates them at compile time
4. Embeds the results into the generated types

### Application Scenarios for Value-Dependent Types

#### Compile-Time Dimensional Validation

```yaoxiang
# Matrix multiplication: dimension matching is validated at compile time
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # Compile-time check: a.Cols == b.Rows; otherwise, a compile error occurs
    result = Matrix(T, Rows, M)()
    # ...
}

# The error is captured at compile time:
# multiply(matrix_2x3, matrix_4x2)  # Compile error: 2 != 4
```

#### Type-Safe Array Sizes

```yaoxiang
# The array size is a compile-time constant
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N is a compile-time constant and can be used in type-level computation
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3 (known at compile time)
```

#### Goal of Compile-Time Coverage for Boundary Failures

> **Implementation status**: container types have been de-specialized—`Array(T, N)` is a const
> generic constructor, and literal-context landing points and `in` membership predicates have been
> implemented. The `N` in the `Array(T, N)` literal landing point and the element type are enforced
> by compile-time validation (E1002), so **N is trustworthy**—the mechanism in this section can be
> built on “annotation N == runtime length.” The current `[]` out-of-bounds error (E6003) and Dict
> missing-key error (E6008) are in a **runtime-error transitional state**; value-dependent types in
> this section are the target mechanism for pushing these boundary failures into **compile time**:
>
> - const index: `a[5]` (where 5 is a compile-time constant) is rejected directly at compile time
>   when `a: Array(Int, 3)`;
> - value index: `a[i]` requires the precondition `i < len(a)`, proven by a value-dependent type
>   contract;
> - the `in` predicate is the basis for Hoare-logic preconditions: `n in 1..10` and `x in some_set`
>   are propositions that can be proven at compile time.
>
> The complete design of refinement types will be added to this section separately when implemented.

#### Conditional Types

```yaoxiang
# Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# Type family
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,
}
```

#### Generic Functions

```yaoxiang
# map: generic function; type parameters T and R are determined at compile time
map: (T: Type, R: Type) -> (
    (list: List(T), f: (x: T) -> R) -> List(R)
) = (list, f) => {
    result = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# Usage is fully transparent; types are inferred automatically
numbers = List(Int)()   # Two-layer value construction (see §9.1); populate with push
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                                          | C++ Templates          | Rust Generics  | Haskell GADT   | **YaoXiang**                                                               |
| ---------------------------------------------------------------- | ---------------------- | -------------- | -------------- | -------------------------------------------------------------------------- |
| Type parameters                                                  | ✅                     | ✅             | ✅             | ✅                                                                         |
| Value-dependent types                                            | ❌                     | ❌             | ✅             | ✅                                                                         |
| Compile-time evaluation                                          | Template instantiation | ❌             | ✅             | ✅                                                                         |
| Termination guarantee                                            | ❌                     | ❌             | ❌ (dangerous) | ✅ (automatic measure exploration + explicit measures, RFC-027)            |
| Type safety                                                      | ❌ (macro expansion)   | ✅             | ✅             | ✅                                                                         |
| Unified syntax                                                   | ❌                     | ❌             | ❌             | ✅                                                                         |
| Compile-time dimensional validation                              | Manual specialization  | Runtime checks | Type families  | Automatic compile-time validation                                          |
| Semi-automatic termination annotations (`decreases`/`invariant`) | ❌                     | ❌             | ❌             | ❌ (no annotation syntax; explicit measures are written in type positions) |

### Termination-Checking Mechanism (Unified with RFC-027)

Compile-time evaluation of value-dependent types must **guarantee termination**; otherwise, the type
system could enter an infinite loop. Termination checking is performed by RFC-027's compile-time
proof pipeline with **automatic processing as the primary path**—the compiler first explores
measures automatically, allowing recursive calls and loops that can be proven to terminate. If
exploration fails and no explicit measure has been provided, a compile error is emitted (RFC-027
§6.9 specifies the fallback for explicit measures in type positions). **No escape hatch is provided
for annotation syntax**: RFC-022's `//! decreases` and `/*! invariant !*/` were deprecated along
with RFC-022, and the reduction itself is the type annotation.

> **Trigger criteria (RFC-027 §7)**: termination obligations are triggered by **refinement types**—a
> type enters verification mode as soon as it is refined. Ordinary types that are not refined do not
> enter verification mode and do not generate termination obligations.

#### Termination Checking of Recursive Functions

For recursive functions with refined signatures, the compiler checks whether the arguments of
recursive calls strictly decrease on every recursive path (RFC-027 §6.7). No reduction annotations
are required:

```yaoxiang
# Recursion with a refined signature: no //! requires/ensures/decreases; the compiler automatically explores the decrease
factorial: (n: NonNegative(n)) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler exploration: n-1 < n → decreases → termination
}
```

| Scenario                                                                                    | Behavior                               |
| ------------------------------------------------------------------------------------------- | -------------------------------------- |
| The compiler can explore a decrease such as `n-1`                                           | Pass                                   |
| Exploration fails, but the type position provides a measure (§6.9)                          | Passes if the SMT solver proves it     |
| Exploration fails and there is no explicit measure, or the SMT solver disproves the measure | Compile error                          |
| The signature has no refinement and does not enter verification mode                        | No termination obligation is generated |

#### Termination Checking of Loops

Loops do not require `: Invariant(...)` or `: decreases(...)` annotations. A refined type annotation
on a variable, such as `UpTo(n)`, simultaneously supplies the loop invariant and the measure bound.
The compiler tries four measure-exploration strategies in priority order and stops when it finds one
(RFC-027 §6.1–6.5):

1. **Automatic synthesis of a linear rank function**—extracts variable bounds from type annotations,
   enumerates linear combinations, and uses the SMT solver to verify m ≥ 0 and m' < m on every path
2. **Predicate violation counting** (experimental)—extracts a violation_count from a target type
   definition such as `Sorted`, covering adjacent swaps/moves
3. **Bounded increment/decrement patterns**—`v += const` → measure `upper - v` (a degenerate case of
   strategy 1 and the fastest path)
4. **Multiplicative scaling measure templates**—`v *= const` (`const > 1`) → measure
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # The type annotation supplies the upper bound arr.len and lower bound 0 → enters verification mode
    while i < arr.len {
        # The compiler automatically explores the measure arr.len - i, which strictly decreases by 1 on each iteration → termination is proven
        s += arr[i]; i += 1
    }
    return s
}
```

When exploration fails, a name can be bound to a loop and a measure supplied in its type position
(RFC-027 §6.9):

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    // The loop body is a `{}` block, whose value is provided by the tail expression (spec §2.9): after an assignment statement at the tail of the body, `i` must still be written; otherwise, the block value is `Void`, and returning `Void` from `-> Int` is invalid.
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}
```

#### Termination-Checking Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  类型检查阶段                                                │
│  遇到带精化类型的位置（参数精化、返回精化、变量精化）      │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. 终止检查（RFC-027 证明管道，全自动优先）                 │
│     - 递归函数：检查参数在每条递归路径上严格递减             │
│     - 循环：四种度量探索策略（线性秩/违反计数/有界模式/      │
│       乘法缩放），SMT 验证递减                                │
│     - 探索不出 → 可由程序员在类型位给出测度（Terminates）   │
│     - 无测度 / 测度被 SMT 判伪 → 编译错误（硬边界）          │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. 编译期求值（由内置解释器执行）                           │
│     - 纯函数：直接求值                                       │
│     - 副作用：编译错误（类型位置必须无副作用）                │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 结果嵌入类型                                            │
│     - Array(Int, factorial(5)) → Array(Int, 120)             │
│     - Matrix(Float, 3, 3) → 具体类型                        │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: ensures that compile-time evaluation necessarily terminates, preventing the type
  system from entering an infinite loop
- **Unification**: termination checking and correctness verification (VC generation) share the same
  compile-time proof pipeline (RFC-027), with no separate reduction syntax
- **Automatic processing first**: the compiler automatically explores measures from type annotations
  and passes whenever it can prove termination; if exploration fails, a measure can be supplied in
  the type position (`Terminates`), with the SMT solver still making the decision—without requiring
  developers to write `decreases` syntax manually

## Motivation

### Why Is a Strong Generic System Needed?

Generics in current mainstream languages have limitations:

| Language     | Generic Capabilities      | Problem                                                                                              |
| ------------ | ------------------------- | ---------------------------------------------------------------------------------------------------- |
| Java         | Bounded types             | Compile-time monomorphization; no generic specialization                                             |
| C#           | Generic constraints       | Runtime type checks and performance overhead                                                         |
| Rust         | Generics + Trait          | The Trait system is complex and has a steep learning curve                                           |
| C++          | Templates                 | Template specialization is complex and produces poor compile-error messages                          |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, with compile-time dimensional validation and a termination guarantee** |

### Core Contradictions

1. **Performance vs. flexibility**: runtime flexibility vs. compile-time optimization
2. **Complexity vs. simplicity**: a powerful type system vs. ease of use
3. **Macros vs. generics**: macro-based code generation vs. generic type safety
4. **Value dependence vs. type safety**: traditional generics cannot validate dimensions at compile
   time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** provide the following core advantages over traditional
generics:

| Advantage                   | Description                                                                                                              |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| **Types depend on values**  | `Array: (T: Type, N: Int) -> Type` allows a type to depend on a concrete value                                           |
| **Compile-time evaluation** | Function calls in type positions are evaluated at compile time, and their results are embedded directly in the type      |
| **Dimensional validation**  | `Matrix(Float, 3, 3)` validates matrix dimensions at compile time                                                        |
| **Type-level computation**  | Conditional types such as `If` and `Match` support type-level computation                                                |
| **Termination guarantee**   | Compile-time termination checking through automatic measure synthesis guarantees that compile-time evaluation terminates |

```yaoxiang
# Compile-time validation that C++/Rust cannot perform
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# Compile-time computation: factorial(3) = 6, factorial(2) = 2
# The type is Matrix(Float, 6, 2)

# Dimension mismatches are captured at compile time
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # Compile error: 2 != 3
```

### Value of the Generic System

```yaoxiang
# Example: unified API design
# map operations for different container types

# Traditional approach: a separate implementation for each type
map_int_array: (array: Vec(Int), f: Fn(Int) -> Int) -> Vec(Int) = ...
map_string_array: (array: Vec(String), f: Fn(String) -> String) -> Vec(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# Generic approach: one generic function covers all types
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## Design Goals

### Core Goals

1. **Zero-cost abstractions**—generic calls are equivalent to calls with concrete types
2. **Dead-code elimination**—compile-time analysis instantiates only the generics that are used
3. **Macro replacement**—generics replace 90% of macro use cases
4. **Type safety**—compile-time checking with no runtime type overhead
5. **IDE-friendly**—smart suggestions and clear error messages
6. **Value-dependent types**—types can depend on values, supporting compile-time dimensional
   validation
7. **Safe compile-time evaluation**—compile-time termination checking through RFC-027 automatic
   measure synthesis guarantees that compile-time evaluation terminates

### Design Principles

- **Compile-time determinism**: generic parameters are determined at compile time
- **Monomorphization first**: generate concrete code and avoid virtual-function calls
- **Constraint-driven**: type constraints guide instantiation
- **Platform optimization**: specialization supports platform-specific optimizations
- **Unified type universe**: functions, type constructors, and value-dependent types are unified at
  the Type2 layer
- **Termination guarantee**: function calls in type positions must be proven to terminate

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key rule**: generic type definitions **must explicitly annotate `: Type`**; otherwise, they are
> inferred as functions by HM.
>
> | Form                              | Meaning                                     |
> | --------------------------------- | ------------------------------------------- |
> | `List: (T: Type) -> Type = {...}` | ✅ Type constructor                         |
> | `List = {...}`                    | ❌ Inferred as a function by HM, not a type |

```yaoxiang
# Generic type definition (must have : Type)
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: (T: Type) -> Type = {
    data: Vec(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,   # self is only a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T),
}

# Generic function (without : Type; inferred as a function by HM)
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# Generic constraint (direct expression; return may be omitted on a single line)
clone: (T: Clone)(value: T) -> T = value.clone()

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### Generic Function Call Syntax

#### 1.1 Unified Signature Syntax

```yaoxiang
# Generic functions use the unified (T: Type, R: Type) signature syntax
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type Self-Description Mechanism

`Type` is a language-level special entity. The compiler can naturally recognize `Type` positions in
signatures and automatically infer and fill them from the actual argument types.

```yaoxiang
# The compiler automatically infers generic parameters
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         Type declaration   Construction call: Int fills T, () constructs the value

# Function-call inference
numbers: List(Int) = List(Int)()
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# Compiler inference: T=Int, R=String
```

#### 1.3 Monomorphization

```yaoxiang
# Source code
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = {
    result: List(R) = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# Use sites
int_list: List(Int) = List(Int)()
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # Instantiates map[Int, Int]

string_list: List(String) = List(String)()
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # Instantiates map[String, String]

# After compilation (equivalent code)
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

#### 1.4 Explicit Population (When Inference Fails)

````yaoxiang
# Omit Type parameters when they can be inferred
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# Type parameters must be populated explicitly when inference is impossible
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R

### 2. 类型约束系统

#### 2.1 单一约束

```yaoxiang
# Basic trait definition (interface type)
Clone: Type = {
    clone: (Self) -> Self,
}

Display: Type = {
    fmt: (Self, Formatter) -> Result,
}

Debug: Type = {
    fmt: (Self, Formatter) -> Result,
}

# Using constraints: declare type constraints directly in the signature
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
````

#### 2.2 Multiple Constraints

```yaoxiang
# Multiple-constraint syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# Sorting a generic container
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # Implement the sorting algorithm
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# Function-type constraint
map: (T: Type, R: FnMut(T))(array: Vec(T), f: R) -> Vec(R) = {
    result: Vec(R) = Vec()
    for item in array {
        result.push(f(item))
    }
    return result
}

# Usage
doubled: Vec(Int) = map(Vec(1, 2, 3), (x: Int) => x * 2)  # Compiler inference
```

> **Sources of constraint names (note dated 2026-09-22)**: operator constraints such as `Add` /
> `Subtract` / `Multiply` / `Divide` / `Modulo` are defined and implemented by
> [RFC-011b: Operator Overloading and Interface-Driven Operators](011b-operator-overloading.md)—`T: Add`
> ≜ an instantiated registered `Add(T, T, T)` interface (three type parameters, with the result type
> `O` explicit). `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` currently **have no definition
> source** and are dangling constraint names awaiting their respective RFCs. Until then, examples
> involving these names are paper designs.

#### 2.3 Function-Type Constraints

```yaoxiang
# Higher-order function constraints
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

call_with_arg: (T: Type, U: Type, F: Fn(T) -> U)(arg: T, f: F) -> U = f(arg)

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))

# Usage example
result: Int = call_with_arg(42, (x: Int) => x * 2)  # result = 84
composed: String = compose(
    "hello",
    (s: String) => s.to_uppercase(),
    (s: String) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

#### 2.4 Built-in Marker Traits: Dup and Clone

**Three copying semantics**:

| Type                     | Meaning                                                                                   | Trigger                                  | Applicable scenarios               |
| ------------------------ | ----------------------------------------------------------------------------------------- | ---------------------------------------- | ---------------------------------- |
| **Primitive value copy** | The value is copied automatically on assignment, and the two values are fully independent | Automatic on assignment/argument passing | Int, Float, Bool, Char             |
| **Dup**                  | Shallow copy: copies a handle/token while the underlying data is shared                   | Automatic on assignment/argument passing | `&T` tokens, `ref T`, String/Bytes |
| **Clone**                | Deep copy: creates a complete, independent copy                                           | `value.clone()`                          | Any type that implements Clone     |

**Semantics of Dup**: a type that implements Dup does not transfer ownership on assignment or
argument passing—the compiler copies the handle/token, allowing multiple holders to refer to the
same underlying data. This complements the default move semantics of RFC-009's ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = 复制句柄，共享数据（修改互相影响）
Clone = 复制数据，副本独立（修改互不影响）
```

**Rules**:

```
1. 原语值类型（Int, Float, Bool, Char） — 编译器内置值复制，不属于 Dup
2. Dup  — 只适用于引用/令牌类型和内部引用计数的类型
3. Clone — 显式深拷贝，任何类型可实现
4. 默认 Move — 其他类型保持默认 Move 语义
```

**Which types are Dup**:

| Type                     | Dup     | Reason                                                                                                |
| ------------------------ | ------- | ----------------------------------------------------------------------------------------------------- |
| `&T` (borrowed token)    | ✅      | Zero-sized token; copying the token gives multiple views of the same data                             |
| `ref T`                  | ✅      | Copying an Rc/Arc increments the reference count and shares the heap data                             |
| String, Bytes            | ✅      | Internal reference counting; copying the handle shares the underlying buffer                          |
| `&mut T` (mutable token) | ❌      | Linear and exclusive; cannot be copied                                                                |
| struct                   | Derived | If all fields ∈ (primitive value types ∪ Dup) → Dup; otherwise Move (RFC-009 §derivation rules, #398) |
| enum                     | Derived | If all fields of all variants are copyable → the enum is Dup (design stage; not yet implemented)      |
| tuple                    | Derived | Determined element by element, following the same rules as struct (#398)                              |
| Fn (closure)             | ❌      | The captured environment may not be Dup                                                               |
| `*T` (raw pointer)       | ❌      | unsafe; does not participate in the ownership system                                                  |

**Int/Float/Bool/Char are not Dup**—they are value types, and the compiler automatically copies
their values on assignment, with the two values fully independent. This is not a “shallow copy”; it
is the compiler's built-in handling of primitives and neither needs nor should be expressed through
a Dup type attribute.

```yaoxiang
# Primitive value types: the compiler automatically copies the value (not Dup)
x: Int = 42
y = x          # Value copy; x and y are fully independent
print(x)       # ✅

# Dup: shallow copy; copying the handle shares the data
view: &Point = &point
view2 = view    # ✅ Dup: copies the token; both point to the same point
print(view.x)   # ✅

# Clone: explicit deep copy that creates an independent copy
backup = big_struct.clone()  # Explicit invocation

# Generic constraints
dup_use: (T: Dup) -> T = x         # T: Dup → shallow copy is permitted
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → deep copy is permitted
```

> **Note**: `Send`/`Sync` are not exposed as user-visible traits. Cross-task safety is guaranteed
> through the `ref` keyword and fully automatic compiler handling—`ref` automatically selects Rc or
> Arc, so users do not need to understand Send/Sync.

### 3. Associated Types

#### 3.1 Associated Type Definition

```yaoxiang
# Iterator trait (using the (Item: Type) -> Type syntax)
Iterator: (Item: Type) -> Type = {
    next: (Self) -> Option(Item),
    has_next: (Self) -> Bool,
    collect: (T: Type)(Self) -> List(T),
}

# Usage
collect_all: (T: Type, I: Iterator(T))(iter: I) -> List(T) = {
    result: List(T) = List(T)
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}

# Vec's Iterator implementation
# Method-syntax sugar: Vec.Item, Vec.next, Vec.has_next
# The iteration position is carried by the wrapper record (Vec itself is a raw buffer with no index field)
VecIter: (T: Type) -> Type = {
    data: &Vec(T),
    index: Int,
}

VecIter.has_next: (T: Type)(self: &VecIter(T)) -> Bool = {
    return self.index < self.data.length
}

VecIter.next: (T: Type)(self: &mut VecIter(T)) -> Option(T) = {
    if self.index < self.data.length {
        item = self.data[self.index]
        self.index = self.index + 1
        return Option.some(item)
    } else {
        return Option.none()
    }
}

VecIter.Item: (T: Type)(arr: &VecIter(T)) -> T = {
    return arr.data[arr.index]
}
```

#### 3.2 Generic Associated Types (GATs)

```yaoxiang
# More complex associated type
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# Associated types can be generic
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # The associated type is also generic
    iter: (Self) -> IteratorType,
}

# Usage
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. Compile-Time Generics

#### 4.1 Compile-Time Value Parameters

**Core design**: `Type` marks type parameters in a generic signature. Parameters annotated with
concrete types, such as `Int`/`Bool`/`Float`, are **candidates for compile-time value parameters**.
Whether one actually becomes a compile-time value parameter depends on whether its value is
**referenced in a type position** (value dependence). No `const` keyword is required.

> The criterion is **reference in a type position**, not “annotated with a concrete type”: in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters because neither
> appears in any type position.

**Decision rules (two steps)**:

1. **Syntactic prefilter**: a parameter annotated with a concrete non-`Type` type, such as `Int`, is
   listed as a candidate.
2. **Usage filter**: the candidate name is **referenced in a type position** → it is confirmed as a
   compile-time value parameter; otherwise, it is treated as a **runtime value parameter**.

A “type position” = **all `Type` subtrees** in a declaration—parameter annotations, function return
types, type-body fields, nested `Fn` types, `Assert` predicates, and type-constructor argument
positions are all instances of `Type` subtrees in different syntactic positions. The decision
traverses all subtrees rather than handling positions case by case; missing any occurrence—for
example, omitting the return type—would misclassify a compile-time value parameter as a runtime
value parameter.

“Referenced” is determined through **namespace resolution**, not by literal comparison of names:

1. An identifier is first resolved in the **type namespace**—declared type names and type parameters
   (`T: Type`). The declaration's own name enters the type namespace before its type body is
   resolved, allowing `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` to refer to itself.
2. Resolution to a declared type ⇒ it is unrelated to formal parameters. A formal parameter and a
   type with the same name **do not shadow one another**: the value and type namespaces do not
   consult the same table, so the two `Int`s in `(Int: Int) -> Int` do not interfere. This rule
   therefore requires no shadowing exception.
3. If resolution fails but the signature contains a candidate with the same name ⇒ that candidate is
   a compile-time value parameter (value dependence).
4. If neither resolution succeeds ⇒ error: a type position contains a bare identifier that is
   neither a type nor a type parameter.

**Self-reference is handled as a monotonic fixed point**. Resolving the `N` in `(k: N)` in
`factorial: (N: Int) -> (k: N) -> Int` requires first knowing whether `N` is a type, but classifying
`N` is precisely what this rule computes. Therefore: ① the initial type namespace contains declared
types, type parameters, and the current declaration's name; ② resolve all type-position identifiers
and confirm a matching candidate as a compile-time value parameter, adding it to the type namespace;
③ repeat ② until the set no longer grows. The set only grows ⇒ it is monotonic; each round continues
only if it adds at least one candidate ⇒ at most |candidates| rounds, and it necessarily terminates.

This produces the **placeholder rule**: compile-time value parameters **do not occupy runtime
formal-parameter slots**—runtime formal-parameter slots = formal parameters − type parameters −
compile-time value parameters, ordered by declaration order. Call-site arguments, named arguments,
and positional binding in implementation headers all follow this sequence.

| Form                                                       | Decision                            | Reason                                                                         |
| ---------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------ |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | They occur only in value positions and do not participate in type construction |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N occurs in the type-constructor argument position `Array(T, N)`               |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N is the type of the inner parameter `k`                                       |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N is unused (see below)             | N is not referenced in the type body and degrades to a runtime value parameter |

> **Essence of value dependence**: a compile-time value parameter is a value-dependent type—a value
> needs to be determined at compile time only when it is used to **construct a type**. Its form
> (`: Int`) determines candidate eligibility, while its use (appearing in a type position)
> determines whether it is a compile-time value parameter. This is the same criterion as “function
> calls in type positions are evaluated at compile time” in §“Compile-Time Determinism Guarantee”.

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameter: N is referenced in a type position (Measure's length slot)
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N occurs in a type-constructor argument position → compile-time value parameter
    length: N,
}

# Usage: factorial(5) is evaluated in a type position (at compile time), and the result 120 is embedded in the type
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# Value dependence: N is the type of the inner parameter k
# ════════════════════════════════════════════════════════
# N is a compile-time value parameter (it occurs in the type position of (k: N));
# k is a runtime value parameter whose type is the literal type N (a single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling of unused candidates**: a candidate annotated with a concrete type but not referenced
> in a type position, such as `N` in `Foo` above, degrades to a runtime value parameter on the
> function-level path. An unused candidate on the type-constructor path cannot occupy a runtime slot
> because type constructors are evaluated at compile time, so the declaration reports [E1094]
> directly: “N is declared as a compile-time value parameter but is not referenced in the type
> body”—previously, silently dropping it caused instantiation-arity inconsistencies.

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Example of compile-time computation
# ════════════════════════════════════════════════════════

# The compiler computes function calls with literal types at compile time
SIZE: Int = factorial(5)  # 120 at compile time

# Matrix type usage
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time dimensional validation
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

# Usage: compile-time computation generates Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never and Void: ⊥ and ⊤ of the Type System

Under the Curry-Howard correspondence, YaoXiang's type system simultaneously provides ⊥ (false/empty
type) and ⊤ (true/Unit), represented by the two built-in type names `Never` and `Void`:

**Never (⊥)** — three non-negotiable kernel properties:

1. **Zero constructor**: no literal or expression can produce a value of type `Never`. This is a
   meta-level property and must be built in.
2. **Explosion principle**: `Never <: T` holds for every type `T`. A `Never` value can be used as
   any type—this is precisely why code following `assert(false)` still passes type checking, even
   though it is never executed.
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this information for dead-code analysis.

`Never` is a built-in type name, not a keyword, and is transparent to the parser. Empty-sum and
type-literal syntax are not exposed.

**Void (⊤, i.e. Unit)** — has exactly one inhabitant (the default void value) and carries the
always-true proposition. `Void` is the identity of the zero-field product type, while `Never` is the
identity of the zero-variant sum type; the two are dual. `x: Void = <default>` is valid, whereas
`x: Never = ...` has no right-hand side that can be written.

#### 4.3 Compile-Time Validation (Standard-Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard-library implementation: using conditional types
# ════════════════════════════════════════════════════════

# Standard-library definition
# IsTrue: a bridge from the value universe to the type universe—maps a Boolean truth value to a type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has a value; the program continues
    false => Never,    # ⊥, has no value; diverges
}

# Assert: a compile-time refinement-type primitive—a type-level expression of a Boolean proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error/divergence)
# cond is undecidable → the proof pipeline chooses based on dispatch mode:
#                  CompileTime → Unknown, prove is required
#                  Runtime     → insert check and inject a Γ assumption

# Usage 1: as a constraint in a type definition
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert is in a type position)
    length: Assert(N > 0),
}

# Usage 2: in an expression
IntArray: (N: Int) -> Type = Array(Int, N)
# Validation: the size of IntArray(10) equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generic Specialization

```yaoxiang
# Small-array optimization: implement compile-time generic specialization through function overloading

# General implementation
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    result = Zero::zero()
    for item in arr.data {
        result = result + item
    }
    return result
}

# N=1 specialization
sum: (T: Type) -> ((arr: Array(T, 1)) -> T) = arr.data[0]

# N=2 specialization
sum: (T: Type) -> ((arr: Array(T, 2)) -> T) = arr.data[0] + arr.data[1]

# Small-array loop unrolling (N <= 4)
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    # Compiler optimization: unroll the loop
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. Conditional Types

> **Curry-Howard correspondence**: from the Curry-Howard perspective, conditional types are **case
> analysis** in logic. The `Bool` type corresponds to a proposition with two possible values
> (True/False), and `If` selects a different result according to the proposition's truth value—this
> is precisely case disjunction in logic. `match C { True => T, False => E }` effectively expresses:
> “if proposition C is known to be True, the conclusion is T; if C is False, the conclusion is E.”

#### 5.1 If Conditional Type

```yaoxiang
# Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# Example: compile-time branching
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# Compile-time validation (unified with the Assert definition in §4.3)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Families

> **Curry-Howard correspondence**: type families are the most direct expression of “propositions as
> types.” `Add: (A: Type, B: Type) -> Type` is not “an addition function written at the type level,”
> but rather **a proposition being constructed about natural-number addition**. `(Zero, B) => B`
> means “the proposition Add(Zero, B) is equivalent to B,” while `(Succ(A'), B) => Succ(Add(A', B))`
> means “if Add(A', B) holds, then Add(Succ(A'), B) also holds.” This is precisely the definition of
> addition from the Peano axioms. The type checker verifying that this match expression is well
> formed is equivalent to verifying the logical consistency of this definition.

```yaoxiang
# Compile-time type conversion
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,  # Default
}

# Type-level computation
Length: (T: Type) -> Type = match T.length {
    0 => Zero,
    1 => Succ(Zero),
    2 => Succ(Succ(Zero)),
    _ => TooLong,
}

# Type-level addition (Curry-Howard: case analysis + recursive calls; termination checking is required for a complete induction)
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# Example: compile-time computation of 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. Function-Overload Specialization

#### 6.1 Basic Specialization

```yaoxiang
# Basic specialization: use function overloading (automatically selected by the compiler)
sum: (arr: Vec(Int)) -> Int = {
    # Compiled into more efficient code
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    # Uses SIMD instructions
    return simd_sum_float(arr.data, arr.length)
}

# General implementation
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

#### 6.2 Conditional Specialization

```yaoxiang
# Specialization using syntax that fully conforms to RFC-010: function overloading

# Concrete-type specialization
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation (the compiler automatically selects the optimal version)
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# Usage is fully transparent
int_arr = Vec(Int)(1, 2, 3)
float_arr = Vec(Float)(1.0, 2.0, 3.0)

# The compiler automatically selects the optimal specialization
sum(int_arr)     # Selects sum: (Vec(Int)) -> Int
sum(float_arr)    # Selects sum: (Vec(Float)) -> Float
```

#### 6.3 Perfect Integration of Function Overloading and Inlining

**Key feature**: function overloading integrates naturally with inlining optimization, enabling
zero-cost abstractions.

```yaoxiang
# ======== Source code ========
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# Usage
int_arr = Vec(Int)(1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== After compilation (equivalent code) ========
# The compiler automatically selects the optimal specialization and then inlines it
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, with no function-call overhead!
```

**Core advantages**:

1. **Intelligent compiler selection**

   ```yaoxiang
   sum(int_arr)      # Automatically selects sum: (Vec(Int)) -> Int
   sum(float_arr)    # Automatically selects sum: (Vec(Float)) -> Float
   sum(custom_arr)  # Automatically selects sum: (T: Type) -> ((arr: Vec(T)) -> T)
   ```

2. **Inlining optimization**
   - Small functions are automatically inlined at call sites
   - No function-call overhead
   - Completely equivalent to hand-written optimized code

3. **Type safety**
   - Compile-time type checking
   - No runtime overhead
   - No virtual-function tables required

4. **Perfect fit with RFC-010**

   ```yaoxiang
   # Uses unified syntax throughout
   name: type = value
   # No new keywords such as impl or where are required
   ```

**Practical example**:

```yaoxiang
# Performance-sensitive numerical computation
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Uses Binet's formula
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# The compiler automatically selects and inlines
fibonacci(10)      # Selects the Int version and is fully inlined
fibonacci(10.5)    # Selects the Float version and uses Binet's formula
```

**What does this mean?**

- ✅ **Generic specialization** → naturally solved through function overloading
- ✅ **Performance optimization** → inlining is performed automatically
- ✅ **Code reuse** → one function name with multiple implementations
- ✅ **Zero-cost abstraction** → compile-time polymorphism with no runtime overhead
- ✅ **No new keywords** → fully conforms to RFC-010's unified syntax

````

### 7. Dead-Code Elimination Mechanism

#### 7.1 Instantiation-Graph Analysis

```rust
// Compiler internals: build the generic instantiation dependency graph
struct InstantiationGraph {
    // Node: generic instantiation
    nodes: HashMap<InstanceKey, InstanceNode>,

    // Edge: usage relationship
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // Generic function ID
    type_args: Vec<TypeId>,  // Type parameters
    const_args: Vec<ConstId>,  // Const parameters
}

// Algorithm: reachability analysis
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // Start from entry points (main, exported functions, etc.)
    let entry_points = find_entry_points();
    for entry in entry_points {
        dfs_visit(entry, &graph, &mut reachable);
    }

    // Unvisited instantiations are dead code
    for node in &graph.nodes {
        if !reachable.contains(node.key) {
            eliminate(node);
        }
    }
}
````

#### 7.2 Use-Point Analysis

```yaoxiang
# Source-code analysis
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Use point 1: instantiates map(Int, Int)
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # Requires map[Int, Int]

# Use point 2: instantiates map(String, String)
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Requires map[String, String]

# Unused: map[Float, Float], etc.
# These generic instantiations will not be generated

# The compiled output contains only the instantiations that are used
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 Compile-Time Generic DCE

```yaoxiang
# Compile-time analysis: compile-time generic usage
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# Actual usage
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # Two layers: type parameters + constructor arguments
arr_100_int = Array(Int, 100)()   # Empty construction; data is assigned later

# Only the used Size is generated after compilation
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# Unused Sizes are not generated
# Array(Int, 50) will not be generated
```

#### 7.4 Cross-Module DCE

```yaoxiang
# Module A
# A.yx
pub map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Module B
# B.yx
use A.{map}
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # Instantiates map(Int, Int)

# Module C
# C.yx
use A.{map}
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Instantiates map(String, String)

# Compile-time analysis:
# - Module B uses map[Int, Int]
# - Module C uses map[String, String]
# - The compiled binary contains only these two instantiations
```

#### 7.5 LLVM-Level DCE

```rust
// Compilation pipeline
fn optimize_ir(ir: &mut IR) {
    // 1. Monomorphization (YaoXiang compiler)
    ir.monomorphize();

    // 2. Inlining optimization
    ir.inline_small_functions();

    // 3. Constant propagation
    ir.constant_propagation();

    // 4. Generate LLVM IR
    let llvm_ir = ir.to_llvm();

    // 5. LLVM optimization passes
    llvm_ir.add_pass(Passes::DEAD_CODE_ELIMINATION);
    llvm_ir.add_pass(Passes::INLINE_FUNCTION);
    llvm_ir.add_pass(Passes::GLOBAL_DCE);
    llvm_ir.add_pass(Passes::MERGE_FUNC);

    // 6. Run optimizations
    llvm_ir.run_optimization_passes();
}
```

### 8. Macro-Replacement Strategy

#### 8.1 Code-Generation Replacement

```yaoxiang
# ❌ Macro approach: code generation
macro_rules! impl_debug {
    ($($t:ty),*) => {
        $(impl Debug for $t {
            fn fmt(&self, f: &mut Formatter) -> Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

# ✅ Generic approach: automatic derivation
# Use function overloading to derive automatically
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# Usage
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # Automatically generates the call
```

#### 8.2 DSL Replacement

```yaoxiang
# ❌ Macro approach: HTML DSL
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

# ✅ Generic approach: type-safe builder
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

# Build the DOM
container = create_element("div")
    |> with_class("container")
    |> with_children(List::new())

title_elem = create_element("h1") |> with_text(title)
items_li = items.map((item) =>
    create_element("li") |> with_text(item)
)
root = container |> with_children(List::new() + [title_elem, ul_elem])
```

#### 8.3 Type-Level Programming Replacement

```yaoxiang
# ❌ Macro approach: type-level computation
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ Generic approach: conditional types
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Int, Int) => Int,
    (Float, Float) => Float,
    (Int, Float) => Float,
    (Float, Int) => Float,
    _ => TypeError,
}

# Compile-time validation
AssertAddable: (A: Type, B: Type) -> Type = If(Add(A, B) != TypeError, (A, B), compile_error("Cannot add"))

# Usage
result_type = Add[Int, Float]  # Inferred as Float
```

> **Relationship with RFC-011b (note dated 2026-09-22)**: the elevated type family `Add(A, B)` in
> this section is a type-level view of the operator-interface registry in
> [RFC-011b](011b-operator-overloading.md)—the core registration `Add(Int, Float, Float)` and the
> entry `(Int, Float) => Float` in this table are the same rule, and each user interface
> instantiation adds a row to this table. The Peano type-level `Add` in §5.2 is pure type-level
> computation (the same name but a different concept) and does not interfere with value-level
> operator interfaces—operator lookup implements registry lookup rather than name resolution.

### 9. Examples

#### 9.1 Complete Generic Container Example

```yaoxiang
# ======== 1. Define a generic container ========
# Use the (T: Type) -> Type syntax
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

List: (T: Type) -> Type = {
    data: Vec(T),
    length: Int,

    # Generic methods (T is automatically brought into scope by the outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generic methods ========
# The function is defined in the List namespace (the List. prefix = namespace ownership)
# To make . call syntax such as list.push(item) work, an explicit binding is required: List.push = push[0]
# self is only a conventional parameter name; the compiler considers the type, not the name

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # Growth
        new_data = Vec(T)(len=self.data.length * 2)
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

# ======== 3. Use type constraints ========
# Implement Clone for List
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. Usage example ========
# Create a generic List
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# Use generic methods
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# Use fold to compute
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# Generic composition
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 Generic Algorithm Example

```yaoxiang
# ======== 1. Generic sorting algorithm ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# Generic quicksort
quicksort: (T: Clone) -> ((array: Vec(T), cmp: Comparator(T)) -> Vec(T)) = {
    if array.length <= 1 {
        return array.clone()
    }

    pivot = array[array.length / 2]
    left = Vec(T)()
    right = Vec(T)()

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

# ======== 2. IntComparator implementation ========
# Use function overloading for the implementation
compare: (a: Int, b: Int) -> Int = {
    if a < b {
        return -1
    } else if a > b {
        return 1
    } else {
        return 0
    }
}

# ======== 3. Usage example ========
# Sort an Int array
numbers = Vec(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# Sort a String array (requires StringComparator)
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-Time Generic Example

```yaoxiang
# ======== 1. Compile-time matrix type ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # Compile-time dimensional validation: uses the Assert standard-library type
    _assert: Assert(Rows > 0),  # Rows > 0; otherwise, a compile error occurs
    _assert: Assert(Cols > 0),  # Cols > 0; otherwise, a compile error occurs

    # Matrix operations
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

# ======== 2. Compile-time matrix creation ========
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

# ======== 3. Usage example ========
# Create a matrix whose size is known at compile time
# 2x3 matrix
matrix_2x3 = Matrix(Float, 2, 3)()
matrix_2x3.data[0][0] = 1.0
matrix_2x3.data[0][1] = 2.0
matrix_2x3.data[0][2] = 3.0
matrix_2x3.data[1][0] = 4.0
matrix_2x3.data[1][1] = 5.0
matrix_2x3.data[1][2] = 6.0

# 3x2 matrix
matrix_3x2 = Matrix(Float, 3, 2)()
matrix_3x2.data[0][0] = 7.0
matrix_3x2.data[0][1] = 8.0
matrix_3x2.data[1][0] = 9.0
matrix_3x2.data[1][1] = 10.0
matrix_3x2.data[2][0] = 11.0
matrix_3x2.data[2][1] = 12.0

# Matrix multiplication: 2x3 * 3x2 = 2x2
result = matrix_2x3.multiply(matrix_3x2)

# Compile-time validation: the type of result is Matrix(Float, 2, 2)
# 2x2 identity matrix
identity_3x3 = identity(Float, 3)()

# Dimension mismatch: compile error
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # Compile error: 3x3 != 2x3
```

## Trade-offs

### Advantages

1. **Zero-cost abstractions**
   - Compile-time monomorphization with no runtime overhead
   - No virtual functions or RTTI required

2. **Dead-code elimination**
   - Compile-time analysis instantiates only the generics that are used
   - Controlled code bloat

3. **Macro replacement**
   - Type-safe code generation
   - IDE-friendly, with clear error messages

4. **Compile-time computation**
   - Compile-time generics support compile-time computation
   - Features such as dimensional validation
   - No `const` keyword required; pure type constraints

### Disadvantages

1. **Compilation time**
   - Generic instantiation increases compilation time
   - Constraint solving may be slow

2. **Memory usage**
   - Increased compiler memory usage
   - Caching mechanisms require memory

3. **Implementation complexity**
   - The constraint solver is complex
   - The type-level computation engine is complex

4. **Error diagnostics**
   - Generic errors may be complex
   - Clear error messages are required

### Mitigations

1. **Caching strategy**
   - Cache instantiation results
   - Use an LRU cache to limit memory usage

2. **Incremental compilation**
   - Cache compilation results
   - Perform incremental instantiation

3. **Error messages**
   - Clear error messages
   - Generic-parameter inference hints

4. **Parallel compilation**
   - Instantiate generics in parallel
   - Solve constraints with multiple threads

## Alternatives

| Alternative         | Why It Was Not Selected             |
| ------------------- | ----------------------------------- |
| Basic generics only | Cannot replace complex macros       |
| Pure macro system   | No type safety; poor error messages |
| Constraints only    | Insufficient flexibility            |
| Runtime generics    | Performance overhead                |

### Risks

| Risk                          | Impact                     | Mitigation                    |
| ----------------------------- | -------------------------- | ----------------------------- |
| Constraint-solving complexity | Excessive compilation time | Incremental solving + caching |
| Code bloat                    | Oversized binary files     | DCE + threshold controls      |
| Implementation complexity     | Extended development cycle | Phased implementation         |
| Error diagnostics             | Poor user experience       | Detailed error messages       |

## Open Questions

### Pending Decisions

| Topic                  | Description                               | Status           |
| ---------------------- | ----------------------------------------- | ---------------- |
| Instantiation strategy | Eager vs Lazy vs Threshold                | Under discussion |
| Cache size             | LRU cache capacity setting                | Under discussion |
| Error diagnostics      | Level of detail in generic error messages | Under discussion |

### Future Optimizations

| Optimization                   | Value  | Implementation difficulty |
| ------------------------------ | ------ | ------------------------- |
| Instantiation-graph analysis   | High   | Medium                    |
| Type-level programming DSL     | Medium | High                      |
| Generic performance benchmarks | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use unified () syntax as part of the function type
# For example, map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# Type constraints (in generic parameters)
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# Parameter declaration (type + name)
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# Function declaration: name: type = expression
# Generic parameters are the first parameter group in the function type: (T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# Method declaration: Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# Type definition (unified Binding syntax)
# Generic types such as List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# Type in generic parameters is automatically populated by the compiler from the argument types
# For example, in map(numbers, f), T is extracted from numbers: List(Int), and R from f: (Int) -> String
```

## Lifecycle and Disposition

```
┌─────────────┐
│   草案      │  ← 当前状态
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 开放社区讨论和反馈
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```

---

## References

### Official YaoXiang Documentation

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-009: Ownership Model](009-ownership-model.md)
- [RFC-024: spawn-Based Concurrency Runtime Semantics](024-concurrency-model.md)
- [RFC-008: Runtime Model](008-runtime-concurrency-model.md)
- [tutorial/ Tutorial](../../tutorial/index.md)

### External References

- [Rust Generic System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead-Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
