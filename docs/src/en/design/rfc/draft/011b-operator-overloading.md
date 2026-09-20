---
title: 'RFC-011b: Operator Overloading and Interface-Driven Operators'
status: 'Draft'
author: 'Chenxu'
created: '2026-09-15'
updated: '2026-09-15'
group: 'rfc-011'
issue: '#341'
---

# RFC-011b: Operator Overloading and Interface-Driven Operators

> **References**:
>
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md) — type constraint
>   `T: Add + Multiply`, associated types
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
>   interface declaration/instantiation mechanism
> - [RFC-009: Ownership Model Design](./009-ownership-model.md) — `Dup` / `Linear` type properties
> - [RFC-004: Multi-Position Joint Binding for Curried Methods](./004-curry-multi-position-binding.md)
>   — `f[0]` positional binding syntax
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type = record type where all
>   fields return the type itself
> - [RFC-039: Pattern Matching Completeness](../draft/039-pattern-matching-completeness.md) —
>   variant deconstruction (dependency)

## Summary

This RFC adds **operator overloading** capability to YaoXiang, so that `a + b` / `a == b` / `a[i]` /
`e?` can be implemented by user-defined types, and the `T: Add + Multiply + Zero` constraint already
written in RFC-011 transitions from a **paper capability** to a working mechanism.

The design adopts a **three-layer separation of concerns**: a fixed operator-to-method mapping table
(Layer 0), a name-based dispatch base (Layer 1, reusing the existing `method_bindings`), and an
interface contract layer (Layer 2, for generic constraints). The **precedence and associativity of
operators remain language-fixed**; users only overload semantics.

First batch scope: eight interfaces — `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
`Try`. No new keywords are introduced, and no new syntactic constructs are added.

## Motivation

### Why this feature is needed

#### 1. RFC-011's core example depends on it, but is currently a paper capability

RFC-011 (already accepted) uses operator names as type constraints in 8 places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

This is RFC-011's showcase example for "value-dependent types + compile-time dimension
verification." But **none of the entire RFC set defines where `Add` / `Multiply` come from, or how
`+` binds to them**. `T: Add` is currently an unenforceable assertion.

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

Collateral damage: `list.contains(list_of_structs, p)` is **completely unusable** — it internally
depends on `==`.

#### 3. `?` hardcodes the type name into the compiler, blocking `Result` from moving to std

The implementation of `?` simultaneously hardcodes the type name and the variant number:

```rust
// typecheck: hardcoded construction of Result type
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hardcoded group name and variant number
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` were made **interface-driven**, any type implementing
that interface (including user-defined types) could be used with `?`, and `Result` could be moved to
std.

#### 4. User-defined containers cannot be indexed

The type check for `Index` is a hardcoded whitelist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// Otherwise → "Better to reject unknown containers than be silent"
```

For any user-defined container type, `c[0]` always reports an error.

### Existing semi-finished foundations

Investigation found that **most of the mechanism already exists**; what's missing is the wiring:

| Mechanism                                                           | Location                                     | Status                                         |
| ------------------------------------------------------------------- | -------------------------------------------- | ---------------------------------------------- |
| Interface declaration `(Self: Type) -> Type`                        | RFC-011a Phase 1                             | ✅ Runnable                                    |
| Interface instantiation `Animal(Dog)` + external method declaration | RFC-011a Phase 2–3                           | ✅ Runnable                                    |
| Method dispatch `method_bindings["Type.method"]`                    | `expressions.rs:1299`                        | ✅ Runnable                                    |
| Associated types (= interface type parameters)                      | RFC-011 §3.1; RFC-011a adopted this approach | ✅ Mechanism fixed                             |
| Type family evaluation `AssociatedTypeDef`                          | `dependent_types.rs`                         | ✅ Production use case `std.assert`'s `IsTrue` |
| `Equal` / `Dup` / `Clone` / `Debug` trait                           | registered in `trait_data.rs`                | ⚠️ `Equal` has **zero consumers**              |
| Operator → method name mapping                                      | —                                            | ❌ Does not exist                              |

**Conclusion**: This is not building a feature from scratch, but wiring up existing components and
documenting them.

## Proposal

