---
title: 'RFC-011: Generic Type System Design'
---

# RFC-011: Generic Type System Design - Zero-Cost Abstraction and Macro Replacement

> **Status**: Accepted
> **Author**: ChenXu
> **Created Date**: 2025-01-25
> **Last Updated**: 2025-02-04 (Removed const keyword, using literal type constraints instead)

## Summary

This document defines the **generic type system design** for YaoXiang language, achieving zero-cost abstraction through powerful generics, reducing macro dependency with compile-time optimization, and providing dead code elimination.

**Core Design**:
- **Basic Generics**: `[T]` type parameters, supporting generic functions and types
- **Type Constraints**: `[T: Clone]` multiple constraints, function type constraints
- **Associated Types**: `type Iterator[T] = { Item: T, next: () -> Option[T] }`
- **Compile-Time Generics**: `[T, N: Int]` compile-time constant parameters, literal type constraints distinguishing compile-time from runtime
- **Conditional Types**: `type If[C: Bool, T, E]` type-level computation, type families
- **Platform Specialization**: `[P: X86_64]` predefined generic parameter P, platform as type

**Value**:
- Zero-cost abstraction: compile-time monomorphization, no runtime overhead
- Dead code elimination: instantiation graph analysis + LLVM optimization
- Macro replacement: generics replace 90% of macro usage scenarios
- Type safety: compile-time checks, IDE-friendly

## Reference Documents

This document's design is based on:

| Document | Relationship | Description |
|----------|--------------|-------------|
| [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) | **Syntax Foundation** | Generic syntax integrates with unified `name: type = value` model |
| [RFC-009: Ownership Model](./accepted/009-ownership-model.md) | **Type System** | Natural combination of Move semantics and generics |
| [RFC-001: Concurrency Model](./accepted/001-concurrent-model-error-handling.md) | **Execution Model** | DAG analysis and generic type checking |
| [RFC-008: Runtime Model](./accepted/008-runtime-concurrency-model.md) | **Compiler Architecture** | Generic monomorphization and compile-time optimization strategy |

## Motivation

### Why is a strong generics system needed?

Current mainstream language generics have limitations:

| Language | Generic Capability | Problem |
|----------|-------------------|---------|
| Java | Bounded types | Compile-time monomorphization, no generic specialization |
| C# | Generic constraints | Runtime type checking, performance overhead |
| Rust | Generics + Trait | Trait system complex, steep learning curve |
| C++ | Templates | Template specialization complex, poor compile error messages |

### Core Contradictions

1. **Performance vs Flexibility**: Runtime flexibility vs compile-time optimization
2. **Complexity vs Simplicity**: Powerful type system vs usability
3. **Macros vs Generics**: Macro code generation vs generic type safety

### Value of Generic System

```yaoxiang
# Example: Unified API Design
# map operation for different container types

# Traditional approach: Implement separately for each type
map_int_array: (array: Array[Int], f: Fn(Int) -> Int) -> Array[Int] = ...
map_string_array: (array: Array[String], f: Fn(String) -> String) -> Array[String] = ...
map_int_list: (list: List[Int], f: Fn(Int) -> Int) -> List[Int] = ...
map_string_list: (list: List[String], f: Fn(String) -> String) -> List[String] = ...

# Generic approach: One generic function covers all types
map: [T, R](container: Container[T], f: Fn(T) -> R) -> Container[R] = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## Core Design

### 1. Basic Generics

```yaoxiang
# Generic function
identity: [T](value: T) -> T = {
    return value
}

# Generic type
type Option[T] = {
    some: (T) -> Self,
    none: () -> Self
}

# Usage
int_opt: Option[Int] = some(42)
str_opt: Option[String] = some("hello")
```

### 2. Type Constraints

```yaoxiang
# Single constraint
clone: [T: Clone](value: T) -> T = {
    return value.clone()
}

# Multiple constraints
combine: [T: Clone + Add](a: T, b: T) -> T = {
    a.clone() + b
}

# Function type constraint
call_twice: [T, F: Fn() -> T](f: F) -> (T, T) = {
    (f(), f())
}

# Where clause syntax
sort: [T: Clone + PartialOrd](list: List[T]) -> List[T] = {
    # ...
}
```

### 3. Associated Types

```yaoxiang
# Iterator trait with associated type
type Iterator[T] = {
    Item: T,                    # Associated type
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool
}

# Using associated types
collect: [T, I: Iterator[T]](iter: I) -> List[T] = {
    result = List[T]()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}
```

### 4. Compile-Time Generics

```yaoxiang
# Literal type constraint
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# Compile-time constant array
type StaticArray[T, N: Int] = {
    data: T[N],
    length: N
}

# Usage
arr: StaticArray[Int, factorial(5)]  # Compiled to StaticArray[Int, 120]
```

### 5. Conditional Types

```yaoxiang
# Type-level If
type If[C: Bool, T, E] = match C {
    True => T,
    False => E
}

# Compile-time branch
type NonEmpty[T] = If[T != Void, T, Never]

# Type family
type AsString[T] = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 6. Platform Specialization

```yaoxiang
# Platform as type
type Platform = X86_64 | AArch64 | RISC_V | ARM | X86

# Platform-specific implementation
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    neon_sum(arr.data, arr.length)
}
```

## Implementation

### Phase 1: Core Generics

| Feature | Status |
|---------|--------|
| Basic generics `[T]` | 🔄 |
| Type constraints | ⏳ |
| Monomorphization | ⏳ |

### Phase 2: Advanced Features

| Feature | Status |
|---------|--------|
| Associated types | ⏳ |
| Compile-time generics | ⏳ |
| Conditional types | ⏳ |

### Phase 3: Optimization

| Feature | Status |
|---------|--------|
| Dead code elimination | ⏳ |
| LLVM integration | ⏳ |
| Cross-module inlining | ⏳ |

---

## Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Generic syntax | `[T]` for type parameters | 2025-01-25 | ChenXu |
| Constraint syntax | `[T: Clone]` | 2025-01-25 | ChenXu |
| Associated types | `Item: T` syntax | 2025-01-25 | ChenXu |
| Compile-time | `[n: Int]` literal constraint | 2025-02-04 | ChenXu |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| Generic | Type or function parameterized by type |
| Constraint | Restriction on type parameter |
| Monomorphization | Compile-time generation of concrete types |
| Associated Type | Type associated with another type |
| Type Family | Type computed from other types |
