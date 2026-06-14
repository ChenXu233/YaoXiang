---
title: "RFC-009: Ownership Model Design"
status: "Accepted"
author: "Chenxu"
created: "2025-01-08"
updated: "2026-06-13 (Token conflict detection corrected to Hoare propositions, body synchronized with RFC-009a)"
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core design—five concepts, one gradient**:

```
Peek/Modify   Take          Share            Copy              System-level
in place
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         Zero-copy     Auto-pick    Explicit        *T
  Zero-size      Default       Rc/Arc       deep copy       User
  token                                                responsible
  Type attrs
  naturally
  derive
  permissions
```

- **Move (default)**: assignment / parameter passing / return = ownership transfer, zero-copy, RAII auto-release
- **`&T` / `&mut T` (Borrow Tokens)**: zero-sized compile-time token types. `&T` is Duplicable (shared read), `&mut T` is Linear (exclusive mutable). Permissions are naturally derived from type attributes, with no special rules. Returnable, storable in structs, capturable by closures.
- **`ref` keyword**: cross-scope sharing. Compiler automatically picks Rc (no cross-task) or Arc (cross-task)
- **`clone()`**: explicit deep copy
- **`unsafe` + `*T`**: raw pointers, system-level escape hatch

**Eliminated complexity**:
- ❌ No lifetime `'a`
- ❌ No standalone borrow checker framework (borrow conflict reduced to Hoare propositions, sharing the proof pipeline with type checking)
- ❌ No GC
- ❌ No special rules like "must not escape" (tokens are ordinary types, scopes handled uniformly by the type system)
- ❌ Users don't need to know the difference between Rc/Arc (compiler picks automatically)

> **Programming burden**: `&T` is Duplicable, `&mut T` is non-Duplicable—two type attributes, zero special rules, fully automatic compiler.
> **Performance guarantee**: Move is zero-cost, tokens are zero-cost (zero-sized types, erased at compile time), ref is pay-as-you-go, no GC pauses.

## Motivation

### Why an ownership model?

| Language | Memory Management | Problems |
|----------|-------------------|----------|
| C/C++ | Manual | Memory leaks, dangling pointers, double free |
| Java/Python | GC | Latency variance, memory overhead, unpredictable pauses |
| Rust | Ownership + Borrow Checker | Steep learning curve for lifetime `'a` |
| **YaoXiang** | **Move + Token + ref** | **Simple, deterministic, no GC** |

### Design goals

```yaoxiang
# 1. Move by default (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p is no longer readable

# 2. &T / &mut T borrow tokens (zero-cost, type attrs naturally derive permissions)
print_info(p2)                 # Compiler auto-creates &Point token, released after use
shift(p2, 1.0, 1.0)           # Compiler auto-creates &mut Point token

# 3. ref = sharing (compiler auto-picks Rc/Arc)
shared = ref p2                # Hold across scopes
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy, independent

# 5. unsafe + *T = system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Core differences from Rust

| Feature | Rust | YaoXiang |
|---------|------|----------|
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero-copy)** |
| Borrowing | `&T`/`&mut T`, returnable, needs lifetime | **`&T`/`&mut T` zero-sized tokens, Dup/Linear type attrs naturally derive** |
| Sharing | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-picks Rc/Arc)** |
| Copying | `clone()` | `clone()` |
| Raw pointer | `*T` | `*T` |
| Lifetime | `'a` | ❌ None |
| Borrow checking | Global inference | **Type checker auto-generates borrow propositions, unified proof pipeline verifies** |
| Cyclic reference | Manual Weak | **Task-end unified release / cross-task lint / std lib Weak** |

---

## Proposal

### 1. Move (default ownership transfer)

```yaoxiang
# Rule: assignment / parameter passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p is no longer readable

# Variable can be reassigned (Python-style, no shadowing)
p = Point(3.0, 4.0)              # p rebound, type must match

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
- Zero-copy (compiler moves the pointer)
- Source binding unreadable after move (compile error)
- RAII: automatically released at scope end
- Function signature `(T) -> T` is self-documenting—consume T, return T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not "references"—they are "type-level proof of access permission."**

#### 2.1 Two type attributes

```
&T      →  Zero-sized, freezes source data (WriteToken forbidden while ReadToken is alive),
          Under freezing guarantee, multiple read views are safe → Duplicable (Dup)
&mut T  →  Zero-sized, exclusive read-write (any other token forbidden while WriteToken is alive),
          Under exclusive access, copying is meaningless → Linear (non-Dup)
