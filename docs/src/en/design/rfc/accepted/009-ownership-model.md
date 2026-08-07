---
title: 'RFC-009: Ownership Model Design'
status: 'Accepted'
author: '晨煦'
created: '2025-01-08'
updated:
  '2026-06-13 (Token conflict detection corrected to Hoare propositions, body synchronized with
  RFC-009a)'
issue: '#126'
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core Design — Five Concepts, One Gradient**:

```
Take a look/Edit in place    Take away        Share             Clone         System level
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         Zero-copy      Compiler auto  Explicit deep   *T
  Zero-size      Default        picks Rc/Arc   copy            User responsible
  token, type
  attribute
  derives
  permission
  naturally
```

- **Move (default)**: Assignment / argument passing / return = ownership transfer, zero-copy, RAII
  auto-release
- **`&T` / `&mut T` (Borrow Tokens)**: Zero-sized compile-time token types. `&T` is duplicable
  (shared read), `&mut T` is linear (exclusive mutation). Permissions are derived naturally from
  type attributes — no special rules needed. Can be returned, stored in structs.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically picks Rc (within task) or Arc
  (across tasks)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, system-level escape hatch

**Eliminated Complexity**:

- ❌ No lifetime `'a`
- ❌ No independent borrow checking framework (borrow conflict is reduced to a Hoare proposition,
  sharing the proof pipeline with type checking)
- ❌ No GC
- ❌ No special rules like "no escape" (tokens are ordinary types, scope is handled uniformly by the
  type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler picks automatically)

> **Programming burden**: `&T` is duplicable, `&mut T` is not — two type attributes, zero special
> rules, fully automatic by the compiler. **Performance guarantee**: Move is zero-cost, tokens are
> zero-cost (zero-sized types, disappear after compilation), ref is pay-as-you-go, no GC pauses.

## Motivation

### Why do we need an ownership model?

| Language     | Memory Management        | Problem                                               |
| ------------ | ------------------------ | ----------------------------------------------------- |
| C/C++        | Manual management        | Memory leaks, dangling pointers, double-free          |
| Java/Python  | GC                       | Latency jitter, memory overhead, unpredictable pauses |
| Rust         | Ownership + borrow check | Steep learning curve for lifetime `'a`                |
| **YaoXiang** | **Move + Token + ref**   | **Simple, deterministic, no GC**                      |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p cannot be read again

# 2. &T / &mut T borrow tokens (zero-cost, type attributes derive permissions naturally)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = sharing (compiler auto-picks Rc/Arc)
shared = ref p2                # Cross-scope holding
spawn { use(shared) }          # Compiler: across task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, exclusive

# 5. unsafe + *T = system level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature           | Rust                                       | YaoXiang                                                                                       |
| ----------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)**                                                            |
| Borrow            | `&T`/`&mut T`, returnable, needs lifetime  | **`&T`/`&mut T` zero-size tokens, Dup/Linear type attributes derive naturally**                |
| Sharing mechanism | `Arc::new()` + manual Weak                 | **`ref` keyword (compiler auto-picks Rc/Arc)**                                                 |
| Copy              | `clone()`                                  | `clone()`                                                                                      |
| Raw pointer       | `*T`                                       | `*T`                                                                                           |
| Lifetime          | `'a`                                       | ❌ None                                                                                        |
| Borrow check      | Global inference                           | **Type checker auto-generates borrow propositions, shared proof pipeline validates uniformly** |
| Cycle reference   | Manual Weak                                | **Unified release on task end / cross-task lint / std lib Weak**                               |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: Assignment / argument passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p cannot be read again

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p rebound, type must be consistent

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
- After move, the original binding cannot be read (compile error)
- RAII: auto-release when scope ends
- Function signature `(T) -> T` is its own documentation — consume T, return T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not
"references" but "type-level proofs of access permission."**

#### 2.1 Two Type Attributes

```
&T      →  Zero-sized, freezes source data (WriteToken forbidden while ReadToken is alive),
          multiple read views safe under freeze guarantee → Duplicable (Dup)
&mut T  →  Zero-sized, exclusive read/write (any other token forbidden while WriteToken is alive),
          copying under exclusive access is meaningless → Linear (non-Dup)
```

**The causal relationship cannot be reversed: freezing is the cause, Dup is the result.** It's not
that `&T` implements Dup so multiple can coexist — it's that the data is frozen (no mutation
possible), so multiple read views are safe, so Dup is implementable. If you treat Dup as the
definition and conflict checking as "an extra patch," the design is wrong.

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

