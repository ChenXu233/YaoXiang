---
title: 'RFC-011: Generic Type System Design - Zero-Cost Abstraction and Macro Replacement'
status: 'Accepted'
author: 'Chen Xu'
updated: '2026-07-15 (type body code blocks + compile-time specifications + effect seeds implemented)'
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

# RFC-011: Generic Type System Design - Zero-Cost Abstraction and Macro Replacement

## Abstract

This document defines the **generic type system design** for YaoXiang language, achieving zero-cost abstraction through powerful generic capabilities, leveraging compile-time optimization to reduce reliance on macros, and providing dead code elimination mechanisms.

**Core design**:

- **Unified Signature Syntax**: `(T: Type, R: Type) -> ...` - generic parameters and regular parameters are unified
- **Type Self-Description Mechanism**: `Type` is a language-level special entity; `Type` positions in signatures can be automatically inferred and filled
- **Type Constraints**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated Types**: `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-Time Generics**: `N: Int` generic value parameters, compile-time constant instantiation
- **Conditional Types**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type families

**Value**:

- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro use cases
- Type safety: compile-time checking, IDE-friendly
- **Explicit over implicit**: `Type` self-description, compiler auto-inference

## Reference Documents

The design of this document is based on the following documents:

| Document                                                                      | Relationship     | Description                                               |
| ---------------------------------------------------------------------------- | ---------------- | --------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                 | **Syntax Basis** | Generic syntax integrated with unified `name: type = value` model |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                 | **Call Syntax**  | Section 6: Generic call syntax - unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./009-ownership-model.md)                | **Type System**  | Natural combination of Move semantics and generics       |
| [RFC-024: Concurrency Runtime Semantics Based on Spawn](./024-concurrency-model.md) | **Execution Model** | DAG analysis and generic type checking              |
| [RFC-008: Runtime Model](./008-runtime-concurrency-model.md)        | **Compiler Architecture** | Generic monomorphization and compile-time optimization strategies |
| Type Universe Theory (see section below)                                     | **Theoretical Core** | Type universe hierarchy model and value-dependent type design |
| [RFC-027: Compile-Time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination Checking** | Automated measure synthesis and compile-time evaluation safety guarantees |

## Type Universe Theory and Value-Dependent Types

YaoXiang's generic system is built on the **Type Universe Theory**, a mental model that unifies all concepts in the language into a hierarchical structure. The core innovation is elevating **value-dependent types** to a first-class citizen in Type2.

### What Are Value-Dependent Types?

**Value-dependent types** are types that depend on one or more **values** (not just on other types). These values can be evaluated at compile time, providing type safety guarantees at the compilation stage.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent type: value parameters
Array: (T: Type, N: Int) -> Type  # Array type depends on length value N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column counts
```

### Container Type Naming Hierarchy

There are three container concepts at the language level, and the ownership of length information is their fundamental difference:

| Type          | Length     | Semantics                            | Underlying                          |
| ------------- | ---------- | ------------------------------------ | ----------------------------------- |
| `Array(T, N)` | Type       | **Fixed-length** array, N in type    | Core primitive (stack/inline-first) |
| `Vec(T)`      | Runtime value | **Runtime-length primitive buffer**, growable | Core primitive (heap contiguous buffer) |
| `List(T)`     | Runtime value | Standard library type               | Library: `{ data: Vec(T), length: Int }` |

Three-way division of responsibilities:

- **`Array(T, N)` is the only form that puts length into the type**—length is a compile-time constant, so out-of-bounds failures can be rejected at compile time (`a[5]` when `a: Array(Int, 3)` directly causes a compile-time error, see "Compile-Time Dimension Verification" below).
- **`Vec(T)` is the minimal foundation for runtime-length**—only provides four things: "can allocate, can get length, can read/write, can grow". Capacity strategy, growth factor, and whether to shrink are all out of scope. It's the raw material for building other containers.
- **`List(T)` is a library type, not a primitive**—defined in YaoXiang itself in `std.list` (`{ data: Vec(T), length: Int }`), treated the same as user-defined generic records. All growable semantics strategies (when to expand, by how much, whether sharing is allowed) are in the library; the compiler doesn't participate.

`Vec(T)` construction forms (two layers: type parameters first, then construction parameters):

```yaoxiang
# Empty construction - length 0, elements added later
v = Vec(Int)()

# Element construction - length determined by element count
w = Vec(Int)(1, 2, 3)          # length 3

# Slot allocation - allocate n zero-value slots
buf = Vec(Int)(len=64)         # length 64, all elements are zero values
```

