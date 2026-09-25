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
> - [RFC-004: Multi-Position Union Binding for Curried Methods](./004-curry-multi-position-binding.md)
>   — `f[0]` positional binding syntax
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — Sum type = record type whose
>   fields all return themselves
> - [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` being
>   relocated to std, E108x
> - [RFC-010b: Pattern Matching Completeness (Variant Deconstruction and Exhaustiveness)](../draft/010b-pattern-matching-completeness.md)
>   — Variant deconstruction (dependency)

## Summary

This RFC adds **operator overloading** to YaoXiang, enabling `a + b` / `a == b` / `a[i]` / `e?` to
be implemented by user-defined types, and makes the **operator constraints** (`T: Add + Multiply`)
in the constraint syntax already written by RFC-011 move from a **paper capability** to a concrete
mechanism (`Zero` in the same clause is not an operator; see Open Questions).

The design adopts **three-layer separation of concerns**: a fixed operator-to-method mapping table
(Layer 0), a name-based dispatch foundation (Layer 1, reusing the existing `method_bindings`), and
an interface contract layer (Layer 2, used for generic constraints). The **precedence and
associativity of operators remain language-fixed**; users only overload semantics.

First-batch scope: seven interfaces `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`.
Arithmetic interfaces use **three type parameters** `(Self, R, O)`—the result type `O` is made
explicit, so heterogeneous operations like `1 + 2.5` (returns `Float`) and `point * 2.0` (scaling)
can be expressed. `Equal` is **automatically derived by default** (records whose fields are all
comparable automatically receive field-by-field `==`), and explicit instantiation can override this.

`Try` (interface-ification of `?`) is moved entirely to Phase 2: it depends on RFC-010
(construction) and RFC-010b (deconstruction) being landed, and the interface shape is not yet
finalized (see Open Questions).

**No new syntax, no new keywords**—everything reuses the existing mechanisms from RFC-011a
(interface declaration / instantiation / external method declaration / overloading).

## Motivation

### Why This Feature Is Needed

#### 1. RFC-011's Core Example Depends on It, and the Current State Is Worse Than "Cannot Deliver"

RFC-011 (accepted) uses operator names as type constraints in 8 places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

None of the RFCs ever define where `Add` / `Multiply` come from or how `+` binds to them. Testing
confirms the current state is even worse: `T: Add` **fails to compile today**—the constraint solver
queries the old trait table (`trait_data.rs`), which doesn't contain `Add`. The constraint names
`Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` are similarly dangling (see "Coordination with Other
RFCs" for this RFC's handling).

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

Consequence: `list.contains(list_of_structs, p)` is **completely unusable**—it internally depends on
`==`. Meanwhile, `(1, 2) == (1, 2)` and `[1] == [1]` go through runtime element-wise comparison and
**have always worked**—structs are the only gap.

#### 3. `?` Hardcodes the Type Name into the Compiler, Blocking `Result` from Being Moved to std

The implementation of `?` simultaneously hardcodes the type name and variant index:

```rust
// typecheck: hardcoded Result type construction
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hardcoded group name and variant index
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` were made **interface-driven**, any type implementing
that interface (including user-defined types) could be used with `?`, and `Result` could belong to
std (RFC-013 already establishes "std library `Result(T, Error)`" as its location).

Note this is only half the problem: the `ok(...)` / `err(...)` / `some(...)` construction syntax is
currently also hardcoded in the parser (language specification §1.4.2 lists them as "constructors
recognized by the parser"). "Moving `Result` to std" requires **both halves—`?` and
constructors—solved together**, both under Phase 2 of this RFC.

#### 4. User-Defined Containers Cannot Be Indexed

The type checking for `Index` is a hardcoded whitelist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// Others → "Type-layer unrecognized containers are refused, not silently accepted"
```

Any container type defined by the user will unconditionally error on `c[0]`.

#### 5. The Implementation of `%` Violates Published Documentation

Testing shows `-7 % 3` returns `-1` (truncated remainder). But the operator precedence table in the
language reference (`reference/index.md`) already states "`* / %`
multiplication/division/**modulo**"—the documentation promises modulo, while the implementation
gives remainder. This is not a design change, but an **implementation defect that violates the
documentation**, which this RFC also fixes.

### Existing Half-Built Foundations

Investigation shows that **most of the mechanism is already in place**; what's missing is the
wiring:

| Mechanism                                                           | Location                                                                                                                | Status                                                                                               |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Interface declaration `(Self: Type) -> Type`                        | RFC-011a Phase 1                                                                                                        | ✅ Runnable                                                                                          |
| Interface instantiation `Animal(Dog)` + external method declaration | RFC-011a Phase 2–3                                                                                                      | ✅ Runnable (with unit tests)                                                                        |
| Method dispatch `method_bindings["Type.method"]`                    | `expressions.rs` (registered in `environment.rs`, queried in `expressions.rs` call resolution and field fallback paths) | ✅ Runnable                                                                                          |
| Associated types (= interface type parameters)                      | RFC-011 §3.1; RFC-011a adopted this approach                                                                            | ✅ Mechanism decided                                                                                 |
| Type family evaluation `AssociatedTypeDef`                          | `dependent_types.rs`                                                                                                    | ✅ Production use case `std.assert`'s `IsTrue`                                                       |
| `Equal` / `Dup` / `Clone` / `Debug` traits                          | Registered in `trait_data.rs`                                                                                           | ⚠️ `Equal` has zero consumers; old auto-derivation only registered signatures without implementation |
| Operator → method name mapping                                      | —                                                                                                                       | ❌ Does not exist                                                                                    |

**Conclusion**: This is not building a feature from scratch, but wiring existing components together
and documenting them.

## Proposal

### Core Design: Three-Layer Separation of Concerns

```
Layer 0  Fixed mapping table (language-level constant, not user-modifiable)
         + → add    == → equal    [] → index    ? → residual
         │  Built into the compiler, doesn't participate in type inference, not exposed to users
         ▼
Layer 1  Dispatch foundation (look up method by name and invoke)        ← Reuses existing mechanism
         method_bindings["Point.add"]
         ▼
Layer 2  Interface contract (for generic constraints)
         Add / Equal / Index / Try …
         Makes T: Add constraints work, and serves as the gate for operators
```

**Reasoning for layering**:

- **Layers 0 and 1 make operators "usable"**, Layer 2 makes operators "constrainable". Merging them
  would cause any method named `add` to be called by `+`, decoupling RFC-011's `T: Add` from
  operators.
- **Layer 1 is not newly built**: `method_bindings` already looks up by `Type.method` keys (the
  fallback path after field lookup fails), so operators can go through the same path.
- **Layer 2 is the gate**: Before an operator is allowed, it **must** confirm that the type
  implements the corresponding interface (see the `Equal` auto-derivation exception in
  §Equal—derivation and registration use the same criterion).

### Names and Registration: Two Independent Channels

**Operator queries are against the "interface implementation registry", not through normal name
resolution.**

- The registry belongs to the core and aggregates two registration sources: the core's default
  registrations for primitive types (`Add(Int, Int, Int)`, etc.), and the interface instantiations
  written by users in type bodies (`Add(Point, Point, Point)`). Regardless of which module the
  implementation code physically resides in, registrations flow into the same table; `+` / `==` /
  `[]` only see this table.
- A **same-named binding** locally defined in a module (such as RFC-011 §5.2's type-level Peano
  addition `Add: (A: Type, B: Type) -> Type = match ...`) goes through the **name lookup channel**,
  only affecting the resolution of the name `Add` within that module, **cannot reach the registry,
  and does not affect operator usability**. Same name, different thing, no shadowing.

This also answers "Does RFC-011's type-level `Add` conflict with this RFC's interface `Add`?": No
conflict. Type-level `Add` is purely a type-level computation (Zero/Succ are types, not values;
there will never be value operations like `Zero + Succ(...)`), and the operator interface at the
value level is a different layer sharing the same name.

### Layer 0: Fixed Mapping Table

| Operator            | Interface                                                 | Method            | First Batch |
| ------------------- | --------------------------------------------------------- | ----------------- | ----------- |
| `+`                 | `Add`                                                     | `add`             | ✅          |
| `-`                 | `Subtract`                                                | `subtract`        | ✅          |
| `*`                 | `Multiply`                                                | `multiply`        | ✅          |
| `/`                 | `Divide`                                                  | `divide`          | ✅          |
| `%`                 | `Modulo`                                                  | `modulo`          | ✅          |
| `==` `!=`           | `Equal`                                                   | `equal`           | ✅          |
| `[]`                | `Index`                                                   | `index`           | ✅          |
| `?`                 | `Try` (four methods, Phase 2 finalized)                   | `is_failure` etc. | ✅ Landed   |
| `<` `<=` `>` `>=`   | — (reserved as native instructions)                       | —                 | ❌          |
| `and` `or`          | — (short-circuit is language semantics, not overloadable) | —                 | ❌          |
| 5 bitwise operators | —                                                         | —                 | ❌          |
| Unary `-` `!`       | —                                                         | —                 | ❌          |

**Interface names use full spellings rather than abbreviations** (`Multiply` instead of `Mul`): to
stay consistent with RFC-011's main text `T: Add + Multiply + Zero`, and **without modifying the
already-accepted RFC**.

**`%` adopts `Modulo` semantics** (mathematical modulo, result sign follows the divisor). The
motivation section has already confirmed that the current remainder implementation violates the
published documentation (`reference/index.md` "multiplication/division/modulo"); this item is
treated as a defect fix, with no compatibility period.

Status note: The current type checking whitelist for `%` only covers Int/Float (different from `+`'s
whitelist of Int/Float/String/List); see Appendix A.2 for test records.

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

Each of the three type parameters has a distinct role:

- `Self`: the left operand type (receiver, borrowed as `&Self`; see RFC-009 borrow tokens and
  RFC-011a receiver conventions);
- `R`: the right operand type—**left and right may be heterogeneous**;
- `O`: **the result type**, declared explicitly.

**Why the result type must be an explicit parameter** (rather than hardcoding `-> Self`):

1. Hardcoding `Self` would force `Add(Int, Float)`'s method to return `Int`, making it impossible to
   correctly express `1 + 2.5`;
2. Once the result type is explicit, the promotion type family in RFC-011 §8.3
   `Add: (A, B) -> Type = match (A, B) { (Int, Float) => Float, ... }` and this RFC's interface
   registration **become two views of the same table**—the core registration
   `Add(Int, Float, Float)` is exactly the row `(Int, Float) => Float`, and each user instantiation
   adds a row to this table;
3. The `Index` interface already has three parameters `(Self, Key, Value)`, with the return type
   `Value` as a parameter—after arithmetic interfaces add `O`, the entire operator interface family
   has a uniform shape, and hardcoding `Self` would be the odd one out.

**Core default registrations** (native instruction path, not via method calls):
`Add(Int, Int, Int)`, `Add(Int, Float, Float)`, `Add(Float, Int, Float)`,
`Add(Float, Float, Float)`, `Add(String, String, String)` (concatenation),
`Add(List(T), List(T), List(T))` (element concatenation), etc.; the same applies to the five
arithmetic interfaces for primitive types. `1 + 2.5` will change from the current compile error (the
whitelist requires both sides to be the same type) to a valid operation that returns `3.5: Float`.

**Constraint syntax sugar**: `T: Add` ≜ already-registered `Add(T, T, T)`—same-type
self-composition, the result is still that type. In RFC-011's matrix multiplication example,
`a * b + c` has type `T` throughout, which is exactly what this means. `T: Equal` similarly ≜
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

#### Equality Interface (Automatically Derived by Default)

```yaoxiang
Equal: (Self: Type, R: Type) -> Type = {
    equal: (self: &Self, other: &R) -> Bool
}
```

**`Equal` is automatically derived by default**, with five rules:

1. **Default derivation**: When defining a record type, if all fields are comparable (primitive
   types, `String`, or otherwise comparable records/tuples/lists, and contains no `&mut` field), the
   compiler automatically generates **field-by-field comparison** for `==`, going through a native
   code path, not generating a user-visible method. User-defined types are naturally comparable,
   with no ceremony required.
2. **Explicit override**: If `Equal(Point, Point)` is written in the type body and a `Point.equal`
   method is provided, the user's version is used (e.g., float comparison with tolerance), and
   automatic derivation is not performed.
3. **When fields are not comparable**: Automatic derivation fails, `==` is unavailable, and the
   diagnostic indicates which field is the cause; the user can still manually write `Equal` to
   customize comparison (e.g., comparing fields containing functions by name).
4. **Precondition**: Types containing `&mut T` linear tokens (recursive fields) do not participate
   in comparison—linear tokens are consumed upon a single read and cannot be used to extract two
   values simultaneously for comparison. Note the premise is "non-linear" rather than "Dup":
   primitive value types (Int/Float/Bool/Char) per RFC-011 §2.4 do not belong to Dup (they are
   compiler-built-in value copying); if Dup were the premise, `Point { x: Float, y: Float }` would
   be erroneously rejected.
5. **Constraint same criterion**: The resolution of `T: Equal` and the gating of `==` follow the
   same "check registry or structural derivation" rule—constraints and operators always give the
   same answer.

**Consistency basis**: `(1, 2) == (1, 2)` and `[1] == [1]` already go through runtime element-wise
comparison today (the comparison whitelist in `executor.rs` includes Tuple/List/Array), and record
types are the only composite type excluded. Automatic derivation is not a new silent default, but
rather fills in the last piece of existing language behavior.

**Old mechanism retirement**: The old auto-derivation of `Equal` in `trait_data.rs` (which only
registered signatures, had no implementation code, and zero consumers) is discontinued; all of
`Equal`'s determination (primitive type default registration, structural derivation, explicit
instantiation) goes through the interface registry. The old trait table retains Clone/Dup/Debug's
existing responsibilities (names don't overlap with operator interfaces; future unification is to be
discussed separately).

#### Index Interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` as a type parameter rather than an associated type member**: The open question in RFC-011a
has been resolved—"associated types are implemented via generic interface parameters"
(`Iterator: (Item: Type) -> Type` is an isomorphic precedent), so no `type` member syntax needs to
be introduced.

**Ownership note**: `index` returns a complete `Value` from a `&Self` borrow. The standard library's
`list.get: (&Vec(A), Int) -> A` is already in the same form, and this interface **is treated the
same as the current std**; the exact semantics of "extracting a complete value from a borrow" for
move-semantic element types will be handled uniformly when RFC-009 is fully enforced, and this RFC
does not invent new rules for this.

**Multi-position indexing relies on tuple packing + overloading**, without introducing variadic
interfaces:

```yaoxiang
// One-dimensional container
List(T) instantiates Index(List(T), Int, T)                   → arr[0]

// Multi-dimensional container
Grid    instantiates Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └─ Key is a tuple

// Two instantiation signatures differ → they coexist
```

**Same-type same-name interfaces allow multiple instantiations**, distinguished by the signature of
the injected method, following RFC-011a's method-level overloading rules. RFC-011a's overloading
only explicitly extends to the method level; this rule adds an instantiation-level rule on top: the
legality of same-name interface instantiation coexistence is determined by whether the expanded
method signatures conflict (conflict is E1097, with the same source as field/method namespace
rules).

#### Propagation Interface Try (Phase 2 Finalized and Implemented)

The interface-ified name for `?` is `Try`, with a four-method shape (Phase 2 finalized on
2026-09-22):

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    is_failure: (self: &Self) -> Bool,
    success:    (self: &Self) -> T,
    residual:   (self: &Self) -> E,
    from_error: (E) -> Self,
}
```

- **Semantic division of labor**: `is_failure` determines success/failure, `success` extracts the
  success payload, `residual` extracts the failure payload, and `from_error` serves as a bridge for
  cross-type propagation (reconstructing the failure value from `E` when `T` of `f()?` differs from
  the outer `U`). The lowering of `?` uniformly generates a four-method call chain—when
  `is_failure(t)` is true, `Ret from_error(residual(t))`; otherwise the expression value is
  `success(t)`; no longer handwritten variant check sequences, `Result` / `Option` (std yx
  implementation) and user-defined Try types follow the same path.
- **Dead-end branches**: The failure arm of `success` and the success arm of `residual` are
  contractually unreachable; implementations use `assert(false)` to diverge (`assert` returns
  `Never`, and the explosion principle `Never <: T` allows it, type-system.md §2.2).
- **Checking**: typecheck queries the interface implementation registry (Self position nominal
  matching, abstract entries instantiated by scrutinee actual arguments); the outer function's
  return type must also implement `Try` and the `E` position must catch the failure value
  (E1081/E1082/E1083 semantics become interface-driven).
- **`Result` belongs to std**: The type definitions and Try implementations of `Result` / `Option`
  are migrated to `std/result.yx` / `std/option.yx` (pure YaoXiang), the native `ok`/`err`
  constructors are retired—variant construction syntax `Result(T, E).ok(v)` is the only construction
  channel (the problem of hardcoded constructors is solved along with the retirement of parser
  special cases). The Try residual type of Option takes `Void` (corresponding to the NoneT semantics
  of Rust's Try experiment).

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

#### Generic Constraints Finally Work

```yaoxiang
// The example from RFC-011 can now be implemented
// T: Add ≜ Add(T, T, T) is registered; T: Multiply ≜ Multiply(T, T, T) is registered
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

(`Zero` / `One` in RFC-011's signature example `T: Add + Multiply + Zero` are not within this RFC's
scope—they are constant members, not operators, and "interface members without a receiver" have no
precedent in RFC-011a; see Open Questions.)

### Syntax Changes

**No new syntax, no new keywords**. All capabilities are composed from existing mechanisms:

| Capability              | Reused Existing Mechanism                                              |
| ----------------------- | ---------------------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                                       |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                              |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)                    |
| Associated types        | RFC-011 §3.1 (interface type parameters)                               |
| Multi-position indexing | Existing tuple packing parsing + instantiation-level overloading rules |
| Operator precedence     | **Language-fixed**, not user-customizable                              |

## Detailed Design

### Type System Impact

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` (`Try`
in Phase 2).

**Interface implementation registry**: The central judgment point for operators. It aggregates core
default registrations (primitive types) and user instantiations (`ImplementationProof` produced by
`check_interface_instantiation`); the gating check of `+` / `==` / `[]` and the resolution of
`T: Add` constraints (`check_trait_bounds` in `bounds.rs`) **query the same table**—there is no gap
where "constraint says yes, operator says no".

**Structural derivation of `Equal`**: A structural rule (all fields comparable ⇒ comparable) is
layered on top of the registry, with primitive types covered by core registrations, closing
recursively. The old `Equal` registration and old auto-derivation in `trait_data.rs` are
discontinued.

**Separation of operator queries from name resolution**: Operators like `+` only query the registry,
not normal name resolution; local same-name bindings (such as type-level `Add` families) do not
affect operators (see §Names and Registration).

**Orphan rules**: Operator implementations can only be written in the **module that defines the
type**—`Int` is defined in core, so the registration for `Int` can only be written in core; `Point`
is defined in the user module, so only its definer can register it. All types follow the same rule,
with no special privileges for built-in types.

### Runtime Behavior

**Zero runtime overhead**: The call target for operators is determined at compile time (static
dispatch). For primitive types (`Int`/`Float`/`String`/`List`), the **native instruction fast path**
is retained, without going through interface dispatch; auto-derived struct `==` generates native
field-by-field comparison; `Any` (dynamic type) position `==` / `!=` maintains the existing runtime
comparison (RFC-036 testing framework `assert_eq` depends on this behavior, unaffected).

**`%` semantic fix**: `-7 % 3` changes from `-1` (truncated remainder) to `2` (mathematical modulo).
The interpreter (`checked_rem`), constant folding (`a % b`), and bytecode (`I64_REM`) are all
modified synchronously.

### Compiler Changes

| Component                            | Changes                                                                                                                                                                                                                                             |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | `infer_binary` whitelist changes to "native fast path + registry query" dual paths; `Expr::Index` whitelist adds interface query branch                                                                                                             |
| `typecheck/inference/bounds.rs`      | `check_trait_bounds` for operator interface names changes to query interface registry (`Equal` includes structural derivation)                                                                                                                      |
| `typecheck/checker.rs`               | New `Equal` structural derivation (after record definition); instantiation-level overloading rules (same-name interface multiple instantiations coexist by method signature)                                                                        |
| `middle/core/ir_gen.rs`              | `Expr::Index` adds method call dispatch branch; structs without explicit `equal` generate native field-by-field comparison for `==`; `Mod` instruction changes to mathematical modulo                                                               |
| Interface registry (new)             | Layer 0 mapping table constants + operator→interface query function + core default registrations (`Int`/`Float`/`String`/`List`, etc.)                                                                                                              |
| `trait_data.rs`                      | `Equal` registration and old auto-derivation discontinued (`Clone`/`Dup`/`Debug` maintain status quo)                                                                                                                                               |
| Diagnostics                          | `Equal` precondition (linear token) dissatisfaction reuses `E1101` (type does not implement interface) family; `E1081`/`E1082` text removes "Result" wording (Phase 2 with `Try` implementation, synchronized in three parties per RFC-013 process) |

### Backward Compatibility

**Primitive type operations unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, `1 < 2` all retain their
native paths, with behavior and performance unchanged.

**`1 + 2.5` changes from compile error to valid**: After core registration `Add(Int, Float, Float)`,
mixed arithmetic previously rejected by the whitelist becomes available, returning `Float`. This is
a new capability; no existing code is affected.

**`%` semantic fix**: `-7 % 3` changes from `-1` to `2`. This is classified as a **defect fix** (the
documentation had long promised modulo, see motivation §5), with no compatibility period; existing
test corpus has no cases depending on negative number `%` (verified).

**Struct `==` changes from runtime error to usable**: Previously `Struct == Struct` always gave
E6007 runtime error; after wiring, automatic derivation makes it work—going from broken to working,
with no existing legal code affected.

**`Any`'s `==` unaffected**: Dynamic type positions maintain runtime comparison (RFC-036's
`assert_eq` assertion family depends on this; previously available, still available afterwards).

**`f[0]` positional binding unchanged**: `distance[0]` is a compiler built-in capability from
RFC-004, **does not go through the `Index` interface**, and is unaffected.

**`?` is transparent to existing code** (Phase 2): After `Result` gets a `Try` instantiation,
existing `?` usages behave exactly the same.

## Trade-offs

### Advantages

- **Fulfills RFC-011's operator constraints**: `T: Add + Multiply` changes from paper (actually a
  compile error) to implementable, without modifying the already-accepted RFC main text
- **Releases `Result` from core binding** (Phase 2): After `?` and constructors are
  interface-driven, `Result` can belong to std, aligning with RFC-013's existing positioning
- **Fixes verified defects**: `Point == Point` works out of the box, also fixing `list.contains`'s
  unavailability for structs
- **Zero new syntax**: Everything reuses RFC-011a's existing mechanisms, not touching parser grammar
  rules
- **Zero runtime overhead**: Static dispatch + primitive type native fast path
- **User-defined containers are usable**: `Box(T)[0]` changes from "always errors" to usable
- **Internal language consistency**: Tuple/List already have element-wise comparison, record types
  are filled in; arithmetic interfaces' three parameters match Index's shape; RFC-011 §8.3 promotion
  table unifies with interface registry

### Disadvantages

- **`Equal` auto-derivation is a silent default**: If we want to retract "records are comparable by
  default" in the future, it will be a breaking change. This stance is accepted—consistent with
  Tuple/List's existing behavior, and explicit instantiation can always override
- **`Equal` precondition rejects types containing linear tokens**: Types containing `&mut` fields
  cannot `==`, which is the cost of semantic correctness, requiring clear diagnostics (indicating
  which field)
- **Interface instantiation is explicit cost**: For arithmetic operators, each one requires writing
  a line of instantiation + a method (`Equal` is exempt—auto-derived). RFC-011a's syntax precludes
  implicit derivation (in exchange for no magic with the `Self` type parameter)
- **`%` semantic fix is a behavior change**: Although classified as a defect fix, it still needs to
  be noted in the migration documentation

## Alternative Approaches

### Option A: Only Do Layer 1 (Method Name Dispatch), No Interface Layer

`+` only checks if there's a method named `add`, without requiring implementation of the `Add`
interface.

**Reason for rejection**: RFC-011's `T: Add` constraint will decouple from operators—the constraint
checks interfaces, the operator checks method names, and the two may give inconsistent answers.
Moreover, it is impossible to provide accurate diagnostics at compile time for "`+` used on
unaddable types".

### Option B: Introduce Constructor Syntax `Ok(x)` / `Some(x)` to Solve the `?` Problem

Instead of interface-ifying `?`, add constructor syntax to sum types.

**Reason for rejection**: Conflicts with RFC-010 (already accepted). RFC-010 explicitly states "use
record types uniformly to express sum types, **no need for two syntaxes**", and explicitly
deprecates the `|` syntax. Introducing constructors is introducing a second set of expressions.
Moreover, it only solves `?`, not `Point == Point` or custom container indexing.

### Option C: Interface-ify Comparison Operators in the First Batch (Introducing `Ordering`)

`<` `<=` `>` `>=` go through the `Compare` interface, returning a three-valued `Ordering`.

**Reason for rejection**: `<` in YaoXiang is already a **first-class IR instruction**
(`Instruction::Lt/Le/Gt/Ge`); interface-ification would force primitive types to take a detour.
Moreover, introducing `Ordering` would bring out a whole set of issues around `Ordering`'s own
comparison/sorting, float `NaN`'s `PartialOrd` vs `Ord`, etc., which is the scope of an independent
RFC. Currently, the needs exposed by testing (`Point == Point`, `list.contains`) **only require
`Equal`**.

### Option D: Use Abbreviations for Operator Names (e.g., the `Add` interface's method is `add`, the interface is `Mul`)

**Reason for rejection**: RFC-011's main text already writes `T: Add + Multiply + Zero`; using
abbreviations would require modifying the already-accepted RFC.

### Option E: `Equal` Only Does Explicit Instantiation, No Auto-Derivation

**Reason for rejection**: User-defined types cannot be `==`, an unacceptable user experience;
moreover, `(1, 2) == (1, 2)` and `[1] == [1]` are already element-wise comparisons today, with only
record types excluded, which is inherently inconsistent. Explicit instantiation is retained as an
override means, balancing customization needs.

### Option F: Hardcode Arithmetic Interface Return Type as `-> Self`

**Reason for rejection**: `Add(Int, Float)`'s method would be forced to return `Int`, making it
impossible to correctly express `1 + 2.5`; vector scaling `Multiply(Point, Float)` cannot be written
for the same reason. Moreover, the promotion type family `(Int, Float) => Float` in RFC-011 §8.3
would lose its landing spot. The three-parameter `(Self, R, O)` form is consistent with
`Index(Key, Value)`.

## Implementation Strategy

### Dependencies

| Dependency                             | Status             | Impact on This RFC                                                                |
| -------------------------------------- | ------------------ | --------------------------------------------------------------------------------- |
| RFC-011a interface mechanism Phase 1–3 | ✅ Implemented     | Layer 2 foundation (verified runnable)                                            |
| RFC-010 record-style construction path | ❌ Not implemented | **Phase 2**: `?`'s construction side + constructor parser special case retirement |
| RFC-010b variant deconstruction        | ❌ On paper        | **Phase 2**: `Result.residual`'s `match` writing, exhaustiveness                  |
| RFC-009 linear token inference         | Partial            | `Equal`'s precondition check                                                      |

### Phasing

Divided into two groups by **interface dependencies** (design constraints, not scheduling):

**Phase 1 — Does not depend on RFC-010/010b**:

- Layer 0 mapping table + interface implementation registry + Layer 1 dispatch wiring
- `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` seven interfaces
  (three-type-parameter form)
- `Equal` auto-derivation + linear precondition + constraint resolution redirected to registry
- `%` semantic fix (including interpreter/constant folding/bytecode three places)
- `Any` position `==` maintains runtime comparison
- **Benefits**: `Point + Point`, `Point == Point` (no ceremony), `Box(T)[0]`, `1 + 2.5`, `T: Add`
  constraints all available

**Phase 2 — Depends on RFC-010 / RFC-010b, `Try` shape must be finalized before work begins**:

- `Try` interface shape finalized (see Open Questions)
- `?` interface-ification + constructors (`ok`/`err`/`some`) parser special case retirement,
  switched to RFC-010 record construction path
- `Result` migrated from core to std (per RFC-013's existing positioning)

### Risks

| Risk                                                                         | Mitigation                                                                                                    |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `%` semantic change affects existing user code                               | Classified as defect fix + verified test corpus has no negative number `%` dependency                         |
| `Equal` auto-derivation silent default will be hard to retract in the future | Position decided: consistent with Tuple/List's existing behavior, explicit instantiation can override         |
| Primitive type dual paths (native + registry) inconsistency                  | Gate: core registration's semantics must be consistent with native instructions (two views of the same table) |
| Registry queries slow down compilation                                       | Table indexed by type name + instantiation result cache (reusing RFC-011a proof)                              |
| Instantiation-level overloading introduces ambiguity                         | Same rules as method-level overloading: signature conflict is E1097                                           |

## Coordination with Other RFCs

This RFC's position is that of a **consumer-side requirements proposer**; the following RFCs need to
be updated synchronously to ensure coordinated consistency (authorized to revise):

| RFC                                    | Content to Update                                                                                                                                                                                                                                                                        |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **RFC-011** (accepted)                 | Note in §Constraints that `Add` / `Multiply` are defined and implemented by 011b (`T: Add` ≜ `Add(T, T, T)`); mark `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` as dangling constraint names (pending subsequent RFC); note §8.3 promotion type family unifies with interface registry |
| **RFC-010** (accepted)                 | Clarify the landing requirement of "record fields are constructors"—it is the prerequisite for Phase 2 constructor parser special case retirement                                                                                                                                        |
| **RFC-010b** (draft, formerly RFC-039) | Construction and deconstruction must come in pairs; exhaustiveness judgment does not depend on `Result`'s core/std belonging (migrated by 011b Phase 2)                                                                                                                                  |
| **RFC-009** (accepted)                 | Cross-reference in §Type Properties: `Equal` precondition is "contains no `&mut` linear token" (not Dup)                                                                                                                                                                                 |
| **RFC-013** (accepted)                 | Upon Phase 2 implementation: `E1081` / `E1082` text removes "Result" wording; `Equal` precondition diagnostics reuses `E1101` family—three-party synchronization per RFC-013 process (codes/*.rs ↔ locales ↔ code table)                                                                 |
| **RFC-018** (accepted)                 | After `%` changes to mathematical modulo, the `Mod → srem/urem` mapping table becomes invalid; needs to change to `srem` + sign correction (or `sdiv`+`mul`+`sub` synthesis), and correct the "modulo/remainder" terminology mixing                                                      |
| **RFC-036** (accepted)                 | No changes needed. Note the relationship: `Any` position `==` maintains runtime comparison, `assert_eq` assertion family is not affected by the gate                                                                                                                                     |

> Basis for `Result` belonging to std: RFC-013 has positioned "std library `Result(T, Error)`", and
> RFC-014's layering places std in the core source. Phase 2 of this RFC is one of its landing paths;
> no other number is cited.

## Open Questions

- [x] ~~Does `Modulo`'s semantic fix need a compatibility period?~~ → **Closed**: The language
      reference has long stated "multiplication/division/modulo"; the current remainder
      implementation violates published documentation, treated as a defect fix, with no
      compatibility period (2026-09-22)
- [x] ~~Can users add operator implementations for existing types (orphan rules)?~~ → **Finalized**:
      Operator implementations can only be written in the module that defines the type. `Int` is
      defined in core, so only core can register it; users can only register for their own types.
      All types are treated equally, with no special privileges for built-in types (2026-09-22)
- [ ] Should `Index` distinguish mutable indexing (similar to Rust's `IndexMut`)? (First batch
      read-only; mutable indexing involves RFC-009's `WriteToken`, and RFC-011a already has the
      `&mut Self` receiver precedent to follow, left for later)
- [ ] Multi-position indexing's Key uses tuple packing (current state) or changes to multiple
      parameters (Swift style)? (Continuing with tuple packing, not changing the parser)
- [ ] Form of `Zero` / `One`: constant members are not operators; "interface members without a
      receiver" have no precedent in RFC-011a; they need to be finalized separately before RFC-011's
      full sentence `T: Add + Multiply + Zero` can be fulfilled
- [ ] Complete shape of the `Try` interface: `?` needs three things—success/failure determination,
      success payload extraction, failure path producing the outer return value—a single `residual`
      method is insufficient; along with the retirement of constructor parser special cases,
      finalized before Phase 2 work begins
- [ ] `?T` prefix type (RFC-026 FFI nullable annotation, RFC-018) and `e?` suffix operator share the
      `?` symbol: different positions (type position vs expression position) do not constitute a
      conflict; a written explanation suffices
- [ ] `PartialOrd` / `Ordering` (interface-ification of comparison operators): independent RFC, this
      RFC explicitly does not do it

---

## Appendix A: Research Evidence

The following tests were all reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Interface Mechanism Availability (Foundation of This RFC)

| Capability                                                                     | Testing Result                                                |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Definable                                                  |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runnable, output correct (unit test in `tests/rfc011a.rs`) |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (field and method share namespace conflict)        |
| Method dispatch `d.speak()`                                                    | ✅ Runnable                                                   |

**Note**: `E1097` means that operator methods must use the **external declaration** form
(`Point.add: (self: &Point, ...)`), consistent with the example in RFC-011a. The registration of
`method_bindings` is in `environment.rs` (`add_method_binding`), and the query is in the call target
resolution and field lookup failure fallback paths in `expressions.rs`.

### A.2 Current State of Operators

| Expression                           | Current State                                                                               |
| ------------------------------------ | ------------------------------------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]`  | ✅ Hardcoded whitelist (Int/Float/String/List, **requires both sides to be the same type**) |
| `1 + 2.5` (Int + Float)              | ❌ Compile error (whitelist requires both sides to be the same type)                        |
| `-7 % 3`                             | `-1` (truncated remainder; `%` whitelist is only Int/Float, different from `+`'s whitelist) |
| `Point(1,2) == Point(1,2)`           | ❌ `E6007` (Eq fails at runtime on Struct)                                                  |
| `(1,2) == (1,2)` / `[1] == [1]`      | ✅ Runtime element-wise comparison (Struct is the only gap)                                 |
| Any position `a == b` (`assert_eq`)  | ✅ Runtime comparison (RFC-036 verified)                                                    |
| `f[0]` (function positional binding) | ⚠️ Only valid within binding declaration, as expression reports `E3006`                     |
| `arr[0, 1]` (multi-position)         | ✅ Tuple packing, `list([1, 2])`                                                            |

### A.3 Hardcoded Locations of `?` and Constructors

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Constructor side: `ok(T)` / `err(E)` / `some(T)` are recognized by the parser (language
specification `syntax.md` §1.4.2); `std/result.rs`'s `is_ok` / `unwrap` etc. match by
`variant_id 0/1` pattern. "Result to std" requires both halves (`?` + constructors) solved together,
both under Phase 2.

## Appendix B: Design Decision Records

| Decision                                | Decision                                                                                                        | Reason                                                                                                                                                   | Date       |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Architecture                            | Three-layer separation (mapping table / dispatch foundation / interface contract)                               | Merging would decouple RFC-011 constraints from operators                                                                                                | 2026-09-15 |
| Operator gating condition               | Must implement interface (Layer 2 as gate)                                                                      | Ensure `T: Add` strictly corresponds to `+`                                                                                                              | 2026-09-15 |
| Interface naming                        | Full spelling (`Multiply` instead of `Mul`)                                                                     | Don't change RFC-011's accepted main text                                                                                                                | 2026-09-15 |
| `%` interface name                      | `Modulo` (mathematical modulo)                                                                                  | Name pins down semantics                                                                                                                                 | 2026-09-15 |
| Comparison operators                    | First batch **not** interface-ified, keep native IR instructions                                                | Already first-class instructions; `Ordering` brings out a whole set of independent issues                                                                | 2026-09-15 |
| `Ordering`                              | Not introduced in the first batch                                                                               | No real demand driving it, the scope of an independent RFC                                                                                               | 2026-09-15 |
| Associated types                        | Use interface type parameters, no `type` member syntax                                                          | RFC-011a has already finalized this approach (`Iterator: (Item: Type)`)                                                                                  | 2026-09-15 |
| Method binding and indexing unification | Conceptually unified, interface only handles container indexing                                                 | Positional binding's key is a compile-time constant, the result type requires type family evaluation, which cannot be user-implemented                   | 2026-09-15 |
| Multi-position indexing                 | Tuple packing + overloading by Key type, not variadic interfaces                                                | Don't change the parser                                                                                                                                  | 2026-09-15 |
| Bitwise / unary operators               | Not done in the first batch                                                                                     | Rare for custom types, YAGNI                                                                                                                             | 2026-09-15 |
| Arithmetic interface shape              | Three type parameters `(Self, R, O)`; `T: Add` ≜ `Add(T, T, T)`                                                 | Explicit result type: `1 + 2.5`, scaling can be expressed; RFC-011 §8.3 promotion table unifies with registry; consistent with `Index(Key, Value)` shape | 2026-09-22 |
| `Equal` derivation                      | Default auto-derivation + explicit instantiation override; constraint resolution same criterion                 | "Types I wrote myself cannot be ==" unacceptable; Tuple/List already have element-wise comparison, Struct is the only gap                                | 2026-09-22 |
| `Equal` precondition                    | "Contains no `&mut` linear token", **not Dup**                                                                  | RFC-011 §2.4 explicitly states primitives don't belong to Dup; using Dup as premise would erroneously reject `Point{Float,Float}`                        | 2026-09-22 |
| Name and registration separation        | Operators only query interface implementation registry, not name resolution                                     | Local same-name bindings (type-level `Add` family, etc.) don't interfere with operators; RFC-011 §5.2 example needs no changes                           | 2026-09-22 |
| Orphan rules                            | Implementation follows the module that defines the type                                                         | All types treated equally, no special privileges for built-in types                                                                                      | 2026-09-22 |
| `Any`'s `==`                            | Maintain runtime comparison, don't query registry                                                               | RFC-036 `assert_eq` assertion family already depends on this behavior                                                                                    | 2026-09-22 |
| `Try` shape                             | Finalized: four methods (is_failure/success/residual/from_error) + assert-Never dead-end semantics; implemented | `?` needs three things, single-method `residual` is insufficient; constructor parser special cases retire along with Result moving to std                | 2026-09-22 |
| `%` semantic classification             | Defect fix (documentation already promised modulo), no compatibility period                                     | `reference/index.md` "multiplication/division/modulo" is the prior evidence                                                                              | 2026-09-22 |
| `Result` to std basis                   | Cite RFC-013's existing positioning, no longer cite non-existent numbers                                        | RFC-013 already writes "std library `Result(T, Error)`"                                                                                                  | 2026-09-22 |

## Appendix C: Glossary

| Term                              | Definition                                                                                                                                                                      |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layer 0                           | The fixed mapping table from operators to method names, a language-level constant, not user-modifiable                                                                          |
| Layer 1                           | The dispatch foundation that looks up and calls by `Type.method` keys                                                                                                           |
| Layer 2                           | The interface contract layer, providing the basis for generic constraints while also being the gate for operators                                                               |
| Interface implementation registry | The total table aggregating core default registrations and user instantiations; the sole criterion for operator queries and constraint resolution, unrelated to name resolution |
| Native fast path                  | The hardcoded computation path retained for primitive types, without going through interface dispatch                                                                           |
| Positional binding                | RFC-004's `f[0]` syntax, which binds function parameter positions as methods, a compile-time behavior                                                                           |
| Auto-derivation                   | The compiler-generated field-by-field `==` for records whose fields are all comparable; explicit instantiation can override                                                     |

## References

- [RFC-011: Generic Type System Design](./011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraints, associated types
- [RFC-011a: Interface Implementation and Dynamic Dispatch](./011a-interface-implementation.md) —
  Interface declaration/instantiation/overloading rules
- [RFC-009: Ownership Model Design](./009-ownership-model.md) — `&mut T` linear token
- [RFC-004: Multi-Position Union Binding for Curried Methods](./004-curry-multi-position-binding.md)
  — `f[0]` syntax
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — Sum type expression
- [RFC-013: Error Code Specification](./013-error-code-specification.md) — `Result` belonging to std
  positioning, error code process
- [RFC-010b: Pattern Matching Completeness (Variant Deconstruction and Exhaustiveness)](../draft/010b-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — Associated type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — Multi-parameter subscripts
