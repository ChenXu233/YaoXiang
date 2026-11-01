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
> - [RFC-011: Generic Type System Design](011-generic-type-system.md) — type constraints
>   `T: Add + Multiply`, associated types
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](011a-interface-implementation.md) —
>   interface declaration/instantiation mechanisms
> - [RFC-009: Ownership Model Design](009-ownership-model.md) — `&mut T` linear tokens
> - [RFC-004: Multi-Position Union Binding for Curried Methods](004-curry-multi-position-binding.md)
>   — `f[0]` positional binding syntax
> - [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — sum types = records whose fields
>   all return the type itself
> - [RFC-013: Error Code Specification](013-error-code-specification.md) — `Result` placement in
>   std, E108x
> - [RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)](010b-pattern-matching-completeness.md)
>   — variant destructuring (dependency)

## Summary

Add **operator overloading** to YaoXiang so that `a + b` / `a == b` / `a[i]` / `e?` can be
implemented by user-defined types, and turn the **operator constraints** in the constraint syntax
already introduced by RFC-011 (`T: Add + Multiply`) from a **capability on paper** into an
implementable mechanism (`Zero` in the same expression is not an operator; see Open Questions).

The design uses **separation of responsibilities across three layers**: a fixed operator-to-method
mapping table (Layer 0), a name-based dispatch substrate (Layer 1, reusing the existing
`method_bindings`), and an interface contract layer (Layer 2, used for generic constraints). An
operator's **precedence and associativity remain fixed by the language**; users overload only its
semantics.

Initial scope: the seven interfaces `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`.
Arithmetic interfaces use **three type parameters** `(Self, R, O)`—the result type `O` is explicit,
allowing heterogeneous operations such as `1 + 2.5` (returning `Float`) and `point * 2.0` (scaling)
to be expressed. `Equal` is **automatically derived by default** (a record whose fields are all
comparable automatically receives a field-wise `==`), while explicit instantiation can override the
derived implementation.

`Try` (the interface-based form of `?` ) is moved entirely to Phase 2: it depends on the
implementation of RFC-010 (construction) and RFC-010b (destructuring), and its interface shape has
not yet been finalized (see Open Questions).

**No new syntax and no new keywords** are introduced. Everything reuses the existing mechanisms from
RFC-011a (interface declaration / instantiation / external method declaration / overloading).

## Motivation

### Why This Feature Is Needed

#### 1. RFC-011's Core Example Depends on It, and the Current State Is Worse Than “Unable to Fulfill It”

RFC-011 (accepted) uses operator names as type constraints in eight places:

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

No RFC in the entire set defines where `Add` / `Multiply` come from or how `+` is bound to them.
Testing confirms that the current state is worse: `T: Add` **fails to compile today**—constraint
solving queries the old trait table (`trait_data.rs`), which contains no `Add`. The constraint names
`Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` are likewise unresolved (see “Coordination with Other
RFCs” for how this RFC handles them).

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

A cascading consequence is that `list.contains(list_of_structs, p)` is **completely unusable**—it
depends internally on `==`. In contrast, `(1, 2) == (1, 2)` and `[1] == [1]` have **always been
available** through runtime element-wise comparison—structs are the only missing case.

#### 3. `?` Hardcodes Type Names into the Compiler, Blocking the Move of `Result` to std

The implementation of `?` hardcodes both type names and variant indices:

```rust
// typecheck: hardcodes construction of the Result type
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen: hardcodes the group name and variant indices
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

This forces `Result` to remain in core. If `?` becomes **interface-driven**, any type implementing
that interface—including user-defined types—can use `?`, allowing `Result` to belong to std (RFC-013
already establishes the placement of “std library `Result(T, Error)`”).

Note that this is only half the problem: the `ok(...)` / `err(...)` / `some(...)` constructor forms
are currently hardcoded into the parser as well (the language specification §1.4.2 lists them as
“constructors recognized by the parser”). Moving “Result to std” requires solving **both halves** of
`?` and the constructors, both of which are assigned to Phase 2 of this RFC.

#### 4. User-Defined Containers Cannot Be Indexed

The type checking for `Index` is a hardcoded allowlist:

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// Otherwise → "reject containers unrecognized by the type system rather than silently accepting them"
```

For any user-defined container type, `c[0]` always produces an error.

#### 5. The Implementation of `%` Violates the Published Documentation

Testing shows that `-7 % 3` returns `-1` (a truncated remainder). However, the operator precedence
table in the language reference (`reference/index.md`) already states that "`* / %` perform
multiplication, division, and **modulo**"—the documentation promises modulo, while the
implementation provides remainder. This is not a design change but an **implementation defect that
violates the documentation**, which this RFC corrects as a side effect.

### Existing Foundations

Research shows that **most of the mechanism already exists**; what is missing is the wiring:

| Mechanism                                                           | Location                                                                                                                       | Status                                                                                                                   |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| Interface declaration `(Self: Type) -> Type`                        | RFC-011a Phase 1                                                                                                               | ✅ Runnable                                                                                                              |
| Interface instantiation `Animal(Dog)` + external method declaration | RFC-011a Phase 2–3                                                                                                             | ✅ Runnable (with unit tests)                                                                                            |
| Method dispatch `method_bindings["Type.method"]`                    | `expressions.rs` (registered in `environment.rs`, queried by the call-resolution and field-fallback paths in `expressions.rs`) | ✅ Runnable                                                                                                              |
| Associated types (= interface type parameters)                      | RFC-011 §3.1; RFC-011a adopted this design                                                                                     | ✅ Mechanism established                                                                                                 |
| Type-family evaluation `AssociatedTypeDef`                          | `dependent_types.rs`                                                                                                           | ✅ Production use case: `IsTrue` in `std.assert`                                                                         |
| `Equal` / `Dup` / `Clone` / `Debug` traits                          | Registered in `trait_data.rs`                                                                                                  | ⚠️ `Equal` has zero consumption sites; the old automatic derivation only registers a signature without an implementation |
| Operator-to-method-name mapping                                     | —                                                                                                                              | ❌ Does not exist                                                                                                        |

