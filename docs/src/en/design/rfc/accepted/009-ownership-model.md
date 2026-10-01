---
title: 'RFC-009: Ownership Model Design'
status: 'Accepted'
author: 'Chenxu'
created: '2025-01-08'
updated:
  '2026-06-13 (Token conflict detection corrected to Hoare propositions, body synchronized with
  RFC-009a)'
issue: '#126'
---

# RFC-009: Ownership Model Design

## Summary

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core Design — Five Concepts, One Gradient**:

```
Peek/in-place    Take            Share              Copy            System-level
   modify
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         Zero-copy      Compiler auto Explicit deep   *T
  Zero-size token  Default      picks Rc/Arc   copy           User-responsible
  Type attribute
  natural permission
  derivation
```

- **Move (default)**: Assignment / parameter passing / return = ownership transfer, zero-copy, RAII
  auto-release
- **`&T` / `&mut T` (Borrow Token)**: Zero-sized compile-time token type. `&T` is duplicable (shared
  read), `&mut T` is linear (exclusive mutable). Permissions are naturally derived from type
  attributes, no special rules required. Can be returned, can be stored in structs.
- **`ref` keyword**: Cross-scope sharing. The compiler automatically picks Rc (within-task) or Arc
  (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointer, system-level escape hatch

**Eliminated Complexity**:

- ❌ No lifetime `'a`
- ❌ No independent borrow check framework (borrow conflict reduced to Hoare propositions, sharing
  the proof pipeline with type checking)
- ❌ No GC
- ❌ No special rules like "no escape" (tokens are regular types, scope handled uniformly by the
  type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler auto-selects)

> **Programming burden**: `&T` is duplicable, `&mut T` is non-duplicable — two type attributes, zero
> special rules, fully automatic compiler. **Performance guarantee**: Move zero overhead, token zero
> overhead (zero-sized type, disappears after compilation), ref pay-as-you-go, no GC pauses.

## Motivation

### Why is an ownership model needed?

| Language     | Memory Management        | Problem                                                     |
| ------------ | ------------------------ | ----------------------------------------------------------- |
| C/C++        | Manual management        | Memory leaks, dangling pointers, double free                |
| Java/Python  | GC                       | Latency fluctuations, memory overhead, unpredictable pauses |
| Rust         | Ownership + borrow check | Steep learning curve for lifetime `'a`                      |
| **YaoXiang** | **Move + Token + ref**   | **Simple, deterministic, no GC**                            |

### Design Goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T borrow tokens (zero overhead, permissions naturally derived from type attributes)
print_info(p2)                 # Compiler auto-creates &Point token, released when done
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = sharing (compiler auto-picks Rc/Arc)
shared = ref p2                # Hold across scope
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, exclusive

# 5. unsafe + *T = system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature             | Rust                                         | YaoXiang                                                                               |
| ------------------- | -------------------------------------------- | -------------------------------------------------------------------------------------- |
| Default semantics   | Borrow `&T` (requires explicit `.clone()`)   | **Move (value passing, zero-copy)**                                                    |
| Borrow              | `&T`/`&mut T`, can return, requires lifetime | **`&T`/`&mut T` zero-sized tokens, Dup/Linear type attributes naturally derived**      |
| Sharing mechanism   | `Arc::new()` + manual Weak                   | **`ref` keyword (compiler auto-picks Rc/Arc)**                                         |
| Copy                | `clone()`                                    | `clone()`                                                                              |
| Raw pointer         | `*T`                                         | `*T`                                                                                   |
| Lifetime            | `'a`                                         | ❌ None                                                                                |
| Borrow check        | Global inference                             | **Type checker auto-generates borrow propositions, unified proof pipeline validation** |
| Circular references | Manual Weak                                  | **Unified release at task end / cross-task lint / standard library Weak**              |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: Assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p can no longer be read

# Variables can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p rebound, type must be the same

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
- RAII: Automatic release at scope end
- Function signature `(T) -> T` is itself the documentation — consume T, return T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not
"references", but "type-level proofs of access permission".**

#### 2.1 Two Type Attributes

```
&T      →  Zero-sized, freezes source data (WriteToken forbidden while ReadToken is alive),
          under freezing guarantee multiple read-only views are safe → Duplicable (Dup)
&mut T  →  Zero-sized, exclusive read-write (any other token forbidden while WriteToken is alive),
          under exclusive access copying is meaningless → Linear (non-Dup)
```

**Causality cannot be reversed: freezing is the cause, Dup is the result.** It is not that `&T`
implements Dup so they can coexist — it is because the data is frozen (no mutation possible),
multiple read-only views are safe, and Dup can be implemented. If you treat Dup as the definition
and conflict checking as an "extra patch", the design is wrong.

#### 2.2 Basic Usage

```yaoxiang
# Method side: Declare parameter type, determine required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point token grants write permission
    self.y = self.y + dy
}

# Caller side: Compiler automatically picks borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, previous token released with end of shift call

# Free functions follow the same principle
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escape" Is Not Needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — can only be parameters, cannot be
returned, cannot be stored in structs. This was patching the "borrow" concept.

The token system does not need these rules. Tokens are **regular types**, following the same scope
rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Tokens propagate along with the return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Child token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Storing in structs — naturally supported**:

```yaoxiang
# ✅ Structs carry tokens as fields
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds read-only view of target
}

