---
title: 'RFC-009: Ownership Model Design'
status: 'Accepted'
author: 'Chenxu'
created: '2025-01-08'
updated:
  '2026-06-13 (Token conflict detection revised to Hoare proposition, body synchronized with
  RFC-009a)'
issue: '#126'
---

# RFC-009: Ownership Model Design

## Summary

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core design — five concepts, one gradient**:

```
Peek/mutate in place   Take away     Share & hold     Clone a copy     System-level
        │               │               │               │               │
       &T             Move            ref            clone()          unsafe
      &mut T         zero-copy      compiler auto    explicit deep    *T
    zero-size token   default       picks Rc/Arc     copy        user is responsible
  type-attribute-
  derived authority
```

- **Move (default)**: assignment / parameter passing / return = ownership transfer, zero-copy, RAII
  auto-release
- **`&T` / `&mut T` (borrow tokens)**: zero-size compile-time token types. `&T` is Duplicable
  (shared read), `&mut T` is linear (exclusive mutable). Authority is derived naturally from type
  attributes — no special rules. Returnable, storable in structs.
- **`ref` keyword**: cross-scope sharing. The compiler auto-picks Rc (not crossing tasks) or Arc
  (crossing tasks).
- **`clone()`**: explicit deep copy
- **`unsafe` + `*T`**: raw pointers, system-level escape hatch

**Eliminated complexity**:

- ❌ No lifetime `'a`
- ❌ No independent borrow-checker framework (borrow conflicts reduce to Hoare propositions, sharing
  the proof pipeline with type checking)
- ❌ No GC
- ❌ No "no escape" special rules (tokens are ordinary types, scope is handled uniformly by the type
  system)
- ❌ Users don't need to know the difference between Rc / Arc (compiler auto-picks)

> **Programming burden**: `&T` is Duplicable, `&mut T` is not — two type attributes, zero special
> rules, fully automated by the compiler. **Performance guarantee**: Move is zero-cost, tokens are
> zero-cost (zero-size types, vanish after compilation), ref pays as needed, no GC pauses.

## Motivation

### Why is an ownership model needed?

| Language     | Memory management          | Problem                                               |
| ------------ | -------------------------- | ----------------------------------------------------- |
| C/C++        | Manual management          | Memory leaks, dangling pointers, double-free          |
| Java/Python  | GC                         | Latency jitter, memory overhead, unpredictable pauses |
| Rust         | Ownership + borrow checker | Steep learning curve of lifetime `'a`                 |
| **YaoXiang** | **Move + Token + ref**     | **Simple, deterministic, no GC**                      |

### Design goals

```yaoxiang
# 1. Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T borrow tokens (zero-cost, type attributes naturally derive authority)
print_info(p2)                 # compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # compiler auto-creates &mut Point token

# 3. ref = share (compiler auto-picks Rc/Arc)
shared = ref p2                # hold across scopes
spawn { use(shared) }          # compiler: crosses task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # deep copy, exclusive

# 5. unsafe + *T = system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Key differences from Rust

| Feature           | Rust                                       | YaoXiang                                                                                  |
| ----------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------- |
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)**                                                       |
| Borrowing         | `&T`/`&mut T`, returnable, needs lifetimes | **`&T`/`&mut T` zero-size tokens, Dup/Linear type attributes naturally derive authority** |
| Sharing mechanism | `Arc::new()` + manual Weak                 | **`ref` keyword (compiler auto-picks Rc/Arc)**                                            |
| Copy              | `clone()`                                  | `clone()`                                                                                 |
| Raw pointer       | `*T`                                       | `*T`                                                                                      |
| Lifetime          | `'a`                                       | ❌ None                                                                                   |
| Borrow checking   | Global inference                           | **Type checker auto-generates borrow propositions, unified proof pipeline validates**     |
| Cycle references  | Manual Weak                                | **Uniform release at task end / cross-task lint / std-lib Weak**                          |

---

## Proposal

### 1. Move (default ownership transfer)

```yaoxiang
# Rule: assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p can no longer be read

# Variable can be re-assigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p re-bound, type must match

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

**Properties**:

- Zero-copy (compiler moves pointer)
- Source binding unreadable after move (compile error)
- RAII: auto-release at scope end
- Function signature `(T) -> T` is itself documentation — consumes T, returns T

---

### 2. &T / &mut T (borrow tokens)

**Core principle: `&T` and `&mut T` are zero-size compile-time token types. They are not
"references" but "type-level proof of access authority."**

#### 2.1 Two type attributes

```
&T      →  zero-size, freezes source data (WriteToken forbidden while ReadToken is alive),
          under the freeze guarantee multiple read views are safe → Duplicable (Dup)
