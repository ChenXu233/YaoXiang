---
title: 'RFC-011: Generics System Design - Zero-Cost Abstractions and Macro Replacement'
status: 'Accepted'
author: 'Chen Xu'
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

# RFC-011: Generics System Design - Zero-Cost Abstractions and Macro Replacement

## Summary

This document defines the **Generics System Design** of the YaoXiang language. Through powerful
generics capabilities, it achieves zero-cost abstractions, reduces reliance on macros via
compile-time optimizations, and provides a dead code elimination mechanism.

**Core Design**:

- **Unified Signature Syntax**: `(T: Type, R: Type) -> ...` generics parameters unified with regular
  parameters
- **Type Self-Description Mechanism**: `Type` is a language-level special entity, and `Type`
  positions in signatures are auto-inferred and filled
- **Type Constraints**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated Types**:
  `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-Time Generics**: `N: Int` generic value parameters, compile-time constant instantiation
- **Conditional Types**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type
  families

**Value**:

- Zero-cost abstractions: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replaces 90% of macro use cases
- Type safety: compile-time check, IDE-friendly
- **Explicit over implicit**: `Type` self-description, compiler auto-inference

## Reference Documents

This document's design is based on the following documents:

| Document                                                                                                   | Relationship              | Description                                                                 |
| ---------------------------------------------------------------------------------------------------------- | ------------------------- | --------------------------------------------------------------------------- |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Syntax Base**           | Generics syntax integrated with the unified `name: type = value` model      |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)                                               | **Call Syntax**           | Section 6: Generic call syntax—unified `()` application, `[]` fully removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md)                                              | **Type System**           | Natural combination of Move semantics and generics                          |
| [RFC-024: spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md)                           | **Execution Model**       | DAG analysis and generic type check                                         |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)                                      | **Compiler Architecture** | Generics monomorphization and compile-time optimization strategy            |
| [Type Universe Thought](../reference/plan/ongoing/类型宇宙思想.md)                                         | **Theoretical Core**      | Type universe hierarchical model and value-dependent type design            |
| [RFC-027: Compile-Time Predicates and Unified Static Verification](./027-compile-time-evaluation-types.md) | **Termination Check**     | Automatic metric synthesis and compile-time evaluation safety guarantees    |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generics system is built upon the **Type Universe Thought**, a mental model that unifies
all concepts in the language into a layered structure. The core innovation is elevating
**value-dependent types** to first-class citizens at the Type2 layer.

### What are Value-Dependent Types?

**Value-dependent types** are types that depend on one or more **values** (rather than just other
types). These values can be evaluated at compile time, providing type safety guarantees during the
compilation phase.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Vec: (n: Int) -> Type  # Vector type depends on the length value n
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on the row and column counts
```

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core
advantages:

| Feature                 | Traditional Generics (C++/Rust)               | YaoXiang Value-Dependent Types                            |
| ----------------------- | --------------------------------------------- | --------------------------------------------------------- |
| Values types depend on  | Only type parameters                          | Can depend on any value, including function call results  |
| Compile-time evaluation | C++ template manual specialization, Rust none | Automatic compile-time evaluation, termination guaranteed |
| Type-level computation  | Template metaprogramming (complex/dangerous)  | Unified type-level computation engine                     |
| Type safety             | C++ none, Rust limited                        | Complete type safety, compile-time check                  |
| Dimension verification  | Runtime check or manual specialization        | Compile-time dimension verification, no runtime overhead  |

### Type Universe Hierarchy and Value-Dependent Types

The Type Universe Thought divides language concepts into different layers by semantic role, with
value-dependent types located at the **Type2 layer**:

| Level     | Role                                               | Example                                                                                              |
| --------- | -------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Type-1    | Values                                             | `42`, `factorial(5)`, function itself                                                                |
| Type0     | Meta-type keyword                                  | `Type`                                                                                               |
| Type1     | Concrete type                                      | `Int`, `String`, `Vec(3)`                                                                            |
| **Type2** | **Function/type constructor/value-dependent type** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at the Type2 layer use
**unified syntax**, all in the form `(params) -> result`:

