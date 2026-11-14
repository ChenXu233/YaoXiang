---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-07'
updated: '2026-11-14'
impl_status: 'in-progress'
impl_detail:
  'Phases 1-2 are complete, Phase 3 is partially complete, and Phase 4 is partially complete. All
  Phases of the unified assert/Assert solution 6 have been implemented (#157-#162 closed): Never
  type, IsTrue bridge, flow-sensitive Γ + kill set, type-level recursion, and a weak
  universe-stratification check. The dispatch dispatch **semantics** in §11 have been implemented
  through call-site obligations (`checker.rs` has three direct calls to `check_predicate`: binding
  site / call-site argument / return site), but the **standalone `layers/dispatch.rs` module has
  been deleted**—it had zero production call sites, and its handling of `Unproven` (downgrading to a
  W1080 warning) conflicted with the production implementation (always reporting an error) (#377-2).
  The proof kernel, logical signature, certificates, and SMT roles have been rewritten in accordance
  with the unified term world of RFC-039 D62 (§4, §8, §13): SMT/tactics/proof search are **untrusted
  certificate producers**, `unsat` is only a search result, and a certificate must be constructed
  and verified by `Kernel::check` before it can become Proved. The current `SMTResult::Unsat =>
  Ok(())` in `predicate.rs` is the gap that trusts SMT outright; see §8.6. §14 completes the
  dependency record Σ in place: proof-space field criterion, data-only constructors + automatically
  synthesized evidence, proof family parameter `P: T -> Proposition`, cross-module evidence slots in
  `TypeDefPayload`, type-level `b.w` via `ConstExpr::FieldPath`, 5A immutability, and the four-code
  division of labor.'
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

This RFC proposes introducing **compile-time predicates** into YaoXiang as first-class citizens and
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an auxiliary specification comment—it is a function. A function returning Type can
be used in type positions; the compiler calls it at compile time and checks the returned value.
Types are propositions, and compile-time evaluation is proof.

**Core argument**: the sole task of type checking at compile time is to construct and verify proof
terms. Type equalities, token conflicts, dependent type reduction, compile-time predicate
evaluation, and Hoare-logic implication are all different kinds of type checking within the same
compile-time proof pipeline. They share one pipeline and one proof kernel, `Kernel::check`. Proof
terms and propositions inhabit the same **unified term world** (RFC-039 D62). A **logical
signature**, `LogicSignature`, assigns meaning to propositions, while proof terms are represented as
**certificates**, `Certificate`. A certificate is meaningful relative to a signature and is verified
only by the kernel according to that signature's rules. SMT solvers, tactics, and proof search are
all **untrusted certificate producers**: they only search for certificates, and `unsat` is not
`Proved`. When the kernel returns Unproven, the programmer writes a YaoXiang proof function as
explicit evidence—it is a proof term, verified by the kernel in exactly the same way as verifying
the return type of any function. Everything is YaoXiang code, and everything is verified by the
proof kernel.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This commits a fundamental error of Curry–Howard isomorphism: **it splits specifications and types
into two layers**. Comments are not types. Comments do not participate in type checking. Comments
embody an “external tool” mental model.

The white paper states clearly:

> “There are no `//!` comments. There is no separate specification language. Everything is within
> the type system.”

### Current Problems

- RFC-022's `//!` comments are an auxiliary syntax independent of the type system.
- Specification types and ordinary types form two separate systems, causing conceptual redundancy.
- The split between verification in Debug builds and its omission in Release builds undermines
  uniformity.
- SMT solvers are traditionally perceived as external tools, or as “trusting Z3's unsat”—YaoXiang
  instead treats them as untrusted certificate producers: they produce candidate certificates that
  are verified by the proof kernel.
- Type checking, borrow checking, compile-time predicate checking, and macro expansion all follow
  different paths.

### The Correct Mental Model

Type checking can be abstracted as a function:

```
Kernel::check(signature, certificate, proposition) → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow-conflict detection, and compile-time predicate
verification—are subtasks of this function. They share one proof pipeline and differ only in the
complexity of the proof term and the construction strategy.

Every obligation carries the currently effective `LogicSignature`. When the kernel returns Unproven,
the programmer supplies explicit evidence: a proof function whose return type equals the proposition
to be proved serves as a certificate. The kernel verifies it with the same signature—this is the
same operation as ordinary type checking. Signatures are replaceable: changing the signature changes
the inference rules, but the kernel and its three-valued result remain unchanged (§4.1).

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside it is an assertion, and the proof
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
#          Parameter at the signature  Only assertions inside {}
#          The compiler verifies x > 0 when called at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter at the signature  The compiler verifies type_of(T) == Type and type_of(data) == Array(T)
```

The pattern is always the same: `name: (params) -> Type = { assertions }`. The compiler makes no
distinction between “type assertions” and “value assertions”—both are evaluation targets in the
proof pipeline.

**Loop invariants do not need to be written separately. A type annotation on a variable is the
Floyd–Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i—it tells the compiler that s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0; verification: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # The compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # Changing i triggers re-verification of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body: induction hypothesis (the type
annotation) → assignment operation → whether the new value satisfies the type annotation. The
producer produces a candidate certificate for the induction step, and once `Kernel::check` succeeds,
all iterations are covered automatically. No `: decreases`, no `: Invariant`, and no separate
induction proof are needed—the induction is decomposed into a local VC for each assignment.

### 2. Preconditions/Postconditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates become type annotations on
parameters or return types.

**The parameter side is a function call.** A compile-time predicate is a function returning Type,
and using it on the parameter side means calling it—just like `factorial(5)`. The return-value side
introduces a new concept: a return-value formal parameter.

```yaoxiang
# Precondition: explicitly invoke a compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name and is passed to Positive
#                       The compiler extracts the argument value at the call site, substitutes b, and verifies Positive(argument)
#                       Example: divide(10, 2) → verifies Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verifies Positive(0) = { 0 > 0 } → False → compilation error

# Postcondition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is a return-value formal parameter, and its value is provided by return
#                                            The compiler substitutes the returned value at the return point and verifies the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current formal parameter name and is passed to
  `Positive`. This uses ordinary function-call syntax, with no implicit behavior.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is a return-value formal parameter
  whose value is supplied by a `return` statement. `result` exists only in the type signature and is
  referenced only by predicates; it does not enter the function body's scope and does not appear at
  the call site.
- **The return-value formal parameter is optional**: without a postcondition, omit it, and the
  signature is identical to an ordinary function (`-> Int`).
- **Uniformity**: parameters and return-value formal parameters are the same
  concept—`formal parameter name: predicate call(arguments)`—and differ only in whether the value is
  supplied by the caller or by `return`.

> **Implementation note (2026-10-02)**: a return-position formal parameter is recognized **strictly
> by its declared name**—the `r` in `-> (r: P(...))` is the sole return-value formal parameter.
> Other free variables in the constraint that are **not in scope** are no longer substituted with
> the return value; instead, an undefined identifier is reported (observed:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` with `m` undefined →
> `error [E1001] Unknown variable: 'm'`). Predicate-call arguments are substituted into the
> constraint according to the **actual arguments**: `P(r + 100)` substitutes to “return value +
> 100,” not “return value.”
>
> **Known gap (2026-10-02)**: **multi-parameter predicates with symbolic arguments in return
> position are not yet supported**. Triggering form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Alternative: use a compile-time-foldable literal as the argument (for example, `SumUpTo(3, r)`),
> or provide a proof function for the predicate that returns `Type`. The gap is removed when the
> proof kernel supports argument substitution for “multi-parameter predicate + symbolic argument.”
> See [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Implementation Record
> (2026-10-02, this hardening round).

### 3. Path-Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used in a binding position, the programmer explicitly supplies its
arguments. When a runtime value enters a refinement-type parameter, the producer collects path
conditions and produces a candidate certificate, which `Kernel::check` verifies under the current
`LogicSignature`—there is no need for the programmer to pass evidence implicitly. Explicit evidence
is a first-class form of supplementary evidence (§4.3).

#### 3.1 Explicit Function Calls

When a compile-time predicate is used in a binding position, its arguments are explicitly supplied
by the programmer—it is simply a function call, with no implicit behavior.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it occurs in
a binding position (parameter declaration, variable declaration, or return type), the programmer
explicitly supplies an already-bound variable name:

```yaoxiang
b: Positive(b)
// b has already been declared as the current formal parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler need not implicitly fill in arguments—`b: Positive(b)` is a function call just like
`f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b`
itself. This is the standard pattern for dependent types, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 established that `self` is not a keyword but merely a
naming convention for parameters (“writing `p`, `this`, or `x` has exactly the same effect”).
`b: Positive(b)` shares the same mechanism: a parameter name can be referenced in a type annotation.
`self` occurs in the position `self: Point`, and `b` occurs in the position `b: Positive(b)`; both
type annotations reference the parameter itself. The only difference is the complexity of the
annotation—the mechanism is exactly the same: after a name is bound, a type may depend on that name.

Return types use explicit function calls in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is a return-value formal parameter, and Sorted(result) is a function call
//                        The compiler substitutes the returned value for result at the return point and verifies Sorted(returned value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compilation error
```

#### 3.2 Path-Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions to form the current scope's **assumption set**. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption { y > 0 } in this branch
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline determines that the implication holds → Proved
} else {
// This branch assumes: { !(y > 0) }
// To call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline determines that the implication does not hold → Disproved
}
```

This is not a special pattern hard-coded into the compiler—it is the natural behavior of the
compile-time proof pipeline. At every type-checking call site, the pipeline receives:

```
{背景假设} ⇒ {验证目标}
```

The proof pipeline determines implication. Proved → pass; Disproved → compilation error +
counterexample; Unproven → compilation error + unsolved proposition. Background assumptions come
from the path conditions at the current program point.

#### 3.3 Assumption Stack

While analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if guard**: `if y > 0` → push `y > 0` onto the true branch, and push `!(y > 0)` onto the false
  branch (if an else branch is used)
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` within the branch
- **Logical connectives**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` within the branch
- **Function preconditions**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the argument's own refinement-type
  annotation (`b` is already annotated as `Positive`, so its type carries `b > 0`)
- **Assignment**: when `let z = y` executes, the refinement conditions already attached to `y` are
  transferred to `z`

All assumptions enter the `context` field of the `Certificate` (`Γ`) and are passed to
`Kernel::check` together with the candidate certificate. Only when the producer needs SMT does it
translate the same `Γ` into SMT-LIB background assertions.

#### 3.4 Compilation Error When No Static Evidence Exists

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no `y > 0` assumption, and `y` itself has no `Positive` type
annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (the implication does not hold) → compilation error:

> Unable to prove that parameter `b` satisfies `Positive` in the `divide` call. `y` comes from
> function input and has no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not permit a runtime value to enter a refinement-type parameter without static
evidence. This is not a restriction—it is the core of the hard-safety philosophy: code for which
`Kernel::check` cannot verify a certificate under the current signature must not compile.

#### 3.5 Relationship to the Unified Pipeline

Path-condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline to control-flow analysis:

| Stage                             | Responsibility                                                                                                                                           |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path-condition collection         | During control-flow analysis, the compiler annotates each basic block with its assumption set                                                            |
| Verification-condition generation | When a type constraint requires verification, combine path conditions with argument type information                                                     |
| Proof-pipeline decision           | The certificate producer produces a candidate certificate → `Kernel::check(signature, certificate, proposition)` → three-valued result                   |
| Result                            | `Proved` → pass; `Disproved` → compilation error + counterexample; `Unproven` → compilation error + unsolved proposition + evidence-supplementation hint |

There are no new components and no special rules. Path conditions are simply the local assumption
set `Γ` of a `LogicSignature`; they share the same pipeline and the same budget system as type
equalities and borrow constraints.

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
returns the same result; it does not read solver state, the clock, or file paths. It does exactly
one thing—replay the certificate's derivation steps according to the `rules` of `signature`,
checking each step's rule instance, premises, context, and substitutions for legality, and verifying
that the final conclusion is exactly the `proposition`. The kernel does not search, guess, or
complete certificates; if the certificate is missing a step, uses the wrong rule, or reaches a
mismatched conclusion, it must not return `Proved`.

This is the same picture as RFC-039 D62's **unified term world**: type positions and value positions
share the same set of compile-time terms; the proposition `Proposition`, proof term `Certificate`,
and the signature's sorts/rules/axioms are all terms in that set, without establishing a second
proof language. Definitional equality (§12.1) is pure decision, the `q=0` evidence (RFC-011 §4.1) is
a call-site obligation, and the Σ witness (§13) is a construction-site obligation—all three flow
into the same `check` rather than opening separate trust channels.

The three values of `CheckVerdict<Certificate>` are consistent with §4.1: `Proved(checked)` carries
the kernel-verified certificate, `Disproved(model)` carries a counterexample, and `Unproven` carries
the unsolved proposition and reason. Only `Proved(checked)` allows an obligation to pass; `checked`
is the certificate that may be cached at compile time and reused incrementally.

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

#### 4.1 `CheckVerdict`: Three-Valued Algebra and Logical Signatures

`check` returns three outcomes—the inevitable consequence of the halting problem, and a natural
partition of proof theory:

```text
CheckVerdict<C> = Proved(checked: C) | Disproved(model: Model) | Unproven(reason, subject)
```

- **`Proved(checked)`** → terminates; the certificate has been verified step by step under
  `signature`, and its conclusion equals `proposition`. The obligation passes; `checked` is the
  certificate that may be cached at compile time (§13).
- **`Disproved(model)`** → terminates; a counterexample `model` exists. Compilation error +
  counterexample + source location.
- **`Unproven(reason, subject)`** → within the current signature, evidence, and resource limits, the
  producer did not deliver a verifiable certificate. Compilation error + unsolved proposition +
  reason + **evidence-supplementation hint** (the diagnostic shape in §12.3).

**Unproven ≠ False.** The kernel saying “this cannot be computed” is not equivalent to the
proposition being false—it means only that no certificate passing verification exists under the
current `signature` and evidence set. This is honesty, not a defect.

The **logical signature `LogicSignature`** determines “what counts as a valid derivation step.” It
is replaceable data:

```text
LogicSignature {
    sorts        : Map<SortName, SortDecl>          // Categories of terms and propositions
    rules        : Map<RuleId, InferenceRule>       // Derivation rules that the kernel may replay
    axioms       : Map<AxiomId, Proposition>        // Axioms accepted by this signature
    libraries    : [LibraryRef]                     // All axiom/proof libraries; see below
    hash         : ContentHash                     // Covers sorts/rules/axioms + every entry in libraries
}
LibraryRef {
    kind         : LibraryKind                     // Axiom | Proof
    id           : LibraryId                       // Namespace for entry IDs within the library
    manifest     : Manifest                        // Entry IDs + proposition/rule shapes + provenance
    hash         : ContentHash                     // Hash of the canonical bytes of the manifest
}
SortDecl {
    space        : SortSpace                       // Proof | Data; see below
    arity        : [SortName]                      // Sort arguments accepted by this sort's constructor
}
SortSpace = Proof | Data
```

Each `SortDecl` in `sorts` carries a `space`, which is the **sole criterion for whether an
application of a type family is a proposition or data**. `space = Proof` means that inhabitants of
the sort are proof terms (propositions / refinement types / results of proof families), while
`space = Data` means that inhabitants are runtime data (`Int`, `Array`, `Vec`, ordinary records).
`Pos` (a refinement predicate such as `{ v > 0 }`), `Assert`, and `Terminates`, as well as
`proof/type family` functions returning a `space=Proof` type, all have `space=Proof`; data type
families such as `Array: (T: Type, N: Int) -> Type` have `space=Data`. The `space` is registered by
the library manifest or predicate declaration, not guessed from field names, argument shapes, or
syntax that “looks like a predicate.” The evidence classification in §14.1 reads only this field. A
data type family remains a data field even when it depends on a strictly previous field.

`libraries` is a **list**: one signature may use multiple axiom libraries and multiple proof
libraries simultaneously (for example, the built-in `linarith` axiom library + the user's `zfc-core`
axiom library + the `std-proofs` proof library + a project-private proof library). Each library
carries its own `kind`, `id`, `manifest`, and `hash`. Every entry in `axioms` belongs to a
`kind=Axiom` library by `LibraryId`, while proof-library entries belong to a `kind=Proof` library by
`LibraryId`. `signature.hash` covers `sorts/rules/axioms` **and every entry in `libraries`**, not
merely one; adding or removing any library, or changing one byte of any manifest, changes the
signature and invalidates cache keys, certificates, and incremental reuse.

- `sorts`, `rules`, and `axioms` form the signature proper; every entry in `axioms` must belong to a
  registered entry in the `manifest` of some `kind=Axiom` library in `libraries`. A certificate that
  references an axiom outside any registered library ⇒ `check` rejects it.
- Each `LibraryRef` in `libraries` has its own manifest and content hash: the manifest lists entry
  IDs, proposition/rule shapes, and provenance, while `hash` is computed over the manifest's
  canonical bytes; `kind` distinguishes axiom libraries from proof libraries. The signature `hash`
  covers `sorts/rules/axioms` and **all** `LibraryRef`s (not two).
- Built-in libraries (`Int` linear arithmetic, `Array` length, the `Assert` bridge, etc.) are
  ordinary `LibraryRef`s in `libraries` that merely ship with the compiler. They are listed
  alongside user and project libraries and are **not hard-coded trust roots in the kernel**. Trust
  derives from “this signature and its `libraries` list were adopted by the current compilation,”
  not from the name of a library or solver.

Signatures are replaceable, and replacement is data replacement rather than replacement of the
kernel:

| Signature                                    | sorts / rules values                                                   | Typical use                                           |
| -------------------------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------- |
| Classical arithmetic fragment                | Linear-arithmetic sorts + no quantifier rules                          | `x > 0`, `0 <= idx < arr.len`                         |
| Classical first order (with excluded middle) | Classical sequent rules + excluded-middle axiom                        | Classical implication, proof by contradiction         |
| Constructive                                 | Constructive rules, no excluded middle, no double-negation elimination | Extracting witnesses, avoiding nonconstructive proofs |
| ZFC fragment                                 | Set-theory sorts + registered choice/extensionality axioms             | Propositions requiring set-theoretic vocabulary       |

`Proved` is always **relative to a signature**: the same proposition may be `Proved` under a
classical signature but `Unproven` under a constructive one. A certificate carries `signature_hash`;
`check` first verifies that the hash matches and then replays the certificate; a mismatch returns
`Unproven` directly, without attempting an “approximately compatible” interpretation.

**Cross-signature use must be explicit.** Let `A`/`B` be two signatures, `P` a proposition in `A`,
and `certA` a certificate verified under `A`. To use the conclusion under `B`, there are exactly
three legal paths, each of which must leave an inspectable trace:

1. **Translation term**: provide `translate: A.Proposition → B.Proposition` and a certificate
   verifying it under `B`; obtain `Q = translate(P)`, then prove `Q` under `B`.
2. **Bridging axiom**: register an explicit bridging axiom in a `kind=Axiom` library in
   `B.libraries` (for example, “the corresponding rule of `A` has a corresponding interpretation on
   the relevant sort of `B`”); the certificate references `(LibraryId, AxiomId)`, and `B.hash`
   changes with that registration.
3. **Reproofing**: construct a new certificate for `P` directly under `B`.

An implicit transfer of the form “`Proved` under `A` ⇒ `Proved` under `B`” is never allowed, and
`certA` must never be reused merely by changing its signature hash. Cross-module export of Σ (§13)
and the cross-module form of `q=0` evidence follow these same three paths.

The hard budget limit is the engineering answer to the halting problem (the numerical values are in
§8.4). There is no knob—providing one would amount to asking the user, “Do you think your program
will halt?” The user does not know, and neither does the compiler. Exhausting the budget yields
`Unproven(reason=BudgetExceeded)`, not `Disproved`.

#### 4.2 `Certificate`: A Proof Term Relative to a Signature

A **certificate** is the compile-time representation of a “proof term,” and has meaning only
relative to some `LogicSignature`:

```text
Certificate {
    signature_hash : ContentHash            // Must equal signature.hash
    proposition    : Proposition           // Conclusion; a term in the unified term world
    steps          : [ProofStep]           // Derivation sequence according to signature.rules
    evidence_refs  : [EvidenceRef]         // References to explicit evidence
}
ProofStep {
    rule       : RuleId                   // Must match an entry in signature.rules
    premises   : [StepId | AxiomId | GammaId | EvidenceId]
    context    : Γ                       // Local assumptions (path conditions, equality bridges, q=0 evidence)
    conclusion : Proposition
    subst      : Substitution            // Term substitution; terms in the unified term world
}
EvidenceRef = ExplicitProof(AstNodeId)     // Proof function / user-written explicit evidence
            | LibraryAxiom(LibraryId, AxiomId)
            | LibraryProof(LibraryId, ProofId)
            | ProducerHint(bytes)          // Hint from SMT/tactics/proof search; not trusted directly
