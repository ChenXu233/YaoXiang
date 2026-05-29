---
title: RFC-009: Ownership Model Design
---

# RFC-009: Ownership Model Design

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-08
> **Last Updated**: 2026-05-11 (Added minimalist borrow &T/&mut T, completed ownership gradient)

## Abstract

This document defines the **ownership model** for the YaoXiang programming language.

**Core Design—Five Concepts, One Gradient**:

```
glance/modify-in-place    take         shared hold        clone it         system-level
     │                    │               │                │                │
    &T                   Move           ref             clone()          unsafe
   &mut T                zero-copy     compiler auto    explicit deep    *T
  function params        default       selects Rc/Arc   copy
  no escaping                         silent within task
                                      cross-task lint
                                      stdlib Weak
```

- **Move (default)**: Assignment/parameter passing/return = ownership transfer, zero-copy, RAII auto-release
- **`&T` / `&mut T` (minimalist borrow)**: For function parameters only, no escaping. Zero annotations, zero lifetimes. Compiler auto-borrows at call site
- **`ref` keyword**: Cross-scope sharing. Compiler auto-selects Rc (not cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointers, system-level escape hatch

**Eliminated complexity**:
- ❌ No lifetimes `'a`
- ❌ No borrow checker (prohibition rather than annotation, no need for Rust-style global lifetime inference)
- ❌ No GC
- ❌ No consumption analysis/ownership return etc. "mini borrow checker"
- ❌ Users don't need to know the difference between Rc/Arc (compiler auto-selects)

> **Programming burden**: `&T`/`&mut T` three rules, `ref` one keyword, fully automatic by compiler.
> **Performance guarantee**: Move zero overhead, borrow zero overhead, ref pay-as-you-go, no GC pauses.

## Motivation

### Why an ownership model?

| Language | Memory Management | Issues |
|----------|-------------------|--------|
| C/C++ | manual | memory leaks, dangling pointers, double-free |
| Java/Python | GC | latency jitter, memory overhead, unpredictable pauses |
| Rust | ownership + borrow checker | lifetimes `'a` steep learning curve |
| **YaoXiang** | **Move + Borrow + ref** | **simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p cannot be read again

# 2. &T / &mut T borrow (zero overhead, read-only/modify-in-place, no escaping)
print_info(p2)                 # Compiler auto-borrows &p2, returned after use
shift(p2, 1.0, 1.0)           # Compiler auto-borrows &mut p2

# 3. ref = shared (compiler auto-selects Rc/Arc)
shared = ref p2                # cross-scope hold
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # deep copy, exclusive

# 5. unsafe + *T = system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature | Rust | YaoXiang |
|---------|------|----------|
| Default semantics | borrow `&T` (needs explicit `.clone()`) | **Move (value semantics, zero-copy)** |
| Borrow | `&T`/`&mut T`, can return, needs lifetimes | **`&T`/`&mut T` only for params, no escaping** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-selects Rc/Arc)** |
| Copy | `clone()` | `clone()` |
| Raw pointers | `*T` | `*T` |
| Lifetimes | `'a` | ❌ None |
| Borrow checking | global inference | **only within function body scope** |
| Cyclic references | manual Weak | **task-end unified release / cross-task lint / stdlib Weak** |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p cannot be read again

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p rebinds, type must remain consistent

# Function parameter: Move
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
- After move, original binding is unreadable (compile error)
- RAII: auto-release when scope ends
- Function signature `(T) -> T` is itself documentation—consumes T, returns T

---

### 2. &T / &mut T (Minimalist Borrow)

**Core principle: borrow is just "taking a glance" or "modifying in place", never taking ownership.**

#### 2.1 Three Rules

```
1. &T / &mut T can only appear as function parameters
2. Cannot return, cannot be stored in structs, cannot be assigned to local variables, cannot escape via closures
3. Call site doesn't need & annotation, compiler auto-selects borrow or Move based on method signature
```

**Zero annotations. Zero lifetimes.** The compiler only does one thing: ensures the borrow doesn't leave the current function. This requires no cross-function analysis—because "prohibited from escaping" blocks it, no inference needed.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare self type, determine borrow mode
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # read field
    print(self.y)
    # function ends, borrow ends
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # modify in place
    self.y = self.y + dy
}

# Call side: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler: &p, borrow auto-releases after print ends
p.shift(1.0, 1.0)                  # Compiler: &mut p, borrow auto-releases after shift ends
p.print()                          # OK, p is still valid

# Free functions work the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # read two params
}
d = distance(p, p2)                     # Compiler: &p, &p2
```

#### 2.3 Forbidden Behaviors

```yaoxiang
# ❌ Forbidden: borrow escaping function
get_x_ref: (p: &Point) -> &Float = { p.x }      # return borrow → compile error
store_ref: (p: &Point) -> Wrapper = {             # store in struct → compile error
    Wrapper { ref: p }
}

# ❌ Forbidden: assign borrow to local variable
bad_bind: (p: &Point) -> Void = {
    q = p                         # &Point assigned to local variable → compile error
}

# ❌ Forbidden: crossing task boundary
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # borrow escaping to task → compile error
}

# ❌ Forbidden: cannot Move borrowed value
bad_move: (p: &Point) -> Void = {
    p2 = p                        # not yours, no right to transfer → compile error
}

# ❌ Forbidden: cannot persistently hold borrowed value
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # borrow is not ownership, cannot ref → compile error
}
```

#### 2.4 &mut Aliasing Protection

```yaoxiang
# ✅ Allowed: multiple &T simultaneously active
read_both: (a: &Point, b: &Point) -> Float = { a.x + b.y }

# ✅ Allowed: borrow after &mut ends
shift_and_read: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)           # &mut p's borrow is within this call
    print_info(p)                 # previous borrow ended, can borrow & again
}

# ❌ Forbidden: &mut and &T simultaneously active
# Compiler does flow-sensitive analysis within function body, ensuring only one &mut at a time
```

#### 2.5 Auto-Borrow Selection Rules

Compiler auto-selects at call site by this priority:

```
1. If actual argument is used later → prefer borrow (&T or &mut T, based on method signature)
2. If actual argument is not used later → Move
3. Preference order: &T < &mut T < Move
```

```yaoxiang
# Example: auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → &p, p still usable after borrow ends
p.shift(1.0, 1.0) # shift declares &mut self → &mut p
p2 = p           # Move, p no longer used
```

---

### 3. ref Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether it's Rc or Arc underneath, users don't need to care.

#### 3.1 Basic Usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # shared, compiler auto-selects implementation

# cross-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }           # Compiler: cross-task → Arc
    spawn { use(data) }           # Compiler: cross-task → Arc
}

# single-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    use(data)                     # Compiler: not cross-task → Rc
}
```

**User mental model**: `ref` = shared hold. That's it.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref dataflow analysis:

Does not escape to other tasks → Rc (non-atomic reference count, low overhead)
Escapes to other tasks   → Arc (atomic reference count, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycles → silently allowed.
  ├── Structured concurrency guarantees unified release of all resources when task ends.
  ├── ref always keeps alive, semantics are not diluted.
  └── Users have the right to build bidirectional strong references within tasks (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, no actual leaks (parent task end releases all child task resources).
  ├── But cross-task strong references mean ownership boundaries are blurred, worth pausing to reconsider.
  ├── Default warn level, compilation passes with hint.
  └── Teams can set to deny in project config, gated in CI quality checks.
```

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | no checking | personal projects |
| `warn` (default) | compile passes with hint | development phase |
| `deny` | compile fails | team CI quality gate |
| `forbid` | compile fails, cannot override | org-level enforced rules |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong reference
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # cycle. Unified release when task ends.
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
cross-task-cycle = "deny"    # cross-task cycles rejected in CI
```

| Cycle Type | Behavior | Reason |
|------------|----------|--------|
| Intra-task ref cycle | no checking | user's right, unified release at task end |
| Cross-task ref cycle | lint (default warn) | reminder to reconsider, configurable deny |

#### 3.4 Weak: Standard Library Provided

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # user explicitly controls which direction is weak
```

**`Weak` is not language built-in, it's a standard library type.** Daily use `ref` is enough. Advanced users who need fine-grained memory control manually import `Weak`.

#### 3.5 Borrow vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | glance/modify in place | shared hold |
| Scope | function parameters, returned after use | cross-scope |
| Cost | zero overhead | Rc or Arc (compiler selects) |
| Escaping | prohibited | designed to escape |
| Cycles | not applicable | silently allowed within task, lint cross-task |

---

### 4. clone() — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # deep copy
# p and p2 are independent, don't affect each other
```

**When to use**: when you need to keep the original and Move or share aren't suitable.

### 5. unsafe + Raw Pointers (System-Level Programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # raw pointer
    (*ptr).x = 0.0                # dereference (user guarantees safety)
    ptr2 = ptr + 1                # pointer arithmetic
}
```

**Restrictions**:
- can only be used in `unsafe` blocks
- user guarantees no dangling, no use-after-free
- for FFI, memory operations etc. system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow (zero overhead)       Move (zero overhead)     Share (pay-as-you-go)   Copy
   │                            │                        │                       │
  &T glance                  default ownership        ref Rc/Arc            clone()
  &mut T modify-in-place      transfer                compiler auto         explicit deep
   │                            │                        │                       │
  function params              within scope            cross-scope            any time
  no escaping                  T -> T return           ref cross-task → Arc   independent
  auto-select                  T -> Void consume       ref not cross-task → Rc
                                                     intra-task cycles silent
                                                     cross-task cycles lint
                                                     stdlib Weak escape
```

---

## Comprehensive Example

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,

    # &T: read-only
    print: (self: &Point) -> Void = {
        print(self.x)
        print(self.y)
    }

    # &mut T: modify in place
    shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
        self.x = self.x + dx
        self.y = self.y + dy
    }

    # Move → Move: consume and return
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # take, modify, give back
    }
}

# Comprehensive usage
p = Point(1.0, 2.0)
p.print()                           # &p, borrow
p.shift(1.0, 1.0)                   # &mut p, borrow
p = p.scale(2.0)                    # Move → return
shared = ref p                      # ref share
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# intra-task cycle: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # cycle, unified release when task ends

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
| value types | ✅ | ✅ | Int, Float, Point... |
| `ref T` | ✅ | ✅ | compiler auto-selects Rc/Arc |
| `&T` / `&mut T` | ❌ | ❌ | borrows cannot cross threads (not allowed to escape) |
| `*T` | ❌ | ❌ | raw pointer, single-threaded |

---

## Performance Analysis

| Operation | Cost | Description |
|-----------|------|-------------|
| Move | zero | pointer move |
| `&T` / `&mut T` | zero | compile-time check, zero runtime overhead |
| `ref` (not cross-task) | low | compiled to Rc, non-atomic operations |
| `ref` (cross-task) | medium | compiled to Arc, atomic operations |
| `clone()` | type-dependent | fast for small objects, slow for large |
| `unsafe + *T` | zero | direct memory access |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|----------|-------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + borrow checker | compile-time check | manual Weak | high |
| Go | chan / pointer | GC | GC | low |
| C++ | shared_ptr | RAII | weak_ptr | medium |
| **YaoXiang** | **ref + minimalist borrow** | **RAII** | **task boundary release / cross-task lint / stdlib Weak** | **low** |

---

## Trade-offs

### Advantages

1. **Simple**: no lifetimes, no global borrow checker. `&T`/`&mut T` three rules
2. **Smart compiler**: ref auto-selects Rc/Arc, call site auto-selects borrow
3. **Deterministic**: ref keeps alive, won't quietly become weak reference
4. **High performance**: Move zero-copy, borrow zero overhead
5. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **ref runtime overhead**: atomic operations have cost (but this is the inevitable cost of sharing)
2. **unsafe risk**: user must guarantee correctness
3. **Cross-task cycles are lint not compile error**: unlike Rust which fails to compile, default warn, teams need to configure deny for quality gates

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|---------------|
| GC | runtime overhead, unpredictable pauses |
| Rust borrow checker | needs lifetimes `'a`, steep learning curve |
| pure Move | cannot handle concurrent sharing |
| no raw pointers | cannot do system-level programming |
| expose Rc/Arc to users | implementation details dumped on users, increased cognitive load |

---

## Design Decision Log

| Decision | Choice | Reason | Date |
|----------|--------|--------|------|
| **Default** | Move (zero-copy) | high performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimizes | user simplicity, compiler responsibility | 2025-01-15 |
| **Borrow** | `&T`/`&mut T`, params only, no escaping | no lifetimes needed, simple and safe | 2025-01-15 |
| **Copy** | `clone()` | explicit semantics | 2025-01-15 |
| **System-level** | `*T` + `unsafe` | supports system programming | 2025-01-15 |
| **Lifetimes** | not implemented | simple borrow rules don't need lifetimes | 2025-01-15 |
| **Rc/Arc** | compiler auto-selects, invisible to user | reduce cognitive load | 2025-01-15 |
| **Cyclic references** | no check within task, cross-task lint (default warn) | structured concurrency naturally guarantees, lint configurable deny | 2025-01-16 |
| **Weak** | stdlib provided | advanced users explicit choice | 2025-01-16 |
| **Consumption analysis** | removed | mini borrow checker, not needed | 2026-05-11 |
| **Ownership return** | removed | `(T) -> T` signature is documentation | 2026-05-11 |
| **Empty state reuse** | removed (as feature) | reassignment after Move is natural behavior | 2026-05-11 |
| **inverse function/partial consume/field three-tier mutability** | removed | over-engineered | 2026-05-11 |

### Version History

| Version | Major Changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| v4 | Default Move + explicit ref | 2025-01-15 |
| v5 | Structured concurrency + cyclic reference handling | 2025-01-16 |
| v6 | Added empty state reuse, ownership return | 2025-02-04 |
| v7 | Added consumption analysis, inverse functions, field-level mutability | 2025-02-05 |
| **v8** | **Removed over-engineering, added minimalist borrow &T/&mut T** | **2026-05-11** |

### Open Issues

| Topic | Description | Status |
|-------|-------------|--------|
| Drop syntax | Whether explicit `drop()` function needed | pending discussion |
| Escape analysis algorithm | ref cross-task detection implementation | pending discussion |
| Cross-borrow checking | call-site immediate check + function body flow-sensitive, see below | ✅ resolved |

### Cross-Borrow Checking: Call-Site Immediate + Flow-Sensitive

**Analysis scope**: within function body only. Because `&T`/`&mut T` can only be parameters, each call chain releases the previous function's borrows when entering the next function. No cross-function analysis needed.

**Layer 1: call-site check**—each actual argument cannot simultaneously appear in `&mut` position and other borrow positions:

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p used as both &mut and &, compiler rejects
```

**Layer 2: function body flow-sensitivity**—after `&mut` passed to call, borrow releases when call returns, can borrow again:

```yaoxiang
process_twice: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)    # &mut passed to shift, borrow ends when shift returns
    print_info(p)          # reborrow &p, no conflict
}
```

**What's not needed**: cross-function lifetime tracking, global alias analysis, borrow graph constraint solving, NLL. Because borrows don't escape, don't store in variables, don't return—invalid once call site ends.

---

## References

### YaoXiang Official Documentation

- [Language Spec](../language-spec.md)
- [Design Manifesto](../manifesto.md)
- [RFC-001 Concurrent Model](./001-concurrent-model-error-handling.md)
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