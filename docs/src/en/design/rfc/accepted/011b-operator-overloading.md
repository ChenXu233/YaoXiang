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
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md) — Type constraints
>   `T: Add + Multiply`, associated types
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
>   Interface declaration/instantiation mechanism
> - [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear token
> - [RFC-004: Multi-Position Combined Binding for Curried Methods](./004-curry-multi-position-binding.md)
>   — `f[0]` positional binding syntax
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — Sum type = record whose fields
>   all return their own type
> - [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` belongs in std
>   (established position), E108x
> - [RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)](../draft/010b-pattern-matching-completeness.md)
>   — Variant destructuring (dependency)

## Summary

This RFC adds **operator overloading** capabilities to YaoXiang, allowing `a + b` / `a == b` /
`a[i]` / `e?` to be implemented by user-defined types, and turns the **operator constraints**
(`T: Add + Multiply`) in the constraint syntax already written in RFC-011 from a **paper
capability** into a workable mechanism (`Zero` in the same line is not an operator — see Open
Questions).

The design adopts a **three-layer responsibility separation**: a fixed operator-to-method mapping
table (Layer 0), a name-based dispatch base (Layer 1, reusing the existing `method_bindings`), and
an interface contract layer (Layer 2, used for generic constraints). The **precedence and
associativity of operators remain language-fixed**; users only overload semantics.

Initial scope: seven interfaces — `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`.
Arithmetic interfaces use **three type parameters** `(Self, R, O)` — the result type `O` is
explicit, so heterogeneous operations like `1 + 2.5` (returns `Float`) and `point * 2.0` (scaling)
can be expressed. `Equal` is **automatically derived by default** (records whose fields are all
comparable automatically get field-by-field `==`), and explicit instantiation can override it.

`Try` (interface-ization of `?`) moves entirely to Phase 2: it depends on RFC-010 (construction) and
RFC-010b (destructuring) being landed, and its interface shape is not yet finalized (see Open
Questions).

**No new syntax, no new keywords** — fully reusing the existing mechanisms from RFC-011a (interface
declaration / instantiation / external method declaration / overloading).

## Motivation

### Why This Feature Is Needed

#### 1. RFC-011's Core Example Depends on It, and the Current State Is Worse Than "Cannot Deliver"

RFC-011 (already accepted) uses operator names as type constraints in 8 places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

None of the entire RFC series defines where `Add` / `Multiply` come from or how `+` binds to them.
Real testing confirms the state is worse: **`T: Add` directly errors at compile today** — constraint
resolution queries the old trait table (`trait_data.rs`), which contains no `Add`. The constraint
names `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` are similarly dangling (handling is in
"Coordination with Other RFCs").

#### 2. User-Defined Types Cannot Participate in Basic Operations (Tested)

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

Collateral consequence: **`list.contains(list_of_structs, p)` is completely unusable** — it
internally depends on `==`. Meanwhile, `(1, 2) == (1, 2)` and `[1] == [1]` running through runtime
element-wise comparison **have always worked** — struct is the only gap.

#### 3. `?` Welds Type Names into the Compiler, Blocking `Result`'s Move to std

The `?` implementation hardcodes both the type name and variant numbers:

```rust
// typecheck: hardcoded construction of Result type
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hardcoded group name and variant numbers
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` becomes **interface-driven**, any type that
implements the interface (including user-defined ones) can be used with `?`, and `Result` can belong
to std (RFC-013 already has the positioning "std library `Result(T, Error)`").

Note this is only half the problem: `ok(...)` / `err(...)` / `some(...)` construction syntax is also
currently welded in the parser (listed in language spec §1.4.2 as "constructors recognized by the
parser"). "Moving `Result` to std" requires **both halves** — `?` and the constructors — to be
resolved together; both fall under Phase 2 of this RFC.

#### 4. User-Defined Containers Cannot Be Indexed

`Index` type checking is a hardcoded whitelist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// Others → "containers not recognized at the type layer are rejected, not silently"
```

For any container type defined by the user, `c[0]` reports an error.

#### 5. The `%` Implementation Violates Published Documentation

Testing shows `-7 % 3` returns `-1` (truncated remainder). But the language reference's operator
precedence table (`reference/index.md`) already states "`* / %` multiplication, division,
**modulo**" — the document promises modulo, but the implementation gives remainder. This is not a
design change but **an implementation violation of documentation**, which this RFC fixes along the
way.

### Existing Semi-Finished Foundation

Investigation found that **most of the mechanism already exists** — only the wiring is missing:

| Mechanism                                                           | Location                                                                                                               | Status                                                                                            |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Interface declaration `(Self: Type) -> Type`                        | RFC-011a Phase 1                                                                                                       | ✅ Runnable                                                                                       |
| Interface instantiation `Animal(Dog)` + external method declaration | RFC-011a Phase 2–3                                                                                                     | ✅ Runnable (with unit tests)                                                                     |
| Method dispatch `method_bindings["Type.method"]`                    | `expressions.rs` (registered in `environment.rs`, queried in `expressions.rs` call resolution and field fallback path) | ✅ Runnable                                                                                       |
| Associated types (= interface type parameters)                      | RFC-011 §3.1; RFC-011a adopted this approach                                                                           | ✅ Mechanism finalized                                                                            |
| Type family evaluation `AssociatedTypeDef`                          | `dependent_types.rs`                                                                                                   | ✅ Production case `std.assert`'s `IsTrue`                                                        |
| `Equal` / `Dup` / `Clone` / `Debug` trait                           | Registered in `trait_data.rs`                                                                                          | ⚠️ `Equal` has zero consumers; old auto-derive only registered signatures without implementations |
| Operator → method name mapping                                      | —                                                                                                                      | ❌ Does not exist                                                                                 |

**Conclusion**: This is not building a feature from scratch — it is wiring existing parts together
and writing it down.

## Proposal

### Core Design: Three-Layer Responsibility Separation

```
Layer 0  Fixed mapping table (language-level constant, user-immutable)
         + → add    == → equal    [] → index    ? → residual
         │  Built-in at compile time, not involved in type inference, not exposed to user
         ▼
Layer 1  Dispatch base (lookup method by name and invoke)        ← Reuse existing mechanism
         method_bindings["Point.add"]
         ▼
Layer 2  Interface contract (for generic constraints)
         Add / Equal / Index / Try …
         Makes T: Add constraint valid, and serves as the operator gating condition
```

**Reasons for layering**:

- **Layers 0 and 1 make operators "usable"**, Layer 2 makes operators "constrainable". Merging them
  would mean any method named `add` gets called by `+`, decoupling RFC-011's `T: Add` from
  operators.
- **Layer 1 is not newly built**: `method_bindings` already looks up by `Type.method` key (the
  fallback path after field lookup fails); operators just take the same path.
- **Layer 2 is the gating condition**: Before an operator is allowed, it **must** verify that the
  type implements the corresponding interface (see the `Equal` auto-derive exception in §Equal —
  derivation and registration are the same criterion).

### Name and Registration: Two Independent Channels

**Operator queries go to the "interface implementation registry", bypassing ordinary name
resolution.**

- The registry belongs to core, aggregating two sources: core's default registrations for primitive
  types (`Add(Int, Int, Int)`, etc.) and user interface instantiations written in type bodies
  (`Add(Point, Point, Point)`). Regardless of which module the implementation code physically
  resides in, registrations feed into the same table; `+` / `==` / `[]` only look at this table.
- A local module defining a **same-named binding** called `Add` (for example, RFC-011 §5.2's Peano
  type-level addition `Add: (A: Type, B: Type) -> Type = match ...`) goes through the **name lookup
  channel**, only affecting the resolution of the name `Add` within that module, **cannot touch the
  registry, and does not affect operator usability**. Same name, different things, no mutual
  shadowing.

This simultaneously answers "Does RFC-011's type-level `Add` conflict with this RFC's interface
`Add`?": No conflict. Type-level `Add` is pure type-level computation (Zero/Succ are types not
values; there will never be a value-level `Zero + Succ(...)` operation), while the value-level
operator interface is a separate layer sharing the same name.

### Layer 0: Fixed Mapping Table

| Operator             | Interface                                                 | Method                 | Initial Scope |
| -------------------- | --------------------------------------------------------- | ---------------------- | ------------- |
| `+`                  | `Add`                                                     | `add`                  | ✅            |
| `-`                  | `Subtract`                                                | `subtract`             | ✅            |
| `*`                  | `Multiply`                                                | `multiply`             | ✅            |
| `/`                  | `Divide`                                                  | `divide`               | ✅            |
| `%`                  | `Modulo`                                                  | `modulo`               | ✅            |
| `==` `!=`            | `Equal`                                                   | `equal`                | ✅            |
| `[]`                 | `Index`                                                   | `index`                | ✅            |
| `?`                  | `Try` (shape TBD, see Open Questions)                     | `residual` (tentative) | ❌ Phase 2    |
| `<` `<=` `>` `>=`    | — (preserve native instructions)                          | —                      | ❌            |
| `and` `or`           | — (short-circuit is language semantics, not overloadable) | —                      | ❌            |
| 5 bitwise operations | —                                                         | —                      | ❌            |
| Unary `-` `!`        | —                                                         | —                      | ❌            |

**Interface names use full words rather than abbreviations** (`Multiply` instead of `Mul`):
consistent with RFC-011's body text `T: Add + Multiply + Zero`, **no changes to already-accepted
RFCs**.

**`%` adopts `Modulo` semantics** (mathematical modulo, result sign follows divisor). The motivation
section confirmed that the current remainder implementation violates published documentation
(`reference/index.md` "multiplication, division, modulo"); this is treated as a bug fix, with no
compatibility period.

Status note: The `%` type-check whitelist currently only includes Int/Float (different from the `+`
whitelist of Int/Float/String/List); Appendix A.2 has test records.

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

- `Self`: left operand type (receiver, `&Self` borrow, see RFC-009 borrow token and RFC-011a
  receiver convention);
- `R`: right operand type — **left and right may be heterogeneous**;
- `O`: **result type**, explicitly declared.

**Why the result type must be an explicit parameter** (rather than hard-coded `-> Self`):

1. Hard-coding `Self` means `Add(Int, Float)`'s method must return `Int`, and `1 + 2.5` cannot be
   correctly expressed;
2. Once the result type is explicit, RFC-011 §8.3's promotion type family
   `Add: (A, B) -> Type = match (A, B) { (Int, Float) => Float, ... }` and this RFC's interface
   registration **become two views of the same table** — core registration `Add(Int, Float, Float)`
   is precisely the `(Int, Float) => Float` row, and every user instantiation adds a row to this
   table;
3. The `Index` interface is already three-parameter `(Self, Key, Value)`, with the return type
   `Value` as a parameter — once arithmetic interfaces add `O`, the entire operator interface family
   has a uniform shape, and the hard-coded `Self` variant is the odd one out.

**Core default registrations** (native instruction path, not via method calls):
`Add(Int, Int, Int)`, `Add(Int, Float, Float)`, `Add(Float, Int, Float)`,
`Add(Float, Float, Float)`, `Add(String, String, String)` (concatenation),
`Add(List(T), List(T), List(T))` (element concatenation), etc.; the five arithmetic interfaces work
the same for primitive types. `1 + 2.5` changes from a current compile error (whitelist requires
both sides of the same type) to a valid operation returning `3.5: Float`.

**Constraint syntax sugar**: `T: Add` ≜ already-registered `Add(T, T, T)` — same-type
self-composition, result remains that type. In the RFC-011 matrix multiplication example,
`a * b + c` has type `T` throughout, which is exactly this meaning. `T: Equal` similarly ≜
`Equal(T, T)`.

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

**`Equal` is auto-derived by default**, five rules:

1. **Default derivation**: When defining a record type, if all fields are comparable (primitive
   types, `String`, or records/tuples/lists that are themselves comparable, and no `&mut` fields),
   the compiler automatically generates **field-by-field comparison** `==`, going through native
   code path without generating user-visible methods. User-written types are naturally comparable,
   with no ceremony required.
2. **Explicit override**: If the type body writes `Equal(Point, Point)` and provides a `Point.equal`
   method, the user's version is used (e.g., with tolerance for float comparison), and
   auto-derivation is no longer applied.
3. **When fields are not comparable**: Auto-derivation fails, `==` is unavailable, and the diagnosis
   indicates which field is the problem; the user can still write `Equal` manually for custom
   comparison (e.g., comparing function fields by name).
4. **Precondition**: Types containing `&mut T` linear tokens (recursive fieldwise) do not
   participate in comparison — a linear token is consumed once read, and two values cannot be taken
   out simultaneously for comparison. Note the premise is "non-linear" rather than "Dup": primitive
   value types (Int/Float/Bool/Char) are not Dup per RFC-011 §2.4 (they are compiler built-in value
   copies); if Dup is the premise, then `Point { x: Float, y: Float }` would be wrongly rejected.
5. **Same criterion for constraints**: `T: Equal` resolution and `==` gating use the same "check
   registry or structural derivation" rule — constraints and operators always give the same answer.

**Consistency basis**: `(1, 2) == (1, 2)` and `[1] == [1]` already use runtime element-wise
comparison today (the comparison whitelist in `executor.rs` includes Tuple/List/Array), and record
types are the only excluded composite type. Auto-derivation is not a new silent default — it
completes the last piece of the language's existing internal behavior.

**Old mechanism retirement**: `Equal`'s old auto-derivation in `trait_data.rs` (only registers
signatures, no implementation code, zero consumers) is decommissioned; all `Equal` judgments
(primitive type default registration, structural derivation, explicit instantiation) go through the
interface registry. The old trait table retains Clone/Dup/Debug's existing responsibilities (names
do not overlap with operator interfaces; future unification is a separate discussion).

