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
>   `T: Add + Multiply`, associated type
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
>   interface declaration/instantiation mechanism
> - [RFC-009: Ownership Model Design](./009-ownership-model.md) — `Dup` / `Linear` type attributes
> - [RFC-004: Multi-Position Union Binding for Curried Methods](./004-curry-multi-position-binding.md)
>   — `f[0]` position-binding syntax
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type = record where all
>   fields return the type itself
> - [RFC-039: Pattern Matching Completeness](../draft/039-pattern-matching-completeness.md) —
>   variant deconstruction (dependency)

## Summary

This RFC completes the **operator overloading** capability for YaoXiang, enabling `a + b` / `a == b`
/ `a[i]` / `e?` to be implemented by user-defined types, and turns the `T: Add + Multiply + Zero`
constraint already written in RFC-011 from a **paper capability** into a deliverable mechanism.

The design adopts a **three-layer separation of responsibilities**: a fixed operator→method mapping
table (Layer 0), a name-based dispatch foundation (Layer 1, reusing the existing `method_bindings`),
and an interface contract layer (Layer 2, used for generics constraint). The **precedence and
associativity of operators remain language-fixed**; users only overload semantics.

First batch scope: eight interfaces — `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
`Try`. No new keywords are introduced, and no new syntactic structures are added.

## Motivation

### Why This Feature Is Needed

#### 1. RFC-011's Core Examples Depend on It, but Are Currently Paper-Only

RFC-011 (already accepted) uses operator names as type constraints in eight places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

This is RFC-011's showcase example of "value-dependent types + compile-time dimension verification."
But **across the entire set of RFCs, not a single document defines where `Add` / `Multiply` come
from or how `+` binds to them**. `T: Add` is currently an unfulfillable assertion.

#### 2. User-Defined Types Cannot Participate in Basic Operations (Verified)

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

A cascading consequence: `list.contains(list_of_structs, p)` is **completely unusable** — it
internally depends on `==`.

#### 3. `?` Hardcodes Type Names into the Compiler, Blocking `Result`'s Move to std

The implementation of `?` hardcodes both the type name and the variant number:

```rust
// typecheck: hardcoded construction of the Result type
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hardcoded group name and variant number
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` were made **interface-driven**, any type that
implements that interface (including user-defined ones) could be used by `?`, allowing `Result` to
be moved to std.

#### 4. User-Defined Containers Cannot Be Indexed

The type check for `Index` is a hardcoded whitelist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// otherwise → "refuse to silently accept unrecognized container types"
```

Any container type defined by the user will trigger an error on `c[0]`.

### Existing Semi-Finished Foundations

Investigation shows that **most of the mechanism is already in place**; what's missing is the
wiring:

| Mechanism                                                           | Location                                     | Status                                          |
| ------------------------------------------------------------------- | -------------------------------------------- | ----------------------------------------------- |
| Interface declaration `(Self: Type) -> Type`                        | RFC-011a Phase 1                             | ✅ Runnable                                     |
| Interface instantiation `Animal(Dog)` + external method declaration | RFC-011a Phase 2–3                           | ✅ Runnable                                     |
| Method dispatch `method_bindings["Type.method"]`                    | `expressions.rs:1299`                        | ✅ Runnable                                     |
| Associated type (= interface type parameter)                        | RFC-011 §3.1; RFC-011a adopted this approach | ✅ Mechanism defined                            |
| Type family evaluation `AssociatedTypeDef`                          | `dependent_types.rs`                         | ✅ Production use case: `std.assert`'s `IsTrue` |
| `Equal` / `Dup` / `Clone` / `Debug` trait                           | Registered in `trait_data.rs`                | ⚠️ `Equal` has **zero consumer sites**          |
| Operator → method-name mapping                                      | —                                            | ❌ Nonexistent                                  |

**Conclusion**: This is not building a feature from scratch, but wiring together existing parts and
documenting them.

## Proposal

### Core Design: Three-Layer Separation of Responsibilities

```
Layer 0  Fixed mapping table (language-level constant, not user-modifiable)
         + → add    == → equal    [] → index    ? → residual
         │  Compile-time built-in, does not participate in type inference, not exposed to users
         ▼
Layer 1  Dispatch foundation (lookup method by name and invoke)  ← Reuses existing mechanism
         method_bindings["Point.add"]
         ▼
Layer 2  Interface contract (used for generics constraint)
         Add / Equal / Index / Try …
         Makes T: Add constraints valid, and serves as the gate for operators
```

**Rationale for the layering**:

- **Layers 0 and 1 make operators "usable"**, while Layer 2 makes operators "constrainable." Merging
  them would cause any method named `add` to be invoked by `+`, decoupling RFC-011's `T: Add` from
  operators.
- **Layer 1 introduces nothing new**: `method_bindings` already looks up entries by the
  `Type.method` key (the fallback path after field lookup fails); operators can simply take the same
  path.
- **Layer 2 is the gate**: Before allowing an operator, the compiler **must** verify that the type
  implements the corresponding interface (see §"Why Implementing an Interface Is Mandatory").

### Layer 0: Fixed Mapping Table

| Operator            | Interface                                                 | Method     | First batch |
| ------------------- | --------------------------------------------------------- | ---------- | ----------- |
| `+`                 | `Add`                                                     | `add`      | ✅          |
| `-`                 | `Subtract`                                                | `subtract` | ✅          |
| `*`                 | `Multiply`                                                | `multiply` | ✅          |
| `/`                 | `Divide`                                                  | `divide`   | ✅          |
| `%`                 | `Modulo`                                                  | `modulo`   | ✅          |
| `==` `!=`           | `Equal`                                                   | `equal`    | ✅          |
| `[]`                | `Index`                                                   | `index`    | ✅          |
| `?`                 | `Try`                                                     | `residual` | ✅          |
| `<` `<=` `>` `>=`   | — (reserved as native instructions)                       | —          | ❌          |
| `and` `or`          | — (short-circuit is language semantics, not overloadable) | —          | ❌          |
| 5 bitwise operators | —                                                         | —          | ❌          |
| Unary `-` `!`       | —                                                         | —          | ❌          |

**Interface names use full words rather than abbreviations** (`Multiply` rather than `Mul`):
consistent with the `T: Add + Multiply + Zero` in the main text of RFC-011, **without modifying
already-accepted RFCs**.

**`%` adopts `Modulo` semantics** (mathematical modulus, result sign follows divisor) rather than
the current remainder behavior (`-7 % 3` currently returns `-1`). This is a **deliberate semantic
correction** — the name pins the semantics to mathematical modulus.

### Layer 2: Interface Definitions

#### Arithmetic Interfaces

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

`R` is the right-operand type, allowing left/right heterogeneous types (e.g., `Int + Float`). The
return value is fixed as `Self` (`&Self` borrows the receiver; see RFC-009 borrow tokens and
RFC-011a receiver convention).

#### Equality Interface

```yaoxiang
Equal: (Self: Type, R: Type) -> Type = {
    equal: (self: &Self, other: &R) -> Bool
}
```

**`Equal` implicitly requires `Dup`**: the semantics of `==` is "comparing two values," which
requires both sides to be readable. A linear (non-`Dup`) value can only be read once, so it cannot
participate in comparison. RFC-009 already provides the causal chain:

> **The causal relationship cannot be reversed: freezing is the cause, Dup is the result.**

Therefore, the prerequisite for instantiating `Equal` is that `Self` has the `Dup` attribute;
otherwise, the compiler rejects the `Equal` instantiation.

#### Index Interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` as a type parameter rather than an associated type member**: the open question in RFC-011a
was settled by adopting "associated types implemented through generics interface parameters"
(`Iterator: (Item: Type) -> Type` is an isomorphic precedent), and no `type` member syntax needs to
be introduced.

**Multi-position indexing is handled via tuple packing + overloading**, without introducing variadic
interfaces:

```yaoxiang
// One-dimensional container
List(T) instantiates Index(List(T), Int, T)                 → arr[0]

// Multi-dimensional container
Grid    instantiates Index(Grid, Tuple(Int, Int), Float)     → g[0, 1]
//                    └─ Key is a tuple

// Two instantiation signatures differ → coexist per RFC-011a overloading rules
```

#### Propagation Interface

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    residual: (self: &Self) -> E
}
```

`T` (success type) and `E` (error type) are both interface type parameters.

**Typing rules for `?`**:

```yaoxiang
// f: () -> Result(T, E)
// The rest of the function body expects Result(U, E) (error type must be consistent)
x = f()?        // x: T, equivalent to:
                //   match f() {
                //       ok(v)  => v
                //       err(e) => return err(e)
                //   }
```

A compile error is reported when error types are inconsistent (corresponding to the existing
`E1083`); `E1081` is reported when the outer function's return type is not a propagatable type; the
wording of `E1082` is updated (no longer referring to the specific name "Result") when an expression
does not implement `Try`.

### Examples

#### User-Defined Type Arithmetic and Equality

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

#### User-Defined Container Indexing

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

#### Moving `Result` to std (Design Goal)

After `Result` implements `Try`, `?` no longer depends on hardcoded type names:

```yaoxiang
// In std.result (no longer needs core special-casing)
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

#### Generics Constraints Finally Deliverable

```yaoxiang
// The example from RFC-011 becomes implementable
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

### Syntax Changes

**No new syntax, no new keywords**. All capabilities are composed from existing mechanisms:

| Capability              | Reused Existing Mechanism                                   |
| ----------------------- | ----------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                            |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                   |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)         |
| Associated type         | RFC-011 §3.1 (interface type parameters)                    |
| Multi-position indexing | Existing tuple-packing parsing + RFC-011a overloading rules |
| Operator precedence     | **Language-fixed**, not user-customizable                   |

## Detailed Design

### Impact on the Type System

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` `Try`.

**Reuse already-registered trait**: `Equal` is already registered in `trait_data.rs` but has **zero
consumer sites** (`==` currently only performs `unify` and then hardcodes a return of `Bool`). This
RFC connects it as the actual contract for `==`.

**`Dup` prerequisite constraint**: when instantiating `Equal`, check whether `Self` is `Dup`. `Dup`
/ `Linear` attributes are inferred per RFC-009 (`&T` freeze ⇒ Dup; `&mut T` exclusive ⇒ Linear).

**Constraint solving**: solving `T: Add` = checking whether `T` has instantiated the `Add`
interface. The mechanism reuses RFC-011a's compile-time type collection, with no new solver
required.

### Runtime Behavior

**Zero runtime overhead**: the call target for an operator is determined at compile time (static
dispatch). For primitive types (`Int` / `Float` / `String` / `List`), the **native instruction fast
path** is preserved and does not go through interface dispatch.

The runtime behavior of `?` remains unchanged (unpack + early return); only the judgment criterion
changes from "type name == 'Result'" to "type implements `Try`."

### Compiler Changes

| Component                            | Change                                                                                                                                          |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | The whitelist in `infer_binary` is replaced with a dual path of "native fast path + interface query"; add interface queries for `Index` / `Try` |
| Same as above (`Try` arm)            | Remove the hardcoded `MonoType::make_result`; replace with `Try` instantiation query                                                            |
| `middle/core/ir_gen.rs`              | `Expr::Try`: remove the hardcoded `group: "Result"`; `Expr::Index`: add interface dispatch branch                                               |
| `typecheck/types/trait_data.rs`      | Register default instantiations for the 8 interfaces (for `Int` / `Float` / `String` / `List`, etc.)                                            |
| New                                  | Layer 0 mapping table constant + operator-to-interface query function                                                                           |
| Diagnostics                          | `E1082` wording changes from "requires Result" to "requires Try" (`E1081` / `E1083` are kept)                                                   |

### Backward Compatibility

