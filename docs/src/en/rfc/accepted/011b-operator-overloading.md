---
title: 'RFC-011b: Operator Overloading and Interface-Driven Operators'
status: 'Accepted'
author: 'Chenxu'
created: '2026-09-15'
updated: '2026-09-22'
group: 'rfc-011'
issue: '#341'
---

# RFC-011b: Operator Overloading and Interface-Driven Operators

> **References**:
>
> - [RFC-011: Generics System Design](011-generic-type-system.md) — Type constraint
>   `T: Add + Multiply`, associated types
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](011a-interface-implementation.md) —
>   Interface declaration/instantiation mechanism
> - [RFC-009: Ownership Model Design](009-ownership-model.md) — `&mut T` linear token
> - [RFC-004: Multi-position Joint Binding of Curried Methods](004-curry-multi-position-binding.md)
>   — `f[0]` position binding syntax
> - [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — Sum type = a record whose fields
>   all return their own type
> - [RFC-013: Error Code Specification](013-error-code-specification.md) — `Result` belongs to std
>   positioning, E108x
> - [RFC-010b: Pattern Matching Completeness (Variant Deconstruction and Exhaustiveness)](010b-pattern-matching-completeness.md)
>   — Variant deconstruction (dependency)

## Summary

Add **operator overloading** to YaoXiang, so that `a + b` / `a == b` / `a[i]` / `e?` can be
implemented by user-defined types, and turn the **operator constraints** (`T: Add + Multiply`) in
the constraint syntax already written in RFC-011 from a **paper capability** into an actionable
mechanism (the `Zero` in the same clause is not an operator — see open questions).

The design uses a **three-layer separation of responsibilities**: a fixed operator→method mapping
table (Layer 0), a name-based dispatch base (Layer 1, reusing the existing `method_bindings`), and
the interface contract layer (Layer 2, for generics constraints). Operators' **precedence and
associativity remain language-fixed**; users only overload the semantics.

First batch scope: the seven interfaces `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal`
`Index`. Arithmetic interfaces use **three type parameters** `(Self, R, O)` — the result type `O` is
made explicit, so that heterogeneous operations like `1 + 2.5` (returns `Float`) and `point * 2.0`
(scaling) can be expressed. `Equal` **defaults to auto-derive** (records whose fields are all
comparable automatically receive a field-wise `==`); explicit instantiation can override.

`Try` (interface-ization of `?`) is moved entirely to phase 2: it depends on RFC-010 (construction)
and RFC-010b (deconstruction) being in place, and the interface shape is not yet finalized (see open
questions).

**No new syntax, no new keywords** — fully reusing the existing mechanisms from RFC-011a (interface
declaration / instantiation / external method declaration / overloading).

## Motivation

### Why this feature is needed

#### 1. RFC-011's core example depends on it, and the current state is worse than "unfulfillable"

RFC-011 (accepted) uses operator names as type constraints in 8 places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

Across all RFCs, not a single one defines where `Add` / `Multiply` come from or how `+` is bound to
them. Empirical testing confirms the situation is worse: `T: Add` **fails to compile today** —
constraint resolution queries the old trait table (`trait_data.rs`), which has no `Add`. The
constraint names `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` are similarly dangling (handling of
those is covered in "Coordination with Other RFCs").

#### 2. User-defined types cannot participate in basic operations (empirically verified)

```
error [E6007] Runtime error: type mismatch in comparison Eq:
    Struct { type_id: TypeId(0), ... } vs Struct { type_id: TypeId(0), ... }
```

Knock-on effect: `list.contains(list_of_structs, p)` is **completely unusable** — it internally
depends on `==`. Yet `(1, 2) == (1, 2)` and `[1] == [1]` have **always worked** via runtime
element-wise comparison — structs are the only gap.

#### 3. `?` hard-codes the type name into the compiler, blocking `Result` from moving to std

The `?` implementation hard-codes both the type name and the variant number:

```rust
// typecheck: hard-codes construction of the Result type
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hard-codes the group name and variant number
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` is made **interface-driven**, any type implementing
the interface (including user-defined ones) can be used with `?`, and `Result` can belong to std
(RFC-013 already has the "std library `Result(T, Error)`" positioning).

Note this is only half the problem: the `ok(...)` / `err(...)` / `some(...)` construction forms are
also currently welded into the parser (language spec §1.4.2 lists them as "constructors recognized
by the parser"). "Moving `Result` to std" requires solving both halves — `?` and the constructors —
together, both in phase 2 of this RFC.

#### 4. User-defined containers cannot be indexed

The type checking for `Index` is a hard-coded whitelist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// others → "containers not recognized at the type layer are refused, not silently accepted"
```

Any container type defined by the user causes `c[0]` to unconditionally fail.

#### 5. The implementation of `%` violates the published documentation

Empirical testing shows `-7 % 3` returns `-1` (truncated remainder). Yet the operator precedence
table in the language reference (`reference/index.md`) already states "`* / %` are
multiplication/division/**modulo**" — the documentation promises modulo, the implementation gives
remainder. This is not a design change but a **defect where the implementation contradicts the
documentation**, which this RFC fixes along the way.

### Existing semi-finished foundation

Investigation reveals that most of the mechanism is **already in place**; what's missing is the
wiring:

| Mechanism                                             | Location                                                                                                                          | State                                                                                              |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Interface declaration `(Self: Type) -> Type`          | RFC-011a Phase 1                                                                                                                  | ✅ Runnable                                                                                        |
| Interface instantiation + external method declaration | RFC-011a Phase 2–3                                                                                                                | ✅ Runnable (with unit tests)                                                                      |
| Method dispatch `method_bindings["Type.method"]`      | `expressions.rs` (registered in `environment.rs`, queried in `expressions.rs` for call resolution and field-lookup fallback path) | ✅ Runnable                                                                                        |
| Associated types (= interface type parameters)        | RFC-011 §3.1; RFC-011a adopted this approach                                                                                      | ✅ Mechanism decided                                                                               |
| Type family evaluation `AssociatedTypeDef`            | `dependent_types.rs`                                                                                                              | ✅ Production use: `std.assert`'s `IsTrue`                                                         |
| `Equal` / `Dup` / `Clone` / `Debug` trait             | Registered in `trait_data.rs`                                                                                                     | ⚠️ `Equal` has zero consumers; old auto-derive only registers the signature without implementation |
| Operator → method-name mapping                        | —                                                                                                                                 | ❌ Does not exist                                                                                  |

**Conclusion**: this is not building a feature from scratch, but wiring together existing parts and
writing it down.

## Proposal

### Core design: three-layer separation of responsibilities

```
Layer 0  Fixed mapping table (language-level constant, not user-modifiable)
         + → add    == → equal    [] → index    ? → residual
         │  Built in at compile time, does not participate in type inference, not exposed to users
         ▼
Layer 1  Dispatch base (look up method by name and invoke)        ← reuses existing mechanism
         method_bindings["Point.add"]
         ▼
Layer 2  Interface contract (for generics constraints)
         Add / Equal / Index / Try …
         Makes the T: Add constraint hold and serves as the precondition for the operator
```

**Rationale for layering**:

- **Layers 0 and 1 make operators "usable"**, Layer 2 makes operators "constrainable". Merging them
  would cause any method named `add` to be invoked by `+`, decoupling RFC-011's `T: Add` from
  operators.
- **Layer 1 is not newly created**: `method_bindings` already looks up by the `Type.method` key (the
  fallback path after field lookup fails); operators go through the same path.
- **Layer 2 is the precondition**: before allowing an operator, it **must** be confirmed that the
  type implements the corresponding interface (see the §Equal auto-derive exception — derivation and
  registration are the same criterion).

### Names and registration: two independent channels

**Operators query the "interface implementation registration table", not the regular name
resolution.**

- The registration table is owned by the core and aggregates registrations from two places: the
  core's default registrations for primitive types (`Add(Int, Int, Int)`, etc.) and users' interface
  instantiations written in type bodies (`Add(Point, Point, Point)`). Regardless of which module the
  implementation code physically resides in, registrations flow into the same table, and `+` / `==`
  / `[]` only see this table.
- A **same-named binding** in a local module (e.g., the type-level `Add` from RFC-011 §5.2 for Peano
  arithmetic `Add: (A: Type, B: Type) -> Type = match ...`) goes through the **name-lookup
  channel**, only affecting resolution of the name `Add` within that module, **not touching the
  registration table, not affecting operator usability**. Same name, different thing, no shadowing.

This also answers the question "do RFC-011's type-level `Add` and this RFC's interface `Add`
conflict?": they do not. Type-level `Add` is a purely type-level computation (Zero/Succ are types,
not values; there will never be a value-level `Zero + Succ(...)`), and the value-level operator
interface is two layers of the same name.

### Layer 0: fixed mapping table

| Operator            | Interface                                                 | Method            | First batch |
| ------------------- | --------------------------------------------------------- | ----------------- | ----------- |
| `+`                 | `Add`                                                     | `add`             | ✅          |
| `-`                 | `Subtract`                                                | `subtract`        | ✅          |
| `*`                 | `Multiply`                                                | `multiply`        | ✅          |
| `/`                 | `Divide`                                                  | `divide`          | ✅          |
| `%`                 | `Modulo`                                                  | `modulo`          | ✅          |
| `==` `!=`           | `Equal`                                                   | `equal`           | ✅          |
| `[]`                | `Index`                                                   | `index`           | ✅          |
| `?`                 | `Try` (four methods, finalized in phase 2)                | `is_failure` etc. | ✅ Landed   |
| `<` `<=` `>` `>=`   | — (keep native instruction)                               | —                 | ❌          |
| `and` `or`          | — (short-circuit is language semantics, not overloadable) | —                 | ❌          |
| 5 bitwise operators | —                                                         | —                 | ❌          |
| unary `-` `!`       | —                                                         | —                 | ❌          |

**Interface names use full spellings rather than abbreviations** (`Multiply` rather than `Mul`):
this keeps consistency with `T: Add + Multiply + Zero` in the body of RFC-011 and **does not modify
accepted RFCs**.

**`%` uses the `Modulo` semantics** (mathematical modulo, result sign follows the divisor). The
Motivation section has already confirmed that the current remainder implementation contradicts the
published documentation (`reference/index.md`'s "multiplication/division/modulo"), and this item is
treated as a defect fix — no compatibility period is reserved.

Implementation note: the type-check whitelist for `%` currently only includes Int/Float (different
from the `+` whitelist of Int/Float/String/List); see Appendix A.2 for empirical records.

### Layer 2: interface definitions

#### Arithmetic interfaces (three type parameters)

```yaoxiang
Add: (Self: Type, R: Type, O: Type) -> Type = {
    add: (self: &Self, other: &R) -> O
}

Subtract: (Self: Type, R: Type, O: Type) -> Type = {
    subtract: (self: &Self, other: &R) -> O
}

Multiply: (Self: Type, R: Type, O: Type) -> Type = {
    multiply: (self: &Self, other: &R) -> O
}

Divide: (Self: Type, R: Type, O: Type) -> Type = {
    divide: (self: &Self, other: &R) -> O
}

Modulo: (Self: Type, R: Type, O: Type) -> Type = {
    modulo: (self: &Self, other: &R) -> O
}
```

Each of the three type parameters has a distinct role:

- `Self`: the left-operand type (the receiver, `&Self` borrowed; see RFC-009 borrowing tokens and
  RFC-011a receiver convention);
- `R`: the right-operand type — **left and right are allowed to be heterogeneous**;
- `O`: **the result type**, declared explicitly.

**Why the result type must be an explicit parameter** (rather than hard-coding `-> Self`):

1. If `-> Self` were hard-coded, then the method for `Add(Int, Float)` would be forced to return
   `Int`, making it impossible to correctly express `1 + 2.5`;
2. Once the result type is explicit, RFC-011 §8.3's promotion type family
   `Add: (A, B) -> Type = match (A, B) { (Int, Float) => Float, ... }` and this RFC's interface
   registration **become two views of the same table** — the core registration
   `Add(Int, Float, Float)` is exactly the `(Int, Float) => Float` row, and every user instantiation
   adds a row to this table;
3. The `Index` interface is already three-parameter `(Self, Key, Value)`, with the return type
   `Value` as a parameter — once arithmetic interfaces add `O` the entire operator interface family
   has a uniform shape, and hard-coding `Self` would be the odd one out.

**Core default registrations** (native instruction path, not through method calls):
`Add(Int, Int, Int)`, `Add(Int, Float, Float)`, `Add(Float, Int, Float)`,
`Add(Float, Float, Float)`, `Add(String, String, String)` (concatenation),
`Add(List(T), List(T), List(T))` (element-wise concatenation), and so on. The five arithmetic
interfaces do the same for primitive types. With the core registration in place, `1 + 2.5` moves
from today's compile error (the whitelist requires both sides to have the same type) to a legal
operation returning `3.5: Float`.

**Constraint syntax sugar**: `T: Add` ≜ already-registered `Add(T, T, T)` — same-type
self-composition with the result still being that type. The matrix-multiplication example in
RFC-011, in which `a * b + c` has type `T` throughout, is asking for exactly this meaning.
`T: Equal` similarly ≜ `Equal(T, T)`.

**Heterogeneous example** — vector scaling:

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,
    Multiply(Point, Float, Point),
}

Point.multiply: (self: &Point, other: &Float) -> Point =
    Point(self.x * other, self.y * other)

main: () -> Void = {
    p = Point(1.0, 2.0)
    q = p * 3.0              # Point(3.0, 6.0)
}
```

#### Equality interface (default auto-derive)

```yaoxiang
Equal: (Self: Type, R: Type) -> Type = {
    equal: (self: &Self, other: &R) -> Bool
}
```

**`Equal` defaults to auto-derive**, under five rules:

1. **Default derive**: when a record type is defined, if all fields are comparable (primitive types,
   `String`, or records/tuples/lists that are themselves comparable, and the type does not contain
   `&mut` fields), the compiler automatically generates **field-wise comparison** `==` along a
   native code path, with no user-visible method generated. User-defined types are naturally
   comparable with no ceremony.
2. **Explicit override**: if the type body writes `Equal(Point, Point)` and provides a `Point.equal`
   method, the user's version is used (e.g., tolerance-based float comparison), and auto-derive is
   skipped.
3. **Field not comparable**: auto-derive fails, `==` is unavailable, and the diagnosis indicates
   which field is the culprit; the user can still hand-write `Equal` for custom comparison (e.g.,
   compare a function field by name).
4. **Precondition**: a type containing `&mut T` linear tokens (recursive in fields) does not
   participate in comparison — a linear token is consumed upon a single read, so two values cannot
   be extracted simultaneously for comparison. Note the precondition is "non-linear" rather than
   "Dup": primitive value types (Int/Float/Bool/Char), per RFC-011 §2.4, do not fall under Dup (they
   are compiler-built-in value copies); using Dup as the precondition would mistakenly reject
   `Point { x: Float, y: Float }`.
5. **Same criterion for constraints**: solving `T: Equal` and gating `==` follow the same "check
   registration or structural derivation" rule — constraints and operators always give the same
   answer.

**Consistency rationale**: `(1, 2) == (1, 2)` and `[1] == [1]` already use runtime element-wise
comparison today (the comparison whitelist in `executor.rs` includes Tuple/List/Array), and records
are the only excluded compound type. Auto-derive does not add a new silent default — it completes an
existing language-internal behavior for the last remaining piece.

**Retirement of old mechanism**: the old `Equal` auto-derive in `trait_data.rs` (which only
registered the signature, had no implementation code, and had zero consumers) is disabled; all
`Equal` judgments (default registration for primitive types, structural derivation, explicit
instantiation) go through the interface registration table. The old trait table retains its existing
responsibilities for Clone/Dup/Debug (their names don't overlap with operator interfaces; future
unification is a separate discussion).

#### Index interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` as a type parameter rather than an associated-type member**: the open question in RFC-011a
has been settled with the conclusion that "associated types are implemented via generics interface
parameters" (`Iterator: (Item: Type) -> Type` is an isomorphic precedent), and there is no need to
introduce a `type` member syntax.

**Ownership note**: `index` returns a full `Value` from a `&Self` borrow. The standard library's
`list.get: (&Vec(A), Int) -> A` is already in the same shape; this interface **receives the same
treatment as the existing std**; the precise semantics of "extracting a full value from a borrow"
for move-semantic element types will be handled uniformly when RFC-009 is fully enforced, and this
RFC does not invent new rules for that.

**Multi-position indexing is handled via tuple packing + overloading**, not by introducing variadic
interfaces:

```yaoxiang
// One-dimensional container
List(T) instantiates Index(List(T), Int, T)                   → arr[0]

// Multi-dimensional container
Grid    instantiates Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └─ Key is a tuple

// Two instantiations with different signatures → coexist
```

**Same-name interfaces for the same type allow multiple instantiations**, distinguished by the
signature of the methods they inject, and they coexist following the RFC-011a method-level
overloading rules. RFC-011a's overloading is only specified down to the method level; this item adds
one layer above it: the legitimacy of same-name interface instantiation coexistence is determined by
whether the expanded method signatures conflict (E1097 if they do, sharing the same source as the
field/method namespace rules).

#### Propagation interface `Try` (finalized and landed in phase 2)

The interface-ized `?` is named `Try`, with a four-method shape (finalized in phase 2 on
2026-09-22):

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    is_failure: (self: &Self) -> Bool,
    success:    (self: &Self) -> T,
    residual:   (self: &Self) -> E,
    from_error: (E) -> Self,
}
```

- **Semantic division of labor**: `is_failure` judges success/failure, `success` extracts the
  success payload, `residual` extracts the failure payload, and `from_error` serves as the bridge
  for cross-type propagation (when `T` of `f()?` ≠ outer `U`, reconstruct a failure value from `E`).
  The lowering of `?` uniformly generates a chain of four method calls — if `is_failure(t)` is true
  then `Ret from_error(residual(t))`, otherwise the expression's value is `success(t)`; no more
  hand-written variant-checking sequences, and `Result` / `Option` (stdlib yx implementations) and
  user-defined Try types all take the same path.
- **Dead branches**: the failure arm of `success` and the success arm of `residual` are
  contractually unreachable, so the implementation uses `assert(false)` to diverge (`assert` returns
  `Never`, the explosion principle `Never <: T` permits it, type-system.md §2.2).
- **Checking**: typecheck consults the interface implementation registration table (nominal match on
  the Self position, abstract entries instantiated by the scrutinee argument); the outer function's
  return type must also implement `Try` and the `E` position must be able to catch the failure value
  (E1081/E1082/E1083 semantics are interface-ized accordingly).
- **Moving `Result` to std**: the type definitions and Try implementations of `Result` / `Option`
  are migrated to `std/result.yx` / `std/option.yx` (pure YaoXiang), and the native `ok`/`err`
  constructors are retired — the variant construction syntax `Result(T, E).ok(v)` becomes the only
  construction path (the hard-coded-constructor problem is solved at the same time as the parser
  special-case for constructors is retired). The Try residual type of `Option` takes `Void`
  (corresponding to the NoneT semantics of Rust's Try experiment).

### Examples

#### User-defined types: arithmetic and equality work out of the box

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,
    Add(Point, Point, Point),
}

Point.add: (self: &Point, other: &Point) -> Point =
    Point(self.x + other.x, self.y + other.y)

main: () -> Void = {
    a = Point(1.0, 2.0)
    b = Point(3.0, 4.0)
    c = a + b                   # Point(4.0, 6.0)
    println(a == b)             # false —— Equal auto-derived, no instantiation needed
    println(a == a)             # true
}
```

#### Custom equality (overriding auto-derive)

```yaoxiang
Vec3: Type = {
    x: Float,
    y: Float,
    z: Float,
    Equal(Vec3, Vec3),
}

# Float comparison with tolerance, overrides field-wise auto-derive
Vec3.equal: (self: &Vec3, other: &Vec3) -> Bool =
    abs(self.x - other.x) < 0.000001
    and abs(self.y - other.y) < 0.000001
    and abs(self.z - other.z) < 0.000001
```

#### Custom container indexing

```yaoxiang
Box: (T: Type) -> Type = {
    data: List(T),
    Index(Box(T), Int, T)
}

Box.index: (T: Type)(self: &Box(T), key: &Int) -> T =
    list.get(self.data, key)

main: () -> Void = {
    b = Box([10, 20, 30])
    println(b[1])               // 20
}
```

#### Generics constraints can finally be fulfilled

```yaoxiang
// RFC-011's example is now implementable
// T: Add ≜ Add(T, T, T) is registered; T: Multiply ≜ Multiply(T, T, T) is registered
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

(The `Zero` / `One` in RFC-011's signature example `T: Add + Multiply + Zero` are not in this RFC's
scope — they are constant members, not operators, and "interface members without a receiver" have no
precedent in RFC-011a; see open questions.)

### Syntax changes

**No new syntax, no new keywords**. All capabilities are composed of existing mechanisms:

| Capability              | Existing mechanism reused                                            |
| ----------------------- | -------------------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                                     |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                            |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)                  |
| Associated types        | RFC-011 §3.1 (interface type parameters)                             |
| Multi-position indexing | Existing tuple-packing parse + instantiation-level overloading rules |
| Operator precedence     | **Language-fixed**, not open to user customization                   |

## Detailed design

### Impact on the type system

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` (`Try`
in phase 2).

**Interface implementation registration table**: the central decision point for operators.
Aggregating core default registrations (primitive types) and user instantiations (the
`ImplementationProof` produced by `check_interface_instantiation`), the gating check for `+` / `==`
/ `[]` and the constraint solving in `T: Add` (the `check_trait_bounds` in `bounds.rs`) **query the
same table** — there is no gap where "the constraint says yes, the operator says no".

**Structural derivation of `Equal`**: a structural rule (all fields comparable ⇒ comparable) is
layered on top of the registration table; primitive types are covered by core default registrations,
and the recursion is closed. The `Equal` registration and the old auto-derive in `trait_data.rs` are
disabled.

**Separation of operator queries from name resolution**: operators like `+` only consult the
registration table, not regular name resolution; local same-name bindings (such as the type-level
`Add` family) do not affect operators (see §Names and registration).

**Orphan rule**: an operator implementation can only be written in **the defining module of the
type** — `Int` is defined in the core, so `Int`'s registration can only be written in the core;
`Point` is defined in a user module, so only its definer can register it. All types follow the same
rule; built-in types have no special privileges.

### Runtime behavior

**Zero runtime overhead**: operators determine their call target at compile time (static dispatch).
For primitive types (`Int`/`Float`/`String`/`List`), the **native instruction fast path** is
retained and does not go through interface dispatch; auto-derived struct `==` generates native
field-wise comparison; `==` / `!=` at `Any` (dynamic) positions keep existing runtime comparison
(RFC-036 testing framework `assert_eq` depends on this behavior and is not affected).

**`%` semantic fix**: `-7 % 3` changes from `-1` (truncated remainder) to `2` (mathematical modulo).
Three locations are updated in sync: the interpreter (`checked_rem`), constant folding (`a % b`),
and the bytecode (`I64_REM`).

### Compiler changes

| Component                            | Changes                                                                                                                                                                                                                                              |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | The `infer_binary` whitelist becomes a "native fast path + registration-table query" dual path; the `Expr::Index` whitelist gains an interface-query branch                                                                                          |
| `typecheck/inference/bounds.rs`      | `check_trait_bounds` queries the interface registration table for operator interface names (`Equal` includes structural derivation)                                                                                                                  |
| `typecheck/checker.rs`               | New `Equal` structural derivation (after record definition); instantiation-level overloading rule (same-name interface multiple instantiations coexist by method signature)                                                                          |
| `middle/core/ir_gen.rs`              | `Expr::Index` gains a method-call dispatch branch; structs without an explicit `equal` generate native field-wise comparison for `==`; `Mod` instruction is changed to mathematical modulo                                                           |
| Interface registration table (new)   | Layer 0 mapping table constants + operator→interface query functions + core default registrations (`Int`/`Float`/`String`/`List` etc.)                                                                                                               |
| `trait_data.rs`                      | `Equal` registration and the old auto-derive are disabled (`Clone`/`Dup`/`Debug` keep their current state)                                                                                                                                           |
| Diagnostics                          | `Equal` precondition (linear token) failure reuses the `E1101` (type does not implement interface) family; the wording of `E1081`/`E1082` removes the word "Result" (phase 2 syncs with `Try` landing, per the RFC-013 process for three-party sync) |

### Backward compatibility

**Primitive-type operations are unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, `1 < 2` all retain
their native paths, and behavior and performance are unchanged.

**`1 + 2.5` changes from compile error to legal**: with the core registration
`Add(Int, Float, Float)` in place, mixed arithmetic previously rejected by the whitelist becomes
usable, returning `Float`. This is a new capability, and no existing code is affected.

**`%` semantic fix**: `-7 % 3` changes from `-1` to `2`. This is classified as a **defect fix** (the
documentation has long promised modulo; see Motivation §5), with no compatibility period; the
existing test corpus has no cases that depend on negative `%` (verified).

**Struct `==` changes from runtime error to usable**: previously `Struct == Struct` uniformly raised
E6007 at runtime; after the wiring, auto-derive makes it work — going from bad to good, with no
existing legal code affected.

**`Any`'s `==` is unaffected**: dynamic-type positions keep runtime comparison (RFC-036's
`assert_eq` assertion family depends on this; it was available before and remains available after).