#### Index Interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` as a type parameter rather than an associated type member**: The RFC-011a open question
has settled on "associated types via generic interface parameters" (`Iterator: (Item: Type) -> Type`
is the same-form precedent), so no `type` member syntax needs to be introduced.

**Ownership note**: `index` borrows from `&Self` and returns the complete `Value`. The standard
library's `list.get: (&Vec(A), Int) -> A` already has the same form; **this interface receives the
same treatment as the current std state**; the precise semantics of "extracting a complete value
from a borrow" for move-semantic element types is handled uniformly when RFC-009 is fully enforced;
this RFC does not invent new rules for this.

**Multi-position index relies on tuple packing + overloading**, no variadic interface:

```yaoxiang
// One-dimensional container
List(T) instantiates Index(List(T), Int, T)                   → arr[0]

// Multi-dimensional container
Grid    instantiates Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └─ Key is a tuple

// Two instantiations with different signatures → coexist
```

**Multiple instantiations of the same-named interface for the same type are allowed**, distinguished
by the signatures of their injected methods, coexisting per RFC-011a method-level overloading rules.
RFC-011a's overloading explicitly only goes up to the method level; this section adds a layer of
instantiation-level rules on top: the validity of same-named interface instantiations coexisting is
determined by whether the expanded method signatures conflict (conflict = E1097, same source as the
field/method namespace rules).