```

For each `ProofStep`, `check` performs four mechanical actions: ① `rule` exists in
`signature.rules`; ② `premises` point to earlier steps, registered axioms, `Γ` facts, or explicit
evidence, and each is resolvable; ③ after applying `subst` to the rule pattern, the result is
identical to `conclusion` (equality is decided by the pure decision procedure in §12.1); ④ the final
step's `conclusion` is identical to `certificate.proposition`. If any condition fails ⇒ `Unproven`,
with the failing step identified; **the kernel never skips these four checks merely because a
producer said “unsat.”**

`ProducerHint` is only a hint. A producer may attach one to a candidate certificate—for example, an
SMT-LIB script, model, or tactic trace—but the kernel neither executes it nor treats it as a rule.
Hints only help “complete the candidate into inspectable `steps`.” If completion fails, the result
remains `Unproven`.

#### 4.3 After Unproven: Supply Explicit Evidence (Proof Functions Are First-Class)

When the kernel returns `Unproven`, the diagnostic must suggest **supplying evidence**, rather than
rendering “cannot compute this” as “this does not hold” (see §12.3 for the diagnostic shape). Legal
forms of supplementary evidence are treated as “first-class”: they are proof terms in the unified
term world, not external additions:

- **Proof function**: a YaoXiang function whose return type equals the proposition to be proved. It
  is an `ExplicitProof(AstNodeId)`, and the kernel verifies it through exactly the same mechanism as
  verifying `add(a, b): Int`.
- **axiom library entry**: an axiom registered in the signature, `EvidenceRef::LibraryAxiom`.
- **proof library entry**: a named proof (reusable, independently testable, and equipped with a
  manifest hash), `EvidenceRef::LibraryProof`.
- **Existing kernel certificate**: a certificate already `Proved(checked)` under the same
  `signature_hash`; it may be reused directly or as a premise.
- **Change the signature / add an axiom / write a translation term**: explicitly register one of the
  three cross-signature paths from §4.1, then run `check` again.

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
# The kernel verifies it—exactly as it verifies add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    ...
}
```

**A property that cannot be proved automatically at compile time → the programmer supplies explicit
evidence (a first-class proof term) → the kernel verifies it under the signature.** The entire
process is a smooth gradient—the producer handles simple certificates, leaving the difficult
reasoning to the programmer.

#### 4.4 Layered Dependencies Within the Pipeline

The producers above share the same `Kernel::check` interface, but there is an orchestration order.
Type equality is a prerequisite for all later analyses; ownership/token checking depends on type
information; refinement-predicate verification depends on the results of the first two layers. The
compiler orchestrates by layer, and expressions that fail at a lower layer do not proceed to higher
layers—this avoids wasting search budget on programs with type errors. **Every layer's conclusion
must still be verified by `check`; the layer order only determines which candidate certificate is
requested from a producer first.**

