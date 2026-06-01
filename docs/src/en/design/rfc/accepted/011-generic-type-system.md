```markdown
---
title: "RFC-011: Generic System Design"
---

# RFC-011: Generic System Design - Zero-Cost Abstraction and Macro Replacement

> **Status**: Accepted
> **Author**: MorningX
> **Created**: 2025-01-25
> **Last Updated**: 2026-04-22 (Updated with Type self-description mechanism, unified generic call syntax)

## Abstract

This document defines the **generic system design** of the YaoXiang language, achieving zero-cost abstraction through powerful generic capabilities, using compile-time optimization to reduce reliance on macros, and providing dead code elimination mechanisms.

**Core Design**:
- **Unified signature syntax**: `(T: Type, R: Type) -> ...` - generic parameters unified with ordinary parameters
- **Type self-description mechanism**: `Type` is a language-level special entity; `Type` positions in signatures are automatically inferred and filled
- **Type constraints**: `T: Dup + Add` - multiple constraints, function type constraints
- **Associated types**: `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **Compile-time generics**: `N: Int` - generic value parameters, compile-time constant instantiation
- **Conditional types**: `If: (C: Bool, T: Type, E: Type) -> Type` - type-level computation, type families

**Value**:
- Zero-cost abstraction: Compile-time monomorphization, no runtime overhead
- Dead code elimination: Instantiation graph analysis + LLVM optimization
- Macro replacement: Generics replace 90% of macro use cases
- Type safety: Compile-time checking, IDE-friendly
- **Explicit over implicit**: `Type` self-description, compiler automatic inference

## Reference Documents

This document's design is based on the following documents:

| Document | Relationship | Description |
|----------|--------------|-------------|
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) | **Syntax foundation** | Generic syntax integrated with unified `name: type = value` model |
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) | **Call syntax** | Section 6: Generic call syntax - unified `()` application, `[]` completely removed |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md) | **Type system** | Natural combination of Move semantics and generics |
| [RFC-001: Concurrent Model](./accepted/001-concurrent-model-error-handling.md) | **Execution model** | DAG analysis and generic type checking |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md) | **Compiler architecture** | Generic monomorphization and compile-time optimization strategies |
| [Type Universe Thought](../reference/plan/ongoing/类型宇宙思想.md) | **Theoretical core** | Type universe hierarchical model and value-dependent type design |
| [RFC-022: Hoare Logic Static Verification](./draft/022-hol-logic-verification.md) | **Termination checking** | decreases specification and compile-time evaluation safety guarantees |

## Type Universe Thought and Value-Dependent Types

YaoXiang's generic system is built on **Type Universe Thought**, a mental model that unifies all concepts in the language into a layered structure. The core innovation is elevating **value-dependent types** to first-class citizens at the Type2 layer.

### What Are Value-Dependent Types?

A **value-dependent type** is a type that depends on one or more **values** (rather than depending solely on other types). These values can be evaluated at compile time, providing type safety guarantees at the compilation stage.

```yaoxiang
# Traditional generics: type parameters
List: (T: Type) -> Type

# Value-dependent types: value parameters
Vec: (n: Int) -> Type  # Vector type depends on length value n
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # Matrix type depends on rows and columns
```

### Core Advantages of Value-Dependent Types

Compared to traditional generics, YaoXiang's value-dependent types have the following core advantages:

| Feature | Traditional Generics (C++/Rust) | YaoXiang Value-Dependent Types |
|---------|--------------------------------|-------------------------------|
| Type-dependent values | Only type parameters | Can depend on any value, including function call results |
| Compile-time evaluation | C++ templates manual specialization, Rust none | Automatic compile-time evaluation with guaranteed termination |
| Type-level computation | Template metaprogramming (complex/dangerous) | Unified type-level computation engine |
| Type safety | C++ none, Rust limited | Complete type safety, compile-time checking |
| Dimension verification | Runtime checking or manual specialization | Compile-time dimension verification, no runtime overhead |

