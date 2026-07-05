---
title: "RFC-009: Ownership Model Design"
status: "Accepted"
author: "Chenxu"
created: "2025-01-08"
updated: "2026-06-13 (Token conflict detection revised to Hoare proposition, body synchronized with RFC-009a)"
issue: "#126"
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core Design — Five Concepts, One Gradient**:

```
Take a look / Modify in place   Take it away    Shared hold       Clone a copy       System level
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         Zero-copy      Compiler       Explicit        *T
  Zero-size       by default    auto-chooses   deep copy       User responsible
  token                          Rc/Arc
  Type attribute
  derives
  permissions
  naturally
```

- **Move (default)**: assignment / argument passing / return = ownership transfer, zero-copy, RAII auto-release
- **`&T` / `&mut T` (Borrow Token)**: zero-size compile-time token type. `&T` is duplicable (shared read), `&mut T` is linear (exclusive write). Permissions are derived naturally from type attributes, no special rules required. Can be returned, can be stored in structs.
- **`ref` keyword**: cross-scope sharing. The compiler auto-chooses Rc (non-cross-task) or Arc (cross-task)
- **`clone()`**: explicit deep copy
- **`unsafe` + `*T`**: raw pointer, system-level escape hatch

**Eliminated Complexity**:
- ❌ No lifetime `'a`
- ❌ No independent borrow checking framework (borrow conflict reduces to a Hoare proposition, sharing the proof pipeline with type checking)
- ❌ No GC
- ❌ No "no escape" or other special rules (tokens are ordinary types, scope handled uniformly by the type system)
- ❌ Users don't need to know the difference between Rc and Arc (compiler auto-selects)

> **Programming burden**: `&T` is duplicable, `&mut T` is not — two type attributes, zero special rules, fully automatic compiler.
> **Performance guarantees**: Move is zero-overhead, tokens are zero-overhead (zero-size type, disappears after compilation), ref is pay-as-you-go, no GC pauses.

## Motivation

### Why an ownership model?

| Language | Memory Management | Problem |
|------|----------|------|
| C/C++ | Manual management | Memory leaks, dangling pointers, double free |
| Java/Python | GC | Latency fluctuations, memory overhead, unpredictable pauses |
| Rust | Ownership + borrow checking | Steep learning curve for lifetime `'a` |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T borrow tokens (zero-overhead, type attributes naturally derive permissions)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = shared (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope hold
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, exclusive

# 5. unsafe + *T = system level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature | Rust | YaoXiang |
|------|------|----------|
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)** |
| Borrow | `&T`/`&mut T`, can be returned, requires lifetimes | **`&T`/`&mut T` zero-size tokens, Dup/Linear type attributes naturally derive permissions** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-selects Rc/Arc)** |
| Copy | `clone()` | `clone()` |
| Raw pointer | `*T` | `*T` |
| Lifetime | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker auto-generates borrow propositions, unified proof pipeline for verification** |
| Circular references | Manual Weak | **Task-end unified release / cross-task lint / standard library Weak** |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: assignment / argument passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p can no longer be read

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p is rebound, type must remain consistent

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
- Zero-copy (compiler moves the pointer)
- Original binding is unreadable after move (compile error)
- RAII: automatic release when scope ends
- Function signature `(T) -> T` is itself the documentation — consume T, return T

---

### 2. &T / &mut T (Borrow Token)

**Core principle: `&T` and `&mut T` are zero-size compile-time token types. They are not "references" but "type-level proofs of access permission."**

#### 2.1 Two Type Attributes

```
&T      →  Zero-size, freezes source data (WriteToken forbidden while ReadToken is alive),
          under the freezing guarantee, multiple read-only views are safe → Duplicable
&mut T  →  Zero-size, exclusive read-write (any other token forbidden while WriteToken is alive),
          under exclusive access, duplication is meaningless → linear (non-Duplicable)
```

**Causality cannot be reversed: freezing is the cause, Dup is the result.** It is not that `&T` implements Dup and therefore can coexist — it is that the data is frozen (mutation is impossible), so multiple read-only views are safe, and Dup is therefore implementable. Treating Dup as the definition and conflict checking as an "extra patch" would be a design error.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter type, determine required permission
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
p.print()                          # OK, previous token was released when shift call ended

# Free function similarly
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escape" is Not Needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — can only be a parameter, cannot be returned, cannot be stored in a struct. This was patching the "borrow" concept.

The token system does not need these rules. Tokens are **ordinary types**, following the same scoping rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Token propagates together with the return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Sub-token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to the caller
print(px_ref)               # OK, token is still in scope
```

**Storing in a struct — naturally supported**:

```yaoxiang
# ✅ Struct carries tokens as fields
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds a read-only view of target
}

# The view token is derived from target; Window holds ownership of both
# As long as Window exists, the view token is valid
```

#### 2.3 Closures and Lambda Explicit Parameters

A Lambda is a function value — it can be returned, stored, and passed out of the current scope. Therefore a Lambda **does not implicitly capture outer local variables**. When outer data is needed, pass it in via explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not affected by this rule — spawn is an immediately-executed concurrent block, the parent task blocks and waits
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x is not in scope

# ✅ Correct way: explicit parameter passing
f = (x) => { x + 1 }
f(x)
```

**`spawn { }` is not a function value.** A block marked by spawn, like if/while bodies, is executed immediately and completes while the parent stack frame is still alive. The spawn body can normally access outer variables.

**Cross-task — tokens cannot cross thread boundaries**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross tasks
}

# This is not a special rule — tokens are compile-time permission proofs; use ref for cross-task sharing
# If cross-task sharing is needed, use ref
```

**Tokens cannot be `ref`'d**:

```yaoxiang
# ❌ Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token Lifetime

The lifetime of a token is determined by **ordinary scoping rules** and does not require lifetime parameters:

- Tokens in function parameters: alive during the call, released when the call ends
- Returned tokens: ownership transferred to the caller
- Tokens stored in a struct: alive together with the struct

The compiler does not need `'a` annotations, because tokens are **values**, and the lifetime of values is uniformly managed by the ownership system (Move/RAII). **Reducing the borrow problem to an ownership problem.**

#### 2.5 Token Conflict Detection

Token conflict detection is a **Hoare logic proposition**, not an independent flow-sensitive analysis.

```
{All conflicting ReadTokens dead} write(data) {WriteToken safely acquired}
```

It shares RFC-027's proof pipeline with type checking and user predicate verification. The compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`) and feeds them into the pipeline for verification. The pipeline returns Proved / Disproved / Unproven.

```yaoxiang
# ❌ &mut tokens are linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # ❌ Compile error: WriteToken has been moved
}

# ✅ &T tokens are Dup type, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking has not disappeared — it has been reduced in dimensionality.** The existing `BorrowChecker` becomes a `BorrowPredicateEmitter` (proposition generator), and the generated borrow propositions share the same proof pipeline as other type propositions. This is completely parallel to the concept of a type checker: the type checker generates type-equality propositions, the borrow proposition generator generates borrow propositions, and the same pipeline verifies them. See [RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler Internals: Brand Mechanism

Users never encounter brands. The compiler internally assigns a compile-time unique identifier to each token:

```
What users see       Compiler-internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Purposes of brands:
- **Anti-forgery**: tokens can only be obtained from the owner capsule, cannot be fabricated out of thin air
- **Association tracking**: when deriving `&Float` from `&Point` (field access), `&Float` carries a derived brand (`#N.field_x`), and the compiler can trace back to the parent token
- **Conflict detection**: same-source `WriteToken` and derived `ReadToken` cannot be simultaneously active

Brands completely disappear after monomorphization and inlining; they do not exist in the generated machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

The call-side compiler auto-selects according to the following priority:

```
1. If the argument is used later → prioritize creating a token (&T or &mut T, per method signature)
2. If the argument is not used later → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
# Example: auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p is no longer used
```

#### 2.9 Comparison with RFC-009 v8 Beggar-Version Borrow

| Feature | Beggar-Version Borrow (v8) | Borrow Token (v9) |
|------|--------------|--------------|
| Returning references | ❌ Hardcoded prohibition | ✅ Tokens propagate with return value |
| Storing in struct | ❌ Hardcoded prohibition | ✅ Tokens as struct fields |
| Lambda explicit parameters | ❌ Hardcoded prohibition | ✅ Lambda uses explicit parameters |
| Special rules | 3 (parameter only / no return / no storage) | 0 — type attributes naturally derive |
| Borrow checking | Dedicated cross-borrow check | Type checker flow-sensitive liveness analysis |
| Lifetime annotations | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-size type, disappears after compilation) |
| Error message | "Borrow cannot escape" | "WriteToken(#3) has been moved" (regular type error) |
| User mental model | Understanding the "special status" of borrow | `&T` is duplicable, `&mut T` is not |

---

### 3. `ref` Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether the underlying implementation is Rc or Arc, the user does not need to care.

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

**User mental model**: `ref` = shared hold. That's enough.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks         → Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycle → silently allowed.
  ├── Each task has a clear lifecycle boundary — when the task ends, all resources (including ref cycles) are released uniformly.
  ├── Long-running services should create sub-tasks per request/connection — sub-task end auto-collects, no accumulated leaks.
  ├── ref always keeps alive; semantics are not watered down.
  └── Users have the right to construct bidirectional strong references within a task (e.g., graph computation intermediates).

Cross-task cycle → lint (default warn, configurable).
  ├── Program behavior is correct, no real leak (when the parent task ends, all child-task resources are released).
  ├── But cross-task strong references imply fuzzy ownership boundaries, worth pausing to rethink.
  ├── Default warn level, compilation passes with hints.
  └── Teams can set to deny in project config, integrating into CI quality gates.
```

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|------|------|------|
| `allow` | No check | Personal project |
| `warn` (default) | Compilation passes, with hint | Development phase |
| `deny` | Compilation fails | Team CI quality gate |
| `forbid` | Compilation fails, cannot override | Organization-level mandatory rule |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong references
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Released uniformly when task ends.
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
cross-task-cycle = "deny"    # Cross-task cycles are rejected directly on CI
```

| Cycle Type | Behavior | Reason |
|--------|------|------|
| Intra-task ref cycle | No check | User's prerogative, task-end uniform release |
| Cross-task ref cycle | Lint (default warn) | Reminder to rethink, configurable deny |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.rc.Weak

# Advanced users explicitly opt in
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not a language built-in, but a standard library type.** For daily use, `ref` is sufficient. Advanced users who need fine-grained memory control manually introduce `Weak`.

#### 3.5 Borrow Token vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Take a look / Modify in place | Shared hold |
| Scope | Follows the scope of the token value | Cross-scope |
| Cost | Zero overhead (zero-size type) | Rc or Arc (compiler selects) |
| Escape | Yes (token propagates with return value/struct/closure) | Designed to escape |
| Cross-task | No (token is compile-time permission proof, cannot cross tasks) | Yes (compiler auto-selects Arc) |
| Cycle | Not involved | Intra-task silently allowed, cross-task lint |

---

### 4. `clone()` — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, do not affect each other
```

**When to use**: scenarios where the original value must be preserved and Move or sharing is not appropriate.

### 5. `unsafe` + Raw Pointer (System-Level Programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Limitations**:
- Can only be used within an `unsafe` block
- User guarantees no dangling, no use-after-free
- Used for FFI, memory operations, and other system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow Token (zero overhead)   Move (zero overhead)   Shared (pay-as-you-go)   Copy
   │                      │                  │                │
  &T Duplicable token    Default ownership   ref Rc/Arc     clone()
  &mut T Linear token    transfer            Compiler       Explicit deep copy
   │                      │                  auto-selects   │
  Token value scope       In-scope            Cross-scope     Anytime
  Can return/store in     T -> T loop-back    ref cross-task Independent copy
  struct                  T -> Void consume   → Arc
  Zero-size disappears                            ref non-cross-task → Rc
  after compilation                             Intra-task cycle silent
                                                Cross-task cycle lint
                                                Stdlib Weak escape
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
        self                            # Take it, modify, give it back
    }

    # Returning reference: token propagates with return value
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# Lambda explicit parameters
double: (x: Int) -> Int = (x) => x * 2

# Comprehensive usage
p = Point(1.0, 2.0)
p.print()                           # &Point token
p.shift(1.0, 1.0)                   # &mut Point token
p = p.scale(2.0)                    # Move → loop-back
shared = ref p                      # ref shared
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# Intra-task cycle: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, released uniformly when task ends

# unsafe system level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Dup Type Attribute

`Dup` (Duplicable) is a type attribute auto-managed by the compiler, meaning **shallow copy**: on assignment/argument-passing, what is copied is the handle/token, and the underlying data is shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creates an independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies the handle and shares the data, Clone creates an independent copy. A type can support both Dup and Clone, or only one of them.

| Type | Dup | Clone | Description |
|------|-----|-------|------|
| `&T` | ✅ (copies token, multiple views point to same data) | ✅ | Read-only token |
| `ref T` | ✅ (reference count +1, shares heap data) | ✅ | Shared hold (compiler auto-selects Rc/Arc) |
| String, Bytes | ✅ (internal reference count, copy handle shares underlying buffer) | ✅ | String/bytes |
| `&mut T` | ❌ (linear, exclusive) | ❌ | Mutable token |
| `*T` | ❌ | ❌ | Raw pointer |
| struct | Derived (auto-derived when all fields are Dup) | ✅ | Struct |

**Primitive value types** (Int, Float, Bool, Char) have assignment behavior that is a compiler built-in value copy — two values are completely independent, not a shallow copy. They do not belong to the Dup type attribute; they are handled natively by the compiler.

---

## Performance Analysis

| Operation | Cost | Description |
|------|------|------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-size type, disappears after compilation, zero runtime overhead |
| `ref` (non-cross-task) | Low | Compiled to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiled to Arc, atomic operation |
| `clone()` | Depends on type | Fast for small objects, slow for large objects |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|------|----------|----------|----------|----------|
| Rust | Arc / Mutex + borrow checking | Compile-time check | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + Borrow Token** | **RAII** | **Task boundary release / cross-task lint / standard library Weak** | **Low** |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, borrow checking reduced to type system propositions. `&T` is duplicable, `&mut T` is not — two type attributes
3. **Powerful**: Can return references, store in structs, capture in closures — expressive power on par with Rust
4. **Compiler smart**: ref auto-selects Rc/Arc, call-side auto-selects borrow
5. **Deterministic**: ref always keeps alive, no silent weakening
6. **High performance**: Move zero-copy, token zero-overhead (zero-size type, disappears after compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter propagation**: tokens carry brand identifiers, and the function signature of returning a reference will reflect additional generic parameters
2. **`ref` runtime overhead**: atomic operations have cost (but this is the inevitable cost of sharing)
3. **`unsafe` risk**: user must guarantee correctness
4. **Cross-task cycle is a lint, not a compile error**: unlike Rust which reports a compile error, default is warn, requiring team configuration of deny to serve as a quality gate

---

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| GC | Has runtime overhead, unpredictable pauses |
| Rust borrow checker | Requires lifetime `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointer | Cannot do system-level programming |
| Expose Rc/Arc to users | Dumps implementation details on users, increases cognitive load |
| Beggar-Version Borrow (v8) | The no-escape policy sacrifices key expressiveness such as closure capture and returning references |

---

## Design Decision Record

| Decision | Determination | Reason | Date |
|------|------|------|------|
| **Default value** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimization | User simplicity, compiler responsibility | 2025-01-15 |
| **Borrow** | `&T`/`&mut T` as zero-size token types | Type attributes (Dup/Linear) naturally derive permissions, unified type system | 2025-01-15 |
| **Borrow Token** | Replaces Beggar-Version Borrow, `&T` Dup, `&mut T` Linear | Eliminate "no escape" and other special rules, supports closure capture / return reference / struct storage | 2026-05-29 |
| **Copy** | `clone()` | Explicit semantics | 2025-01-15 |
| **System level** | `*T` + `unsafe` | Supports system programming | 2025-01-15 |
| **Lifetime** | Not implemented | Tokens are values, lifetimes uniformly managed by Move/RAII, reducing borrow to ownership problem | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, invisible to user | Reduce cognitive load | 2025-01-15 |
| **Circular reference** | Intra-task unchecked, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint configurable to deny | 2025-01-16 |
| **Weak** | Provided by standard library | Advanced user explicit choice | 2025-01-16 |
| **Consumption analysis** | Removed | Mini borrow checker, not needed | 2026-05-11 |
| **Ownership loop-back** | Removed | `(T) -> T` signature is itself documentation | 2026-05-11 |
| **Empty-state reuse** | Removed (as a feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function / partial consumption / field three-level mutability** | Removed | Over-engineered | 2026-05-11 |
| **Lambda does not implicitly capture** | Lambda uses only explicit parameters, does not implicitly capture outer variables | Explicit philosophy, simplifies compiler | 2026-06-16 |

### Version History

| Version | Major Changes | Date |
|------|----------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| **v8** | **Removed over-engineering (inverse function / partial consumption / field three-level mutability / consumption analysis / ownership loop-back / empty-state reuse), added Beggar-Version Borrow &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow Token system replaces Beggar-Version Borrow, unified type system; token conflict detection revised to Hoare proposition, see RFC-009a** | **2026-06-13** |

### Pending Issues

| Issue | Description | Status |
|------|------|------|
| Drop syntax | Whether an explicit `drop()` function is needed | To be discussed |
| Escape analysis algorithm | Implementation of ref cross-task detection | To be discussed |
| Token conflict detection | Hoare logic proposition, see below | ✅ Resolved (see RFC-009a for details) |

### Token Conflict Detection: Hoare Logic Proposition

The complete scheme for token conflict detection is in [RFC-009a: Token Lifetime Analysis — Based on the Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md). Core points:

**Token liveness is a Hoare logic proposition.** `{All conflicting ReadTokens dead} write(data) {WriteToken safely acquired}` — sharing RFC-027's proof pipeline with type checking and user predicate verification. The compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), and the pipeline returns Proved / Disproved / Unproven.

**Borrow checking has not disappeared — it has been reduced in dimensionality.** `BorrowChecker` becomes a `BorrowPredicateEmitter`, generating propositions rather than performing checks. This is completely parallel to the concept of a "type checker": the type checker generates type-equality propositions, the borrow proposition generator generates borrow propositions, and the same pipeline verifies them.

**The brand ID (`#42`) is `'a`.** Exactly the same information, different encoding. `'a` is visible in the type signature, `#42` is internal to the compiler. No new analysis is invented — lifetimes are reduced from the type layer to the proof layer.

**Algorithm summary** (see RFC-009a for details):
- Brand tree prefix matching → identify conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumer, break cuts back edges, structural analysis covers 95%+ scenarios (fast path)
- SMT logic cutting → only invoked on `while` + path conditions (slow path, extremely rare)

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
|------|------|------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission for review |
| **Under review** | `docs/design/rfc/` | Open community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes official design document |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory |