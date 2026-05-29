---
title: "RFC-009: Ownership Model Design"
---

# RFC-009: Ownership Model Design

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-08
> **Last Updated**: 2026-05-29 (Borrow token system replaces simplified borrowing, unified type system)

## Abstract

This document defines the **ownership model** of the YaoXiang programming language.

**Core design—five concepts, one gradient**:

```
Glance/Modify in place    Take away           Shared ownership        Clone a copy        System-level
        │                    │                    │                    │                 │
       &T                  Move                ref                clone()            unsafe
     &mut T              Zero-copy          Compiler auto         Explicit           *T
  Zero-sized token        Default          selects Rc/Arc         deep copy         User responsible
  Type attributes                                              No special rules needed
  naturally derive                                               for permissions
```

- **Move (Default)**: Assignment/argument passing/return = ownership transfer, zero-copy, RAII automatic deallocation
- **`&T` / `&mut T` (Borrow tokens)**: Zero-sized compile-time token types. `&T` is duplicable (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are naturally derived from type attributes, no special rules needed. Can be returned, stored in structs, and captured by closures.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically selects Rc (non-cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, system-level escape hatch

**Eliminated complexity**:
- ❌ No lifetimes `'a`
- ❌ No borrow checker (token type attributes—Dup/Linear—replace a dedicated borrow checker)
- ❌ No GC
- ❌ No "no escaping" special rules (tokens are ordinary types, scope handled uniformly by type system)
- ❌ Users don't need to know the difference between Rc and Arc (compiler auto-selects)

> **Programming burden**: `&T` is duplicable, `&mut T` is non-duplicable—two type attributes, zero special rules, fully automated by compiler.
> **Performance guarantee**: Move is zero-overhead, tokens are zero-overhead (zero-sized types, disappear after compilation), ref pays on demand, no GC pauses.

## Motivation

### Why do we need an ownership model?

| Language | Memory Management | Problems |
|----------|-------------------|----------|
| C/C++ | Manual management | Memory leaks, dangling pointers, double free |
| Java/Python | GC | Latency jitter, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow Checker | Lifetimes `'a` steep learning curve |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p cannot be read again

# 2. &T / &mut T Borrow tokens (zero-overhead, type attributes naturally derive permissions)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = Shared (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope ownership
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = Explicit copy
backup = p2.clone()            # Deep copy, exclusive

# 5. unsafe + *T = System-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature | Rust | YaoXiang |
|---------|------|----------|
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value semantics, zero-copy)** |
| Borrowing | `&T`/`&mut T`, can be returned, requires lifetimes | **`&T`/`&mut T` zero-sized tokens, Dup/Linear type attributes naturally derive** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-selects Rc/Arc)** |
| Copying | `clone()` | `clone()` |
| Raw pointers | `*T` | `*T` |
| Lifetimes | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker flow-sensitive liveness analysis (token state tracking)** |
| Circular references | Manual Weak | **Task boundary unified release / cross-task lint / standard library Weak** |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: Assignment / argument passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p cannot be read again

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p rebinds, type must be consistent

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
- After move, original binding is unreadable (compilation error)
- RAII: automatic deallocation at scope end
- Function signature `(T) -> T` is self-documenting—consumes T, returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not "references" but "type-level proof of access permissions".**

#### 2.1 Two Type Attributes

```
&T      →  Zero-sized, duplicable (Dup), grants read-only permission
&mut T  →  Zero-sized, linear (non-Dup), grants exclusive read-write permission
```

**This is not a "rule" to memorize—it's a fundamental property of the type system.** Dup types can be freely copied (multiple `&T` coexist), linear types cannot be copied (`&mut T` is inherently unique). There is no "borrow checker"—only a type checker doing what it has always done.

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

# Call site: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, previous token released after shift call

# Free functions work the same way
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist—Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escaping" Is Unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T`—can only be parameters, cannot be returned, cannot be stored in structs. This is patching the "borrow" concept.

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scoping rules as all other types.

**Returning references—naturally supported**:

```yaoxiang
# ✅ Tokens propagate with return values
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Child token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Stored in structs—naturally supported**:

```yaoxiang
# ✅ Structs carry tokens as fields
Window: Type = {
    target: Point,
    view: &Point,      # Token field—holds read-only view of target
}

# view token is derived from target, Window owns both
# As long as Window exists, view token is valid
```

**Closure capture—naturally supported**:

```yaoxiang
# ✅ Closures capture tokens, just like capturing any value
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    # Closure captures threshold's &Float token (Dup type, freely copied into closure)
    items.filter(|p| p.x > threshold)
}

# This is something RFC-009 v8 cannot do—v8 prohibits closure capture of borrows
```

**Cross-task—tokens cannot cross task boundaries**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compilation error: tokens cannot be passed cross-task
}

# This is not a special rule—tokens are compile-time permission proofs
# For cross-task sharing, use ref
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compilation error: &T is not ownable
}
```

#### 2.4 Token Lifetime

Token lifetime is governed by **ordinary scoping rules**, no lifetime parameters needed:

- Tokens in function parameters: live during call, released after call returns
- Returned tokens: ownership transfers to caller
- Tokens stored in structs: live with the struct
- Tokens captured by closures: live with the closure

The compiler doesn't need `'a` annotations because tokens are **values**, and value lifetimes are managed uniformly by the ownership system (Move/RAII). **Dimensionality reduction: borrowing becomes an ownership problem.**

#### 2.5 Freeze Mechanism

`&mut T` tokens can be temporarily "frozen" to produce `&T` tokens:

```yaoxiang
modify_and_read: (p: &mut Point) -> Void = {
    p.x = 10.0                      # Use &mut Point to modify
    
    # Freeze &mut, get read-only view
    view: &Point = freeze(p)         # p is frozen here
    print(view.x)                   # Read through &Point
    print(view.y)
    # view exits scope, freeze released
    
    p.y = 20.0                      # &mut Point restored
}
```

`freeze` semantics:
- Accepts `&mut T`, returns `&T`
- During `&T` lifetime, original `&mut T` is unavailable
- After `&T` exits scope, `&mut T` automatically restored
- This is **flow-sensitive analysis**—compiler tracks token state within function body

#### 2.6 Token Conflict Detection

Replaces RFC-009 v8's "aliasing borrow check". The principle is simpler—**flow-sensitive liveness analysis** on token values:

```yaoxiang
# ❌ &mut and derived &T cannot be simultaneously active
bad_alias: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)        # p is frozen
    p.x = 10.0                      # ❌ Compilation error: WriteToken in frozen state
    print(view.x)                   
}

# ✅ After freeze released, &mut can be used again
good_seq: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)        # p is frozen
    print(view.x)                   # Use &T
    # view exits scope
    p.x = 10.0                      # ✅ WriteToken restored
}
```

**Detection method**: This is not a dedicated "borrow checker"—it's **flow-sensitive liveness analysis** on token values. The compiler tracks each token's state (active/frozen/moved) within function bodies, exactly the same way it tracks any linear type value.

#### 2.7 Compiler Internals: Branding Mechanism

Users never encounter brands. The compiler internally assigns each token a compile-time unique identifier:

```
What users see         Compiler internal representation
────────────────────────────────────────
&Point              →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point          →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand uses:
- **Forgery prevention**: Tokens can only be obtained from owner capsules or freeze operations, cannot be constructed from thin air
- **Association tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries derived brand (`#N.field_x`), compiler can trace to parent token
- **Conflict detection**: Same-source `WriteToken` and derived `ReadToken` cannot be simultaneously active

Brands completely disappear after monomorphization and inlining; they do not exist in generated machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

Compiler auto-selects at call site based on priority:

```
1. If actual argument will be used later → prefer creating token (&T or &mut T, per method signature)
2. If actual argument won't be used later → Move
3. Preference order: &T < &mut T < Move
```

```yaoxiang
# Example: Auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Simplified Borrowing

| Feature | Simplified Borrow (v8) | Borrow Token (v9) |
|---------|----------------------|-------------------|
| Return references | ❌ Hardcoded prohibition | ✅ Tokens propagate with return values |
| Store in structs | ❌ Hardcoded prohibition | ✅ Tokens as struct fields |
| Closure capture | ❌ Hardcoded prohibition | ✅ Closures capture token values |
| Special rules | 3 (only parameters/cannot return/cannot store) | 0—type attributes naturally derive |
| Borrow checking | Dedicated aliasing check | Type checker flow-sensitive liveness analysis |
| Lifetime annotations | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-sized types, disappear after compilation) |
| Error messages | "Borrow cannot escape" | "WriteToken(#3) has been frozen" (regular type error) |
| User mental model | Understand "borrowing" special status | `&T` is duplicable, `&mut T` is non-duplicable |

---

### 3. ref Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether it's backed by Rc or Arc is an implementation detail users don't need to care about.

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
    use(data)                     # Compiler: non-cross-task → Rc
}
```

**User mental model**: `ref` = shared ownership. That's it.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks         → Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
In-task cycles → Silently allowed.
  ├── Structured concurrency guarantees unified release of all resources at task end.
  ├── ref always keeps alive, semantics are not diluted.
  └── Users have the right to build bidirectional strong references within tasks (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, won't actually leak (parent task end releases all child task resources).
  ├── But cross-task strong references imply ownership boundary blurring, worth pausing to reconsider.
  ├── Default warn level, compilation passes but with hints.
  └── Teams can set to deny in project config, gate in CI quality checks.
```

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | No checking | Personal projects |
| `warn` (default) | Compilation passes, with hints | Development stage |
| `deny` | Compilation fails | Team CI quality gate |
| `forbid` | Compilation fails, cannot override | Organization-level hard rule |

```yaoxiang
# In-task cycle: silently allowed, bidirectional strong references
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
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task circular reference
    }
}
```

**Project config example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles rejected in CI
```

| Cycle type | Behavior | Reason |
|------------|----------|--------|
| In-task ref cycle | No checking | User's prerogative, unified release at task end |
| Cross-task ref cycle | lint (default warn) | Reminder to reconsider, can be set to deny |

#### 3.4 Weak: Standard Library Provided

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not language-built-in, it's a standard library type.** Daily use `ref` is sufficient. Advanced users needing fine-grained memory control manually introduce `Weak`.

#### 3.5 Borrow Token vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Glance/modify in place | Shared ownership |
| Scope | Follows token value's scope | Cross-scope |
| Cost | Zero overhead (zero-sized type) | Rc or Arc (compiler-selected) |
| Escape | Can (tokens propagate with returns/structs/closures) | Designed for escaping |
| Cross-task | Cannot (tokens are compile-time permission proofs, cannot cross task boundaries) | Can (compiler auto-selects Arc) |
| Cycles | Not applicable | In-task silently allowed, cross-task lint |

---

### 4. clone() — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, no side effects
```

**When to use**: When you need to retain the original value and Move or sharing aren't suitable.

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
- Used for FFI, memory operations, etc. system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow tokens (zero overhead)    Move (zero overhead)    Shared (pay on demand)    Copy
         │                              │                    │                    │
   &T duplicable token              Default ownership     ref Rc/Arc            clone()
   &mut T linear token             transfer              compiler auto         explicit
         │                              │                    │                    │
   Token value scope                Scope-local          Cross-scope          Anytime
   Can return/store in structs      T -> T return         ref cross-task → Arc   independent
   Can be captured by closures       T -> Void consume     ref non-cross-task → Rc  copies
   Zero-sized, disappear after                            In-task cycles silently
     compilation                                           Cross-task cycles lint
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

    # Move → Move: consume and return
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # Take, modify, return to you
    }

    # Return references: tokens propagate with return values
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
p = p.scale(2.0)                    # Move → return
shared = ref p                      # ref shared
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# In-task cycle: silently allowed
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

### Dup Type Attribute

`Dup` (Duplicable, can be implicitly copied) is a key type attribute auto-managed by the compiler. Types implementing `Dup` don't transfer ownership on assignment; instead, they implicitly shallow-copy. `Dup` implies `Clone`—if a type is `Dup`, it can always be field-by-field cloned; but `Clone` does not imply `Dup`.

| Type | Dup | Clone | Description |
|------|-----|-------|-------------|
| Int, Float, Bool, Char, String, Bytes | ✅ (auto-implemented by compiler) | ✅ | Primitive types |
| `&T` | ✅ (zero-sized, freely aliasable) | ✅ | Read-only token |
| `&mut T` | ❌ (linear, exclusive) | ❌ | Mutable token |
| struct | ✅ (auto-derived when all fields are Dup) | ✅ | Struct |
| `ref T` | ✅ | ✅ | Shared ownership (compiler auto-selects Rc/Arc) |
| `*T` | ❌ | ❌ | Raw pointer |

> **Note**: YaoXiang has no user-visible `Send`/`Sync` trait. The `ref` keyword has the compiler auto-select Rc (non-cross-task) or Arc (cross-task); users don't need to care about whether types can cross task boundaries. Tokens cannot cross task boundaries is a compiler builtin rule, not a trait constraint.

---

## Performance Analysis

| Operation | Cost | Description |
|-----------|------|-------------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-sized type, disappears after compilation, zero runtime overhead |
| `ref` (non-cross-task) | Low | Compiled to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiled to Arc, atomic operation |
| `clone()` | Varies | Fast for small objects, slow for large objects |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|----------|-------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + Borrow checker | Compile-time checked | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + Borrow tokens** | **RAII** | **Task boundary release / cross-task lint / standard library Weak** | **Low** |

---

## Tradeoffs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, no global borrow checker. `&T` is duplicable, `&mut T` is non-duplicable—two type attributes
3. **Powerful**: Can return references, store in structs, closure capture—expressiveness on par with Rust
4. **Compiler intelligent**: ref auto-selects Rc/Arc, call site auto-selects borrow
5. **Deterministic**: ref always keeps alive, won't silently become weak reference
6. **High performance**: Move zero-copy, tokens zero-overhead (zero-sized types, disappear after compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter propagation**: Tokens carry brand identifiers, function signatures returning references include additional generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the inevitable cost of sharing)
3. **unsafe risk**: Users must guarantee correctness
4. **Cross-task cycles are lint not compilation error**: Unlike Rust's compilation failure, default warn, requires team configuration deny for quality gate

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| GC | Runtime overhead, unpredictable pauses |
| Rust borrow checker | Requires lifetimes `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointers | Cannot do system-level programming |
| Expose Rc/Arc to users | Implementation details dumped on users, increased cognitive burden |
| Simplified borrowing (v8) | No-escaping strategy sacrificed critical capabilities like closure capture, returning references, etc. |

---

## Design Decision Records

| Decision | Resolution | Reason | Date |
|----------|------------|--------|------|
| **Default** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimizes | User simplicity, compiler responsibility | 2025-01-15 |
| **Borrowing** | `&T`/`&mut T` as zero-sized token types | Type attributes (Dup/Linear) naturally derive permissions, unified type system | 2025-01-15 |
| **Borrow tokens** | Replace simplified borrowing, `&T` Dup, `&mut T` Linear | Eliminate special rules like "no escaping", support closure capture/return references/store in structs | 2026-05-29 |
| **Copying** | `clone()` | Explicit semantics | 2025-01-15 |
| **System-level** | `*T` + `unsafe` | Support system programming | 2025-01-15 |
| **Lifetimes** | Not implemented | Tokens are values, lifetimes managed uniformly by Move/RAII, dimensionality reduction: borrowing becomes ownership | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, not user-visible | Reduced cognitive burden | 2025-01-15 |
| **Circular references** | In-task no checking, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint configurable deny | 2025-01-16 |
| **Weak** | Standard library provided | Advanced users explicitly choose | 2025-01-16 |
| **Consumption analysis** | Removed | Mini borrow checker, unnecessary | 2026-05-11 |
| **Ownership return flow** | Removed | `(T) -> T` signature is self-documenting | 2026-05-11 |
| **Empty state reuse** | Removed (as feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function/partial consume/field-level three-tier mutability** | Removed | Over-engineering | 2026-05-11 |

### Version History

| Version | Major Changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| v4 | Default Move + explicit ref | 2025-01-15 |
| v5 | Structured concurrency + circular reference handling | 2025-01-16 |
| v6 | Added empty state reuse, ownership return flow | 2025-02-04 |
| v7 | Added consumption analysis, inverse functions, field-level mutability | 2025-02-05 |
| **v8** | **Removed over-engineering, added simplified borrowing &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces simplified borrowing, unified type system** | **2026-05-29** |

### Pending Issues

| Topic | Description | Status |
|-------|-------------|--------|
| Drop syntax | Whether explicit `drop()` function is needed | Pending discussion |
| Escape analysis algorithm | ref cross-task detection implementation | Pending discussion |
| Token conflict detection | Flow-sensitive liveness analysis, see below | ✅ Resolved |

### Token Conflict Detection: Flow-Sensitive Liveness Analysis

**Analysis scope**: Function body only. Flow-sensitive liveness analysis on token values, tracking each token's state (active/frozen/moved).

**Layer 1: Call site check**—same actual argument cannot simultaneously create `&mut` token and other tokens:

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p simultaneously derives &mut and & tokens, compiler rejects
```

**Layer 2: Function body flow-sensitive**—after `&mut` token passed to call, token released after call returns, can create token again:

```yaoxiang
process_twice: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)    # &mut token passed to shift, token released after shift returns
    print_info(p)          # Recreate &Point token, no conflict
}
```

**Layer 3: Frozen state tracking**—during `&T` token lifetime from `freeze`, original `&mut` token is unavailable:

```yaoxiang
frozen: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)    # p enters frozen state
    print(view.x)
    p.x = 10.0                  # ❌ Compilation error: WriteToken in frozen state
}
```

**What's not needed**: Cross-function lifetime tracking, global alias analysis, borrow graph constraint solving, NLL, `'a` annotations. Because tokens are values, value liveness analysis is handled uniformly by the type checker—exactly the same as tracking any linear type value.

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