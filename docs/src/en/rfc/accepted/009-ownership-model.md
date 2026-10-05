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

**Core design — five concepts, one gradient**:

```
Peek/in-place edit    Take away         Shared holding     Clone one copy      System-level
    │                 │                 │                  │                  │
   &T              Move               ref              clone()             unsafe
  &mut T           Zero copy          Compiler auto     Explicit deep copy  *T
  Zero-size token  Default             selects Rc/Arc                       User responsible
  Type attributes
  naturally derive
  permissions
```

- **Move (default)**: assignment / parameter passing / return = ownership transfer, zero-copy,
  automatic RAII release
- **`&T` / `&mut T` (borrow tokens)**: zero-size compile-time token types. `&T` is Duplicable
  (shared read-only), `&mut T` is linear (exclusive mutable). Permissions are naturally derived from
  type attributes, with no special rules. Returnable, storable in structs.
- **`ref` keyword**: cross-scope sharing. The compiler automatically selects Rc (non-cross-task) or
  Arc (cross-task).
- **`clone()`**: explicit deep copy
- **`unsafe` + `*T`**: raw pointers, system-level escape hatch

**Complexity eliminated**:

- ❌ No lifetime `'a`
- ❌ No separate borrow checking framework (borrow conflicts reduced to Hoare propositions, sharing
  the proof pipeline with type checking)
- ❌ No GC
- ❌ No special rules like "no escape" (tokens are ordinary types, scope handled uniformly by the
  type system)
- ❌ Users do not need to know the difference between Rc and Arc (compiler selects automatically)

> **Programming burden**: `&T` is Duplicable, `&mut T` is not — two type attributes, zero special
> rules, fully automatic compiler. **Performance guarantee**: Move is zero overhead, tokens are zero
> overhead (zero-size types, disappear after compilation), `ref` is pay-as-you-go, no GC pauses.

## Motivation

### Why is an ownership model needed?

| Language     | Memory management           | Problem                                                     |
| ------------ | --------------------------- | ----------------------------------------------------------- |
| C/C++        | Manual                      | Memory leaks, dangling pointers, double free                |
| Java/Python  | GC                          | Latency fluctuations, memory overhead, unpredictable pauses |
| Rust         | Ownership + borrow checking | Steep learning curve for lifetime `'a`                      |
| **YaoXiang** | **Move + Token + ref**      | **Simple, deterministic, no GC**                            |

### Design goals

```yaoxiang
# 1. Default Move (zero copy)
p = Point(1.0, 2.0)
p2 = p                         # Move, p can no longer be read

# 2. &T / &mut T borrow tokens (zero overhead, type attributes naturally derive permissions)
print_info(p2)                 # Compiler auto-creates &Point token, released when done
shift(p2, 1.0, 1.0)            # Compiler auto-creates &mut Point token

# 3. ref = sharing (compiler auto-selects Rc/Arc)
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

### Core differences from Rust

| Feature           | Rust                                       | YaoXiang                                                                                    |
| ----------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------- |
| Default semantics | Borrow `&T` (requires explicit `.clone()`) | **Move (value passing, zero copy)**                                                         |
| Borrow            | `&T`/`&mut T`, returnable, needs lifetime  | **`&T`/`&mut T` zero-size tokens, Dup/Linear type attributes naturally derive permissions** |
| Sharing           | `Arc::new()` + manual Weak                 | **`ref` keyword (compiler auto-selects Rc/Arc)**                                            |
| Copy              | `clone()`                                  | `clone()`                                                                                   |
| Raw pointer       | `*T`                                       | `*T`                                                                                        |
| Lifetime          | `'a`                                       | ❌ None                                                                                     |
| Borrow checking   | Global inference                           | **Type checker auto-generates borrow propositions, unified proof pipeline validates**       |
| Cycles            | Manual Weak                                | **Task end uniform release / cross-task lint / standard library Weak**                      |

---

## Proposal

### 1. Move (default ownership transfer)

```yaoxiang
# Rule: assignment / parameter passing / return = Move, zero copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move, p can no longer be read

# Variable can be reassigned (Python style, no shadowing)
p = Point(3.0, 4.0)              # p rebound, type must match

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

**Characteristics**:

- Zero copy (compiler moves the pointer)
- Original binding unreadable after move (compile error)
- RAII: automatically released when scope ends
- Function signature `(T) -> T` is itself documentation — consume T, return T

---

### 2. &T / &mut T (borrow tokens)

**Core principle: `&T` and `&mut T` are zero-size compile-time token types. They are not
"references", but "type-level proofs of access permission".**

#### 2.1 Two type attributes

```
&T      →  Zero size, freezes source data (WriteToken forbidden while ReadToken lives),
          freezing guarantee makes multiple read-only views safe → Duplicable (Dup)
&mut T  →  Zero size, exclusive read-write (any other token forbidden while WriteToken lives),
          exclusive access makes copying meaningless → Linear (non-Dup)
```

**Causality cannot be reversed: freezing is the cause, Dup is the result.** It is not that `&T`
implements Dup so it can coexist — it is because data is frozen (no mutation possible) that multiple
read-only views are safe, and Dup can be implemented. If Dup is taken as the definition and conflict
checking as an "extra patch", the design is wrong.

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

#### 2.3 Why "no escape" rules are not needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T` — only usable as parameters, cannot be
returned, cannot be stored in structs. This was patching the concept of "borrow".

The token system does not need these rules. Tokens are **ordinary types**, following the same scope
rules as all other types.

**Returning references — naturally supported**:

```yaoxiang
# ✅ Tokens propagate along with return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # Sub-token and parent token return together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # Token returned to caller
print(px_ref)               # OK, token still in scope
```

**Storing in structs — naturally supported**:

```yaoxiang
# ✅ Struct carries tokens as fields
Window: Type = {
    target: Point,
    view: &Point,      # Token field — holds a read-only view of target
}

# The view token derives from target, Window holds ownership of both
# As long as Window exists, the view token is valid
```

#### 2.3 Closures and Lambdas with explicit parameters

Lambdas are function values — they can be returned, stored, and escape the current scope. Therefore
Lambdas **do not implicitly capture outer local variables**. When outer data is needed, pass it in
via explicit parameters:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is not affected by this rule — spawn is an immediately executed concurrent block, parent task blocks waiting
shared = ref data
spawn { use(shared) }

# ❌ Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x not in scope

# ✅ Correct way: explicit parameter
f = (x) => { x + 1 }
f(x)

# ✅ Correct way two: context固化 (currying) at creation point — closure only takes parameters, does not capture
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
evens = list.filter(nums, gt(threshold))
```

> Supplement (2026-08-17): The proper solution for context dependence is currying固化, not capture.
> After a closure escapes, its definition-site scope may have died, so implicit capture is
> forbidden; but the call site (creation point) scope must be alive, and固化 of context as a value
> at that point is safe. See SPEC §12.3.

**spawn { } is not a function value.** A block marked spawn, like if/while bodies, executes
immediately and completes while the parent stack frame is alive. spawn bodies can access outer
variables normally.

**Cross-task — tokens cannot cross threads**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: token cannot cross task
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

#### 2.4 Token lifetime

Token lifetime is determined by **ordinary scope rules**, with no lifetime parameter needed:

- Tokens in function parameters: live during the call, released after the call ends
- Returned tokens: ownership transfers to the caller
- Tokens stored in structs: live together with the struct

The compiler does not need `'a` annotation, because tokens are **values**, and value lifetime is
uniformly managed by the ownership system (Move/RAII). **Reduce the borrow problem to an ownership
problem.**

#### 2.5 Token conflict detection

Token conflict detection is a **Hoare logic proposition**, not a separate flow-sensitive analysis.

```
{All conflicting ReadTokens dead} write(data) {WriteToken safely acquired}
```

It shares the RFC-027 proof pipeline with type checking and user predicate verification. The
compiler auto-generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`) and feeds them into the pipeline for validation. The pipeline returns Proved /
Disproved / Unproven.

```yaoxiang
# ❌ &mut tokens are linear, cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move, p can no longer be read
    p.x = 10.0                      # ❌ Compile error: WriteToken has been moved
}

# ✅ &T tokens are Dup type, can be freely copied
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK, &T is Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK, two read-only tokens coexist
}
```

**Borrow checking has not disappeared — it has been reduced.** The existing `BorrowChecker` becomes
a `BorrowPredicateEmitter` (proposition generator), and the generated borrow propositions share the
same proof pipeline as other type propositions. This is completely parallel to the type checker
concept: the type checker generates type equality propositions, the borrow proposition generator
generates borrow propositions, the same pipeline validates. See
[RFC-009a](009a-borrow-proof-pipeline.md) for detailed design.

#### 2.7 Compiler internals: brand mechanism

Users never encounter brands. The compiler internally assigns a compile-time unique identifier to
each token:

```
User-visible           Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N is a compile-time unique integer
&mut Point     →  WriteToken(Point, #M)   // #M is a compile-time unique integer
```

Uses of brands:

- **Anti-counterfeiting**: tokens can only be obtained from the owner capsule, cannot be fabricated
  out of thin air
- **Association tracking**: when deriving `&Float` (field access) from `&Point`, `&Float` carries a
  derived brand (`#N.field_x`), and the compiler can trace back to the parent token
- **Conflict detection**: same-source `WriteToken` and derived `ReadToken` cannot be simultaneously
  active

Brands completely disappear after monomorphization and inlining, and do not exist in the generated
machine code. **Zero runtime overhead.**

#### 2.8 Auto borrow selection rules

The caller-side compiler automatically selects by the following priority:

```
1. If the argument is used later → prefer to create a token (&T or &mut T, per method signature)
2. If the argument is not used later → Move
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

| Feature                | Bare-bones borrow (v8)                       | Borrow token (v9)                                     |
| ---------------------- | -------------------------------------------- | ----------------------------------------------------- |
| Return reference       | ❌ Hardcoded forbidden                       | ✅ Token propagates with return value                 |
| Store in struct        | ❌ Hardcoded forbidden                       | ✅ Token as struct field                              |
| Lambda explicit params | ❌ Hardcoded forbidden                       | ✅ Lambda uses explicit parameters                    |
| Special rules          | 3 (only as parameter / no return / no store) | 0 — type attributes naturally derive                  |
| Borrow checking        | Dedicated cross-borrow checker               | Type checker flow-sensitive liveness analysis         |
| Lifetime annotation    | Not needed                                   | Not needed                                            |
| Runtime overhead       | Zero                                         | Zero (zero-size type, disappears after compilation)   |
| Error message          | "Borrow cannot escape"                       | "WriteToken(#3) has been moved" (ordinary type error) |
| User mental model      | Understand the special status of "borrow"    | `&T` Duplicable, `&mut T` not Duplicable              |

---

### 3. ref keyword (compiler auto-optimization)

`ref` is the only way to share across scopes. Whether the underlying implementation is Rc or Arc,
the user does not need to care.

#### 3.1 Basic usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Share, compiler auto-selects implementation

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
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks        → Arc (atomic reference counting, thread safe)
```

#### 3.3 Cycle detection strategy

```
Intra-task cycle → silently allowed.
  ├── Each task has a clear lifecycle boundary — when the task ends, all resources (including ref cycles) are uniformly released.
  ├── Long-running services should create sub-tasks per request/connection — sub-tasks are automatically reclaimed when ended, no accumulating leak.
  ├── ref is always alive, semantics are not watered down.
  └── Users have the right to build bidirectional strong references within a task (e.g. intermediate state of graph computation).

Cross-task cycle → lint (default warn, configurable).
  ├── Program behavior is correct, won't actually leak (when parent task ends, child task resources are fully released).
  ├── But cross-task strong references mean blurred ownership boundaries, worth stopping to reconsider.
  ├── Default warn level, compile passes with hint.
  └── Team can set to deny in project config, include in CI quality gate.
```

**Lint levels** (similar to Rust clippy):

| Level            | Behavior                         | Scenario                          |
| ---------------- | -------------------------------- | --------------------------------- |
| `allow`          | No check                         | Personal project                  |
| `warn` (default) | Compiles, with hint              | Development stage                 |
| `deny`           | Compile failure                  | Team CI quality gate              |
| `forbid`         | Compile failure, cannot override | Organization-level mandatory rule |

```yaoxiang
# Intra-task cycle: silently allowed, bidirectional strong reference
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
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task cycle reference
    }
}
```

**Project configuration example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles rejected directly on CI
```

| Cycle type           | Behavior            | Reason                                          |
| -------------------- | ------------------- | ----------------------------------------------- |
| Intra-task ref cycle | No check            | User's right, uniformly released when task ends |
| Cross-task ref cycle | lint (default warn) | Remind to reconsider, configurable deny         |

#### 3.4 Weak: provided by the standard library

```yaoxiang
use std.weak

# Advanced user explicit choice
a.next = ref b
b.prev = std.weak.new(a.next)   # User explicitly controls which direction is weak
```

**`Weak` is not a language builtin, but a standard library type.** For daily use `ref` is enough.
Advanced users who need fine-grained memory control manually introduce `Weak`.

> 2026-08-03 revision: Implemented as an independent `std.weak` module (`std.rc` does not exist —
> `ref` is a language keyword not a module; module path uniformly `std.weak`, construction/upgrade
> entry points `std.weak.new` / `std.weak.upgrade`). The original draft's envisioned `std.rc.Weak`
> did not land; this revision prevails.

#### 3.5 Borrow token vs ref

|              | `&T` / `&mut T`                                                       | `ref`                                        |
| ------------ | --------------------------------------------------------------------- | -------------------------------------------- |
| What it does | Peek / in-place edit                                                  | Shared holding                               |
| Scope        | Follows the token value's scope                                       | Cross-scope                                  |
| Cost         | Zero overhead (zero-size type)                                        | Rc or Arc (compiler selects)                 |
| Escape       | Allowed (token propagates via return / struct / closure)              | Designed to escape                           |
| Cross-task   | Forbidden (token is compile-time permission proof, cannot cross task) | Allowed (compiler auto-selects Arc)          |
| Cycles       | Not involved                                                          | Intra-task silently allowed, cross-task lint |

---

### 4. clone() — explicit copy

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent, don't affect each other
```

**When to use**: scenarios where you need to keep the original value and Move or sharing is not
appropriate.

### 5. unsafe + raw pointers (system-level programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Restrictions**:

- Can only be used inside `unsafe` blocks
- User guarantees no dangling, no use-after-free
- Used for FFI, memory operations, and other system-level programming

---

### 6. Ownership gradient overview

```
  Borrow token (zero overhead)   Move (zero overhead)   Share (pay-as-you-go)  Copy
   │                              │                      │                      │
  &T Duplicable token         Default ownership transfer  ref Rc/Arc         clone()
  &mut T linear token         Chain consumption回流     Compiler auto-selects Explicit deep copy
   │                              │                      │                      │
  Token value scope            Within scope            Cross-scope           Any time
  Returnable / store in struct  T -> T 回流             ref cross-task → Arc  Independent copy
  Zero-size disappears after compilation  T -> Void consume  ref not cross-task → Rc
  Zero-size disappears after compilation                     Intra-task cycle silent
                                                             Cross-task cycle lint
                                                             Standard library Weak escape
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
        self                            # Take it, modify, return it
    }

    # Return reference: token propagates with return value
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# Lambda with explicit parameters
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
b.prev = ref a                      # Cycle, uniformly released when task ends

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type system constraints

### Dup type attribute

`Dup` (Duplicable) is a compiler-auto-managed type attribute, meaning **shallow copy**: when
assigning / passing, what is copied is the handle/token, the underlying data is shared. This forms a
three-level gradient with Move (ownership transfer) and Clone (explicit deep copy, creates
independent copy).

**Dup and Clone are orthogonal concepts** — Dup copies the handle to share data, Clone creates an
independent copy. A type can support both Dup and Clone, or only one of them.

| Type          | Dup                                                             | Clone | Note                                          |
| ------------- | --------------------------------------------------------------- | ----- | --------------------------------------------- |
| `&T`          | ✅ (copy token, multiple views point to same data)              | ✅    | Read-only token                               |
| `ref T`       | ✅ (ref count +1, share heap data)                              | ✅    | Shared holding (compiler auto-selects Rc/Arc) |
| String, Bytes | ✅ (internal ref count, copy handle to share underlying buffer) | ✅    | String/byte                                   |
| `&mut T`      | ❌ (linear, exclusive)                                          | ❌    | Mutable token                                 |
| `*T`          | ❌                                                              | ❌    | Raw pointer                                   |
| struct        | Derived (see "Derivation rules" below, #398)                    | ✅    | Struct                                        |
| tuple         | Derived (element-wise, same as struct rules, #398)              | ✅    | Tuple                                         |

**Primitive value types** (Int, Float, Bool, Char) have compiler-builtin value-copy assignment
behavior — two values are completely independent, not shallow copies. They do not belong to the Dup
type attribute, but are native compiler handling.

#### Derivation rules (#398 finalized)

"All fields are Dup then auto-derive" cannot be literally executed — primitive fields (Int, etc.)
are themselves not Dup, so `{ x: Int, y: Int }` would be misjudged as Move. The executable form:

1. **Copyable field set** = ValueCopy (Int / Float / Bool / Char / Range) ∪ Dup (`&T`, `ref T`,
   String / Bytes, function value (#352), already-Dup composite types);
2. **struct**: all fields are in the copyable field set → derive Dup; **any** field is Linear
   (`&mut T`) or Move (nested Move struct / Vec, Dict and other containers / resources) → keep
   overall Move (do not introduce "partially copyable" intermediate state — semantically it must be
   transferred);
3. **tuple**: same rules as struct, element-wise judgment; empty tuple (unit) i.e. `Void`;
4. **Derivation is recursive**: when a field is a named type (e.g. `target: Point`), expand its
   definition then judge, `A = { b: B }` follows B's derivation result; circular aliases
   conservatively fall to Move by depth limit;
5. **Not in this row's scope** (still Move, separate case): containers (Vec / Dict / Set / Option /
   Result / Array) and enum.

---

## Performance analysis

| Operation              | Cost            | Description                                                         |
| ---------------------- | --------------- | ------------------------------------------------------------------- |
| Move                   | Zero            | Pointer move                                                        |
| `&T` / `&mut T`        | Zero            | Zero-size type, disappears after compilation, zero runtime overhead |
| `ref` (not cross-task) | Low             | Compiles to Rc, non-atomic operation                                |
| `ref` (cross-task)     | Medium          | Compiles to Arc, atomic operation                                   |
| `clone()`              | Depends on type | Small objects fast, large objects slow                              |
| `unsafe + *T`          | Zero            | Direct memory operation                                             |

### Comparison

| Language     | Sharing mechanism             | Memory management  | Cycle handling                                                      | Complexity |
| ------------ | ----------------------------- | ------------------ | ------------------------------------------------------------------- | ---------- |
| Rust         | Arc / Mutex + borrow checking | Compile-time check | Manual Weak                                                         | High       |
| Go           | chan / pointer                | GC                 | GC                                                                  | Low        |
| C++          | shared_ptr                    | RAII               | weak_ptr                                                            | Medium     |
| **YaoXiang** | **ref + borrow token**        | **RAII**           | **Task boundary release / cross-task lint / standard library Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types, not special language features. Fully consistent
   with RFC-010's `name: type = value`.
2. **Simple**: No lifetime, borrow checking reduced to type system propositions. `&T` Duplicable,
   `&mut T` not — two type attributes.
3. **Powerful**: Can return references, store in structs, capture closures — expressive power on par
   with Rust.
4. **Smart compiler**: ref auto-selects Rc/Arc, caller side auto-selects borrow.
5. **Deterministic**: ref is always alive, won't silently become weak.
6. **High performance**: Move zero copy, tokens zero overhead (zero-size types, disappear after
   compilation).
7. **Flexible**: `unsafe + *T` supports system-level programming.

### Disadvantages

1. **Generic brand parameter contagion**: tokens carry brand identifiers, function signatures that
   return references show additional generic parameters.
2. **ref runtime overhead**: atomic operations have cost (but this is the inevitable cost of
   sharing).
3. **unsafe risk**: users must guarantee correctness.
4. **Cross-task cycles are lint, not compile error**: unlike Rust, default is warn, team needs to
   configure deny to be a quality gate.

---

## Alternatives

| Option                 | Why not chosen                                                                                         |
| ---------------------- | ------------------------------------------------------------------------------------------------------ |
| GC                     | Runtime overhead, unpredictable pauses                                                                 |
| Rust borrow checker    | Requires lifetime `'a`, steep learning curve                                                           |
| Pure Move              | Cannot handle concurrent sharing                                                                       |
| No raw pointer         | Cannot do system-level programming                                                                     |
| Expose Rc/Arc to user  | Dumps implementation details to user, increases cognitive load                                         |
| Bare-bones borrow (v8) | The "no escape" strategy sacrifices key expressive capabilities like closure capture, return reference |

---

## Design decision record

| Decision                                                                  | Decision                                                                                                                                      | Reason                                                                                                   | Date       |
| ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ---------- |
| **Default value**                                                         | Move (zero copy)                                                                                                                              | High performance, zero overhead                                                                          | 2025-01-15 |
| **Sharing mechanism**                                                     | `ref` keyword, compiler auto-optimization                                                                                                     | User simple, compiler responsible                                                                        | 2025-01-15 |
| **Borrow**                                                                | `&T`/`&mut T` as zero-size token types                                                                                                        | Type attributes (Dup/Linear) naturally derive permissions, unified type system                           | 2025-01-15 |
| **Borrow token**                                                          | Replaces bare-bones borrow, `&T` Dup, `&mut T` Linear                                                                                         | Eliminates special rules like "no escape", supports closure capture / return reference / store in struct | 2026-05-29 |
| **Copy**                                                                  | `clone()`                                                                                                                                     | Explicit semantics                                                                                       | 2025-01-15 |
| **System-level**                                                          | `*T` + `unsafe`                                                                                                                               | Supports system programming                                                                              | 2025-01-15 |
| **Lifetime**                                                              | Not implemented                                                                                                                               | Tokens are values, lifetime uniformly managed by Move/RAII, reduces borrow to ownership problem          | 2025-01-15 |
| **Rc/Arc**                                                                | Compiler auto-selects, user invisible                                                                                                         | Lower cognitive load                                                                                     | 2025-01-15 |
| **Cycle reference**                                                       | Intra-task no check, cross-task lint (default warn)                                                                                           | Structured concurrency natural guarantee, lint can be deny                                               | 2025-01-16 |
| **Weak**                                                                  | Standard library provided                                                                                                                     | Advanced user explicit choice                                                                            | 2025-01-16 |
| **Consumption analysis**                                                  | Removed                                                                                                                                       | Mini borrow checker, not needed                                                                          | 2026-05-11 |
| **Ownership return**                                                      | Removed                                                                                                                                       | `(T) -> T` signature is itself documentation                                                             | 2026-05-11 |
| **Empty state reuse**                                                     | Removed (as a feature)                                                                                                                        | Reassignment after Move is natural behavior                                                              | 2026-05-11 |
| **Inverse function / partial consumption / field three-level mutability** | Removed                                                                                                                                       | Over-engineering                                                                                         | 2026-05-11 |
| **Lambda no implicit capture**                                            | Lambda only uses explicit parameters, does not implicitly capture outer variables; context is固化 via currying at creation point (SPEC §12.3) | Closure definition-site scope may have died;固化 value at creation point (call site scope alive) is safe | 2026-06-16 |

### Version history

| Version | Major change                                                                                                                                                                                       | Date           |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| v1      | Initial draft: based on Rust ownership model                                                                                                                                                       | 2025-01-08     |
| **v8**  | **Remove over-engineering (inverse function / partial consumption / field three-level mutability / consumption analysis / ownership return / empty state reuse), add bare-bones borrow &T/&mut T** | **2026-05-11** |
| **v9**  | **Borrow token system replaces bare-bones borrow, unified type system; token conflict detection corrected to Hoare proposition, see RFC-009a**                                                     | **2026-06-13** |

### Pending issues

| Issue                     | Description                                  | Status                                 |
| ------------------------- | -------------------------------------------- | -------------------------------------- |
| Drop syntax               | Whether explicit `drop()` function is needed | To be discussed                        |
| Escape analysis algorithm | ref's cross-task detection implementation    | To be discussed                        |
| Token conflict detection  | Hoare logic proposition, see below           | ✅ Resolved (see RFC-009a for details) |

### Token conflict detection: Hoare logic proposition

The complete solution for token conflict detection is in
[RFC-009a: Token lifetime analysis — based on Hoare proof pipeline](009a-borrow-proof-pipeline.md).
Core points:

**Token liveness is a Hoare logic proposition.**
`{All conflicting ReadTokens dead} write(data) {WriteToken safely acquired}` — shares the RFC-027
proof pipeline with type checking and user predicate verification. The compiler auto-generates
borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), and
the pipeline returns Proved / Disproved / Unproven.

**Borrow checking has not disappeared — it has been reduced.** `BorrowChecker` becomes
`BorrowPredicateEmitter`, generating propositions rather than performing checks. This is completely
parallel to the "type checker" concept: the type checker generates type equality propositions, the
borrow proposition generator generates borrow propositions, the same pipeline validates.

**Brand ID (`#42`) is `'a`.** The information is exactly the same, only the encoding is different.
`'a` is visible in type signatures, `#42` is internal to the compiler. Nothing new was invented —
lifetime is reduced from the type layer to the proof layer.

**Algorithm summary** (see RFC-009a for details):

- Brand tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from consumer, break cuts back edge, structural analysis covers 95%+ of cases
  (fast path)
- SMT logic cut → only invoked when while + path conditions (slow path, extremely rare)

---

## References

### YaoXiang official documentation

- [Language specification](../../reference/language-spec/index.md)
- [Design manifesto](../../explanation/manifesto.md)
- [RFC-009a: Token lifetime analysis — based on Hoare proof pipeline](009a-borrow-proof-pipeline.md)
- [RFC-010 Unified type syntax](010-unified-type-syntax.md)
- [tutorial/ tutorials](../../tutorial/index.md)

### External references

- [Rust ownership model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang message passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and destination

| Status        | Location                | Description                              |
| ------------- | ----------------------- | ---------------------------------------- |
| **Draft**     | `docs/design/rfc/`      | Author draft, awaiting submission review |
| **Reviewing** | `docs/design/rfc/`      | Open community discussion and feedback   |
| **Accepted**  | `docs/design/accepted/` | Becomes formal design document           |
| **Rejected**  | `docs/design/rfc/`      | Kept in RFC directory                    |
