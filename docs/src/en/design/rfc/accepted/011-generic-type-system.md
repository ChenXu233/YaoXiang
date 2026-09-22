---
title: 'RFC-011: Generic System Design - Zero-Cost Abstraction and Macro Replacement'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-15 (Type body code blocks + compile-time contracts + effect seeds implemented)'
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

# RFC-011: Generic System Design - Zero-Cost Abstraction and Macro Replacement

## Summary

This document defines YaoXiang's **generic system design**, which achieves zero-cost abstraction
through powerful generic capabilities, reduces reliance on macros through compile-time optimization,
and provides dead code elimination mechanisms.

**Core Design**:

- **Unified Signature Syntax**: `(T: Type, R: Type) -> ...` unified generic parameters and normal
  parameters
- **Type Self-Description Mechanism**: `Type` is a language-level special entity; `Type` positions
  in signatures can be automatically inferred and filled
- **Type Constraint**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated Type**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-Time Generic**: `N: Int` generic value parameters, compile-time constant instantiation
- **Conditional Type**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type
  family

**Value**:

- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro use cases
- Type safety: compile-time checking, IDE friendly
- **Explicit over implicit**: `Type` self-description, compiler auto-inference

## Reference Documents

This document's design is based on the following documents:

| Document                                                                                                   | Relationship              | Description                                                                        |
| ---------------------------------------------------------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax Foundation**     | Generic syntax integrated with unified `name: type = value` model                  |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call Syntax**           | Section 6: Generic call syntax — unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md)                                              | **Type System**           | Natural combination of Move semantics and generics                                 |
| [RFC-024: spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution Model**       | DAG analysis and generic type checking                                             |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)                                      | **Compiler Architecture** | Generic monomorphization and compile-time optimization strategy                    |
| [Type Universe Idea](../reference/plan/ongoing/类型宇宙思想.md)                                            | **Theoretical Core**      | Type universe hierarchy model and value-dependent type design                      |
| [RFC-027: Compile-Time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination Check**     | Automatic measure synthesis and compile-time evaluation safety guarantee           |

## Type Universe Idea and Value-Dependent Types

YaoXiang's generic system is built on the **Type Universe Idea**, a mental model that unifies all
concepts in the language into a hierarchical structure. The core innovation is elevating
**value-dependent types** to first-class citizens at the Type2 layer.

### What are Value-Dependent Types?

**Value-dependent types** are types that depend on one or more **values** (rather than only on other
types). These values can be evaluated at compile-time, providing type safety guarantees at the
compile stage.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Array: (T: Type, N: Int) -> Type  # Array type depends on length value N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column counts
```

### Container Type Naming Layering

The language layer has three container concepts; where length information resides is their
fundamental difference:

| Type          | Length        | Semantics                                          | Underlying                               |
| ------------- | ------------- | -------------------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | **Fixed-length** array, N is in the type           | Core primitive (stack/inline first)      |
| `Vec(T)`      | Runtime value | **Primitive buffer with runtime length**, growable | Core primitive (heap contiguous buffer)  |
| `List(T)`     | Runtime value | Standard library type                              | Library: `{ data: Vec(T), length: Int }` |

Division of labor among the three:

- **`Array(T, N)` is the only form that puts length in the type** — length is a compile-time
  constant, so compile-time rejection of boundary failures is possible (`a[5]` when
  `a: Array(Int, 3)` is directly rejected at compile-time, see "Compile-Time Dimension Validation"
  below).
- **`Vec(T)` is the minimal foundation for runtime length** — it only provides four things:
  "allocate, get length, read/write, grow". It does not handle capacity strategy, growth factor, or
  shrinking. It is the raw material for building other containers.
- **`List(T)` is a library type, not a primitive** — defined in YaoXiang itself in `std.list`
  (`{ data: Vec(T), length: Int }`), treated the same as user-defined generic records. All growable
  semantics strategies (when to expand, how much, whether to share) live in the library; the
  compiler does not participate.

`Vec(T)`'s construction form (two layers: type parameters first, then construction parameters):

```yaoxiang
# Empty construction — length 0, elements appended later
v = Vec(Int)()

# Element construction — length determined by number of elements
w = Vec(Int)(1, 2, 3)          # length 3

# Slot allocation — allocate n zero-value slots
buf = Vec(Int)(len=64)         # length 64, all elements are zero values
```

> Slot allocation uses **field name form** (`len=`) rather than positional: a single positional
> integer would be ambiguous with "single-element vector" (`Vec(Int)(64)` cannot distinguish "length
> 64" from "one element 64"). This is consistent with the unified rules for generic construction:
> field-named arguments are bound by name, unaffected by positional inference.
>
> This is the only primitive needed for `List` expansion — `List` allocates new slots and moves
> elements when needed:
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> When to expand, how much, and whether to shrink are all decided by `List`. `Vec` does not handle
> capacity strategy.

From bottom to top, performance decreases and flexibility increases: `Array` > `Vec` > `List`.

> Naming rationale: `Vec`/`vector` refers to a growable sequence with runtime length in mainstream
> languages (Rust/C++); `Array` refers to a fixed-length array.

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core
advantages:

| Feature                    | Traditional Generics (C++/Rust)                | YaoXiang Value-Dependent Types                            |
| -------------------------- | ---------------------------------------------- | --------------------------------------------------------- |
| Values the type depends on | Only type parameters                           | Can depend on any value, including function call results  |
| Compile-time evaluation    | C++ template manual specialization, Rust: none | Automatic compile-time evaluation, termination guaranteed |
| Type-level computation     | Template metaprogramming (complex/dangerous)   | Unified type-level computation engine                     |
| Type safety                | C++: none, Rust: limited                       | Complete type safety, compile-time checking               |
| Dimension validation       | Runtime check or manual specialization         | Compile-time dimension validation, no runtime overhead    |

### Type Universe Hierarchy and Value-Dependent Types

The Type Universe idea divides language concepts into different layers according to their semantic
roles. Value-dependent types reside at the **Type2 layer**:

| Layer     | Role                                               | Example                                                                                                         |
| --------- | -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | Value                                              | `42`, `factorial(5)`, the function itself                                                                       |
| Type0     | Meta type keyword                                  | `Type`                                                                                                          |
| Type1     | Concrete type                                      | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **Function/type constructor/value-dependent type** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at the Type2 layer use
**unified syntax**, all in the form `(params) -> result`:

- Normal function: `(Int, Int) -> Int` → the return value is a value
- Type constructor: `(T: Type) -> Type` → the return value is a type
- Value-dependent type: `(T: Type, N: Int) -> Type` → the return value is a type, depending on value
  parameter N

> **Curry-Howard Isomorphism**: this unification is no coincidence. Curry-Howard Isomorphism states
> "types are propositions, programs are proofs" — function type `A → B` corresponds to logical
> implication "if A then B", generic `(T: Type) -> Type` corresponds to universal quantification
> "for all types T", value-dependent type `(n: Int) -> Type` corresponds to "for every integer n
> there exists a type". YaoXiang unifies functions, type constructors, and value-dependent types at
> the Type2 layer, essentially unifying "proof" and "computation" into a single concept —
> **constructive proof**. This is the direct embodiment of Curry-Howard Isomorphism in language
> design: one form (`(params) -> result`) simultaneously carries logical propositions and
> computational processes.

### Compile-Time Determinism Guarantee

YaoXiang's Type Universe idea requires: **everything at the Type layer is determined at
compile-time**.

```yaoxiang
# Compile-time dimension validation example
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # Compile-time check: dimensions must be positive
    _assert: Assert(Rows > 0),
    _assert: Assert(Cols > 0),
}

