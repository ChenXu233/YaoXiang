---
title: 'RFC-011: Generic System Design - Zero-Cost Abstraction and Macro Replacement'
status: 'Accepted'
author: 'Chen Xu'
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

This document defines YaoXiang's **generic system design**, achieving zero-cost abstraction through
powerful generic capabilities, reducing reliance on macros through compile-time optimization, and
providing a dead code elimination mechanism.

**Core Design**:

- **Unified signature syntax**: `(T: Type, R: Type) -> ...` generic parameters unified with regular
  parameters
- **Type self-describing mechanism**: `Type` is a language-level special existence; the `Type`
  position in signatures can be auto-inferred and filled
- **Type constraints**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated types**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time generics**: `N: Int` generic value parameters, compile-time constant instantiation
- **Conditional types**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type
  families

**Value**:

- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro usage scenarios
- Type safety: compile-time checking, IDE-friendly
- **Explicit over implicit**: `Type` self-describing, compiler auto-inference

## Reference Documents

This document's design is based on the following documents:

| Document                                                                                                   | Relationship              | Description                                                                      |
| ---------------------------------------------------------------------------------------------------------- | ------------------------- | -------------------------------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax Foundation**     | Generic syntax integrated with the unified `name: type = value` model            |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call Syntax**           | Section 6: Generic call syntax—unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md)                                              | **Type System**           | Natural combination of Move semantics with generics                              |
| [RFC-024: spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution Model**       | DAG analysis and generic type checking                                           |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)                                      | **Compiler Architecture** | Generic monomorphization and compile-time optimization strategies                |
| [Type Universe Thought](../reference/plan/ongoing/类型宇宙思想.md)                                         | **Theoretical Core**      | Type universe hierarchy model and value-dependent type design                    |
| [RFC-027: Compile-time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination Check**     | Automatic metric synthesis and compile-time evaluation safety guarantees         |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generic system is built upon the **Type Universe thought**, a mental model that unifies
all concepts in the language into a hierarchical structure. The core innovation is promoting
**value-dependent types** to first-class citizens at the Type2 layer.

### What are Value-Dependent Types?

**Value-dependent types** are types that depend on one or more **values** (not just other types).
These values can be evaluated at compile time, providing type safety guarantees during the
compilation phase.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Array: (T: Type, N: Int) -> Type  # Array type depends on length value N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column counts
```

### Container Type Naming Layering

The language layer has three container concepts; the ownership of length information is their
fundamental distinction:

| Type          | Length        | Semantics                                | Underlying                               |
| ------------- | ------------- | ---------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | **Fixed-length** array, N is in the type | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | **Runtime-length raw buffer**, growable  | Core primitive (contiguous heap buffer)  |
| `List(T)`     | Runtime value | Standard library type                    | Library: `{ data: Vec(T), length: Int }` |

Division of labor among the three:

- **`Array(T, N)` is the only form that puts length into the type**—length is a compile-time
  constant, so boundary failure compile-time rejection is possible (`a[5]` when `a: Array(Int, 3)`
  directly produces a compile-time error, see "Compile-time dimension validation" below).
- **`Vec(T)` is the minimal foundation for runtime length**—only provides four things: "can
  allocate, can get length, can read/write, can grow". Capacity strategy, growth factor, and
  shrinkage are all not handled. It is the raw material for building other containers.
- **`List(T)` is a library type, not a primitive**—defined in YaoXiang itself in `std.list`
  (`{ data: Vec(T), length: Int }`), on equal footing with user-defined generic records. All
  growable semantics strategies (when to grow, by how much, whether to share) are in the library;
  the compiler does not participate.

`Vec(T)` construction form (two layers: type parameters first, then construction parameters):

```yaoxiang
# Empty construction—length 0, elements appended later
v = Vec(Int)()

# Element construction—length determined by element count
w = Vec(Int)(1, 2, 3)          # length 3

# Slot allocation—allocate n zero-value slots
buf = Vec(Int)(len=64)         # length 64, all elements are zero values
```

> Slot allocation uses **field name form** (`len=`) rather than positional form: a single positional
> integer would be ambiguous with a "single-element vector" (`Vec(Int)(64)` cannot distinguish
> between "length 64" and "containing one element 64"). This is consistent with the unified rules
> for generic construction: field-name-form arguments bind by name and are not affected by
> positional inference.
>
> This is the only primitive needed for `List` growth—`List` allocates new slots and moves elements
> when needed:
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> When to grow, by how much, and whether to shrink are all decided by `List`. `Vec` does not do
> capacity strategy.

From bottom to top, performance decreases and flexibility increases: `Array` > `Vec` > `List`.

> Naming rationale: `Vec`/`vector` refers to runtime-length growable sequences in mainstream
> languages (Rust/C++); `Array` refers to fixed-length.

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core
advantages:

| Feature                 | Traditional Generics (C++/Rust)               | YaoXiang Value-Dependent Types                               |
| ----------------------- | --------------------------------------------- | ------------------------------------------------------------ |
| Type-dependent values   | Only depends on type parameters               | Can depend on any value, including function call results     |
| Compile-time evaluation | C++ template manual specialization, Rust none | Automatic compile-time evaluation with termination guarantee |
| Type-level computation  | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                        |
| Type safety             | C++ none, Rust limited                        | Complete type safety, compile-time checking                  |
| Dimension validation    | Runtime check or manual specialization        | Compile-time dimension validation, no runtime overhead       |

### Type Universe Hierarchy and Value-Dependent Types

The Type Universe thought divides language concepts into different layers based on semantic roles,
with value-dependent types located at the **Type2 layer**:

| Layer     | Role                                                  | Examples                                                                                                        |
| --------- | ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | Values                                                | `42`, `factorial(5)`, functions themselves                                                                      |
| Type0     | Meta-type keyword                                     | `Type`                                                                                                          |
| Type1     | Concrete types                                        | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **Functions/type constructors/value-dependent types** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key design**: Functions, type constructors, and value-dependent types at the Type2 layer share
**unified syntax**, all in the form of `(params) -> result`:

- Regular function: `(Int, Int) -> Int` → return value is a value
- Type constructor: `(T: Type) -> Type` → return value is a type
- Value-dependent type: `(T: Type, N: Int) -> Type` → return value is a type, and depends on the
  value parameter N

> **Curry-Howard Isomorphism**: This unification is no coincidence. The Curry-Howard Isomorphism
> states that "types are propositions, programs are proofs"—the function type `A → B` corresponds to
> the logical implication "if A then B", the generic `(T: Type) -> Type` corresponds to universal
> quantification "for all types T", and the value-dependent type `(n: Int) -> Type` corresponds to
> "for every integer n there exists a type". YaoXiang unifies functions, type constructors, and
> value-dependent types at the Type2 layer, essentially unifying "proof" and "computation" into a
> single concept—**constructive proof**. This is the direct embodiment of the Curry-Howard
> Isomorphism in language design: one form (`(params) -> result`) simultaneously carries logical
> propositions and computational processes.

### Compile-Time Determinism Guarantees

YaoXiang's Type Universe thought requires: **everything at the Type level is determined at compile
time**.

```yaoxiang
# Compile-time dimension validation example
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

1. Detects function calls in type positions
2. Performs compile-time termination check on the function (see termination check mechanism below)
3. Executes evaluation at compile time
4. Embeds the result into the generated type

### Application Scenarios of Value-Dependent Types

#### Compile-Time Dimension Validation

```yaoxiang
# Matrix multiplication: compile-time dimension match validation
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

#### Compile-Time Coverage Goal for Boundary Failures

> **Implementation status note**: Container types have been de-specialized—`Array(T, N)` is a const
> generic constructor; literal context landing points and `in` membership predicates are in place.
> `Array(T, N)` literal landing point's N and element type have been enforced by compile-time
> validation (E1002), **N is trustworthy**—this section's mechanism can be built on the foundation
> of "annotation N == runtime length". Currently `[]` index out-of-bounds (E6003) and Dict missing
> key (E6008) are in a **runtime error transitional state**; This section's value-dependent types
> are the target mechanism to push these boundary failures to **compile time**:
>
> - const index: `a[5]` (5 is a compile-time constant) when `a: Array(Int, 3)` is directly rejected
>   at compile time;
> - value index: `a[i]` requires precondition `i < len(a)`, proved by value-dependent type contract;
> - `in` predicate is the basis of Hoare logic preconditions: `n in 1..10`, `x in some_set` are all
>   propositions provable at compile time.
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

# Completely transparent at use, type auto-inferred
numbers = List(Int)()   # Correction: value construction two-layer form (see §9.1); elements use push to fill
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                                     | C++ Templates          | Rust Generics | Haskell GADT   | **YaoXiang**                              |
| ----------------------------------------------------------- | ---------------------- | ------------- | -------------- | ----------------------------------------- |
| Type parameters                                             | ✅                     | ✅            | ✅             | ✅                                        |
| Value-dependent types                                       | ❌                     | ❌            | ✅             | ✅                                        |
| Compile-time evaluation                                     | Template instantiation | ❌            | ✅             | ✅                                        |
| Termination guarantee                                       | ❌                     | ❌            | ❌ (dangerous) | ✅ (automatic metric synthesis, RFC-027)  |
| Type safety                                                 | ❌ (macro expansion)   | ✅            | ✅             | ✅                                        |
| Unified syntax                                              | ❌                     | ❌            | ❌             | ✅                                        |
| Compile-time dimension validation                           | Manual specialization  | Runtime check | Type families  | Compile-time automatic validation         |
| Semi-automatic termination annotation (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (only fully automatic at compile time) |

### Termination Check Mechanism (Unified with RFC-027)

Compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the type
system will fall into infinite loops. Termination checks are performed **fully automatically** by
the compile-time proof pipeline of RFC-027—the compiler automatically synthesizes metrics, and
recursions/loops that can be proven pass through, those that cannot be proven directly report
compile errors. **No leeway for semi-automatic annotations**: RFC-022's `//! decreases`,
`/*! invariant !*/` have been deprecated along with RFC-022, the contract is the type annotation
itself.

#### Termination Check for Recursive Functions

Before compile-time evaluation, the compiler checks whether the parameters of recursive calls
strictly decrease on each recursive path (RFC-027 §6.7). No contract comment is needed:

```yaoxiang
# Compile-time factorial: no //! requires/ensures/decreases, compiler auto-analyzes
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Use: called in type position, compiler verifies termination before evaluation
arr: Array(Int, factorial(5)) = Array(Int, 120)()  # Compile-time evaluation factorial(5) = 120
```

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| Not decreasing / cannot determine decrease            | Compile error               |
| Runtime call (non-type position)                      | No termination check needed |

#### Termination Check for Loops

Loops do not need `: Invariant(...)` or `: decreases(...)` annotations. Refined type annotations on
variables (such as `UpTo(n)`) simultaneously provide loop invariants and metric bounds. The compiler
attempts four metric synthesis strategies in priority order, stopping when one is found (RFC-027
§7):

1. **Automatic linear rank function synthesis**—extract variable bounds from type annotations,
   enumerate linear combinations, SMT verifies m ≥ 0 and m' < m for all paths
2. **Predicate violation count** (experimental)—extract violation_count from target type definitions
   (such as `Sorted`), covering adjacent swaps/moves
3. **Bounded increase/decrease patterns**—`v += const` → metric `upper - v` (degeneration of
   strategy 1, fastest path)
4. **Multiplicative scaling metric template**—`v *= const` (const > 1) → metric
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # Type annotation gives upper bound arr.len and lower bound 0
    while i < arr.len {
        # Compiler auto-derives: metric arr.len - i, strictly decreases by 1 each iteration → termination proven
        s += arr[i]; i += 1
    }
    return s
}
```

#### Workflow of Termination Check

```
┌─────────────────────────────────────────────────────────────┐
│  Type checking phase                                        │
│  Encounters function call in type position (e.g., factorial(5))│
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination check (RFC-027 proof pipeline, fully automatic)│
│     - Recursive functions: check whether parameters strictly│
│       decrease on each recursive path                       │
│     - Loops: four metric synthesis strategies (linear rank/│
│       violation count/bounded pattern/multiplicative scaling),│
│       SMT verifies decrease                                 │
│     - Cannot prove → compile error (hard boundary, no       │
│       semi-automatic annotation fallback)                  │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time evaluation (executed by built-in interpreter)│
│     - Pure functions: direct evaluation                     │
│     - Side effects: compile error (type position must be side-effect-free)│
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result embedded in type                                 │
│     - Array(Int, factorial(5)) → Array(Int, 120)            │
│     - Matrix(Float, 3, 3) → concrete type                   │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensures that compile-time evaluation must terminate, avoiding infinite loops in the
  type system