### Type Universe Layers and Value-Dependent Types

Type Universe Thought divides language concepts into different layers based on semantic roles. Value-dependent types are located at the **Type2 layer**:

| Layer | Role | Example |
|-------|------|---------|
| Type-1 | Values | `42`, `factorial(5)`, function itself |
| Type0 | Metatype keyword | `Type` |
| Type1 | Concrete types | `Int`, `String`, `Vec(3)` |
| **Type2** | **Functions/Type constructors/Value-dependent types** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**Key Design**: Functions, type constructors, and value-dependent types at the Type2 layer share **unified syntax**, all in the form `(params) -> result`:
- Ordinary functions: `(Int, Int) -> Int` → return value is a value
- Type constructors: `(T: Type) -> Type` → return value is a type
- Value-dependent types: `(n: Int) -> Type` → return value is a type, but depends on value parameters

> **Curry-Howard Isomorphism**: This unity is not a coincidence. The Curry-Howard isomorphism states that "types are propositions, programs are proofs" — the function type `A → B` corresponds to logical implication "if A then B", generics `(T: Type) -> Type` correspond to universal quantification "for all types T", and value-dependent types `(n: Int) -> Type` correspond to "for each integer n there exists a type". YaoXiang unifies functions, type constructors, and value-dependent types at the Type2 layer, essentially unifying "proofs" and "computation" into the same concept — **constructive proofs**. This is the direct manifestation of the Curry-Howard isomorphism in language design: one form (`(params) -> result`) simultaneously carries logical propositions and computational processes.

### Compile-Time Determinism Guarantee

YaoXiang's Type Universe Thought requires: **Everything at the Type level must be compile-time deterministic**.

```yaoxiang
# Compile-time dimension verification example
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # Compile-time check: dimensions must be positive
    _assert: Assert[Rows > 0],
    _assert: Assert[Cols > 0],
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
2. Verifies if the function has a `decreases` specification (see termination checking mechanism below)
3. Evaluates at compile time
4. Embeds the result in the generated type

### Application Scenarios of Value-Dependent Types

#### Compile-Time Dimension Verification
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
    result
}

# Completely transparent when used, types automatically inferred
numbers = List(1, 2, 3)
doubled = map(numbers, (x) => x * 2)  # Inferred as map[Int, Int]
```

### Comparison with Other Languages

| Feature | C++ Templates | Rust Generics | Haskell GADT | **YaoXiang** |
|---------|---------------|--------------|--------------|--------------|
| Type parameters | ✅ | ✅ | ✅ | ✅ |
| Value-dependent types | ❌ | ❌ | ✅ | ✅ |
| Compile-time evaluation | Template instantiation | ❌ | ✅ | ✅ |
| Termination guarantee | ❌ | ❌ | ❌ (dangerous) | ✅ (decreases specification) |
| Type safety | ❌ (macro expansion) | ✅ | ✅ | ✅ |
| Unified syntax | ❌ | ❌ | ❌ | ✅ |
| Compile-time dimension verification | Manual specialization | Runtime checking | Type families | Compile-time automatic verification |
| decreases specification | ❌ | ❌ | ❌ | ✅ |

### Termination Checking Mechanism (Integrated with RFC-022)

Compile-time evaluation of value-dependent types must **guarantee termination**, otherwise the type system will fall into infinite loops. YaoXiang ensures this through **decreases specifications**, seamlessly integrated with RFC-022.

#### Termination Specification for Recursive Functions
```yaoxiang
# Compile-time factorial: must prove termination
factorial: (n: Int) -> Int = {
    //! requires: n >= 0
    //! ensures: result == n!
    //! decreases: n    # Each recursion n strictly decreases
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Usage: call at type position
vec: Vec(factorial(5)) = Vec(120)()  # Compile-time evaluation factorial(5) = 120
```

#### Termination Specification for Loops
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

