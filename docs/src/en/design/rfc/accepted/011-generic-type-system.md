---
title: 'RFC-011: Generics System Design - Zero-Cost Abstraction and Macro Replacement'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-15 (type body code blocks + compile-time contracts + effect seeds implemented)'
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

# RFC-011: Generics System Design - Zero-Cost Abstraction and Macro Replacement

## Summary

This document defines the **generics system design** of the YaoXiang language, achieving zero-cost
abstraction through powerful generics capabilities, reducing reliance on macros via compile-time
optimization, and providing a dead code elimination mechanism.

**Core design**:

- **Unified signature syntax**: `(T: Type, R: Type) -> ...` generics parameters unified with regular
  parameters
- **Type self-description mechanism**: `Type` is a language-level special existence; the `Type`
  position in signatures can be auto-inferred and filled
- **Type constraints**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated types**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time generics**: `N: Int` generics value parameters, compile-time constant instantiation
- **Conditional types**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type
  family

**Value**:

- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro use cases
- Type safety: compile-time checking, IDE-friendly
- **Explicit over implicit**: `Type` self-description, compiler auto-inference

## Reference Documents

This document's design is based on the following documents:

| Document                                                                                                   | Relationship              | Description                                                                       |
| ---------------------------------------------------------------------------------------------------------- | ------------------------- | --------------------------------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax foundation**     | Generics syntax integrated with the unified `name: type = value` model            |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call syntax**           | Section 6: generics call syntax—unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./009-ownership-model.md)                                                       | **Type system**           | Natural combination of Move semantics with generics                               |
| [RFC-024: spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution model**       | DAG analysis and generics type checking                                           |
| [RFC-008: Runtime Model](./008-runtime-concurrency-model.md)                                               | **Compiler architecture** | Generics monomorphization and compile-time optimization strategies                |
| [Type Universe Thought](../reference/plan/ongoing/类型宇宙思想.md)                                         | **Theoretical core**      | Type universe hierarchy model and value-dependent type design                     |
| [RFC-027: Compile-Time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination checking**  | Automatic measure synthesis and compile-time evaluation safety guarantee          |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generics system is built upon the **type universe thought**, a mental model that unifies
all concepts in the language into a hierarchical structure. The core innovation elevates
**value-dependent types** to first-class citizens at the Type2 layer.

### What are Value-Dependent Types?

**Value-dependent types** are types that depend on one or more **values** (not just other types).
These values can be evaluated at compile time, providing type safety guarantees at the compilation
stage.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent type: value parameters
Array: (T: Type, N: Int) -> Type  # Array type depends on length value N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column counts
```

### Container Type Naming Hierarchy

The language layer has three container concepts; the attribution of length information is their
fundamental distinction:

| Type          | Length        | Semantics                               | Underlying                                        |
| ------------- | ------------- | --------------------------------------- | ------------------------------------------------- |
| `Array(T, N)` | Type          | **Fixed-length** array, N in the type   | Core primitive (stack/inline preferred)           |
| `Vec(T)`      | Runtime value | **Runtime-length raw buffer**, growable | Core primitive (heap-allocated contiguous buffer) |
| `List(T)`     | Runtime value | Standard library type                   | Library: `{ data: Vec(T), length: Int }`          |

The division of labor among the three:

- **`Array(T, N)` is the only form that puts length in the type**—length is a compile-time constant,
  so compile-time rejection of boundary failures is possible (`a[5]` when `a: Array(Int, 3)`
  directly gives a compile-time error; see "Compile-Time Dimension Verification" below).
- **`Vec(T)` is the minimal foundation for runtime length**—only provides the four operations "can
  allocate, can take length, can read/write, can expand", with no capacity strategy, growth factor,
  or shrinking policy. It is the raw material for building other containers.
- **`List(T)` is a library type, not a primitive**—defined in YaoXiang itself in `std.list`
  (`{ data: Vec(T), length: Int }`), treated on equal footing with user-defined generics records.
  All growable semantics strategies (when to expand, by how much, whether to share) are in the
  library; the compiler is not involved.

Construction forms of `Vec(T)` (two layers: first type parameters, then construction parameters):

```yaoxiang
# Empty construction—length 0, elements appended later
v = Vec(Int)()

# Element construction—length determined by number of elements
w = Vec(Int)(1, 2, 3)          # length 3

# Slot allocation—allocate n zero-value slots
buf = Vec(Int)(len=64)         # length 64, all elements are zero values
```

> Slot allocation uses **field-name style** (`len=`) rather than positional: positional single
> integer would be ambiguous with "single-element vector" (`Vec(Int)(64)` cannot distinguish "length
> 64" from "contains one element 64"). This is consistent with the unified rule for generics
> construction: field-name style arguments are bound by name, unaffected by positional inference.
>
> This is the only primitive `List` needs for expansion—when `List` needs more space, it allocates
> new slots and moves elements:
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> When to expand, by how much, whether to shrink are all decided by `List`. `Vec` does not do
> capacity strategy.

From bottom to top, performance decreases and flexibility increases: `Array` > `Vec` > `List`.

> Naming rationale: `Vec`/`vector` in mainstream languages (Rust/C++) both refer to runtime-length
> growable sequences; `Array` specifies fixed length.

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core
advantages:

| Feature                    | Traditional Generics (C++/Rust)               | YaoXiang Value-Dependent Types                               |
| -------------------------- | --------------------------------------------- | ------------------------------------------------------------ |
| Values the type depends on | Only depends on type parameters               | Can depend on any value, including function call results     |
| Compile-time evaluation    | C++ template manual specialization, Rust none | Automatic compile-time evaluation with termination guarantee |
| Type-level computation     | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                        |
| Type safety                | C++ none, Rust limited                        | Full type safety, compile-time checking                      |
| Dimension verification     | Runtime check or manual specialization        | Compile-time dimension verification, no runtime overhead     |

### Type Universe Hierarchy and Value-Dependent Types

The type universe thought divides language concepts into different layers by semantic role;
value-dependent types are at the **Type2 layer**:

| Layer     | Role                                               | Example                                                                                                         |
| --------- | -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | Values                                             | `42`, `factorial(5)`, the function itself                                                                       |
| Type0     | Meta-type keyword                                  | `Type`                                                                                                          |
| Type1     | Concrete types                                     | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **Function/type constructor/value-dependent type** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key design**: Functions, type constructors, and value-dependent types at the Type2 layer **share
unified syntax**, all in the form `(params) -> result`:

- Regular function: `(Int, Int) -> Int` → return value is a value
- Type constructor: `(T: Type) -> Type` → return value is a type
- Value-dependent type: `(T: Type, N: Int) -> Type` → return value is a type, and depends on value
  parameter N

> **Curry-Howard isomorphism**: This unification is no coincidence. Curry-Howard isomorphism states
> "types as propositions, programs as proofs"—the function type `A → B` corresponds to logical
> implication "if A then B", generics `(T: Type) -> Type` corresponds to universal quantification
> "for all types T", value-dependent type `(n: Int) -> Type` corresponds to "for every integer n
> there exists a type". YaoXiang unifies functions, type constructors, and value-dependent types at
> the Type2 layer, essentially unifying "proof" and "computation" as a single concept—**constructive
> proof**. This is precisely the direct embodiment of Curry-Howard isomorphism in language design:
> one form (`(params) -> result`) simultaneously carries logical propositions and computational
> processes.

### Compile-Time Determinism Guarantee

YaoXiang's type universe thought requires: **everything at the Type layer is determined at compile
time**.

```yaoxiang
# Compile-time dimension verification example
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # Compile-time check: dimensions must be positive
    _assert: Assert(Rows > 0),
    _assert: Assert(Cols > 0),
}

# Create 3x3 identity matrix - completed at compile time
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# Compile-time computation: factorial(3) = 6, array size determined at compile time
arr: Array(Int, factorial(3)) = Array(Int, 6)()
```

The compiler automatically:

1. Detects function calls at type positions
2. Performs compile-time termination check on the function (see termination checking mechanism
   below)
3. Performs evaluation at compile time
4. Embeds the result into the generated type

### Application Scenarios of Value-Dependent Types

#### Compile-Time Dimension Verification

```yaoxiang
# Matrix multiplication: compile-time verification of dimension matching
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # Compile-time check: a.Cols == b.Rows, otherwise compile error
    result = Matrix(T, Rows, M)()
    # ...
}

