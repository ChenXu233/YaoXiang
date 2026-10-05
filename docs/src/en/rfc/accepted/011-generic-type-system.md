---
title: 'RFC-011: Generic Type System Design - Zero-Cost Abstraction and Macro Replacement'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-14'
updated:
  '2026-07-15 (type body code blocks + compile-time specifications + effect seeds implemented)'
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

## Summary

This document defines the **Generic Type System Design** of the YaoXiang language. It achieves
zero-cost abstraction through powerful generic capabilities, reduces the dependency on macros via
compile-time optimization, and provides a dead code elimination mechanism.

**Core Design**:

- **Unified Signature Syntax**: `(T: Type, R: Type) -> ...` - generic parameters are unified with
  regular parameters
- **Type Self-Describing Mechanism**: `Type` is a language-level special entity; the `Type` position
  in signatures is automatically inferred and filled
- **Type Constraints**: `T: Dup + Add` for multiple constraints, function type constraints
- **Associated Types**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-Time Generics**: `N: Int` for generic value parameters, compile-time constant
  instantiation
- **Conditional Types**: `If: (C: Bool, T: Type, E: Type) -> Type` for type-level computation, type
  families

**Value**:

- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro use cases
- Type safety: compile-time checking, IDE-friendly
- **Explicitness over implicitness**: `Type` is self-describing, the compiler auto-infers

## Reference Documents

This document's design is based on the following documents:

| Document                                                                                                 | Relationship              | Description                                                                      |
| -------------------------------------------------------------------------------------------------------- | ------------------------- | -------------------------------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)                                               | **Syntax Foundation**     | Generic syntax integrates with the unified `name: type = value` model            |
| [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)                                               | **Call Syntax**           | Section 6: Generic call syntax—unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](009-ownership-model.md)                                                       | **Type System**           | Natural combination of Move semantics and generics                               |
| [RFC-024: Spawn-Based Concurrency Runtime Semantics](024-concurrency-model.md)                           | **Execution Model**       | DAG analysis and generic type checking                                           |
| [RFC-008: Runtime Model](008-runtime-concurrency-model.md)                                               | **Compiler Architecture** | Generic monomorphization and compile-time optimization strategy                  |
| Type Universe philosophy (see same-named section below)                                                  | **Theoretical Core**      | Type universe hierarchy model and value-dependent type design                    |
| [RFC-027: Compile-Time Predicates and Unified Static Verification](027-compile-time-evaluation-types.md) | **Termination Check**     | Automatic measure synthesis and compile-time evaluation safety guarantees        |

## Type Universe Philosophy and Value-Dependent Types

YaoXiang's generic system is built upon the **Type Universe philosophy**. This mental model unifies
all concepts in the language into a hierarchical structure, and the core innovation is elevating
**value-dependent types** to first-class citizens at the Type2 level.

### What are Value-Dependent Types?

A **value-dependent type** is a type that depends on one or more **values** (not just on other
types). These values can be evaluated at compile time, thereby providing type-safety guarantees at
the compile phase.

```yaoxiang
# Traditional generics: type parameter
List: (T: Type) -> Type

# Value-dependent type: value parameter
Array: (T: Type, N: Int) -> Type  # Array type depends on the length value N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column counts
```

### Container Type Naming Hierarchy

The language layer has three container concepts; where the length information lives is their
fundamental distinction:

| Type          | Length        | Semantics                                | Underlying                               |
| ------------- | ------------- | ---------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | **Fixed-length** array, N is in the type | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | **Runtime-length raw buffer**, growable  | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type                    | Library: `{ data: Vec(T), length: Int }` |

Division of labor:

- **`Array(T, N)` is the only form that puts length into the type**—since the length is a
  compile-time constant, it enables compile-time rejection of boundary failures (e.g., `a[5]` when
  `a: Array(Int, 3)` is a direct compile-time error; see "Compile-Time Dimension Verification"
  below).
- **`Vec(T)` is the minimal foundation for runtime-length**—it provides only four things: "can
  allocate, can get length, can read/write, can grow". It does not handle capacity strategy, growth
  factor, or shrinking. It is the raw material for building other containers.
- **`List(T)` is a library type, not a primitive**—defined in YaoXiang itself within `std.list`
  (`{ data: Vec(T), length: Int }`), treated the same as user-defined generic records. All growable
  semantics strategies (when to grow, by how much, whether sharing is allowed) live in the library;
  the compiler is not involved.

`Vec(T)` construction forms (two layers: first type parameters, then construction parameters):