> Slot allocation uses **field-name style** (`len=`) instead of positional style: a single integer in positional style would be ambiguous with "single-element vector" (`Vec(Int)(64)` cannot distinguish between "length 64" and "contains one element 64"). This is consistent with the unified rules for generic construction: field-name style arguments are bound by name and are not affected by position inference.
>
> This is the only primitive needed for `List` expansion—`List` allocates new slots and moves elements when needed:
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> When to expand, by how much, whether to shrink are all decided by `List`. `Vec` does no capacity strategy.

Bottom-up, performance decreases, flexibility increases: `Array` > `Vec` > `List`.

> Naming rationale: `Vec`/`vector` in mainstream languages (Rust/C++) both refer to runtime-length growable sequences; `Array` specifies length.

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core advantages:

| Feature           | Traditional Generics (C++/Rust)     | YaoXiang Value-Dependent Types      |
| ----------------- | ------------------------------------ | ---------------------------------- |
| Type-dependent values | Only depends on type parameters      | Can depend on any value, including function call results |
| Compile-time evaluation | C++ manual template specialization, Rust none | Automatic compile-time evaluation, guaranteed termination |
| Type-level computation | Template metaprogramming (complex/dangerous) | Unified type-level computation engine |
| Type safety       | C++ none, Rust limited               | Complete type safety, compile-time checking |
| Dimension verification | Runtime checking or manual specialization | Compile-time dimension verification, no runtime overhead |

### Type Universe Hierarchy and Value-Dependent Types

Type Universe Theory categorizes language concepts by semantic role into different levels, with value-dependent types located at **Type2**:

| Level      | Role                                | Example                                                                                                       |
| ---------- | ----------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Type-1     | Values                              | `42`, `factorial(5)`, function itself                                                                        |
| Type0      | Metatype keyword                    | `Type`                                                                                                        |
| Type1      | Concrete types                      | `Int`, `String`, `Array(Int, 3)`                                                                              |
| **Type2**  | **Functions/Type Constructors/Value-Dependent Types** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at Type2 layer are **unified in syntax**, all in the form `(params) -> result`:

- Regular functions: `(Int, Int) -> Int` → return value is a value
- Type constructors: `(T: Type) -> Type` → return value is a type
- Value-dependent types: `(T: Type, N: Int) -> Type` → return value is a type, and depends on value parameter N

> **Curry-Howard Isomorphism**: This unification is not a coincidence. The Curry-Howard isomorphism states that "types are propositions, programs are proofs"—function type `A → B` corresponds to logical implication "if A then B", generic `(T: Type) -> Type` corresponds to universal quantification "for all types T", value-dependent type `(n: Int) -> Type` corresponds to "for each integer n there exists a type". YaoXiang unifies functions, type constructors, and value-dependent types at the Type2 layer, essentially unifying "proofs" and "computation"—**constructive proofs**. This is the direct manifestation of Curry-Howard isomorphism in language design: one form (`(params) -> result`) carries both logical propositions and computational processes.

### Compile-Time Determinism Guarantee

YaoXiang's Type Universe Theory requires: **everything at the Type level is compile-time deterministic**.

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

1. Detect function calls at type positions
2. Perform compile-time termination checking on functions (see termination checking mechanism below)
3. Evaluate at compile time
4. Embed the result in the generated type

### Application Scenarios for Value-Dependent Types

#### Compile-Time Dimension Verification

