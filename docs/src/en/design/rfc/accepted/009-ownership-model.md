```yaml
---
title: "RFC-009: Ownership Model Design"
---

# RFC-009: Ownership Model Design

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-08
> **Last Updated**: 2026-05-29 (Borrow token system replaces stripped-down borrowing, unified type system)

## Abstract

This document defines the **Ownership Model** for the YaoXiang programming language.

**Core Design—Five Concepts, One Gradient**:

```
Glance/Modify in-place    Move          Shared持有         Clone一份        System-level
    │                     │              │              │              │
   &T                   Move           ref           clone()        unsafe
  &mut T              Zero-copy      Compiler auto    Explicit      *T
  Zero-size token      Default        picks Rc/Arc   deep copy     User responsible
  Type properties                                               
  naturally infer                                               
  permissions                                                  
```

- **Move (Default)**: Assignment/parameter passing/return = ownership transfer, zero-copy, RAII auto-release
- **`&T` / `&mut T` (Borrow Tokens)**: Zero-size compile-time token types. `&T` is copyable (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are naturally inferred from type properties, no special rules needed. Can be returned, stored in structs, and captured by closures.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically picks Rc (not cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointers, system-level escape hatch

**Eliminated Complexity**:
- ❌ No lifetime `'a`
- ❌ No borrow checker (the type properties of tokens—Copy/Linear—replace a dedicated borrow checker)
- ❌ No GC
- ❌ No special rules like "no escape" (tokens are ordinary types, scope handled uniformly by type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler picks automatically)

> **Programming burden**: `&T` is copyable, `&mut T` is not copyable—two type properties, zero special rules, fully automated by compiler.
> **Performance guarantee**: Move is zero-overhead, tokens are zero-overhead (zero-size types, disappear after compilation), ref is pay-for-play, no GC pauses.

## Motivation

### Why Do We Need an Ownership Model?

| Language | Memory Management | Problems |
|----------|-------------------|----------|
| C/C++ | Manual | Memory leaks, dangling pointers, double free |
| Java/Python | GC | Latency spikes, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow Checker | Lifetime `'a` steep learning curve |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p is no longer readable

# 2. &T / &mut T Borrow tokens (zero-overhead, type properties naturally infer permissions)
print_info(p2)                 # Compiler automatically creates &Point token, released after use
shift(p2, 1.0, 1.0)            # Compiler automatically creates &mut Point token

# 3. ref = sharing (compiler automatically picks Rc/Arc)
shared = ref p2                # Cross-scope holding
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
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value semantics, zero-copy)** |
| Borrowing | `&T`/`&mut T`, returnable, requires lifetimes | **`&T`/`&mut T` zero-size tokens, Copy/Linear type properties naturally infer** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler automatically picks Rc/Arc)** |
| Copying | `clone()` | `clone()` |
| Raw pointers | `*T` | `*T` |
| Lifetimes | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker flow-sensitive liveness analysis (token state tracking)** |
| Cyclic references | Manual Weak | **Task-end unified release / cross-task lint / std Weak** |

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
- Original binding is unreadable after move (compile error)
- RAII: auto-release when scope ends
- Function signature `(T) -> T` is self-documenting—consumes T, returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core Principle**: `&T` and `&mut T` are zero-size compile-time token types. They are not "references" but "type-level proof of access permissions.**

#### 2.1 Two Type Properties

```
&T      →  Zero-size, copyable (Copy), grants read-only permission
&mut T  →  Zero-size, linear (non-Copy), grants exclusive read-write permission
```

**This is not a "rule" to memorize—this is a fundamental property of the type system.** Copy types can be freely copied (multiple `&T` coexist), linear types cannot be copied (`&mut T` is inherently unique). There is no "borrow checker"—only the type checker doing what it has always done.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter type, decide permission needed
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point token grants write permission
    self.y = self.y + dy
}

# Call side: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler automatically creates &Point token
p.shift(1.0, 1.0)                  # Compiler automatically creates &mut Point token
p.print()                          # OK, previous token was released when shift call ended

# Free functions work the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist—Copy type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escape" Is Unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T`—can only be used as parameters, cannot be returned, cannot be stored in structs. This was patching the "borrow" concept.

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scoping rules as all other types.

**Returning references—naturally supported**:

```yaoxiang
# ✅ Token propagates with return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Child token and parent token return together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Storing in structs—naturally supported**:

```yaoxiang
# ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,      # Token field—holds read-only view of target
}