# Create a 3x3 identity matrix - done at compile-time
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# Compile-time computation: factorial(3) = 6, array size determined at compile-time
arr: Array(Int, factorial(3)) = Array(Int, 6)()
```

The compiler will automatically:

1. Detect function calls in type positions
2. Perform compile-time termination check on the function (see termination check mechanism below)
3. Perform evaluation at compile-time
4. Embed the result in the generated type

### Application Scenarios of Value-Dependent Types

#### Compile-Time Dimension Validation

```yaoxiang
# Matrix multiplication: compile-time validation of dimension matching
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # Compile-time check: a.Cols == b.Rows, otherwise compile error
    result = Matrix(T, Rows, M)()
    # ...
}

# Error caught at compile-time:
# multiply(matrix_2x3, matrix_4x2)  # Compile error: 2 != 4
```

#### Type-Safe Array Size

```yaoxiang
# Array size is a compile-time constant
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N is a compile-time constant and can be used in type-level computation
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3 (known at compile-time)
```

#### Goal: Boundary Failure Coverage at Compile-Time

> **Implementation Status Note**: Container types have been de-specialized — `Array(T, N)` is a
> const generic constructor, with literal context landing points and `in` membership predicates
> already implemented. The N and element type of `Array(T, N)` literal landing points have been
> enforced by compile-time validation (E1002), **N is now trustworthy** — the mechanisms described
> in this section can build on the "annotation N == runtime length" foundation. Currently `[]` index
> out-of-bounds (E6003) and Dict missing key (E6008) are **runtime error transitional states**; this
> section's value-dependent types are the target mechanism to push these boundary failures to
> **compile-time**:
>
> - const index: `a[5]` (5 is a compile-time constant) is directly rejected at compile-time when
>   `a: Array(Int, 3)`;
> - value index: `a[i]` requires the precondition `i < len(a)`, proven by value-dependent type
>   contracts;
> - the `in` predicate is the foundation of Hoare logic preconditions: `n in 1..10`, `x in some_set`
>   are all propositions provable at compile-time.
>
> The complete design of refinement types will supplement this section upon landing.

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
# map: generic function, type parameters T, R determined at compile-time
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
numbers = List(Int)()   # Two-layer value construction (see §9.1); elements populated via push
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                                     | C++ Templates          | Rust Generics | Haskell GADT   | **YaoXiang**                              |
| ----------------------------------------------------------- | ---------------------- | ------------- | -------------- | ----------------------------------------- |
| Type parameters                                             | ✅                     | ✅            | ✅             | ✅                                        |
| Value-dependent types                                       | ❌                     | ❌            | ✅             | ✅                                        |
| Compile-time evaluation                                     | Template instantiation | ❌            | ✅             | ✅                                        |
| Termination guarantee                                       | ❌                     | ❌            | ❌ (dangerous) | ✅ (automatic measure synthesis, RFC-027) |
| Type safety                                                 | ❌ (macro expansion)   | ✅            | ✅             | ✅                                        |
| Unified syntax                                              | ❌                     | ❌            | ❌             | ✅                                        |
| Compile-time dimension validation                           | Manual specialization  | Runtime check | Type family    | Compile-time automatic validation         |
| Semi-automatic termination annotation (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (only fully automatic compile-time)    |

### Termination Check Mechanism (Unified with RFC-027)

The compile-time evaluation of value-dependent types must **guarantee termination**; otherwise the
type system falls into infinite loops. The termination check is completed **fully automatically** by
RFC-027's compile-time proof pipeline — the compiler automatically synthesizes measures;
recursive/loop constructs that can be proven pass, those that cannot are directly reported as
compile errors. **No escape hatch is left for semi-automatic annotations**: RFC-022's
`//! decreases`, `/*! invariant !*/` have been deprecated along with RFC-022; specifications are the
type annotations themselves.