**`f[0]` position binding is unchanged**: `distance[0]` is RFC-004's compiler-built-in capability,
**does not go through the `Index` interface**, and is unaffected.

**`?` is transparent to existing code** (phase 2): after `Result` is supplemented with a `Try`
instantiation, all existing `?` usages behave exactly the same.

## Trade-offs

### Advantages

- **Fulfills RFC-011's operator constraints**: `T: Add + Multiply` moves from paper (in fact a
  compile error) to an implementable reality, without modifying the body of the already-accepted RFC
- **Removes `Result`'s core binding** (phase 2): after `?` and constructors are interface-ized,
  `Result` can move to std, aligning with RFC-013's existing positioning
- **Fixes empirically verified defects**: `Point == Point` works out of the box, incidentally fixing
  `list.contains`'s unavailability for structs
- **Zero new syntax**: fully reuses the existing mechanisms of RFC-011a, not touching parser grammar
  rules
- **Zero runtime overhead**: static dispatch + native fast path for primitive types
- **User-defined containers become usable**: `Box(T)[0]` changes from "unconditionally errors" to
  usable
- **Internal language consistency**: Tuple/List already use element-wise comparison, records are now
  filled in; arithmetic interfaces' three parameters align with the shape of `Index`; RFC-011 §8.3's
  promotion table merges with the interface registration table

