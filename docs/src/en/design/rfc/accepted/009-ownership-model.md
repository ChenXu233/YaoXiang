---
title: "RFC-009: Ownership Model Design"
---

# RFC-009: Ownership Model Design

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-08
> **Last Updated**: 2026-05-29 (Borrow Token System Replaces Simplified Borrow, Unified Type System)

## Abstract

This document defines the **Ownership Model** for the YaoXiang programming language.

**Core Design — Five Concepts, One Gradient**:

```
Glance / Modify In-Place    Move           Shared Hold         Clone          Systems-Level
     │                       │                │                 │               │
    &T                     Move            ref              clone()        unsafe
   &mut T                  Zero-Copy       Compiler Auto     Explicit       *T
   Zero-Size Token         Default         picks Rc/Arc     Deep Copy      User Responsible
   Type Properties                                               
   Naturally Infer                                             
   Permissions                                                 
```

- **Move (default)**: Assignment/parameter passing/return = ownership transfer, zero-copy, RAII automatic release
- **`&T` / `&mut T` (Borrow Tokens)**: Zero-size compile-time token types. `&T` is duplicable (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are naturally inferred from type properties, no special rules needed. Can be returned, stored in structs, and captured by closures.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically picks Rc (not cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, systems-level escape hatch

**Eliminated Complexity**:
- ❌ No lifetime `'a`
- ❌ No borrow checker (token type properties — Dup/Linear — replace a dedicated borrow checker)
- ❌ No GC
- ❌ No "no escape" special rules (tokens are ordinary types, scope handled uniformly by type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler auto-selects)

> **Programming burden**: `&T` is duplicable, `&mut T` is non-duplicable — two type properties, zero special rules, fully automated by compiler.
> **Performance guarantee**: Move is zero overhead, tokens are zero overhead (zero-size types, disappear after compilation), ref is pay-per-use, no GC pauses.

## Motivation

### Why Do We Need an Ownership Model?

| Language | Memory Management | Problems |
|----------|-------------------|----------|
| C/C++ | Manual | Memory leaks, dangling pointers, double frees |
| Java/Python | GC | Latency spikes, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow Checker | Lifetime `'a` steep learning curve |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p is no longer readable

# 2. &T / &mut T Borrow Tokens (zero overhead, type properties naturally infer permissions)
print_info(p2)                 # Compiler automatically creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler automatically creates &mut Point token

# 3. ref = Shared (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope hold
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = Explicit copy
backup = p2.clone()            # Deep copy, exclusive

# 5. unsafe + *T = Systems-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature | Rust | YaoXiang |
|---------|------|----------|
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)** |
| Borrowing | `&T`/`&mut T`, returnable, lifetimes required | **`&T`/`&mut T` zero-size tokens, Dup/Linear type properties naturally infer** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-selects Rc/Arc)** |
| Cloning | `clone()` | `clone()` |
| Raw pointers | `*T` | `*T` |
| Lifetimes | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker flow-sensitive liveness analysis (token state tracking)** |
| Circular references | Manual Weak | **Task end unified release / cross-task lint / standard library Weak** |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: Assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p is no longer readable

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p is rebound, type must match

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
- After move, original binding is not readable (compile error)
- RAII: automatic release when scope ends
- Function signature `(T) -> T` is itself documentation — consumes T, returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core Principle: `&T` and `&mut T` are zero-size compile-time token types. They are not "references" but "type-level proof of access permissions".**

#### 2.1 Two Type Properties

```
&T      →  zero-size, duplicable (Dup), grants read-only permission
&mut T  →  zero-size, linear (non-Dup), grants exclusive read-write permission
```

**This is not a "rule" to memorize — it is a fundamental property of the type system.** Dup types can be freely duplicated (multiple `&T` coexist), linear types cannot be duplicated (`&mut T` is inherently unique). There is no "borrow checker" — only the type checker doing what it has always done.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter types, decide required permissions
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
p.print()                          # OK, previous token was released when shift call ended

# Free functions work the same way
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escape" Is Unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — can only be parameters, cannot be returned, cannot be stored in structs. This is patching the "borrow" concept.

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scope rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Token propagates with return value
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