```yaoxiang
# Matrix multiplication: compile-time dimension matching verification
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

#### Compile-Time Coverage Targets for Bounds Failures

> **Implementation Status Note**: Container types have been de-specialized—`Array(T, N)` is a const generic constructor, literal contexts and `in` membership predicates are already implemented. The N in `Array(T, N)` literal contexts has been enforced by compile-time checks (E1002), **N is now trusted**—the mechanism in this section can be built on "annotate N == runtime length". Currently `[]` index out-of-bounds (E6003) and Dict missing key (E6008) are **runtime error transition states**; the value-dependent types in this section are the target mechanism for pushing these bounds failures to **compile time**:
>
> - Const index: `a[5]` (5 is a compile-time constant) when `a: Array(Int, 3)` directly rejected at compile time;
> - Value index: `a[i]` requires precondition `i < len(a)`, proven by value-dependent type contracts;
> - `in` predicate is the basis of Hoare logic preconditions: `n in 1..10`, `x in some_set` are both compile-time provable propositions.
>
> The complete design of refinement types will be supplemented separately when implemented.

#### Conditional Types

```yaoxiang
# Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# Type families
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,
}
```

#### Generics Functions

```yaoxiang
# map: generic function, type parameters T, R determined at compile time
map: (T: Type, R: Type) -> (
    (list: List(T), f: (x: T) -> R) -> List(R)
) = (list, f) => {
    result = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# Usage is completely transparent, types automatically inferred
numbers = List(Int)()   # value construction has two layers (see §9.1); elements filled with push
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                | C++ Templates   | Rust Generics | Haskell GADT | **YaoXiang**                        |
| -------------------------------------- | --------------- | ------------- | ------------ | ----------------------------------- |
| Type parameters                        | ✅              | ✅            | ✅           | ✅                                  |
| Value-dependent types                  | ❌              | ❌            | ✅           | ✅                                  |
| Compile-time evaluation                | Template instantiation | ❌      | ✅           | ✅                                  |
| Termination guarantee                  | ❌              | ❌            | ❌ (dangerous) | ✅ (automatic measure exploration + explicit measures, RFC-027) |
| Type safety                            | ❌ (macro expansion) | ✅       | ✅           | ✅                                  |
| Unified syntax                         | ❌              | ❌            | ❌           | ✅                                  |
| Compile-time dimension verification    | Manual specialization | Runtime checks | Type families | Compile-time automatic verification |
| Semi-automatic termination annotations (decreases/invariant) | ❌ | ❌       | ❌           | ❌ (no annotation syntax; explicit measures written in type position) |

### Termination Checking Mechanism (Unified with RFC-027)

Compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the type system will fall into infinite loops. Termination checking is performed **fully automatically with priority** by RFC-027's compile-time proof pipeline—the compiler first automatically explores measures, recursive/loops that can be proven pass; if exploration fails and no explicit measures are given, a compile error is reported (RFC-027 §6.9 gives explicit measure fallback in type position). **No opening for annotation syntax**: RFC-022's `//! decreases`, `/*! invariant !*/` have been deprecated with RFC-022; the specification is the type annotation itself.

> **Trigger Criterion (RFC-027 §7)**: Termination obligation is triggered by **refinement types**—when a type is refined, it enters verification mode. Ordinary types without refinement do not enter verification mode and do not generate termination obligations.

#### Termination Checking for Recursive Functions

The compiler checks recursive functions with refined signatures, verifying that recursive call arguments strictly decrease on every recursive path (RFC-027 §6.7). No specification comments needed:

```yaoxiang
# Recursive with refined signature: no //! requires/ensures/decreases, compiler automatically explores decreasing
factorial: (n: NonNegative(n)) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler explores: n-1 < n → decreasing → terminates
}
```

| Scenario                                     | Behavior                                  |
| -------------------------------------------- | ----------------------------------------- |
| Compiler can explore decreasing (e.g., `n-1`) | Pass                                      |
| Cannot explore but measure given in type position (§6.9) | SMT decides, passes if true     |
| Cannot explore and no explicit measure / measure refuted by SMT | Compile error           |
| Signature without refinement (not in verification mode) | No termination obligation generated |

#### Termination Checking for Loops

Loops do not need `: Invariant(...)` or `: decreases(...)` annotations. Refinement type annotations on variables (such as `UpTo(n)`) provide both loop invariants and measure bounds simultaneously. The compiler tries four measure exploration strategies in priority order, stops when one is found (RFC-027 §6.1–6.5):

1. **Linear rank function automatic synthesis**—extract variable bounds from type annotations, enumerate linear combinations, SMT verifies m ≥ 0 and all paths m' < m
2. **Predicate violation counting** (experimental)—extract from target type definitions (like `Sorted`) violation_count, covering adjacent swaps/moves
3. **Bounded increment/decrement patterns**—`v += const` → measure `upper - v` (degenerate case of strategy 1, fastest path)
4. **Multiplicative scaling measure template**—`v *= const` (const > 1) → measure `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # Type annotation gives upper bound arr.len and lower bound 0 → enters verification mode
    while i < arr.len {
        # Compiler automatically explores: measure arr.len - i, strictly decreasing by 1 each iteration → termination proven
        s += arr[i]; i += 1
    }
    return s
}
```

When exploration fails, you can name the loop and give a measure in the type position (RFC-027 §6.9):

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}
```

#### Termination Checking Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  Type Checking Phase                                         │
│  Encounter positions with refined types (parameter refinement, return refinement, variable refinement) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination Checking (RFC-027 proof pipeline, automatic priority) │
│     - Recursive functions: check arguments strictly decrease on every recursive path │
│     - Loops: four measure exploration strategies (linear rank/violation counting/bounded patterns/multiplicative scaling), SMT verifies decreasing │
│     - Cannot explore → programmer can give measure in type position (Terminates) │
│     - No measure / measure refuted by SMT → compile error (hard boundary) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-Time Evaluation (executed by builtin interpreter) │
│     - Pure functions: direct evaluation                      │
│     - Side effects: compile error (type positions must be side-effect free) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Embed Results in Types                                   │
│     - Array(Int, factorial(5)) → Array(Int, 120)             │
│     - Matrix(Float, 3, 3) → concrete type                   │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensures compile-time evaluation always terminates, avoiding infinite loops in the type system
- **Uniformity**: Termination checking and correctness verification (VC generation) share the same compile-time proof pipeline (RFC-027), no separate specification syntax
- **Automatic Priority**: Compiler automatically explores measures from type annotations; if it can prove, it passes; if it cannot explore, it can give measures in type position (`Terminates`), still decided by SMT—no reliance on programmers writing `decreases` syntax

## Motivation

### Why Do We Need a Strong Generics System?

Current mainstream language generics have limitations:

| Language       | Generic Capability     | Problem                                    |
| -------------- | ---------------------- | ------------------------------------------ |
| Java           | Bounded types          | Compile-time monomorphization, no generic specialization |
| C#             | Generic constraints    | Runtime type checking, performance overhead |
| Rust           | Generics + Traits      | Trait system is complex, steep learning curve |
| C++            | Templates              | Template specialization is complex, poor compile error messages |
| **YaoXiang**   | **Value-Dependent Types** | **Types can depend on values, compile-time dimension verification, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: powerful type system vs usability
3. **Macros vs Generics**: macro code generation vs generic type safety
4. **Value-Dependent vs Type Safety**: traditional generics cannot verify dimensions at compile time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage             | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| **Types depend on values** | `Array: (T: Type, N: Int) -> Type` makes types depend on specific values |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, results directly embedded in types |
| **Dimension verification** | `Matrix(Float, 3, 3)` verifies matrix dimensions at compile time |
| **Type-level computation** | `If`, `Match` and other conditional types support type-level computation |
| **Termination guarantee** | Compile-time termination checking (automatic measure synthesis) ensures compile-time evaluation always terminates |

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
# Example: Unified API Design
# map operations for different container types

# Traditional approach: implement separately for each type
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

1. **Zero-cost abstraction** - generic calls are equivalent to concrete type calls
2. **Dead code elimination** - compile-time analysis, only instantiate used generics
3. **Macro replacement** - generics replace 90% of macro use cases
4. **Type safety** - compile-time checking, no runtime type overhead
5. **IDE-friendly** - intelligent hints, clear error messages
6. **Value-dependent types** - types can depend on values, supporting compile-time dimension verification
7. **Compile-time evaluation safety** - compile-time termination checking (RFC-027 automatic measure synthesis) ensures compile-time evaluation terminates

### Design Principles

- **Compile-time determined**: generic parameters are determined at compile time
- **Monomorphization priority**: generate concrete code, avoid virtual function calls
- **Constraint-driven**: type constraints guide instantiation
- **Platform optimization**: specialization supports platform-specific optimization
- **Type universe unification**: functions/type constructors/value-dependent types unified at Type2 layer
- **Termination guarantee**: function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generics Type Parameters

> **Key Rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise HM will infer them as functions.
>
> | Syntax                                | Meaning                       |
> | ------------------------------------- | ----------------------------- |
> | `List: (T: Type) -> Type = {...}`     | ✅ Type constructor           |
> | `List = {...}`                        | ❌ HM infers as function, not type |

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
# Generic functions use unified (T: Type, R: Type) signature syntax
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type Self-Description Mechanism

`Type` is a language-level special entity; the compiler naturally recognizes `Type` positions in signatures and automatically infers and fills them from actual argument types.

```yaoxiang
# Compiler automatically infers generics parameters
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         type declaration   construction call: Int fills T, () is value construction

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