```yaoxiang
# Empty construction—length 0, elements appended afterwards
v = Vec(Int)()

# Element construction—length determined by the number of elements
w = Vec(Int)(1, 2, 3)          # length 3

# Slot allocation—allocate n zero-value slots
buf = Vec(Int)(len=64)         # length 64, all elements are zero values
```

> Slot allocation uses the **field-name style** (`len=`) instead of positional: the positional
> single integer would be ambiguous with "single-element vector" (`Vec(Int)(64)` cannot distinguish
> between "length 64" and "containing one element 64"). This is consistent with the unified rule for
> generic construction: field-name arguments bind by name and are not affected by positional
> inference.
>
> This is the only primitive `List` needs for growth—when `List` needs to grow, it allocates new
> slots and moves elements:
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> When to grow, by how much, and whether to shrink are all decided by `List`. `Vec` does not handle
> capacity strategy.

From bottom to top, performance decreases and flexibility increases: `Array` > `Vec` > `List`.

> Naming rationale: `Vec`/`vector` in mainstream languages (Rust/C++) refers to a runtime-length
> growable sequence; `Array` is fixed-length.

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core
advantages:

| Feature                      | Traditional Generics (C++/Rust)               | YaoXiang Value-Dependent Types                               |
| ---------------------------- | --------------------------------------------- | ------------------------------------------------------------ |
| Values on which types depend | Only type parameters                          | Any value, including function call results                   |
| Compile-time evaluation      | C++ template manual specialization, Rust none | Automatic compile-time evaluation with termination guarantee |
| Type-level computation       | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                        |
| Type safety                  | C++ none, Rust limited                        | Full type safety, compile-time checking                      |
| Dimension verification       | Runtime check or manual specialization        | Compile-time dimension verification, no runtime overhead     |

### Type Universe Hierarchy and Value-Dependent Types

The Type Universe philosophy divides language concepts into different levels by semantic role, with
value-dependent types at the **Type2 level**:

| Level     | Role                                                      | Example                                                                                                         |
| --------- | --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | Values                                                    | `42`, `factorial(5)`, the function itself                                                                       |
| Type0     | Meta-type keyword                                         | `Type`                                                                                                          |
| Type1     | Concrete types                                            | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **Functions / type constructors / value-dependent types** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at the Type2 level share
**unified syntax**, all in the form of `(params) -> result`:

- Regular function: `(Int, Int) -> Int` → the return value is a value
- Type constructor: `(T: Type) -> Type` → the return value is a type
- Value-dependent type: `(T: Type, N: Int) -> Type` → the return value is a type and depends on the
  value parameter N

> **Curry-Howard Isomorphism**: This unification is not a coincidence. The Curry-Howard isomorphism
> states that "types are propositions, programs are proofs"—the function type `A → B` corresponds to
> the logical implication "if A then B", the generic `(T: Type) -> Type` corresponds to universal
> quantification "for all types T", and the value-dependent type `(n: Int) -> Type` corresponds to
> "for every integer n there exists a type". YaoXiang unifies functions, type constructors, and
> value-dependent types at the Type2 level, which essentially unifies "proof" and "computation" into
> the same concept—**constructive proof**. This is the direct embodiment of the Curry-Howard
> isomorphism in language design: one form (`(params) -> result`) simultaneously carries logical
> propositions and computational processes.

### Compile-Time Determinism Guarantees

YaoXiang's Type Universe philosophy requires: **everything at the Type level is determined at
compile time**.

```yaoxiang
# Compile-time dimension verification example
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # Compile-time check: dimensions must be positive
    _assert: Assert(Rows > 0),
    _assert: Assert(Cols > 0),
}

# Create a 3x3 identity matrix - completed at compile time
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# Compile-time computation: factorial(3) = 6, array size determined at compile time
arr: Array(Int, factorial(3)) = Array(Int, 6)()
```

The compiler automatically:

1. Detects function calls at type positions
2. Performs compile-time termination check on the function (see termination check mechanism below)
3. Executes the evaluation at compile time
4. Embeds the result into the generated type

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

#### Type-Safe Array Sizes

```yaoxiang
# Array size is a compile-time constant
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N is a compile-time constant, can be used in type-level computation
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3 (known at compile time)
```

#### Compile-Time Coverage Goals for Boundary Failures

