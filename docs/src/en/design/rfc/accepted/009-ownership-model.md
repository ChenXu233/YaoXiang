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
Glance/Modify in Place    Take        Shared Ownership    Clone a Copy       System-Level
       │                   │                │               │                 │
      &T                 Move           ref             clone()           unsafe
     &mut T             Zero-Copy      Compiler auto     Explicit         *T
     Zero-size token    Default         chooses Rc/Arc   deep copy       User responsible
     Type attributes
     naturally derive
     permissions
```

- **Move (Default)**: Assignment/parameter passing/return = ownership transfer, zero-copy, RAII automatic deallocation
- **`&T` / `&mut T` (Borrow Tokens)**: Zero-size compile-time token types. `&T` is copyable (shared read-only), `&mut T`
  is linear (exclusive mutable). Permissions are naturally derived from type attributes, no special rules needed. Can be returned, can be stored in structs.
- **`ref` keyword**: Cross-scope sharing. Compiler automatically chooses Rc (not cross-task) or Arc (cross-task)
- **`clone()`**: Explicit deep copy
- **`unsafe` + `*T`**: Raw pointers, system-level escape hatch

**Eliminated Complexity**:

- ❌ No lifetime `'a`
- ❌ No standalone borrow checker framework (borrow conflicts reduced to Hoare propositions, sharing proof pipeline with type checking)
- ❌ No GC
- ❌ No special "no escaping" rules (tokens are ordinary types, scopes handled uniformly by type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler auto-selects)

> **Programming burden**: `&T` is copyable, `&mut T` is not copyable—two type attributes, zero special rules, fully automatic by compiler.
> **Performance guarantee**: Move is zero-overhead, tokens are zero-overhead (zero-size types, disappear after compilation), ref is pay-as-you-go, no GC pauses.

## Motivation

### Why Do We Need an Ownership Model?

| Language       | Memory Management             | Problems                                      |
| -------------- | ----------------------------- | --------------------------------------------- |
| C/C++          | Manual                        | Memory leaks, dangling pointers, double free  |
| Java/Python    | GC                            | Latency jitter, memory overhead, unpredictable pauses |
| Rust           | Ownership + Borrow Checker    | Lifetime `'a` steep learning curve            |
| **YaoXiang**   | **Move + Token + ref**        | **Simple, deterministic, no GC**              |

### Design goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p cannot be read again

# 2. &T / &mut T Borrow Tokens (zero-overhead, type attributes naturally derive permissions)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = Shared (compiler auto-selects Rc/Arc)
shared = ref p2                # Cross-scope ownership
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = Explicit copy
backup = p2.clone()            # Deep copy, exclusive

# 5. unsafe + *T = System-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core Differences from Rust

| Feature       | Rust                              | YaoXiang                                                 |
| ------------ | --------------------------------- | -------------------------------------------------------- |
| Default semantics | Borrow `&T` (needs explicit `.clone()`) | **Move (value semantics, zero-copy)**                |
| Borrowing     | `&T`/`&mut T`, returnable, needs lifetimes | **`&T`/`&mut T` zero-size tokens, Dup/Linear type attributes naturally derive permissions |
| Sharing mechanism | `Arc::new()` + manual Weak          | **`ref` keyword (compiler auto-selects Rc/Arc)**       |
| Copy          | `clone()`                         | `clone()`                                                |
| Raw pointers  | `*T`                              | `*T`                                                     |
| Lifetimes     | `'a`                              | ❌ None                                                  |
| Borrow checking | Global inference                 | **Type checker auto-generates borrow propositions, unified proof pipeline verifies** |
| Cyclic references | Manual Weak                     | **Task boundary release / cross-task lint / std Weak** |

---

## Proposal

### 1. Move (default ownership transfer)