#### Termination Check for Recursive Functions

Before compile-time evaluation, the compiler checks whether the parameters of recursive calls
strictly decrease on every recursive path (RFC-027 §6.7). No specification comments are needed:

```yaoxiang
# Compile-time factorial: no //! requires/ensures/decreases, compiler analyzes automatically
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Use: called in a type position, compiler first verifies termination, then evaluates
arr: Array(Int, factorial(5)) = Array(Int, 120)()  # Compile-time evaluation: factorial(5) = 120
```

| Scenario                                             | Behavior                    |
| ---------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g. `n-1`) | Compile-time evaluation     |
| Non-decreasing/cannot determine decrease             | Compile error               |
| Runtime call (not in type position)                  | No termination check needed |

#### Termination Check for Loops

Loops do not need `: Invariant(...)` or `: decreases(...)` annotations. Refinement type annotations
on variables (e.g. `UpTo(n)`) provide both loop invariants and measure bounds. The compiler tries
four measure synthesis strategies in priority order, stopping once one succeeds (RFC-027 §7):

1. **Automatic linear rank function synthesis** — extract variable bounds from type annotations,
   enumerate linear combinations, SMT verifies m ≥ 0 and m' < m on all paths
2. **Predicate violation count** (experimental) — extract violation_count from target type
   definitions (e.g. `Sorted`), covers adjacent swaps/moves
3. **Bounded increase/decrease patterns** — `v += const` → measure `upper - v` (degeneration of
   strategy 1, fastest path)
4. **Multiplicative scaling measure template** — `v *= const` (const > 1) → measure
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # Type annotation gives upper bound arr.len and lower bound 0
    while i < arr.len {
        # Compiler auto-derives: measure arr.len - i, strictly decreases by 1 each iteration → termination proven
        s += arr[i]; i += 1
    }
    return s
}
```

#### Workflow of Termination Check

```
┌─────────────────────────────────────────────────────────────┐
│  Type Checking Phase                                       │
│  Encountering function call in type position (e.g. factorial(5))│
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination Check (RFC-027 proof pipeline, fully automatic)│
│     - Recursive function: check parameters strictly        │
│       decrease on every recursive path                     │
│     - Loop: four measure synthesis strategies (linear rank/ │
│       violation count/bounded pattern/                     │
│       multiplicative scaling), SMT verifies decrease       │
│     - Cannot prove → compile error (hard boundary, no      │
│       semi-automatic annotation fallback)                  │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-Time Evaluation (executed by built-in interpreter)│
│     - Pure function: evaluate directly                     │
│     - Side effects: compile error (type position must be  │
│       side-effect free)                                   │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result Embedded in Type                                │
│     - Array(Int, factorial(5)) → Array(Int, 120)           │
│     - Matrix(Float, 3, 3) → concrete type                  │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: ensures that compile-time evaluation must terminate, preventing the type system from
  falling into infinite loops
- **Uniformity**: termination check and correctness verification (VC generation) share the same
  compile-time proof pipeline (RFC-027), no separate specification syntax
- **Fully automatic**: the compiler automatically synthesizes measures from type annotations;
  provable ones pass, unprovable ones error — no reliance on programmers hand-writing `decreases`

## Motivation

### Why Do We Need a Strong Generic System?

Current mainstream languages have limitations in generics:

| Language     | Generic Capability        | Problem                                                                                  |
| ------------ | ------------------------- | ---------------------------------------------------------------------------------------- |
| Java         | Bounded types             | Compile-time monomorphization, no specialization                                         |
| C#           | Generic constraints       | Runtime type checking, performance overhead                                              |
| Rust         | Generics + Trait          | Trait system is complex, steep learning curve                                            |
| C++          | Templates                 | Template specialization is complex, poor compile error messages                          |
| **YaoXiang** | **Value-Dependent Types** | **Types can depend on values, compile-time dimension validation, termination guarantee** |

### Core Contradiction

1. **Performance vs Flexibility**: runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: powerful type system vs ease of use
3. **Macro vs Generics**: macro code generation vs generic type safety
4. **Value-Dependent vs Type Safety**: traditional generics cannot validate dimensions at
   compile-time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                 |
| --------------------------- | ----------------------------------------------------------------------------------------------------------- |
| **Type depends on value**   | `Array: (T: Type, N: Int) -> Type` lets the type depend on specific values                                  |
| **Compile-time evaluation** | Function calls in type positions are evaluated at compile-time, results directly embedded in the type       |
| **Dimension validation**    | `Matrix(Float, 3, 3)` validates matrix dimensions at compile-time                                           |
| **Type-level computation**  | Conditional types like `If`, `Match` support type-level computation                                         |
| **Termination guarantee**   | Compile-time termination check (automatic measure synthesis) ensures compile-time evaluation must terminate |

```yaoxiang
# Compile-time validation that C++/Rust cannot do
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# Compile-time computation: factorial(3) = 6, factorial(2) = 2
# Type is Matrix(Float, 6, 2)

# Dimension mismatch caught at compile-time
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # Compile error: 2 != 3
```

### Value of the Generic System

```yaoxiang
# Example: unified API design
# map operation on different container types

# Traditional approach: each type implemented separately
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

1. **Zero-Cost Abstraction** - generic calls equivalent to concrete type calls
2. **Dead Code Elimination** - compile-time analysis, only instantiate used generics
3. **Macro Replacement** - generics replace 90% of macro use cases
4. **Type Safety** - compile-time checking, no runtime type overhead
5. **IDE Friendly** - smart hints, clear error messages
6. **Value-Dependent Types** - types can depend on values, supporting compile-time dimension
   validation
7. **Compile-Time Evaluation Safety** - guarantee compile-time evaluation termination through
   compile-time termination check (RFC-027 automatic measure synthesis)

### Design Principles

- **Compile-Time Determination**: generic parameters determined at compile-time
- **Monomorphization First**: generate concrete code, avoid virtual function calls
- **Constraint Driven**: type constraints guide instantiation
- **Platform Optimization**: specialization supports platform-specific optimization
- **Type Universe Unification**: function/type constructor/value-dependent type unified at Type2
  layer
- **Termination Guarantee**: function calls in type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key Rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise they will
> be inferred by HM as functions.
>
> | Syntax                            | Meaning                            |
> | --------------------------------- | ---------------------------------- |
> | `List: (T: Type) -> Type = {...}` | ✅ Type constructor                |
> | `List = {...}`                    | ❌ HM infers as function, not type |

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
    push: (self: List(T), item: T) -> Void,   # self is just a conventional name, not a keyword
    get: (self: List(T), index: Int) -> Option(T),
}

# Generic function (no : Type, HM infers as function)
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# Generic constraint (direct expression, return can be omitted for single-line)
clone: (T: Clone)(value: T) -> T = value.clone()

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### Generic Function Call Syntax

#### 1.1 Unified Signature Syntax

```yaoxiang
# Generic function uses unified (T: Type, R: Type) signature syntax
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type Self-Description Mechanism