&mut T  →  zero-size, exclusive read-write (any other token forbidden while WriteToken is alive),
          copying under exclusive access is meaningless → linear (non-Dup)
```

**Causality cannot be reversed: freeze is the cause, Dup is the effect.** It is not that "`&T`
implements Dup, so they can coexist" — it is because the data is frozen (no mutation possible) that
multiple read views are safe, and thus Dup can be implemented. If Dup is taken as the definition and
conflict checking as an "extra patch," the design is wrong.

#### 2.2 Basic usage

```yaoxiang
# Method side: declare parameter type, decide required authority
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point token grants read authority
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point token grants write authority
    self.y = self.y + dy
}

# Caller side: compiler auto-picks borrow or Move
p = Point(1.0, 2.0)
p.print()                          # compiler auto-creates &Point token
p.shift(1.0, 1.0)                  # compiler auto-creates &mut Point token
p.print()                          # OK, previous token was released with shift's call

# Free functions are the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # two &Point tokens coexist — Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "no escape" is not needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — only usable as parameters, cannot be
returned, cannot be stored in structs. This was patching up the concept of "borrowing."

The token system does not need these rules. Tokens are **ordinary types**, following the same scope
rules as every other type.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Token propagates with the return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # child token and parent token returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # token returned to caller
print(px_ref)               # OK, token is still in scope
```

**Stored in structs — naturally supported**:

```yaoxiang
# ✅ Struct carries token as a field
Window: Type = {
    target: Point,
    view: &Point,      # token field — holds a read-only view of target
}

# view's token derives from target; Window holds ownership of both
# As long as Window exists, view's token is valid
```

#### 2.3 Closures and Lambdas use explicit parameters

A lambda is a function value — it can be returned, stored, and passed out of the current scope.
Therefore a lambda **does not implicitly capture outer local variables**. When outer data is needed,
pass it in via explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not affected by this rule — spawn is an immediately executing concurrent block, parent task blocks and waits
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # compile error: x is not in scope

# ✅ Correct way: explicit parameter
f = (x) => { x + 1 }
f(x)

# ✅ Second correct way: context baked in at creation point (currying) — closure only takes parameters, doesn't capture
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
evens = list.filter(nums, gt(threshold))
```

> Addendum (2026-08-17): The correct way to handle context dependency is currying-based baking, not
> capture. Once a closure escapes, its definition-scope may have died, so implicit capture is
> forbidden; however, the call site (creation point) scope must be alive, so baking the context as a
> value into the closure at that point is safe. See SPEC §12.3.

**`spawn { }` is not a function value.** A block marked `spawn`, like an `if`/`while` body, executes
immediately and completes while the parent stack frame is alive. The `spawn` body can freely access
outer variables.

**Cross-task — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ compile error: token cannot cross tasks
}

# This is not a special rule — tokens are compile-time authority proofs; for cross-task sharing use ref
# If you need cross-task sharing, use ref
```

**Tokens cannot be `ref`'d**:

```yaoxiang
# ❌ Tokens are authority proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ compile error: &T is not an ownable type
}
```

#### 2.4 Lifetime of tokens

A token's lifetime is determined by **ordinary scope rules**; no lifetime parameter is needed:

- Tokens in function parameters: alive during the call, released after the call ends
- Returned tokens: ownership transfers to the caller
- Tokens stored in structs: alive together with the struct

The compiler does not need `'a` annotations, because tokens are **values**, and the lifetime of
values is uniformly managed by the ownership system (Move/RAII). **The borrow problem is reduced to
an ownership problem.**

#### 2.5 Token conflict detection

Token conflict detection is a **Hoare-logic proposition**, not an independent flow-sensitive
analysis.

```
{all conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}
```

It shares the RFC-027 proof pipeline with type checking and user-predicate verification. The
compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`) and feeds them to the pipeline. The pipeline returns Proved / Disproved / Unproven.

```yaoxiang
# ❌ &mut token is linear and cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # ❌ compile error: WriteToken has been moved
}