**Conclusion**: This is not a feature built from scratch, but an act of wiring together and
documenting existing components.

## Proposal

### Core Design: Separation of Responsibilities Across Three Layers

```
Layer 0  固定映射表（语言级常量，用户不可改）
         + → add    == → equal    [] → index    ? → residual
         │  编译期内置，不参与类型推导，不暴露给用户
         ▼
Layer 1  派发基座（按名查方法并调用）        ← 复用既有机制
         method_bindings["Point.add"]
         ▼
Layer 2  接口契约（泛型约束用）
         Add / Equal / Index / Try …
         使 T: Add 约束成立，且作为运算符的放行条件
```

**Rationale for the layers**:

- **Layers 0 and 1 make operators “usable,” while Layer 2 makes them “usable as constraints.”**
  Combining them would cause every method named `add` to be called by `+`, disconnecting RFC-011's
  `T: Add` constraint from the operator.
- **Layer 1 introduces nothing new**: `method_bindings` already looks up methods by the
  `Type.method` key on the fallback path after field lookup fails, so operators can use the same
  path.
- **Layer 2 is the release condition**: before an operator is permitted, the compiler **must**
  confirm that the type implements the corresponding interface (see the automatic-derivation
  exception for `Equal`—derivation and registration use the same criterion).

### Names and Registration: Two Independent Channels

**Operators query the “interface implementation registry”; they do not pass through ordinary name
resolution.**

- The registry is owned by the core and aggregates registrations from two places: the core's default
  registrations for primitive types (`Add(Int, Int, Int)` and similar) and interface instantiations
  written by users inside type bodies (`Add(Point, Point, Point)`). Regardless of which module
  physically contains the implementation code, registrations flow into the same table, and `+` /
  `==` / `[]` consult only this table.
- A local module that defines a **same-name binding** (for example, the Peano type-level addition in
  RFC-011 §5.2, `Add: (A: Type, B: Type) -> Type = match ...`) uses the **name-lookup channel**. It
  affects only resolution of the name `Add` within that module, **cannot reach the registry, and
  does not affect operator availability**. The two names are different entities and do not shadow
  each other.

This also answers whether RFC-011's type-level `Add` conflicts with this RFC's interface `Add`: it
does not. Type-level `Add` is purely a type-level computation (`Zero`/`Succ` are types, not values,
so there is never a value operation such as `Zero + Succ(...)`), whereas the value-level operator
interface is the same name at a different layer.

### Layer 0: Fixed Mapping Table

| Operator               | Interface                                                           | Method             | Initial Scope  |
| ---------------------- | ------------------------------------------------------------------- | ------------------ | -------------- |
| `+`                    | `Add`                                                               | `add`              | ✅             |
| `-`                    | `Subtract`                                                          | `subtract`         | ✅             |
| `*`                    | `Multiply`                                                          | `multiply`         | ✅             |
| `/`                    | `Divide`                                                            | `divide`           | ✅             |
| `%`                    | `Modulo`                                                            | `modulo`           | ✅             |
| `==` `!=`              | `Equal`                                                             | `equal`            | ✅             |
| `[]`                   | `Index`                                                             | `index`            | ✅             |
| `?`                    | `Try` (four methods; finalized in Phase 2)                          | `is_failure`, etc. | ✅ Implemented |
| `<` `<=` `>` `>=`      | — (native instructions retained)                                    | —                  | ❌             |
| `and` `or`             | — (short-circuiting is language semantics and cannot be overloaded) | —                  | ❌             |
| Five bitwise operators | —                                                                   | —                  | ❌             |
| Unary `-` `!`          | —                                                                   | —                  | ❌             |

**Interface names use full words rather than abbreviations** (`Multiply`, not `Mul`): this remains
consistent with `T: Add + Multiply + Zero` in the body of RFC-011 and **does not modify an accepted
RFC**.