```
编排顺序（同一内核，分层调度）
├── 第 0 层：类型等式（T1 == T2）
│   └── 内核直判规则（defeq）→ 产出候选证书 → check
├── 第 1 层：所有权/令牌冲突
│   └── 流敏感活性事实 → 产出候选证书 → check
└── 第 2 层：精化谓词/霍尔蕴含
    └── producer（Γ 证据 / proof library / SMT 搜索）→ 候选证书 → check
```

Every layer still returns `Proved/Disproved/Unproven` and shares the same interface, `signature`,
and budget system.

### 5. Uniformity Across Three Layers of Functions

| Layer                  | Execution time | Input      | Output | Example                                        |
| ---------------------- | -------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time   | Type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time   | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All three use the same `name: type = value` syntax. Compile-time predicates and type constructors
follow the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd–Hoare Verification-Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd–Hoare-style assertions—the compiler derives
verification conditions from the annotations, and the proof pipeline checks whether every assignment
preserves the type.

The core mechanism is that every assignment corresponds to a Hoare triple `{P} x := e {Q}`, with
verification condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop
body—once the proof pipeline verifies the induction step, all iterations are covered automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i; at initialization i=0; verification: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verification: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Premises: s satisfies SumUpTo(arr, i), and i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (i is unchanged for now)
        #   Substitute s_new = s_old + arr[i]:
        #     Requires s_old + arr[i] == sum(arr[0..i+1])
        #     From the induction hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → s's type annotation references i in the dependency graph → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the preceding step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

The loop invariant is the type annotation on a variable—the programmer writes the type, and the
compiler checks the induction step. The compiler need not “discover” the invariant or “perform
induction automatically”; it decomposes the induction proof into local verification conditions for
each assignment and delegates them to the proof pipeline.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The mechanism above requires the compiler to know that the type annotation `SumUpTo(arr, i)` of `s`
references `i`—when `i` changes, the type constraint of `s` changes as well. This requires the
compiler to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# The key is the depended-upon variable; the value is the set of variables whose type annotations reference that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it resolves
free-variable references in the arguments of `Pred(...)`. If an argument references another mutable
variable `x` in the current scope, the dependency graph records `x → v`.

**Trigger**: when a depended-upon variable `x` is assigned, the compiler:

1. Finds all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`.
2. For each `v`, generates the verification condition:
   `does v's current value satisfy the updated type Pred(... x_new ...)?`
3. Sends the VC into the proof pipeline.

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. With `SumUpTo(arr, i)`:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify that s satisfies SumUpTo(arr, i_new) → True

# Wrong order—the compiler rejects it
i += 1        # i changes → re-verify that s satisfies SumUpTo(arr, i_new)
              # s has not yet been updated; s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compilation error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: one variable may depend on several variables. A type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—a change to either triggers re-verification.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent verification mechanism. It answers “when must a VC be generated?”—the proof pipeline
answers “does the VC hold?”

### 7. Termination Checking

**Scope: refinement types.** Termination is not an independent switch; it is part of a
**verification mode**. Once a type is refined (`Refined { base, constraint }`), the computation
described by that refinement enters verification mode, and termination is required within the mode.
Ordinary types that are not refined do not enter verification mode and generate no termination
obligations.

Two consequences follow:

- **Loops**: a bare `while` does not enter verification mode; when a measure variable has a
  refinement annotation (such as `i: UpTo(n)`), the mode is entered and termination must be proved.
- **Recursion**: when a function signature contains a refinement (a refined parameter or a refined
  return type), the mode is entered and the measure must be shown to decrease strictly at every
  recursive call site.

Automatic approaches have priority within the mode: the compiler first explores measures
automatically and accepts those it can prove; if exploration fails and no explicit measure was
supplied, compilation fails. There is no **annotation-syntax** escape hatch—the measure and
termination proposition both live in type position, introducing no new `decreases`-like syntax. See
§6.9 for the forms of measures and explicit fallbacks.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two places:

1. **Variable type annotations**: boundary constraints in refinement types (for example, `UpTo(n)`
   supplies the upper bound `n` and lower bound `0`)
2. **Loop-body operations**: the operations applied to variables on each iteration

The compiler tries four measure-synthesis strategies in priority order and stops as soon as one
succeeds. The four strategies form a **restricted template sequence for measure exploration**. Their
input consists of refinement constraints (all four strategies begin from a “variable of bounded
type”), not compile-time-evaluated code; they are the automatic and manual sides of the same
mechanism as the explicit measures in §6.9.

> **Measure exploration is exploration, not inference.** Exploration enumerates only templates
> (linear rank, violation count, bounded patterns, multiplicative scaling) and does not guarantee
> that a solution exists—general measure inference is generally undecidable (it reduces to the
> halting problem). Cases outside the templates must therefore be expressible by an explicit
> programmer-supplied measure (§6.9), or compilation fails.

#### 6.2 Strategy 1: Automatic Synthesis of a Linear Rank Function

When a variable has a linear bound annotation, the producer enumerates candidate linear measures and
produces candidate certificates for the decrease obligations, which `Kernel::check` verifies.

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
bounded type annotation. This includes `i += const`, `i -= const`, and interval contraction in
binary search:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# The measure high - low strictly decreases on both paths
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

#### 6.3 Strategy 2: Predicate Violation Count—Automatically Extracting a Measure from the Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: an experimental strategy whose inclusion will be decided during Phase 3
> implementation according to practical feasibility.** This strategy works for adjacent-swap
> operations (bubble sort, insertion sort), but cannot automatically prove non-adjacent operations
> (quicksort partition, heap-sort sift-down). See the coverage boundaries in the table below. If
> Phase 3 validation proves infeasible, this strategy will be removed or deferred to future work.

Core insight: **the specification written by the user is material for compiler reasoning.** The
compiler need not have a built-in definition of “what sorting means”—it reads the definition of
`Sorted` and automatically extracts a measure from it.

```
输入：
  目标类型：Sorted(arr) = { forall i in range: cond(i, arr) }
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

| Algorithm      | Operation pattern | Provable by Strategy 2? | Reason                                                    |
| -------------- | ----------------- | :---------------------: | --------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |           ✅            | `violation_count` strictly decreases with each swap       |
| Insertion sort | Adjacent move     |           ✅            | Each shift eliminates one violating pair                  |
| Selection sort | Non-adjacent swap |           ❌            | A single swap may increase `violation_count`              |
| Quicksort      | Partition         |           ❌            | Non-adjacent swap, with no guaranteed monotonic decrease  |
| Heap sort      | sift-down         |           ❌            | Tree-shaped operation; `violation_count` is not monotonic |

**Complementary strategies**: for quicksort, contraction of the `low < high` interval can be covered
by Strategy 1 (a linear rank function)—the outer partition recursion halves the interval each time.
Strategies 1 and 2 provide complementary coverage, and most practical algorithms can be proved by
one or the other. Generalizing Strategy 2 to non-adjacent and tree-shaped operations remains an open
problem.

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

`v += const` (a positive constant) with an upper-bound type annotation on the variable → the measure
`upper_bound - v` decreases by `const` each time and has lower bound 0. This is a degenerate case of
Strategy 1, handled quickly by the compiler at the front of the sequence.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1) with upper- and lower-bound type annotations on the variable. The compiler
includes the logarithmic measure template `ceil(log_const(upper/v))`, which decreases by one each
time the value is multiplied by `const`.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically derives the measure ceil(log₂(n/i)), which decreases by one on each multiplication by 2
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove that the loop exits after finitely
  many steps; when exploration fails, the programmer supplies a measure in type position (§6.9)
- **Correctness**: whether the loop body makes progress toward the target type, checked by the
  compile-time proof pipeline through verification conditions

If both pass, compilation passes. If termination is proved but correctness fails → compilation
error + counterexample. If correctness is proved but termination cannot be proved → compilation
error identifying the variable or operation that could not be analyzed. If both fail → compilation
error reporting both failure causes separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks that the formal parameters
decrease at every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // The compiler explores: the well-founded measure of (b, a % b) decreases → terminates
}
```

Formal-parameter decrease is the strongest automatic path (structural recursion). When exploration
fails, the programmer explicitly supplies a measure in type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is not invertible, not closed, and preserves no monotonicity—automatic
termination proof is mathematically impossible. Compilation fails:

> This loop cannot be automatically proved to terminate. The loop variable depends on the
> unanalyzable function `f`. Use an iteration pattern the compiler can analyze, or bind a name to
> this loop and provide a measure in type position (§6.9).

This is not a compiler failure. Code that cannot be statically proved safe must not compile. Even
when an explicit measure is supplied, it must include certificates for both decrease and
well-foundedness, each verified by `Kernel::check`—an incorrect measure is rejected with a
counterexample. Humans may fail to prove termination, but may not prove it incorrectly.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure in **type
position**—through exactly the same mechanism as `Positive(b)` and `IsMax(T, arr, result)`
(predicate application), with zero new syntax:

```yaoxiang
// Measure: an ordinary function; unit-testable, reusable, and absent at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor, and the measure obtains values from the names in scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The rationale for the loop form is that `Terminates(m)` refines the value type of the loop body's
tail expression, and the binding name `acc` provides the anchor—the loop can therefore be referred
to, eliminating the “anonymous construction cannot be referred to” corner case.

> **The body tail must produce a value**: the value of a loop = the value of its body block (§2.9),
> and the value of a block = its tail expression. If the tail is an assignment statement (such as
> `{ i = i + 1 }`), the block value is `Void`, and `return acc` would return `Void` from `-> Int`,
> making the type invalid. The example therefore explicitly writes `i` as the body tail. This does
> not change the measure's semantics—the measure `n - i` is a **state expression**, independent of
> the loop's value type.

> **Implementation note (recorded 2026-10-02; updated 2026-10-04)**: the loop example in this
> section is valid on the **type side** (`check` reports 0 errors and the back-edge obligation is
> Proved). **The runtime-value defect (#409) has been fixed**: a `while` expression used as a value
> now takes the value of the body block's tail expression from the final iteration, consistently
> with the type-side `block_value_ty`— `loop(4) == 4` and `countdown(3) == 30` now hold in actual
> execution (`for` received the same fix).
>
> **Known residue (zero iterations)**: when the loop body is never evaluated, there is no “body
> value from the final iteration,” so the loop value remains `Void`—binding it to a non-`Void` type
> position and using it at runtime reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a loud type mismatch, not a
> silently wrong value). This is a **static-approximation boundary**: the `block_value_ty` criterion
> provides the type of the tail expression, but cannot determine whether “the loop executes at least
> once” (which requires dataflow/provability analysis). The type side therefore does not silently
> downgrade (doing so would break the value type required by this section's `Terminates` loop
> binding). The residue is pinned by the corpus
> `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residue has been confirmed as an intentionally retained
> design boundary** (decision of 2026-10-04, temporarily not fixed), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) was closed with the main-form fix and is
> no longer tracked separately. See the minimal reproduction and historical impact in
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Examples, Loop section, “Known
> Defect D6.” In addition, termination Strategy 1 (linear rank function) is effective again in this
> round, so the section's original order—“write an explicit measure only when automatic exploration
> fails”—remains unchanged: an explicit measure is still the fallback after exploration fails.

**`Terminates` is a built-in predicate**, on the same level as `Int` and `Never`. It is the only
predicate **whose function body the compiler writes on the programmer's behalf**—its assertion (“the
measure strictly decreases at every recursive call site/loop back edge”) lives in the computational
structure, so user-written predicates cannot refer to the function body or loop body and therefore
cannot be expressed with the predicate-definition syntax in the “Syntax” section. The built-in
surface is therefore limited to this one name.

**Arities**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two different constructs:

| Form                    | Anchor                 | Use                                                           |
| ----------------------- | ---------------------- | ------------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding    | Self-recursive functions and loops—the default form           |
| `Terminates(FnType, m)` | Explicit function type | Cases such as mutual recursion where the anchor is not unique |

The two are fundamentally the same: a termination obligation always lands on “the computation
described by the type position in which the refinement appears.”