# Usage point 1: instantiate map[Int, Int]
int_list: List(Int) = List(Int)()
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # instantiate map[Int, Int]

string_list: List(String) = List(String)()
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # instantiate map[String, String]

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

#### 1.4 Explicit Fill (When Inference Fails)

```yaoxiang
# Can omit Type parameters when inferable
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# Must explicitly fill when inference fails
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

# Using constraints: declare type constraints directly in signature
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
```

#### 2.2 Multiple Constraints

```yaoxiang
# Multiple constraint syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# Sorting generic containers
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # Sorting algorithm implementation
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# Function type constraints
map: (T: Type, R: FnMut(T))(array: Vec(T), f: R) -> Vec(R) = {
    result: Vec(R) = Vec()
    for item in array {
        result.push(f(item))
    }
    return result
}

# Usage
doubled: Vec(Int) = map(Vec(1, 2, 3), (x: Int) => x * 2)  # compiler infers
```

> **Constraint Name Sources (2026-09-22 Note)**: Operator constraints like `Add` / `Subtract` / `Multiply` / `Divide` / `Modulo` are defined and implemented by [RFC-011b: Operator Overloading and Interface-Driven Operators](./011b-operator-overloading.md)—`T: Add` ≜ `Add(T, T, T)` interface instance registered (three type parameters, result type `O` explicit). `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` currently have **no definition source** and are dangling constraint names, pending subsequent RFCs; before then, examples involving these names are illustrative.

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

