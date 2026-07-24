---
title: "RFC-009: Ownership Model Design"
status: "Accepted"
author: "Chenxu"
created: "2025-01-08"
updated: "2026-06-13 (Token conflict detection corrected to Hoare proposition, main text synchronized with RFC-009a)"
issue: "#126"
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core Design — Five concepts, one gradient**:

```
Peek/In-place   Take away      Shared hold        Clone one        System-level
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         Zero-copy      Compiler     Explicit       *T
  Zero-size      Default        auto picks   deep copy
  token                        Rc/Arc
  Type property
  derives
  permission
```

- **Move (Default)**: Assignment / parameter passing / return = ownership transfer, zero-copy, automatic release via RAII
- **`&T` / `&mut T` (Borrow Token)**: Zero-sized compile-time token types. `&T` is Duplicable (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are derived naturally from type properties, no special rules needed. Can be returned, stored in structs.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically picks Rc (not crossing tasks) or Arc (crossing tasks)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, system-level escape hatch

**Eliminated Complexity**:
- ❌ No lifetime `'a`
- ❌ No standalone borrow checking framework (borrow conflict is reduced to Hoare proposition, sharing the proof pipeline with type checking)
- ❌ No GC
- ❌ No special rules like "no escape" (tokens are ordinary types, scope handled uniformly by the type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler picks automatically)

> **Programming burden**: `&T` is Duplicable, `&mut T` is non-Duplicable — two type properties, zero special rules, fully automatic compiler.
> **Performance guarantee**: Move is zero-cost, tokens are zero-cost (zero-sized type, disappears after compilation), ref is pay-as-you-go, no GC pauses.

## Motivation

### Why need an ownership model?

| Language | Memory Management | Problem |
|------|----------|------|
| C/C++ | Manual management | Memory leaks, dangling pointers, double-free |
| Java/Python | GC | Latency jitter, memory overhead, unpredictable pauses |
| Rust | Ownership + borrow checking | Steep learning curve with lifetime `'a` |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T borrow tokens (zero-cost, permissions derived naturally from type properties)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = sharing (compiler auto-picks Rc/Arc)
shared = ref p2                # Cross-scope hold
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, independent

# 5. unsafe + *T = system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature | Rust | YaoXiang |
|------|------|----------|
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)** |
| Borrow | `&T`/`&mut T`, can return, requires lifetime | **`&T`/`&mut T` zero-sized tokens, Dup/Linear type properties derive permissions naturally** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-picks Rc/Arc)** |
| Copy | `clone()` | `clone()` |
| Raw pointer | `*T` | `*T` |
| Lifetime | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker auto-generates borrow propositions, unified proof pipeline verification** |
| Cycle reference | Manual Weak | **Task-end unified release / cross-task lint / std lib Weak** |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: Assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p can no longer be read

# Variables can be reassigned (Python-style, no shadowing)
p = Point(3.0, 4.0)              # p re-bound, type must be consistent

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

**Features**:
- Zero-copy (compiler moves pointer)
- Original binding cannot be read after move (compile error)
- RAII: automatic release when scope ends
- Function signature `(T) -> T` is itself the documentation — consume T, return T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not "references", but "type-level proofs of access permission".**

#### 2.1 Two Type Properties

```
&T      →  Zero-sized, freezes source data (WriteToken forbidden while ReadToken lives),
          freezing guarantee makes multiple read-only views safe → Duplicable (Dup)
&mut T  →  Zero-sized, exclusive read-write (any other token forbidden while WriteToken lives),
          exclusive access makes copying meaningless → linear (non-Dup)
```

**Causality cannot be reversed: freezing is the cause, Dup is the result.** It's not because `&T` implements Dup that it can coexist — it's because the data is frozen (no mutation possible), making multiple read-only views safe, which enables Dup. If we treat Dup as the definition and conflict checking as an "extra patch", the design is wrong.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter type, determines required permission
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
p.print()                          # OK, previous token released when shift call ended

# Free functions are the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escape" Is Not Needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — only as parameters, cannot return, cannot store in structs. This was patching the concept of "borrowing".

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scope rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Token propagates along with the return value
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
# ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds read-only view of target
}

# view's token derives from target, Window owns both
# As long as Window exists, view token is valid
```

#### 2.3 Closures and Lambdas Use Explicit Parameters

Lambdas are function values — they can be returned, stored, passed out of the current scope. Therefore Lambdas **do not implicitly capture outer local variables**. When outer data is needed, pass it via explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not affected by this rule — spawn is an immediately executed concurrent block, parent task blocks waiting
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x is not in scope

# ✅ Correct way: explicit parameter passing
f = (x) => { x + 1 }
f(x)
```

**spawn { } is not a function value.** A spawn-marked block is like if/while body — executes immediately, completes while the parent stack frame is still alive. spawn body can normally access outer variables.

**Cross-task — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross tasks
}

# This is not a special rule — tokens are compile-time permission proofs, use ref for cross-task sharing
# Use ref for cross-task sharing
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token Lifetime

Token lifetime is determined by **ordinary scope rules**, no lifetime parameters needed:

- Tokens in function parameters: live during the call, released after call ends
- Returned tokens: ownership transferred to caller
- Tokens stored in structs: live together with the struct

The compiler doesn't need `'a` annotations, because tokens are **values**, and value lifetimes are managed uniformly by the ownership system (Move/RAII). **Reducing the borrow problem to an ownership problem.**

#### 2.5 Token Conflict Detection

Token conflict detection is a **Hoare logic proposition**, not a standalone flow-sensitive analysis.

```
{Conflicting ReadTokens all dead} write(data) {WriteToken safely obtained}
```

It shares the proof pipeline from RFC-027 with type checking and user predicate verification. The compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`) and feeds them into the pipeline. The pipeline returns Proved / Disproved / Unproven.

```yaoxiang
# ❌ &mut token is linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # ❌ Compile error: WriteToken already moved
}

# ✅ &T token is Dup type, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking hasn't disappeared — it's been reduced.** The existing `BorrowChecker` becomes `BorrowPredicateEmitter` (proposition generator), generating borrow propositions that share the same proof pipeline with other type propositions. This parallels the type checker concept perfectly: type checker generates type equality propositions, borrow proposition generator generates borrow propositions, the same pipeline verifies. See [RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler Internals: Brand Mechanism

Users never touch brands. The compiler internally assigns a compile-time unique identifier to each token:

```
User sees            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is compile-time unique integer
```

Brand purposes:
- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of thin air
- **Relation tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries the derived brand (`#N.field_x`), compiler can trace to parent token
- **Conflict detection**: Same-source `WriteToken` and derived `ReadToken` cannot be active simultaneously

Brands completely disappear after monomorphization and inlining, not present in generated machine code. **Zero runtime overhead.**

#### 2.8 Auto Borrow Selection Rules

The compiler at the call site auto-selects according to the following priority:

```
1. If the argument is used later → prefer creating token (&T or &mut T, based on method signature)
2. If the argument is not used later → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
# Example: auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Minimalist Borrowing

| Feature | Minimalist Borrowing (v8) | Borrow Tokens (v9) |
|------|--------------|--------------|
| Return reference | ❌ Hardcoded forbidden | ✅ Token propagates with return value |
| Store in struct | ❌ Hardcoded forbidden | ✅ Token as struct field |
| Lambda explicit parameters | ❌ Hardcoded forbidden | ✅ Lambda uses explicit parameters |
| Special rules | 3 (only as parameter / cannot return / cannot store) | 0 — type properties derive naturally |
| Borrow checking | Dedicated cross-borrow checking | Type checker flow-sensitive liveness analysis |
| Lifetime annotation | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-sized type, disappears after compilation) |
| Error message | "Borrow cannot escape" | "WriteToken(#3) already moved" (regular type error) |
| User mental model | Understanding the special status of "borrow" | `&T` is Duplicable, `&mut T` is non-Duplicable |

---

### 3. `ref` Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether the underlying implementation is Rc or Arc, users don't need to care.

#### 3.1 Basic Usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Share, compiler auto-picks implementation

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
    use(data)                     # Compiler: not crossing tasks → Rc
}
```

**User mental model**: `ref` = shared hold. That's enough.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
Data flow analysis for ref:

Does not escape to other tasks → Rc (non-atomic reference count, low overhead)
Escapes to other tasks         → Arc (atomic reference count, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycle → silently allowed.
  ├── Each task has a clear lifetime boundary — all resources (including ref cycles) are released uniformly when task ends.
  ├── Long-running services should create child tasks per request/connection — child tasks auto-recycle on end, no accumulated leaks.
  ├── ref is always kept alive, semantics not diluted.
  └── Users have the right to build bidirectional strong references within tasks (e.g., graph computation intermediate state).

Cross-task cycle → lint (default warn, configurable).
  ├── Program behavior is correct, no real leak (child task resources all released when parent ends).
  ├── But cross-task strong references mean blurry ownership boundaries, worth pausing to rethink.
  ├── Default warn level, compile passes with hint.
  └── Teams can set to deny in project config, incorporate into CI quality gate.
```

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|------|------|------|
| `allow` | Don't check | Personal projects |
| `warn` (default) | Compile passes, with hint | Development phase |
| `deny` | Compile fails | Team CI quality gate |
| `forbid` | Compile fails, cannot override | Organization-level mandatory rules |

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
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task cycle reference
    }
}
```

**Project configuration example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycle directly rejected on CI
```

| Cycle Type | Behavior | Reason |
|--------|------|------|
| Intra-task ref cycle | No check | User's right, released uniformly at task end |
| Cross-task ref cycle | Lint (default warn) | Reminder to rethink, configurable to deny |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not a language built-in, it's a standard library type.** Daily use of `ref` is sufficient. Advanced users who need fine-grained memory control manually import `Weak`.

#### 3.5 Borrow Tokens vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| What it does | Peek / in-place modify | Shared hold |
| Scope | Follows token value's scope | Cross-scope |
| Cost | Zero overhead (zero-sized type) | Rc or Arc (compiler picks) |
| Escape | Allowed (token propagates with return value / struct / closure) | Designed to escape |
| Cross-task | Not allowed (token is compile-time permission proof, cannot cross tasks) | Allowed (compiler auto-picks Arc) |
| Cycle | Not involved | Intra-task silently allowed, cross-task lint |

---

### 4. `clone()` — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, don't affect each other
```

**When to use**: When you need to retain the original value and Move or sharing isn't suitable.

### 5. `unsafe` + Raw Pointers (System-Level Programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Restrictions**:
- Can only be used inside `unsafe` blocks
- User guarantees no dangling, no use-after-free
- Used for FFI, memory operations, and other system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow token (zero-cost)  Move (zero-cost)    Sharing (pay-as-you-go)  Copy
   │                      │                  │                │
  &T Duplicable token   Default ownership   ref Rc/Arc       clone()
  &mut T linear token   transfer            Compiler         Explicit deep copy
   │                      │   chain consume  auto-picks      Any time
  Token value scope      Scope              ref cross-task → Arc  Independent copy
  Can return / store     T -> T return      ref not cross-task → Rc
  Zero-sized, disappears T -> Void consume  Intra-task cycle silent
  after compilation                          Cross-task cycle lint
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

    # Move → Move: consume and return
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

# Lambda explicit parameters
double: (x: Int) -> Int = (x) => x * 2

# Comprehensive usage
p = Point(1.0, 2.0)
p.print()                           # &Point token
p.shift(1.0, 1.0)                   # &mut Point token
p = p.scale(2.0)                    # Move → return
shared = ref p                      # ref share
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# Intra-task cycle: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, released uniformly at task end

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Dup Type Property

`Dup` (Duplicable) is a type property automatically managed by the compiler, meaning **shallow copy**: when assigning / passing parameters, the handle/token is copied, the underlying data is shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creating independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies the handle sharing data, Clone creates independent copy. A type can support both Dup and Clone, or only one of them.

| Type | Dup | Clone | Notes |
|------|-----|-------|------|
| `&T` | ✅ (copy token, multiple views point to same data) | ✅ | Read-only token |
| `ref T` | ✅ (reference count +1, share heap data) | ✅ | Shared hold (compiler auto-picks Rc/Arc) |
| String, Bytes | ✅ (internal reference count, copy handle sharing underlying buffer) | ✅ | String/bytes |
| `&mut T` | ❌ (linear, exclusive) | ❌ | Mutable token |
| `*T` | ❌ | ❌ | Raw pointer |
| struct | Derived (auto-derived when all fields are Dup) | ✅ | Struct |

**Primitive value types** (Int, Float, Bool, Char) assignment behavior is the compiler's built-in value copy — two values are completely independent, not shallow copy. They don't belong to the Dup type property, but are native compiler handling.

---

## Performance Analysis

| Operation | Cost | Notes |
|------|------|------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-sized type, disappears after compilation, zero runtime overhead |
| `ref` (not cross-task) | Low | Compiled to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiled to Arc, atomic operation |
| `clone()` | Depends on type | Fast for small objects, slow for large objects |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|------|----------|----------|----------|----------|
| Rust | Arc / Mutex + borrow checking | Compile-time check | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + borrow tokens** | **RAII** | **Task boundary release / cross-task lint / std lib Weak** | **Low** |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetime, borrow checking reduced to type system propositions. `&T` is Duplicable, `&mut T` is non-Duplicable — two type properties
3. **Powerful**: Can return references, store in structs, closure capture — expressiveness on par with Rust
4. **Compiler intelligence**: ref auto-picks Rc/Arc, call site auto-selects borrow
5. **Deterministic**: ref keeps alive, won't quietly become weak reference
6. **High performance**: Move is zero-copy, tokens are zero-cost (zero-sized type, disappears after compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, return reference function signatures show extra generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the necessary cost of sharing)
3. **unsafe risk**: Users must guarantee correctness
4. **Cross-task cycle is lint, not compile error**: Unlike Rust that compile-fails, default warn, requires team config deny to act as quality gate

---

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| GC | Runtime overhead, unpredictable pauses |
| Rust borrow checker | Requires lifetime `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointer | Cannot do system-level programming |
| Expose Rc/Arc to users | Push implementation details to users, increase cognitive burden |
| Minimalist borrowing (v8) | The "no escape" strategy sacrifices key expressiveness like closure capture, return reference |

---

## Design Decision Record

| Decision | Determination | Reason | Date |
|------|------|------|------|
| **Default value** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimization | User simple, compiler responsible | 2025-01-15 |
| **Borrowing** | `&T`/`&mut T` as zero-sized token types | Type properties (Dup/Linear) derive permissions naturally, unified type system | 2025-01-15 |
| **Borrow tokens** | Replace minimalist borrowing, `&T` Dup, `&mut T` Linear | Eliminate special rules like "no escape", support closure capture / return reference / store in struct | 2026-05-29 |
| **Copy** | `clone()` | Explicit semantics | 2025-01-15 |
| **System-level** | `*T` + `unsafe` | Support system programming | 2025-01-15 |
| **Lifetime** | Not implemented | Tokens are values, lifetime managed uniformly by Move/RAII, reducing borrow to ownership problem | 2025-01-15 |
| **Rc/Arc** | Compiler auto-select, invisible to users | Reduce cognitive burden | 2025-01-15 |
| **Cycle reference** | No check within task, lint across tasks (default warn) | Structured concurrency natural guarantee, lint configurable to deny | 2025-01-16 |
| **Weak** | Provided by standard library | Advanced users explicitly choose | 2025-01-16 |
| **Consume analysis** | Removed | Mini borrow checker, not needed | 2026-05-11 |
| **Ownership return** | Removed | `(T) -> T` signature is the documentation | 2026-05-11 |
| **Empty state reuse** | Removed (as feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function / partial consume / field three-level mutability** | Removed | Over-engineering | 2026-05-11 |
| **Lambda non-implicit capture** | Lambda only uses explicit parameters, does not implicitly capture outer variables | Explicit philosophy, simplify compiler | 2026-06-16 |

### Version History

| Version | Major Changes | Date |
|------|----------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| **v8** | **Remove over-engineering (inverse function / partial consume / field three-level mutability / consume analysis / ownership return / empty state reuse), add minimalist borrowing &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces minimalist borrowing, unified type system; token conflict detection corrected to Hoare proposition, see RFC-009a** | **2026-06-13** |

### Pending Issues

| Issue | Description | Status |
|------|------|------|
| Drop syntax | Whether explicit `drop()` function is needed | To discuss |
| Escape analysis algorithm | Cross-task detection implementation for ref | To discuss |
| Token conflict detection | Hoare logic proposition, see below | ✅ Resolved (see RFC-009a for details) |

### Token Conflict Detection: Hoare Logic Proposition

The complete solution for token conflict detection is in [RFC-009a: Token Lifetime Analysis — Based on Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md). Key points:

**Token liveness is a Hoare logic proposition.** `{Conflicting ReadTokens all dead} write(data) {WriteToken safely obtained}` — shares the proof pipeline from RFC-027 with type checking and user predicate verification. The compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), pipeline returns Proved / Disproved / Unproven.

**Borrow checking hasn't disappeared — it's been reduced.** `BorrowChecker` becomes `BorrowPredicateEmitter`, generating propositions rather than performing checks. This parallels the "type checker" concept perfectly: type checker generates type equality propositions, borrow proposition generator generates borrow propositions, the same pipeline verifies.

**Brand ID (`#42`) is `'a`.** Information is exactly the same, encoding differs. `'a` is visible in type signatures, `#42` is inside the compiler. No new analysis invented — lifetime is reduced from the type layer to the proof layer.

**Algorithm summary** (see RFC-009a for details):
- Brand tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumer, break cuts back-edges, structural analysis covers 95%+ scenarios (fast path)
- SMT logical cut → only called when while + path conditions (slow path, very rare)

---

## References

### YaoXiang Official Documents

- [Language Specification](../language-spec.md)
- [Design Manifesto](../manifesto.md)
- [RFC-001 Concurrent Model](./001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [tutorial/](../../../../../tutorial/)
### External References

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting review submission |
| **Under Review** | `docs/design/rfc/` | Open community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes official design document |
| **Rejected** | `docs/design/rfc/` | Retained in RFC directory |