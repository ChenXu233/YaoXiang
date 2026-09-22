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
>   interface declaration / instantiation mechanisms
> - [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear tokens
> - [RFC-004: Multi-Position Joint Binding for Curried Methods](./004-curry-multi-position-binding.md)
>   — `f[0]` positional binding syntax
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum types = record types where
>   all fields return the type itself
> - [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` existing
>   positioning under std, E108x
> - [RFC-039: Pattern Matching Completion](../draft/039-pattern-matching-completeness.md) — variant
>   destructuring (dependency)

## Summary

This RFC completes the **operator overloading** capability for YaoXiang, allowing `a + b` / `a == b`
/ `a[i]` / `e?` to be implemented by user-defined types, and turns the **operator constraints**
(`T: Add + Multiply`) in the constraint syntax already written by RFC-011 from a **paper
capability** into a workable mechanism (`Zero` in the same clause is not an operator, see Open
Questions).

The design adopts a **three-layer separation of responsibilities**: a fixed operator-to-method
mapping table (Layer 0), a name-based dispatch base (Layer 1, reusing the existing
`method_bindings`), and an interface contract layer (Layer 2, for generic constraints). The
**precedence and associativity of operators remain language-fixed**; users only overload the
semantics.

The first batch covers seven interfaces: `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal`
`Index`. The arithmetic interfaces use **three type parameters** `(Self, R, O)`—the result type `O`
is made explicit, so that heterogeneous-type operations like `1 + 2.5` (returns `Float`) and
`point * 2.0` (scaling) can be expressed. `Equal` is **automatically derived by default** (records
whose all fields are comparable automatically get field-wise `==`); explicit instantiation can
override this.

`Try` (the interface-ification of `?`) is entirely moved to Phase 2: it depends on RFC-010
(construction) and RFC-039 (destructuring) being landed, and its interface shape is not yet
finalized (see Open Questions).

**No new syntax, no new keywords**. All capabilities reuse the existing mechanisms of RFC-011a
(interface declaration / instantiation / external method declaration / overloading).

## Motivation

### Why This Feature Is Needed

#### 1. RFC-011's core examples depend on it, and the current state is worse than "cannot be delivered"

RFC-011 (accepted) uses operator names as type constraints in 8 places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

No RFC in the entire series has ever defined where `Add` / `Multiply` come from, or how `+` binds to
them. Empirical testing confirms the current state is worse: `T: Add` **directly produces a compile
error today**—the constraint solver queries the old trait table (`trait_data.rs`), and `Add` is not
in the table. The constraint names `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` are similarly
dangling (see "Coordination with Other RFCs" for how this RFC handles them).

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

A knock-on consequence: `list.contains(list_of_structs, p)` is **completely unusable**—it internally
depends on `==`. Meanwhile, `(1, 2) == (1, 2)` and `[1] == [1]` have always been available via
runtime element-wise comparison—**structs are the only gap**.

#### 3. `?` hardcodes type names into the compiler, blocking `Result` from being assigned to std

The implementation of `?` simultaneously hardcodes the type name and the variant number:

```rust
// typecheck: hardcoded construction of Result type
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hardcoded group name and variant number
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` is made **interface-driven**, any type (including
user-defined ones) that implements this interface can be used with `?`, and `Result` can belong to
std (RFC-013 already has the positioning of "std library `Result(T, Error)`").

Note that this is only half the problem: the construction syntaxes `ok(...)` / `err(...)` /
`some(...)` are also currently welded into the parser (the language specification §1.4.2 lists them
as "constructors recognized by the parser"). "Result belongs to std" requires both halves of `?` and
the constructors to be solved together, all of which are part of Phase 2 of this RFC.

#### 4. User-defined containers cannot be indexed

The type check for `Index` is a hardcoded whitelist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// others → "container not recognized at the type layer, refuse silently"
```

Any container type defined by the user will unconditionally error on `c[0]`.

#### 5. The implementation of `%` violates published documentation

Empirically, `-7 % 3` returns `-1` (truncated remainder). However, the operator precedence table in
the language reference (`reference/index.md`) already states "`* / %` multiplication, division,
**modulo**"—the documentation promises modulo, but the implementation gives remainder. This is not a
design change; it is a **defect where the implementation violates the documentation**, which this
RFC opportunistically fixes.

### Existing Half-Finished Foundations

Investigation finds that most of the mechanism **already exists**; what is missing is the wiring:

| Mechanism                                                           | Location                                                                                                               | Status                                                                                                        |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Interface declaration `(Self: Type) -> Type`                        | RFC-011a Phase 1                                                                                                       | ✅ Runnable                                                                                                   |
| Interface instantiation `Animal(Dog)` + external method declaration | RFC-011a Phase 2–3                                                                                                     | ✅ Runnable (with unit tests)                                                                                 |
| Method dispatch `method_bindings["Type.method"]`                    | `expressions.rs` (registered in `environment.rs`, queried in `expressions.rs` call resolution and field fallback path) | ✅ Runnable                                                                                                   |
| Associated types (= interface type parameters)                      | RFC-011 §3.1; RFC-011a has adopted this approach                                                                       | ✅ Mechanism decided                                                                                          |
| Type family evaluation `AssociatedTypeDef`                          | `dependent_types.rs`                                                                                                   | ✅ Production use case `std.assert`'s `IsTrue`                                                                |
| `Equal` / `Dup` / `Clone` / `Debug` traits                          | Registered in `trait_data.rs`                                                                                          | ⚠️ `Equal` has zero consumption points; old auto-derivation only registers signatures without implementations |
| Operator → method name mapping                                      | —                                                                                                                      | ❌ Does not exist                                                                                             |

**Conclusion**: This is not building a feature from scratch, but wiring existing parts together and
writing them up.

## Proposal

### Core Design: Three-Layer Separation of Responsibilities

```
Layer 0  Fixed mapping table (language-level constant, unchangeable by users)
         + → add    == → equal    [] → index    ? → residual
         │  Built in at compile-time, does not participate in type inference, not exposed to users
         ▼
Layer 1  Dispatch base (look up methods by name and call them)        ← Reuse existing mechanism
         method_bindings["Point.add"]
         ▼
Layer 2  Interface contract (for generic constraints)
         Add / Equal / Index / Try …
         Makes T: Add constraint valid, and serves as the pass condition for operators
```

**Rationale for the layering**:

- **Layer 0 and 1 make operators "usable"**, Layer 2 makes operators "constrainable". Merging them
  would cause any method named `add` to be called by `+`, disconnecting RFC-011's `T: Add` from
  operators.
- **Layer 1 is not newly created**: `method_bindings` already looks up by `Type.method` keys (the
  fallback path after field lookup fails), and operators can take the same path.
- **Layer 2 is the pass condition**: before an operator is allowed, it **must** be confirmed that
  the type implements the corresponding interface (see the auto-derivation exception in
  §Equal—derivation and registration use the same criterion).

### Names and Registration: Two Independent Channels

**Operator queries go to the "interface implementation registry", not through normal name
resolution.**

- The registry belongs to the core, aggregating two sources of registration: the default
  registrations done by the core for primitive types (`Add(Int, Int, Int)`, etc.), and the interface
  instantiations written by users in type bodies (`Add(Point, Point, Point)`). Regardless of which
  module the implementation code physically lives in, registrations are aggregated into the same
  table, and `+` / `==` / `[]` only look at this table.
- A **same-named binding** defined in a local module (e.g., the Peano type-level addition
  `Add: (A: Type, B: Type) -> Type = match ...` from RFC-011 §5.2) goes through the **name lookup
  channel**, only affecting the resolution of the name `Add` within that module, **never touching
  the registry, never affecting operator availability**. Same name, different things, no mutual
  shadowing.

This also answers "whether the type-level `Add` from RFC-011 conflicts with the interface `Add` from
this RFC": they do not conflict. Type-level `Add` is a purely type-level computation (Zero/Succ are
types, not values, and there will never be a value-level operation like `Zero + Succ(...)`), and is
on a different level from the value-level operator interface despite sharing the same name.

### Layer 0: Fixed Mapping Table

| Operator          | Interface                                                 | Method                 | First Batch |
| ----------------- | --------------------------------------------------------- | ---------------------- | ----------- |
| `+`               | `Add`                                                     | `add`                  | ✅          |
| `-`               | `Subtract`                                                | `subtract`             | ✅          |
| `*`               | `Multiply`                                                | `multiply`             | ✅          |
| `/`               | `Divide`                                                  | `divide`               | ✅          |
| `%`               | `Modulo`                                                  | `modulo`               | ✅          |
| `==` `!=`         | `Equal`                                                   | `equal`                | ✅          |
| `[]`              | `Index`                                                   | `index`                | ✅          |
| `?`               | `Try` (shape undecided, see Open Questions)               | `residual` (tentative) | ❌ Phase 2  |
| `<` `<=` `>` `>=` | — (native instructions reserved)                          | —                      | ❌          |
| `and` `or`        | — (short-circuit is language semantics, non-overloadable) | —                      | ❌          |
| 5 bitwise ops     | —                                                         | —                      | ❌          |
| Unary `-` `!`     | —                                                         | —                      | ❌          |

**Interface names use full spellings rather than abbreviations** (`Multiply` instead of `Mul`): to
keep consistency with `T: Add + Multiply + Zero` in the main text of RFC-011, **without modifying
the already accepted RFC**.

**`%` adopts `Modulo` semantics** (mathematical modulo, the sign of the result follows the divisor).
The motivation section has already confirmed that the current remainder implementation violates the
published documentation (`reference/index.md` "multiplication, division, modulo"), and this is
handled as a defect fix with no compatibility period.

Current state note: The type check whitelist for `%` currently only includes Int/Float (different
from the Int/Float/String/List whitelist for `+`), with empirical records in Appendix A.2.

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

The three type parameters each have their own role:

- `Self`: the left operand type (receiver, borrowed as `&Self`, see RFC-009 borrow tokens and
  RFC-011a receiver conventions);
- `R`: the right operand type—**left and right operands may be of different types**;
- `O`: **the result type**, explicitly declared.

**Why the result type must be an explicit parameter** (rather than hardcoding `-> Self`):

1. If `Self` is hardcoded, the method for `Add(Int, Float)` would be forced to return `Int`, making
   it impossible to correctly express `1 + 2.5`;
2. Once the result type is made explicit, the promotion type family in RFC-011 §8.3
   `Add: (A, B) -> Type = match (A, B) { (Int, Float) => Float, ... }` and this RFC's interface
   registration **become two views of the same table**—the core registration
   `Add(Int, Float, Float)` is precisely the row `(Int, Float) => Float`, and each user
   instantiation adds rows to this table;
3. The `Index` interface already takes three parameters `(Self, Key, Value)`, with the return type
   `Value` as a parameter—after arithmetic interfaces are supplemented with `O`, the entire operator
   interface family has a uniform shape, and the odd one out would be the one that hardcodes `Self`.

**Core default registrations** (native instruction path, not going through method calls):
`Add(Int, Int, Int)`, `Add(Int, Float, Float)`, `Add(Float, Int, Float)`,
`Add(Float, Float, Float)`, `Add(String, String, String)` (concatenation),
`Add(List(T), List(T), List(T))` (element concatenation), etc. The five arithmetic interfaces work
similarly for primitive types. `1 + 2.5` changes from the current compile error (the whitelist
requires both sides to be of the same type) to a legal operation returning `3.5: Float`.

**Constraint syntactic sugar**: `T: Add` ≜ `Add(T, T, T)` is already registered—same-type
self-composition, result still of that type. In RFC-011's matrix multiplication example, `a * b + c`
maintains type `T` throughout, which is exactly this meaning. `T: Equal` similarly ≜ `Equal(T, T)`.

**Heterogeneous-type example**—vector scaling:

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

1. **Default derivation**: When a record type is defined, if all fields are comparable (primitive
   types, `String`, or records/tuples/lists that are themselves comparable, and contain no `&mut`
   fields), the compiler automatically generates a **field-wise comparison** `==`, going through a
   native code path, without generating a user-visible method. User-defined types are naturally
   comparable without any ceremony.
2. **Explicit override**: If the type body contains `Equal(Point, Point)` and provides a
   `Point.equal` method, the user's version is used (e.g., float comparison with tolerance), and
   auto-derivation no longer applies.
3. **When fields are not comparable**: Auto-derivation fails, `==` is unavailable, and the
   diagnostic specifies which field is the culprit; the user can still manually write `Equal` to
   define a custom comparison (e.g., comparing function-typed fields by name).
4. **Precondition**: Types containing `&mut T` linear tokens (recursive through fields) do not
   participate in comparison—a linear token is consumed upon a single read and cannot simultaneously
   produce two values for comparison. Note that the precondition is "non-linear" rather than "Dup":
   primitive value types (Int/Float/Bool/Char) are not subject to Dup per RFC-011 §2.4 (they are
   compiler-builtin value copying), so if Dup were the precondition, `Point { x: Float, y: Float }`
   would be incorrectly rejected.
5. **Unified criterion for constraints**: The solving of `T: Equal` and the passing of `==` follow
   the same rule of "query the registry or structural derivation"—constraints and operators always
   give the same answer.

**Consistency basis**: `(1, 2) == (1, 2)` and `[1] == [1]` already use runtime element-wise
comparison today (the comparison whitelist in `executor.rs` includes Tuple/List/Array), and record
types are the only composite type excluded. Auto-derivation is not a new silent default, but rather
the completion of the last missing piece of the language's existing internal behavior.

**Old mechanism retirement**: The old auto-derivation of `Equal` in `trait_data.rs` (which only
registers signatures, has no implementation code, and has zero consumption points) is disabled; all
`Equal` determinations (default registration of primitive types, structural derivation, explicit
instantiation) go through the interface registry. The old trait table retains the existing
responsibilities of Clone/Dup/Debug (their names do not overlap with operator interfaces, and
unification will be discussed separately in the future).

#### Index Interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` as a type parameter rather than an associated type member**: The open question in RFC-011a
has already settled on "associated types are implemented through generic interface parameters" (the
`Iterator: (Item: Type) -> Type` is an isomorphic precedent), and there is no need to introduce a
`type` member syntax.

**Ownership note**: `index` returns a complete `Value` from an `&Self` borrow. The standard
library's `list.get: (&Vec(A), Int) -> A` already has this shape, and this interface **receives the
same treatment as the current std state**; the precise semantics of "extracting a complete value
from a borrow" for move-semantic element types will be uniformly handled when RFC-009 is fully
enforced, and this RFC does not invent new rules for this.

**Multi-position indexing relies on tuple packing + overloading**, without introducing variadic
interfaces:

```yaoxiang
// One-dimensional container
List(T) instantiates Index(List(T), Int, T)                   → arr[0]

// Multi-dimensional container
Grid    instantiates Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └── Key is a tuple

// Two instantiations with different signatures → coexist
```

**Same-named interfaces on the same type allow multiple instantiations**, distinguished by the
signature of the method they inject, following the method-level overloading rules of RFC-011a for
coexistence. The overloading of RFC-011a is explicitly limited to the method level, and this RFC
adds a layer of instantiation-level rules on top: the legality of same-named interface instantiation
coexistence is determined by whether the expanded method signatures conflict (conflict yields E1097,
with the same source as field/method namespace rules).

#### Propagation Interface (Phase 2, Shape Undecided)

The interface-ification target for `?` is named `Try`, but the **complete shape is undecided** and
does not enter the first batch. The reasons are:

1. `?` actually does three things: determining success/failure (currently via hardcoded variant 0),
   extracting the success payload, and returning the entire value as-is from the current function on
   failure. Having only `residual: (self: &Self) -> E` covers only a corner of the third thing, and
   writing it now would likely require rework later;
2. Phase 2 inherently depends on RFC-010 (constructing `Result` values) and RFC-039 (variant
   destructuring) being landed;
3. The accompanying **constructor welding problem** (`ok` / `err` / `some` recognized by the parser,
   language specification §1.4.2) is the other half of the same problem as the type name welding of
   `?`; "Result belongs to std" must solve both halves together, both of which fall within the scope
   of Phase 2.

### Examples

#### User-Defined Types: Arithmetic and Equality Work Out of the Box

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
    println(a == b)             # false —— Equal is auto-derived, no instantiation needed
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

# Float comparison with tolerance, overriding field-wise auto-derivation
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

#### Generic Constraints Finally Deliverable

```yaoxiang
// The example from RFC-011 can now be landed
// T: Add ≜ Add(T, T, T) is registered; T: Multiply ≜ Multiply(T, T, T) is registered
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

(`Zero` / `One` in the RFC-011 signature example `T: Add + Multiply + Zero` are not within the scope
of this RFC—they are constant members rather than operators, and "interface members without a
receiver" have no precedent in RFC-011a, see Open Questions.)

### Syntax Changes

**No new syntax, no new keywords**. All capabilities are composed of existing mechanisms:

| Capability              | Reused Existing Mechanism                                              |
| ----------------------- | ---------------------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                                       |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                              |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)                    |
| Associated types        | RFC-011 §3.1 (interface type parameters)                               |
| Multi-position indexing | Existing tuple packing parsing + instantiation-level overloading rules |
| Operator precedence     | **Language-fixed**, not open to user customization                     |

## Detailed Design

### Type System Impact

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` (`Try`
in Phase 2).

**Interface implementation registry**: The central hub for operator decisions. Aggregating core
default registrations (for primitive types) and user instantiations (`ImplementationProof` produced
by `check_interface_instantiation`), the pass-through check for `+` / `==` / `[]` and `T: Add`
constraint solving (`check_trait_bounds` in `bounds.rs`) **query the same table**—there is no gap
where "the constraint says yes, but the operator says no".

**Structural derivation of `Equal`**: A structural rule is layered on top of the registry (all
fields comparable ⇒ comparable), with primitive types backed by core registration and recursive
closure. The old `Equal` registration and auto-derivation in `trait_data.rs` are disabled.

**Separation of operator queries from name resolution**: Operators like `+` only query the registry,
not normal name resolution; local same-named bindings (such as the type-level `Add` family) do not
affect operators (see §Names and Registration).

**Orphan rule**: Operator implementations can only be written in the **module where the type is
defined**—`Int` is defined in core, so its registration can only be done by core; `Point` is defined
in the user module, so only its definer can register it. All types follow the same rule; built-in
types have no special privileges.

### Runtime Behavior

**Zero runtime overhead**: The call target for operators is determined at compile time (static
dispatch). For primitive types (`Int`/`Float`/`String`/`List`), the **native instruction fast path**
is preserved, not going through interface dispatch; auto-derived struct `==` generates native
field-wise comparison; `==` / `!=` at `Any` (dynamic type) positions maintains the existing runtime
comparison (RFC-036's testing framework `assert_eq` depends on this behavior, and is not affected).

**`%` semantic correction**: `-7 % 3` changes from `-1` (truncated remainder) to `2` (mathematical
modulo). Three places are modified synchronously: the interpreter (`checked_rem`), constant folding
(`a % b`), and bytecode (`I64_REM`).

### Compiler Changes

| Component                            | Change                                                                                                                                                                                                                                            |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | The `infer_binary` whitelist is changed to "native fast path + registry query" dual paths; the `Expr::Index` whitelist adds an interface query branch                                                                                             |
| `typecheck/inference/bounds.rs`      | `check_trait_bounds` changes to query the interface registry for operator interface names (including structural derivation for `Equal`)                                                                                                           |
| `typecheck/checker.rs`               | Add `Equal` structural derivation (after record definition); instantiation-level overloading rules (same-named interface multiple instantiations coexist by method signatures)                                                                    |
| `middle/core/ir_gen.rs`              | `Expr::Index` adds a method call dispatch branch; structs without an explicit `equal` generate native field-wise comparison; `Mod` instruction changes to mathematical modulo                                                                     |
| Interface registry (new)             | Layer 0 mapping table constants + operator-to-interface query function + core default registrations (`Int`/`Float`/`String`/`List`, etc.)                                                                                                         |
| `trait_data.rs`                      | `Equal` registration and old auto-derivation disabled (Clone/Dup/Debug maintain current state)                                                                                                                                                    |
| Diagnostics                          | `Equal` precondition (linear tokens) non-satisfaction reuses the `E1101` (type does not implement interface) family; `E1081`/`E1082` text removes "Result" wording (in Phase 2 when `Try` lands, three-party synchronization per RFC-013 process) |

### Backward Compatibility

**Primitive type operations remain unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, `1 < 2` all
retain native paths, with behavior and performance unchanged.

**`1 + 2.5` changes from a compile error to legal**: After core registration of
`Add(Int, Float, Float)`, the mixed arithmetic previously rejected by the whitelist becomes
available, returning `Float`. This is a new capability, with no impact on existing code.

**`%` semantic correction**: `-7 % 3` changes from `-1` to `2`. Characterized as a **defect fix**
(the documentation has long promised modulo, see Motivation §5), with no compatibility period; no
cases in the existing test corpus depend on negative `%` (verified).

**Struct `==` changes from a runtime error to usable**: Previously, `Struct == Struct` uniformly
produced the E6007 runtime error; after wiring, auto-derivation makes it directly usable—going from
broken to working, with no impact on existing legal code.

**`Any`'s `==` is unaffected**: Dynamic type positions maintain runtime comparison (RFC-036's
`assert_eq` assertion family depends on this; previously available, still available afterward).

**`f[0]` positional binding unchanged**: `distance[0]` is a compiler-builtin capability from
RFC-004, **not going through the `Index` interface**, and is unaffected.

**`?` is transparent to existing code** (Phase 2): After `Result` gains a `Try` instantiation,
existing `?` usages behave exactly the same.

## Trade-offs

### Advantages

- **Delivers RFC-011's operator constraints**: `T: Add + Multiply` goes from being on paper
  (effectively a compile error) to being landable, without modifying the main text of the accepted
  RFC
- **Removes the core binding of `Result`** (Phase 2): After `?` and constructors are
  interface-ified, `Result` can belong to std, aligning with the existing positioning of RFC-013
- **Fixes empirical defects**: `Point == Point` works out of the box, with a knock-on fix for
  `list.contains` being unusable on structs
- **Zero new syntax**: All capabilities reuse RFC-011a's existing mechanisms, without touching
  parser grammar rules
- **Zero runtime overhead**: Static dispatch + native fast path for primitive types
- **User-defined containers are usable**: `Box(T)[0]` changes from "uniformly errors" to usable
- **Internal language consistency**: Tuple/List are already element-wise compared, record types are
  filled in; the three-parameter shape of arithmetic interfaces is uniform with `Index`; RFC-011
  §8.3's promotion table and the interface registry are merged into one

### Disadvantages

- **`Equal` auto-derivation is a silent default**: In the future, if we want to retract "records are
  comparable by default", it will be a breaking change. This stance is accepted—it aligns with the
  existing behavior of Tuple/List, and explicit instantiation can always override it
- **`Equal`'s precondition rejects types containing linear tokens**: Types containing `&mut` fields
  cannot use `==`, which is the cost of semantic correctness and requires clear diagnostics
  (specifying which field)
- **Interface instantiation is an explicit cost**: Each arithmetic operator requires one line of
  instantiation + one method (`Equal` is exempt—auto-derived). The syntax of RFC-011a makes implicit
  derivation impossible (the trade-off being no magic in the `Self` type parameter)
- **`%` semantic correction is a behavior change**: Although characterized as a defect fix, it still
  needs to be noted in the migration guide

## Alternative Approaches

### Approach A: Only Do Layer 1 (Dispatch by Method Name), No Interface Layer

`+` only checks whether there is a method named `add`, without requiring the implementation of the
`Add` interface.

**Reason for rejection**: RFC-011's `T: Add` constraint will be disconnected from operators—the
constraint queries interfaces, operators query method names, and the two may give inconsistent
answers. Moreover, it is impossible to provide accurate diagnostics at compile time for "using `+`
on a non-addable type".

### Approach B: Introduce Constructor Syntax `Ok(x)` / `Some(x)` to Solve the `?` Problem

Do not interface-ify `?`, but add constructor syntax to sum types.

**Reason for rejection**: Conflicts with RFC-010 (accepted). RFC-010 explicitly states "uniformly
use record types to express sum types, **no need for two syntaxes**", and explicitly deprecates the
`|` syntax. Introducing constructors is introducing a second set of expressions. And it only solves
`?`, not `Point == Point` or custom container indexing.

### Approach C: Comparison Operators Also Interface-ified in the First Batch (Introduce `Ordering`)

`<` `<=` `>` `>=` go through the `Compare` interface, returning the three-value `Ordering`.

**Reason for rejection**: `<` is already a **first-class IR instruction** in YaoXiang
(`Instruction::Lt/Le/Gt/Ge`); interface-ification would force primitive types to take a detour.
Moreover, introducing `Ordering` would bring up a whole set of issues such as `Ordering`'s own
comparison/sorting, `PartialOrd` vs `Ord` for floats with `NaN`, and is of the magnitude of an
independent RFC. The current empirically exposed needs (`Point == Point`, `list.contains`) **only
require `Equal`**.

### Approach D: Operator Names Use Abbreviations (Interface `Add` Has Method `add`, But Interface Is Called `Mul`)

**Reason for rejection**: The main text of RFC-011 already writes `T: Add + Multiply + Zero`; using
abbreviations would require modifying the already accepted RFC.

### Approach E: `Equal` Only Supports Explicit Instantiation, No Auto-Derivation

**Reason for rejection**: User-defined types unexpectedly cannot use `==`, which is an unacceptable
experience; moreover, `(1, 2) == (1, 2)` and `[1] == [1]` are already element-wise compared today,
and only record types are excluded, which was already inconsistent. Explicit instantiation is
retained as an override means, balancing custom needs.

### Approach F: Arithmetic Interface Return Type Hardcoded as `-> Self`

**Reason for rejection**: The method for `Add(Int, Float)` would be forced to return `Int`, making
it impossible to correctly express `1 + 2.5`; vector scaling `Multiply(Point, Float)` cannot be
written either. Moreover, the promotion type family `(Int, Float) => Float` in RFC-011 §8.3 would
lose its landing place. The three-parameter `(Self, R, O)` is uniform in shape with
`Index(Key, Value)`.

## Implementation Strategy

### Dependencies

| Dependency                             | Status        | Impact on This RFC                                                                |
| -------------------------------------- | ------------- | --------------------------------------------------------------------------------- |
| RFC-011a interface mechanism Phase 1–3 | ✅ Landed     | Foundation of Layer 2 (empirically runnable)                                      |
| RFC-010 record-based construction path | ❌ Not landed | **Phase 2**: `?`'s construction side + constructor parser special-case retirement |
| RFC-039 variant destructuring          | ❌ On paper   | **Phase 2**: `match` syntax for `Result.residual`, exhaustiveness                 |
| RFC-009 linear token inference         | Partial       | `Equal`'s precondition check                                                      |

### Phasing

Divided into two groups by **interface dependencies** (design constraints, not scheduling):

**Phase 1 — Does not depend on RFC-010/039**:

- Layer 0 mapping table + interface implementation registry + Layer 1 dispatch wiring
- The seven interfaces `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
  (three-type-parameter shape)
- `Equal` auto-derivation + linear precondition + constraint solving rerouted to the registry
- `%` semantic correction (including interpreter/constant folding/bytecode)
- `Any` position `==` maintains runtime comparison
- **Benefits**: `Point + Point`, `Point == Point` (no ceremony), `Box(T)[0]`, `1 + 2.5`, `T: Add`
  constraints all become available

**Phase 2 — Depends on RFC-010 / RFC-039; the `Try` shape must be finalized before work begins**:

- Finalize the `Try` interface shape (see Open Questions)
- `?` interface-ification + constructors (`ok`/`err`/`some`) parser special-case retirement,
  switching to the RFC-010 record construction path
- `Result` migrated from core to std (per RFC-013's existing positioning)

### Risks

| Risk                                                                         | Mitigation                                                                                                         |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `%` semantic change affects existing user code                               | Characterized as a defect fix + verified that the test corpus has no negative `%` dependencies                     |
| Silent default of `Equal` auto-derivation hard to retract in the future      | Position settled: aligns with existing Tuple/List behavior, explicit instantiation can override                    |
| Inconsistency between dual paths for primitive types (native + registration) | Gate: the semantics of core registration must be consistent with native instructions (two views of the same table) |
| Registry queries slow down compilation                                       | Table indexed by type name + instantiation result caching (reusing RFC-011a proof)                                 |
| Instantiation-level overloading introduces ambiguity                         | Same rules as method-level overloading: signature conflict yields E1097                                            |

## Coordination with Other RFCs

This RFC is positioned as a **consumer-side requirements proposer** and needs to synchronously
update the following RFCs to ensure coordination consistency (authorized to revise):

| RFC                    | Content to Update                                                                                                                                                                                                                                                                                          |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **RFC-011** (accepted) | §Constraints section notes that `Add` / `Multiply` are defined and landed by 011b (`T: Add` ≜ `Add(T, T, T)`); mark `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` as dangling constraint names (awaiting subsequent RFCs); §8.3 promotion type family notes that it is merged with the interface registry |
| **RFC-010** (accepted) | Clarify the landing requirement of "record fields are constructors"—it is the prerequisite for the constructor parser special-case retirement in Phase 2                                                                                                                                                   |
| **RFC-039** (draft)    | Construction and destructuring must come in pairs; exhaustiveness determination does not depend on `Result`'s core/std ownership (Phase 2 of 011b will migrate it)                                                                                                                                         |
| **RFC-009** (accepted) | §Type properties cross-reference: `Equal`'s precondition is "does not contain `&mut` linear tokens" (not Dup)                                                                                                                                                                                              |
| **RFC-013** (accepted) | When Phase 2 lands: `E1081` / `E1082` text removes "Result" wording; `Equal` precondition diagnostics reuse the `E1101` family—three-party synchronization per RFC-013 process (codes/*.rs ↔ locales ↔ code table)                                                                                         |
| **RFC-018** (accepted) | After `%` is changed to mathematical modulo, the `Mod → srem/urem` mapping table becomes invalid and needs to be changed to `srem` + sign correction (or `sdiv`+`mul`+`sub` synthesis), and the term mix-up of "modulo/remainder" should be corrected                                                      |
| **RFC-036** (accepted) | No changes needed. Note the relationship: `Any` position `==` maintains runtime comparison, and the `assert_eq` assertion family is not gated                                                                                                                                                              |

> Basis for `Result` belonging to std: RFC-013 has already positioned "std library
> `Result(T, Error)`", and RFC-014's layering assigns std to the core source. Phase 2 of this RFC is
> one of its landing paths, and no longer references other numbers.

## Open Questions

- [x] ~~Does the semantic correction of `Modulo` require a compatibility period?~~ → **Closed**: The
      language reference has long stated "multiplication, division, modulo", and the current
      remainder implementation violates the published documentation, so it is handled as a defect
      fix with no compatibility period (2026-09-22)
- [x] ~~Can users supplement operator implementations for existing types (orphan rule)?~~ →
      **Decision**: Operator implementations can only be written in the module where the type is
      defined. `Int` is defined in core, so only core can register it; users can only register for
      their own types. All types are treated equally, and built-in types have no special privileges
      (2026-09-22)
- [ ] Does `Index` need to distinguish mutable indexing (like Rust's `IndexMut`)? (First batch is
      read-only; mutable indexing involves RFC-009's `WriteToken`, and RFC-011a already has the
      `&mut Self` receiver precedent, so leave for later)
- [ ] Should multi-position indexing's Key use tuple packing (current state) or be changed to
      multiple parameters (Swift style)? (Continue using tuple packing, do not change the parser)
- [ ] The shape of `Zero` / `One`: Constant members are not operators, "interface members without a
      receiver" have no precedent in RFC-011a, and require a separate decision before the entire
      sentence `T: Add + Multiply + Zero` from RFC-011 can be delivered
- [ ] The complete shape of the `Try` interface: `?` needs three things—success/failure
      determination, success payload extraction, and producing an outer return value on the failure
      path—and the single-method `residual` is insufficient; together with the constructor parser
      special-case retirement, finalize before Phase 2 begins
- [ ] The `?T` prefix type (RFC-026 FFI nullable annotation, RFC-018) and the `e?` suffix operator
      share the `?` symbol: different positions (type position vs expression position) do not
      constitute a conflict, and a written explanation is sufficient
- [ ] `PartialOrd` / `Ordering` (comparison operator interface-ification): Independent RFC, this RFC
      explicitly does not do it

---

## Appendix A: Research Evidence

All the following empirical tests are reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Interface Mechanism Availability (Foundation of This RFC)

| Capability                                                                     | Empirical                                                          |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------ |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Definable                                                       |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runnable, output correct (unit tests in `tests/rfc011a.rs`)     |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (conflict between fields and methods sharing namespace) |
| Method dispatch `d.speak()`                                                    | ✅ Runnable                                                        |

**Note**: `E1097` means that operator methods must use the **external declaration** form
(`Point.add: (self: &Point, ...)`), consistent with the RFC-011a examples. The registration of
`method_bindings` is in `environment.rs` (`add_method_binding`), and the query is in
`expressions.rs`'s call target resolution and field lookup failure fallback path.

### A.2 Current State of Operators

| Expression                            | Current State                                                                                  |
| ------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]`   | ✅ Hardcoded whitelist (Int/Float/String/List, **requires both sides to be of the same type**) |
| `1 + 2.5` (Int + Float)               | ❌ Compile error (whitelist requires both sides to be of the same type)                        |
| `-7 % 3`                              | `-1` (truncated remainder; `%` whitelist only Int/Float, different from `+`'s whitelist)       |
| `Point(1,2) == Point(1,2)`            | ❌ `E6007` (Eq fails at runtime on Struct)                                                     |
| `(1,2) == (1,2)` / `[1] == [1]`       | ✅ Runtime element-wise comparison (Struct is the only gap)                                    |
| `Any` position `a == b` (`assert_eq`) | ✅ Runtime comparison (RFC-036 verified)                                                       |
| `f[0]` (function positional binding)  | ⚠️ Only legal within binding declarations, as an expression produces `E3006`                   |
| `arr[0, 1]` (multi-position)          | ✅ Tuple packing, `list([1, 2])`                                                               |

### A.3 Hardcoded Locations of `?` and Constructors

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Constructor side: `ok(T)` / `err(E)` / `some(T)` are recognized by the parser (language
specification `syntax.md` §1.4.2); `std/result.rs`'s `is_ok` / `unwrap` etc. pattern-match according
to `variant_id 0/1`. "Result belongs to std" requires both halves (`?` + constructors) to be solved
together, both falling within Phase 2.

## Appendix B: Design Decision Records

| Decision                                   | Decision                                                                                             | Reason                                                                                                                                             | Date       |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Architecture                               | Three-layer separation (mapping table / dispatch base / interface contract)                          | Merging them would disconnect RFC-011 constraints from operators                                                                                   | 2026-09-15 |
| Operator pass condition                    | Must implement the interface (Layer 2 is the pass condition)                                         | Ensures strict correspondence between `T: Add` and `+`                                                                                             | 2026-09-15 |
| Interface naming                           | Full spellings (`Multiply` instead of `Mul`)                                                         | Do not modify the main text of the accepted RFC-011                                                                                                | 2026-09-15 |
| `%` interface name                         | `Modulo` (mathematical modulo)                                                                       | The name nails down the semantics                                                                                                                  | 2026-09-15 |
| Comparison operators                       | **Not** interface-ified in the first batch, native IR instructions reserved                          | Already first-class instructions; `Ordering` brings up a whole set of independent issues                                                           | 2026-09-15 |
| `Ordering`                                 | Not introduced in the first batch                                                                    | No real need driving it, of the magnitude of an independent RFC                                                                                    | 2026-09-15 |
| Associated types                           | Use interface type parameters, do not introduce `type` member syntax                                 | RFC-011a has already settled on this approach (`Iterator: (Item: Type)`)                                                                           | 2026-09-15 |
| Unification of method binding and indexing | Conceptually unified, interfaces only handle container indexing                                      | The key for positional binding is a compile-time constant, and the result type requires type family evaluation, which users cannot implement       | 2026-09-15 |
| Multi-position indexing                    | Tuple packing + overloading by Key type, no variadic interfaces                                      | Do not change the parser                                                                                                                           | 2026-09-15 |
| Bitwise / unary operators                  | Not done in the first batch                                                                          | Rare for user-defined types, YAGNI                                                                                                                 | 2026-09-15 |
| Arithmetic interface shape                 | Three type parameters `(Self, R, O)`; `T: Add` ≜ `Add(T, T, T)`                                      | Explicit result type: `1 + 2.5`, scaling can be expressed; RFC-011 §8.3 promotion table merged with the registry; uniform with `Index(Key, Value)` | 2026-09-22 |
| `Equal` derivation                         | Auto-derive by default + explicit instantiation override; constraint solving uses the same criterion | "User-defined types cannot ==" is unacceptable; Tuple/List are already element-wise compared, Struct is the only gap                               | 2026-09-22 |
| `Equal` precondition                       | "Does not contain `&mut` linear tokens", **not Dup**                                                 | RFC-011 §2.4 explicitly states that primitives are not subject to Dup; using Dup as the precondition would incorrectly reject `Point{Float,Float}` | 2026-09-22 |
| Separation of names and registration       | Operators only query the interface implementation registry, not name resolution                      | Local same-named bindings (type-level `Add` family, etc.) do not interfere with operators; RFC-011 §5.2 examples do not need changes               | 2026-09-22 |
| Orphan rule                                | Implementation follows the module where the type is defined                                          | All types are treated equally, built-in types have no special privileges                                                                           | 2026-09-22 |
| `Any`'s `==`                               | Maintain runtime comparison, do not query the registry                                               | RFC-036's `assert_eq` assertion family already depends on this behavior                                                                            | 2026-09-22 |
| `Try` shape                                | Suspended until Phase 2 work begins and finalized                                                    | `?` needs three things, single-method `residual` is insufficient; constructor parser special cases are included together                           | 2026-09-22 |
| `%` semantic characterization              | Defect fix (documentation has long promised modulo), no compatibility period                         | `reference/index.md` "multiplication, division, modulo" is the prior evidence                                                                      | 2026-09-22 |
| Basis for `Result` belonging to std        | Cite RFC-013's existing positioning, no longer reference non-existent numbers                        | RFC-013 already writes "std library `Result(T, Error)`"                                                                                            | 2026-09-22 |

## Appendix C: Glossary

| Term                              | Definition                                                                                                                                                                                  |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layer 0                           | Fixed mapping table from operators to method names, language-level constant, unchangeable by users                                                                                          |
| Layer 1                           | Dispatch base that looks up by `Type.method` keys and calls                                                                                                                                 |
| Layer 2                           | Interface contract layer, providing the basis for generic constraints, and serving as the pass condition for operators                                                                      |
| Interface implementation registry | Aggregating the implementation table for core default registrations and user instantiations; the sole criterion for operator queries and constraint solving, independent of name resolution |
| Native fast path                  | Hardcoded operation path preserved for primitive types, not going through interface dispatch                                                                                                |
| Positional binding                | The `f[0]` syntax from RFC-004, binding function parameter positions as methods, compile-time behavior                                                                                      |
| Auto-derivation                   | Field-wise `==` generated by the compiler for records whose all fields are comparable, overridable by explicit instantiation                                                                |

## References

- [RFC-011: Generic Type System Design](./011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraints, associated types
- [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
  interface declaration/instantiation/overloading rules
- [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear tokens
- [RFC-004: Multi-Position Joint Binding for Curried Methods](./004-curry-multi-position-binding.md)
  — `f[0]` syntax
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type expressions
- [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` belongs to std
  positioning, error code workflow
- [RFC-039: Pattern Matching Completion](../draft/039-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — associated type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — multi-parameter subscripts
