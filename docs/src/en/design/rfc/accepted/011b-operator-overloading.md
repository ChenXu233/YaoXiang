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
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md) — type constraints
>   `T: Add + Multiply`, associated types
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
>   interface declaration/instantiation mechanism
> - [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear token
> - [RFC-004: Multi-Position Joint Binding for Curried Methods](./004-curry-multi-position-binding.md)
>   — `f[0]` position binding syntax
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type = record where all
>   fields return their own type
> - [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` 's existing
>   positioning under std, E108x
> - [RFC-010b: Pattern Matching Completeness (Variant Deconstruction and Exhaustiveness)](../accepted/010b-pattern-matching-completeness.md)
>   — variant deconstruction (dependency)

## Summary

To round out **operator overloading** capability for YaoXiang, enabling `a + b` / `a == b` / `a[i]`
/ `e?` to be implemented by user-defined types, and making the **operator constraints**
(`T: Add + Multiply`) already written into RFC-011's constraint syntax go from **paper capability**
to a landable mechanism (`Zero` in the same sentence is not an operator, see Open Questions).

The design adopts **three-layer responsibility separation**: a fixed operator→method mapping table
(Layer 0), a name-based dispatch foundation (Layer 1, reusing the existing `method_bindings`), and
an interface contract layer (Layer 2, for generic constraints). Operator **precedence and
associativity remain language-fixed**; users only overload semantics.

The first batch of scope: `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` seven
interfaces. Arithmetic interfaces adopt **three type parameters** `(Self, R, O)`—the result type `O`
is made explicit, so that heterogeneous operations like `1 + 2.5` (returns `Float`) and
`point * 2.0` (scaling) can be expressed. `Equal` is **auto-derived by default** (records with all
comparable fields automatically receive field-by-field `==`); explicit instantiation can override.

`Try` (the interfacing of `?`) is moved entirely to Phase 2: it depends on RFC-010 (construction)
and RFC-010b (deconstruction) landing, and the interface shape is not yet finalized (see Open
Questions).

**No new syntax, no new keywords**, all reusing the existing mechanisms of RFC-011a (interface
declaration / instantiation / external method declaration / overloading).

## Motivation

### Why This Feature Is Needed

#### 1. RFC-011's core example depends on it, and the current state is worse than "unable to deliver"

RFC-011 (accepted) uses operator names as type constraints in 8 places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

None of the RFCs in the series ever defines where `Add` / `Multiply` come from or how `+` binds to
them. Empirically confirmed the situation is even worse: `T: Add` **directly fails to compile
today**—constraint solving queries the old trait table (`trait_data.rs`), which doesn't contain
`Add`. Constraint names `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` are similarly dangling (this
RFC's handling is in "Coordination with Other RFCs").

#### 2. User-defined types cannot participate in basic operations (empirically verified)

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

Knock-on consequence: `list.contains(list_of_structs, p)` **completely unusable**—it internally
depends on `==`. Meanwhile, `(1, 2) == (1, 2)` and `[1] == [1]` use runtime element-by-element
comparison and **have always worked**—structs are the only gap.

#### 3. `?` welds the type name into the compiler, blocking `Result` from being moved to std

The implementation of `?` hardcodes both the type name and variant numbers:

```rust
// typecheck: hardcoded construction of Result type
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hardcoded group name and variant number
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` is changed to **interface-driven**, any type that
implements the interface (including user-defined types) can be used by `?`, and `Result` can belong
to std (RFC-013 already has the positioning of "std library `Result(T, Error)`").

Note this is only half the problem: the `ok(...)` / `err(...)` / `some(...)` construction syntax is
currently also welded in the parser (the language specification §1.4.2 lists them as "constructors
recognized by the parser"). "Result belongs to std" requires both halves of `?` and constructors to
be **unwound together**, both assigned to Phase 2 of this RFC.

#### 4. User-defined containers cannot be indexed

The type checking for `Index` is a hardcoded whitelist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// Otherwise → "Containers not recognized at the type level are refused, not silently accepted"
```

Any container type defined by users, `c[0]` always errors out.

#### 5. The implementation of `%` violates published documentation

Empirically, `-7 % 3` returns `-1` (truncated remainder). But the operator precedence table in the
language reference (`reference/index.md`) already states "`* / %` multiplication/division
**modulo**"—the documentation promises modulo, but the implementation gives remainder. This is not a
design change, it's a **defect where implementation violates documentation**, and this RFC fixes it
as a side note.

### Existing Semi-Complete Foundations

Investigation found that the mechanism is **mostly already in place**; what's missing is the wiring:

| Mechanism                                                           | Location                                                                                                               | Status                                                                                            |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Interface declaration `(Self: Type) -> Type`                        | RFC-011a Phase 1                                                                                                       | ✅ Runnable                                                                                       |
| Interface instantiation `Animal(Dog)` + external method declaration | RFC-011a Phase 2–3                                                                                                     | ✅ Runnable (with unit tests)                                                                     |
| Method dispatch `method_bindings["Type.method"]`                    | `expressions.rs` (registered in `environment.rs`, queried in `expressions.rs` call resolution and field fallback path) | ✅ Runnable                                                                                       |
| Associated types (= interface type parameters)                      | RFC-011 §3.1; RFC-011a has adopted this approach                                                                       | ✅ Mechanism decided                                                                              |
| Type family evaluation `AssociatedTypeDef`                          | `dependent_types.rs`                                                                                                   | ✅ Production use case: `std.assert`'s `IsTrue`                                                   |
| `Equal` / `Dup` / `Clone` / `Debug` trait                           | Registered in `trait_data.rs`                                                                                          | ⚠️ `Equal` has zero consumption; old auto-derive only registers signatures with no implementation |
| Operator → method name mapping                                      | —                                                                                                                      | ❌ Doesn't exist                                                                                  |

**Conclusion**: This is not building a feature from scratch, but wiring up existing parts and
documenting them.

## Proposal

### Core Design: Three-Layer Responsibility Separation

```
Layer 0  Fixed mapping table (language-level constant, user-immutable)
         + → add    == → equal    [] → index    ? → residual
         │  Compile-time built-in, doesn't participate in type inference, not exposed to users
         ▼
Layer 1  Dispatch foundation (look up method by name and call)        ← Reuses existing mechanism
         method_bindings["Point.add"]
         ▼
Layer 2  Interface contract (for generic constraints)
         Add / Equal / Index / Try …
         Makes T: Add constraint valid, and serves as the gating condition for operators
```

**Rationale for layering**:

- **Layers 0 and 1 make operators "usable"**, Layer 2 makes operators "constrainable". Merging would
  cause any method named `add` to be called by `+`, disconnecting RFC-011's `T: Add` from operators.
- **Layer 1 doesn't reinvent**: `method_bindings` already looks up by the `Type.method` key (the
  fallback path after field lookup fails), operators can take the same path.
- **Layer 2 is the gating condition**: before allowing an operator, it **must** be confirmed that
  the type implements the corresponding interface (see the `Equal` auto-derive exception—derivation
  and registration are the same criterion).

### Names and Registration: Two Independent Channels

**Operators query the "interface implementation registration table", not ordinary name resolution.**

- The registration table is owned by the core, aggregating registrations from two places: default
  registrations done by the core for basic types (`Add(Int, Int, Int)`, etc.), and interface
  instantiations written by users in type bodies (`Add(Point, Point, Point)`). No matter which
  module the implementation code physically resides in, registrations flow into the same table; `+`
  / `==` / `[]` only look at this table.
- A **same-named binding** locally defined in a module (such as the Peano type-level addition
  `Add: (A: Type, B: Type) -> Type = match ...` from RFC-011 §5.2) goes through the **name lookup
  channel**, only affecting the resolution of the name `Add` within that module, **doesn't touch the
  registration table, doesn't affect operator availability**. Same name, different things, no
  shadowing of each other.

This also answers "Whether RFC-011's type-level `Add` conflicts with this RFC's interface `Add`": no
conflict. Type-level `Add` is purely type-level computation (Zero/Succ are types, not values, and
there will never be value operations like `Zero + Succ(...)`); the value-level operator interface is
two aspects of the same name.

### Layer 0: Fixed Mapping Table

| Operator             | Interface                                                     | Method             | First Batch |
| -------------------- | ------------------------------------------------------------- | ------------------ | ----------- |
| `+`                  | `Add`                                                         | `add`              | ✅          |
| `-`                  | `Subtract`                                                    | `subtract`         | ✅          |
| `*`                  | `Multiply`                                                    | `multiply`         | ✅          |
| `/`                  | `Divide`                                                      | `divide`           | ✅          |
| `%`                  | `Modulo`                                                      | `modulo`           | ✅          |
| `==` `!=`            | `Equal`                                                       | `equal`            | ✅          |
| `[]`                 | `Index`                                                       | `index`            | ✅          |
| `?`                  | `Try` (four methods, decided in Phase 2)                      | `is_failure`, etc. | ✅ Landed   |
| `<` `<=` `>` `>=`    | — (reserved as native instructions)                           | —                  | ❌          |
| `and` `or`           | — (short-circuit is language semantics, cannot be overloaded) | —                  | ❌          |
| 5 bitwise operations | —                                                             | —                  | ❌          |
| Unary `-` `!`        | —                                                             | —                  | ❌          |

**Interface names use full spelling rather than abbreviations** (`Multiply` instead of `Mul`):
consistent with RFC-011's body text `T: Add + Multiply + Zero`, **without modifying already-accepted
RFCs**.

**`%` adopts `Modulo` semantics** (mathematical modulo, result sign follows divisor). The motivation
section has already confirmed that the current remainder implementation violates published
documentation (`reference/index.md`'s "multiplication/division modulo"); this item is handled as a
defect fix, no compatibility period.

Status note: The type-checking whitelist for `%` currently only has Int/Float (different from `+`'s
Int/Float/String/List whitelist); see Appendix A.2 for empirical records.

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

The three type parameters have distinct roles:

- `Self`: the left operand type (receiver, borrowed as `&Self`, see RFC-009 borrow tokens and
  RFC-011a receiver convention);
- `R`: the right operand type—**left and right can be heterogeneous**;
- `O`: **result type**, explicitly declared.

**Why the result type must be an explicit parameter** (rather than hardcoded `-> Self`):

1. If hardcoded as `Self`, the method of `Add(Int, Float)` must return `Int`, and `1 + 2.5` cannot
   be correctly expressed;
2. Once the result type is explicit, the lifting type family
   `Add: (A, B) -> Type = match (A, B) { (Int, Float) => Float, ... }` from RFC-011 §8.3 and this
   RFC's interface registration **become two views of the same table**—the core registration
   `Add(Int, Float, Float)` is exactly the `(Int, Float) => Float` row, and each user instantiation
   adds a row to this table;
3. The `Index` interface is already three-parameter `(Self, Key, Value)`, with the return type
   `Value` as a parameter—after arithmetic interfaces add `O`, the entire operator interface family
   has a uniform shape, and the one hardcoded as `Self` is the odd one out.

**Core default registrations** (native instruction path, doesn't go through method calls):
`Add(Int, Int, Int)`, `Add(Int, Float, Float)`, `Add(Float, Int, Float)`,
`Add(Float, Float, Float)`, `Add(String, String, String)` (concatenation),
`Add(List(T), List(T), List(T))` (element concatenation), etc.; the five arithmetic interfaces work
similarly for basic types. `1 + 2.5` changes from the current compile error (whitelist requires same
type on both sides) to a legal operation returning `3.5: Float`.

**Constraint syntax sugar**: `T: Add` ≜ already-registered `Add(T, T, T)`—homogeneous
self-composition, result is still the type. The `a * b + c` in RFC-011's matrix multiplication
example has type `T` throughout, which is exactly this meaning. `T: Equal` is similarly ≜
`Equal(T, T)`.

**Heterogeneous example**—vector scaling:

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

#### Equality Interface (Default Auto-Derive)

```yaoxiang
Equal: (Self: Type, R: Type) -> Type = {
    equal: (self: &Self, other: &R) -> Bool
}
```

**`Equal` is auto-derived by default**, with five rules:

1. **Default derive**: when defining a record type, if all fields are comparable (basic types,
   `String`, or records/tuples/lists that are themselves comparable, and not containing `&mut`
   fields), the compiler automatically generates a **field-by-field comparison** `==`, going through
   native code path, without generating user-visible methods. User-defined types are naturally
   comparable without any ceremony.
2. **Explicit override**: if the type body writes `Equal(Point, Point)` and provides a `Point.equal`
   method, the user's version is used (e.g., floating-point comparison with tolerance), and
   auto-derive no longer applies.
3. **When fields are not comparable**: auto-derive fails, `==` is unavailable, diagnosis indicates
   which field is the cause; at this point users can still manually write `Equal` for custom
   comparison (e.g., comparing function fields by name).
4. **Precondition**: types containing `&mut T` linear tokens (recursive fields) do not participate
   in comparison—linear tokens are consumed upon single read, and two values cannot be retrieved
   simultaneously for comparison. Note the prerequisite is "non-linear" rather than "Dup": primitive
   value types (Int/Float/Bool/Char) per RFC-011 §2.4 do not belong to Dup (they are compiler
   built-in value copy); if Dup were the prerequisite, `Point { x: Float, y: Float }` would be
   mistakenly rejected.
5. **Constraints use the same criterion**: `T: Equal` solving and `==` gating use the same "check
   registration or structural derivation" rule—constraints and operators always give the same
   answer.

**Basis for consistency**: `(1, 2) == (1, 2)` and `[1] == [1]` already use runtime
element-by-element comparison today (the comparison whitelist in `executor.rs` includes
Tuple/List/Array); record types are the only composite type being excluded. Auto-derive is not a new
silent default, but rather filling in the last piece of the language's existing internal behavior.

**Old mechanism retirement**: the old auto-derive of `Equal` in `trait_data.rs` (only registers
signatures, no implementation code, zero consumption) is discontinued; all determination of `Equal`
(basic type default registration, structural derivation, explicit instantiation) goes through the
interface registration table. The old trait table retains the existing responsibilities of
Clone/Dup/Debug (names don't overlap with operator interfaces, unification to be discussed later).

#### Index Interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` as a type parameter rather than an associated type member**: RFC-011a's open question has
been decided on "associated types implemented through generic interface parameters"
(`Iterator: (Item: Type) -> Type` is an isomorphic precedent), no need to introduce `type` member
syntax.

**Ownership note**: `index` returns a complete `Value` from the `&Self` borrow. The standard
library's `list.get: (&Vec(A), Int) -> A` is already in the same form; **this interface is on equal
footing with the std status quo**; the precise semantics of "extracting complete value from borrow"
for move-semantic element types is left to RFC-009's full enforcement to handle uniformly, this RFC
does not invent new rules for this.

**Multi-position indexing relies on tuple packing + overloading**, no variadic interface introduced:

```yaoxiang
// One-dimensional container
List(T) instantiates Index(List(T), Int, T)                   → arr[0]

// Multi-dimensional container
Grid    instantiates Index(Grid, Tuple(Int, Int), Float)      → g[0, 1]
//                    └─ Key is a tuple

// Two instantiation signatures differ → coexist
```

**Same-typed same-named interfaces allow multiple instantiations**, differentiated by the signatures
of their injected methods, coexisting per RFC-011a's method-level overloading rules. RFC-011a's
overloading explicitly only goes up to method-level; this item adds a layer of instantiation-level
rules on top: the legality of same-named interface instantiations coexisting is determined by
whether the expanded method signatures conflict (conflict is E1097, originating from the same source
as field/method namespace rules).

#### Propagation Interface Try (Phase 2 Finalized and Landed)

The interface name for `?`'s interfacing is `Try`, with a four-method shape (Phase 2 finalized
2026-09-22):

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    is_failure: (self: &Self) -> Bool,
    success:    (self: &Self) -> T,
    residual:   (self: &Self) -> E,
    from_error: (E) -> Self,
}
```

- **Semantic division of labor**: `is_failure` judges success or failure, `success` extracts the
  success payload, `residual` extracts the failure payload, `from_error` bridges cross-type
  propagation (when `T` of `f()?` ≠ outer `U`, rebuild the failure value from `E`). The lowering of
  `?` uniformly generates a four-method call chain—if `is_failure(t)` is true,
  `Ret from_error(residual(t))`, otherwise the expression value is `success(t)`; no more
  hand-written variant checking sequences, `Result` / `Option` (std yx implementation) and
  user-defined Try types take the same path.
- **Dead branch**: the failure arm of `success` and the success arm of `residual` are contractually
  unreachable, the implementation uses `assert(false)` to diverge (`assert` returns `Never`,
  divergence principle `Never <: T` gates through, type-system.md §2.2).
- **Checking**: typecheck queries the interface implementation registration table (Self position
  nominal matching, abstract entries instantiated by the scrutinee's actual arguments); the outer
  function's return type must also implement `Try` and the `E` position can catch the failure value
  (E1081/E1082/E1083 semantics become interface-driven as a result).
- **`Result` belongs to std**: the type definitions of `Result` / `Option` and Try implementations
  migrate to `std/result.yx` / `std/option.yx` (pure YaoXiang), native `ok`/`err` constructors are
  retired—variant construction syntax `Result(T, E).ok(v)` is the only construction channel (the
  constructor welding problem is solved along with the parser special case retirement). Option's Try
  residual type takes `Void` (corresponding to Rust Try experiment's NoneT semantics).

### Examples

#### User-Defined Types: Arithmetic and Equality Out of the Box

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
    println(a == b)             # false —— Equal auto-derives, no instantiation needed
    println(a == a)             # true
}
```

#### Custom Equality (Overriding Auto-Derive)

```yaoxiang
Vec3: Type = {
    x: Float,
    y: Float,
    z: Float,
    Equal(Vec3, Vec3),
}

# Float comparison with tolerance, overriding field-by-field auto-derive
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

#### Generic Constraints Finally Fulfillable

```yaoxiang
// RFC-011's example is now landable
// T: Add ≜ Add(T, T, T) registered; T: Multiply ≜ Multiply(T, T, T) registered
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

(The `Zero` / `One` in RFC-011's signature example `T: Add + Multiply + Zero` are not in this RFC's
scope—they are constant members, not operators; "interface members without a receiver" has no
precedent in RFC-011a, see Open Questions.)

### Syntax Changes

**No new syntax, no new keywords**. All capabilities are composed of existing mechanisms:

| Capability              | Reused Existing Mechanism                                              |
| ----------------------- | ---------------------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                                       |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                              |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)                    |
| Associated type         | RFC-011 §3.1 (interface type parameters)                               |
| Multi-position indexing | Existing tuple packing parsing + instantiation-level overloading rules |
| Operator precedence     | **Language-fixed**, user-customization not opened                      |

## Detailed Design

### Type System Impact

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` (`Try`
in Phase 2).

**Interface implementation registration table**: the central nervous system of operator
determination. Aggregates core default registrations (basic types) and user instantiations (the
`ImplementationProof` produced by `check_interface_instantiation`); the gating check for `+` / `==`
/ `[]` and `T: Add` constraint solving (`check_trait_bounds` in `bounds.rs`) **query the same
table**—there is no gap of "constraint says yes, operator says no".

**`Equal`'s structural derivation**: a structural rule layered on top of the registration table (all
fields comparable ⇒ comparable); basic types are backed by core registration, recursive closure. The
old `Equal` registration and old auto-derive in `trait_data.rs` are discontinued.

**Separation of operator query and name resolution**: operators like `+` only check the registration
table, not ordinary name resolution; local same-named bindings (such as the type-level `Add` family)
don't affect operators (see §Names and Registration).

**Orphan rule**: operator implementations can only be written in the **type's definition
module**—`Int`'s definition is in the core, so `Int`'s registration can only be written by the core;
`Point`'s definition is in the user module, so only its definer can register for it. All types obey
the same rule, built-in types have no special privilege.

### Runtime Behavior

**Zero runtime overhead**: operators determine call targets at compile time (static dispatch). For
basic types (`Int`/`Float`/`String`/`List`), **native instruction fast paths** are preserved, no
interface dispatch; auto-derived struct `==` generates native field-by-field comparison; `Any`
(dynamic type) position's `==` / `!=` maintains existing runtime comparison (RFC-036 test framework
`assert_eq` depends on this behavior, not affected).

**`%` semantic fix**: `-7 % 3` changes from `-1` (truncated remainder) to `2` (mathematical modulo).
Three locations are modified in sync: interpreter (`checked_rem`), constant folding (`a % b`),
bytecode (`I64_REM`).

### Compiler Changes

| Component                            | Changes                                                                                                                                                                                                                         |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | `infer_binary` whitelist changed to "native fast path + registration table query" dual path; `Expr::Index` whitelist adds interface query branch                                                                                |
| `typecheck/inference/bounds.rs`      | `check_trait_bounds` for operator interface names changed to query interface registration table (`Equal` includes structural derivation)                                                                                        |
| `typecheck/checker.rs`               | New `Equal` structural derivation (after record definition); instantiation-level overloading rules (same-named interface multiple instantiations coexist by method signature)                                                   |
| `middle/core/ir_gen.rs`              | `Expr::Index` adds method call dispatch branch; struct `==` without explicit `equal` generates native field-by-field comparison; `Mod` instruction changed to mathematical modulo                                               |
| Interface registration table (new)   | Layer 0 mapping table constants + operator→interface query function + core default registrations (`Int`/`Float`/`String`/`List`, etc.)                                                                                          |
| `trait_data.rs`                      | `Equal` registration and old auto-derive discontinued (`Clone`/`Dup`/`Debug` maintain status quo)                                                                                                                               |
| Diagnostics                          | `Equal` precondition (linear token) unsatisfied reuses `E1101` (type does not implement interface) family; `E1081`/`E1082` text removes the word "Result" (Phase 2 follows `Try` landing, three-party sync per RFC-013 process) |

### Backward Compatibility

**Basic type operations unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, `1 < 2` all retain native
paths, behavior and performance unchanged.

**`1 + 2.5` changes from compile error to legal**: after core registration `Add(Int, Float, Float)`,
originally rejected by the whitelist, mixed arithmetic is now usable, returning `Float`. This is an
added capability, no existing code is affected.

**`%` semantic fix**: `-7 % 3` changes from `-1` to `2`. Qualified as **defect fix** (documentation
long promised modulo, see Motivation §5), no compatibility period; existing test corpus has no cases
depending on negative `%` (verified).

**Struct `==` changes from runtime error to usable**: previously `Struct == Struct` always produced
E6007 runtime error, after wiring it works via auto-derive—from broken to working, no existing legal
code is affected.

**`Any`'s `==` unaffected**: dynamic type position maintains runtime comparison (RFC-036's
`assert_eq` assertion family depends on this, previously usable, still usable afterwards).

**`f[0]` position binding unchanged**: `distance[0]` is RFC-004's compiler built-in capability,
**doesn't go through the `Index` interface**, not affected.

**`?` transparent to existing code** (Phase 2): after `Result` adds the `Try` instantiation,
existing `?` usage behavior is completely consistent.

## Trade-offs

### Advantages

- **Fulfills RFC-011's operator constraints**: `T: Add + Multiply` changes from paper (actually a
  compile error) to landable, without modifying the already-accepted RFC body
- **Releases `Result`'s core binding** (Phase 2): after `?` and constructor interface-ization,
  `Result` can belong to std, aligning with RFC-013's existing positioning
- **Fixes empirical defects**: `Point == Point` works out of the box, concurrently fixing
  `list.contains` being unusable for structs
- **Zero new syntax**: fully reuses RFC-011a's existing mechanisms, doesn't touch parser syntax
  rules
- **Zero runtime overhead**: static dispatch + basic type native fast paths
- **User-defined containers usable**: `Box(T)[0]` changes from "always error" to usable
- **Internal language consistency**: Tuple/List already have element-by-element comparison, record
  types are filled in; arithmetic interface three parameters unified with Index shape; RFC-011 §8.3
  lifting table unified with interface registration

### Disadvantages

- **`Equal` auto-derive is a silent default**: if you want to retract "records are comparable by
  default" in the future, it's a breaking change. This position is accepted—consistent with existing
  Tuple/List behavior, and explicit instantiation can always override
- **`Equal` precondition will reject types containing linear tokens**: types containing `&mut`
  fields cannot `==`, this is the cost of semantic correctness, requires clear diagnostics
  (indicating which field)
- **Interface instantiation is explicit cost**: each arithmetic operator requires a line of
  instantiation + a method (`Equal` is exempt—auto-derive). RFC-011a's syntax determines that
  implicit derivation is not possible (in exchange, `Self` type parameter has no magic)
- **`%` semantic fix is a behavior change**: although qualified as a defect fix, it still needs to
  be noted in the migration guide

## Alternatives

### Option A: Only do Layer 1 (dispatch by method name), not the interface layer

`+` only checks if there's a method named `add`, doesn't require implementing the `Add` interface.

**Reason for rejection**: RFC-011's `T: Add` constraint will be disconnected from
operators—constraints check interfaces, operators check method names, the two can give inconsistent
answers. And it cannot provide accurate diagnosis at compile time for "`+` used on non-addable
type".

### Option B: Introduce constructor syntax `Ok(x)` / `Some(x)` to solve the `?` problem

Don't interface-ize `?`, but add constructor syntax for sum types.

**Reason for rejection**: conflicts with RFC-010 (accepted). RFC-010 explicitly states "uniformly
use record types to express sum types, **no need for two sets of syntax**", and explicitly
deprecates the `|` syntax. Introducing constructors is introducing a second set of expressions. And
it only solves `?`, not `Point == Point` and custom container indexing.

### Option C: Comparison operators also interface-ized in the first batch (introduce `Ordering`)

`<` `<=` `>` `>=` go through the `Compare` interface, returning the three-value `Ordering`.

**Reason for rejection**: `<` is already a **first-class IR instruction** in YaoXiang
(`Instruction::Lt/Le/Gt/Ge`), interface-ization would force basic types to take a detour. And
introducing `Ordering` would bring out a whole set of issues such as `Ordering`'s own
comparison/sorting, float `NaN`'s `PartialOrd` vs `Ord`, which is the size of an independent RFC.
The empirically exposed needs (`Point == Point`, `list.contains`) **only need `Equal`**.

### Option D: Operator names use abbreviations (the method of the `Add` interface is called `add`, the interface is called `Mul`)

**Reason for rejection**: RFC-011's body already writes `T: Add + Multiply + Zero`, using
abbreviations would require modifying already-accepted RFCs.

### Option E: `Equal` only does explicit instantiation, no auto-derive

**Reason for rejection**: user-defined types cannot `==`, unacceptable experience; and
`(1, 2) == (1, 2)`, `[1] == [1]` already use element-by-element comparison today, only record types
are excluded, inherently inconsistent. Explicit instantiation is preserved as an override means,
accommodating custom needs.

### Option F: Arithmetic interface return type hardcoded as `-> Self`

**Reason for rejection**: the method of `Add(Int, Float)` will be forced to return `Int`, `1 + 2.5`
cannot be correctly expressed; vector scaling `Multiply(Point, Float)` similarly cannot be written
out. And the lifting type family `(Int, Float) => Float` from RFC-011 §8.3 will lose its landing
spot. Three parameters `(Self, R, O)` is unified with `Index(Key, Value)` shape.

## Implementation Strategy

### Dependencies

| Dependency                             | Status        | Which part of this RFC is affected                                                 |
| -------------------------------------- | ------------- | ---------------------------------------------------------------------------------- |
| RFC-011a interface mechanism Phase 1–3 | ✅ Landed     | Layer 2 foundation (empirically runnable)                                          |
| RFC-010 record-based construction path | ❌ Not landed | **Phase 2**: construction side of `?` + constructor parser special case retirement |
| RFC-010b variant deconstruction        | ❌ On paper   | **Phase 2**: `Result.residual`'s `match` syntax, exhaustiveness                    |
| RFC-009 linear token derivation        | Partial       | `Equal`'s precondition check                                                       |

### Phases

Divided into two groups by **interface dependency** (design constraint, not scheduling):

**Phase 1 — Does not depend on RFC-010/010b**:

- Layer 0 mapping table + interface implementation registration table + Layer 1 dispatch wiring
- `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` seven interfaces (three type
  parameter form)
- `Equal` auto-derive + linear precondition + constraint solving diverted to registration table
- `%` semantic fix (including interpreter/constant folding/bytecode three locations)
- `Any` position `==` maintains runtime comparison
- **Benefits**: `Point + Point`, `Point == Point` (no ceremony), `Box(T)[0]`, `1 + 2.5`, `T: Add`
  constraints all usable

**Phase 2 — Depends on RFC-010 / RFC-010b, `Try` shape must be finalized before starting work**:

- `Try` interface shape finalized (see Open Questions)
- `?` interface-ized + constructor (`ok`/`err`/`some`) parser special case retired, switched to
  RFC-010 record construction path
- `Result` migrates from core to std (based on RFC-013's existing positioning)

### Risks

| Risk                                                               | Mitigation                                                                                                     |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| `%` semantic change affects existing user code                     | Qualified as defect fix + verified test corpus has no negative `%` dependency                                  |
| `Equal` auto-derive's silent default hard to retract in the future | Position decided: consistent with existing Tuple/List behavior, explicit instantiation can override            |
| Basic type dual path (native + registration) inconsistent          | Gate: semantics of core registration must be consistent with native instructions (two views of the same table) |
| Registration table queries slow down compilation                   | Table indexed by type name + instantiation result cache (reusing RFC-011a proof)                               |
| Instantiation-level overloading introduces ambiguity               | Same rules as method-level overloading: signature conflict is E1097                                            |

## Coordination with Other RFCs

This RFC is positioned as a **consumer-side requirements proposer**, requiring synchronous updates
of the following RFCs to ensure coordinated consistency (authorized to modify):

| RFC                                      | Content to update                                                                                                                                                                                                                                                                          |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **RFC-011** (accepted)                   | §Constraints notes `Add` / `Multiply` are defined and landed by 011b (`T: Add` ≜ `Add(T, T, T)`); mark `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` as dangling constraint names (awaiting follow-up RFCs); §8.3 lifting type family notes unification with interface registration table |
| **RFC-010** (accepted)                   | Clarify the landing requirement of "record fields as constructor"—it's the prerequisite for Phase 2 constructor parser special case retirement                                                                                                                                             |
| **RFC-010b** (draft, originally RFC-039) | Construction and deconstruction must be paired; exhaustiveness determination doesn't depend on `Result`'s core/std ownership (migrated by 011b Phase 2)                                                                                                                                    |
| **RFC-009** (accepted)                   | §Type attributes cross-reference: `Equal` precondition is "no `&mut` linear tokens" (not Dup)                                                                                                                                                                                              |
| **RFC-013** (accepted)                   | When Phase 2 lands: `E1081` / `E1082` text removes the word "Result"; `Equal` precondition diagnosis reuses `E1101` family—three-party sync per RFC-013 process (codes/*.rs ↔ locales ↔ code table)                                                                                        |
| **RFC-018** (accepted)                   | After `%` changes to mathematical modulo, `Mod → srem/urem` mapping table becomes invalid, needs to be changed to `srem` + sign fix (or `sdiv`+`mul`+`sub` synthesis), and correct "modulo/remainder" terminology mixing                                                                   |
| **RFC-036** (accepted)                   | No changes needed. Note relationship: `Any` position `==` maintains runtime comparison, `assert_eq` assertion family not affected by the gate                                                                                                                                              |

> Basis for `Result` belonging to std: RFC-013 has positioned "std library `Result(T, Error)`",
> RFC-014's layering places std in core sources. Phase 2 of this RFC is one of its landing paths, no
> longer citing other numbers.

## Open Questions

- [x] ~~Does `Modulo`'s semantic fix require a compatibility period?~~ → **Closed**: The language
      reference has long stated "multiplication/division modulo", the current remainder
      implementation violates published documentation, handled as a defect fix, no compatibility
      period (2026-09-22)
- [x] ~~Can users supplement operator implementations for existing types (orphan rule)?~~ →
      **Finalized**: Operator implementations can only be written in the type's definition module.
      `Int`'s definition is in the core, so only the core can register for it; users can only
      register for their own types. All types are treated equally, built-in types have no special
      privilege (2026-09-22)
- [ ] Should `Index` distinguish mutable indexing (like Rust's `IndexMut`)? (First batch read-only;
      mutable indexing involves RFC-009's `WriteToken`, and RFC-011a has `&mut Self` receiver
      precedent to follow, left for follow-up)
- [ ] Should multi-position indexing's Key use tuple packing (current) or be changed to
      multi-parameter (Swift-style)? (Continue with tuple packing, not changing the parser)
- [ ] `Zero` / `One` form: constant members are not operators, "interface members without a
      receiver" has no precedent in RFC-011a, needs separate finalization before RFC-011's
      `T: Add + Multiply + Zero` entire sentence can be fulfilled
- [ ] Complete shape of the `Try` interface: `?` needs three things—success/failure judgment,
      success payload extraction, failure path producing outer function return value—single method
      `residual` is not enough; finalize together with constructor parser special case retirement
      before Phase 2 work starts
- [ ] `?T` prefix type (RFC-026 FFI nullable annotation, RFC-018) and `e?` suffix operator share `?`
      symbol: position is different (type position vs expression position) does not constitute a
      conflict, just need a written explanation
- [ ] `PartialOrd` / `Ordering` (comparison operator interface-ization): independent RFC, this RFC
      explicitly doesn't do it

---

## Appendix A: Investigation Evidence

The following empirical tests were all reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Interface Mechanism Availability (Foundation of this RFC)

| Capability                                                                     | Empirical                                                     |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Definable                                                  |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runnable, output correct (unit test in `tests/rfc011a.rs`) |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (field and method share namespace conflict)        |
| Method dispatch `d.speak()`                                                    | ✅ Runnable                                                   |

**Note**: `E1097` means that operator methods must use the **external declaration** form
(`Point.add: (self: &Point, ...)`), consistent with the examples in RFC-011a. `method_bindings` is
registered in `environment.rs` (`add_method_binding`), and queried in the call target resolution in
`expressions.rs` and the field lookup failure fallback path.

### A.2 Current State of Operators

| Expression                          | Current State                                                                               |
| ----------------------------------- | ------------------------------------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]` | ✅ Hardcoded whitelist (Int/Float/String/List, **requires same type on both sides**)        |
| `1 + 2.5` (Int + Float)             | ❌ Compile error (whitelist requires same type on both sides)                               |
| `-7 % 3`                            | `-1` (truncated remainder; `%` whitelist is only Int/Float, different from `+`'s whitelist) |
| `Point(1,2) == Point(1,2)`          | ❌ `E6007` (Eq fails at runtime on Struct)                                                  |
| `(1,2) == (1,2)` / `[1] == [1]`     | ✅ Runtime element-by-element comparison (Struct is the only gap)                           |
| Any position `a == b` (`assert_eq`) | ✅ Runtime comparison (RFC-036 evidence)                                                    |
| `f[0]` (function position binding)  | ⚠️ Only legal within binding declaration, `E3006` as expression                             |
| `arr[0, 1]` (multi-position)        | ✅ Tuple packing, `list([1, 2])`                                                            |

### A.3 Hardcoded Locations of `?` and Constructors

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Constructor side: `ok(T)` / `err(E)` / `some(T)` are recognized by the parser (language
specification `syntax.md` §1.4.2); `std/result.rs`'s `is_ok` / `unwrap` etc. pattern-match by
`variant_id 0/1`. "Result belongs to std" requires both halves (`?` + constructor) to be unwound
together, both assigned to Phase 2.

## Appendix B: Design Decision Record

| Decision                                   | Resolution                                                                                                    | Rationale                                                                                                                                                     | Date       |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Architecture                               | Three-layer separation (mapping table / dispatch foundation / interface contract)                             | Merging will disconnect RFC-011 constraints from operators                                                                                                    | 2026-09-15 |
| Operator gating condition                  | Must implement interface (Layer 2 is the gating condition)                                                    | Ensure `T: Add` strictly corresponds to `+`                                                                                                                   | 2026-09-15 |
| Interface naming                           | Full spelling (`Multiply` instead of `Mul`)                                                                   | Don't modify RFC-011's already-accepted body                                                                                                                  | 2026-09-15 |
| `%` interface name                         | `Modulo` (mathematical modulo)                                                                                | Name pins down semantics                                                                                                                                      | 2026-09-15 |
| Comparison operators                       | First batch **not** interface-ized, preserve native IR instructions                                           | Already first-class instructions; `Ordering` brings out a whole set of independent issues                                                                     | 2026-09-15 |
| `Ordering`                                 | Not introduced in the first batch                                                                             | No real demand driver, the size of an independent RFC                                                                                                         | 2026-09-15 |
| Associated types                           | Use interface type parameters, don't introduce `type` member syntax                                           | RFC-011a has decided on this approach (`Iterator: (Item: Type)`)                                                                                              | 2026-09-15 |
| Unification of method binding and indexing | Conceptually unified, interface only handles container indexing                                               | The key of position binding is a compile-time constant, the result type requires type family evaluation, cannot be user-implemented                           | 2026-09-15 |
| Multi-position indexing                    | Tuple packing + overloading by Key type, no variadic interface                                                | Don't change parser                                                                                                                                           | 2026-09-15 |
| Bitwise / unary operators                  | Not done in the first batch                                                                                   | Rare for custom types, YAGNI                                                                                                                                  | 2026-09-15 |
| Arithmetic interface shape                 | Three type parameters `(Self, R, O)`; `T: Add` ≜ `Add(T, T, T)`                                               | Result type explicit: `1 + 2.5`, scaling can be expressed; RFC-011 §8.3 lifting table unified with registration table; shape unified with `Index(Key, Value)` | 2026-09-22 |
| `Equal` derivation                         | Default auto-derive + explicit instantiation override; constraint solving uses same criterion                 | "User-defined types cannot ==" unacceptable; Tuple/List already have element-by-element comparison, Struct is the only gap                                    | 2026-09-22 |
| `Equal` precondition                       | "No `&mut` linear tokens", **not Dup**                                                                        | RFC-011 §2.4 explicitly states primitives don't belong to Dup, using Dup as prerequisite would mistakenly reject `Point{Float,Float}`                         | 2026-09-22 |
| Separation of names and registration       | Operators only query interface implementation registration table, not name resolution                         | Local same-named bindings (type-level `Add` family, etc.) don't interfere with operators; RFC-011 §5.2 examples don't need modification                       | 2026-09-22 |
| Orphan rule                                | Implementation follows the type's definition module                                                           | All types treated equally, built-in types have no special privilege                                                                                           | 2026-09-22 |
| `Any`'s `==`                               | Maintain runtime comparison, don't query registration table                                                   | RFC-036 `assert_eq` assertion family depends on this behavior                                                                                                 | 2026-09-22 |
| `Try` shape                                | Finalized: four methods (is_failure/success/residual/from_error) + assert-Never dead branch semantics; landed | `?` needs three things, single method `residual` is not enough; constructor parser special case retires along with Result moving to std                       | 2026-09-22 |
| `%` semantics characterization             | Defect fix (documentation long promised modulo), no compatibility period                                      | `reference/index.md` "multiplication/division modulo" as primary evidence                                                                                     | 2026-09-22 |
| Basis for `Result` belonging to std        | Cite RFC-013's existing positioning, no longer citing non-existent numbers                                    | RFC-013 already writes "std library `Result(T, Error)`"                                                                                                       | 2026-09-22 |

## Appendix C: Glossary

| Term                                        | Definition                                                                                                                                                                         |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layer 0                                     | Fixed mapping table of operator→method name, language-level constant, user-immutable                                                                                               |
| Layer 1                                     | Dispatch foundation that looks up by `Type.method` key and calls                                                                                                                   |
| Layer 2                                     | Interface contract layer, provides basis for generic constraints, also operator gating condition                                                                                   |
| Interface implementation registration table | Aggregates core default registrations and user instantiations' implementation master table; only criterion for operator query and constraint solving, unrelated to name resolution |
| Native fast path                            | Hardcoded operation path preserved for basic types, doesn't go through interface dispatch                                                                                          |
| Position binding                            | RFC-004's `f[0]` syntax, binds function parameter positions as methods, compile-time behavior                                                                                      |
| Auto-derive                                 | Field-by-field `==` automatically generated by the compiler for records with all comparable fields, can be overridden by explicit instantiation                                    |

## References

- [RFC-011: Generic Type System Design](./011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraint, associated types
- [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
  interface declaration/instantiation/overloading rules
- [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear token
- [RFC-004: Multi-Position Joint Binding for Curried Methods](./004-curry-multi-position-binding.md)
  — `f[0]` syntax
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type = record where all fields
  return their own type
- [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` belongs to std
  positioning, error code process
- [RFC-010b: Pattern Matching Completeness (Variant Deconstruction and Exhaustiveness)](../accepted/010b-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — associated type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — multi-parameter subscripts
