---
title: RFC-009: Ownership Model Design
---

# RFC-009: Ownership Model Design

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-08
> **Last Updated**: 2026-05-29 (Borrow token system replaces the simplified borrowing, unified type system)

## Summary

This document defines the **Ownership Model** for the YaoXiang programming language.

**Core Design — Five Concepts, One Gradient**:

```
glance/transform     Take away           shared hold          clone a copy        system-level
    │                  │                    │                   │                 │
   &T                Move               ref                clone()            unsafe
  &mut T             Zero-copy          Compiler auto      Explicit           *T
  Zero-size          Default            picks Rc/Arc       deep copy          User responsible
  token              RAII auto          for shared
  Type properties    release
  naturally
  infer permissions

```

- **Move (Default)**: Assignment/param passing/return = ownership transfer, zero-copy, RAII auto release
- **`&T` / `&mut T` (Borrow tokens)**: Zero-size compile-time token types. `&T` is copyable (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are naturally inferred from type properties, no special rules needed. Can be returned, stored in structs, and captured by closures.
- **`ref` keyword**: Cross-scope sharing. Compiler auto-selects Rc (not cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointers, system-level escape hatch

**Eliminated Complexity**:
- ❌ No lifetimes `'a`
- ❌ No borrow checker (token type properties — Copy/Linear — replace the dedicated borrow checker)
- ❌ No GC
- ❌ No "no escape" special rules (tokens are ordinary types, scope handled uniformly by type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler auto-selects)

> **Programming burden**: `&T` is copyable, `&mut T` is not copyable — two type properties, zero special rules, fully automated by compiler.
> **Performance guarantee**: Move zero overhead, token zero overhead (zero-size type, disappears after compilation), ref pay-as-you-go, no GC pauses.

## Motivation

### Why Do We Need an Ownership Model?

| Language | Memory Management | Problems |
|----------|-------------------|----------|
| C/C++ | Manual management | Memory leaks, dangling pointers, double-free |
| Java/Python | GC | Latency jitter, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow checker | Lifetime `'a` steep learning curve |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p is no longer readable

# 2. &T / &mut T Borrow tokens (zero overhead, type properties naturally infer permissions)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = sharing (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope hold
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, exclusive

# 5. unsafe + *T = system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature | Rust | YaoXiang |
|---------|------|----------|
| Default semantics | Borrow `&T` (needs explicit `.clone()`) | **Move (value semantics, zero-copy)** |
| Borrowing | `&T`/`&mut T`, returnable, needs lifetimes | **`&T`/`&mut T` zero-size tokens, Copy/Linear type properties naturally infer** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-selects Rc/Arc)** |
| Cloning | `clone()` | `clone()` |
| Raw pointers | `*T` | `*T` |
| Lifetimes | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker flow-sensitive liveness analysis (token state tracking)** |
| Circular references | Manual Weak | **Task termination unified release / cross-task lint / std Weak** |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: Assignment / param passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p is no longer readable

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p re-bound, type must match

# Function params: Move
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
- Zero-copy (compiler moves the pointer)
- Original binding unreadable after move (compile error)
- RAII: auto-release at scope end
- Function signature `(T) -> T` is self-documenting — consumes T, returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-size compile-time token types. They are not "references" but "type-level proof of access permission".**

#### 2.1 Two Type Properties

```
&T      →  zero-size, copyable (Copy), grants read-only permission
&mut T  →  zero-size, linear (non-Copy), grants exclusive read-write permission
```

**This is not a "rule" to memorize — it's a fundamental property of the type system.** Copy types can be freely copied (multiple `&T` coexist), linear types cannot be copied (`&mut T` is inherently unique). There is no "borrow checker" — only the type checker doing what it's always done.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare param types, determine required permissions
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point token grants write permission
    self.y = self.y + dy
}

# Call side: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, previous token released after shift call

# Free functions work the same way
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist — Copy type
}
d = distance(p, p2)
```

#### 2.3 Why "No-Escape" Is Unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — can only be params, cannot be returned, cannot be stored in structs. This is patching the "borrowing" concept.

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scoping rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Token propagates with return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Sub-token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Storing in structs — naturally supported**:

```yaoxiang
# ✅ Struct carries token as a field
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds read-only view of target
}

# view token derived from target, Window holds ownership of both
# As long as Window exists, view token is valid
```

**Closure capture — naturally supported**:

```yaoxiang
# ✅ Closures capture tokens, just like capturing any value
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    # Closure captures threshold's &Float token (Copy type, freely copied into closure)
    items.filter(|p| p.x > threshold)
}