**Primitive type operations are unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, `1 < 2` all retain
their native paths, with the same behavior and performance.

**`%` semantic change**: `-7 % 3` changes from `-1` (remainder) to `2` (modulo). This is a
**deliberate correction** and should be noted in the migration guide; existing test corpora have no
dependencies on negative-number `%` (verified).

**`f[0]` position binding is unchanged**: `distance[0]` is a compiler built-in capability from
RFC-004 and **does not go through the `Index` interface**, so it is unaffected.

**`?` is transparent to existing code**: after `Result` adds the `Try` instantiation, the behavior
of existing `?` usages is completely identical.

## Tradeoffs

### Advantages

- **Delivers on RFC-011**: `T: Add + Multiply + Zero` transitions from paper to deliverable, without
  modifying the main text of the already-accepted RFC
- **Removes the core binding for `Result`**: after `?` is interface-based, `Result` can move to std,
  supporting the layering goal
- **Fixes a verified defect**: `Point == Point` becomes usable, and also fixes `list.contains` being
  unusable on structs
- **Zero new syntax**: all mechanisms from RFC-011a are reused, without touching parser grammar
  rules (complying with the zero-grammar-change principle of RFC-036)
- **Zero runtime overhead**: static dispatch + native fast path for primitive types
- **User-defined containers are usable**: `Box(T)[0]` transitions from "always errors" to "usable"

### Disadvantages

- **`%` semantic change is a breaking change**: despite the real semantic rationale, a migration
  note is still required
- **`Equal` implicitly requiring `Dup` rejects some types**: linear-type values cannot be `==`,
  which is the cost of semantic correctness, but may surprise users, so clear diagnostics are
  required
- **Interface instantiation is an explicit cost**: each operator requires writing one interface
  instantiation line + one method. RFC-011a's syntax makes implicit derivation impossible (the
  trade-off is no magic with the `Self` type parameter)

## Alternatives

### Option A: Only Do Layer 1 (dispatch by method name), Without the Interface Layer

`+` only checks whether a method named `add` exists, without requiring the `Add` interface to be
implemented.

**Reason for rejection**: RFC-011's `T: Add` constraint would become decoupled from operators — the
constraint checks the interface, while the operator checks the method name, and the two could yield
inconsistent answers. It would also be impossible to provide accurate compile-time diagnostics for
"`+` used on non-addable types."

### Option B: Introduce Constructor Syntax `Ok(x)` / `Some(x)` to Solve the `?` Problem

Don't make `?` interface-based; instead, add constructor syntax for sum types.

**Reason for rejection**: Conflicts with RFC-010 (already accepted). RFC-010 explicitly states that
"sum types are uniformly expressed using record types, **no two syntaxes are needed**" and
explicitly deprecates the `|` syntax. Introducing constructors would introduce a second expression.
Moreover, it only solves `?`, not `Point == Point` or user-defined container indexing.

### Option C: Also Make Comparison Operators Part of the First Batch (Introducing `Ordering`)

`<` `<=` `>` `>=` go through a `Compare` interface, returning the three-valued `Ordering`.

**Reason for rejection**: `<` is already a **first-class IR instruction** in YaoXiang
(`Instruction::Lt/Le/Gt/Ge`); interface-ization would force primitive types to take a detour.
Moreover, introducing `Ordering` brings along a whole set of issues including `Ordering`'s own
comparison/sorting and the `PartialOrd` vs `Ord` distinction for float `NaN`, which warrants an
independent RFC. The needs exposed by current verification (`Point == Point`, `list.contains`)
**only require `Equal`**.

### Option D: Use Abbreviated Names for Operators (the `Add` interface has method `add`, but the interface is called `Mul`)

**Reason for rejection**: The main text of RFC-011 already reads `T: Add + Multiply + Zero`; using
abbreviations would require modifying the already-accepted RFC.

## Implementation Strategy

### Dependencies

| Dependency                             | Status        | Effect on This RFC                                                                                           |
| -------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------ |
| RFC-011a interface mechanism Phase 1–3 | ✅ Landed     | Foundation of Layer 2 (verified runnable)                                                                    |
| RFC-010 record-style construction path | ❌ Not landed | **Construction side of `?`** (without being able to construct `Result` values, `Try` cannot be fully tested) |
| RFC-039 variant deconstruction         | ❌ Paper      | `match` syntax in `Result.residual`                                                                          |
| RFC-009 `Dup` / `Linear` inference     | Partial       | Prerequisite constraint check for `Equal`                                                                    |

### Phasing

Grouped by **interface dependencies** (a design constraint, not a schedule):

**Phase 1 — Does not depend on RFC-010 / 039**:

- Layer 0 mapping table + Layer 1 dispatch wiring
- Seven interfaces: `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
- `Equal` ⇒ `Dup` prerequisite constraint
- Primitive type native fast path preserved
- **Benefit**: `Point + Point`, `Point == Point`, `Box(T)[0]` all become usable

**Phase 2 — Depends on RFC-010 / RFC-039**:

- `Try` interface + interface-based `?`
- Move `Result` from core to std
- **Blocker reason**: requires being able to construct `Result` values (RFC-010) and deconstruct
  them (RFC-039)

### Risks

| Risk                                                                      | Mitigation                                                                                                     |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| The `%` semantic change breaks existing user code                         | Migration note + verified that the test corpus has no negative `%` dependencies                                |
| The explicit cost of interface instantiation annoys users                 | Diagnostics indicate "must implement interface X" and provide an instantiation example                         |
| Inconsistency between dual paths (native + interface) for primitive types | Gate: although `Int` et al. implement the interface, their method semantics must match the native instructions |
| Layer 2 gate causes existing code to fail to compile                      | `Equal` currently has zero consumer sites, so wiring it up only affects usages that were already erroring      |

## Coordination With Other RFCs

This RFC is positioned as a **consumer-side requirements originator**, and the following RFCs must
be updated synchronously to maintain coordinated consistency (revisions authorized):

| RFC                    | Content to be updated                                                                                               |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **RFC-011** (accepted) | In the §Constraints section, note that `Add` / `Multiply` etc. are defined and implemented by RFC-011b              |
| **RFC-010** (accepted) | Clarify the landing requirement of "record fields act as constructors" — it is the prerequisite for Phase 2 of `?`  |
| **RFC-039** (draft)    | Construction and deconstruction must come in pairs; handling of exhaustiveness checks against the `std` variant set |
| **RFC-009** (accepted) | Cross-reference `Equal` ⇒ `Dup` prerequisite in §Type Attributes                                                    |

## Open Questions

- [ ] Does the semantic correction of `Modulo` need a compatibility period? (@Chenxu: leaning toward
      a direct change, since the current behavior is a bug, not a feature)
- [ ] Should `Index` distinguish mutable indexing (similar to Rust's `IndexMut`)? (@Chenxu:
      read-only for the first batch; mutable indexing involves RFC-009's `WriteToken` and is left
      for later)
- [ ] Should multi-position indexing keys be packed as tuples (current approach) or changed to
      multiple parameters (Swift-style)? (@Chenxu: stick with the current tuple approach to avoid
      modifying the parser)
- [ ] Can users **add operator implementations to existing types**? (e.g., adding a custom `Add` to
      `List`) — involves the orphan rule

---

## Appendix A: Investigation Evidence

All the following verifications were reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Interface Mechanism Usability (Foundation of This RFC)

| Capability                                                                     | Verification                                               |
| ------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Definable                                               |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runnable, correct output                                |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (collision between field and method namespaces) |
| Method dispatch `d.speak()`                                                    | ✅ Runnable                                                |

**Note**: `E1097` means that operator methods must use the **external declaration** form
(`Point.add: (self: &Point, ...)`), consistent with the examples in RFC-011a.

### A.2 Current State of Operators

| Expression                          | Current State                                                                |
| ----------------------------------- | ---------------------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]` | ✅ Hardcoded whitelist (Int/Float/String/List)                               |
| `Point(1,2) == Point(1,2)`          | ❌ `E6007` (Eq does not hold on Struct)                                      |
| `f[0]` (function position binding)  | ⚠️ Only legal within binding declarations; as an expression, reports `E3006` |
| `arr[0, 1]` (multi-position)        | ✅ Tuple packing, `list([1, 2])`                                             |
| `-7 % 3`                            | `-1` (remainder, not modulo)                                                 |

### A.3 Hardcoded Locations of `?`

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Both locations need to be changed to interface queries.

## Appendix B: Design Decision Record

| Decision                                   | Choice                                                                        | Rationale                                                                                                                          | Date       |
| ------------------------------------------ | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Architecture                               | Three-layer separation (mapping / dispatch / interface contract)              | Merging them would decouple RFC-011's constraints from operators                                                                   | 2026-09-15 |
| Operator allowance condition               | Must implement the interface (Layer 2 is the gate)                            | Ensures strict correspondence between `T: Add` and `+`                                                                             | 2026-09-15 |
| `Equal` and `Dup`                          | Hardcode the `Dup` prerequisite inside the `Equal` definition                 | RFC-009 causal chain: freezing is the cause, Dup is the result                                                                     | 2026-09-15 |
| Interface naming                           | Full words (`Multiply` rather than `Mul`)                                     | Do not modify the already-accepted main text of RFC-011                                                                            | 2026-09-15 |
| `%` interface name                         | `Modulo` (mathematical modulus)                                               | The name pins the semantics; the current remainder behavior is a defect                                                            | 2026-09-15 |
| Comparison operators                       | **Not** interface-based in the first batch; native IR instructions retained   | Already first-class instructions; `Ordering` brings along a set of independent issues                                              | 2026-09-15 |
| `Ordering`                                 | Not introduced in the first batch                                             | No real demand; warrants an independent RFC                                                                                        | 2026-09-15 |
| `?` interface name                         | `Try`, with method `residual`                                                 | Do not pre-assume unification with a future ternary sugar, to avoid over-design                                                    | 2026-09-15 |
| Associated type                            | Use interface type parameters, no `type` member syntax introduced             | RFC-011a settled this approach (`Iterator: (Item: Type)`)                                                                          | 2026-09-15 |
| Unification of method binding and indexing | Conceptually unified; the interface only governs container indexing (Layer A) | Position-binding keys are compile-time constants; the result type requires type family evaluation, which is not user-implementable | 2026-09-15 |
| Multi-position indexing                    | Tuple packing + overloading by Key type; no variadic interface                | Do not modify the parser; coexistence via RFC-011a overloading rules                                                               | 2026-09-15 |
| Bitwise / unary operators                  | Not in the first batch                                                        | Rare for user-defined types; YAGNI                                                                                                 | 2026-09-15 |

## Appendix C: Glossary

| Term             | Definition                                                                                                              |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Layer 0          | Fixed mapping table from operator to method name; a language-level constant, not user-modifiable                        |
| Layer 1          | Dispatch foundation that looks up and invokes by the `Type.method` key                                                  |
| Layer 2          | Interface contract layer that provides the basis for generics constraints, and also serves as the operator gate         |
| Native fast path | The hardcoded operation path retained for primitive types (Int/Float/String/List), which does not go through interfaces |
| Position binding | The `f[0]` syntax from RFC-004 that binds a function parameter position as a method; a compile-time behavior            |

## References

- [RFC-011: Generic Type System Design](./011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraints, associated type
- [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
  interface declaration/instantiation/overloading rules
- [RFC-009: Ownership Model Design](./009-ownership-model.md) — `Dup` / `Linear`
- [RFC-004: Multi-Position Union Binding for Curried Methods](./004-curry-multi-position-binding.md)
  — `f[0]` syntax
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — sum type expression
- [RFC-039: Pattern Matching Completeness](../draft/039-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — associated type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — multi-parameter subscripts