#### Termination Checking Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  Type Checking Phase                                        │
│  Encounter function call at type position (e.g., Vec(factorial(5))) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Check decreases specification                            │
│     - Has decreases: verify decreasing condition holds on all recursive paths │
│     - No decreases but obviously terminating: evaluate directly │
│     - No decreases and possibly non-terminating: compile error │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Compile-time evaluation (executed by builtin interpreter) │
│     - Pure functions: evaluate directly                       │
│     - Side effects: compile error (type positions must be side-effect free) │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Embed result in type                                     │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → concrete type                    │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages

- **Safety**: Ensures compile-time evaluation necessarily terminates, avoiding infinite loops in the type system
- **Unity**: Termination checking and partial correctness verification share the same specification mechanism
- **Progressive enhancement**: Can gradually transition from runtime checking to fully static proofs

## Motivation

### Why Do We Need a Strong Generic System?

Current mainstream languages have limitations with generics:

| Language | Generic Capability | Problem |
|----------|-------------------|---------|
| Java | Bounded types | Compile-time monomorphization, no generic specialization |
| C# | Generic constraints | Runtime type checking, performance overhead |
| Rust | Generics + Trait | Complex trait system, steep learning curve |
| C++ | Templates | Template specialization complex, poor compile error messages |
| **YaoXiang** | **Value-dependent types** | **Types can depend on values, compile-time dimension verification, termination guarantee** |

### Core Contradictions

1. **Performance vs Flexibility**: Runtime flexibility vs compile-time optimization
2. **Complex vs Simple**: Powerful type system vs usability
3. **Macros vs Generics**: Macro code generation vs generic type safety
4. **Value-dependent vs Type-safe**: Traditional generics cannot verify dimensions at compile time

### Core Advantages of Value-Dependent Types

YaoXiang's **value-dependent types** are the core advantage over traditional generics:

| Advantage | Description |
|-----------|-------------|
| **Types depend on values** | `Vec: (n: Int) -> Type` makes types depend on specific values |
| **Compile-time evaluation** | Function calls at type positions are evaluated at compile time, results directly embedded in types |
| **Dimension verification** | `Matrix(Float, 3, 3)` verifies matrix dimensions at compile time |
| **Type-level computation** | Conditional types like `If`, `Match` support type-level computation |
| **Termination guarantee** | decreases specification ensures compile-time evaluation necessarily terminates |

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

1. **Zero-cost abstraction** - Generic calls equivalent to concrete type calls
2. **Dead code elimination** - Compile-time analysis, only instantiate used generics
3. **Macro replacement** - Generics replace 90% of macro use cases
4. **Type safety** - Compile-time checking, no runtime type overhead
5. **IDE-friendly** - Smart hints, clear error messages
6. **Value-dependent types** - Types can depend on values, support compile-time dimension verification
7. **Compile-time evaluation safety** - Guaranteed termination of compile-time evaluation through decreases specifications

### Design Principles

- **Compile-time determinism**: Generic parameters determined at compile time
- **Monomorphization priority**: Generate concrete code, avoid virtual function calls
- **Constraint-driven**: Type constraints guide instantiation
- **Platform optimization**: Specialization supports platform-specific optimization
- **Type universe unity**: Functions/type constructors/value-dependent types unified at Type2 layer
- **Termination guarantee**: Function calls at type positions must prove termination

## Proposal

### 1. Basic Generics

#### 1.1 Generic Type Parameters

> **Key rule**: Generic type definitions **must explicitly annotate `: Type`**, otherwise HM inference will infer them as functions.
>
> | Syntax | Meaning |
> |--------|---------|
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
    push: (self: List(T), item: T) -> Void,
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
# Generic functions use unified (T: Type, R: Type) signature syntax
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# Multiple type parameters
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type Self-Description Mechanism

`Type` is a language-level special entity. The compiler naturally recognizes `Type` positions in signatures and automatically infers and fills them from actual parameter types.