### Core design: three-layer separation of concerns

```
Layer 0  Fixed mapping table (language-level constant, user-immutable)
         + → add    == → equal    [] → index    ? → residual
         │  Built in at compile time, not involved in type inference, not exposed to users
         ▼
Layer 1  Dispatch base (look up method by name and invoke)  ← reuses existing mechanism
         method_bindings["Point.add"]
         ▼
Layer 2  Interface contract (for generic constraints)
         Add / Equal / Index / Try …
         Makes the T: Add constraint valid, and serves as the gate for operators
```

**Rationale for layering**:

- **Layer 0 and 1 make operators "usable"**; Layer 2 makes operators "constrainable." Merging them
  would cause any method named `add` to be called by `+`, decoupling RFC-011's `T: Add` from
  operators.
- **Layer 1 is not new**: `method_bindings` already looks up by `Type.method` key (fallback path
  after field lookup fails); operators can use the same path.
- **Layer 2 is the gate**: before an operator is allowed, the type **must** be confirmed to have
  implemented the corresponding interface (see §"Why interfaces must be implemented").

### Layer 0: Fixed mapping table

| Operator          | Interface                                                 | Method     | First batch |
| ----------------- | --------------------------------------------------------- | ---------- | ----------- |
| `+`               | `Add`                                                     | `add`      | ✅          |
| `-`               | `Subtract`                                                | `subtract` | ✅          |
| `*`               | `Multiply`                                                | `multiply` | ✅          |
| `/`               | `Divide`                                                  | `divide`   | ✅          |
| `%`               | `Modulo`                                                  | `modulo`   | ✅          |
| `==` `!=`         | `Equal`                                                   | `equal`    | ✅          |
| `[]`              | `Index`                                                   | `index`    | ✅          |
| `?`               | `Try`                                                     | `residual` | ✅          |
| `<` `<=` `>` `>=` | — (reserved native instruction)                           | —          | ❌          |
| `and` `or`        | — (short-circuit is language semantics, not overloadable) | —          | ❌          |
| 5 bitwise ops     | —                                                         | —          | ❌          |
| Unary `-` `!`     | —                                                         | —          | ❌          |

**Interface names use full words rather than abbreviations** (`Multiply` rather than `Mul`): this is
consistent with RFC-011's text `T: Add + Multiply + Zero`, and **does not modify already-accepted
RFCs**.

**`%` adopts `Modulo` semantics** (mathematical modulo, result sign follows the divisor), rather
than the current implementation's remainder behavior (`-7 % 3` currently returns `-1`). This is an
**intentional semantic correction** — the name pins the semantics to mathematical modulo.

### Layer 2: Interface definitions

#### Arithmetic interfaces

```yaoxiang
Add: (Self: Type, R: Type) -> Type = {
    add: (self: &Self, other: &R) -> Self
}

Subtract: (Self: Type, R: Type) -> Type = {
    subtract: (self: &Self, other: &R) -> Self
}

Multiply: (Self: Type, R: Type) -> Type = {
    multiply: (self: &Self, other: &R) -> Self
}

Divide: (Self: Type, R: Type) -> Type = {
    divide: (self: &Self, other: &R) -> Self
}

Modulo: (Self: Type, R: Type) -> Type = {
    modulo: (self: &Self, other: &R) -> Self
}
```

`R` is the right operand type, allowing heterogeneous left/right operands (e.g. `Int + Float`). The
return value is fixed as `Self` (`&Self` borrows the receiver, see RFC-009 borrow tokens and
RFC-011a receiver conventions).

#### Equality interface

```yaoxiang
Equal: (Self: Type, R: Type) -> Type = {
    equal: (self: &Self, other: &R) -> Bool
}
```

**`Equal` implicitly requires `Dup`**: the semantics of `==` is "compare two values," which requires
both to be readable. Values of linear (non-`Dup`) types can only be read once and cannot participate
in comparison. RFC-009 already provides the causal chain:

> **Causality cannot be reversed: freeze is the cause, Dup is the result.**

Therefore the implementation prerequisite of `Equal` is that `Self` has the `Dup` property; the
compiler rejects `Equal` instantiation when this is not satisfied.