- **Uniformity**: Termination check shares the same compile-time proof pipeline as correctness
  verification (VC generation) (RFC-027), no separate contract syntax
- **Fully automatic**: Compiler auto-synthesizes metrics from type annotations, passes if provable,
  errors if not—no dependency on programmers writing `decreases`

## Motivation

### Why Do We Need a Strong Generic System?

Current mainstream languages have limitations in their generics:

| Language     | Generic Capability        | Problem                                                                                  |
| ------------ | ------------------------- | ---------------------------------------------------------------------------------------- |
| Java         | Bounded types             | Compile-time monomorphization, no generic specialization                                 |
| C#           | Generic constraints       | Runtime type checking, performance overhead                                              |
| Rust         | Generics + Trait          | Complex Trait system, steep learning curve                                               |
| C++          | Templates                 | Complex template specialization, poor compile error messages                             |
| **YaoXiang** | **Value-Dependent Types** | **Types can depend on values, compile-time dimension validation, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: Runtime flexibility vs compile-time optimization
2. **Complex vs Concise**: Powerful type system vs ease of use
3. **Macros vs Generics**: Macro code generation vs generic type safety
4. **Value-Dependent vs Type Safety**: Traditional generics cannot validate dimensions at compile
   time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                |
| --------------------------- | ---------------------------------------------------------------------------------------------------------- |
| **Type depends on value**   | `Array: (T: Type, N: Int) -> Type` lets the type depend on specific values                                 |
| **Compile-time evaluation** | Function calls in type positions are evaluated at compile time, results directly embedded in the type      |
| **Dimension validation**    | `Matrix(Float, 3, 3)` validates matrix dimensions at compile time                                          |
| **Type-level computation**  | `If`, `Match` and other conditional types support type-level computation                                   |
| **Termination guarantee**   | Compile-time termination check (automatic metric synthesis) ensures compile-time evaluation must terminate |