```yaoxiang
# Compiler automatically infers generic parameters
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         type declaration   constructor call: Int fills T

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

# Usage points
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
# Type parameters can be omitted when inferrable
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

# Sorting generic containers
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

# Usage example
result: Int = call_with_arg(42, (x: Int) => x * 2)  # result = 84
composed: String = compose(
    "hello",
    (s: String) => s.to_uppercase(),
    (s: String) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

#### 2.4 Builtin Marker Traits: Dup and Clone

**Three kinds of copy semantics**:

| Type | Meaning | Trigger | Applicable Scenarios |
|------|---------|---------|---------------------|
| **Primitive value copy** | Automatic value copy on assignment, two values completely independent | Automatic on assignment/passing | Int, Float, Bool, Char |
| **Dup** | Shallow copy: copy handle/token, underlying data shared | Automatic on assignment/passing | `&T` tokens, `ref T`, String/Bytes |
| **Clone** | Deep copy: create complete independent copy | `value.clone()` | Any type implementing Clone |

**Dup semantics**: Types implementing Dup do not transfer ownership on assignment/passing—the compiler copies the handle/token, and multiple holders point to the same underlying data. This is complementary to the default Move semantics in RFC-009's ownership model.

**Dup and Clone are orthogonal concepts**:

```
Dup = Copy handle, share data (modifications affect each other)
Clone = Copy data, copies independent (modifications don't affect each other)
```

**Rules**:

```
1. Primitive value types (Int, Float, Bool, Char) — compiler builtin value copy, not part of Dup
2. Dup — only applicable to reference/token types and internally reference-counted types
3. Clone — explicit deep copy, any type can implement
4. Default Move — other types maintain default Move semantics
```

**Which types are Dup**:

| Type | Dup | Reason |
|------|-----|--------|
| `&T` (borrow token) | ✅ | Zero-size token, copying token = multiple views point to same data |
| `ref T` | ✅ | Rc/Arc copy = ref count +1, shared heap data |
| String, Bytes | ✅ | Internal reference count, copy handle shares underlying buffer |
| `&mut T` (mutable token) | ❌ | Linear exclusive, cannot copy |
| struct | derived | All fields Dup → struct Dup |
| enum | derived | All fields of all variants Dup → enum Dup |
| tuple | derived | All elements Dup → tuple Dup |
| Fn (closure) | ❌ | Captured environment may not be Dup |
| `*T` (raw pointer) | ❌ | unsafe, not part of ownership system |

**Int/Float/Bool/Char are NOT Dup** — they are value types, the compiler automatically performs value copy on assignment (two values completely independent). This is not "shallow copy", this is the compiler's builtin handling of primitives, and should not be expressed through the Dup type property.

```yaoxiang
# Primitive value types: compiler automatically value copies (not Dup)
x: Int = 42
y = x          # Value copy, x and y completely independent
print(x)       # ✅

# Dup: shallow copy, copy handle shares data
view: &Point = &point
view2 = view    # ✅ Dup: copy token, both point to same point
print(view.x)   # ✅

# Clone: explicit deep copy, create independent copy
backup = big_struct.clone()  # Explicit call

# Generic constraints
dup_use: (T: Dup) -> T = x         # T: Dup → can shallow copy
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → can deep copy
```

> **Note**: `Send`/`Sync` are not user-visible traits. Cross-task security guarantees are handled entirely automatically by the `ref` keyword and compiler — `ref` automatically selects Rc or Arc, users don't need to understand Send/Sync.

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

#### 4.1 Compile-Time Constant Parameters

**Core design**: `Type` in generic signatures marks compile-time type parameters. Value parameters like `Int` are compile-time determinable by default in generic contexts. No `const` keyword needed.

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time constant parameters: Int in generics is compile-time determinable by default
# ════════════════════════════════════════════════════════

# Compile-time factorial: N must be a compile-time known literal
factorial: (N: Int) -> (n: N) -> Int = {
    match n {
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
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120), compiler computes at compile time
```

#### 4.2 Compile-Time Computation

```yaoxiang
# ════════════════════════════════════════════════════════
# Compile-time computation examples
# ════════════════════════════════════════════════════════

# Compiler computes literal-type function calls at compile time
SIZE: Int = factorial(5)  # Compile to 120

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

#### 4.3 Compile-Time Verification (Standard Library Implementation)

```yaoxiang
# ════════════════════════════════════════════════════════
# Standard library implementation: using conditional types
# ════════════════════════════════════════════════════════

# Standard library definition: Assert[C] is a type
# - When C is True, derives to Void
# - When C is False, derives to compile_error("Assertion failed")
Assert: (C: Type) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# Usage method 1: as constraint in type definition
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # Compile-time check: N must be greater than 0 (Assert at type position)
    length: Assert(N > 0),
}

