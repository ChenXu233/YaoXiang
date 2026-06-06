```markdown
---
title: "RFC-009: Ownership Model Design"
status: "Accepted"
author: "Chen Xu"
created: "2025-01-08"
updated: "2026-05-29 (Borrow token system replaces the bare-minimum borrowing, unifying the type system)"
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **ownership model** for the YaoXiang programming language.

**Core design—five concepts, one gradient**:

```
Glance/Modify-in-place    Take away         Shared hold         Clone a copy         Systems-level
      │                      │                  │                   │                 │
     &T                    Move            ref                clone()             unsafe
   &mut T               Zero-copy        Compiler auto       Explicit          *T
   Zero-size token      Default           picks Rc/Arc       deep copy         Users responsible
   Type properties                               
   naturally                                          
   infer                                          
   permissions                                          
```

- **Move (default)**: Assignment/argument passing/return = ownership transfer, zero-copy, RAII automatic deallocation
- **`&T` / `&mut T` (borrow tokens)**: Zero-size compile-time token types. `&T` is duplicable (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are naturally inferred from type properties, no special rules needed. Can be returned, stored in structs, and captured by closures.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically picks Rc (non-cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, systems-level escape hatch

**Eliminated complexity**:
- ❌ No lifetime `'a`
- ❌ No borrow checker (token type properties—Dup/Linear—replace the dedicated borrow checker)
- ❌ No GC
- ❌ No special rules like "no escaping" (tokens are ordinary types, scopes are handled uniformly by the type system)
- ❌ Users don't need to know the difference between Rc and Arc (compiler auto-selects)

> **Programming burden**: `&T` is duplicable, `&mut T` is not—two type properties, zero special rules, fully automated by compiler.
> **Performance guarantee**: Move is zero-overhead, tokens are zero-overhead (zero-size types, disappear after compilation), ref is pay-for-use, no GC pauses.

## Motivation

### Why does YaoXiang need an ownership model?

| Language | Memory Management | Problems |
|----------|-------------------|----------|
| C/C++ | Manual | Memory leaks, dangling pointers, double frees |
| Java/Python | GC | Latency jitter, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow checker | Lifetime `'a` steep learning curve |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p cannot be read again

# 2. &T / &mut T borrow tokens (zero-overhead, type properties naturally infer permissions)
print_info(p2)                 # Compiler automatically creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler automatically creates &mut Point token

# 3. ref = shared (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope hold
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, unique

# 5. unsafe + *T = systems-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature | Rust | YaoXiang |
|---------|------|----------|
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value semantics, zero-copy)** |
| Borrowing | `&T`/`&mut T`, returnable, requires lifetimes | **`&T`/`&mut T` zero-size tokens, Dup/Linear type properties naturally infer** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-selects Rc/Arc)** |
| Copying | `clone()` | `clone()` |
| Raw pointers | `*T` | `*T` |
| Lifetimes | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker flow-sensitive liveness analysis (token state tracking)** |
| Cyclic references | Manual Weak | **Task-end unified release / cross-task lint / std library Weak** |

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
- Zero-copy (compiler moves the pointer)
- After moving, the original binding is unreadable (compile error)
- RAII: automatic deallocation at scope end
- Function signature `(T) -> T` is self-documenting—consumes T, returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-size compile-time token types. They are not "references" but "type-level proofs of access permission".**

#### 2.1 Two Type Properties

```
&T      →  Zero-size, duplicable (Dup), grants read-only permission
&mut T  →  Zero-size, linear (non-Dup), grants exclusive read-write permission
```

**This is not a "rule" to memorize—it is a fundamental property of the type system.** Dup types can be freely duplicated (multiple `&T` coexist), linear types cannot be duplicated (`&mut T` is inherently unique). There is no "borrow checker"—only a type checker doing what it has always done.

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

# Call side: compiler auto-selects borrow or Move
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

#### 2.3 Why "No-Escape" Is Unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T`—can only be parameters, cannot be returned, cannot be stored in structs. This was patching the "borrowing" concept.

The token system needs none of these rules. Tokens are **ordinary types**, following the same scope rules as all other types.

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

**Stored in structs—naturally supported**:

```yaoxiang
# ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,      # Token field—holds read-only view of target
}

# view token is derived from target, Window owns both
# As long as Window exists, view token is valid
```

**Closure capture—naturally supported**:

```yaoxiang
# ✅ Closure captures token, just like capturing any value
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    # Closure captures threshold's &Float token (Dup type, freely copied into closure)
    items.filter(|p| p.x > threshold)
}

# This is something RFC-009 v8 cannot do—v8 forbids closure capture of borrows
```

**Cross-task—tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task boundary
}

# This is not a special rule—token is a compile-time permission proof, use ref for cross-task sharing
# If you need cross-task sharing, use ref
```

**Token cannot be ref'd**:

```yaoxiang
# ❌ Token is a permission proof, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not ownable
}
```

#### 2.4 Token Lifetime

Token lifetime is governed by **ordinary scope rules**, no lifetime parameters needed:

- Token in function parameter: lives during call, released after call returns
- Token being returned: ownership transferred to caller
- Token stored in struct: lives with the struct
- Token captured by closure: lives with the closure

The compiler doesn't need `'a` annotations because tokens are **values**, and value lifetimes are uniformly managed by the ownership system (Move/RAII). **Reduces borrowing to an ownership problem.**

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
- During `&T` lifetime, the original `&mut T` is unavailable
- After `&T` leaves scope, `&mut T` automatically restores
- This is **flow-sensitive analysis**—compiler tracks token state within function body

#### 2.6 Token Conflict Detection

Replaces RFC-009 v8's "cross-borrow checking". The principle is simpler—**flow-sensitive liveness analysis** on token values:

```yaoxiang
# ❌ &mut and derived &T cannot both be live
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

**Detection approach**: This is not a dedicated "borrow checker"—this is **flow-sensitive liveness analysis** on token values. The compiler tracks each token's state (live/frozen/moved) within the function body, exactly the same way it tracks any linear type value.

#### 2.7 Compiler Internals: Branding Mechanism

Users never see brands. The compiler internally assigns a compile-time unique identifier to each token:

```
What user sees         Compiler internal representation
────────────────────────────────────────
&Point              →  ReadToken(Point, #N)    // #N is compile-time unique integer
&mut Point          →  WriteToken(Point, #M)   // #M is compile-time unique integer
```

Brand uses:
- **Anti-forgery**: Tokens can only be obtained from owner capsule or freeze operation, cannot be fabricated from thin air
- **Provenance tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries derived brand (`#N.field_x`), compiler can trace back to parent token
- **Conflict detection**: Same-origin `WriteToken` and derived `ReadToken` cannot both be live

Brands disappear completely after monomorphization and inlining; they do not exist in generated machine code. **Zero runtime overhead.**

#### 2.8 Auto-Borrow Selection Rules

Compiler at call site auto-selects by this priority:

```
1. If actual argument is used later → prefer creating token (&T or &mut T, based on method signature)
2. If actual argument is not used later → Move
3. Prefer matching order: &T < &mut T < Move
```

```yaoxiang
# Example: auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Bare-Minimum Borrowing

| Feature | Bare-Minimum Borrowing (v8) | Borrow Tokens (v9) |
|---------|----------------------------|--------------------|
| Return reference | ❌ Hardcoded forbid | ✅ Token propagates with return |
| Store in struct | ❌ Hardcoded forbid | ✅ Token as struct field |
| Closure capture | ❌ Hardcoded forbid | ✅ Closure captures token value |
| Special rules | 3 (param-only/no-return/no-store) | 0—type properties naturally infer |
| Borrow checking | Dedicated cross-borrow check | Type checker flow-sensitive liveness analysis |
| Lifetime annotations | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-size types, disappear after compilation) |
| Error messages | "Borrow cannot escape" | "WriteToken(#3) is frozen" (regular type error) |
| User mental model | Understand "borrowing" as special concept | `&T` is duplicable, `&mut T` is not |

---

### 3. ref Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether the underlying is Rc or Arc is irrelevant to the user.

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

**User mental model**: `ref` = shared hold. That's it.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref dataflow analysis:

Does not escape to other tasks → Rc (non-atomic reference count, low overhead)
Escapes to other tasks         → Arc (atomic reference count, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycle → silently allowed.
  ├── Structured concurrency guarantees unified release of all resources when task ends.
  ├── ref always keeps alive, semantics uncompromised.
  └── User has right to build bidirectional strong references (e.g., graph computation intermediate state).

Cross-task cycle → lint (default warn, configurable).
  ├── Program behavior is correct, no true leak will occur (parent task end releases all child resources).
  ├── But cross-task strong references mean ownership boundary is blurred, worth pausing to reconsider.
  ├── Default warn level, compilation passes with hint.
  └── Team can set to deny in project config,纳入 CI 质量门.

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | No check | Personal project |
| `warn` (default) | Compile passes, with hint | Development phase |
| `deny` | Compile fails | Team CI quality gate |
| `forbid` | Compile fails, cannot override | Organization-level mandatory rule |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong reference
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
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task cycle
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
| Intra-task ref cycle | No check | User's right, unified release at task end |
| Cross-task ref cycle | lint (default warn) | Reminder to reconsider, configurable deny |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not language-built-in; it is a standard library type.** Daily use `ref` is sufficient. Users who need fine-grained memory control manually introduce `Weak`.

#### 3.5 Borrow Tokens vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| Purpose | Glance/modify-in-place | Shared hold |
| Scope | Follows token value's scope | Cross-scope |
| Cost | Zero overhead (zero-size type) | Rc or Arc (compiler selects) |
| Escape | Yes (token propagates with return/struct/closure) | Originally designed to escape |
| Cross-task | No (token is compile-time permission proof, cannot cross task boundary) | Yes (compiler auto-selects Arc) |
| Cycles | Not applicable | Intra-task silently allowed, cross-task lint |

---

### 4. clone() —— Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, no mutual effect
```

**When to use**: When you need to keep the original value and borrowing or sharing are not suitable.

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
- Only usable within `unsafe` block
- User guarantees no dangling, no use-after-free
- Used for FFI, memory operations, etc.

---

### 6. Ownership Gradient Overview

```
  Borrow tokens (zero overhead)     Move (zero overhead)     Shared (pay-for-use)     Copy
   │                              │                      │                    │
  &T duplicable token           Default ownership       ref Rc/Arc          clone()
  &mut T linear token           transfer               Compiler auto       Explicit
   │                              │                    selects              deep copy
   │                              │                      │                    │
  Token value scope              Within scope           Cross-scope         Anytime
  Can return/store in struct     T -> T flowback        ref cross-task → Arc  Independent
  Can be captured by closure     T -> Void consume      ref non-cross-task → Rc   copy
  Zero-size disappears after                            Intra-task cycle silent
  compilation                                           Cross-task cycle lint
                                                        Std lib Weak escape
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

    # Move → Move: consume and flow back
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

# Closure capture token (capability v8 doesn't have)
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

# Intra-task cycle: silently allowed
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

`Dup` (Duplicable) is a compiler-managed type property meaning **shallow copy**: on assignment/argument passing, what is copied is the handle/token, with underlying data shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creates independent copy).

**Dup and Clone are orthogonal concepts**—Dup copies handles sharing data, Clone creates independent copies. A type can support both Dup and Clone, or only one.

| Type | Dup | Clone | Description |
|------|-----|-------|-------------|
| `&T` | ✅ (copy token, multiple views point to same data) | ✅ | Read-only token |
| `ref T` | ✅ (refcount +1, shared heap data) | ✅ | Shared hold (compiler auto-selects Rc/Arc) |
| String, Bytes | ✅ (internal refcount, copy handle shares underlying buffer) | ✅ | String/bytes |
| `&mut T` | ❌ (linear, exclusive) | ❌ | Mutable token |
| `*T` | ❌ | ❌ | Raw pointer |
| struct | Derived (auto-derived when all fields are Dup) | ✅ | Struct |

**Primitive value types** (Int, Float, Bool, Char) have compiler-built-in value-copy semantics on assignment—both values are completely independent, not shallow copy. They are not part of the Dup type property but are natively handled by the compiler.

---

## Performance Analysis

| Operation | Cost | Description |
|-----------|------|-------------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-size type, disappears after compilation, zero runtime overhead |
| `ref` (non-cross-task)| Low | Compiled to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiled to Arc, atomic operation |
| `clone()` | Depends on type | Fast for small objects, slow for large |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|----------|-------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + borrow checker | Compile-time check | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + borrow tokens** | **RAII** | **Task boundary release / cross-task lint / std lib Weak** | **Low** |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, no global borrow checker. `&T` is duplicable, `&mut T` is not—two type properties
3. **Powerful**: Can return references, store in structs, capture in closures—expressiveness on par with Rust
4. **Compiler intelligent**: ref auto-selects Rc/Arc, call site auto-selects borrowing
5. **Deterministic**: ref keeps alive, never silently becomes weak reference
6. **High-performance**: Move zero-copy, tokens zero-overhead (zero-size types, disappear after compilation)
7. **Flexible**: `unsafe + *T` supports systems-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, function signatures returning references will reflect extra generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the inevitable cost of sharing)
3. **unsafe risk**: Users must guarantee correctness
4. **Cross-task cycles are lint not compile error**: Unlike Rust's compile error, default warn, requires team config deny for quality gate

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| GC | Runtime overhead, unpredictable pauses |
| Rust borrow checker | Requires lifetime `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointers | Cannot do systems-level programming |
| Expose Rc/Arc to users | Exposing implementation details to users, increases cognitive burden |
| Bare-minimum borrowing (v8) | "No escape" strategy sacrificed key expressive capabilities like closure capture, returning references |

---

## Design Decision Record

| Decision | Determination | Reason | Date |
|----------|---------------|--------|------|
| **Default** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimizes | Simple for users, compiler responsible | 2025-01-15 |
| **Borrowing** | `&T`/`&mut T` as zero-size token types | Type properties (Dup/Linear) naturally infer permissions, unified type system | 2025-01-15 |
| **Borrow tokens** | Replace bare-minimum borrowing, `&T` Dup, `&mut T` Linear | Eliminate special rules like "no escape", support closure capture/return reference/store in struct | 2026-05-29 |
| **Copying** | `clone()` | Explicit semantics | 2025-01-15 |
| **Systems-level** | `*T` + `unsafe` | Support systems programming | 2025-01-15 |
| **Lifetimes** | Not implemented | Tokens are values, lifetimes managed uniformly by Move/RAII, reduces borrowing to ownership problem | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, invisible to users | Reduce cognitive burden | 2025-01-15 |
| **Cyclic references** | Intra-task no check, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint configurable deny | 2025-01-16 |
| **Weak** | Provided by standard library | Advanced users explicitly choose | 2025-01-16 |
| **Consume analysis** | Removed | Mini borrow checker, not needed | 2026-05-11 |
| **Ownership flowback** | Removed | `(T) -> T` signature is self-documenting | 2026-05-11 |
| **Empty state reuse** | Removed (as feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function/partial consume/three-tier mutability** | Removed | Over-engineering | 2026-05-11 |

### Version History

| Version | Major Changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| v4 | Default Move + explicit ref | 2025-01-15 |
| v5 | Structured concurrency + cycle handling | 2025-01-16 |
| v6 | Added empty state reuse, ownership flowback | 2025-02-04 |
| v7 | Added consume analysis, inverse functions, field-level mutability | 2025-02-05 |
| **v8** | **Removed over-engineering, added bare-minimum borrowing &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces bare-minimum borrowing, unified type system** | **2026-05-29** |

### Open Issues

| Issue | Description | Status |
|-------|-------------|--------|
| Drop syntax | Whether explicit `drop()` function needed | Pending discussion |
| Escape analysis algorithm | ref cross-task detection implementation | Pending discussion |
| Token conflict detection | Flow-sensitive liveness analysis, see below | ✅ Resolved |

### Token Conflict Detection: Flow-Sensitive Liveness Analysis

**Analysis scope**: Function body only. Flow-sensitive liveness analysis on token values, tracking each token's state (live/frozen/moved).

**Layer 1: Call site check**—same actual argument cannot simultaneously create `&mut` token and other tokens:

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p simultaneously derives &mut and & tokens, compiler rejects
```

**Layer 2: Function body flow-sensitivity**—after `&mut` token passed to call, token released on return, can create token again:

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
    p.x = 10.0                  # ❌ Compile error: WriteToken in frozen state
}
```

**What is NOT needed**: Cross-function lifetime tracking, global alias analysis, borrow graph constraint solving, NLL, `'a` annotations. Because tokens are values, value liveness analysis is handled uniformly by the type checker—exactly the same way it tracks any linear type value.

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