# view's token is derived from target, Window holds ownership of both
# As long as Window exists, view token is valid
```

**Closure capture — naturally supported**:

```yaoxiang
# ✅ Closures capture tokens, like capturing any value
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    # Closure captures threshold's &Float token (Dup type, freely copied into closure)
    items.filter(|p| p.x > threshold)
}

# This is something RFC-009 v8 couldn't do — v8 forbids closure capture of borrows
```

**Cross-task — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task boundary
}

# This is not a special rule — tokens are compile-time permission proofs
# For cross-task sharing, use ref
# If cross-task sharing is needed, use ref
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Token is a permission proof, not an ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not ownable
}
```

#### 2.4 Token Lifetime

Token lifetime is determined by **ordinary scope rules**, no lifetime parameters needed:

- Token in function parameter: alive during call, released after call ends
- Returned token: ownership transfers to caller
- Token stored in struct: lives with the struct
- Closure-captured token: lives with the closure

Compiler doesn't need `'a` annotations because tokens are **values**, and value lifetimes are uniformly managed by the ownership system (Move/RAII). **Dimensionality reduction: borrow problem → ownership problem.**

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
- Original `&mut T` is unavailable while `&T` is alive
- After `&T` leaves scope, `&mut T` automatically restores
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

# ✅ After freeze lifted, &mut can be used again
good_seq: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)        # p is frozen
    print(view.x)                   # Use &T
    # view leaves scope
    p.x = 10.0                      # ✅ WriteToken restored
}
```

**Detection method**: This is not a dedicated "borrow checker" — it is **flow-sensitive liveness analysis** on token values. Compiler tracks each token's state (alive/frozen/moved) within function body, exactly the same way it tracks any linear type value.

#### 2.7 Compiler Internals: Branding Mechanism

Users never see brands. Compiler internally assigns compile-time unique identifiers to each token:

```
What user sees         Compiler internal representation
────────────────────────────────────────────────────────
&Point               →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point           →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand uses:
- **Anti-counterfeiting**: Tokens can only be obtained from owner capsule or freeze operation, cannot be fabricated
- **Derivation tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries derived brand (`#N.field_x`), compiler can trace back to parent token
- **Conflict detection**: Same-origin `WriteToken` and derived `ReadToken` cannot be simultaneously alive

Brands completely disappear after monomorphization and inlining. They don't exist in generated machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

Call-site compiler automatically selects by this priority:

```
1. If actual argument is used later → prioritize creating token (&T or &mut T, based on method signature)
2. If actual argument is not used later → Move
3. Match priority order: &T < &mut T < Move
```

```yaoxiang
# Example: Auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Simplified Borrow

| Feature | Simplified Borrow (v8) | Borrow Tokens (v9) |
|---------|------------------------|---------------------|
| Return references | ❌ Hard-coded prohibition | ✅ Token propagates with return value |
| Store in structs | ❌ Hard-coded prohibition | ✅ Token as struct field |
| Closure capture | ❌ Hard-coded prohibition | ✅ Closure captures token value |
| Special rules | 3 (parameters only/no return/no storage) | 0 — type properties naturally infer |
| Borrow checking | Dedicated cross-borrow checking | Type checker flow-sensitive liveness analysis |
| Lifetime annotations | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-size types, disappear after compilation) |
| Error messages | "Borrow cannot escape" | "WriteToken(#3) is frozen" (normal type error) |
| User mental model | Understand "borrow's" special status | `&T` is duplicable, `&mut T` is non-duplicable |

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

Does not escape to other tasks → Rc (non-atomic reference count, low overhead)
Escapes to other tasks         → Arc (atomic reference count, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Within-task cycles → silently allowed.
  ├── Structured concurrency guarantees unified release of all resources when task ends.
  ├── ref always keeps alive, semantics are not diluted.
  └── Users have the right to build bidirectional strong references within a task (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, won't actually leak (parent task end releases all child resources).
  ├── But cross-task strong references mean ownership boundaries are blurred, worth pausing to reconsider.
  ├── Default warn level, compiles but with a hint.
  └── Teams can set to deny in project config,纳入 CI quality gate.
```

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | No checking | Personal projects |
| `warn` (default) | Compile with hint | Development stage |
| `deny` | Compile failure | Team CI quality gate |
| `forbid` | Compile failure, cannot override | Organization-level mandatory rules |

