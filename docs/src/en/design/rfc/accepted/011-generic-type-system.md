---
title: 'RFC-011: Generics System Design - Zero-cost Abstraction and Macro Replacement'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-15 (Type body code blocks + Compile-time contracts + Effect seeds implemented)'
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

# RFC-011: Generics System Design - Zero-cost Abstraction and Macro Replacement

## Abstract

This document defines YaoXiang's **generics system design**, achieving zero-cost abstraction through
powerful generics capabilities, reducing dependence on macros via compile-time optimization, and
providing dead code elimination mechanisms.

**Core Design**:

- **Unified Signature Syntax**: `(T: Type, R: Type) -> ...` unifies generics parameters with regular
  parameters
- **Type Self-Description Mechanism**: `Type` is a language-level special entity; positions in
  signatures marked `Type` are automatically inferred and filled
- **Type Constraints**: `T: Dup + Add` for multiple constraints, function type constraints
- **Associated Types**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time Generics**: `N: Int` generics value parameters, compile-time constant instantiation
- **Conditional Types**: `If: (C: Bool, T: Type, E: Type) -> Type` for type-level computation and
  type families

**Value**:

- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replaces 90% of macro usage scenarios
- Type safety: compile-time checking, IDE-friendly
- **Explicit is better than implicit**: `Type` self-description, compiler auto-inference

## Reference Documents

This document's design is based on the following documents:

| Document                                                                                                   | Relationship              | Description                                                                       |
| ---------------------------------------------------------------------------------------------------------- | ------------------------- | --------------------------------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax Foundation**     | Generics syntax integrated with unified `name: type = value` model                |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call Syntax**           | Section 6: generics call syntax—unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md)                                              | **Type System**           | Natural combination of Move semantics and generics                                |
| [RFC-024: spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution Model**       | DAG analysis and generics type checking                                           |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)                                      | **Compiler Architecture** | Generics monomorphization and compile-time optimization strategies                |
| [Type Universe Concept](../reference/plan/ongoing/类型宇宙思想.md)                                         | **Theoretical Core**      | Type universe hierarchy model and value-dependent type design                     |
| [RFC-027: Compile-time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination Check**     | Automatic metric synthesis and compile-time evaluation safety guarantee           |

## Type Universe Concept and Value-Dependent Types

YaoXiang's generics system is built on the **Type Universe concept**, a mental model that unifies
all language concepts into a hierarchical structure. The core innovation is elevating
**value-dependent types** to first-class citizens at the Type2 level.

### What are Value-Dependent Types?

**Value-dependent types** are types that depend on one or more **values** (rather than only on other
types). These values can be evaluated at compile time, providing type safety guarantees during the
compile phase.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Vec: (n: Int) -> Type  # Vector type depends on length value n
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column counts
```

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types offer the following core
advantages:

| Feature                 | Traditional Generics (C++/Rust)               | YaoXiang Value-Dependent Types                               |
| ----------------------- | --------------------------------------------- | ------------------------------------------------------------ |
| Values types depend on  | Only on type parameters                       | Can depend on any value, including function call results     |
| Compile-time evaluation | C++ template manual specialization, Rust none | Automatic compile-time evaluation with termination guarantee |
| Type-level computation  | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                        |
| Type safety             | C++ none, Rust limited                        | Complete type safety, compile-time checking                  |
| Dimension validation    | Runtime check or manual specialization        | Compile-time dimension validation, no runtime overhead       |

### Type Universe Hierarchy and Value-Dependent Types

The Type Universe concept divides language concepts into different hierarchies by semantic role,
with value-dependent types located at the **Type2 level**:

| Level     | Role                                                  | Examples                                                                                             |
| --------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Type-1    | Values                                                | `42`, `factorial(5)`, functions themselves                                                           |
| Type0     | Meta-type keywords                                    | `Type`                                                                                               |
| Type1     | Concrete types                                        | `Int`, `String`, `Vec(3)`                                                                            |
| **Type2** | **Functions/type constructors/value-dependent types** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at the Type2 level share
**unified syntax**, all in the form `(params) -> result`:

- Regular function: `(Int, Int) -> Int` → return value is a value
- Type constructor: `(T: Type) -> Type` → return value is a type
- Value-dependent type: `(n: Int) -> Type` → return value is a type, but depends on value parameters

> **Curry-Howard Isomorphism**: This unification is no coincidence. Curry-Howard Isomorphism states
> "types as propositions, programs as proofs"—the function type `A → B` corresponds to logical
> implication "if A then B", generics `(T: Type) -> Type` corresponds to universal quantification
> "for all types T", value-dependent types `(n: Int) -> Type` corresponds to "for each integer n
> there exists a type". YaoXiang unifies functions, type constructors, and value-dependent types at
> the Type2 level, which essentially unifies "proof" and "computation" into the same
> concept—**constructive proof**. This is precisely the direct embodiment of Curry-Howard
> Isomorphism in language design: one form (`(params) -> result`) simultaneously carries logical
> propositions and computational processes.

### Compile-time Determinism Guarantee

YaoXiang's Type Universe concept requires: **Everything at the Type level is compile-time
determined**.

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

# Compile-time computation: factorial(3) = 6, vector size determined at compile time
vec: Vec(factorial(3)) = Vec(6)()
```

The compiler automatically:

1. Detects function calls at type positions
2. Performs compile-time termination check on functions (see termination check mechanism below)
3. Executes evaluation at compile time
4. Embeds the result into the generated type

### Application Scenarios of Value-Dependent Types

#### Compile-time Dimension Validation

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

# Error caught at compile time:
# multiply(matrix_2x3, matrix_4x2)  # Compile error: 2 != 4
```

#### Type-safe Array Sizes

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

#### Compile-time Coverage Goals for Boundary Failures

> **Implementation Status Note**: Container types have been de-specialized—`Array(T, N)` is a const
> generics constructor, with literal context landing points and `in` membership predicates
> implemented. The N and element type of `Array(T, N)` literal landing points are now enforced by
> compile-time checks (E1002), **N is trustworthy**—this section's mechanism can be built on the
> contract "annotation N == runtime length". The current `[]` index out-of-bounds (E6003) and Dict
> missing key (E6008) are in a **runtime error transitional state**; this section's value-dependent
> types are the target mechanism to push these boundary failures to **compile time**:
>
> - const indexing: `a[5]` (5 is a compile-time constant) when `a: Array(Int, 3)` is directly
>   rejected at compile time;
> - value indexing: `a[i]` requires precondition `i < len(a)`, proved by value-dependent type
>   contracts;
> - `in` predicate is the foundation of Hoare logic preconditions: `n in 1..10`, `x in some_set` are
>   all compile-time provable propositions.
>
> The complete design of refinement types will supplement this section when implemented.

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

# Usage is completely transparent, types auto-inferred
numbers = List(Int)()   # Errata: two-layer value construction form (see §9.1); elements filled via push
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                                      | C++ Templates          | Rust Generics | Haskell GADT   | **YaoXiang**                             |
| ------------------------------------------------------------ | ---------------------- | ------------- | -------------- | ---------------------------------------- |
| Type parameters                                              | ✅                     | ✅            | ✅             | ✅                                       |
| Value-dependent types                                        | ❌                     | ❌            | ✅             | ✅                                       |
| Compile-time evaluation                                      | Template instantiation | ❌            | ✅             | ✅                                       |
| Termination guarantee                                        | ❌                     | ❌            | ❌ (dangerous) | ✅ (automatic metric synthesis, RFC-027) |
| Type safety                                                  | ❌ (macro expansion)   | ✅            | ✅             | ✅                                       |
| Unified syntax                                               | ❌                     | ❌            | ❌             | ✅                                       |
| Compile-time dimension validation                            | Manual specialization  | Runtime check | Type families  | Automatic compile-time validation        |
| Semi-automatic termination annotations (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (compile-time fully automatic only)   |

### Termination Check Mechanism (Unified with RFC-027)

Compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the type
system will fall into infinite loops. Termination checking is performed **fully automatically** by
RFC-027's compile-time proof pipeline—the compiler automatically synthesizes metrics, recursive/loop
constructs that can be proven pass, those that cannot be proven directly report compile errors. **No
room left for semi-automatic annotations**: RFC-022's `//! decreases`, `/*! invariant !*/` have been
deprecated along with RFC-022, the specification is the type annotation itself.

#### Termination Check for Recursive Functions

Before compile-time evaluation, the compiler checks whether the arguments of recursive calls
strictly decrease on every recursive path (RFC-027 §6.7). No specification comments required:

```yaoxiang
# Compile-time factorial: no //! requires/ensures/decreases, compiler auto-analyzes
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Usage: at type position, compiler verifies termination before evaluation
vec: Vec(factorial(5)) = Vec(120)()  # Compile-time evaluation of factorial(5) = 120
```

| Scenario                                              | Behavior                      |
| ----------------------------------------------------- | ----------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation       |
| Not decreasing / cannot determine decrease            | Compile error                 |
| Runtime call (non-type position)                      | No termination check required |

#### Termination Check for Loops

Loops do not require `: Invariant(...)` or `: decreases(...)` annotations. Refinement type
annotations on variables (e.g., `UpTo(n)`) simultaneously provide loop invariants and metric bounds.
The compiler tries four metric synthesis strategies by priority, stopping once one is found (RFC-027
§7):

1. **Automatic linear rank function synthesis**—extract variable bounds from type annotations,
   enumerate linear combinations, SMT verifies m ≥ 0 and all paths m' < m
2. **Violation count** (experimental)—extract violation_count from target type definitions (e.g.,
   `Sorted`), cover adjacent swap/move
3. **Bounded increase/decrease pattern**—`v += const` → metric `upper - v` (degenerate case of
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

#### Termination Check Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  Type Checking Phase                                        │
│  Encountering function calls at type positions              │
│  (e.g., Vec(factorial(5)))                                  │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination Check (RFC-027 proof pipeline, fully auto)  │
│     - Recursive functions: check arguments strictly decrease│
│       on every recursive path                               │
│     - Loops: four metric synthesis strategies (linear rank/ │
│       violation count/bounded pattern/                       │
│       multiplicative scaling), SMT verifies decrease        │
│     - Cannot prove → compile error (hard boundary, no       │
│       semi-automatic annotation fallback)                   │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time Evaluation (executed by built-in interpreter)│
│     - Pure functions: direct evaluation                     │
│     - Side effects: compile error (type position must be    │
│       side-effect free)                                     │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result Embedded into Type                               │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → concrete type                   │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensures compile-time evaluation necessarily terminates, preventing the type system
  from falling into infinite loops
- **Unification**: Termination check and correctness verification (VC generation) share the same
  compile-time proof pipeline (RFC-027), no independent specification syntax
- **Fully Automatic**: Compiler automatically synthesizes metrics from type annotations—passes if
  provable, errors if not—no reliance on programmer-written `decreases`

## Motivation

### Why do we need a strong generics system?

Current mainstream languages have limitations in their generics:

| Language     | Generics Capability       | Problem                                                                                  |
| ------------ | ------------------------- | ---------------------------------------------------------------------------------------- |
| Java         | Bounded types             | Compile-time monomorphization, no generics specialization                                |
| C#           | Generics constraints      | Runtime type checking, performance overhead                                              |
| Rust         | Generics + Trait          | Trait system complex, steep learning curve                                               |
| C++          | Templates                 | Template specialization complex, poor compile error messages                             |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, compile-time dimension validation, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: Runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: Powerful type system vs ease of use
3. **Macros vs Generics**: Macro code generation vs generics type safety
4. **Value Dependence vs Type Safety**: Traditional generics cannot validate dimensions at compile
   time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                        |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Types depend on values**  | `Vec: (n: Int) -> Type` makes types depend on specific values                                                      |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, results directly embedded in types                 |
| **Dimension validation**    | `Matrix(Float, 3, 3)` validates matrix dimensions at compile time                                                  |
| **Type-level computation**  | Conditional types like `If`, `Match` support type-level computation                                                |
| **Termination guarantee**   | Compile-time termination check (automatic metric synthesis) ensures compile-time evaluation necessarily terminates |

```yaoxiang
# Compile-time validation impossible in C++/Rust
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# Compile-time computation: factorial(3) = 6, factorial(2) = 2
# Type is Matrix(Float, 6, 2)

# Dimension mismatch caught at compile time
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # Compile error: 2 != 3
```

### Value of the Generics System

```yaoxiang
# Example: Unified API design
# map operation on different container types

# Traditional approach: separate implementation per type
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# Generics approach: one generic function covers all types
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
3. **Macro replacement** - generics replaces 90% of macro usage scenarios
4. **Type safety** - compile-time checking, no runtime type overhead
5. **IDE-friendly** - smart hints, clear error messages
6. **Value-dependent types** - types can depend on values, supporting compile-time dimension
   validation
7. **Compile-time evaluation safety** - guaranteed by compile-time termination check (RFC-027
   automatic metric synthesis)

### Design Principles

- **Compile-time determinism**: generics parameters determined at compile time
- **Monomorphization first**: generate concrete code, avoid virtual function calls
- **Constraint-driven**: type constraints guide instantiation
- **Platform optimization**: specialization supports platform-specific optimization
- **Type universe unification**: functions/type constructors/value-dependent types unified at Type2
  level
- **Termination guarantee**: function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key Rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise they will
> be inferred as functions by HM.
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
    data: Array(T),
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
# Generic function uses unified (T: Type, R: Type) signature syntax
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type Self-Description Mechanism

`Type` is a language-level special entity. The compiler naturally recognizes `Type` positions in
signatures and automatically infers and fills them from actual argument types.

```yaoxiang
# Compiler auto-infers generics parameters
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         type decl   construction call: Int fills T, () value construction

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
# Multiple constraints syntax
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# Generic container sort
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # Implement sort algorithm
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# Function type constraint
map: (T: Type, R: FnMut(T))(array: Array(T), f: R) -> Array(R) = {
    result: Array(R) = Array()
    for item in array {
        result.push(f(item))
    }
    return result
}

# Usage
doubled: Array(Int) = map(Array(1, 2, 3), (x: Int) => x * 2)  # Compiler inference
```

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

#### 2.4 Built-in Marker Traits: Dup and Clone

**Three Types of Copy Semantics**:

| Type                     | Meaning                                                       | Trigger Method                       | Applicable Scenarios               |
| ------------------------ | ------------------------------------------------------------- | ------------------------------------ | ---------------------------------- |
| **Primitive value copy** | Assignment auto value copy, two values completely independent | Auto on assignment/parameter passing | Int, Float, Bool, Char             |
| **Dup**                  | Shallow copy: copy handle/token, underlying data shared       | Auto on assignment/parameter passing | `&T` tokens, `ref T`, String/Bytes |
| **Clone**                | Deep copy: create complete independent replica                | `value.clone()`                      | Any type implementing Clone        |

**Dup Semantics**: Types implementing Dup do not transfer ownership on assignment/parameter
passing—the compiler copies the handle/token, with multiple holders pointing to the same underlying
data. This complements the default Move semantics in RFC-009's ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = copy handle, share data (modifications affect each other)
Clone = copy data, replicas independent (modifications do not affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not belonging to Dup
2. Dup — only applies to reference/token types and internally reference-counted types
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which Types are Dup**:

| Type                     | Dup     | Reason                                                             |
| ------------------------ | ------- | ------------------------------------------------------------------ |
| `&T` (borrow token)      | ✅      | Zero-size token, copy token = multiple views pointing to same data |
| `ref T`                  | ✅      | Rc/Arc copy = reference count +1, share heap data                  |
| String, Bytes            | ✅      | Internal reference counting, copy handle shares underlying buffer  |
| `&mut T` (mutable token) | ❌      | Linear exclusive, cannot copy                                      |
| struct                   | Derived | All fields Dup → struct Dup                                        |
| enum                     | Derived | All variants' all fields Dup → enum Dup                            |
| tuple                    | Derived | All elements Dup → tuple Dup                                       |
| Fn (closure)             | ❌      | Captured environment may not be Dup                                |
| `*T` (raw pointer)       | ❌      | unsafe, not participating in ownership system                      |

**Int/Float/Bool/Char are not Dup**—they are value types, with the compiler automatically performing
value copy on assignment (two values completely independent). This is not "shallow copy", but the
compiler's built-in handling of primitives, which does not require and should not be expressed
through the Dup type attribute.

```yaoxiang
# Primitive value types: compiler auto value copy (not Dup)
x: Int = 42
y = x          # Value copy, x and y completely independent
print(x)       # ✅

# Dup: shallow copy, copy handle shares data
view: &Point = &point
view2 = view    # ✅ Dup: copy token, both point to the same point
print(view.x)   # ✅

# Clone: explicit deep copy, create independent replica
backup = big_struct.clone()  # Explicit call

# Generics constraint
dup_use: (T: Dup) -> T = x         # T: Dup → can shallow copy
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can deep copy
```

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety guarantees are handled
> fully automatically by the `ref` keyword and compiler—`ref` automatically selects Rc or Arc, users
> don't need to understand Send/Sync.

### 3. Associated Types

#### 3.1 Associated Type Definitions

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

# Array's Iterator implementation
# Using method syntax sugar: Array.Item, Array.next, Array.has_next
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

### 4. Compile-time Generics

#### 4.1 Compile-time Value Parameters

> **Errata**: The original text stated "value parameters like `Int` are compile-time determinable by
> default in generics context", this statement is **strictly incorrect**—`a`/`b` in
> `add: (a: Int, b: Int) -> Int = a + b` are runtime value parameters. Only specific type parameters
> **referenced at type positions** are compile-time value parameters. See below for the correct
> definition.

**Core Design**: `Type` markers in generics signatures mark type parameters; parameters annotated
with concrete types (e.g., `Int`/`Bool`/`Float`) are listed as **compile-time value parameter
candidates**, whether they become compile-time value parameters depends on whether their values are
**referenced at type positions** (value-dependence). No `const` keyword required.

**Decision Rule (Two Steps)**:

1. **Form Rough Screening**: Parameter annotated with non-`Type` concrete type (e.g., `Int`) →
   listed as candidate.
2. **Use Precise Screening**: Candidate name appears at **type positions** (type body field types,
   inner `Fn` parameter types, `Assert` predicates, type constructor argument positions like
   `Array(T, N)`) → confirmed as compile-time value parameter; otherwise considered **runtime value
   parameter**.

| Notation                                                   | Decision                    | Reason                                                                 |
| ---------------------------------------------------------- | --------------------------- | ---------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value params    | Only appear at value positions, not participating in type construction |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value param  | N appears at `Array(T, N)` type constructor argument position          |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value param  | N serves as type of inner parameter `k`                                |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through (see below) | N not referenced in type body, degrades to runtime value param         |

> **Value-Dependence Essence**: Compile-time value parameters are value-dependent types—only when
> values are used to **construct types** is compile-time determination required. Form (`: Int`) only
> determines candidate eligibility, use (type position appearance) determines whether it is a
> compile-time value parameter. This is the same root criterion as "function calls at type positions
> are evaluated at compile time" in §"Compile-time Determinism Guarantee".

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameter: N referenced at type position (Array length slot)
# ════════════════════════════════════════════════════════
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears at type constructor argument position → compile-time value parameter
    length: N,
}

# Usage: factorial(5) evaluated at type position (compile-time), result 120 embedded in type
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120)

# ════════════════════════════════════════════════════════
# Value-dependence: N serves as type of inner parameter k
# ════════════════════════════════════════════════════════
# N is compile-time value parameter (appears at (k: N) type position);
# k is runtime value parameter, whose type is the literal type N (single-value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Fall-through Candidate Handling**: Candidates annotated with concrete types but not referenced
> at type positions (e.g., `N` in `Foo` above) degrade to runtime value parameters (function-level
> path). Type constructor path fall-through candidates cannot occupy runtime slots (type
> constructors are evaluated at compile time), declaration side directly reports error [E1094]: "N
> declared as compile-time value parameter but not referenced in type body"—previously silent
> discard led to instantiation arity inconsistency.

#### 4.2 Compile-time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation example
# ════════════════════════════════════════════════════════

# Compiler computes function calls of literal types at compile time
SIZE: Int = factorial(5)  # Compile-time value 120

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

# Usage: compile-time computation, generates Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never and Void: The ⊥ and ⊤ of the Type System

YaoXiang's type system has both ⊥ (false/empty type) and ⊤ (true/Unit) in the Curry-Howard
Isomorphism, carried by two built-in type names `Never` and `Void`:

**Never (⊥)** — Three non-negotiable core properties:

1. **Zero Constructors**: No literal or expression can produce a value of type `Never`. This is a
   meta-level property, must be built-in.
2. **Principle of Explosion**: `Never <: T` holds for any type `T`. A `Never` value can be used as
   any type—this is why code after `assert(false)` still passes type checking (although never
   executed).
3. **Divergence Marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler uses this for dead code analysis.

`Never` is a built-in type name, not a keyword, parser is unaware. No empty sum type literal syntax
is opened.

**Void (⊤, i.e., Unit)** — exactly one inhabitant (default void value), the carrier of the true
proposition "always true". `Void` is the identity element of the zero-field product type, `Never` is
the identity element of the zero-variant sum type—dual to each other. `x: Void = <default>` is
legal, `x: Never = ...` has no right side to write.

#### 4.3 Compile-time Validation (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: using conditional types
# ════════════════════════════════════════════════════════

# Standard library definitions
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
# cond undecidable → decided by proof pipeline per dispatch mode:
#                    CompileTime → Unknown, requires prove
#                    Runtime     → insert check, inject Γ assumption

# Usage 1: as constraint in type definition
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at type position)
    length: Assert(N > 0),
}

# Usage 2: in expression
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# Validation: IntArray(10)'s size equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-time Generic Specialization

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

> **Curry-Howard Isomorphism**: From the Curry-Howard perspective, conditional types are **case
> analysis** in logic. The `Bool` type corresponds to a proposition with two possible values
> (True/False), `If` selects different results based on the truth value of that proposition—this is
> precisely the case disjunction in logic. `match C { True => T, False => E }` is actually
> expressing: "given proposition C is True the conclusion is T, given C is False the conclusion is
> E".

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

# Compile-time validation (unified to §4.3's Assert definition)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Families

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but rather **constructing a proposition about natural number addition**. `(Zero, B) => B` says
> "proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` says "if
> Add(A', B) holds, then Add(Succ(A'), B) also holds". This is the addition definition in Peano
> axioms itself. The type checker verifying this match expression passes is equivalent to verifying
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
sum: (arr: Array(Int)) -> Int = {
    # Compiles to more efficient code
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    # Use SIMD instructions
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

#### 6.2 Conditional Specialization

```yaoxiang
# Specialization method fully complying with RFC-010 syntax: function overloading

# Concrete type specialization
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation (compiler auto-selects optimal)
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# Usage is completely transparent
int_arr = Array(Int)(1, 2, 3)
float_arr = Array(Float)(1.0, 2.0, 3.0)

# Compiler auto-selects optimal specialization
sum(int_arr)     # Selects sum: (Array(Int)) -> Int
sum(float_arr)    # Selects sum: (Array(Float)) -> Float
```

#### 6.3 Perfect Combination of Function Overload and Inlining

**Key Feature**: Function overloading and inline optimization naturally combine, achieving zero-cost
abstraction.

```yaoxiang
# ======== Source Code ========
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

# Usage
int_arr = Array(Int)(1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== After Compilation (equivalent code) ========
# Compiler auto-selects optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core Advantages**:

1. **Compiler Intelligent Selection**

   ```yaoxiang
   sum(int_arr)      # Auto-selects sum: (Array(Int)) -> Int
   sum(float_arr)    # Auto-selects sum: (Array(Float)) -> Float
   sum(custom_arr)  # Auto-selects sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **Inline Optimization**
   - Small functions auto-inlined to call site
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type Safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function table required

4. **Perfect Fit with RFC-010**

   ```yaoxiang
   # Fully uses unified syntax
   name: type = value
   # No need for impl, where, or other new keywords
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

- ✅ **Generic specialization** → Function overloading solves it naturally
- ✅ **Performance optimization** → Inlining auto-completed
- ✅ **Code reuse** → One function name, multiple implementations
- ✅ **Zero-cost abstraction** → Compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords required** → Perfectly complies with RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internal: build generics instantiation dependency graph
struct InstantiationGraph {
    // Nodes: generics instantiations
    nodes: HashMap<InstanceKey, InstanceNode>,

    // Edges: usage relations
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
# These generics instances will not be generated

# After compilation only contains used instances
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 Compile-time Generic DCE

```yaoxiang
# Compile-time analysis: compile-time generics usage
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# Actual usage
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # Two-layer: type parameters + construction parameters
# Errata: earlier version wrote Array(Int, 10)(1, 2, 3, ...) (elements directly spread out),
# inconsistent with §9.3's authoritative pattern (Type(params)(field construction params)/empty construction),
# unified to field-name-style construction parameters, see SPEC type-system.md §4.3.
arr_100_int = Array(Int, 100)()   # Empty construction, data assigned afterward

# After compilation only generates used Sizes
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# Unused Sizes will not be generated
# Array(Int, 50) will not be generated
```

#### 7.4 Cross-module DCE

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
# - After compilation, binary only contains these two instances
```

#### 7.5 LLVM-level DCE

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

# ✅ Generics approach: automatic derivation
# Use function overloading for automatic derivation
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

#### 8.3 Type-level Programming Replacement

```yaoxiang
# ❌ Macro approach: type-level computation
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ Generics approach: conditional types
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
    data: Array(T),
    length: Int,

    # Generic methods (T is automatically brought into scope by outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generic methods ========
# Functions defined under List namespace (List. prefix = namespace attribution)
# For list.push(item) . call syntax to work, requires explicit binding: List.push = push[0]
# self is just a convention parameter name, compiler looks at type not name

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # Expand
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
# ======== 1. Generic sort algorithm ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# Generic quicksort
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
# Sort Int array
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# Sort String array (requires StringComparator)
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-time Generic Example

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

# Compile-time validation: result type is Matrix(Float, 2, 2)
# 2x2 identity matrix
identity_3x3 = identity(Float, 3)()

# Dimension mismatch: compile error
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # Compile error: 3x3 != 2x3
```

## Trade-offs

### Pros

1. **Zero-cost Abstraction**
   - Compile-time monomorphization, no runtime overhead
   - No virtual functions, no RTTI

2. **Dead Code Elimination**
   - Compile-time analysis, only instantiate used generics
   - Code bloat controllable

3. **Macro Replacement**
   - Type-safe code generation
   - IDE-friendly, clear error messages

4. **Compile-time Computation**
   - Compile-time generics supports compile-time computation
   - Dimension validation and other features
   - No `const` keyword required, pure type constraints

### Cons

1. **Compile Time**
   - Generics instantiation increases compile time
   - Constraint solving may be slow

2. **Memory Usage**
   - Compiler memory usage increases
   - Caching mechanisms require memory

3. **Implementation Complexity**
   - Constraint solver complex
   - Type-level computation engine complex

4. **Error Diagnostics**
   - Generics errors can be complex
   - Clear error messages required

### Mitigation

1. **Caching Strategy**
   - Instantiation result cache
   - LRU cache limits memory

2. **Incremental Compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error Messages**
   - Clear error messages
   - Generics parameter inference hints

4. **Parallel Compilation**
   - Parallel instantiation of generics
   - Multi-threaded constraint solving

## Alternatives

| Approach            | Why Not Chosen                      |
| ------------------- | ----------------------------------- |
| Basic generics only | Cannot replace complex macros       |
| Pure macro system   | No type safety, poor error messages |
| Constraint-only     | Insufficient flexibility            |
| Runtime generics    | Performance overhead                |

### Risks

| Risk                          | Impact                     | Mitigation                  |
| ----------------------------- | -------------------------- | --------------------------- |
| Constraint solving complexity | Compile time too long      | Incremental solving + cache |
| Code bloat                    | Binary file too large      | DCE + threshold control     |
| Implementation complexity     | Extended development cycle | Phased implementation       |
| Error diagnostics             | Poor user experience       | Detailed error messages     |

## Open Questions

### Pending Issues

| Topic                  | Description                         | Status |
| ---------------------- | ----------------------------------- | ------ |
| Instantiation strategy | Eager vs Lazy vs Threshold          | TBD    |
| Cache size             | LRU cache capacity setting          | TBD    |
| Error diagnostics      | Generics error message detail level | TBD    |

### Future Optimizations

| Optimization Item              | Value  | Implementation Difficulty |
| ------------------------------ | ------ | ------------------------- |
| Instantiation graph analysis   | High   | Medium                    |
| Type-level programming DSL     | Medium | High                      |
| Generics performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generics parameters use unified () syntax, as part of function type
# e.g., map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

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
# Generics type e.g., List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# Type in generics parameters is auto-filled by compiler from actual argument types
# e.g., map(numbers, f), T extracted from numbers: List(Int), R extracted from f: (Int) -> String
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
│ (Official Design) │  (Preserved in place) │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documents

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./accepted/009-ownership-model.md)
- [RFC-001: spawn Model](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)
- [tutorial/ Tutorials](../../../../../tutorial/)

### External References

- [Rust Generics System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