# This is something RFC-009 v8 couldn't do — v8 prohibited closure capture of borrows
```

**Cross-task — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token type doesn't impl Send
}

# This is not a special rule — &T token type doesn't impl Send trait
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

Token lifetime is governed by **ordinary scoping rules**, no lifetime parameters needed:

- Token in function param: alive during call, released after call
- Returned token: ownership transfers to caller
- Token stored in struct: lives with the struct
- Captured token in closure: lives with the closure

Compiler doesn't need `'a` annotations because tokens are **values**; value lifetimes are uniformly managed by the ownership system (Move/RAII). **Reduces borrowing to an ownership problem.**

#### 2.5 Freeze Mechanism

`&mut T` token can be temporarily "frozen" to produce a `&T` token:

```yaoxiang
modify_and_read: (p: &mut Point) -> Void = {
    p.x = 10.0                      # Use &mut Point to modify
    
    # Freeze &mut, get read-only view
    view: &Point = freeze(p)         # p is frozen here
    print(view.x)                   # Read through &Point
    print(view.y)
    # view leaves scope, freeze lifted
    
    p.y = 20.0                      # &mut Point restored
}
```

`freeze` semantics:
- Takes `&mut T`, returns `&T`
- Original `&mut T` unavailable while `&T` is alive
- After `&T` leaves scope, `&mut T` auto-restored
- This is **flow-sensitive analysis** — compiler tracks token state within function body

#### 2.6 Token Conflict Detection

Replaces RFC-009 v8's "cross-borrow checking". The principle is simpler — **flow-sensitive liveness analysis** on token values:

```yaoxiang
# ❌ &mut and derived &T cannot be simultaneously alive
bad_alias: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)        # p is frozen
    p.x = 10.0                      # ❌ Compile error: WriteToken in frozen state
    print(view.x)                   
}

# ✅ After freeze lifted, can continue using &mut
good_seq: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)        # p is frozen
    print(view.x)                   # Use &T
    # view leaves scope
    p.x = 10.0                      # ✅ WriteToken restored
}
```

**Detection method**: This is not a dedicated "borrow checker" — it's **flow-sensitive liveness analysis** on token values. Compiler tracks each token's state (alive/frozen/moved) within function body, exactly the same way it tracks any linear type value.

#### 2.7 Compiler Internals: Branding Mechanism

Users never see brands. Compiler internally assigns each token a compile-time unique identifier:

```
User-visible         Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is compile-time unique integer
```

Brand uses:
- **Anti-forgery**: Tokens can only be obtained from owner capsule or freeze operation, cannot be constructed out of thin air
- **Provenance tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries derived brand (`#N.field_x`), compiler can trace to parent token
- **Conflict detection**: Same-origin `WriteToken` and derived `ReadToken` cannot be simultaneously alive

Brands completely disappear after monomorphization and inlining. They don't exist in generated machine code. **Zero runtime overhead.**

#### 2.8 Auto-Borrow Selection Rules

Compiler auto-selects at call site by this priority:

```
1. If actual arg is used later → prefer creating token (&T or &mut T, based on method signature)
2. If actual arg is not used later → Move
3. Match priority order: &T < &mut T < Move
```

```yaoxiang
# Example: Auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Simplified Borrowing

| Feature | Simplified Borrowing (v8) | Borrow Tokens (v9) |
|---------|--------------------------|-------------------|
| Return references | ❌ Hard-coded prohibition | ✅ Token propagates with return value |
| Store in structs | ❌ Hard-coded prohibition | ✅ Token as struct field |
| Closure capture | ❌ Hard-coded prohibition | ✅ Closure captures token value |
| Special rules | 3 (params only / no return / no storage) | 0 — type properties naturally infer |
| Borrow checking | Dedicated cross-borrow checking | Type checker flow-sensitive liveness analysis |
| Lifetime annotations | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-size type, disappears after compilation) |
| Error messages | "Borrow cannot escape" | "WriteToken(#3) is frozen" (regular type error) |
| User mental model | Understand "borrowing's" special status | `&T` is copyable, `&mut T` is not copyable |

---

### 3. ref Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether it's backed by Rc or Arc, users don't need to care.

#### 3.1 Basic Usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Shared, compiler auto-selects implementation

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
    use(data)                     # Compiler: not cross-task → Rc
}
```

**User mental model**: `ref` = shared hold. That's it.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data flow analysis:

Doesn't escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks   → Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycles → silently allowed.
  ├── Structured concurrency guarantees unified release of all resources when task ends.
  ├── ref always stays alive, semantics aren't diluted.
  └── Users have the right to build bidirectional strong references within tasks (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, won't actually leak (parent task end releases all child task resources).
  ├── But cross-task strong references mean blurred ownership boundaries, worth pausing to rethink.
  ├── Default warn level, compilation passes but with a hint.
  └── Teams can set to deny in project config,纳入 CI quality gate.

```

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | No checking | Personal projects |
| `warn` (default) | Compilation passes, with hint | Development phase |
| `deny` | Compilation fails | Team CI quality gate |
| `forbid` | Compilation fails, cannot override | Org-level enforced rules |

```yaoxiang
# Intra-task cycles: silently allowed, bidirectional strong references
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Unified release at task end.
}

# Cross-task cycles: lint (default warn)
@block
parent_task: () -> Void = {
    shared_a = ref a
    shared_b = ref b
    spawn {
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task circular reference
    }
}
```

**Project config example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles rejected on CI
```

| Cycle type | Behavior | Reason |
|------------|----------|--------|
| Intra-task ref cycles | No checking | User's prerogative, unified release at task end |
| Cross-task ref cycles | lint (default warn) | Reminder to rethink, can be configured to deny |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not language built-in, it's a standard library type.** Use `ref` for daily needs. Advanced users who need fine-grained memory control manually introduce `Weak`.