`Type` is a language-level special entity. The compiler naturally recognizes `Type` positions in
signatures and automatically infers and fills them from actual argument types.

```yaoxiang
# Compiler automatically infers generic parameters
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

# Use constraints: directly declare type constraints in the signature
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

# Use
doubled: Vec(Int) = map(Vec(1, 2, 3), (x: Int) => x * 2)  # Compiler infers
```

> **Source of constraint names (2026-09-22 note)**: `Add` / `Subtract` / `Multiply` / `Divide` /
> `Modulo` and other operator constraints are defined and implemented by
> [RFC-011b: Operator Overloading and Interface-Driven Operators](./011b-operator-overloading.md) —
> `T: Add` ≜ `Add(T, T, T)` interface instantiation has been registered (three type parameters,
> result type `O` explicit). `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` currently have **no
> defined source**, are dangling constraint names, to be implemented by subsequent RFCs; until then,
> examples involving these names are paper sketches.

#### 2.3 Function Type Constraints

```yaoxiang
# Higher-order function constraints
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

call_with_arg: (T: Type, U: Type, F: Fn(T) -> U)(arg: T, f: F) -> U = f(arg)

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))

# Use example
result: Int = call_with_arg(42, (x: Int) => x * 2)  # result = 84
composed: String = compose(
    "hello",
    (s: String) => s.to_uppercase(),
    (s: String) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

#### 2.4 Built-in Marker Trait: Dup and Clone

**Three Types of Copy Semantics**:

| Type                     | Meaning                                                          | Trigger Method                            | Applicable Scenario               |
| ------------------------ | ---------------------------------------------------------------- | ----------------------------------------- | --------------------------------- |
| **Primitive Value Copy** | Auto value copy on assignment, two values completely independent | Automatic on assignment/parameter passing | Int, Float, Bool, Char            |
| **Dup**                  | Shallow copy: copy handle/token, underlying data shared          | Automatic on assignment/parameter passing | `&T` token, `ref T`, String/Bytes |
| **Clone**                | Deep copy: create complete independent replica                   | `value.clone()`                           | Any type implementing Clone       |

**Dup Semantics**: types implementing Dup do not transfer ownership on assignment/parameter passing
— the compiler copies the handle/token, and multiple holders point to the same underlying data. This
complements the Move default semantics in RFC-009's ownership model.

**Dup and Clone are Orthogonal Concepts**:

```
Dup = copy handle, share data (modifications affect each other)
Clone = copy data, replica independent (modifications do not affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not Dup
2. Dup — only applies to reference/token types and internally reference-counted types
3. Clone — explicit deep copy, any type can implement it
4. Default Move — other types maintain default Move semantics
```

**Which Types are Dup**:

| Type                     | Dup     | Reason                                                                |
| ------------------------ | ------- | --------------------------------------------------------------------- |
| `&T` (borrow token)      | ✅      | Zero-size token, copying token = multiple views pointing to same data |
| `ref T`                  | ✅      | Rc/Arc copy = reference count +1, sharing heap data                   |
| String, Bytes            | ✅      | Internal reference counting, copy handle shares underlying buffer     |
| `&mut T` (mutable token) | ❌      | Linear exclusive, cannot copy                                         |
| struct                   | Derived | All fields Dup → struct Dup                                           |
| enum                     | Derived | All variants' all fields Dup → enum Dup                               |
| tuple                    | Derived | All elements Dup → tuple Dup                                          |
| Fn (closure)             | ❌      | Captured environment may not be Dup                                   |
| `*T` (raw pointer)       | ❌      | unsafe, not part of ownership system                                  |

**Int/Float/Bool/Char are not Dup** — they are value types; the compiler automatically copies the
value on assignment (two values are completely independent). This is not "shallow copy", but the
compiler's built-in handling of primitives, and does not need or should be expressed through the Dup
type attribute.

```yaoxiang
# Primitive value types: compiler auto value copy (not Dup)
x: Int = 42
y = x          # Value copy, x and y are completely independent
print(x)       # ✅

# Dup: shallow copy, copy handle shares data
view: &Point = &point
view2 = view    # ✅ Dup: copy token, both point to the same point
print(view.x)   # ✅

# Clone: explicit deep copy, create independent replica
backup = big_struct.clone()  # Explicit call

# Generic constraints
dup_use: (T: Dup) -> T = x         # T: Dup → can shallow copy
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can deep copy
```

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety is guaranteed by the `ref`
> keyword and fully automatic compiler handling — `ref` automatically chooses Rc or Arc, users don't
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

# Use
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
# The iteration position is carried by the wrapper record (Vec itself is the primitive buffer, no index field)
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

#### 3.2 Generic Associated Type (GAT)

```yaoxiang
# More complex associated types
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# Associated types can be generic
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # Associated type is also generic
    iter: (Self) -> IteratorType,
}