# Usage method 2: use in expression
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# Verification: size of IntArray(10) equals sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 Compile-Time Generic Specialization

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
    # Compiler optimization: unroll loop
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. Conditional Types

> **Curry-Howard Isomorphism**: Conditional types viewed from the Curry-Howard perspective are **case analysis** in logic. The `Bool` type corresponds to a proposition with two possible values (True/False), and `If` selects different results based on the truth value of that proposition — this is exactly case disjunction in logic. `match C { True => T, False => E }` actually expresses: "Given proposition C is True, the conclusion is T; when C is False, the conclusion is E".

#### 5.1 If Conditional Type

```yaoxiang
# Type-level If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# Compile-time branching
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# Compile-time verification
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# Usage
# Type computation: If(True, Int, String) => Int
# Type computation: If(False, Int, String) => String
```

#### 5.2 Type Families

> **Curry-Howard Isomorphism**: Type families are the most direct embodiment of "propositions as types". `Add: (A: Type, B: Type) -> Type` is not "writing an addition function at the type level", but rather **constructing a proposition about natural number addition**. `(Zero, B) => B` says "proposition Add(Zero, B) is equivalent to B", and `(Succ(A'), B) => Succ(Add(A', B))` says "if Add(A', B) holds, then Add(Succ(A'), B) also holds". This is the definition of addition itself from Peano axioms. The type checker verifying this match expression passes is equivalent to verifying the logical consistency of this definition.

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