```yaoxiang
# Rule: Assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p cannot be read again

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

**Characteristics**:

- Zero-copy (compiler moves pointers)
- Original binding unreadable after move (compile error)
- RAII: automatic deallocation at scope end
- Function signature `(T) -> T` is self-documenting—consumes T, returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core Principle: `&T` and `&mut T`
are zero-size compile-time token types. They are not "references", but "type-level proofs of access permission".**

#### 2.1 Two Type Attributes

```
&T      →  Zero-size, freezes source data (ReadToken alive forbids WriteToken),
          freeze guarantee makes multiple read-only views safe → Copyable (Dup)
&mut T  →  Zero-size, exclusive read-write (WriteToken alive forbids any other token),
          exclusive access makes copying meaningless → Linear (non-Dup)
```

**The causal relationship cannot be reversed: freezing is the cause, Dup is the result.** It's not because `&T`
implements Dup that they can coexist—it's because data is frozen (no mutation possible), multiple read-only views are safe, so Dup can be implemented. If you treat Dup as the definition and conflict checking as "extra patching", the design is wrong.

#### 2.2 Basic usage

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

# Call side: compiler auto-selects borrow or Move
p = Point(1.0, 2.0)
p.print()                          # Compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # Compiler auto-creates &mut Point token
p.print()                          # OK, previous token released after shift call ends

# Free functions work the same way
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist—Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "No Escaping" Is Unnecessary

RFC-009 v8 imposed three special rules on `&T`/`&mut T`—can only be parameters, cannot be returned, cannot be stored in structs. This is patching the "borrowing" concept.

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scope rules as all other types.

**Returning references—naturally supported**:

```yaoxiang
# ✅ Token propagates with return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Sub-token and parent token return together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Storing in structs—naturally supported**:

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

Lambdas are function values—they can be returned, stored, and passed out of current scope. Therefore lambdas **do not implicitly capture outer local variables**. When outer data is needed, use explicit parameters:

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

# ✅ Correct way two: context solidified at creation point (currying)—closure only takes parameters, no capture
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
evens = list.filter(nums, gt(threshold))
```

> Supplement (2026-08-17): The correct solution for context-dependent behavior is currying solidification, not capture. After closure escapes, its definition scope may be dead, so implicit capture is prohibited; but the call site (creation point) scope is guaranteed alive, so solidifying context as a value into the closure at that point is safe. See SPEC §12.3.

**spawn { } is not a function value.** The block marked by spawn, like if/while bodies, executes immediately and completes while the parent stack frame is alive. Spawn body can normally access outer variables.

**Cross-task—tokens cannot thread**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task boundary
}

# This is not a special rule—tokens are compile-time permission proofs, use ref for cross-task sharing
# If you need cross-task sharing, use ref
```

**Tokens cannot be ref'd**:

```yaoxiang
# ❌ Token is a permission proof, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not ownable
}
```

#### 2.4 Token Lifetime

Token lifetime is determined by **ordinary scope rules**, no lifetime parameters needed:

- Token in function parameter: alive during call, released after call
- Returned token: ownership transfers to caller
- Token stored in struct: alive with the struct

Compiler doesn't need `'a` annotations because tokens are **values**, and value lifetime is uniformly managed by the ownership system (Move/RAII). **Reduces borrowing problem to ownership problem.**

#### 2.5 Token conflict detection

Token conflict detection is **a Hoare logic proposition**, not independent flow-sensitive analysis.

```
{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}
```

Shares RFC-027's proof pipeline with type checking and user predicate verification. Compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), feeds to pipeline for verification. Pipeline returns Proved
/ Disproved / Unproven.

```yaoxiang
# ❌ &mut tokens are linear, cannot be duplicated
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p cannot be read again
    p.x = 10.0                      # ❌ Compile error: WriteToken already moved
}

