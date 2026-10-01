---
title: 'RFC-011: Generic Type System Design - Zero-Cost Abstraction and Macro Replacement'
status: 'Accepted'
author: 'Chenxu'
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

# RFC-011: Generic Type System Design - Zero-Cost Abstraction and Macro Replacement

## Summary

This document defines the **generic type system design** of the YaoXiang language, achieving
zero-cost abstraction through powerful generic capabilities, reducing reliance on macros through
compile-time optimization, and providing dead code elimination mechanisms.

**Core Design**:

- **Unified Signature Syntax**: `(T: Type, R: Type) -> ...` generic parameters unified with regular
  parameters
- **Type Self-Description Mechanism**: `Type` is a language-level special existence; the `Type`
  position in signatures can be auto-inferred and filled
- **Type Constraint**: `T: Dup + Add` multiple constraint, function type constraint
- **Associated Type**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time Generic**: `N: Int` generic value parameter, compile-time constant instantiation
- **Conditional Type**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type
  family

**Value**:

- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro usage scenarios
- Type safety: compile-time checking, IDE-friendly
- **Explicit over implicit**: `Type` self-description, compiler auto-inference

## Reference Documents

This document's design is based on the following documents:

| Document                                                                                                   | Relationship              | Description                                                                        |
| ---------------------------------------------------------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax Foundation**     | Generic syntax integrated with unified `name: type = value` model                  |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call Syntax**           | Section 6: Generic call syntax - unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./009-ownership-model.md)                                                       | **Type System**           | Move semantics naturally combined with generics                                    |
| [RFC-024: spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution Model**       | DAG analysis and generic type checking                                             |
| [RFC-008: Runtime Model](./008-runtime-concurrency-model.md)                                               | **Compiler Architecture** | Generic monomorphization and compile-time optimization strategy                    |
| Type Universe Thought (see chapter of the same name below)                                                 | **Theoretical Core**      | Type universe hierarchy model and value-dependent type design                      |
| [RFC-027: Compile-time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination Check**     | Automatic metric synthesis and compile-time evaluation safety guarantee            |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generic system is built upon the **Type Universe thought**, a mental model that unifies
all concepts in the language into a hierarchical structure. The core innovation elevates
**value-dependent types** to first-class citizens at the Type2 level.

### What are value-dependent types?

**Value-dependent types** are types that depend on one or more **values** (rather than only on other
types). These values can be evaluated at compile-time, thereby providing type safety guarantees at
the compilation stage.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Array: (T: Type, N: Int) -> Type  # Array type depends on length value N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on number of rows and columns
```

### Container Type Naming Hierarchy

The language layer has three container concepts; where the length information resides is their
fundamental difference:

| Type          | Length        | Semantics                               | Underlying                               |
| ------------- | ------------- | --------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | **Fixed-length** array, N in the type   | Core primitive (stack/inline priority)   |
| `Vec(T)`      | Runtime value | **Runtime-length raw buffer**, growable | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type                   | Library: `{ data: Vec(T), length: Int }` |

Division of labor principles:

- **`Array(T, N)` is the only form that puts length in the type** - length is a compile-time
  constant, so boundary failures can be rejected at compile time (`a[5]` when `a: Array(Int, 3)` is
  a direct compile-time error, see "Compile-time Dimension Verification" below).
- **`Vec(T)` is the minimal foundation of runtime length** - it only provides the four capabilities:
  "can allocate, can get length, can read/write, can grow". Capacity strategy, growth factor,
  whether to shrink are not done. It is the raw material for building other containers.
- **`List(T)` is a library type, not a primitive** - defined in YaoXiang itself in `std.list`
  (`{ data: Vec(T), length: Int }`), treated the same as user-defined generic records. All resizing
  semantics strategies (when to grow, how much to grow, whether to share) are in the library; the
  compiler does not participate.

The construction form of `Vec(T)` (two layers: type parameters first, then construction parameters):

```yaoxiang
# Empty construction - length 0, elements appended later
v = Vec(Int)()

# Element construction - length determined by number of elements
w = Vec(Int)(1, 2, 3)          # length 3

# Slot allocation - allocate n zero-valued slots
buf = Vec(Int)(len=64)         # length 64, all elements zero
```

> Slot allocation uses **field-name form** (`len=`) rather than positional form: the positional
> single integer would be ambiguous with "single-element vector" (`Vec(Int)(64)` cannot distinguish
> "length 64" from "containing one element 64"). This is consistent with the unified rule for
> generic construction: field-name arguments are bound by name and are not affected by positional
> inference.
>
> This is the only primitive needed for `List` to grow - `List` allocates new slots and moves
> elements when needed:
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> When to grow, how much to grow, whether to shrink is all decided by `List`. `Vec` does not do
> capacity strategy.

From bottom to top, performance decreases and flexibility increases: `Array` > `Vec` > `List`.

> Naming basis: `Vec`/`vector` in mainstream languages (Rust/C++) both refer to runtime-length
> growable sequences; `Array` refers to fixed length.

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core
advantages:

| Feature                 | Traditional Generics (C++/Rust)               | YaoXiang Value-Dependent Types                            |
| ----------------------- | --------------------------------------------- | --------------------------------------------------------- |
| Values types depend on  | Only depend on type parameters                | Can depend on any value, including function call results  |
| Compile-time evaluation | C++ template manual specialization, Rust none | Automatic compile-time evaluation, termination guaranteed |
| Type-level computation  | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                     |
| Type safety             | C++ none, Rust limited                        | Complete type safety, compile-time checking               |
| Dimension verification  | Runtime check or manual specialization        | Compile-time automatic verification, no runtime overhead  |

### Type Universe Hierarchy and Value-Dependent Types

The Type Universe thought divides language concepts into different layers by semantic role;
value-dependent types reside at the **Type2 layer**:

| Layer     | Role                                                   | Examples                                                                                                        |
| --------- | ------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | Value                                                  | `42`, `factorial(5)`, the function itself                                                                       |
| Type0     | Meta-type keyword                                      | `Type`                                                                                                          |
| Type1     | Concrete type                                          | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **Function / type constructor / value-dependent type** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at the Type2 layer have
**unified syntax**, all in the form of `(params) -> result`:

- Regular function: `(Int, Int) -> Int` → return value is a value
- Type constructor: `(T: Type) -> Type` → return value is a type
- Value-dependent type: `(T: Type, N: Int) -> Type` → return value is a type, depending on value
  parameter N

> **Curry-Howard Isomorphism**: This unification is not a coincidence. The Curry-Howard isomorphism
> states that "types are propositions, programs are proofs" - the function type `A → B` corresponds
> to the logical implication "if A then B", the generic `(T: Type) -> Type` corresponds to universal
> quantification "for all types T", and the value-dependent type `(n: Int) -> Type` corresponds to
> "for every integer n there exists a type". YaoXiang unifies functions, type constructors, and
> value-dependent types at the Type2 layer, essentially unifying "proof" and "computation" into the
> same concept - **constructive proof**. This is precisely the direct embodiment of the Curry-Howard
> isomorphism in language design: one form (`(params) -> result`) simultaneously carries logical
> propositions and computational processes.

### Compile-time Determinism Guarantee

The Type Universe thought of YaoXiang requires: **Everything at the Type level is determined at
compile time**.

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
2. Performs compile-time termination check on functions (see termination check mechanism below)
3. Executes evaluation at compile time
4. Embeds the result into the generated type

### Application Scenarios of Value-Dependent Types

#### Compile-time Dimension Verification

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

#### Compile-time Coverage Target for Boundary Failures

> **Implementation Status Note**: Container types have been de-specialized - `Array(T, N)` is a
> const generic constructor, with literal context anchoring and `in` membership predicates both
> landed. The N and element type at the `Array(T, N)` literal anchor are already enforced by
> compile-time validation (E1002), **N is now trustworthy** - the mechanism described here can be
> built on the basis of "annotated N == runtime length". The current `[]` index out-of-bounds
> (E6003) and Dict missing key (E6008) are **runtime error transition states**; the value-dependent
> types in this section are the target mechanism to push these boundary failures to **compile
> time**:
>
> - const index: `a[5]` (5 is a compile-time constant) is directly rejected at compile time when
>   `a: Array(Int, 3)`;
> - value index: `a[i]` requires the precondition `i < len(a)`, proven by value-dependent type
>   contracts;
> - `in` predicate is the basis of Hoare logic preconditions: `n in 1..10`, `x in some_set` are all
>   compile-time provable propositions.
>
> The complete design of refinement types will supplement this section when landed.

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

# Completely transparent at use, types auto-inferred
numbers = List(Int)()   # Two-layer value construction (see §9.1); elements filled with push
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                                     | C++ Templates          | Rust Generics | Haskell GADT   | **YaoXiang**                                                             |
| ----------------------------------------------------------- | ---------------------- | ------------- | -------------- | ------------------------------------------------------------------------ |
| Type parameters                                             | ✅                     | ✅            | ✅             | ✅                                                                       |
| Value-dependent types                                       | ❌                     | ❌            | ✅             | ✅                                                                       |
| Compile-time evaluation                                     | Template instantiation | ❌            | ✅             | ✅                                                                       |
| Termination guarantee                                       | ❌                     | ❌            | ❌ (dangerous) | ✅ (automatic metric exploration + explicit measure, RFC-027)            |
| Type safety                                                 | ❌ (macro expansion)   | ✅            | ✅             | ✅                                                                       |
| Unified syntax                                              | ❌                     | ❌            | ❌             | ✅                                                                       |
| Compile-time dimension verification                         | Manual specialization  | Runtime check | Type family    | Compile-time automatic verification                                      |
| Semi-automatic termination annotation (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (no annotation syntax; explicit measure written at the type position) |

### Termination Check Mechanism (Unified with RFC-027)

The compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the
type system will fall into infinite loops. Termination checking is handled by RFC-027's compile-time
proof pipeline with **full automation priority** - the compiler first automatically explores
metrics; recursive/loop constructs that can be proven pass; those that cannot be explored and have
no explicit measure produce a compile error (RFC-027 §6.9 provides explicit measure fallback at the
type position). **No annotation syntax is provided**: RFC-022's `//! decreases`, `/*! invariant !*/`
have been deprecated along with RFC-022; the specification is the type annotation itself.

> **Trigger Criteria (RFC-027 §7)**: Termination obligation is triggered by **refined types** - a
> type being refined enters verification mode. Ordinary types that are not refined do not enter
> verification mode and generate no termination obligation.

#### Termination Check for Recursive Functions

For recursive functions with refined signatures, the compiler checks whether the arguments of
recursive calls strictly decrease on each recursive path (RFC-027 §6.7). No specification comments
are required:

```yaoxiang
# Recursive function with refined signature: no //! requires/ensures/decreases, compiler auto-explores decrease
factorial: (n: NonNegative(n)) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler explores: n-1 < n → decrease → terminate
}
```

| Scenario                                                             | Behavior                            |
| -------------------------------------------------------------------- | ----------------------------------- |
| Compiler can explore the decrease (e.g., `n-1`)                      | Pass                                |
| Cannot explore but a measure is given at the type position (§6.9)    | SMT judgment; passes if holds       |
| Cannot explore and no explicit measure / measure is falsified by SMT | Compile error                       |
| Signature has no refinement (does not enter verification mode)       | No termination obligation generated |

#### Termination Check for Loops

Loops do not require `: Invariant(...)` or `: decreases(...)` annotations. Refined type annotations
on variables (such as `UpTo(n)`) simultaneously provide loop invariants and metric bounds. The
compiler tries four metric exploration strategies in priority order, stopping when one is found
(RFC-027 §6.1–6.5):

1. **Linear rank function automatic synthesis** - extract variable bounds from type annotations,
   enumerate linear combinations, SMT verify m ≥ 0 and all paths m' < m
2. **Predicate violation count** (experimental) - extract violation_count from target type
   definitions (e.g., `Sorted`), cover adjacent swap/move
3. **Bounded increase/decrease pattern** - `v += const` → metric `upper - v` (degenerate form of
   strategy 1, fastest path)
4. **Multiplicative scaling metric template** - `v *= const` (const > 1) → metric
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # Type annotation gives upper bound arr.len and lower bound 0 → enter verification mode
    while i < arr.len {
        # Compiler auto-explores: metric arr.len - i, strictly decreases by 1 each iteration → termination proven
        s += arr[i]; i += 1
    }
    return s
}
```

When exploration fails, a loop can be bound to a name and a measure given at the type position
(RFC-027 §6.9):

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}
```

#### Termination Check Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  Type Checking Phase                                        │
│  Encounter position with refined type (parameter refinement,│
│  return refinement, variable refinement)                   │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination Check (RFC-027 proof pipeline, full auto   │
│     priority)                                               │
│     - Recursive functions: check arguments strictly decrease│
│       on each recursive path                                │
│     - Loops: four metric exploration strategies             │
│       (linear rank/violation count/bounded pattern/         │
│       multiplicative scaling), SMT verifies decrease        │
│     - Exploration fails → programmer can give a measure at  │
│       the type position (Terminates)                        │
│     - No measure / measure falsified by SMT → compile error │
│       (hard boundary)                                       │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time Evaluation (executed by built-in           │
│     interpreter)                                            │
│     - Pure functions: directly evaluate                     │
│     - Side effects: compile error (type positions must be   │
│       side-effect free)                                     │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result Embedded into Type                               │
│     - Array(Int, factorial(5)) → Array(Int, 120)            │
│     - Matrix(Float, 3, 3) → concrete type                   │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: ensures compile-time evaluation is guaranteed to terminate, avoiding the type system
  falling into infinite loops
- **Uniformity**: termination check and correctness verification (VC generation) share the same
  compile-time proof pipeline (RFC-027), with no independent specification syntax
- **Full automation priority**: the compiler automatically explores metrics from type annotations;
  if it can prove, it passes; if exploration fails, a measure can be given at the type position
  (`Terminates`), still judged by SMT - does not rely on programmers writing `decreases` syntax

## Motivation

### Why do we need a strong generic type system?

Current mainstream language generics have limitations:

| Language     | Generic Capability        | Problem                                                                                    |
| ------------ | ------------------------- | ------------------------------------------------------------------------------------------ |
| Java         | Bounded types             | Compile-time monomorphization, no generic specialization                                   |
| C#           | Generic constraints       | Runtime type checking, has performance overhead                                            |
| Rust         | Generics + Trait          | Complex Trait system, steep learning curve                                                 |
| C++          | Templates                 | Complex template specialization, poor compile error messages                               |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, compile-time dimension verification, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: powerful type system vs usability
3. **Macro vs Generics**: macro code generation vs generic type safety
4. **Value-dependence vs Type safety**: traditional generics cannot verify dimensions at compile
   time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                |
| --------------------------- | ---------------------------------------------------------------------------------------------------------- |
| **Type depends on value**   | `Array: (T: Type, N: Int) -> Type` allows types to depend on concrete values                               |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, results directly embedded in the type      |
| **Dimension verification**  | `Matrix(Float, 3, 3)` verifies matrix dimensions at compile time                                           |
| **Type-level computation**  | `If`, `Match` and other conditional types support type-level computation                                   |
| **Termination guarantee**   | Compile-time termination check (automatic metric synthesis) ensures compile-time evaluation must terminate |

```yaoxiang
# Compile-time verification impossible in C++/Rust
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# Compile-time computation: factorial(3) = 6, factorial(2) = 2
# Type is Matrix(Float, 6, 2)

# Dimension mismatch caught at compile time
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # Compile error: 2 != 3
```

### Value of the Generic System

```yaoxiang
# Example: unified API design
# map operation on different container types

# Traditional solution: separate implementation for each type
map_int_array: (array: Vec(Int), f: Fn(Int) -> Int) -> Vec(Int) = ...
map_string_array: (array: Vec(String), f: Fn(String) -> String) -> Vec(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# Generic solution: one generic function covers all types
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## Design Goals

### Core Goals

1. **Zero-cost abstraction** - generic calls equivalent to concrete type calls
2. **Dead code elimination** - compile-time analysis, only instantiate used generics
3. **Macro replacement** - generics replace 90% of macro usage scenarios
4. **Type safety** - compile-time checking, no runtime type overhead
5. **IDE-friendly** - intelligent hints, clear error messages
6. **Value-dependent types** - types can depend on values, supporting compile-time dimension
   verification
7. **Compile-time evaluation safety** - ensure compile-time evaluation terminates through
   compile-time termination check (RFC-027 automatic metric synthesis)

### Design Principles

- **Compile-time determined**: generic parameters determined at compile time
- **Monomorphization priority**: generate concrete code, avoid virtual function calls
- **Constraint-driven**: type constraints guide instantiation
- **Platform optimization**: specialization supports platform-specific optimization
- **Type Universe unification**: functions / type constructors / value-dependent types unified at
  Type2 layer
- **Termination guarantee**: function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key Rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise they will
> be inferred by HM as a function.
>
> | Writing                           | Meaning                              |
> | --------------------------------- | ------------------------------------ |
> | `List: (T: Type) -> Type = {...}` | ✅ Type constructor                  |
> | `List = {...}`                    | ❌ HM infers as function, not a type |

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

#### 1.2 Type Self-Description Mechanism

`Type` is a language-level special existence; the compiler can naturally recognize the `Type`
position in signatures and automatically infer and fill it from actual argument types.

```yaoxiang
# Compiler auto-infers generic parameters
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         Type declaration  Construction call: Int fills T, () value construction

# Function call inference
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
# Omit Type parameters when inferable
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# Must explicitly fill when not inferable
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

# Using constraints: declare type constraints directly in signatures
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
````

#### 2.2 Multiple Constraint

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

# Usage
doubled: Vec(Int) = map(Vec(1, 2, 3), (x: Int) => x * 2)  # Compiler infers
```

> **Constraint name sources (2026-09-22 note)**: Operator constraints like `Add` / `Subtract` /
> `Multiply` / `Divide` / `Modulo` are defined and landed by
> [RFC-011b: Operator Overloading and Interface-Driven Operators](./011b-operator-overloading.md) -
> `T: Add` ≜ registered `Add(T, T, T)` interface instantiation (three type parameters, result type
> `O` explicit). `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` currently **have no defined
> source**; they are dangling constraint names, to be landed in subsequent RFCs. Until then,
> examples involving these names are paper sketches.

#### 2.3 Function Type Constraint

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

**Three types of copy semantics**:

| Type                     | Meaning                                                                   | Trigger                   | Applicable Scenarios              |
| ------------------------ | ------------------------------------------------------------------------- | ------------------------- | --------------------------------- |
| **Primitive value copy** | Automatic value copy on assignment, two values are completely independent | Assignment/parameter auto | Int, Float, Bool, Char            |
| **Dup**                  | Shallow copy: copy handle/token, underlying data shared                   | Assignment/parameter auto | `&T` token, `ref T`, String/Bytes |
| **Clone**                | Deep copy: create a complete independent copy                             | `value.clone()`           | Any type implementing Clone       |

**Dup Semantics**: Types implementing Dup do not transfer ownership on assignment/parameter
passing - the compiler copies the handle/token, and multiple holders point to the same underlying
data. This is complementary to the default Move semantics in the RFC-009 ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = copy handle, share data (modifications affect each other)
Clone = copy data, independent copies (modifications don't affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) - compiler built-in value copy, not part of Dup
2. Dup  - only applies to reference/token types and internally reference-counted types
3. Clone - explicit deep copy, any type can implement
4. Default Move - other types maintain default Move semantics
```

**Which types are Dup**:

| Type                     | Dup     | Reason                                                                                         |
| ------------------------ | ------- | ---------------------------------------------------------------------------------------------- |
| `&T` (borrow token)      | ✅      | Zero-sized token, copy token = multiple views point to the same data                           |
| `ref T`                  | ✅      | Rc/Arc copy = reference count +1, share heap data                                              |
| String, Bytes            | ✅      | Internal reference counting, copy handle shares underlying buffer                              |
| `&mut T` (mutable token) | ❌      | Linearly exclusive, cannot copy                                                                |
| struct                   | Derived | All fields ∈ (primitive value type ∪ Dup) → Dup, otherwise Move (RFC-009 §derived rules, #398) |
| enum                     | Derived | All fields of all variants can be copied → enum Dup (design state, not landed yet)             |
| tuple                    | Derived | Element-by-element check, same as struct rules (#398)                                          |
| Fn (closure)             | ❌      | Captured environment may not be Dup                                                            |
| `*T` (raw pointer)       | ❌      | unsafe, not participating in ownership system                                                  |

**Int/Float/Bool/Char are not Dup** - they are value types, and the compiler automatically
value-copies on assignment (two values are completely independent). This is not "shallow copy", but
the compiler's built-in handling of primitives; it does not need and should not be expressed through
the Dup type attribute.

```yaoxiang
# Primitive value type: compiler auto value copy (not Dup)
x: Int = 42
y = x          # Value copy, x and y are completely independent
print(x)       # ✅

# Dup: shallow copy, copy handle shares data
view: &Point = &point
view2 = view    # ✅ Dup: copy token, both point to the same point
print(view.x)   # ✅

# Clone: explicit deep copy, create independent copy
backup = big_struct.clone()  # Explicit call

# Generic constraints
dup_use: (T: Dup) -> T = x         # T: Dup → can shallow copy
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can deep copy
```

> **Note**: `Send`/`Sync` are not exposed as user-visible traits. Cross-task safety guarantees are
> handled fully automatically by the `ref` keyword and compiler - `ref` automatically chooses Rc or
> Arc, users do not need to understand Send/Sync.

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
# Using method syntactic sugar: Vec.Item, Vec.next, Vec.has_next
# The iteration position is carried by the wrapper record (Vec itself is the raw buffer, no index field)
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

# Associated types can be generic
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

### 4. Compile-time Generics

#### 4.1 Compile-time Value Parameters

**Core Design**: The `Type` marker in the generic signature marks type parameters; parameters
annotated with concrete types (`Int`/`Bool`/`Float`, etc.) are listed as **compile-time value
parameter candidates**, and whether they become compile-time value parameters depends on whether
their value is **referenced at a type position** (value-dependence). No `const` keyword is needed.

> The basis for judgment is **being referenced at a type position**, not "annotated with a concrete
> type": in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters because
> they do not appear in any type position.

**Judgment Rules (Two Steps)**:

1. **Form Coarse Filter**: Parameters annotated as non-`Type` concrete types (such as `Int`) →
   listed as candidates.
2. **Usage Refinement**: Candidate names appear in **type positions** (type body field types, inner
   `Fn` parameter types, `Assert` predicates, `Array(T, N)` and other type construction argument
   positions) → confirmed as compile-time value parameters; otherwise treated as **runtime value
   parameters**.

| Writing                                                    | Judgment                       | Reason                                                                       |
| ---------------------------------------------------------- | ------------------------------ | ---------------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters   | Appear only at value positions, do not participate in type construction      |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter | N appears in the `Array(T, N)` type construction argument position           |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter | N acts as the type of inner parameter `k`                                    |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N dropped (see below)          | N is not referenced in the type body, degenerates to runtime value parameter |

> **Value-dependence essence**: Compile-time value parameters are value-dependent types - only when
> a value is used to **construct a type** does it need to be determined at compile time. Form
> (`: Int`) only determines candidate eligibility, usage (appearing at a type position) determines
> whether it is a compile-time value parameter. This is the same root criterion as "function calls
> at type positions are evaluated at compile time" in the §"Compile-time Determinism Guarantee"
> section.

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameter: N is referenced at the type position (Measure length slot)
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears at the type construction argument position → compile-time value parameter
    length: N,
}

# Usage: factorial(5) is evaluated at the type position (compile time), result 120 is embedded in the type
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# Value-dependence: N as the type of inner parameter k
# ════════════════════════════════════════════════════════
# N is a compile-time value parameter (appears at the type position of (k: N));
# k is a runtime value parameter, whose type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling of dropped candidates**: Candidates annotated with concrete types but not referenced at
> type positions (such as `N` in `Foo` in the table above) degenerate to runtime value parameters
> (function-level path). Dropped candidates on the type constructor path cannot occupy runtime slots
> (type constructors are evaluated at compile time), and the declaration side directly reports error
> [E1094]: "N declared as a compile-time value parameter but not referenced in the type body" -
> previously, silent discarding led to instantiation arity inconsistency.

#### 4.2 Compile-time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation example
# ════════════════════════════════════════════════════════

# Compiler computes function calls of literal types at compile time
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

# Usage: computed at compile time, generates Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never and Void: ⊥ and ⊤ of the Type System

YaoXiang's type system simultaneously possesses ⊥ (false/empty type) and ⊤ (true/Unit) in the
Curry-Howard isomorphism, carried by the two built-in type names `Never` and `Void`:

**Never (⊥)** - Three non-negotiable core properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. This is a
   meta-level property that must be built-in.
2. **Explosion principle**: `Never <: T` holds for any type `T`. A `Never` value can be used as any
   type - this is exactly why code after `assert(false)` still passes type checking (although it
   never executes).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   performs dead code analysis based on this.

`Never` is a built-in type name, not a keyword; the parser has no special handling. Empty sum and
type literal syntax are not provided.

**Void (⊤, i.e., Unit)** - exactly one inhabitant (the default void value), carrying the true
proposition "always true". `Void` is the identity element of the zero-field product type, and
`Never` is the identity element of the zero-variant sum type - the two are duals.
`x: Void = <default>` is legal, `x: Never = ...` has no right-hand side that can be written.

#### 4.3 Compile-time Verification (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: using conditional types
# ════════════════════════════════════════════════════════

# Standard library definition
# IsTrue: bridge from the value universe to the type universe - Bool truth value mapped to a type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has value, program continues
    false => Never,    # ⊥, no value, diverge
}

# Assert: compile-time refined type primitive - type-level expression of a Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error/diverge)
# cond not decidable → determined by proof pipeline in dispatch mode:
#                       CompileTime → Unknown, requires prove
#                       Runtime     → insert check, inject Γ hypothesis

# Usage 1: as a constraint in type definition
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at the type position)
    length: Assert(N > 0),
}