> **Implementation status note**: Container types have been despecialized—`Array(T, N)` is a const
> generic constructor, the literal context landing point and `in` membership predicate are both
> landed. The N and element type at the literal landing point of `Array(T, N)` have been enforced by
> compile-time validation (E1002); **N is trusted**—the mechanism described in this section can be
> built on top of "annotating N == runtime length". The current `[]` index out-of-bounds (E6003) and
> Dict missing key (E6008) are **transitional runtime error states**; the value-dependent types in
> this section aim to push these boundary failures to the **compile-time** mechanism:
>
> - const indexing: `a[5]` (5 is a compile-time constant) when `a: Array(Int, 3)` is directly
>   rejected at compile time;
> - value indexing: `a[i]` requires the precondition `i < len(a)`, proven by value-dependent type
>   contract;
> - the `in` predicate is the base of Hoare-logic preconditions: `n in 1..10`, `x in some_set` are
>   both compile-time-provable propositions.
>
> The full design of refinement types will supplement this section when it is landed.

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

#### Generic Functions

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

# Completely transparent at use site, types auto-inferred
numbers = List(Int)()   # Value construction two-layer form (see §9.1); elements are filled with push
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
| Compile-time dimension verification                         | Manual specialization  | Runtime check | Type families  | Automatic compile-time verification                                  |
| Semi-automatic termination annotation (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (no annotation syntax; explicit measure written at type position) |

### Termination Check Mechanism (Unified with RFC-027)

Compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the type
system would fall into infinite loops. Termination checking is handled **fully automatically first**
by RFC-027's compile-time proof pipeline—the compiler first automatically explores measures, and
recursive/loop calls that can be proven pass; if exploration fails and no explicit measure is given,
a compile error is reported (RFC-027 §6.9 provides explicit measure fallback at the type position).
**No annotation syntax is provided**: RFC-022's `//! decreases`, `/*! invariant !*/` have been
deprecated along with RFC-022, the specification IS the type annotation itself.

> **Trigger criterion (RFC-027 §7)**: The termination obligation is triggered by **refined
> types**—once a type is refined, it enters verification mode. Non-refined ordinary types do not
> enter verification mode and generate no termination obligation.

#### Termination Check for Recursive Functions

For recursive functions with refined signatures, the compiler checks whether the arguments of
recursive calls strictly decrease on every recursive path (RFC-027 §6.7). No specification comments
are required:

```yaoxiang
# Recursive with refined signature: no //! requires/ensures/decreases, compiler auto-explores decrease
factorial: (n: NonNegative(n)) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler explores: n-1 < n → decrease → termination
}
```

| Scenario                                                             | Behavior                            |
| -------------------------------------------------------------------- | ----------------------------------- |
| Compiler can explore the decrease (e.g., `n-1`)                      | Pass                                |
| Cannot explore but a measure is given at the type position (§6.9)    | SMT decides; pass if it holds       |
| Cannot explore and no explicit measure / measure is falsified by SMT | Compile error                       |
| Signature not refined (not in verification mode)                     | No termination obligation generated |

#### Termination Check for Loops

Loops do not require `: Invariant(...)` or `: decreases(...)` annotations. Refined type annotations
on variables (such as `UpTo(n)`) provide both loop invariants and measure boundaries. The compiler
tries four measure exploration strategies in priority order and stops as soon as one succeeds
(RFC-027 §6.1–6.5):

1. **Automatic linear rank function synthesis**—extracts variable bounds from type annotations,
   enumerates linear combinations, SMT verifies m ≥ 0 and m' < m on all paths
2. **Predicate violation counting** (experimental)—extracts violation_count from target type
   definitions (e.g., `Sorted`), covering adjacent swap/move
3. **Bounded increasing/decreasing pattern**—`v += const` → measure `upper - v` (degenerate form of
   strategy 1, fastest path)
4. **Multiplicative scaling measure template**—`v *= const` (const > 1) → measure
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # Type annotation gives upper bound arr.len and lower bound 0 → enter verification mode
    while i < arr.len {
        # Compiler auto-explores: measure arr.len - i, strictly decreases by 1 each iteration → termination proven
        s += arr[i]; i += 1
    }
    return s
}
```

When exploration fails, a name can be bound to the loop and a measure can be given at the type
position (RFC-027 §6.9):

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    // The loop body is a `{}` block, the value is given by the tail expression (spec §2.9): after writing the assignment statement at the tail of the body, `i` must also be written,
    // otherwise the block value is `Void`, and `return acc` returning `Void` from `-> Int` does not hold.
    acc: Terminates(n - i) = while i < n { i = i + 1; i }
    return acc
}
```

#### Workflow of Termination Check