# view's token is derived from target, Window owns both
# As long as Window exists, view token is valid
```

**Closure capture—naturally supported**:

```yaoxiang
# ✅ Closures capture tokens, just like capturing any value
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    # Closure captures threshold's &Float token (Copy type, freely copied into closure)
    items.filter(|p| p.x > threshold)
}

# This is something RFC-009 v8 couldn't do—v8 prohibited closure capture of borrows
```

**Cross-task—tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token type does not implement Send
}

# This is not a special rule—&T token type does not implement Send trait
# If cross-task sharing is needed, use ref
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token Lifetime

Token lifetime is governed by **ordinary scoping rules**, no lifetime parameters needed:

- Tokens in function parameters: live during the call, released after the call ends
- Returned tokens: ownership transfers to the caller
- Tokens stored in structs: live with the struct
- Tokens captured by closures: live with the closure

The compiler doesn't need `'a` annotations because tokens are **values**, and value lifetimes are uniformly managed by the ownership system (Move/RAII). **Dimensionality reduction: borrow problem → ownership problem.**

#### 2.5 Freeze Mechanism

`&mut T` tokens can be temporarily "frozen" to produce `&T` tokens:

```yaoxiang
modify_and_read: (p: &mut Point) -> Void = {
    p.x = 10.0                      # Use &mut Point to modify
    
    # Freeze &mut, get read-only view
    view: &Point = freeze(p)         # p is frozen here
    print(view.x)                   # Read through &Point
    print(view.y)
    # view goes out of scope, freeze lifted
    
    p.y = 20.0                      # &mut Point restored
}
```

`freeze` semantics:
- Accepts `&mut T`, returns `&T`
- During `&T` lifetime, original `&mut T` is unavailable
- After `&T` leaves scope, `&mut T` auto-restores
- This is **flow-sensitive analysis**—compiler tracks token state within function body

#### 2.6 Token Conflict Detection

Replaces RFC-009 v8's "cross-borrow checking." The principle is simpler—**flow-sensitive liveness analysis** on token values:

```yaoxiang
# ❌ &mut and derived &T cannot be simultaneously live
bad_alias: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)        # p is frozen
    p.x = 10.0                      # ❌ Compile error: WriteToken is frozen
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

**Detection method**: This is not a dedicated "borrow checker"—this is **flow-sensitive liveness analysis** on token values. The compiler tracks each token's state (live/frozen/moved) within the function body, exactly the same way it tracks any linear type value.

#### 2.7 Compiler Internals: Branding Mechanism

Users never see brands. The compiler internally assigns a compile-time unique identifier to each token:

```
User view            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand uses:
- **Anti-counterfeiting**: Tokens can only be obtained from owner capsules or freeze operations, cannot be凭空 constructed
- **Tracing derivations**: When deriving `&Float` from `&Point` (field access), `&Float` carries a derived brand (`#N.field_x`), compiler can trace back to parent token
- **Conflict detection**: Same-source `WriteToken` and derived `ReadToken` cannot be simultaneously live

Brands completely disappear after monomorphization and inlining. They don't exist in generated machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

The call-side compiler selects automatically by this priority:

```
1. If actual argument has subsequent uses → prefer creating token (&T or &mut T, based on method signature)
2. If actual argument has no subsequent uses → Move
3. Matching priority: &T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Stripped-Down Borrowing

| Feature | Stripped-down Borrowing (v8) | Borrow Tokens (v9) |
|---------|------------------------------|-------------------|
| Return references | ❌ Hardcoded prohibition | ✅ Token propagates with return value |
| Store in structs | ❌ Hardcoded prohibition | ✅ Token as struct field |
| Closure capture | ❌ Hardcoded prohibition | ✅ Closure captures token value |
| Special rules | 3 (parameter only/no return/no storage) | 0—type properties naturally infer |
| Borrow checking | Dedicated cross-borrow check | Type checker flow-sensitive liveness analysis |
| Lifetime annotations | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-size types, disappear after compilation) |
| Error messages | "Borrow cannot escape" | "WriteToken(#3) is frozen" (regular type error) |
| User mental model | Understand "borrow" special status | `&T` is copyable, `&mut T` is not copyable |

---

### 3. ref Keyword (Compiler Auto-optimization)

`ref` is the only way to share across scopes. Whether it's backed by Rc or Arc, users don't need to care.

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
    use(data)                     # Compiler: not cross-task → Rc
}
```

**User mental model**: `ref` = shared holding. That's it.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks        → Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycles → silently allowed.
  ├── Structured concurrency guarantees unified release of all resources when task ends.
  ├── ref always stays alive, semantics are not diluted.
  └── Users have the right to build bidirectional strong references within tasks (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, no actual leaks will occur (parent task end releases all child task resources).
  ├── But cross-task strong references mean ownership boundaries are blurred, worth pausing to rethink.
  ├── Default warn level, compilation passes but with a hint.
  └── Teams can set to deny in project config,纳入 CI quality gate.

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | No checking | Personal projects |
| `warn` (default) | Compilation passes, hint shown | Development phase |
| `deny` | Compilation fails | Team CI quality gate |
| `forbid` | Compilation fails, cannot be overridden | Organization-level mandatory rules |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong reference
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Unified release when task ends.
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
cross-task-cycle = "deny"    # Cross-task cycles fail CI
```

| Cycle type | Behavior | Reason |
|------------|----------|--------|
| Intra-task ref cycle | No checking | User's prerogative, unified release at task end |
| Cross-task ref cycle | lint (default warn) | Prompt to reconsider, can be set to deny |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not language built-in, it's a standard library type.** For daily use, `ref` is enough. Advanced users who need fine-grained memory control manually import `Weak`.

#### 3.5 Borrow Tokens vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Glance / modify in-place | Shared holding |
| Scope | Follows token value's scope | Cross-scope |
| Cost | Zero overhead (zero-size type) | Rc or Arc (compiler picks) |
| Escape | Can (token propagates with return/struct/closure) | Is inherently for escaping |
| Cross-task | Cannot (token doesn't implement Send) | Can (compiler auto-picks Arc) |
| Can form cycles | Not applicable | Intra-task silently allowed, cross-task lint |

---

### 4. clone() —— Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, don't affect each other
```

**When to use**: When the original value needs to be retained and Move or sharing aren't suitable.

### 5. unsafe + Raw Pointers (System-level Programming)

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
- Used for FFI, memory operations, other system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow tokens (zero overhead)    Move (zero overhead)    Sharing (pay-for-play)    Clone
   │                              │                    │                    │
  &T copyable token            Default ownership    ref Rc/Arc           clone()
  &mut T linear token          transfer             Compiler auto        Explicit deep copy
   │                              │                    │                    │
  Token value scope              Scoped               Cross-scope          Anytime
  Can return/store in structs    T -> T reflux        ref cross-task → Arc  Independent copy
  Can be captured by closures    T -> Void consume    ref not cross-task → Rc
  Zero-size disappears after                            Intra-task cycles silent
  compilation                                           Cross-task cycles lint
                                                         Std Weak escape
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
        self                            # Take it, modify, give it back
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
p = p.scale(2.0)                    # Move → reflux
shared = ref p                      # ref shared
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# Intra-task cycles: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, unified release when task ends

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Send / Sync

| Type | Send | Sync | Notes |
|------|------|------|-------|
| Value types | ✅ | ✅ | Int, Float, Point... |
| `ref T` | ✅ | ✅ | Compiler auto-picks Rc/Arc |
| `&T` / `&mut T` | ❌ | ❌ | Token does not implement Send/Sync, cannot cross tasks |
| `*T` | ❌ | ❌ | Raw pointer, single-threaded |

---

## Performance Analysis

| Operation | Cost | Notes |
|-----------|------|-------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-size types, disappear after compilation, zero runtime overhead |
| `ref` (not cross-task) | Low | Compiled to Rc, non-atomic operations |
| `ref` (cross-task) | Medium | Compiled to Arc, atomic operations |
| `clone()` | Varies by type | Fast for small objects, slow for large objects |
| `unsafe + *T` | Zero | Direct memory operations |

### Comparison

| Language | Sharing mechanism | Memory management | Cycle handling | Complexity |
|----------|-------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + borrow checker | Compile-time checking | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + borrow tokens** | **RAII** | **Task boundary release / cross-task lint / std Weak** | **Low** |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, no global borrow checker. `&T` is copyable, `&mut T` is not copyable—two type properties
3. **Powerful**: Can return references, store in structs, capture in closures—expressiveness on par with Rust
4. **Compiler intelligent**: ref auto-picks Rc/Arc, call side auto-selects borrowing
5. **Deterministic**: ref is always kept alive, won't silently become weak reference
6. **High performance**: Move is zero-copy, tokens are zero-overhead (zero-size types, disappear after compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, which appear in function signatures that return references as additional generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the inevitable cost of sharing)
3. **unsafe risk**: User must guarantee correctness
4. **Cross-task cycles are lint, not compile errors**: Unlike Rust's compile-time rejection, default is warn, team needs to configure deny to use as quality gate

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| GC | Has runtime overhead, unpredictable pauses |
| Rust borrow checker | Requires lifetime `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointers | Cannot do system-level programming |
| Expose Rc/Arc to users | Dumping implementation details on users, increases cognitive burden |
| Stripped-down borrowing (v8) | The no-escape strategy sacrificed key expressiveness like closure capture, returning references, etc. |

---

## Design Decision Log

| Decision | Choice | Reason | Date |
|----------|--------|--------|------|
| **Default** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimizes | User simplicity, compiler responsible | 2025-01-15 |
| **Borrowing** | `&T`/`&mut T` as zero-size token types | Type properties (Copy/Linear) naturally infer permissions, unified type system | 2025-01-15 |
| **Borrow tokens** | Replace stripped-down borrowing, `&T` Copy, `&mut T` Linear | Eliminate special rules like "no escape", support closure capture/return references/store in structs | 2026-05-29 |
| **Copying** | `clone()` | Explicit semantics | 2025-01-15 |
| **System-level** | `*T` + `unsafe` | Support system programming | 2025-01-15 |
| **Lifetimes** | Not implemented | Tokens are values, lifetimes uniformly managed by Move/RAII, dimensionality reduction: borrow → ownership | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, invisible to users | Reduce cognitive burden | 2025-01-15 |
| **Cyclic references** | Intra-task no checking, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint can be set to deny | 2025-01-16 |
| **Weak** | Provided by standard library | Advanced users explicitly choose | 2025-01-16 |
| **Consume analysis** | Removed | Mini borrow checker, not needed | 2026-05-11 |
| **Ownership reflux** | Removed | `(T) -> T` signature is self-documenting | 2026-05-11 |
| **Empty state reuse** | Removed (as feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function / partial consume / field three-layer mutability** | Removed | Over-engineering | 2026-05-11 |

### Version History

| Version | Major Changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| v4 | Default Move + explicit ref | 2025-01-15 |
| v5 | Structured concurrency + cyclic reference handling | 2025-01-16 |
| v6 | Added empty state reuse, ownership reflux | 2025-02-04 |
| v7 | Added consume analysis, inverse functions, field-level mutability | 2025-02-05 |
| **v8** | **Removed over-engineering, added stripped-down borrowing &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces stripped-down borrowing, unified type system** | **2026-05-29** |

### Open Issues

| Topic | Description | Status |
|-------|-------------|--------|
| Drop syntax | Whether explicit `drop()` function is needed | Pending discussion |
| Escape analysis algorithm | ref cross-task detection implementation | Pending discussion |
| Token conflict detection | Flow-sensitive liveness analysis, see below | ✅ Resolved |

### Token Conflict Detection: Flow-Sensitive Liveness Analysis

**Analysis scope**: Function body only. Flow-sensitive liveness analysis on token values, tracking each token's state (live/frozen/moved).

**Layer 1: Call site check**—the same actual argument cannot simultaneously create `&mut` token and other tokens:

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p simultaneously derives &mut and & tokens, compiler rejects
```

**Layer 2: Function body flow-sensitivity**—after `&mut` token is passed to a call, it's released when the call returns, and tokens can be created again:

```yaoxiang
process_twice: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)    # &mut token passed to shift, token released when shift returns
    print_info(p)          # Recreate &Point token, no conflict
}
```

**Layer 3: Frozen state tracking**—during `&T` token's lifetime from `freeze`, original `&mut` token is unavailable:

```yaoxiang
frozen: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)    # p enters frozen state
    print(view.x)
    p.x = 10.0                  # ❌ Compile error: WriteToken is frozen
}
```

**What is not needed**: Cross-function lifetime tracking, global alias analysis, borrow graph constraint solving, NLL, `'a` annotations. Because tokens are values, value liveness analysis is handled uniformly by the type checker—exactly the same as tracking any linear type value.

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

## Lifecycle and Disposition

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author draft, awaiting submission for review |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes a formal design document |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory |
```