# Type-level addition (Curry-Howard: This is also the inductive definition of natural number addition)
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
sum: (arr: Array(Int)) -> Int = {
    # Compile to more efficient code
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
# Specialization way fully compliant with RFC-010 syntax: function overloading

# Concrete type specialization
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# Generic implementation (compiler automatically selects optimal)
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

# Compiler automatically selects optimal specialization
sum(int_arr)     # Select sum: (Array(Int)) -> Int
sum(float_arr)    # Select sum: (Array(Float)) -> Float
```

#### 6.3 Perfect Combination of Function Overloading and Inlining

**Key feature**: Function overloading naturally combines with inlining optimization, achieving zero-cost abstraction.

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
# Compiler automatically selects optimal specialization, then inlines
result = native_sum_int(int_arr.data, int_arr.length)

# Completely equivalent to hand-written optimized code, no function call overhead!
```

**Core advantages**:

1. **Compiler intelligent selection**
   ```yaoxiang
   sum(int_arr)      # Automatically select sum: (Array(Int)) -> Int
   sum(float_arr)    # Automatically select sum: (Array(Float)) -> Float
   sum(custom_arr)  # Automatically select sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **Inlining optimization**
   - Small functions automatically inlined at call site
   - Zero function call overhead
   - Completely equivalent to hand-written optimized code

3. **Type safety**
   - Compile-time type checking
   - Zero runtime overhead
   - No virtual function table needed

4. **Perfectly aligned with RFC-010**
   ```yaoxiang
   # Uses unified syntax completely
   name: type = value
   # No new keywords like impl, where needed
   ```

**Practical application example**:

```yaoxiang
# Performance-sensitive numerical computation
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Use Binet formula
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# Compiler automatically selects and inlines
fibonacci(10)      # Select Int version, completely inlined
fibonacci(10.5)    # Select Float version, use Binet formula
```

**What does this mean?**

- ✅ **Generic specialization** → naturally solved by function overloading
- ✅ **Performance optimization** → inlining automatically completed
- ✅ **Code reuse** → one function name, multiple implementations
- ✅ **Zero-cost abstraction** → compile-time polymorphism, zero runtime overhead
- ✅ **No new keywords needed** → perfectly compliant with RFC-010 unified syntax
```

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
```

#### 7.2 Usage Point Analysis

```yaoxiang
# Source code analysis
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# Usage point 1: instantiate map(Int, Int)
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # Needs map[Int, Int]

# Usage point 2: instantiate map(String, String)
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # Needs map[String, String]

# Not used: map[Float, Float], etc.
# These generic instances won't be generated

# After compilation, only includes used instances
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

# After compilation, only generates used sizes
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# Unused sizes won't be generated
# Array(Int, 50) won't be generated
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

# ✅ Generic approach: automatic derivation
# Using function overloading for automatic derivation
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# Usage
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # Automatically generated call
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
result_type = Add[Int, Float]  # Inferred as Float
```

### 9. Examples

#### 9.1 Complete Generic Container Example

```yaoxiang
# ======== 1. Define generic container ========
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
    data: Array(T),
    length: Int,

    # Generic method (T automatically brought into scope by outer List(T))
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. Implement generic methods ========
# Use Type.method syntax sugar: automatically associate with List type

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
# Using function overloading for implementation
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

#### 9.3 Compile-Time Generic Example

```yaoxiang
# ======== 1. Compile-time matrix type ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # Compile-time dimension verification: using Assert standard library type
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
   - Generic errors can be complex
   - Clear error hints needed

### Mitigation Measures

1. **Caching strategy**
   - Cache instantiation results
   - LRU cache to limit memory

2. **Incremental compilation**
   - Cache compilation results
   - Incremental instantiation

3. **Error hints**
   - Clear error messages
   - Generic parameter inference hints

4. **Parallel compilation**
   - Parallel instantiation of generics
   - Multi-threaded constraint solving

## Alternatives

| Alternative | Why Not Chosen |
|-------------|----------------|
| Basic generics only | Cannot replace complex macros |
| Pure macro system | No type safety, poor error messages |
| Constraint-only dependency | Insufficient flexibility |
| Runtime generics | Has performance overhead |

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Constraint solving complexity | Compilation time too long | Incremental solving + caching |
| Code bloat | Binary file too large | DCE + threshold control |
| Implementation complexity | Extended development cycle | Phased implementation |
| Error diagnostics | Poor user experience | Detailed error messages |

## Open Issues

### Pending Decisions

| Topic | Description | Status |
|-------|-------------|--------|
| Instantiation strategy | Eager vs Lazy vs Threshold | Pending discussion |
| Cache size | LRU cache capacity setting | Pending discussion |
| Error diagnostics | Generic error message detail level | Pending discussion |

### Future Optimizations

| Optimization | Value | Implementation Difficulty |
|--------------|-------|---------------------------|
| Instantiation graph analysis | High | Medium |
| Type-level programming DSL | Medium | High |
| Generic performance benchmarks | Medium | Low |

## Appendix

### Syntax BNF

```bnf
# Generic parameters use unified () syntax, as part of function type
# e.g., map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

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

# Type in generic parameters is automatically filled by compiler from actual arguments
# e.g., map(numbers, f), T extracted from numbers: List(Int), R extracted from f: (Int) -> String
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
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (official   │    │ (kept in    │
│  design)    │    │  place)     │
└─────────────┘    └─────────────┘
```

---

## References

### YaoXiang Official Documentation

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
```