```
┌─────────────────────────────────────────────────────────────┐
│  Type checking phase                                        │
│  Encounter a position with a refined type (parameter       │
│  refinement, return refinement, variable refinement)        │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination check (RFC-027 proof pipeline, full         │
│     automation first)                                       │
│     - Recursive functions: check that arguments strictly    │
│       decrease on every recursive path                      │
│     - Loops: four measure exploration strategies           │
│       (linear rank / violation count / bounded pattern /    │
│       multiplicative scaling), SMT verifies decrease        │
│     - Cannot explore → programmer can give a measure at    │
│       the type position (Terminates)                        │
│     - No measure / measure falsified by SMT → compile error │
│       (hard boundary)                                       │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time evaluation (executed by built-in           │
│     interpreter)                                            │
│     - Pure function: evaluate directly                      │
│     - Side effect: compile error (type position must be     │
│       side-effect free)                                     │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result embedded in type                                 │
│     - Array(Int, factorial(5)) → Array(Int, 120)           │
│     - Matrix(Float, 3, 3) → concrete type                   │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensures that compile-time evaluation must terminate, preventing the type system from
  falling into infinite loops
- **Uniformity**: Termination check and correctness verification (VC generation) share the same
  compile-time proof pipeline (RFC-027), with no separate specification syntax
- **Full automation first**: The compiler automatically explores measures from type annotations; if
  it can prove, it passes; if it cannot, a measure can be given at the type position (`Terminates`),
  still decided by SMT—does not depend on programmers writing `decreases` syntax manually

## Motivation

### Why Do We Need a Strong Generic System?

Existing mainstream languages have limitations in their generics:

| Language     | Generic Capability        | Problem                                                                                    |
| ------------ | ------------------------- | ------------------------------------------------------------------------------------------ |
| Java         | Bounded types             | Compile-time monomorphization, no generic specialization                                   |
| C#           | Generic constraints       | Runtime type checking, performance overhead                                                |
| Rust         | Generics + Trait          | Trait system is complex, steep learning curve                                              |
| C++          | Templates                 | Template specialization is complex, compile error messages are poor                        |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, compile-time dimension verification, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: Runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: Powerful type system vs ease of use
3. **Macros vs Generics**: Macro code generation vs generic type safety
4. **Value-dependence vs Type safety**: Traditional generics cannot verify dimensions at compile
   time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                             |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **Types depend on values**  | `Array: (T: Type, N: Int) -> Type` lets types depend on concrete values                                                 |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, the result is directly embedded in the type             |
| **Dimension verification**  | `Matrix(Float, 3, 3)` verifies matrix dimensions at compile time                                                        |
| **Type-level computation**  | `If`, `Match` and other conditional types support type-level computation                                                |
| **Termination guarantee**   | Compile-time termination check (automatic measure synthesis per RFC-027) ensures compile-time evaluation must terminate |

```yaoxiang
# Compile-time verification that C++/Rust cannot do
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# Compile-time computation: factorial(3) = 6, factorial(2) = 2
# Type is Matrix(Float, 6, 2)

# Dimension mismatch caught at compile time
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # Compile error: 2 != 3
```

### Value of the Generic System

```yaoxiang
# Example: Unified API design
# map operation for different container types

# Traditional approach: separate implementation for each type
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

1. **Zero-cost abstraction** - Generic calls equivalent to concrete type calls
2. **Dead code elimination** - Compile-time analysis, only instantiate used generics
3. **Macro replacement** - Generics replace 90% of macro use cases
4. **Type safety** - Compile-time checking, no runtime type overhead
5. **IDE-friendly** - Smart hints, clear error messages
6. **Value-dependent types** - Types can depend on values, supporting compile-time dimension
   verification
7. **Compile-time evaluation safety** - Guarantee termination of compile-time evaluation through
   compile-time termination checks (RFC-027 automatic measure synthesis)

### Design Principles

- **Compile-time determinism**: Generic parameters are determined at compile time
- **Monomorphization first**: Generate concrete code, avoid virtual function calls
- **Constraint-driven**: Type constraints guide instantiation
- **Platform optimization**: Specialization supports platform-specific optimization
- **Type universe unification**: Functions / type constructors / value-dependent types are unified
  at the Type2 level
- **Termination guarantee**: Function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise HM will
> infer them as functions.
>
> | Writing                           | Meaning                                |
> | --------------------------------- | -------------------------------------- |
> | `List: (T: Type) -> Type = {...}` | ✅ Type constructor                    |
> | `List = {...}`                    | ❌ HM infers as a function, not a type |

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
    push: (self: List(T), item: T) -> Void,   # self is just a convention name, not a keyword
    get: (self: List(T), index: Int) -> Option(T),
}