#### 2.4 Builtin Marker Traits: Dup and Clone

**Three types of copy semantics**:

| Type               | Meaning                                      | Trigger                   | Use Cases                              |
| ------------------ | -------------------------------------------- | ------------------------- | -------------------------------------- |
| **Primitive value copy** | Automatic value copy on assignment, two values completely independent | Automatic on assignment/passing | Int, Float, Bool, Char            |
| **Dup**            | Shallow copy: copy handles/tokens, underlying data shared | Automatic on assignment/passing | `&T` tokens, `ref T`, String/Bytes |
| **Clone**          | Deep copy: create complete independent copy  | `value.clone()`           | Any type implementing Clone            |

**Dup semantics**: Types implementing Dup do not transfer ownership on assignment/passing—the compiler copies handles/tokens, multiple holders point to the same underlying data. This complements the default Move semantics in RFC-009 ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = copy handles, share data (changes affect each other)
Clone = copy data, copies independent (changes don't affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler builtin value copy, not Dup
2. Dup — only applicable to reference/token types and types with internal reference counting
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which types are Dup**:

| Type                | Dup  | Reason                                             |
| ------------------- | ---- | -------------------------------------------------- |
| `&T` (borrow token) | ✅   | Zero-size token, copying token = multiple views pointing to same data |
| `ref T`             | ✅   | Rc/Arc copy = refcount +1, shared heap data        |
| String, Bytes       | ✅   | Internal refcount, copying handles shares underlying buffer |
| `&mut T` (mutable token) | ❌ | Linear exclusive, cannot copy                   |
| struct               | derived | All fields Dup → struct Dup                  |
| enum                | derived | All variant fields Dup → enum Dup            |
| tuple               | derived | All elements Dup → tuple Dup                 |
| Fn (closure)        | ❌   | Captured environment may not be Dup               |
| `*T` (raw pointer)  | ❌   | unsafe, not part of ownership system              |

**Int/Float/Bool/Char are NOT Dup**—they are value types, the compiler automatically value-copies on assignment (two values completely independent). This is not "shallow copy"; it's the compiler's builtin handling of primitives, neither needed nor should be expressed through Dup type properties.

```yaoxiang
# Primitive value types: compiler automatically value copies (not Dup)
x: Int = 42
y = x          # value copy, x and y completely independent
print(x)       # ✅

# Dup: shallow copy, copy handles share data
view: &Point = &point
view2 = view    # ✅ Dup: copy token, both point to same point
print(view.x)   # ✅

# Clone: explicit deep copy, create independent copy
backup = big_struct.clone()  # explicit call

# Generics constraints
dup_use: (T: Dup) -> T = x         # T: Dup → can shallow copy
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can deep copy
```

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety guarantees are handled by the `ref` keyword and compiler fully automatically—`ref` automatically chooses Rc or Arc, users don't need to understand Send/Sync.

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
# Using method sugar: Vec.Item, Vec.next, Vec.has_next
# Iteration position carried by wrapper record (Vec itself is a primitive buffer, no index field)
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

**Core Design**: Parameters annotated with concrete types (`Int`/`Bool`/`Float` etc.) in generic signatures are **compile-time value parameter candidates**; whether they become compile-time value parameters depends on whether their values are **referenced at type positions** (value-dependent). No `const` keyword needed.

> The determination is based on **being referenced at type positions**, not "annotated with concrete types": in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters because neither appears at any type position.

**Determination Rules (two steps)**:

1. **Form coarse filter**: parameters annotated with concrete non-`Type` types (like `Int`) → listed as candidates.
2. **Usage fine filter**: candidate names appear at **type positions** (type body field types, inner `Fn` parameter types, `Assert` predicates, `Array(T, N)` type constructor argument positions) → confirmed as compile-time value parameters; otherwise treated as **runtime value parameters**.

| Syntax                                                           | Determination            | Reason                                        |
| ---------------------------------------------------------------- | ------------------------ | --------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                          | a/b runtime value params | only appear at value positions, not in type construction |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }`      | N compile-time value param | N appears in `Array(T, N)` type constructor argument position |
| `factorial: (N: Int) -> (k: N) -> Int`                          | N compile-time value param | N as type of inner parameter `k`             |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                     | N dropped (see below)    | N not referenced in type body, degrades to runtime value param |

> **Value-Dependent Essence**: Compile-time value parameters are value-dependent types—only when a value is used to **construct a type** does it need to be compile-time determined. Form (`: Int`) only determines candidacy; usage (type position exposure) determines whether it's a compile-time value parameter. This is the same criterion as "function calls at type positions are evaluated at compile time" in § "Compile-Time Determinism Guarantee".

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameters: N referenced at type position (Measure length slot)
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears at type constructor argument position → compile-time value param
    length: N,
}

# Usage: factorial(5) evaluated at type position (compile-time), result 120 embedded in type
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# Value-dependent: N as type of inner parameter k
# ════════════════════════════════════════════════════════
# N is compile-time value param (appears at type position of (k: N));
# k is runtime value param, its type is literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling Dropped Candidates**: Candidates annotated with concrete types but not referenced at type positions (like `Foo`'s `N` in the table above) degrade to runtime value parameters (function-level path). Dropped candidates at type constructor paths cannot occupy runtime slots (type constructors are evaluated at compile time), so the declaration side directly reports error [E1094]: "N declared as compile-time value parameter but not referenced in type body"—previously silent discard led to instantiation arity inconsistency.

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation examples
# ════════════════════════════════════════════════════════

# Compiler evaluates literal type function calls at compile time
SIZE: Int = factorial(5)  # compile-time value is 120

# Matrix type uses
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

# Usage: compile-time computation, generates Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never and Void: ⊥ and ⊤ of the Type System

YaoXiang's type system has both ⊥ (false/empty type) and ⊤ (true/Unit) in the Curry-Howard isomorphism, carried by the two builtin type names `Never` and `Void`:

**Never (⊥)** — Three non-negotiable kernel properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. This is a metalevel property and must be builtin.
2. **Explosion principle**: `Never <: T` holds for any type `T`. A `Never` value can be used as any type—this is why code after `assert(false)` still passes type checking (though never executes).
3. **Divergence marker**: `f: (...) -> Never` indicates `f` guarantees not to return. The compiler uses this for dead code analysis.

`Never` is a builtin type name, not a keyword; the parser is unaware of it. Empty sum type literal syntax is not exposed.

**Void (⊤, i.e., Unit)** — Exactly one inhabitant (default void value), the carrier of the always-true proposition. `Void` is the identity of zero-field product types, `Never` is the identity of zero-variant sum types—these two are duals. `x: Void = <default>` is legal, `x: Never = ...` has nothing to write on the right.

#### 4.3 Compile-Time Verification (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: using conditional types
# ════════════════════════════════════════════════════════

# Standard library definitions
# IsTrue: bridge from value universe to type universe — Bool truth mapped to type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has value, program continues
    false => Never,    # ⊥, no value, diverges
}

# Assert: compile-time refinement type primitive — type-level expression of Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error/diverges)
# cond undecidable → proof pipeline decides by dispatch mode:
#                  CompileTime → Unknown, requires prove
#                  Runtime     → insert check, inject Γ assumption

# Usage method 1: as constraint in type definition
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at type position)
    length: Assert(N > 0),
}

# Usage method 2: in expression
IntArray: (N: Int) -> Type = Array(Int, N)
# Verification: size of IntArray(10) equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generics Specialization

```yaoxiang
# Small array optimization: using function overloading to implement compile-time generic specialization

# Generic implementation
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
    # Compiler optimization: unroll the loop
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. Conditional Types

> **Curry-Howard Isomorphism**: From the Curry-Howard perspective, conditional types are **case analysis** in logic. The `Bool` type corresponds to a proposition with two possible values (True/False), and `If` chooses different results based on the truth of that proposition—this is case disjunction in logic. `match C { True => T, False => E }` actually expresses: "Given proposition C is True, conclusion is T; when C is False, conclusion is E".

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

# Compile-time verification (unified to Assert definition in §4.3)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Families

> **Curry-Howard Isomorphism**: Type families are the most direct manifestation of "propositions are types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level", but **constructing a proposition about natural number addition**. `(Zero, B) => B` states "proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` states "if Add(A', B) holds, then Add(Succ(A'), B) also holds". This is the definition of addition itself in Peano axioms. The type checker verifying this match expression passing is equivalent to verifying the logical consistency of this definition.

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

# Type-level addition (Curry-Howard: case analysis + recursive call, needs termination checking for complete induction)
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# Example: compile-time computation of 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. Function Overloading Specialization

#### 6.1 Basic Specialization

```yaoxiang
# Basic specialization: using function overloading (compiler automatically selects)
sum: (arr: Vec(Int)) -> Int = {
    # Compiled to more efficient code
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    # Use SIMD instructions
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation
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
# Specialization method fully conforming to RFC-010 syntax: function overloading

# Concrete type specialization
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation (compiler automatically selects optimal)
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# Usage is completely transparent
int_arr = Vec(Int)(1, 2, 3)
float_arr = Vec(Float)(1.0, 2.0, 3.0)

# Compiler automatically selects optimal specialization
sum(int_arr)     # selects sum: (Vec(Int)) -> Int
sum(float_arr)    # selects sum: (Vec(Float)) -> Float
```

#### 6.3 Perfect Combination of Function Overloading and Inlining

**Key Feature**: Function overloading naturally combines with inlining optimization, achieving zero-cost abstraction.

```yaoxiang
# ======== Source Code ========
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

# ======== After Compilation (Equivalent Code) ========
# Compiler automatically selects optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core advantages**:

1. **Compiler intelligent selection**

   ```yaoxiang
   sum(int_arr)      # automatically selects sum: (Vec(Int)) -> Int
   sum(float_arr)    # automatically selects sum: (Vec(Float)) -> Float
   sum(custom_arr)  # automatically selects sum: (T: Type) -> ((arr: Vec(T)) -> T)
   ```

2. **Inlining Optimization**
   - Small functions automatically inlined at call sites
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function tables

4. **Perfect Fit for RFC-010**

   ```yaoxiang
   # Fully uses unified syntax
   name: type = value
   # No new keywords like impl, where needed
   ```

**Practical Application Examples**:

```yaoxiang
# Performance-sensitive numeric computation
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Use Binet's formula
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# Compiler automatically selects and inlines
fibonacci(10)      # selects Int version, fully inlined
fibonacci(10.5)    # selects Float version, uses Binet's formula
```

**What does this mean?**

- ✅ **Generic specialization** → naturally solved by function overloading
- ✅ **Performance optimization** → inlining automatically done
- ✅ **Code reuse** → one function name, multiple implementations
- ✅ **Zero-cost abstraction** → compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords needed** → perfectly conforms to RFC-010 unified syntax

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internals: build generics instantiation dependency graph
struct InstantiationGraph {
    // Nodes: generic instantiations
    nodes: HashMap<InstanceKey, InstanceNode>,

    // Edges: usage relationships
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // generic function ID
    type_args: Vec<TypeId>,  // type arguments
    const_args: Vec<ConstId>,  // const arguments
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
```

#### 7.2 Usage Point Analysis

```yaoxiang
# Source code analysis
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Usage point 1: instantiate map(Int, Int)
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # needs map[Int, Int]

# Usage point 2: instantiate map(String, String)
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # needs map[String, String]

# Not used: map[Float, Float], etc.
# These generic instances will not be generated

# After compilation, only includes used instances
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
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # two layers: type params + construction params
arr_100_int = Array(Int, 100)()   # empty construction, data assigned later

# After compilation, only generates used sizes
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# Unused sizes are not generated
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
doubled = map(int_list, (x) => x * 2)  # instantiate map(Int, Int)

# Module C
# C.yx
use A.{map}
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # instantiate map(String, String)

# Compile analysis:
# - Module B uses map[Int, Int]
# - Module C uses map[String, String]
# - Binary after compilation only contains these two instances
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

# ✅ Generic approach: automatic derivation
# Using function overloading for automatic derivation
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# Usage
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # automatically generated call
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

# ✅ Generic approach: conditional types
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
result_type = Add[Int, Float]  # inferred as Float
```