```yaoxiang
# What C++/Rust cannot do: compile-time validation
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

1. **Zero-cost abstraction** - Generic calls equivalent to concrete type calls
2. **Dead code elimination** - Compile-time analysis, only instantiate used generics
3. **Macro replacement** - Generics replace 90% of macro usage scenarios
4. **Type safety** - Compile-time checking, no runtime type overhead
5. **IDE-friendly** - Smart hints, clear error messages
6. **Value-dependent types** - Types can depend on values, supporting compile-time dimension
   validation
7. **Compile-time evaluation safety** - Through compile-time termination check (RFC-027 automatic
   metric synthesis) to ensure compile-time evaluation terminates

### Design Principles

- **Compile-time determination**: Generic parameters determined at compile time
- **Monomorphization first**: Generate concrete code, avoid virtual function calls
- **Constraint-driven**: Type constraints guide instantiation
- **Platform optimization**: Specialization supports platform-specific optimizations
- **Type Universe unification**: Functions/type constructors/value-dependent types unified at Type2
  layer
- **Termination guarantee**: Function calls in type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise they will
> be inferred by HM as functions.
>
> | Notation                          | Meaning                              |
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

`Type` is a language-level special existence. The compiler can naturally identify the `Type`
position in the signature and automatically infer and fill it from the actual parameter types.

```yaoxiang
# Compiler auto-inferring generic parameters
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

