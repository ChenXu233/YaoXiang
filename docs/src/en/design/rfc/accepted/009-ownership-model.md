```yaml
---
title: "RFC-009: Ownership Model Design"
status: "Accepted"
author: "晨煦"
created: "2025-01-08"
updated: "2026-05-29 (Borrow token system replaces the simplified borrowing, unifying the type system)"
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **Ownership Model** for the YaoXiang programming language.

**Core Design — Five Concepts, One Gradient**:

```
Glance/Modify In-Place    Take Ownership    Shared Holding       Clone a Copy         Systems-Level
    │                        │                  │                  │                    │
   &T                     Move             ref                clone()              unsafe
  &mut T                  Zero-Copy        Compiler picks      Explicit deep copy   *T
  Zero-size token         Default          Rc/Arc automatically                    User responsible
  Type properties                                                     
  naturally derive                                               
  permissions                                                     
```

- **Move (default)**: Assignment/parameter passing/return = ownership transfer, zero-copy, RAII auto-release
- **`&T` / `&mut T` (borrow tokens)**: Zero-sized compile-time token types. `&T` is duplicable (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are naturally derived from type properties, no special rules needed. Can be returned, stored in structs, and captured by closures.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically picks Rc (no cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, systems-level escape hatch

**Eliminated Complexity**:
- ❌ No lifetime `'a`
- ❌ No borrow checker (token type properties — Dup/Linear — replace a dedicated borrow checker)
- ❌ No GC
- ❌ No "no-escape" special rules (tokens are ordinary types, scope handled uniformly by the type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler picks automatically)

> **Programming burden**: `&T` is duplicable, `&mut T` is not — two type properties, zero special rules, fully automatic by compiler.
> **Performance guarantee**: Move has zero overhead, tokens have zero overhead (zero-sized types, disappear after compilation), ref pays as needed, no GC pauses.

## Motivation

### Why Do We Need an Ownership Model?

| Language | Memory Management | Problems |
|----------|-------------------|----------|
| C/C++ | Manual management | Memory leaks, dangling pointers, double frees |
| Java/Python | GC | Latency jitter, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow Checker | Lifetime `'a` steep learning curve |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p cannot be read again

# 2. &T / &mut T borrow tokens (zero overhead, type properties naturally derive permissions)
print_info(p2)                 # Compiler automatically creates &Point token, released after use
shift(p2, 1.0, 1.0)            # Compiler automatically creates &mut Point token

# 3. ref = sharing (compiler automatically picks Rc/Arc)
shared = ref p2                # Cross-scope holding
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()             # Deep copy, unique

# 5. unsafe + *T = systems-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature | Rust | YaoXiang |
|---------|------|----------|
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)** |
| Borrowing | `&T`/`&mut T`, can return, requires lifetimes | **`&T`/`&mut T` zero-size tokens, Dup/Linear type properties naturally derive** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler automatically picks Rc/Arc)** |
| Copying | `clone()` | `clone()` |
| Raw pointers | `*T` | `*T` |
| Lifetimes | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker flow-sensitive liveness analysis (token state tracking)** |
| Cyclic references | Manual Weak | **Unified release at task end / cross-task lint / std Weak** |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: Assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p cannot be read again

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p rebinds, type must remain consistent

# Function parameters: Move
process: (p: Point) -> Point = {
    p.transform()
    p                            # Move return
}

# Function return: Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    p                            # Move return, zero-copy
}
```

**Characteristics**:
- Zero-copy (compiler moves pointers)
- Original binding unreadable after move (compile error)
- RAII: auto-release at scope end
- Function signature `(T) -> T` is self-documenting — consumes T, returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core Principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not "references" but "type-level proof of access permissions".**

#### 2.1 Two Type Properties

```
&T      →  Zero-size, duplicable (Dup), grants read-only permission
&mut T  →  Zero-size, linear (non-Dup), grants exclusive read-write permission
```

**This is not a "rule" to memorize — it's a fundamental property of the type system.** Dup types can be freely copied (multiple `&T` coexisting), linear types cannot be copied (`&mut T` is inherently unique). There is no "borrow checker" — only the type checker doing what it's always done.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter types, determine required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point token grants write permission
    self.y = self.y + dy
}

# Call site: compiler automatically selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler automatically creates &Point token
p.shift(1.0, 1.0)                  # Compiler automatically creates &mut Point token
p.print()                          # OK, previous token released after shift call ends

# Free functions work the same way
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexisting — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No-Escape" Is Unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — can only be parameters, cannot be returned, cannot be stored in structs. This is patching the "borrowing" concept.

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scoping rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Tokens propagate with return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Sub-token and parent token return together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Stored in structs — naturally supported**:

```yaoxiang
# ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds read-only view of target
}

# view's token derived from target, Window holds ownership of both
# As long as Window exists, view token is valid
```

**Closure capture — naturally supported**:

```yaoxiang
# ✅ Closures capture tokens, just like capturing any value
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    # Closure captures threshold's &Float token (Dup type, freely copied into closure)
    items.filter(|p| p.x > threshold)
}

# This is something RFC-009 v8 cannot do — v8 forbids closure capture of borrows
```

**Cross-task — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: tokens cannot cross task boundaries
}

# This is not a special rule — tokens are compile-time permission proofs,
# use ref for cross-task sharing
# If cross-task sharing is needed, use ref
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not ownable
}
```

#### 2.4 Token Lifetime

Token lifetime is governed by **ordinary scope rules**, no lifetime parameters needed:

- Tokens in function parameters: alive during call, released after call ends
- Returned tokens: ownership transferred to caller
- Tokens stored in structs: alive with the struct
- Tokens captured by closures: alive with the closure

Compiler doesn't need `'a` annotations because tokens are **values**, and value lifetime is uniformly managed by the ownership system (Move/RAII). **Reduces borrowing to an ownership problem.**

#### 2.5 Token Conflict Detection

Replaces RFC-009 v8's "cross-borrow checking". The principle is simpler — **flow-sensitive liveness analysis** on token values:

```yaoxiang
# ❌ &mut token is linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p cannot be read again
    p.x = 10.0                      # ❌ Compile error: WriteToken already moved
}

# ✅ &T token is Dup type, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexisting
}
```

**Detection method**: This is not a dedicated "borrow checker" — it's **flow-sensitive liveness analysis** on token values. The compiler tracks each token's state (live/moved) within function bodies, exactly the same way it tracks any linear type value.

#### 2.7 Compiler Internals: Branding Mechanism

Users never see brands. The compiler internally assigns a compile-time unique identifier to each token:

```
What user sees         Compiler internal representation
────────────────────────────────────────────────────────
&Point              →  ReadToken(Point, #N)    // #N is compile-time unique integer
&mut Point          →  WriteToken(Point, #M)   // #M is compile-time unique integer
```

Brand uses:
- **Anti-forgery**: Tokens can only be obtained from owner capsules, cannot be conjured from thin air
- **Provenance tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries derived brand (`#N.field_x`), allowing compiler to trace back to parent token
- **Conflict detection**: Same-origin `WriteToken` and derived `ReadToken` cannot be alive simultaneously

Brands disappear completely after monomorphization and inlining; they don't exist in generated machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

Call-site compiler auto-selects by priority:

```
1. If actual argument is used later → prefer creating token (&T or &mut T, based on method signature)
2. If actual argument is not used later → Move
3. Preference order: &T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Simplified Borrowing

| Feature | Simplified Borrowing (v8) | Borrow Tokens (v9) |
|---------|---------------------------|---------------------|
| Return references | ❌ Hard-coded prohibition | ✅ Tokens propagate with return value |
| Store in struct | ❌ Hard-coded prohibition | ✅ Tokens as struct fields |
| Closure capture | ❌ Hard-coded prohibition | ✅ Closures capture token values |
| Special rules | 3 (only as params/cannot return/cannot store) | 0 — type properties naturally derive |
| Borrow checking | Dedicated cross-borrow checking | Type checker flow-sensitive liveness analysis |
| Lifetime annotations | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-sized types, disappear after compilation) |
| Error messages | "Borrow cannot escape" | "WriteToken(#3) already moved" (regular type error) |
| User mental model | Understand "borrowing"'s special status | `&T` is duplicable, `&mut T` is not |

---

### 3. ref Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether the underlying implementation is Rc or Arc is irrelevant to users.

#### 3.1 Basic Usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Shared, compiler auto-picks implementation

# Cross-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }           # Compiler: cross-task → Arc
    spawn { use(data) }           # Compiler: cross-task → Arc
}

# Single-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    use(data)                     # Compiler: no cross-task → Rc
}
```

**User mental model**: `ref` = shared holding. That's it.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference count, low overhead)
Escapes to other tasks         → Arc (atomic reference count, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycles → silently allowed.
  ├── Structured concurrency guarantees unified release of all resources when task ends.
  ├── ref always stays alive, semantics are not diluted.
  └── Users have the right to build bidirectional strong references within a task
      (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, no actual leaks will occur
      (parent task end releases all child task resources).
  ├── But cross-task strong references imply ownership boundary ambiguity,
      worth pausing to reconsider.
  ├── Default warn level, compilation passes with a hint.
  └── Teams can set deny in project config,纳入 CI quality gate.

**Lint levels** (like Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | No checking | Personal projects |
| `warn` (default) | Compilation passes, with hint | Development phase |
| `deny` | Compilation fails | Team CI quality gate |
| `forbid` | Compilation fails, cannot be overridden | Organization-level mandatory rules |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong references
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Unified release at task end.
}

# Cross-task cycle: lint (default warn)
@block
parent_task: () -> Void = {
    shared_a = ref a
    shared_b = ref b
    spawn {
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task cyclic reference
    }
}
```

**Project config example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles fail on CI
```

| Cycle Type | Behavior | Reason |
|------------|----------|--------|
| Intra-task ref cycle | No checking | User's prerogative, unified release at task end |
| Cross-task ref cycle | lint (default warn) | Reminder to reconsider, configurable deny |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not language-built-in, it's a standard library type.** For daily use, `ref` is sufficient. Advanced users who need fine-grained memory control manually introduce `Weak`.

#### 3.5 Borrow Tokens vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Glance / modify in place | Shared holding |
| Scope | Follows token value's scope | Cross-scope |
| Cost | Zero overhead (zero-sized type) | Rc or Arc (compiler picks) |
| Escape | Can (token propagates with return value/struct/closure) | Designed for escaping |
| Cross-task | Cannot (token is compile-time permission proof, cannot cross task boundary) | Can (compiler auto-picks Arc) |
| Cycle formation | Not applicable | Intra-task silently allowed, cross-task lint |

---

### 4. clone() — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, don't affect each other
```

**When to use**: When you need to keep the original value and Move/sharing is inappropriate.

### 5. unsafe + Raw Pointers (Systems-Level Programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Restrictions**:
- Can only be used in `unsafe` blocks
- User guarantees no dangling, no use-after-free
- Used for FFI, memory operations, etc. systems-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow Tokens (Zero Overhead)    Move (Zero Overhead)      Sharing (Pay As Needed)    Clone
   │                               │                         │                        │
  &T duplicable token           Default ownership transfer  ref Rc/Arc              clone()
  &mut T linear token           Chained consume-return      Compiler auto-picks     Explicit deep copy
   │                               │                         │                        │
  Token value scope              Scope-relative             Cross-scope              Anytime
  Can return/store in struct     T -> T return              ref cross-task → Arc     Independent copy
  Can be captured by closure    T -> Void consume          ref non-cross-task → Rc
  Zero-size, disappears after                            Intra-task cycles silent
  compilation                                           Cross-task cycles lint
                                                         std Weak escape
```

---

## Comprehensive Example

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,

    # &T: read-only token
    print: (self: &Point) -> Void = {
        print(self.x)
        print(self.y)
    }

    # &mut T: mutable token
    shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
        self.x = self.x + dx
        self.y = self.y + dy
    }

    # Move → Move: consume-return
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # Take, modify, return to you
    }

    # Return reference: token propagates with return value
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# Closure captures tokens (capability v8 cannot have)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# Comprehensive usage
p = Point(1.0, 2.0)
p.print()                           # &Point token
p.shift(1.0, 1.0)                   # &mut Point token
p = p.scale(2.0)                    # Move → return
shared = ref p                      # ref shared
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# Intra-task cycles: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, unified release at task end

# unsafe systems-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Dup Type Property

`Dup` (Duplicable) is a compiler-managed type property meaning **shallow copy**: assignment/parameter passing copies the handle/token, with underlying data shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creates independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies handles to share data, Clone creates independent copies. A type can support both Dup and Clone, or only one.

| Type | Dup | Clone | Description |
|------|-----|-------|-------------|
| `&T` | ✅ (copy token, multiple views point to same data) | ✅ | Read-only token |
| `ref T` | ✅ (ref count +1, shared heap data) | ✅ | Shared holding (compiler auto-picks Rc/Arc) |
| String, Bytes | ✅ (internal ref count, copy handle shares underlying buffer) | ✅ | String/bytes |
| `&mut T` | ❌ (linear, exclusive) | ❌ | Mutable token |
| `*T` | ❌ | ❌ | Raw pointer |
| struct | derived (auto-derived when all fields are Dup) | ✅ | Struct |

**Primitive value types** (Int, Float, Bool, Char) have compiler-built-in value copy semantics on assignment — two values are completely independent, not shallow copy. They are not part of the Dup type property but native compiler handling.

---

## Performance Analysis

| Operation | Cost | Description |
|-----------|------|-------------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-sized type, disappears after compilation, zero runtime overhead |
| `ref` (non-cross-task) | Low | Compiles to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiles to Arc, atomic operation |
| `clone()` | Type-dependent | Fast for small objects, slow for large objects |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|----------|------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + Borrow Checker | Compile-time checking | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + Borrow Tokens** | **RAII** | **Task boundary release / cross-task lint / std Weak** | **Low** |

---

## Trade-offs

### Advantages

1. **Uniform**: `&T`/`&mut T` are ordinary types, not special language features. Completely consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, no global borrow checker. `&T` is duplicable, `&mut T` is not — two type properties
3. **Powerful**: Can return references, store in structs, capture in closures — same expressiveness as Rust
4. **Compiler intelligent**: ref auto-picks Rc/Arc, call site auto-selects borrowing
5. **Deterministic**: ref keeps alive, won't silently become weak reference
6. **High performance**: Move zero-copy, tokens zero overhead (zero-sized types, disappear after compilation)
7. **Flexible**: `unsafe + *T` supports systems-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, function signatures returning references will reflect additional generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the unavoidable cost of sharing)
3. **unsafe risk**: User must guarantee correctness
4. **Cross-task cycles are lint not compile error**: Unlike Rust's compile error, default is warn, team must configure deny to use as quality gate

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| GC | Runtime overhead, unpredictable pauses |
| Rust Borrow Checker | Requires lifetime `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointers | Cannot do systems-level programming |
| Expose Rc/Arc to users | Leaking implementation details to users, increased cognitive load |
| Simplified borrowing (v8) | "No-escape" strategy sacrifices key expressiveness like closure capture, returning references |

---

## Design Decision Log

| Decision | Choice | Reason | Date |
|----------|--------|--------|------|
| **Default value** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimization | User simplicity, compiler responsibility | 2025-01-15 |
| **Borrowing** | `&T`/`&mut T` as zero-sized token types | Type properties (Dup/Linear) naturally derive permissions, unified type system | 2025-01-15 |
| **Borrow tokens** | Replace simplified borrowing, `&T` Dup, `&mut T` Linear | Eliminate "no-escape" special rules, support closure capture / return references / store in structs | 2026-05-29 |
| **Copying** | `clone()` | Explicit semantics | 2025-01-15 |
| **Systems-level** | `*T` + `unsafe` | Supports systems programming | 2025-01-15 |
| **Lifetimes** | Not implemented | Tokens are values, lifetime managed uniformly by Move/RAII, reduces borrowing to ownership problem | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, not visible to users | Reduced cognitive load | 2025-01-15 |
| **Cyclic references** | No checking intra-task, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint can be configured deny | 2025-01-16 |
| **Weak** | Provided by standard library | Advanced users explicitly choose | 2025-01-16 |
| **Consume analysis** | Removed | Mini borrow checker, not needed | 2026-05-11 |
| **Ownership return** | Removed | `(T) -> T` signature is self-documenting | 2026-05-11 |
| **Empty state reuse** | Removed (as feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function / partial consume / field three-level mutability** | Removed | Over-engineering | 2026-05-11 |

### Version History

| Version | Major Changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| v4 | Default Move + explicit ref | 2025-01-15 |
| v5 | Structured concurrency + cyclic reference handling | 2025-01-16 |
| v6 | Added empty state reuse, ownership return | 2025-02-04 |
| v7 | Added consume analysis, inverse functions, field-level mutability | 2025-02-05 |
| **v8** | **Removed over-engineering, added simplified borrowing &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces simplified borrowing, unifying type system** | **2026-05-29** |

### Open Issues

| Issue | Description | Status |
|-------|-------------|--------|
| Drop syntax | Whether explicit `drop()` function is needed | Pending discussion |
| Escape analysis algorithm | ref cross-task detection implementation | Pending discussion |
| Token conflict detection | Flow-sensitive liveness analysis, see below | ✅ Resolved |

### Token Conflict Detection: Flow-Sensitive Liveness Analysis

**Analysis scope**: Function body only. Flow-sensitive liveness analysis on token values, tracking each token's state (live/moved).

**Layer 1: Call site checking** — same actual argument cannot simultaneously create `&mut` token and other tokens:

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p simultaneously derives &mut and & tokens, compiler rejects
```

**Layer 2: Function body flow-sensitivity** — after `&mut` token passed to call, token released on call return, can create new token subsequently:

```yaoxiang
process_twice: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)    # &mut token passed to shift, token released after shift returns
    print_info(p)          # Recreate &Point token, no conflict
}
```

**What is not needed**: Cross-function lifetime tracking, global alias analysis, borrow graph constraint solving, NLL, `'a` annotations. Because tokens are values, value liveness analysis is handled uniformly by the type checker — exactly the same as tracking any linear type value.

---

## References

### YaoXiang Official Documentation

- [Language Specification](../language-spec.md)
- [Design Manifesto](../manifesto.md)
- [RFC-001 Concurrent Model and Error Handling](./001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generic Type System Design](./011-generic-type-system.md)
- [YaoXiang Guide](../guides/YaoXiang-book.md)

### External References

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and Disposition

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author draft, awaiting review submission |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes formal design document |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory |
```