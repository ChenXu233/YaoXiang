---
title: "RFC-011: Generics System Design - Zero-Cost Abstraction and Macro Replacement"
status: "Accepted"
author: "晨煦 (Chenxu)"
updated: "2026-07-15 (Type body code blocks + compile-time contracts + effect seeds implemented)"
issue: "#128"
issues_impl:
  - "#45"
  - "#46"
  - "#73"
  - "#90"
  - "#96"
  - "#40"
  - "#151"
pr_impl:
  - "#122"
---

# RFC-011: Generics System Design - Zero-Cost Abstraction and Macro Replacement

## Abstract

This document defines the **generics system design** of the YaoXiang language, achieving zero-cost abstraction through powerful generics capabilities, leveraging compile-time optimization to reduce dependency on macros, and providing dead code elimination mechanisms.

**Core design**:
- **Unified signature syntax**: `(T: Type, R: Type) -> ...` generics parameters unified with regular parameters
- **Type self-description mechanism**: `Type` is a language-level special existence; `Type` positions in signatures can be auto-inferred and filled
- **Type constraints**: `T: Dup + Add` multiple constraints, function type constraints
- **Associated types**: `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time generics**: `N: Int` generic value parameters, compile-time constant instantiation
- **Conditional types**: `If: (C: Bool, T: Type, E: Type) -> Type` type-level computation, type family

**Value**:
- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro use cases
- Type safety: compile-time checks, IDE-friendly
- **Explicit is better than implicit**: `Type` self-describes, compiler auto-infers

## Reference Documents

This document's design is based on the following documents:

| Document | Relationship | Description |
|------|------|------|
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) | **Syntax foundation** | Generics syntax integrates with unified `name: type = value` model |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) | **Call syntax** | Section 6: Generic call syntax—unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md) | **Type system** | Natural combination of Move semantics and generics |
| [RFC-001: Concurrent Model](./accepted/001-concurrent-model-error-handling.md) | **Execution model** | DAG analysis and generics type checking |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md) | **Compiler architecture** | Generics monomorphization and compile-time optimization strategy |
| [Type Universe Thought](../reference/plan/ongoing/类型宇宙思想.md) | **Theoretical core** | Type universe hierarchy model and value-dependent type design |
| [RFC-022: Hoare Logic Static Verification](./draft/022-hol-logic-verification.md) | **Termination check** | decreases contract and compile-time evaluation safety guarantee |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generics system is built upon the **type universe thought**, a mental model that unifies all language concepts into a hierarchical structure. The core innovation is elevating **value-dependent types** to first-class citizens at the Type2 layer.

### What Are Value-Dependent Types?

**Value-dependent types** are types that depend on one or more **values** (not just other types). These values can be evaluated at compile-time, providing type safety guarantees at the compilation stage.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Vec: (n: Int) -> Type  # Vector type depends on length value n
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on row and column count
```

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core advantages:

| Feature | Traditional Generics (C++/Rust) | YaoXiang Value-Dependent Types |
|------|-------------------|---------------------|
| Values that types depend on | Only type parameters | Any value, including function call results |
| Compile-time evaluation | C++ template manual specialization, Rust: none | Automatic compile-time evaluation with termination guarantee |
| Type-level computation | Template metaprogramming (complex/dangerous) | Unified type-level computation engine |
| Type safety | C++: none, Rust: limited | Complete type safety, compile-time checks |
| Dimensional verification | Runtime check or manual specialization | Compile-time dimensional verification, no runtime overhead |

### Type Universe Hierarchy and Value-Dependent Types

The type universe thought divides language concepts by semantic role into different layers, with value-dependent types located at the **Type2 layer**:

| Layer | Role | Example |
|------|------|------|
| Type-1 | Values | `42`, `factorial(5)`, functions themselves |
| Type0 | Meta-type keyword | `Type` |
| Type1 | Concrete types | `Int`, `String`, `Vec(3)` |
| **Type2** | **Functions/type constructors/value-dependent types** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key design**: Functions, type constructors, and value-dependent types at the Type2 layer share **unified syntax**, all in the form of `(params) -> result`:
- Regular function: `(Int, Int) -> Int` → return value is a value
- Type constructor: `(T: Type) -> Type` → return value is a type
- Value-dependent type: `(n: Int) -> Type` → return value is a type, but depends on value parameters

> **Curry-Howard isomorphism**: This unification is no coincidence. The Curry-Howard isomorphism states "types as propositions, programs as proofs"—the function type `A → B` corresponds to the logical implication "if A then B", generics `(T: Type) -> Type` corresponds to universal quantification "for all types T", and value-dependent types `(n: Int) -> Type` corresponds to "for each integer n there exists a type". YaoXiang unifies functions, type constructors, and value-dependent types at the Type2 layer, essentially unifying "proof" and "computation" into the same concept—**constructive proof**. This is the direct embodiment of the Curry-Howard isomorphism in language design: one form (`(params) -> result`) simultaneously carries logical propositions and computational processes.

### Compile-Time Determinism Guarantee

YaoXiang's type universe thought requires: **everything at the Type layer is determined at compile-time**.

```yaoxiang
# Compile-time dimensional verification example
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # Compile-time check: dimensions must be positive
    _assert: Assert[Rows > 0],
    _assert: Assert[Cols > 0],
}

# Create 3x3 identity matrix - completed at compile-time
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# Compile-time computation: factorial(3) = 6, vector size determined at compile-time
vec: Vec(factorial(3)) = Vec(6)()
```

The compiler automatically:
1. Detects function calls at type positions
2. Verifies whether the function is marked with a `decreases` contract (see termination check mechanism below)
3. Executes evaluation at compile-time
4. Embeds the result into the generated type

### Application Scenarios for Value-Dependent Types

#### Compile-Time Dimensional Verification
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

# N is a compile-time constant, can be used for type-level computation
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3 (known at compile-time)
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