# Use
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. Compile-Time Generics

#### 4.1 Compile-Time Value Parameters

**Core Design**: `Type` markers in generic signatures mark type parameters; parameters annotated
with concrete types (such as `Int`/`Bool`/`Float`) are listed as **compile-time value parameter
candidates**. Whether they become compile-time value parameters depends on whether their values are
**referenced in type positions** (value-dependent). No `const` keyword is needed.

> The criterion is **referenced in a type position**, not "annotated with a concrete type": in
> `add: (a: Int, b: Int) -> Int = a + b` `a`/`b` are runtime value parameters because neither
> appears in any type position.

**Determination Rules (Two Steps)**:

1. **Shape Coarse Filtering**: parameter annotated as a non-`Type` concrete type (e.g. `Int`) → list
   as candidate.
2. **Use Fine Filtering**: the candidate name appears in a **type position** (type body field type,
   inner `Fn` parameter type, `Assert` predicate, type constructor argument position such as
   `Array(T, N)`) → confirmed as compile-time value parameter; otherwise treated as **runtime value
   parameter**.

| Syntax                                                     | Determination                       | Reason                                                                 |
| ---------------------------------------------------------- | ----------------------------------- | ---------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b are runtime value parameters    | Only appears in value position, not participating in type construction |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N is a compile-time value parameter | N appears in the type constructor argument position of `Array(T, N)`   |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N is a compile-time value parameter | N serves as the type of inner parameter `k`                            |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through (see below)         | N not referenced in type body, degrades to runtime value parameter     |

> **Value-Dependent Essence**: compile-time value parameters are value-dependent types — only when a
> value is used to **construct a type**, does it need to be determined at compile-time. Shape
> (`: Int`) only determines candidate eligibility; use (appears in type position) determines whether
> it is a compile-time value parameter. This is the same criterion as "function calls in type
> positions are evaluated at compile-time" in §"Compile-Time Determinism Guarantee".

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameter: N is referenced in type position (Measure length slot)
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears in type constructor argument position → compile-time value parameter
    length: N,
}

# Usage: factorial(5) is evaluated at type position (compile-time), result 120 is embedded in type
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# Value-dependent: N serves as the type of inner parameter k
# ════════════════════════════════════════════════════════
# N is a compile-time value parameter (appears in the type position of (k: N));
# k is a runtime value parameter, its type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling of Fall-Through Candidates**: candidates annotated with concrete types but not
> referenced in type positions (e.g. `N` in `Foo` in the table above) degrade to runtime value
> parameters (function-level path). Fall-through candidates in the type constructor path cannot
> occupy runtime slots (type constructors are evaluated at compile-time), and the declaration side
> directly reports error [E1094]: "N is declared as a compile-time value parameter but not
> referenced in the type body" — previous silent dropping led to instantiation arity inconsistency.

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation example
# ════════════════════════════════════════════════════════

# Compiler computes literal type function calls at compile-time
SIZE: Int = factorial(5)  # Compile-time is 120

# Matrix type usage
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time dimension validation
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

# Use: compile-time computation, generates Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never and Void: ⊥ and ⊤ of the Type System

YaoXiang's type system has both ⊥ (false/empty type) and ⊤ (true/Unit) in the Curry-Howard
Isomorphism, carried by the two built-in type names `Never` and `Void`:

**Never (⊥)** — Three Non-Negotiable Kernel Properties:

1. **Zero Constructor**: no literal or expression can produce a value of type `Never`. This is a
   meta-level property that must be built in.
2. **Principle of Explosion**: `Never <: T` holds for any type `T`. A `Never` value can be used as
   any type — this is exactly why code after `assert(false)` still passes type checking (although it
   never executes).
3. **Divergence Marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis.

`Never` is a built-in type name, not a keyword, the parser doesn't care. No empty sum type literal
syntax is exposed.

**Void (⊤, i.e. Unit)** — Exactly one inhabitant (default void value), the carrier of the true
proposition "always true". `Void` is the unit of zero-field product types, `Never` is the unit of
zero-variant sum types — they are dual. `x: Void = <default>` is legal, `x: Never = ...` has no
right-hand side to write.

#### 4.3 Compile-Time Validation (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: leveraging conditional types
# ════════════════════════════════════════════════════════