```yaoxiang
# Within-task cycles: silently allowed, bidirectional strong references
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Released together when task ends.
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
| Within-task ref cycles | No checking | User's prerogative, released together when task ends |
| Cross-task ref cycles | lint (default warn) | Reminder to reconsider, can be configured deny |

#### 3.4 Weak: Standard Library Provided

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not language built-in, it's a standard library type.** Daily use is covered by `ref`. Advanced users who need fine-grained memory control manually introduce `Weak`.

#### 3.5 Borrow Tokens vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Glance / modify in-place | Shared hold |
| Scope | Follows token value's scope | Cross-scope |
| Cost | Zero overhead (zero-size type) | Rc or Arc (compiler selected) |
| Escape | Can (token propagates with return value/struct/closure) | By nature escapes |
| Cross-task | Cannot (token is compile-time permission proof, cannot cross task boundary) | Can (compiler auto-selects Arc) |
| Cycles | Not involved | Within-task silently allowed, cross-task lint |

---

### 4. clone() — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, don't affect each other
```

**When to use**: When you need to keep the original value and it's not suitable for Move or sharing.

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
- User guarantees no dangling, no use after free
- Used for FFI, memory operations, etc. systems-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow Tokens (Zero Overhead)    Move (Zero Overhead)    Shared (Pay-Per-Use)    Copy
   │                                │                    │                │
  &T Duplicable Token            Default Ownership      ref Rc/Arc       clone()
  &mut T Linear Token            Transfer              Compiler Auto     Explicit
   │                                │                    │              Deep Copy
   │                                │                    │                │
  Token value scope               Within scope          Cross-scope      Any time
  Returnable/storable in struct   T -> T flowback       ref cross-task → Arc
  Capturable by closure           T -> Void consume     ref single-task → Rc
  Zero-size disappears                                    Within-task cycles allowed
  after compilation                                        Cross-task cycles lint
                                                           Standard library Weak escape
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

    # Move → Move: consume and flowback
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # Take it, modify it, give it back
    }

    # Return reference: token propagates with return value
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# Closure capture tokens (capability v8 couldn't do)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# Comprehensive usage
p = Point(1.0, 2.0)
p.print()                           # &Point token
p.shift(1.0, 1.0)                   # &mut Point token
p = p.scale(2.0)                    # Move → flowback
shared = ref p                      # ref shared
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# Within-task cycles: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, released together when task ends

# unsafe systems-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Dup Type Property

`Dup` (Duplicable) is a compiler-managed type property meaning **shallow copy**: assignment/parameter passing copies the handle/token, underlying data is shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creates independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies handles to share data, Clone creates independent copies. A type can support both Dup and Clone, or only one.

| Type | Dup | Clone | Description |
|------|-----|-------|-------------|
| `&T` | ✅ (duplicate token, multiple views point to same data) | ✅ | Read-only token |
| `ref T` | ✅ (ref count +1, shared heap data) | ✅ | Shared hold (compiler auto-selects Rc/Arc) |
| String, Bytes | ✅ (internal ref count, copy handle shares underlying buffer) | ✅ | String/bytes |
| `&mut T` | ❌ (linear, exclusive) | ❌ | Mutable token |
| `*T` | ❌ | ❌ | Raw pointer |
| struct | Derived (automatically derived when all fields are Dup) | ✅ | Struct |

**Primitive value types** (Int, Float, Bool, Char) have compiler-built-in value copy semantics on assignment — two values are completely independent, not shallow copy. They don't belong to the Dup type property but are handled natively by the compiler.

---

## Performance Analysis

| Operation | Cost | Description |
|-----------|------|-------------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-size type, disappears after compilation, zero runtime overhead |
| `ref` (single-task) | Low | Compiles to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiles to Arc, atomic operation |
| `clone()` | Varies by type | Fast for small objects, slow for large objects |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|----------|-------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + borrow checker | Compile-time check | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + borrow tokens** | **RAII** | **Task boundary release / cross-task lint / standard library Weak** | **Low** |

---

