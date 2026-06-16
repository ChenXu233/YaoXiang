---
title: "RFC-009: Ownership Model Design"
status: "Accepted"
author: "Chenxu"
created: "2025-01-08"
updated: "2026-06-13 (Token conflict detection corrected to Hoare proposition, body synchronized with RFC-009a)"
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core Design — Five Concepts, One Gradient**:

```
Peek/Mutate    Take         Share-Hold      Copy        System-Level
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         Zero-Copy      Compiler     Explicit      *T
  Zero-Size                    Auto-Select   Deep Copy
  Token            Default      Rc/Arc       User-Driven
  Type Property
  Natural Permission
  Inference
```

- **Move (default)**: Assignment / parameter passing / return = ownership transfer, zero-copy, automatic RAII release
- **`&T` / `&mut T` (Borrow Tokens)**: Zero-sized compile-time token types. `&T` is Duplicable (shared read-only), `&mut T` is Linear (exclusive mutable). Permissions are derived naturally from type properties, no special rules needed. Returnable, storable in structs.
- **`ref` keyword**: Cross-scope sharing. Compiler auto-selects Rc (within task) or Arc (across tasks).
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointers, system-level escape hatch

**Complexity Eliminated**:
- ❌ No lifetime `'a`
- ❌ No separate borrow checker framework (borrow conflict reduced to Hoare proposition, sharing the proof pipeline with type checking)
- ❌ No GC
- ❌ No "no escape" or other special rules (tokens are ordinary types, scope handled uniformly by the type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler selects automatically)

> **Programming Burden**: `&T` is Duplicable, `&mut T` is non-Duplicable — two type properties, zero special rules, fully automatic.
> **Performance Guarantee**: Move is zero-cost, tokens are zero-cost (zero-sized types, disappear after compilation), ref is pay-as-you-go, no GC pauses.

## Motivation

### Why an Ownership Model?

| Language | Memory Management | Problem |
|----------|-------------------|---------|
| C/C++ | Manual management | Memory leaks, dangling pointers, double-free |
| Java/Python | GC | Latency fluctuation, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow checker | Steep learning curve for lifetimes `'a` |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T Borrow Tokens (zero-cost, permissions derived naturally from type properties)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)            # Compiler auto-creates &mut Point token

# 3. ref = Share (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope holding
spawn { use(shared) }          # Compiler: across task → Arc

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
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)** |
| Borrow | `&T`/`&mut T`, returnable, needs lifetime | **`&T`/`&mut T` zero-sized tokens, Dup/Linear type properties derive permissions naturally** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-selects Rc/Arc)** |
| Copy | `clone()` | `clone()` |
| Raw pointer | `*T` | `*T` |
| Lifetime | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker auto-generates borrow propositions, unified proof pipeline verifies** |
| Cyclic reference | Manual Weak | **Task-end unified release / cross-task lint / standard library Weak** |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p can no longer be read

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p rebound, type must be consistent

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
- Original binding unreadable after move (compile error)
- RAII: automatic release at scope end
- Function signature `(T) -> T` is self-documenting — consume T, return T

---

### 2. &T / &mut T (Borrow Tokens)

**Core Principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not "references" — they are "type-level proof of access permission".**

#### 2.1 Two Type Properties

```
&T      →  Zero-sized, freezes source data (WriteToken forbidden while ReadToken is alive),
          multiple read-only views are safe under the freeze guarantee → Duplicable (Dup)
&mut T  →  Zero-sized, exclusive read-write (any other token forbidden while WriteToken is alive),
          copying is meaningless under exclusive access → Linear (non-Dup)
```

**Causality must not be reversed: freeze is the cause, Dup is the result.** It is not that `&T` implements Dup and therefore can coexist — it is that data is frozen (no mutation possible), so multiple read-only views are safe, so Dup can be implemented. If Dup is treated as the definition and conflict checking as an "extra patch", the design is wrong.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter type, determining required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point token grants write permission
    self.y = self.y + dy
}

# Caller side: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, previous token was released when shift call ended

# Free functions work the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escape" Rules Are Not Needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — only as parameters, no return, no storage in struct. This was patching the "borrow" concept.

The token system needs none of these rules. Tokens are **ordinary types**, following the same scope rules as every other type.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Token propagates with the return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Sub-token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Storing in struct — naturally supported**:

```yaoxiang
# ✅ Struct carries token as a field
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds a read-only view of target
}

# The view token derives from target, Window holds ownership of both
# As long as Window exists, the view token is valid
```

#### 2.3 Closures and Lambdas: Explicit Parameters

A lambda is a function value — it can be returned, stored, and passed out of the current scope. Therefore a lambda **does not implicitly capture outer local variables**. To use outer data, pass it as an explicit parameter:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not subject to this rule — spawn is an immediately-executed concurrent block, parent task blocks waiting
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x is not in scope

# ✅ Correct way: explicit parameter passing
f = (x) => { x + 1 }
f(x)
```

**`spawn { }` is not a function value.** A block marked by spawn executes immediately, like the body of if/while — completing while the parent stack frame is alive. The spawn body can access outer variables normally.

**Cross-task — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task
}

# This is not a special rule — tokens are compile-time permission proof, use ref for cross-task sharing
# If you need to share across tasks, use ref
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Token is permission proof, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token Lifetime

Token lifetime is determined by **ordinary scope rules**, with no lifetime parameters needed:

- Tokens in function parameters: alive during the call, released after the call ends
- Returned tokens: ownership transferred to the caller
- Tokens stored in structs: alive together with the struct

The compiler does not need `'a` annotations, because tokens are **values**, and value lifetime is managed uniformly by the ownership system (Move/RAII). **Borrow problem is reduced to ownership problem.**

#### 2.5 Token Conflict Detection

Token conflict detection is a **Hoare logic proposition**, not an independent flow-sensitive analysis.

```
{all conflicting ReadTokens are dead} write(data) {WriteToken safely obtained}
```

It shares the proof pipeline from RFC-027 with type checking and user predicate verification. The compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`) and feeds them into the pipeline, which returns Proved / Disproved / Unproven.

```yaoxiang
# ❌ &mut token is linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # ❌ Compile error: WriteToken has been moved
}

# ✅ &T token is Dup type, freely copyable
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking has not disappeared — it has been reduced.** The existing `BorrowChecker` becomes `BorrowPredicateEmitter` (proposition generator); the borrow propositions it generates share the same proof pipeline as other type propositions. This parallels the type checker perfectly: the type checker generates type equality propositions, the borrow proposition generator generates borrow propositions, and the same pipeline verifies them. See [RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler Internals: Brand Mechanism

Users never encounter brands. The compiler internally assigns each token a unique compile-time identifier:

```
User sees            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a unique compile-time integer
&mut Point     →  WriteToken(Point, #M)   // #M is a unique compile-time integer
```

Purposes of brands:
- **Anti-counterfeiting**: Tokens can only be obtained from owner capsules, not fabricated out of thin air
- **Correlation tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries a derived brand (`#N.field_x`), and the compiler can trace back to the parent token
- **Conflict detection**: Same-origin `WriteToken` and derived `ReadToken` cannot both be active

Brands completely disappear after monomorphization and inlining — they do not exist in the generated machine code. **Zero runtime overhead.**

#### 2.8 Auto Borrow Selection Rules

The compiler on the caller side auto-selects with the following priority:

```
1. If the argument is used later → prefer creating a token (&T or &mut T, per method signature)
2. If the argument is not used later → Move
3. Priority order: &T < &mut T < Move
```

```yaoxiang
# Example: auto-selection
p = Point(1.0, 2.0)
p.print()         # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p            # Move, p is no longer used
```

#### 2.9 Comparison with RFC-009 v8 Bare-Bones Borrowing

| Feature | Bare-Bones Borrowing (v8) | Borrow Token (v9) |
|---------|--------------------------|-------------------|
| Returning reference | ❌ Hard-coded forbidden | ✅ Token propagates with return value |
| Storage in struct | ❌ Hard-coded forbidden | ✅ Token as struct field |
| Lambda explicit parameters | ❌ Hard-coded forbidden | ✅ Lambda uses explicit parameters |
| Special rules | 3 (param-only/no-return/no-store) | 0 — type properties derive naturally |
| Borrow checking | Dedicated cross-borrow checking | Type checker flow-sensitive liveness analysis |
| Lifetime annotation | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-sized types, disappear after compilation) |
| Error message | "Borrow cannot escape" | "WriteToken(#3) has been moved" (regular type error) |
| User mental model | Understand "borrow" has special status | `&T` is Duplicable, `&mut T` is non-Duplicable |

---

### 3. The `ref` Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether the underlying implementation is Rc or Arc is not the user's concern.

#### 3.1 Basic Usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Share, compiler auto-selects implementation

# Cross-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }           # Compiler: across task → Arc
    spawn { use(data) }           # Compiler: across task → Arc
}

# Single-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    use(data)                     # Compiler: not across task → Rc
}
```

**User mental model**: `ref` = shared ownership. That's enough.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks         → Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
In-task cycles → silently allowed.
  ├── Each task has a clear lifetime boundary — when the task ends, all resources (including ref cycles) are released together.
  ├── Long-running services should create child tasks per request/connection — child tasks release automatically on completion, no accumulation.
  ├── ref always keeps alive, semantics are not watered down.
  └── Users have the right to construct bidirectional strong references within a task (e.g., intermediate states in graph computation).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, no real leak (when parent task ends, child task resources are all released).
  ├── But cross-task strong references mean ownership boundaries are blurred, worth pausing to reconsider.
  ├── Default warn level — compilation passes with a hint.
  └── Teams can set it to deny in project config, integrating into CI quality gate.
```

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | No check | Personal projects |
| `warn` (default) | Compiles, with hint | Development stage |
| `deny` | Compile failure | Team CI quality gate |
| `forbid` | Compile failure, cannot override | Organization-level mandatory rule |

```yaoxiang
# In-task cycle: silently allowed, bidirectional strong references
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Released together when task ends.
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
cross-task-cycle = "deny"    # Cross-task cycles directly rejected in CI
```

| Cycle type | Behavior | Reason |
|------------|----------|--------|
| In-task ref cycle | No check | User's privilege, released together at task end |
| Cross-task ref cycle | lint (default warn) | Reminder to reconsider, configurable to deny |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.rc.Weak

# Advanced user explicit choice
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not a language built-in, but a standard library type.** `ref` is enough for daily use. Advanced users who need fine-grained memory control introduce `Weak` manually.

#### 3.5 Borrow Token vs ref

| | `&T` / `&mut T` | `ref` |
|---|------|------|
| What it does | Peek / mutate in place | Shared ownership |
| Scope | Within the token value's scope | Cross-scope |
| Cost | Zero (zero-sized types) | Rc or Arc (compiler selects) |
| Escape | Yes (token propagates with return value / struct / closure) | Designed for escape |
| Cross-task | No (token is compile-time permission proof, cannot cross task) | Yes (compiler auto-selects Arc) |
| Cycles | N/A | In-task silently allowed, cross-task lint |

---

### 4. `clone()` — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, do not affect each other
```

**When to use**: Scenarios where the original value must be kept and neither Move nor sharing is suitable.

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
- Only usable inside `unsafe` blocks
- User guarantees no dangling, no use-after-free
- Used for FFI, memory operations, and other system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow Token (zero-cost)     Move (zero-cost)      Share (pay-as-you-go)    Copy
   │                      │                  │                │
  &T Duplicable token        Default ownership transfer  ref Rc/Arc       clone()
  &mut T Linear token        Chained consume-return      Compiler auto-select  Explicit deep copy
   │                      │                  │                │
  Token value scope         Within scope       Cross-scope         Anytime
  Returnable / storable in struct  T -> T return-flow    ref cross-task → Arc  Independent copy
  Zero-sized, disappears after compilation  T -> Void consume    ref within task → Rc
  Zero-sized, disappears after compilation                      In-task cycle silent
                                                Cross-task cycle lint
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

    # Move → Move: consume-return
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # Take, modify, give back
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
p = p.scale(2.0)                    # Move → return-flow
shared = ref p                      # ref share
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# In-task cycle: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, released together when task ends

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Dup Type Property

`Dup` (Duplicable) is a type property auto-managed by the compiler, meaning **shallow copy**: what gets copied on assignment/parameter passing is the handle/token, with the underlying data shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creating an independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies the handle and shares data, Clone creates an independent copy. A type can support both Dup and Clone, or only one of them.

| Type | Dup | Clone | Notes |
|------|-----|-------|-------|
| `&T` | ✅ (Copy token, multiple views to the same data) | ✅ | Read-only token |
| `ref T` | ✅ (Reference count +1, share heap data) | ✅ | Shared ownership (compiler auto-selects Rc/Arc) |
| String, Bytes | ✅ (Internal reference counting, copy handle shares underlying buffer) | ✅ | String / Bytes |
| `&mut T` | ❌ (Linear, exclusive) | ❌ | Mutable token |
| `*T` | ❌ | ❌ | Raw pointer |
| struct | Derived (auto-derived when all fields are Dup) | ✅ | Struct |

**Primitive value types** (Int, Float, Bool, Char) have assignment behavior that is a built-in value copy by the compiler — the two values are completely independent, not shallow copy. They are not Dup type properties, but rather native compiler handling.

---

## Performance Analysis

| Operation | Cost | Notes |
|-----------|------|-------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-sized types, disappear after compilation, zero runtime overhead |
| `ref` (within task) | Low | Compiled to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiled to Arc, atomic operation |
| `clone()` | Type-dependent | Fast for small objects, slow for large ones |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|----------|-------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + borrow checking | Compile-time check | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + borrow token** | **RAII** | **Task boundary release / cross-task lint / standard library Weak** | **Low** |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with `name: type = value` from RFC-010.
2. **Simple**: No lifetimes, borrow checking reduced to type system propositions. `&T` is Duplicable, `&mut T` is non-Duplicable — two type properties.
3. **Powerful**: Returnable references, storable in structs, closure-capturable — expression power on par with Rust.
4. **Compiler intelligence**: ref auto-selects Rc/Arc, caller side auto-selects borrow.
5. **Deterministic**: ref keeps alive — never silently weakens.
6. **High performance**: Move zero-copy, tokens zero-cost (zero-sized types, disappear after compilation).
7. **Flexible**: `unsafe + *T` supports system-level programming.

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers; return-reference function signatures reflect extra generic parameters.
2. **ref runtime overhead**: Atomic operations have a cost (but this is the inevitable cost of sharing).
3. **`unsafe` risk**: Users must guarantee correctness.
4. **Cross-task cycle is lint, not compile error**: Unlike Rust which compile-errors by default, default is warn, requires team to configure deny to use as a quality gate.

---

## Alternatives

| Alternative | Why Not Chosen |
|-------------|----------------|
| GC | Has runtime overhead, unpredictable pauses |
| Rust borrow checker | Requires lifetime `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointer | Cannot do system-level programming |
| Expose Rc/Arc to users | Dumps implementation details on users, increases cognitive load |
| Bare-bones borrow (v8) | No-escape policy sacrifices key expression abilities like closure capture, returning references |

---

## Design Decision Records

| Decision | Resolution | Reason | Date |
|----------|------------|--------|------|
| **Default value** | Move (zero-copy) | High performance, zero overhead | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimizes | Simple for users, compiler responsible | 2025-01-15 |
| **Borrow** | `&T`/`&mut T` as zero-sized token types | Type properties (Dup/Linear) derive permissions naturally, unified type system | 2025-01-15 |
| **Borrow token** | Replaces bare-bones borrow, `&T` is Dup, `&mut T` is Linear | Eliminates "no-escape" special rules, supports closure capture / return reference / struct storage | 2026-05-29 |
| **Copy** | `clone()` | Explicit semantics | 2025-01-15 |
| **System-level** | `*T` + `unsafe` | Supports system programming | 2025-01-15 |
| **Lifetime** | Not implemented | Token is a value, lifetime managed uniformly by Move/RAII, borrow reduced to ownership | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, invisible to users | Lower cognitive load | 2025-01-15 |
| **Cyclic reference** | In-task unchecked, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint configurable to deny | 2025-01-16 |
| **Weak** | Provided by standard library | Advanced user explicit choice | 2025-01-16 |
| **Consumption analysis** | Removed | Mini borrow checker not needed | 2026-05-11 |
| **Ownership return-flow** | Removed | `(T) -> T` signature is self-documenting | 2026-05-11 |
| **Empty state reuse** | Removed (as a feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function / partial consumption / field three-layer mutability** | Removed | Over-engineered | 2026-05-11 |
| **Lambda no implicit capture** | Lambda uses only explicit parameters, no implicit capture of outer variables | Explicit philosophy, simplifies compiler | 2026-06-16 |

### Version History

| Version | Major Changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| **v8** | **Removed over-engineering (inverse function / partial consumption / field three-layer mutability / consumption analysis / ownership return-flow / empty state reuse), added bare-bones borrow &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces bare-bones borrow, unified type system; token conflict detection corrected to Hoare proposition, see RFC-009a** | **2026-06-13** |

### Pending Issues

| Issue | Description | Status |
|-------|-------------|--------|
| Drop syntax | Whether explicit `drop()` function is needed | To be discussed |
| Escape analysis algorithm | Implementation of ref's cross-task detection | To be discussed |
| Token conflict detection | Hoare logic proposition, see below | ✅ Resolved (see RFC-009a for details) |

### Token Conflict Detection: Hoare Logic Proposition

The complete solution for token conflict detection is in [RFC-009a: Token Lifetime Analysis — Based on Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md). Core points:

**Token liveness is a Hoare logic proposition.** `{all conflicting ReadTokens are dead} write(data) {WriteToken safely obtained}` — shares the proof pipeline from RFC-027 with type checking and user predicate verification. The compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), and the pipeline returns Proved / Disproved / Unproven.

**Borrow checking has not disappeared — it has been reduced.** `BorrowChecker` becomes `BorrowPredicateEmitter`, generating propositions instead of performing checks. This parallels the "type checker" concept exactly: the type checker generates type equality propositions, the borrow proposition generator generates borrow propositions, and the same pipeline verifies them.

**Brand ID (`#42`) is `'a`.** The information is identical, only the encoding differs. `'a` is visible in the type signature, `#42` lives in compiler internals. No new analysis is invented — lifetimes are reduced from the type layer to the proof layer.

**Algorithm summary** (see RFC-009a for details):
- Brand tree prefix matching → identify conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumer, break back edges, structural analysis covers 95%+ of cases (fast path)
- SMT logic cut → only invoked for while + path conditions (slow path, extremely rare)

---

## References

### YaoXiang Official Documents

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

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission review |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes official design document |
| **Rejected** | `docs/design/rfc/` | Retained in RFC directory |