---
title: 'RFC-009: Ownership Model Design'
status: 'Accepted'
author: 'Chenxu'
created: '2025-01-08'
updated:
  '2026-06-13 (Token conflict detection corrected to Hoare propositions; body synchronized with
  RFC-009a)'
issue: '#126'
---

# RFC-009: Ownership Model Design

## Summary

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core design—five concepts, one gradient**:

```
Peek/mutate in place   Take away     Shared holding     Clone one     System-level
    │                   │                  │              │              │
   &T                  Move              ref           clone()       unsafe
  &mut T              zero-copy        compiler auto   explicit deep  *T
  zero-size token      default         chooses Rc/Arc    copy        user responsible
  type attr naturally
  derive permission
```

- **Move (default)**: Assignment / argument passing / return = ownership transfer, zero-copy, RAII
  auto-release
- **`&T` / `&mut T` (borrow tokens)**: Zero-size compile-time token types. `&T` is duplicable
  (shared read), `&mut T` is linear (exclusive mut). Permissions are naturally derived from type
  attributes, no special rules needed. Can be returned, stored in structs.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically picks Rc (within a single task) or
  Arc (across tasks)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, system-level escape hatch

**Complexity eliminated**:

- ❌ No lifetime `'a`
- ❌ No standalone borrow-checking framework (borrow conflict demoted to Hoare propositions, sharing
  the proof pipeline with type checking)
- ❌ No GC
- ❌ No "no escape" or other special rules (tokens are ordinary types; scope is handled uniformly by
  the type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler picks automatically)

> **Programming burden**: `&T` is duplicable, `&mut T` is non-duplicable—two type attributes, zero
> special rules, fully automatic for the compiler. **Performance guarantee**: Move is zero-overhead,
> tokens are zero-overhead (zero-size types, disappear after compilation), ref pays only as needed,
> no GC pauses.

## Motivation

### Why an ownership model?

| Language     | Memory management        | Problems                                              |
| ------------ | ------------------------ | ----------------------------------------------------- |
| C/C++        | Manual                   | Memory leaks, dangling pointers, double-free          |
| Java/Python  | GC                       | Latency spikes, memory overhead, unpredictable pauses |
| Rust         | Ownership + borrow check | Steep learning curve for lifetime `'a`                |
| **YaoXiang** | **Move + Token + ref**   | **Simple, deterministic, no GC**                      |

### Design goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T borrow tokens (zero-overhead, permissions naturally derived from type attributes)
print_info(p2)                 # Compiler auto-creates &Point token, released when done
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = shared (compiler auto-picks Rc/Arc)
shared = ref p2                # Hold across scopes
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, exclusively owned

# 5. unsafe + *T = system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core differences from Rust

| Feature          | Rust                                      | YaoXiang                                                                                    |
| ---------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------- |
| Default semantic | Borrow `&T` (need explicit `.clone()`)    | **Move (value passing, zero-copy)**                                                         |
| Borrow           | `&T`/`&mut T`, returnable, needs lifetime | **`&T`/`&mut T` zero-size tokens, Dup/Linear type attributes naturally derive permissions** |
| Shared mechanism | `Arc::new()` + manual Weak                | **`ref` keyword (compiler auto-picks Rc/Arc)**                                              |
| Copy             | `clone()`                                 | `clone()`                                                                                   |
| Raw pointer      | `*T`                                      | `*T`                                                                                        |
| Lifetime         | `'a`                                      | ❌ None                                                                                     |
| Borrow check     | Global inference                          | **Type checker auto-generates borrow propositions, unified proof pipeline verifies**        |
| Cycle references | Manual Weak                               | **Task-end unified release / cross-task lint / stdlib Weak**                                |

---

## Proposal

### 1. Move (default ownership transfer)

```yaoxiang
# Rule: assignment / argument passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p can no longer be read

# Variable can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p rebinds, type must be consistent

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
- Original binding unreadable after move (compile error)
- RAII: auto-release when scope ends
- Function signature `(T) -> T` is itself documentation—consume T, return T

---

### 2. `&T` / `&mut T` (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-size compile-time token types. They are not
"references", but "type-level proofs of access permission".**

#### 2.1 Two type attributes

```
&T      →  Zero-size, freezes source data (WriteToken forbidden while ReadToken is alive),
          Under the freezing guarantee, multiple read-only views are safe → Duplicable