# The view token is derived from target, Window holds ownership of both
# As long as Window exists, the view token is valid
```

#### 2.3 Closures and Lambda Explicit Parameters

Lambda is a function value — it can be returned, stored, and passed out of the current scope.
Therefore Lambda **does not implicitly capture outer local variables**. When you need outer data,
pass it in via explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not affected by this rule — spawn is an immediately executing concurrent block, parent task blocks waiting
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x not in scope

# ✅ Correct way: Pass explicitly as parameter
f = (x) => { x + 1 }
f(x)

# ✅ Correct way two: Context is fixed at creation point (currying) — closures only take parameters, no capture
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
evens = list.filter(nums, gt(threshold))
```

> Supplement (2026-08-17): The correct way to handle context dependencies is currying fixity, not
> capture. After a closure escapes, its definition scope may be dead, so it cannot implicitly
> capture; but the call point (creation point) scope must be alive, and fixing the context at that
> point as a value entering the closure is safe. See SPEC §12.3.

**spawn { } is not a function value.** A block marked by spawn is the same as if/while body —
executes immediately, completes while the parent stack frame is alive. spawn body can normally
access outer variables.

**Cross-task — tokens cannot pass through threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task boundary
}

# This is not a special rule — tokens are compile-time permission proofs, use ref for cross-task sharing
# If you need cross-task sharing, use ref
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token Lifetime

The lifetime of a token is determined by **normal scope rules**, no lifetime parameters required:

- Tokens in function parameters: alive during the call, released after the call ends
- Returned tokens: ownership transferred to the caller
- Tokens stored in structs: alive together with the struct

The compiler does not need `'a` annotations, because tokens are **values**, and the value's lifetime
is uniformly managed by the ownership system (Move/RAII). **Reducing the borrow problem to an
ownership problem.**

#### 2.5 Token Conflict Detection

Token conflict detection is a **Hoare logic proposition**, not an independent flow-sensitive
analysis.

```
{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}
```

It shares the proof pipeline of RFC-027 with type checking and user predicate verification. The
compiler automatically generates borrow propositions (`borrow_conflict`, `use_after_move`,
`use_after_drop`, `mut_violation`), which are sent to the pipeline for verification. The pipeline
returns Proved / Disproved / Unproven.

```yaoxiang
# ❌ &mut tokens are linear and cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # ❌ Compile error: WriteToken already moved
}

# ✅ &T tokens are Dup type and can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow check has not disappeared — it has been reduced.** The existing `BorrowChecker` becomes
`BorrowPredicateEmitter` (proposition generator), and the generated borrow propositions share the
same proof pipeline as other type propositions. This is exactly parallel to the concept of type
checker: type checker generates type equality propositions, borrow proposition generator generates
borrow propositions, the same pipeline validates. Detailed design see
[RFC-009a](../accepted/009a-borrow-proof-pipeline.md).

#### 2.7 Compiler Internals: Brand Mechanism

Users never touch brands. The compiler internally assigns a compile-time unique identifier to each
token:

```
What users see         Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand purposes:

- **Anti-forgery**: Tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Association tracking**: When deriving `&Float` from `&Point` (field access), `&Float` carries
  the derived brand (`#N.field_x`), the compiler can trace back to the parent token
- **Conflict detection**: Same-source `WriteToken` and derived `ReadToken` cannot be active
  simultaneously

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

#### 2.8 Automatic Borrow Selection Rules

The caller-side compiler automatically selects by the following priority:

```
1. If the argument is still used later → Prioritize creating a token (&T or &mut T, per method signature)
2. If the argument is no longer used → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Bare-Bones Borrow

| Feature                    | Bare-bones Borrow (v8)                            | Borrow Token (v9)                                    |
| -------------------------- | ------------------------------------------------- | ---------------------------------------------------- |
| Returning references       | ❌ Hard-coded prohibition                         | ✅ Tokens propagate with return value                |
| Storing in structs         | ❌ Hard-coded prohibition                         | ✅ Tokens as struct fields                           |
| Lambda explicit parameters | ❌ Hard-coded prohibition                         | ✅ Lambda uses explicit parameters                   |
| Special rules              | 3 (only as parameters/cannot return/cannot store) | 0 — type attributes naturally derived                |
| Borrow check               | Dedicated cross-borrow check                      | Type checker flow-sensitive liveness analysis        |
| Lifetime annotation        | Not needed                                        | Not needed                                           |
| Runtime overhead           | Zero                                              | Zero (zero-sized type, disappears after compilation) |
| Error message              | "Borrow cannot escape"                            | "WriteToken(#3) has been moved" (regular type error) |
| User mental model          | Understanding the special status of "borrow"      | `&T` is duplicable, `&mut T` is non-duplicable       |

---

### 3. `ref` Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether the underlying is Rc or Arc, the user does not
need to care.

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
    use(data)                     # Compiler: not cross-task → Rc
}
```

**User mental model**: `ref` = shared holding. That's enough.

#### 3.2 Compiler Escape Analysis: Rc vs Arc

```
Data flow analysis of ref:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks         → Arc (atomic reference counting, thread-safe)
```

#### 3.3 Cycle Detection Strategy

```
Within-task cycles → Silently allowed.
  ├── Each task has a clear lifecycle boundary — all resources (including ref cycles) are uniformly released when the task ends.
  ├── Long-running services should create sub-tasks per request/connection — sub-tasks auto-recycle when they end, no accumulating leaks.
  ├── ref is always kept alive, semantics are not watered down.
  └── Users have the right to build bidirectional strong references within a task (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, no real leak (when parent task ends, all child task resources are released).
  ├── But cross-task strong references mean ownership boundaries are blurred, worth stopping to reconsider.
  ├── Default warn level, compiles successfully with hints.
  └── Teams can set to deny in project config, included in CI quality gates.
```

**Lint levels** (similar to Rust clippy):

| Level            | Behavior                          | Scenario                          |
| ---------------- | --------------------------------- | --------------------------------- |
| `allow`          | Not checked                       | Personal project                  |
| `warn` (default) | Compiles successfully, with hints | Development phase                 |
| `deny`           | Compile fails                     | Team CI quality gate              |
| `forbid`         | Compile fails, cannot override    | Organization-level mandatory rule |

```yaoxiang
# Within-task cycle: silently allowed, bidirectional strong reference
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Uniformly released at task end.
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

**Project configuration example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles are rejected on CI
```

| Cycle type            | Behavior            | Reason                                       |
| --------------------- | ------------------- | -------------------------------------------- |
| Within-task ref cycle | No check            | User's right, uniformly released at task end |
| Cross-task ref cycle  | lint (default warn) | Remind to reconsider, configurable deny      |

#### 3.4 Weak: Provided by the Standard Library

```yaoxiang
use std.weak

# Advanced users explicitly choose
a.next = ref b
b.prev = std.weak.new(a.next)   # User explicitly controls which direction is weak
```

**`Weak` is not a language built-in, but a standard library type.** Daily use of `ref` is enough.
Advanced users who need fine-grained memory control manually introduce `Weak`.

> 2026-08-03 revision: Implemented as a standalone `std.weak` module (`std.rc` does not exist —
> `ref` is a language keyword rather than a module; module path uniformly `std.weak`,
> construction/upgrade entries `std.weak.new` / `std.weak.upgrade`). The initially envisioned
> `std.rc.Weak` did not land; this revision prevails.

#### 3.5 Borrow Tokens vs `ref`