# ✅ &T token is Dup, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is a Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read tokens coexist
}
```

**Borrow checking has not disappeared — it has been reduced.** The existing `BorrowChecker` becomes
a `BorrowPredicateEmitter`, generating propositions instead of performing checks. This is exactly
parallel to the concept of a "type checker": the type checker generates type-equality propositions,
the borrow-proposition generator generates borrow propositions, and a single pipeline validates
them. See [RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for the detailed design.

#### 2.7 Compiler internals: brand mechanism

Users never see brands. The compiler internally assigns each token a unique compile-time identifier:

```
User sees              Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a unique compile-time integer
&mut Point     →  WriteToken(Point, #M)   // #M is a unique compile-time integer
```

Purpose of brands:

- **Anti-forgery**: tokens can only be obtained from the owner capsule, cannot be constructed out of
  thin air
- **Correlation tracking**: when deriving `&Float` from `&Point` (field access), `&Float` carries a
  derived brand (`#N.field_x`), and the compiler can track back to the parent token
- **Conflict detection**: same-source `WriteToken` and derived `ReadToken` cannot both be alive

Brands fully vanish after monomorphization and inlining; they do not exist in the generated machine
code. **Zero runtime overhead.**

#### 2.8 Auto-borrow selection rules

On the caller side, the compiler auto-selects with the following priority:

```
1. If the argument is used afterwards → prefer creating a token (&T or &mut T, based on method signature)
2. If the argument is not used afterwards → Move
3. Match priority: &T < &mut T < Move
```

```yaoxiang
# Example: auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p is no longer used
```

#### 2.9 Comparison with RFC-009 v8 bare-bones borrowing

| Feature              | Bare-bones borrowing (v8)                 | Borrow tokens (v9)                                   |
| -------------------- | ----------------------------------------- | ---------------------------------------------------- |
| Return reference     | ❌ Hard-coded forbidden                   | ✅ Token propagates with return value                |
| Store in struct      | ❌ Hard-coded forbidden                   | ✅ Token as a struct field                           |
| Lambda explicit args | ❌ Hard-coded forbidden                   | ✅ Lambda uses explicit parameters                   |
| Special rules        | 3 (parameter-only / no return / no store) | 0 — type attributes derive naturally                 |
| Borrow check         | Dedicated cross-borrow checker            | Type checker's flow-sensitive liveness analysis      |
| Lifetime annotation  | Not needed                                | Not needed                                           |
| Runtime overhead     | Zero                                      | Zero (zero-size types, vanish after compilation)     |
| Error message        | "borrow cannot escape"                    | "WriteToken(#3) has been moved" (regular type error) |
| User mental model    | Understand "borrowing" as special         | `&T` is Duplicable, `&mut T` is not                  |

---

### 3. `ref` keyword (compiler auto-optimization)

`ref` is the only way to share across scopes. Whether the underlying is Rc or Arc, users don't need
to care.

#### 3.1 Basic usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # share, compiler auto-picks implementation

# Cross-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }           # compiler: crosses task → Arc
    spawn { use(data) }           # compiler: crosses task → Arc
}

# Single-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    use(data)                     # compiler: does not cross task → Rc
}
```

**User mental model**: `ref` = shared holding. That's enough.

#### 3.2 Compiler escape analysis: Rc vs Arc

```
ref's data-flow analysis:

does not escape to other tasks → Rc (non-atomic ref count, low overhead)
escapes to other tasks       → Arc (atomic ref count, thread-safe)
```

#### 3.3 Cycle-detection strategy

```
Intra-task cycle → silently allowed.
  ├── each task has a clear lifecycle boundary — all resources (incl. ref cycles) are uniformly released at task end.
  ├── long-running services should spawn child tasks per request/connection — child tasks auto-recycle on completion, no accumulating leak.
  ├── ref always keeps alive, semantics are not watered down.
  └── users have the right to build bidirectional strong references within a task (e.g., graph-computation intermediates).

Cross-task cycle → lint (default warn, configurable).
  ├── program behavior is correct, no real leak (parent task end releases all child task resources).
  ├── but cross-task strong references imply blurred ownership boundaries, worth pausing to reconsider.
  ├── default warn level, compile passes with a hint.
  └── teams can set deny in project config, integrate into CI quality gate.