&mut T  →  Zero-size, exclusive read-write (any other token forbidden while WriteToken is alive),
          Under exclusive access, duplication is meaningless → linear (non-Dup)
```

**The causality cannot be reversed: freezing is the cause, Duplicable is the result.** It's not that
"`&T` implements Duplicable, so it can coexist"—it's that the data is frozen (no mutation possible),
so multiple read-only views are safe, and Duplicable can be implemented. If you take Duplicable as
the definition and treat conflict checking as "extra patches", the design is wrong.

#### 2.2 Basic usage

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

# Call site: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, previous token was released when shift call ended

# Free functions are the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist—Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "no escape" is unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T`—only as parameters, no return, no struct
storage. This was patching the "borrow" concept.

The token system does not need these rules. Tokens are **ordinary types**, following the same scope
rules as every other type.

**Returning references—naturally supported**:

```yaoxiang
# ✅ Tokens propagate along with the return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Sub-token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Stored in structs—naturally supported**:

```yaoxiang
# ✅ Struct carries tokens as fields
Window: Type = {
    target: Point,
    view: &Point,      # Token field—holds a read-only view of target
}

# view's token derives from target, Window owns both
# As long as Window exists, the view token is valid
```

#### 2.3 Closures and Lambda explicit parameters

Lambda is a function value—it can be returned, stored, and escape the current scope. Therefore
Lambda **does not implicitly capture outer local variables**. When outer data is needed, pass it in
as explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not affected by this rule—spawn is an immediately executed concurrent block, parent task blocks and waits
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x is not in scope

# ✅ Correct way: explicit parameter passing
f = (x) => { x + 1 }
f(x)

# ✅ Correct way two: context frozen at creation point (currying)—closure only takes parameters, does not capture
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
evens = list.filter(nums, gt(threshold))
```

> Supplement (2026-08-17): The proper solution for context dependency is currying-based freezing,
> not capture. After a closure escapes, its definition site's scope may be dead, so implicit capture
> is forbidden; but at the call site (creation point), the scope is guaranteed to be alive, so
> freezing the context as a value at that point to enter the closure is safe. See SPEC §12.3.

**`spawn { }` is not a function value.** A block marked with `spawn` is like an `if`/`while` body—it
executes immediately and completes while the parent stack frame is alive. The spawn body can freely
access outer variables.

**Across tasks—tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: tokens cannot be passed across tasks
}

# This is not a special rule—tokens are compile-time permission proofs, use ref for cross-task sharing
# If you need cross-task sharing, use ref
```

**Tokens cannot be `ref`'d**:

```yaoxiang
# ❌ Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token lifetime

Token lifetime is determined by **ordinary scope rules**, no lifetime parameters needed:

- Tokens in function parameters: alive during the call, released when the call ends
- Returned tokens: ownership transferred to the caller
- Tokens stored in structs: alive together with the struct

The compiler does not need `'a` annotations, because tokens are **values**, and value lifetimes are
managed uniformly by the ownership system (Move/RAII). **Demote the borrow problem to an ownership
problem.**

#### 2.5 Token conflict detection

Token conflict detection is a **Hoare logic proposition**, not a standalone flow-sensitive analysis.

```
{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}
```

It shares the RFC-027 proof pipeline with type checking and user predicate verification. The
compiler automatically generates borrow propositions (`borrow_conflict`, `use_after_move`,
`use_after_drop`, `mut_violation`) and feeds them into the pipeline. The pipeline returns Proved /
Disproved / Unproven.