|              | `&T` / `&mut T`                                                                | `ref`                                         |
| ------------ | ------------------------------------------------------------------------------ | --------------------------------------------- |
| What it does | Take a look / modify in-place                                                  | Shared holding                                |
| Scope        | Follows the token value's scope                                                | Cross-scope                                   |
| Cost         | Zero overhead (zero-sized type)                                                | Rc or Arc (compiler chooses)                  |
| Escape       | Yes (token propagates with return value/struct/closure)                        | That's what it's for                          |
| Cross-task   | Not allowed (token is compile-time permission proof, cannot pass across tasks) | Yes (compiler auto-picks Arc)                 |
| Cycles       | Not involved                                                                   | Within-task silently allowed, cross-task lint |

---

### 4. `clone()` — Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, do not affect each other
```

**When to use**: Scenarios where you need to preserve the original value and Move or sharing is not
suitable.

### 5. `unsafe` + Raw Pointers (System-Level Programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Limitations**:

- Can only be used in `unsafe` blocks
- Users guarantee no dangling, no use after free
- Used for FFI, memory operations, and other system-level programming

---

### 6. Ownership Gradient Overview

```
  Borrow tokens       Move                Share                Copy
  (zero overhead)     (zero overhead)     (pay-as-you-go)
   │                      │                  │                │
  &T Duplicable       Default ownership   ref Rc/Arc       clone()
  &mut T Linear        transfer           Compiler auto    Explicit deep copy
   │                      │                  │                │
  Token value scope   Within scope        Cross-scope      Anytime
  Can return/store    T -> T return       ref cross-task → Arc  Independent copy
  in struct
  Zero-sized,         T -> Void consume   ref not cross-task → Rc
  disappears after
  compilation
  Zero-sized,                            Within-task cycle silent
  disappears after
  compilation                             Cross-task cycle lint
                                          Standard library Weak escape
