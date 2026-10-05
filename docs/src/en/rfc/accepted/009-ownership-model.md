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

## Summary

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core Design — Five concepts, one gradient**:

```
Peek/Modify in place   Take ownership    Shared holding     Copy once      System level
    │                       │                  │                  │               │
   &T                     Move              ref              clone()         unsafe
  &mut T                 Zero copy        Compiler auto      Explicit deep    *T
  Zero-size token          Default         selects Rc/Arc        copy         User responsible
  Type properties
  naturally derive
  permissions
```

- **Move (default)**: Assignment / pass argument / return = ownership transfer, zero copy, RAII
  auto-release
- **`&T` / `&mut T` (borrow tokens)**: Zero-size compile-time token types. `&T` is duplicable
  (shared read), `&mut T` is linear (exclusive mutable). Permissions are naturally derived from type
  properties, no special rules required. Can be returned, can be stored in structs.
- **`ref` keyword**: Cross-scope sharing. The compiler automatically selects Rc (not across tasks)
  or Arc (across tasks)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, system-level escape hatch

**Eliminated complexity**:

- ❌ No lifetime `'a`
- ❌ No independent borrow checking framework (borrow conflicts reduced to Hoare propositions,
  sharing the proof pipeline with type checking)
- ❌ No GC
- ❌ No "no escape" or other special rules (tokens are ordinary types, scope is handled uniformly by
  the type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler auto-selects)

> **Programming burden**: `&T` is duplicable, `&mut T` is not — two type properties, zero special
> rules, fully automatic compiler. **Performance guarantee**: Move is zero cost, tokens are zero
> cost (zero-size types, disappear after compilation), ref is pay-as-you-go, no GC pauses.

## Motivation

### Why do we need an ownership model?

| Language     | Memory Management        | Problems                                                    |
| ------------ | ------------------------ | ----------------------------------------------------------- |
| C/C++        | Manual management        | Memory leaks, dangling pointers, double free                |
| Java/Python  | GC                       | Latency fluctuations, memory overhead, unpredictable pauses |
| Rust         | Ownership + borrow check | Lifetime `'a` has a steep learning curve                    |
| **YaoXiang** | **Move + Token + ref**   | **Simple, deterministic, no GC**                            |

### Design Goals

```yaoxiang
# 1. Default Move (zero copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T borrow tokens (zero cost, type properties naturally derive permissions)
print_info(p2)                 # Compiler auto-creates &Point token, released when done
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = shared (compiler auto-selects Rc/Arc)
shared = ref p2                # Hold across scopes
spawn { use(shared) }          # Compiler: across tasks -> Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, exclusive

# 5. unsafe + *T = system level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature           | Rust                                         | YaoXiang                                                                             |
| ----------------- | -------------------------------------------- | ------------------------------------------------------------------------------------ |
| Default semantics | Borrow `&T` (requires explicit `.clone()`)   | **Move (value passing, zero copy)**                                                  |
| Borrow            | `&T`/`&mut T`, returnable, requires lifetime | **`&T`/`&mut T` zero-size tokens, Dup/Linear type properties naturally derive**      |
| Sharing mechanism | `Arc::new()` + manual Weak                   | **`ref` keyword (compiler auto-selects Rc/Arc)**                                     |
| Copy              | `clone()`                                    | `clone()`                                                                            |
| Raw pointer       | `*T`                                         | `*T`                                                                                 |
| Lifetime          | `'a`                                         | ❌ None                                                                              |
| Borrow check      | Global inference                             | **Type checker auto-generates borrow propositions, unified proof pipeline verifies** |
| Cyclic reference  | Manual Weak                                  | **Task-end unified release / cross-task lint / stdlib Weak**                         |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: assignment / pass argument / return = Move, zero copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p can no longer be read

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
    p                            # Move return, zero copy
}
```

**Features**:

- Zero copy (compiler moves pointers)
- Original binding unreadable after move (compile error)
- RAII: auto-released when scope ends
- Function signature `(T) -> T` is itself the documentation — consume T, return T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-size compile-time token types. They are not
"references" but "type-level proofs of access permission".**

#### 2.1 Two Type Properties

```
&T      ->  Zero size, freezes source data (WriteToken forbidden while ReadToken is alive),
          Under the freeze guarantee, multiple read-only views are safe -> Duplicable (Dup)
&mut T  ->  Zero size, exclusive read/write (any other token forbidden while WriteToken is alive),
          Under exclusive access, copying is meaningless -> Linear (non-Dup)
```

**The causal relationship cannot be reversed: freezing is the cause, Dup is the result.** It is not
that `&T` can coexist because it implements Dup — it is that the data is frozen (no mutation
possible), so multiple read-only views are safe, and Dup can be implemented. If you treat Dup as the
definition and conflict checking as an "extra patch", the design is wrong.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter type, which determines the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point token grants write permission
    self.y = self.y + dy
}

# Caller side: compiler automatically chooses borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, the previous token was released when shift() returned

# Free functions work the same way
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escape" Is Not Needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — they can only be parameters, cannot be
returned, and cannot be stored in structs. This was patching the "borrow" concept.

The token system doesn't need these rules. Tokens are **ordinary types** that follow the same scope
rules as every other type.

**Returning references — naturally supported**:

```yaoxiang
# Tokens propagate together with the return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Child token and parent token are returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to the caller
print(px_ref)               # OK, token is still in scope
```

**Storing in structs — naturally supported**:

```yaoxiang
# Struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds a read-only view of target
}

# view's token is derived from target, Window owns both
# As long as Window exists, the view token is valid
```

#### 2.3 Closures and Explicit Lambda Parameters

A lambda is a function value — it can be returned, stored, and passed out of the current scope.
Therefore, a lambda **does not implicitly capture outer local variables**. When outer data is
needed, use explicit parameters:

```yaoxiang
# Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# spawn { } is not affected by this rule — spawn is an immediately executed concurrent block, the parent task blocks waiting
shared = ref data
spawn { use(shared) }

# Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x is not in scope

# Correct way: explicit parameter passing
f = (x) => { x + 1 }
f(x)

# Second correct way: context is frozen at the creation site (currying) — the closure only takes parameters, it does not capture
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
evens = list.filter(nums, gt(threshold))
```

> Addendum (2026-08-17): The proper solution for context dependencies is currying-based freezing,
> not capture. After a closure escapes, the scope at its definition site may already be dead, so it
> must not implicitly capture; but the call site (creation site) scope is guaranteed to be alive,
> and freezing the context at that point as a value entering the closure is safe. See SPEC §12.3.

**`spawn { }` is not a function value.** A block marked `spawn` is like an `if`/`while` body — it
executes immediately, completing while the parent stack frame is still alive. The spawn body can
freely access outer variables.

**Across tasks — tokens cannot cross threads**:

```yaoxiang
# Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # Compile error: token cannot cross tasks
}

# This is not a special rule — tokens are compile-time permission proofs, use ref for cross-task sharing
# If you need cross-task sharing, use ref
```

**Tokens cannot be `ref`**:

```yaoxiang
# Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # Compile error: &T is not an ownable type
}
```

#### 2.4 Token Lifecycle

The lifecycle of a token is determined by **ordinary scope rules**, no lifetime parameter is needed:

- Tokens in function parameters: live during the call, released when the call ends
- Returned tokens: ownership is transferred to the caller
- Tokens stored in structs: live together with the struct

The compiler does not need `'a` annotations, because tokens are **values**, and the lifecycle of
values is uniformly managed by the ownership system (Move / RAII). **The borrow problem is reduced
to an ownership problem.**

#### 2.5 Token Conflict Detection

Token conflict detection is a **Hoare-logic proposition**, not an independent flow-sensitive
analysis.

```
{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}
```

It shares the RFC-027 proof pipeline with type checking and user predicate verification. The
compiler automatically generates borrow propositions (`borrow_conflict`, `use_after_move`,
`use_after_drop`, `mut_violation`) and feeds them into the pipeline. The pipeline returns Proved /
Disproved / Unproven.

```yaoxiang
# &mut tokens are linear, they cannot be duplicated
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # Compile error: WriteToken has been moved
}

# &T tokens are Dup type, they can be freely duplicated
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking hasn't disappeared — it has been reduced.** The existing `BorrowChecker` becomes a
`BorrowPredicateEmitter` (proposition generator), and the generated borrow propositions share the
same proof pipeline as other type propositions. This is fully parallel to the concept of the type
checker: the type checker generates type-equality propositions, the borrow-proposition generator
generates borrow propositions, and the same pipeline verifies both. See
[RFC-009a](009a-borrow-proof-pipeline.md) for the detailed design.

#### 2.7 Compiler Internals: Brand Mechanism

Users never touch brands. Internally, the compiler assigns a unique compile-time identifier to each
token:

```
User sees              Compiler internal representation
----------------------------------------
&Point         ->  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     ->  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

The purpose of brands:

- **Anti-forgery**: A token can only be obtained from its owner capsule, and cannot be constructed
  out of thin air
- **Association tracking**: When `&Float` is derived from `&Point` (field access), `&Float` carries
  a derived brand (`#N.field_x`), and the compiler can trace it back to the parent token
- **Conflict detection**: Same-origin `WriteToken` and derived `ReadToken` cannot be active
  simultaneously

Brands completely disappear after monomorphization and inlining — they do not exist in the generated
machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

The caller-side compiler automatically chooses in the following priority order:

```
1. If the argument is still used afterwards -> Prefer creating a token (&T or &mut T, per the method signature)
2. If the argument is no longer used afterwards -> Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self -> compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self -> compiler creates &mut Point token
p2 = p           # Move, p is no longer used
```

#### 2.9 Comparison with RFC-009 v8 Minimal Borrow

| Feature                    | Minimal borrow (v8)                          | Borrow tokens (v9)                                    |
| -------------------------- | -------------------------------------------- | ----------------------------------------------------- |
| Return reference           | ❌ Hardcoded forbidden                       | ✅ Token propagates with return value                 |
| Store in struct            | ❌ Hardcoded forbidden                       | ✅ Token as struct field                              |
| Lambda explicit parameters | ❌ Hardcoded forbidden                       | ✅ Lambda uses explicit parameters                    |
| Special rules              | 3 (param-only / no-return / no-store)        | 0 — type properties naturally derive                  |
| Borrow check               | Dedicated cross-borrow check                 | Type checker's flow-sensitive liveness analysis       |
| Lifetime annotation        | Not needed                                   | Not needed                                            |
| Runtime overhead           | Zero                                         | Zero (zero-size type, disappears after compilation)   |
| Error message              | "Borrow cannot escape"                       | "WriteToken(#3) has been moved" (ordinary type error) |
| User mental model          | Understanding the "special" status of borrow | `&T` is duplicable, `&mut T` is not                   |

---

### 3. `ref` Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether the underlying implementation is Rc or Arc,
the user doesn't need to care.

#### 3.1 Basic Usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Share, compiler auto-selects implementation

# Cross-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }           # Compiler: across tasks -> Arc
    spawn { use(data) }           # Compiler: across tasks -> Arc
}

# Single-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    use(data)                     # Compiler: not across tasks -> Rc
}
```

**User mental model**: `ref` = shared ownership. That's enough.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data-flow analysis:

Does not escape to other tasks -> Rc (non-atomic reference counting, low overhead)
Escapes to other tasks          -> Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycle -> silently allowed.
  ├── Every task has a clear lifecycle boundary — when the task ends, all resources (including ref cycles) are released uniformly.
  ├── Long-running services should spawn child tasks per request/connection — child tasks end and are reclaimed automatically, never accumulating leaks.
  ├── ref always keeps things alive, semantics are undiluted.
  └── Users have the right to construct bidirectional strong references within a task (e.g. intermediate states of graph computation).

Cross-task cycle -> lint (default warn, configurable).
  ├── Program behavior is correct, no real leak occurs (when the parent task ends, all child task resources are released).
  ├── But cross-task strong references mean ownership boundaries are blurred — worth stopping to rethink.
  ├── Default warn level, compiles with a hint.
  └── Teams can set it to deny in the project config, integrating it into the CI quality gate.
```

**Lint levels** (similar to Rust clippy):

| Level            | Behavior                                | Scenario                          |
| ---------------- | --------------------------------------- | --------------------------------- |
| `allow`          | No check                                | Personal projects                 |
| `warn` (default) | Compiles, with hint                     | Development stage                 |
| `deny`           | Compilation fails                       | Team CI quality gate              |
| `forbid`         | Compilation fails, cannot be overridden | Organization-level mandatory rule |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong reference
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Released uniformly when the task ends.
}

# Cross-task cycle: lint (default warn)
@block
parent_task: () -> Void = {
    shared_a = ref a
    shared_b = ref b
    spawn {
        shared_a.child = ref shared_b   # Warning: cross-task cyclic reference
    }
}
```

**Project config example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles are rejected directly on CI
```

| Cycle type           | Behavior            | Reason                                       |
| -------------------- | ------------------- | -------------------------------------------- |
| Intra-task ref cycle | Not checked         | User's right, released uniformly at task end |
| Cross-task ref cycle | Lint (default warn) | Reminder to rethink, configurable to deny    |

#### 3.4 Weak: Provided by the Standard Library

```yaoxiang
use std.weak

# Advanced user explicit choice
a.next = ref b
b.prev = std.weak.new(a.next)   # User explicitly controls which direction is weak
```

**`Weak` is not a language built-in, it is a standard library type.** `ref` is enough for daily use.
Advanced users who need fine-grained memory control manually introduce `Weak`.

> Revision 2026-08-03: Implemented as a standalone `std.weak` module (`std.rc` does not exist —
> `ref` is a language keyword, not a module; the module path is uniformly `std.weak`, with
> construction/upgrade entry points `std.weak.new` / `std.weak.upgrade`). The originally envisioned
> `std.rc.Weak` was not implemented; this revision prevails.

#### 3.5 Borrow Tokens vs `ref`

|                 | `&T` / `&mut T`                                                            | `ref`                                        |
| --------------- | -------------------------------------------------------------------------- | -------------------------------------------- |
| What it does    | Peek / modify in place                                                     | Shared ownership                             |
| Scope           | Follows the scope of the token value                                       | Cross-scope                                  |
| Cost            | Zero cost (zero-size type)                                                 | Rc or Arc (compiler-chosen)                  |
| Escape          | Allowed (token propagates through return value / struct / closure)         | Designed for escape                          |
| Across tasks    | Not allowed (token is a compile-time permission proof, cannot cross tasks) | Allowed (compiler auto-selects Arc)          |
| Cycle formation | Not involved                                                               | Intra-task silently allowed, cross-task lint |

---

### 4. `clone()` — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, neither affects the other
```

**When to use**: scenarios where you need to keep the original value and neither Move nor sharing is
suitable.

### 5. `unsafe` + Raw Pointer (System-Level Programming)

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
- Used for FFI, memory operations, and other system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow tokens (zero cost)    Move (zero cost)     Sharing (pay-as-you-go)    Copy
       │                          │                       │                    │
   &T duplicable token        Default ownership       ref Rc/Arc           clone()
   &mut T linear token        transfer               Compiler auto-select   Explicit deep copy
       │                          │                       │                    │
   Token value scope           Within scope            Cross-scope           Anytime
   Returnable / storable       T -> T round-trip       ref across task -> Arc  Independent copy
   Zero size, vanishes         T -> Void consume      ref not across task -> Rc
   after compilation           T -> T round-trip
                               Zero size, vanishes
                               after compilation       Intra-task cycle silent
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

    # Move -> Move: consume and return
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # Take, modify, return
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
p = p.scale(2.0)                    # Move -> return
shared = ref p                      # ref share
spawn { use(shared) }

# clone for independent copy
backup = p.clone()

# Intra-task cycle: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, released uniformly when the task ends

# unsafe system level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Dup Type Property

`Dup` (Duplicable) is a type property automatically managed by the compiler, meaning **shallow
copy**: assigning / passing a value copies the handle / token, and the underlying data is shared.
This forms a three-level gradient together with Move (ownership transfer) and Clone (explicit deep
copy, creating an independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies a handle to share data, Clone creates an
independent copy. A type can support both Dup and Clone simultaneously, or only one of them.

| Type          | Dup                                                                 | Clone | Description                                     |
| ------------- | ------------------------------------------------------------------- | ----- | ----------------------------------------------- |
| `&T`          | ✅ (copy token, multiple views point to the same data)              | ✅    | Read-only token                                 |
| `ref T`       | ✅ (reference count + 1, share heap data)                           | ✅    | Shared ownership (compiler auto-selects Rc/Arc) |
| String, Bytes | ✅ (internal reference count, copy handle shares underlying buffer) | ✅    | String / bytes                                  |
| `&mut T`      | ❌ (linear, exclusive)                                              | ❌    | Mutable token                                   |
| `*T`          | ❌                                                                  | ❌    | Raw pointer                                     |
| struct        | Derived (see "Derivation rules" below, #398)                        | ✅    | Struct                                          |
| tuple         | Derived (element-wise, same as struct rules, #398)                  | ✅    | Tuple                                           |

**Primitive value types** (Int, Float, Bool, Char) have assignment behavior that is a
compiler-built-in value copy — the two values are completely independent, not shallow copies. They
don't belong to the Dup type property; they are the compiler's native handling.

#### Derivation Rules (Finalized in #398)

"Auto-derive when all fields are Dup" cannot be executed literally — primitive fields (Int etc.)
themselves don't belong to Dup, so `{ x: Int, y: Int }` would be misclassified as Move. The
executable form is:

1. **Copyable field set** = ValueCopy (Int / Float / Bool / Char / Range) ∪ Dup (`&T`, `ref T`,
   String / Bytes, function values (#352), composite types that have reached Dup);
2. **struct**: All fields are in the copyable field set -> derive Dup; **any** field is Linear
   (`&mut T`) or Move (nested Move struct / Vec, Dict, etc. containers / resources) -> the whole
   type stays Move (no "partially copyable" intermediate state — under copy semantics it must be
   transferred);
3. **tuple**: Same rule as struct, judged element-wise; the empty tuple (unit) is `Void`;
4. **Derivation is recursive**: when a field is a named type (e.g. `target: Point`), expand its
   definition and judge again — `A = { b: B }` follows B's derivation result; cyclic aliases are
   conservatively classified as Move up to a depth limit;
5. **Not in scope of this entry** (still Move, handled separately): containers (Vec / Dict / Set /
   Option / Result / Array) and enum.

---

## Performance Analysis

| Operation          | Cost           | Description                                                         |
| ------------------ | -------------- | ------------------------------------------------------------------- |
| Move               | Zero           | Pointer move                                                        |
| `&T` / `&mut T`    | Zero           | Zero-size type, disappears after compilation, zero runtime overhead |
| `ref` (intra-task) | Low            | Compiled to Rc, non-atomic operations                               |
| `ref` (cross-task) | Medium         | Compiled to Arc, atomic operations                                  |
| `clone()`          | Type-dependent | Fast for small objects, slow for large ones                         |
| `unsafe + *T`      | Zero           | Direct memory operation                                             |

### Comparison

| Language     | Sharing mechanism          | Memory management  | Cycle handling                                            | Complexity |
| ------------ | -------------------------- | ------------------ | --------------------------------------------------------- | ---------- |
| Rust         | Arc / Mutex + borrow check | Compile-time check | Manual Weak                                               | High       |
| Go           | chan / pointer             | GC                 | GC                                                        | Low        |
| C++          | shared_ptr                 | RAII               | weak_ptr                                                  | Medium     |
| **YaoXiang** | **ref + borrow tokens**    | **RAII**           | **Task boundary release / cross-task lint / stdlib Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T` / `&mut T` are ordinary types, not special language features. Fully consistent
   with RFC-010's `name: type = value`
2. **Simple**: No lifetime, borrow checking reduced to type system propositions. `&T` is duplicable,
   `&mut T` is not — two type properties
3. **Powerful**: Returnable references, storable in structs, closure capture — expressive power on
   par with Rust
4. **Compiler is smart**: `ref` auto-selects Rc/Arc, caller side auto-selects borrow
5. **Deterministic**: `ref` always keeps things alive, never silently weakens the reference
6. **High performance**: Move is zero copy, tokens are zero cost (zero-size type, disappears after
   compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, extra generic parameters
   appear in the signatures of functions that return references
2. **`ref` runtime overhead**: Atomic operations have a cost (but this is the inevitable cost of
   sharing)
3. **`unsafe` risk**: Users must guarantee correctness
4. **Cross-task cycles are lint, not compile errors**: Unlike Rust which errors at compile time,
   default is `warn`, teams need to configure `deny` to use it as a quality gate

---

## Alternatives

| Option                    | Why not chosen                                                                                           |
| ------------------------- | -------------------------------------------------------------------------------------------------------- |
| GC                        | Runtime overhead, unpredictable pauses                                                                   |
| Rust's borrow checker     | Requires lifetime `'a`, steep learning curve                                                             |
| Pure Move                 | Cannot handle concurrent sharing                                                                         |
| No raw pointers           | Cannot do system-level programming                                                                       |
| Expose Rc/Arc to the user | Dumps implementation details on the user, increases cognitive load                                       |
| Minimal borrow (v8)       | "No escape" strategy sacrifices critical expressiveness such as closure capture and returning references |

---

## Design Decision Record

| Decision                                                                   | Decision                                                                                                                                       | Reason                                                                                                                       | Date       |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------- |
| **Default value**                                                          | Move (zero copy)                                                                                                                               | High performance, zero overhead                                                                                              | 2025-01-15 |
| **Sharing mechanism**                                                      | `ref` keyword, compiler auto-optimizes                                                                                                         | Simple for users, compiler is responsible                                                                                    | 2025-01-15 |
| **Borrow**                                                                 | `&T` / `&mut T` as zero-size token types                                                                                                       | Type properties (Dup / Linear) naturally derive permissions, unified type system                                             | 2025-01-15 |
| **Borrow tokens**                                                          | Replaces minimal borrow, `&T` Dup, `&mut T` Linear                                                                                             | Eliminate "no escape" and other special rules, support closure capture / return references / struct storage                  | 2026-05-29 |
| **Copy**                                                                   | `clone()`                                                                                                                                      | Explicit semantics                                                                                                           | 2025-01-15 |
| **System level**                                                           | `*T` + `unsafe`                                                                                                                                | Supports systems programming                                                                                                 | 2025-01-15 |
| **Lifetime**                                                               | Not implemented                                                                                                                                | Tokens are values, lifecycle is uniformly managed by Move / RAII, borrow reduced to ownership                                | 2025-01-15 |
| **Rc / Arc**                                                               | Compiler auto-selects, invisible to user                                                                                                       | Lower cognitive load                                                                                                         | 2025-01-15 |
| **Cyclic reference**                                                       | No check intra-task, lint cross-task (default warn)                                                                                            | Structured concurrency guarantees it, lint can be configured to deny                                                         | 2025-01-16 |
| **Weak**                                                                   | Provided by standard library                                                                                                                   | Advanced users explicitly opt in                                                                                             | 2025-01-16 |
| **Consumption analysis**                                                   | Removed                                                                                                                                        | Mini borrow checker, not needed                                                                                              | 2026-05-11 |
| **Ownership round-trip**                                                   | Removed                                                                                                                                        | `(T) -> T` signature is itself the documentation                                                                             | 2026-05-11 |
| **Empty-state reuse**                                                      | Removed (as a feature)                                                                                                                         | Reassigning after Move is natural behavior                                                                                   | 2026-05-11 |
| **Inverse functions / partial consumption / three-layer field mutability** | Removed                                                                                                                                        | Over-engineering                                                                                                             | 2026-05-11 |
| **Lambda does not implicitly capture**                                     | Lambda only uses explicit parameters, no implicit capture of outer variables; context is frozen via currying at the creation site (SPEC §12.3) | Scope at closure definition site may already be dead; freezing as value at the creation site (call-site scope alive) is safe | 2026-06-16 |

### Version History

| Version | Major change                                                                                                                                                                                                  | Date           |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| v1      | Initial draft: based on Rust's ownership model                                                                                                                                                                | 2025-01-08     |
| **v8**  | **Removed over-engineering (inverse functions / partial consumption / three-layer field mutability / consumption analysis / ownership round-trip / empty-state reuse), added minimal borrow `&T` / `&mut T`** | **2026-05-11** |
| **v9**  | **Borrow token system replaces minimal borrow, unifying the type system; token conflict detection corrected to Hoare propositions, see RFC-009a**                                                             | **2026-06-13** |

### Pending Issues

| Issue                     | Description                                     | Status                                 |
| ------------------------- | ----------------------------------------------- | -------------------------------------- |
| Drop syntax               | Whether an explicit `drop()` function is needed | To be discussed                        |
| Escape analysis algorithm | Cross-task detection implementation for `ref`   | To be discussed                        |
| Token conflict detection  | Hoare-logic proposition, see below              | ✅ Resolved (see RFC-009a for details) |

### Token Conflict Detection: Hoare-Logic Propositions

The complete plan for token conflict detection is in
[RFC-009a: Token Liveness Analysis — Based on the Hoare Proof Pipeline](009a-borrow-proof-pipeline.md).
Core points:

**Token liveness is a Hoare-logic proposition.**
`{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}` — it shares the
RFC-027 proof pipeline with type checking and user predicate verification. The compiler
automatically generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`), and the pipeline returns Proved / Disproved / Unproven.

**Borrow checking hasn't disappeared — it has been reduced.** `BorrowChecker` becomes
`BorrowPredicateEmitter`, generating propositions rather than performing checks. This is fully
parallel to the "type checker" concept: the type checker generates type-equality propositions, the
borrow-proposition generator generates borrow propositions, and the same pipeline verifies both.

**Brand ID (`#42`) is `'a`.** The information is exactly the same, only the encoding differs. `'a`
is visible in type signatures, `#42` is internal to the compiler. No new analysis was invented — the
lifetime was demoted from the type layer to the proof layer.

**Algorithm outline** (details in RFC-009a):

- Brand-tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → starting from the consumer, break cuts back edges, structural analysis covers 95%+
  of scenarios (fast path)
- SMT logic cut → invoked only when there is `while` + path conditions (slow path, extremely rare)

---

## References

### YaoXiang Official Documentation

- [Language Specification](../../reference/language-spec/index.md)
- [Design Manifesto](../../explanation/manifesto.md)
- [RFC-009a: Token Liveness Analysis — Based on the Hoare Proof Pipeline](009a-borrow-proof-pipeline.md)
- [RFC-010 Unified Type Syntax](010-unified-type-syntax.md)
- [tutorial/ Tutorials](../../tutorial/index.md)

### External References

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and Destination

| Status           | Location                | Description                                |
| ---------------- | ----------------------- | ------------------------------------------ |
| **Draft**        | `docs/design/rfc/`      | Author's draft, awaiting review submission |
| **Under review** | `docs/design/rfc/`      | Open community discussion and feedback     |
| **Accepted**     | `docs/design/accepted/` | Becomes official design document           |
| **Rejected**     | `docs/design/rfc/`      | Retained in the RFC directory              |