# Generic function (no : Type, HM infers as function)
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# Generic constraint (direct expression, single-line can omit return)
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

#### 1.2 Type Self-Describing Mechanism

`Type` is a language-level special entity. The compiler naturally recognizes the `Type` position in
signatures and automatically infers and fills it from the actual argument types.

```yaoxiang
# Compiler auto-infers generic parameters
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

# Use sites
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
# Omit Type parameter when inference is possible
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# Must explicitly fill when inference is not possible
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
# Multiple constraint syntax
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

> **Source of constraint names (note 2026-09-22)**: Operator constraints such as `Add` / `Subtract`
> / `Multiply` / `Divide` / `Modulo` are defined and landed by
> [RFC-011b: Operator Overloading and Interface-Driven Operators](011b-operator-overloading.md)—`T: Add`
> ≜ a registered `Add(T, T, T)` interface instantiation (three type parameters, with result type `O`
> explicit). `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` currently have **no defined source** and
> are dangling constraint names, to be landed in subsequent RFCs; before that, examples involving
> these names are paper sketches.

#### 2.3 Function Type Constraints

```yaoxiang
# Higher-order function constraints
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

call_with_arg: (T: Type, U: Type, F: Fn(T) -> U)(arg: T, f: F) -> U = f(arg)

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))