# Completely transparent when used, types auto-inferred
numbers = List(1, 2, 3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature | C++ Templates | Rust Generics | Haskell GADT | **YaoXiang** |
|------|---------|----------|--------------|--------------|
| Type parameters | ✅ | ✅ | ✅ | ✅ |
| Value-dependent types | ❌ | ❌ | ✅ | ✅ |
| Compile-time evaluation | Template instantiation | ❌ | ✅ | ✅ |
| Termination guarantee | ❌ | ❌ | ❌ (dangerous) | ✅ (decreases contract) |
| Type safety | ❌ (macro expansion) | ✅ | ✅ | ✅ |
| Unified syntax | ❌ | ❌ | ❌ | ✅ |
| Compile-time dimensional verification | Manual specialization | Runtime check | Type family | Automatic compile-time verification |
| decreases contract | ❌ | ❌ | ❌ | ✅ |

### Termination Check Mechanism (Integrated with RFC-022)

The compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the type system will fall into infinite loops. YaoXiang ensures this through the **decreases contract**, seamlessly integrating with RFC-022.

#### Termination Contract for Recursive Functions
```yaoxiang
# Compile-time factorial: must prove termination
factorial: (n: Int) -> Int = {
    //! requires: n >= 0
    //! ensures: result == n!
    //! decreases: n    # n strictly decreases on each recursion
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Usage: called at type position
vec: Vec(factorial(5)) = Vec(120)()  # Compile-time evaluate factorial(5) = 120
```

#### Termination Contract for Loops
```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n {
        /*! invariant: s == sum(arr[0..i]) && 0 <= i <= n !*/
        /*! decreases: n - i !*/
        s += arr[i]; i += 1
    }
    return s
}
```

#### Termination Check Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  Type checking phase                                        │
│  Encounters function call at type position (e.g. Vec(factorial(5))) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Check decreases contract                                │
│     - Has decreases: verify decreasing condition holds on all recursion paths │
│     - No decreases but obviously terminating: evaluate directly │
│     - No decreases and possibly non-terminating: compile error │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time evaluation (executed by builtin interpreter) │
│     - Pure function: evaluate directly                      │
│     - Side effects: compile error (type position must be side-effect-free) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Result embedded into type                               │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → concrete type                  │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: ensures compile-time evaluation necessarily terminates, avoiding infinite loops in the type system
- **Unification**: termination check shares the same contract mechanism as partial correctness verification
- **Progressive enhancement**: can transition from runtime checks to complete static proof

## Motivation

### Why Do We Need a Strong Generics System?

Current mainstream languages have limitations in generics:

| Language | Generics Capability | Problem |
|------|----------|------|
| Java | Bounded types | Compile-time erasure, no generics specialization |
| C# | Generics constraints | Runtime type checking, performance overhead |
| Rust | Generics + Trait | Complex Trait system, steep learning curve |
| C++ | Templates | Complex template specialization, poor compile error messages |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, compile-time dimensional verification, termination guarantee** |

### Core Contradictions

1. **Performance vs flexibility**: runtime flexibility vs compile-time optimization
2. **Complex vs simple**: powerful type system vs ease of use
3. **Macros vs generics**: macro code generation vs generics type safety
4. **Value-dependence vs type safety**: traditional generics cannot verify dimensions at compile-time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are a core advantage over traditional generics:

| Advantage | Description |
|------|------|
| **Types depend on values** | `Vec: (n: Int) -> Type` lets types depend on concrete values |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile-time, results embedded directly into types |
| **Dimensional verification** | `Matrix(Float, 3, 3)` verifies matrix dimensions at compile-time |
| **Type-level computation** | `If`, `Match` and other conditional types support type-level computation |
| **Termination guarantee** | decreases contract ensures compile-time evaluation necessarily terminates |

```yaoxiang
# Compile-time verification impossible in C++/Rust
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# Compile-time computation: factorial(3) = 6, factorial(2) = 2
# Type is Matrix(Float, 6, 2)

# Dimension mismatch caught at compile-time
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # Compile error: 2 != 3
```

### Value of the Generics System

```yaoxiang
# Example: unified API design
# map operation for different container types

# Traditional approach: separate implementation for each type
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
3. **Macro replacement** - generics replace 90% of macro use cases
4. **Type safety** - compile-time checking, no runtime type overhead
5. **IDE-friendly** - smart hints, clear error messages
6. **Value-dependent types** - types can depend on values, support compile-time dimensional verification
7. **Compile-time evaluation safety** - guarantees compile-time evaluation terminates through decreases contract

### Design Principles

- **Compile-time determined**: generics parameters determined at compile-time
- **Monomorphization first**: generate concrete code, avoid virtual function calls
- **Constraint-driven**: type constraints guide instantiation
- **Platform optimization**: specialization supports platform-specific optimization
- **Type universe unification**: functions/type constructors/value-dependent types unified at Type2 layer
- **Termination guarantee**: function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise they will be inferred by HM as functions.
>
> | Notation | Meaning |
> |------|------|
> | `List: (T: Type) -> Type = {...}` | ✅ Type constructor |
> | `List = {...}` | ❌ HM infers as function, not type |

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

# Generic constraint (direct expression, single-line may omit return)
clone: (T: Clone)(value: T) -> T = value.clone()

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### Generic Function Call Syntax

#### 1.1 Unified Signature Syntax

```yaoxiang
# Generic functions use unified (T: Type, R: Type) signature syntax
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type Self-Description Mechanism

`Type` is a language-level special existence; the compiler naturally recognizes `Type` positions in signatures and auto-infers filling from actual argument types.

```yaoxiang
# Compiler auto-infers generics parameters
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         Type declaration  Construction call: Int fills T

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

```yaoxiang
# Omit Type parameters when inferable
numbers: List(Int) = List(Int)
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

# Generic container sorting
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

#### 2.4 Builtin Marker Traits: Dup and Clone

**Three categories of copy semantics**:

| Type | Meaning | Trigger | Use Case |
|------|------|----------|----------|
| **Primitive value copy** | Auto value copy on assignment, two values completely independent | Auto on assignment/parameter passing | Int, Float, Bool, Char |
| **Dup** | Shallow copy: copy handle/token, underlying data shared | Auto on assignment/parameter passing | `&T` token, `ref T`, String/Bytes |
| **Clone** | Deep copy: create complete independent replica | `value.clone()` | Any type implementing Clone |

**Dup semantics**: Types implementing Dup do not transfer ownership on assignment/parameter passing—the compiler copies the handle/token, with multiple holders pointing to the same underlying data. This complements the Move default semantics in RFC-009 ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = copy handle, share data (modifications affect each other)
Clone = copy data, replica independent (modifications don't affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler built-in value copy, not Dup
2. Dup — only applicable to reference/token types and types with internal reference counting
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which types are Dup**:

| Type | Dup | Reason |
|------|-----|------|
| `&T` (borrow token) | ✅ | Zero-sized token, copying token = multiple views point to same data |
| `ref T` | ✅ | Rc/Arc copy = reference count +1, share heap data |
| String, Bytes | ✅ | Internal reference counting, copy handle shares underlying buffer |
| `&mut T` (mutable token) | ❌ | Linearly exclusive, cannot copy |
| struct | Derived | All fields Dup → struct Dup |
| enum | Derived | All variants' all fields Dup → enum Dup |
| tuple | Derived | All elements Dup → tuple Dup |
| Fn (closure) | ❌ | Captured environment may not be Dup |
| `*T` (raw pointer) | ❌ | unsafe, doesn't participate in ownership system |

**Int/Float/Bool/Char are not Dup**—they are value types, the compiler auto-copies on assignment (two values completely independent). This is not "shallow copy", but the compiler's built-in handling of primitives, neither needing nor should be expressed through the Dup type attribute.

```yaoxiang
# Primitive value types: compiler auto value copy (not Dup)
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

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task safety guarantees are fully automatically handled by the `ref` keyword and compiler—`ref` auto-selects Rc or Arc, users don't need to understand Send/Sync.

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
    IteratorType: Iterator(Item),  # Associated types are also generic
    iter: (Self) -> IteratorType,
}

# Usage
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. Compile-Time Generics

#### 4.1 Compile-Time Constant Parameters

**Core design**: `Type` in generics signatures marks compile-time type parameters; value parameters like `Int` are default compile-time determinable in generics context. No `const` keyword needed.

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time constant parameters: Int in generics defaults to compile-time determined
# ════════════════════════════════════════════════════════

# Compile-time factorial: N must be a compile-time known literal
factorial: (N: Int) -> (n: N) -> Int = {
    return match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# Compile-time addition
add: (a: Int, b: Int) -> (a: a, b: b) -> Int = a + b

# ════════════════════════════════════════════════════════
# Compile-time constant arrays
# ════════════════════════════════════════════════════════
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # Array with compile-time known size
    length: N,
}

# Usage
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120), compiler computes at compile-time
```

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation examples
# ════════════════════════════════════════════════════════

# Compiler computes function calls of literal types at compile-time
SIZE: Int = factorial(5)  # 120 at compile-time

# Matrix type usage
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time dimensional verification
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

YaoXiang's type system simultaneously possesses ⊥ (false/empty type) and ⊤ (true/Unit) in the Curry-Howard isomorphism, carried by two builtin type names `Never` and `Void`:

**Never (⊥)** — three non-negotiable kernel properties:

1. **Zero constructors**: No literal or expression can produce a value of type `Never`. This is a meta-level property and must be builtin.
2. **Explosion principle**: `Never <: T` holds for any type `T`. A `Never` value can be used as any type—this is why code after `assert(false)` still passes type checking (though never executed).
3. **Divergence marker**: `f: (...) -> Never` indicates `f` is guaranteed not to return. The compiler uses this for dead code analysis.

`Never` is a builtin type name, not a keyword, the parser doesn't care. Empty and type literal syntax is not opened up.

**Void (⊤, i.e. Unit)** — exactly one inhabitant (default void value), is the carrier of the true proposition "always true". `Void` is the unit of zero-field product types, `Never` is the unit of zero-variant sum types—the two are dual. `x: Void = <default>` is legal, `x: Never = ...` has no right side to write.

#### 4.3 Compile-Time Verification (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: leveraging conditional types
# ════════════════════════════════════════════════════════

# Standard library definitions
# IsTrue: bridge from value universe to type universe—Bool true value maps to type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, has value, program continues
    false => Never,    # ⊥, no value, diverges
}

# Assert: compile-time refinement type primitive—type-level expression of Bool proposition
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond is true  → Assert(true)  = Void    (always true, erased)
# cond is false → Assert(false) = Never   (always false, compile error/divergence)
# cond undecidable → proof pipeline decides by dispatch mode:
#                    CompileTime → Unknown, require prove
#                    Runtime     → insert check, inject Γ assumption

# Usage 1: as constraint in type definition
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at type position)
    length: Assert(N > 0),
}

