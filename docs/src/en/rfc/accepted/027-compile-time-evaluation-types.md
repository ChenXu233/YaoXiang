---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-11-14'
impl_status: 'in-progress'
impl_detail:
  'Phases 1-2 are complete, Phase 3 is partially complete, and Phase 4 is partially complete. The
  unified assert/Assert approach has been fully implemented across all six phases (#157-#162
  closed): Never type, IsTrue bridge, flow-sensitive Γ + kill set, type-level recursion, and weak
  universe stratification check. The dispatch routing **semantics** in §11 has been implemented
  through call-site obligations (`checker.rs` directly invokes `check_predicate` in three places:
  binding position / call-site argument / return position), but the **standalone
  `layers/dispatch.rs` module has been deleted**—it had zero production call sites, and its handling
  of `Unproven` (downgrading it to a W1080 warning) conflicted with the production implementation
  (always reporting an error) (#377-2). The proof kernel, logic signatures, certificates, and the
  SMT role have been rewritten around the unified term world of RFC-039 D62 (§4, §8, §13):
  SMT/tactics/proof search are **untrusted certificate producers**, `unsat` is merely a search
  result, and a certificate must be constructed and verified through `Kernel::check` before it can
  become Proved. `predicate.rs` currently has `SMTResult::Unsat => Ok(())`, a gap that trusts SMT
  outright; see §8.6. §14 completes the dependent record Σ in place: proof-space field criterion,
  constructor data-only + automatic evidence synthesis, proof family parameter `P: T ->
  Proposition`, cross-module evidence slots in `TypeDefPayload`, type-level `b.w` in
  `ConstExpr::FieldPath`, 5A immutability, and division of labor among the four codes.'
impl_percent: 82
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax - name: type = value Model](010-unified-type-syntax.md)
> - [RFC-011: Generic System Design](011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** as first-class citizens of YaoXiang and
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an add-on specification comment—it is a function. A function returning Type can be
used in type positions; the compiler calls it at compile time and checks its return value. Types are
propositions, and compile-time evaluation is proof.

**Core argument**: the sole task of type checking at compile time is to construct and verify proof
terms. Type equalities, token conflicts, dependent type reduction, compile-time predicate
evaluation, and Hoare-logic implication are all different kinds of type checking within the
compile-time proof pipeline. They share one pipeline and one proof kernel, `Kernel::check`. Proof
terms and propositions live in the same **unified term world** (RFC-039 D62). Propositions receive
their meaning from a **logic signature**, `LogicSignature`, while proof terms are represented by
**certificates**, `Certificate`. A certificate is relative to a signature and is validated only by
the kernel according to that signature's rules. SMT solvers, tactics, and proof search are all
**untrusted certificate producers**: they are responsible only for finding certificates, and `unsat`
is not `Proved`. When the kernel returns Unproven, the programmer writes a YaoXiang proof function
as explicit evidence—it is a proof term, and the kernel validates it in exactly the same way it
validates the return type of any other function. Everything is YaoXiang code, and everything is
validated by the proof kernel.

## Motivation

### Why Should RFC-022 Be Deprecated?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This commits a fundamental Curry-Howard isomorphism error: **splitting specifications and types into
two layers**. A comment is not a type. Comments do not participate in type checking. Comments embody
the mental model of an “external tool.”

The whitepaper states this plainly:

> “There are no `//!` comments. There is no separate specification language. Everything is within
> the type system.”

### Current Problems

- RFC-022's `//!` comments are an add-on syntax independent of the type system
- Specification types and ordinary types constitute two separate systems, causing conceptual
  redundancy
- The split between verification in Debug builds and its omission in Release builds undermines
  uniformity
- In the traditional understanding, SMT solvers are positioned as external tools or as “trusting
  Z3's `unsat`”—YaoXiang instead positions them as untrusted certificate producers: they produce
  candidate certificates that are validated by the proof kernel
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each follow
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
Kernel::check(signature, certificate, proposition) → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow-conflict detection, and compile-time predicate
validation—are subtasks of this function. They share one proof pipeline and differ only in
proof-term complexity and construction strategy.

Every obligation carries the currently effective `LogicSignature`. When the kernel returns Unproven,
the programmer supplies explicit evidence: a proof function whose return type equals the proposition
to be proved serves as a certificate. The kernel validates it with the same signature—this is the
same operation as ordinary type checking. Signatures are replaceable: changing the signature changes
the inference rules, but neither the kernel nor the three-valued result changes (§4.1).

## Proposal

### 1. `{}` Is Proof Space: Types Are Assertions, and Verification Is Type Checking

YaoXiang's `{}` is a compile-time proof space. Everything inside it is an assertion. The proof
kernel guarantees, under the current `LogicSignature`, that every item has a verifiable
certificate—either found automatically by a producer or supplied explicitly by the programmer
(§4.3).

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  The compiler guarantees that x is Float and y is Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  The compiler guarantees that data is Array(T)
```

**Generics are a special case of compile-time predicates.**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          Parameter at the signature       Only assertions inside {}
#          The compiler validates x > 0 when called at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter at the signature           The compiler validates type_of(T) == Type and type_of(data) == Array(T)
```

The pattern is always the same: `name: (params) -> Type = { assertions }`. The compiler does not
distinguish “type assertions” from “value assertions”—both are evaluation targets in the proof
pipeline.

**Loop invariants do not need to be written separately. A type annotation on a variable is already a
Floyd-Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i—it tells the compiler that s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0; validation: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # The compiler validates: s_new == sum(arr[0..i+1])
        i += 1       # Changing i triggers revalidation of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—the inductive hypothesis (the
type annotation) → the assignment → whether the new value satisfies the type annotation. Once the
producer produces a candidate certificate for the inductive step and `Kernel::check` validates it,
all iterations are covered automatically. No `: decreases` is needed, no `: Invariant` is needed,
and no inductive proof is needed—the induction is decomposed into local VCs for each assignment.

### 2. Preconditions/Postconditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates become type annotations on
parameters or return values.

**The parameter side is a function call.** A compile-time predicate is a function returning Type, so
using it on the parameter side means calling it—just like `factorial(5)`. The return-value side
introduces a new concept: the return-value formal parameter.

```yaoxiang
# Precondition: explicitly call a compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name and is passed to Positive
#                       The compiler extracts the actual argument at the call site, substitutes it for b, and validates Positive(actual argument)
#                       Example: divide(10, 2) → validates Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → validates Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value formal parameter; its value is provided by return
#                                            The compiler substitutes the returned value at the return point and validates the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current formal parameter name and is passed to
  `Positive`. This uses ordinary function-call syntax, with zero implicitness.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is the return-value formal
  parameter, and its value is provided by the `return` statement. `result` exists only in the type
  signature and is referenced only by predicates; it does not enter the function-body scope and does
  not appear at the call site.
- **The return-value formal parameter is optional**: with no postcondition, omit it; the signature
  is then identical to that of an ordinary function (`-> Int`).
- **Uniformity**: parameter and return-value formal parameters are the same
  concept—`formal-parameter-name: predicate-call(actual arguments)`—and differ only in whether the
  value is provided by the caller or by `return`.

> **Implementation note (2026-10-02)**: return-position formal parameters are recognized **strictly
> by their declared names**—the `r` in `-> (r: P(...))` is the sole return-value formal parameter.
> Any other free variables in the constraint that are **not in scope** are no longer substituted
> with the returned value; instead, an undefined-identifier error is reported (observed:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` is undefined) →
> `error [E1001] Unknown variable: 'm'`). Predicate-call arguments are substituted into the
> constraint according to their **actual arguments**: `P(r + 100)` becomes “returned value + 100”
> after substitution, rather than merely “returned value.”
>
> **Known gap (2026-10-02)**: **multi-parameter predicates with symbolic arguments in return
> position are not yet supported**. Triggering form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Alternative: replace the actual argument with a compile-time foldable literal (such as
> `SumUpTo(3, r)`), or provide a proof function for the predicate that returns `Type`. Resolution
> condition: support actual-argument substitution for “multi-parameter predicate + symbolic
> argument” in the proof kernel. See [RFC-027a](../accepted/027a-termination-explicit-measure.md)
> §Implementation Notes (2026-10-02, this hardening round).

### 3. Path-Condition Propagation: Compile-Time Validation of Runtime Values

When a compile-time predicate is used in a binding position, the programmer explicitly supplies its
arguments. When a runtime value enters a refinement-type parameter, the producer collects path
conditions and produces a candidate certificate, which `Kernel::check` validates under the current
`LogicSignature`. The programmer does not need to implicitly propagate a proof; explicit evidence is
a first-class form of supplementary evidence (§4.3).

#### 3.1 Explicit Function Calls

When a compile-time predicate is used in a binding position, the programmer explicitly supplies its
arguments—it is an ordinary function call, with zero implicitness.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (a parameter declaration, variable declaration, or return type), the programmer
explicitly supplies an already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to fill in arguments implicitly—`b: Positive(b)` is a function call, just
like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` refers to `b`
itself—this is the standard pattern for dependent types, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 established that `self` is not a keyword but merely a
conventional parameter name (“writing `p`, `this`, or `x` has exactly the same effect”).
`b: Positive(b)` uses the same mechanism—a parameter name can be referenced in a type annotation.
`self` appears in the position `self: Point`, while `b` appears in the position `b: Positive(b)`;
both type annotations refer to the parameter itself. The only difference is the complexity of the
type annotation; the mechanism is exactly the same—once a name is bound, the type may depend on that
name.

Return types likewise use explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal parameter; Sorted(result) is a function call
//                        The compiler substitutes the returned value for result at the return point and validates Sorted(returned value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5; Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0; Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path-Condition Collection

When a runtime value occurs in a conditional branch, the compiler automatically collects path
conditions to form the current scope's **assumption set**. These assumptions participate in
validation as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Validation condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline determines that the implication holds → Proved
} else {
// The assumption in this branch: { !(y > 0) }
// To call divide(x, y), the validation condition is !(y > 0) ⇒ y > 0
    // The proof pipeline determines that the implication does not hold → Disproved
}
```

This is not a hard-coded special case in the compiler—it is the natural behavior of the compile-time
proof pipeline. At every type-checking call site, the pipeline receives:

```
{背景假设} ⇒ {验证目标}
```

The proof pipeline determines entailment. Proved → pass; Disproved → compile error + counterexample;
Unproven → compile error + unresolved proposition. The background assumptions come from the path
conditions at the current program point.

#### 3.3 Assumption Stack

While analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → push `y > 0` onto the true branch, and `!(y > 0)` onto the false branch
  (if an else is present)
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` within the branch
- **Logical conjunction**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` within the branch
- **Function precondition**: when calling `divide(a, b)`, the evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the actual argument's own refinement-type
  annotation (`b` is annotated as `Positive`, so its type carries `b > 0`)
- **Assignment**: when writing `let z = y`, existing refinement conditions on `y` are transferred to
  `z`

All assumptions enter the `context` field (`Γ`) of the `Certificate` and are passed to
`Kernel::check` together with the candidate certificate; when a producer needs SMT, it translates
the same `Γ` into SMT-LIB background assertions.

#### 3.4 Compile Error When No Static Evidence Exists

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the actual argument `y` itself has
no `Positive` type annotation. The validation condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (no entailment) → compile error:

> Unable to prove that argument `b` satisfies `Positive` in the call to `divide`. `y` comes from a
> function input and has no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not permit a runtime value to enter a refinement-type parameter without static
evidence. This is not a restriction—it is the core of the hard-safety philosophy: no code may
compile if `Kernel::check` cannot validate a certificate under the current signature.

#### 3.5 Relationship to the Unified Pipeline

Path-condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline into control-flow analysis:

| Stage                           | Responsibility                                                                                                                                   |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Path-condition collection       | During compiler control-flow analysis, annotate each basic block with an assumption set                                                          |
| Validation-condition generation | When a type constraint requires validation, merge path conditions with actual-argument type information                                          |
| Proof-pipeline decision         | The certificate producer produces a candidate certificate → `Kernel::check(signature, certificate, proposition)` → three-valued result           |
| Result                          | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unresolved proposition + supplementary-evidence hint |

There are no new components and no special rules. Path conditions are the local assumption set `Γ`
of a `LogicSignature`; they share one pipeline and one budget system with type equalities and borrow
constraints.

### 4. Proof Kernel: `Kernel::check`

All compile-time checks share one pipeline and one **proof kernel**. The kernel's sole decision
entry point is:

```text
Kernel::check(
    signature   : LogicSignature,
    certificate : Certificate,
    proposition : Proposition,
) -> CheckVerdict<Certificate>
```

`check` is a **pure function**: given the same signature, certificate, and proposition, it always
returns the same result; it does not read solver state, the clock, or file paths. It does only one
thing—replay the certificate's derivation steps one by one according to the `rules` of `signature`,
checking the rule instance, premises, context, and substitutions of every step, and verifying that
the final conclusion is exactly `proposition`. The kernel does not search, guess, or complete
certificates; a missing step, an incorrect rule, or a mismatched conclusion must not yield `Proved`.

This is the same picture as the **unified term world** of RFC-039 D62: type positions and value
positions share one set of compile-time terms. Propositions `Proposition`, proof terms
`Certificate`, and the sorts/rules/axioms in signatures are all terms in this one world; there is no
second proof language. Equality (§12.1) is a pure decision, the evidence for a `q=0` slot (RFC-011
§4.1) is a call-site obligation, and a Σ witness (§13) is a construction-site obligation—all three
flow into the same `check` rather than opening separate trust channels.

The three values of `CheckVerdict<Certificate>` are consistent with §4.1: `Proved(checked)` carries
a kernel-validated certificate, `Disproved(model)` carries a counterexample, and `Unproven` carries
the unresolved proposition and its reason. Only `Proved(checked)` allows an obligation to pass;
`checked` is the certificate that can be cached at compile time and reused incrementally.

```
编译期义务（命题 + 当前 LogicSignature）
        │
        ├── producer 编排（都不可信，见 §8）
        │   ├── 内核直判规则：结构等价、βδι-归约、令牌活性事实
        │   ├── Γ 等式桥 / 调用侧 q=0 证据（RFC-011 §4.1）
        │   ├── 显式证据：证明函数、axiom library / proof library 条目
        │   └── SMT / tactics / proof search：产出候选 Certificate
        │
        ▼
Kernel::check(signature, certificate, proposition)
        │  按 signature.rules 重放每一步；不用「unsat」顶替检查
        ▼
 ┌──────────────────────────────────────────────┐
 │ Proved(checked)   → 编译通过（证书编译期留存）│
 │ Disproved(model)  → 编译错误 + 反例           │
 │ Unproven          → 未解命题 + 原因 + 补证据  │
 └──────────────────────────────────────────────┘
        │
        ▼
Unproven → 提示补显式证据（证明函数 / 换签名 / 补公理）
        │
        ▼
再次 check ──→ Proved(checked) ──→ 编译通过
```

#### 4.1 `CheckVerdict`: Three-Valued Algebra and Logic Signatures

`check` returns three possible results—an inevitable consequence of the halting problem and a
natural partition of proof theory:

```text
CheckVerdict<C> = Proved(checked: C) | Disproved(model: Model) | Unproven(reason, subject)
```

- **`Proved(checked)`** → halts; the certificate has been validated step by step according to
  `signature`, and the conclusion equals `proposition`. The obligation passes; `checked` is the
  certificate that can be cached at compile time (§13).
- **`Disproved(model)`** → halts; a counterexample `model` exists. Compile error + counterexample +
  source location.
- **`Unproven(reason, subject)`** → within the current signature, evidence, and resource limits, the
  producer did not deliver a verifiable certificate. Compile error + unresolved proposition +
  reason + **supplementary-evidence hint** (the diagnostic shape from §12.3).

**Unproven ≠ False.** The kernel saying “it cannot be computed” is not equivalent to the proposition
being false—it means only that there is no certificate that passes validation under the current
`signature` and current evidence set. This is honest, not a defect.

The **logic signature**, `LogicSignature`, determines “what counts as a valid derivation step.” It
is replaceable data:

```text
LogicSignature {
    sorts        : Map<SortName, SortDecl>          // Categories of terms and propositions
    rules        : Map<RuleId, InferenceRule>       // Derivation rules that the kernel can replay
    axioms       : Map<AxiomId, Proposition>        // Axioms accepted by this signature
    libraries    : [LibraryRef]                     // All axiom/proof libraries; see below
    hash         : ContentHash                     // Covers all entries in sorts/rules/axioms + libraries
}
LibraryRef {
    kind         : LibraryKind                     // Axiom | Proof
    id           : LibraryId                       // Namespace for entry IDs within the library
    manifest     : Manifest                        // Entry IDs + proposition/rule shapes + provenance
    hash         : ContentHash                     // Content hash over the canonical bytes of the manifest
}
SortDecl {
    space        : SortSpace                       // Proof | Data; see below
    arity        : [SortName]                      // Argument sorts accepted by this sort's constructor
}
SortSpace = Proof | Data
```

Each `SortDecl` in `sorts` has a `space`, which is the **sole criterion for deciding whether an
application of a type family is a proposition or data**. `space = Proof` means that inhabitants of
the sort are proof terms (the results of propositions/refinement types/proof families), while
`space = Data` means that inhabitants are runtime data (`Int`, `Array`, `Vec`, ordinary records).
`Pos` (refinement predicates such as `{ v > 0 }`), `Assert`, and `Terminates`, as well as
`proof/type family` values whose return type has `space=Proof`, have `space=Proof`; data type
families such as `Array: (T: Type, N: Int) -> Type` have `space=Data`. `space` is registered by a
library manifest or predicate declaration; it is never guessed from field names, argument shapes, or
syntax that merely “looks like a predicate.” The evidence classification in §14.1 reads only this
field. A data type family remains a data field even when it depends on a strictly preceding field.

`libraries` is a **list**: one signature may use multiple axiom libraries and multiple proof
libraries simultaneously (for example, the built-in `linarith` axiom library + a user `zfc-core`
axiom library + the `std-proofs` proof library + a project-private proof library). Each library
carries its own `kind`, `id`, `manifest`, and `hash`. Every entry in `axioms` belongs to a
`kind=Axiom` library by `LibraryId`, and proof-library entries belong to a `kind=Proof` library by
`LibraryId`. `signature.hash` covers `sorts/rules/axioms` **and every entry in `libraries`** (not
just one); adding or removing any library, or changing even one byte of any manifest, changes the
signature and invalidates every cache key, certificate, and incremental-reuse result according to
`hash`.

- `sorts`, `rules`, and `axioms` form the body of the signature; every entry in `axioms` must belong
  to a manifest entry in a `kind=Axiom` library in `libraries`. If a certificate references an axiom
  not registered by any library ⇒ `check` rejects it.
- Every `LibraryRef` in `libraries` has its own manifest and content hash: the manifest lists entry
  IDs, proposition/rule shapes, and provenance, while `hash` is computed over the manifest's
  canonical bytes; `kind` distinguishes axiom libraries from proof libraries. The signature `hash`
  covers `sorts/rules/axioms` and **all** `LibraryRef`s (not two separate values).
- Built-in libraries (`Int` linear arithmetic, `Array` lengths, `Assert` bridging, etc.) are
  ordinary `LibraryRef`s in `libraries`; they merely ship with the compiler. They are peers of user
  and project libraries, **not hard-coded trust roots in the kernel**. Trust comes from the fact
  that “the signature and its `libraries` list are adopted by this compilation,” not from the name
  of a library or solver.

Signatures are replaceable, and replacement is data replacement rather than kernel replacement:

| Signature                                    | sorts / rules values                                                   | Typical use                                           |
| -------------------------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------- |
| Classical arithmetic fragment                | Linear-arithmetic sorts + no quantifier rules                          | `x > 0`, `0 <= idx < arr.len`                         |
| Classical first order (with excluded middle) | Classical sequent rules + excluded-middle axiom                        | Classical implication, proof by contradiction         |
| Constructive                                 | Constructive rules, no excluded middle, no double-negation elimination | Extracting witnesses, avoiding nonconstructive proofs |
| ZFC fragment                                 | Set-theory sorts + registered choice/extensionality axioms and similar | Propositions requiring set-theoretic vocabulary       |

`Proved` is always **relative to a signature**: the same proposition may be `Proved` under a
classical signature but `Unproven` under a constructive signature. A certificate carries
`signature_hash`; `check` first verifies that the hash matches and then replays the proof. A hash
mismatch immediately yields `Unproven`, without attempting “roughly compatible” reuse.

**Cross-signature use must be explicit.** Let `A`/`B` be two signatures, `P` a proposition in `A`,
and `certA` a certificate validated under `A`. To use the conclusion under `B`, there are exactly
three legal paths, and each must leave an inspectable trace:

1. **Translation term**: provide `translate: A.Proposition → B.Proposition` and a certificate for it
   validated under `B`; obtain `Q = translate(P)`, then prove `Q` under `B`.
2. **Bridging axiom**: register an explicit bridging axiom in a `kind=Axiom` library in
   `B.libraries` (for example, “the corresponding rule of `A` has a corresponding interpretation
   over that sort in `B`”); the certificate references `(LibraryId, AxiomId)`; `B.hash` changes with
   that registration.
3. **Reproofing**: construct a new certificate directly for `P` under `B`.

Implicit transport of the form “`Proved` under `A` ⇒ `Proved` under `B`” is absolutely forbidden, as
is reusing `certA` under a different signature hash. Cross-module export in Σ (§13) and the
cross-module form of `q=0` evidence follow the same three paths.

A hard budget limit is the engineering solution to the halting problem (the numerical values appear
in §8.4). No knob is provided—asking for one amounts to asking the user “do you think your program
will halt?”, which neither the user nor the compiler knows. Exhausting the budget yields
`Unproven(reason=BudgetExceeded)`, not `Disproved`.

#### 4.2 `Certificate`: Signature-Relative Proof Terms

A **certificate** is the compile-time representation of a “proof term” and has meaning only relative
to some `LogicSignature`:

```text
Certificate {
    signature_hash : ContentHash            // Must equal signature.hash
    proposition    : Proposition           // Conclusion; a term in the unified term world
    steps          : [ProofStep]           // Derivation sequence according to signature.rules
    evidence_refs  : [EvidenceRef]         // References to explicit evidence
}
ProofStep {
    rule       : RuleId                   // Must occur in signature.rules
    premises   : [StepId | AxiomId | GammaId | EvidenceId]
    context    : Γ                       // Local assumptions (path conditions, equality bridges, q=0 evidence)
    conclusion : Proposition
    subst      : Substitution            // Term substitution; terms in the unified term world
}
EvidenceRef = ExplicitProof(AstNodeId)     // Proof function / explicit evidence written by the user
            | LibraryAxiom(LibraryId, AxiomId)
            | LibraryProof(LibraryId, ProofId)
            | ProducerHint(bytes)          // A hint from SMT/tactics/proof search; not trusted directly
```

`check` performs four mechanical tasks for each `ProofStep`: ① `rule` exists in `signature.rules`; ②
`premises` point to earlier steps, registered axioms, `Γ` facts, or explicit evidence, and each is
resolvable; ③ `subst` is applied to the rule pattern and is identical to `conclusion` (equality uses
the pure decision procedure of §12.1); ④ the final step's `conclusion` is identical to
`certificate.proposition`. If any condition does not hold ⇒ `Unproven`, with the failing step
identified; **the four checks are never skipped merely because the producer says “unsat.”**

`ProducerHint` is only a hint: a producer may attach it to a candidate certificate (for example, an
SMT-LIB script, model, or tactic trace), but the kernel neither executes it nor treats it as a rule.
The hint serves only to indicate “how to complete the candidate into inspectable `steps`.” If
completion fails, the result remains `Unproven`.

#### 4.3 After Unproven: Supply Explicit Evidence (Proof Functions Are First-Class)

When the kernel returns `Unproven`, the diagnostic must suggest **supplementary evidence**, rather
than rendering “cannot be computed” as “does not hold” (for the diagnostic shape, see §12.3). Valid
forms of supplementary evidence are treated as **first-class**—they are proof terms in the unified
term world, not add-ons:

- **Proof function**: a YaoXiang function whose return type equals the proposition to be proved. It
  is an `ExplicitProof(AstNodeId)`, and the kernel validates it by the same mechanism used to
  validate `add(a, b): Int`.
- **axiom library entry**: an axiom registered in the signature, `EvidenceRef::LibraryAxiom`.
- **proof library entry**: a named proof (reusable, unit-testable, and equipped with a manifest
  hash), `EvidenceRef::LibraryProof`.
- **Existing kernel certificate**: a certificate already validated as `Proved(checked)` under the
  same `signature_hash`, reused directly or as a premise.
- **Change signature / add an axiom / write a translation term**: follow the three cross-signature
  paths from §4.1, register the change explicitly, and run `check` again.

```text
命题    = 类型（统一项世界里的 Proposition）
证明    = 程序（该类型的一个值，即 Certificate 的 steps）
验证    = Kernel::check(signature, certificate, proposition)
信任    = 该签名被本次编译采用 + 证书逐步可重放；不来自求解器
```

If a candidate certificate produced by SMT/tactics/proof search cannot be completed into `steps`,
the result falls back to `Unproven`—the programmer supplies explicit evidence, or asks the producer
to try another search path. No AI is needed, no export to Coq is needed, and no new concept is
needed.

```yaoxiang
# Proposition: a refinement property the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Explicit evidence: the programmer writes a function whose return type is the proposition above
# The kernel validates it—exactly as it validates add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    ...
}
```

**A property that cannot be proved automatically at compile time → the programmer supplies explicit
evidence (a first-class proof term) → the kernel validates it according to the signature.** The
entire process is a smooth gradient—the producer finds easy certificates for you, leaving the hard
thinking to you.

#### 4.4 Layered Dependencies Within the Pipeline

The producers above share the same `Kernel::check` interface, but there is an orchestration order.
Type equality is a prerequisite for all subsequent analyses; ownership/token checks depend on type
information; and refinement-predicate validation depends on the results of the first two layers. The
compiler orchestrates by layer, and expressions that fail at a lower layer do not enter higher
layers—this avoids spending search budget on programs with type errors; **the conclusion of every
layer must still be validated by `check`; the layer order merely determines which candidate
certificate is requested from a producer first**.

```
编排顺序（同一内核，分层调度）
├── 第 0 层：类型等式（T1 == T2）
│   └── 内核直判规则（defeq）→ 产出候选证书 → check
├── 第 1 层：所有权/令牌冲突
│   └── 流敏感活性事实 → 产出候选证书 → check
└── 第 2 层：精化谓词/霍尔蕴含
    └── producer（Γ 证据 / proof library / SMT 搜索）→ 候选证书 → check
```

Every layer still returns `Proved/Disproved/Unproven` and shares the same interface, the same
`signature`, and the same budget system.

### 5. Unification of the Three Function Layers

| Layer                  | Execution time | Input      | Output | Example                                        |
| ---------------------- | -------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time   | Type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time   | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors follow
the same compile-time proof pipeline—`{}` is proof space.

### 6. Loops: Floyd-Hoare Validation-Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates
validation conditions from the type annotations, and the proof pipeline checks whether each
assignment preserves the type.

The core mechanism is that each assignment corresponds to a Hoare triple `{P} x := e {Q}`, with the
validation condition `P ⇒ Q[e/x]`. The compiler generates one validation condition for the loop
body—once the proof pipeline validates the inductive step, all iterations are covered automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i; at initialization i=0; validation: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Validation: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Premises: s satisfies SumUpTo(arr, i), and i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Validation obligation: s_new satisfies SumUpTo(arr, i) (i is unchanged for now)
        #   Substituting s_new = s_old + arr[i]:
        #     Requires s_old + arr[i] == sum(arr[0..i+1])
        #     From the inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → s's type annotation references i in the dependency graph → triggers revalidation
        #   New validation target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), that is, s == sum(arr[0..arr.len])
}
```

A loop invariant is the type annotation on a variable—the programmer writes the type, and the
compiler checks the inductive step. The compiler does not need to “discover” invariants or “perform
induction automatically”; it decomposes the inductive proof into local validation conditions for
each assignment and hands them to the proof pipeline to solve divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The mechanism above presupposes that the compiler knows that the type annotation `SumUpTo(arr, i)`
on `s` references `i`—when `i` changes, the type constraint on `s` changes as well. This requires
the compiler to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key: the variable being depended on; value: the set of variables whose type annotations reference it
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: while processing `mut v: Pred(... x ...) = init`, the type checker resolves
free-variable references in the arguments of `Pred(...)`. If an argument references another mutable
variable `x` in the current scope, the dependency graph records `x → v`.

**Trigger**: when the depended-on variable `x` is assigned, the compiler:

1. Finds all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a validation condition: “does the current value of `v` satisfy the
   updated type `Pred(... x_new ...)`?”
3. Sends the VC into the proof pipeline

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Using `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → revalidate that s satisfies SumUpTo(arr, i_new) → True

# Incorrect order—the compiler rejects it
i += 1        # i changes → revalidate that s satisfies SumUpTo(arr, i_new)
              # s has not yet been updated; s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: one variable may depend on several variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—a change to either triggers revalidation.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent verification mechanism. It answers “when must a VC be generated?”; the proof pipeline
answers “does the VC hold?”

### 7. Termination Checking

**Scope: refinement types.** Termination is not an independent switch; it is part of a
**verification mode**. Once a type is refined (`Refined { base, constraint }`), the computation
described by that refinement enters verification mode, and termination is required within that mode.
Ordinary types that are not refined do not enter verification mode and generate no termination
obligations.

This yields two corollaries:

- **Loops**: a bare `while` does not enter verification mode; a measure variable with a refinement
  annotation (such as `i: UpTo(n)`) enters verification mode and requires a termination proof.
- **Recursion**: when a function signature contains a refinement (a refined parameter or a
  refinement in the return type), it enters verification mode, and the measure must be shown to
  decrease strictly at every recursive call site.

Fully automatic within the mode: the compiler first explores measures automatically and accepts
those it can prove; if exploration fails and no explicit measure was provided, it reports a compile
error. There is **no annotation-syntax escape hatch**—the measure and termination proposition are
both written in type position, introducing no new syntax such as `decreases`. See §6.9 for the
shapes of the measure and explicit fallback.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for a termination proof from two places:

1. **Variable type annotations**: boundary constraints in refinement types (for example, `UpTo(n)`
   supplies the upper bound `n` and lower bound `0`)
2. **Loop-body operations**: the operation applied to variables on each iteration

The compiler tries four measure-synthesis strategies in priority order and stops when it finds one.
The four strategies form a **restricted template sequence for measure exploration**. Their input
consists of refinement constraints (strategies 1–4 all begin with a “variable of a bounded type”),
not “code evaluated at compile time”; they are the automatic and manual sides of the same mechanism
as the explicit measure in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded patterns, multiplicative scaling) and does not guarantee
> that a solution exists—general measure inference is generally undecidable (it reduces to the
> halting problem). Therefore, cases outside the templates must allow the programmer to provide an
> explicit measure (§6.9); otherwise, compilation fails.

#### 6.2 Strategy 1: Automatic Synthesis of a Linear Rank Function

When a variable has a linear-bound annotation, the producer enumerates candidate linear measures and
produces candidate certificates for the decrease obligations, which `Kernel::check` validates.

```
输入：
  变量 v₁: UpTo(u₁), v₂: UpTo(u₂), ...（有上下界的变量）
  循环条件 cond
  循环体中的赋值集合

算法：
  1. 从类型标注提取每个变量的界：[low_i, high_i]
  2. 枚举候选度量：v_i, u_i - v_i, v_i - v_j, 等线性组合
  3. 对每个候选度量 m：
     - 产出 m ≥ 0 的候选证书（从类型界推导）→ check
     - 对循环体的每条执行路径，产出 m' < m 的候选证书 → check
  4. 找到符合条件的线性组合 → 终止得证
```

Coverage: loops in which any variable is assigned a linear expression (`v = a·v + b`) and has a
bounded-type annotation. This includes `i += const`, `i -= const`, and interval contraction in
binary search:

```yaoxiang
# 二分查找：low = mid + 1 或 high = mid
# 度量 high - low 在两条路径上都严格递减
binary_search: (arr: Sorted(Int, arr), key: Int) -> Option(Int) = {
    mut low: UpTo(arr.len) = 0
    mut high: UpTo(arr.len) = arr.len
    while low < high {
        let mid = (low + high) / 2
        if arr.data[mid] < key { low = mid + 1 }
        else if arr.data[mid] > key { high = mid }
        else { return Some(mid) }
    }
    return None
}
```

#### 6.3 Strategy 2: Predicate Violation Count—Automatically Extracting a Measure from the Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: an experimental strategy whose inclusion will be decided during Phase 3
> implementation based on practical feasibility.** This strategy works for adjacent-swap operations
> (bubble sort, insertion sort), but cannot automatically prove non-adjacent operations (quicksort
> partition, heap-sort sift-down). The coverage boundary appears in the table below. If Phase 3
> validation proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specification written by the user is material for compiler reasoning.** The
compiler does not need to hard-code “what sorting means”—it reads the definition of `Sorted` and
automatically extracts a measure from it.

```
输入：
  目标类型：Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  循环体操作：相邻元素交换

算法：
  1. 解析谓词定义：forall i in range: cond(i, arr)
  2. 自动生成度量：violation_count = |{ i | ¬cond(i, arr) }|
  3. 分析操作对度量的影响：
     - 相邻交换 arr[j], arr[j+1] = arr[j+1], arr[j]
     - 只影响索引 j-1, j, j+1 三对
     - 如果 arr[j] > arr[j+1]（违反谓词），交换后这对满足谓词
     - violation_count 至少减 1
  4. 上界：n·(n-1)/2（最大相邻逆序数），下界：0
  → 终止得证
```

**Current coverage**:

| Algorithm      | Operation pattern | Provable by Strategy 2? | Reason                                                |
| -------------- | ----------------- | :---------------------: | ----------------------------------------------------- |
| Bubble sort    | Adjacent swap     |           ✅            | violation_count strictly decreases on every swap      |
| Insertion sort | Adjacent movement |           ✅            | Every shift eliminates one violating pair             |
| Selection sort | Non-adjacent swap |           ❌            | A single swap may increase violation_count            |
| Quicksort      | Partition         |           ❌            | Non-adjacent swap; no guarantee of monotonic decrease |
| Heap sort      | sift-down         |           ❌            | Tree operation; violation_count is not monotonic      |

**Complementary strategies**: for quicksort, contraction of the `low < high` interval can be covered
by Strategy 1 (linear rank function)—the outer partition recursion halves the interval on each step.
Strategies 1 and 2 provide complementary coverage, and termination for most practical algorithms can
be proved by one or the other. Generalizing Strategy 2 (non-adjacent operations, tree operations)
remains an open problem.

```yaoxiang
sort: (arr: Array(Int)) -> (result: Sorted(result)) = {
    mut i: UpTo(arr.len) = 0
    while i < arr.len - 1 {
        mut j: UpTo(arr.len - i - 1) = 0
        while j < arr.len - i - 1 {
            if arr.data[j] > arr.data[j+1] {
                arr.data[j], arr.data[j+1] = arr.data[j+1], arr.data[j]
            }
            j += 1
        }
        i += 1
    }
    return arr
}
```

#### 6.4 Strategy 3: Bounded Increasing/Decreasing Patterns

For `v += const` (a positive constant), a variable with an upper-bound type annotation yields the
measure `upper_bound - v`, which decreases by `const` each time and has lower bound 0. This is a
degenerate case of Strategy 1, and the compiler handles it first for speed.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

For `v *= const` (const > 1), the variable has lower- and upper-bound type annotations. The compiler
includes the logarithmic measure template `ceil(log_const(upper/v))`, which decreases by 1 each time
the value is multiplied by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically derives the measure ceil(log₂(n/i)), which decreases by 1 each time the value is multiplied by 2
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proofs and correctness proofs are independent:

- **Termination**: the four strategies above automatically prove that the loop exits in finitely
  many steps; when exploration fails, the programmer supplies a measure in type position (§6.9)
- **Correctness**: whether the loop body advances toward the target type, checked by the
  compile-time proof pipeline through validation conditions

When both pass → compilation passes. Termination is proved but correctness fails → compile error +
counterexample. Correctness is proved but termination cannot be proved → compile error identifying
the variable or operation that could not be analyzed. When both fail → compile error reporting both
failure reasons separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks that formal parameters decrease
at every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // The compiler explores: the well-founded-order measure of (b, a % b) decreases → termination
}
```

Formal-parameter decrease is the strongest automatic path (structural recursion). When exploration
fails, the programmer explicitly supplies a measure in type position (§6.9).

#### 6.8 Hard Boundary

For `i = f(i)`, where `f` is not invertible, does not form a closed set, and preserves no
monotonicity—automatic termination proof is mathematically impossible. Compile error:

> This loop cannot be proven to terminate automatically. The loop variable depends on the
> unanalyzable function `f`. Use an iteration pattern that the compiler can analyze, or give the
> loop a name and provide a measure in type position (§6.9).

This is not a compiler failure. No code that cannot be statically proven safe may compile. Even when
an explicit measure is provided, one kernel-validated certificate must be supplied for each of
decrease and well-foundedness—an incorrect measure is rejected with a counterexample. A human can
fail to prove termination, but cannot prove it incorrectly.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure in **type
position**—through exactly the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (the
application of a predicate), with zero new syntax:

```yaoxiang
// Measure: an ordinary function; unit-testable, reusable, and absent at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component is attached to the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor; the measure obtains scoped quantities by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The rationale for the loop case is that `Terminates(m)` refines the value type of the loop body's
tail expression, and the anchor is supplied by the binding name `acc`—the loop can therefore be
referred to, eliminating the “an anonymous construct cannot be referred to” corner case.

> **The body tail must provide a value**: the value of a loop = the value of the loop body block
> (§2.9), and the value of a block = the value of its tail expression. If the body tail is an
> assignment statement (such as `{ i = i + 1 }`), the block value is `Void`, and `return acc` would
> be returning `Void` from `-> Int`, which is ill-typed. The example therefore explicitly writes `i`
> as the body tail. This does not change the semantics of the measure—the measure `n - i` is a
> **state expression**, independent of the loop's value type.

> **Implementation note (recorded 2026-10-02; updated 2026-10-04)**: the **type-side** example in
> this section is valid (`check` reports 0 errors, and the back-edge obligation is Proved). The
> **runtime-value defect (#409) has been fixed**: a `while` used as a value now takes the value of
> the body block's tail expression from the loop body's final iteration, consistently with the
> type-side `block_value_ty`— `loop(4) == 4` and `countdown(3) == 30` now hold in actual execution
> (the same fix applies to `for`).
>
> **Known residual issue (zero iterations)**: when the loop body is never evaluated, there is no
> “body value from the final iteration,” and the loop value remains `Void`—binding it to a
> non-`Void` type position and using it at runtime therefore reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a conspicuous type
> mismatch, not a silently incorrect value). This is a **static approximation boundary**: the
> `block_value_ty` criterion determines the type of the tail expression, but it cannot determine
> “whether the loop executes at least once” (which would require data-flow/provability analysis).
> The type side therefore does not silently downgrade (doing so would break the value type on which
> the `Terminates` loop binding in this section depends). This residual behavior is pinned by the
> corpus `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residual issue has been confirmed as an intentionally
> retained design boundary** (decision of 2026-10-04; no fix for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) was closed with the main form fixed and
> is no longer tracked separately. See the history of the minimal reproduction and affected areas in
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Examples, Loop section, “Known
> Defect D6.” In addition, termination Strategy 1 (linear rank function) is active again in this
> round, so the order of “write an explicit measure only after automatic exploration fails” remains
> unchanged— an explicit measure is still the fallback.

**`Terminates` is a built-in predicate**, at the same level as `Int` and `Never`. It is the only
predicate **whose function body is written by the compiler**—its assertions (“the measure decreases
strictly at every recursive call site/loop back edge”) reside in the computational structure, and
user-written predicates cannot refer to function or loop bodies, so this cannot be expressed using
the predicate-definition syntax described in the “Syntax” section. The built-in surface is
consolidated under this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two separate constructs:

| Form                    | Anchor                         | Use                                                           |
| ----------------------- | ------------------------------ | ------------------------------------------------------------- |
| `Terminates(m)`         | Name of the containing binding | Self-recursive functions and loops—the default form           |
| `Terminates(FnType, m)` | Explicit function type         | Cases such as mutual recursion where the anchor is not unique |

The two are essentially the same: a termination obligation always applies to “the computation
denoted by the refined type position.”

**The measure does not constrain the return type.** The measure may be an expression of any type (it
is not required to be a natural number); “strict decrease” is determined by the well-founded order
available on that type. Whether the measure is well-founded—for example, whether an `Int` measure is
`>= 0`—is an **independent obligation**, just like the decrease obligation. The producer produces
candidate certificates for both and hands them to `Kernel::check` for validation; if neither can be
proved, the diagnostic **does not directly reject** the code, but suggests what to check (whether
the lower bound holds and whether recursive parameters really move in that direction).

**Relationship to automatic exploration**: an explicit measure is not a separate pipeline; it is an
input used after producer exploration fails. Once the measure is supplied, the producer still
produces candidate certificates and hands them to the same `Kernel::check` to validate decrease and
well-foundedness; if either does not hold, an error is reported with a counterexample.

See
[RFC-027a: Explicit Measures for Termination Checking](../accepted/027a-termination-explicit-measure.md)
for obligation generation, the decision pipeline, diagnostic directions, and the shared termination
measures for mutual recursion (SCC).

### 8. Certificate Producers: SMT / Tactics / Proof Search Are All Untrusted

SMT solvers, tactic scripts, and proof-search processes all play the same role in this design:
**certificate producers**. They are untrusted: the compiler does not treat their return values as
proofs, but only as hints for “finding a candidate certificate.” The only trusted component is
`Kernel::check(signature, certificate, proposition)` (§4).

#### 8.1 Trust Model

- **The kernel is the verifier**: `check` replays `certificate.steps` according to `signature.rules`
  (§4.2). It does not call solvers, parse `unsat`, or read producer logs.
- **The producer is the searcher**: given a `signature` and a `proposition`, the producer makes a
  best effort to deliver a `Certificate`. Whatever it returns or claims does not constitute
  `Proved`.
- **Prohibit `Unsat ⇒ Proved`**: `unsat` says at most that “no counterexample model was found under
  the producer's own translation and theory.” It is not a certificate and does not guarantee that
  `certificate.steps` exists. Mapping `unsat` directly to `Proved` outsources the trust root to the
  solver and directly conflicts with the pure-function kernel of §4. **This RFC explicitly prohibits
  that mapping.**
- **`sat` is not sufficient for `Disproved` either**: `sat + model` is only a candidate
  counterexample supplied by the producer; it must become a disproof certificate that
  `Kernel::check` can decide (replayed according to the signature's disproof rules, or computed by a
  kernel-direct rule showing that “the two sides differ after normalization”; see §12.1) before
  `Disproved(model)` can be returned. Translation distortion (for example, the integer-literal
  fidelity problem in issue #561) can cause `sat` to carry a spurious model; trusting it directly
  would falsely reject valid programs.
- **`unknown` / timeout / budget exhaustion ⇒ `Unproven`**: the diagnostic includes the unresolved
  proposition, reason, and supplementary-evidence hint (§12.3), and must not be downgraded to an
  internal compiler error (E8001).

#### 8.2 Data Flow from Producer to Kernel

```text
proposition + signature
        │
        ▼
┌───────────────────────────────────────────────┐
│ certificate producer（不可信）                  │
│  · 内核直判规则（结构等价、βδι、defeq）         │
│  · Γ 等式桥 / q=0 证据（RFC-011 §4.1）         │
│  · axiom library / proof library 条目           │
│  · 证明函数（显式证据，§4.3）                   │
│  · SMT-LIB 2.6 / Z3 / CVC5                     │
│  · tactics / proof search                       │
└───────────────────────────────────────────────┘
        │  候选 Certificate（steps 可能为空 / 只带 ProducerHint）
        ▼
Kernel::check(signature, certificate, proposition)
        │
        ├── steps 齐备且逐步合法、结论同一 → Proved(checked)
        ├── 证伪证书经 check 确认（或内核直判）→ Disproved(model)
        └── 证书缺步 / 规则未登记 / 哈希不符 → Unproven（补证据提示）
```

Producers may run in parallel, may cache, and may switch backends; none of this freedom changes the
single requirement that they “deliver a replayable certificate.” SMT-LIB 2.6 remains the translation
target (an ISO standard supported natively by Z3, CVC5, MathSAT, and Yices), with Z3 as the default
backend, CVC5 as an alternative, and switching at compile time—**SMT-LIB is an input format for
producers, not a trust boundary**. There is no “general solver abstraction layer” and no “general
producer abstraction layer”: a producer has exactly one output contract, “deliver a `Certificate`.”

#### 8.3 From `unsat` to a Legal Certificate: Registration of Translation Rules

For `unsat` to become useful, the producer's theory must first be folded into rules registered by
the `signature`. For example, the linear-arithmetic fragment registers a set of `linarith` rules in
the manifest of a `kind=Axiom` library in `libraries` (including their applicable sorts and premise
shapes); the SMT-LIB `unsat` result is expanded by a **proof-producing adapter** into a sequence of
`ProofStep`s using those rules and then handed to `check`. The adapter is itself an untrusted
producer: the steps it produces are still replayed one by one.

This requirement transforms “trust Z3's `unsat`” into “trust that the `linarith` rules are
registered in the signature and that every certificate step is inspectable.” Translation bugs are no
longer left to the hope that “some other test will expose them”—if the adapter produces a
certificate with a missing step or an incorrect rule, `check` immediately returns `Unproven`. The
`unsat` result for an unregistered theory must never enter the `Proved` path.

Before the proof-producing adapter is implemented, a **conservative downgrade** is permitted:
arithmetic fragments use kernel-direct rules (structural equivalence, constant folding,
βδι-reduction) and registered proof-library entries; the producer's `unsat` is attached to the
candidate certificate only as a `ProducerHint`, and does not independently trigger `Proved`. The
downgrade surface is smaller than trusting `unsat` outright, and there is no silent pass.

#### 8.4 Budgets (The Producer's, Not the Kernel's)

Budgets constrain producer search, not the correctness of `check`—`check` is a deterministic replay
whose step count is determined by the certificate's length.

| Budget dimension               | Default    | Description                                                                           |
| ------------------------------ | ---------- | ------------------------------------------------------------------------------------- |
| Search steps                   | 10,000     | Maximum number of solver/search steps for a producer; exceeding it returns `Unproven` |
| Time                           | 100ms      | Maximum search time for a single obligation                                           |
| Quantifier-instantiation depth | 3          | Maximum depth of nested quantifiers translated by a producer                          |
| Certificate replay steps       | Hard limit | Maximum `steps.len()` for `check`; exceeding it ⇒ `Unproven`                          |

Exceeding the budget returns `Unproven`, producing a compile error + predicate location + a one-line
budget summary (§12.3). There is no downgrade, no runtime checking, and no silent pass.
Dependent-type pre-reduction (`factorial(5) → 120`, `append([1,2],[3]) → [1,2,3]`) uses
kernel-direct rules and consumes no producer budget.

#### 8.5 Relationship to the Three-Valued Result, Γ, and `q=0` Evidence

Constant-equality facts in `Γ` (§12) and the four sources of evidence for `q=0` slots (RFC-011 §4.1:
literals, value environments, single-value pinning, and Γ equalities) are all `EvidenceRef` sources
available to producers; they do not constitute another decision channel. §3.5's “path conditions are
background knowledge” is refined here to “path conditions are the local `Γ` context of the
`signature`, passed to `check` together with the `Certificate`'s `context` field.”

#### 8.6 Current Implementation Gap: `predicate.rs` Trusted SMT Outright

This section's design is inconsistent with the current production implementation. The gap is
recorded honestly rather than pretending that the design has already landed:

- `try_implication` in `src/frontend/core/sema/layers/predicate.rs` maps `SMTResult::Unsat` directly
  to `Some(Ok(()))`; `try_smt_solve` maps `SMTResult::Unsat` directly to `Ok(())`. `Ok(())` is the
  `Proved` arm of `ProofResult`—this is **trusting `unsat` outright**, with no `Certificate`
  constructed or replayed.
- In the same file, `SMTResult::Sat { model }` directly constructs `ProofFailure::Disproved`; the
  counterexample is not validated by the kernel according to the signature. When translation is
  distorted (issue #561), this can falsely reject valid programs.
- `ProofContext` (`src/frontend/core/sema/proof/context.rs`) currently carries only
  `env / assumptions / dep_graph / budget / dep_env` and has **no `LogicSignature` field**;
  `ProofFailure` and `ProofResult` are three-valued results, but there is no `Certificate` type.
- The current pipeline is therefore “the producer's return value = the decision result,” whereas the
  target pipeline is “the producer delivers a certificate → `Kernel::check` → the decision result.”
  These are not differences of wording but differences of trust model: the former treats Z3's result
  as a trust root, while the latter treats the signature and kernel as trust roots.
- Direction for repair (no code is written in this RFC): ① introduce `LogicSignature` and
  `Certificate` representations and carry the current signature in `ProofContext`; ② change
  `try_implication`/`try_smt_solve` to “produce a candidate certificate,” with `unsat` serving only
  as a `ProducerHint`; ③ make `Kernel::check` the sole exit to `Proved`, and ensure `Sat`
  counterexamples are also confirmed by `check`; ④ register translation rules such as `linarith` in
  accordance with §8.3. Implementation proceeds through RFC-039 D62's “mandatory evidence / complete
  Π/Σ” stage; this RFC defines the contract only.

#### 8.7 wasm and Backend Differences

When Z3 is unavailable under wasm (the SMT code is excluded by `cfg(not(target_arch = "wasm32"))`),
the producer has one fewer search path and downgrades to `Unproven` (conservative, with fewer
reports). This is unrelated to “whether signatures agree”: under the same `signature`, native has
one more producer candidate, wasm has one fewer, and the behavior of `Kernel::check` is exactly the
same. The existing native/wasm difference table (§Open Questions) is now interpreted in terms of
“producer availability,” not “decision precision.”

Programmers do not need to know that SMT exists. The mental model is: **the compiler passes if it
can validate a certificate under the current signature; otherwise it reports Unproven and suggests
supplementary evidence—if an automatic producer cannot find one, write a proof function (a
first-class form of explicit evidence) and let the kernel inspect it.**

### 9. Composition of Compile-Time Predicates

Compile-time predicates are functions returning Type, and composition is implemented naturally
through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
}
```

### 10. Code Examples

#### 9.1 Safe Division

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ The compiler validates Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ The compiler validates Positive(0) = { 0 > 0 } → False
```

#### 9.2 Safe Array Access

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ The compiler validates InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ The compiler validates InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
```

#### 9.3 Sorting Correctness

```yaoxiang
Sorted: (T: Ord, arr: Array(T)) -> Type = {
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

sort: (T: Ord) -> ((arr: Array(T))) -> (result: Sorted(T, result)) = {
    result = arr.clone()
    # ... sorting algorithm implementation ...
    return result
}
```

#### 9.4 Loops: Compiler VC Generation

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0
    mut i: UpTo(arr.len) = 0
    while i < arr.len {
        s += arr[i]
        i += 1
    }
    return s
}
```

### 11. dispatch Routing Pipeline: Unified Compile-Time and Runtime Dispatch

> **Implementation note (recorded 2026-10-03, #377-2)**: the **semantics** of this section are in
> effect and integrated into production, but the **structure** is not a standalone
> `layers/dispatch.rs` module. Production dispatch is implemented through the **three call-site
> obligations** in `checker.rs`—binding-position revalidation (`revalidate_refined`), call-site
> actual arguments (`check_call_arg_refinements`), and the return-position postcondition
> (`check_return_refinement`). Of these, **two directly invoke**
> `layers::predicate::check_predicate`: call-site actual arguments and the return-position
> postcondition share the sole decision unit `judge_refined_obligation` (`checker/refinement.rs`),
> while `revalidate_refined` uses its own E2030 semantics. A **fourth obligation surface exists in
> the proof executor** (`proof_execution.rs`, wired on 2026-11-14, issue #537): when executing a
> proof call, it first expands **the called proof function's own formal-parameter refinements**
> (RFC-027 §3.3 nested obligations for call-site preconditions—a breadth-first worklist with a
> deduplication set and the §8 budget cutoff; a closed Bool constraint that is disproved reports
> E4018, and Call-shaped items are enqueued for continued execution). The original
> `layers/dispatch.rs` had zero production call sites, and its handling of
> `Unproven { ProofFunctionRequired }` was “downgrade it to a W1080 warning and inject Γ,” which
> conflicts with the production implementation (`Unproven` always reports an error; see
> `refined_unproven`), so the module was removed in its entirety. Note also that the Runtime row's
> “insert a runtime check” in the table below has **no emission point in production**
> (`RuntimeOutcome::InsertCheck` has no consumer), while Γ injection is independently handled by
> branch guards in `ownership.rs` and by `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement-type primitive. The dispatch pipeline
`dispatch` automatically decides between compile-time proof and runtime checking according to
**whether the predicate's free variables are accessible at compile time**:

| Criterion                                                                                      | Mode            | Behavior                                                                                                                                                                                        |
| ---------------------------------------------------------------------------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants)      | **CompileTime** | Enter the proof pipeline: the certificate producer delivers a candidate certificate → `Kernel::check` → Proved erases / Disproved is a compile error / Unproven requests supplementary evidence |
| Some free variables come from runtime (function parameters, external input, mutable variables) | **Runtime**     | Insert a runtime check and inject the refinement fact into the flow-sensitive assumption set Γ                                                                                                  |

**Key point**: “cannot decide” ≠ “disproved.” In CompileTime mode, Unknown requires proof (no silent
downgrade); in Runtime mode, the proposition has no truth value at compile time in the first
place—no prover, however strong, can write a universally true proof for “the user might supply a
negative number,” so a runtime check is the only sound choice. This is not a weakness of the prover;
it is a mathematical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest-Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ that tracks the propositions known to hold
at each control-flow point.

**Strongest-postcondition (SP) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for mutable variables**: after a `mut` variable is reassigned, every assumption involving
that variable is removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard soundness requirement—once a variable's value changes, old assumptions no longer
hold.

**Branch confluence**: when IF/ELSE or match branches are merged, Γ takes the intersection of the
branch assumptions. Only propositions that hold on every path leave the branch.

**Equality bridge (automatic evidence for the `q=0` slot)**: constant-equality facts in Γ are valid
evidence sources for compile-time value parameters (RFC-011 §4.1, `q=0` slots). Within the guard
branch of `if y == 42 { f(y)(y) }`, Γ records `y == 42`, so y is treated as 42 at the q=0 slot, and
the call follows the existing check path for `f(42)`. One side of the equality must be a
compile-time constant; filling a q=0 slot with a runtime value that has no equality evidence is
still reported at the call site. This is not a new mechanism—the validation condition
`{y == 42} ⇒ {y ≡ 42}` follows the existing implication decision procedure.

#### 12.1 Definition of Equality (defeq)

This section was added by RFC-039 decision D62: the equality bridge above supplies an evidence
source for q=0 slots, while this section completes the definition of equality itself, the role of
Unproven in type equality, and the diagnostic shape.

Equality is the sole equivalence relation on types. It is given by exactly one rule: after each side
is normalized, the two sides are **identical**.

- **Normalization** consists of three kinds of reduction: δ unfolds type aliases and definitions; β
  reduces type families and dependent-type applications; ι normalizes literal types—the identity of
  a literal type is determined solely by its value, not by source spelling (`03` and `3` are the
  same type).
- **Identity** is fully structural: type constructors have the same name, and arguments are
  identical one by one. Equality does not inspect cache keys or rendered text, and performs no
  reasoning about refinement predicates.
- **Subtyping is an independent relation**: `A <: B` is decided by the subtyping rules; the fact
  that both `A <: B` and `B <: A` hold does not define equality. Bidirectional subtyping would mix
  covariance and interface duck-typing rules into equivalence, contaminating the distinction between
  “assignable” and “equivalent.”
- **Different `Refined` predicates ⇒ Unproven**: when the base types are identical but the
  predicates differ, the compiler returns Unproven rather than Disproved. Predicate equivalence
  belongs to the assumption-set Γ layer (the equality bridge above and the Γ equality rules that
  follow), and until that layer is implemented, the compiler can only say “cannot determine.”

The failure surface of equality is consistent with the three-valued algebra of §4.1: both sides
normalize successfully and are identical ⇒ `Proved`; both sides normalize successfully but differ ⇒
`Disproved`; either side fails to normalize ⇒ `Unproven` (budget exhausted or capability boundary).
**A reduction failure does not produce `Disproved`**—inability to compute does not mean inequality.

Equality is a pure decision: it **does not bind** type variables, accepts on success, and leaves no
instantiation behind. Decision points (assignments, actual arguments, and return positions) first
use unification; after unification fails, equality may be used instead, but any partial bindings
produced by the unification attempt must first be rolled back. Without rollback, an attempted
unification judged unequal leaves bindings behind that pollute subsequent inference.

#### 12.2 Unproven ≠ False: Where Equality Lands

The three-valued algebra of §4.1 applies to every obligation point, and type equality is one of
them. This section grounds “Unproven ≠ False” at this layer.

- When an obligation point returns `Unproven`, the compiler **must** report a compile error and must
  not admit the code. The diagnostic must suggest supplementary evidence: in the refinement domain,
  write a proof function or reference a proof-library entry (§4.3); in the equality domain, provide
  a normalizable form; changing the signature or adding a bridging axiom follows the three
  cross-signature paths of §4.1.
- `Unproven` and `Disproved` must be distinguishable: `Disproved` carries a counterexample (a
  concrete assignment or two unequal sides), while `Unproven` carries the unresolved proposition and
  its reason. Rendering `Unproven` as “does not hold” is an incorrect report.
- `Unproven` **must not** be downgraded to an internal compiler error (E8001). E8001 indicates only
  a consistency failure in the compiler itself; budget exhaustion and capability boundaries belong
  to the user-domain diagnostics of compiler reasoning (RFC-013 “Message Quality Specification,”
  §5).
- `Unproven` **must not** pass silently. When a decision point ends in failure, the user's
  diagnostic must come only from an existing failure path; no second diagnostic is added, and
  inability to decide does not result in acceptance.

#### 12.3 Diagnostic Shape

An `Unproven` diagnostic must carry all three of the following; missing any one makes it incomplete:

- **Unresolved proposition**: renderable proposition text (a refinement constraint, the two sides of
  an equality, or a termination measure);
- **Reason**: a capability boundary (beyond the kernel's inference rules), budget exhaustion, or an
  unbound symbolic name—one of these;
- **Location**: source location. If a decision point genuinely has no location, use the default
  location and explain the reason in an implementation comment.

Budget details are presented as a **one-line summary** (for example, “budget 10000 steps, 10000
steps used”). The full step count, solver-internal trace, and failure shape go to the verbose
channel or logs, not to default output.

This section formally relaxes §4.1's “Unproven → compile error + unresolved proposition +
budget-consumption report”: the substance is unchanged—users must still distinguish “cannot be
computed” from “does not hold”; only the granularity of the budget-consumption report is relaxed.
The reason is that the full step count has no actionable value for the user (who cannot use it to
modify the program), and it drifts with implementation details; after limiting it to a one-line
summary, diagnostic stability and readability improve, with distinguishability guaranteed by the
“reason” field.

The “supplementary-evidence hint” for `Unproven` belongs in the diagnostic's **reason** field: a
capability boundary explains that the current `signature.rules` are insufficient; budget exhaustion
provides a one-line summary; an unbound symbolic name identifies the name for which evidence is
needed. The division of labor between the general `Unproven` result and refinement-specific codes is
specified in RFC-013's code-table draft section (E4023 / E2031).

### 13. Erasure Model: Certificates Are Retained at Compile Time and Erased at Runtime

Three kinds of “erasure” must be distinguished, because they act on different objects:

| Object                                                    | Compile time                                                                                                                       | Runtime                                                                                           | Basis                        |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ---------------------------- |
| `Certificate` (a proof term validated by `Kernel::check`) | Retained: `Proved(checked)` enters the compile-time cache for incremental reuse and reconciliation with Σ construction obligations | Completely erased: no runtime code is generated and no data slot is occupied                      | §4.1, §4.2                   |
| `q=0` compile-time value parameter / Σ witness field      | Exists as a quantified fact and evidence obligation (§14)                                                                          | Occupies no slot; `CreateStruct` contains only data fields; value-level `b.w` is rejected (E1114) | RFC-011 §4.1, D62 I4/I6, §14 |
| Bool check inserted by dispatch in Runtime mode           | Is neither a witness nor a certificate                                                                                             | Retained: executed at the value level                                                             | §11                          |

**Certificates are compile-time objects.** After `check` succeeds, `Proved(checked)` exists only at
compile time: it pins down “this obligation has been validated according to `signature.hash`,” and
the cache key contains `signature_hash + proposition + certificate`, so changing the signature
invalidates it (§4.1). The runtime neither needs nor may carry a certificate—the type-level proof
has been erased, and `q=0` slots and Σ witnesses likewise occupy no runtime layout (D62 I4 “complete
erasure,” I6 “witness erasure ≠ runtime check”).

**Runtime checks are unrelated to certificates.** When §11 dispatches to Runtime mode, it inserts a
value-level Bool check that examines a runtime value; it is not a kernel-validated proof term.
Retaining it does not violate “certificate erasure at runtime”; the two act on different objects and
do not replace one another: witness erasure, certificate erasure, and check retention.

**Consistent with the D62 unified term world**: propositions, certificates, `q=0` terms, and Σ
witnesses are all terms in the same compile-time world; erasure is a consequence of “compile-time
terms do not enter the runtime layout,” not a separate erasure rule for proof terms. See §14 for how
Σ record fields become evidence obligations, how constructors produce certificates, and how `b.w` is
used at type level. §13 and the implementation gap in §8.6 are recorded in parallel: the current
`predicate.rs` does not yet produce a `Certificate`, so “certificate retention at compile time” has
not yet been implemented.

### 14. Dependent Record Σ: Evidence Fields, Construction, and Type-Level Use

The dependent record Σ is a **record type** in the unified term world: later fields may refer to the
names of earlier fields. The records in this section are one source of obligations for the §4 proof
kernel—a constructor must supply, for every evidence field, a certificate validated by
`Kernel::check(signature, certificate, proposition)`; the certificate is retained at compile time
and erased at runtime according to §13.

This section defines the complete semantics: position rules (§14.1), construction and layout
(§14.2), generics (§14.3), cross-module export (§14.4), type-level use (§14.5), and immutability and
diagnostics (§14.6). Generics, cross-module behavior, and type-level use are all defined here rather
than deferred.

#### 14.1 Field Position Rules: The Proof-Space Criterion for Evidence

Suppose the fields of a record declaration `R`, in source order, are `f₀, f₁, …, fₙ₋₁`, and field
`fᵢ` has type `Tᵢ`. The decision has three steps: **first determine `space` (proposition family or
data family), then determine argument position (preceding / self / forward), and finally determine
the field category**. There is no cross-field inference, and categories are never guessed from field
names.

**First step (proof-space criterion, the sole criterion).** Normalize `Tᵢ` and inspect the
`SortDecl.space` of the resulting type family (§4.1):

- `space = Proof` (`Tᵢ` reduces to a proposition/refinement type/`Assert`/`Terminates`/proof-family
  result) ⇒ proceed to evidence classification;
- `space = Data` (`Tᵢ` is `Int`, `Array`, `Vec`, an ordinary record, or a **data type-family
  application**) ⇒ **data field**, regardless of how many preceding fields it references.

The `n` in `Array(Int, n)` references a preceding data field and changes the “length of the data
slot”; it does not “generate a proof.” Classifying it as a witness would erase the buffer from the
runtime layout—the misclassification this section is intended to eliminate. `space` is registered in
`signature.sorts`/library manifests, and `Kernel::check` and field classification read the same
registration.

**Second step (position criterion).** This applies only to `space = Proof` and `Tᵢ = P(a₀,…,aₖ)`,
and distinguishes three cases by the positions of the arguments:

- If all arguments are **strictly preceding data fields** `fₕ (h < i)`, **compile-time constants**,
  **compile-time value parameters** (q=0 parameters such as `Rows` and `Cols` in the `Matrix` of
  RFC-011 §4.1 / SPEC §4.3), or **type parameters** ⇒ the third step classifies the field as
  evidence. A compile-time value parameter obtains a concrete constant after instantiation and is
  counted as a compile-time constant (exactly as in SPEC §8.3.1); an uninstantiated open item
  remains for construction-point classification as `Unproven` (§14.3);
- If an argument **is `fᵢ` itself** (a self-referential refinement such as `v: Pos(v)`) ⇒ the third
  step classifies it as a **data field (admission)**: this field carries a value, and the `Refined`
  constraint merely checks the value carried by that field rather than proving a fact elsewhere;
- **Forward references** to a later field `fₕ (h > i)`, **references to another evidence field**, or
  **using `fᵢ` itself as an evidence argument while mixing it with other fields** (SelfMixed:
  `P(fᵢ, fₕ)`; pure self-reference `P(fᵢ)` is the admission case above, and the two are not
  confused) ⇒ rejected at declaration time. A `P` whose kind is a data family is a separate
  category, decided at declaration/instantiation time under §14.3.

**Third step (category criterion).** `space=Data` ⇒ data field; `space=Proof` + preceding arguments
(including compile-time value parameters/compile-time constants/type parameters) ⇒ evidence;
`space=Proof` + self-reference ⇒ data field (admission).

| #   | Shape                                                                                                                                                                                                                                                       | Decision                                                            | Quantity               | Runtime layout                                                                                                                    |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `space=Proof`, and all arguments are strictly preceding data fields/compile-time constants/compile-time value parameters/type parameters (positive example `w: Pos(v)`)                                                                                     | **Evidence/witness field**                                          | `Some(Quantity::Zero)` | Occupies no slot, does not enter `CreateStruct`, and `base` does not enter the runtime layout                                     |
| 2   | `space=Data`: ordinary value types (`n: Int`), data-type-family applications (positive example `data: Array(Int, n)`), and ordinary records                                                                                                                 | **Data field**                                                      | `None`                 | Occupies a data slot and enters `CreateStruct`                                                                                    |
| 3   | `space=Proof`, but the argument is `fᵢ` itself (self-referential refinement `v: Pos(v)`)                                                                                                                                                                    | **Data field (admission)**                                          | `None`                 | Occupies a data slot; the `Refined` constraint is checked at the construction point under the existing obligations of §3.1 / §3.3 |
| 4   | `space=Proof`, but there is a **forward reference** to a later field `fₕ (h > i)`, a **reference to another evidence field**, or **use of `fᵢ` itself as an evidence argument while mixing it with other fields** (SelfMixed; pure self-reference is row 3) | **Illegal**: rejected at declaration time, with no silent downgrade | —                      | Produces no field                                                                                                                 |

The four rows do not overlap: row 2 means “ordinary value type/data family” is always a data field;
row 3 means “proposition family with **pure** self-reference” is an admission data field; row 4
means “proposition family with an illegal position” is always rejected at declaration time.
**SelfMixed falls under row 4**: `P(fᵢ)` with pure self-reference is the admission data field from
row 3, while `P(fᵢ, fₕ)` mixes the field itself with another field as an evidence argument and is
rejected at declaration time; the two are distinguished by whether the free-name set is exactly
`{fᵢ}`. **There is no path in which “forward references count as data fields”**—a forward reference
occupies neither a data slot nor an entry in `runtime_field_index`.

**A witness may depend only on strictly preceding data fields.** The arguments of an evidence field
`fᵢ: P(fₕ,…)` must not reference another evidence field, a later field, or `fᵢ` itself as an
evidence argument (pure self-reference is the admission case in row 3 of the table, while mixing
self-reference with other fields is the declaration-time SelfMixed rejection in row 4); the kind
criterion for `P` is given in §14.3. This combination of “proof space + strictly preceding + data
fields only” ensures that, when the construction point reaches `fᵢ` in declaration order, every
argument already has a compile-time value, the proposition is closed, and a certificate can be
handed to `check`.

**Positive example (evidence) and negative example (data family):**

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }        // space=Proof

Box: Type = {                            // Positive example: w is evidence
    v: Int,          // f0 data field
    w: Pos(v),       // f1 evidence: P=Pos is Proof, and argument v is a strictly preceding data field => q=0
    z: Int,          // f2 data field; runtime index = 1 (skipping w)
}

Buf: Type = {                            // Negative example: data is still a data field
    n: Int,                 // f0 data field
    data: Array(Int, n),    // f1 data field: Array is a Data family; n only determines the length and does not generate a proof
                            // Runtime index = 1; occupies a slot and enters CreateStruct
}

Admission: Type = {                      // Self-referential refinement: data field, not evidence
    v: Pos(v),              // Proof family, but the argument is itself => admission (table row 3)
}

// Compile-time value parameters as evidence arguments (RFC-011 §4.1 / SPEC §4.3, §8.3.1)
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),   // Rows is a compile-time value parameter (q=0) => evidence
    data: Array(Array(T, Cols), Rows), // Array is a Data family => data field
}
```

`Rows` and `Cols` in `Matrix` are compile-time value parameters (they obtain concrete constants
after instantiation). `Assert(Rows > 0)` has `space=Proof`, and its argument is a compile-time value
parameter, so `_assert_rows` is an evidence field; `data` references the same compile-time value
parameters, but `Array` has `space=Data`, so it remains a data field. The criterion in B.2 for
“compile-time value parameters” is therefore clear: inspect whether it is referenced in type
position (the strength declaration in RFC-011 §4.1), and do not guess from the field name.

**`base` does not enter the runtime layout**: the `w` in `w: Pos(v)` is not “an Int value”; it is “a
proof of `Pos(v)`.” Its `Refined.base` (`Int`) is merely type information used by the kernel for
equality/normalization and must never receive a runtime slot. `StructType.fields` represents it
separately as `Vec<StructFieldInfo>{ name, ty, quantity, is_mut, has_default }`:
`quantity == Some(Quantity::Zero)` denotes a witness, while `None` denotes a data field; here,
`None` means “data field,” not “unresolved.”

**Evidence fields must not have default values (source-level rule).** An evidence field has no
runtime value to supply, and the construction view in §14.2 has no argument position for it.
Therefore, the form `field: Type = expression` must not appear on an evidence field;
`StructFieldInfo.has_default` is always `false` for a witness. Giving an evidence field a default
value (`w: Pos(v) = expr`) is rejected at declaration time (DefaultOnEvidence); it is not silently
treated as “a data field with a default value” and does not enter the constructor's default-value
filling. Data-field default rules (optional during construction / required when no default is
present) apply only to data fields; this rule is the basis for the corresponding clauses in SPEC
§3.1.1 and §8.3.1.

Representation interfaces: `ast::StructField.quantity`, `mono::StructFieldInfo.quantity`, and
`ir::TypeDeclField.quantity` have the same shape; `mono::quantity_is_witness` /
`StructFieldInfo::is_witness` / `is_witness_bearing` are the sole decision predicates; and
`mono::runtime_field_index` is the **sole mapping point** from declaration order to runtime data
order (it returns `None` for a witness). The `space` query performed before classification reads
`signature.sorts` / `LibraryRef.manifest`, not a second classification table.

#### 14.2 Constructors: Source-Level Data-Only, with Evidence Synthesized Automatically at Compile Time

**There is only one source-level construction form: `Box(v, z)` (data arguments, in declaration
order).** Evidence fields **occupy no constructor argument positions**—there is neither a form that
“passes a witness as a value” nor an “evidence argument position”; the arity of `Box` counts only
the data fields `v`/`z`, not `w`. **Evidence does not come from constructor arguments; it comes from
proof obligations generated automatically at the construction point.** Three views are thereby
distinguished:

| View                                                                              | Form        | Arity                                                                         | Who produces the evidence                                                                                                                                                        |
| --------------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source-level / compile-time construction view                                     | `Box(3, 5)` | **compile-time arity = runtime arity = number of data fields (2 here: v, z)** | The construction point automatically generates a `FieldObligation`; the producer delivers a candidate `Certificate` → `Kernel::check`                                            |
| Runtime construction view (`translator::synthesize_constructor` / `CreateStruct`) | `Box(3, 5)` | runtime arity = 2                                                             | None: `CreateStruct.operands` contains only the data-field local slots `v=3, z=5` (`z` has runtime index 1, skipping `w`), and `local_count` counts the data fields, with no `w` |

Automatic synthesis path: substitute the compile-time values of strictly preceding data fields into
`P` to obtain a closed proposition. The producer (§8: kernel-direct rules / `Γ` evidence / proof
library / SMT search) delivers a candidate `Certificate`, which
`Kernel::check(signature, certificate, proposition)` validates; `Proved(checked)` is retained at
compile time and erased at runtime according to §13.

**Default values serve data fields only.** Under the source-level rule in §14.1, evidence fields
must not have default values: the `optional` list produced by `bind_args_to_params` comes only from
the `has_default` flags of **data fields**; witness fields enter neither `param_names` nor
`optional`. The default-value filling in `translator::synthesize_constructor` and the default-field
slicing in `ir_gen` both follow non-witness field order, so they never generate a default-value
expression or zero register for an evidence field. When `Box(3, 5)` omits a data field, the
constructor fills in the data field's default value/zero value; omitting an evidence field is not a
meaningful concept—it has no argument position to omit.

**Explicit evidence is not a constructor argument; it is an independent declaration.** When the
automatic producer cannot deliver a verifiable certificate (`Unproven`), the programmer supplies
explicit evidence under §4.3—writing an **independent proof function** (whose return type equals the
proposition to be proved) or registering a named proof in a `proof library`, referenced as
`EvidenceRef::ExplicitProof` / `LibraryProof` by the `FieldObligation`. The construction expression
is not rewritten, no `Box(..., proof)` form is added, and no default value is given to an evidence
field (§14.1's DefaultOnEvidence is rejected at declaration time). **Passing one additional evidence
argument in a source-level construction** (`Box(3, 5, evidence)`) **is not allowed**: under D60's
six binding rules, the extra argument cannot be assigned to any data-parameter slot and reports an
E1010 / E1014-family parameter error rather than being silently swallowed.

`FieldObligation` must be able to represent both “no certificate has been produced yet” and “a
certificate has been validated,” so the original `evidence: EvidenceRef` (which can refer only to
explicit evidence and cannot hold a producer-synthesized `Certificate`) is split into two slots:

```text
FieldObligation {
    record         : TypeId,
    field          : FieldId,
    proposition    : Proposition,
    evidence_ref   : Option<EvidenceRef>,   // Explicit evidence supplied by the programmer; None during automatic synthesis
    certificate_ref: Option<CertificateRef>,// A certificate synthesized by the producer and validated by check; None when not yet validated
    signature_hash : ContentHash,
}
CertificateRef { id: CertId, signature_hash: ContentHash }  // Points to Proved(checked) in the compile-time cache from §13
```

The legal combinations are: `(Some(ev), None)` = explicit evidence awaiting replay;
`(None, Some(cr))` = automatically synthesized and already validated; `(None, None)` = awaiting
producer search; `(Some, Some)` is illegal (one obligation must not consume two proofs). After
`check` succeeds, `certificate_ref` is written while `evidence_ref` is retained as a provenance
annotation.

`FieldObligation` is generated during the refined walk in `checker/refinement.rs`; field binding
reuses `bind_args_to_params` (D60's six rules), and decisions reuse the sole unit
`judge_refined_obligation`. **No value is invented when proof fails**: under §4.3, supply explicit
evidence or return `Unproven` (E2031 / general E4023); there is no such thing as an incorrectly
written witness value.

A self-referential refinement field (`v: Pos(v)`) is a data field: its `Refined` constraint is
decided at the construction point under the existing call-site obligations (§3.1 / §3.3). It is not
an evidence field, does not generate a `FieldObligation`, and does not affect `runtime_field_index`.

#### 14.3 Generic Records: Proof-Family Parameter `P` (Kind `T -> Proposition`)

Types are “parameterized records + a **proof-family parameter**,” not “generic witnesses deferred
until later.” The kind of `P` comes from the same source as the proof-space criterion in §14.1:
**the result sort must be `space=Proof`**, meaning that `P` is a proof/type family (a family of
propositions), not a data family. `T -> Proposition` is kind notation (`Proposition` is an alias for
the `SortSpace::Proof` sort in §4.1), **not a source-level type name**: the `yaoxiang` block below
uses the `P: (T: Type) -> Type` form accepted by the current parser, while the kind constraint is
decided at declaration and instantiation and is not written into parseable code.

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

// Source-level form: P is a type-level function parameter; the kind constraint (result space=Proof) is explained in the prose
Cell: (T: Type, P: (T: Type) -> Type) -> Type = {
    v: T,            // Data field
    w: P(v),         // Evidence: P is a Proof family, and argument v is a strictly preceding data field
}
```

- **Kind criterion** (prose notation, not source-level syntax): in `P: T -> Proposition`,
  `Proposition` is an alias for the `SortSpace::Proof` sort (§4.1); equivalently, the application
  result of `P` has `space=Proof`. `Array: (T: Type, N: Int) -> Type` has `space=Data`, and passing
  it to `Cell` is a **kind mismatch rejected at declaration time** (the parameter declaration's kind
  is `T -> Proposition`, so binding a data family is a type error, not a reason for “`w` to degrade
  into a data field”; row 2 of §14.1 governs only fields that **directly name** a data type family).
  The multiple-argument form `(T, U) -> Proposition` follows the same rule, and all arguments must
  fall on strictly preceding data fields in declaration order.
- `P` and `T: Type` both occupy compile-time parameter positions (q=0, RFC-011 §4.1). When
  `Cell(Int, Pos)` is instantiated, `P` is bound to the concrete predicate `Pos` (`space=Proof`),
  and the proposition for `w` becomes `Pos(v)`; the position rules are exactly those of §14.1.
  `Cell(Int, Array)` is rejected at declaration/instantiation time because of the kind mismatch.
- `P` is not a value parameter and occupies no runtime slot. When `P` is unbound (the generic is not
  instantiated), the result sort of `w` remains `Proof` (the kind is declared), but the proposition
  is an open term, so the **construction point returns `Unproven`** (not a declaration-time
  rejection), without silently admitting the code or degrading it into a data field. After
  instantiation, the decision is rerun under §14.1.
- Evidence classification for a generic record is rerun once **after instantiation** under §14.1:
  the `quantity` of `StructType` remains `Some(Zero)` after `substitute` substitutes `P`; the Struct
  arms of `mono::substitute` and `types::solver` pass the field `quantity` through and do not create
  a second classification.
- Exports of the generic `Cell` carry the kind of `P` (`T -> Proposition` notation) and the
  field-obligation template (§14.4) in `TypeDefPayload`; the correspondence between the source-level
  `(T) -> Type` and kind notation is provided by the declaration/instantiation criteria.

#### 14.4 Cross-Module Export: `TypeDefPayload` Carries Proposition/Evidence Slots

When a record containing evidence fields is used across modules, its propositions and evidence slots
must be exported with the type definition, so the importer does not “re-guess them by field name.”
The carrier is `frontend::module::TypeDefPayload`:

```text
TypeDefPayload {
    // …existing fields…
    type_params      : [TypeParam { name, kind }]        // Includes P: T -> Proposition (§14.3)
    field_obligations: [FieldObligation]                 // See §14.2; [] for records without witnesses
    evidence_slots   : [EvidenceSlot {
        field          : FieldId,
        quantity       : Quantity,                       // Always Zero
        proposition    : Proposition,                    // May contain a preceding FieldPath reference
        evidence_ref   : Option<EvidenceRef],            // Explicit-evidence provenance; None during automatic synthesis
        certificate_ref: Option<CertificateRef>,         // Validated-certificate reference; None when not yet validated
        signature_hash : ContentHash,                    // Invalidated according to the §4.1 signature
    }]
    libraries        : [LibraryRef]                      // Libraries referenced by propositions/evidence; carried across modules
}
```

- Export side: when `checker.rs` constructs `TypeDefPayload`, it fills one `EvidenceSlot` for every
  field whose `quantity == Some(Zero)`; `field_obligations` and `evidence_slots` have the same
  source and are not maintained separately.
- Import side: after obtaining an `EvidenceSlot`, the importer validates it using **its own current
  `LogicSignature`**—if `signature_hash` does not match, the three cross-signature paths of §4.1
  apply (translation term/bridging axiom/reproofing), and the exporter's `Proved` must not be reused
  directly. Libraries referenced by `libraries` must occur in the importer's `libraries` list;
  otherwise the result is `Unproven`, with a prompt to register the library or write a bridge.
- Construction points in multiple files/modules reuse the same `judge_refined_obligation`;
  cross-module use does not introduce a second decision entry point (RFC-039 D55 silent-channel
  prohibition).

#### 14.5 Type-Level `b.w`: `ConstExpr::FieldPath` + Evidence Term

A witness field may be used **only at type level** (`b.w` is user-writable type-level syntax, as
described at the end of this section), and value-level reads are always rejected:

- **Term representation**: `ConstExpr::FieldPath { base: Box<ConstExpr>, field: FieldId }` is a
  field-path term in the unified term world. `b.w` in type position resolves to
  `FieldPath(NamedVar("b"), "w")`; it does not denote a runtime value, but the **evidence term**
  (proof term) corresponding to `EvidenceSlot.evidence_ref`.
- **Evidence term**: `EvidenceTerm(FieldPath, EvidenceRef, signature_hash)` is one source of
  `EvidenceRef` for a `Certificate` (§4.2), corresponding to the explicit-evidence or
  validated-certificate slot in the two slots of `FieldObligation` (§14.2). When `b.w` is used in
  another proposition (such as `Q(b.w)`), the producer uses `EvidenceTerm` as a premise of the
  candidate certificate, and `Kernel::check` replays it according to the signature; it is not reused
  if `signature_hash` does not match.
- **Type-level uses**: `FieldPath` may appear in predicate arguments, type-constructor argument
  positions, as an evidence-term premise in another proposition, and in
  `TypeDefPayload.evidence_slots[i].proposition` (indexed slot by slot). Note: when `FieldPath`
  points to a witness, it is a **proof-term premise**, not a “data-field argument”—the rule in §14.1
  that “evidence-field arguments accept only strictly preceding data fields” constrains the data
  arguments of `P`; it does not prohibit using the proof term of a preceding evidence field as a
  premise for a new proposition (the latter is replayed through `EvidenceTerm` by `check`). `b.w`
  produces no runtime value and occupies no data slot.
- **Value-level rejection**: when `b.w` occurs in a value expression (the `Struct` arm of
  `FieldAccess` and the `TypeRef→Struct` arm), report **E1114** (`witness_field_not_readable`).
  `FieldPath` is not a value term, and neither the `Evaluator` nor IR generation materializes a
  value for it.
- The exhaustive `match` completion points for `ConstExpr` (`const_eval::substitute`, `free_names`,
  `Display`, the `translate::` family, `proof_execution`, `fold_proof_call_arg`, and so on) must
  handle `FieldPath`; an evaluator whose item language does not yet support it returns `Unproven`
  upon encountering `FieldPath` and must not silently treat it as `NamedVar`.

```text
// Design sketch (non-compilable fragment: Proved / FieldPath / evidence term are kernel terms; see the interface definitions in this section)
// Type-level use: treat b.w as an evidence premise, not as a value
useWitness: (b: Box) -> Type = { Proved(b.w) }   // b.w resolves to FieldPath + evidence term
// Value-level b.w => E1114
```

> **Superseding 4A's “no user syntax at type level.”** The premise of the 4A decision (task 6.9.5's
> commit message, H0 decision, 2026-11-13) was that “`b.w` requires `ConstExpr::FieldPath` and a
> proof token and belongs to an independent term-language design; adding it now would create a
> second term world.” The decision therefore provided no user-facing `b.w` projection under V1.
> `ConstExpr::FieldPath`, `EvidenceTerm`, and `TypeDefPayload.evidence_slots` all belong to the
> **same unified term world** (§4, not a second term world), so that premise does not hold:
> **type-level `b.w` is user-facing syntax** (the `useWitness` example above). The 4A rule “no
> user-facing type-level `b.w` syntax” is superseded by this section and is no longer in effect. The
> other two 4A rules—witnesses are compile-time fields, proven at construction, and erased at
> runtime; field representation uses `quantity` + proposition type—remain in effect. Value-level
> `b.w` is still rejected under 4A (E1114). `FieldPath` exists only in type/proposition positions,
> is not a runtime field path, and does not constitute an “A-style runtime field.”

#### 14.6 Immutability, Erasure, and Diagnostic Codes

- **5A: Entire records are immutable**: a record containing any evidence field is wholly
  immutable—data fields must not be declared `mut`, assigned, mutably borrowed with `&mut`, or used
  as the receiver of a write method (including automatic `WriteBorrow`). Otherwise, a witness
  proposition could become invalid after a field rewrite, and there is currently no runtime
  check/revalidation; violations report **E1115** (`witness_record_immutable`). Finer-grained
  linearity/uniqueness designs are deferred.
- **Erasure**: an evidence field's `quantity=Zero` ⇒ it occupies no runtime slot; `CreateStruct` /
  `GetField` / `SetField` / `resolve_field_index` recognize only the data order from
  `runtime_field_index`; a `Proved(checked)` certificate is retained at compile time and erased at
  runtime according to §13; the Bool check in Runtime mode is unrelated to witnesses.
- **Division of labor among diagnostic codes** (each code covers one aspect; no code borrowing and
  no silent behavior):

| Code      | Trigger                                                                                                                              | Meaning                                                                       |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| **E4018** | An evidence proposition at a construction point is disproved (a counterexample confirmed through `check` / a kernel-direct decision) | Refinement-predicate violation, with counterexample                           |
| **E2031** | An evidence proposition at a construction point is `Unproven` (refinement channel, with `assigned`/`var`/`constraint` slots)         | The refinement constraint cannot be proved; suggests explicit evidence (§4.3) |
| **E1114** | Value-level read of witness field `b.w`                                                                                              | Witness fields cannot be read at value level                                  |
| **E1115** | `mut`/field assignment/`&mut`/write receiver on a witness-bearing record                                                             | The entire record is immutable                                                |
| **E4023** | General `Unproven` outside the refinement channel (for example, an unsupported `FieldPath` evaluator)                                | General inability to prove, with proposition/reason/location (§12.3)          |

`ProofFunctionRequired` continues to use E4020; the division of labor between general `Unproven` and
E2031 is unchanged (§12.3, RFC-013). The **declaration-time rejection list** (aligned item by item
with row 4 of §14.1 and the kind criterion in §14.3) contains four categories:

| #   | Rejected shape                                                                                            | Description                                                                                                       |
| --- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| 1   | `space=Proof` but a **forward reference** to a later field `fₕ (h > i)`                                   | Illegal position                                                                                                  |
| 2   | **Reference to another evidence field** as an argument                                                    | Illegal position                                                                                                  |
| 3   | **Use of `fᵢ` itself as an evidence argument while mixing it with other fields** (SelfMixed: `P(fᵢ, fₕ)`) | Illegal position; pure self-reference `P(fᵢ)` is the admission data field from row 3 of §14.1 and is not rejected |
| 4   | The **kind of `P` is a data family** (§14.3)                                                              | Kind mismatch; a type error at declaration/instantiation time                                                     |

**In addition, giving an evidence field a default value** (`w: Pos(v) = expr`, DefaultOnEvidence,
§14.1 source-level rule / §14.2 construction rule) is also rejected at declaration time—it has no
runtime value to supply and must not be silently treated as a data field. Items (1)–(3) above are
the illegal-position forms from row 4 of §14.1 (item 3 is SelfMixed), and item (4) is the kind
mismatch from §14.3; together they form the four declaration-time rejection categories. Default
values are an independent field-declaration check and are not counted with the four categories.

All of these items are rejected directly at declaration time, do not reach the construction point,
and are not silently downgraded to data fields. A data type-family application with `space=Data`
(including `Array(Int, n)`) and a pure self-referential refinement with `space=Proof` (`v: Pos(v)`,
admission) are both legal data fields. **An uninstantiated generic is not a declaration-time
rejection**: under §14.3, when `P` is unbound, the result sort of `w` remains `Proof` and the
proposition is an open term, so the **construction point** returns `Unproven` (E2031 / E4023),
without silently admitting the code or degrading it into a data field. The assignment of
declaration-time diagnostic codes (a new declaration-time code or reuse of E1001/E1114/E1092) is
decided during implementation round D2; this section defines only the rejection list and semantics
and does not reserve a code.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                                      | After (this RFC)                                                                                                                                                          |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`                   | Compile-time predicate as a parameter type `(b: Positive(b))`                                                                                                             |
| `//! ensures: ExistsMax(result, arr)`                 | Return type uses a return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                                     |
| `/*! invariant: ... !*/`                              | Compile-time predicate type annotations on variables—Floyd-Hoare invariants                                                                                               |
| `//! decreases: n`                                    | The measure is written in a refinement-type position (`Terminates`); the compiler explores automatically first and requires an explicit measure only if exploration fails |
| Specifications are comments                           | Specifications are part of the type system                                                                                                                                |
| `/*! invariant: ... !*/` and no other record evidence | Dependent record Σ: evidence fields (q=0) and data-family fields are distinguished by the proof-space criterion; see §14                                                  |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}` is proof space and uses exactly the same
syntax as existing type definitions. A compile-time predicate is simply a function returning
Type—`name: (params) -> Type = { assertions }`. Usage is an ordinary function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type; {} contains assertions validated by the compiler
# Uses the existing function/type syntax and requires no new BNF rules
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**The arguments of a predicate application must have compile-time-constant shapes**—literals,
variables (bound by name), type applications (recursively extracted), or
**compile-time-dereferenceable function references** (function names). An argument shape that cannot
be converted to a constant expression reports **E1092**, and an argument count that does not match
the predicate declaration's formal-parameter count reports **E1093**. Arguments are bound
positionally to the formal-parameter list, and predicate arity is determined by the
declaration—`Positive(x)` is unary, `IsMax(T, arr, result)` is ternary, and `Terminates(m)` and
`Terminates(FnType, m)` are unary and binary. Refinement constraints are **never silently
discarded** (previously, an argument that could not be converted caused the constraint to disappear
silently, allowing a binding that violated the constraint to pass silently).

**New syntactic concept: the return-value formal parameter**—in `-> (name: Type)`, `name` is the
return-value formal parameter.

The return-value formal parameter is one of the syntactic concepts YaoXiang introduces over its
existing function syntax (record evidence fields reuse the existing field syntax; see §14). Its
semantics are:

- The value of `name` is supplied by a `return` statement
- `name` exists only in the type signature and is referenced by postcondition predicates (such as
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function-body scope and does not appear at the call site
- The return-value formal parameter is **optional**—without a postcondition, the signature is
  identical to that of an ordinary function (`-> Int`), with no additional burden

The reason for introducing it is that a postcondition needs to refer to “the value the function is
about to return.” Without a return-value formal parameter, the compiler could only let a predicate
refer to the return value through a special rule, such as an implicit variable `$result` or
`__retval__`. The return-value formal parameter makes that reference explicit—it is simply a formal
parameter whose value is supplied by `return` rather than by the caller.

**A proof function** is not a new concept—it is simply a YaoXiang function whose return type is the
proposition being asserted. When the compiler returns Unproven, the programmer supplies a proof
function, and the type checker validates it in exactly the same way it validates the return type of
any function. No new syntax, keywords, or rules are needed.

> **The boundary between the two.** In the correctness domain (predicate Unproven), a proof is
> supplied in the **body**: write a function whose return type is the proposition to be proved. In
> the termination domain (§6.9), the measure is supplied in **type position**: write
> `Terminates(measure)` as the type of a binding or function signature. The former means “write a
> proof for a proposition that cannot be proved,” and the latter means “explicitly declare a measure
> that cannot be found by exploration”; the mechanisms are identical (both are refinement-type
> applications), but their locations differ. The termination domain **does not need** a `_proof`
> function.

### Type-System Impact

- **Type universe**: compile-time predicates reside at the Type₂ level—they accept values and return
  Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates may carry generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules and may
  only be read, not written
- **Type inference**: parameters of compile-time predicates participate in HM type inference
- **Dependent record Σ**: record fields are divided into data fields and evidence fields
  (`Quantity::Zero`) under the proof-space and position rules of §14; the proof-family parameter
  `P: T -> Proposition` and `TypeDefPayload.evidence_slots` provide complete semantics for generics
  and cross-module use

### Runtime Representation

Compile-time predicates are handled at runtime according to the dispatch result:

- **CompileTime mode** (all free variables are known at compile time): after `Kernel::check`
  validates the certificate, `Proved(checked)` is retained at compile time and completely erased at
  runtime. In `Positive: (x: Int) -> Type = { x > 0 }`, the runtime representation of the parameter
  `b: Positive(5)` is simply `Int`; the refinement condition `{ 5 > 0 }` has passed, and the
  certificate does not enter the runtime layout. Σ evidence fields follow the same rule under §14:
  `base` does not enter the layout, `CreateStruct` is data-only, and value-level `b.w` reports
  E1114.
- **Runtime mode** (some free variables are runtime values): retain the runtime check—execute a Bool
  check at the value level and inject into the flow-sensitive assumption set Γ. See §11's dispatch
  routing pipeline and §13's erasure model.

Placing a compile-time predicate in a type position (such as `f(x: Positive(x))`) produces no
wrapper type and allocates no extra memory. However, when `x` comes from runtime input, a runtime
Bool check **is** inserted.

**Interaction constraint with `ref`**: compile-time predicates may refer only to immutable borrows
or values whose ownership has been transferred. The compiler cannot guarantee at compile time that a
predicate referring to a mutable borrow remains valid at runtime— such uses report a compile error
directly.

### Compiler Changes

1. **Parser**: compile-time predicates and proof functions use standard function syntax and require
   no additional parsing rules
2. **Proof kernel**: `Kernel::check(signature, certificate, proposition)` and `CheckVerdict`;
   replaying `Certificate.steps` according to `signature.rules` is the sole exit to `Proved`
3. **Logic signatures and libraries**: `LogicSignature` (sorts / rules / axioms /
   `libraries: [LibraryRef]` / hash), with each `LibraryRef` carrying `kind/id/manifest/hash`; the
   signature hash covers all libraries; signatures are replaceable, and cross-signature use requires
   explicit translation or bridging axioms
4. **Certificate producers** (untrusted): kernel-direct rules, Γ/`q=0` evidence, proof functions,
   SMT-LIB 2.6 translation (Z3 by default, CVC5 as an alternative), tactics, and proof search; all
   deliver only candidate `Certificate`s, and `unsat` never leads directly to `Proved` (§8)
5. **Validation-condition generation**: WP/SP calculus, loop invariants, and the `FieldObligation`
   for record construction in §14 (field position rules / `bind_args_to_params` /
   `judge_refined_obligation`), producing propositions and candidate certificates without deciding
   them itself
6. **Error reporting**: counterexamples (confirmed through `check`) + unresolved proposition +
   reason + supplementary-evidence hint + source-location association (§12.3)

### Backward Compatibility

- ✅ Code that does not use compile-time predicates remains completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; Runtime mode retains
  only the necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden
- ⚠️ Record types gain the evidence-field position rules of §14: programs in which “a field name
  happens to be a predicate application and is silently treated as a data field” now receive
  compile-time witness classification (value-level reads report E1114; witness-bearing records are
  immutable under E1115). This is an externally visible behavior change, registered as a D/E-class
  change under RFC-039 D59.

## Trade-offs

### Advantages

- **Complete realization of the Curry-Howard isomorphism**: types are propositions, programs are
  proofs, and `name: Proposition = Proof`; propositions and certificates inhabit D62's unified term
  world
- **Uniformity**: compile-time predicates and ordinary functions use exactly the same syntax, with
  no conceptual split
- **Producer transparency**: programmers do not need to know that SMT exists; the mental model is
  “supply explicit evidence → the kernel validates it according to the signature”; the backend is
  replaceable
- **Gradual adoption**: begin with one compile-time predicate and progressively expand coverage
- **Minimal runtime overhead**: certificates and `q=0` terms are retained at compile time and erased
  at runtime; Runtime mode retains only the necessary Bool checks

### Disadvantages

- **Compilation time**: certificate-producer search and kernel replay both add compilation time, but
  producer budgets (§8.4) and a certificate-step limit keep this under control
- **Limits of automatic proof**: when a producer cannot deliver a verifiable certificate, the kernel
  returns `Unproven` and suggests supplementary evidence. This is not a language defect—it is an
  inevitable consequence of the halting problem. The kernel honestly distinguishes “cannot be
  computed” from “does not hold”
- **Learning curve**: writing effective compile-time predicates and proof functions and choosing
  logic signatures requires understanding the Curry-Howard isomorphism and the basic intuition of
  “signature-relative” reasoning
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard producer budget limits (10,000 search steps / 100ms / instantiation depth 3) and a
  certificate-replay step limit; exceeding the budget returns `Unproven` (§8.4)
- Dependent-type pre-reduction: deterministic value computation is handled first by kernel-direct
  rules, and producers address only the nondeterministic remainder
- `Unproven` is not a dead end: supply explicit evidence (a proof function, a proof-library entry, a
  different signature/bridging axiom; §4.3); in the termination domain, provide a measure in type
  position (§6.9)—all are validated by `Kernel::check`
- Incremental validation: `Proved(checked)` is cached according to
  `signature_hash + proposition + certificate`, and only modules whose signature changed or whose
  certificates became invalid are recomputed
- Clear error messages + counterexamples confirmed through `check` + unresolved proposition +
  reason + supplementary-evidence hint + a one-line budget summary (§12.3)

## Alternatives

| Alternative                                                        | Why it was not selected                                                                                                                                                                                                                                            |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| RFC-022: comment-based `//!` specifications                        | Splits specifications from types, violating the Curry-Howard isomorphism                                                                                                                                                                                           |
| Separate specification files (such as CVL)                         | Separates specifications from code and increases maintenance costs                                                                                                                                                                                                 |
| Runtime assertions only                                            | Cannot statically guarantee correctness                                                                                                                                                                                                                            |
| External proof assistants (such as Coq)                            | Separate from the compiler and require an independent proof language and trust boundary. YaoXiang's choice is that a proof is YaoXiang code, and certificates are validated by `Kernel::check` according to a replaceable signature, with no hard-coded trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                                                                                                                 |

## Implementation Strategy

### Phase Breakdown

| Phase       | Contents                                                                                                                                                                                                       |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | `Kernel::check` + `LogicSignature` + `Certificate` + `CheckVerdict`; kernel-direct rules (structural equivalence, βδι-reduction, defeq)                                                                        |
| **Phase 2** | Manifests and hashes for axiom/proof libraries; integration of proof functions and explicit evidence; a proof-producing adapter expands `unsat` into `ProofStep`s for registered rules                         |
| **Phase 3** | Loop-invariant VC generation + termination checking (§6, §6.9) + the field position rules, construction obligations, and type-level `FieldPath` use for dependent record Σ (§14), all through the same `check` |
| **Phase 4** | Certificate caching by `signature_hash` + incremental validation + cross-module evidence slots in `TypeDefPayload` + cross-signature translation/bridging axioms + IDE support (§14.4)                         |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates build on `name: type = value`
- RFC-011: Generic System — compile-time predicates may carry generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **Z3 availability for the wasm32 target**: Z3 is unavailable under wasm (all SMT code is
      excluded by `cfg(not(target_arch = "wasm32"))`). Scope and downgrade direction: -
      `ownership.rs` `smt_cut` always returns `false` → back-edge traversal → **conservative
      rejection** (the sound direction, merely with less narrowing) - Termination-check Strategy 1
      and predicate levels 2b/3 are skipped entirely

  **Correction (2026-10-01)**: the original text described these paths as “currently
  **unreachable**” (on the grounds that `predicate_defs` was never populated in production, `parser`
  never produced `MonoType::Refined`, and production `with_solver` was never called). That is no
  longer true—all three have landed (predicate-definition registration #377-3; refinement-type
  normalization; production-pipeline solver injection). The wasm/native difference is therefore no
  longer limited to the precision of `smt_cut`:

  | Path                                            | native   | wasm       | Downgrade direction                   |
  | ----------------------------------------------- | -------- | ---------- | ------------------------------------- |
  | Explicit termination-measure decision           | Decided  | `Unjudged` | Does not report E4022 (fewer reports) |
  | Well-foundedness decision                       | Decided  | `Unjudged` | Does not report E4022 (fewer reports) |
  | SMT implication level for refinement predicates | Executed | Skipped    | Falls back to `Unproven`              |

  All are **conservative** directions (one fewer producer path ⇒ fewer candidate certificates ⇒
  `Unproven`; `Kernel::check` itself is unchanged), and do not affect soundness. Under the
  terminology of §8.7, the table below is interpreted in terms of “producer availability,” not
  “decision precision.” To obtain consistent precision, see issue #376 for options and costs (the
  conclusion is to prioritize a JS-side Z3 instance rather than linking Z3 into the main wasm
  build—the latter requires replacing the emcc build system and grows the artifact from 3 MB to ~20
  MB).

- [x] **SMT solver selection**: Z3 by default (MIT license, the broadest verification support). CVC5
      as an SMT-LIB-compatible alternative, switched through a compiler flag. SMT-LIB 2.6 is the
      producer's translation target—it is an input format for candidate certificates, not a trust
      boundary; the sole exit to `Proved` remains `Kernel::check` (§8). No custom general solver
      interface is implemented, and `unsat` is never treated as a decision result.
- [x] **Specific solver-budget values**: 10,000 producer search steps / 100ms /
      quantifier-instantiation depth 3, plus a separate certificate-replay step limit (§8.4). These
      are fixed internally, with no knob. They are adjusted only if actual use demonstrates
      insufficiency (as opposed to “the user wrote it incorrectly”).
- [x] **Quantifier support range**: the language imposes no limit on quantifier order. Compile-time
      predicates accept Type parameters—and Type includes function types—so higher-order quantifiers
      are a natural consequence of the type system and require no special syntax. Producers can
      handle first-order quantifiers (forall/exists, including interleaved nesting, limited by
      budget depth 3) and deliver candidate certificates; when a higher-order quantifier cannot
      yield a verifiable certificate, the kernel returns `Unproven` and suggests explicit
      evidence—write a YaoXiang proof function whose return type equals that proposition (§4.3), and
      `Kernel::check` validates it under the current signature. No external export, no AI, and no
      interactive proof mode are needed. Everything is YaoXiang code, and everything is validated by
      the proof kernel.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefixes to avoid conflicts). The `sat + model` returned by a producer is a
      candidate counterexample and must be confirmed by `Kernel::check` before the proposition is
      rendered as false. Output format: variable name = concrete value + source location +
      predicate-definition location. No complex mapping layer is implemented.
- [x] ~~**Interaction between compile-time predicates and the `ref` smart pointer?**~~ → Decision:
      compile-time predicates permit only immutable borrows or values whose ownership has been
      transferred. A value under a mutable borrow cannot occur in a compile-time predicate.
- [x] **Extending the forall-predicate violation-count measure to non-adjacent operations?** → Not
      extended. The current coverage (adjacent swaps and adjacent movement) is complemented by
      Strategy 1 (linear rank function)—the outer interval contraction of quicksort is covered by
      Strategy 1, and heap sort is covered by Strategy 1 (array-index pattern). A loop that no
      strategy can prove terminating is rejected directly by the compiler—this is the hard-safety
      philosophy, not a defect. If a real-world scenario (rather than an academic construction)
      arises that none of the four strategies can cover, the matter will be reconsidered. →
      **Reconsideration was triggered (2026-09-14, #318)**: non-structural recursion (gcd-like
      recursion without direct decrease, mutual recursion, and merge partitioning) is precisely the
      kind of real-world scenario this clause anticipated—the template sequence for measure
      exploration cannot handle it automatically, and it cannot be rewritten as an analyzable
      iteration pattern without sacrificing readability. Conclusion: the hard-safety philosophy
      remains unchanged (an incorrect measure is still rejected by a counterexample confirmed
      through `check`; a human can fail to prove termination but cannot prove it incorrectly), but
      “failure to explore ⇒ rejection” is relaxed to “failure to explore may be addressed by an
      explicit measure supplied by the programmer in type position”—see §6.9.
- [x] **Combinatorial explosion in linear-rank-function enumeration**: the candidate-enumeration
      limit is three bounded variables. At ≤3, all linear combinations are enumerated and candidate
      certificates are delivered to `Kernel::check` one by one. At >3, only single-variable measures
      (`v_i`, `u_i - v_i`) are attempted; failure reports a compile error immediately, with a prompt
      for the programmer: “the loop has >3 bounded variables, so the automatic producer cannot
      synthesize a multivariable measure; please supply explicit evidence.” This is not an
      engineering compromise—it is designed to force programmers to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-011: Generic System Design](011-generic-type-system.md)
- [RFC-009: Ownership Model](009-ownership-model.md)
- Howard, W. A. (1969). The Formulae-as-Types Notion of Construction.
- Swamy, N. et al. (2016). Dependent Types and Multi-Monadic Effects in F\*. _POPL 2016_.
- Vazou, N. et al. (2014). Refinement Types for Haskell. _ICFP 2014_.
- Leino, K. R. M. (2010). Dafny: An Automatic Program Verifier for Functional Correctness. _LPAR
  2010_.
- De Moura, L. & Bjørner, N. (2008). Z3: An Efficient SMT Solver. _TACAS 2008_.

---

## Lifecycle and Final Destination

```
┌─────────────┐
│   草案      │  ← 作者创建
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 当前状态：社区讨论
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│ accepted/   │    │  rejected/  │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```