# Use points
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

# Using constraints: directly declare type constraints in signatures
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
````

#### 2.2 Multiple Constraints

```yaoxiang
# Multiple constraints syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# Generic container sorting
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
doubled: Vec(Int) = map(Vec(1, 2, 3), (x: Int) => x * 2)  # Compiler inference
```

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

#### 2.4 Built-in Marker Traits: Dup and Clone

**Three types of copy semantics**:

| Type                     | Meaning                                                                   | Trigger Method                      | Applicable Scenario               |
| ------------------------ | ------------------------------------------------------------------------- | ----------------------------------- | --------------------------------- |
| **Primitive value copy** | Automatic value copy on assignment, two values are completely independent | Assignment/parameter pass automatic | Int, Float, Bool, Char            |
| **Dup**                  | Shallow copy: copy handle/token, underlying data is shared                | Assignment/parameter pass automatic | `&T` token, `ref T`, String/Bytes |
| **Clone**                | Deep copy: create complete independent copy                               | `value.clone()`                     | Any type that implements Clone    |

**Dup semantics**: Types implementing Dup do not transfer ownership on assignment/parameter
passing—the compiler copies the handle/token, and multiple holders point to the same underlying
data. This complements the default Move semantics in RFC-009's ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = copy handle, share data (modifications affect each other)
Clone = copy data, copy is independent (modifications don't affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not belonging to Dup
2. Dup  — only applies to reference/token types and internal reference-counted types
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which types are Dup**:

| Type                     | Dup     | Reason                                                                |
| ------------------------ | ------- | --------------------------------------------------------------------- |
| `&T` (borrow token)      | ✅      | Zero-size token, copying token = multiple views pointing to same data |
| `ref T`                  | ✅      | Rc/Arc copy = reference count+1, share heap data                      |
| String, Bytes            | ✅      | Internal reference counting, copy handle shares underlying buffer     |
| `&mut T` (mutable token) | ❌      | Linear exclusive, cannot copy                                         |
| struct                   | Derived | All fields Dup → struct Dup                                           |
| enum                     | Derived | All fields of all variants Dup → enum Dup                             |
| tuple                    | Derived | All elements Dup → tuple Dup                                          |
| Fn (closure)             | ❌      | Captured environment may not be Dup                                   |
| `*T` (raw pointer)       | ❌      | unsafe, not participating in ownership system                         |

**Int/Float/Bool/Char are not Dup**—they are value types. On assignment, the compiler automatically
performs value copy (two values are completely independent). This is not "shallow copy", but a
built-in compiler handling of primitives, and should not be expressed through the Dup type
attribute.

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

# Generic constraint
dup_use: (T: Dup) -> T = x         # T: Dup → can shallow copy
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can deep copy
```

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety guarantees are handled
> fully automatically by the `ref` keyword and compiler—`ref` automatically chooses Rc or Arc, users
> don't need to understand Send/Sync.

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
# Iteration position is carried by wrapper record (Vec itself is a raw buffer, no index field)
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

# Use
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. Compile-Time Generics

#### 4.1 Compile-Time Value Parameters

> **Correction**: The original text stated "value parameters like `Int` are determinable at compile
> time by default in generic contexts", which is **strictly
> incorrect**—`add: (a: Int, b: Int) -> Int = a + b` has `a`/`b` as runtime value parameters. Only
> concrete type parameters **referenced in type positions** are compile-time value parameters. The
> correct definition is below.

**Core design**: Generic signatures use `Type` to mark type parameters; parameters annotated with
concrete types (like `Int`/`Bool`/`Float`) are listed as **compile-time value parameter
candidates**. Whether they become compile-time value parameters depends on whether their values are
**referenced in type positions** (value-dependent). No `const` keyword is needed.

**Determination rules (two steps)**:

1. **Form coarse screening**: Parameters annotated with non-`Type` concrete types (such as `Int`) →
   listed as candidates.
2. **Use fine screening**: The candidate name appears in a **type position** (type body field type,
   inner `Fn` parameter type, `Assert` predicate, `Array(T, N)` type construction argument position,
   etc.) → confirmed as a compile-time value parameter; otherwise treated as a **runtime value
   parameter**.

| Notation                                                   | Determination                  | Reason                                                                |
| ---------------------------------------------------------- | ------------------------------ | --------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters   | Only appears in value positions, not involved in type construction    |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter | N appears in `Array(T, N)` type construction argument position        |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter | N is the type of inner parameter `k`                                  |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N drops out (see below)        | N not referenced in type body, degenerates to runtime value parameter |

> **Value-dependent essence**: Compile-time value parameters are value-dependent types—only when a
> value is used to **construct a type** is compile-time determination needed. The form (`: Int`)
> only determines candidate qualification; the use (showing up in type position) determines whether
> it is a compile-time value parameter. This is the same judgment basis as "function calls in type
> positions are evaluated at compile time" in §"Compile-Time Determinism Guarantees".

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameter: N is referenced in type position (Measure length slot)
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears in type construction argument position → compile-time value parameter
    length: N,
}

# Usage: factorial(5) evaluated in type position (compile time), result 120 embedded in type
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# Value-dependent: N as the type of inner parameter k
# ════════════════════════════════════════════════════════
# N is a compile-time value parameter (appears in (k: N) type position);
# k is a runtime value parameter, its type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling of dropped candidates**: Candidates annotated with concrete types but not referenced in
> type positions (like the `N` of `Foo` in the table above) degenerate to runtime value parameters
> (function-level path). Dropped candidates on the type constructor path cannot occupy runtime slots
> (type constructors are evaluated at compile time), and the declaration side directly reports error
> [E1094]: "N is declared as a compile-time value parameter but not referenced in the type
> body"—previously silent discarding led to inconsistent instantiation arity.

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation example
# ════════════════════════════════════════════════════════

# Compiler computes function calls on literal types at compile time
SIZE: Int = factorial(5)  # compile-time 120

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

### Never and Void: The ⊥ and ⊤ of the Type System

YaoXiang's type system in the Curry-Howard Isomorphism simultaneously has ⊥ (false/empty type) and ⊤
(true/Unit), carried by the two built-in type names `Never` and `Void`:

**Never (⊥)** — Three non-negotiable core properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. This is a
   meta-level property and must be built-in.
2. **Explosion principle**: `Never <: T` holds for any type `T`. A `Never` value can be used as any
   type—this is exactly why the code after `assert(false)` still passes type checking (though it
   will never be executed).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis.

`Never` is a built-in type name, not a keyword, and the parser is unaware. Empty and type literal
syntax are not opened up.

**Void (⊤, i.e. Unit)** — exactly one inhabitant (default void value), the carrier of the true
proposition "always true". `Void` is the unit of zero-field product types, `Never` is the unit of
zero-variant sum types—the two are dual. `x: Void = <default>` is legal, `x: Never = ...` has no
right-hand side to write.

#### 4.3 Compile-Time Validation (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: using conditional types
# ════════════════════════════════════════════════════════

# Standard library definitions
# IsTrue: bridge from value universe to type universe—Bool true value mapped to type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has value, program continues
    false => Never,    # ⊥, no value, diverges
}

# Assert: compile-time refined type primitive—type-level expression of a Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error/divergence)
# cond cannot be decided → proof pipeline decides by dispatch pattern:
#                  CompileTime → Unknown, requires prove
#                  Runtime     → insert check, inject Γ hypothesis

# Usage 1: as constraint in type definition
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert in type position)
    length: Assert(N > 0),
}

# Usage 2: in expressions
IntArray: (N: Int) -> Type = Array(Int, N)
# Validation: size of IntArray(10) equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generic Specialization

```yaoxiang
# Small array optimization: use function overloading for compile-time generic specialization

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

> **Curry-Howard Isomorphism**: From the perspective of Curry-Howard, conditional types are **case
> analysis** in logic. The `Bool` type corresponds to a proposition with two possible values
> (True/False), and `If` chooses different results based on the truth of that proposition—this is
> exactly case disjunction in logic. `match C { True => T, False => E }` is essentially expressing:
> "knowing proposition C is True, the conclusion is T; knowing C is False, the conclusion is E".

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

# Compile-time validation (unified with §4.3's Assert definition)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Use
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Families

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but **constructing a proposition about the addition of natural numbers**. `(Zero, B) => B` says
> "proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` says "if
> Add(A', B) holds, then Add(Succ(A'), B) also holds". This is exactly the definition of addition in
> Peano axioms. The type checker verifies that this match expression passes, equivalent to verifying
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