```

---

## Comprehensive Example

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,

    # &T: Read-only token
    print: (self: &Point) -> Void = {
        print(self.x)
        print(self.y)
    }

    # &mut T: Mutable token
    shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
        self.x = self.x + dx
        self.y = self.y + dy
    }

    # Move → Move: Consume and return
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # Take, modify, return to you
    }

    # Returning reference: Token propagates with return value
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

# Within-task cycle: silently allowed
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, uniformly released at task end

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### Dup Type Attribute

`Dup` (Duplicable) is a type attribute automatically managed by the compiler, meaning **shallow
copy**: what is copied during assignment/parameter passing is the handle/token, with the underlying
data shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit
deep copy, creating an independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies the handle to share data, Clone creates an
independent copy. A type can support both Dup and Clone, or only one of them.

| Type          | Dup                                                                    | Clone | Description                                 |
| ------------- | ---------------------------------------------------------------------- | ----- | ------------------------------------------- |
| `&T`          | ✅ (Copy token, multiple views point to the same data)                 | ✅    | Read-only token                             |
| `ref T`       | ✅ (Reference count +1, share heap data)                               | ✅    | Shared holding (compiler auto-picks Rc/Arc) |
| String, Bytes | ✅ (Internal reference counting, copy handle shares underlying buffer) | ✅    | String/bytes                                |
| `&mut T`      | ❌ (Linear, exclusive)                                                 | ❌    | Mutable token                               |
| `*T`          | ❌                                                                     | ❌    | Raw pointer                                 |
| struct        | Derived (see "Derivation Rules" below, #398)                           | ✅    | Struct                                      |
| tuple         | Derived (element-by-element, same struct rules, #398)                  | ✅    | Tuple                                       |

**Primitive value types** (Int, Float, Bool, Char) have compiler-built-in value copy semantics for
assignment — the two values are completely independent, not shallow copies. They do not belong to
the Dup type attribute, but are native compiler handling.

#### Derivation Rules (#398 Final)

"Automatically derived when all fields are Dup" cannot be executed literally — primitive fields (Int
etc.) themselves do not belong to Dup, and `{ x: Int, y: Int }` would be misjudged as Move.
Executable form:

1. **Copyable field set** = ValueCopy (Int / Float / Bool / Char / Range) ∪ Dup (`&T`, `ref T`,
   String / Bytes, function values (#352), composite types that are already Dup);
2. **struct**: All fields are in the copyable field set → derive Dup; **any** field is Linear
   (`&mut T`) or Move (nested Move struct / Vec, Dict, and other containers / resources) → overall
   remains Move (not introducing a "partially copyable" intermediate state — under copy semantics it
   must be transferred);
3. **tuple**: Same rule as struct, judged element by element; empty tuple (unit) is `Void`;
4. **Derivation is recursive**: When a field is a named type (e.g., `target: Point`), its definition
   is unfolded and then judged; `A = { b: B }` follows B's derivation result; circular aliases are
   conservatively downgraded to Move by depth limit;
5. **Not in this row's scope** (still Move, separate case): containers (Vec / Dict / Set / Option /
   Result / Array) and enum.

---

## Performance Analysis

| Operation              | Cost           | Description                                                          |
| ---------------------- | -------------- | -------------------------------------------------------------------- |
| Move                   | Zero           | Pointer move                                                         |
| `&T` / `&mut T`        | Zero           | Zero-sized type, disappears after compilation, zero runtime overhead |
| `ref` (not cross-task) | Low            | Compiles to Rc, non-atomic operation                                 |
| `ref` (cross-task)     | Medium         | Compiles to Arc, atomic operation                                    |
| `clone()`              | Type-dependent | Fast for small objects, slow for large objects                       |
| `unsafe + *T`          | Zero           | Direct memory operation                                              |

### Comparison

| Language     | Sharing mechanism          | Memory management  | Cycle handling                                                      | Complexity |
| ------------ | -------------------------- | ------------------ | ------------------------------------------------------------------- | ---------- |
| Rust         | Arc / Mutex + borrow check | Compile-time check | Manual Weak                                                         | High       |
| Go           | chan / pointer             | GC                 | GC                                                                  | Low        |
| C++          | shared_ptr                 | RAII               | weak_ptr                                                            | Medium     |
| **YaoXiang** | **ref + borrow tokens**    | **RAII**           | **Task boundary release / cross-task lint / standard library Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are regular types, not special language features. Completely
   consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetime, borrow check reduced to type system propositions. `&T` is duplicable,
   `&mut T` is non-duplicable — two type attributes
3. **Powerful**: Can return references, store in structs, closure capture — expressive power at the
   same level as Rust
4. **Compiler intelligence**: `ref` auto-picks Rc/Arc, caller-side automatically selects borrow
5. **Deterministic**: `ref` keeps alive, will not quietly become a weak reference
6. **High performance**: Move zero-copy, token zero overhead (zero-sized type, disappears after
   compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, function signatures
   returning references reflect additional generic parameters
2. **`ref` runtime overhead**: Atomic operations have cost (but this is the inevitable cost of
   sharing)
3. **`unsafe` risk**: Users must guarantee correctness
4. **Cross-task cycles are lint, not compile errors**: Unlike Rust which reports compile errors,
   default is warn, requires team config of deny to serve as a quality gate

---

## Alternatives

| Option                 | Why not chosen                                                                                           |
| ---------------------- | -------------------------------------------------------------------------------------------------------- |
| GC                     | Has runtime overhead, unpredictable pauses                                                               |
| Rust borrow checker    | Requires lifetime `'a`, steep learning curve                                                             |
| Pure Move              | Cannot handle concurrent sharing                                                                         |
| No raw pointers        | Cannot do system-level programming                                                                       |
| Expose Rc/Arc to users | Throw implementation details to users, increase cognitive burden                                         |
| Bare-bones borrow (v8) | The no-escape strategy sacrifices key expressive capabilities like closure capture, returning references |

---

## Design Decision Record