**The measure does not restrict the return type.** A measure may be an expression of any type (it is
not required to be a natural number); its “strict decrease” is determined by the well-founded order
available on that type. Whether the measure is well-founded (for example, whether an `Int` result is
`>= 0`) is an **independent obligation**, just like the decrease obligation; a producer produces a
candidate certificate for it and `Kernel::check` verifies it. When neither can be established, the
diagnostic **does not reject directly**; instead, it suggests checking the direction (whether the
measure has a lower bound and whether recursive parameters actually move in that direction).

**Relationship to automatic exploration**: an explicit measure is not a separate pipeline, but the
input used after producer exploration fails. Once a measure is supplied, the producer still produces
candidate certificates and submits them to the same `Kernel::check` to verify decrease and
well-foundedness; if they do not hold, an error with a counterexample is reported.

The implementation mechanisms for obligation generation, the decision pipeline, diagnostic
directions, and measures for mutual recursion (SCC) are described in
[RFC-027a: Explicit Measures for Termination Checking](../accepted/027a-termination-explicit-measure.md).

### 8. Certificate Producers: SMT / Tactics / Proof Search Are All Untrusted

SMT solvers, tactic scripts, and proof-search processes all play the same role in this design:
**certificate producers**. They are untrusted: the compiler does not treat their return values as
proofs, but only as hints for “finding a candidate certificate.” The only trusted component is
`Kernel::check(signature, certificate, proposition)` (§4).

#### 8.1 Trust Model

- **The kernel is the verifier**: `check` replays `certificate.steps` according to `signature.rules`
  (§4.2). It does not call a solver, parse `unsat`, or read producer logs.
- **The producer is the searcher**: given a `signature` and `proposition`, the producer makes a best
  effort to deliver a `Certificate`. Whatever it returns or says does not constitute `Proved`.
- **Forbidding `Unsat ⇒ Proved`**: at most, `unsat` indicates that “no countermodel was found under
  the producer's own translation and theory.” It is not a certificate and does not guarantee that
  `certificate.steps` exists. Mapping `unsat` directly to `Proved` outsources the trust root to the
  solver and directly conflicts with the pure-function kernel in §4. **This RFC explicitly forbids
  that mapping.**