# Caller side: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, previous token was released when shift call ended

# Free functions are the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "Escape Prevention" Is Not Needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — only as parameter, cannot return, cannot
store in struct. This was patching the "borrow" concept.

The token system doesn't need these rules. Tokens are **ordinary types** that follow the same scope
rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Token propagates with return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Child token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Storing in struct — naturally supported**:

```yaoxiang
# ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds read-only view of target
}

# view's token is derived from target, Window owns both
# As long as Window exists, view token is valid
```

#### 2.3 Closures and Lambda Explicit Parameters

Lambda is a function value — it can be returned, stored, and passed out of the current scope.
Therefore Lambda **does not implicitly capture outer local variables**. When outer data is needed,
pass it in as explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not affected by this rule — spawn is an immediately executed concurrent block, parent task blocks and waits
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x is not in scope

# ✅ Correct way: explicit parameter passing
f = (x) => { x + 1 }
f(x)
```

**`spawn { }` is not a function value.** The block marked with spawn, like an if/while body,
executes immediately and completes while the parent stack frame is still alive. The spawn body can
normally access outer variables.

**Across tasks — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task boundary
}

# This is not a special rule — token is a compile-time permission proof, use ref for cross-task sharing
# If you need cross-task sharing, use ref
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Token is a permission proof, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token Lifetime

The token's lifetime is determined by **ordinary scope rules**, no lifetime parameters needed:

- Token in function parameter: alive during the call, released after the call ends
- Returned token: ownership transferred to the caller
- Token stored in struct: lives together with the struct

The compiler doesn't need `'a` annotations because tokens are **values**, and value lifetimes are
uniformly managed by the ownership system (Move/RAII). **Reduce the borrow problem to an ownership
problem.**

#### 2.5 Token Conflict Detection

Token conflict detection is a **Hoare logic proposition**, not an independent flow-sensitive
analysis.

```
{All conflicting ReadTokens dead} write(data) {WriteToken safely acquired}
```

It shares the proof pipeline from RFC-027 with type checking and user predicate verification. The
compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`) and feeds them into the pipeline for validation. The pipeline returns Proved /
Disproved / Unproven.

```yaoxiang
# ❌ &mut token is linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p cannot be read again
    p.x = 10.0                      # ❌ Compile error: WriteToken has been moved
}

# ✅ &T token is Dup type, freely copyable
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read tokens coexist
}
```

**Borrow checking hasn't disappeared — it's been reduced.** The existing `BorrowChecker` becomes
`BorrowPredicateEmitter` (proposition generator), and the generated borrow propositions share the
same proof pipeline with other type propositions. This parallels the type checker concept exactly:
the type checker generates type equality propositions, the borrow proposition generator generates
borrow propositions, and the same pipeline validates them. See
[RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler Internals: Brand Mechanism

Users never see brands. The compiler internally assigns a compile-time unique identifier to each
token:

```
User sees            Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand purposes:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Association tracking**: When deriving `&Float` from `&Point` (field access), the `&Float`
  carries a derived brand (`#N.field_x`), which the compiler can trace back to the parent token
- **Conflict detection**: Co-sourced `WriteToken` and derived `ReadToken` cannot be simultaneously
  active

Brands completely disappear after monomorphization and inlining; they don't exist in the generated
machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

On the caller side, the compiler auto-selects by the following priority:

```
1. If the argument is used again afterwards → prefer creating a token (&T or &mut T, based on method signature)
2. If the argument is not used afterwards → Move
3. Priority order: &T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p not used afterwards
```

#### 2.9 Comparison with RFC-009 v8 Bare-Bones Borrow

| Feature               | Bare-bones Borrow (v8)                | Borrow Token (v9)                                    |
| --------------------- | ------------------------------------- | ---------------------------------------------------- |
| Returning reference   | ❌ Hardcoded forbidden                | ✅ Token propagates with return value                |
| Storing in struct     | ❌ Hardcoded forbidden                | ✅ Token as struct field                             |
| Lambda explicit param | ❌ Hardcoded forbidden                | ✅ Lambda uses explicit parameter                    |
| Special rules         | 3 (param only / no return / no store) | 0 — type attributes derive naturally                 |
| Borrow check          | Dedicated cross-borrow check          | Type checker flow-sensitive liveness analysis        |
| Lifetime annotation   | Not needed                            | Not needed                                           |
| Runtime overhead      | Zero                                  | Zero (zero-sized type, disappears after compile)     |
| Error message         | "Borrow cannot escape"                | "WriteToken(#3) has been moved" (regular type error) |
| User mental model     | Understand "borrow" special status    | `&T` is duplicable, `&mut T` is not                  |