#### 3.5 Borrow Tokens vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Glance / transform in place | Shared hold |
| Scope | Follows token value's scope | Cross-scope |
| Cost | Zero overhead (zero-size type) | Rc or Arc (compiler selects) |
| Escape | Can (token propagates with return/struct/closure) | Designed to escape |
| Cross-task | Cannot (token doesn't impl Send) | Can (compiler auto-selects Arc) |
| Cycles | Not applicable | Intra-task silently allowed, cross-task lint |

---

### 4. clone() —— Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, no interference
```

**When to use**: When you need to keep the original value, and Move or sharing aren't suitable.

### 5. unsafe + Raw Pointers (System-Level Programming)

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
- User guarantees no dangling / use-after-free
- Used for FFI, memory operations, etc. system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow tokens (zero overhead)     Move (zero overhead)      Sharing (pay-as-you-go)    Copy
   │                      │                  │                │
  &T copyable token      Default ownership   ref Rc/Arc       clone()
  &mut T linear token    transfer            Compiler auto    Explicit deep copy
   │                      │                  │                │
  Token value scope       Scope within        T -> T reflux     Any time
  Can return/store        T -> Void consume  ref cross-task → Arc
  structs/capture                               ref not cross-task → Rc
  closures                                        Intra-task cycles silently
  Zero-size, disappears                         Cross-task cycles lint
  after compilation                              std Weak escape

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

    # Move → Move: consume and reflux
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # Take it, transform, return it
    }

    # Return reference: token propagates with return value
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# Closure captures tokens (capability v8 couldn't have)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# Comprehensive usage
p = Point(1.0, 2.0)
p.print()                           # &Point token
p.shift(1.0, 1.0)                   # &mut Point token
p = p.scale(2.0)                    # Move → reflux
shared = ref p                      # ref share
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# Intra-task cycles: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, unified release at task end

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Send / Sync

| Type | Send | Sync | Description |
|------|------|------|-------------|
| Value types | ✅ | ✅ | Int, Float, Point... |
| `ref T` | ✅ | ✅ | Compiler auto-selects Rc/Arc |
| `&T` / `&mut T` | ❌ | ❌ | Tokens don't impl Send/Sync, cannot cross tasks |
| `*T` | ❌ | ❌ | Raw pointer, single-threaded |

---

## Performance Analysis

| Operation | Cost | Description |
|-----------|------|-------------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-size type, disappears after compilation, zero runtime overhead |
| `ref` (not cross-task)| Low | Compiled to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiled to Arc, atomic operation |
| `clone()` | Varies by type | Fast for small objects, slow for large |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|----------|-------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + Borrow checker | Compile-time checked | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + Borrow tokens** | **RAII** | **Task boundary release / cross-task lint / std Weak** | **Low** |

---

## Tradeoffs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, no global borrow checker. `&T` is copyable, `&mut T` is not copyable — two type properties
3. **Powerful**: Can return references, store in structs, capture in closures — expression power on par with Rust
4. **Compiler intelligent**: ref auto-selects Rc/Arc, call site auto-selects borrowing
5. **Deterministic**: ref means stays alive, won't silently become weak reference
6. **High performance**: Move zero-copy, tokens zero overhead (zero-size type, disappears after compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, function signatures returning references include extra generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the inevitable cost of sharing)
3. **unsafe risks**: User must guarantee correctness
4. **Cross-task cycles are lint not compile errors**: Unlike Rust which gives compile errors, default warn, teams need to configure deny to use as quality gate

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| GC | Runtime overhead, unpredictable pauses |
| Rust borrow checker | Needs lifetimes `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointers | Cannot do system-level programming |
| Expose Rc/Arc to users | Exposing implementation details to users, increased cognitive burden |
| Simplified borrowing (v8) | The "no escape" strategy sacrificed key expressiveness like closure capture, returning references, etc. |

---

## Design Decision Record

| Decision | Resolution | Reason | Date |
|----------|------------|--------|------|
| **Default** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimizes | Simple for users, compiler responsible | 2025-01-15 |
| **Borrowing** | `&T`/`&mut T` as zero-size token types | Type properties (Copy/Linear) naturally infer permissions, unified type system | 2025-01-15 |
| **Borrow tokens** | Replace simplified borrowing, `&T` Copy, `&mut T` Linear | Eliminate "no escape" special rules, support closure capture / return references / struct storage | 2026-05-29 |
| **Copying** | `clone()` | Explicit semantics | 2025-01-15 |
| **System-level** | `*T` + `unsafe` | Support system programming | 2025-01-15 |
| **Lifetimes** | Not implemented | Tokens are values, lifetimes uniformly managed by Move/RAII, reduces borrowing to ownership problem | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, invisible to users | Reduced cognitive burden | 2025-01-15 |
| **Circular references** | Intra-task no checking, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint configurable to deny | 2025-01-16 |
| **Weak** | Provided by standard library | Advanced users explicitly choose | 2025-01-16 |
| **Consume analysis** | Removed | Mini borrow checker, not needed | 2026-05-11 |
| **Ownership reflux** | Removed | `(T) -> T` signature is self-documenting | 2026-05-11 |
| **Empty state reuse** | Removed (as feature) | Reassigning after Move is natural behavior | 2026-05-11 |
| **Inverse functions / partial consume / three-tier field mutability** | Removed | Over-engineering | 2026-05-11 |

### Version History

| Version | Major Changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| v4 | Default Move + explicit ref | 2025-01-15 |
| v5 | Structured concurrency + circular reference handling | 2025-01-16 |
| v6 | Added empty state reuse, ownership reflux | 2025-02-04 |
| v7 | Added consume analysis, inverse functions, field-level mutability | 2025-02-05 |
| **v8** | **Removed over-engineering, added simplified borrowing &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces simplified borrowing, unified type system** | **2026-05-29** |

### Open Issues

| Topic | Description | Status |
|-------|-------------|--------|
| Drop syntax | Need explicit `drop()` function | Pending discussion |
| Escape analysis algorithm | ref cross-task detection implementation | Pending discussion |
| Token conflict detection | Flow-sensitive liveness analysis, see below | ✅ Resolved |

### Token Conflict Detection: Flow-Sensitive Liveness Analysis

**Analysis scope**: Function body only. Flow-sensitive liveness analysis on token values, tracking each token's state (alive/frozen/moved).

**Level 1: Call site checking** — same actual arg cannot simultaneously create `&mut` token and other tokens:

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p simultaneously derives &mut and & tokens, compiler rejects
```

**Level 2: Function body flow-sensitivity** — after `&mut` token passed to call, released on return, can create new token afterwards:

```yaoxiang
process_twice: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)    # &mut token passed to shift, released after shift returns
    print_info(p)          # Recreate &Point token, no conflict
}
```

**Level 3: Frozen state tracking** — while `&T` token from `freeze` is alive, original `&mut` token unavailable:

```yaoxiang
frozen: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)    # p enters frozen state
    print(view.x)
    p.x = 10.0                  # ❌ Compile error: WriteToken in frozen state
}
```

**Not needed**: Cross-function lifetime tracking, global alias analysis, borrow graph constraint solving, NLL, `'a` annotations. Because tokens are values, value liveness analysis is uniformly handled by type checker — exactly the same as tracking any linear type value.

---

## References

### YaoXiang Official Documentation

- [Language Specification](../language-spec.md)
- [Design Manifesto](../manifesto.md)
- [RFC-001 Concurrent Model](./001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generic Type System Design](./011-generic-type-system.md)
- [YaoXiang Guide](../guides/YaoXiang-book.md)

### External References

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author draft, awaiting review submission |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes formal design document |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory |