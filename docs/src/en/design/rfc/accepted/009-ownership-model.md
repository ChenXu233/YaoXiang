---
title: 'RFC-009: Ownership Model Design'
status: 'Accepted'
author: 'Chen Xu'
created: '2025-01-08'
updated: '2026-06-13 (Token conflict detection corrected to Hoare propositions, body synced with RFC-009a)'
issue: '#126'
---

# RFC-009: Ownership Model Design

## Summary

This document defines the **Ownership Model** for the YaoXiang programming language.

**Core Design—Five Concepts, One Gradient**:

```
Glance/Modify in-place    Take ownership         Shared持有          Clone              System-level
      │                      │                     │                │                     │
     &T                    Move                ref              clone()              unsafe
   &mut T                  Zero-copy         Compiler auto      Explicit           *T
   Zero-size token         Default          picks Rc/Arc       deep copy         User responsible
   Type properties
   naturally derive
   permissions
```

- **Move (Default)**: Assignment/parameter passing/return = ownership transfer, zero-copy, RAII auto-release
- **`&T` / `&mut T` (Borrow Tokens)**: Zero-size compile-time token types. `&T` is duplicable (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are naturally derived from type properties, no special rules needed. Can be returned, can be stored in structs.
- **`ref` keyword**: Cross-scope sharing. Compiler auto-selects Rc (not cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointers, system-level escape hatch

**Complexity Eliminated**:

- ❌ No lifetimes `'a`
- ❌ No separate borrow checking framework (borrow conflict reduced to Hoare propositions, sharing proof pipeline with type checking)
- ❌ No GC
- ❌ No special rules like "forbidden escape" (tokens are ordinary types, scope handled uniformly by type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler auto-selects)

> **Programming Burden**: `&T` is duplicable, `&mut T` is not—two type properties, zero special rules, fully automated by compiler.
> **Performance Guarantee**: Move is zero-cost, tokens are zero-cost (zero-size types, disappear after compilation), ref is pay-for-play, no GC pauses.

## Motivation

### Why Do We Need an Ownership Model?

| Language       | Memory Management           | Problems                                       |
| -------------- | ---------------------------- | ---------------------------------------------- |
| C/C++          | Manual                       | Memory leaks, dangling pointers, double-free   |
| Java/Python    | GC                           | Latency jitter, memory overhead, unpredictable pauses |
| Rust           | Ownership + Borrow Checker   | Lifetime `'a` steep learning curve             |
| **YaoXiang**   | **Move + Token + ref**      | **Simple, deterministic, no GC**               |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p is no longer readable

# 2. &T / &mut T Borrow Tokens (zero-cost, type properties naturally derive permissions)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)            # Compiler auto-creates &mut Point token

# 3. ref = Shared (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope持有
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = Explicit copy
backup = p2.clone()            # Deep copy, owned

# 5. unsafe + *T = System-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature        | Rust                                     | YaoXiang                                                      |
| -------------- | ---------------------------------------- | ------------------------------------------------------------- |
| Default        | Borrow `&T` (explicit `.clone()` needed) | **Move (value semantics, zero-copy)**                         |
| Borrowing      | `&T`/`&mut T`, returnable, lifetimes     | **`&T`/`&mut T` zero-size tokens, Dup/Linear type properties naturally derive** |
| Sharing        | `Arc::new()` + manual Weak              | **`ref` keyword (compiler auto-selects Rc/Arc)**              |
| Copying        | `clone()`                                | `clone()`                                                     |
| Raw pointers   | `*T`                                     | `*T`                                                          |
| Lifetimes      | `'a`                                     | ❌ None                                                       |
| Borrow check   | Global inference                         | **Type checker auto-generates borrow propositions, unified proof pipeline** |
| Cyclic refs    | Manual Weak                              | **Task end unified release / cross-task lint / std Weak**     |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: Assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p is no longer readable

# Variables can be reassigned (Python-style, no shadowing)
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

- Zero-copy (compiler moves pointers)
- Original binding unreadable after move (compile error)
- RAII: auto-release at scope end
- Function signature `(T) -> T` is self-documenting—consumes T, returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core Principle**: `&T` and `&mut T` are zero-size compile-time token types. They are not "references" but "type-level proofs of access permission".**

#### 2.1 Two Type Properties

```
&T      →  Zero-size, freezes source data (ReadToken liveness prohibits WriteToken),
          under freeze guarantee multiple read-only views are safe → Duplicable (Dup)
&mut T  →  Zero-size, exclusive read-write (WriteToken liveness prohibits any other token),
          under exclusive access copying is meaningless → Linear (non-Dup)
```

**Causality cannot be reversed: freeze is the cause, Dup is the result.** It's not that `&T` implements Dup so coexistence is allowed—it's that data is frozen (no mutation possible), multiple read-only views are safe, and Dup can be implemented. If Dup is treated as the definition and conflict checking as "extra patches", the design is wrong.

#### 2.2 Basic Usage

```yaoxiang
# Method side: declare parameter type, determines required permissions
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

# Free functions work the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist—Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "Forbidden Escape" Is Unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T`—can only be parameters, cannot be returned, cannot be stored in structs. This was patching the "borrow" concept.

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scope rules as all other types.

**Returning References—Naturally Supported**:

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

**Storing in Structs—Naturally Supported**:

```yaoxiang
# ✅ Struct carries token as field
Window: Type = {
    target: Point,
    view: &Point,      # Token field—holds read-only view of target
}

# view token derived from target, Window owns both
# As long as Window exists, view token is valid
```

#### 2.3 Closures and Lambda Explicit Parameters

Lambdas are function values—can be returned, stored, passed out of current scope. Therefore Lambda **does not implicitly capture outer local variables**. When outer data is needed, use explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not affected—spawn is immediately executed concurrent block, parent task blocks waiting
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x not in scope

# ✅ Correct way: explicit parameter
f = (x) => { x + 1 }
f(x)
```

**spawn { } is not a function value.** The block marked with spawn, like if/while body, executes immediately and completes while parent stack frame is alive. Spawn body can access outer variables normally.

**Cross-Task—Tokens Cannot Cross Threads**:

```yaoxiang
# ❌ Token cannot cross task boundary
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task boundary
}

# This is not a special rule—token is compile-time permission proof, use ref for cross-task sharing
# If cross-task sharing is needed, use ref
```

**Token Cannot Be ref'd**:

```yaoxiang
# ❌ Token is permission proof, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not ownable
}
```

#### 2.4 Token Lifetime

Token lifetime is determined by **ordinary scope rules**, no lifetime parameters needed:

- Token in function parameter: alive during call, released after call
- Returned token: ownership transferred to caller
- Token stored in struct: alive with struct

Compiler doesn't need `'a` annotation because token is **a value**, value lifetime is uniformly managed by ownership system (Move/RAII). **Reducing borrow problem to ownership problem.**

#### 2.5 Token Conflict Detection

Token conflict detection is **Hoare logic proposition**, not independent flow-sensitive analysis.

```
{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}
```

Shares RFC-027 proof pipeline with type checking and user predicate verification. Compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), pipeline returns Proved / Disproved / Unproven.

```yaoxiang
# ❌ &mut token is linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p is no longer readable
    p.x = 10.0                      # ❌ Compile error: WriteToken already moved
}

# ✅ &T token is Dup type, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking hasn't disappeared—it has been reduced in dimension.** Existing `BorrowChecker` becomes `BorrowPredicateEmitter` (proposition generator), generated borrow propositions share same proof pipeline with other type propositions. This is completely parallel to the type checker: type checker generates type equality propositions, borrow proposition generator generates borrow propositions, same pipeline verifies. See [RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler Internals: Branding Mechanism

Users never see brands. Compiler internally assigns compile-time unique identifier to each token:

```
User-facing         Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is compile-time unique integer
```

Brand uses:

- **Anti-forgery**: Token can only be obtained from owner, cannot be constructed from thin air
- **Provenance tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries derived brand (`#N.field_x`), compiler can trace back to parent token
- **Conflict detection**: Same-origin `WriteToken` and derived `ReadToken` cannot be alive simultaneously

Brands completely disappear after monomorphization and inlining, don't exist in generated machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

Compiler at call site auto-selects by priority:

```
1. If actual argument has further uses → prefer creating token (&T or &mut T, based on method signature)
2. If actual argument has no further uses → Move
3. Preference order: &T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Limited Borrowing

| Feature              | Limited Borrowing (v8)                | Borrow Token (v9)                                |
| -------------------- | -------------------------------------- | ------------------------------------------------- |
| Return reference     | ❌ Hard-coded prohibition              | ✅ Token propagates with return value             |
| Store in struct      | ❌ Hard-coded prohibition              | ✅ Token as struct field                          |
| Lambda explicit param | ❌ Hard-coded prohibition              | ✅ Lambda uses explicit parameters                |
| Special rules        | 3 (params only/no return/no storage)   | 0—type properties naturally derive                |
| Borrow checking      | Dedicated cross borrow checking        | Type checker flow-sensitive liveness analysis     |
| Lifetime annotation  | Not needed                             | Not needed                                        |
| Runtime overhead     | Zero                                   | Zero (zero-size type, disappears after compile)  |
| Error messages       | "Borrow cannot escape"                | "WriteToken(#3) already moved" (regular type error) |
| User mental model    | Understand "borrow" special status    | `&T` is duplicable, `&mut T` is not               |

---

### 3. ref Keyword (Compiler Auto-Optimization)

`ref` is the only way for cross-scope sharing. Whether underlying is Rc or Arc, user doesn't need to care.

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
    use(data)                     # Compiler: not cross-task → Rc
}
```

**User Mental Model**: `ref` = shared持有. That's it.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference count, low overhead)
Escapes to other tasks   → Arc (atomic reference count, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Intra-task cycles → Silently allowed.
  ├── Each task has clear lifecycle boundary—all resources (including ref cycles) released when task ends.
  ├── Long-running services should create subtasks per request/connection—subtask end auto-reclaims, no accumulation.
  ├── ref always keeps alive, semantics are clean.
  └── Users have right to build bidirectional strong references within task (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, no real leak (parent task end releases all child resources).
  ├── But cross-task strong reference means ownership boundary is blurry, worth pausing to reconsider.
  ├── Default warn level, compiles but has hint.
  └── Teams can set to deny in project config,纳入 CI quality gate.
```

**Lint Levels** (like Rust clippy):

| Level             | Behavior                  | Scenario                      |
| ----------------- | ------------------------- | ----------------------------- |
| `allow`           | No checking               | Personal project              |
| `warn` (default)  | Compiles, has hints       | Development phase             |
| `deny`            | Compile failure           | Team CI quality gate          |
| `forbid`          | Compile failure, non-overridable | Organization-level mandatory rule |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong reference
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Released uniformly at task end.
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

**Project Config Example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles rejected on CI
```

| Cycle Type        | Behavior                 | Reason                                      |
| ----------------- | ------------------------ | ------------------------------------------- |
| Intra-task ref cycle | No checking           | User's right, task end unified release      |
| Cross-task ref cycle | lint (default warn)  | Hint to reconsider, configurable deny        |

#### 3.4 Weak: Provided by Standard Library

```yaoxiang
use std.rc.Weak

# Advanced users explicitly choose
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not language built-in, it's std type.** Daily use `ref` is enough. Advanced users who need fine-grained memory control manually introduce `Weak`.

#### 3.5 Borrow Token vs ref

|        | `&T` / `&mut T`                              | `ref`                       |
| ------ | -------------------------------------------- | --------------------------- |
| What   | Glance / modify in-place                     | Shared持有                  |
| Scope  | Follows token value's scope                  | Cross-scope                 |
| Cost   | Zero (zero-size type)                        | Rc or Arc (compiler selects) |
| Escape | Can (token propagates with return/struct/closure) | It's designed to escape       |
| Cross-task | Cannot (token is compile-time permission proof, cannot cross task boundary) | Can (compiler auto-selects Arc) |
| Cycles | Not involved                                 | Intra-task silently allowed, cross-task lint |

---

### 4. clone() — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 independent, don't affect each other
```

**When to use**: When original value needs to be preserved and Move or sharing are not suitable.

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
- User guarantees no dangling, no use after free
- For FFI, memory operations, system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow token (zero-cost)     Move (zero-cost)      Shared (pay-for-play)    Clone
   │                            │                   │                     │
  &T duplicable token        Default ownership     ref Rc/Arc          clone()
  &mut T linear token        transfer            Compiler auto         Explicit
   │                            │                   │                     │
  Token value scope           Chain consume       Cross-scope          Anytime
  Returnable/storable        return             ref cross-task → Arc  Independent
  Zero-size disappears       T -> T return       ref not cross → Rc   copy
  after compile               T -> Void consume    Intra-task cycles silent
                               T -> Void consume   Cross-task cycles lint
                                                     std Weak escape
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
p = p.scale(2.0)                    # Move → return
shared = ref p                      # ref shared
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

`Dup` (Duplicable) is a compiler-managed type property meaning **shallow copy**: on assignment/parameter passing, what is copied is the handle/token, underlying data is shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creates independent copy).

**Dup and Clone are orthogonal concepts**—Dup copies handle sharing data, Clone creates independent copy. A type can support both Dup and Clone, or only one.

| Type            | Dup                                         | Clone | Description                            |
| --------------- | ------------------------------------------- | ----- | -------------------------------------- |
| `&T`            | ✅ (copy token, multiple views point to same data) | ✅    | Read-only token                        |
| `ref T`         | ✅ (ref count +1, shared heap data)        | ✅    | Shared持有 (compiler auto-selects Rc/Arc) |
| String, Bytes   | ✅ (internal ref count, copy handle shares underlying buffer) | ✅    | String/bytes                           |
| `&mut T`        | ❌ (linear, exclusive)                      | ❌    | Mutable token                          |
| `*T`            | ❌                                          | ❌    | Raw pointer                            |
| struct          | Derived (auto-derived when all fields are Dup) | ✅    | Struct                                 |

**Primitive value types** (Int, Float, Bool, Char) have compiler-built-in value copy on assignment—two values are completely independent, not shallow copy. They don't belong to Dup type property but to compiler's native handling.

---

## Performance Analysis

| Operation              | Cost   | Description                                 |
| ---------------------- | ------ | ------------------------------------------- |
| Move                   | Zero   | Pointer move                                |
| `&T` / `&mut T`        | Zero   | Zero-size type, disappears after compile, zero runtime overhead |
| `ref` (not cross-task) | Low    | Compiled to Rc, non-atomic operation        |
| `ref` (cross-task)     | Medium | Compiled to Arc, atomic operation            |
| `clone()`              | Varies | Fast for small objects, slow for large      |
| `unsafe + *T`          | Zero   | Direct memory operation                     |

### Comparison

| Language       | Sharing Mechanism        | Memory Management | Cycle Handling                                   | Complexity |
| -------------- | ------------------------ | ----------------- | ------------------------------------------------ | ---------- |
| Rust           | Arc / Mutex + borrow check | Compile-time check | Manual Weak                                    | High       |
| Go             | chan / pointer           | GC                | GC                                              | Low        |
| C++            | shared_ptr               | RAII              | weak_ptr                                        | Medium     |
| **YaoXiang**   | **ref + borrow tokens** | **RAII**          | **Task boundary release / cross-task lint / std Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, borrow checking reduced to type system propositions. `&T` is duplicable, `&mut T` is not—two type properties
3. **Powerful**: Can return references, store in structs, closure capture—expressiveness on par with Rust
4. **Compiler intelligent**: ref auto-selects Rc/Arc, call site auto-selects borrow
5. **Deterministic**: ref keeps alive, won't silently become weak reference
6. **High-performance**: Move zero-copy, tokens zero overhead (zero-size type, disappears after compile)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifier, function signatures returning references will reflect extra generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the unavoidable cost of sharing)
3. **unsafe risk**: User must guarantee correctness
4. **Cross-task cycles are lint not compile error**: Unlike Rust which errors at compile, default warn, team needs to configure deny to act as quality gate

---

## Alternative Approaches

| Approach              | Why Not Chosen                                         |
| --------------------- | ------------------------------------------------------ |
| GC                    | Runtime overhead, unpredictable pauses                |
| Rust borrow checker   | Requires lifetime `'a`, steep learning curve          |
| Pure Move             | Cannot handle concurrent sharing                       |
| No raw pointers       | Cannot do system-level programming                     |
| Expose Rc/Arc to user | Leaking implementation details to user, increases cognitive load |
| Limited borrowing (v8) | "Forbidden escape" strategy sacrificed key expressiveness like closure capture, returning references, etc. |

---

## Design Decision Record

| Decision                                | Decision                                   | Reason                                                            | Date         |
| --------------------------------------- | ------------------------------------------ | ----------------------------------------------------------------- | ------------ |
| **Default value**                       | Move (zero-copy)                           | High performance, zero overhead                                  | 2025-01-15   |
| **Sharing mechanism**                  | `ref` keyword, compiler auto-optimizes    | Simple for user, compiler is responsible                         | 2025-01-15   |
| **Borrowing**                           | `&T`/`&mut T` as zero-size token types    | Type properties (Dup/Linear) naturally derive permissions, unified type system | 2025-01-15 |
| **Borrow tokens**                       | Replace limited borrowing, `&T` Dup, `&mut T` Linear | Eliminate special rules like "forbidden escape", support closure capture/return ref/store in struct | 2026-05-29 |
| **Copying**                             | `clone()`                                  | Explicit semantics                                                | 2025-01-15   |
| **System-level**                        | `*T` + `unsafe`                            | Support system programming                                        | 2025-01-15   |
| **Lifetimes**                           | Not implemented                            | Token is value, lifetime managed by Move/RAII, reducing borrow to ownership | 2025-01-15 |
| **Rc/Arc**                              | Compiler auto-selects, invisible to user   | Reduce cognitive load                                             | 2025-01-15   |
| **Cyclic references**                   | No check intra-task, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint configurable deny | 2025-01-16 |
| **Weak**                                | Provided by std                            | Advanced users explicitly choose                                  | 2025-01-16   |
| **Consume analysis**                    | Removed                                    | Mini borrow checker, not needed                                  | 2026-05-11   |
| **Ownership return**                    | Removed                                    | `(T) -> T` signature is self-documenting                         | 2026-05-11   |
| **Empty state reuse**                   | Removed (as feature)                       | Reassignment after Move is natural behavior                       | 2026-05-11   |
| **Inverse function / partial consume / three-level field mutability** | Removed | Over-engineering | 2026-05-11 |
| **Lambda no implicit capture**          | Lambda uses only explicit parameters, no implicit outer capture | Explicit philosophy, simplifies compiler | 2026-06-16 |

### Version History

| Version   | Major Changes                                                                                              | Date          |
| --------- | ---------------------------------------------------------------------------------------------------------- | ------------- |
| v1        | Initial draft: based on Rust ownership model                                                               | 2025-01-08    |
| **v8**    | **Removed over-engineering (inverse function/partial consume/three-level field mutability/consume analysis/ownership return/empty state reuse), added limited borrowing &T/&mut T** | **2026-05-11** |
| **v9**    | **Borrow token system replaces limited borrowing, unified type system; token conflict detection corrected to Hoare propositions, see RFC-009a** | **2026-06-13** |

### Open Issues

| Issue              | Description                       | Status                       |
| ------------------ | --------------------------------- | ---------------------------- |
| Drop syntax        | Whether explicit `drop()` needed  | Pending discussion           |
| Escape analysis algorithm | Cross-task detection for ref | Pending discussion           |
| Token conflict detection | Hoare logic propositions, see below | ✅ Resolved (see RFC-009a) |

### Token Conflict Detection: Hoare Logic Proposition

Full plan for token conflict detection see [RFC-009a: Token Lifetime Analysis—Based on Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md). Key points:

**Token liveness is Hoare logic proposition.**
`{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}`—shares RFC-027 proof pipeline with type checking and user predicate verification. Compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), pipeline returns Proved / Disproved / Unproven.

**Borrow checking hasn't disappeared—it has been reduced in dimension.** `BorrowChecker` becomes `BorrowPredicateEmitter`, generates propositions rather than performing checks. This is completely parallel to "type checker": type checker generates type equality propositions, borrow proposition generator generates borrow propositions, same pipeline verifies.

**Brand ID (`#42`) is `'a`.** Information is identical, encoding differs. `'a` is visible in type signature, `#42` is compiler-internal. No new analysis invented—lowered lifetimes from type layer to proof layer.

**Algorithm Summary** (see RFC-009a for details):

- Brand tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumer, break cuts back edges, structural analysis covers 95%+ scenarios (fast path)
- SMT logic cutting → only called when while + path conditions present (slow path, extremely rare)

---

## References

### YaoXiang Official Documentation

- [Language Specification](../language-spec.md)
- [Design Manifesto](../manifesto.md)
- [RFC-001 Concurrent Model and Error Handling](./001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [tutorial/ Tutorials](../../../../../tutorial/)

### External References

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and Disposition

| Status         | Location                 | Description                    |
| -------------- | ------------------------ | ------------------------------ |
| **Draft**      | `docs/design/rfc/`       | Author draft, awaiting review  |
| **Under Review** | `docs/design/rfc/`       | Open for community discussion  |
| **Accepted**   | `docs/design/accepted/`  | Becomes official design doc   |
| **Rejected**   | `docs/design/rfc/`       | Preserved in RFC directory     |