- **`sat` is not sufficient for `Disproved` either**: `sat + model` is only a candidate
  counterexample supplied by the producer; it must become a falsification certificate decidable by
  `Kernel::check` (replayed under the signature's falsification rules, or computed by a
  kernel-direct rule as “the two sides differ after normalization”; see §12.1) before it may yield
  `Disproved(model)`. Translation distortion (for example, fidelity of integer literals, issue #561)
  can cause `sat` to carry a spurious model; blindly trusting it falsely rejects valid programs.
- **`unknown` / timeout / budget exhaustion ⇒ `Unproven`**: the diagnostic includes the unsolved
  proposition, reason, and evidence-supplementation hint (§12.3), and must not be downgraded to an
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

Producers may run in parallel, cache results, and switch backends; none of these freedoms changes
the single requirement that they “deliver a replayable certificate.” SMT-LIB 2.6 remains the
translation target (an ISO standard supported natively by Z3 / CVC5 / MathSAT / Yices), with Z3 as
the default backend, CVC5 as an alternative, and compile-time switching—**SMT-LIB is an input format
for producers, not a trust boundary**. There is no “general-purpose solver abstraction layer” and no
“general-purpose producer abstraction layer”: a producer has exactly one output contract, “deliver a
`Certificate`.”

#### 8.3 From `unsat` to a Legal Certificate: Registration of Translation Rules

For `unsat` to become useful, the producer's theory must first be folded into rules registered in
the `signature`. For example, the linear-arithmetic fragment may register a set of `linarith` rules
in the `manifest` of a `kind=Axiom` library in `libraries` (including their applicable sorts and
premise shapes); an SMT-LIB `unsat` result is expanded by a **proof-producing adapter** into a
sequence of `ProofStep`s for those rules and then submitted to `check`. The adapter is itself an
untrusted producer: the steps it produces are still replayed one by one.

This requirement turns “trusting Z3's unsat” into “trusting that the `linarith` rules are registered
in the signature and that the certificate is inspectable step by step.” Translation bugs are no
longer left to the hope that “some other test will expose them”—if the adapter produces a
certificate with a missing step or the wrong rule, `check` immediately returns `Unproven`. An
`unsat` result for an unregistered theory must never enter the `Proved` path.

Before the proof-producing adapter is implemented, a **conservative downgrade** is allowed: the
arithmetic fragment uses kernel-direct rules (structural equivalence, constant folding, βδι
reduction) and registered proof-library entries; the producer's `unsat` is attached only as a
`ProducerHint` to the candidate certificate and does not independently trigger `Proved`. This
downgrade surface is smaller than blindly trusting `unsat`, and does not silently pass.

#### 8.4 Budget (The Producer's, Not the Kernel's)

The budget constrains producer search, not the correctness of `check`—`check` is deterministic
replay, and its step count is determined by the certificate length.

| Budget dimension               | Default    | Description                                                                   |
| ------------------------------ | ---------- | ----------------------------------------------------------------------------- |
| Search steps                   | 10,000     | Maximum solver/search steps for the producer; exceeding it returns `Unproven` |
| Time                           | 100ms      | Maximum search time per obligation                                            |
| Quantifier instantiation depth | 3          | Maximum nested-quantifier depth translated by the producer                    |
| Certificate replay steps       | Hard limit | Maximum `steps.len()` for `check`; exceeding it ⇒ `Unproven`                  |

Exceeding the budget returns `Unproven`: compilation error + predicate location + a one-line budget
summary (§12.3). There is no downgrade, no runtime check, and no silent pass. Dependent-type
pre-reduction (`factorial(5) → 120`, `append([1,2],[3]) → [1,2,3]`) uses kernel-direct rules and
consumes no producer budget.

#### 8.5 Relationship to the Three-Valued Result, Γ, and `q=0` Evidence

Constant-equality facts in `Γ` (§12) and the four sources of evidence for the `q=0` slot (RFC-011
§4.1: literals, value environments, single-value pinning, and Γ equalities) are sources of
`EvidenceRef` that producers may reference; they are not another decision channel. “Path conditions
are background knowledge” from §3.5 is refined here to “path conditions are the local `Γ` context of
the `signature`, passed to `check` together with the `context` field of the `Certificate`.”

#### 8.6 Current Implementation Gap: `predicate.rs` Blindly Trusts SMT

This section's design differs from the current production implementation. The gap is recorded
honestly rather than pretending that the design has landed:

- `src/frontend/core/sema/layers/predicate.rs`'s `try_implication` maps `SMTResult::Unsat` directly
  to `Some(Ok(()))`; `try_smt_solve` maps `SMTResult::Unsat` directly to `Ok(())`. `Ok(())` is the
  `Proved` arm of `ProofResult`—this is **blindly trusting `unsat`**, without constructing or
  replaying any `Certificate`.
- The same file's `SMTResult::Sat { model }` directly constructs `ProofFailure::Disproved`; the
  counterexample is not verified by the kernel under the signature, and translation distortion
  (issue #561) can falsely reject valid programs.
- `ProofContext` (`src/frontend/core/sema/proof/context.rs`) currently carries only
  `env / assumptions / dep_graph / budget / dep_env` and has **no `LogicSignature` field**;
  `ProofFailure` and `ProofResult` are three-valued results, but there is no `Certificate` type.
- The current pipeline therefore equates “the producer's return value = the decision result,”
  whereas the target pipeline is “the producer delivers a certificate → `Kernel::check` → decision
  result.” These are not merely wording differences; they are different trust models: the former
  treats Z3's result as a trust root, whereas the latter treats the signature and kernel as the
  trust roots.
- Remediation direction (code is not included in this RFC): ① introduce `LogicSignature` and
  `Certificate` representations and carry the current signature in `ProofContext`; ② convert
  `try_implication` / `try_smt_solve` into “produce candidate certificates,” with `unsat` used only
  as a `ProducerHint`; ③ make `Kernel::check` the sole path to `Proved`, and confirm `Sat`
  counterexamples through `check` as well; ④ register translation rules such as `linarith` under
  §8.3. Implementation follows the “evidence enforcement / complete Π/Σ bodies” stage of RFC-039
  D62; this round defines only the contract.

#### 8.7 wasm and Backend Differences

Under wasm, when Z3 is unavailable (the SMT code is excluded by `cfg(not(target_arch = "wasm32"))`),
the producer has one fewer search path and downgrades to `Unproven` (conservative and less
rejection). This is unrelated to “signature consistency”: under the same `signature`, native has one
more producer candidate and wasm has one fewer, while `Kernel::check` behaves identically. The
existing native/wasm difference table (§Open Questions) is now interpreted in terms of “producer
availability,” not “decision precision.”

Programmers need not know that SMT exists. The mental model is: **the compiler passes if it can
verify a certificate under the current signature; otherwise it reports Unproven and suggests adding
evidence—if the automatic producer cannot find one, write a proof function (a first-class form of
explicit evidence) and let the kernel inspect it.**

### 9. Composition of Compile-Time Predicates

Compile-time predicates are functions returning Type, and composition is naturally implemented
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

result = divide(10, 2)   # ✅ The compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ The compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Safe Array Access

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ The compiler verifies InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ The compiler verifies InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
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

#### 9.4 Loops: Compiler-Generated VCs

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

### 11. The `dispatch` Dispatch Pipeline: Unifying Compile-Time and Runtime Dispatch

> **Implementation note (recorded 2026-10-03, #377-2)**: the **semantics** of this section are
> implemented and integrated into production, but its **structure** is not a standalone
> `layers/dispatch.rs` module. Production dispatch is implemented through **three call-site
> obligations** in `checker.rs`—binding-site revalidation (`revalidate_refined`), call-site
> arguments (`check_call_arg_refinements`), and return-position postconditions
> (`check_return_refinement`). The **direct** calls to `layers::predicate::check_predicate` occur in
> **two places**: call-site arguments and return-position postconditions share the sole decision
> unit `judge_refined_obligation` (`checker/refinement.rs`), while `revalidate_refined` has its own
> E2030 behavior. A **fourth obligation surface** is in the proof executor (`proof_execution.rs`,
> wired on 2026-11-14, issue #537): when executing a proof call, it first expands **the called proof
> function's own parameter refinements** (RFC-027 §3.3 nested call-site precondition
> obligations—breadth-first worklist, deduplication set + §8 budget closure; a closed Bool
> constraint is falsified into E4018, and the Call form is queued for continued execution). The
> original `layers/dispatch.rs` had zero production call sites, and its handling of
> `Unproven { ProofFunctionRequired }` (“downgrade to a W1080 warning and inject Γ”) conflicted with
> the production implementation (`Unproven` always reports an error; see `refined_unproven`), so it
> was deleted in its entirety. Note also that the “insert a runtime check” behavior in the Runtime
> row of this section's table has **no production emission point yet**
> (`RuntimeOutcome::InsertCheck` has no consumer), while Γ injection is performed independently by
> branch guards in `ownership.rs` and `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement-type primitive. The dispatch pipeline
`dispatch` automatically chooses between compile-time proof and runtime checking according to
**whether the predicate's free variables are accessible at compile time**:

| Criterion                                                                                    | Mode            | Behavior                                                                                                                                                                                 |
| -------------------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants)    | **CompileTime** | Enter the proof pipeline: the certificate producer supplies a candidate certificate → `Kernel::check` → Proved is erased / Disproved is a compilation error / Unproven requests evidence |
| Some free variable comes from runtime (function parameters, external input, `mut` variables) | **Runtime**     | Insert a runtime check and inject the refinement fact into the flow-sensitive assumption set Γ                                                                                           |

**Key point**: “cannot decide” ≠ “disproved.” In CompileTime mode, Unknown requires proof (no silent
downgrade). In Runtime mode, the proposition has no truth value at compile time in the first
place—no matter how strong the prover is, it cannot write an always-true proof for “the user may
have entered a negative number.” A runtime check is the only sound choice. This is not a limitation
of the prover; it is a mathematical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest-Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ that tracks the propositions known to hold
at each control-flow point.

**SP (strongest-postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for `mut` variables**: after a `mut` variable is reassigned, all assumptions involving
that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard soundness requirement—once a variable's value changes, old assumptions are invalid.

**Branch confluence**: when IF/ELSE or match branches are merged, Γ contains the intersection of the
branches' assumptions. Only propositions that hold on every path leave the branch.

**Equality bridge (automatic evidence for the `q=0` slot)**: constant-equality facts in Γ are a
legal source of evidence for compile-time value parameters (RFC-011 §4.1, the `q=0` slot). Within
the guard branch of `if y == 42 { f(y)(y) }`, Γ records `y == 42`, so y is treated as 42 in the
`q=0` slot, and the call follows the existing `f(42)` path. One side of the equality is a
compile-time constant; a runtime value without equality evidence still reports an error when placed
in the `q=0` slot. This is not a new mechanism—the verification condition `{y == 42} ⇒ {y ≡ 42}`
follows the existing implication decision procedure.

#### 12.1 Definition of Equality (`defeq`)

This section was added by RFC-039 Decision D62: the equality bridge above supplies a source of
evidence for the `q=0` slot, while this section completes the definition of equality itself, its
treatment of Unproven for type equality, and its diagnostic shape.

Equality is the sole equivalence relation on types. It is defined by exactly one rule: after both
sides are normalized, they are **identical**.

- **Normalization** consists of three kinds of reduction: δ expands type aliases and definitions; β
  reduces type families and dependent-type applications; ι normalizes literal types—the identity of
  a literal type is determined only by its value, and source spelling does not participate (`03` and
  `3` are the same type).
- **Identity** is fully structural: type constructors have the same name, and arguments are
  identical one by one. Equality does not inspect cache keys, rendered text, or perform any
  inference over refinement predicates.
- **Subtyping is an independent relation**: `A <: B` is decided by the subtyping rules; the fact
  that both `A <: B` and `B <: A` hold does not define equality. Bidirectional subtyping would mix
  covariance and interface-duck-typing rules into equivalence, contaminating the notions of
  “assignable” and “equivalent.”
- **`Refined` with different predicates ⇒ Unproven**: when the base types are the same but the
  predicates differ, the compiler returns Unproven, not Disproved. Predicate equivalence belongs to
  the assumption-set Γ layer (the equality bridge above and its subsequent Γ equality rules); before
  that layer is implemented, the compiler can only say “cannot decide.”

The failure surface of equality is consistent with the three-valued algebra in §4.1: both sides
normalize successfully and are identical ⇒ `Proved`; both sides normalize successfully but differ ⇒
`Disproved`; either side fails to normalize ⇒ `Unproven` (budget exhaustion or capability boundary).
**Failed reduction does not produce `Disproved`**—being unable to compute does not mean being
unequal.

Equality is a pure decision: it **does not bind** type variables, and success accepts the equation
without leaving an instantiation. Decision points (assignment, argument, return position) first use
unification; after unified failure, equality may be used, but any partial bindings produced by the
unification attempt must first be rolled back. Without rollback, an attempt judged unequal leaves
bindings behind and contaminates later inference.

#### 12.2 Unproven ≠ False: Placement in Type Equality

The three-valued algebra in §4.1 applies to every obligation point, and type equality is one of
them. This section applies “Unproven ≠ False” at that layer.

- When an obligation point returns `Unproven`, the compiler **must** report a compilation error and
  must not allow the code through. The diagnostic must suggest adding evidence: write a proof
  function in the refinement domain or reference a proof-library entry (§4.3); in the equality
  domain, provide a normalizable form; changing the signature or adding a bridging axiom follows the
  three cross-signature paths in §4.1.
- `Unproven` and `Disproved` must be distinguishable: `Disproved` carries a counterexample (a
  concrete assignment or the two unequal sides), whereas `Unproven` carries the unsolved proposition
  and its reason. Rendering `Unproven` as “does not hold” is an incorrect report.
- `Unproven` **must not** be downgraded to an internal compiler error (E8001). E8001 denotes only a
  consistency failure in the compiler itself; budget exhaustion and capability boundaries are
  boundaries of the compiler's reasoning ability and belong to user-facing diagnostics (RFC-013
  “Message Quality Specification” §5).
- `Unproven` **must not** pass silently. When a decision point ends in failure, the user must see a
  diagnostic from the existing failure path; no second diagnostic is added, and the code is not
  accepted merely because the result could not be decided.

#### 12.3 Diagnostic Shape

An `Unproven` diagnostic must carry three items; omitting any one makes it incomplete:

- **Unsolved proposition**: renderable proposition text (refinement constraint, both sides of an
  equality, termination measure);
- **Reason**: a capability boundary (outside the kernel's inference rules), budget exhaustion, or an
  unbound symbolic name—one of these;
- **Location**: source location. A decision point with genuinely no location uses the default
  location, and the implementation comment states the reason.

Budget details are presented as a **one-line summary** (for example, “budget 10000 steps, 10000
steps used”). The complete step count, solver-internal trace, and failure shape belong in the
verbose channel or logs, not in default output.

This section formally relaxes “Unproven → compilation error + unsolved proposition +
budget-consumption report” from §4.1: the substance is unchanged—the user must still distinguish
“cannot compute” from “does not hold.” Only the granularity of the budget-consumption report is
relaxed. The reason is that the complete step count offers no actionable value to the user (the user
cannot modify the program based on it) and drifts with implementation details; reducing it to a
one-line summary improves diagnostic stability and readability, while distinguishability is
guaranteed by the “reason” field.

The “evidence-supplementation hint” for `Unproven` belongs in the diagnostic's **reason** field: a
capability boundary explains that the result is outside the current `signature.rules`; budget
exhaustion includes a one-line summary; an unbound symbolic name identifies the name for which
evidence must be supplied. The division of labor between general `Unproven` and refinement-specific
codes is described in RFC-013's draft code table (E4023 / E2031).

### 13. Erasure Model: Certificates Retained at Compile Time, Erased at Runtime

Three forms of “erasure” must be distinguished because they apply to different objects:

| Object                                                   | Compile time                                                                                                                   | Runtime                                                                                           | Basis                        |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- | ---------------------------- |
| `Certificate` (a proof term verified by `Kernel::check`) | Retained: `Proved(checked)` enters the compile-time cache for incremental reuse and verification of Σ construction obligations | Completely erased: no runtime code is generated and no data slot is used                          | §4.1, §4.2                   |
| `q=0` compile-time value parameter / Σ witness field     | Exists as a quantifier fact and evidence obligation (§14)                                                                      | Occupies no slot; `CreateStruct` contains only data fields; value-level `b.w` is rejected (E1114) | RFC-011 §4.1, D62 I4/I6, §14 |
| Bool check inserted by dispatch in Runtime mode          | Is neither a witness nor a certificate                                                                                         | Retained: executed at the value level                                                             | §11                          |

**A certificate is a compile-time object.** After `check` succeeds, `Proved(checked)` exists only at
compile time: it pins down “this obligation has been verified under `signature.hash`,” and the cache
key contains `signature_hash + proposition + certificate`, so changing the signature invalidates it
(§4.1). The runtime neither needs nor may carry certificates—the proof at the type level has been
erased, and the `q=0` slot and Σ witness likewise occupy no runtime layout (D62 I4 “Complete
Erasure,” I6 “witness erasure ≠ runtime check”).

**A runtime check is unrelated to certificates.** When §11 dispatches to Runtime mode, it inserts a
value-level Bool check that inspects a runtime value; it is not a kernel-verified proof term.
Retaining it does not violate “runtime erasure of certificates”: the two act on different objects
and do not replace one another—witness erasure, certificate erasure, and check retention.

**Consistent with D62's unified term world**: propositions, certificates, `q=0` terms, and Σ
witnesses are all compile-time terms in the same system. Erasure is a consequence of the single rule
“compile-time terms do not enter the runtime layout,” not a separate erasure rule invented for proof
terms. See §14 for how Σ record fields become evidence obligations, how constructors produce
certificates, and how `b.w` is used at the type level. §13 and the implementation gap in §8.6 are
recorded in parallel: the current `predicate.rs` does not yet produce a `Certificate`, so
“certificates are retained at compile time” has not yet been implemented.

### 14. Dependency Record Σ: Evidence Fields, Construction, and Type-Level Use

A Σ dependency record is a **record type** in the unified term world: later fields may refer to the
names of earlier fields. The records in this section are one source of obligations for the §4 proof
kernel—constructors must deliver, for every evidence field, a certificate verified by
`Kernel::check(signature, certificate, proposition)`; the certificate is retained at compile time
and erased at runtime under §13.

This section gives the complete semantics: position rules (§14.1), construction and layout (§14.2),
generics (§14.3), cross-module export (§14.4), type-level use (§14.5), and immutability and
diagnostics (§14.6). Generics, cross-module use, and type-level use are all defined completely here
rather than deferred.

#### 14.1 Field Position Rules: The Proof-Space Criterion for Evidence

Let the fields of record declaration `R` be `f₀, f₁, …, fₙ₋₁` in source order, with field `fᵢ`
having type `Tᵢ`. Classification proceeds in three steps: **first determine `space` (proposition
family or data family), then determine argument position (previous/self/forward), and finally
determine the field category**. No cross-field inference is performed, and field names are never
used for guessing:

**First step (proof-space criterion, the sole criterion).** Normalize `Tᵢ` and take the
`SortDecl.space` of its resulting type family (§4.1):

- `space = Proof` (`Tᵢ` reduces to a proposition / refinement type / `Assert` / `Terminates` /
  proof-family result) ⇒ enter evidence classification;
- `space = Data` (`Tᵢ` is `Int`, `Array`, `Vec`, an ordinary record, or a **data type family
  application**) ⇒ **data field**, regardless of how many previous fields it references.

The `n` in `Array(Int, n)` references a previous data field and changes the “length of the data
slot”; it does not “generate a proof.” Classifying it as a witness would erase the buffer from the
runtime layout—the misclassification this section is intended to eliminate. `space` is registered in
`signature.sorts` / the library manifest, and `Kernel::check` and field classification read the same
registration.

**Second step (position criterion).** This applies only to `space = Proof` types of the form
`Tᵢ = P(a₀,…,aₖ)`, and distinguishes three cases by the positions of the arguments:

- All arguments are **strictly previous data fields** `fₕ (h < i)`, **compile-time constants**,
  **compile-time value parameters** (RFC-011 §4.1 / SPEC §4.3, q=0 parameters such as `Rows` and
  `Cols` for `Matrix`), or **type parameters** ⇒ the third step classifies the field as evidence. A
  compile-time value parameter takes a concrete value after instantiation and is counted as a
  compile-time constant (word-for-word consistent with SPEC §8.3.1); an uninstantiated open term
  remains for construction-time `Unproven` (§14.3);
- An argument **is `fᵢ` itself** (a self-referential refinement such as `v: Pos(v)`) ⇒ the third
  step classifies it as a **data field (admission)**: the field carries a value, and the `Refined`
  constraint merely checks that value rather than proving a fact elsewhere;
- A forward reference to a later field, a reference to another evidence field, or using `fᵢ` itself
  as an evidence argument ⇒ rejected at declaration time.

**Third step (category criterion).** `space=Data` ⇒ data field; `space=Proof` + previous arguments
(including compile-time value parameters / compile-time constants / type parameters) ⇒ evidence;
`space=Proof` + self-reference ⇒ data field (admission).

| #   | Form                                                                                                                                                                        | Decision                                                            | Quantifier             | Runtime layout                                                                                                                   |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `space=Proof` and all arguments are strictly previous data fields / compile-time constants / compile-time value parameters / type parameters (positive example `w: Pos(v)`) | **Evidence / witness field**                                        | `Some(Quantity::Zero)` | Occupies no slot, is absent from `CreateStruct`, and `base` is absent from the runtime layout                                    |
| 2   | `space=Data`: ordinary value types (`n: Int`), data type family applications (positive example `data: Array(Int, n)`), ordinary records                                     | **Data field**                                                      | `None`                 | Occupies a data slot and enters `CreateStruct`                                                                                   |
| 3   | `space=Proof` but an argument is `fᵢ` itself (self-referential refinement `v: Pos(v)`)                                                                                      | **Data field (admission)**                                          | `None`                 | Occupies a data slot; the `Refined` constraint is checked at the construction site under the existing obligations in §3.1 / §3.3 |
| 4   | `space=Proof` but with a **forward reference** to a later field `fₕ (h > i)`, a reference to another evidence field, or `fᵢ` itself as an evidence argument                 | **Illegal**: rejected at declaration time, with no silent downgrade | —                      | Produces no field                                                                                                                |

The four rows do not overlap: row 2 means “ordinary value type / data family” is always a data
field; row 3 means “proposition family with self-reference” is an admission data field; row 4 means
“proposition family with an illegal position” is always rejected at declaration time. **There is no
path in which “a forward reference is also a data field”**—a forward reference neither occupies a
data slot nor enters `runtime_field_index`.

**A witness may depend only on strictly previous data fields.** The arguments of an evidence field
`fᵢ: P(fₕ,…)` must not reference another evidence field, a later field, or `fᵢ` itself; the kind
criterion for `P` is given in §14.3. This “proof-space + strictly previous + data-fields-only” rule
guarantees that, when construction reaches `fᵢ` in declaration order, every argument already has a
compile-time value, the proposition is closed, and a certificate may be submitted to `check`.

**Positive example (evidence) and negative example (data family):**

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }        // space=Proof

Box: Type = {                            // Positive example: w is evidence
    v: Int,          // f0 data field
    w: Pos(v),       // f1 evidence: P=Pos is Proof, and argument v is a strictly previous data field => q=0
    z: Int,          // f2 data field; runtime index = 1 (w is skipped)
}

Buf: Type = {                            // Negative example: data is still a data field
    n: Int,                 // f0 data field
    data: Array(Int, n),    // f1 data field: Array is a Data family; n only determines the length and does not generate a proof
                            // Runtime index = 1; occupies a slot and enters CreateStruct
}

Admission: Type = {                      // Self-referential refinement: data field, not evidence
    v: Pos(v),              // Proof family but the argument is itself => admission (row 3 of the table)
}

// Compile-time value parameter used as an evidence argument (RFC-011 §4.1 / SPEC §4.3, §8.3.1)
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),   // Rows is a compile-time value parameter (q=0) => evidence
    data: Array(Array(T, Cols), Rows), // Array is a Data family => data field
}
```

`Matrix`'s `Rows` and `Cols` are compile-time value parameters (which take concrete values after
instantiation). `Assert(Rows > 0)` has `space=Proof` and an argument that is a compile-time value
parameter, so `_assert_rows` is an evidence field. `data` refers to the same compile-time value
parameters, but `Array` has `space=Data`, so it remains a data field. The B.2 classifier therefore
has an explicit criterion for “compile-time value parameter”: it checks whether the name is
referenced in type position (the strength declaration in RFC-011 §4.1), rather than guessing from
the field name.

**`base` does not enter the runtime layout**: `w` in `w: Pos(v)` is not “an Int value” but “a proof
of `Pos(v)`.” Its `Refined.base` (`Int`) is merely type information for kernel
equality/normalization and must never receive a runtime slot. `StructType.fields` uses
`Vec<StructFieldInfo>{ name, ty, quantity, is_mut, has_default }` as a separate representation:
`quantity == Some(Quantity::Zero)` denotes a witness, and `None` denotes a data field; here `None`
means “data field,” not “unresolved.”

Representation-layer interfaces: `ast::StructField.quantity`, `mono::StructFieldInfo.quantity`, and
`ir::TypeDeclField.quantity` have the same type; `mono::quantity_is_witness` /
`StructFieldInfo::is_witness` / `is_witness_bearing` are the sole decision functions; and
`mono::runtime_field_index` is the **sole mapping point** from declaration order to runtime data
order (it returns `None` for a witness). The pre-classification `space` query reads
`signature.sorts` / `LibraryRef.manifest`, not a second classification table.

#### 14.2 Constructors: Source-Level Data-Only, with Automatically Synthesized Compile-Time Evidence

**There is only one source-level construction form: `Box(v, z)` (data arguments, in declaration
order).** An evidence field **occupies no constructor-argument position**—there is neither a “pass
the witness as a value” form nor an “evidence-argument position”; `Box`'s arity counts only the data
fields `v` / `z`, not `w`. **Evidence does not come from a constructor argument; it comes from a
proof obligation automatically generated at the construction site.** Three views are thereby
distinguished:

| View                                                                              | Form        | Arity                                                                         | Who produces the evidence                                                                                                                                                                       |
| --------------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source-level / compile-time construction view                                     | `Box(3, 5)` | **compile-time arity = runtime arity = number of data fields (2 here: v, z)** | The construction site automatically generates a `FieldObligation`; the producer supplies a candidate `Certificate` → `Kernel::check`                                                            |
| Runtime construction view (`translator::synthesize_constructor` / `CreateStruct`) | `Box(3, 5)` | runtime arity = 2                                                             | None: `CreateStruct.operands` contains only the data-field local slots `v=3, z=5` (`z`'s runtime index = 1, skipping `w`), and `local_count` is based on the number of data fields, with no `w` |

The automatic-synthesis path substitutes the compile-time values of strictly previous data fields
into `P` to obtain a closed proposition. A producer (§8: kernel-direct rules / `Γ` evidence / proof
library / SMT search) supplies a candidate `Certificate`, and
`Kernel::check(signature, certificate, proposition)` verifies it; `Proved(checked)` is retained at
compile time and erased at runtime under §13.

**Explicit evidence is not a constructor argument; it is an independent declaration.** When the
automatic producer cannot supply a verifiable certificate (`Unproven`), the programmer supplies
explicit evidence under §4.3—write an **independent proof function** whose return type equals the
proposition, or register a named proof in a `proof library`. It is referenced by the
`FieldObligation` as `EvidenceRef::ExplicitProof` / `LibraryProof`; the construction expression is
not rewritten, and no `Box(..., proof)` form is added. **Passing one additional evidence argument to
a source-level constructor** (`Box(3, 5, evidence)`) is **not allowed**: under D60's six binding
rules, the extra argument matches no data-parameter slot and produces an E1010 / E1014-family
parameter error rather than being silently swallowed.

A `FieldObligation` must represent both “no certificate has yet been produced” and “a verified
certificate exists.” Therefore, the original `evidence: EvidenceRef` (which can refer only to
explicit evidence and cannot hold a producer-synthesized `Certificate`) is split into two slots:

```text
FieldObligation {
    record         : TypeId,
    field          : FieldId,
    proposition    : Proposition,
    evidence_ref   : Option<EvidenceRef>,   // Explicit evidence supplied by the programmer; None during automatic synthesis
    certificate_ref: Option<CertificateRef>,// Certificate synthesized by the producer and verified by check; None while unverified
    signature_hash : ContentHash,
}
CertificateRef { id: CertId, signature_hash: ContentHash }  // Points to Proved(checked) in the compile-time cache from §13
```

The two legal slot combinations are: `(Some(ev), None)` = explicit evidence awaiting replay;
`(None, Some(cr))` = automatically synthesized and already verified; `(None, None)` = awaiting
producer search; `(Some, Some)` is illegal (one obligation may not consume two proofs). After
`check` succeeds, `certificate_ref` is written, while `evidence_ref` is retained as provenance.

A `FieldObligation` is generated during the refined walk in `checker/refinement.rs`; field binding
reuses `bind_args_to_params` (D60's six rules), and decision reuses the sole unit
`judge_refined_obligation`. **No value is supplied when proof fails**: under §4.3, the programmer
supplies explicit evidence or the result is `Unproven` (E2031 / general E4023); there is no such
thing as an incorrectly written witness value.

A self-referential refinement field (`v: Pos(v)`) is a data field: its `Refined` constraint is
decided at the construction site under the existing call-site obligations (§3.1 / §3.3), is not an
evidence field, does not generate a `FieldObligation`, and does not change `runtime_field_index`.

#### 14.3 Generic Records: Proof Family Parameter `P` (kind `T -> Proposition`)

A type is a “parameterized record + a **proof family parameter**,” not a “generic witness to be
handled later.” The kind of `P` comes from the same source as the proof-space criterion in §14.1:
**the result sort must be `space=Proof`**, meaning that `P` is a proof/type family (a family of
propositions), not a data family (a data type family). `T -> Proposition` is kind notation
(`Proposition` is an alias for the `SortSpace::Proof` sort in §4.1), **not a source-level type
name**: the `yaoxiang` block below uses the `P: (T) -> Type` form accepted by the current parser;
the kind constraint is checked at declaration and instantiation and is not written into parseable
code.

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

// Source-level form: P is a type-level function parameter; the kind constraint (result space=Proof) is explained in the prose
Cell: (T: Type, P: (T) -> Type) -> Type = {
    v: T,            // Data field
    w: P(v),         // Evidence: P is a Proof family, and argument v is a strictly previous data field
}
```

- **Kind criterion** (prose notation, not source-level syntax): in `P: T -> Proposition`,
  `Proposition` is an alias for the `SortSpace::Proof` sort (§4.1); equivalently, the application
  result of `P` has `space=Proof`. `Array: (T: Type, N: Int) -> Type` has `space=Data`, so passing
  it to `Cell` is a **kind mismatch rejected at declaration time** (the parameter declaration's kind
  is `T -> Proposition`, so binding a data family is a type error, not “`w` degrades into a data
  field”; row 2 of §14.1 applies only to fields that **directly mention** a data type family). The
  multi-argument form `(T, U) -> Proposition` is analogous; all arguments land on strictly previous
  data fields in declaration order.
- `P` and `T: Type` both occupy compile-time parameter positions (q=0; RFC-011 §4.1). When
  `Cell(Int, Pos)` is instantiated, `P` is bound to the concrete predicate `Pos` (`space=Proof`),
  `w`'s proposition becomes `Pos(v)`, and the position rules are word-for-word those of §14.1;
  `Cell(Int, Array)` is rejected at declaration/instantiation because of the kind mismatch.
- `P` is not a value parameter and occupies no runtime slot. When `P` is unbound (the generic is not
  instantiated), `w`'s result sort remains `Proof` (the kind has been declared), but the proposition
  is open, so the **construction site returns `Unproven`** (not a declaration-time rejection),
  without silently passing or degrading to a data field. After instantiation, classification runs
  again under §14.1.
- Evidence classification for a generic record is rerun under §14.1 **after instantiation**:
  `StructType`'s `quantity` remains `Some(Zero)` after `substitute` substitutes `P`; the Struct arms
  of `mono::substitute` and `types::solver` pass `quantity` through per field and do not create a
  second classification.
- Export of the generic `Cell` carries the kind of `P` (the `T -> Proposition` notation) and the
  field-obligation template (§14.4) in `TypeDefPayload`; the correspondence between source-level
  `(T) -> Type` and the kind notation is given by the declaration/instantiation criterion.

#### 14.4 Cross-Module Export: `TypeDefPayload` Carries Propositions/Evidence Slots

When a record containing evidence fields is used across modules, its propositions and evidence slots
must be exported with the type definition; the importer must not “guess again by field name.” The
carrier is `frontend::module::TypeDefPayload`:

```text
TypeDefPayload {
    // …existing fields…
    type_params      : [TypeParam { name, kind }]        // Includes P: T -> Proposition (§14.3)
    field_obligations: [FieldObligation]                 // See §14.2; empty for records without witnesses
    evidence_slots   : [EvidenceSlot {
        field          : FieldId,
        quantity       : Quantity,                       // Always Zero
        proposition    : Proposition,                    // May contain a previous FieldPath reference
        evidence_ref   : Option<EvidenceRef>,            // Explicit-evidence provenance; None during automatic synthesis
        certificate_ref: Option<CertificateRef>,         // Verified-certificate reference; None while unverified
        signature_hash : ContentHash,                    // Invalidated according to the signature in §4.1
    }]
    libraries        : [LibraryRef]                      // Libraries referenced by propositions/evidence; carried across modules
}
```

- Export surface: when `checker.rs` constructs `TypeDefPayload`, it fills one `EvidenceSlot` for
  every field whose `quantity == Some(Zero)`; `field_obligations` and `evidence_slots` have the same
  source and are not written independently.
- Import surface: after receiving an `EvidenceSlot`, the importer verifies it using **its own
  current `LogicSignature`**—if `signature_hash` does not match, it follows the three
  cross-signature paths from §4.1 (translation term / bridging axiom / reproofing) and must not
  directly reuse the exporter's `Proved`; every library referenced from `libraries` must appear in
  the importer's `libraries` list, otherwise the result is `Unproven` with a prompt to register the
  library or write a bridge.
- Construction sites in multiple files/modules reuse the same `judge_refined_obligation`;
  cross-module use does not introduce a second decision entry point (RFC-039 D55 silent-channel
  prohibition).

#### 14.5 Type-Level `b.w`: `ConstExpr::FieldPath` + Evidence Term

A witness field may be used **only at the type level** (`b.w` is user-writable type-level syntax, as
described in the final note of this section), and value-level reads are always rejected:

- **Term representation**: `ConstExpr::FieldPath { base: Box<ConstExpr>, field: FieldId }` is a
  field-path term in the unified term world. `b.w` in type position resolves to
  `FieldPath(NamedVar("b"), "w")`; it does not denote a runtime value but the **evidence term**
  corresponding to `EvidenceSlot.evidence_ref` (a proof term).
- **Evidence term**: `EvidenceTerm(FieldPath, EvidenceRef, signature_hash)` is one source of
  `EvidenceRef` for a `Certificate` (§4.2), corresponding to the explicit evidence or verified
  certificate in the two slots `FieldObligation.evidence_ref` / `certificate_ref` (§14.2). When
  `b.w` is used in another proposition (for example, `Q(b.w)`), the producer treats the
  `EvidenceTerm` as a premise of the candidate certificate, and `Kernel::check` replays it under the
  signature; if `signature_hash` does not match, it is not reused.
- **Type-level uses**: `FieldPath` may occur in predicate arguments, type-constructor argument
  positions, evidence-term premises of another proposition, and
  `TypeDefPayload.evidence_slots[i].proposition` (indexed per slot). Note: when a `FieldPath` points
  to a witness, it is a **proof-term premise**, not a “data-field argument”—the §14.1 constraint
  that “evidence-field arguments consume only strictly previous data fields” applies to the data
  arguments of `P`; it does not prohibit using the proof term of a previous evidence field as a
  premise of a new proposition (the latter is replayed through `EvidenceTerm` and `check`). `b.w`
  produces no runtime value and occupies no data slot.
- **Value-level rejection**: when `b.w` appears in a value expression (the `Struct` arm of
  `FieldAccess` and the `TypeRef→Struct` arm), report **E1114** (`witness_field_not_readable`).
  `FieldPath` is not a value term, and neither the `Evaluator` nor IR generation materializes a
  value for it.
- The exhaustive `match` arms over `ConstExpr` (`const_eval::substitute`, `free_names`, `Display`,
  the `translate::` family, `proof_execution`, `fold_proof_call_arg`, etc.) must handle `FieldPath`;
  an evaluator that does not support the term language returns `Unproven` for `FieldPath` and must
  not silently treat it as `NamedVar`.

```text
// Design sketch (non-compilable fragment: Proved / FieldPath / evidence terms are kernel terms; see the interface definitions in this section)
// Type-level use: treat b.w as an evidence premise, not as a value read
useWitness: (b: Box) -> Type = { Proved(b.w) }   // b.w resolves to FieldPath + evidence term
// Value-level b.w => E1114
```

> **Superseding 4A's “no user syntax at the type level.”** The premise of the 4A decision (09a §P6
> 6.9.5 H0 decision, 2026-11-13) was that “`b.w` requires `ConstExpr::FieldPath` and a proof token,
> constituting an independent term-language design that would create a second term world,” and it
> therefore decided that V1 would not provide user-facing `b.w` projection. `ConstExpr::FieldPath`,
> `EvidenceTerm`, and `TypeDefPayload.evidence_slots` all belong to **the same unified term world**
> (§4, not a second term world), so that premise does not hold: **type-level `b.w` is user-facing
> syntax** (the `useWitness` example above). 4A's “no user syntax for type-level `b.w`” rule is
> superseded by this section and is no longer effective. The other two 4A rules—witnesses are
> compile-time fields, proofs are produced at construction time, and erasure occurs at runtime;
> field representation uses `quantity` + proposition type—remain in effect. Value-level `b.w` is
> still rejected under 4A (E1114). `FieldPath` exists only in type/proposition positions, is not a
> runtime field path, and does not constitute an “A-style runtime field.”

#### 14.6 Immutability, Erasure, and Diagnostic Codes

- **5A is wholly immutable**: a record containing any evidence field is wholly immutable—its data
  fields must not be declared `mut`, assigned as fields, borrowed through `&mut`, or used as the
  receiver of a write method (including automatic `WriteBorrow`). Otherwise, a witness proposition
  could become invalid after a field mutation, and there is currently no runtime check or
  revalidation; violations report **E1115** (`witness_record_immutable`). A finer-grained
  linearity/uniqueness design is deferred to a separate RFC.
- **Erasure**: an evidence field's `quantity=Zero` ⇒ it occupies no runtime slot; `CreateStruct` /
  `GetField` / `SetField` / `resolve_field_index` recognize only the data order from
  `runtime_field_index`; a `Proved(checked)` certificate is retained at compile time and erased at
  runtime under §13; a Runtime-mode Bool check is unrelated to witnesses.
- **Diagnostic-code division of labor** (four codes, each covering one concern, with no borrowed
  codes and no silent behavior):

| Code      | Trigger                                                                                                                       | Semantics                                                                       |
| --------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| **E4018** | A construction-site evidence proposition is disproved (a counterexample confirmed through `check` / a kernel-direct decision) | Refinement-predicate violation, with a counterexample                           |
| **E2031** | A construction-site evidence proposition is `Unproven` (refinement path, with the three slots `assigned`/`var`/`constraint`)  | The refinement constraint cannot be proved; prompt for explicit evidence (§4.3) |
| **E1114** | Value-level read of witness field `b.w`                                                                                       | Witness fields cannot be read at the value level                                |
| **E1115** | `mut` / field assignment / `&mut` / write receiver on a witness-bearing record                                                | The record is wholly immutable                                                  |
| **E4023** | General `Unproven` outside the refinement path (for example, the `FieldPath` evaluator does not support it)                   | General failure to prove, with proposition/reason/location (§12.3)              |

`ProofFunctionRequired` still uses E4020; the division of labor between general `Unproven` and E2031
is unchanged (§12.3, RFC-013). **Only three kinds of declaration-time rejection remain**:
`space=Proof` with a forward reference to a later field, a reference to another evidence field, or a
`P` whose kind is a data family (§14.3). These are rejected directly at the declaration site and do
not proceed to construction or silently degrade to data fields. A `space=Data` data type family
application (including `Array(Int, n)`) and a `space=Proof` self-referential refinement
(`v: Pos(v)`, admission) are legal data fields. **An uninstantiated generic is not a
declaration-time rejection**: under §14.3, when `P` is unbound, `w`'s result sort remains `Proof`
and its proposition is open, so the **construction site** returns `Unproven` (E2031 / E4023),
without silently passing or degrading to a data field.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                                       | After (this RFC)                                                                                                                                                            |
| ------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`                    | Compile-time predicate as the parameter type `(b: Positive(b))`                                                                                                             |
| `//! ensures: ExistsMax(result, arr)`                  | Return type uses a return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                                       |
| `/*! invariant: ... !*/`                               | Compile-time predicate type annotations on variables—Floyd–Hoare invariants                                                                                                 |
| `//! decreases: n`                                     | The measure is written in the refinement type position (`Terminates`); the compiler explores automatically first and requests an explicit measure only if exploration fails |
| Specifications are comments                            | Specifications are part of the type system                                                                                                                                  |
| No evidence record other than `/*! invariant: ... !*/` | Dependency record Σ: evidence fields (q=0) are distinguished from data-family fields by the proof-space criterion; see §14                                                  |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is the proof space and is exactly the same as
the existing type-definition syntax. A compile-time predicate is simply a function returning
Type—`name: (params) -> Type = { assertions }`. Its use is a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type; the compiler verifies the assertions inside {}
# Uses the existing function/type syntax; no new BNF rule is needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate-application arguments must have compile-time constant forms**—literals, variables (bound
by name), type applications (recursively extracted), or **compile-time referencable function
references** (function names). An argument shape that cannot be converted to a constant expression
reports **E1092**, and a mismatch between the number of arguments and the predicate's declared
parameters reports **E1093**. Arguments are bound positionally to the parameter list, and predicate
arity is determined by the declaration—`Positive(x)` is unary, `IsMax(T, arr, result)` is ternary,
and `Terminates(m)` and `Terminates(FnType, m)` are unary and binary. Refinement constraints are
**never silently discarded** (previously, an argument that could not be converted caused the
constraint to disappear silently, allowing a violating binding to pass).

**New syntactic concept: return-value formal parameter**—in `-> (name: Type)`, `name` is a
return-value formal parameter.

A return-value formal parameter is one of the syntactic concepts YaoXiang introduces over the
existing function syntax (record evidence fields reuse the existing field syntax; see §14). Its
semantics are:

- `name`'s value is supplied by a `return` statement
- `name` exists only in the type signature and is referenced by postcondition predicates (such as
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body's scope and does not appear at the call site
- A return-value formal parameter is **optional**—without a postcondition, the signature is exactly
  like an ordinary function (`-> Int`), with no additional burden

The reason for introducing it is that a postcondition needs to refer to “the value the function is
about to return.” Without a return-value formal parameter, the compiler could only enable such
references through a special rule, such as an implicit variable `$result` or `__retval__`. A
return-value formal parameter makes the reference explicit—it is simply a formal parameter whose
value is supplied by `return` rather than by the caller.

**A proof function** is not a new concept—it is simply a YaoXiang function whose return type is the
proposition being asserted. When the compiler returns Unproven, the programmer provides a proof
function, and the type checker verifies it in exactly the same way as verifying any function's
return type. No new syntax, keywords, or rules are needed.

> **The boundary between the two.** In the correctness domain (predicate Unproven), proof is
> supplied **in the body**: write a function whose return type is the proposition to be proved. In
> the termination domain (§6.9), the measure is supplied **in type position**: write
> `Terminates(measure)` as the type of a binding or function signature. The former means “write a
> proof for a proposition that cannot be proved,” whereas the latter means “explicitly declare a
> measure that exploration cannot find”—the mechanism is the same in both cases (a refinement-type
> application), but the location differs. The termination domain **does not require** a `_proof`
> function.

### Type-System Impact

- **Type universe**: compile-time predicates reside at the Type₂ layer—functions accepting values
  and returning Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates may carry generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules and may
  only be read, not written
- **Type inference**: compile-time predicate arguments participate in HM type inference
- **Dependent record Σ**: record fields are divided into data fields and evidence fields
  (`Quantity::Zero`) under §14's proof-space + position rules; proof family parameter
  `P: T -> Proposition` and `TypeDefPayload.evidence_slots` provide complete semantics for generics
  and cross-module use

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables are known at compile time): after `Kernel::check`
  verifies the certificate, `Proved(checked)` is retained at compile time and completely erased at
  runtime. `Positive: (x: Int) -> Type = { x > 0 }`—a parameter such as `b: Positive(5)` has runtime
  representation `Int`; the refinement condition `{ 5 > 0 }` has passed, and the certificate does
  not enter the runtime layout. Σ evidence fields behave the same way under §14: `base` does not
  enter the layout, `CreateStruct` is data-only, and value-level `b.w` reports E1114.
- **Runtime mode** (runtime free variables exist): retain a runtime check—execute a Bool check at
  the value level and inject it into the flow-sensitive assumption set Γ. See §11's dispatch
  pipeline and §13's erasure model.

Placing a compile-time predicate in a type position (such as `f(x: Positive(x))`) does not create a
wrapper type or allocate additional memory. However, when `x` comes from runtime input, a **runtime
Bool check** is inserted.

**Interaction constraint with `ref`**: a compile-time predicate may refer only to an immutable
borrow or a value whose ownership has been transferred. The compiler cannot guarantee at compile
time that a verification result remains valid at runtime when a predicate refers to a mutable
borrow—this use is rejected with a compilation error.

### Compiler Changes

1. **Parser**: compile-time predicates and proof functions use standard function syntax and require
   no additional parsing rules
2. **Proof kernel**: `Kernel::check(signature, certificate, proposition)` and `CheckVerdict`;
   replaying `Certificate.steps` according to `signature.rules` is the sole path to `Proved`
3. **Logical signatures and libraries**: `LogicSignature` (sorts / rules / axioms /
   `libraries: [LibraryRef]` / hash), with every `LibraryRef` carrying `kind/id/manifest/hash`; the
   signature hash covers all libraries; signatures are replaceable, and cross-signature use requires
   an explicit translation or bridging axiom
4. **Certificate producers** (untrusted): kernel-direct rules, Γ/`q=0` evidence, proof functions,
   SMT-LIB 2.6 translation (Z3 by default, CVC5 as an alternative), tactics, and proof search; all
   supply only candidate `Certificate`s, and `unsat` never reaches `Proved` directly (§8)
5. **Verification-condition generation**: WP/SP calculus, loop invariants, and the §14
   record-construction `FieldObligation` (field position rules / `bind_args_to_params` /
   `judge_refined_obligation`) produce propositions and candidate certificates without deciding them
   directly
6. **Error reporting**: counterexamples (confirmed through `check`) + unsolved proposition +
   reason + evidence-supplementation hint + source-location association (§12.3)

### Backward Compatibility

- ✅ Code that does not use compile-time predicates remains completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; Runtime mode retains
  only the necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden
- ⚠️ Record types gain §14's evidence-field position rules: programs that previously treated “a
  field name happened to be a predicate application and silently treated it as a data field” now
  receive compile-time witness classification (value-level reads report E1114, and witness-bearing
  records are immutable under E1115). This is an externally visible behavior change, registered as a
  D/E-class change under RFC-039 D59.

## Trade-offs

### Advantages

- **Complete realization of the Curry–Howard isomorphism**: types are propositions, programs are
  proofs, and `name: Proposition = Proof`; propositions and certificates inhabit D62's unified term
  world
- **Uniformity**: compile-time predicates and ordinary functions use exactly the same syntax, with
  no conceptual split
- **Transparent producers**: programmers need not know that SMT exists; the mental model is “supply
  explicit evidence → the kernel verifies it under the signature”; backends are replaceable
- **Incremental adoption**: begin with one compile-time predicate and gradually expand coverage
- **Minimal runtime overhead**: certificates and `q=0` terms are retained at compile time and erased
  at runtime; Runtime mode retains only the necessary Bool checks

### Disadvantages

- **Compilation time**: both certificate-producer search and kernel replay increase compilation
  time, but producer budgets (§8.4) and a certificate-step limit keep the cost bounded
- **Automatic-proof boundary**: when a producer cannot supply a verifiable certificate, the kernel
  returns `Unproven` and prompts for evidence. This is not a language defect—it is an inevitable
  consequence of the halting problem. The kernel honestly distinguishes “cannot compute” from “does
  not hold”
- **Learning curve**: writing effective compile-time predicates and proof functions, and choosing a
  logical signature, require understanding the Curry–Howard isomorphism and the basic intuition of
  “signature-relative” reasoning
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard producer budget limits (10,000 search steps / 100ms / instantiation depth 3) and a
  certificate-replay step limit; exceeding the budget returns `Unproven` (§8.4)
- Dependent-type pre-reduction: deterministic value computation is consumed first by kernel-direct
  rules, leaving only nondeterministic work to the producer
- `Unproven` is not a dead end: supply explicit evidence (proof functions, proof-library entries, a
  changed signature / bridging axiom; §4.3); in the termination domain, supply a measure in type
  position (§6.9)—all are verified by `Kernel::check`
- Incremental verification: `Proved(checked)` is cached by
  `signature_hash + proposition + certificate`, and only modules whose signature changed or whose
  certificate was invalidated are recomputed
- Clear error messages + counterexamples confirmed through `check` + unsolved proposition + reason +
  evidence-supplementation hint + a one-line budget summary (§12.3)

## Alternatives

| Alternative                                                        | Why it was not selected                                                                                                                                                                                                                                     |
| ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                        | Splits specifications from types, violating the Curry–Howard isomorphism                                                                                                                                                                                    |
| Separate specification files (such as CVL)                         | Separates specifications from code and increases maintenance cost                                                                                                                                                                                           |
| Runtime assertions only                                            | Cannot statically guarantee correctness                                                                                                                                                                                                                     |
| External proof assistants (such as Coq)                            | Disconnect from the compiler and require an independent proof language and trust boundary. YaoXiang's choice is that proof is YaoXiang code and certificates are verified by `Kernel::check` under a replaceable signature, without a hard-coded trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                                                                                                          |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                                           |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | `Kernel::check` + `LogicSignature` + `Certificate` + `CheckVerdict`; kernel-direct rules (structural equivalence, βδι reduction, defeq)                                                           |
| **Phase 2** | axiom library / proof library manifests and hashes; integration of proof functions and explicit evidence; a proof-producing adapter expands `unsat` into `ProofStep`s for registered rules        |
| **Phase 3** | Loop-invariant VC generation + termination checking (§6, §6.9) + dependent record Σ field position rules, construction obligations, and type-level `FieldPath` use (§14) all use the same `check` |
| **Phase 4** | Certificate caching by `signature_hash` + incremental verification + cross-module evidence slots in `TypeDefPayload` + cross-signature translation/bridging axioms + IDE support (§14.4)          |

### Dependencies

- RFC-010: Unified Type Syntax—compile-time predicates are based on `name: type = value`
- RFC-011: Generic System—compile-time predicates may carry generic parameters
- RFC-009: Ownership Model—expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **Z3 availability for the wasm32 target**: Z3 is unavailable under wasm (all SMT code is
      excluded by `cfg(not(target_arch = "wasm32"))`). Scope and downgrade direction: -
      `ownership.rs`'s `smt_cut` always returns `false` → back-edge traversal → **conservative
      rejection** (sound direction, merely less narrowing) - Termination-checking Strategy 1 and
      predicate level 2b/3 are skipped entirely

  **Correction (2026-10-01)**: the original text claimed that these paths were “currently
  **unreachable**” because `predicate_defs` was never populated in production, the parser never
  produced `MonoType::Refined`, and production never called `with_solver`. That is no longer
  true—all three have landed (predicate-definition registration #377-3; refinement-type
  normalization; production-pipeline solver injection). The wasm/native difference is therefore no
  longer limited to `smt_cut` precision:

  | Path                                            | native   | wasm       | Downgrade direction                   |
  | ----------------------------------------------- | -------- | ---------- | ------------------------------------- |
  | Explicit-measure termination decision           | Decided  | `Unjudged` | Does not report E4022 (fewer reports) |
  | Well-foundedness decision                       | Decided  | `Unjudged` | Does not report E4022 (fewer reports) |
  | SMT implication level for refinement predicates | Executed | Skipped    | Falls back to `Unproven`              |

  All are **conservative** directions (the producer has one fewer search path ⇒ fewer candidate
  certificates ⇒ `Unproven`; `Kernel::check` itself is unchanged), and soundness is unaffected.
  Under §8.7's terminology, the table below is interpreted in terms of “producer availability,” not
  “decision precision.” If identical precision is desired, the options and costs are described in
  issue #376 (conclusion: prioritize a JS-side Z3 instance rather than linking Z3 into the main wasm
  build— the latter would require replacing the emcc build system and would increase the artifact
  from 3 MB to ~20 MB).

- [x] **SMT solver choice**: Z3 by default (MIT license, the broadest verification adoption). CVC5
      is an SMT-LIB-compatible alternative, selected through a compiler flag. SMT-LIB 2.6 is the
      producer's translation target—it is an input format for candidate certificates, not a trust
      boundary, and the sole path to `Proved` remains `Kernel::check` (§8). No custom
      general-purpose solver interface is provided, and `unsat` is never treated as a decision
      result.
- [x] **Concrete solver-budget values**: producer search steps 10,000 / time 100ms / quantifier
      instantiation depth 3, plus a certificate-replay step limit (§8.4). These are fixed inside the
      compiler and provide no knob. They may be adjusted if actual use cases demonstrate that they
      are insufficient and that the problem is not “the user wrote it incorrectly.”
- [x] **Quantifier support range**: the language does not restrict quantifier order. Compile-time
      predicates accept Type parameters—Type includes function types—so higher-order quantifiers are
      a natural consequence of the type system and require no special syntax. A producer can handle
      first-order quantifiers (forall/exists, including interleaved nesting, limited by the budget
      depth of 3) and supply a candidate certificate; when it cannot supply a verifiable certificate
      for a higher-order quantifier, the kernel returns `Unproven` and prompts for explicit
      evidence—a YaoXiang proof function whose return type equals the proposition (§4.3), verified
      by `Kernel::check` under the current signature. No external export, AI, or interactive proof
      mode is needed. Everything is YaoXiang code, and everything is verified by the proof kernel.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names,
      with a module prefix to avoid conflicts. The producer's `sat + model` is a candidate
      counterexample and must be confirmed through `Kernel::check` before the proposition is
      rendered as false. Output format: variable name = concrete value + source location +
      predicate-definition location. No complex mapping layer is provided.
- [x] ~~**How do compile-time predicates interact with the `ref` smart pointer?**~~ → Decision:
      compile-time predicates may refer only to immutable borrows or values whose ownership has been
      transferred. Values under mutable borrow may not occur in compile-time predicates.
- [x] **Extending the `forall` predicate-violation-count measure to non-adjacent operations?** → Do
      not extend it. The current coverage (adjacent swaps, adjacent moves) is complemented by
      Strategy 1 (linear rank function)—the outer interval contraction of quicksort is covered by
      Strategy 1, and heap sort is covered by Strategy 1 (array-index pattern). A loop that cannot
      be proved terminating by any strategy causes a direct compilation error—this is the
      hard-safety philosophy, not a defect. If a real-world (rather than academic) algorithm arises
      that none of the four strategies can handle, the issue will be reconsidered.→
      **Reconsideration was triggered (2026-09-14, #318)**: non-structural recursion (gcd-style
      non-direct decrease, mutual recursion, merge partitioning) is exactly the kind of real-world
      scenario this clause anticipated—the template sequence for measure exploration cannot handle
      it automatically, and it cannot be rewritten as an analyzable iteration pattern without
      sacrificing readability. Conclusion: the hard-safety philosophy remains unchanged (an
      incorrect measure is still rejected by a counterexample confirmed through `check`; humans may
      fail to prove termination but may not prove it incorrectly), but “failure to explore ⇒ reject”
      is relaxed to “failure to explore may be handled by the programmer supplying an explicit
      measure in type position”—see §6.9.
- [x] **Combinatorial explosion in linear-rank-function enumeration**: candidate enumeration is
      limited to three bounded variables. At ≤3, all linear combinations are enumerated and each
      produces a candidate certificate for `Kernel::check`. At >3, only single-variable measures are
      attempted (`v_i`, `u_i - v_i`), and failure immediately reports a compilation error prompting
      the programmer: “the loop has >3 bounded variables; the automatic producer cannot synthesize a
      multivariable measure; please supply explicit evidence.” This is not an engineering
      compromise—it forces programmers to write simpler loops.

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

## Lifecycle and Destination

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
