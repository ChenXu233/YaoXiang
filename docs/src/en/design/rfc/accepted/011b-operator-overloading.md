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
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md) — type constraint
>   `T: Add + Multiply`, associated type
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
>   interface declaration / instantiation mechanism
> - [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear token
> - [RFC-004: Multi-Position Binding for Curried Methods](./004-curry-multi-position-binding.md) —
>   `f[0]` position-binding syntax
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type = record whose fields
>   all return their own type
> - [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` existing
>   position of being attributed to std, E108x
> - [RFC-010b: Pattern Matching Completion (Variant Deconstruction and Exhaustiveness)](../accepted/010b-pattern-matching-completeness.md)
>   — variant deconstruction (dependency)

## Summary

This RFC completes **operator overloading** for YaoXiang, allowing `a + b` / `a == b` / `a[i]` /
`e?` to be implemented by user-defined types, and turns the **operator constraints**
(`T: Add + Multiply`) already written into RFC-011's constraint syntax from a **paper capability**
into a landable mechanism (`Zero` in the same sentence is not an operator; see Open Questions).

The design adopts a **separation of three layers of responsibilities**: a fixed operator→method
mapping table (Layer 0), a name-based dispatch substrate (Layer 1, reusing the existing
`method_bindings`), and the interface contract layer (Layer 2, for generic constraints). The
**precedence and associativity of operators remain language-fixed**; users only overload semantics.

First-batch scope: seven interfaces — `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`.
Arithmetic interfaces use **three type parameters** `(Self, R, O)` — making the result type `O`
explicit so that heterogeneous operations such as `1 + 2.5` (returns `Float`) and `point * 2.0`
(scaling) can be expressed. `Equal` is **automatically derived by default** (records whose fields
are all comparable automatically obtain field-by-field `==`); explicit instantiation can override
this.

`Try` (the interfacification of `?`) is moved entirely to Phase 2: it depends on RFC-010
(construction) and RFC-010b (deconstruction) being landed, and the interface shape is not yet
finalized (see Open Questions).

**No new syntax, no new keywords** — all of it reuses the existing mechanisms from RFC-011a
(interface declaration / instantiation / external method declaration / overloading).

## Motivation

### Why this feature is needed

#### 1. RFC-011's core example depends on it, and the current state is worse than "undeliverable"

RFC-011 (accepted) uses operator names as type constraints in 8 places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

Not a single RFC in the entire set defines where `Add` / `Multiply` come from or how `+` binds to
them. Actual testing confirms the situation is worse: `T: Add` **errors out at compile time today**
— constraint resolution queries the old trait table (`trait_data.rs`), and `Add` is not in that
table. The constraint names `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` are similarly dangling
(their treatment is described in "Coordination with Other RFCs").

#### 2. User-defined types cannot participate in basic operations (verified)

```
Point: Type = { x: Int, y: Int }
a = Point(1, 2)
b = Point(1, 2)
a == b
```

```
error [E6007] Runtime error: type mismatch in comparison Eq:
    Struct { type_id: TypeId(0), ... } vs Struct { type_id: TypeId(0), ... }
```

A collateral consequence: `list.contains(list_of_structs, p)` is **completely unusable** — it
depends internally on `==`. Meanwhile, `(1, 2) == (1, 2)` and `[1] == [1]` have **always worked**
through runtime element-wise comparison — structs are the only gap.

#### 3. `?` welds type names into the compiler, blocking `Result` from being attributed to std

The implementation of `?` hardcodes both the type name and the variant index:

```rust
// typecheck: hardcoded construction of the Result type
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hardcoded group name and variant index
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` is made **interface-driven**, any type (including
user-defined ones) that implements the interface can be used by `?`, and `Result` can be attributed
to std (RFC-013 already has the position of "std library `Result(T, Error)`").

Note that this is only half the problem: the `ok(...)` / `err(...)` / `some(...)` construction forms
are also welded into the parser today (the language specification §1.4.2 lists them as "constructors
recognized by the parser"). "Result belongs to std" requires **untying both halves** of `?` and the
constructors together, both falling under Phase 2 of this RFC.

#### 4. User-defined containers cannot be indexed

The type checking of `Index` is a hardcoded whitelist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// others → "Containers not recognized at the type level: better reject than silently fail"
```

For any container type defined by the user, `c[0]` is unconditionally rejected.

#### 5. `%` implementation violates already-published documentation

Actual testing shows `-7 % 3` returns `-1` (truncated remainder). But the operator precedence table
in the language reference (`reference/index.md`) already reads "`* / %` multiplication/division
**modulo**" — what the documentation promises is mathematical modulo, while the implementation
delivers remainder. This is not a design change; it is a **defect where the implementation violates
the documentation**, and this RFC fixes it as a side improvement.

### Existing semi-finished foundations

Investigation found that most of the mechanism is already in place; what's missing is the wiring:

| Mechanism                                                           | Location                                                                                                                   | State                                                                                                        |
| ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Interface declaration `(Self: Type) -> Type`                        | RFC-011a Phase 1                                                                                                           | ✅ Runnable                                                                                                  |
| Interface instantiation `Animal(Dog)` + external method declaration | RFC-011a Phase 2–3                                                                                                         | ✅ Runnable (with unit tests)                                                                                |
| Method dispatch `method_bindings["Type.method"]`                    | `expressions.rs` (registered in `environment.rs`, queried in `expressions.rs` for call resolution and field fallback path) | ✅ Runnable                                                                                                  |
| Associated type (= interface type parameter)                        | RFC-011 §3.1; RFC-011a has already adopted this scheme                                                                     | ✅ Mechanism decided                                                                                         |
| Type family evaluation `AssociatedTypeDef`                          | `dependent_types.rs`                                                                                                       | ✅ Production use case: `IsTrue` of `std.assert`                                                             |
| `Equal` / `Dup` / `Clone` / `Debug` trait                           | Registered in `trait_data.rs`                                                                                              | ⚠️ `Equal` has zero consumption points; old auto-derivation only registers signatures with no implementation |
| Operator → method name mapping                                      | —                                                                                                                          | ❌ Does not exist                                                                                            |

**Conclusion**: This is not a feature built from scratch, but rather wiring and documenting existing
parts.

## Proposal

### Core Design: Separation of Three Layers of Responsibilities

```
Layer 0  Fixed mapping table (language-level constant, not user-modifiable)
         + → add    == → equal    [] → index    ? → residual
         │  Built in at compile time, not involved in type inference, not exposed to users
         ▼
Layer 1  Dispatch substrate (look up method by name and call it)  ← reuse existing mechanism
         method_bindings["Point.add"]
         ▼
Layer 2  Interface contract (for generic constraints)
         Add / Equal / Index / Try …
         Makes T: Add constraints valid, and serves as the gating condition for operators
```

**Reasons for the layering**:

- **Layers 0 and 1 make operators "usable"**, Layer 2 makes operators "constrainable". Merging them
  would cause any method named `add` to be called by `+`, decoupling RFC-011's `T: Add` from the
  operator.
- **Layer 1 is not newly built**: `method_bindings` already looks up tables by the `Type.method` key
  (the fallback path after field lookup fails), and operators can simply take the same path.
- **Layer 2 is the gating condition**: before an operator is allowed, it **must** be confirmed that
  the type implements the corresponding interface (see the `Equal` auto-derivation exception —
  derivation and registration use the same criterion).

### Names and Registration: Two Independent Channels

**Operator queries go to the "interface implementation registry", not through ordinary name
resolution.**

- The registry belongs to the core and aggregates two kinds of registration: the core's default
  registration for primitive types (e.g. `Add(Int, Int, Int)`), and interface instantiations written
  by users in type bodies (e.g. `Add(Point, Point, Point)`). No matter which module the
  implementation code physically lives in, registrations flow into the same table; `+` / `==` / `[]`
  only look at this table.
- A local module defining a same-named binding (such as the type-level `Add` in RFC-011 §5.2's Peano
  type-level addition `Add: (A: Type, B: Type) -> Type = match ...`) goes through the **name lookup
  channel**, only affecting the resolution of the name `Add` within that module, **and cannot reach
  the registry, so it does not affect operator usability**. Same name, different things, no mutual
  shadowing.

This also answers "whether RFC-011's type-level `Add` conflicts with this RFC's interface `Add`":
they do not conflict. Type-level `Add` is purely a type-level computation (Zero/Succ are types, not
values, and there will never be a value-level `Zero + Succ(...)`), and the value-level operator
interface is the same name at two different layers.

### Layer 0: Fixed Mapping Table

| Operator          | Interface                                                 | Method            | First Batch |
| ----------------- | --------------------------------------------------------- | ----------------- | ----------- |
| `+`               | `Add`                                                     | `add`             | ✅          |
| `-`               | `Subtract`                                                | `subtract`        | ✅          |
| `*`               | `Multiply`                                                | `multiply`        | ✅          |
| `/`               | `Divide`                                                  | `divide`          | ✅          |
| `%`               | `Modulo`                                                  | `modulo`          | ✅          |
| `==` `!=`         | `Equal`                                                   | `equal`           | ✅          |
| `[]`              | `Index`                                                   | `index`           | ✅          |
| `?`               | `Try` (four methods, finalized in Phase 2)                | `is_failure` etc. | ✅ Landed   |
| `<` `<=` `>` `>=` | — (reserved as native instructions)                       | —                 | ❌          |
| `and` `or`        | — (short-circuit is language semantics, not overloadable) | —                 | ❌          |
| Bitwise (5)       | —                                                         | —                 | ❌          |
| Unary `-` `!`     | —                                                         | —                 | ❌          |

**Interface names are spelled out in full rather than abbreviated** (`Multiply` rather than `Mul`):
this keeps consistency with the `T: Add + Multiply + Zero` in RFC-011's body, and **does not modify
already-accepted RFCs**.

**`%` adopts the `Modulo` semantics** (mathematical modulo, the sign of the result follows the
divisor). The motivation section has already confirmed that the current remainder implementation
violates the already-published documentation (`reference/index.md`
"multiplication/division/modulo"); this item is handled as a defect fix, with no compatibility
period.

Current state note: the type-check whitelist for `%` currently only includes Int/Float (different
from the Int/Float/String/List whitelist of `+`); see Appendix A.2 for actual test records.

### Layer 2: Interface Definitions

#### Arithmetic Interfaces (Three Type Parameters)

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

The three type parameters each have a clear role:

- `Self`: the left operand type (the receiver, borrowed as `&Self`; see RFC-009's borrow token and
  RFC-011a's receiver convention);
- `R`: the right operand type — **left and right are allowed to be of different types**;
- `O`: the **result type**, declared explicitly.

**Why the result type must be an explicit parameter** (rather than hardcoded as `-> Self`):

1. If it is hardcoded as `Self`, then the method of `Add(Int, Float)` must return `Int`, and
   `1 + 2.5` cannot be expressed correctly;
2. Once the result type is made explicit, the lifting type family from RFC-011 §8.3,
   `Add: (A, B) -> Type = match (A, B) { (Int, Float) => Float, ... }`, and this RFC's interface
   registry **become two views of the same table** — the core's registration
   `Add(Int, Float, Float)` is precisely the row `(Int, Float) => Float`, and each user
   instantiation is adding a row to this table;
3. The `Index` interface is already a three-parameter form `(Self, Key, Value)` with `Value` as the
   return type parameter — once the arithmetic interfaces add `O`, the entire operator interface
   family has a unified shape, and the hardcoded-`Self` form becomes the odd one out.

**Core default registration** (native instruction path, not via method calls): `Add(Int, Int, Int)`,
`Add(Int, Float, Float)`, `Add(Float, Int, Float)`, `Add(Float, Float, Float)`,
`Add(String, String, String)` (concatenation), `Add(List(T), List(T), List(T))` (element
concatenation), etc.; the same applies to the five arithmetic interfaces for primitive types.
`1 + 2.5` changes from the current compile error (the whitelist requires both sides to be of the
same type) to a legal operation returning `3.5: Float`.

**Constraint syntax sugar**: `T: Add` ≜ registered `Add(T, T, T)` — same-type self-composition, with
the result still of that type. In the RFC-011 matrix multiplication example, `a * b + c` is of type
`T` throughout, which is exactly this meaning. `T: Equal` similarly ≜ `Equal(T, T)`.

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

#### Equality Interface (Auto-Derived by Default)

```yaoxiang
Equal: (Self: Type, R: Type) -> Type = {
    equal: (self: &Self, other: &R) -> Bool
}
```

**`Equal` is auto-derived by default**, with five rules:

1. **Default derivation**: when a record type is defined, if all fields are comparable (primitive
   types, `String`, or comparable records/tuples/lists themselves, and containing no `&mut` fields),
   the compiler automatically generates a **field-by-field comparison** for `==`, taking a native
   code path with no user-visible method. User-defined types are naturally comparable with no
   ceremony required.
2. **Explicit override**: if the type body contains `Equal(Point, Point)` and provides a
   `Point.equal` method, the user's version is used (e.g. float comparison with tolerance), and
   auto-derivation no longer applies.
3. **When fields are incomparable**: auto-derivation fails, `==` becomes unavailable, and the
   diagnostic specifies which field is the cause; the user can still hand-write `Equal` to define
   custom comparison (e.g. comparing fields containing closures by name).
4. **Precondition**: types containing `&mut T` linear tokens (recursive in fields) do not
   participate in comparison — a linear token is consumed upon a single read and cannot be used to
   extract two values at once for comparison. Note that the premise is "non-linear" rather than
   "Dup": primitive value types (Int/Float/Bool/Char), per RFC-011 §2.4, are not subject to Dup
   (they are compiler-builtin value copies); if Dup were used as the premise,
   `Point { x: Float, y: Float }` would be wrongly rejected.
5. **Constraints share the same criterion**: the resolution of `T: Equal` and the gating of `==`
   follow the same "check registry or structural derivation" rule — constraints and operators always
   give the same answer.

**Justification for consistency**: `(1, 2) == (1, 2)` and `[1] == [1]` already go through runtime
element-wise comparison today (the comparison whitelist in `executor.rs` includes Tuple/List/Array),
and record types are the only composite type excluded. Auto-derivation is not a new silent default,
but rather the final missing piece in the language's existing internal behavior.

**Old mechanism retirement**: the old auto-derivation of `Equal` in `trait_data.rs` (which only
registered signatures, had no implementation code, and had zero consumption points) is retired; all
`Equal` judgments (primitive type default registration, structural derivation, explicit
instantiation) go through the interface registry. The old trait table retains its existing duties
for Clone/Dup/Debug (their names do not overlap with operator interfaces; unification to be
discussed separately in the future).

#### Index Interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` as a type parameter rather than an associated type member**: the open question in RFC-011a
has already been settled with "associated types implemented via generic interface parameters"
(`Iterator: (Item: Type) -> Type` is an isomorphic precedent), and no `type` member syntax needs to
be introduced.

**Ownership note**: `index` returns a full `Value` from a `&Self` borrow. The standard library's
`list.get: (&Vec(A), Int) -> A` already has the same shape, and this interface **enjoys the same
treatment as the existing std situation**; the precise semantics of "extracting a full value from a
borrow" for move-semantic element types will be handled uniformly when RFC-009 is fully enforced,
and this RFC does not invent new rules for this.

**Multi-position index relies on tuple packing + overloading**, no variadic interface introduced:

```yaoxiang
// One-dimensional container
List(T) instantiates Index(List(T), Int, T)                   → arr[0]

// Multi-dimensional container
Grid    instantiates Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └─ Key is a tuple

// Two instantiations with different signatures → coexisting
```

**Multiple instantiations of a same-name interface for the same type are allowed**, distinguished by
the signature of the injected method and coexisting according to the method-level overloading rules
of RFC-011a. RFC-011a's overloading rules explicitly only extend to the method level; this RFC adds
a layer of instantiation-level rules on top: the legality of same-name interface instantiation
coexistence is determined by whether the expanded method signatures conflict (conflict → E1097,
sharing the same source as the field/method namespace rules).

#### Propagation Interface Try (Finalized and Landed in Phase 2)

The interface name for the interfacification of `?` is `Try`, with a four-method shape (finalized on
2026-09-22 in Phase 2):

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    is_failure: (self: &Self) -> Bool,
    success:    (self: &Self) -> T,
    residual:   (self: &Self) -> E,
    from_error: (E) -> Self,
}
```

- **Semantic division of labor**: `is_failure` decides success/failure, `success` extracts the
  success payload, `residual` extracts the failure payload, and `from_error` acts as the bridge for
  cross-type propagation (reconstructing a failure value from `E` when the `T` of `f()?` differs
  from the outer `U`). The lowering of `?` uniformly generates a chain of four method calls — when
  `is_failure(t)` is true, `Ret from_error(residual(t))`; otherwise the expression value is
  `success(t)`; the handwritten variant check sequence is gone, and `Result` / `Option` (implemented
  in std yx) and user-defined Try types all take the same path.
- **Dead-end branches**: the failure arm of `success` and the success arm of `residual` are
  contractually unreachable, and the implementation uses `assert(false)` to diverge (`assert`
  returns `Never`, and the explosion principle `Never <: T` lets the check pass, per type-system.md
  §2.2).
- **Checking**: typecheck looks up the interface implementation registry (nominal match at the Self
  position, abstract entries instantiated according to the scrutinee's actual arguments); the outer
  function's return type must also implement `Try` and the `E` position must be able to catch the
  failure value (E1081/E1082/E1083 semantics become interfacified accordingly).
- **Result belongs to std**: the type definitions and Try implementations of `Result` / `Option`
  move to `std/result.yx` / `std/option.yx` (pure YaoXiang), and the native `ok`/`err` constructors
  are retired — the variant construction syntax `Result(T, E).ok(v)` becomes the only construction
  channel (the constructor welding problem is resolved together with the retirement of the parser's
  special case). The Try residual type of Option takes `Void` (corresponding to the NoneT semantics
  of the Rust Try experiment).

### Examples

#### User-Defined Type: Arithmetic and Equality Work Out of the Box

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
    println(a == b)             # false — Equal auto-derived, no instantiation needed
    println(a == a)             # true
}
```

#### Custom Equality (Overriding Auto-Derivation)

```yaoxiang
Vec3: Type = {
    x: Float,
    y: Float,
    z: Float,
    Equal(Vec3, Vec3),
}

# Float comparison with tolerance, overriding field-by-field auto-derivation
Vec3.equal: (self: &Vec3, other: &Vec3) -> Bool =
    abs(self.x - other.x) < 0.000001
    and abs(self.y - other.y) < 0.000001
    and abs(self.z - other.z) < 0.000001
```

#### Custom Container Indexing

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

#### Generic Constraints Finally Landable

```yaoxiang
// RFC-011's example can now land
// T: Add ≜ Add(T, T, T) registered; T: Multiply ≜ Multiply(T, T, T) registered
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

(The `Zero` / `One` in RFC-011's signature example `T: Add + Multiply + Zero` are not in this RFC's
scope — they are constant members rather than operators, and "interface members without a receiver"
have no precedent in RFC-011a; see Open Questions.)

### Syntax Changes

**No new syntax, no new keywords**. All capabilities are composed from existing mechanisms:

| Capability              | Reused Existing Mechanism                                              |
| ----------------------- | ---------------------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                                       |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                              |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)                    |
| Associated type         | RFC-011 §3.1 (interface type parameter)                                |
| Multi-position index    | Existing tuple packing parsing + instantiation-level overloading rules |
| Operator precedence     | **Language-fixed**, not open to user customization                     |

## Detailed Design

### Type System Impact

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` (`Try`
in Phase 2).

**Interface implementation registry**: the central nervous system for operator gating. Aggregating
core default registration (primitive types) and user instantiations (the `ImplementationProof`
produced by `check_interface_instantiation`), the gating check for `+` / `==` / `[]` and the
constraint resolution of `T: Add` (the `check_trait_bounds` of `bounds.rs`) **query the same table**
— there is no gap where "constraints say yes, operators say no".

**Structural derivation of `Equal`**: a structural rule is layered on top of the registry (all
fields comparable ⇒ comparable), with primitive types backed by core registration and recursing to
closure. The `Equal` registration and old auto-derivation in the original `trait_data.rs` are
retired.

**Separation of operator query and name resolution**: operators like `+` only look up the registry
and do not go through ordinary name resolution; local same-name bindings (such as the type-level
`Add` family) do not affect operators (see §Names and Registration).

**Orphan rule**: operator implementations can only be written in the **module where the type is
defined** — `Int` is defined in core, so the registration of `Int` can only be written in core;
`Point` is defined in a user module, so only its definer can register for it. All types follow the
same rule, with no privileges for built-in types.

### Runtime Behavior

**Zero runtime overhead**: operators determine the call target at compile time (static dispatch).
For primitive types (`Int`/`Float`/`String`/`List`), a **native instruction fast path** is retained,
not going through interface dispatch; auto-derived struct `==` generates native field-by-field
comparison; `==` / `!=` at `Any` (dynamic type) positions maintains the existing runtime comparison
(RFC-036's testing framework `assert_eq` depends on this behavior, which is not affected).

**`%` semantic fix**: `-7 % 3` changes from `-1` (truncated remainder) to `2` (mathematical modulo).
Three places are modified in sync: the interpreter (`checked_rem`), constant folding (`a % b`), and
the bytecode (`I64_REM`).

### Compiler Changes

| Component                            | Changes                                                                                                                                                                                                                             |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | `infer_binary` whitelist changes to "native fast path + registry query" dual path; `Expr::Index` whitelist adds an interface query branch                                                                                           |
| `typecheck/inference/bounds.rs`      | `check_trait_bounds` for operator interface names changes to look up the interface registry (`Equal` includes structural derivation)                                                                                                |
| `typecheck/checker.rs`               | New `Equal` structural derivation (after record definition); instantiation-level overloading rules (same-name interface multi-instantiation coexists by method signature)                                                           |
| `middle/core/ir_gen.rs`              | `Expr::Index` adds method-call dispatch branch; struct `==` without explicit `equal` generates native field-by-field comparison; `Mod` instruction changes to mathematical modulo                                                   |
| Interface registry (new)             | Layer 0 mapping table constant + operator→interface query function + core default registration (`Int`/`Float`/`String`/`List` etc.)                                                                                                 |
| `trait_data.rs`                      | `Equal` registration and old auto-derivation retired (`Clone`/`Dup`/`Debug` remain as-is)                                                                                                                                           |
| Diagnostics                          | `Equal` precondition (linear token) unsatisfied reuses the `E1101` (type does not implement interface) family; `E1081`/`E1082` text removes the word "Result" (in Phase 2, with `Try` landed, three-party sync per RFC-013 process) |

### Backward Compatibility

**Primitive type operations are unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, `1 < 2` all retain
their native paths, with unchanged behavior and performance.

**`1 + 2.5` changes from compile error to legal**: after the core registers
`Add(Int, Float, Float)`, the mixed arithmetic previously rejected by the whitelist becomes usable
and returns `Float`. This is a new capability, with no existing code affected.

**`%` semantic fix**: `-7 % 3` changes from `-1` to `2`. Classified as a **defect fix** (the
documentation has long promised modulo, see motivation §5), with no compatibility period; the
existing test corpus has no cases depending on negative `%` (verified).

**Struct `==` changes from runtime error to usable**: previously `Struct == Struct` unconditionally
produced an E6007 runtime error, and after wiring it becomes usable with auto-derivation — from
broken to working, with no existing legal code affected.

**`Any`'s `==` is not affected**: the dynamic type position retains runtime comparison (RFC-036's
`assert_eq` assertion family depends on this, which was usable before and remains usable after).

**`f[0]` position binding is unchanged**: `distance[0]` is RFC-004's compiler-builtin capability,
**does not go through the `Index` interface**, and is not affected.

**`?` is transparent to existing code** (Phase 2): after `Result` gets its `Try` instantiation, the
behavior of existing `?` usages is completely consistent.

## Trade-offs

### Advantages

- **Delivers RFC-011's operator constraints**: `T: Add + Multiply` changes from paper (in fact a
  compile error) to landable, without touching the body of already-accepted RFCs
- **Removes the core binding of `Result`** (Phase 2): after `?` and constructors are interfacified,
  `Result` can be attributed to std, aligning with RFC-013's existing position
- **Fixes verified defects**: `Point == Point` works out of the box, which also fixes
  `list.contains` being unusable for structs
- **Zero new syntax**: all of it reuses existing mechanisms from RFC-011a, not touching the parser's
  grammar rules
- **Zero runtime overhead**: static dispatch + native fast path for primitive types
- **User-defined containers become usable**: `Box(T)[0]` changes from "unconditionally rejected" to
  usable
- **Internal language consistency**: Tuple/List already have element-wise comparison, and record
  types are now complete; the three-parameter shape of arithmetic interfaces is consistent with
  `Index`; RFC-011 §8.3's lifting table is unified with the interface registry

### Disadvantages

- **`Equal` auto-derivation is a silent default**: if we ever want to take back "records are
  comparable by default" in the future, it will be a breaking change. This position is accepted — it
  is consistent with the existing behavior of Tuple/List, and explicit instantiation can always
  override
- **`Equal`'s precondition rejects types containing linear tokens**: types containing `&mut` fields
  cannot use `==`, which is the cost of semantic correctness and requires clear diagnostics
  (specifying which field)
- **Interface instantiation is an explicit cost**: each arithmetic operator requires a line of
  instantiation + a method (already exempt for `Equal` — auto-derived). The syntax of RFC-011a makes
  implicit derivation impossible (in exchange for no magic in the `Self` type parameter)
- **`%` semantic fix is a behavior change**: although classified as a defect fix, it still needs to
  be noted in the migration documentation

## Alternatives

### Option A: Only Do Layer 1 (Dispatch by Method Name), No Interface Layer

`+` only checks whether a method named `add` exists, without requiring the `Add` interface to be
implemented.

**Reason for rejection**: RFC-011's `T: Add` constraint would become decoupled from the operator —
constraints look up interfaces, while operators look up method names, and the two may give
inconsistent answers. It is also impossible to provide accurate diagnostics at compile time for "`+`
used on non-addable types".

### Option B: Introduce Constructor Syntax `Ok(x)` / `Some(x)` to Solve the `?` Problem

Do not interfacify `?`, but rather add constructor syntax for sum types.

**Reason for rejection**: conflicts with RFC-010 (accepted). RFC-010 clearly states "uniformly use
record types to express sum types, **no need for two sets of syntax**", and explicitly deprecates
the `|` syntax. Introducing constructors would be introducing a second set of expressions. And it
only solves `?`, not `Point == Point` and custom container indexing.

### Option C: Also Interfacify Comparison Operators in the First Batch (Introducing `Ordering`)

`<` `<=` `>` `>=` go through a `Compare` interface, returning the three-value `Ordering`.

**Reason for rejection**: `<` is already a **first-class IR instruction** in YaoXiang
(`Instruction::Lt/Le/Gt/Ge`), and interfacification would force primitive types to take a detour.
Moreover, introducing `Ordering` brings up a whole set of issues such as `Ordering`'s own
comparison/sorting and the `PartialOrd` vs `Ord` of float `NaN`, which is the size of an independent
RFC. The actual needs exposed by current testing (`Point == Point`, `list.contains`) **only need
`Equal`**.

### Option D: Use Abbreviations for Operator Names (the `Add` interface's method is called `add`, the interface is called `Mul`)

**Reason for rejection**: the body of RFC-011 already writes `T: Add + Multiply + Zero`; using
abbreviations would require modifying already-accepted RFCs.

### Option E: `Equal` Only Does Explicit Instantiation, No Auto-Derivation

**Reason for rejection**: it is unacceptable for users to find that their own types cannot use `==`;
moreover, `(1, 2) == (1, 2)` and `[1] == [1]` are already element-wise compared today, and only
record types are excluded, which was already inconsistent. Explicit instantiation is retained as an
override means, balancing custom needs.

### Option F: Hardcode the Return Type of Arithmetic Interfaces as `-> Self`

**Reason for rejection**: the method of `Add(Int, Float)` would be forced to return `Int`, and
`1 + 2.5` could not be expressed correctly; vector scaling `Multiply(Point, Float)` similarly could
not be written. And the lifting type family `(Int, Float) => Float` from RFC-011 §8.3 would lose its
landing place. The three-parameter form `(Self, R, O)` is unified with the shape of
`Index(Key, Value)`.

## Implementation Strategy

### Dependencies

| Dependency                             | Status    | Which Part of this RFC It Affects                                                     |
| -------------------------------------- | --------- | ------------------------------------------------------------------------------------- |
| RFC-011a interface mechanism Phase 1–3 | ✅ Landed | The foundation of Layer 2 (verified runnable)                                         |
| RFC-010 record-style construction path | ✅ Landed | **Phase 2**: construction side of `?` + retirement of constructor parser special case |
| RFC-010b variant deconstruction        | ✅ Landed | **Phase 2**: `match` style of `Result.residual`, exhaustiveness                       |
| RFC-009 linear token derivation        | Partial   | Precondition check of `Equal`                                                         |

### Phases

Divided into two groups by **interface dependency** (design constraint, not scheduling):

**Phase 1 — does not depend on RFC-010/010b**:

- Layer 0 mapping table + interface implementation registry + Layer 1 dispatch wiring
- Seven interfaces: `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` (three type
  parameter form)
- `Equal` auto-derivation + linear precondition + constraint resolution rerouted to registry
- `%` semantic fix (including interpreter / constant folding / bytecode)
- `Any`-position `==` retains runtime comparison
- **Benefit**: `Point + Point`, `Point == Point` (no ceremony), `Box(T)[0]`, `1 + 2.5`, `T: Add`
  constraint all become usable

**Phase 2 — depends on RFC-010 / RFC-010b, the `Try` shape must be finalized before work begins**:

- `Try` interface shape finalized (see Open Questions)
- `?` interfacification + constructors (`ok`/`err`/`some`) parser special case retired, switched to
  RFC-010 record construction path
- `Result` migrated from core to std (based on RFC-013's existing position)

### Risks

| Risk                                                                                | Mitigation                                                                                                         |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `%` semantic change affects existing user code                                      | Classified as a defect fix + verified that the test corpus has no negative `%` dependencies                        |
| `Equal` auto-derivation as a silent default will be hard to take back in the future | Position decided: consistent with existing Tuple/List behavior, explicit instantiation can override                |
| Inconsistency between dual paths (native + registry) for primitive types            | Gate: the semantics of core registration must be consistent with native instructions (two views of the same table) |
| Registry query slows down compilation                                               | Table indexed by type name + instantiation result caching (reuses RFC-011a proof)                                  |
| Ambiguity introduced by instantiation-level overloading                             | Same rule as method-level overloading: signature conflict → E1097                                                  |

## Coordination with Other RFCs

This RFC is positioned as a **consumer-side requirements initiator**, and the following RFCs need to
be updated synchronously to ensure coordinated consistency (authorized for revision):

| RFC                                    | Content to Update                                                                                                                                                                                                                                                                                     |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **RFC-011** (accepted)                 | §Constraints notes that `Add` / `Multiply` are defined and landed by 011b (`T: Add` ≜ `Add(T, T, T)`); marks `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` as dangling constraint names (to be addressed by subsequent RFCs); §8.3 lifting type family notes unification with the interface registry |
| **RFC-010** (accepted)                 | Clarify the landing requirement of "record fields are constructors" — it is the prerequisite for retiring the constructor parser special case in Phase 2                                                                                                                                              |
| **RFC-010b** (draft, original RFC-039) | Construction and deconstruction must come in pairs; exhaustiveness judgment does not depend on `Result`'s core/std attribution (Phase 2 of 011b will migrate)                                                                                                                                         |
| **RFC-009** (accepted)                 | §Type attributes cross-reference: `Equal` precondition is "not containing `&mut` linear tokens" (not Dup)                                                                                                                                                                                             |
| **RFC-013** (accepted)                 | When Phase 2 lands: `E1081` / `E1082` text removes the word "Result"; `Equal` precondition diagnostics reuses the `E1101` family — three-party sync per RFC-013 process (codes/*.rs ↔ locales ↔ code table)                                                                                           |
| **RFC-018** (accepted)                 | After `%` changes to mathematical modulo, the `Mod → srem/urem` mapping table becomes invalid and needs to be changed to `srem` + sign correction (or composed of `sdiv`+`mul`+`sub`), and the mixing of the terms "modulo / remainder" needs to be corrected                                         |
| **RFC-036** (accepted)                 | No change required. Note the relationship: `Any`-position `==` retains runtime comparison, and the `assert_eq` assertion family is not affected by the gate                                                                                                                                           |

> The basis for `Result` being attributed to std: RFC-013 has already positioned "std library
> `Result(T, Error)`", and the layering in RFC-014 attributes std to the core source. Phase 2 of
> this RFC is one of its landing paths, and no other numbers are cited.

## Open Questions

- [x] ~~Does the semantic fix of `Modulo` need a compatibility period?~~ → **Closed**: the language
      reference has long stated "multiplication/division modulo", and the current remainder
      implementation violates the already-published documentation, so it is handled as a defect fix
      with no compatibility period (2026-09-22)
- [x] ~~Can users supplement operator implementations for existing types (orphan rule)?~~ →
      **Decision**: operator implementations can only be written in the module where the type is
      defined. `Int` is defined in core, so only the core can register for it; users can only
      register for their own types. All types are treated equally, with no privileges for built-in
      types (2026-09-22)
- [ ] Does `Index` need to distinguish mutable indexing (similar to Rust's `IndexMut`)? (First batch
      is read-only; mutable indexing involves RFC-009's `WriteToken`, and the `&mut Self` receiver
      precedent already exists in RFC-011a, so it is left for follow-up)
- [ ] Multi-position index keys: use tuple packing (current) or change to multiple parameters (Swift
      style)? (Continue with tuple packing, do not change the parser)
- [ ] The shape of `Zero` / `One`: constant members are not operators, and "interface members
      without a receiver" have no precedent in RFC-011a; they need to be finalized separately before
      the entire sentence `T: Add + Multiply + Zero` of RFC-011 can be delivered
- [ ] The full shape of the `Try` interface: `?` requires three things — success/failure judgment,
      success payload extraction, and producing the outer return value on the failure path — a
      single `residual` method is not enough; finalized together with the retirement of the
      constructor parser special case before Phase 2 begins
- [ ] The `?T` prefix type (RFC-026 FFI nullable annotation, RFC-018) shares the `?` symbol with the
      `e?` suffix operator: the position is different (type position vs expression position) and
      does not constitute a conflict; documentation is sufficient
- [ ] `PartialOrd` / `Ordering` (interfacification of comparison operators): independent RFC, this
      RFC explicitly does not do it

---

## Appendix A: Investigation Evidence

The following tests were all reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Interface Mechanism Availability (Foundation of this RFC)

| Capability                                                                     | Verification                                                   |
| ------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Can be defined                                              |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runnable, output correct (unit tests in `tests/rfc011a.rs`) |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (field and method share namespace conflict)         |
| Method dispatch `d.speak()`                                                    | ✅ Runnable                                                    |

**Note**: `E1097` means that operator methods must use the **external declaration** form
(`Point.add: (self: &Point, ...)`), consistent with the examples in RFC-011a. Registration of
`method_bindings` is in `environment.rs` (`add_method_binding`), and querying is in the call-target
resolution and field-lookup-failure fallback path of `expressions.rs`.

### A.2 Current State of Operators

| Expression                          | Current State                                                                                       |
| ----------------------------------- | --------------------------------------------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]` | ✅ Hardcoded whitelist (Int/Float/String/List, **requires both sides to be of the same type**)      |
| `1 + 2.5` (Int + Float)             | ❌ Compile error (whitelist requires both sides to be of the same type)                             |
| `-7 % 3`                            | `-1` (truncated remainder; `%` whitelist only includes Int/Float, different from the `+` whitelist) |
| `Point(1,2) == Point(1,2)`          | ❌ `E6007` (Eq fails at runtime on Struct)                                                          |
| `(1,2) == (1,2)` / `[1] == [1]`     | ✅ Runtime element-wise comparison (Struct is the only gap)                                         |
| Any-position `a == b` (`assert_eq`) | ✅ Runtime comparison (verified by RFC-036)                                                         |
| `f[0]` (function position binding)  | ⚠️ Only legal within the binding declaration, `E3006` when used as an expression                    |
| `arr[0, 1]` (multi-position)        | ✅ Tuple packing, `list([1, 2])`                                                                    |

### A.3 Hardcoded Locations of `?` and Constructors

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Constructor side: `ok(T)` / `err(E)` / `some(T)` are recognized by the parser (language
specification `syntax.md` §1.4.2); `is_ok` / `unwrap` etc. in `std/result.rs` pattern-match
according to `variant_id 0/1`. "Result belongs to std" requires both halves (`?` + constructors) to
be untied together, both falling under Phase 2.

## Appendix B: Design Decision Records

| Decision                                   | Decision                                                                                                      | Reason                                                                                                                                                                 | Date       |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Architecture                               | Three-layer separation (mapping table / dispatch substrate / interface contract)                              | Merging would decouple RFC-011 constraints from operators                                                                                                              | 2026-09-15 |
| Operator gating condition                  | Must implement the interface (Layer 2 is the gating condition)                                                | Ensures `T: Add` and `+` correspond strictly                                                                                                                           | 2026-09-15 |
| Interface naming                           | Spelled out in full (`Multiply` instead of `Mul`)                                                             | Does not modify the body of already-accepted RFC-011                                                                                                                   | 2026-09-15 |
| `%` interface name                         | `Modulo` (mathematical modulo)                                                                                | The name pins down the semantics                                                                                                                                       | 2026-09-15 |
| Comparison operators                       | **Not** interfacified in the first batch, retain native IR instructions                                       | Already first-class instructions; `Ordering` brings up a whole set of independent issues                                                                               | 2026-09-15 |
| `Ordering`                                 | Not introduced in the first batch                                                                             | No real demand driving it, the size of an independent RFC                                                                                                              | 2026-09-15 |
| Associated type                            | Use interface type parameter, do not introduce `type` member syntax                                           | RFC-011a has already settled on this scheme (`Iterator: (Item: Type)`)                                                                                                 | 2026-09-15 |
| Unification of method binding and indexing | Conceptually unified, interface only handles container indexing                                               | The key of position binding is a compile-time constant, and the result type requires type family evaluation, which cannot be implemented by users                      | 2026-09-15 |
| Multi-position index                       | Tuple packing + overloading by Key type, no variadic interface                                                | Does not change the parser                                                                                                                                             | 2026-09-15 |
| Bitwise / unary operators                  | Not done in the first batch                                                                                   | Rare for user-defined types, YAGNI                                                                                                                                     | 2026-09-15 |
| Arithmetic interface shape                 | Three type parameters `(Self, R, O)`; `T: Add` ≜ `Add(T, T, T)`                                               | Result type made explicit: `1 + 2.5`, scaling can be expressed; RFC-011 §8.3 lifting table is unified with the registry; unified with the shape of `Index(Key, Value)` | 2026-09-22 |
| `Equal` derivation                         | Auto-derived by default + explicit instantiation to override; constraint resolution shares the same criterion | "Types you wrote yourself cannot ==" is unacceptable; Tuple/List are already element-wise compared, Struct is the only gap                                             | 2026-09-22 |
| `Equal` precondition                       | "Not containing `&mut` linear tokens", **not Dup**                                                            | RFC-011 §2.4 explicitly states primitives are not subject to Dup; using Dup as the premise would wrongly reject `Point{Float,Float}`                                   | 2026-09-22 |
| Separation of names and registry           | Operators only look up the interface implementation registry, not through name resolution                     | Local same-name bindings (type-level `Add` family etc.) do not interfere with operators; the example in RFC-011 §5.2 does not need to be changed                       | 2026-09-22 |
| Orphan rule                                | Implementation follows the module where the type is defined                                                   | All types are treated equally, built-in types have no privileges                                                                                                       | 2026-09-22 |
| `Any`'s `==`                               | Retain runtime comparison, do not look up the registry                                                        | RFC-036's `assert_eq` assertion family already depends on this behavior                                                                                                | 2026-09-22 |
| `Try` shape                                | Finalized: four methods (is_failure/success/residual/from_error) + assert-Never dead-end semantics; landed    | `?` requires three things, a single `residual` method is not enough; constructor parser special case is retired together with Result being attributed to std           | 2026-09-22 |
| `%` semantic classification                | Defect fix (documentation has long promised modulo), no compatibility period                                  | `reference/index.md` "multiplication/division modulo" is the prior evidence                                                                                            | 2026-09-22 |
| Basis for `Result` being attributed to std | Cite RFC-013's existing position, no longer cite non-existent numbers                                         | RFC-013 already writes "std library `Result(T, Error)`"                                                                                                                | 2026-09-22 |

## Appendix C: Glossary

| Term                              | Definition                                                                                                                                                                                           |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layer 0                           | Fixed mapping table from operator to method name, a language-level constant, not user-modifiable                                                                                                     |
| Layer 1                           | Dispatch substrate that looks up the table by the `Type.method` key and calls it                                                                                                                     |
| Layer 2                           | Interface contract layer, providing the basis for generic constraints, and the gating condition for operators                                                                                        |
| Interface implementation registry | The implementation master table that aggregates core default registration and user instantiations; the only criterion for operator queries and constraint resolution, independent of name resolution |
| Native fast path                  | The hardcoded operation path retained for primitive types, not going through interface dispatch                                                                                                      |
| Position binding                  | RFC-004's `f[0]` syntax, binding function parameter positions as methods, a compile-time behavior                                                                                                    |
| Auto-derivation                   | The field-by-field `==` generated by the compiler for records whose fields are all comparable; explicit instantiation can override it                                                                |

## References

- [RFC-011: Generic Type System Design](./011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraint, associated type
- [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
  interface declaration / instantiation / overloading rules
- [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear token
- [RFC-004: Multi-Position Binding for Curried Methods](./004-curry-multi-position-binding.md) —
  `f[0]` syntax
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type expression
- [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` attribution to
  std position, error code process
- [RFC-010b: Pattern Matching Completion (Variant Deconstruction and Exhaustiveness)](../accepted/010b-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — associated type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — multi-parameter subscripts