- Regular function: `(Int, Int) -> Int` → return value is a value
- Type constructor: `(T: Type) -> Type` → return value is a type
- Value-dependent type: `(n: Int) -> Type` → return value is a type, but depends on value parameters

> **Curry-Howard Isomorphism**: This unification is no coincidence. The Curry-Howard Isomorphism
> states "types as propositions, programs as proofs"—function type `A → B` corresponds to the
> logical implication "if A then B", generics `(T: Type) -> Type` corresponds to universal
> quantification "for all types T", and value-dependent type `(n: Int) -> Type` corresponds to "for
> every integer n there exists a type". YaoXiang unifies functions, type constructors, and
> value-dependent types at the Type2 layer, essentially unifying "proof" and "computation" into the
> same concept—**constructive proof**. This is the Curry-Howard Isomorphism's direct embodiment in
> language design: one form (`(params) -> result`) simultaneously carries logical propositions and
> computational processes.

### Compile-Time Determinism Guarantee

YaoXiang's Type Universe Thought requires: **Everything at the Type layer is compile-time
determined**.

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

# Compile-time computation: factorial(3) = 6, vector size determined at compile time
vec: Vec(factorial(3)) = Vec(6)()
```

The compiler automatically:

1. Detects function calls at type positions
2. Performs compile-time termination check on functions (see the termination check mechanism below)
3. Executes evaluation at compile time
4. Embeds the result in the generated type

### Application Scenarios of Value-Dependent Types

#### Compile-Time Dimension Verification

```yaoxiang
# Matrix multiplication: compile-time dimension match verification
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # Compile-time check: a.Cols == b.Rows, otherwise compile error
    result = Matrix(T, Rows, M)()
    # ...
}

# Errors captured at compile time:
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

# Completely transparent when used, types auto-inferred
numbers = List(Int)()   # Errata: two-layer value construction form (see §9.1); elements filled with push
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature                                                      | C++ Templates          | Rust Generics | Haskell GADT   | **YaoXiang**                              |
| ------------------------------------------------------------ | ---------------------- | ------------- | -------------- | ----------------------------------------- |
| Type parameters                                              | ✅                     | ✅            | ✅             | ✅                                        |
| Value-dependent types                                        | ❌                     | ❌            | ✅             | ✅                                        |
| Compile-time evaluation                                      | Template instantiation | ❌            | ✅             | ✅                                        |
| Termination guarantee                                        | ❌                     | ❌            | ❌ (dangerous) | ✅ (automatic metric synthesis, RFC-027)  |
| Type safety                                                  | ❌ (macro expansion)   | ✅            | ✅             | ✅                                        |
| Unified syntax                                               | ❌                     | ❌            | ❌             | ✅                                        |
| Compile-time dimension verification                          | Manual specialization  | Runtime check | Type family    | Compile-time automatic verification       |
| Semi-automatic termination annotations (decreases/invariant) | ❌                     | ❌            | ❌             | ❌ (fully automatic at compile time only) |

### Termination Check Mechanism (Unified with RFC-027)

Compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the type
system will fall into infinite loops. The termination check is performed **fully automatically** by
RFC-027's compile-time proof pipeline—the compiler automatically synthesizes metrics; recursive/loop
constructs that can be proven pass, those that cannot be proven directly report a compile error.
**No loopholes for semi-automatic annotations**: RFC-022's `//! decreases`, `/*! invariant !*/` have
been deprecated along with RFC-022; the specification is the type annotation itself.

#### Termination Check for Recursive Functions

Before compile-time evaluation, the compiler checks whether the parameters of recursive calls
strictly decrease on every recursive path (RFC-027 §6.7). No specification comments are required:

```yaoxiang
# Compile-time factorial: no //! requires/ensures/decreases, compiler auto-analyzes
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreases → terminates
}

# Usage: called at type position, compiler verifies termination before evaluation
vec: Vec(factorial(5)) = Vec(120)()  # Compile-time evaluation factorial(5) = 120
```

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| Does not decrease / cannot determine decrease         | Compile error               |
| Runtime call (non-type position)                      | No termination check needed |