# Standard library definition
# IsTrue: bridge from value universe to type universe — Bool truth value maps to type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has value, program continues
    false => Never,    # ⊥, no value, diverges
}

# Assert: compile-time refinement type primitive — type-level statement of Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error/divergence)
# cond cannot be decided → decided by proof pipeline in dispatch mode:
#                  CompileTime → Unknown, requires prove
#                  Runtime     → insert check, inject Γ assumption

# Usage 1: as constraint in type definition
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert in type position)
    length: Assert(N > 0),
}

# Usage 2: used in expression
IntArray: (N: Int) -> Type = Array(Int, N)
# Validate: IntArray(10) size equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generic Specialization

```yaoxiang
# Small array optimization: use function overloading for compile-time generic specialization

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

> **Curry-Howard Isomorphism**: from the Curry-Howard perspective, conditional types are **case
> analysis** in logic. `Bool` type corresponds to a proposition with two possible values
> (True/False), `If` chooses different results based on the truth of that proposition — this is
> exactly case disjunction in logic. `match C { True => T, False => E }` actually expresses: "when
> the known proposition C is True, the conclusion is T; when C is False, the conclusion is E".

#### 5.1 If Conditional Type

```yaoxiang
# Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# Example: compile-time branch
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# Compile-time validation (unified with §4.3's Assert definition)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Use
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Family

> **Curry-Howard Isomorphism**: type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but **constructing a proposition about natural number addition**. `(Zero, B) => B` means "the
> proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` means "if
> Add(A', B) holds, then Add(Succ(A'), B) also holds". This is exactly the addition definition in
> Peano axioms. The type checker verifying this match expression passes is equivalent to verifying
> the logical consistency of this definition.

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

# Type-level addition (Curry-Howard: case analysis + recursive call, requires termination check to be complete induction)
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
# Fully conforms to RFC-010 syntax specialization: function overloading

# Concrete type specialization
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation (compiler auto-selects optimal)
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

**Key Feature**: function overloading naturally combines with inlining optimization, achieving
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

# Use
int_arr = Vec(Int)(1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== After compilation (equivalent code) ========
# Compiler auto-selects optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core Advantages**:

1. **Compiler Intelligent Selection**

   ```yaoxiang
   sum(int_arr)      # Auto-selects sum: (Vec(Int)) -> Int
   sum(float_arr)    # Auto-selects sum: (Vec(Float)) -> Float
   sum(custom_arr)  # Auto-selects sum: (T: Type) -> ((arr: Vec(T)) -> T)
   ```

2. **Inlining Optimization**
   - Small functions auto-inline to call site
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type Safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function table needed

4. **Perfect Fit with RFC-010**

   ```yaoxiang
   # Fully uses unified syntax
   name: type = value
   # No need for new keywords like impl, where
   ```

**Practical Application Example**:

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

**What Does This Mean?**

- ✅ **Generic Specialization** → function overloading naturally solves it
- ✅ **Performance Optimization** → inlining auto-completed
- ✅ **Code Reuse** → one function name, multiple implementations
- ✅ **Zero-Cost Abstraction** → compile-time polymorphism, zero runtime overhead
- ✅ **No New Keywords Needed** → perfectly fits RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internal: build generic instantiation dependency graph
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

# Unused: map[Float, Float] etc.
# These generic instances will not be generated

# After compilation, only used instances are included
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
arr_100_int = Array(Int, 100)()   # Empty construction, data assigned later

# After compilation, only used Sizes are generated
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# Unused Size will not be generated
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

# Compilation analysis:
# - Module B uses map[Int, Int]
# - Module C uses map[String, String]
# - After compilation, the binary only contains these two instances
```

#### 7.5 LLVM-Level DCE

```rust
// Compilation pipeline
fn optimize_ir(ir: &mut IR) {
    // 1. Monomorphization (YaoXiang compiler)
    ir.monomorphize();

    // 2. Inline optimization
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

# ✅ Generic approach: automatic derivation
# Auto-derive using function overloading
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# Use
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

# ✅ Generic approach: conditional type
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Int, Int) => Int,
    (Float, Float) => Float,
    (Int, Float) => Float,
    (Float, Int) => Float,
    _ => TypeError,
}

# Compile-time validation
AssertAddable: (A: Type, B: Type) -> Type = If(Add(A, B) != TypeError, (A, B), compile_error("Cannot add"))

# Use
result_type = Add[Int, Float]  # Inferred as Float
```

> **Relationship with RFC-011b (2026-09-22 note)**: the lifting type family `Add(A, B)` in this
> section is [RFC-011b](./011b-operator-overloading.md)'s perspective of the operator interface
> registration table at the type level — the core registration `Add(Int, Float, Float)` and the
> table entry `(Int, Float) => Float` in this section are the same rule, each interface
> instantiation by the user adds a row to this table. §5.2's Peano type-level `Add` is pure
> type-level computation (same name, different thing), not interfering with the value-level operator
> interface — operator queries go through the implementation registry, not name resolution.

### 9. Examples

#### 9.1 Complete Generic Container Example

```yaoxiang
# ======== 1. Define generic container ========
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

    # Generic methods (T is automatically brought into scope by outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generic methods ========
# Function definitions under List namespace (List. prefix = namespace attribution)
# To make the . call syntax like list.push(item) work, need explicit binding: List.push = push[0]
# self is just a conventional parameter name, compiler looks at type, not name

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

# ======== 4. Use example ========
# Create generic List
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
# Implemented using function overloading
compare: (a: Int, b: Int) -> Int = {
    if a < b {
        return -1
    } else if a > b {
        return 1
    } else {
        return 0
    }
}

# ======== 3. Use example ========
# Sort Int array
numbers = Vec(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# Sort String array (needs StringComparator)
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-Time Generic Example

```yaoxiang
# ======== 1. Compile-time matrix type ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # Compile-time dimension validation: leveraging Assert standard library type
    _assert: Assert(Rows > 0),  # Rows > 0, otherwise compile error
    _assert: Assert(Cols > 0),  # Cols > 0, otherwise compile error

    # Matrix operation
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

# ======== 3. Use example ========
# Create a matrix with compile-time known size
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

# Compile-time validation: result type is Matrix(Float, 2, 2)
# 2x2 identity matrix
identity_3x3 = identity(Float, 3)()

# Dimension mismatch: compile error
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # Compile error: 3x3 != 2x3
```

## Trade-offs

### Advantages

1. **Zero-Cost Abstraction**
   - Compile-time monomorphization, no runtime overhead
   - No virtual functions, no RTTI

2. **Dead Code Elimination**
   - Compile-time analysis, only instantiate used generics
   - Code bloat is controllable

3. **Macro Replacement**
   - Type-safe code generation
   - IDE friendly, clear error messages

4. **Compile-Time Computation**
   - Compile-time generics support compile-time computation
   - Features like dimension validation
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compile Time**
   - Generic instantiation increases compile time
   - Constraint solving may be slow

2. **Memory Footprint**
   - Compiler memory footprint increases
   - Caching mechanism needs memory

3. **Implementation Complexity**
   - Constraint solver is complex
   - Type-level computation engine is complex

4. **Error Diagnosis**
   - Generic errors may be complex
   - Need clear error hints

### Mitigation Measures

1. **Caching Strategy**
   - Cache instantiation results
   - LRU cache limits memory

2. **Incremental Compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error Hints**
   - Clear error messages
   - Generic parameter inference hints

4. **Parallel Compilation**
   - Parallel instantiation of generics
   - Multi-threaded constraint solving

## Alternatives

| Alternative         | Why Not Chosen                      |
| ------------------- | ----------------------------------- |
| Basic generics only | Cannot replace complex macros       |
| Pure macro system   | No type safety, poor error messages |
| Constraints only    | Insufficient flexibility            |
| Runtime generics    | Has performance overhead            |

### Risks

| Risk                          | Impact                     | Mitigation                    |
| ----------------------------- | -------------------------- | ----------------------------- |
| Constraint solving complexity | Compile time too long      | Incremental solving + caching |
| Code bloat                    | Binary file too large      | DCE + threshold control       |
| Implementation complexity     | Development cycle extended | Phased implementation         |
| Error diagnosis               | Poor user experience       | Detailed error messages       |

## Open Issues

### Issues to be Resolved

| Topic                  | Description                            | Status     |
| ---------------------- | -------------------------------------- | ---------- |
| Instantiation strategy | Eager vs Lazy vs Threshold             | To discuss |
| Cache size             | LRU cache capacity setting             | To discuss |
| Error diagnosis        | Detail level of generic error messages | To discuss |

### Future Optimization

| Optimization Item             | Value  | Implementation Difficulty |
| ----------------------------- | ------ | ------------------------- |
| Instantiation graph analysis  | High   | Medium                    |
| Type-level programming DSL    | Medium | High                      |
| Generic performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use unified () syntax, as part of function type
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
# Generic type e.g. List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# Type in generic parameters is automatically filled by the compiler from actual argument types
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
│  Under Review│  ← Open community discussion and feedback
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
│ (Formal Design)│  │ (Kept in place)│
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