```

**The causal relationship cannot be reversed: freezing is the cause, Dup is the result.** It's not that `&T` implements Dup so it can coexist—it's that the data is frozen (no mutation possible), making multiple read views safe, and only then can Dup be implemented. Treating Dup as the definition and conflict checking as "extra patch" is the wrong design.

#### 2.2 Basic usage

```yaoxiang
# Method side: declare parameter type, decides required permission
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

# Free function is the same
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist—Dup type
}
d = distance(p, p2)
```

#### 2.3 Why "must not escape" is not needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T`—only as parameters, cannot return, cannot store in structs. That was patching the "borrow" concept.

The token system doesn't need these rules. Tokens are **ordinary types**, following the same scope rules as all other types.

**Returning references—naturally supported**:

```yaoxiang
# ✅ Tokens propagate with return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Sub-token and parent token return together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returns to caller
print(px_ref)               # OK, token still in scope
```

**Storing in struct—naturally supported**:

```yaoxiang
# ✅ Struct carries token as a field
Window: Type = {
    target: Point,
    view: &Point,      # Token field—holds read-only view of target
}

# view's token is derived from target; Window owns both
# As long as Window lives, view token is valid
```

**Closure capture—naturally supported**:

```yaoxiang
# ✅ Closure captures tokens, just like any value
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    # Closure captures threshold's &Float token (Dup type, freely copyable into closure)
    items.filter(|p| p.x > threshold)
}

# RFC-009 v8 could not do this—v8 forbids closures from capturing borrows
```

**Cross-task—tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: tokens cannot cross tasks
}

# This is not a special rule—tokens are compile-time permission proofs; use ref for cross-task sharing
# Use ref if you need cross-task sharing
```

**Tokens cannot be `ref`**:

```yaoxiang
# ❌ Tokens are permission proofs, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token lifetime

Token lifetime is determined by **ordinary scope rules**, no lifetime parameter required:

- Tokens in function parameters: live during the call, released after
- Returned tokens: ownership transferred to caller
- Tokens stored in struct: live with the struct
- Tokens captured by closure: live with the closure

The compiler needs no `'a` annotation, because tokens are **values**, and value lifetime is uniformly managed by the ownership system (Move/RAII). **Reduces the borrow problem to an ownership problem.**

#### 2.5 Token conflict detection

Token conflict detection is a **Hoare logic proposition**, not a standalone flow-sensitive analysis.

```
{All conflicting ReadTokens dead} write(data) {WriteToken safely acquired}
```

It shares the proof pipeline from RFC-027 with type checking and user predicate verification. The compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), which the pipeline validates, returning Proved / Disproved / Unproven.

```yaoxiang
# ❌ &mut tokens are linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p is no longer readable
    p.x = 10.0                      # ❌ Compile error: WriteToken already moved
}

# ✅ &T tokens are Dup, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read tokens coexist
}
```

**Borrow checking has not disappeared—it has been reduced.** The existing `BorrowChecker` becomes `BorrowPredicateEmitter` (proposition generator), generating borrow propositions that share the same proof pipeline as other type propositions. This is perfectly parallel to the type checker concept: the type checker generates type-equality propositions, the borrow proposition generator generates borrow propositions, and the same pipeline validates. See [RFC-009a](../accepted/009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler internals: brand mechanism

Users never see brands. The compiler internally assigns a compile-time unique identifier to each token:

```
User-visible        Compiler internal
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Brand purposes:
- **Anti-counterfeiting**: tokens can only be obtained from owner capsules, not constructed out of thin air
- **Relation tracking**: when deriving `&Float` from `&Point` (field access), `&Float` carries a derived brand (`#N.field_x`), which the compiler can trace back to the parent token
- **Conflict detection**: same-source `WriteToken` and derived `ReadToken` cannot be simultaneously active

Brands completely disappear after monomorphization and inlining, and do not exist in the generated machine code. **Zero runtime overhead.**

#### 2.8 Automatic borrow selection rules

The compiler at the call site auto-selects by the following priority:

```
1. If the argument is used later → prefer creating a token (&T or &mut T, per method signature)
2. If the argument is not used later → Move
3. Priority matching order: &T < &mut T < Move
```

```yaoxiang
# Example: auto-selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → compiler creates &Point token
p.shift(1.0, 1.0) # shift declares &mut self → compiler creates &mut Point token
p2 = p           # Move, p is not used again
```

#### 2.9 Comparison with RFC-009 v8 bare-bones borrow

| Feature | Bare-bones borrow (v8) | Borrow Token (v9) |
|---------|------------------------|-------------------|
| Return reference | ❌ Hardcoded forbidden | ✅ Token propagates with return value |
| Store in struct | ❌ Hardcoded forbidden | ✅ Token as struct field |
| Closure capture | ❌ Hardcoded forbidden | ✅ Closure captures token values |
| Special rules | 3 (only param / no return / no store) | 0—type attrs naturally derive |
| Borrow checking | Dedicated cross-borrow check | Type checker flow-sensitive liveness analysis |
| Lifetime annotation | Not needed | Not needed |
| Runtime overhead | Zero | Zero (zero-sized type, erased at compile time) |
| Error message | "Borrow cannot escape" | "WriteToken(#3) already moved" (regular type error) |
| User mental model | Understand borrow's special status | `&T` is Duplicable, `&mut T` is non-Duplicable |

---

### 3. `ref` keyword (compiler auto-optimization)

`ref` is the only way to share across scopes. Whether the underlying implementation is Rc or Arc, the user does not need to care.

#### 3.1 Basic usage

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

**User mental model**: `ref` = shared holding. That's all you need to know.

#### 3.2 Compiler escape analysis: Rc vs Arc

```
ref's data-flow analysis:

Does not escape to other tasks → Rc (non-atomic ref count, low overhead)
Escapes to other tasks    → Arc (atomic ref count, thread-safe)
```

#### 3.3 Cycle detection strategy

```
Intra-task cycle → silently allowed.
  ├── Each task has a clear lifecycle boundary—resources (including ref cycles) are uniformly released when the task ends.
  ├── Long-running services should create child tasks per request/connection—child task end auto-collects, no accumulation leak.
  ├── ref keeps things alive, semantics are pure.
  └── Users have the right to build bidirectional strong references within a task (e.g., graph computing intermediate state).

Cross-task cycle → lint (default warn, configurable).
  ├── Program behavior is correct, no real leak (parent task end releases all child task resources).
  ├── But cross-task strong references mean blurred ownership boundaries, worth pausing to reconsider.
  ├── Default warn level, compile passes with hint.
  └── Teams can set to deny in project config, integrating into CI quality gate.
```

**Lint levels** (similar to Rust clippy):

| Level | Behavior | Scenario |
|-------|----------|----------|
| `allow` | Don't check | Personal projects |
| `warn` (default) | Compile passes, with hint | Development phase |
| `deny` | Compile fails | Team CI quality gate |
| `forbid` | Compile fails, cannot override | Organization-level mandatory rule |

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
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task cyclic reference
    }
}
```

**Project config example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycle rejected directly in CI
```

| Cycle type | Behavior | Reason |
|------------|----------|--------|
| Intra-task ref cycle | Not checked | User's right, released uniformly at task end |
| Cross-task ref cycle | Lint (default warn) | Reminder to reconsider, configurable to deny |

#### 3.4 Weak: provided by the standard library

```yaoxiang
use std.rc.Weak

# Advanced user explicitly chooses
a.next = ref b
b.prev = Weak.new(a.next)        # User explicitly controls which direction is weak
```

**`Weak` is not a language built-in, it's a standard library type.** Daily use of `ref` is sufficient. Advanced users who need fine-grained memory control manually introduce `Weak`.

#### 3.5 Borrow tokens vs `ref`

| | `&T` / `&mut T` | `ref` |
|---|----------------|-------|
| What it does | Peek / modify in place | Share holding |
| Range | With token value's scope | Cross-scope |
| Cost | Zero (zero-sized type) | Rc or Arc (compiler picks) |
| Escape | Yes (token propagates with return / struct / closure) | Meant to escape |
| Cross-task | No (tokens are compile-time permission proofs, cannot cross tasks) | Yes (compiler auto-picks Arc) |
| Cycle formation | Not involved | Intra-task silently allowed, cross-task lint |

---

### 4. `clone()` — explicit copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, do not affect each other
```

**When to use**: scenarios where you need to keep the original value and neither Move nor sharing is appropriate.

### 5. `unsafe` + raw pointer (system-level programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Constraints**:
- Only usable in `unsafe` blocks
- User guarantees no dangling, no use-after-free
- For FFI, memory manipulation and other system-level programming

---

### 6. Ownership gradient overview

```
  Borrow token (zero-cost)  Move (zero-cost)   Share (pay-as-you-go)  Copy
   │                        │                  │                      │
  &T Duplicable token   Default ownership   ref Rc/Arc           clone()
  &mut T Linear token   transfer chain     Compiler auto-pick    Explicit
                         consume-and-return  │                    deep copy
   │                        │                  │                      │
  Token value scope     Within scope       Cross-scope          Anytime
  Returnable / in struct  T -> T round-trip  ref cross-task → Arc Independent copy
  Capturable by closure  T -> Void consume  ref not cross-task → Rc
  Zero-sized, erased                           Intra-task cycle silent
                                               Cross-task cycle lint
                                               Std lib Weak escape
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
        self                            # Take it, modify, give it back
    }

    # Return reference: token propagates with return value
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# Closure captures token (v8 couldn't do this)
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# Comprehensive usage
p = Point(1.0, 2.0)
p.print()                           # &Point token
p.shift(1.0, 1.0)                   # &mut Point token
p = p.scale(2.0)                    # Move → round-trip
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

### Dup type attribute

`Dup` (Duplicable) is a compiler-auto-managed type attribute, meaning **shallow copy**: when assigning / passing parameters, the handle/token is copied and the underlying data is shared. This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creating independent copies).

**Dup and Clone are orthogonal concepts**—Dup copies the handle to share data, Clone creates an independent copy. A type can support both Dup and Clone, or only one of them.

| Type | Dup | Clone | Description |
|------|-----|-------|-------------|
| `&T` | ✅ (Copy token, multiple views to same data) | ✅ | Read-only token |
| `ref T` | ✅ (Ref count +1, share heap data) | ✅ | Shared holding (compiler auto-picks Rc/Arc) |
| String, Bytes | ✅ (Internal ref count, copy handle shares underlying buffer) | ✅ | String/bytes |
| `&mut T` | ❌ (Linear, exclusive) | ❌ | Mutable token |
| `*T` | ❌ | ❌ | Raw pointer |
| struct | Derived (auto-derived when all fields are Dup) | ✅ | Struct |

**Primitive value types** (Int, Float, Bool, Char) have assignment behavior that is the compiler's built-in value copy—the two values are fully independent, not shallow copy. They are not Dup type attributes; they are the compiler's native handling.

---

## Performance Analysis

| Operation | Cost | Description |
|-----------|------|-------------|
| Move | Zero | Pointer move |
| `&T` / `&mut T` | Zero | Zero-sized type, erased at compile time, zero runtime overhead |
| `ref` (not cross-task) | Low | Compiled to Rc, non-atomic operation |
| `ref` (cross-task) | Medium | Compiled to Arc, atomic operation |
| `clone()` | Depends on type | Fast for small objects, slow for large objects |
| `unsafe + *T` | Zero | Direct memory operation |

### Comparison

| Language | Sharing Mechanism | Memory Management | Cycle Handling | Complexity |
|----------|-------------------|-------------------|----------------|------------|
| Rust | Arc / Mutex + borrow checker | Compile-time check | Manual Weak | High |
| Go | chan / pointer | GC | GC | Low |
| C++ | shared_ptr | RAII | weak_ptr | Medium |
| **YaoXiang** | **ref + borrow tokens** | **RAII** | **Task boundary release / cross-task lint / std lib Weak** | **Low** |

---

## Trade-offs

### Pros

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent with RFC-010's `name: type = value`
2. **Simple**: No lifetime, borrow checking reduced to type system propositions. `&T` is Duplicable, `&mut T` is non-Duplicable—two type attributes
3. **Powerful**: Returnable references, storable in structs, capturable by closures—expressive power on par with Rust
4. **Compiler intelligence**: `ref` auto-picks Rc/Arc, call site auto-selects borrow
5. **Deterministic**: `ref` keeps things alive, won't quietly become weak
6. **High performance**: Move zero-copy, token zero-cost (zero-sized type, erased at compile time)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Cons

1. **Generic brand parameter contamination**: Tokens carry brand identifiers, function signatures returning references will reflect additional generic parameters
2. **`ref` runtime overhead**: Atomic operations have cost (but this is the inevitable price of sharing)
3. **`unsafe` risk**: User must guarantee correctness
4. **Cross-task cycle is lint, not compile error**: Not like Rust's compile error, default warn, requires team config deny to serve as quality gate

---

## Alternatives

| Approach | Why not chosen |
|----------|----------------|
| GC | Runtime overhead, unpredictable pauses |
| Rust borrow checker | Needs lifetime `'a`, steep learning curve |
| Pure Move | Cannot handle concurrent sharing |
| No raw pointer | Cannot do system-level programming |
| Expose Rc/Arc to users | Dumps implementation details on user, increases cognitive burden |
| Bare-bones borrow (v8) | "Must not escape" policy sacrifices key expressive powers like closure capture, return reference |

---

## Design Decision Record

| Decision | Choice | Reason | Date |
|----------|--------|--------|------|
| **Default** | Move (zero-copy) | High performance, zero-cost | 2025-01-15 |
| **Sharing mechanism** | `ref` keyword, compiler auto-optimizes | User simple, compiler responsible | 2025-01-15 |
| **Borrowing** | `&T`/`&mut T` as zero-sized token types | Type attributes (Dup/Linear) naturally derive permissions, unified type system | 2025-01-15 |
| **Borrow token** | Replaces bare-bones borrow, `&T` Dup, `&mut T` Linear | Eliminates special rules like "must not escape", supports closure capture / return reference / store in struct | 2026-05-29 |
| **Copying** | `clone()` | Explicit semantics | 2025-01-15 |
| **System-level** | `*T` + `unsafe` | Supports systems programming | 2025-01-15 |
| **Lifetime** | Not implemented | Tokens are values, lifetime uniformly managed by Move/RAII, reduces borrow to ownership problem | 2025-01-15 |
| **Rc/Arc** | Compiler auto-selects, invisible to user | Lower cognitive burden | 2025-01-15 |
| **Cyclic reference** | Intra-task not checked, cross-task lint (default warn) | Structured concurrency naturally guarantees, lint configurable to deny | 2025-01-16 |
| **Weak** | Standard library provides | Advanced user explicit choice | 2025-01-16 |
| **Consume analysis** | Removed | Mini borrow checker, not needed | 2026-05-11 |
| **Ownership round-trip** | Removed | `(T) -> T` signature is self-documenting | 2026-05-11 |
| **Empty state reuse** | Removed (as feature) | Reassignment after Move is natural behavior | 2026-05-11 |
| **Inverse function / partial consume / field three-level mutability** | Removed | Over-engineered | 2026-05-11 |

### Version history

| Version | Major changes | Date |
|---------|---------------|------|
| v1 | Initial draft: based on Rust ownership model | 2025-01-08 |
| **v8** | **Removed over-engineering (inverse function / partial consume / field three-level mutability / consume analysis / ownership round-trip / empty state reuse), added bare-bones borrow &T/&mut T** | **2026-05-11** |
| **v9** | **Borrow token system replaces bare-bones borrow, unifies type system; token conflict detection corrected to Hoare propositions, see RFC-009a** | **2026-06-13** |

### Pending topics

| Topic | Description | Status |
|-------|-------------|--------|
| Drop syntax | Whether explicit `drop()` function is needed | Pending |
| Escape analysis algorithm | ref's cross-task detection implementation | Pending |
| Token conflict detection | Hoare logic propositions, see below | ✅ Resolved (see RFC-009a for details) |

### Token conflict detection: Hoare logic propositions

The complete scheme for token conflict detection is in [RFC-009a: Token Lifetime Analysis—Based on Hoare Proof Pipeline](../accepted/009a-borrow-proof-pipeline.md). Key points:

**Token liveness is a Hoare logic proposition.** `{All conflicting ReadTokens dead} write(data) {WriteToken safely acquired}`—it shares the proof pipeline from RFC-027 with type checking and user predicate verification. The compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), and the pipeline returns Proved / Disproved / Unproven.

**Borrow checking has not disappeared—it has been reduced.** `BorrowChecker` becomes `BorrowPredicateEmitter`, generating propositions rather than executing checks. This is perfectly parallel to the "type checker" concept: the type checker generates type-equality propositions, the borrow proposition generator generates borrow propositions, and the same pipeline validates.

**Brand ID (`#42`) is `'a`.** Same information, different encoding. `'a` is visible in type signatures, `#42` is inside the compiler. No new analysis invented—lifetime has been reduced from the type layer to the proof layer.

**Algorithm summary** (see RFC-009a for details):
- Brand tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumer, break cuts back edges, structural analysis covers 95%+ scenarios (fast path)
- SMT logic cut → only called for `while` + path conditions (slow path, very rare)

---

## References

### YaoXiang official documentation

- [Language Spec](../language-spec.md)
- [Design Manifesto](../manifesto.md)
- [RFC-001 Concurrent Model & Error Handling](./001-concurrent-model-error-handling.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generic Type System Design](./011-generic-type-system.md)
- [YaoXiang Guide](../guides/YaoXiang-book.md)

### External references

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission for review |
| **Under review** | `docs/design/rfc/` | Open community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes formal design document |
| **Rejected** | `docs/design/rfc/` | Retained in RFC directory |