# Error caught at compile time:
# multiply(matrix_2x3, matrix_4x2)  # Compile error: 2 != 4
```

#### Type-Safe Array Size

```yaoxiang
# Array size is a compile-time constant
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N is a compile-time constant, can be used for type-level computation
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3 (known at compile time)
```

#### Compile-Time Coverage Target for Boundary Failures

> **Implementation status note**: Container types have been despecialized—`Array(T, N)` is a const
> generics constructor, literal context landing, `in` membership predicates are all in place. The N
> of the `Array(T, N)` literal landing and element type have been enforced by compile-time
> validation (E1002), **N is now trustworthy**—the mechanism of this section can be built on
> "annotation N == runtime length". The current `[]` index out-of-bounds (E6003) and Dict missing
> key (E6008) are **runtime error transition states**; this section's value-dependent types target
> mechanism to push these boundary failures to **compile time**:
>
> - const index: `a[5]` (5 is a compile-time constant) when `a: Array(Int, 3)` is directly rejected
>   at compile time;
> - value index: `a[i]` requires the precondition `i < len(a)`, proved by value-dependent type
>   contract;
> - `in` predicate is the basis of Hoare logic preconditions: `n in 1..10`, `x in some_set` are all
>   compile-time provable propositions.
>
> The complete design of refinement types will supplement this section when landing.

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

#### Generics Functions

```yaoxiang
# map: generics function, type parameters T, R determined at compile time
map: (T: Type, R: Type) -> (
    (list: List(T), f: (x: T) -> R) -> List(R)
) = (list, f) => {
    result = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# Usage is completely transparent, types auto-inferred
numbers = List(Int)()   # Two-layer value construction form (see §9.1); elements filled with push
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                                     | C++ Templates          | Rust Generics | Haskell GADT   | **YaoXiang**                                                         |
| ----------------------------------------------------------- | ---------------------- | ------------- | -------------- | -------------------------------------------------------------------- |
| Type parameters                                             | ✅                     | ✅            | ✅             | ✅                                                                   |
| Value-dependent types                                       | ❌                     | ❌            | ✅             | ✅                                                                   |
| Compile-time evaluation                                     | Template instantiation | ❌            | ✅             | ✅                                                                   |
| Termination guarantee                                       | ❌                     | ❌            | ❌ (dangerous) | ✅ (automatic measure exploration + explicit measure, RFC-027)       |
| Type safety                                                 | ❌ (macro expansion)   | ✅            | ✅             | ✅                                                                   |
| Unified syntax                                              | ❌                     | ❌            | ❌             | ✅                                                                   |
| Compile-time dimension verification                         | Manual specialization  | Runtime check | Type family    | Automatic compile-time verification                                  |
| Semi-automatic termination annotation (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (no annotation syntax; explicit measure written at type position) |

### Termination Checking Mechanism (Unified with RFC-027)

Compile-time evaluation of value-dependent types must **guarantee termination**; otherwise the type
system will fall into an infinite loop. Termination checking is done by the compile-time proof
pipeline of RFC-027 **automatically first**—the compiler first automatically explores measures;
recursions/loops that can be proven pass; if exploration fails and no explicit measure is given, a
compile error is reported (RFC-027 §6.9 provides explicit measure fallback at type position). **No
room left for annotation syntax**: RFC-022's `//! decreases`, `/*! invariant !*/` were deprecated
with RFC-022; the contract is the type annotation itself.

> **Trigger criterion (RFC-027 §7)**: The termination obligation is triggered by **refinement
> types**—a type being refined means entering verification mode. Unrefined ordinary types do not
> enter verification mode and generate no termination obligation.

#### Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks whether the parameters strictly
decrease on every recursive path (RFC-027 §6.7). No contract comments needed:

```yaoxiang
# Recursive function with refined signature: no //! requires/ensures/decreases, compiler automatically explores decrease
factorial: (n: NonNegative(n)) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler explores: n-1 < n → decreases → terminates
}
```

| Scenario                                                             | Behavior                            |
| -------------------------------------------------------------------- | ----------------------------------- |
| Compiler can explore decrease (e.g., `n-1`)                          | Pass                                |
| Exploration fails but type position gives measure (§6.9)             | SMT judgment, passes if holds       |
| Exploration fails and no explicit measure / measure falsified by SMT | Compile error                       |
| Signature not refined (does not enter verification mode)             | No termination obligation generated |

#### Termination Checking for Loops

Loops do not need `: Invariant(...)` or `: decreases(...)` annotations. Refinement type annotations
on variables (e.g., `UpTo(n)`) simultaneously provide the loop invariant and measure bound. The
compiler tries four measure exploration strategies in priority order and stops when one is found
(RFC-027 §6.1–6.5):

1. **Automatic linear rank function synthesis**—extract variable bounds from type annotations,
   enumerate linear combinations, SMT verifies m ≥ 0 and all paths m' < m
2. **Predicate violation count** (experimental)—extract violation_count from target type definitions
   (e.g., `Sorted`), covering adjacent swap/move
3. **Bounded increment/decrement pattern**—`v += const` → measure `upper - v` (degenerate case of
   strategy 1, fastest path)
4. **Multiplicative scaling measure template**—`v *= const` (const > 1) → measure
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # Type annotation gives upper bound arr.len and lower bound 0 → enters verification mode
    while i < arr.len {
        # Compiler automatically explores: measure arr.len - i, strictly decreases by 1 each iteration → termination proven
        s += arr[i]; i += 1
    }
    return s
}
```

When exploration fails, the loop can be named and a measure given at the type position (RFC-027
§6.9):

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}
```

#### Workflow of Termination Checking

```
┌─────────────────────────────────────────────────────────────┐
│  Type checking phase                                                │
│  Encounter positions with refined types (parameter refinement, return refinement, variable refinement)      │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination check (RFC-027 proof pipeline, automatic first)                 │
│     - Recursive function: check that parameters strictly decrease on every recursive path             │
│     - Loop: four measure exploration strategies (linear rank / violation count / bounded pattern /      │
│       multiplicative scaling), SMT verifies decrease                                │
│     - Exploration fails → programmer can give measure at type position (Terminates)   │
│     - No measure / measure falsified by SMT → compile error (hard boundary)          │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time evaluation (executed by the built-in interpreter)                           │
│     - Pure function: direct evaluation                                       │
│     - Side effects: compile error (type positions must be side-effect free)                │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result embedded in type                                            │
│     - Array(Int, factorial(5)) → Array(Int, 120)             │
│     - Matrix(Float, 3, 3) → concrete type                        │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: ensures compile-time evaluation necessarily terminates, avoiding the type system
  falling into infinite loops
- **Unification**: termination checking shares the same compile-time proof pipeline (RFC-027) as
  correctness verification (VC generation), no independent contract syntax
- **Automatic first**: the compiler automatically explores measures from type annotations; passes if
  it can prove; if exploration fails, a measure can be given at the type position (`Terminates`),
  still judged by SMT—does not rely on programmers handwriting `decreases` syntax

## Motivation

### Why Do We Need a Strong Generics System?

Current mainstream languages have limitations in generics:

| Language     | Generics Capability       | Problem                                                                                    |
| ------------ | ------------------------- | ------------------------------------------------------------------------------------------ |
| Java         | Bounded types             | Compile-time monomorphization, no generics specialization                                  |
| C#           | Type constraints          | Runtime type checking, has performance overhead                                            |
| Rust         | Generics + Trait          | Trait system complex, steep learning curve                                                 |
| C++          | Templates                 | Template specialization complex, poor compile error messages                               |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, compile-time dimension verification, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: powerful type system vs ease of use
3. **Macros vs Generics**: macro code generation vs generics type safety
4. **Value Dependency vs Type Safety**: traditional generics cannot verify dimensions at compile
   time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                                     |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **Types depend on values**  | `Array: (T: Type, N: Int) -> Type` makes types depend on concrete values                                                        |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, results directly embedded in types                              |
| **Dimension verification**  | `Matrix(Float, 3, 3)` verifies matrix dimensions at compile time                                                                |
| **Type-level computation**  | `If`, `Match` and other conditional types support type-level computation                                                        |
| **Termination guarantee**   | Compile-time termination check (automatic measure synthesis per RFC-027) ensures compile-time evaluation necessarily terminates |

```yaoxiang
# Compile-time verification impossible in C++/Rust
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# Compile-time computation: factorial(3) = 6, factorial(2) = 2
# Type is Matrix(Float, 6, 2)

# Dimension mismatch caught at compile time
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # Compile error: 2 != 3
```

### Value of the Generics System

```yaoxiang
# Example: unified API design
# map operation for different container types

# Traditional approach: each type implemented separately
map_int_array: (array: Vec(Int), f: Fn(Int) -> Int) -> Vec(Int) = ...
map_string_array: (array: Vec(String), f: Fn(String) -> String) -> Vec(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# Generics approach: one generics function covers all types
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## Design Goals

### Core Goals

1. **Zero-cost abstraction** - generics calls equivalent to concrete type calls
2. **Dead code elimination** - compile-time analysis, only instantiate used generics
3. **Macro replacement** - generics replace 90% of macro use cases
4. **Type safety** - compile-time checking, no runtime type overhead
5. **IDE friendly** - smart hints, clear error messages
6. **Value-dependent types** - types can depend on values, support compile-time dimension
   verification
7. **Compile-time evaluation safety** - guarantee termination of compile-time evaluation through
   compile-time termination check (RFC-027 automatic measure synthesis)

### Design Principles

- **Compile-time determinism**: generics parameters determined at compile time
- **Monomorphization first**: generate concrete code, avoid virtual function calls
- **Constraint driven**: type constraints guide instantiation
- **Platform optimization**: specialization supports platform-specific optimization
- **Type universe unification**: functions / type constructors / value-dependent types unified at
  Type2 layer
- **Termination guarantee**: function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generics Type Parameters

> **Key rule**: Generics type definitions **must explicitly annotate `: Type`**, otherwise it will
> be inferred as a function by HM.
>
> | Syntax                            | Meaning                            |
> | --------------------------------- | ---------------------------------- |
> | `List: (T: Type) -> Type = {...}` | ✅ Type constructor                |
> | `List = {...}`                    | ❌ HM infers as function, not type |

```yaoxiang
# Generics type definition (must have : Type)
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
    push: (self: List(T), item: T) -> Void,   # self is just a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T),
}

# Generics function (no : Type, HM infers as function)
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# Generics constraint (direct expression, return can be omitted on one line)
clone: (T: Clone)(value: T) -> T = value.clone()

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### Generics Function Call Syntax

#### 1.1 Unified Signature Syntax

```yaoxiang
# Generics function uses unified (T: Type, R: Type) signature syntax
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type Self-Description Mechanism

`Type` is a language-level special existence; the compiler naturally recognizes the `Type` position
in signatures and automatically infers and fills it from actual argument types.

```yaoxiang
# Compiler automatically infers generics parameters
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         Type declaration   Construction call: Int fills T, () value construction

# Function call inference
numbers: List(Int) = List(Int)()
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# Compiler infers: T=Int, R=String
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

# Use site
int_list: List(Int) = List(Int)()
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # Instantiate map[Int, Int]

string_list: List(String) = List(String)()
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # Instantiate map[String, String]

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

#### 1.4 Explicit Filling (When Inference Fails)

````yaoxiang
# Omit Type parameters when inferrable
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# Must explicitly fill when not inferrable
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R

### 2. Type Constraint System

#### 2.1 Single Constraint

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

# Use constraint: declare type constraint directly in signature
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
````

#### 2.2 Multiple Constraints

```yaoxiang
# Multiple constraint syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# Generics container sorting
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # Implement sorting algorithm
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# Function type constraint
map: (T: Type, R: FnMut(T))(array: Vec(T), f: R) -> Vec(R) = {
    result: Vec(R) = Vec()
    for item in array {
        result.push(f(item))
    }
    return result
}

# Usage
doubled: Vec(Int) = map(Vec(1, 2, 3), (x: Int) => x * 2)  # Compiler infers
```

> **Constraint name sources (2026-09-22 note)**: `Add` / `Subtract` / `Multiply` / `Divide` /
> `Modulo` and other operator constraints are defined and landed by
> [RFC-011b: Operator Overloading and Interface-Driven Operators](./011b-operator-overloading.md)—
> `T: Add` ≜ has registered an `Add(T, T, T)` interface instantiation (three type parameters, result
> type `O` explicit). `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` currently have **no defined
> source**, are dangling constraint names, to be landed by subsequent RFCs respectively; before
> that, examples involving these names are paper sketches.

#### 2.3 Function Type Constraints

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

#### 2.4 Built-in marker trait: Dup and Clone

**Three kinds of copy semantics**:

| Type                     | Meaning                                                          | Trigger                              | Applicable scenario               |
| ------------------------ | ---------------------------------------------------------------- | ------------------------------------ | --------------------------------- |
| **Primitive value copy** | Auto value copy on assignment, two values completely independent | Auto on assignment/parameter passing | Int, Float, Bool, Char            |
| **Dup**                  | Shallow copy: copy handle/token, underlying data shared          | Auto on assignment/parameter passing | `&T` token, `ref T`, String/Bytes |
| **Clone**                | Deep copy: create complete independent replica                   | `value.clone()`                      | Any type that implements Clone    |

**Semantics of Dup**: Types implementing Dup do not transfer ownership on assignment/parameter
passing—the compiler copies the handle/token, multiple holders point to the same underlying data.
This is complementary to the default Move semantics in RFC-009's ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = copy handle, share data (modifications affect each other)
Clone = copy data, replica independent (modifications do not affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not belonging to Dup
2. Dup  — only applicable to reference/token types and internal reference-counted types
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which types are Dup**:

| Type                     | Dup     | Reason                                                                |
| ------------------------ | ------- | --------------------------------------------------------------------- |
| `&T` (borrow token)      | ✅      | Zero-size token, copying token = multiple views pointing to same data |
| `ref T`                  | ✅      | Rc/Arc copy = ref count +1, share heap data                           |
| String, Bytes            | ✅      | Internal ref count, copying handle shares underlying buffer           |
| `&mut T` (mutable token) | ❌      | Linear exclusive, cannot copy                                         |
| struct                   | Derived | All fields Dup → struct Dup                                           |
| enum                     | Derived | All variants' all fields Dup → enum Dup                               |
| tuple                    | Derived | All elements Dup → tuple Dup                                          |
| Fn (closure)             | ❌      | Captured environment may not be Dup                                   |
| `*T` (raw pointer)       | ❌      | unsafe, not participating in ownership system                         |

**Int/Float/Bool/Char are not Dup**—they are value types; the compiler automatically performs value
copy on assignment (two values are completely independent). This is not "shallow copy"; it is the
compiler's built-in handling of primitives and should not—and need not—be expressed through the Dup
type attribute.

```yaoxiang
# Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x          # Value copy, x and y completely independent
print(x)       # ✅

# Dup: shallow copy, copy handle shares data
view: &Point = &point
view2 = view    # ✅ Dup: copy token, both point to same point
print(view.x)   # ✅

# Clone: explicit deep copy, create independent replica
backup = big_struct.clone()  # Explicit call

# Generics constraints
dup_use: (T: Dup) -> T = x         # T: Dup → can shallow copy
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can deep copy
```

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety guarantees are handled
> automatically by the `ref` keyword and compiler—`ref` automatically chooses Rc or Arc, users don't
> need to understand Send/Sync.

### 3. Associated Types

#### 3.1 Associated Type Definition

```yaoxiang
# Iterator trait (using (Item: Type) -> Type syntax)
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
# Using method syntax sugar: Vec.Item, Vec.next, Vec.has_next
# Iteration position is carried by a wrapper record (Vec itself is a raw buffer, no index field)
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

#### 3.2 Generic Associated Types (GAT)

```yaoxiang
# More complex associated types
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# Associated type can be generics
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # Associated type is also generics
    iter: (Self) -> IteratorType,
}

# Usage
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. Compile-Time Generics

#### 4.1 Compile-Time Value Parameters

**Core design**: `Type` in generics signature marks type parameters; parameters annotated with
concrete types (e.g., `Int`/`Bool`/`Float`) are listed as **compile-time value parameter
candidates**; whether they become compile-time value parameters depends on whether their value is
**referenced at type position** (value dependency). No `const` keyword needed.

> The criterion is **being referenced at type position**, not "annotated with concrete type": in
> `add: (a: Int, b: Int) -> Int = a + b` `a`/`b` are runtime value parameters, because neither
> appears at any type position.

**Decision rule (two steps)**:

1. **Shape screening**: parameter annotated with a non-`Type` concrete type (e.g., `Int`) → listed
   as candidate.
2. **Usage filtering**: candidate name appears at **type position** (type body field type, inner
   `Fn` parameter type, `Assert` predicate, `Array(T, N)` and other type constructor argument
   positions) → confirmed as compile-time value parameter; otherwise treated as **runtime value
   parameter**.

| Syntax                                                     | Decision                       | Reason                                                                 |
| ---------------------------------------------------------- | ------------------------------ | ---------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters   | Only appear at value positions, not participating in type construction |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter | N appears at the type constructor argument position of `Array(T, N)`   |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter | N serves as the type of inner parameter `k`                            |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through (see below)    | N not referenced in type body, degrades to runtime value parameter     |

> **Value dependency essence**: compile-time value parameters are value-dependent types—only when a
> value is used to **construct a type** does it need to be determined at compile time. Shape
> (`: Int`) only determines candidate qualification; usage (appearing at type position) determines
> whether it is a compile-time value parameter. This is the same root criterion as "function calls
> at type positions are evaluated at compile time" in §"Compile-Time Determinism Guarantee".

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameter: N referenced at type position (Measure length slot)
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears at type constructor argument position → compile-time value parameter
    length: N,
}

# Usage: factorial(5) evaluated at type position (compile time), result 120 embedded in type
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# Value dependency: N as type of inner parameter k
# ════════════════════════════════════════════════════════
# N is a compile-time value parameter (appears at type position of (k: N));
# k is a runtime value parameter, its type is literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling of fallen-through candidates**: candidates annotated with concrete types but not
> referenced at type position (e.g., `N` in `Foo` above) degrade to runtime value parameters
> (function-level path). Fallen-through candidates of the type constructor path cannot occupy
> runtime slots (type constructors are evaluated at compile time), and the declaration side directly
> reports error [E1094]: "N is declared as a compile-time value parameter but not referenced in the
> type body"—previously silently discarded caused instantiation arity inconsistency.

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation examples
# ════════════════════════════════════════════════════════

# Compiler computes function calls of literal types at compile time
SIZE: Int = factorial(5)  # Becomes 120 at compile time

# Matrix type usage
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time dimension verification
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

# Usage: compile-time computation, generate Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never and Void: ⊥ and ⊤ of the Type System

YaoXiang's type system simultaneously possesses ⊥ (false/empty type) and ⊤ (true/Unit) in
Curry-Howard isomorphism, carried by the two built-in type names `Never` and `Void`:

**Never (⊥)** — three non-negotiable core properties:

1. **Zero constructors**: no literal or expression can produce a value of type `Never`. This is a
   meta-level property and must be built-in.
2. **Explosion principle**: `Never <: T` holds for any type `T`. A `Never` value can be used as any
   type—this is why code after `assert(false)` still passes type checking (though never actually
   executed).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   performs dead code analysis based on this.

`Never` is a built-in type name, not a keyword; the parser is oblivious. No empty sum type literal
syntax is opened up.

**Void (⊤, i.e., Unit)** — exactly one inhabitant (default void value), the carrier of the true
proposition "always true". `Void` is the identity of zero-field product type, `Never` is the
identity of zero-variant sum type—the two are dual. `x: Void = <default>` is valid, `x: Never = ...`
has no right side to write.

#### 4.3 Compile-Time Verification (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: leveraging conditional types
# ════════════════════════════════════════════════════════

# Standard library definition
# IsTrue: bridge from value universe to type universe—Bool truth value maps to type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has value, program continues
    false => Never,    # ⊥, no value, diverges
}

# Assert: compile-time refinement type primitive—type-level expression of Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error/diverges)
# cond cannot be decided → proof pipeline decides by dispatch mode:
#                  CompileTime → Unknown, requires prove
#                  Runtime     → insert check, inject Γ assumption

# Usage mode 1: as constraint in type definition
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at type position)
    length: Assert(N > 0),
}

# Usage mode 2: use in expression
IntArray: (N: Int) -> Type = Array(Int, N)
# Verify: size of IntArray(10) equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generics Specialization

```yaoxiang
# Small array optimization: compile-time generics specialization via function overloading

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

# Small array loop unrolling (N <= 4)
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    # Compiler optimization: unroll loop
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. Conditional Types

> **Curry-Howard isomorphism**: Conditional types, from the Curry-Howard perspective, are **case
> analysis** in logic. The `Bool` type corresponds to a proposition with two possible values
> (True/False), and `If` chooses different results based on whether that proposition is true or
> false—this is precisely the case disjunction in logic. `match C { True => T, False => E }` is
> actually expressing: "when the known proposition C is True, the conclusion is T; when C is False,
> the conclusion is E".

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

# Compile-time verification (unified with Assert definition in §4.3)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Family

> **Curry-Howard isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but **constructing a proposition about natural number addition**. `(Zero, B) => B` says "the
> proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` says "if Add(A',
> B) holds, then Add(Succ(A'), B) also holds". This is exactly the addition definition in Peano
> axioms. The type checker verifying that this match expression passes is equivalent to verifying
> the logical consistency of this definition.

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

# Type-level addition (Curry-Howard: case analysis + recursive call, needs termination check to be complete induction)
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# Example: compile-time computation 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. Function Overloading Specialization

#### 6.1 Basic Specialization

```yaoxiang
# Basic specialization: using function overloading (compiler auto-selects)
sum: (arr: Vec(Int)) -> Int = {
    # Compiled into more efficient code
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    # Use SIMD instructions
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
# Fully conforming to RFC-010 syntax specialization: function overloading

# Concrete type specialization
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# Generics implementation (compiler auto-selects optimal)
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# Completely transparent at use site
int_arr = Vec(Int)(1, 2, 3)
float_arr = Vec(Float)(1.0, 2.0, 3.0)

# Compiler auto-selects optimal specialization
sum(int_arr)     # Selects sum: (Vec(Int)) -> Int
sum(float_arr)    # Selects sum: (Vec(Float)) -> Float
```

#### 6.3 Perfect Combination of Function Overloading and Inlining

**Key feature**: Function overloading naturally combines with inlining optimization to achieve
zero-cost abstraction.

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

# ======== After compilation (equivalent code) =======
# Compiler auto-selects optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core advantages**:

1. **Compiler intelligent selection**

   ```yaoxiang
   sum(int_arr)      # Auto-selects sum: (Vec(Int)) -> Int
   sum(float_arr)    # Auto-selects sum: (Vec(Float)) -> Float
   sum(custom_arr)  # Auto-selects sum: (T: Type) -> ((arr: Vec(T)) -> T)
   ```

2. **Inlining optimization**
   - Small functions auto-inlined at call site
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function table needed

4. **Perfect fit with RFC-010**

   ```yaoxiang
   # Fully uses unified syntax
   name: type = value
   # No need for impl, where and other new keywords
   ```

**Practical application example**:

```yaoxiang
# Performance-sensitive numerical computation
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Use Binet's formula
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# Compiler auto-selects and inlines
fibonacci(10)      # Selects Int version, completely inlined
fibonacci(10.5)    # Selects Float version, uses Binet's formula
```

**What does this mean?**

- ✅ **Generics specialization** → function overloading naturally solves it
- ✅ **Performance optimization** → inlining done automatically
- ✅ **Code reuse** → one function name, multiple implementations
- ✅ **Zero-cost abstraction** → compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords needed** → perfectly conforms to RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internals: build generics instantiation dependency graph
struct InstantiationGraph {
    // Nodes: generics instantiations
    nodes: HashMap<InstanceKey, InstanceNode>,

    // Edges: usage relationships
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // generics function ID
    type_args: Vec<TypeId>,  // type parameters
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

#### 7.2 Use Site Analysis

```yaoxiang
# Source code analysis
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Use site 1: instantiate map(Int, Int)
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # Needs map[Int, Int]

# Use site 2: instantiate map(String, String)
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Needs map[String, String]

# Not used: map[Float, Float] etc.
# These generics instances will not be generated

# After compilation only contains used instances
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 Compile-Time Generics DCE

```yaoxiang
# Compile-time analysis: compile-time generics usage
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# Actual usage
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # Two layers: type parameters + construction parameters
arr_100_int = Array(Int, 100)()   # Empty construction, data assigned later

# After compilation only generates used Sizes
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# Unused Sizes will not be generated
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
doubled = map(int_list, (x) => x * 2)  # Instantiate map(Int, Int)

# Module C
# C.yx
use A.{map}
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Instantiate map(String, String)

# Compile analysis:
# - Module B uses map[Int, Int]
# - Module C uses map[String, String]
# - Compiled binary only contains these two instances
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

    // 6. Run optimization passes
    llvm_ir.run_optimization_passes();
}
```

### 8. Macro Replacement Strategy

#### 8.1 Code Generation Replacement

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

# ✅ Generics approach: auto-derive
# Use function overloading for auto-derivation
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# Usage
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # Auto-generated call
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

# ✅ Generics approach: type-safe builder
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

# Build DOM
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

# ✅ Generics approach: conditional type
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Int, Int) => Int,
    (Float, Float) => Float,
    (Int, Float) => Float,
    (Float, Int) => Float,
    _ => TypeError,
}

# Compile-time verification
AssertAddable: (A: Type, B: Type) -> Type = If(Add(A, B) != TypeError, (A, B), compile_error("Cannot add"))

# Usage
result_type = Add[Int, Float]  # Inferred as Float
```

> **Relationship with RFC-011b (2026-09-22 note)**: The promotion type family `Add(A, B)` in this
> section is the perspective of the [RFC-011b](./011b-operator-overloading.md) operator interface
> registry at the type level—the core registration `Add(Int, Float, Float)` and the entry
> `(Int, Float) => Float` in this table are the same rule; each interface instantiation by the user
> is adding a row to this table. The Peano type-level `Add` in §5.2 is purely type-level computation
> (same name, different thing), and does not interfere with the value-level operator
> interface—operator query goes to the implementation registry, not name resolution.

### 9. Examples

#### 9.1 Complete Generics Container Example

```yaoxiang
# ======== 1. Define generics container ========
# Using (T: Type) -> Type syntax
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

    # Generics methods (T auto-brought into scope by outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generics methods ========
# Function definition under the List namespace (List. prefix = namespace ownership)
# To make list.push(item) this kind of . call syntax work, need explicit binding: List.push = push[0]
# self is just a conventional parameter name, the compiler looks at type not name

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # Expand
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

# ======== 3. Type constraint usage ========
# Implement Clone for List
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. Usage example ========
# Create generics List
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# Use generics methods
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# Use fold for computation
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# Generics composition
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 Generics Algorithm Example

```yaoxiang
# ======== 1. Generics sorting algorithm ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# Generics quicksort
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
# Use function overloading
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
# Sort Int array
numbers = Vec(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# Sort String array (needs StringComparator)
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-Time Generics Example

```yaoxiang
# ======== 1. Compile-time matrix type ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # Compile-time dimension verification: using Assert standard library type
    _assert: Assert(Rows > 0),  # Rows > 0, otherwise compile error
    _assert: Assert(Cols > 0),  # Cols > 0, otherwise compile error

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
# Create matrix with compile-time known size
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

# Compile-time verification: result type is Matrix(Float, 2, 2)
# 2x2 identity matrix
identity_3x3 = identity(Float, 3)()

# Dimension mismatch: compile error
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # Compile error: 3x3 != 2x3
```

## Trade-offs

### Advantages

1. **Zero-cost abstraction**
   - Compile-time monomorphization, no runtime overhead
   - No virtual functions, no RTTI

2. **Dead code elimination**
   - Compile-time analysis, only instantiate used generics
   - Code bloat controllable

3. **Macro replacement**
   - Type-safe code generation
   - IDE-friendly, clear error messages

4. **Compile-time computation**
   - Compile-time generics support compile-time computation
   - Features like dimension verification
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compile time**
   - Generics instantiation increases compile time
   - Constraint solving may be slow

2. **Memory footprint**
   - Compiler memory footprint increases
   - Cache mechanism requires memory

3. **Implementation complexity**
   - Constraint solver complex
   - Type-level computation engine complex

4. **Error diagnosis**
   - Generics errors may be complex
   - Clear error messages needed

### Mitigations

1. **Caching strategy**
   - Cache instantiation results
   - LRU cache limits memory

2. **Incremental compilation**
   - Cache compile results
   - Incremental instantiation

3. **Error messages**
   - Clear error messages
   - Generics parameter inference hints

4. **Parallel compilation**
   - Parallel generics instantiation
   - Multi-threaded constraint solving

## Alternative Solutions

| Solution                    | Why not chosen                      |
| --------------------------- | ----------------------------------- |
| Basic generics only         | Cannot replace complex macros       |
| Pure macro system           | No type safety, poor error messages |
| Dependency constraints only | Insufficient flexibility            |
| Runtime generics            | Has performance overhead            |

### Risks

| Risk                         | Impact                     | Mitigation                    |
| ---------------------------- | -------------------------- | ----------------------------- |
| Constraint solver complexity | Compile time too long      | Incremental solving + caching |
| Code bloat                   | Binary file too large      | DCE + threshold control       |
| Implementation complexity    | Development cycle extended | Phased implementation         |
| Error diagnosis              | Poor user experience       | Detailed error messages       |

## Open Questions

### Pending Issues

| Topic                  | Description                         | Status          |
| ---------------------- | ----------------------------------- | --------------- |
| Instantiation strategy | Eager vs Lazy vs Threshold          | To be discussed |
| Cache size             | LRU cache capacity setting          | To be discussed |
| Error diagnosis        | Generics error message detail level | To be discussed |

### Future Optimizations

| Optimization item              | Value  | Implementation difficulty |
| ------------------------------ | ------ | ------------------------- |
| Instantiation graph analysis   | High   | Medium                    |
| Type-level programming DSL     | Medium | High                      |
| Generics performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generics parameters use unified () syntax as part of function type
# E.g., map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# Type constraint (in generics parameters)
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# Parameter declaration (type + name)
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# Function declaration: name: type = expression
# Generics parameters are the first parameter group in function type: (T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# Method declaration: Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# Type definition (unified Binding syntax)
# Generics type like List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# Type in generics parameters is auto-filled by the compiler from actual argument types
# E.g., map(numbers, f), T extracted from numbers: List(Int), R extracted from f: (Int) -> String
```

## Lifecycle and Destination

```
┌─────────────┐
│   Draft      │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under review     │  ← Open community discussion and feedback
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted     │    │  Rejected     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (formal design)  │    │ (retain original location)  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│ Implemented  │    │ Archived   │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documents

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./009-ownership-model.md)
- [RFC-001: spawn Model](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./008-runtime-concurrency-model.md)
- [tutorial/ Tutorials](../../../../../tutorial/)

### External References

- [Rust Generics System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