### Disadvantages

- **`Equal` auto-derive is a silent default**: if we ever want to withdraw "records are comparable
  by default", that's a breaking change. The position is accepted — it aligns with the existing
  behavior of Tuple/List, and explicit instantiation can always override
- **The `Equal` precondition rejects types containing linear tokens**: a type with `&mut` fields
  cannot use `==`; this is the cost of semantic correctness, requiring a clear diagnosis (specifying
  which field)
- **Interface instantiation is an explicit cost**: each arithmetic operator requires one line of
  instantiation + one method (`Equal` is exempt — auto-derive). The syntax of RFC-011a makes
  implicit derivation impossible (in exchange, the `Self` type parameter has no magic)
- **`%` semantic fix is a behavior change**: although classified as a defect fix, it still needs to
  be flagged in migration notes

## Alternatives

### Option A: only do Layer 1 (dispatch by method name), no interface layer

`+` only checks whether a method named `add` exists, without requiring the `Add` interface to be
implemented.

**Reason for rejection**: RFC-011's `T: Add` constraint would be decoupled from the operator — the
constraint consults the interface, the operator consults the method name, and the two can give
inconsistent answers. And it would be impossible to provide an accurate diagnosis at compile time
for "`+` used on a non-addable type".

### Option B: introduce constructor syntax `Ok(x)` / `Some(x)` to solve the `?` problem

Don't interface-ize `?`; instead, add constructor syntax to sum types.

**Reason for rejection**: this conflicts with RFC-010 (accepted). RFC-010 explicitly states
"uniformly use record types to express sum types, **with no need for two syntaxes**" and explicitly
deprecates the `|` syntax. Introducing constructors introduces a second way of expression. And it
only solves `?`, not `Point == Point` or custom container indexing.

### Option C: also interface-ize comparison operators in the first batch (introduce `Ordering`)

`<` `<=` `>` `>=` go through a `Compare` interface returning a three-value `Ordering`.

**Reason for rejection**: `<` is already a **first-class IR instruction** in YaoXiang
(`Instruction::Lt/Le/Gt/Ge`); interface-izing it would force primitive types to take a detour. And
introducing `Ordering` would bring up a whole set of issues around `Ordering`'s own
comparison/sorting, `PartialOrd` vs `Ord` for float `NaN`, etc. — that's the size of an independent
RFC. The empirically verified need today (`Point == Point`, `list.contains`) **only requires
`Equal`**.

### Option D: use abbreviations for operator names (the `Add` interface's method is `add`, the interface is `Mul`)

**Reason for rejection**: the body of RFC-011 already writes `T: Add + Multiply + Zero`; using
abbreviations requires modifying an already-accepted RFC.

### Option E: `Equal` only supports explicit instantiation, no auto-derive

**Reason for rejection**: a user-defined type unexpectedly cannot be `==`, which is unacceptable as
user experience; moreover, `(1, 2) == (1, 2)` and `[1] == [1]` already use element-wise comparison
today, with only record types excluded — that was already inconsistent. Explicit instantiation is
retained as an override means, accommodating custom needs.

### Option F: hard-code arithmetic interface return type as `-> Self`

**Reason for rejection**: the method for `Add(Int, Float)` would be forced to return `Int`, making
it impossible to correctly express `1 + 2.5`; the same applies to vector scaling
`Multiply(Point, Float)`. And RFC-011 §8.3's promotion type family `(Int, Float) => Float` would
lose its landing spot. The three-parameter `(Self, R, O)` aligns with the shape of
`Index(Key, Value)`.

## Implementation strategy

### Dependencies

| Dependency                             | Status    | Affected part of this RFC                                                              |
| -------------------------------------- | --------- | -------------------------------------------------------------------------------------- |
| RFC-011a interface mechanism Phase 1–3 | ✅ Landed | The foundation of Layer 2 (empirically runnable)                                       |
| RFC-010 record-based construction path | ✅ Landed | **Phase 2**: construction side of `?` + retirement of constructor parser special-cases |
| RFC-010b variant deconstruction        | ✅ Landed | **Phase 2**: `Result.residual`'s `match` writing, exhaustiveness                       |
| RFC-009 linear token inference         | Partial   | `Equal`'s precondition check                                                           |

### Phasing

Divided into two groups by **interface dependency** (design constraint, not a schedule):

**Phase 1 — does not depend on RFC-010/010b**:

- Layer 0 mapping table + interface implementation registration table + Layer 1 dispatch wiring
- The seven interfaces `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
  (three-type-parameter shape)
- `Equal` auto-derive + linear-token precondition + constraint solving rerouted to the registration
  table
- `%` semantic fix (interpreter / constant folding / bytecode, three places)
- `==` at `Any` positions keeps runtime comparison
- **Benefits**: `Point + Point`, `Point == Point` (no ceremony), `Box(T)[0]`, `1 + 2.5`, and the
  `T: Add` constraint all become usable

**Phase 2 — depends on RFC-010 / RFC-010b, requires the `Try` shape to be finalized before
starting**:

- Finalize the `Try` interface shape (see open questions)
- `?` interface-ization + retirement of the `ok`/`err`/`some` constructor parser special-cases,
  switching to the RFC-010 record-construction path
- Move `Result` from core to std (per RFC-013's existing positioning)

### Risks

| Risk                                                                      | Mitigation                                                                                                 |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `%` semantic change affects existing user code                            | Classified as a defect fix + verified that the test corpus has no negative-`%` dependency                  |
| Silent default of `Equal` auto-derive is hard to retract in the future    | Position is decided: aligned with the existing behavior of Tuple/List, explicit instantiation can override |
| Inconsistency between primitive types' dual paths (native + registration) | Gate: the semantics of core registrations must match native instructions (two views of the same table)     |
| Registration-table queries slow compilation                               | Table indexed by type name + caching of instantiation results (reusing RFC-011a proof)                     |
| Instantiation-level overloading introduces ambiguity                      | Same rule as method-level overloading: signature conflict ⇒ E1097                                          |

## Coordination with other RFCs

This RFC's positioning is that of a **consumer-side requirements raiser**, and the following RFCs
need to be updated in sync to ensure coordinated consistency (authorized to amend):

| RFC                                    | Content to update                                                                                                                                                                                                                                                                                                      |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **RFC-011** (accepted)                 | Note in the §Constraints section that `Add` / `Multiply` are defined and landed by 011b (`T: Add` ≜ `Add(T, T, T)`); mark `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` as dangling constraint names (pending a future RFC); note in §8.3 that the promotion type family merges with the interface registration table |
| **RFC-010** (accepted)                 | Clarify the landing requirement that "record fields are constructors" — this is the prerequisite for the retirement of constructor parser special-cases in phase 2                                                                                                                                                     |
| **RFC-010b** (draft, formerly RFC-039) | Construction and deconstruction must come in pairs; exhaustiveness judgment does not depend on `Result`'s core/std attribution (011b phase 2 will migrate)                                                                                                                                                             |
| **RFC-009** (accepted)                 | Cross-reference in the §Type-attributes section: the `Equal` precondition is "does not contain `&mut` linear tokens" (not Dup)                                                                                                                                                                                         |
| **RFC-013** (accepted)                 | At phase 2 landing: remove the word "Result" from the `E1081` / `E1082` text; `Equal` precondition diagnostics reuse the `E1101` family — three-party sync per the RFC-013 process (codes/*.rs ↔ locales ↔ code table)                                                                                                 |
| **RFC-018** (accepted)                 | After `%` changes to mathematical modulo, the `Mod → srem/urem` mapping table becomes invalid and must be changed to `srem` + sign correction (or composed via `sdiv`+`mul`+`sub`), and the "modulo/remainder" term mix must be corrected                                                                              |
| **RFC-036** (accepted)                 | No changes needed. Note the relationship: `==` at `Any` positions keeps runtime comparison, and the `assert_eq` assertion family is not affected by the gate                                                                                                                                                           |

> The basis for moving `Result` to std: RFC-013 already positions "stdlib `Result(T, Error)`", and
> RFC-014's layering puts std into the core source. Phase 2 of this RFC is one of its landing paths,
> and no other (non-existent) number is referenced.

## Open questions

- [x] ~~Does the `Modulo` semantic fix need a compatibility period?~~ → **Closed**: the language
      reference has long stated "multiplication/division/modulo"; the current remainder
      implementation contradicts the published documentation, so it is treated as a defect fix with
      no compatibility period (2026-09-22)
- [x] ~~Can users add operator implementations to existing types (orphan rule)?~~ → **Finalized**:
      an operator implementation can only be written in the defining module of the type. `Int` is
      defined in the core, so only the core can register it; users can only register their own
      types. All types are treated equally; built-in types have no special privileges (2026-09-22)
- [ ] Should `Index` distinguish mutable indexing (like Rust's `IndexMut`)? (Read-only in the first
      batch; mutable indexing involves RFC-009's `WriteToken`, and there's a `&mut Self` receiver
      precedent in RFC-011a; left for follow-up)
- [ ] Multi-position indexing Key: tuple packing (current state) or change to multi-parameter (Swift
      style)? (Sticking with tuple packing, not changing the parser)
- [ ] The shape of `Zero` / `One` : constant members, not operators; "interface members without a
      receiver" have no precedent in RFC-011a and need to be finalized separately before RFC-011's
      full clause `T: Add + Multiply + Zero` can be fulfilled
- [ ] The full shape of the `Try` interface: `?` needs three things — success/failure judgment,
      success-payload extraction, and failure-path output to the outer return value — and a single
      `residual` method is not enough; together with the retirement of constructor parser
      special-cases, finalize before phase 2 starts
- [ ] `?T` prefix type (RFC-026 FFI nullability annotation, RFC-018) and the `e?` suffix operator
      share the `?` symbol: different positions (type-level vs expression-level) do not constitute a
      conflict, just a written note
- [ ] `PartialOrd` / `Ordering` (interface-izing comparison operators): an independent RFC; this RFC
      explicitly does not do it

---

## Appendix A: Investigation evidence

All empirical results below are reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Availability of the interface mechanism (the foundation of this RFC)

| Capability                                                                     | Empirical result                                               |
| ------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Can be defined                                              |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runnable, correct output (unit tests in `tests/rfc011a.rs`) |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (field/method shared namespace conflict)            |
| Method dispatch `d.speak()`                                                    | ✅ Runnable                                                    |

**Note**: `E1097` means that operator methods must use the **external-declaration** form
(`Point.add: (self: &Point, ...)`), consistent with the example in RFC-011a. The `method_bindings`
registration is in `environment.rs` (`add_method_binding`); the query is in `expressions.rs`'s
call-target resolution and field-lookup-failure fallback path.

### A.2 Current state of operators

| Expression                                | Current state                                                                                   |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]`       | ✅ Hard-coded whitelist (Int/Float/String/List, **requires both sides to be the same type**)    |
| `1 + 2.5` (Int + Float)                   | ❌ Compile error (whitelist requires both sides to be the same type)                            |
| `-7 % 3`                                  | `-1` (truncated remainder; `%` whitelist only covers Int/Float, different from `+`'s whitelist) |
| `Point(1,2) == Point(1,2)`                | ❌ `E6007` (Eq on Struct fails at runtime)                                                      |
| `(1,2) == (1,2)` / `[1] == [1]`           | ✅ Runtime element-wise comparison (Struct is the only gap)                                     |
| `a == b` at `Any` positions (`assert_eq`) | ✅ Runtime comparison (RFC-036 empirical evidence)                                              |
| `f[0]` (function position binding)        | ⚠️ Legal only within the binding declaration, reported as `E3006` when used as an expression    |
| `arr[0, 1]` (multi-position)              | ✅ Tuple-packing, `list([1, 2])`                                                                |

### A.3 Hard-coded locations of `?` and constructors

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Constructor side: `ok(T)` / `err(E)` / `some(T)` are recognized by the parser (language spec
`syntax.md` §1.4.2); `std/result.rs`'s `is_ok` / `unwrap` etc. match by `variant_id 0/1` patterns.
"Moving `Result` to std" requires both halves (`?` + constructors) to be solved together, both in
phase 2.

## Appendix B: Design decision record

| Decision                                   | Decision                                                                                                      | Rationale                                                                                                                                                              | Date       |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Architecture                               | Three-layer separation (mapping table / dispatch base / interface contract)                                   | Merging would decouple RFC-011 constraints from operators                                                                                                              | 2026-09-15 |
| Operator precondition                      | Must implement the interface (Layer 2 is the precondition)                                                    | Ensures `T: Add` and `+` strictly correspond                                                                                                                           | 2026-09-15 |
| Interface naming                           | Full spelling (`Multiply` rather than `Mul`)                                                                  | Doesn't change the body of the already-accepted RFC-011                                                                                                                | 2026-09-15 |
| `%` interface name                         | `Modulo` (mathematical modulo)                                                                                | The name pins down the semantics                                                                                                                                       | 2026-09-15 |
| Comparison operators                       | Not interface-ized in the first batch; retain native IR instructions                                          | Already first-class instructions; `Ordering` brings up a whole set of independent issues                                                                               | 2026-09-15 |
| `Ordering`                                 | Not introduced in the first batch                                                                             | No real-world need; size of an independent RFC                                                                                                                         | 2026-09-15 |
| Associated types                           | Use interface type parameters, no `type` member syntax                                                        | This approach is already finalized in RFC-011a (`Iterator: (Item: Type)`)                                                                                              | 2026-09-15 |
| Unification of method binding and indexing | Conceptually unified; the interface only handles container indexing                                           | Position-binding keys are compile-time constants, and the result type requires type-family evaluation, which users cannot implement                                    | 2026-09-15 |
| Multi-position indexing                    | Tuple packing + overloading by Key type, no variadic interface                                                | Doesn't change the parser                                                                                                                                              | 2026-09-15 |
| Bitwise / unary operators                  | Not in the first batch                                                                                        | Rare for user-defined types, YAGNI                                                                                                                                     | 2026-09-15 |
| Arithmetic interface shape                 | Three type parameters `(Self, R, O)`; `T: Add` ≜ `Add(T, T, T)`                                               | Explicit result type: `1 + 2.5` and scaling are expressible; RFC-011 §8.3's promotion table merges with the registration table; uniform shape with `Index(Key, Value)` | 2026-09-22 |
| `Equal` derivation                         | Default auto-derive + explicit instantiation override; same criterion for constraint solving                  | "A user-defined type cannot be ==" is unacceptable; Tuple/List already use element-wise comparison, Struct is the only gap                                             | 2026-09-22 |
| `Equal` precondition                       | "Does not contain `&mut` linear tokens", **not Dup**                                                          | RFC-011 §2.4 explicitly says primitives don't fall under Dup; using Dup as the precondition would mistakenly reject `Point{Float,Float}`                               | 2026-09-22 |
| Names and registration separation          | Operators only consult the interface implementation registration table, not name resolution                   | Local same-name bindings (type-level `Add` family, etc.) and operators don't interfere; RFC-011 §5.2's example needs no change                                         | 2026-09-22 |
| Orphan rule                                | Implementation follows the defining module of the type                                                        | All types are treated equally; built-in types have no special privileges                                                                                               | 2026-09-22 |
| `Any`'s `==`                               | Keep runtime comparison, no registration-table query                                                          | RFC-036's `assert_eq` assertion family already depends on this behavior                                                                                                | 2026-09-22 |
| `Try` shape                                | Finalized: four methods (is_failure/success/residual/from_error) + assert-Never dead-branch semantics; landed | `?` needs three things, a single `residual` method is not enough; constructor parser special-cases are retired together with `Result` moving to std                    | 2026-09-22 |
| `%` semantic classification                | Defect fix (documentation has long promised modulo), no compatibility period                                  | `reference/index.md`'s "multiplication/division/modulo" is the prior evidence                                                                                          | 2026-09-22 |
| Basis for moving `Result` to std           | Reference RFC-013's existing positioning, not a non-existent number                                           | RFC-013 already writes "stdlib `Result(T, Error)`"                                                                                                                     | 2026-09-22 |

## Appendix C: Glossary

| Term                                        | Definition                                                                                                                                                                                           |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layer 0                                     | The fixed mapping table from operators to method names; a language-level constant that is not user-modifiable                                                                                        |
| Layer 1                                     | The dispatch base that looks up by the `Type.method` key and invokes                                                                                                                                 |
| Layer 2                                     | The interface contract layer, providing the basis for generics constraints and serving as the precondition for operators                                                                             |
| Interface implementation registration table | The aggregated table of implementations combining core default registrations and user instantiations; the sole criterion for operator queries and constraint solving, independent of name resolution |
| Native fast path                            | The hard-coded computation path retained for primitive types, not going through interface dispatch                                                                                                   |
| Position binding                            | RFC-004's `f[0]` syntax, which binds a function parameter position as a method; a compile-time behavior                                                                                              |
| Auto-derive                                 | The field-wise `==` the compiler generates for records whose fields are all comparable; explicit instantiation can override                                                                          |

## References

- [RFC-011: Generics System Design](011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraints, associated types
- [RFC-011a: Interface Implementation and Dynamic Dispatch](011a-interface-implementation.md) —
  Interface declaration/instantiation/overloading rules
- [RFC-009: Ownership Model Design](009-ownership-model.md) — `&mut T` linear token
- [RFC-004: Multi-position Joint Binding of Curried Methods](004-curry-multi-position-binding.md) —
  `f[0]` syntax
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — Sum-type expression
- [RFC-013: Error Code Specification](013-error-code-specification.md) — `Result` belongs to std
  positioning, error code process
- [RFC-010b: Pattern Matching Completeness (Variant Deconstruction and Exhaustiveness)](010b-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — Associated type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — Multi-parameter subscripts