```yaoxiang
# ❌ &mut tokens are linear, cannot be duplicated
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # ❌ Compile error: WriteToken has been moved
}

# ✅ &T tokens are Dup type, can be freely duplicated
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking has not disappeared—it has been demoted.** The existing `BorrowChecker` becomes
`BorrowPredicateEmitter` (a proposition generator); the generated borrow propositions share the same
proof pipeline as other type propositions. This perfectly parallels the type checker concept: the
type checker generates type equality propositions, the borrow proposition generator generates borrow
propositions, and the same pipeline verifies them. See
[RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler internals: brand mechanism

Users never touch brands. The compiler internally assigns a unique compile-time identifier to each
token:

```
User sees              Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a unique compile-time integer
&mut Point     →  WriteToken(Point, #M)   // #M is a unique compile-time integer
```

Uses of brands:

- **Anti-forgery**: Tokens can only be obtained from the owner's capsule, not constructed out of
  thin air
- **Association tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries
  the derived brand (`#N.field_x`), and the compiler can track back to the parent token
- **Conflict detection**: The same-source `WriteToken` and derived `ReadToken` cannot be
  simultaneously alive

Brands completely disappear after monomorphization and inlining; they do not exist in the generated
machine code. **Zero runtime overhead.**

#### 2.8 Automatic borrow selection rules

The compiler automatically selects at the call site according to the following priority:

```
1. If the argument is used afterwards → prefer creating a token (&T or &mut T, based on method signature)
2. If the argument is not used afterwards → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p is no longer used
```

#### 2.9 Comparison with RFC-009 v8 bare-bones borrow

| Feature                | Bare-bones borrow (v8)                | Borrow token (v9)                                    |
| ---------------------- | ------------------------------------- | ---------------------------------------------------- |
| Return reference       | ❌ Hardcoded forbidden                | ✅ Token propagates with return value                |
| Store in struct        | ❌ Hardcoded forbidden                | ✅ Token as struct field                             |
| Lambda explicit params | ❌ Hardcoded forbidden                | ✅ Lambda uses explicit parameters                   |
| Special rules          | 3 (param only / no return / no store) | 0—type attributes naturally derive                   |
| Borrow check           | Dedicated cross-borrow check          | Type checker's flow-sensitive liveness analysis      |
| Lifetime annotation    | Not needed                            | Not needed                                           |
| Runtime overhead       | Zero                                  | Zero (zero-size type, disappears after compilation)  |
| Error message          | "Borrow cannot escape"                | "WriteToken(#3) has been moved" (regular type error) |
| User mental model      | Understand "borrow"'s special status  | `&T` is duplicable, `&mut T` is not                  |

---

### 3. `ref` keyword (compiler auto-optimization)

`ref` is the only way to share across scopes. Whether the underlying is Rc or Arc, the user doesn't
need to care.

#### 3.1 Basic usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Shared, compiler auto-picks implementation

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

**User mental model**: `ref` = shared holding. That's enough.

#### 3.2 Compiler escape analysis: Rc vs Arc

```
Data-flow analysis of ref:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks          → Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle detection strategy

```
Intra-task cycle → silently allowed.
  ├── Each task has a clear lifecycle boundary—when the task ends, all resources (including ref cycles) are released uniformly.
  ├── Long-running services should create subtasks per request/connection—subtasks auto-recycle when ended, no accumulating leaks.
  ├── ref always keeps alive, semantics not watered down.
  └── Users have the right to build bidirectional strong references within a task (e.g., graph computation intermediate state).

Cross-task cycle → lint (default warn, configurable).
  ├── Program behavior is correct, won't truly leak (when parent task ends, all subtask resources are released).
  ├── But cross-task strong references mean blurred ownership boundaries, worth pausing to reconsider.
  ├── Default warn level, compilation passes with a hint.
  └── Teams can set to deny in project config, incorporated into CI quality gate.
```

**Lint levels** (similar to Rust clippy):

| Level            | Behavior                           | Scenario                          |
| ---------------- | ---------------------------------- | --------------------------------- |
| `allow`          | No check                           | Personal project                  |
| `warn` (default) | Compilation passes, with hint      | Development stage                 |
| `deny`           | Compilation fails                  | Team CI quality gate              |
| `forbid`         | Compilation fails, cannot override | Organization-level mandatory rule |

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
cross-task-cycle = "deny"    # Cross-task cycle is directly rejected on CI
```

| Cycle type           | Behavior            | Reason                                     |
| -------------------- | ------------------- | ------------------------------------------ |
| Intra-task ref cycle | No check            | User's privilege, task-end uniform release |
| Cross-task ref cycle | Lint (default warn) | Reminder to rethink, configurable to deny  |

#### 3.4 Weak: provided by standard library

```yaoxiang
use std.weak

# Advanced users explicitly choose
a.next = ref b
b.prev = std.weak.new(a.next)   # User explicitly controls which direction is weak
```

**`Weak` is not a language built-in, it is a standard library type.** Daily use of `ref` is enough.
Advanced users who need fine-grained memory control manually import `Weak`.

> 2026-08-03 revision: Implemented as an independent `std.weak` module (`std.rc` does not
> exist—`ref` is a language keyword, not a module; module path is uniformly `std.weak`,
> construction/upgrade entry points are `std.weak.new` / `std.weak.upgrade`). The originally
> envisioned `std.rc.Weak` did not land; this revision prevails.

#### 3.5 Borrow token vs `ref`

|            | `&T` / `&mut T`                                                       | `ref`                                        |
| ---------- | --------------------------------------------------------------------- | -------------------------------------------- |
| What       | Peek / mutate in place                                                | Shared holding                               |
| Scope      | Follows the token value's scope                                       | Cross-scope                                  |
| Cost       | Zero overhead (zero-size type)                                        | Rc or Arc (compiler picks)                   |
| Escape     | Yes (token propagates with return / struct / closure)                 | Designed to escape                           |
| Cross-task | No (token is compile-time permission proof, cannot pass across tasks) | Yes (compiler auto-picks Arc)                |
| Cycle      | N/A                                                                   | Intra-task silently allowed, cross-task lint |

---

### 4. `clone()` — Explicit copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, mutually unaffected
```

**When to use**: scenarios where you need to keep the original value and neither Move nor sharing is
appropriate.

### 5. `unsafe` + raw pointer (system-level programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Restrictions**:

- Can only be used inside an `unsafe` block
- User guarantees no dangling, no use-after-free
- Used for FFI, memory operations, and other system-level programming

---

### 6. Ownership gradient overview

```
  Borrow token (zero-overhead)   Move (zero-overhead)   Shared (pay as you go)   Copy
   │                            │                       │                       │
  &T duplicable token          Default ownership transfer  ref Rc/Arc        clone()
  &mut T linear token          Chain consumption & return  compiler auto-picks  explicit deep copy
   │                            │                       │                       │
  Token value scope            Within scope            Cross-scope             Anytime
  Returnable / storable in struct  T -> T return     ref cross-task → Arc    Independent copy
  Zero-size, disappears after compilation  T -> Void consume  ref not cross-task → Rc
  Zero-size, disappears after compilation                  Intra-task cycle silent
                                                          Cross-task cycle lint
                                                          Stdlib Weak escape hatch
```

---

## Comprehensive example

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
        self                            # Take away, modify, return to you
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
b.prev = ref a                      # Cycle, released uniformly when task ends

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type system constraints

### Dup type attribute

`Dup` (Duplicable) is a type attribute automatically managed by the compiler, meaning **shallow
copy**: when assigning / passing arguments, what is copied is the handle / token, while the
underlying data is shared. This forms a three-level gradient with Move (ownership transfer) and
Clone (explicit deep copy, creating an independent copy).

**Dup and Clone are orthogonal concepts**—Dup copies the handle to share data, Clone creates an
independent copy. A type can support both Dup and Clone, or only one of them.

| Type          | Dup                                                             | Clone | Note                                        |
| ------------- | --------------------------------------------------------------- | ----- | ------------------------------------------- |
| `&T`          | ✅ (copy token, multiple views point to same data)              | ✅    | Read-only token                             |
| `ref T`       | ✅ (reference count +1, share heap data)                        | ✅    | Shared holding (compiler auto-picks Rc/Arc) |
| String, Bytes | ✅ (internal ref count, copy handle to share underlying buffer) | ✅    | String / bytes                              |
| `&mut T`      | ❌ (linear, exclusive)                                          | ❌    | Mutable token                               |
| `*T`          | ❌                                                              | ❌    | Raw pointer                                 |
| struct        | Derived (auto-derived when all fields are Dup)                  | ✅    | Struct                                      |

**Primitive value types** (Int, Float, Bool, Char) have assignment behavior as compiler-built-in
value copy—the two values are completely independent, not shallow copies. They do not belong to the
Dup type attribute, but are the compiler's native handling.

---

## Performance analysis

| Operation              | Cost           | Note                                                                |
| ---------------------- | -------------- | ------------------------------------------------------------------- |
| Move                   | Zero           | Pointer move                                                        |
| `&T` / `&mut T`        | Zero           | Zero-size type, disappears after compilation, zero runtime overhead |
| `ref` (not cross-task) | Low            | Compiled to Rc, non-atomic operations                               |
| `ref` (cross-task)     | Medium         | Compiled to Arc, atomic operations                                  |
| `clone()`              | Type-dependent | Fast for small objects, slow for large objects                      |
| `unsafe + *T`          | Zero           | Direct memory operation                                             |

### Comparison

| Language     | Shared mechanism           | Memory mgmt        | Cycle handling                                            | Complexity |
| ------------ | -------------------------- | ------------------ | --------------------------------------------------------- | ---------- |
| Rust         | Arc / Mutex + borrow check | Compile-time check | Manual Weak                                               | High       |
| Go           | chan / pointer             | GC                 | GC                                                        | Low        |
| C++          | shared_ptr                 | RAII               | weak_ptr                                                  | Medium     |
| **YaoXiang** | **ref + borrow token**     | **RAII**           | **Task boundary release / cross-task lint / stdlib Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent
   with RFC-010's `name: type = value`
2. **Simple**: No lifetime, borrow check demoted to a type system proposition. `&T` is duplicable,
   `&mut T` is not—two type attributes
3. **Powerful**: Can return references, store in structs, capture in closures—expressive power on
   par with Rust
4. **Compiler intelligence**: ref auto-picks Rc/Arc, call site auto-selects borrow
5. **Deterministic**: ref is always kept alive, never quietly downgraded to weak reference
6. **High performance**: Move is zero-copy, tokens are zero-overhead (zero-size types, disappear
   after compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers; function signatures
   returning references expose additional generic parameters
2. **`ref` runtime overhead**: Atomic operations have cost (but this is the necessary price of
   sharing)
3. **`unsafe` risk**: User must guarantee correctness
4. **Cross-task cycle is lint, not compile error**: Unlike Rust which raises a compile error,
   default is warn; team must configure deny to act as a quality gate

---

## Alternatives

| Alternative            | Why not chosen                                                                            |
| ---------------------- | ----------------------------------------------------------------------------------------- |
| GC                     | Runtime overhead, unpredictable pauses                                                    |
| Rust borrow checker    | Requires lifetime `'a`, steep learning curve                                              |
| Pure Move              | Cannot handle concurrent sharing                                                          |
| No raw pointer         | Cannot do system-level programming                                                        |
| Expose Rc/Arc to users | Dumps implementation details on users, increases cognitive load                           |
| Bare-bones borrow (v8) | "No escape" strategy sacrifices key expressiveness like closure capture, return reference |

---

## Design decision record

| Decision                                                                  | Decision                                                                                                                                      | Reason                                                                                                      | Date       |
| ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ---------- |
| **Default value**                                                         | Move (zero-copy)                                                                                                                              | High performance, zero overhead                                                                             | 2025-01-15 |
| **Shared mechanism**                                                      | `ref` keyword, compiler auto-optimization                                                                                                     | Simple for users, compiler handles                                                                          | 2025-01-15 |
| **Borrow**                                                                | `&T`/`&mut T` as zero-size token types                                                                                                        | Type attributes (Dup/Linear) naturally derive permissions, unified type system                              | 2025-01-15 |
| **Borrow token**                                                          | Replaces bare-bones borrow, `&T` Dup, `&mut T` Linear                                                                                         | Eliminate "no escape" special rules, support closure capture / return reference / struct storage            | 2026-05-29 |
| **Copy**                                                                  | `clone()`                                                                                                                                     | Explicit semantic                                                                                           | 2025-01-15 |
| **System level**                                                          | `*T` + `unsafe`                                                                                                                               | Support systems programming                                                                                 | 2025-01-15 |
| **Lifetime**                                                              | Not implemented                                                                                                                               | Token is a value, lifetime managed uniformly by Move/RAII, demote borrow to ownership problem               | 2025-01-15 |
| **Rc/Arc**                                                                | Compiler auto-select, invisible to user                                                                                                       | Lower cognitive load                                                                                        | 2025-01-15 |
| **Cycle reference**                                                       | Intra-task no check, cross-task lint (default warn)                                                                                           | Structured concurrency naturally guarantees, lint can be configured to deny                                 | 2025-01-16 |
| **Weak**                                                                  | Standard library provided                                                                                                                     | Advanced users explicitly choose                                                                            | 2025-01-16 |
| **Consumption analysis**                                                  | Removed                                                                                                                                       | Mini borrow checker, not needed                                                                             | 2026-05-11 |
| **Ownership return**                                                      | Removed                                                                                                                                       | `(T) -> T` signature is itself documentation                                                                | 2026-05-11 |
| **Empty state reuse**                                                     | Removed (as a feature)                                                                                                                        | Reassignment after Move is natural behavior                                                                 | 2026-05-11 |
| **Inverse function / partial consumption / three-level field mutability** | Removed                                                                                                                                       | Over-engineering                                                                                            | 2026-05-11 |
| **Lambda does not implicitly capture**                                    | Lambda only uses explicit parameters, does not implicitly capture outer variables; context frozen via currying at creation point (SPEC §12.3) | Closure's definition site scope may be dead; frozen value at creation point (call site scope alive) is safe | 2026-06-16 |

### Version history

| Version | Major changes                                                                                                                                                                                      | Date           |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| v1      | Initial draft: based on Rust ownership model                                                                                                                                                       | 2025-01-08     |
| **v8**  | **Remove over-engineering (inverse function / partial consumption / three-level field mutability / consumption analysis / ownership return / empty state reuse), add bare-bones borrow &T/&mut T** | **2026-05-11** |
| **v9**  | **Borrow token system replaces bare-bones borrow, unified type system; token conflict detection corrected to Hoare propositions, see RFC-009a**                                                    | **2026-06-13** |

### Open issues

| Issue                     | Description                                  | Status                                 |
| ------------------------- | -------------------------------------------- | -------------------------------------- |
| Drop syntax               | Whether explicit `drop()` function is needed | Open for discussion                    |
| Escape analysis algorithm | Cross-task detection implementation for ref  | Open for discussion                    |
| Token conflict detection  | Hoare logic proposition, see below           | ✅ Resolved (see RFC-009a for details) |

### Token conflict detection: Hoare logic proposition

The complete solution for token conflict detection is in
[RFC-009a: Token Lifetime Analysis—Based on Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md).
Key points:

**Token liveness is a Hoare logic proposition.**
`{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}`—shares the RFC-027
proof pipeline with type checking and user predicate verification. The compiler automatically
generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`), and the pipeline returns Proved / Disproved / Unproven.

**Borrow checking has not disappeared—it has been demoted.** `BorrowChecker` becomes
`BorrowPredicateEmitter`, generating propositions rather than performing checks. This perfectly
parallels the "type checker" concept: the type checker generates type equality propositions, the
borrow proposition generator generates borrow propositions, and the same pipeline verifies them.

**Brand ID (`#42`) is `'a`.** Same information, different encoding. `'a` is visible in the type
signature, `#42` is internal to the compiler. No new analysis invented—lifetime is demoted from the
type layer to the proof layer.

**Algorithm outline** (see RFC-009a for details):

- Brand tree prefix match → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → starting from consumer, break cut-back edges, structural analysis covers 95%+
  scenarios (fast path)
- SMT logical cut → only invoked for while + path conditions (slow path, extremely rare)

---

## References

### YaoXiang official documentation

- [Language Specification](../language-spec.md)
- [Design Manifesto](../../manifesto.md)
- [RFC-001 Concurrent Model](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [tutorial/ Tutorials](../../../../../tutorial/)

### External references

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and destination

| Status           | Location                | Description                                |
| ---------------- | ----------------------- | ------------------------------------------ |
| **Draft**        | `docs/design/rfc/`      | Author's draft, awaiting review submission |
| **Under review** | `docs/design/rfc/`      | Open community discussion and feedback     |
| **Accepted**     | `docs/design/accepted/` | Becomes official design document           |
| **Rejected**     | `docs/design/rfc/`      | Retained in the RFC directory              |
