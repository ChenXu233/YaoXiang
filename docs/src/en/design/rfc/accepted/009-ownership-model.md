---
title: 'RFC-009: Ownership Model Design'
status: 'Accepted'
author: 'Chenxu'
created: '2025-01-08'
updated:
  '2026-06-13 (Token conflict detection corrected to Hoare proposition, body synchronized with
  RFC-009a)'
issue: '#126'
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core Design — Five Concepts, One Gradient**:

```
Look/Modify In-Place   Take Away        Shared Holding       Clone Once        System-Level
       │                  │                  │                  │                  │
      &T                Move              ref               clone()           unsafe
     &mut T            Zero-Copy       Compiler Auto-       Explicit         *T
    Zero-Size                              Select            Deep Copy
    Token             Default         Rc/Arc             User Responsible
    Type-Property
    Inferred Rights
```

- **Move (default)**: Assignment / parameter passing / return = ownership transfer, zero-copy,
  automatic RAII release
- **`&T` / `&mut T` (Borrow Tokens)**: Zero-sized compile-time token types. `&T` is duplicable
  (shared read), `&mut T` is linear (exclusive write). Rights are naturally inferred from type
  properties — no special rules needed. Can be returned, stored in structs.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically selects Rc (no cross-task) or Arc
  (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, system-level escape hatch

**Complexity Eliminated**:

- ❌ No lifetime `'a`
- ❌ No independent borrow checking framework (borrow conflict is reduced to a Hoare proposition,
  sharing the proof pipeline with type checking)
- ❌ No GC
- ❌ No "no escape" special rules (tokens are ordinary types, scope is handled uniformly by the type
  system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler selects automatically)

> **Programming Burden**: `&T` is duplicable, `&mut T` is not — two type properties, zero special
> rules, fully automated by the compiler. **Performance Guarantee**: Move is zero-cost, tokens are
> zero-cost (zero-sized types, disappear after compilation), `ref` is pay-as-you-go, no GC pauses.

## Motivation

### Why is an Ownership Model Needed?

| Language     | Memory Management        | Problems                                              |
| ------------ | ------------------------ | ----------------------------------------------------- |
| C/C++        | Manual management        | Memory leaks, dangling pointers, double free          |
| Java/Python  | GC                       | Latency jitter, memory overhead, unpredictable pauses |
| Rust         | Ownership + borrow check | Steep learning curve for lifetime `'a`                |
| **YaoXiang** | **Move + Token + ref**   | **Simple, deterministic, no GC**                      |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T borrow tokens (zero-cost, rights naturally inferred from type properties)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = shared (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope holding
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, independent

# 5. unsafe + *T = system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Key Differences from Rust

| Feature           | Rust                                       | YaoXiang                                                                                      |
| ----------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------- |
| Default Semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)**                                                           |
| Borrow            | `&T`/`&mut T`, returnable, needs lifetime  | **`&T`/`&mut T` zero-size tokens, rights naturally inferred from Dup/Linear type properties** |
| Sharing           | `Arc::new()` + manual Weak                 | **`ref` keyword (compiler auto-selects Rc/Arc)**                                              |
| Copy              | `clone()`                                  | `clone()`                                                                                     |
| Raw Pointer       | `*T`                                       | `*T`                                                                                          |
| Lifetime          | `'a`                                       | ❌ None                                                                                       |
| Borrow Check      | Global inference                           | **Type checker auto-generates borrow propositions, unified proof pipeline for verification**  |
| Cyclic Refs       | Manual Weak                                | **Unified release on task end / cross-task lint / standard library Weak**                     |

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
- After move, the original binding cannot be read (compile error)
- RAII: automatic release when scope ends
- Function signature `(T) -> T` is its own documentation — consume T, return T

---

### 2. `&T` / `&mut T` (Borrow Tokens)

**Core Principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not
"references" but "type-level proofs of access rights."**

#### 2.1 Two Type Properties

```
&T      →  Zero-sized, freezes source data (WriteToken forbidden while ReadToken lives),
          multiple read views are safe under freeze guarantee → Duplicable (Dup)
&mut T  →  Zero-sized, exclusive read-write (any other token forbidden while WriteToken lives),
          copying is meaningless under exclusive access → Linear (not Dup)
```

**Causality cannot be reversed: freeze is the cause, Dup is the effect.** It is not because `&T`
implements Dup that multiple can coexist — it is because the data is frozen (no mutation possible),
so multiple read views are safe, and thus Dup can be implemented. If you treat Dup as the definition
and conflict checking as an "extra patch," the design is wrong.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter types, determine required rights
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point token grants read right
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point token grants write right
    self.y = self.y + dy
}

# Caller side: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, previous token released at end of shift call

# Free functions are the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escape" Is Not Needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — they can only be parameters, cannot be
returned, cannot be stored in structs. This is patching up the concept of "borrow."

The token system does not need these rules. Tokens are **ordinary types**, following the same scope
rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Tokens propagate with the return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # child token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # token returned to caller
print(px_ref)               # OK, token still in scope
```

**Storing in structs — naturally supported**:

```yaoxiang
# ✅ Structs carry tokens as fields
Window: Type = {
    target: Point,
    view: &Point,      # token field — holds a read-only view of target
}

# view's token derives from target; Window owns both
# As long as Window exists, the view token is valid
```

#### 2.3 Closures and Lambdas: Explicit Parameters

A Lambda is a function value — it can be returned, stored, and passed out of the current scope.
Therefore a Lambda **does not implicitly capture outer local variables**. When outer data is needed,
use explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not subject to this rule — spawn is an immediately-executed concurrent block, parent task blocks and waits
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x is not in scope

# ✅ Correct: explicit parameter
f = (x) => { x + 1 }
f(x)

# ✅ Correct alternative: context frozen at creation point (currying) — closure only takes parameters, no capture
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
evens = list.filter(nums, gt(threshold))
```

> Addendum (2026-08-17): The correct resolution for context dependency is currying freezing, not
> capture. After a closure escapes, the scope at its definition site may be dead, so it must not
> implicitly capture; however, the scope at the call site (creation point) is guaranteed to be
> alive, and freezing the context as a value entering the closure at that point is safe. See SPEC
> §12.3.

**`spawn { }` is not a function value.** A `spawn`-marked block, like an `if`/`while` body, executes
immediately and completes while the parent stack frame is alive. A `spawn` body can normally access
outer variables.

**Cross-task — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task
}

# This is not a special rule — tokens are compile-time right proofs; for cross-task sharing, use ref
# If you need cross-task sharing, use ref
```

**Tokens cannot be `ref`'d**:

```yaoxiang
# ❌ Tokens are right proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token Lifetimes

Token lifetimes are determined by **ordinary scope rules**, no lifetime parameters needed:

- Tokens in function parameters: live during the call, released after the call ends
- Returned tokens: ownership transferred to caller
- Tokens stored in structs: live with the struct

The compiler doesn't need `'a` annotations, because tokens are **values**, and value lifetimes are
managed uniformly by the ownership system (Move/RAII). **Reducing the borrow problem to an ownership
problem.**

#### 2.5 Token Conflict Detection

Token conflict detection is a **Hoare logic proposition**, not an independent flow-sensitive
analysis.

```
{all conflicting ReadTokens dead} write(data) {WriteToken safely acquired}
```

It shares the proof pipeline of RFC-027 with type checking and user predicate verification. The
compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`) and feeds them into the pipeline for verification. The pipeline returns Proved /
Disproved / Unproven.

```yaoxiang
# ❌ &mut token is linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # ❌ Compile error: WriteToken has been moved
}

# ✅ &T token is a Dup type, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is a Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking hasn't disappeared — it has been reduced in dimension.** The existing
`BorrowChecker` becomes a `BorrowPredicateEmitter` (proposition generator); the borrow propositions
it generates share the same proof pipeline as other type propositions. This is perfectly parallel to
the type checker concept: the type checker generates type-equality propositions, the borrow
proposition generator generates borrow propositions, and the same pipeline verifies them. See
[RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler Internals: Brand Mechanism

Users never touch brands. The compiler internally assigns each token a compile-time unique
identifier:

```
User sees            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, not constructed out of thin
  air
- **Association tracking**: When deriving `&Float` (field access) from `&Point`, the `&Float`
  carries a derived brand (`#N.field_x`), which the compiler can trace back to the parent token
- **Conflict detection**: Same-source `WriteToken` and derived `ReadToken` cannot be live
  simultaneously

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

The caller-side compiler automatically selects according to the following priority:

```
1. If the argument is used afterwards → prefer creating a token (&T or &mut T, based on method signature)
2. If the argument is not used afterwards → Move
3. Preferred match order: &T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p is no longer used
```

#### 2.9 Comparison with RFC-009 v8 Minimalist Borrow

| Feature               | Minimalist Borrow (v8)                    | Borrow Tokens (v9)                                   |
| --------------------- | ----------------------------------------- | ---------------------------------------------------- |
| Return reference      | ❌ Hardcoded forbidden                    | ✅ Tokens propagate with return value                |
| Store in struct       | ❌ Hardcoded forbidden                    | ✅ Tokens as struct fields                           |
| Lambda explicit param | ❌ Hardcoded forbidden                    | ✅ Lambda uses explicit parameters                   |
| Special rules         | 3 (param only / no return / no store)     | 0 — type properties naturally infer                  |
| Borrow check          | Dedicated cross-borrow check              | Type checker flow-sensitive liveness analysis        |
| Lifetime annotation   | Not needed                                | Not needed                                           |
| Runtime overhead      | Zero                                      | Zero (zero-sized type, disappears after compilation) |
| Error message         | "Borrow cannot escape"                    | "WriteToken(#3) has been moved" (regular type error) |
| User mental model     | Understand the special status of "borrow" | `&T` is duplicable, `&mut T` is not                  |

---

### 3. The `ref` Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether the underlying is Rc or Arc, the user does not
need to care.

#### 3.1 Basic Usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # share, compiler auto-selects implementation

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

**User mental model**: `ref` = shared holding. That's enough.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref's data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks          → Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
In-task cycle → silently allowed.
  ├── Each task has a clear lifetime boundary — when the task ends, all resources (including ref cycles) are released uniformly.
  ├── Long-running services should spawn child tasks per request/connection — child tasks auto-recycle when done, no accumulating leaks.
  ├── `ref` always keeps alive, semantics not diluted.
  └── The user has the right to build bidirectional strong references within a task (e.g., intermediate graph computation state).

Cross-task cycle → lint (default warn, configurable).
  ├── Program behavior is correct, will not actually leak (when parent task ends, all child task resources are released).
  ├── But cross-task strong references mean blurred ownership boundaries — worth pausing to reconsider.
  ├── Default warn level, compilation passes with hints.
  └── Teams can set it to deny in project config, integrating into CI quality gate.
```

**Lint Levels** (similar to Rust clippy):

| Level            | Behavior                           | Scenario                           |
| ---------------- | ---------------------------------- | ---------------------------------- |
| `allow`          | Not checked                        | Personal projects                  |
| `warn` (default) | Compiles, with hints               | Development stage                  |
| `deny`           | Compilation fails                  | Team CI quality gate               |
| `forbid`         | Compilation fails, cannot override | Organization-level mandatory rules |

```yaoxiang
# In-task cycle: silently allowed, bidirectional strong reference
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Uniformly released when task ends.
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

**Project configuration example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # cross-task cycles directly rejected on CI
```

| Cycle Type           | Behavior            | Reason                                       |
| -------------------- | ------------------- | -------------------------------------------- |
| In-task ref cycle    | Not checked         | User's right, uniformly released at task end |
| Cross-task ref cycle | Lint (default warn) | Reminder to reconsider, configurable deny    |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.weak

# Advanced users explicitly choose
a.next = ref b
b.prev = std.weak.new(a.next)   # user explicitly controls which direction is weak
```

**`Weak` is not a language built-in, it is a standard library type.** Daily use of `ref` is enough.
Advanced users who need fine-grained memory control manually introduce `Weak`.

> 2026-08-03 revision: Implemented as a standalone `std.weak` module (`std.rc` does not exist —
> `ref` is a language keyword, not a module; module path unified as `std.weak`, construction/upgrade
> entry points are `std.weak.new` / `std.weak.upgrade`). The initial draft's envisioned
> `std.rc.Weak` was not adopted; this revision prevails.

#### 3.5 Borrow Tokens vs `ref`

|              | `&T` / `&mut T`                                               | `ref`                                     |
| ------------ | ------------------------------------------------------------- | ----------------------------------------- |
| What it does | Look/modify in-place                                          | Shared holding                            |
| Scope        | With the scope of the token value                             | Cross-scope                               |
| Cost         | Zero overhead (zero-sized type)                               | Rc or Arc (compiler selects)              |
| Escape       | Yes (tokens propagate with return/struct/closure)             | Designed to escape                        |
| Cross-task   | No (tokens are compile-time right proofs, cannot cross tasks) | Yes (compiler auto-selects Arc)           |
| Cycle        | Not involved                                                  | In-task silently allowed, cross-task lint |

---

### 4. `clone()` — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # deep copy
# p and p2 are independent, no mutual influence
```

**When to use**: Scenarios where the original value must be retained and neither Move nor sharing is
appropriate.

### 5. `unsafe` + Raw Pointer (System-Level Programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # raw pointer
    (*ptr).x = 0.0                # dereference (user guarantees safety)
    ptr2 = ptr + 1                # pointer arithmetic
}
```

**Restrictions**:

- Can only be used inside `unsafe` blocks
- User guarantees no dangling, no use-after-free
- For FFI, memory operations, and other system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow Tokens (Zero-Cost)   Move (Zero-Cost)     Sharing (Pay-as-You-Go)   Copy
   │                          │                    │                          │
  &T duplicable token      Default ownership    ref Rc/Arc                clone()
  &mut T linear token      transfer            compiler auto-select     explicit deep copy
   │                          │                    │                          │
  Token value scope         Within scope         Cross-scope               Anytime
  Returnable / struct field  T -> T return        ref cross-task → Arc     Independent copy
  Zero-sized, disappears    T -> Void consume    ref not cross-task → Rc
  after compilation         T -> Void consume    In-task cycle silent
                            Zero-sized,         Cross-task cycle lint
                            disappears after    std library Weak escape
                            compilation
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
        self                            # take, modify, return
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

# In-task cycle: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # cycle, uniformly released at task end

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Dup Type Property

`Dup` (Duplicable) is a type property automatically managed by the compiler, meaning **shallow
copy**: on assignment/parameter passing, the handle/token is copied while the underlying data is
shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep
copy, creating an independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies the handle to share data, Clone creates an
independent copy. A type can support both Dup and Clone, or only one of them.

| Type          | Dup                                                             | Clone | Description                                   |
| ------------- | --------------------------------------------------------------- | ----- | --------------------------------------------- |
| `&T`          | ✅ (copy token, multiple views to same data)                    | ✅    | Read-only token                               |
| `ref T`       | ✅ (ref count +1, share heap data)                              | ✅    | Shared holding (compiler auto-selects Rc/Arc) |
| String, Bytes | ✅ (internal ref count, copy handle to share underlying buffer) | ✅    | String / bytes                                |
| `&mut T`      | ❌ (linear, exclusive)                                          | ❌    | Mutable token                                 |
| `*T`          | ❌                                                              | ❌    | Raw pointer                                   |
| struct        | Derived (auto-derived when all fields are Dup)                  | ✅    | Struct                                        |

**Primitive value types** (Int, Float, Bool, Char) have assignment behavior that is the compiler's
built-in value copy — the two values are completely independent, not shallow copies. They do not
belong to the Dup type property, but are the compiler's native handling.

---

## Performance Analysis

| Operation             | Cost           | Description                                                          |
| --------------------- | -------------- | -------------------------------------------------------------------- |
| Move                  | Zero           | Pointer move                                                         |
| `&T` / `&mut T`       | Zero           | Zero-sized type, disappears after compilation, zero runtime overhead |
| `ref` (no cross-task) | Low            | Compiles to Rc, non-atomic operations                                |
| `ref` (cross-task)    | Medium         | Compiles to Arc, atomic operations                                   |
| `clone()`             | Type-dependent | Fast for small objects, slow for large ones                          |
| `unsafe + *T`         | Zero           | Direct memory operations                                             |

### Comparison

| Language     | Sharing Mechanism          | Memory Mgmt        | Cycle Handling                                                 | Complexity |
| ------------ | -------------------------- | ------------------ | -------------------------------------------------------------- | ---------- |
| Rust         | Arc / Mutex + borrow check | Compile-time check | Manual Weak                                                    | High       |
| Go           | chan / pointer             | GC                 | GC                                                             | Low        |
| C++          | shared_ptr                 | RAII               | weak_ptr                                                       | Medium     |
| **YaoXiang** | **ref + borrow tokens**    | **RAII**           | **Task-boundary release / cross-task lint / std library Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent
   with RFC-010's `name: type = value`
2. **Simple**: No lifetime, borrow checking is reduced to type system propositions. `&T` is
   duplicable, `&mut T` is not — two type properties
3. **Powerful**: Can return references, store in structs, capture in closures — expressive
   capability on par with Rust
4. **Compiler intelligence**: `ref` auto-selects Rc/Arc, caller side auto-selects borrow
5. **Deterministic**: `ref` keeps alive, won't silently turn into a weak reference
6. **High performance**: Move is zero-copy, tokens are zero-cost (zero-sized types, disappear after
   compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter propagation**: Tokens carry brand identifiers; return-reference
   function signatures will reflect additional generic parameters
2. **`ref` runtime overhead**: Atomic operations have a cost (but this is the inevitable price of
   sharing)
3. **`unsafe` risk**: User must guarantee correctness
4. **Cross-task cycle is a lint, not a compile error**: Unlike Rust's compile error, default warn
   requires team-configured deny to serve as a quality gate

---

## Alternatives

| Alternative            | Why Not Chosen                                                                                               |
| ---------------------- | ------------------------------------------------------------------------------------------------------------ |
| GC                     | Runtime overhead, unpredictable pauses                                                                       |
| Rust borrow checker    | Requires lifetime `'a`, steep learning curve                                                                 |
| Pure Move              | Cannot handle concurrent sharing                                                                             |
| No raw pointers        | Cannot do system-level programming                                                                           |
| Expose Rc/Arc to user  | Throws implementation details at the user, increases cognitive load                                          |
| Minimalist borrow (v8) | The "no escape" strategy sacrifices critical expressive abilities like closure capture and return references |

---

## Design Decision Records

| Decision                                                                 | Determination                                                                                                                                   | Reason                                                                                                   | Date       |
| ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ---------- |
| **Default value**                                                        | Move (zero-copy)                                                                                                                                | High performance, zero overhead                                                                          | 2025-01-15 |
| **Sharing mechanism**                                                    | `ref` keyword, compiler auto-optimization                                                                                                       | User simplicity, compiler responsibility                                                                 | 2025-01-15 |
| **Borrow**                                                               | `&T`/`&mut T` as zero-sized token types                                                                                                         | Type properties (Dup/Linear) naturally infer rights, unified type system                                 | 2025-01-15 |
| **Borrow tokens**                                                        | Replace minimalist borrow, `&T` Dup, `&mut T` Linear                                                                                            | Eliminate "no escape" special rules, support closure capture / return references / struct storage        | 2026-05-29 |
| **Copy**                                                                 | `clone()`                                                                                                                                       | Explicit semantics                                                                                       | 2025-01-15 |
| **System-level**                                                         | `*T` + `unsafe`                                                                                                                                 | Support system programming                                                                               | 2025-01-15 |
| **Lifetime**                                                             | Not implemented                                                                                                                                 | Tokens are values, lifetimes managed uniformly by Move/RAII, reducing borrow to ownership                | 2025-01-15 |
| **Rc/Arc**                                                               | Compiler auto-selects, invisible to user                                                                                                        | Reduce cognitive load                                                                                    | 2025-01-15 |
| **Cyclic reference**                                                     | No in-task check, cross-task lint (default warn)                                                                                                | Structured concurrency naturally guarantees, lint configurable to deny                                   | 2025-01-16 |
| **Weak**                                                                 | Provided by standard library                                                                                                                    | Explicit choice for advanced users                                                                       | 2025-01-16 |
| **Consumption analysis**                                                 | Removed                                                                                                                                         | Minimalist borrow checker, not needed                                                                    | 2026-05-11 |
| **Ownership return**                                                     | Removed                                                                                                                                         | `(T) -> T` signature is its own documentation                                                            | 2026-05-11 |
| **Empty-state reuse**                                                    | Removed (as a feature)                                                                                                                          | Reassigning after Move is natural behavior                                                               | 2026-05-11 |
| **Inverse function / partial consumption / three-tier field mutability** | Removed                                                                                                                                         | Over-engineering                                                                                         | 2026-05-11 |
| **Lambda no implicit capture**                                           | Lambda only uses explicit parameters, no implicit capture of outer variables; context is frozen via currying at the creation point (SPEC §12.3) | Closure definition site scope may be dead; frozen values at creation point (caller scope alive) are safe | 2026-06-16 |

### Version History

| Version | Main Changes                                                                                                                                                                                         | Date           |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| v1      | Initial draft: based on Rust ownership model                                                                                                                                                         | 2025-01-08     |
| **v8**  | **Removed over-engineering (inverse function / partial consumption / three-tier field mutability / consumption analysis / ownership return / empty-state reuse), added minimalist borrow &T/&mut T** | **2026-05-11** |
| **v9**  | **Borrow token system replaces minimalist borrow, unified type system; token conflict detection corrected to Hoare proposition, see RFC-009a**                                                       | **2026-06-13** |

### Open Issues

| Issue                     | Description                                     | Status                                 |
| ------------------------- | ----------------------------------------------- | -------------------------------------- |
| Drop syntax               | Whether an explicit `drop()` function is needed | To be discussed                        |
| Escape analysis algorithm | `ref`'s cross-task detection implementation     | To be discussed                        |
| Token conflict detection  | Hoare logic proposition, see below              | ✅ Resolved (see RFC-009a for details) |

### Token Conflict Detection: Hoare Logic Propositions

The complete solution for token conflict detection is in
[RFC-009a: Token Lifetime Analysis — Based on Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md).
Core points:

**Token liveness is a Hoare logic proposition.**
`{all conflicting ReadTokens dead} write(data) {WriteToken safely acquired}` — it shares the proof
pipeline of RFC-027 with type checking and user predicate verification. The compiler auto-generates
borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), and
the pipeline returns Proved / Disproved / Unproven.

**Borrow checking hasn't disappeared — it has been reduced in dimension.** `BorrowChecker` becomes
`BorrowPredicateEmitter`, generating propositions rather than performing checks. This is perfectly
parallel to the concept of "type checker": the type checker generates type-equality propositions,
the borrow proposition generator generates borrow propositions, and the same pipeline verifies them.

**Brand ID (`#42`) is `'a`.** The information is exactly the same, only the encoding differs. `'a`
is visible in the type signature, `#42` is internal to the compiler. No new analysis was invented —
lifetime was reduced from the type layer to the proof layer.

**Algorithm Summary** (see RFC-009a for details):

- Brand tree prefix match → identify conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → from consumer, break cuts back-edges, structural analysis covers 95%+ scenarios
  (fast path)
- SMT logic cutting → only invoked for `while` + path conditions (slow path, extremely rare)

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

| Status           | Location                | Description                                |
| ---------------- | ----------------------- | ------------------------------------------ |
| **Draft**        | `docs/design/rfc/`      | Author's draft, awaiting submission review |
| **Under Review** | `docs/design/rfc/`      | Open community discussion and feedback     |
| **Accepted**     | `docs/design/accepted/` | Becomes a formal design document           |
| **Rejected**     | `docs/design/rfc/`      | Retained in the RFC directory              |