#### Propagation Interface (Phase 2, Shape TBD)

The interface-ization target of `?` is named `Try`, but the **complete shape is TBD** and is not in
the initial scope. Reasons:

1. `?` actually does three things: determine success/failure (currently via hardcoded variant 0),
   extract the success payload, and on failure, return the entire value as-is from the current
   function. A single method `residual: (self: &Self) -> E` only covers part of the third; writing
   it now would likely require rework later;
2. Phase 2 inherently depends on RFC-010 (constructing `Result` values) and RFC-010b (variant
   destructuring) being landed;
3. The accompanying **constructor welding problem** (`ok` / `err` / `some` recognized by the parser,
   language spec §1.4.2) and `?`'s type name welding are two halves of the same thing; "Moving
   `Result` to std" must resolve both halves together, both within Phase 2 scope.

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
    println(a == b)             # false —— Equal auto-derived, no instantiation needed
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

#### Generic Constraints Finally Deliverable

```yaoxiang
// RFC-011's example is now deliverable
// T: Add ≜ Add(T, T, T) registered; T: Multiply ≜ Multiply(T, T, T) registered
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

(`Zero` / `One` in RFC-011's signature example `T: Add + Multiply + Zero` are not within this RFC's
scope — they are constant members rather than operators; "interface members without a receiver" has
no precedent in RFC-011a; see Open Questions.)

### Syntax Changes

**No new syntax, no new keywords**. All capabilities are composed of existing mechanisms:

| Capability              | Existing Mechanism Reused                                              |
| ----------------------- | ---------------------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                                       |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                              |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)                    |
| Associated types        | RFC-011 §3.1 (interface type parameters)                               |
| Multi-position index    | Existing tuple packing parsing + instantiation-level overloading rules |
| Operator precedence     | **Language-fixed**, not open to user customization                     |

## Detailed Design

### Type System Impact

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` (`Try`
in Phase 2).

**Interface implementation registry**: The central nervous system of operators. Aggregates core
default registrations (primitive types) and user instantiations (`ImplementationProof` produced by
`check_interface_instantiation`). `+` / `==` / `[]` gating checks and `T: Add` constraint resolution
(`check_trait_bounds` in `bounds.rs`) **query the same table** — there is no gap where "constraint
says yes, operator says no".

**`Equal` structural derivation**: A structural rule layered on top of the registry (all fields
comparable ⇒ comparable), with primitive types covered by core registration and recursive closure.
`Equal` registration and old auto-derivation in `trait_data.rs` are decommissioned.

**Operator query and name resolution separated**: Operators like `+` only query the registry, not
ordinary name resolution; local same-named bindings (such as type-level `Add` family) do not affect
operators (see §Name and Registration).

**Orphan rule**: Operator implementations can only be written in the **type's defining module** —
`Int`'s definition is in core, so `Int`'s registration can only be done by core; `Point`'s
definition is in the user module, so only its definer can register for it. All types follow the same
rule, with no privilege for built-in types.

### Runtime Behavior

**Zero runtime overhead**: Operators determine the call target at compile time (static dispatch).
For primitive types (`Int`/`Float`/`String`/`List`), the **native instruction fast path** is
preserved, without going through interface dispatch; auto-derived struct `==` generates native
field-by-field comparison; `Any` (dynamic type) position's `==` / `!=` maintains existing runtime
comparison (RFC-036 test framework's `assert_eq` depends on this behavior and is unaffected).

**`%` semantic fix**: `-7 % 3` changes from `-1` (truncated remainder) to `2` (mathematical modulo).
The interpreter (`checked_rem`), constant folding (`a % b`), and bytecode (`I64_REM`) are modified
in three places synchronously.

### Compiler Changes

| Component                            | Changes                                                                                                                                                                                                                   |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | `infer_binary` whitelist changed to "native fast path + registry query" dual path; `Expr::Index` whitelist adds interface query branch                                                                                    |
| `typecheck/inference/bounds.rs`      | `check_trait_bounds` switches to interface registry for operator interface names (with `Equal` structural derivation)                                                                                                     |
| `typecheck/checker.rs`               | Add `Equal` structural derivation (after record definition); instantiation-level overloading rules (same-named interface multiple instantiations coexist by method signature)                                             |
| `middle/core/ir_gen.rs`              | `Expr::Index` adds method call dispatch branch; struct `==` without explicit `equal` generates native field-by-field comparison; `Mod` instruction changed to mathematical modulo                                         |
| Interface registry (new)             | Layer 0 mapping table constants + operator→interface query functions + core default registrations (`Int`/`Float`/`String`/`List`, etc.)                                                                                   |
| `trait_data.rs`                      | `Equal` registration and old auto-derivation decommissioned (`Clone`/`Dup`/`Debug` remain as-is)                                                                                                                          |
| Diagnostics                          | `Equal` precondition (linear token) unsatisfied reuses `E1101` (type does not implement interface) family; `E1081`/`E1082` text removes "Result" wording (with Phase 2 Try landing, three-party sync per RFC-013 process) |

### Backward Compatibility

**Primitive type operations unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, `1 < 2` all retain
native paths, with unchanged behavior and performance.

**`1 + 2.5` changes from compile error to valid**: After core registration `Add(Int, Float, Float)`,
mixed arithmetic previously rejected by the whitelist becomes usable, returning `Float`. This is a
new capability, with no impact on existing code.

**`%` semantic fix**: `-7 % 3` changes from `-1` to `2`. Classified as a **bug fix** (documentation
already promised modulo, see motivation §5), with no compatibility period; existing test corpus has
no cases relying on negative `%` (verified).

**Struct `==` changes from runtime error to usable**: Previously `Struct == Struct` always produced
E6007 runtime error; after wiring, auto-derivation makes it usable — going from bad to good, with no
impact on existing valid code.

**`Any`'s `==` unaffected**: Dynamic type position maintains runtime comparison (RFC-036's
`assert_eq` assertion family depends on this; previously usable, still usable).

**`f[0]` positional binding unchanged**: `distance[0]` is a compiler built-in capability from
RFC-004, **does not go through the `Index` interface**, and is unaffected.

**`?` is transparent to existing code** (Phase 2): After `Result` adds `Try` instantiation, existing
`?` usage behaves exactly the same.

## Trade-offs

### Advantages

- **Delivers RFC-011's operator constraints**: `T: Add + Multiply` goes from paper (actually a
  compile error) to deliverable, without modifying the already-accepted RFC-011 body
- **Removes `Result`'s core binding** (Phase 2): After `?` and constructors are interface-ized,
  `Result` can belong to std, aligning with RFC-013's established positioning
- **Fixes tested bugs**: `Point == Point` works out of the box, collaterally fixing
  `list.contains`'s unavailability for structs
- **Zero new syntax**: Fully reuses RFC-011a's existing mechanisms, not touching parser syntax rules
- **Zero runtime overhead**: Static dispatch + primitive type native fast path
- **User-defined containers become usable**: `Box(T)[0]` changes from "always error" to usable
- **Internal language consistency**: Tuple/List already have element-wise comparison, record types
  complete the picture; arithmetic interfaces with three parameters and Index have a uniform shape;
  RFC-011 §8.3 promotion table and interface registry are unified

### Disadvantages

- **`Equal` auto-derivation is a silent default**: If in the future we want to withdraw "records are
  comparable by default", it is a breaking change. This position is accepted — consistent with
  Tuple/List's existing behavior, and explicit instantiation can always override
- **`Equal` precondition rejects types with linear tokens**: Types containing `&mut` fields cannot
  `==`; this is a cost of semantic correctness, requiring clear diagnosis (indicating which field)
- **Interface instantiation is explicit cost**: Each arithmetic operator requires one line of
  instantiation + one method (`Equal` is exempted — auto derived). RFC-011a's syntax determines
  implicit derivation is not possible (in exchange for `Self` type parameter without magic)
- **`%` semantic fix is a behavior change**: Although classified as a bug fix, it still needs to be
  noted in migration documentation

## Alternatives

### Option A: Only Layer 1 (dispatch by method name), no interface layer

`+` only checks whether there is a method named `add`, without requiring implementation of the `Add`
interface.

**Reason for rejection**: RFC-011's `T: Add` constraint would be decoupled from operators — the
constraint checks the interface, while the operator checks the method name, possibly giving
inconsistent answers. And there is no way to provide accurate compile-time diagnosis for "`+` used
on non-addable type".

### Option B: Introduce constructor syntax `Ok(x)` / `Some(x)` to solve the `?` problem

Do not interface-ize `?`, but add constructor syntax to sum types.

**Reason for rejection**: Conflicts with RFC-010 (already accepted). RFC-010 explicitly states
"uniformly use record types to express sum types, **two sets of syntax not required**", and
explicitly deprecates the `|` syntax. Introducing constructors introduces a second set of
expressions. And it only solves `?`, not `Point == Point` and custom container indexing.

### Option C: Comparison operators also interface-ized in initial scope (introducing `Ordering`)

`<` `<=` `>` `>=` go through the `Compare` interface, returning three-valued `Ordering`.

**Reason for rejection**: `<` is already a **first-class IR instruction** in YaoXiang
(`Instruction::Lt/Le/Gt/Ge`); interface-ization would force primitive types to detour. And
introducing `Ordering` brings out a whole set of issues including `Ordering`'s own
comparison/sorting, float `NaN`'s `PartialOrd` vs `Ord`, etc., which is the scope of an independent
RFC. The currently tested needs (`Point == Point`, `list.contains`) **only need `Equal`**.

### Option D: Use abbreviations for operator names (`Add` interface's method is called `add`, interface called `Mul`)

**Reason for rejection**: RFC-011's body already writes `T: Add + Multiply + Zero`; using
abbreviations would require changing already-accepted RFCs.

### Option E: `Equal` only does explicit instantiation, no auto-derivation

**Reason for rejection**: User-written types cannot `==`, which is unacceptable user experience; and
`(1, 2) == (1, 2)`, `[1] == [1]` already have element-wise comparison today, with only record types
excluded, which is inconsistent to begin with. Explicit instantiation is retained as an override
means, accommodating custom needs.

### Option F: Arithmetic interface return type hard-coded to `-> Self`

**Reason for rejection**: `Add(Int, Float)`'s method would be forced to return `Int`, and `1 + 2.5`
cannot be correctly expressed; vector scaling `Multiply(Point, Float)` similarly cannot be written.
And RFC-011 §8.3's promotion type family `(Int, Float) => Float` would lose its landing place. The
three-parameter `(Self, R, O)` has a uniform shape with `Index(Key, Value)`.

## Implementation Strategy

### Dependencies

| Dependency                             | Status        | Impact on This RFC Part                                                         |
| -------------------------------------- | ------------- | ------------------------------------------------------------------------------- |
| RFC-011a interface mechanism Phase 1–3 | ✅ Landed     | Layer 2 foundation (verified runnable)                                          |
| RFC-010 record-style construction path | ❌ Not landed | **Phase 2**: `?` construction side + constructor parser special-case retirement |
| RFC-010b variant destructuring         | ❌ On paper   | **Phase 2**: `Result.residual`'s `match` writing, exhaustiveness                |
| RFC-009 linear token inference         | Partial       | `Equal` precondition check                                                      |

### Phased

Divided into two groups by **interface dependency** (design constraint, not scheduling):

**Phase 1 — Does not depend on RFC-010/010b**:

- Layer 0 mapping table + interface implementation registry + Layer 1 dispatch wiring
- Seven interfaces `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
  (three-type-parameter form)
- `Equal` auto-derivation + linear precondition + constraint resolution switchover to registry
- `%` semantic fix (including interpreter/constant folding/bytecode three places)
- `Any` position `==` maintains runtime comparison
- **Benefits**: `Point + Point`, `Point == Point` (no ceremony), `Box(T)[0]`, `1 + 2.5`, `T: Add`
  constraints all become available

**Phase 2 — Depends on RFC-010 / RFC-010b, `Try` shape must be finalized before work begins**:

- `Try` interface shape finalized (see Open Questions)
- `?` interface-ization + constructor (`ok`/`err`/`some`) parser special-case retirement, switched
  to RFC-010 record construction path
- `Result` moved from core to std (per RFC-013 established positioning)

### Risks

| Risk                                                            | Mitigation                                                                                             |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `%` semantic change affects existing user code                  | Classified as bug fix + verified test corpus has no negative `%` dependency                            |
| `Equal` auto-derivation's silent default hard to withdraw later | Position decided: consistent with Tuple/List existing behavior, explicit instantiation can override    |
| Primitive type dual paths (native + registry) inconsistent      | Gate: core registration's semantics must be consistent with native instructions (same table two views) |
| Registry query slows down compilation                           | Table indexed by type name + instantiation result caching (reuse RFC-011a proof)                       |
| Instantiation-level overloading introduces ambiguity            | Same rules as method-level overloading: signature conflict = E1097                                     |

## Coordination with Other RFCs

This RFC's positioning is that of a **consumer-requirements proposer**, requiring synchronous
updates to the following RFCs for coordination consistency (revision authorized):

| RFC                                    | Updates Needed                                                                                                                                                                                                                                                                                   |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **RFC-011** (accepted)                 | In the constraints section, note `Add` / `Multiply` are defined and landed by 011b (`T: Add` ≜ `Add(T, T, T)`); mark `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` as dangling constraint names (awaiting subsequent RFCs); §8.3 promotion type family noted as unified with interface registry |
| **RFC-010** (accepted)                 | Clarify the landing requirements of "record fields as constructors" — it is the prerequisite for Phase 2 constructor parser special-case retirement                                                                                                                                              |
| **RFC-010b** (draft, formerly RFC-039) | Construction and destructuring must be paired; exhaustiveness determination does not depend on `Result`'s core/std attribution (will migrate in 011b Phase 2)                                                                                                                                    |
| **RFC-009** (accepted)                 | In §Type Attributes, cross-reference: `Equal` precondition is "does not contain `&mut` linear tokens" (not Dup)                                                                                                                                                                                  |
| **RFC-013** (accepted)                 | When Phase 2 lands: `E1081` / `E1082` text removes "Result" wording; `Equal` precondition diagnosis reuses `E1101` family — three-party sync per RFC-013 process (codes/*.rs ↔ locales ↔ code table)                                                                                             |
| **RFC-018** (accepted)                 | After `%` changes to mathematical modulo, the `Mod → srem/urem` mapping table becomes invalid and needs to be changed to `srem` + sign correction (or `sdiv`+`mul`+`sub` synthesis), and correct the "modulo/remainder" terminology confusion                                                    |
| **RFC-036** (accepted)                 | No changes needed. Note relationship: `Any` position `==` maintains runtime comparison, `assert_eq` assertion family not affected by the gate                                                                                                                                                    |

> Basis for `Result` moving to std: RFC-013 has already positioned "std library `Result(T, Error)`",
> and RFC-014's layering places std under core source. This RFC's Phase 2 is one of its landing
> paths and no longer references other numbers.

## Open Questions

- [x] ~~Does the `Modulo` semantic fix need a compatibility period?~~ → **Closed**: The language
      reference already specifies "multiplication, division, modulo", and the current remainder
      implementation violates published documentation; treated as a bug fix with no compatibility
      period (2026-09-22)
- [x] ~~Can users add operator implementations to existing types (orphan rule)?~~ → **Finalized**:
      Operator implementations can only be written in the type's defining module. `Int`'s definition
      is in core, so only core can register for it; users can only register for their own types. All
      types are treated equally, with no privilege for built-in types (2026-09-22)
- [ ] Should `Index` distinguish mutable indexing (similar to Rust's `IndexMut`)? (Initial scope is
      read-only; mutable indexing involves RFC-009's `WriteToken`, and RFC-011a already has the
      `&mut Self` receiver precedent, deferred to later)
- [ ] Multi-position index Key uses tuple packing (current state) or is changed to multiple
      parameters (Swift style)? (Sticking with tuple packing, not changing the parser)
- [ ] Form of `Zero` / `One`: constant members are not operators; "interface members without a
      receiver" has no precedent in RFC-011a and requires separate finalization before RFC-011's
      `T: Add + Multiply + Zero` can be fully delivered
- [ ] Complete shape of the `Try` interface: `?` needs three things — success/failure determination,
      success payload extraction, and failure path producing outer function return value — and a
      single method `residual` is insufficient; together with constructor parser special-case
      retirement, must be finalized before Phase 2 begins
- [ ] The `?T` prefix type (RFC-026 FFI nullable annotation, RFC-018) and the `e?` suffix operator
      share the `?` symbol: different positions (type position vs expression position) do not
      constitute a conflict, just needs to be written into the spec
- [ ] `PartialOrd` / `Ordering` (comparison operator interface-ization): independent RFC, this RFC
      explicitly does not do it

---

## Appendix A: Investigation Evidence

The following real tests were all reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Interface Mechanism Availability (This RFC's Foundation)

| Capability                                                                     | Real Test                                                      |
| ------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Definable                                                   |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runnable, correct output (unit tests in `tests/rfc011a.rs`) |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (field and method share namespace conflict)         |
| Method dispatch `d.speak()`                                                    | ✅ Runnable                                                    |

**Note**: `E1097` means operator methods must use the **external declaration** form
(`Point.add: (self: &Point, ...)`), consistent with RFC-011a's example. `method_bindings` is
registered in `environment.rs` (`add_method_binding`) and queried in `expressions.rs`'s call target
resolution and field lookup failure fallback path.

### A.2 Operator Current State

| Expression                            | Current State                                                                            |
| ------------------------------------- | ---------------------------------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]`   | ✅ Hardcoded whitelist (Int/Float/String/List, **requires both sides of the same type**) |
| `1 + 2.5` (Int + Float)               | ❌ Compile error (whitelist requires both sides of the same type)                        |
| `-7 % 3`                              | `-1` (truncated remainder; `%` whitelist only Int/Float, different from `+` whitelist)   |
| `Point(1,2) == Point(1,2)`            | ❌ `E6007` (Eq runtime failure on Struct)                                                |
| `(1,2) == (1,2)` / `[1] == [1]`       | ✅ Runtime element-wise comparison (Struct is the only gap)                              |
| `Any` position `a == b` (`assert_eq`) | ✅ Runtime comparison (verified by RFC-036)                                              |
| `f[0]` (function positional binding)  | ⚠️ Only valid within binding declaration, as expression reports `E3006`                  |
| `arr[0, 1]` (multi-position)          | ✅ Tuple packing, `list([1, 2])`                                                         |

### A.3 Hardcoded Locations of `?` and Constructors

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Constructor side: `ok(T)` / `err(E)` / `some(T)` are recognized by the parser (language spec
`syntax.md` §1.4.2); `std/result.rs`'s `is_ok` / `unwrap` etc. match by `variant_id 0/1` pattern.
"Moving `Result` to std" requires both halves (`?` + constructors) to be resolved together, both
within Phase 2.

## Appendix B: Design Decision Record

| Decision                             | Decision                                                                                        | Reason                                                                                                                                          | Date       |
| ------------------------------------ | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Architecture                         | Three-layer separation (mapping table / dispatch base / interface contract)                     | Merging would decouple RFC-011 constraints from operators                                                                                       | 2026-09-15 |
| Operator gating condition            | Must implement interface (Layer 2 as gating condition)                                          | Guarantees `T: Add` strictly corresponds to `+`                                                                                                 | 2026-09-15 |
| Interface naming                     | Full words (`Multiply` not `Mul`)                                                               | Not changing RFC-011's already-accepted body                                                                                                    | 2026-09-15 |
| `%` interface name                   | `Modulo` (mathematical modulo)                                                                  | Name pins down semantics                                                                                                                        | 2026-09-15 |
| Comparison operators                 | **Not** interface-ized in initial scope, preserve native IR instructions                        | Already first-class instructions; `Ordering` brings out a whole set of independent issues                                                       | 2026-09-15 |
| `Ordering`                           | Not introduced in initial scope                                                                 | No real demand driving it, scope of an independent RFC                                                                                          | 2026-09-15 |
| Associated types                     | Use interface type parameters, no `type` member syntax                                          | RFC-011a has finalized this approach (`Iterator: (Item: Type)`)                                                                                 | 2026-09-15 |
| Method binding and index unification | Conceptually unified, interface only handles container indexing                                 | Positional binding's key is a compile-time constant, result type requires type family evaluation, cannot be implemented by users                | 2026-09-15 |
| Multi-position index                 | Tuple packing + overloading by Key type, no variadic interface                                  | Not changing the parser                                                                                                                         | 2026-09-15 |
| Bitwise / unary operators            | Not in initial scope                                                                            | Rare for user-defined types, YAGNI                                                                                                              | 2026-09-15 |
| Arithmetic interface shape           | Three type parameters `(Self, R, O)`; `T: Add` ≜ `Add(T, T, T)`                                 | Result type explicit: `1 + 2.5`, scaling expressible; RFC-011 §8.3 promotion table and registry unified; uniform shape with `Index(Key, Value)` | 2026-09-22 |
| `Equal` derivation                   | Default auto-derivation + explicit instantiation override; constraint resolution same criterion | "Types you write cannot ==" unacceptable; Tuple/List already have element-wise comparison, Struct is the only gap                               | 2026-09-22 |
| `Equal` precondition                 | "Does not contain `&mut` linear tokens", **not Dup**                                            | RFC-011 §2.4 explicitly states primitives are not Dup; using Dup as premise would wrongly reject `Point{Float,Float}`                           | 2026-09-22 |
| Name and registration separation     | Operators only query interface implementation registry, not name resolution                     | Local same-named bindings (type-level `Add` family, etc.) do not interfere with operators; RFC-011 §5.2 example needs no changes                | 2026-09-22 |
| Orphan rule                          | Implementation follows the type's defining module                                               | All types treated equally, no privilege for built-in types                                                                                      | 2026-09-22 |
| `Any`'s `==`                         | Maintain runtime comparison, do not query registry                                              | RFC-036 `assert_eq` assertion family already depends on this behavior                                                                           | 2026-09-22 |
| `Try` shape                          | Suspended until Phase 2 work begins for finalization                                            | `?` needs three things, single method `residual` insufficient; constructor parser special case included together                                | 2026-09-22 |
| `%` semantic classification          | Bug fix (documentation already promised modulo), no compatibility period                        | `reference/index.md` "multiplication, division, modulo" as first evidence                                                                       | 2026-09-22 |
| Basis for `Result` moving to std     | Cite RFC-013 established positioning, no longer reference non-existent number                   | RFC-013 already writes "std library `Result(T, Error)`"                                                                                         | 2026-09-22 |

## Appendix C: Glossary

| Term                              | Definition                                                                                                                                                     |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layer 0                           | Fixed mapping table of operator → method name, language-level constant, user-immutable                                                                         |
| Layer 1                           | Dispatch base that looks up by `Type.method` key and calls                                                                                                     |
| Layer 2                           | Interface contract layer, providing basis for generic constraints and also serving as the operator gating condition                                            |
| Interface implementation registry | Aggregates core default registrations and user instantiations; the sole criterion for operator query and constraint resolution, independent of name resolution |
| Native fast path                  | Hardcoded operation path preserved for primitive types, not through interface dispatch                                                                         |
| Positional binding                | RFC-004's `f[0]` syntax, which binds function parameter positions as methods, compile-time behavior                                                            |
| Auto-derivation                   | Compiler-generated field-by-field `==` for records whose fields are all comparable; explicit instantiation can override                                        |

## References

- [RFC-011: Generic Type System Design](./011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraints, associated types
- [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
  Interface declaration/instantiation/overloading rules
- [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear token
- [RFC-004: Multi-Position Combined Binding for Curried Methods](./004-curry-multi-position-binding.md)
  — `f[0]` syntax
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — Sum type expression
- [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` belongs to std
  positioning, error code process
- [RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)](../draft/010b-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — Associated type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — Multi-parameter subscript