# Usage examples
result: Int = call_with_arg(42, (x: Int) => x * 2)  # result = 84
composed: String = compose(
    "hello",
    (s: String) => s.to_uppercase(),
    (s: String) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

#### 2.4 Built-in Marker Trait: Dup and Clone

**Three types of copy semantics**:

| Type                     | Meaning                                                         | Trigger                              | Use Case                          |
| ------------------------ | --------------------------------------------------------------- | ------------------------------------ | --------------------------------- |
| **Primitive value copy** | Auto value copy on assignment, two values are fully independent | Auto on assignment/parameter passing | Int, Float, Bool, Char            |
| **Dup**                  | Shallow copy: copy handle/token, underlying data is shared      | Auto on assignment/parameter passing | `&T` token, `ref T`, String/Bytes |
| **Clone**                | Deep copy: create a complete independent replica                | `value.clone()`                      | Any type that implements Clone    |

**Semantics of Dup**: Types that implement Dup do not transfer ownership on assignment/parameter
passing—the compiler copies the handle/token, and multiple holders point to the same underlying
data. This complements the default Move semantics in the RFC-009 ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = copy handle, share data (modifications affect each other)
Clone = copy data, replica independent (modifications do not affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not Dup
2. Dup — only applies to reference/token types and internally reference-counted types
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types keep default Move semantics
```

**Which types are Dup**:

| Type                     | Dup     | Reason                                                                                         |
| ------------------------ | ------- | ---------------------------------------------------------------------------------------------- |
| `&T` (borrow token)      | ✅      | Zero-sized token, copying the token = multiple views point to the same data                    |
| `ref T`                  | ✅      | Rc/Arc copy = reference count +1, shared heap data                                             |
| String, Bytes            | ✅      | Internal reference counting, copying handle shares underlying buffer                           |
| `&mut T` (mutable token) | ❌      | Linear exclusive, cannot copy                                                                  |
| struct                   | Derived | All fields ∈ (primitive value type ∪ Dup) → Dup, otherwise Move (RFC-009 §derived rules, #398) |
| enum                     | Derived | All fields of all variants are copyable → enum Dup (design state, not yet landed)              |
| tuple                    | Derived | Per-element decision, same as struct rules (#398)                                              |
| Fn (closure)             | ❌      | Captured environment may be non-Dup                                                            |
| `*T` (raw pointer)       | ❌      | unsafe, not part of ownership system                                                           |

**Int/Float/Bool/Char are not Dup**—they are value types, and the compiler auto-copies values on
assignment (the two values are fully independent). This is not "shallow copy", but the compiler's
built-in handling of primitives, which does not need and should not be expressed through the Dup
type attribute.

```yaoxiang
# Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x          # Value copy, x and y are fully independent
print(x)       # ✅

# Dup: shallow copy, copy handle shares data
view: &Point = &point
view2 = view    # ✅ Dup: copy token, both point to the same point
print(view.x)   # ✅

# Clone: explicit deep copy, create independent replica
backup = big_struct.clone()  # Explicit call

# Generic constraints
dup_use: (T: Dup) -> T = x         # T: Dup → can be shallow-copied
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can be deep-copied
```

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety is fully handled
> automatically by the `ref` keyword and the compiler—`ref` auto-selects Rc or Arc, and users do not
> need to understand Send/Sync.

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
# Use method syntax sugar: Vec.Item, Vec.next, Vec.has_next
# Iteration position is carried by the wrapper record (Vec itself is a raw buffer with no index field)
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

# Associated type can be generic
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # Associated type is also generic
    iter: (Self) -> IteratorType,
}

# Usage
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. Compile-Time Generics

#### 4.1 Compile-Time Value Parameters

**Core design**: `Type` in a generic signature marks a type parameter; parameters with concrete
types annotated (e.g., `Int`/`Bool`/`Float`) are **candidates for compile-time value
parameters**—whether they become compile-time value parameters depends on whether their values are
**referenced at type positions** (value-dependent). No `const` keyword is needed.

> The criterion is **referenced at type positions**, not "annotated with a concrete type": in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters because they do not
> appear at any type position.

**Decision rule (two steps)**:

1. **Shape filter**: A parameter annotated with a concrete type other than `Type` (e.g., `Int`) →
   listed as a candidate.
2. **Usage refinement**: The candidate name appears at a **type position** (type body field type,
   inner `Fn` parameter type, `Assert` predicate, type-construction argument position such as
   `Array(T, N)`) → confirmed as a compile-time value parameter; otherwise treated as a **runtime
   value parameter**.

| Writing                                                    | Decision                            | Reason                                                                      |
| ---------------------------------------------------------- | ----------------------------------- | --------------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | Only appear at value positions, do not participate in type construction     |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N appears at the type-construction argument position of `Array(T, N)`       |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N acts as the type of the inner parameter `k`                               |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through (see below)         | N is not referenced in the type body, degrades to a runtime value parameter |

> **Essence of value-dependence**: A compile-time value parameter is a value-dependent type—it only
> needs to be determined at compile time when the value is used to **construct a type**. Shape
> (`: Int`) only determines candidacy; usage (appearing at a type position) determines whether it is
> a compile-time value parameter. This is the same root criterion as "function calls at type
> positions are evaluated at compile time" in §"Compile-Time Determinism Guarantees".

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameter: N is referenced at the type position (Measure length slot)
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears at the type-construction argument position → compile-time value parameter
    length: N,
}

# Usage: factorial(5) is evaluated at the type position (compile time), result 120 is embedded in the type
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# Value-dependence: N as the type of the inner parameter k
# ════════════════════════════════════════════════════════
# N is a compile-time value parameter (appears at the type position of (k: N));
# k is a runtime value parameter, its type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling of falling-through candidates**: Candidates that are annotated with concrete types but
> not referenced at type positions (e.g., `N` of `Foo` in the table above) degrade to runtime value
> parameters (function-level path). A falling-through candidate on the type-constructor path cannot
> occupy a runtime slot (type constructors are evaluated at compile time), and the declaration side
> directly reports error [E1094]: "N is declared as a compile-time value parameter but not
> referenced in the type body"—previously silent dropping led to instantiation arity inconsistency.

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation examples
# ════════════════════════════════════════════════════════

# The compiler evaluates the function call of the literal type at compile time
SIZE: Int = factorial(5)  # 120 at compile time

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

# Usage: compile-time computation, generates Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never and Void: ⊥ and ⊤ in the Type System

YaoXiang's type system, in the Curry-Howard isomorphism, simultaneously has ⊥ (false/empty type) and
⊤ (true/Unit), carried by the two built-in type names `Never` and `Void`:

**Never (⊥)** — three non-negotiable kernel properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. This is a
   meta-level property and must be built in.
2. **Principle of explosion**: `Never <: T` holds for any type `T`. A `Never` value can be used as
   any type—this is why code after `assert(false)` still passes the type check (although it never
   executes to that point).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis.

`Never` is a built-in type name, not a keyword, and the parser is unaware. No sum or type literal
syntax is opened.

**Void (⊤, i.e., Unit)** — has exactly one inhabitant (the default void value), and is the carrier
of the true proposition "always true". `Void` is the identity element of the zero-field product
type, `Never` is the identity element of the zero-variant sum type—the two are dual.
`x: Void = <default>` is valid, `x: Never = ...` has no right-hand side to write.

#### 4.3 Compile-Time Verification (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: leveraging conditional types
# ════════════════════════════════════════════════════════

# Standard library definition
# IsTrue: the bridge from the value universe to the type universe—Bool truth value maps to a type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has a value, program continues
    false => Never,    # ⊥, no value, diverge
}

# Assert: compile-time refinement-type primitive—the type-level formulation of a Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error / diverge)
# cond undecidable → proof pipeline decides by dispatch mode:
#                    CompileTime → Unknown, require prove
#                    Runtime     → insert check, inject Γ assumption

# Usage 1: as a constraint in type definitions
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at the type position)
    length: Assert(N > 0),
}

# Usage 2: used in expressions
IntArray: (N: Int) -> Type = Array(Int, N)
# Verify: the size of IntArray(10) equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generic Specialization

```yaoxiang
# Small array optimization: use function overloading to implement compile-time generic specialization

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
    # Compiler optimization: unroll the loop
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. Conditional Types

> **Curry-Howard Isomorphism**: From the Curry-Howard perspective, conditional types are **case
> analysis** in logic. The `Bool` type corresponds to a proposition with two possible values
> (True/False), and `If` chooses different results based on the truth or falsity of that
> proposition—this is exactly case disjunction in logic. `match C { True => T, False => E }` is
> actually expressing: "when the known proposition C is True, the conclusion is T; when C is False,
> the conclusion is E".

#### 5.1 If Conditional Types

```yaoxiang
# Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# Example: compile-time branch
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# Compile-time verification (unified with the Assert definition in §4.3)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Families

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but **constructing a proposition about natural-number addition**. `(Zero, B) => B` says "the
> proposition Add(Zero, B) is equivalent to B", and `(Succ(A'), B) => Succ(Add(A', B))` says "if
> Add(A', B) holds, then Add(Succ(A'), B) also holds". This is the addition definition in the Peano
> axioms itself. The type checker verifying that this match expression passes is equivalent to
> verifying the logical consistency of this definition.

```yaoxiang
# Compile-time type conversion
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,  # default
}

# Type-level computation
Length: (T: Type) -> Type = match T.length {
    0 => Zero,
    1 => Succ(Zero),
    2 => Succ(Succ(Zero)),
    _ => TooLong,
}

# Type-level addition (Curry-Howard: case analysis + recursive call, needs termination check to be full induction)
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# Example: compile-time computation 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. Function Overload Specialization

#### 6.1 Basic Specialization

```yaoxiang
# Basic specialization: use function overloading (compiler auto-selects)
sum: (arr: Vec(Int)) -> Int = {
    # Compiles to more efficient code
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
# A specialization method fully conforming to the RFC-010 syntax: function overloading

# Concrete type specialization
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation (compiler auto-selects the optimal one)
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

# Compiler auto-selects the optimal specialization
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
# The compiler auto-selects the optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core Advantages**:

1. **Compiler intelligent selection**

   ```yaoxiang
   sum(int_arr)      # Auto-selects sum: (Vec(Int)) -> Int
   sum(float_arr)    # Auto-selects sum: (Vec(Float)) -> Float
   sum(custom_arr)  # Auto-selects sum: (T: Type) -> ((arr: Vec(T)) -> T)
   ```

2. **Inlining optimization**
   - Small functions are auto-inlined to the call site
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function table required

4. **Perfectly fits RFC-010**

   ```yaoxiang
   # Fully using unified syntax
   name: type = value
   # No new keywords like impl, where needed
   ```

**Practical Application Examples**:

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
fibonacci(10)      # Selects Int version, fully inlined
fibonacci(10.5)    # Selects Float version, uses Binet's formula
```

**What does this mean?**

- ✅ **Generic specialization** → Function overloading solves it naturally
- ✅ **Performance optimization** → Inlining done automatically
- ✅ **Code reuse** → One function name, multiple implementations
- ✅ **Zero-cost abstraction** → Compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords needed** → Perfectly fits RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internals: build the generic instantiation dependency graph
struct InstantiationGraph {
    // Nodes: generic instantiations
    nodes: HashMap<InstanceKey, InstanceNode>,

    // Edges: usage relationships
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // Generic function ID
    type_args: Vec<TypeId>,  // Type arguments
    const_args: Vec<ConstId>,  // Const arguments
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

#### 7.2 Use-Site Analysis

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

# Unused: map[Float, Float] etc.
# These generic instances are not generated

# After compilation only the used instances are included
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
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # Two layers: type parameters + construction parameters
arr_100_int = Array(Int, 100)()   # Empty construction, data assigned afterwards

# After compilation only the used Sizes are generated
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# Unused Sizes are not generated
# Array(Int, 50) is not generated
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

# Compilation analysis:
# - Module B uses map[Int, Int]
# - Module C uses map[String, String]
# - The compiled binary only contains these two instances
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

# ✅ Generic approach: auto-derive
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

# ✅ Generic approach: conditional type
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

> **Relationship with RFC-011b (note 2026-09-22)**: The lifted type family `Add(A, B)` in this
> section is the perspective of the [RFC-011b](011b-operator-overloading.md) operator interface
> registration table at the type level—the core entry `Add(Int, Float, Float)` and the
> `(Int, Float) => Float` in this table are the same rule, and each user interface instantiation is
> adding a row to this table. The Peano type-level `Add` in §5.2 is pure type-level computation
> (same name, different thing), and does not interfere with the value-level operator
> interface—operator query goes through the implementation registration table, not name resolution.

### 9. Examples

#### 9.1 Complete Generic Container Example

```yaoxiang
# ======== 1. Define generic container ========
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
# Functions are defined under the List namespace (List. prefix = namespace ownership)
# To make the . call syntax of list.push(item) work, explicit binding is needed: List.push = push[0]
# self is just a conventional parameter name, the compiler looks at the type, not the name

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # Grow capacity
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
# Create a generic List
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# Use generic methods
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# Use fold for computation
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
# Implement using function overloading
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

# Sort a String array (needs StringComparator)
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-Time Generic Example

```yaoxiang
# ======== 1. Compile-time matrix type ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # Compile-time dimension verification: leveraging the Assert standard library type
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
# Create a matrix with a compile-time-known size
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
   - Code bloat is controllable

3. **Macro replacement**
   - Type-safe code generation
   - IDE-friendly, clear error messages

4. **Compile-time computation**
   - Compile-time generics support compile-time computation
   - Dimension verification and other features
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compile time**
   - Generic instantiation increases compile time
   - Constraint solving may be slow

2. **Memory usage**
   - Compiler memory usage increases
   - Caching mechanisms require memory

3. **Implementation complexity**
   - Constraint solver is complex
   - Type-level computation engine is complex

4. **Error diagnosis**
   - Generic errors can be complex
   - Clear error messages are required

### Mitigation Measures

1. **Caching strategy**
   - Cache instantiation results
   - LRU cache limits memory

2. **Incremental compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error messages**
   - Clear error messages
   - Generic parameter inference hints

4. **Parallel compilation**
   - Parallel generic instantiation
   - Multi-threaded constraint solving

## Alternatives

| Approach            | Why Not Chosen                      |
| ------------------- | ----------------------------------- |
| Basic generics only | Cannot replace complex macros       |
| Pure macro system   | No type safety, poor error messages |
| Constraint-only     | Insufficient flexibility            |
| Runtime generics    | Has performance overhead            |

### Risks

| Risk                         | Impact                     | Mitigation                    |
| ---------------------------- | -------------------------- | ----------------------------- |
| Constraint solver complexity | Long compile time          | Incremental solving + caching |
| Code bloat                   | Oversized binary           | DCE + threshold control       |
| Implementation complexity    | Extended development cycle | Phased implementation         |
| Error diagnosis              | Poor user experience       | Detailed error messages       |

## Open Questions

### Pending Decisions

| Topic                  | Description                            | Status     |
| ---------------------- | -------------------------------------- | ---------- |
| Instantiation strategy | Eager vs Lazy vs Threshold             | To discuss |
| Cache size             | LRU cache capacity setting             | To discuss |
| Error diagnosis        | Detail level of generic error messages | To discuss |

### Future Optimizations

| Optimization                  | Value  | Implementation Difficulty |
| ----------------------------- | ------ | ------------------------- |
| Instantiation graph analysis  | High   | Medium                    |
| Type-level programming DSL    | Medium | High                      |
| Generic performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use the unified () syntax as part of the function type
# e.g. map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# Type constraint (in generic parameters)
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
# Generic type such as List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# The Type in generic parameters is automatically filled by the compiler from the actual argument types
# e.g. map(numbers, f), T is extracted from numbers: List(Int), R is extracted from f: (Int) -> String
```

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  In Review  │  ← Open community discussion and feedback
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
│ (Formal design) │  │ (Stays in place) │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documentation

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-009: Ownership Model](009-ownership-model.md)
- [RFC-024: Spawn-Based Concurrency Runtime Semantics](024-concurrency-model.md)
- [RFC-008: Runtime Model](008-runtime-concurrency-model.md)
- [tutorial/](../../tutorial/index.md)

### External References

- [Rust Generics](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