# ✅ &T token is Dup type, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking hasn't disappeared—it has been reduced in dimensionality.** Existing `BorrowChecker` becomes
`BorrowPredicateEmitter` (proposition generator), generating borrow propositions that share the same proof pipeline with other type propositions. This is exactly parallel to the type checker: type checker generates type equality propositions, borrow proposition generator generates borrow propositions, same pipeline verifies both. See
[RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler Internals: Branding Mechanism

Users never see brands. Compiler internally assigns a compile-time unique identifier to each token:

```
What user sees         Compiler internal representation
────────────────────────────────────────
&Point            →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point        →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand uses:

- **Anti-forgery**: Token can only be obtained from owner capsule, cannot be constructed from thin air
- **Association tracking**: When deriving `&Float` from `&Point` (field access), `&Float`
  carries derived brand (`#N.field_x`), compiler can track back to parent token
- **Conflict detection**: Same-origin WriteToken and derived ReadToken cannot be alive simultaneously

Brands completely disappear after monomorphization and inlining, not present in generated machine code. **Zero runtime overhead.**

#### 2.8 Automatic borrow selection rules

Compiler at call site auto-selects by this priority:

```
1. If actual argument will be used later → prefer creating token (&T or &mut T, based on method signature)
2. If actual argument will not be used later → Move
3. Preference order: &T < &mut T < Move
```

```yaoxiang
# Example: auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p no longer used
```

#### 2.9 Comparison with RFC-009 v8 Bare-Bones Borrowing

| Feature              | Bare-bones borrowing (v8)               | Borrow Tokens (v9)                            |
| -------------------- | ----------------------------------------- | --------------------------------------------- |
| Return references    | ❌ Hardcoded forbidden                    | ✅ Token propagates with return value         |
| Store in structs     | ❌ Hardcoded forbidden                    | ✅ Token as struct field                       |
| Lambda explicit params| ❌ Hardcoded forbidden                    | ✅ Lambda uses explicit parameters            |
| Special rules        | 3 (params only/no return/no storage)      | 0—type attributes naturally derive            |
| Borrow checking      | Dedicated cross-reference borrow checking | Type checker flow-sensitive liveness analysis  |
| Lifetime annotations | Not needed                                | Not needed                                    |
| Runtime overhead     | Zero                                      | Zero (zero-size type, disappears after compilation) |
| Error messages       | "Borrow cannot escape"                    | "WriteToken(#3) already moved" (regular type error) |
| User mental model    | Understand "borrowing" special status     | `&T` is copyable, `&mut T` is not copyable    |

---

### 3. ref Keyword (Compiler Auto-Optimization)

`ref` is the only way to share across scopes. Whether it's Rc or Arc underneath, users don't need to care.

#### 3.1 Basic usage

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

**User mental model**: `ref` = shared ownership. That's enough.

#### 3.2 Compiler escape analysis: Rc vs Arc

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference count, low overhead)
Escapes to other tasks         → Arc (atomic reference count, thread-safe)
```

#### 3.3 Cycle detection strategy

```
Intra-task cycles → silently allowed.
  ├── Each task has clear lifetime boundary—all resources (including ref cycles) uniformly released at task end.
  ├── Long-running services should create subtasks per request/connection—subtask end auto-reclaims, no accumulation.
  ├── ref always keeps alive, semantics unadulterated.
  └── Users have the right to build bidirectional strong references within tasks (e.g., graph computation intermediate state).

Cross-task cycles → lint (default warn, configurable).
  ├── Program behavior is correct, no real leak (parent task end releases all subtask resources).
  ├── But cross-task strong references mean fuzzy ownership boundaries, worth pausing to rethink.
  ├── Default warn level, compilation passes but with suggestion.
  └── Teams can set to deny in project config, gate in CI quality checks.
```

**Lint levels** (similar to Rust clippy):

| Level          | Behavior                    | Scenario               |
| -------------- | --------------------------- | ---------------------- |
| `allow`        | No checking                 | Personal projects      |
| `warn` (default) | Compile passes, with note  | Development phase      |
| `deny`         | Compilation fails           | Team CI quality gate    |
| `forbid`       | Compilation fails, unc覆盖able | Organizational mandatory rules |

```yaoxiang
# Intra-task cycles: silently allowed, bidirectional strong references
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Released uniformly at task end.
}

# Cross-task cycles: lint (default warn)
@block
parent_task: () -> Void = {
    shared_a = ref a
    shared_b = ref b
    spawn {
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task cycle reference
    }
}
```

**Project config example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles rejected in CI
```

| Cycle type         | Behavior             | Reason                                        |
| ------------------ | -------------------- | --------------------------------------------- |
| Intra-task ref cycle | No checking          | User's right, released at task end           |
| Cross-task ref cycle | lint (default warn) | Prompt to rethink, configurable deny          |

#### 3.4 Weak: Provided in Standard Library

```yaoxiang
use std.weak

# Advanced users explicitly choose
a.next = ref b
b.prev = std.weak.new(a.next)   # User explicitly controls which direction is weak
```

**`Weak` is not built into the language, it's a standard library type.** Daily use of `ref` is enough. Advanced users who need fine-grained memory control manually introduce `Weak`.

> 2026-08-03 revision: implemented as standalone `std.weak` module (`std.rc` does not exist—`ref` is a language keyword, not a module; module path unified to `std.weak`, construction/upgrade entry points are `std.weak.new` / `std.weak.upgrade`). The draft's `std.rc.Weak` was not adopted, this revision prevails.

#### 3.5 Borrow Tokens vs ref

|        | `&T` / `&mut T`                            | `ref`                       |
| ------ | ------------------------------------------ | --------------------------- |
| What it does | Glance / modify in place               | Shared ownership            |
| Scope    | Follows token value's scope               | Cross-scope                 |
| Cost     | Zero overhead (zero-size type)            | Rc or Arc (compiler selects)|
| Escape   | Can (token propagates with return/struct/closure) | Naturally for escaping   |
| Cross-task | Not allowed (token is compile-time permission proof, cannot cross task boundary) | Can (compiler auto-selects Arc) |
| Cycles   | Not involved                               | Intra-task silently allowed, cross-task lint |

---

### 4. clone() —— Explicit Copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, don't affect each other
```

**When to use**: when you need to keep the original value and it's not suitable for Move or sharing.

### 5. unsafe + Raw Pointers (System-Level Programming)

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
- Used for FFI, memory operations, etc.

---

### 6. Ownership gradient overview

```
  Borrow tokens (zero overhead)   Move (zero overhead)    Sharing (pay-as-needed)   Copy
   │                               │                    │                     │
  &T copyable token            Default ownership     ref Rc/Arc            clone()
  &mut T linear token          transfer              compiler auto-select  explicit deep copy
   │                               │                    │                     │
  Token value scope              Within scope          Cross-scope           Anytime
  Can return/store in struct     T -> T return         ref cross-task → Arc  Independent copy
  Zero-size disappears after compile     T -> Void consume   ref not cross-task → Rc
                                          T -> Void consume     intra-task cycle silently allowed
                                                              cross-task cycle lint
                                                              std Weak escape
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
shared = ref p                      # ref sharing
spawn { use(shared) }

# clone independent copy
backup = p.clone()

# Intra-task cycles: silently allowed
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

## Type system constraints

### Dup Type Attribute

`Dup` (Duplicable) is a compiler-managed type attribute meaning **shallow copy**: on assignment/parameter passing, what gets copied is the handle/token, with underlying data shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creates independent copy).

**Dup and Clone are orthogonal concepts**—Dup copies handles sharing data, Clone creates independent copies. A type can support both Dup and Clone, or just one.

| Type          | Dup                                       | Clone | Description                         |
| ------------- | ----------------------------------------- | ----- | ----------------------------------- |
| `&T`          | ✅ (copy token, multiple views point to same data) | ✅    | Read-only token                     |
| `ref T`       | ✅ (ref count +1, shared heap data)       | ✅    | Shared ownership (compiler auto-selects Rc/Arc) |
| String, Bytes | ✅ (internal ref count, copy handle shares underlying buffer) | ✅    | String/bytes                        |
| `&mut T`      | ❌ (linear, exclusive)                    | ❌    | Mutable token                       |
| `*T`          | ❌                                        | ❌    | Raw pointer                         |
| struct        | Derived (auto-derived when all fields are Dup) | ✅    | Struct                              |

