---
title: 'RFC-009: Ownership Model Design'
---

# RFC-009: Ownership Model Design

> **Status**: Accepted
> **Author**: ChenXu
> **Created Date**: 2025-01-08
> **Last Updated**: 2025-02-05 (Simplified v1)

## Summary

This document defines the **Ownership Model** for YaoXiang programming language, including ownership semantics, move semantics, sharing mechanisms, and cycle reference handling.

**Core Design**:
- Default **Move (value passing)**, zero-copy
- Explicit **`ref` keyword** = Arc (thread-safe reference counting)
- **`clone()`** = Explicit copy
- **`*T` raw pointer** + `unsafe` = Systems-level programming
- Standard library provides **`Rc`** / **`Arc`** / **`Weak`**

**Enhanced Features**:
- **Empty State Reuse**: Variables can be reassigned after Move
- **Ownership Backflow**: Functions automatically return modified ownership
- **Consume Analysis**: Auto-infer whether parameters are consumed (Consumes/Returns)
- **Inverse Function Generation**: ⚠️ **Future Feature**
- **Partial Consume**: ⚠️ **Future Feature**

**Cycle Reference Handling**:
- Intra-task cycles: Allowed (leak controllable, released after task ends)
- Cross-task cycles: Compiler detects and reports error
- `unsafe` escape hatch: Bypass detection (user responsible)

**Complexity Eliminated**:
- ❌ No lifetime `'a`
- ❌ No borrow checker
- ❌ No GC

> **Programming Burden**: ⭐☆☆☆☆ (Almost zero)
> **Performance Guarantee**: Zero runtime overhead, no GC pauses

## Motivation

### Why is an ownership model needed?

| Language | Memory Management | Problem |
|----------|-----------------|---------|
| C/C++ | Manual | Memory leaks, dangling pointers, double free |
| Java/Python | GC | Latency jitter, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow | High complexity, steep learning curve |
| **YaoXiang** | **Ownership + ref** | **Simple safety, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
create_and_forget: () -> Point = () => {
    p = Point(1.0, 2.0)
    # p automatically freed when leaving scope
}

# 2. Explicit ref = Arc (safe sharing, type auto-inferred)
shared = ref p   # Arc, type inferred from p
spawn(() => print(shared.x))

# 3. Explicit clone() = copy
p2 = p.clone()

# 4. Systems-level = unsafe + raw pointer
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}

# 5. Standard library Rc/Arc/Weak
use std.rc.{Rc, Weak}
use std.sync.Arc

rc: Rc[Node] = Rc.new(node)
```

## Core Semantics

### 1. Move Semantics (Default)

```yaoxiang
# Assignment = Move
p = Point(1.0, 2.0)
p2 = p              # p becomes invalid

# Function parameter = Move
process: (p: Point) -> Void = {
    # p ownership transferred in
}

# Return value = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Ownership transferred
}
```

### 2. ref Keyword (Arc)

```yaoxiang
# Create Arc reference
p = Point(1.0, 2.0)
shared = ref p      # Arc reference

# Thread-safe sharing
spawn(() => {
    print(shared.x)   # ✅ Safe
})

# Arc automatically manages lifecycle
# When shared goes out of scope, count decrements to zero
```

### 3. clone() Explicit Copy

```yaoxiang
# Explicit copy when needed
p = Point(1.0, 2.0)
p2 = p.clone()      # p and p2 are independent

p.x = 0.0           # ✅ Valid
p2.x = 0.0          # ✅ Valid
```

### 4. unsafe and Raw Pointers

```yaoxiang
# Raw pointer type
PtrType ::= '*' TypeExpr

# unsafe block
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'

# Example
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### 5. Standard Library Types

```yaoxiang
# Rc - Non-thread-safe reference counting
type Rc[T] = {
    new: (T) -> Rc[T],
    clone: (Self) -> Rc[T],
    drop: (Self) -> Void,
}

# Arc - Thread-safe reference counting
type Arc[T] = {
    new: (T) -> Arc[T],
    clone: (Self) -> Arc[T],
    drop: (Self) -> Void,
}

# Weak - Non-owning reference
type Weak[T] = {
    upgrade: () -> Option[Arc[T]],
}
```

## Advanced Features

### 1. Empty State Reuse

```yaoxiang
# After Move, variable can be reassigned
p = Point(1.0, 2.0)
p2 = p              # p is now "empty"

p = Point(3.0, 4.0)  # p can be reused
```

### 2. Ownership Backflow

```yaoxiang
# Function automatically returns modified ownership
modify_point: (Point) -> Point = (p) => {
    p.x = p.x + 1.0
    return p        # Ownership backflows
}

p = Point(1.0, 2.0)
p2 = modify_point(p)  # p invalid, p2 has modified value
```

### 3. Consume Analysis

```yaoxiang
# Compiler auto-infers parameter consumption
consume: [T](value: T) -> T = (value) => {
    # value is consumed
    return value
}
```

## Cycle Reference Handling

### Intra-task Cycles

```yaoxiang
# Allowed within single task
# Memory leak is controllable
# Released when task ends
```

### Cross-task Cycles

```yaoxiang
# Compiler detects and reports error
# Example: Task A holds Arc[B], Task B holds Arc[A]
# Would cause deadlock - compiler prevents this
```

## Send / Sync Constraints

### Send Constraint

```yaoxiang
# Types that can be safely transferred across threads
trait Send = {
    # Marker trait
}

# Auto-derived rules
struct[T1, T2]: Send ⇐ T1: Send 且 T2: Send
```

### Sync Constraint

```yaoxiang
# Types that can be safely shared across threads
trait Sync = {
    # Marker trait
}

# Auto-derived rules
struct[T1, T2]: Sync ⇐ T1: Sync 且 T2: Sync
```

### Constraint Hierarchy

```
Send ──► Can safely transfer across threads
  │
  └──► Sync ──► Can safely share across threads
       │
       └──► Types satisfying Send + Sync can automatically be concurrent
```

## Implementation

### Phase 1: Core Ownership

| Feature | Status |
|---------|--------|
| Move semantics | ✅ |
| ref Arc | ✅ |
| clone() | 🔄 |
| unsafe | ⏳ |

### Phase 2: Advanced Features

| Feature | Status |
|---------|--------|
| Empty state reuse | ⏳ |
| Ownership backflow | ⏳ |
| Consume analysis | ⏳ |

### Phase 3: Standard Library

| Type | Status |
|------|--------|
| Rc | ⏳ |
| Arc | ⏳ |
| Weak | ⏳ |

---

## Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Default Move | Zero-copy ownership transfer | 2025-01-08 | ChenXu |
| ref = Arc | Thread-safe sharing | 2025-01-08 | ChenXu |
| No lifetimes | Simplified model | 2025-01-08 | ChenXu |
| No borrow checker | Ref instead | 2025-01-08 | ChenXu |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| Ownership | Each value has unique owner |
| Move | Transfer ownership without copying |
| Arc | Thread-safe reference counting |
| Rc | Non-thread-safe reference counting |
| Send | Can transfer across threads |
| Sync | Can share across threads |