```

**Lint levels** (similar to Rust clippy):

| Level            | Behavior                            | Scenario                   |
| ---------------- | ----------------------------------- | -------------------------- |
| `allow`          | No check                            | Personal project           |
| `warn` (default) | Compile passes, with hint           | Development phase          |
| `deny`           | Compile fails                       | Team CI quality gate       |
| `forbid`         | Compile fails, cannot be overridden | Organization-level mandate |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong references
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # cycle. Uniformly released at task end.
}

# Cross-task cycle: lint (default warn)
@block
parent_task: () -> Void = {
    shared_a = ref a
    shared_b = ref b
    spawn {
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task cycle
    }
}
```

**Project config example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # cross-task cycles are directly rejected in CI
```

| Cycle type           | Behavior            | Reason                                        |
| -------------------- | ------------------- | --------------------------------------------- |
| Intra-task ref cycle | No check            | User's authority, uniform release at task end |
| Cross-task ref cycle | lint (default warn) | Remind to reconsider, configurable to deny    |

#### 3.4 Weak: provided by the standard library

```yaoxiang
use std.weak

# Advanced users' explicit choice
a.next = ref b
b.prev = std.weak.new(a.next)   # user explicitly controls which direction is weak
```

**`Weak` is not built into the language, it is a standard-library type.** Daily use of `ref` is
enough. Advanced users who need fine-grained memory control manually import `Weak`.

> Revised 2026-08-03: implemented as a separate `std.weak` module (`std.rc` does not exist — `ref`
> is a language keyword, not a module; module path is uniformly `std.weak`, and the
> construction/upgrade entry points are `std.weak.new` / `std.weak.upgrade`). The originally
> envisioned `std.rc.Weak` did not land; this revision is authoritative.

#### 3.5 Borrow tokens vs `ref`

|            | `&T` / `&mut T`                                                  | `ref`                                        |
| ---------- | ---------------------------------------------------------------- | -------------------------------------------- |
| What       | Peek / mutate in place                                           | Shared holding                               |
| Range      | Lifetime of the token's value scope                              | Cross-scope                                  |
| Cost       | Zero (zero-size type)                                            | Rc or Arc (compiler picks)                   |
| Escape     | Yes (token propagates with return/struct/closure)                | Designed to escape                           |
| Cross-task | No (token is a compile-time authority proof, cannot cross tasks) | Yes (compiler auto-picks Arc)                |
| Cycle      | Not involved                                                     | Intra-task silently allowed, cross-task lint |

---

### 4. `clone()` — explicit copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # deep copy
# p and p2 are independent, no effect on each other
```

**When to use**: when the original value must be kept and neither Move nor sharing is appropriate.

### 5. `unsafe` + raw pointers (system-level programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # raw pointer
    (*ptr).x = 0.0                # dereference (user guarantees safety)
    ptr2 = ptr + 1                # pointer arithmetic
}
```

**Restrictions**:

- Only usable inside `unsafe` blocks
- User guarantees no dangling, no use-after-free
- Used for FFI, memory operations, and other system-level programming

---

### 6. Ownership gradient overview

```
  Borrow tokens (zero-cost)   Move (zero-cost)    Share (pay as needed)   Copy
       │                          │                     │                 │
  &T Dup token               Default ownership   ref Rc/Arc          clone()
  &mut T linear token        transfer chain      compiler auto-picks  explicit deep copy
       │                          │                     │                 │
  token value scope          within scope        cross-scope         anytime
  returnable/storable in     T -> T return       ref cross-task → Arc  independent copy
  struct                    T -> Void consume   ref not cross-task → Rc
  zero-size vanishes after  zero-size vanishes  intra-task cycle silent
  compilation                                     cross-task cycle lint
                                                 std-lib Weak escape
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

# Intra-task cycle: silently allowed
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

## Type-system constraints

### Dup type attribute

`Dup` (Duplicable) is a compiler-auto-managed type attribute, meaning **shallow copy**: what gets
copied on assignment / parameter passing is the handle / token, while the underlying data is shared.
This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy,
creating an independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies the handle to share data, Clone creates an
independent copy. A type can support both Dup and Clone, or only one of them.

| Type          | Dup                                                           | Clone | Note                                        |
| ------------- | ------------------------------------------------------------- | ----- | ------------------------------------------- |
| `&T`          | ✅ (copy token, multiple views point to same data)            | ✅    | Read-only token                             |
| `ref T`       | ✅ (ref count + 1, share heap data)                           | ✅    | Shared holding (compiler auto-picks Rc/Arc) |
| String, Bytes | ✅ (internal ref count, copy handle shares underlying buffer) | ✅    | String / bytes                              |
| `&mut T`      | ❌ (linear, exclusive)                                        | ❌    | Mutable token                               |
| `*T`          | ❌                                                            | ❌    | Raw pointer                                 |
| struct        | Derived (auto-derived when all fields are Dup)                | ✅    | Struct                                      |