#### Termination Check for Loops

Loops do not require `: Invariant(...)` or `: decreases(...)` annotations. Refined type annotations
on variables (such as `UpTo(n)`) simultaneously provide the loop invariant and metric boundary. The
compiler attempts four metric synthesis strategies in priority order, stopping when one is found
(RFC-027 §7):

1. **Automatic linear rank function synthesis**—extract variable bounds from type annotations,
   enumerate linear combinations, SMT verifies m ≥ 0 and m' < m on all paths
2. **Predicate violation counting** (experimental)—extract violation_count from target type
   definition (e.g., `Sorted`), covering adjacent swap/move
3. **Bounded increase/decrease pattern**—`v += const` → metric `upper - v` (degenerate form of
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
│  Encounters function call at type position (e.g., Vec(factorial(5))) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Termination Check (RFC-027 proof pipeline, fully automatic) │
│     - Recursive functions: check parameters strictly decrease on each recursive path │
│     - Loops: four metric synthesis strategies (linear rank/violation count/bounded pattern/ │
│       multiplicative scaling), SMT verifies decrease         │
│     - Cannot prove → compile error (hard boundary, no semi-automatic annotation fallback) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-Time Evaluation (executed by built-in interpreter) │
│     - Pure function: direct evaluation                       │
│     - Side effect: compile error (type position must be side-effect-free) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result Embedded in Type                                 │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → concrete type                    │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensures compile-time evaluation must terminate, avoiding the type system falling into
  infinite loops
- **Unification**: Termination check and correctness verification (VC generation) share the same
  compile-time proof pipeline (RFC-027), no separate specification syntax
- **Fully automatic**: Compiler automatically synthesizes metrics from type annotations; provable
  ones pass, unprovable ones report errors—does not rely on programmers handwriting `decreases`

## Motivation

### Why Do We Need a Strong Generics System?

Current mainstream language generics have limitations:

| Language     | Generics Capability       | Problem                                                                                    |
| ------------ | ------------------------- | ------------------------------------------------------------------------------------------ |
| Java         | Bounded types             | Compile-time monomorphization, no generic specialization                                   |
| C#           | Generic constraints       | Runtime type checking, performance overhead                                                |
| Rust         | Generics + Trait          | Trait system complex, steep learning curve                                                 |
| C++          | Templates                 | Template specialization complex, poor compile error messages                               |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, compile-time dimension verification, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: Runtime flexibility vs compile-time optimization
2. **Complex vs Concise**: Powerful type system vs usability
3. **Macros vs Generics**: Macro code generation vs generic type safety
4. **Value-dependent vs Type-safe**: Traditional generics cannot verify dimensions at compile time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage                   | Description                                                                                                |
| --------------------------- | ---------------------------------------------------------------------------------------------------------- |
| **Type depends on value**   | `Vec: (n: Int) -> Type` allows types to depend on concrete values                                          |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, results directly embedded in type          |
| **Dimension verification**  | `Matrix(Float, 3, 3)` verifies matrix dimensions at compile time                                           |
| **Type-level computation**  | Conditional types like `If`, `Match` support type-level computation                                        |
| **Termination guarantee**   | Compile-time termination check (automatic metric synthesis) ensures compile-time evaluation must terminate |

```yaoxiang
# Compile-time verification not possible in C++/Rust
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# Compile-time computation: factorial(3) = 6, factorial(2) = 2
# Type is Matrix(Float, 6, 2)

# Dimension mismatch captured at compile time
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # Compile error: 2 != 3
```

### Value of the Generics System

```yaoxiang
# Example: Unified API design
# map operation for different container types

# Traditional approach: each type implemented separately
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

1. **Zero-Cost Abstractions** - Generic calls equivalent to concrete type calls
2. **Dead Code Elimination** - Compile-time analysis, only instantiate used generics
3. **Macro Replacement** - Generics replaces 90% of macro use cases
4. **Type Safety** - Compile-time check, no runtime type overhead
5. **IDE-Friendly** - Smart hints, clear error messages
6. **Value-Dependent Types** - Types can depend on values, supporting compile-time dimension
   verification
7. **Compile-Time Evaluation Safety** - Guarantees compile-time evaluation termination through
   compile-time termination check (RFC-027 automatic metric synthesis)

### Design Principles

- **Compile-time determined**: Generic parameters determined at compile time
- **Monomorphization priority**: Generate concrete code, avoid virtual function calls
- **Constraint-driven**: Type constraints guide instantiation
- **Platform optimization**: Specialization supports platform-specific optimization
- **Type universe unification**: Function/type constructor/value-dependent type unified to Type2
  layer
- **Termination guarantee**: Function calls at type positions must prove termination

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

# Generic constraint (direct expression, return can be omitted for single line)
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

`Type` is a language-level special entity. The compiler naturally identifies `Type` positions in
signatures and automatically infers and fills them from the actual argument types.

```yaoxiang
# Compiler auto-infers generic parameters
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         Type declaration  Constructor call: Int fills T, () value construction

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

# Usage point
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
# Omit Type arguments when inferable
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

# Sorting for generic containers
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # Implement sorting algorithm
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# Function type constraints
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
| **Clone**                | Deep copy: create complete independent copy                           | `value.clone()`                        | Any type implementing Clone       |

**Semantics of Dup**: Types implementing Dup do not transfer ownership on assignment/parameter
passing—the compiler copies the handle/token, with multiple holders pointing to the same underlying
data. This is complementary to the default Move semantics in the RFC-009 Ownership Model.

**Dup and Clone are Orthogonal Concepts**:

```
Dup = Copy handle, share data (modifications affect each other)
Clone = Copy data, independent copies (modifications don't affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not part of Dup
2. Dup  — only applicable to reference/token types and internally reference-counted types
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which Types are Dup**:

| Type                     | Dup     | Reason                                                            |
| ------------------------ | ------- | ----------------------------------------------------------------- |
| `&T` (borrow token)      | ✅      | Zero-size token, copy token = multiple views point to same data   |
| `ref T`                  | ✅      | Rc/Arc copy = reference count + 1, share heap data                |
| String, Bytes            | ✅      | Internal reference counting, copy handle shares underlying buffer |
| `&mut T` (mutable token) | ❌      | Linear exclusive, cannot copy                                     |
| struct                   | Derived | All fields Dup → struct Dup                                       |
| enum                     | Derived | All fields of all variants Dup → enum Dup                         |
| tuple                    | Derived | All elements Dup → tuple Dup                                      |
| Fn (closure)             | ❌      | Captured environment may not be Dup                               |
| `*T` (raw pointer)       | ❌      | unsafe, not participating in ownership system                     |

**Int/Float/Bool/Char are not Dup**—they are value types. The compiler automatically performs value
copying on assignment (the two values are completely independent). This is not "shallow copy", but
the compiler's built-in handling of primitives, not needed and should not be expressed through the
Dup type attribute.

```yaoxiang
# Primitive value types: compiler automatic value copy (not Dup)
x: Int = 42
y = x          # Value copy, x and y completely independent
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

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety guarantees are handled
> **fully automatically** by the `ref` keyword and compiler—`ref` automatically selects Rc or Arc,
> users don't need to understand Send/Sync.

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

### 4. Compile-Time Generics

#### 4.1 Compile-Time Value Parameters

> **Errata (#296)**: The original text stated "Int and other value parameters are default
> compile-time determined in generic contexts". This statement is **strictly wrong**—in
> `add: (a: Int, b: Int) -> Int = a + b`, `a`/`b` are runtime value parameters. Only concrete type
> parameters **referenced at type positions** are compile-time value parameters. The correct
> definition is below.

**Core Design**: `Type` in generic signatures marks type parameters; parameters annotated with
concrete types (`Int`/`Bool`/`Float`, etc.) are listed as **compile-time value parameter
candidates**. Whether they become compile-time value parameters depends on whether their values are
**referenced at type positions** (value-dependence). No `const` keyword is needed.

**Decision Rules (Two Steps)**:

1. **Form Coarse Filter**: Parameters annotated with concrete types other than `Type` (e.g., `Int`)
   → listed as candidates.
2. **Usage Fine Filter**: Candidate names appear at **type positions** (type body field types, inner
   `Fn` parameter types, `Assert` predicates, type construction argument positions like
   `Array(T, N)`) → confirmed as compile-time value parameters; otherwise treated as **runtime value
   parameters**.

| Notation                                                   | Decision                       | Reason                                                                |
| ---------------------------------------------------------- | ------------------------------ | --------------------------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime value parameters   | Only appear at value positions, not involved in type construction     |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time value parameter | N appears at the type construction argument position of `Array(T, N)` |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time value parameter | N is the type of inner parameter `k`                                  |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N fails (see below)            | N not referenced in type body, degrades to runtime value parameter    |

> **Essence of Value Dependency**: A compile-time value parameter is a value-dependent type—only
> when a value is used to **construct a type** does it need to be compile-time determined. Form
> (`: Int`) only determines candidate eligibility; usage (exposure at type position) determines
> whether it is a compile-time value parameter. This is the same root criterion as "function calls
> at type positions are evaluated at compile time" in §"Compile-Time Determinism Guarantee".

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time value parameter: N is referenced at type position (Array length slot)
# ════════════════════════════════════════════════════════
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N appears at type construction argument position → compile-time value parameter
    length: N,
}

# Usage: factorial(5) evaluated at type position (compile time), result 120 embedded in type
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120)