# Type-level addition (Curry-Howard: case analysis + recursive call, needs termination check for complete induction)
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
    # Compiled into more efficient code
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

# Generic implementation (compiler auto-selects the best)
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

# Compiler auto-selects the best specialization
sum(int_arr)     # Selects sum: (Vec(Int)) -> Int
sum(float_arr)    # Selects sum: (Vec(Float)) -> Float
```

#### 6.3 Perfect Combination of Function Overloading and Inlining

**Key feature**: Function overloading naturally combines with inline optimization, achieving
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
# Compiler auto-selects the best specialization, then inlines
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

2. **Inline optimization**
   - Small functions auto-inlined to call site
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
   # No need for new keywords like impl, where
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
fibonacci(10)      # Selects Int version, fully inlined
fibonacci(10.5)    # Selects Float version, uses Binet's formula
```

**What does this mean?**

- ✅ **Generic specialization** → Function overloading naturally solves it
- ✅ **Performance optimization** → Inlining automatically completed
- ✅ **Code reuse** → One function name, multiple implementations
- ✅ **Zero-cost abstraction** → Compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords** → Perfectly fits RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internal: build generic instantiation dependency graph
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
doubled = map(int_list, (x) => x * 2)  # Needs map[Int, Int]

# Use point 2: instantiate map(String, String)
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Needs map[String, String]