**`%` uses `Modulo` semantics** (mathematical modulo, with the result's sign following the divisor).
The Motivation section established that the current remainder implementation violates the published
documentation (`reference/index.md`, “multiplication, division, and modulo”). This entry is
therefore treated as a defect correction, with no compatibility period.

Current-state note: the type-checking allowlist for `%` currently covers only Int/Float (and differs
from the Int/Float/String/List allowlist for `+`); Appendix A.2 documents the tested results.

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

The three type parameters each have a distinct responsibility:

- `Self`: the left operand type (the receiver, `&Self` borrow; see RFC-009's borrow tokens and
  RFC-011a's receiver conventions);
- `R`: the right operand type—**the two sides may have different types**;
- `O`: the **result type**, declared explicitly.

**Why the result type must be an explicit parameter** (rather than being fixed to `-> Self`):

1. If it were fixed to `Self`, the method for `Add(Int, Float)` would have to return `Int`, making
   `1 + 2.5` impossible to express correctly;
2. Once the result type is explicit, the promotion type family in RFC-011 §8.3
   `Add: (A, B) -> Type = match (A, B) { (Int, Float) => Float, ... }` and this RFC's interface
   registrations become **two views of the same table**—the core registration
   `Add(Int, Float, Float)` is precisely the `(Int, Float) => Float` row, and every user
   instantiation adds a row to the table;
3. The `Index` interface already has three parameters `(Self, Key, Value)`, with the return type
   `Value` supplied as a parameter—once arithmetic interfaces add `O`, the entire operator-interface
   family has a unified shape, making a fixed `Self` the outlier.

**Core default registrations** (native instruction paths; they do not go through method calls):
`Add(Int, Int, Int)`, `Add(Int, Float, Float)`, `Add(Float, Int, Float)`,
`Add(Float, Float, Float)`, `Add(String, String, String)` (concatenation),
`Add(List(T), List(T), List(T))` (element-wise concatenation), and so on for the five arithmetic
interfaces and primitive types. `1 + 2.5` changes from its current compilation error (the allowlist
requires both sides to have the same type) to a valid operation returning `3.5: Float`.

**Constraint syntax sugar**: `T: Add` ≜ a registered `Add(T, T, T)`—same-type self-composition whose
result remains that type. In RFC-011's matrix multiplication example, `a * b + c` has type `T`
throughout, which is exactly the intended meaning. Likewise, `T: Equal` ≜ `Equal(T, T)`.

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

**`Equal` is automatically derived by default**, under five rules:

1. **Default derivation**: when defining a record type, if all fields are comparable (primitive
   types, `String`, or recursively comparable records/tuples/lists, with no `&mut` fields), the
   compiler automatically generates a **field-by-field comparison** for `==`, using the native code
   path without creating a user-visible method. User-written types are therefore naturally
   comparable, with no ceremony required.
2. **Explicit override**: if the type body contains `Equal(Point, Point)` and provides a
   `Point.equal` method, the user's implementation is used (for example, floating-point comparison
   with a tolerance), and automatic derivation is not performed.
3. **When a field is not comparable**: automatic derivation fails, `==` is unavailable, and the
   diagnostic identifies the field responsible. The user may still write `Equal` manually to provide
   a custom comparison (for example, comparing function fields by name).
4. **Precondition**: types containing an `&mut T` linear token (including recursively through
   fields) do not participate in comparison—a linear token is consumed when read, so two values
   cannot be extracted simultaneously for comparison. The premise is “non-linear,” not “Dup”:
   primitive value types (Int/Float/Bool/Char) are not Dup under RFC-011 §2.4 (they use
   compiler-built-in value copying); using Dup as the premise would incorrectly reject
   `Point { x: Float, y: Float }`.
5. **Same criterion for constraints**: solving `T: Equal` and permitting `==` follow the same “look
   up the registration or derive structurally” rule—constraints and operators always produce the
   same answer.

**Rationale**: `(1, 2) == (1, 2)` and `[1] == [1]` already use runtime element-wise comparison today
(the comparison allowlist in `executor.rs` contains Tuple/List/Array); record types are the only
excluded composite type. Automatic derivation does not introduce a new silent default; it completes
the language's existing behavior.

**Retirement of the old mechanism**: the old automatic derivation of `Equal` in `trait_data.rs`
(which only registered a signature, provided no implementation, and had zero consumption sites) is
disabled. All `Equal` decisions (primitive-type default registrations, structural derivation, and
explicit instantiations) use the interface registry. The old trait table retains the existing
responsibilities of Clone/Dup/Debug (their names do not overlap with operator interfaces;
unification can be considered separately in the future).

#### Index Interface

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value` is a type parameter rather than an associated-type member**: the open question in RFC-011a
has been resolved in favor of implementing associated types through generic interface parameters
(`Iterator: (Item: Type) -> Type` is an isomorphic precedent), so there is no need to introduce
`type` member syntax.

**Ownership note**: `index` returns a complete `Value` from an `&Self` borrow. The standard
library's `list.get: (&Vec(A), Int) -> A` already has the same form, so this interface **receives
the same treatment as the current std implementation**. The precise semantics for “extracting a
complete value from a borrow” when the element type has move semantics will be handled uniformly
when RFC-009 is fully enforced; this RFC does not invent a new rule for it.

**Multi-position indexing relies on tuple packing and overloading**, without introducing a variadic
interface:

```yaoxiang
// One-dimensional container
List(T) 实例化 Index(List(T), Int, T)                   → arr[0]

// Multidimensional container
Grid    实例化 Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └─ Key is a tuple

// The two instantiation signatures differ → coexist
```

**Multiple instantiations of the same interface for the same type are permitted**. They coexist by
distinguishing them through the signatures of their injected methods and following RFC-011a's
method-level overloading rules. RFC-011a explicitly defines only method-level overloading; this
entry adds an instantiation-level rule: whether multiple instantiations of the same interface may
coexist is determined by whether their expanded method signatures conflict (a conflict produces
E1097, sharing the same origin as the field/method namespace rules).

#### Propagation Interface Try (Finalized and Implemented in Phase 2)

The interface name for the interface-based form of `?` is `Try`, with the following four-method
shape (finalized in Phase 2 on 2026-09-22):

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    is_failure: (self: &Self) -> Bool,
    success:    (self: &Self) -> T,
    residual:   (self: &Self) -> E,
    from_error: (E) -> Self,
}
```

- **Division of responsibilities**: `is_failure` determines success or failure, `success` retrieves
  the success payload, `residual` retrieves the failure payload, and `from_error` provides the
  bridge for cross-type propagation (when the `T` of `f()?` differs from the outer `U`, the failure
  value is reconstructed from `E`). Lowering for `?` uniformly generates a four-method call chain:
  if `is_failure(t)` is true, `Ret from_error(residual(t))`; otherwise, the expression's value is
  `success(t)`. It no longer handwrites a sequence of variant checks; `Result` / `Option` (the std
  yx implementation) and user-defined `Try` types follow the same path.
- **Dead branches**: the failure arm of `success` and the success arm of `residual` are unreachable
  by contract. The implementation uses `assert(false)` to diverge (`assert` returns `Never`, and the
  explosion principle `Never <: T` permits it, type-system.md §2.2).
- **Checking**: type checking queries the interface implementation registry (nominal matching for
  the Self position, with abstract entries instantiated using the actual scrutinee arguments); the
  enclosing function's return type must also implement `Try`, and the E position must be able to
  accept the failure value (E1081/E1082/E1083 semantics become interface-driven accordingly).
- **Moving `Result` to std**: the type definitions and `Try` implementations for `Result` / `Option`
  move into `std/result.yx` / `std/option.yx` (pure YaoXiang), and the native `ok`/`err`
  constructors are retired—the variant construction syntax `Result(T, E).ok(v)` becomes the only
  construction path (the hardcoded-constructor issue is resolved together with the parser
  special-case retirement). The residual type of `Option`'s `Try` is `Void` (corresponding to the
  NoneT semantics of Rust's Try experiments).

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
    println(a == b)             # false — Equal is automatically derived; no instantiation needed
    println(a == a)             # true
}
```

#### Custom Equality (Overriding Automatic Derivation)

```yaoxiang
Vec3: Type = {
    x: Float,
    y: Float,
    z: Float,
    Equal(Vec3, Vec3),
}