#### Index interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` is a type parameter rather than an associated type member**: RFC-011a's open issue has
settled on "associated types via generic interface parameters" (`Iterator: (Item: Type) -> Type` is
an isomorphic precedent), and there's no need to introduce `type` member syntax.

**Multi-position indexing relies on tuple packing + overloading**, without introducing variadic
interfaces:

```yaoxiang
// 1D container
List(T) instantiates Index(List(T), Int, T)                   → arr[0]

// Multi-dimensional container
Grid    instantiates Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └─ Key is a tuple

// Two instantiation signatures differ → coexist via RFC-011a overloading rules
```

#### Propagation interface

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    residual: (self: &Self) -> E
}
```

`T` (success type) and `E` (error type) are both interface type parameters.

**Type rules for `?`**:

```yaoxiang
// f: () -> Result(T, E)
// The rest of the function body requires Result(U, E) (error types must be consistent)
x = f()?        // x: T, equivalent to:
                //   match f() {
                //       ok(v)  => v
                //       err(e) => return err(e)
                //   }
```

When the error types are inconsistent, a compile error is reported (corresponding to the existing
`E1083`); when the outer function's return type is not a propagatable type, `E1081` is reported;
when the expression does not implement `Try`, the text of `E1082` is replaced (no longer mentioning
the specific name "Result").

### Examples

#### Arithmetic and equality for user-defined types

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,
    Add(Point, Point),
    Equal(Point, Point)
}

Point.add: (self: &Point, other: &Point) -> Point =
    Point(self.x + other.x, self.y + other.y)

Point.equal: (self: &Point, other: &Point) -> Bool =
    self.x == other.x and self.y == other.y

main: () -> Void = {
    a = Point(1.0, 2.0)
    b = Point(3.0, 4.0)
    c = a + b                   // Point(4.0, 6.0)
    println(a == b)             // false
    println(a == a)             // true
}
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

#### Moving `Result` to std (design goal)

After `Result` implements `Try`, `?` no longer depends on hardcoded type names:

```yaoxiang
// inside std.result (no longer needs core special-casing)
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E),
    Try(Result(T, E), T, E)
}

Result.residual: (T: Type, E: Type)(self: &Result(T, E)) -> E =
    match self {
        ok(_) => abort_invalid_residual(),   // the ok path should not reach residual
        err(e) => e
    }
}
```

#### Generic constraints are finally enforceable

```yaoxiang
// RFC-011's example now becomes usable
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

### Syntax changes

**No new syntax, no new keywords**. All capabilities are composed of existing mechanisms:

| Capability              | Reused existing mechanism                                   |
| ----------------------- | ----------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                            |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                   |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)         |
| Associated types        | RFC-011 §3.1 (interface type parameters)                    |
| Multi-position indexing | Existing tuple packing parsing + RFC-011a overloading rules |
| Operator precedence     | **Language-fixed**, no user customization                   |

## Detailed design

### Type system impact

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` `Try`.

**Reuse already-registered trait**: `Equal` is already registered in `trait_data.rs` but has **zero
consumers** (`==` currently only does `unify` then hardcodes returning `Bool`). This RFC wires it up
as the actual contract for `==`.

**`Dup` prerequisite constraint**: when instantiating `Equal`, check whether `Self` is `Dup`. The
`Dup` / `Linear` properties are inferred by RFC-009 (`&T` freeze ⇒ Dup; `&mut T` exclusive ⇒
Linear).

**Constraint solving**: solving `T: Add` = checking whether `T` instantiates the `Add` interface.
The mechanism reuses RFC-011a's compile-time type collection, with no new solver added.

### Runtime behavior

**Zero runtime overhead**: operators determine the call target at compile time (static dispatch).
For primitive types (`Int`/`Float`/`String`/`List`), the **native instruction fast path is
retained**, without going through interface dispatch.

The runtime behavior of `?` is unchanged (unwrapping + early return); only the basis for judgment
changes from "type name == 'Result'" to "type implements `Try`".

### Compiler changes

| Component                            | Changes                                                                                                                        |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| `typecheck/inference/expressions.rs` | `infer_binary`'s whitelist is changed to "native fast path + interface query" dual path; add `Index` / `Try` interface queries |
| Same as above (`Try` branch)         | Remove `MonoType::make_result` hardcoding, change to query `Try` instantiation                                                 |
| `middle/core/ir_gen.rs`              | `Expr::Try` removes `group: "Result"` hardcoding; `Expr::Index` adds interface dispatch branch                                 |
| `typecheck/types/trait_data.rs`      | Register default instantiations for the 8 interfaces (`Int` / `Float` / `String` / `List` etc.)                                |
| New                                  | Layer 0 mapping table constants + operator→interface query function                                                            |
| Diagnostics                          | `E1082` text changes from "requires Result" to "requires Try" (`E1081`/`E1083` retained)                                       |

### Backward compatibility

**Primitive type operations are unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, `1 < 2` all retain
native paths, with behavior and performance unchanged.

**`%` semantic change**: `-7 % 3` changes from `-1` (remainder) to `2` (modulo). This is an
**intentional correction** and must be noted in the migration guide; existing test corpora have no
cases that depend on negative `%` (verified).

**`f[0]` positional binding is unchanged**: `distance[0]` is a compiler-built-in capability from
RFC-004, **does not go through the `Index` interface**, and is unaffected.

**`?` is transparent to existing code**: after `Result` gets its `Try` instantiation, existing `?`
usages behave identically.

## Trade-offs

### Advantages

- **Delivers on RFC-011**: `T: Add + Multiply + Zero` goes from paper to working, without modifying
  the already-accepted RFC text
- **Removes the core binding of `Result`**: after `?` is interface-ified, `Result` can move to std,
  fulfilling the layering goal
- **Fixes verified defects**: `Point == Point` works, which also fixes `list.contains` being
  unusable on structs
- **Zero new syntax**: fully reuses RFC-011a's existing mechanisms, without touching parser grammar
  rules (adheres to RFC-036's zero-syntax-change principle)
- **Zero runtime overhead**: static dispatch + native fast path for primitive types
- **User-defined containers are usable**: `Box(T)[0]` changes from "always errors" to usable

### Disadvantages

- **`%` semantic change is a breaking change**: despite genuine semantic reasons, a migration note
  is still required
- **`Equal` implicitly requiring `Dup` will reject some types**: values of linear types cannot be
  `==`. This is the cost of semantic correctness, but users may find it surprising; clear diagnostic
  messages are needed
- **Interface instantiation is an explicit cost**: each operator requires writing one interface
  instantiation line + one method. RFC-011a's syntax makes implicit derivation impossible (the
  trade-off is that the `Self` type parameter has no magic)

## Alternatives

### Option A: Only do Layer 1 (dispatch by method name), no interface layer

`+` only checks whether there is a method named `add`, without requiring the `Add` interface to be
implemented.

**Rejection reason**: RFC-011's `T: Add` constraint would be decoupled from operators — the
constraint queries the interface while the operator queries the method name, which can give
inconsistent answers. Furthermore, it would be impossible to provide accurate diagnostics at compile
time for "`+` used on a non-addable type".

### Option B: Introduce constructor syntax `Ok(x)` / `Some(x)` to solve the `?` problem

Instead of interface-ifying `?`, add constructor syntax for sum types.

**Rejection reason**: Conflicts with RFC-010 (already accepted). RFC-010 explicitly stipulates that
"record types uniformly express sum types, **no two syntax sets are needed**" and explicitly
deprecates the `|` syntax. Introducing constructors is introducing a second set of expressions.
Moreover, it only solves `?`, not `Point == Point` and custom container indexing.

### Option C: Make comparison operators also part of the first batch (introduce `Ordering`)

`<` `<=` `>` `>=` go through the `Compare` interface, returning the three-valued `Ordering`.

**Rejection reason**: `<` is already a **first-class IR instruction** in YaoXiang
(`Instruction::Lt/Le/Gt/Ge`); interface-ifying it would force primitive types to take a detour.
Moreover, introducing `Ordering` would bring up a whole set of issues such as `Ordering`'s own
comparison/sorting and `PartialOrd` vs `Ord` for float `NaN`, which is the scope of an independent
RFC. The current verified need (`Point == Point`, `list.contains`) only requires `Equal`.

### Option D: Use abbreviated names for operators (`Add` interface's method is `add`, but interface is `Mul`)

**Rejection reason**: RFC-011 text already writes `T: Add + Multiply + Zero`; using abbreviations
would require modifying already-accepted RFCs.

## Implementation strategy

### Dependencies

| Dependency                             | Status             | Impact on this RFC                                                                                       |
| -------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------------- |
| RFC-011a interface mechanism Phase 1–3 | ✅ Implemented     | Layer 2 foundation (verified runnable)                                                                   |
| RFC-010 record-based construction path | ❌ Not implemented | **`?` construction side** (cannot test `Try` completely without being able to construct `Result` values) |
| RFC-039 variant deconstruction         | ❌ Paper           | `Result.residual`'s `match` syntax                                                                       |
| RFC-009 `Dup`/`Linear` inference       | Partial            | `Equal`'s prerequisite constraint check                                                                  |

### Phasing

By **interface dependency** (design constraint, not scheduling), divided into two groups:

**Phase 1 — Does not depend on RFC-010/039**:

- Layer 0 mapping table + Layer 1 dispatch wiring
- Seven interfaces: `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
- `Equal` ⇒ `Dup` prerequisite constraint
- Native fast paths for primitive types are retained
- **Benefit**: `Point + Point`, `Point == Point`, `Box(T)[0]` all become usable

**Phase 2 — Depends on RFC-010 / RFC-039**:

- `Try` interface + `?` interface-ification
- `Result` migrates from core to std
- **Blocking reason**: needs to be able to construct `Result` values (RFC-010) and deconstruct them
  (RFC-039)

### Risks

| Risk                                                            | Mitigation                                                                                                 |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `%` semantic change breaks existing user code                   | Migration note + verified that test corpora have no negative `%` dependence                                |
| Explicit cost of interface instantiation annoys users           | Diagnostic message suggests "Need to implement interface X" with instantiation example                     |
| Inconsistency between primitive dual paths (native + interface) | Gate: `Int` etc. implement interfaces but method semantics must match native instructions                  |
| Layer 2 gate causes existing code to fail compilation           | `Equal` currently has zero consumers; after wiring up, only usages that were already erroring are affected |

## Coordination with other RFCs

This RFC is positioned as a **consumer-side requirements proposer**, and the following RFCs need to
be updated in sync to ensure coordinated consistency (authorized for revision):

| RFC                    | Content to be updated                                                                                            |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **RFC-011** (accepted) | In §Constraints, note that `Add` / `Multiply` etc. are defined and implemented by RFC-011b                       |
| **RFC-010** (accepted) | Clarify the implementation requirement for "record fields as constructors" — it's a prerequisite for `?` Phase 2 |
| **RFC-039** (draft)    | Construction and deconstruction must be paired; exhaustiveness check for `std` variant set handling              |
| **RFC-009** (accepted) | Cross-reference `Equal` ⇒ `Dup` prerequisite in §Type Properties                                                 |

## Open issues

- [ ] Does `Modulo`'s semantic correction need a compatibility period? (@Chenxu: leaning toward
      changing it directly, because the current behavior is a bug, not a feature)
- [ ] Should `Index` distinguish mutable indexing (like Rust's `IndexMut`)? (@Chenxu: read-only in
      the first batch; mutable indexing involves RFC-009's `WriteToken`, deferred)
- [ ] Multi-position indexing: pack keys in a tuple (current) or use multiple parameters (Swift
      style)? (@Chenxu: keep the current tuple approach, avoid changing the parser)
- [ ] Can users add operator implementations to **already-existing types**? (e.g., add custom `Add`
      to `List`) — involves orphan rules

---

## Appendix A: Investigation evidence

All of the following tests were reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Interface mechanism availability (foundation for this RFC)

| Capability                                                                     | Test Result                                                        |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------ |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Can be defined                                                  |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runnable, output correct                                        |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (conflict between fields and methods sharing namespace) |
| Method dispatch `d.speak()`                                                    | ✅ Runnable                                                        |

**Note**: `E1097` means operator methods must use the **external declaration** form
(`Point.add: (self: &Point, ...)`), consistent with RFC-011a's examples.

### A.2 Current operator status

| Expression                           | Current Status                                                      |
| ------------------------------------ | ------------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]`  | ✅ Hardcoded whitelist (Int/Float/String/List)                      |
| `Point(1,2) == Point(1,2)`           | ❌ `E6007` (Eq does not hold on Struct)                             |
| `f[0]` (function positional binding) | ⚠️ Only legal in binding declaration; as expression reports `E3006` |
| `arr[0, 1]` (multi-position)         | ✅ Tuple packing, `list([1, 2])`                                    |
| `-7 % 3`                             | `-1` (remainder, not modulo)                                        |

### A.3 `?` hardcoding locations

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Both locations need to be changed to interface queries.

## Appendix B: Design decision record

| Decision                                   | Determination                                                             | Reason                                                                                                                   | Date       |
| ------------------------------------------ | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ---------- |
| Architecture                               | Three-layer separation (mapping / dispatch / contract)                    | Merging would decouple RFC-011 constraints from operators                                                                | 2026-09-15 |
| Operator gating condition                  | Must implement interface (Layer 2 is the gate)                            | Ensures `T: Add` strictly corresponds to `+`                                                                             | 2026-09-15 |
| `Equal` and `Dup`                          | `Equal`'s definition hardcodes the `Dup` prerequisite                     | RFC-009 causal chain: freeze is the cause, Dup is the result                                                             | 2026-09-15 |
| Interface naming                           | Full words (`Multiply` rather than `Mul`)                                 | Don't change RFC-011's already-accepted text                                                                             | 2026-09-15 |
| `%` interface name                         | `Modulo` (mathematical modulo)                                            | The name pins the semantics; current remainder behavior is a defect                                                      | 2026-09-15 |
| Comparison operators                       | **Not** interface-ified in first batch, retain native IR instructions     | Already first-class instructions; `Ordering` brings up an entire separate set of issues                                  | 2026-09-15 |
| `Ordering`                                 | Not introduced in the first batch                                         | No real need driving it; belongs to an independent RFC's scope                                                           | 2026-09-15 |
| `?` interface name                         | `Try`, method `residual`                                                  | Don't pre-empt unification with future ternary syntactic sugar, avoid over-design                                        | 2026-09-15 |
| Associated types                           | Use interface type parameters, don't introduce `type` member syntax       | RFC-011a already settled on this approach (`Iterator: (Item: Type)`)                                                     | 2026-09-15 |
| Unification of method binding and indexing | Conceptually unified, interfaces only handle container indexing (Layer A) | Positional binding keys are compile-time constants, result types need type family evaluation, cannot be user-implemented | 2026-09-15 |
| Multi-position indexing                    | Tuple packing + overloading by Key type, no variadic interface            | Don't change the parser; rely on RFC-011a overloading rules to coexist                                                   | 2026-09-15 |
| Bitwise / unary operators                  | Not done in the first batch                                               | Rare for user-defined types, YAGNI                                                                                       | 2026-09-15 |

## Appendix C: Glossary

| Term               | Definition                                                                                                    |
| ------------------ | ------------------------------------------------------------------------------------------------------------- |
| Layer 0            | Fixed mapping table from operators to method names; language-level constant, user-immutable                   |
| Layer 1            | Dispatch base that looks up by `Type.method` key and invokes                                                  |
| Layer 2            | Interface contract layer; provides basis for generic constraints, and is also the operator gating gate        |
| Native fast path   | Hardcoded operation path retained for primitive types (Int/Float/String/List), does not go through interfaces |
| Positional binding | RFC-004's `f[0]` syntax, which binds a function's parameter position as a method; compile-time behavior       |

## References

- [RFC-011: Generic Type System Design](./011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraint, associated types
- [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
  interface declaration/instantiation/overloading rules
- [RFC-009: Ownership Model Design](./009-ownership-model.md) — `Dup` / `Linear`
- [RFC-004: Multi-Position Joint Binding for Curried Methods](./004-curry-multi-position-binding.md)
  — `f[0]` syntax
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type expression
- [RFC-039: Pattern Matching Completeness](../draft/039-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — associated type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — multi-parameter subscripts