| Decision                                                             | Decision                                                                                                                                        | Reason                                                                                             | Date       |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ---------- |
| **Default**                                                          | Move (zero-copy)                                                                                                                                | High performance, zero overhead                                                                    | 2025-01-15 |
| **Sharing mechanism**                                                | `ref` keyword, compiler auto-optimization                                                                                                       | Simple for users, compiler responsible                                                             | 2025-01-15 |
| **Borrow**                                                           | `&T`/`&mut T` as zero-sized token types                                                                                                         | Type attributes (Dup/Linear) naturally derive permissions, unified type system                     | 2025-01-15 |
| **Borrow token**                                                     | Replace bare-bones borrow, `&T` Dup, `&mut T` Linear                                                                                            | Eliminate special rules like "no escape", support closure capture/return reference/store in struct | 2026-05-29 |
| **Copy**                                                             | `clone()`                                                                                                                                       | Explicit semantics                                                                                 | 2025-01-15 |
| **System-level**                                                     | `*T` + `unsafe`                                                                                                                                 | Support system programming                                                                         | 2025-01-15 |
| **Lifetime**                                                         | Not implemented                                                                                                                                 | Tokens are values, lifetime uniformly managed by Move/RAII, reducing borrow to ownership problem   | 2025-01-15 |
| **Rc/Arc**                                                           | Compiler auto-selects, invisible to users                                                                                                       | Reduce cognitive burden                                                                            | 2025-01-15 |
| **Circular references**                                              | No check within task, cross-task lint (default warn)                                                                                            | Structured concurrency naturally guarantees, lint can be deny                                      | 2025-01-16 |
| **Weak**                                                             | Provided by standard library                                                                                                                    | Advanced users explicitly choose                                                                   | 2025-01-16 |
| **Consumption analysis**                                             | Deleted                                                                                                                                         | Mini borrow checker, not needed                                                                    | 2026-05-11 |
| **Ownership return**                                                 | Deleted                                                                                                                                         | `(T) -> T` signature is itself the documentation                                                   | 2026-05-11 |
| **Empty state reuse**                                                | Deleted (as a feature)                                                                                                                          | Reassignment after Move is natural behavior                                                        | 2026-05-11 |
| **Inverse function/partial consumption/field three-tier mutability** | Deleted                                                                                                                                         | Over-engineering                                                                                   | 2026-05-11 |
| **Lambda does not implicitly capture**                               | Lambda only uses explicit parameters, does not implicitly capture outer variables; context is fixed via currying at creation point (SPEC §12.3) | Closure definition scope may be dead; fixity value at creation point (caller scope alive) is safe  | 2026-06-16 |

### Version History

| Version | Major Changes                                                                                                                                                                              | Date           |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------- |
| v1      | Initial draft: Based on Rust ownership model                                                                                                                                               | 2025-01-08     |
| **v8**  | **Deleted over-engineering (inverse function/partial consumption/field three-tier mutability/consumption analysis/ownership return/empty state reuse), added bare-bones borrow &T/&mut T** | **2026-05-11** |
| **v9**  | **Borrow token system replaces bare-bones borrow, unified type system; token conflict detection corrected to Hoare propositions, see RFC-009a**                                            | **2026-06-13** |

### Pending Issues

| Issue                     | Description                                  | Status                                 |
| ------------------------- | -------------------------------------------- | -------------------------------------- |
| Drop syntax               | Whether explicit `drop()` function is needed | TBD                                    |
| Escape analysis algorithm | `ref` cross-task detection implementation    | TBD                                    |
| Token conflict detection  | Hoare logic proposition, see below           | ✅ Resolved (see RFC-009a for details) |

### Token Conflict Detection: Hoare Logic Proposition

The complete scheme for token conflict detection is in
[RFC-009a: Token Lifetime Analysis — Based on the Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md).
Key points:

**Token liveness is a Hoare logic proposition.**
`{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}` — sharing the proof
pipeline of RFC-027 with type checking and user predicate verification. The compiler automatically
generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`), and the pipeline returns Proved / Disproved / Unproven.

**Borrow check has not disappeared — it has been reduced.** `BorrowChecker` becomes
`BorrowPredicateEmitter`, generating propositions rather than performing checks. This is exactly
parallel to the concept of "type checker": type checker generates type equality propositions, borrow
proposition generator generates borrow propositions, the same pipeline validates.

**Brand ID (`#42`) is `'a`.** The information is exactly the same, the encoding is different. `'a`
is visible in the type signature, `#42` is inside the compiler. No new analysis invented — lifetime
is reduced from the type layer to the proof layer.

**Algorithm summary** (see RFC-009a for details):

- Brand tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumers, break cuts back edges, structural analysis covers 95%+
  scenarios (fast path)
- SMT logic cutoff → called only with while + path conditions (slow path, extremely rare)

---

## References

### YaoXiang Official Documentation

- [Language Specification](../../../reference/language-spec/index.md)
- [Design Manifesto](../../manifesto.md)
- [RFC-001 Concurrent Model](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [tutorial/ Tutorials](../../../tutorial/index.md)

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
| **Accepted**     | `docs/design/accepted/` | Becomes official design document           |
| **Rejected**     | `docs/design/rfc/`      | Retained in RFC directory                  |