# Floating-point comparison with a tolerance, overriding field-by-field automatic derivation
Vec3.equal: (self: &Vec3, other: &Vec3) -> Bool =
    abs(self.x - other.x) < 0.000001
    and abs(self.y - other.y) < 0.000001
    and abs(self.z - other.z) < 0.000001
```

#### Indexing a Custom Container

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

#### Generic Constraints Can Finally Be Fulfilled

```yaoxiang
// The RFC-011 example can now be implemented
// T: Add ≜ Add(T, T, T) is registered; T: Multiply ≜ Multiply(T, T, T) is registered
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

(The `Zero` / `One` in RFC-011's signature example `T: Add + Multiply + Zero` are outside this RFC's
scope—they are constant members rather than operators, and RFC-011a provides no precedent for
“interface members without a receiver”; see Open Questions.)

### Syntax Changes

**No new syntax and no new keywords** are introduced. Every capability is composed from existing
mechanisms:

| Capability              | Existing Mechanism Reused                                             |
| ----------------------- | --------------------------------------------------------------------- |
| Interface declaration   | RFC-011a Phase 1                                                      |
| Interface instantiation | RFC-011a Phase 2 (`Dog: { Animal(Dog) }`)                             |
| Method implementation   | RFC-011a Phase 3 (external declaration `Point.add`)                   |
| Associated types        | RFC-011 §3.1 (interface type parameters)                              |
| Multi-position indexing | Existing tuple-packing parser + instantiation-level overloading rules |
| Operator precedence     | **Fixed by the language**; not open to user definition                |

## Detailed Design

### Type-System Impact

**New interfaces** (Layer 2): `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` (`Try`
in Phase 2).

**Interface implementation registry**: the central decision point for operators. It aggregates core
default registrations (primitive types) and user instantiations (the `ImplementationProof` produced
by `check_interface_instantiation`). Release checks for `+` / `==` / `[]` and constraint solving for
`T: Add` (`check_trait_bounds` in `bounds.rs`) **query the same table**—there is no gap in which
“the constraint says yes, but the operator says no.”

**Structural derivation for `Equal`**: a structural rule is layered on top of the registry (all
fields comparable ⇒ comparable), with primitive types covered by core registrations and the
recursion closed. The `Equal` registration and old automatic derivation in `trait_data.rs` are
disabled.

**Separation of operator lookup from name resolution**: operators such as `+` query only the
registry and do not use ordinary name resolution. A local same-name binding (such as the type-level
`Add` family) does not affect operators (see §Names and Registration).

**Orphan rule**: an operator implementation may be written only in **the module that defines the
type**—because `Int` is defined in the core, only the core may register `Int`; because `Point` is
defined in a user module, only its definer may register it. All types follow the same rule; built-in
types receive no privilege.

### Runtime Behavior

**Zero runtime overhead**: the compiler determines the call target for an operator at compile time
(static dispatch). Primitive types (`Int`/`Float`/`String`/`List`) retain **native instruction fast
paths** that bypass interface dispatch; automatically derived struct `==` generates native
field-by-field comparison; `==` / `!=` in `Any` (dynamic type) positions retain the existing runtime
comparison (the RFC-036 test framework's `assert_eq` depends on this behavior and is unaffected).

**Correction of `%` semantics**: `-7 % 3` changes from `-1` (truncated remainder) to `2`
(mathematical modulo). The interpreter (`checked_rem`), constant folding (`a % b`), and bytecode
(`I64_REM`) are modified in all three places.

### Compiler Changes

| Component                               | Change                                                                                                                                                                                                                                                                        |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs`    | The `infer_binary` allowlist becomes a dual path of “native fast path + registry lookup”; the `Expr::Index` allowlist gains an interface-query branch                                                                                                                         |
| `typecheck/inference/bounds.rs`         | `check_trait_bounds` queries the interface registry for operator interface names (`Equal` includes structural derivation)                                                                                                                                                     |
| `typecheck/checker.rs`                  | Adds `Equal` structural derivation (after record definition); instantiation-level overloading rules (multiple instantiations of the same interface coexist when method signatures permit it)                                                                                  |
| `middle/core/ir_gen.rs`                 | `Expr::Index` gains a method-call dispatch branch; struct `==` without an explicit `equal` generates native field-by-field comparison; the `Mod` instruction changes to mathematical modulo                                                                                   |
| Interface implementation registry (new) | Layer 0 mapping-table constants + operator-to-interface lookup function + core default registrations (`Int`/`Float`/`String`/`List`, etc.)                                                                                                                                    |
| `trait_data.rs`                         | Disables `Equal` registration and the old automatic derivation (`Clone`/`Dup`/`Debug` remain unchanged)                                                                                                                                                                       |
| Diagnostics                             | Failure to satisfy the `Equal` precondition (linear token) reuses the `E1101` (type does not implement interface) family; the text of `E1081`/`E1082` removes the word "Result" (implemented with `Try` in Phase 2, with three-way synchronization under the RFC-013 process) |

### Backward Compatibility

**Primitive-type operations are unchanged**: `1 + 2`, `"a" + "b"`, `[1] + [2]`, and `1 < 2` all
retain their native paths, with unchanged behavior and performance.

**`1 + 2.5` changes from a compilation error to valid code**: after the core registration
`Add(Int, Float, Float)`, mixed arithmetic that was previously rejected by the allowlist becomes
available and returns `Float`. This is a new capability and affects no existing code.

**Correction of `%` semantics**: `-7 % 3` changes from `-1` to `2`. It is classified as a **defect
correction** (the documentation has long promised modulo; see Motivation §5), with no compatibility
period. No existing test corpus depends on negative `%` (the corpus has been verified).

**Struct `==` changes from a runtime error to available functionality**: previously, every
`Struct == Struct` produced the runtime error E6007; after the wiring, automatic derivation makes it
available—an improvement from broken to working, with no impact on existing valid code.

**`Any` equality is unaffected**: dynamic-type positions retain runtime comparison (the RFC-036
`assert_eq` assertion family depends on it; it was available before and remains available
afterward).

**`f[0]` positional binding is unchanged**: `distance[0]` is a compiler built-in capability from
RFC-004. It **does not use the `Index` interface** and is unaffected.

**`?` is transparent to existing code** (Phase 2): after adding the `Try` instantiation to `Result`,
existing uses of `?` behave exactly as before.

## Trade-offs

### Advantages

- **Fulfills RFC-011's operator constraints**: `T: Add + Multiply` changes from a paper capability
  (in fact, a compilation error) into an implementable mechanism without modifying the body of the
  accepted RFC
- **Removes `Result`'s binding to core** (Phase 2): after `?` and the constructors become
  interface-driven, `Result` can move to std, aligning with RFC-013's existing placement
- **Fixes a tested defect**: `Point == Point` works out of the box, also fixing `list.contains` for
  structs
- **No new syntax**: everything reuses the existing mechanisms from RFC-011a, without touching
  parser syntax rules
- **Zero runtime overhead**: static dispatch + native fast paths for primitive types
- **User-defined containers become usable**: `Box(T)[0]` changes from “always an error” to available
- **Internal language consistency**: Tuple/List already use element-wise comparison, and record
  types complete the set; the three-parameter arithmetic interfaces have the same shape as Index;
  the promotion table in RFC-011 §8.3 and the interface registry are unified

### Disadvantages

- **Automatic derivation of `Equal` is a silent default**: in the future, withdrawing “records are
  comparable by default” would be a breaking change. This stance is accepted because it is
  consistent with the existing behavior of Tuple/List, and explicit instantiation can always
  override it
- **The `Equal` precondition rejects types containing linear tokens**: a type containing an `&mut`
  field cannot be `==`. This is the semantically correct cost, but it requires clear diagnostics
  identifying the responsible field
- **Interface instantiation has an explicit cost**: each arithmetic operator requires one line of
  instantiation and one method (`Equal` is exempt because it is automatically derived). RFC-011a's
  syntax makes implicit derivation impossible (in exchange, the `Self` type parameter has no magic
  behavior)
- **Correcting `%` semantics is a behavior change**: although classified as a defect correction, it
  must still be noted in migration documentation

## Alternatives

### Option A: Implement Only Layer 1 (Dispatch by Method Name), Without the Interface Layer

`+` checks only whether a method named `add` exists and does not require an `Add` interface.

**Reason for rejection**: RFC-011's `T: Add` constraint would become disconnected from the
operator—the constraint queries interfaces, while the operator queries method names, and the two may
produce inconsistent answers. It also prevents the compiler from providing an accurate diagnostic
when `+` is used with a non-addable type.

### Option B: Introduce Constructor Syntax `Ok(x)` / `Some(x)` to Solve the `?` Problem

Rather than making `?` interface-driven, add constructor syntax for sum types.

**Reason for rejection**: this conflicts with RFC-010 (accepted). RFC-010 explicitly states that
“sum types are uniformly expressed using record types, **without requiring two syntaxes**” and
explicitly deprecates `|` syntax. Introducing constructors introduces a second way to express the
same concept. It also solves only `?`, not `Point == Point` or custom-container indexing.

### Option C: Make Comparison Operators Interface-Driven in the Initial Scope (Introducing `Ordering`)

`<` `<=` `>` `>=` use a `Compare` interface that returns a three-valued `Ordering`.

**Reason for rejection**: `<` is already a **first-class IR instruction** in YaoXiang
(`Instruction::Lt/Le/Gt/Ge`), and making it interface-driven would force primitive types through an
unnecessary detour. Introducing `Ordering` would also bring a separate set of problems involving
comparison and sorting of `Ordering` itself, floating-point `NaN` and the distinction between
`PartialOrd` and `Ord`, and so on; that scope requires a separate RFC. The currently demonstrated
requirements (`Point == Point`, `list.contains`) need **only `Equal`**.

### Option D: Use Abbreviated Operator Names (the `Add` Interface's Method Is `add`, but the Interface Is `Mul`)

**Reason for rejection**: the body of RFC-011 already states `T: Add + Multiply + Zero`; using
abbreviations would require modifying an accepted RFC.

### Option E: Make `Equal` Explicit-Only, Without Automatic Derivation

**Reason for rejection**: it would be unacceptable for a user-written type not to support `==`;
moreover, `(1, 2) == (1, 2)` and `[1] == [1]` already use element-wise comparison today, with only
record types excluded, creating an inconsistency. Explicit instantiation remains available as an
override mechanism, accommodating custom requirements.

### Option F: Fix the Arithmetic Interface Return Type to `-> Self`

**Reason for rejection**: the method for `Add(Int, Float)` would be forced to return `Int`, making
`1 + 2.5` impossible to express correctly; vector scaling `Multiply(Point, Float)` could not be
expressed either. The promotion type family `(Int, Float) => Float` in RFC-011 §8.3 would also lose
its implementation site. The three parameters `(Self, R, O)` match the shape of `Index(Key, Value)`.

## Implementation Strategy

### Dependencies

| Dependency                              | Status         | Affected Part of This RFC                                                              |
| --------------------------------------- | -------------- | -------------------------------------------------------------------------------------- |
| RFC-011a interface mechanism, Phase 1–3 | ✅ Implemented | Foundation for Layer 2 (tested and runnable)                                           |
| RFC-010 record construction path        | ✅ Implemented | **Phase 2**: construction side of `?` + retirement of constructor parser special cases |
| RFC-010b variant destructuring          | ✅ Implemented | **Phase 2**: `match` form for `Result.residual`, exhaustiveness                        |
| RFC-009 linear-token inference          | Partial        | Precondition checks for `Equal`                                                        |

### Phased Implementation

The implementation is divided into two groups according to **interface dependencies** (design
constraints, not a schedule):

**Phase 1 — Independent of RFC-010/010b**:

- Layer 0 mapping table + interface implementation registry + Layer 1 dispatch wiring
- The seven interfaces `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
  (three-parameter form)
- `Equal` automatic derivation + linear-token precondition + constraint solving redirected to the
  registry
- Correction of `%` semantics (in the interpreter, constant folding, and bytecode)
- `Any`-position `==` retains runtime comparison
- **Benefit**: `Point + Point`, `Point == Point` (without ceremony), `Box(T)[0]`, `1 + 2.5`, and
  `T: Add` constraints all become available

**Phase 2 — Depends on RFC-010 / RFC-010b; the `Try` shape must be finalized before work begins**:

- Finalize the `Try` interface shape (see Open Questions)
- Make `?` interface-driven + retire parser special cases for constructors (`ok`/`err`/`some`),
  switching to RFC-010's record construction path
- Move `Result` from core to std (based on RFC-013's existing placement)

### Risks

| Risk                                                                                            | Mitigation                                                                                                              |
| ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| The `%` semantic change affects existing user code                                              | Classify it as a defect correction + verify that the test corpus has no dependency on negative `%`                      |
| The silent default for automatic `Equal` derivation will be difficult to withdraw in the future | Position established: consistent with the existing behavior of Tuple/List, with explicit instantiation able to override |
| The two paths for primitive types (native + registered) may diverge                             | Gate: the semantics of core registrations must match native instructions (two views of the same table)                  |
| Registry queries slow down compilation                                                          | Index the table by type name + cache instantiation results (reusing RFC-011a proofs)                                    |
| Instantiation-level overloading introduces ambiguity                                            | Same rule as method-level overloading: conflicting signatures produce E1097                                             |

## Coordination with Other RFCs

This RFC is the **requester of consumer-side functionality** and requires synchronized updates to
the following RFCs to maintain consistency (authorized revisions have been granted):

| RFC                                    | Required Updates                                                                                                                                                                                                                                                                                                     |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **RFC-011** (accepted)                 | In the constraints section, note that `Add` / `Multiply` are defined and implemented by 011b (`T: Add` ≜ `Add(T, T, T)`); mark `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` as unresolved constraint names (for a future RFC); in §8.3, note that the promotion type family is unified with the interface registry |
| **RFC-010** (accepted)                 | Clarify the implementation requirement that “record fields serve as constructors”—a prerequisite for retiring constructor parser special cases in Phase 2                                                                                                                                                            |
| **RFC-010b** (draft, formerly RFC-039) | Construction and destructuring must be paired; exhaustiveness checking must not depend on whether `Result` belongs to core or std (011b will migrate it in Phase 2)                                                                                                                                                  |
| **RFC-009** (accepted)                 | Cross-reference in the type-attributes section: the `Equal` precondition is “contains no `&mut` linear token” (not Dup)                                                                                                                                                                                              |
| **RFC-013** (accepted)                 | During Phase 2: remove the word "Result" from the text of `E1081` / `E1082`; reuse the `E1101` family for `Equal` precondition diagnostics—three-way synchronization under the RFC-013 process (codes/*.rs ↔ locales ↔ code table)                                                                                   |
| **RFC-018** (accepted)                 | After `%` changes to mathematical modulo, the `Mod → srem/urem` mapping table becomes invalid and must be changed to `srem` + sign correction (or composed from `sdiv`+`mul`+`sub`); terminology mixing between “modulo” and “remainder” must also be corrected                                                      |
| **RFC-036** (accepted)                 | No changes required. Document the relationship: `==` in `Any` positions retains runtime comparison, and the `assert_eq` assertion family is unaffected by the gate                                                                                                                                                   |

> Rationale for moving `Result` to std: RFC-013 already places it as the “std library
> `Result(T, Error)`,” and RFC-014's layering assigns std to the core source tree. Phase 2 of this
> RFC is one implementation path for that decision and no longer references other RFC numbers.

## Open Questions

- [x] ~~Does the semantic correction of `Modulo` require a compatibility period?~~ → **Closed**: the
      language reference has long stated “multiplication, division, and modulo,” and the current
      remainder implementation violates the published documentation. Treat it as a defect correction
      with no compatibility period (2026-09-22)
- [x] ~~Can users add operator implementations for existing types (orphan rule)?~~ → **Finalized**:
      an operator implementation may be written only in the module that defines the type. The
      definition of `Int` is in the core, so only the core may register it; users may register only
      their own types. All types are treated equally, and built-in types receive no privilege
      (2026-09-22)
- [ ] Should `Index` distinguish mutable indexing (similar to Rust's `IndexMut`)? (The initial scope
      is read-only; mutable indexing involves RFC-009's `WriteToken`, and RFC-011a already provides
      a precedent with the `&mut Self` receiver, so it is left for a future RFC)
- [ ] Should the Key for multi-position indexing use tuple packing (the current approach) or
      multiple parameters (Swift style)? (Continue with tuple packing; do not change the parser)
- [ ] The shape of `Zero` / `One`: constant members are not operators, and RFC-011a has no precedent
      for “interface members without a receiver.” A separate RFC must finalize this before the full
      `T: Add + Multiply + Zero` expression in RFC-011 can be fulfilled
- [ ] The complete shape of the `Try` interface: `?` requires success/failure determination,
      success-payload extraction, and failure-path production of the enclosing return value; the
      single method `residual` is insufficient. This must be finalized before Phase 2 begins,
      together with retiring the constructor parser special cases
- [ ] The `?T` prefix type (RFC-026 FFI nullable annotation, RFC-018) and the `e?` suffix operator
      share the `?` symbol: their positions differ (type position vs. expression position), so they
      do not conflict; this simply needs to be documented
- [ ] `PartialOrd` / `Ordering` (interface-driven comparison operators): a separate RFC; this RFC
      explicitly does not implement them

---

## Appendix A: Research Evidence

All measurements below were reproduced on **0.8.0** (`target/debug/yaoxiang-rs.exe`).

### A.1 Availability of the Interface Mechanism (Foundation of This RFC)

| Capability                                                                     | Measured Result                                                              |
| ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| Interface declaration `Animal: (Self: Type) -> Type = {...}`                   | ✅ Can be defined                                                            |
| Interface instantiation + external method `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ Runs successfully with correct output (`tests/rfc011a.rs` has unit tests) |
| Interface instantiation + **internal** method declaration                      | ❌ `E1097` (field and method share the same namespace)                       |
| Method dispatch `d.speak()`                                                    | ✅ Runs successfully                                                         |

**Note**: `E1097` means operator methods must use the **external declaration** form
(`Point.add: (self: &Point, ...)`), consistent with RFC-011a's examples. `method_bindings` is
registered in `environment.rs` (`add_method_binding`) and queried by the call-target resolution and
field-lookup-failure fallback path in `expressions.rs`.

### A.2 Current State of Operators

| Expression                                | Current State                                                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `1 + 2` / `"a" + "b"` / `[1] + [2]`       | ✅ Hardcoded allowlist (Int/Float/String/List, **requires both sides to have the same type**)                |
| `1 + 2.5` (Int + Float)                   | ❌ Compilation error (the allowlist requires both sides to have the same type)                               |
| `-7 % 3`                                  | `-1` (truncated remainder; the `%` allowlist contains only Int/Float and differs from the allowlist for `+`) |
| `Point(1,2) == Point(1,2)`                | ❌ `E6007` (Eq fails at runtime for Struct)                                                                  |
| `(1,2) == (1,2)` / `[1] == [1]`           | ✅ Runtime element-wise comparison (Struct is the only gap)                                                  |
| `a == b` in an Any position (`assert_eq`) | ✅ Runtime comparison (demonstrated by RFC-036)                                                              |
| `f[0]` (function positional binding)      | ⚠️ Legal only within a binding declaration; as an expression, it reports `E3006`                             |
| `arr[0, 1]` (multiple positions)          | ✅ Tuple packing, `list([1, 2])`                                                                             |

### A.3 Hardcoded Locations for `?` and the Constructors

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

Constructor side: `ok(T)` / `err(E)` / `some(T)` are recognized by the parser (language
specification `syntax.md` §1.4.2); `is_ok` / `unwrap` and similar functions in `std/result.rs`
pattern-match on `variant_id 0/1`. Moving “Result to std” requires solving both halves (`?` + the
constructors) together; both are assigned to Phase 2.

## Appendix B: Design Decision Record

| Decision                                    | Resolution                                                                                                                                                         | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Date       |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Architecture                                | Three-layer separation (mapping table / dispatch substrate / interface contracts)                                                                                  | Combining them would disconnect RFC-011 constraints from operators                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | 2026-09-15 |
| Operator release condition                  | The interface must be implemented (Layer 2 is the release condition)                                                                                               | Guarantees that `T: Add` and `+` correspond strictly                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | 2026-09-15 |
| Interface naming                            | Full words (`Multiply`, not `Mul`)                                                                                                                                 | Does not modify the accepted body of RFC-011                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 2026-09-15 |
| `%` interface name                          | `Modulo` (mathematical modulo)                                                                                                                                     | The name fixes the semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 2026-09-15 |
| Comparison operators                        | **Not** interface-driven in the initial scope; native IR instructions retained                                                                                     | They are already first-class instructions; `Ordering` brings an independent set of problems                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 2026-09-15 |
| `Ordering`                                  | Not introduced in the initial scope                                                                                                                                | No demonstrated requirement; it is the scope of a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | 2026-09-15 |
| Associated types                            | Use interface type parameters; do not introduce `type` member syntax                                                                                               | RFC-011a finalized this design (`Iterator: (Item: Type)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 2026-09-15 |
| Unification of method bindings and indexing | Conceptually unified; the interface governs container indexing only                                                                                                | A positional-binding key is a compile-time constant, and its result type requires type-family evaluation, so users cannot implement it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 2026-09-15 |
| Multi-position indexing                     | Tuple packing + overloading by Key type; no variadic interface                                                                                                     | No parser changes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 2026-09-15 |
| Bitwise / unary operators                   | Not implemented in the initial scope                                                                                                                               | Rare for custom types; YAGNI                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 2026-09-15 |
| Arithmetic interface shape                  | Three type parameters `(Self, R, O)`; `T: Add` ≜ `Add(T, T, T)`                                                                                                    | The result type is explicit: `1 + 2.5` and scaling can be expressed; the promotion table in RFC-011 §8.3 and the registry are unified; the shape matches `Index(Key, Value)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 2026-09-22 |
| `Equal` derivation                          | Automatic derivation by default + explicit instantiation override; constraint solving uses the same criterion                                                      | “User-written types cannot use ==” is unacceptable; Tuple/List already use element-wise comparison, and Struct is the only gap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | 2026-09-22 |
| `Equal` precondition                        | “Contains no `&mut` linear token,” **not Dup**                                                                                                                     | RFC-011 §2.4 explicitly states that primitives are not Dup; using Dup as the premise would incorrectly reject `Point{Float,Float}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | 2026-09-22 |
| Separation of names and registration        | Operators query only the interface implementation registry, not name resolution                                                                                    | Local same-name bindings (such as the type-level `Add` family) do not interfere with operators; RFC-011 §5.2 examples require no changes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 2026-09-22 |
| Orphan rule                                 | Implementations follow the module that defines the type                                                                                                            | All types are treated equally, and built-in types receive no privilege                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 2026-09-22 |
| `Any` equality                              | Retains runtime comparison and does not query the registry                                                                                                         | The RFC-036 `assert_eq` assertion family already depends on this behavior                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 2026-09-22 |
| `Try` shape                                 | Finalized: four methods (is_failure/success/residual/from_error) + assert-Never dead-path semantics; implemented                                                   | `?` needs three capabilities, and the single method `residual` is insufficient; constructor parser special cases are retired together with moving Result to std                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | 2026-09-22 |
| Classification of `%` semantics             | Defect correction (the documentation has promised modulo); no compatibility period                                                                                 | “Multiplication, division, and modulo” in `reference/index.md` is prior evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | 2026-09-22 |
| Rationale for moving `Result` to std        | Cite RFC-013's existing placement; do not reference a nonexistent RFC number                                                                                       | RFC-013 already states “std library `Result(T, Error)`”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | 2026-09-22 |
| Removal of call-site constraint rechecking  | Option A: remove the call-site recheck dead code and the `generic_fn_constraints` propagation chain; retain the E1106 code registration but do not emit it for now | Structural dead code: on arrival, `last_type_args` is always empty ⇒ `get(idx)` is always `None` ⇒ `continue` short-circuits ⇒ E1106 is unreachable on every path; moreover, under the “same criterion for constraints” principle, the recheck criterion (an interface-implementation-registry query) inherently diverges from the live execution path (mono rewrites `Call "T.add"` according to the function table). Empirical evidence: when the constraint is unimplemented, type checking reports 0 errors and failure is deferred to runtime (the rewritten target is absent from the function table), which can be reproduced with “constrained type + unimplemented interface + call” input. The layer and criterion for checking unimplemented constraints are registered as pending design under issue #519 | 2026-10-31 |

## Appendix C: Glossary

| Term                              | Definition                                                                                                                                                                                 |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Layer 0                           | Fixed operator-to-method-name mapping table; a language-level constant that users cannot change                                                                                            |
| Layer 1                           | Dispatch substrate that looks up and invokes methods by the `Type.method` key                                                                                                              |
| Layer 2                           | Interface contract layer that provides the basis for generic constraints and serves as the operator release condition                                                                      |
| Interface Implementation Registry | Master implementation table aggregating core default registrations and user instantiations; the sole criterion for operator queries and constraint solving, independent of name resolution |
| Native Fast Path                  | Hardcoded operation path retained for primitive types; bypasses interface dispatch                                                                                                         |
| Positional Binding                | RFC-004's `f[0]` syntax, which binds a function parameter position as a method; a compile-time behavior                                                                                    |
| Automatic Derivation              | Compiler-generated field-wise `==` for a record whose fields are all comparable; explicit instantiation can override it                                                                    |

## References

- [RFC-011: Generic Type System Design](011-generic-type-system.md) — `T: Add + Multiply + Zero`
  constraints and associated types
- [RFC-011a: Interface Implementation and Dynamic Dispatch](011a-interface-implementation.md) —
  interface declaration/instantiation/overloading rules
- [RFC-009: Ownership Model Design](009-ownership-model.md) — `&mut T` linear tokens
- [RFC-004: Multi-Position Union Binding for Curried Methods](004-curry-multi-position-binding.md) —
  `f[0]` syntax
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — sum-type expressions
- [RFC-013: Error Code Specification](013-error-code-specification.md) — `Result` placement in std
  and the error-code process
- [RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)](010b-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — associated-type
  `Output` design
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — multi-parameter subscripts