**Primitive value types** (Int, Float, Bool, Char) have compiler-built-in value copy semantics—two values are completely independent, not shallow copy. They don't belong to the Dup type attribute but are handled natively by the compiler.

---

## Performance analysis

| Operation            | Cost    | Description                                 |
| -------------------- | ------- | ------------------------------------------- |
| Move                 | Zero    | Pointer movement                            |
| `&T` / `&mut T`      | Zero    | Zero-size type, disappears after compilation, zero runtime overhead |
| `ref` (not cross-task) | Low   | Compiled to Rc, non-atomic operation        |
| `ref` (cross-task)   | Medium  | Compiled to Arc, atomic operation           |
| `clone()`            | Depends on type | Small objects fast, large objects slow |
| `unsafe + *T`        | Zero    | Direct memory operation                     |

### Comparison

| Language       | Sharing mechanism               | Memory management   | Cycle handling                                  | Complexity |
| -------------- | ------------------------------ | ------------------- | ---------------------------------------------- | ---------- |
| Rust           | Arc / Mutex + borrow checking  | Compile-time check  | Manual Weak                                    | High       |
| Go             | chan / pointer                 | GC                  | GC                                             | Low        |
| C++            | shared_ptr                     | RAII                | weak_ptr                                       | Medium     |
| **YaoXiang**   | **ref + borrow tokens**        | **RAII**            | **Task boundary release / cross-task lint / std Weak** | **Low** |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetimes, borrow checking reduced to type system propositions. `&T` is copyable, `&mut T` is not copyable—two type attributes
3. **Powerful**: Can return references, store in structs, capture in closures—expression power on par with Rust
4. **Compiler intelligent**: ref auto-selects Rc/Arc, call site auto-selects borrowing
5. **Deterministic**: ref just keeps alive, won't silently become weak reference
6. **High performance**: Move is zero-copy, tokens are zero-overhead (zero-size types, disappear after compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: Tokens carry brand identifiers, function signatures returning references will show extra generic parameters
2. **ref runtime overhead**: Atomic operations have cost (but this is the inevitable cost of sharing)
3. **unsafe risk**: User must guarantee correctness
4. **Cross-task cycles are lint not compile error**: Unlike Rust's compile error, default warn, team needs to configure deny for quality gate

---

## Alternative Approaches

| Approach            | Why not chosen                                   |
| ------------------ | ------------------------------------------------ |
| GC                 | Has runtime overhead, unpredictable pauses       |
| Rust borrow checker | Needs lifetime `'a`, steep learning curve       |
| Pure Move           | Cannot handle concurrent sharing                 |
| No raw pointers     | Cannot do system-level programming               |
| Expose Rc/Arc to users | Implementation details pushed to user, increased cognitive burden |
| Bare-bones borrowing (v8) | "No escaping" strategy sacrifices key expressiveness like closure capture, returning references, etc. |

---

## Design Decision Log

| Decision                                  | Decision                                       | Reason                                                           | Date         |
| ----------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------- | ------------ |
| **Default**                               | Move (zero-copy)                              | High performance, zero overhead                                  | 2025-01-15   |
| **Sharing mechanism**                     | `ref` keyword, compiler auto-optimizes        | Simple for users, compiler does the work                         | 2025-01-15   |
| **Borrowing**                             | `&T`/`&mut T` as zero-size token types        | Type attributes (Dup/Linear) naturally derive permissions, unified type system | 2025-01-15   |
| **Borrow tokens**                         | Replace bare-bones borrowing, `&T` Dup, `&mut T` Linear | Eliminate special rules like "no escaping", support closure capture/return references/store in structs | 2026-05-29   |
| **Copying**                               | `clone()`                                      | Explicit semantics                                               | 2025-01-15   |
| **System-level**                          | `*T` + `unsafe`                                | Supports system programming                                      | 2025-01-15   |
| **Lifetimes**                             | Not implemented                                | Tokens are values, lifetime managed uniformly by Move/RAII, reduces borrowing to ownership | 2025-01-15   |
| **Rc/Arc**                                | Compiler auto-selects, invisible to users      | Reduced cognitive burden                                         | 2025-01-15   |
| **Cyclic references**                     | No checking within task, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint can be deny | 2025-01-16   |
| **Weak**                                  | Provided in standard library                  | Advanced users explicitly choose                                 | 2025-01-16   |
| **Consume analysis**                      | Deleted                                       | Mini borrow checker, not needed                                  | 2026-05-11   |
| **Ownership return**                      | Deleted                                       | `(T) -> T` signature is self-documenting                         | 2026-05-11   |
| **Empty state reuse**                     | Deleted (as feature)                           | Reassignment after Move is natural behavior                      | 2026-05-11   |
| **Inverse function / partial consume / three-layer mutability** | Deleted | Over-engineered                                                   | 2026-05-11   |
| **Lambda no implicit capture**            | Lambda only uses explicit parameters, no implicit capture of outer variables; context solidified via currying at creation point (SPEC §12.3) | Closure definition scope may be dead; solidified value at creation point (call site scope alive) is safe | 2026-06-16   |

### Version history

| Version | Major Changes                                                                                              | Date          |
| ------- | ---------------------------------------------------------------------------------------------------------- | ------------- |
| v1      | Initial draft: based on Rust ownership model                                                               | 2025-01-08    |
| **v8**  | **Deleted over-engineering (inverse function/partial consume/three-layer mutability/consume analysis/ownership return/empty state reuse), added bare-bones borrowing &T/&mut T** | **2026-05-11** |
| **v9**  | **Borrow token system replaces bare-bones borrowing, unified type system; token conflict detection corrected to Hoare propositions, see RFC-009a** | **2026-06-13** |

### Open issues

| Issue             | Description                          | Status                       |
| ----------------- | ------------------------------------ | ---------------------------- |
| Drop syntax       | Whether explicit `drop()` function needed | Under discussion             |
| Escape analysis algorithm | ref cross-task detection implementation | Under discussion             |
| Token conflict detection | Hoare logic propositions, see below | ✅ Resolved (details in RFC-009a) |

### Token conflict detection: Hoare logic proposition

Complete solution for token conflict detection is in
[RFC-009a: Token Lifetime Analysis—Based on Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md). Key points:

**Token liveness is a Hoare logic proposition.**
`{All conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}`—shares RFC-027's proof pipeline with type checking and user predicate verification. Compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), pipeline returns Proved
/ Disproved / Unproven.

**Borrow checking hasn't disappeared—it has been reduced in dimensionality.** `BorrowChecker` becomes
`BorrowPredicateEmitter`, generates propositions instead of performing checks. This is exactly parallel to "type checker": type checker generates type equality propositions, borrow proposition generator generates borrow propositions, same pipeline verifies both.

**Brand ID (`#42`) is `'a`.** Information is identical, encoding differs. `'a` is visible in type signatures, `#42` is internal to compiler. No new analysis invented—lowered lifetimes from type layer to proof layer.

**Algorithm summary** (details in RFC-009a):

- Brand tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumer, break cuts back-edges, structural analysis covers 95%+ scenarios (fast path)
- SMT logic cutting → only invoked when while + path conditions present (slow path, extremely rare)

---

## References

### YaoXiang Official Documentation

- [Language Specification](../../../reference/language-spec/index.md)
- [Design Manifesto](../../manifesto.md)
- [RFC-001 Concurrency Model](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [Tutorial](../../../tutorial/index.md)

### External references

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and Disposition

| Status        | Location                  | Description                       |
| ------------- | ------------------------- | --------------------------------- |
| **Draft**     | `docs/design/rfc/`        | Author draft, awaiting review     |
| **Under Review** | `docs/design/rfc/`     | Open for community discussion     |
| **Accepted** | `docs/design/accepted/`   | Becomes official design document  |
| **Rejected**  | `docs/design/rfc/`        | Preserved in RFC directory        |