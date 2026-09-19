---
title: 'RFC-011: Generics System Design - Zero-Cost Abstractions and Macro Replacement'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-15 (Type body code blocks + Compile-time specs + Effect seeds implemented)'
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

## Summary

This document defines YaoXiang language's **generics system design**, achieving zero-cost
abstractions through powerful generics capabilities, reducing dependency on macros through
compile-time optimization, and providing dead code elimination mechanisms.

**Core Design**:

- **Unified signature syntax**: `(T: Type, R: Type) -> ...` generics parameters unified with regular
  parameters
- **Type self-description mechanism**: `Type` is a language-level special existence, `Type`
  positions in signatures can be automatically inferred and filled
- **Type constraints**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated types**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time generics**: `N: Int` generics value parameters, compile-time constant instantiation
- **Conditional types**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type
  families

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
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax Foundation**     | Generics syntax integrated with unified `name: type = value` model                |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call Syntax**           | Section 6: generics call syntax—unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md)                                              | **Type System**           | Natural combination of Move semantics and generics                                |
| [RFC-024: spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution Model**       | DAG analysis and generics type checking                                           |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)                                      | **Compiler Architecture** | Generics monomorphization and compile-time optimization strategies                |
| [Type Universe Thought](../reference/plan/ongoing/类型宇宙思想.md)                                         | **Theoretical Core**      | Type universe layered model and value-dependent type design                       |
| [RFC-027: Compile-time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination Check**     | Automatic metric synthesis and compile-time evaluation safety guarantee           |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generics system is built on the **Type Universe Thought**, a mental model that unifies
all concepts in the language into a layered structure, with the core innovation of elevating
**value-dependent types** to first-class citizens at the Type2 layer.

### What are Value-Dependent Types?

**Value-dependent types** are types that depend on one or more **values** (not just other types).
These values can be evaluated at compile-time, thereby providing type safety guarantees at the
compile stage.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Array: (T: Type, N: Int) -> Type  # Array type depends on length value N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column counts
```

### Container Type Naming Layers

The language layer has three container concepts, with length information ownership being their
fundamental distinction:

| Type          | Length        | Semantics                                     | Underlying                               |
| ------------- | ------------- | --------------------------------------------- | ---------------------------------------- |
| `Array(T, N)` | Type          | **Fixed-length** array, N in the type         | Core primitive (stack/inline preferred)  |
| `Vec(T)`      | Runtime value | **Runtime-length primitive buffer**, growable | Core primitive (heap contiguous buffer)  |
| `List(T)`     | Runtime value | Standard library type                         | Library: `{ data: Vec(T), length: Int }` |

The division of labor principle:

- **`Array(T, N)` is the only form that puts length into the type**—length is a compile-time
  constant, so it can reject boundary failures at compile time (`a[5]` when `a: Array(Int, 3)`
  directly errors at compile time, see "Compile-time Dimension Validation" below).
- **`Vec(T)` is the minimal foundation for runtime length**—only provides "allocate, get length,
  read/write, expand" four things, no capacity policy, growth factor, or shrinkage. It is the raw
  material for building other containers.
- **`List(T)` is a library type, not a primitive**—defined in YaoXiang itself in `std.list`
  (`{ data: Vec(T), length: Int }`), with the same treatment as user-defined generic records. All
  growable semantics strategies (when to expand, how much, whether to share) are in the library, the
  compiler is not involved.

`Vec(T)` construction form (two layers: type parameters first, then construction parameters):

```yaoxiang
# Empty construction—length 0, elements appended afterwards
v = Vec(Int)()

# Element construction—length determined by number of elements
w = Vec(Int)(1, 2, 3)          # length 3

# Slot allocation—allocate n zero-value slots
buf = Vec(Int)(len=64)         # length 64, all elements are zero values
```

> Slot allocation uses **field name style** (`len=`) rather than positional: a positional single
> integer would be ambiguous with a "single-element vector" (`Vec(Int)(64)` cannot distinguish
> "length 64" from "containing one element 64"). This is consistent with the unified rule of
> generics construction: field name style arguments are bound by name, unaffected by positional
> inference.
>
> This is the only primitive needed for `List` expansion—`List` allocates new slots and moves
> elements when needed:
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> When to expand, how much, whether to shrink are all decided by `List`. `Vec` does not do capacity
> policy.

From bottom to top, performance decreases and flexibility increases: `Array` > `Vec` > `List`.

> Naming basis: `Vec`/`vector` in mainstream languages (Rust/C++) all refer to runtime-length
> growable sequences; `Array` refers to fixed-length.

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core
advantages:

| Feature                    | Traditional Generics (C++/Rust)               | YaoXiang Value-Dependent Types                            |
| -------------------------- | --------------------------------------------- | --------------------------------------------------------- |
| Values the type depends on | Only type parameters                          | Can depend on any value, including function call results  |
| Compile-time evaluation    | C++ template manual specialization, Rust none | Automatic compile-time evaluation, termination guaranteed |
| Type-level computation     | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                     |
| Type safety                | C++ none, Rust limited                        | Complete type safety, compile-time checking               |
| Dimension validation       | Runtime check or manual specialization        | Compile-time dimension validation, no runtime overhead    |

### Type Universe Layers and Value-Dependent Types

The Type Universe Thought divides language concepts by semantic role into different layers, with
value-dependent types located at the **Type2 layer**:

| Layer     | Role                                               | Examples                                                                                                        |
| --------- | -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | Values                                             | `42`, `factorial(5)`, the function itself                                                                       |
| Type0     | Meta-type keyword                                  | `Type`                                                                                                          |
| Type1     | Concrete type                                      | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **Function/Type constructor/Value-dependent type** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at the Type2 layer use
**unified syntax**, all in the form `(params) -> result`:

- Regular function: `(Int, Int) -> Int` → return value is a value
- Type constructor: `(T: Type) -> Type` → return value is a type
- Value-dependent type: `(T: Type, N: Int) -> Type` → return value is a type, and depends on value
  parameter N

> **Curry-Howard Isomorphism**: This unification is not a coincidence. The Curry-Howard Isomorphism
> states that "types are propositions, programs are proofs"—the function type `A → B` corresponds to
> logical implication "if A then B", the generics `(T: Type) -> Type` corresponds to universal
> quantification "for all types T", and the value-dependent type `(n: Int) -> Type` corresponds to
> "for every integer n there exists a type". YaoXiang unifies functions, type constructors, and
> value-dependent types at the Type2 layer, essentially unifying "proof" and "computation" as the
> same concept—**constructive proof**. This is the direct embodiment of the Curry-Howard Isomorphism
> in language design: one form (`(params) -> result`) simultaneously carries logical propositions
> and computational processes.

### Compile-time Determinism Guarantee

YaoXiang's Type Universe Thought requires: **everything in the Type layer is determined at
compile-time**.

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

1. Detects function calls at type positions
2. Performs compile-time termination check on functions (see termination check mechanism below)
3. Performs evaluation at compile time
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

#### Compile-time Coverage Goal for Boundary Failures

> **Implementation Status Note**: Container types have been de-specialized—`Array(T, N)` is a const
> generics constructor, literal context landing and `in` membership predicates have been
> implemented. The N and element type for the `Array(T, N)` literal landing are now enforced by
> compile-time checks (E1002), **N is now trusted**—the mechanism described in this section can be
> built on top of "annotation N == runtime length". Currently `[]` index out-of-bounds (E6003) and
> Dict missing key (E6008) are in a **runtime error transition state**; the value-dependent types in
> this section is the target mechanism to push these boundary failures to **compile time**:
>
> - const index: `a[5]` (5 is a compile-time constant) is directly rejected at compile time when
>   `a: Array(Int, 3)`;
> - value index: `a[i]` requires the precondition `i < len(a)`, proven by value-dependent type
>   contracts;
> - `in` predicate is the base of Hoare logic preconditions: `n in 1..10`, `x in some_set` are all
>   propositions provable at compile time.
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

# Completely transparent when used, type auto-inferred
numbers = List(Int)()   # Value construction two-layer form (see §9.1); elements filled with push
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
| Termination guarantee                                       | ❌                     | ❌            | ❌ (dangerous) | ✅ (automatic metric synthesis, RFC-027)  |
| Type safety                                                 | ❌ (macro expansion)   | ✅            | ✅             | ✅                                        |
| Unified syntax                                              | ❌                     | ❌            | ❌             | ✅                                        |
| Compile-time dimension validation                           | Manual specialization  | Runtime check | Type family    | Automatic at compile time                 |
| Semi-automatic termination annotation (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (only fully automatic at compile time) |

### Termination Check Mechanism (Unified with RFC-027)

The compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the
type system will fall into an infinite loop. The termination check is **fully automatic** by the
compile-time proof pipeline of RFC-027—the compiler automatically synthesizes metrics,
recursive/loop cases that can be proven pass through, and those that cannot be proven directly
report compile errors. **No room for semi-automatic annotation**: RFC-022's `//! decreases`,
`/*! invariant !*/` have been deprecated along with RFC-022, the specification is the type
annotation itself.

#### Termination Check for Recursive Functions

The compiler checks whether the arguments of recursive calls strictly decrease on every recursive
path before compile-time evaluation (RFC-027 §6.7). No specification comments are needed:

```yaoxiang
# Compile-time factorial: no //! requires/ensures/decreases, compiler auto-analyzes
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Use: called at type position, compiler first verifies termination then evaluates
arr: Array(Int, factorial(5)) = Array(Int, 120)()  # Compile-time evaluation factorial(5) = 120
```

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| Non-decreasing or unable to determine decrease        | Compile error               |
| Runtime call (not at type position)                   | No termination check needed |

#### Termination Check for Loops

Loops do not need `: Invariant(...)` or `: decreases(...)` annotations. Refined type annotations on
variables (e.g., `UpTo(n)`) simultaneously provide loop invariants and metric bounds. The compiler
tries four metric synthesis strategies in priority order, stopping when one is found (RFC-027 §7):

1. **Automatic linear ranking function synthesis**—extract variable bounds from type annotations,
   enumerate linear combinations, SMT verifies m ≥ 0 and all paths m' < m
2. **Predicate violation count** (experimental)—extract violation_count from target type definitions
   (e.g., `Sorted`), covering adjacent swap/move
3. **Bounded increment/decrement pattern**—`v += const` → metric `upper - v` (degeneration of
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
│  Type checking phase                                        │
│  Encountering function calls at type positions (e.g., factorial(5)) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination check (RFC-027 proof pipeline, fully automatic) │
│     - Recursive functions: check that arguments strictly decrease on every recursive path │
│     - Loops: four metric synthesis strategies (linear ranking/violation count/bounded pattern/ │
│       multiplicative scaling), SMT verifies decrease                                │
│     - Cannot prove → compile error (hard boundary, no semi-automatic annotation fallback)        │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time evaluation (executed by built-in interpreter)    │
│     - Pure functions: direct evaluation                     │
│     - Side effects: compile error (type positions must be side-effect free)                  │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result embedded in type                                 │
│     - Array(Int, factorial(5)) → Array(Int, 120)            │
│     - Matrix(Float, 3, 3) → concrete type                   │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensure that compile-time evaluation must terminate, avoiding the type system falling
  into infinite loops
- **Unification**: Termination check and correctness verification (VC generation) share the same
  compile-time proof pipeline (RFC-027), no independent specification syntax
- **Fully automatic**: The compiler automatically synthesizes metrics from type annotations; if
  provable, it passes; if not provable, it errors—no reliance on programmer-written `decreases`

## Motivation

### Why Need a Strong Generics System?

Current mainstream languages have limitations in generics:

| Language     | Generics Capability       | Problem                                                                                 |
| ------------ | ------------------------- | --------------------------------------------------------------------------------------- |
| Java         | Bounded types             | Compile-time monomorphization, no generics specialization                               |
| C#           | Generics constraints      | Runtime type check, has performance overhead                                            |
| Rust         | Generics + Trait          | Trait system complex, steep learning curve                                              |
| C++          | Templates                 | Template specialization complex, poor compile error messages                            |
| **YaoXiang** | **Value-dependent types** | **Type can depend on values, compile-time dimension validation, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: Runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: Powerful type system vs ease of use
3. **Macros vs Generics**: Macro code generation vs generics type safety
4. **Value-dependence vs Type safety**: Traditional generics cannot validate dimensions at compile
   time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                |
| --------------------------- | ---------------------------------------------------------------------------------------------------------- |
| **Type depends on value**   | `Array: (T: Type, N: Int) -> Type` lets the type depend on specific values                                 |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, results directly embedded in type          |
| **Dimension validation**    | `Matrix(Float, 3, 3)` validates matrix dimensions at compile time                                          |
| **Type-level computation**  | `If`, `Match` and other conditional types support type-level computation                                   |
| **Termination guarantee**   | Compile-time termination check (automatic metric synthesis) ensures compile-time evaluation must terminate |

```yaoxiang
# Compile-time validation that C++/Rust cannot do
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

# Traditional solution: each type implemented separately
map_int_array: (array: Vec(Int), f: Fn(Int) -> Int) -> Vec(Int) = ...
map_string_array: (array: Vec(String), f: Fn(String) -> String) -> Vec(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# Generics solution: one generic function covers all types
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## Design Goals

### Core Goals

1. **Zero-cost abstraction** - generics call equivalent to concrete type call
2. **Dead code elimination** - compile-time analysis, only instantiate used generics
3. **Macro replacement** - generics replace 90% of macro use cases
4. **Type safety** - compile-time check, no runtime type overhead
5. **IDE-friendly** - smart hints, clear error messages
6. **Value-dependent types** - types can depend on values, support compile-time dimension validation
7. **Compile-time evaluation safety** - guarantee compile-time evaluation termination through
   compile-time termination check (RFC-027 automatic metric synthesis)

### Design Principles

- **Compile-time determined**: generics parameters determined at compile time
- **Monomorphization priority**: generate concrete code, avoid virtual function calls
- **Constraint-driven**: type constraints guide instantiation
- **Platform optimization**: specialization supports platform-specific optimization
- **Type universe unification**: functions/type constructors/value-dependent types unified at Type2
  layer
- **Termination guarantee**: function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generics Type Parameters

> **Key Rule**: Generics type definitions **must explicitly annotate `: Type`**, otherwise they will
> be inferred by HM as a function.
>
> | Writing                           | Meaning                                |
> | --------------------------------- | -------------------------------------- |
> | `List: (T: Type) -> Type = {...}` | ✅ Type constructor                    |
> | `List = {...}`                    | ❌ HM infers as a function, not a type |

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
    push: (self: List(T), item: T) -> Void,   # self is just a convention name, not a keyword
    get: (self: List(T), index: Int) -> Option(T),
}

# Generics function (no : Type, HM infers as function)
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# Generics constraint (direct expression, return can be omitted for single line)
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

`Type` is a language-level special existence, the compiler can naturally recognize `Type` positions
in signatures and automatically infer and fill from actual parameter types.

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

# Use point
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

# Using constraints: declare type constraints directly in signature
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

# Sort generics container
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

| Type                     | Meaning                                                               | Trigger Method                         | Applicable Scenarios              |
| ------------------------ | --------------------------------------------------------------------- | -------------------------------------- | --------------------------------- |
| **Primitive value copy** | Automatic value copy on assignment, two values completely independent | Assignment/parameter passing automatic | Int, Float, Bool, Char            |
| **Dup**                  | Shallow copy: copy handle/token, underlying data shared               | Assignment/parameter passing automatic | `&T` token, `ref T`, String/Bytes |
| **Clone**                | Deep copy: create complete independent replica                        | `value.clone()`                        | Any type that implements Clone    |

**Semantics of Dup**: Types that implement Dup do not transfer ownership on assignment/parameter
passing—the compiler copies the handle/token, and multiple holders point to the same underlying
data. This is complementary to the default Move semantics in RFC-009's ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = copy handle, share data (modifications affect each other)
Clone = copy data, replica independent (modifications don't affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not belonging to Dup
2. Dup — only applies to reference/token types and internal reference counting types
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which Types are Dup**:

| Type                     | Dup     | Reason                                                             |
| ------------------------ | ------- | ------------------------------------------------------------------ |
| `&T` (borrow token)      | ✅      | Zero-size token, copy token = multiple views pointing to same data |
| `ref T`                  | ✅      | Rc/Arc copy = reference count +1, share heap data                  |
| String, Bytes            | ✅      | Internal reference count, copy handle shares underlying buffer     |
| `&mut T` (mutable token) | ❌      | Linear exclusive, cannot copy                                      |
| struct                   | Derived | All fields Dup → struct Dup                                        |
| enum                     | Derived | All fields of all variants Dup → enum Dup                          |
| tuple                    | Derived | All elements Dup → tuple Dup                                       |
| Fn (closure)             | ❌      | Captured environment may not be Dup                                |
| `*T` (raw pointer)       | ❌      | unsafe, not participating in ownership system                      |

**Int/Float/Bool/Char are not Dup**—they are value types, the compiler automatically copies values
on assignment (two values are completely independent). This is not "shallow copy", it's the
compiler's built-in handling of primitives, no need and should not be expressed through the Dup type
property.

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

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety is guaranteed by the `ref`
> keyword and fully automatic compiler processing—`ref` automatically selects Rc or Arc, users don't
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
# Iteration position is carried by the wrapper record (Vec itself is primitive buffer, no index field)
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

# Use
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. Compile-time Generics

#### 4.1 Compile-time Value Parameters

**Core Design**: `Type` in generics signatures marks type parameters; parameters annotated with
concrete types (`Int`/`Bool`/`Float`, etc.) are listed as **compile-time value parameter
candidates**, and whether they become compile-time value parameters depends on whether their value
is **referenced at a type position** (value-dependence). No `const` keyword is needed.

> The judgment is based on **being referenced at a type position**, not "annotated with a concrete
> type": in `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters because
> they don't appear at any type position.

**Judgment Rule (Two Steps)**:

1. **Form coarse screening**: Parameters annotated with concrete types other than `Type` (e.g.,
   `Int`) → listed as candidates.
2. **Usage fine screening**: Candidate name appears at **type positions** (type body field types,
   inner `Fn` parameter types, `Assert` predicates, `Array(T, N)` and other type construction
   argument positions) → confirmed as compile-time value parameters; otherwise treated as **runtime
   value parameters**.

| Writing                                                    | Judgment                       | Reason                                                                 |
| ---------------------------------------------------------- | ------------------------------ | ---------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters   | Only appear at value positions, not participating in type construction |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter | N appears at type construction argument position of `Array(T, N)`      |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter | N serves as the type of inner parameter `k`                            |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N falls through (see below)    | N not referenced in type body, degenerates to runtime value parameter  |

> **Value-dependence Essence**: Compile-time value parameters are value-dependent types—only when
> values are used to **construct types** do they need to be determined at compile time. Form
> (`: Int`) only determines candidacy, usage (appearing at type positions) determines whether it is
> a compile-time value parameter. This is the same root criterion as "function calls at type
> positions are evaluated at compile time" in §"Compile-time Determinism Guarantee".

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameters: N is referenced at type position (Measure length slot)
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears at type construction argument position → compile-time value parameter
    length: N,
}

# Use: factorial(5) evaluated at type position (compile time), result 120 embedded in type
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# Value-dependence: N serves as the type of inner parameter k
# ════════════════════════════════════════════════════════
# N is a compile-time value parameter (appears at the type position of (k: N));
# k is a runtime value parameter, its type is the literal type N (single value type).
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **Handling of Falling-through Candidates**: Candidates annotated with concrete types but not
> referenced at type positions (e.g., `N` in `Foo` in the table above) degenerate to runtime value
> parameters (function-level path). Falling-through candidates in the type constructor path cannot
> occupy runtime slots (type constructors are evaluated at compile time), and the declaration side
> directly reports error [E1094]: "N declared as compile-time value parameter but not referenced in
> type body"—the previous silent discarding caused instantiation arity inconsistency.

#### 4.2 Compile-time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation example
# ════════════════════════════════════════════════════════

# Compiler evaluates function calls of literal types at compile time
SIZE: Int = factorial(5)  # Compile-time 120

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

YaoXiang's type system simultaneously has ⊥ (false/empty type) and ⊤ (true/Unit) in the Curry-Howard
Isomorphism, carried by the two built-in type names `Never` and `Void`:

**Never (⊥)** — Three non-negotiable kernel properties:

1. **Zero constructors**: No literal or expression can produce a value of `Never` type. This is a
   meta-level property and must be built-in.
2. **Explosion principle**: `Never <: T` holds for any type `T`. A `Never` value can be used as any
   type—this is exactly why the code after `assert(false)` still passes type checking (although it
   never executes to there).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler performs dead code analysis based on this.

`Never` is a built-in type name, not a keyword, the parser doesn't see it. Empty sum and type
literal syntax is not opened.

**Void (⊤, i.e., Unit)** — Exactly one inhabitant (default void value), is the carrier of the true
proposition "always true". `Void` is the unit of zero-field product types, `Never` is the unit of
zero-variant sum types—the two are duals. `x: Void = <default>` is legal, `x: Never = ...` has no
right side to write.

#### 4.3 Compile-time Validation (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: using conditional types
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
# cond is false → Assert(false) = Never   (always false, compile error/diverge)
# cond undetermined → proof pipeline decides by dispatch mode:
#                     CompileTime → Unknown, requires prove
#                     Runtime     → insert check, inject Γ hypothesis

# Usage 1: as constraint in type definition
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at type position)
    length: Assert(N > 0),
}

# Usage 2: use in expression
IntArray: (N: Int) -> Type = Array(Int, N)
# Validation: size of IntArray(10) equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-time Generics Specialization

```yaoxiang
# Small array optimization: use function overloading to implement compile-time generics specialization

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
> analysis** in logic. `Bool` type corresponds to a proposition with two possible values
> (True/False), `If` chooses different results based on the truth or falsity of that
> proposition—this is exactly case disjunction in logic. `match C { True => T, False => E }`
> actually expresses: "when proposition C is known to be True, the conclusion is T, when C is False,
> the conclusion is E".

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

#### 5.2 Type Families

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but **constructing a proposition about natural number addition**. `(Zero, B) => B` says
> "proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` says "if
> Add(A', B) holds, then Add(Succ(A'), B) also holds". This is the addition definition itself in
> Peano axioms. The type checker verifying that this match expression passes is equivalent to
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
# Basic specialization: use function overloading (compiler auto-selects)
sum: (arr: Vec(Int)) -> Int = {
    # Compile to more efficient code
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
# Specialization way fully compliant with RFC-010 syntax: function overloading

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

# Completely transparent when used
int_arr = Vec(Int)(1, 2, 3)
float_arr = Vec(Float)(1.0, 2.0, 3.0)

# Compiler auto-selects optimal specialization
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

# Use
int_arr = Vec(Int)(1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== After compilation (equivalent code) ========
# Compiler auto-selects optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core Advantages**:

1. **Compiler Smart Selection**

   ```yaoxiang
   sum(int_arr)      # Auto-selects sum: (Vec(Int)) -> Int
   sum(float_arr)    # Auto-selects sum: (Vec(Float)) -> Float
   sum(custom_arr)   # Auto-selects sum: (T: Type) -> ((arr: Vec(T)) -> T)
   ```

2. **Inlining Optimization**
   - Small functions auto-inlined to call site
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
   # No new keywords like impl, where needed
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

- ✅ **Generics Specialization** → Function overloading naturally solves
- ✅ **Performance Optimization** → Inlining automatic
- ✅ **Code Reuse** → One function name, multiple implementations
- ✅ **Zero-cost Abstraction** → Compile-time polymorphism, zero runtime overhead
- ✅ **No New Keywords** → Perfectly fits RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internal: build generics instantiation dependency graph
struct InstantiationGraph {
    // Nodes: generics instantiation
    nodes: HashMap<InstanceKey, InstanceNode>,

    // Edges: usage relationships
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // generics function ID
    type_args: Vec<TypeId>,  // type arguments
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
# These generics instances will not be generated

# After compilation only includes used instances
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 Compile-time Generics DCE

```yaoxiang
# Compile-time analysis: compile-time generics usage
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# Actual usage
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # Two layers: type parameters + construction parameters
arr_100_int = Array(Int, 100)()   # Empty construction, data assigned afterwards

# After compilation only generated used Sizes
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

# Compilation analysis:
# - Module B uses map[Int, Int]
# - Module C uses map[String, String]
# - After compilation the binary only contains these two instances
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

# ✅ Generics solution: auto-derive
# Use function overloading way to auto-derive
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# Use
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

# ✅ Generics solution: type-safe builder
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

# ✅ Generics solution: conditional types
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

#### 9.1 Complete Generics Container Example

```yaoxiang
# ======== 1. Define generics container ========
# Use (T: Type) -> Type syntax
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

    # Generics method (T is automatically brought into scope by outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generics methods ========
# Function definition under List namespace (List. prefix = namespace ownership)
# To make list.push(item) . call syntax work, explicit binding is needed: List.push = push[0]
# self is just a convention parameter name, compiler looks at type not name

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

# Use fold computation
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
# Use function overloading to implement
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

#### 9.3 Compile-time Generics Example

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

### Advantages

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
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compile Time**
   - Generics instantiation increases compile time
   - Constraint solving may be slow

2. **Memory Usage**
   - Compiler memory usage increases
   - Cache mechanism needs memory

3. **Implementation Complexity**
   - Constraint solver complex
   - Type-level computation engine complex

4. **Error Diagnosis**
   - Generics errors may be complex
   - Need clear error hints

### Mitigation Measures

1. **Cache Strategy**
   - Instantiation result cache
   - LRU cache limits memory

2. **Incremental Compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error Hints**
   - Clear error messages
   - Generics parameter inference hints

4. **Parallel Compilation**
   - Parallel instantiation of generics
   - Multi-threaded constraint solving

## Alternatives

| Solution            | Why Not Choose                      |
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
| Implementation complexity     | Development cycle extended | Phased implementation       |
| Error diagnosis               | Poor user experience       | Detailed error messages     |

## Open Questions

### Pending Issues

| Topic                  | Description                         | Status     |
| ---------------------- | ----------------------------------- | ---------- |
| Instantiation strategy | Eager vs Lazy vs Threshold          | To discuss |
| Cache size             | LRU cache capacity setting          | To discuss |
| Error diagnosis        | Generics error message detail level | To discuss |

### Future Optimization

| Optimization Item              | Value  | Implementation Difficulty |
| ------------------------------ | ------ | ------------------------- |
| Instantiation graph analysis   | High   | Medium                    |
| Type-level programming DSL     | Medium | High                      |
| Generics performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generics parameters use unified () syntax as part of function type
# e.g. map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

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

# Type in generics parameters is automatically filled by compiler from argument types
# e.g. map(numbers, f), T extracted from numbers: List(Int), R extracted from f: (Int) -> String
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
│ (Formal Design) │  │ (Kept in place) │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documentation

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