**Primitive value types** (Int, Float, Bool, Char) use the compiler's built-in value-copy semantics
on assignment — the two values are fully independent, not shallow copies. They do not fall under the
Dup type attribute, but are handled natively by the compiler.

---

## Performance analysis

| Operation              | Cost           | Description                                                       |
| ---------------------- | -------------- | ----------------------------------------------------------------- |
| Move                   | Zero           | Pointer move                                                      |
| `&T` / `&mut T`        | Zero           | Zero-size type, vanishes after compilation, zero runtime overhead |
| `ref` (not cross-task) | Low            | Compiles to Rc, non-atomic operation                              |
| `ref` (cross-task)     | Medium         | Compiles to Arc, atomic operation                                 |
| `clone()`              | Type-dependent | Fast for small objects, slow for large                            |
| `unsafe + *T`          | Zero           | Direct memory operation                                           |

### Comparison

| Language     | Sharing mechanism          | Memory management  | Cycle handling                                             | Complexity |
| ------------ | -------------------------- | ------------------ | ---------------------------------------------------------- | ---------- |
| Rust         | Arc / Mutex + borrow check | Compile-time check | Manual Weak                                                | High       |
| Go           | chan / pointer             | GC                 | GC                                                         | Low        |
| C++          | shared_ptr                 | RAII               | weak_ptr                                                   | Medium     |
| **YaoXiang** | **ref + borrow tokens**    | **RAII**           | **Task-boundary release / cross-task lint / std-lib Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent
   with RFC-010's `name: type = value`.
2. **Simple**: No lifetimes; borrow checking reduces to type-system propositions. `&T` is
   Duplicable, `&mut T` is not — two type attributes.
3. **Powerful**: Returnable references, storable in structs, closure capture — expressive power on
   par with Rust.
4. **Smart compiler**: `ref` auto-picks Rc/Arc, caller side auto-selects borrow.
5. **Deterministic**: `ref` keeps alive, never silently weakens.
6. **High performance**: Move is zero-copy, tokens are zero-cost (zero-size types, vanish after
   compilation).
7. **Flexible**: `unsafe + *T` supports system-level programming.

### Disadvantages

1. **Generic brand-parameter pollution**: tokens carry brand identifiers; function signatures that
   return references reflect additional generic parameters.
2. **Runtime cost of `ref`**: atomic operations have a cost (but this is the inevitable price of
   sharing).
3. **`unsafe` risk**: users must guarantee correctness.
4. **Cross-task cycle is a lint, not a compile error**: unlike Rust which errors at compile time;
   default warn requires teams to configure deny to serve as a quality gate.

---

## Alternatives

| Option                    | Why not chosen                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------ |
| GC                        | Runtime overhead, unpredictable pauses                                                                 |
| Rust borrow checker       | Needs lifetime `'a`, steep learning curve                                                              |
| Pure Move                 | Cannot handle concurrent sharing                                                                       |
| No raw pointers           | Cannot do system-level programming                                                                     |
| Expose Rc/Arc to users    | Shoves implementation details to the user, increases cognitive load                                    |
| Bare-bones borrowing (v8) | The "no escape" strategy sacrifices key expressive power like closure capture and returning references |

---

## Decision log

| Decision                                                                  | Decision                                                                                                                                | Reason                                                                                                       | Date       |
| ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------- |
| **Default value**                                                         | Move (zero-copy)                                                                                                                        | High performance, zero overhead                                                                              | 2025-01-15 |
| **Sharing mechanism**                                                     | `ref` keyword, compiler auto-optimization                                                                                               | Simple for users, compiler handles it                                                                        | 2025-01-15 |
| **Borrowing**                                                             | `&T`/`&mut T` as zero-size token types                                                                                                  | Type attributes (Dup/Linear) naturally derive authority, unified type system                                 | 2025-01-15 |
| **Borrow tokens**                                                         | Replace bare-bones borrowing, `&T` Dup, `&mut T` Linear                                                                                 | Eliminate "no escape" special rules, support closure capture / return reference / store in struct            | 2026-05-29 |
| **Copy**                                                                  | `clone()`                                                                                                                               | Explicit semantics                                                                                           | 2025-01-15 |
| **System-level**                                                          | `*T` + `unsafe`                                                                                                                         | Support systems programming                                                                                  | 2025-01-15 |
| **Lifetime**                                                              | Not implemented                                                                                                                         | Tokens are values, lifetime uniformly managed by Move/RAII; reduces borrow to ownership                      | 2025-01-15 |
| **Rc/Arc**                                                                | Compiler auto-selects, invisible to user                                                                                                | Lower cognitive load                                                                                         | 2025-01-15 |
| **Cycle references**                                                      | No check intra-task, cross-task lint (default warn)                                                                                     | Structured concurrency naturally guarantees, lint can be configured to deny                                  | 2025-01-16 |
| **Weak**                                                                  | Provided by standard library                                                                                                            | Advanced users' explicit choice                                                                              | 2025-01-16 |
| **Consumption analysis**                                                  | Removed                                                                                                                                 | Mini borrow checker, not needed                                                                              | 2026-05-11 |
| **Ownership return**                                                      | Removed                                                                                                                                 | `(T) -> T` signature is itself documentation                                                                 | 2026-05-11 |
| **Null-state reuse**                                                      | Removed (as a feature)                                                                                                                  | Reassignment after Move is natural behavior                                                                  | 2026-05-11 |
| **Inverse function / partial consumption / three-level field mutability** | Removed                                                                                                                                 | Over-engineering                                                                                             | 2026-05-11 |
| **Lambda no implicit capture**                                            | Lambda uses only explicit parameters, no implicit capture of outer variables; context baked via currying at creation point (SPEC §12.3) | Definition-scope may have died after escape; baking values at creation point (call-site scope alive) is safe | 2026-06-16 |

### Version history

| Version | Main change                                                                                                                                                                                             | Date           |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| v1      | Initial draft: based on Rust ownership model                                                                                                                                                            | 2025-01-08     |
| **v8**  | **Removed over-engineering (inverse function / partial consumption / three-level field mutability / consumption analysis / ownership return / null-state reuse), added bare-bones borrowing &T/&mut T** | **2026-05-11** |
| **v9**  | **Borrow-token system replaces bare-bones borrowing, unified type system; token conflict detection revised to Hoare propositions, see RFC-009a**                                                        | **2026-06-13** |

### Open issues

| Issue                     | Description                                     | Status                     |
| ------------------------- | ----------------------------------------------- | -------------------------- |
| Drop syntax               | Whether an explicit `drop()` function is needed | TBD                        |
| Escape analysis algorithm | Cross-task detection for `ref`                  | TBD                        |
| Token conflict detection  | Hoare-logic proposition, see below              | ✅ Resolved (see RFC-009a) |

### Token conflict detection: Hoare-logic proposition

The full scheme for token conflict detection is in
[RFC-009a: Token Lifetime Analysis — Based on the Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md).
Key points:

**Token liveness is a Hoare-logic proposition.**
`{all conflicting ReadTokens are dead} write(data) {WriteToken safely acquired}` — it shares the
RFC-027 proof pipeline with type checking and user-predicate verification. The compiler
auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`), and the pipeline returns Proved / Disproved / Unproven.

**Borrow checking has not disappeared — it has been reduced.** `BorrowChecker` becomes
`BorrowPredicateEmitter`, generating propositions instead of performing checks. This is exactly
parallel to "type checker": the type checker generates type-equality propositions, the
borrow-proposition generator generates borrow propositions, and a single pipeline validates them.

**Brand IDs (`#42`) are exactly `'a`.** The information is identical, the encoding is different.
`'a` is visible in type signatures, `#42` is internal to the compiler. No new analysis is invented —
lifetime is moved from the type layer to the proof layer.

**Algorithm outline** (see RFC-009a for details):

- Brand-tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumers, break cuts back-edges, structural analysis covers 95%+ of
  scenarios (fast path)
- SMT logical cut → only invoked with `while` + path conditions (slow path, extremely rare)

---

## References

### YaoXiang official documentation

- [Language Specification](../../../reference/language-spec/index.md)
- [Design Manifesto](../../manifesto.md)
- [RFC-001 Concurrent Model](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [tutorial/](../../../tutorial/index.md)

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
| **Accepted**     | `docs/design/accepted/` | Becomes a formal design document           |
| **Rejected**     | `docs/design/rfc/`      | Retained in the RFC directory              |