---

### 3. ref Keyword (Compiler-Automated Optimization)

`ref` is the only way to share across scopes. Whether the underlying is Rc or Arc, the user doesn't
need to care.

#### 3.1 Basic Usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Share, compiler auto-picks implementation

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

**User mental model**: `ref` = shared holding. That's enough.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic ref count, low overhead)
Escapes to other tasks         → Arc (atomic ref count, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycle → Silently allowed.
  ├── Each task has a clear lifecycle boundary — when the task ends, all resources (including ref cycles) are released uniformly.
  ├── Long-running services should create sub-tasks per request/connection — sub-tasks auto-recycle on end, no accumulated leak.
  ├── ref always keeps alive, semantics not watered down.
  └── User has the right to build bidirectional strong references within a task (e.g., graph computation intermediate state).

Cross-task cycle → lint (default warn, configurable).
  ├── Program behavior is correct, no real leak (sub-task resources fully released when parent task ends).
  ├── But cross-task strong reference means blurred ownership boundaries, worth pausing to reconsider.
  ├── Default warn level, compile passes with hint.
  └── Teams can set deny in project config, integrate into CI quality gate.
```

**Lint Levels** (similar to Rust clippy):

| Level            | Behavior                         | Scenario                          |
| ---------------- | -------------------------------- | --------------------------------- |
| `allow`          | Don't check                      | Personal project                  |
| `warn` (default) | Compile passes, hint             | Development stage                 |
| `deny`           | Compile failure                  | Team CI quality gate              |
| `forbid`         | Compile failure, non-overridable | Organization-level mandatory rule |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong reference
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

| Cycle type           | Behavior            | Reason                                       |
| -------------------- | ------------------- | -------------------------------------------- |
| Intra-task ref cycle | No check            | User's right, released uniformly on task end |
| Cross-task ref cycle | lint (default warn) | Remind to reconsider, configurable to deny   |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.weak

# Advanced users explicitly choose
a.next = ref b
b.prev = std.weak.new(a.next)   # User explicitly controls which direction is weak
```

**`Weak` is not a language built-in, but a standard library type.** For daily use, `ref` is enough.
Advanced users who need fine-grained memory control manually import `Weak`.

> Revised 2026-08-03: Implemented as an independent `std.weak` module (`std.rc` does not exist —
> `ref` is a language keyword, not a module; the module path is unified as `std.weak`, with
> construction/upgrade entry points `std.weak.new` / `std.weak.upgrade`). The original draft's
> envisioned `std.rc.Weak` was not adopted; this revision takes precedence.

#### 3.5 Borrow Token vs ref

|            | `&T` / `&mut T`                                                | `ref`                                        |
| ---------- | -------------------------------------------------------------- | -------------------------------------------- |
| What       | Take a look / edit in place                                    | Shared holding                               |
| Scope      | Follows the token value's scope                                | Cross-scope                                  |
| Cost       | Zero (zero-sized type)                                         | Rc or Arc (compiler picks)                   |
| Escape     | Yes (token propagates with return value/struct/closure)        | Designed to escape                           |
| Cross-task | No (token is compile-time permission proof, cannot cross task) | Yes (compiler auto-picks Arc)                |
| Cycle      | Not involved                                                   | Intra-task silently allowed, cross-task lint |

---

### 4. clone() — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, not affecting each other
```

**When to use**: When you need to keep the original value and Move or sharing isn't suitable.

### 5. unsafe + Raw Pointer (System-Level Programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Limitations**:

- Can only be used inside `unsafe` blocks
- User guarantees no dangling, no use-after-free
- For FFI, memory operations, and other system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow token (zero-cost)   Move (zero-cost)   Share (pay-as-you-go)  Copy
   │                      │                  │                │
  &T duplicable token   Default ownership   ref Rc/Arc      clone()
  &mut T linear token   transfer           Compiler auto   Explicit deep
                       Chained consumption  picks          copy
   │                      │                  │                │
  Token value scope     Within scope       Cross-scope     Anytime
  Returnable /          T -> T return      ref across      Independent copy
  storable in struct                      task → Arc
  Zero-sized,                              ref within
  disappears                               task → Rc
  after compile                            Intra-task cycle silent
                                           Cross-task cycle lint
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
        self                            # Take, modify, give back
    }

    # Returning reference: token propagates with return value
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# Lambda with explicit parameter
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

`Dup` (Duplicable) is a compiler-managed type attribute meaning **shallow copy**: when
assigning/passing arguments, the handle/token is copied while the underlying data is shared. This
forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creating
independent copies).

**Dup and Clone are orthogonal concepts** — Dup copies the handle sharing data, Clone creates an
independent copy. A type can support both Dup and Clone, or only one of them.

| Type          | Dup                                                            | Clone | Description                                 |
| ------------- | -------------------------------------------------------------- | ----- | ------------------------------------------- |
| `&T`          | ✅ (Copy token, multiple views point to same data)             | ✅    | Read-only token                             |
| `ref T`       | ✅ (ref count +1, share heap data)                             | ✅    | Shared holding (compiler auto-picks Rc/Arc) |
| String, Bytes | ✅ (internal ref count, copy handle sharing underlying buffer) | ✅    | String/byte                                 |
| `&mut T`      | ❌ (linear, exclusive)                                         | ❌    | Mutable token                               |
| `*T`          | ❌                                                             | ❌    | Raw pointer                                 |
| struct        | Derived (auto-derived when all fields are Dup)                 | ✅    | Struct                                      |

**Primitive value types** (Int, Float, Bool, Char) have assignment behavior that is the compiler's
built-in value copy — the two values are completely independent, not shallow copy. They don't belong
to the Dup type attribute but are handled natively by the compiler.

---

## Performance Analysis

| Operation           | Cost           | Description                                                      |
| ------------------- | -------------- | ---------------------------------------------------------------- |
| Move                | Zero           | Pointer move                                                     |
| `&T` / `&mut T`     | Zero           | Zero-sized type, disappears after compile, zero runtime overhead |
| `ref` (within task) | Low            | Compiles to Rc, non-atomic operations                            |
| `ref` (across task) | Medium         | Compiles to Arc, atomic operations                               |
| `clone()`           | Type-dependent | Fast for small objects, slow for large                           |
| `unsafe + *T`       | Zero           | Direct memory operations                                         |

### Comparison

| Language     | Sharing mechanism          | Memory management  | Cycle handling                                             | Complexity |
| ------------ | -------------------------- | ------------------ | ---------------------------------------------------------- | ---------- |
| Rust         | Arc / Mutex + borrow check | Compile-time check | Manual Weak                                                | High       |
| Go           | chan / pointer             | GC                 | GC                                                         | Low        |
| C++          | shared_ptr                 | RAII               | weak_ptr                                                   | Medium     |
| **YaoXiang** | **ref + borrow token**     | **RAII**           | **Task boundary release / cross-task lint / std lib Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent
   with RFC-010's `name: type = value`
2. **Simple**: No lifetime, borrow check reduced to type system propositions. `&T` is duplicable,
   `&mut T` is not — two type attributes
3. **Powerful**: Returnable references, storable in structs, closure capture — expressiveness on par
   with Rust
4. **Compiler smart**: ref auto-picks Rc/Arc, caller side auto-selects borrow
5. **Deterministic**: ref means keep-alive, never silently becomes weak reference
6. **High performance**: Move zero-copy, tokens zero-cost (zero-sized types, disappear after
   compile)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, returnable reference
   function signatures will reflect extra generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the inevitable cost of
   sharing)
3. **unsafe risk**: User must guarantee correctness
4. **Cross-task cycle is lint, not compile error**: Unlike Rust, which compile-errors; default warn
   requires team configuration of deny to be a quality gate

---

## Alternatives

| Alternative            | Why Not Chosen                                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------------------------------- |
| GC                     | Runtime overhead, unpredictable pauses                                                                        |
| Rust borrow checker    | Requires lifetime `'a`, steep learning curve                                                                  |
| Pure Move              | Cannot handle concurrent sharing                                                                              |
| No raw pointer         | Cannot do system-level programming                                                                            |
| Expose Rc/Arc to user  | Throws implementation details at user, increases cognitive load                                               |
| Bare-bones borrow (v8) | Escape prevention strategy sacrifices key expressive capabilities like closure capture, returnable references |

---

## Design Decision Records

| Decision                                                              | Decision                                                                  | Reason                                                                                                | Date       |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ---------- |
| **Default value**                                                     | Move (zero-copy)                                                          | High performance, zero overhead                                                                       | 2025-01-15 |
| **Sharing mechanism**                                                 | `ref` keyword, compiler auto-optimize                                     | User simple, compiler responsible                                                                     | 2025-01-15 |
| **Borrow**                                                            | `&T`/`&mut T` as zero-size token types                                    | Type attributes (Dup/Linear) naturally derive permissions, unified type system                        | 2025-01-15 |
| **Borrow token**                                                      | Replace bare-bones borrow, `&T` Dup, `&mut T` Linear                      | Eliminate "no escape" special rules, support closure capture / returnable reference / store in struct | 2026-05-29 |
| **Copy**                                                              | `clone()`                                                                 | Explicit semantics                                                                                    | 2025-01-15 |
| **System level**                                                      | `*T` + `unsafe`                                                           | Support system programming                                                                            | 2025-01-15 |
| **Lifetime**                                                          | Not implemented                                                           | Token is a value, lifetime managed uniformly by Move/RAII, reduce borrow to ownership problem         | 2025-01-15 |
| **Rc/Arc**                                                            | Compiler auto-selects, not visible to user                                | Reduce cognitive load                                                                                 | 2025-01-15 |
| **Cycle reference**                                                   | Intra-task not checked, cross-task lint (default warn)                    | Structured concurrency naturally guarantees, lint can be configured to deny                           | 2025-01-16 |
| **Weak**                                                              | Provided by standard library                                              | Advanced users explicitly choose                                                                      | 2025-01-16 |
| **Consumption analysis**                                              | Deleted                                                                   | Mini borrow checker not needed                                                                        | 2026-05-11 |
| **Ownership return**                                                  | Deleted                                                                   | `(T) -> T` signature is its own documentation                                                         | 2026-05-11 |
| **Empty state reuse**                                                 | Deleted (as a feature)                                                    | Reassigning after Move is natural behavior                                                            | 2026-05-11 |
| **Inverse function/partial consumption/three-layer field mutability** | Deleted                                                                   | Over-engineering                                                                                      | 2026-05-11 |
| **Lambda non-implicit capture**                                       | Lambda uses only explicit parameters, not implicit outer variable capture | Explicit philosophy, simplifies compiler                                                              | 2026-06-16 |

### Version History

| Version | Major Changes                                                                                                                                                                               | Date           |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| v1      | Initial draft: based on Rust ownership model                                                                                                                                                | 2025-01-08     |
| **v8**  | **Deleted over-engineering (inverse function/partial consumption/three-layer field mutability/consumption analysis/ownership return/empty state reuse), added bare-bones borrow &T/&mut T** | **2026-05-11** |
| **v9**  | **Borrow token system replaces bare-bones borrow, unified type system; token conflict detection corrected to Hoare propositions, see RFC-009a**                                             | **2026-06-13** |

### Pending Issues

| Issue                     | Description                                  | Status                                 |
| ------------------------- | -------------------------------------------- | -------------------------------------- |
| Drop syntax               | Whether explicit `drop()` function is needed | To be discussed                        |
| Escape analysis algorithm | ref's cross-task detection implementation    | To be discussed                        |
| Token conflict detection  | Hoare logic proposition, see below           | ✅ Resolved (see RFC-009a for details) |

### Token Conflict Detection: Hoare Logic Proposition

The complete scheme for token conflict detection is in
[RFC-009a: Token Lifetime Analysis — Based on Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md).
Core points:

**Token liveness is a Hoare logic proposition.**
`{All conflicting ReadTokens dead} write(data) {WriteToken safely acquired}` — it shares the proof
pipeline from RFC-027 with type checking and user predicate verification. The compiler
auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`), and the pipeline returns Proved / Disproved / Unproven.

**Borrow checking hasn't disappeared — it's been reduced.** `BorrowChecker` becomes
`BorrowPredicateEmitter`, generating propositions rather than performing checks. This exactly
parallels the "type checker" concept: the type checker generates type equality propositions, the
borrow proposition generator generates borrow propositions, and the same pipeline validates them.

**Brand ID (`#42`) is `'a`.** The information is completely the same, the encoding is different.
`'a` is visible in the type signature, `#42` is internal to the compiler. No new analysis invented —
lifetimes are reduced from the type layer to the proof layer.

**Algorithm summary** (see RFC-009a for details):

- Brand tree prefix matching → identify conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → starting from consumer, break cuts back edges, structural analysis covers 95%+
  scenarios (fast path)
- SMT logic cut → called only when while + path conditions (slow path, extremely rare)

---

## References

### YaoXiang Official Documentation

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
| **Accepted**     | `docs/design/accepted/` | Becomes formal design document             |
| **Rejected**     | `docs/design/rfc/`      | Preserved in the RFC directory             |