# Usage 2: use in expressions
IntArray: (N: Int) -> Type = Array(Int, N)
# Verify: IntArray(10) size equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-time Generic Specialization

```yaoxiang
# Small array optimization: use function overloading to implement compile-time generic specialization

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
    # Compiler optimization: unroll loop
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. Conditional Types

> **Curry-Howard Isomorphism**: Conditional types, viewed from the Curry-Howard perspective, are
> **case analysis** in logic. The `Bool` type corresponds to a proposition with two possible values
> (True/False), and `If` chooses different results based on whether the proposition is true - this
> is precisely case disjunction in logic. `match C { True => T, False => E }` is actually
> expressing: "given that proposition C is True, the conclusion is T; given that C is False, the
> conclusion is E".

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

# Compile-time verification (unified with the Assert definition in §4.3)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Family

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but **constructing a proposition about natural number addition**. `(Zero, B) => B` says "the
> proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` says "if Add(A',
> B) holds, then Add(Succ(A'), B) also holds". This is the addition definition in the Peano axioms
> itself. The type checker verifying this match expression passes is equivalent to verifying the
> logical consistency of this definition.

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

# Type-level addition (Curry-Howard: case analysis + recursive call, requires termination check for complete induction)
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# Example: compile-time computation of 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. Function Overload Specialization

#### 6.1 Basic Specialization

```yaoxiang
# Basic specialization: use function overloading (compiler auto-selects)
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
# Specialization fully conforming to RFC-010 syntax: function overloading

# Concrete type specialization
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation (compiler auto-selects the optimal)
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# Completely transparent at use
int_arr = Vec(Int)(1, 2, 3)
float_arr = Vec(Float)(1.0, 2.0, 3.0)

# Compiler auto-selects the optimal specialization
sum(int_arr)     # Selects sum: (Vec(Int)) -> Int
sum(float_arr)    # Selects sum: (Vec(Float)) -> Float
```

#### 6.3 Perfect Combination of Function Overloading and Inlining

**Key Feature**: Function overloading naturally combines with inlining optimization to achieve
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

# ======== After compilation (equivalent code) ========
# Compiler auto-selects the optimal specialization, then inlines
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
   - Small functions auto-inlined to call site
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function table required

4. **Perfect fit with RFC-010**

   ```yaoxiang
   # Fully using the unified syntax
   name: type = value
   # No new keywords like impl, where required
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

**What does this mean?**

- ✅ **Generic specialization** → Function overloading solves naturally
- ✅ **Performance optimization** → Inlining completed automatically
- ✅ **Code reuse** → One function name, multiple implementations
- ✅ **Zero-cost abstraction** → Compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords required** → Perfectly conforms to RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internals: build generic instantiation dependency graph
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

#### 7.2 Use Point Analysis

```yaoxiang
# Source code analysis
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Use point 1: instantiate map(Int, Int)
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # Need map[Int, Int]

# Use point 2: instantiate map(String, String)
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Need map[String, String]

# Unused: map[Float, Float] etc.
# These generic instances will not be generated

# After compilation only contains used instances
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 Compile-time Generic DCE

```yaoxiang
# Compile-time analysis: compile-time generic usage
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# Actual usage
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # Two layers: type parameters + construction parameters
arr_100_int = Array(Int, 100)()   # Empty construction, data assigned later

# After compilation only generates used sizes
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# Unused sizes will not be generated
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
# - The compiled binary contains only these two instances
```

#### 7.5 LLVM-level DCE

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
# ❌ Macro solution: code generation
macro_rules! impl_debug {
    ($($t:ty),*) => {
        $(impl Debug for $t {
            fn fmt(&self, f: &mut Formatter) -> Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

# ✅ Generic solution: auto derive
# Use function overloading for auto derivation
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# Usage
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # Auto-generate call
```

#### 8.2 DSL Replacement

```yaoxiang
# ❌ Macro solution: HTML DSL
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

# ✅ Generic solution: type-safe builder
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

#### 8.3 Type-level Programming Replacement

```yaoxiang
# ❌ Macro solution: type-level computation
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ Generic solution: conditional type
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

> **Relationship with RFC-011b (2026-09-22 note)**: The promoted type family `Add(A, B)` in this
> section is the type-level view of the operator interface registry table in
> [RFC-011b](./011b-operator-overloading.md) - the core registration `Add(Int, Float, Float)` and
> the `(Int, Float) => Float` rule in this table are the same rule; each user interface
> instantiation adds a row to this table. The Peano type-level `Add` in §5.2 is pure type-level
> computation (same name, different thing), not interfering with the value-level operator
> interface - operator queries implement the registry table, not name resolution.

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

    # Generic methods (T is automatically brought into scope by the outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generic methods ========
# Function definition under the List namespace (List. prefix = namespace ownership)
# To make the . call syntax like list.push(item) effective, explicit binding is required: List.push = push[0]
# self is just a conventional parameter name, the compiler looks at the type not the name

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # Grow
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

# Sort String array (need StringComparator)
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-time Generic Example

```yaoxiang
# ======== 1. Compile-time matrix type ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # Compile-time dimension verification: use Assert standard library type
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
   - Dimension verification and other features
   - No `const` keyword required, pure type constraints

### Disadvantages

1. **Compile time**
   - Generic instantiation increases compile time
   - Constraint solving may be slow

2. **Memory usage**
   - Compiler memory usage increases
   - Caching mechanisms need memory

3. **Implementation complexity**
   - Constraint solver is complex
   - Type-level computation engine is complex

4. **Error diagnosis**
   - Generic errors can be complex
   - Clear error messages needed

### Mitigation Measures

1. **Caching strategy**
   - Instantiation result cache
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

| Solution            | Why Not Chosen                      |
| ------------------- | ----------------------------------- |
| Basic generics only | Cannot replace complex macros       |
| Pure macro system   | No type safety, poor error messages |
| Constraint only     | Insufficient flexibility            |
| Runtime generics    | Has performance overhead            |

### Risks

| Risk                          | Impact                     | Mitigation                  |
| ----------------------------- | -------------------------- | --------------------------- |
| Constraint solving complexity | Compile time too long      | Incremental solving + cache |
| Code bloat                    | Binary file too large      | DCE + threshold control     |
| Implementation complexity     | Extended development cycle | Phased implementation       |
| Error diagnosis               | Poor user experience       | Detailed error messages     |

## Open Questions

### Pending Issues

| Issue                  | Description                              | Status     |
| ---------------------- | ---------------------------------------- | ---------- |
| Instantiation strategy | Eager vs Lazy vs Threshold               | To discuss |
| Cache size             | LRU cache capacity setting               | To discuss |
| Error diagnosis        | Detailed level of generic error messages | To discuss |

### Future Optimizations

| Optimization Item             | Value  | Implementation Difficulty |
| ----------------------------- | ------ | ------------------------- |
| Instantiation graph analysis  | High   | Medium                    |
| Type-level programming DSL    | Medium | High                      |
| Generic performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use unified () syntax, as part of the function type
# E.g., map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

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
# Generic type like List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# The Type in generic parameters is auto-filled by the compiler from actual argument types
# E.g., in map(numbers, f), T is extracted from numbers: List(Int), R is extracted from f: (Int) -> String
```

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
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
│ (Official Design) │  │ (Kept at original location) │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documents

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./009-ownership-model.md)
- [RFC-001: spawn Model](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./008-runtime-concurrency-model.md)
- [tutorial/ Tutorial](../../../tutorial/index.md)

### External References

- [Rust Generics System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