# ════════════════════════════════════════════════════════
# Value-dependence: N is the type of inner parameter k
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

> **Handling of Failed Candidates**: Candidates annotated with concrete types but not referenced at
> type positions (e.g., `N` of `Foo` in the above table) degrade to runtime value parameters. The
> function-level path already handles this; failed candidates in the type constructor path are
> currently silently discarded, leading to inconsistent instantiation error reports—see issue
> [#297](https://github.com/ChenXu233/YaoXiang/issues/297).

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-Time Computation Example
# ════════════════════════════════════════════════════════

# Compiler evaluates function calls of literal types at compile time
SIZE: Int = factorial(5)  # Compile-time 120

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

### Never and Void: ⊥ and ⊤ in the Type System

YaoXiang's type system has both ⊥ (false/empty type) and ⊤ (true/Unit) in the Curry-Howard
Isomorphism, carried by the two built-in type names `Never` and `Void`:

**Never (⊥)** — Three non-negotiable kernel properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. This is a
   meta-level property and must be built-in.
2. **Ex falso quodlibet**: `Never <: T` holds for any type `T`. A `Never` value can be used as any
   type—this is why code after `assert(false)` still passes type check (although never executed).
3. **Divergence marker**: `f: (...) -> Never` indicates that `f` is guaranteed not to return. The
   compiler performs dead code analysis based on this.

`Never` is a built-in type name, not a keyword; the parser is unaware of it. Empty sum and type
literal syntax are not exposed.

**Void (⊤, i.e., Unit)** — Has exactly one inhabitant (default void value), the carrier of the true
proposition "always true". `Void` is the identity of zero-field product type, `Never` is the
identity of zero-variant sum type—the two are dual. `x: Void = <default>` is legal; `x: Never = ...`
has no right-hand side to write.

#### 4.3 Compile-Time Verification (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard Library Implementation: Using Conditional Types
# ════════════════════════════════════════════════════════

# Standard library definition
# IsTrue: Bridge from value universe to type universe—Bool truth value mapped to type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has value, program continues
    false => Never,    # ⊥, no value, diverges
}