# Usage 2: in expression
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# Verify: IntArray(10) size equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generics Specialization

```yaoxiang
# Small array optimization: use function overloading for compile-time generics specialization

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

> **Curry-Howard isomorphism**: Conditional types, viewed through Curry-Howard, are **case analysis** in logic. The `Bool` type corresponds to a proposition with two possible values (True/False), `If` chooses different results based on the truth of that proposition—this is precisely case disjunction in logic. `match C { True => T, False => E }` actually expresses: "when the known proposition C is True, the conclusion is T, when C is False, the conclusion is E".

#### 5.1 If Conditional Type

```yaoxiang
# Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# Examples: compile-time branches
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# Compile-time verification (unified with §4.3 Assert definition)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Family

> **Curry-Howard isomorphism**: Type family is the most direct embodiment of "propositions as types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level", but **constructing a proposition about natural number addition**. `(Zero, B) => B` says "the proposition Add(Zero, B) is equivalent to B", `(Succ(A'), B) => Succ(Add(A', B))` says "if Add(A', B) holds, then Add(Succ(A'), B) also holds". This is the addition definition itself in Peano axioms. The type checker verifying this match expression passes is equivalent to verifying the logical consistency of this definition.

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

### 6. Function Overloading Specialization

#### 6.1 Basic Specialization

```yaoxiang
# Basic specialization: using function overloading (compiler auto-selects)
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

**Key feature**: Function overloading naturally combines with inlining optimization to achieve zero-cost abstraction.

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

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core advantages**:

1. **Compiler intelligent selection**
   ```yaoxiang
   sum(int_arr)      # Auto-selects sum: (Array(Int)) -> Int
   sum(float_arr)    # Auto-selects sum: (Array(Float)) -> Float
   sum(custom_arr)  # Auto-selects sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **Inlining optimization**
   - Small functions auto-inlined to call site
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function tables needed

4. **Perfect fit with RFC-010**
   ```yaoxiang
   # Completely using unified syntax
   name: type = value
   # No new keywords like impl, where needed
   ```

**Practical application examples**:

```yaoxiang
# Performance-sensitive numerical computation
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Using Binet's formula
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# Compiler auto-selects and inlines
fibonacci(10)      # Selects Int version, fully inlined
fibonacci(10.5)    # Selects Float version, uses Binet's formula
```

**What does this mean?**

- ✅ **Generics specialization** → Function overloading solves it naturally
- ✅ **Performance optimization** → Inlining auto-completed
- ✅ **Code reuse** → One function name, multiple implementations
- ✅ **Zero-cost abstraction** → Compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords needed** → Perfectly conforms to RFC-010 unified syntax
```

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
```

#### 7.2 Use Site Analysis