> **Relationship with RFC-011b (2026-09-22 Note)**: The type family `Add(A, B)` in this section is the type-level perspective of [RFC-011b](./011b-operator-overloading.md)'s operator interface registration table—the core registration `Add(Int, Float, Float)` and this table's `(Int, Float) => Float` are the same rule; every user's interface instantiation adds a row to this table. The Peano type-level `Add` in §5.2 is pure type-level computation (same name, different entity), not interfering with value-level operator interfaces—operator lookup queries the implementation registration table, not name resolution.

### 9. Examples

#### 9.1 Complete Generics Container Example

```yaoxiang
# ======== 1. Define Generic Container ========
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

    # Generic method (T automatically brought into scope by outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement Generic Methods ========
# Function definitions in List namespace (List. prefix = namespace ownership)
# To make list.push(item) . call syntax work, explicit binding needed: List.push = push[0]
# self is just a conventional parameter name, compiler looks at type, not name

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # expand capacity
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

# ======== 3. Using Type Constraints ========
# Implement Clone for List
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. Usage Examples ========
# Create generic List
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
# ======== 1. Generic Sorting Algorithm ========
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

# ======== 2. IntComparator Implementation ========
# Using function overloading to implement
compare: (a: Int, b: Int) -> Int = {
    if a < b {
        return -1
    } else if a > b {
        return 1
    } else {
        return 0
    }
}

# ======== 3. Usage Examples ========
# Sort Int array
numbers = Vec(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# Sort String array (needs StringComparator)
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-Time Generics Example

```yaoxiang
# ======== 1. Compile-Time Matrix Type ========
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

# ======== 2. Compile-Time Matrix Creation ========
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

# ======== 3. Usage Examples ========
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

## Tradeoffs

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
   - Dimension verification and other features
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compilation time**
   - Generic instantiation increases compilation time
   - Constraint solving may be slow

2. **Memory usage**
   - Increased compiler memory usage
   - Caching mechanism requires memory

3. **Implementation complexity**
   - Constraint solver is complex
   - Type-level computation engine is complex

4. **Error diagnostics**
   - Generic errors may be complex
   - Need clear error hints

### Mitigations

1. **Caching strategy**
   - Cache instantiation results
   - LRU cache for memory limits

2. **Incremental compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error hints**
   - Clear error messages
   - Generics parameter inference hints

4. **Parallel compilation**
   - Parallel instantiation of generics
   - Multi-threaded constraint solving

## Alternative Solutions

| Solution        | Why Not Chosen                     |
| --------------- | ---------------------------------- |
| Basic generics only | Cannot replace complex macros     |
| Pure macro system | No type safety, poor error messages |
| Constraints only | Insufficient flexibility           |
| Runtime generics | Has performance overhead           |

### Risks

| Risk                | Impact           | Mitigation           |
| ------------------- | ---------------- | -------------------- |
| Constraint solving complexity | Compilation too long | Incremental solving + caching |
| Code bloat       | Binary too large | DCE + threshold control |
| Implementation complexity | Development cycle extended | Phased implementation |
| Error diagnostics | Poor user experience | Detailed error messages |

## Open Questions

### Pending Issues

| Topic           | Description                         | Status    |
| --------------- | ----------------------------------- | --------- |
| Instantiation strategy | Eager vs Lazy vs Threshold | Pending discussion |
| Cache size      | LRU cache capacity setting          | Pending discussion |
| Error diagnostics | Generic error message verbosity    | Pending discussion |

### Future Optimizations

| Optimization       | Value | Implementation Difficulty |
| ------------------ | ----- | ------------------------- |
| Instantiation graph analysis | High | Medium |
| Type-level programming DSL | Medium | High |
| Generic performance benchmarks | Medium | Low |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use unified () syntax, as part of function type
# e.g., map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# Type constraints (in generic parameters)
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# Parameter declaration (type + name)
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# Function declaration: name: type = expression
# Generic parameters are the first parameter group in function type: (T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# Method declaration: Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# Type definition (unified Binding syntax)
# Generic types like List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# Type in generic parameters is automatically filled by compiler from actual argument types
# e.g., map(numbers, f), T extracted from numbers: List(Int), R from f: (Int) -> String
```

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review │  ← Open for community discussion and feedback
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   Accepted  │    │   Rejected  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │     rfc/    │
│ (official design) | (kept in place) │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documents

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./009-ownership-model.md)
- [RFC-001: Concurrency Model](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./008-runtime-concurrency-model.md)
- [tutorial/ Tutorial](../../../tutorial/index.md)

### External References

- [Rust Generics System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)