# Unused: map[Float, Float] etc.
# These generic instances will not be generated

# Compiled only contains used instances
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
# Correction: earlier version wrote Array(Int, 10)(1, 2, 3, ...) (elements spread out),
# inconsistent with the authoritative pattern in §9.3 (Type(parameters)(field construction parameters)/empty construction),
# unified to field name form construction parameters, see SPEC type-system.md §4.3.
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
# - Compiled binary only contains these two instances
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

    // 6. Run optimization
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
# Using function overloading for auto-derivation
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# Use
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # Auto-generated call
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

#### 8.3 Type-Level Programming Replacement

```yaoxiang
# ❌ Macro solution: type-level computation
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ Generic solution: conditional types
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
# Function definitions are under the List namespace (List. prefix = namespace affiliation)
# To make . call syntax like list.push(item) effective, need explicit binding: List.push = push[0]
# self is just a convention parameter name, the compiler looks at the type not the name

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
# Create generic List
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

# ======== 3. Usage example ========
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

    # Compile-time dimension validation: using Assert standard library type
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
# Create compile-time known-size matrix
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
   - Dimension validation and other features
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compile time**
   - Generic instantiation increases compile time
   - Constraint solving may be slow

2. **Memory usage**
   - Compiler memory usage increases
   - Caching mechanism needs memory

3. **Implementation complexity**
   - Constraint solver is complex
   - Type-level computation engine is complex

4. **Error diagnosis**
   - Generic errors can be complex
   - Clear error hints needed

### Mitigation Measures

1. **Caching strategy**
   - Instantiation result caching
   - LRU cache limits memory

2. **Incremental compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error hints**
   - Clear error messages
   - Generic parameter inference hints

4. **Parallel compilation**
   - Parallel generic instantiation
   - Multi-threaded constraint solving

## Alternatives

| Alternative                | Why Not Chosen                      |
| -------------------------- | ----------------------------------- |
| Basic generics only        | Cannot replace complex macros       |
| Pure macro system          | No type safety, poor error messages |
| Constraint-only dependency | Insufficient flexibility            |
| Runtime generics           | Has performance overhead            |

### Risks

| Risk                          | Impact                     | Mitigation Measures           |
| ----------------------------- | -------------------------- | ----------------------------- |
| Constraint solving complexity | Long compile time          | Incremental solving + caching |
| Code bloat                    | Binary file too large      | DCE + threshold control       |
| Implementation complexity     | Extended development cycle | Phased implementation         |
| Error diagnosis               | Poor user experience       | Detailed error messages       |

## Open Issues

### Issues to be Resolved

| Topic                  | Description                        | Status     |
| ---------------------- | ---------------------------------- | ---------- |
| Instantiation strategy | Eager vs Lazy vs Threshold         | To discuss |
| Cache size             | LRU cache capacity setting         | To discuss |
| Error diagnosis        | Generic error message detail level | To discuss |

### Future Optimizations

| Optimization Item             | Value  | Implementation Difficulty |
| ----------------------------- | ------ | ------------------------- |
| Instantiation graph analysis  | High   | Medium                    |
| Type-level programming DSL    | Medium | High                      |
| Generic performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use unified () syntax as part of function type
# e.g. map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# Type constraint (in generic parameters)
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

# Type in generic parameters is auto-filled by compiler from argument types
# e.g. map(numbers, f), T extracted from numbers: List(Int), R extracted from f: (Int) -> String
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
│ (Official Design)│  │ (Original Location)│
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documentation

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./accepted/009-ownership-model.md)
- [RFC-001: Concurrency Model](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)
- [tutorial/](../../../../../tutorial/)

### External References

- [Rust Generic System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