```yaoxiang
# Source code analysis
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Use site 1: instantiate map(Int, Int)
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # Requires map[Int, Int]

# Use site 2: instantiate map(String, String)
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Requires map[String, String]

# Unused: map[Float, Float], etc.
# These generics instances will not be generated

# After compilation, only used instances are included
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
arr_10_int = Array(Int, 10)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array(Int, 100)(...)

# After compilation, only used sizes are generated
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
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # Instantiate map(Int, Int)

# Module C
# C.yx
use A.{map}
string_list = List("a", "b", "c")
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

    // 6. Run optimization
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

# ✅ Generics approach: auto derive
# Using function overloading for auto-derive
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

# ✅ Generics approach: conditional types
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

    # Generic methods (T auto-brought into scope by outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generic methods ========
# Function defined under List namespace (List. prefix = namespace ownership)
# To make list.push(item) . call syntax work, need explicit binding: List.push = push[0]
# self is just conventional parameter name, compiler looks at type not name

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # Resize
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
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# Sort String array (needs StringComparator)
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 Compile-Time Generics Example

```yaoxiang
# ======== 1. Compile-time matrix type ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # Compile-time dimensional verification: using Assert standard library type
    _assert: Assert[Rows > 0],  # Rows > 0, otherwise compile error
    _assert: Assert[Cols > 0],  # Cols > 0, otherwise compile error

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
   - Compile-time generics supports compile-time computation
   - Dimensional verification and other features
   - No `const` keyword needed, pure type constraints

### Disadvantages

1. **Compile time**
   - Generics instantiation increases compile time
   - Constraint solving may be slow

2. **Memory usage**
   - Compiler memory usage increases
   - Caching mechanism needs memory

3. **Implementation complexity**
   - Complex constraint solver
   - Complex type-level computation engine

4. **Error diagnosis**
   - Generics errors may be complex
   - Need clear error messages

### Mitigation Measures

1. **Caching strategy**
   - Cache instantiation results
   - LRU cache limits memory

2. **Incremental compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error messages**
   - Clear error information
   - Generics parameter inference hints

4. **Parallel compilation**
   - Parallel instantiation of generics
   - Multi-threaded constraint solving

## Alternatives

| Approach | Why Not Chosen |
|------|--------------|
| Basic generics only | Cannot replace complex macros |
| Pure macro system | No type safety, poor error messages |
| Only dependent constraints | Insufficient flexibility |
| Runtime generics | Has performance overhead |

### Risks

| Risk | Impact | Mitigation |
|------|------|----------|
| Constraint solving complexity | Compile time too long | Incremental solving + caching |
| Code bloat | Binary file too large | DCE + threshold control |
| Implementation complexity | Extended development cycle | Phased implementation |
| Error diagnosis | Poor user experience | Detailed error messages |

## Open Questions

### Pending Issues

| Topic | Description | Status |
|------|------|------|
| Instantiation strategy | Eager vs Lazy vs Threshold | To be discussed |
| Cache size | LRU cache capacity setting | To be discussed |
| Error diagnosis | Generics error information detail level | To be discussed |

### Future Optimizations

| Optimization | Value | Implementation Difficulty |
|--------|------|----------|
| Instantiation graph analysis | High | Medium |
| Type-level programming DSL | Medium | High |
| Generics performance benchmark | Medium | Low |

## Appendix

### Syntax BNF

```bnf
# Generics parameters use unified () syntax, as part of function type
# e.g. map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# Type constraints (in generics parameters)
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
# Generic type like List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# Type in generics parameters is auto-filled by compiler from actual argument types
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
│ (Formal design) │ (Stay in place) │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documents

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-009: Ownership Model](./accepted/009-ownership-model.md)
- [RFC-001: Concurrent Model](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md)
- [Language Specification](../language-spec.md)
- [YaoXiang Guide](../guides/YaoXiang-book.md)

### External References

- [Rust Generics System](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ Template Specialization](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell Type Classes](https://www.haskell.org/tutorial/classes.html)
- [Swift Generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [Monomorphization Optimization](https://llvm.org/docs/Monomorphization.html)
- [Dead Code Elimination](https://en.wikipedia.org/wiki/Dead_code_elimination)