## Tradeoffs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, no global borrow checker. `&T` is duplicable, `&mut T` is non-duplicable — two type properties
3. **Powerful**: Can return references, store in structs, closure capture — expression power on par with Rust
4. **Compiler intelligent**: ref auto-selects Rc/Arc, call site auto-selects borrow
5. **Deterministic**: ref keeps alive, won't quietly become weak reference
6. **High performance**: Move zero-copy, tokens zero overhead (zero-size types, disappear after compilation)
7. **Flexible**: `unsafe + *T` supports systems-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, function signatures returning references will reflect additional generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the inevitable cost of sharing)
3. **unsafe risk**: Users must guarantee correctness
4. **Cross-task cycles are lint not compile error**: Unlike Rust's compile error, default warn, requires team config to set deny for quality gate

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| GC | Has runtime overhead, unpredictable pauses |
| Rust borrow checker | Requires lifetime `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointers | Cannot do systems-level programming |
| Expose Rc/Arc to users | Leaking implementation details to users, increases cognitive burden |
| Simplified borrow (v8) | The "no escape" strategy sacrificed critical expressiveness like closure capture, returning references, etc. |

---

## Design Decision Record

| Decision | Made | Reason | Date |
|----------|------|--------|------|
| **Default value** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimizes | Simple for users, compiler responsible | 2025-01-15 |
| **Borrowing** | `&T`/`&mut T` as zero-size token types | Type properties (Dup/Linear) naturally infer permissions, unified type system | 2025-01-15 |
| **Borrow tokens** | Replaces simplified borrow, `&T` Dup, `&mut T` Linear | Eliminates special rules like "no escape", supports closure capture/return references/store in structs | 2026-05-29 |
| **Copying** | `clone()` | Explicit semantics | 2025-01-15 |
| **Systems-level** | `*T` + `unsafe` | Supports systems programming | 2025-01-15 |
| **Lifetimes** | Not implemented | Tokens are values, lifetimes managed by Move/RAII, dimensionality reduction: borrow → ownership | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, invisible to users | Reduces cognitive burden | 2025-01-15 |
| **Circular references** | No checking within task, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint configurable deny | 2025-01-16 |
| **Weak** | Standard library provided | Advanced users explicitly choose | 2025-01-16 |
| **Consume analysis** | Removed | Mini borrow checker, not needed | 2026-05-11 |
| **Ownership flowback** | Removed | `(T) -> T` signature is itself documentation | 2026-05-11 |
| **Empty state reuse** | Removed (as feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function/partial consume/field three-tier mutability** | Removed | Over-engineering | 2026-05-11 |

### Version History

| Version | Major Changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| v4 | Default Move + explicit ref | 2025-01-15 |
| v5 | Structured concurrency + circular reference handling | 2025-01-16 |
| v6 | Added empty state reuse, ownership flowback | 2025-02-04 |
| v7 | Added consume analysis, inverse functions, field-level mutability | 2025-02-05 |
| **v8** | **Removed over-engineering, added simplified borrow &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces simplified borrow, unified type system** | **2026-05-29** |

### Open Issues

| Issue | Description | Status |
|-------|-------------|--------|
| Drop syntax | Whether explicit `drop()` function needed | Pending discussion |
| Escape analysis algorithm | ref cross-task detection implementation | Pending discussion |
| Token conflict detection | Flow-sensitive liveness analysis, see below | ✅ Resolved |

### Token Conflict Detection: Flow-Sensitive Liveness Analysis

**Analysis scope**: Function body only. Flow-sensitive liveness analysis on token values, tracking each token's state (alive/frozen/moved).

**Layer 1: Call site checking** — same actual argument cannot simultaneously create `&mut` token and other tokens:

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p simultaneously derives &mut and & tokens, compiler rejects
```

**Layer 2: Function body flow-sensitivity** — after `&mut` token passed to call, token released when call returns, can create token again:

```yaoxiang
process_twice: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)    # &mut token passed to shift, token released when shift returns
    print_info(p)          # Recreate &Point token, no conflict
}
```

**Layer 3: Frozen state tracking** — while `&T` token from `freeze` is alive, original `&mut` token is unavailable:

```yaoxiang
frozen: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)    # p enters frozen state
    print(view.x)
    p.x = 10.0                  # ❌ Compile error: WriteToken is in frozen state
}
```

**What's not needed**: Cross-function lifetime tracking, global alias analysis, borrow graph constraint solving, NLL, `'a` annotations. Because tokens are values, value liveness analysis is handled uniformly by the type checker — exactly the same as tracking any linear type value.

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
| **Accepted** | `docs/design/accepted/` | Becomes official design document |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory |