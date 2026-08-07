---
title: 'RFC-011: Generics System Design - Zero-Cost Abstractions and Macro Replacement'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-15 (type body code blocks + compile-time constraints + effect seeds implemented)'
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

This document defines the **generics system design** of the YaoXiang language. Through powerful
generic capabilities, it achieves zero-cost abstractions, reduces dependence on macros via
compile-time optimizations, and provides dead code elimination mechanisms.

**Core Design**:

- **Unified Signature Syntax**: `(T: Type, R: Type) -> ...` generic parameters unified with regular
  parameters
- **Type Self-Describing Mechanism**: `Type` is a language-level special entity, and the `Type`
  position in a signature can be automatically inferred and filled
- **Type Constraint**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated Type**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-Time Generics**: `N: Int` generic value parameters, instantiated via compile-time
  constants
- **Conditional Type**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type
  family

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
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax Base**           | Generic syntax integrated with the unified `name: type = value` model            |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call Syntax**           | Section 6: Generic call syntax—unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md)                                              | **Type System**           | Natural combination of Move semantics and generics                               |
| [RFC-024: Spawn-Based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution Model**       | DAG analysis and generic type checking                                           |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)                                      | **Compiler Architecture** | Generic monomorphization and compile-time optimization strategy                  |
| [Type Universe Thought](../reference/plan/ongoing/类型宇宙思想.md)                                         | **Theoretical Core**      | Type universe hierarchy model and value-dependent type design                    |
| [RFC-027: Compile-Time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination Check**     | Automatic metric synthesis and compile-time evaluation safety guarantee          |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generics system is built on the **Type Universe Thought**. This mental model unifies all
language concepts into a hierarchical structure, and its core innovation elevates **value-dependent
types** to first-class citizens at the Type2 layer.

### What Are Value-Dependent Types?

A **value-dependent type** is a type that depends on one or more **values** (not just on other
types). These values can be evaluated at compile time, providing type safety guarantees during the
compilation phase.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent type: value parameters
Vec: (n: Int) -> Type  # Vector type depends on length value n
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column counts
```

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types offer the following core
advantages:

| Feature                 | Traditional Generics (C++/Rust)               | YaoXiang Value-Dependent Types                            |
| ----------------------- | --------------------------------------------- | --------------------------------------------------------- |
| Values types depend on  | Only type parameters                          | Any value, including function call results                |
| Compile-time evaluation | C++ template manual specialization, Rust none | Automatic compile-time evaluation, guarantees termination |
| Type-level computation  | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                     |
| Type safety             | C++ none, Rust limited                        | Complete type safety, compile-time checking               |
| Dimension validation    | Runtime checks or manual specialization       | Compile-time dimension validation, no runtime overhead    |

### Type Universe Hierarchy and Value-Dependent Types

The Type Universe Thought divides language concepts by semantic role into different layers.
Value-dependent types reside at the **Type2 layer**:

| Layer     | Role                                                      | Examples                                                                                             |
| --------- | --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Type-1    | Values                                                    | `42`, `factorial(5)`, functions themselves                                                           |
| Type0     | Meta-type keywords                                        | `Type`                                                                                               |
| Type1     | Concrete types                                            | `Int`, `String`, `Vec(3)`                                                                            |
| **Type2** | **Functions / Type Constructors / Value-Dependent Types** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at the Type2 layer **share
unified syntax**, all in the form `(params) -> result`:

- Regular function: `(Int, Int) -> Int` → return value is a value
- Type constructor: `(T: Type) -> Type` → return value is a type
- Value-dependent type: `(n: Int) -> Type` → return value is a type, but depends on value parameters

> **Curry-Howard Isomorphism**: This unification is no coincidence. The Curry-Howard Isomorphism
> states "types as propositions, programs as proofs"—the function type `A → B` corresponds to the
> logical implication "if A then B", generics `(T: Type) -> Type` corresponds to universal
> quantification "for all types T", and value-dependent types `(n: Int) -> Type` corresponds to "for
> every integer n there exists a type". YaoXiang unifies functions, type constructors, and
> value-dependent types at the Type2 layer, essentially unifying "proof" and "computation" into a
> single concept—**constructive proof**. This is the direct embodiment of the Curry-Howard
> Isomorphism in language design: a single form (`(params) -> result`) simultaneously carries
> logical propositions and computational processes.

### Compile-Time Determinism Guarantee

YaoXiang's Type Universe Thought requires: **Everything at the Type layer is determined at compile
time**.

```yaoxiang
# Compile-time dimension validation example
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

# Compile-time computation: factorial(3) = 6, vector size determined at compile time
vec: Vec(factorial(3)) = Vec(6)()
```

The compiler will automatically:

1. Detect function calls at type positions
2. Perform compile-time termination check on functions (see termination check mechanism below)
3. Execute evaluation at compile time
4. Embed the result into the generated type

### Application Scenarios for Value-Dependent Types

#### Compile-Time Dimension Validation

```yaoxiang
# Matrix multiplication: compile-time dimension matching validation
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

# N is a compile-time constant, can be used for type-level computation
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3 (known at compile time)
```

#### Conditional Type

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

# Completely transparent at usage, types auto-inferred
numbers = List(1, 2, 3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                                     | C++ Templates          | Rust Generics  | Haskell GADT   | **YaoXiang**                              |
| ----------------------------------------------------------- | ---------------------- | -------------- | -------------- | ----------------------------------------- |
| Type parameters                                             | ✅                     | ✅             | ✅             | ✅                                        |
| Value-dependent types                                       | ❌                     | ❌             | ✅             | ✅                                        |
| Compile-time evaluation                                     | Template instantiation | ❌             | ✅             | ✅                                        |
| Termination guarantee                                       | ❌                     | ❌             | ❌ (dangerous) | ✅ (automatic metric synthesis, RFC-027)  |
| Type safety                                                 | ❌ (macro expansion)   | ✅             | ✅             | ✅                                        |
| Unified syntax                                              | ❌                     | ❌             | ❌             | ✅                                        |
| Compile-time dimension validation                           | Manual specialization  | Runtime checks | Type families  | Automatic compile-time validation         |
| Semi-automatic termination annotation (decreases/invariant) | ❌                     | ❌             | ❌             | ❌ (fully automatic at compile time only) |

### Termination Check Mechanism (Unified with RFC-027)

The compile-time evaluation of value-dependent types must **guarantee termination**; otherwise the
type system will fall into infinite loops. The termination check is performed **fully
automatically** by the RFC-027 compile-time proof pipeline—the compiler automatically synthesizes
metrics, and recursive/loop calls that can be proven pass, while those that cannot are reported as
compile errors. **No semi-automatic annotations are allowed**: RFC-022's `//! decreases`,
`/*! invariant !*/` have been deprecated along with RFC-022, and the specification is the type
annotation itself.

#### Termination Check for Recursive Functions

Before compile-time evaluation, the compiler checks whether the parameters of recursive calls
strictly decrease on every recursive path (RFC-027 §6.7). No specification annotations required:

```yaoxiang
# Compile-time factorial: no //! requires/ensures/decreases, compiler auto-analyzes
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Usage: called at a type position, compiler verifies termination before evaluation
vec: Vec(factorial(5)) = Vec(120)()  # Compile-time evaluates factorial(5) = 120
```

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| Not decreasing / cannot determine decrease            | Compile error               |
| Runtime call (not at a type position)                 | No termination check needed |

#### Termination Check for Loops

Loops do not require `: Invariant(...)` or `: decreases(...)` annotations. Refined type annotations
on variables (such as `UpTo(n)`) simultaneously provide loop invariants and metric bounds. The
compiler tries four metric synthesis strategies in order of priority, stopping once one is found
(RFC-027 §7):

1. **Automatic linear rank function synthesis**—extract variable bounds from type annotations,
   enumerate linear combinations, SMT verifies m ≥ 0 and all paths m' < m
2. **Predicate violation count** (experimental)—extract violation_count from target type definition
   (e.g., `Sorted`), covering adjacent swaps/moves
3. **Bounded increment/decrement patterns**—`v += const` → metric `upper - v` (degenerate case of
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
│  Encounter function call at a type position (e.g. Vec(factorial(5))) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination Check (RFC-027 proof pipeline, fully automatic) │
│     - Recursive functions: check parameters strictly decrease on every recursive path │
│     - Loops: four metric synthesis strategies (linear rank/violation count/bounded pattern/ │
│       multiplicative scaling), SMT verifies decrease         │
│     - Cannot prove → compile error (hard boundary, no semi-automatic annotation fallback) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-Time Evaluation (executed by built-in interpreter) │
│     - Pure functions: direct evaluation                      │
│     - Side effects: compile error (type positions must be side-effect free) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result Embedded into Type                                │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → concrete type                   │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensures compile-time evaluation necessarily terminates, preventing the type system
  from falling into infinite loops
- **Uniformity**: Termination check shares the same compile-time proof pipeline as correctness
  verification (VC generation) (RFC-027), with no separate specification syntax
- **Fully automatic**: Compiler automatically synthesizes metrics from type annotations. If it can
  prove, it passes; if not, it errors—no reliance on programmers writing `decreases` manually

## Motivation

### Why Do We Need a Strong Generics System?

Current mainstream languages have limitations in generics:

| Language     | Generic Capability        | Problem                                                                                  |
| ------------ | ------------------------- | ---------------------------------------------------------------------------------------- |
| Java         | Bounded types             | Compile-time monomorphization, no generic specialization                                 |
| C#           | Generic constraints       | Runtime type checking, performance overhead                                              |
| Rust         | Generics + Trait          | Trait system is complex, steep learning curve                                            |
| C++          | Templates                 | Complex template specialization, poor compile error messages                             |
| **YaoXiang** | **Value-Dependent Types** | **Types can depend on values, compile-time dimension validation, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: Runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: Powerful type system vs ease of use
3. **Macros vs Generics**: Macro code generation vs generic type safety
4. **Value Dependence vs Type Safety**: Traditional generics cannot verify dimensions at compile
   time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                        |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Types depend on values**  | `Vec: (n: Int) -> Type` allows types to depend on concrete values                                                  |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, with results directly embedded into types          |
| **Dimension validation**    | `Matrix(Float, 3, 3)` validates matrix dimensions at compile time                                                  |
| **Type-level computation**  | `If`, `Match` and other conditional types support type-level computation                                           |
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
# map operations for different container types

# Traditional approach: separate implementation for each type
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
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

1. **Zero-Cost Abstraction** - Generic calls equivalent to concrete-type calls
2. **Dead Code Elimination** - Compile-time analysis, only instantiate used generics
3. **Macro Replacement** - Generics replace 90% of macro usage scenarios
4. **Type Safety** - Compile-time checking, no runtime type overhead
5. **IDE-Friendly** - Smart hints, clear error messages
6. **Value-Dependent Types** - Types can depend on values, supporting compile-time dimension
   validation
7. **Compile-Time Evaluation Safety** - Guarantee compile-time evaluation termination via
   compile-time termination check (RFC-027 automatic metric synthesis)

### Design Principles

- **Compile-time determined**: Generic parameters are determined at compile time
- **Monomorphization first**: Generate concrete code, avoid virtual function calls
- **Constraint-driven**: Type constraints guide instantiation
- **Platform optimization**: Specialization supports platform-specific optimization
- **Type Universe unification**: Functions/type constructors/value-dependent types unified as Type2
  layer
- **Termination guarantee**: Function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key Rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise HM will
> infer them as functions.
>
> | Syntax                            | Meaning                              |
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

# Generic constraint (direct expression, return can be omitted for single line)
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

`Type` is a language-level special entity. The compiler can naturally recognize `Type` positions in
signatures and automatically infer and fill them from actual argument types.

```yaoxiang
# Compiler auto-inferring generic parameters
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         Type declaration   Constructor call: Int fills T

# Function call inference
numbers: List(Int) = List(Int)
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
int_list: List(Int) = List(Int)
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # Instantiate map[Int, Int]

string_list: List(String) = List(String)
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # Instantiate map[String, String]

# After compilation (equivalent code)
map_Int_Int: (list: List(Int), f: (Int) -> Int) -> List(Int) = {
    result: List(Int) = List(Int)
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
numbers: List(Int) = List(Int)
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

# Using constraints: declare type constraints directly in the signature
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
map: (T: Type, R: FnMut(T))(array: Array(T), f: R) -> Array(R) = {
    result: Array(R) = Array()
    for item in array {
        result.push(f(item))
    }
    return result
}

# Usage
doubled: Array(Int) = map(Array(1, 2, 3), (x: Int) => x * 2)  # Compiler infers
```

#### 2.3 Function Type Constraint

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

#### 2.4 Builtin Marker Trait: Dup and Clone

**Three categories of copy semantics**:

| Type                     | Meaning                                                          | Trigger                          | Applicable Scenarios               |
| ------------------------ | ---------------------------------------------------------------- | -------------------------------- | ---------------------------------- |
| **Primitive value copy** | Auto value copy on assignment, two values completely independent | Auto on assignment/param passing | Int, Float, Bool, Char             |
| **Dup**                  | Shallow copy: copies handle/token, underlying data shared        | Auto on assignment/param passing | `&T` tokens, `ref T`, String/Bytes |
| **Clone**                | Deep copy: creates a complete independent replica                | `value.clone()`                  | Any type implementing Clone        |

**Dup Semantics**: Types implementing Dup do not transfer ownership on assignment/param passing—the
compiler copies the handle/token, and multiple holders point to the same underlying data. This
complements the default Move semantics in the RFC-009 ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = Copy handle, share data (modifications affect each other)
Clone = Copy data, independent replica (modifications do not affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not part of Dup
2. Dup — only applies to reference/token types and internally reference-counted types
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which types are Dup**:

| Type                     | Dup     | Reason                                                            |
| ------------------------ | ------- | ----------------------------------------------------------------- |
| `&T` (borrow token)      | ✅      | Zero-sized token, copying token = multiple views to same data     |
| `ref T`                  | ✅      | Rc/Arc copy = reference count+1, shares heap data                 |
| String, Bytes            | ✅      | Internal reference counting, copy handle shares underlying buffer |
| `&mut T` (mutable token) | ❌      | Linearly exclusive, cannot copy                                   |
| struct                   | Derived | All fields Dup → struct Dup                                       |
| enum                     | Derived | All fields of all variants Dup → enum Dup                         |
| tuple                    | Derived | All elements Dup → tuple Dup                                      |
| Fn (closure)             | ❌      | Captured environment may be non-Dup                               |
| `*T` (raw pointer)       | ❌      | unsafe, not part of ownership system                              |

**Int/Float/Bool/Char are not Dup**—they are value types, and the compiler auto-copies values on
assignment (the two values are completely independent). This is not "shallow copy", but the
compiler's built-in handling of primitives, which doesn't need and shouldn't be expressed through
the Dup type property.

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

# Generic constraints
dup_use: (T: Dup) -> T = x         # T: Dup → can shallow copy
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can deep copy
```

> **Note**: `Send`/`Sync` are not exposed as user-visible traits. Cross-task safety guarantees are
> handled by the `ref` keyword and the compiler fully automatically—`ref` auto-selects Rc or Arc,
> users don't need to understand Send/Sync.

### 3. Associated Type

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

# Iterator implementation for Array
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

#### 3.2 Generic Associated Type (GAT)

```yaoxiang
# More complex associated type
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

#### 4.1 Compile-Time Constant Parameters

**Core Design**: The `Type` marker in a generic signature marks a compile-time type parameter. Value
parameters like `Int` are by default determinable at compile time in a generic context. No `const`
keyword is needed.

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time constant parameters: Int in generics is determined at compile time by default
# ════════════════════════════════════════════════════════

# Compile-time factorial: N must be a compile-time-known literal
factorial: (N: Int) -> (n: N) -> Int = {
    return match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# Compile-time addition
add: (a: Int, b: Int) -> (a: a, b: b) -> Int = a + b

# ════════════════════════════════════════════════════════
# Compile-time constant array
# ════════════════════════════════════════════════════════
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # Array with compile-time-known size
    length: N,
}

# Usage
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120), compiler computes at compile time
```

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation examples
# ════════════════════════════════════════════════════════

# Compiler computes function calls on literal types at compile time
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

### Never and Void: ⊥ and ⊤ of the Type System

YaoXiang's type system simultaneously possesses ⊥ (false/empty type) and ⊤ (true/Unit) in the
Curry-Howard Isomorphism, carried by the two builtin type names `Never` and `Void`:

**Never (⊥)** — three non-negotiable core properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. This is a
   meta-level property and must be built-in.
2. **Explosion principle**: `Never <: T` holds for any type `T`. A `Never` value can be used as any
   type—this is why code after `assert(false)` still passes type checking (though it never
   executes).
3. **Divergence marker**: `f: (...) -> Never` means `f` is guaranteed not to return. The compiler
   uses this for dead code analysis.

`Never` is a builtin type name, not a keyword, and the parser is unaware of it. No empty sum type or
type literal syntax is exposed.

**Void (⊤, i.e., Unit)** — has exactly one inhabitant (the default void value), the carrier of the
true proposition "always true". `Void` is the unit of zero-field product types, `Never` is the unit
of zero-variant sum types—the two are dual. `x: Void = <default>` is legal, `x: Never = ...` has no
right-hand side possible.

#### 4.3 Compile-Time Validation (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: utilizing conditional types
# ════════════════════════════════════════════════════════

# Standard library definition
# IsTrue: bridge from value universe to type universe—Bool truth value maps to a type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has a value, program continues
    false => Never,    # ⊥, no value, diverges
}

# Assert: compile-time refinement type primitive—type-level statement of a Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (tautology, erased)
# cond is false → Assert(false) = Never   (contradiction, compile error/divergence)
# cond undecidable → proof pipeline decides by dispatch mode:
#                    CompileTime → Unknown, requires prove
#                    Runtime     → insert check, inject Γ assumption

# Usage 1: as a constraint in type definition
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert in type position)
    length: Assert(N > 0),
}

# Usage 2: in expressions
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# Validate: IntArray(10) size equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generic Specialization

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

### 5. Conditional Type

> **Curry-Howard Isomorphism**: Conditional types are **case analysis** in logic from the
> Curry-Howard perspective. The `Bool` type corresponds to a proposition with two possible values
> (True/False), and `If` selects different results based on whether the proposition is true or
> false—this is exactly case disjunction in logic. `match C { True => T, False => E }` is
> essentially expressing: "given proposition C, the conclusion is T when C is True, and E when C is
> False".

#### 5.1 If Conditional Type

```yaoxiang
# Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# Examples: compile-time branching
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# Compile-time validation (unified with the Assert definition in §4.3)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Family

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but rather **constructing a proposition about natural number addition**. `(Zero, B) => B` says
> "proposition Add(Zero, B) is equivalent to B", and `(Succ(A'), B) => Succ(Add(A', B))` says "if
> Add(A', B) holds, then Add(Succ(A'), B) also holds". This is exactly the addition definition in
> the Peano axioms. The type checker verifying that this match expression passes is equivalent to
> verifying the logical consistency of this definition.

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

# Type-level addition (Curry-Howard: case analysis + recursive call, requires termination check to be full induction)
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# Example: compile-time compute 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. Function Overload Specialization

#### 6.1 Basic Specialization

```yaoxiang
# Basic specialization: use function overloading (compiler auto-selects)
sum: (arr: Array(Int)) -> Int = {
    # Compiled into more efficient code
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    # Uses SIMD instructions
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
# Fully compliant with RFC-010 syntax specialization method: function overloading

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

# Completely transparent at usage
int_arr = Array(Int)(1, 2, 3)
float_arr = Array(Float)(1.0, 2.0, 3.0)

# Compiler auto-selects optimal specialization
sum(int_arr)     # Selects sum: (Array(Int)) -> Int
sum(float_arr)    # Selects sum: (Array(Float)) -> Float
```

#### 6.3 Perfect Combination of Function Overloading and Inlining

**Key Feature**: Function overloading naturally combines with inlining optimization, achieving
zero-cost abstraction.

```yaoxiang
# ======== Source code ========
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

# ======== After compilation (equivalent code) ========
# Compiler auto-selects optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to manually written optimized code, no function call overhead!
```

**Core Advantages**:

1. **Compiler intelligent selection**

   ```yaoxiang
   sum(int_arr)      # Auto-select sum: (Array(Int)) -> Int
   sum(float_arr)    # Auto-select sum: (Array(Float)) -> Float
   sum(custom_arr)  # Auto-select sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **Inlining optimization**
   - Small functions auto-inlined at call site
   - Zero function call overhead
   - Completely equivalent to manually written optimized code

3. **Type safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function tables needed

4. **Perfect fit with RFC-010**

   ```yaoxiang
   # Fully uses unified syntax
   name: type = value
   # No need for impl, where, or other new keywords
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

**What Does This Mean?**

- ✅ **Generic specialization** → function overloading naturally solves it
- ✅ **Performance optimization** → inlining auto-completed
- ✅ **Code reuse** → one function name, multiple implementations
- ✅ **Zero-cost abstraction** → compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords** → perfectly conforms to RFC-010 unified syntax

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

#### 7.2 Use-Site Analysis

```yaoxiang
# Source code analysis
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Use site 1: instantiate map(Int, Int)
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # Needs map[Int, Int]

# Use site 2: instantiate map(String, String)
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Needs map[String, String]

# Unused: map[Float, Float], etc.
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
arr_10_int = Array(Int, 10)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array(Int, 100)(...)

# After compilation, only used Sizes are generated
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
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # Instantiate map(Int, Int)

# Module C
# C.yx
use A.{map}
string_list = List("a", "b", "c")
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
# Use function overloading for auto-derive
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

    # Generic method (T is auto-imported into scope by outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generic methods ========
# Function defined under List namespace (List. prefix = namespace ownership)
# To make . call syntax like list.push(item) work, explicit binding needed: List.push = push[0]
# self is just a conventional parameter name, compiler looks at type not name

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # Expand capacity
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

# ======== 4. Usage examples ========
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
# Implemented via function overloading
compare: (a: Int, b: Int) -> Int = {
    if a < b {
        return -1
    } else if a > b {
        return 1
    } else {
        return 0
    }
}

# ======== 3. Usage examples ========
# Sort Int array
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# Sort String array (requires StringComparator)
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-Time Generic Example

```yaoxiang
# ======== 1. Compile-time matrix type ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # Compile-time dimension validation: using the Assert standard library type
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

# ======== 3. Usage examples ========
# Create a matrix with compile-time-known size
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
   - Controllable code bloat

3. **Macro Replacement**
   - Type-safe code generation
   - IDE-friendly, clear error messages

4. **Compile-Time Computation**
   - Compile-time generics support compile-time computation
   - Dimension validation and other features
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compile Time**
   - Generic instantiation increases compile time
   - Constraint solving may be slow

2. **Memory Usage**
   - Compiler memory usage increases
   - Caching mechanism requires memory

3. **Implementation Complexity**
   - Constraint solver is complex
   - Type-level computation engine is complex

4. **Error Diagnostics**
   - Generic errors may be complex
   - Clear error messages needed

### Mitigations

1. **Caching Strategy**
   - Cache instantiation results
   - LRU cache limits memory

2. **Incremental Compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error Messages**
   - Clear error messages
   - Generic parameter inference hints

4. **Parallel Compilation**
   - Parallel generic instantiation
   - Multi-threaded constraint solving

## Alternatives

| Alternative                 | Why Not Chosen                      |
| --------------------------- | ----------------------------------- |
| Basic generics only         | Cannot replace complex macros       |
| Pure macro system           | No type safety, poor error messages |
| Dependency constraints only | Insufficient flexibility            |
| Runtime generics            | Performance overhead                |

### Risks

| Risk                          | Impact                     | Mitigation                    |
| ----------------------------- | -------------------------- | ----------------------------- |
| Constraint solving complexity | Long compile time          | Incremental solving + caching |
| Code bloat                    | Binary too large           | DCE + threshold control       |
| Implementation complexity     | Extended development cycle | Phased implementation         |
| Error diagnostics             | Poor user experience       | Detailed error messages       |

## Open Questions

### Issues to be Resolved

| Topic                  | Description                               | Status     |
| ---------------------- | ----------------------------------------- | ---------- |
| Instantiation strategy | Eager vs Lazy vs Threshold                | To discuss |
| Cache size             | LRU cache capacity setting                | To discuss |
| Error diagnostics      | Level of detail in generic error messages | To discuss |

### Future Optimizations

| Optimization                   | Value  | Implementation Difficulty |
| ------------------------------ | ------ | ------------------------- |
| Instantiation graph analysis   | High   | Medium                    |
| Type-level programming DSL     | Medium | High                      |
| Generic performance benchmarks | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use unified () syntax, as part of the function type
# E.g. map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

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

# Type in generic parameters is auto-filled by compiler from actual argument types
# E.g. map(numbers, f), T is extracted from numbers: List(Int), R is extracted from f: (Int) -> String
```

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Reviewing  │  ← Open community discussion and feedback
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
│ (Official)  │    │ (Preserved) │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documentation

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./accepted/009-ownership-model.md)
- [RFC-001: spawn Model](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)
- [tutorial/](../../../../../tutorial/)

### External References

- [Rust Generics System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