# Assert: Compile-time refined type primitive—type-level expression of Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error/divergence)
# cond undecidable → proof pipeline decides according to dispatch mode:
#                  CompileTime → Unknown, requires prove
#                  Runtime     → insert check, inject Γ assumption

# Usage 1: as constraint in type definition
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at type position)
    length: Assert(N > 0),
}

# Usage 2: used in expression
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# Verify: size of IntArray(10) equals sizeof(Int) * 10
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

### 5. Conditional Types

> **Curry-Howard Isomorphism**: Conditional types are **case analysis** in logic from the
> Curry-Howard perspective. The `Bool` type corresponds to a proposition with two possible values
> (True/False), and `If` chooses different results based on whether the proposition is true or
> false—this is precisely case disjunction in logic. `match C { True => T, False => E }` essentially
> expresses: "given proposition C is True, the conclusion is T; given C is False, the conclusion is
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

# Compile-time verification (unified with the Assert definition in §4.3)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Families

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as
> types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level",
> but **constructing a proposition about the addition of natural numbers**. `(Zero, B) => B` states
> "proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` states "if
> Add(A', B) holds, then Add(Succ(A'), B) also holds". This is the addition definition itself in
> Peano axioms. The type checker verifying this match expression passes is equivalent to verifying
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

### 6. Function Overload Specialization

#### 6.1 Basic Specialization

```yaoxiang
# Basic specialization: using function overloading (compiler auto-selects)
sum: (arr: Array(Int)) -> Int = {
    # Compiled to more efficient code
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
# Specialization method fully conforming to RFC-010 syntax: function overloading

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

# Completely transparent when used
int_arr = Array(Int)(1, 2, 3)
float_arr = Array(Float)(1.0, 2.0, 3.0)

# Compiler auto-selects optimal specialization
sum(int_arr)     # Selects sum: (Array(Int)) -> Int
sum(float_arr)    # Selects sum: (Array(Float)) -> Float
```

#### 6.3 Perfect Combination of Function Overloading and Inlining

**Key Feature**: Function overloading and inlining optimization naturally combine to achieve
zero-cost abstractions.

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

# ======== After Compilation (Equivalent Code) ========
# Compiler auto-selects optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core Advantages**:

1. **Compiler Smart Selection**

   ```yaoxiang
   sum(int_arr)      # Auto-selects sum: (Array(Int)) -> Int
   sum(float_arr)    # Auto-selects sum: (Array(Float)) -> Float
   sum(custom_arr)  # Auto-selects sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **Inlining Optimization**
   - Small functions auto-inlined to call site
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type Safety**
   - Compile-time type check
   - Zero runtime overhead
   - No virtual function table needed

4. **Perfect Fit with RFC-010**

   ```yaoxiang
   # Completely using unified syntax
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
fibonacci(10)      # Selects Int version, completely inlined
fibonacci(10.5)    # Selects Float version, uses Binet's formula
```

**What does this mean?**

- ✅ **Generic specialization** → naturally solved by function overloading
- ✅ **Performance optimization** → inlining automatically completed
- ✅ **Code reuse** → one function name, multiple implementations
- ✅ **Zero-cost abstractions** → compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords needed** → perfectly conforms to RFC-010 unified syntax

````

### 7. Dead Code Elimination Mechanism

#### 7.1 Instantiation Graph Analysis

```rust
// Compiler internal: build generic instantiation dependency graph
struct InstantiationGraph {
    // Node: generic instantiation
    nodes: HashMap<InstanceKey, InstanceNode>,

    // Edge: usage relation
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

#### 7.2 Usage Point Analysis

```yaoxiang
# Source code analysis
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Usage point 1: instantiate map(Int, Int)
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # Requires map[Int, Int]

# Usage point 2: instantiate map(String, String)
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Requires map[String, String]

# Unused: map[Float, Float], etc.
# These generic instances will not be generated

# After compilation only contains used instances
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
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # Two layers: type arguments + constructor arguments
# Errata: Earlier version wrote Array(Int, 10)(1, 2, 3, ...) (elements spread directly),
# inconsistent with the authoritative pattern in §9.3 (Type(arguments)(field constructor arguments)/empty construction),
# unified to field-name style constructor arguments, see SPEC type-system.md §4.3.
arr_100_int = Array(Int, 100)()   # Empty construction, data assigned after

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

# ✅ Generic approach: auto-derivation
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

# Compile-time verification
AssertAddable: (A: Type, B: Type) -> Type = If(Add(A, B) != TypeError, (A, B), compile_error("Cannot add"))

# Usage
result_type = Add[Int, Float]  # Inferred as Float
```

### 9. Examples

#### 9.1 Complete Generic Container Example

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
    data: Array(T),
    length: Int,

    # Generic method (T is automatically brought into scope from outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement Generic Methods ========
# Function defined under List namespace (List. prefix = namespace attribution)
# To make . call syntax like list.push(item) work, need explicit binding: List.push = push[0]
# self is just a convention parameter name, compiler looks at type not name

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

# ======== 3. Type Constraint Usage ========
# Implement Clone for List
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. Usage Example ========
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
# ======== 1. Generic Sorting Algorithm ========
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

# ======== 2. IntComparator Implementation ========
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

# ======== 3. Usage Example ========
# Sort Int array
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# Sort String array (needs StringComparator)
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-Time Generic Example

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

# ======== 3. Usage Example ========
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

1. **Zero-Cost Abstractions**
   - Compile-time monomorphization, no runtime overhead
   - No virtual functions, no RTTI

2. **Dead Code Elimination**
   - Compile-time analysis, only instantiate used generics
   - Code bloat controllable

3. **Macro Replacement**
   - Type-safe code generation
   - IDE-friendly, clear error messages

4. **Compile-Time Computation**
   - Compile-time generics supports compile-time computation
   - Features like dimension verification
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compilation Time**
   - Generic instantiation increases compilation time
   - Constraint solving may be slow

2. **Memory Footprint**
   - Compiler memory usage increased
   - Caching mechanism needs memory

3. **Implementation Complexity**
   - Constraint solver complex
   - Type-level computation engine complex

4. **Error Diagnostics**
   - Generic errors may be complex
   - Need clear error prompts

### Mitigation Measures

1. **Caching Strategy**
   - Instantiation result cache
   - LRU cache limits memory

2. **Incremental Compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error Prompts**
   - Clear error messages
   - Generic argument inference prompts

4. **Parallel Compilation**
   - Parallel instantiate generics
   - Multi-threaded constraint solving

## Alternative Approaches

| Approach                    | Why Not Choose                      |
| --------------------------- | ----------------------------------- |
| Only basic generics         | Cannot replace complex macros       |
| Pure macro system           | No type safety, poor error messages |
| Only dependency constraints | Insufficient flexibility            |
| Runtime generics            | Has performance overhead            |

### Risks

| Risk                          | Impact                     | Mitigation Measures         |
| ----------------------------- | -------------------------- | --------------------------- |
| Constraint solving complexity | Compilation time too long  | Incremental solving + cache |
| Code bloat                    | Binary file too large      | DCE + threshold control     |
| Implementation complexity     | Development cycle extended | Phased implementation       |
| Error diagnostics             | Poor user experience       | Detailed error messages     |

## Open Questions

### Pending Decisions

| Topic                  | Description                        | Status          |
| ---------------------- | ---------------------------------- | --------------- |
| Instantiation strategy | Eager vs Lazy vs Threshold         | To be discussed |
| Cache size             | LRU cache capacity setting         | To be discussed |
| Error diagnostics      | Generic error message detail level | To be discussed |

### Future Optimizations

| Optimization Item             | Value  | Implementation Difficulty |
| ----------------------------- | ------ | ------------------------- |
| Instantiation graph analysis  | High   | Medium                    |
| Type-level programming DSL    | Medium | High                      |
| Generic performance benchmark | Medium | Low                       |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use unified () syntax, as part of function type
# E.g., map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# Type constraint (in generic arguments)
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# Parameter declaration (type + name)
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# Function declaration: name: type = expression
# Generic arguments are the first argument group in function type: (T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# Method declaration: Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# Type definition (unified Binding syntax)
# Generic type e.g., List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# Type in generic arguments is automatically filled by compiler from argument types
# E.g., map(numbers, f), T extracted from numbers: List(Int), R extracted from f: (Int) -> String
```

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current Status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review│  ← Open community discussion and feedback
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
│ (Formal Design)│ │ (Keep in Place)│
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documents

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./accepted/009-ownership-model.md)
- [RFC-001: Concurrent Model](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)
- [tutorial/ Tutorial](../../../../../tutorial/)

### External References

- [Rust Generics System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
