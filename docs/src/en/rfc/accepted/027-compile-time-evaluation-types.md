---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-11-14'
impl_status: 'in-progress'
impl_detail:
  'Phases 1-2 are complete, Phase 3 is partially complete, and Phase 4 is partially complete. The
  unified assert/Assert approach is fully implemented across all 6 phases (#157-#162 closed): the
  Never type, the IsTrue bridge, flow-sensitive Γ + kill set, type-level recursion, and
  universe-stratified weak checking. The dispatch **semantics** in §11 have been implemented through
  call-site obligations (`checker.rs` calls `check_predicate` directly in three places: binding
  position / call-site actual arguments / return position), but the standalone `layers/dispatch.rs`
  module has been deleted—it had zero production call sites, and its handling of `Unproven`
  (downgrading it to a W1080 warning) conflicted with the production implementation (always
  reporting an error) (#377-2). The proof kernel, logic signature, certificates, and SMT role have
  been rewritten in accordance with the unified term world of RFC-039 D62 (§4, §8, §13):
  SMT/tactics/proof search are **untrusted certificate producers**; `unsat` is merely a search
  result, and Proved may be obtained only by constructing a certificate and verifying it with
  `Kernel::check`. The current `SMTResult::Unsat => Ok(())` in `predicate.rs` is the gap that
  blindly trusts SMT; see §8.6. §14 completes the dependent record Σ in place: the proof-space field
  criterion, data-only constructors + automatic evidence synthesis, the proof family parameter `P: T
  -> Proposition`, `TypeDefPayload` cross-module evidence slots, type-level `b.w` for
  `ConstExpr::FieldPath`, 5A immutability, and the division of responsibilities among the four
  codes.'
impl_percent: 82
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax — The name: type = value Model](010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Refinement Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This paper proposes introducing **compile-time predicates** into YaoXiang as first-class citizens
and unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an add-on specification annotation—it is a function. A function that returns a Type
can be used in type position; the compiler calls it at compile time and checks its return value.
Types are propositions, and compile-time evaluation is proof.

**Core argument**: the sole task of type checking at compile time is to construct and verify proof
terms. Type equalities, token conflicts, dependent type reduction, compile-time predicate
evaluation, and Hoare-logic implication are all different kinds of type checking within the same
compile-time proof pipeline. They share one pipeline and one proof kernel, `Kernel::check`. Proof
terms and propositions live in the same **unified term world** (RFC-039 D62). A **logic signature**,
`LogicSignature`, gives propositions their meaning; proof terms are represented by **certificates**,
`Certificate`. Relative to a signature, a certificate is validated only by the kernel according to
that signature’s rules. SMT solvers, tactics, and proof search are all **untrusted certificate
producers**: they merely search for certificates, and `unsat` is not `Proved`. When the kernel
returns Unproven, the programmer writes a YaoXiang proof function as explicit evidence—it is a proof
term, and the kernel verifies it in exactly the same way it verifies any function’s return type.
Everything is YaoXiang code, and everything is verified by the proof kernel.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This makes a fundamental error in the Curry–Howard correspondence: **it separates specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments embody an “external tool” mental model.

The white paper states this plainly:

> “There are no `//!` comments. There is no separate specification language. Everything is within
> the type system.”

### Current Problems

- RFC-022’s `//!` comments are an add-on syntax independent of the type system.
- Refinement types and ordinary types form two separate systems, causing conceptual redundancy.
- The split model in which verification is performed in Debug builds but ignored in Release builds
  breaks unification.
- In conventional understanding, an SMT solver is positioned as an external tool or as “trusting
  Z3’s `unsat`”—YaoXiang instead positions it as an untrusted certificate producer: it produces
  candidate certificates that are verified by the proof kernel.
- Type checking, borrow checking, compile-time predicate checking, and macro expansion all follow
  different paths.

### The Correct Mental Model

Type checking can be abstracted as a function:

```
Kernel::check(signature, certificate, proposition) → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, and compile-time predicate
verification—are subtasks of this function. They share one proof pipeline and differ only in the
complexity of the proof term and the construction strategy.

Every obligation carries the currently effective `LogicSignature`. When the kernel returns Unproven,
the programmer supplies explicit evidence: a proof function whose return type equals the proposition
to be proved serves as the certificate. The kernel verifies it using the same signature—and this is
the same operation as ordinary type checking. Signatures are replaceable: changing the signature
changes the set of inference rules, but neither the kernel nor the three-valued result changes
(§4.1).

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, and Verification Is Type Checking

In YaoXiang, `{}` is the compile-time proof space. Everything inside it is an assertion, and the
proof kernel guarantees under the current `LogicSignature` that every item has a verifiable
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
#          Parameter at the signature         {} contains only assertions
#          The compiler verifies x > 0 when invoking it at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter at the signature           The compiler verifies type_of(T) == Type and type_of(data) == Array(T)
```

The pattern is the same in each case: `name: (params) -> Type = { assertions }`. The compiler does
not distinguish “type assertions” from “value assertions”—both are evaluation targets in the proof
pipeline.

**Loop invariants do not need to be written separately. A type annotation on a variable is a
Floyd–Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation refers to i—it tells the compiler that the type of s depends on i
    mut i: UpTo(arr.len) = 0     # During initialization i=0; verification: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # The compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # Changing i triggers re-verification of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—the inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
producer produces a candidate certificate for the inductive step and `Kernel::check` verifies it,
all iterations are covered automatically. No `: decreases` is needed, no `: Invariant` is needed,
and no inductive proof is needed—the induction is decomposed into a local VC for each assignment.

### 2. Preconditions/Postconditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022’s `//! requires`/`//! ensures`. A compile-time predicate is used as a type
annotation on a parameter or return type.

**On the parameter side, it is a function call.** A compile-time predicate is a function that
returns Type; its use on the parameter side is simply a call—just like `factorial(5)`. The
return-value side introduces a new concept: a return-value formal parameter.

```yaoxiang
# Precondition: explicitly invoke a compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name and is passed to Positive as an argument
#                       At the call site, the compiler extracts the actual value, substitutes it for b, and verifies Positive(actual)
#                       Example: divide(10, 2) → verifies Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verifies Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value formal parameter; its value is supplied by return
#                                            At the return point, the compiler substitutes the return value and verifies the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current formal parameter name and is passed to
  `Positive` as an argument. This uses function-call syntax, with zero implicit behavior.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is a return-value formal parameter
  whose value is supplied by a `return` statement. `result` exists only in the type signature and is
  referenced only by predicates. It does not enter the function-body scope and does not appear at
  the call site.
- **Return-value formal parameters are optional**: when there is no postcondition, it is omitted and
  the signature is identical to that of an ordinary function (`-> Int`).
- **Unification**: parameter and return-value formal parameters are the same
  concept—`formal parameter name: predicate call(actual value)`—and differ only in whether the value
  is supplied by the caller or by `return`.

> **Implementation note (2026-10-02)**: recognition of return-position formal parameters is
> **strictly based on the declared name**—the `r` in `-> (r: P(...))` is the sole return-value
> formal parameter. Any other free variables in the constraint that are **not in scope** are no
> longer substituted with the return value; instead, an undefined-identifier error is reported
> (observed: `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` is undefined) →
> `error [E1001] Unknown variable: 'm'`). The actual arguments of a predicate call are substituted
> into the constraint according to the **actual arguments**: `P(r + 100)` becomes “return value +
> 100” after substitution, rather than merely “return value”.
>
> **Known gap (2026-10-02)**: **multi-parameter predicates at the return position + symbolic
> arguments are not yet supported**. Triggering form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Alternative: replace the actual argument with a compile-time-foldable literal (such as
> `SumUpTo(3, r)`), or provide a proof function for the predicate that returns `Type`. Resolution
> condition: the proof kernel must support argument substitution for “multi-parameter predicate +
> symbolic argument”. See [RFC-027a](../accepted/027a-termination-explicit-measure.md)
> §Implementation Record (2026-10-02, this hardening round).

### 3. Path-Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding position, its arguments are explicitly supplied
by the programmer. When a runtime value enters a refinement-type parameter, the producer collects
path conditions and produces a candidate certificate, which `Kernel::check` verifies under the
current `LogicSignature`. The programmer need not implicitly pass proof evidence; explicit evidence
is a first-class form of supplementary evidence (§4.3).

#### 3.1 Explicit Function Calls

When a compile-time predicate is used at a binding position, its arguments are explicitly supplied
by the programmer—this is just a function call, with zero implicit behavior.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, or return type), the programmer
explicitly supplies the name of an already-bound variable:

```yaoxiang
b: Positive(b)
// b has already been declared as the current formal parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler need not fill in arguments implicitly—`b: Positive(b)` is a function call, just like
`f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` refers to itself.
This is the standard pattern for dependent types, not an implicit unfolding rule.

**Unification with RFC-010 `self`**: RFC-010 established that `self` is not a keyword, merely a
naming convention for parameters (“writing `p`, `this`, or `x` has exactly the same effect”).
`b: Positive(b)` uses the same mechanism—the name of a parameter may be referenced in a type
annotation. `self` appears in the position `self: Point`, whereas `b` appears in the position
`b: Positive(b)`; both type annotations refer to the parameter itself. The only difference is the
complexity of the type annotation; the mechanism is exactly the same—once a name is bound, the type
may depend on that name.

Return types use explicit function calls in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal parameter; Sorted(result) is a function call
//                        At the return point, the compiler substitutes the return value for result and verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5; Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0; Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path-Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions to form the current scope’s **assumption set**. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline determines that the implication holds → Proved
} else {
// The assumption in this branch: { !(y > 0) }
// To call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline determines that the implication does not hold → Disproved
}
```

This is not a hard-coded special mode in the compiler—it is the natural behavior of the compile-time
proof pipeline. At every type-check call site, the pipeline receives:

```
{background assumptions} ⇒ {verification target}
```

The pipeline determines whether the implication holds. Proved → pass; Disproved → compile error +
counterexample; Unproven → compile error + unsolved proposition. Background assumptions come from
the path conditions at the current program point.

#### 3.3 Assumption Stack

While analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → push `y > 0` onto the true branch and `!(y > 0)` onto the false branch
  (if an else branch is used).
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` within the branch.
- **Logical connectives**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` within the branch.
- **Function preconditions**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the actual argument’s own refinement-type
  annotation (if `b` is annotated as `Positive`, its type carries `b > 0`).
- **Assignment**: when executing `let z = y`, the refinement conditions already attached to `y` are
  transferred to `z`.

All assumptions enter the `context` field of `Certificate` (`Γ`) and are handed to `Kernel::check`
together with the candidate certificate; only when the producer needs SMT does it translate the same
`Γ` into SMT-LIB background assertions.

#### 3.4 Compile Error When No Static Evidence Exists

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no `y > 0` assumption, and the actual argument `y` has no `Positive`
type annotation of its own. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (the implication does not hold) → compile error:

> Unable to prove that parameter `b` satisfies `Positive` in the call to `divide`. `y` comes from a
> function input and has no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not permit a runtime value to enter a refinement-type parameter without static
evidence. This is not a limitation—it is the core of the strict safety philosophy: code for which
`Kernel::check` cannot verify a certificate under the current signature must not compile.

#### 3.5 Relationship to the Unified Pipeline

Path-condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline into control-flow analysis:

| Stage                             | Responsibility                                                                                                                                   |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Path-condition collection         | During control-flow analysis, the compiler annotates each basic block with an assumption set                                                     |
| Verification-condition generation | When it encounters a type constraint requiring verification, it merges path conditions + actual-argument type information                        |
| Proof-pipeline verdict            | The certificate producer produces a candidate certificate → `Kernel::check(signature, certificate, proposition)` → three-valued result           |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition + supplementary-evidence prompt |

There are no new components and no special rules. Path conditions are the local assumption set `Γ`
of a `LogicSignature`; they share the same pipeline and budget system as type equalities and borrow
constraints.

### 4. Proof Kernel: `Kernel::check`

All compile-time checks share one pipeline and one **proof kernel**. The kernel’s sole decision
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
one thing—replay the derivation steps of the `certificate` according to the `rules` of the
`signature`, checking whether each step’s rule instance, premises, context, and substitutions are
valid, and verifying that the final conclusion is exactly `proposition`. The kernel does not search,
guess, or complete certificates; a certificate missing a step, using the wrong rule, or reaching the
wrong conclusion must not yield `Proved`.

This is the same picture as the **unified term world** of RFC-039 D62: type positions and value
positions share the same compile-time terms. The proposition `Proposition`, proof term
`Certificate`, and the signature’s sorts/rules/axioms are all terms in this one system; there is no
second proof language. Definitional equality (§12.1) is pure decision, the evidence for a `q=0` slot
(RFC-011 §4.1) is a call-side obligation, and a Σ witness (§13) is a construction-site
obligation—all three feed into the same `check` rather than opening separate trust channels.

The three values of `CheckVerdict<Certificate>` are consistent with §4.1: `Proved(checked)` carries
a certificate verified by the kernel, `Disproved(model)` carries a counterexample, and `Unproven`
carries the unsolved proposition and reason. Only `Proved(checked)` can satisfy an obligation;
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

`check` returns three results—an inevitable consequence of the halting problem and a natural
partition of proof theory:

```text
CheckVerdict<C> = Proved(checked: C) | Disproved(model: Model) | Unproven(reason, subject)
```

- **`Proved(checked)`** → halts; the certificate has been verified step by step according to
  `signature`, and its conclusion equals `proposition`. The obligation passes; `checked` is the
  certificate that can be cached at compile time (§13).
- **`Disproved(model)`** → halts; a counterexample `model` exists. Compile error + counterexample +
  source location.
- **`Unproven(reason, subject)`** → within the current signature, evidence, and resource limits, the
  producer did not deliver a verifiable certificate. Compile error + unsolved proposition + reason +
  **supplementary-evidence prompt** (the diagnostic shape in §12.3).

**Unproven ≠ False.** The kernel saying “I cannot compute it” is not equivalent to the proposition
being false—it only means that, under the current `signature` and current evidence set, there is no
certificate that passes verification. This is honesty, not a defect.

The **logic signature** `LogicSignature` determines “what counts as a valid derivation step.” It is
replaceable data:

```text
LogicSignature {
    sorts        : Map<SortName, SortDecl>          // Categories of terms and propositions
    rules        : Map<RuleId, InferenceRule>       // Derivation rules replayable by the kernel
    axioms       : Map<AxiomId, Proposition>        // Axioms accepted by this signature
    libraries    : [LibraryRef]                     // All axiom/proof libraries; see below
    hash         : ContentHash                     // Covers all entries in sorts/rules/axioms + libraries
}
LibraryRef {
    kind         : LibraryKind                     // Axiom | Proof
    id           : LibraryId                       // Namespace for entry ids within the library
    manifest     : Manifest                        // Entry ids + proposition/rule shapes + provenance
    hash         : ContentHash                     // Content hash over the canonical bytes of the manifest
}
SortDecl {
    space        : SortSpace                       // Proof | Data; see below
    arity        : [SortName]                      // The argument sorts accepted by this sort constructor
}
SortSpace = Proof | Data
```

Each `SortDecl` in `sorts` has a `space`, which is the **sole criterion for whether an application
of a type family is a proposition or data**. `space = Proof` means that the inhabitant of the sort
is a proof term (the result of a proposition, refinement type, or proof family); `space = Data`
means that the inhabitant is runtime data (`Int`, `Array`, `Vec`, or an ordinary record). Refinement
predicates such as `Pos` (`{ v > 0 }`), `Assert`, and `Terminates`, as well as `proof`/`type`
families returning a `space=Proof` type, have `space=Proof`; data type families such as
`Array: (T: Type, N: Int) -> Type` have `space=Data`. `space` is registered by the library manifest
or predicate declaration, not guessed from field names, argument shapes, or syntax that “looks like
a predicate”; the evidence classification in §14.1 reads only this field. A data type family remains
data even when it depends on strictly preceding fields.

`libraries` is a **list**: one signature may use multiple axiom libraries and multiple proof
libraries simultaneously—for example, the built-in `linarith` axiom library + the user’s `zfc-core`
axiom library + the `std-proofs` proof library + a project-private proof library. Each library
carries its own `kind`, `id`, `manifest`, and `hash`; every entry in `axioms` belongs to a library
with `kind=Axiom` according to its `LibraryId`, while proof-library entries belong to a library with
`kind=Proof`. `signature.hash` covers all entries in `sorts/rules/axioms` **and `libraries`** (not
merely one library); adding or removing any library or changing even one byte of any manifest
changes the signature and invalidates the cache key, certificates, and incremental reuse.

- `sorts`, `rules`, and `axioms` are the signature proper; every entry in `axioms` must belong to a
  manifest entry of some `kind=Axiom` library in `libraries`. If a certificate references an axiom
  outside any registered library entry, `check` rejects it.
- Every `LibraryRef` in `libraries` has its own manifest and content hash: the manifest lists entry
  ids, proposition/rule shapes, and provenance, and `hash` is computed over the manifest’s canonical
  bytes; `kind` distinguishes axiom libraries from proof libraries. The signature `hash` covers
  `sorts/rules/axioms` and **all** `LibraryRef` entries in `libraries` (not just two).
- Built-in libraries (`Int` linear arithmetic, `Array` lengths, the `Assert` bridge, etc.) are also
  ordinary `LibraryRef` entries in `libraries`; they merely ship with the compiler. They are listed
  alongside user and project libraries and are **not hard-coded trust roots in the kernel**. Trust
  derives from “this signature and its `libraries` list were adopted by this compilation,” not from
  the name of a library or solver.

Signatures are replaceable, and replacement is replacement of data rather than replacement of the
kernel:

| Signature                                         | Values of sorts / rules                                                | Typical use                                           |
| ------------------------------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------- |
| Classical arithmetic fragment                     | Linear-arithmetic sorts + no quantifier rules                          | `x > 0`, `0 <= idx < arr.len`                         |
| Classical first order (including excluded middle) | Classical sequent rules + law of excluded middle                       | Classical implication, proof by contradiction         |
| Constructive                                      | Constructive rules, no excluded middle, no double-negation elimination | Extracting witnesses, avoiding nonconstructive proofs |
| ZFC fragment                                      | Set-theory sorts + registered axioms such as choice and extensionality | Propositions requiring set-theoretic vocabulary       |

`Proved` is always **relative to a signature**: the same proposition may be `Proved` under a
classical signature but `Unproven` under a constructive signature. A certificate carries
`signature_hash`; `check` first verifies that the hash matches and then replays the certificate; a
mismatched hash immediately yields `Unproven`, without attempting “rough compatibility.”

**Cross-signature reasoning must be explicit.** Let `A`/`B` be two signatures, `P` a proposition in
`A`, and `certA` a certificate verified under `A`. To use the conclusion in `B`, there are exactly
three legal paths, each of which must leave an inspectable trace:

1. **Translation term**: provide `translate: A.Proposition → B.Proposition` and a certificate for it
   verified by `check` under `B`; obtain `Q = translate(P)`, then prove `Q` under `B`.
2. **Bridging axiom**: register an explicit bridging axiom in a `kind=Axiom` library in
   `B.libraries` (for example, “the corresponding rule of `A` has an interpretation on the relevant
   sort in `B`”); the certificate references `(LibraryId, AxiomId)`, and `B.hash` changes with that
   registration.
3. **Reproofing**: directly construct a new certificate for `P` under `B`.

Implicit transfer of “`Proved` under `A` ⇒ `Proved` under `B`” is never permitted, and neither is
reusing `certA` after replacing its signature hash. Cross-module export under Σ (§13) and the
cross-module form of `q=0` evidence follow these same three paths.

A hard budget limit is the engineering solution to the halting problem (values in §8.4). There is no
knob—providing one amounts to asking the user “do you think your program will halt?”, which neither
the user nor the compiler knows. Exhausting the budget yields `Unproven(reason=BudgetExceeded)`, not
`Disproved`.

#### 4.2 `Certificate`: A Signature-Relative Proof Term

A **certificate** is the compile-time representation of a “proof term” and has meaning only relative
to some `LogicSignature`:

```text
Certificate {
    signature_hash : ContentHash            // Must equal signature.hash
    proposition    : Proposition           // The conclusion, a term in the unified term world
    steps          : [ProofStep]           // A derivation sequence according to signature.rules
    evidence_refs  : [EvidenceRef]         // References to explicit evidence
}
ProofStep {
    rule       : RuleId                   // Must match signature.rules
    premises   : [StepId | AxiomId | GammaId | EvidenceId]
    context    : Γ                       // Local assumptions (path conditions, equality bridges, q=0 evidence)
    conclusion : Proposition
    subst      : Substitution            // Term substitution, a term in the unified term world
}
EvidenceRef = ExplicitProof(AstNodeId)     // A proof function / explicit evidence written by the user
            | LibraryAxiom(LibraryId, AxiomId)
            | LibraryProof(LibraryId, ProofId)
            | ProducerHint(bytes)          // A hint from SMT/tactics/proof search; not trusted directly
```

For each `ProofStep`, `check` performs four mechanical tasks: ① `rule` exists in `signature.rules`;
② `premises` refer to earlier steps, registered axioms, `Γ` facts, or explicit evidence, and each is
resolvable; ③ after applying `subst` to the rule pattern, it is identical to `conclusion` (identity
is decided by the pure procedure in §12.1); ④ the final step’s `conclusion` is identical to
`certificate.proposition`. If any of these does not hold ⇒ `Unproven`, and the kernel identifies the
offending step; **it does not skip these four checks merely because the producer said “unsat.”**

`ProducerHint` is only a hint: a producer may attach it to a candidate certificate—for example, an
SMT-LIB script, a model, or a tactic trace. The kernel neither executes it nor treats it as a rule;
the hint serves only to describe “how to complete the candidate into checkable `steps`.” If
completion fails, the result remains `Unproven`.

#### 4.3 After Unproven: Supply Explicit Evidence (Proof Functions Are First-Class)

When the kernel returns `Unproven`, diagnostics must prompt the user to **supply evidence**, rather
than rendering “cannot compute it” as “does not hold” (for the diagnostic shape, see §12.3). Legal
forms of supplementary evidence are treated as “first-class”—they are proof terms in the unified
term world, not add-ons:

- **Proof function**: a YaoXiang function whose return type equals the proposition to be proved. It
  is `ExplicitProof(AstNodeId)`, and the kernel verifies it by exactly the same mechanism as
  verifying `add(a, b): Int`.
- **axiom library entry**: an axiom registered in the signature, `EvidenceRef::LibraryAxiom`.
- **proof library entry**: a named proof (reusable, unit-testable, and equipped with a manifest
  hash), `EvidenceRef::LibraryProof`.
- **Existing kernel certificate**: a certificate already `Proved(checked)` under the same
  `signature_hash`, reused directly or as a premise.
- **Change the signature / add an axiom / write a translation term**: explicitly register and re-run
  `check` according to the three cross-signature paths in §4.1.

```text
命题    = 类型（统一项世界里的 Proposition）
证明    = 程序（该类型的一个值，即 Certificate 的 steps）
验证    = Kernel::check(signature, certificate, proposition)
信任    = 该签名被本次编译采用 + 证书逐步可重放；不来自求解器
```

If an SMT/tactics/proof-search candidate certificate cannot be completed into `steps`, it falls back
to `Unproven`—the programmer supplies explicit evidence or asks the producer to try another search
path. No AI is needed, no export to Coq is needed, and no new concepts are needed.

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
evidence (a first-class proof term) → the kernel verifies it according to the signature.** The
entire process is a smooth continuum—the producer finds easy certificates for you, leaving the
difficult thinking to you.

#### 4.4 Layered Dependencies Within the Pipeline

The producers above share the same `Kernel::check` interface, but there is an orchestration order.
Type equality is a prerequisite for all subsequent analyses; ownership/token checking depends on
type information; refinement-predicate verification depends on the results of the preceding two
layers. The compiler orchestrates the layers, and expressions that fail at a lower layer do not
proceed to higher layers, avoiding wasted search budget on programs that already contain type
errors; **the conclusion of every layer must still be verified by `check`; the layer order only
determines which candidate certificate is requested first**.

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

### 5. Unification of the Three Kinds of Function

| Layer                  | Execution time | Input      | Output | Example                                        |
| ---------------------- | -------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time   | Type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time   | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors follow
the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd–Hoare Verification-Condition Generation

Loops do not need a separate `: Invariant(...)` or `: decreases(...)` annotation. Compile-time
predicate type annotations on variables define Floyd–Hoare-style assertions—the compiler generates
verification conditions from the type annotations, and the proof pipeline checks that every
assignment preserves the type.

The core mechanism is that every assignment corresponds to a Hoare triple `{P} x := e {Q}`, with the
verification condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop
body—once the proof pipeline verifies the inductive step, all iterations are covered automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation refers to i; during initialization i=0; verification: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verification: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Premises: s satisfies SumUpTo(arr, i), and i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (i is unchanged)
        #   Substituting s_new = s_old + arr[i]:
        #     Requires s_old + arr[i] == sum(arr[0..i+1])
        #     From the inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → the type annotation of s refers to i in the dependency graph → re-verification is triggered
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), that is, s == sum(arr[0..arr.len])
}
```

A loop invariant is the type annotation on a variable—the programmer writes the type, and the
compiler checks the inductive step. The compiler need not “discover” invariants or “perform
induction automatically”; it decomposes an inductive proof into a local verification condition for
each assignment operation and hands them to the proof pipeline to solve by divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The mechanism above requires the compiler to know that the type annotation `SumUpTo(arr, i)` of `s`
refers to `i`—when `i` changes, the type constraint of `s` changes as well. This requires the
compiler to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# The key is the depended-on variable; the value is the set of variables whose type annotations refer to that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it parses
free-variable references in the arguments of `Pred(...)`. If an argument refers to another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: when a depended-on variable `x` is assigned, the compiler:

1. Finds every variable in the dependency graph that depends on `x`, namely `{v₁, v₂, ...}`.
2. For each `v`, generates a verification condition: “does the current value of `v` satisfy the
   updated type `Pred(... x_new ...)`?”
3. Sends the VC into the proof pipeline.

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Using `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify that s satisfies SumUpTo(arr, i_new) → True

# Wrong order—the compiler rejects it
i += 1        # i changes → re-verify that s satisfies SumUpTo(arr, i_new)
              # s has not yet been updated; s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: one variable may depend on several variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—a change to either triggers re-verification.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent verification mechanism. It answers “when must a VC be generated?”—the proof pipeline
answers “does the VC hold?”

### 7. Termination Checking

**Scope: refinement types.** Termination is not an independent switch; it is part of a
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation it
annotates enters verification mode, and termination is required within that mode. Ordinary types
that are not refined do not enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: a bare `while` does not enter verification mode; when a measure variable has a
  refinement annotation (such as `i: UpTo(n)`), the mode is entered and termination must be proved.
- **Recursion**: when a function signature contains a refinement (a refined parameter or a return
  type containing a refinement), the mode is entered, and the measure must be shown to decrease
  strictly at every recursive call site.

Automation has priority within the mode: the compiler first explores for a measure automatically; if
it proves one, the code passes. If exploration fails and no explicit measure was supplied,
compilation fails. There is no **annotation syntax** escape hatch—the measure and termination
proposition are written in type position, introducing no new `decreases`-like syntax. See §6.9 for
the forms of measures and explicit fallbacks.

#### 6.1 Design Principles

The compiler automatically extracts the information needed to prove termination from two places:

1. **Variable type annotations**: bounds in refinement types (such as `UpTo(n)`, which supplies the
   upper bound `n` and lower bound `0`).
2. **Loop-body operations**: the operation applied to a variable on each iteration.

The compiler tries four measure-synthesis strategies in priority order and stops at the first
success. The four strategies are a **restricted template sequence for measure exploration**. Their
input consists of refinement constraints (strategies 1–4 all begin with “a variable of bounded
type”), rather than “code evaluated at compile time”; they are the automatic and manual faces of the
same mechanism as the explicit measure in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded patterns, multiplicative scaling); it does not guarantee
> that a solution exists—general measure inference is undecidable (it reduces to the halting
> problem). Cases outside the templates must therefore be supplied explicitly by the programmer
> (§6.9), or compilation fails.

#### 6.2 Strategy 1: Automatic Synthesis of a Linear Rank Function

When a variable has a linear-bound annotation, the producer enumerates candidate linear measures and
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
binary searches:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# The measure high - low decreases strictly on both paths
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

#### 6.3 Strategy 2: Predicate Violation Count—Automatic Measure Extraction from the Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy; whether to include it will be decided during Phase 3
> implementation according to practical feasibility.** This strategy works for adjacent-swap
> operations (bubble sort, insertion sort), but cannot automatically prove non-adjacent operations
> (quicksort partition, heap-sort sift-down). The coverage boundary is given in the table below. If
> Phase 3 validation proves it infeasible, the strategy will be removed or deferred to future work.

Core insight: **the specifications written by the user are material for compiler reasoning.** The
compiler needs no built-in definition of “what sorting means”—it reads the definition of `Sorted`
and automatically extracts a measure from it.

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

| Algorithm      | Operation pattern  | Can Strategy 2 prove it? | Reason                                                        |
| -------------- | ------------------ | :----------------------: | ------------------------------------------------------------- |
| Bubble sort    | Adjacent swaps     |            ✅            | `violation_count` decreases strictly on every swap            |
| Insertion sort | Adjacent moves     |            ✅            | Every shift eliminates one violating pair                     |
| Selection sort | Non-adjacent swaps |            ❌            | A single swap may increase `violation_count`                  |
| Quicksort      | Partitioning       |            ❌            | Non-adjacent swaps do not guarantee monotonic decrease        |
| Heap sort      | sift-down          |            ❌            | Tree-shaped operations are not monotonic in `violation_count` |

**Complementary strategy**: for quicksort, contraction of the `low < high` interval can be covered
by Strategy 1 (the linear rank function)—the outer partition recursion halves the interval each
time. Strategies 1 and 2 complement each other and together cover most practical algorithms.
Generalizing Strategy 2 (non-adjacent operations, tree-shaped operations) remains an open problem.

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

For `v += const` (a positive constant), if the variable has an upper-bound type annotation → the
measure `upper_bound - v` decreases by `const` each time, with lower bound 0. This is a degenerate
case of Strategy 1, which the compiler handles first as a fast path.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

For `v *= const` (const > 1), if the variable has lower- and upper-bound type annotations. The
compiler includes a built-in logarithmic measure template `ceil(log_const(upper/v))`; multiplying by
const decreases the measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically derives the measure ceil(log₂(n/i)); multiplying by 2 decreases it by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove that a loop exits in finitely many
  steps; if exploration fails, the programmer supplies a measure in type position (§6.9).
- **Correctness**: whether the loop body makes progress toward the target type is checked by the
  compile-time proof pipeline through verification conditions.

If both pass → compilation passes. If termination is proved but correctness fails → compile error +
counterexample. If correctness is proved but termination cannot be proved → compile error
identifying the variable or operation that could not be analyzed. If both fail → compile error
reporting the two failure causes separately.

#### 6.7 Termination Checking of Recursive Functions

For recursive functions with refinement signatures, the compiler checks that formal parameters
decrease at every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // The compiler explores: the well-founded-order measure of (b, a % b) decreases → termination
}
```

Formal-parameter decrease is the strongest automatic path (structural recursion). If exploration
fails, the programmer explicitly supplies a measure in type position (§6.9).

#### 6.8 Hard Boundary

For `i = f(i)`, where `f` is neither invertible nor closure-preserving nor monotonicity-preserving,
automatically proving termination is mathematically impossible. Compilation fails:

> This loop cannot be automatically proved to terminate. The loop variable depends on the
> unanalyzable function `f`. Use an iteration pattern that the compiler can analyze, or bind a name
> to the loop and provide a measure in type position (§6.9).

This is not a compiler failure. Code that cannot be statically proved safe must not compile. Even
when an explicit measure is supplied, a certificate verified by `Kernel::check` must be provided for
both decrease and well-foundedness—an incorrect measure is rejected by a counterexample. A human can
fail to prove something, but cannot prove something false.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure in **type
position**—through the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with no new syntax:

```yaoxiang
// Measure: an ordinary function, unit-testable and reusable, not participating at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component grows in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor; the measure obtains values from names in scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The rationale for the loop is that `Terminates(m)` refines the value type of the loop body’s tail
expression, and the binding name `acc` provides the anchor—the loop can therefore be referred to,
eliminating the “no way to refer to an anonymous construction” corner case.

> **The body tail must provide a value**: the value of a loop = the value of the loop-body block
> (§2.9), and the value of a block = its tail expression. If the body tail is an assignment
> statement (such as `{ i = i + 1 }`), the block value is `Void`, and `return acc` would return
> `Void` from `-> Int`, which is ill-typed. The example therefore explicitly writes `i` as the body
> tail. This does not change the measure’s semantics—the measure `n - i` is a **state expression**,
> independent of the loop’s value type.

> **Implementation note (recorded 2026-10-02; updated 2026-10-04)**: the loop example in this
> section is valid on the **type side** (`check` reports 0 errors, and the back-edge obligation is
> Proved). **The runtime-value defect (#409) has been fixed**: a `while` used as a value now takes
> the value of the body block’s tail expression from the final iteration, using the same definition
> as the type-side `block_value_ty`— `loop(4) == 4` and `countdown(3) == 30` both hold at runtime
> (the `for` form has the same fix).
>
> **Known residual issue (zero iterations)**: when a loop body is never evaluated, there is no “body
> value from the final iteration,” so the loop value remains `Void`— binding it to a non-`Void` type
> position and using it at runtime reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a loud type mismatch, not a
> silently incorrect value). This is a **static approximation boundary**: the `block_value_ty`
> criterion yields the type of the body tail expression, but cannot determine whether the loop
> executes at least once (which would require data-flow/provability analysis), so the type side does
> not silently degrade (such degradation would break the value type on which the `Terminates` loop
> binding in this section depends). This residual behavior is pinned by the corpus
> `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residual behavior has been confirmed as an
> intentionally retained design boundary** (ruling of 2026-10-04, not to be fixed for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) was closed along with the main-form fix,
> and no separate tracking item was opened. For the minimal reproduction and historical impact, see
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Example Loop “Known Defect D6”. In
> addition: termination Strategy 1 (the linear rank function) is effective again in this round, so
> the order of “write an explicit measure only after automatic exploration fails” remains unchanged—
> an explicit measure is still the fallback.

**`Terminates` is a built-in predicate**, a core primitive alongside `Int` and `Never`. It is the
only predicate **for which the compiler writes the function body**—its assertions (“the measure
decreases strictly at every recursive call site/loop back edge”) are embedded in the computation
structure, and user-written predicates cannot refer to function or loop bodies, so they cannot be
expressed using the predicate-definition syntax in the “Syntax” section. The built-in surface
converges on this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two different constructs:

| Form                    | Anchor                            | Use                                                               |
| ----------------------- | --------------------------------- | ----------------------------------------------------------------- |
| `Terminates(m)`         | The name of the enclosing binding | Self-recursive functions and loops—the default form               |
| `Terminates(FnType, m)` | An explicit function type         | Scenarios such as mutual recursion where the anchor is not unique |

The two are essentially identical: a termination obligation always belongs to “the computation
annotated by the type position in which the refinement occurs.”

**The measure does not constrain the return type.** A measure may be an expression of any type (it
is not required to be a natural number); “strict decrease” is determined by the well-founded order
available on that type. Whether the measure is well-founded (for example, whether it returns an
`Int` that is `>= 0`) is an **independent obligation**, just like the decrease obligation: the
producer produces a candidate certificate and hands it to `Kernel::check` for verification. If
neither can be established, the diagnostic **does not directly reject the code**; instead, it
suggests checking the direction (whether the lower bound holds and whether recursive arguments
really move in that direction).

**Relationship to automatic exploration**: an explicit measure is not another pipeline; it is the
input used after producer exploration fails. Even after a measure is supplied, the producer still
produces a candidate certificate and hands it to the same `Kernel::check` to verify decrease and
well-foundedness; if either does not hold, an error and counterexample are reported.

For obligation generation, the decision pipeline, diagnostic directions, and shared SCC mechanisms
for mutually recursive measures, see
[RFC-027a: Explicit Measures for Termination Checking](../accepted/027a-termination-explicit-measure.md).

### 8. Certificate Producers: SMT / Tactics / Proof Search Are All Untrusted

SMT solvers, tactic scripts, and proof-search processes all have the same role in this design:
**certificate producers**. They are untrusted: the compiler does not treat their return values as
proofs, but only as hints for “finding a candidate certificate.” The only trusted component is
`Kernel::check(signature, certificate, proposition)` (§4).

#### 8.1 Trust Model

- **The kernel is the verifier**: `check` replays `certificate.steps` according to `signature.rules`
  (§4.2). It does not call solvers, parse `unsat`, or read producer logs.
- **The producer is the searcher**: given a `signature` and `proposition`, the producer makes a best
  effort to deliver a `Certificate`. What it returns or says does not constitute `Proved`.
- **`Unsat ⇒ Proved` is forbidden**: `unsat` says at most that “under the producer’s own translation
  and theory, no negative model was found.” It is neither a certificate nor a guarantee that
  `certificate.steps` exists. Mapping `unsat` directly to `Proved` delegates the trust root to the
  solver, directly conflicting with the pure-function kernel in §4. **This RFC explicitly forbids
  that mapping.**
- **`sat` is not sufficient for `Disproved` either**: `sat + model` is only a candidate
  counterexample supplied by the producer; it must become a falsification certificate that
  `Kernel::check` can decide (replayed according to the falsification rules of the `signature`, or
  computed by a kernel-direct rule showing that “the two sides differ after normalization,” as in
  §12.1) before `Disproved(model)` can be returned. Translation distortion (for example, issues with
  preserving integer literals, issue #561) can cause `sat` to carry a spurious model; blindly
  trusting it would falsely reject valid programs.
- **`unknown` / timeout / budget exhaustion ⇒ `Unproven`**: diagnostics include the unsolved
  proposition, reason, and supplementary-evidence prompt (§12.3), and must not be degraded into an
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

Producers may run in parallel, may cache results, and may switch backends; none of these freedoms
changes the single requirement: “a replayable certificate must be delivered.” SMT-LIB 2.6 remains
the translation target (an ISO standard supported natively by Z3 / CVC5 / MathSAT / Yices), with Z3
as the default backend, CVC5 as an alternative, and switching at compile time—**SMT-LIB is an input
format for producers, not a trust boundary**. There is no “general solver abstraction layer” and no
“general producer abstraction layer”: a producer has only one output contract, “deliver a
`Certificate`.”

#### 8.3 From `unsat` to a Valid Certificate: Registered Translation Rules

For `unsat` to become useful, the producer’s theory must first be folded into rules registered in
the `signature`. For example, the linear-arithmetic fragment registers a set of `linarith` rules in
the manifest of a `kind=Axiom` library in `libraries` (including their applicable sorts and premise
shapes); the SMT-LIB `unsat` result is expanded by a **proof-producing adapter** into a sequence of
`ProofStep`s for these rules and then handed to `check`. The adapter is itself an untrusted
producer: the steps it produces are still replayed one by one.

This requirement transforms “trusting Z3’s `unsat`” into “trusting that the `linarith` rules are
registered in the signature and that the certificate is inspectable step by step.” Translation bugs
are no longer papered over by the hope that “some other test will expose them”—if the adapter
produces a certificate missing a step or using the wrong rule, `check` immediately returns
`Unproven`. The `unsat` result for an unregistered theory must never enter the `Proved` path.

Before the proof-producing adapter is implemented, a **conservative degradation** is permitted:
arithmetic fragments use kernel-direct rules (structural equivalence, constant folding, and βδι
reduction) and entries from registered proof libraries; the producer’s `unsat` is attached to the
candidate certificate only as a `ProducerHint`, and does not independently trigger `Proved`. The
degraded surface is smaller than blindly trusting `unsat`, and there is no silent pass.

#### 8.4 Budget (For the Producers, Not the Kernel)

The budget constrains producer search, not the correctness of `check`—`check` is deterministic
replay, and the number of steps is determined by the certificate’s length.

| Budget dimension               | Default    | Description                                                                   |
| ------------------------------ | ---------- | ----------------------------------------------------------------------------- |
| Search steps                   | 10,000     | Upper bound on producer solving/search steps; exceeding it returns `Unproven` |
| Time                           | 100ms      | Upper bound on search time for one obligation                                 |
| Quantifier instantiation depth | 3          | Maximum nested-quantifier depth in producer translation                       |
| Certificate replay steps       | Hard limit | Maximum `steps.len()` for `check`; exceeding it ⇒ `Unproven`                  |

Exceeding the budget returns `Unproven`, producing a compile error + predicate location + a one-line
budget summary (§12.3). There is no degradation, no runtime check, and no silent pass.
Dependent-type pre-reduction (`factorial(5) → 120`, `append([1,2],[3]) → [1,2,3]`) uses
kernel-direct rules and consumes no producer budget.

#### 8.5 Relationship to the Three-Valued Result, Γ, and q=0 Evidence

Constant-equality facts in `Γ` (§12) and the four sources of evidence for `q=0` slots (RFC-011 §4.1:
literals, value environments, single-value pinning, and Γ equalities) are all sources of
`EvidenceRef` that producers may reference; they are not separate decision channels. The statement
in §3.5 that “path conditions are background knowledge” is refined here precisely to “path
conditions are the local `Γ` context of the `signature`, handed to `check` together with the
`context` field of the `Certificate`.”

#### 8.6 Current Implementation Gap: `predicate.rs` Blindly Trusts SMT

The design of this section is inconsistent with the current production implementation. The gap is
recorded honestly rather than pretending that it has been implemented:

- `src/frontend/core/sema/layers/predicate.rs` maps `SMTResult::Unsat` directly to `Some(Ok(()))` in
  `try_implication`; `try_smt_solve` maps `SMTResult::Unsat` directly to `Ok(())`. `Ok(())` is the
  `Proved` arm of `ProofResult`—this is **blindly trusting `unsat`**, with no construction or replay
  of any `Certificate`.
- `SMTResult::Sat { model }` in the same file directly constructs `ProofFailure::Disproved`; the
  counterexample is not verified by the kernel according to the signature, so translation distortion
  (issue #561) can falsely reject valid programs.
- `ProofContext` (`src/frontend/core/sema/proof/context.rs`) currently carries only
  `env / assumptions / dep_graph / budget / dep_env` and has **no `LogicSignature` field**;
  `ProofFailure` and `ProofResult` are three-valued results, but there is no `Certificate` type.
- The current pipeline therefore equates “the producer’s return value” with “the decision result,”
  whereas the target pipeline is “the producer delivers a certificate → `Kernel::check` → decision
  result.” These are not differences in wording but differences in trust model: the former treats
  Z3’s result as a trust root, while the latter treats the signature and kernel as trust roots.
- Remediation direction (no code is written in this document): ① introduce `LogicSignature` and
  `Certificate` representations and carry the current signature in `ProofContext`; ② turn
  `try_implication` / `try_smt_solve` into “produce a candidate certificate,” with `unsat` used only
  as a `ProducerHint`; ③ make `Kernel::check` the sole exit to `Proved`, and confirm `Sat`
  counterexamples through `check`; ④ register translation rules such as `linarith` under §8.3.
  Implementation proceeds under the RFC-039 D62 “mandatory evidence / complete Π/Σ” phase; this
  round defines the contract only.

#### 8.7 Differences Between wasm and Backends

When Z3 is unavailable under wasm (the SMT code is excluded by `cfg(not(target_arch = "wasm32"))`),
a producer has one fewer search path and degrades to `Unproven` (conservative, with fewer reports).
This is unrelated to “signature consistency”: under the same `signature`, native has one more
producer candidate and wasm has one fewer, while `Kernel::check` behaves identically. The existing
native/wasm difference table (§Open Questions) is now interpreted as “producer availability,” not
“decision precision.”

Programmers need not know that SMT exists. The mental model is: **the compiler passes if it can
verify a certificate under the current signature; otherwise it reports Unproven and prompts for
supplementary evidence—if an automatic producer cannot find one, write a proof function (a
first-class form of explicit evidence) and hand it to the kernel.**

### 9. Composition of Compile-Time Predicates

Compile-time predicates are functions that return Type, and composition is implemented naturally
through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
}
```

### 10. Code Examples

#### 9.1 Division Safety

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ The compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ The compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Array Access Safety

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

### 11. dispatch Dispatch Pipeline: Unified Compile-Time and Runtime Dispatch

> **Implementation note (recorded 2026-10-03, #377-2)**: the **semantics** of this section are
> established and integrated into production, but its **structure** is not a standalone
> `layers/dispatch.rs` module. Production dispatch is implemented by three **call-site obligations**
> in `checker.rs`—binding-position re-verification (`revalidate_refined`), call-site actual
> arguments (`check_call_arg_refinements`), and the return-position postcondition
> (`check_return_refinement`). Of these, **two** directly call `layers::predicate::check_predicate`:
> call-site actual arguments and the return-position postcondition share the sole decision unit
> `judge_refined_obligation` (`checker/refinement.rs`), while `revalidate_refined` uses the E2030
> convention directly. **A fourth obligation surface exists in the proof executor**
> (`proof_execution.rs`, wired on 2026-11-14, issue #537): when executing a proof call, first expand
> **the called proof function’s own formal-parameter refinements** (RFC-027 §3.3 nested obligations
> for call-site preconditions—breadth-first worklist, deduplication set + §8 budget closure; closed
> Bool constraints falsified as E4018, and Call forms are enqueued for continued execution). The
> former `layers/dispatch.rs` had zero production call sites, and its handling of
> `Unproven { ProofFunctionRequired }` was to “downgrade it to a W1080 warning and inject Γ,” which
> conflicted with the production implementation (`Unproven` always reports an error; see
> `refined_unproven`), so the module was deleted in its entirety. Note also that the “insert runtime
> check” action in the Runtime row of the table below has **no** emission point in production
> (`RuntimeOutcome::InsertCheck` has no consumer), while Γ injection is handled independently by the
> branch guards in `ownership.rs` and `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement-type primitive. The dispatch pipeline
`dispatch` automatically chooses between compile-time proof and runtime checking according to
**whether the predicate’s free variables are accessible at compile time**:

| Criterion                                                                                 | Mode            | Behavior                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants) | **CompileTime** | Enter the proof pipeline: the certificate producer delivers a candidate certificate → `Kernel::check` → Proved erased / Disproved compile error / Unproven supplementary evidence |
| A free variable comes from runtime (function parameters, external input, mut variables)   | **Runtime**     | Insert a runtime check and inject the refinement fact into the flow-sensitive assumption set Γ                                                                                    |

**Key point**: “cannot decide” ≠ “disproved”. In CompileTime mode, Unknown requires proof (no silent
degradation); in Runtime mode, the proposition has no compile-time truth value at all—no prover,
however powerful, can write an identically true proof for “the user might enter a negative number,”
so a runtime check is the only sound choice. This is not a weakness of the prover; it is a
mathematical necessity.

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

This is a hard soundness requirement—once a variable’s value changes, old assumptions are invalid.

**Branch confluence**: when IF/ELSE or match branches are merged, Γ takes the intersection of the
branches’ assumptions. Only propositions that hold on every path are carried out of the branch.

**Equality bridge (automatic evidence for q=0 slots)**: constant-equality facts in Γ are valid
evidence sources for compile-time value parameters (RFC-011 §4.1, q=0 slots). Within the guard
branch of `if y == 42 { f(y)(y) }`, Γ records `y == 42`, so y is treated as 42 in the q=0 slot, and
the call is checked along the existing `f(42)` path. One side of the equality must be a compile-time
constant; a runtime value without equality evidence still produces a call-site report when placed in
a q=0 slot. This is not a new mechanism—the verification condition `{y == 42} ⇒ {y ≡ 42}` uses the
existing implication decision procedure.

#### 12.1 Definition of Identity (defeq)

This section was added by RFC-039 resolution D62: the equality bridge above supplies an evidence
source for q=0 slots, while this section completes the definition of identity itself, the treatment
of Unproven for type equality, and the diagnostic shape.

Identity is the sole equivalence relation on types. It is given by exactly one rule: after each side
is normalized, they are **identical**.

- **Normalization** consists of three kinds of reduction: δ expands type aliases and definitions; β
  reduces type families and dependent-type applications; ι normalizes literal types—the identity of
  a literal type is determined only by its value, not by its source spelling (`03` and `3` are the
  same type).
- **Identity** is fully structural: type constructors have the same name and arguments are
  individually identical. Identity does not read cache keys or rendered text, and performs no
  reasoning about refinement predicates.
- **Subtyping is an independent relation**: `A <: B` is decided by the subtyping rules; the fact
  that both `A <: B` and `B <: A` hold does not define identity. Bidirectional subtyping would mix
  covariance and interface duck-typing rules into equivalence, contaminating the notions of
  “assignable” and “equivalent.”
- **Different `Refined` predicates ⇒ Unproven**: when the base types are identical but the
  predicates differ, the compiler returns Unproven, not Disproved. Predicate equivalence belongs to
  the assumption-set Γ layer (the equality bridge above and the Γ equality rules that follow), and
  until that layer is implemented, the compiler can only say “cannot decide.”

The failure surface of identity is consistent with the three-valued algebra in §4.1: both sides
normalize successfully and are identical ⇒ `Proved`; both sides normalize successfully but differ ⇒
`Disproved`; either side fails to normalize ⇒ `Unproven` (budget exhaustion or capability boundary).
**Reduction failure does not produce `Disproved`**—inability to compute does not mean inequality.

Identity is a pure decision: it **does not bind** type variables, accepts the result directly, and
leaves no instantiation behind. Decision points (assignment, actual arguments, return position)
first use unification; after unification fails, identity may be used, but any partial bindings
produced by the unification attempt must first be rolled back. Without rollback, an attempt judged
non-equivalent would leave bindings behind and pollute subsequent inference.

#### 12.2 Unproven ≠ False: Where Type Equality Lands

The three-valued algebra in §4.1 applies to every obligation point, including type equality. This
section applies “Unproven ≠ False” at that layer.

- When an obligation point returns `Unproven`, the compiler **must** report a compile error and must
  not let the code through. The diagnostic must prompt for supplementary evidence: in the refinement
  domain, write a proof function or reference a proof-library entry (§4.3); in the equality domain,
  provide a normalizable form; changing the signature or adding a bridging axiom follows the three
  cross-signature paths in §4.1.
- `Unproven` and `Disproved` must be distinguishable: `Disproved` includes a counterexample (a
  concrete assignment or two inequivalent sides), while `Unproven` includes the unsolved proposition
  and its reason. Rendering `Unproven` as “does not hold” is an incorrect report.
- `Unproven` **must not** be degraded into an internal compiler error (E8001). E8001 indicates only
  a consistency failure in the compiler itself; budget exhaustion and capability boundaries are
  boundaries of the compiler’s reasoning ability and belong to user-facing diagnostics (RFC-013
  “Message Quality Specification” §5).
- `Unproven` **must not** silently pass. When a decision point ends in failure, the user’s
  diagnostic must come only from an existing failure path; no second diagnostic is added, and code
  is not accepted merely because it could not be decided.

#### 12.3 Diagnostic Shape

A diagnostic for `Unproven` must carry all three of the following; a missing item makes it
incomplete:

- **Unsolved proposition**: renderable proposition text (a refinement constraint, the two sides of
  an equality, or a termination measure).
- **Reason**: a capability boundary (outside the kernel’s inference rules), budget exhaustion, or an
  unbound symbolic name—one of these three.
- **Location**: source location. A decision point that genuinely has no location uses the default
  location, and the implementation comment states the reason.

Budget details are presented as a **one-line summary** (for example, “Budget 10000 steps, 10000
steps used”). Full step counts, solver-internal traces, and failure shapes go to the verbose channel
or logs, not to default output.

This section is a **formal relaxation** of §4.1’s “Unproven → compile error + unsolved proposition +
budget-consumption report”: the substance is unchanged—users must still be able to distinguish
“cannot compute it” from “does not hold”; only the granularity of “budget-consumption report”
presentation is relaxed. The reason is that complete step counts offer users no actionable value
(users cannot modify their programs based on them), and they drift with implementation details.
After restricting this to a one-line summary, diagnostic stability and readability improve, while
distinguishability is guaranteed by the “reason” field.

The “supplementary-evidence prompt” for `Unproven` belongs to the diagnostic’s **reason** field: a
capability boundary explains that the current `signature.rules` are insufficient, budget exhaustion
supplies a one-line summary, and an unbound symbolic name identifies the name for which evidence is
needed. For the division of responsibility between the general `Unproven` case and
refinement-specific codes, see the draft code table in RFC-013 (E4023 / E2031).

### 13. Erasure Model: Certificates Are Retained at Compile Time and Erased at Runtime

Three different forms of “erasure” must be distinguished, because they apply to different objects:

| Object                                                   | Compile time                                                                                                            | Runtime                                                                                         | Basis                        |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ---------------------------- |
| `Certificate` (a proof term verified by `Kernel::check`) | Retained: `Proved(checked)` enters the compile-time cache for incremental reuse and checking Σ construction obligations | Completely erased: no runtime code is generated and no data slot is occupied                    | §4.1, §4.2                   |
| `q=0` compile-time value parameters / Σ witness fields   | Exist as quantifier facts and evidence obligations (§14)                                                                | Occupy no slot; `CreateStruct` contains only data fields; value-level `b.w` is rejected (E1114) | RFC-011 §4.1, D62 I4/I6, §14 |
| Bool checks inserted by dispatch in Runtime mode         | Neither witnesses nor certificates                                                                                      | Retained: executed at the value level                                                           | §11                          |

**Certificates are compile-time objects.** After `check` succeeds, `Proved(checked)` exists only at
compile time: it pins down “this obligation has been verified according to `signature.hash`,” and
the cache key contains `signature_hash + proposition + certificate`, so changing the signature
invalidates it (§4.1). Certificates are neither needed nor permitted at runtime—the type-level proof
has been erased, and q=0 slots and Σ witnesses likewise occupy no runtime layout (D62 I4 “Erasure
Completeness”, I6 “witness erasure ≠ runtime check”).

**Runtime checks are unrelated to certificates.** When §11 dispatches to Runtime mode, it inserts a
value-level Bool check that inspects runtime values; it is not a proof term verified by the kernel.
Retaining it does not violate “certificate erasure at runtime”; the two operate on different objects
and neither substitutes for the other: witness erasure, certificate erasure, and check retention.

**Consistency with D62’s unified term world**: propositions, certificates, `q=0` terms, and Σ
witnesses are all terms in the same compile-time system; erasure is a consequence of the single rule
that “compile-time terms do not enter the runtime layout,” rather than a separate erasure rule for
proof terms. For how Σ record fields become evidence obligations, how constructors produce
certificates, and how `b.w` is used at type level, see §14. §13 and the implementation gap in §8.6
are recorded side by side: the current `predicate.rs` does not yet produce a `Certificate`, so
“certificates are retained at compile time” has not yet been implemented.

### 14. Dependent Record Σ: Evidence Fields, Construction, and Type-Level Use

A dependent record Σ is a **record type** in the unified term world: later fields may refer to the
names of earlier fields. The records in this section are an obligation source for the §4 proof
kernel—the constructor must deliver, for every evidence field, a certificate verified by
`Kernel::check(signature, certificate, proposition)`; certificates are retained at compile time and
erased at runtime according to §13.

This section gives the complete semantics: position rules (§14.1), construction and layout (§14.2),
generics (§14.3), cross-module export (§14.4), type-level use (§14.5), and immutability and
diagnostics (§14.6). Generics, cross-module use, and type-level use are all defined here rather than
deferred.

#### 14.1 Field Position Rules: The proof-space Criterion for Evidence

Suppose the fields of record declaration `R`, in source order, are `f₀, f₁, …, fₙ₋₁`, and the type
of field `fᵢ` is `Tᵢ`. The decision has three steps: **first determine `space` (proposition family
or data family), then determine argument position (preceding / self / forward), and finally
determine the field category**. No cross-field inference is performed, and nothing is guessed from
field names:

**First step (the proof-space criterion, the sole criterion).** Normalize `Tᵢ` and take the
`SortDecl.space` of the resulting type family (§4.1):

- `space = Proof` (`Tᵢ` reduces to a proposition / refinement type / `Assert` / `Terminates` /
  proof-family result) ⇒ enter evidence classification.
- `space = Data` (`Tᵢ` is `Int`, `Array`, `Vec`, an ordinary record, or a **data type family
  application**) ⇒ a **data field**, regardless of how many preceding fields it refers to.

The `n` in `Array(Int, n)` refers to a preceding data field and changes “the length of a data slot,”
not “the generation of a proof.” Classifying it as a witness would erase the buffer from the runtime
layout—the misclassification this section is intended to eliminate. `space` is registered in
`signature.sorts` / the library manifest, and `Kernel::check` and field classification read the same
registration.

**Second step (the position criterion).** This applies only to `space = Proof` and
`Tᵢ = P(a₀,…,aₖ)`, and classifies the field by the positions indicated by its arguments:

- All arguments are **strictly preceding data fields** `fₕ (h < i)`, **compile-time constants**,
  **compile-time value parameters** (RFC-011 §4.1 / SPEC §4.3 q=0 parameters such as `Rows` and
  `Cols` in `Matrix`), or **type parameters** ⇒ the third step classifies it as evidence. After
  instantiation, a compile-time value parameter takes a concrete constant and is counted as a
  compile-time constant (exactly as in SPEC §8.3.1); an uninstantiated open term is left for the
  construction site to judge as `Unproven` (§14.3).
- An argument **is `fᵢ` itself** (a self-referential refinement such as `v: Pos(v)`) ⇒ the third
  step classifies it as a **data field (admission)**: the field carries a value, and the `Refined`
  constraint checks that value itself rather than proving a fact elsewhere.
- A **forward reference** to a later field `fₕ (h > i)`, a **reference to another evidence field**,
  or **using `fᵢ` itself as an evidence argument while mixing it with other fields** (SelfMixed:
  `P(fᵢ, fₕ)`; pure self-reference `P(fᵢ)` is the admission from the preceding case) ⇒ rejected at
  declaration time. A `P` whose kind is data family is a different category, judged at
  declaration/instantiation under §14.3.

**Third step (the category criterion).** `space=Data` ⇒ data field; `space=Proof` + preceding
arguments (including compile-time value parameters / compile-time constants / type parameters) ⇒
evidence; `space=Proof` + self-reference ⇒ data field (admission).

| #   | Form                                                                                                                                                                                                                                              | Decision                                                              | Quantity               | Runtime layout                                                                                                                    |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `space=Proof`, and all arguments are strictly preceding data fields / compile-time constants / compile-time value parameters / type parameters (positive example `w: Pos(v)`)                                                                     | **evidence / witness field**                                          | `Some(Quantity::Zero)` | Occupies no slot, does not enter `CreateStruct`, and `base` does not enter the runtime layout                                     |
| 2   | `space=Data`: ordinary value types (`n: Int`), data type family applications (positive example `data: Array(Int, n)`), and ordinary records                                                                                                       | **data field**                                                        | `None`                 | Occupies a data slot and enters `CreateStruct`                                                                                    |
| 3   | `space=Proof`, but the argument is `fᵢ` itself (self-referential refinement `v: Pos(v)`)                                                                                                                                                          | **data field (admission)**                                            | `None`                 | Occupies a data slot; the `Refined` constraint is checked at the construction point under the existing obligations in §3.1 / §3.3 |
| 4   | `space=Proof`, but there is a **forward reference** to a later field `fₕ (h > i)`, a **reference to another evidence field**, or **use of `fᵢ` itself as an evidence argument mixed with other fields** (SelfMixed; pure self-reference is row 3) | **Illegal**: rejected at declaration time, with no silent degradation | —                      | Produces no field                                                                                                                 |

The four rows do not overlap: row 2 says that ordinary value types and data families are always data
fields; row 3 says that a proposition family with **pure** self-reference is an admission data
field; row 4 says that a proposition family with an invalid position is always rejected at
declaration time. **SelfMixed falls in row 4**: pure self-reference `P(fᵢ)` is the admission in row
3, while `P(fᵢ, fₕ)`, which mixes the field itself with another field as an evidence argument, is
rejected at declaration time. The two are distinguished by whether the free-name set is exactly
`{fᵢ}`. **There is no path in which a forward reference becomes a data field**—a forward reference
neither occupies a data slot nor enters `runtime_field_index`.

**A witness may depend only on strictly preceding data fields.** The arguments of an evidence field
`fᵢ: P(fₕ,…)` must not refer to another evidence field, must not forward-reference a later field,
and must not use `fᵢ` itself as an evidence argument (pure self-reference is the admission in row 3
of the table, while self-mixing is the SelfMixed declaration-time rejection in row 4); the kind
criterion for `P` is given in §14.3. This combination of “proof-space + strictly preceding + data
fields only” ensures that, when the construction point reaches `fᵢ` in declaration order, all
arguments already have compile-time values and the proposition is closed and can be handed to
`check`.

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
    data: Array(Int, n),    // f1 data field: Array is a Data family; n only determines the length and does not generate proof
                            // Runtime index = 1; occupies a slot and enters CreateStruct
}

Admission: Type = {                      // Self-referential refinement: data field, not evidence
    v: Pos(v),              // Proof family, but the argument is itself => admission (row 3 of the table)
}

// Compile-time value parameters as evidence arguments (RFC-011 §4.1 / SPEC §4.3, §8.3.1)
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),   // Rows is a compile-time value parameter (q=0) => evidence
    data: Array(Array(T, Cols), Rows), // Array is a Data family => data field
}
```

`Rows` and `Cols` in `Matrix` are compile-time value parameters (which take concrete constants after
instantiation). `Assert(Rows > 0)` has `space=Proof`, and its argument is a compile-time value
parameter, so `_assert_rows` is an evidence field; `data` refers to the same compile-time value
parameters, but `Array` has `space=Data`, so it remains a data field. The B.2 decision procedure
therefore has an explicit criterion for “compile-time value parameter”: inspect whether it is
referenced in type position (the strength declaration in RFC-011 §4.1), rather than guessing from
the field name.

**`base` does not enter the runtime layout**: the `w` in `w: Pos(v)` is not “an Int value”; it is
“the proof of `Pos(v)`.” Its `Refined.base` (`Int`) is only type information for kernel
identity/normalization and must never cause a runtime slot to be allocated for `w`.
`StructType.fields` represents it separately as
`Vec<StructFieldInfo>{ name, ty, quantity, is_mut, has_default }`:
`quantity == Some(Quantity::Zero)` denotes a witness, while `None` denotes a data field; here,
`None` means “data field,” not “unresolved.”

**Evidence fields must not have default values (source-level rule).** An evidence field has no
runtime value to supply, and the construction view in §14.2 has no argument position for it;
therefore, the `field: Type = expression` form must not appear on an evidence field.
`StructFieldInfo.has_default` is always `false` for a witness. Supplying a default value for an
evidence field (`w: Pos(v) = expr`) is rejected at declaration time (DefaultOnEvidence); it is
neither silently treated as “a data field with a default value” nor included in constructor default
filling. Data-field default rules (optional at construction / required when no default is provided)
apply only to data fields. The corresponding provisions in SPEC §3.1.1 and §8.3.1 are based on this
rule.

Representation-layer interfaces: `ast::StructField.quantity`, `mono::StructFieldInfo.quantity`, and
`ir::TypeDeclField.quantity` have the same type; `mono::quantity_is_witness` /
`StructFieldInfo::is_witness` / `is_witness_bearing` are the sole decision procedures; and
`mono::runtime_field_index` is the **sole mapping point** from declaration order to runtime data
order (a witness hit returns `None`). Before classification, the `space` query reads
`signature.sorts` / `LibraryRef.manifest`, not a second classification table.

#### 14.2 Constructors: Data-Only at Source Level, Evidence Synthesized Automatically at Compile Time

**The source-level constructor has exactly one form: `Box(v, z)` (data arguments, in declaration
order).** Evidence fields **occupy no constructor argument positions**—there is neither a “pass the
witness as a value” form nor an “evidence argument position”; the arity of `Box` counts only the
data fields `v` / `z`, not `w`. **Evidence does not come from constructor arguments; it comes from
proof obligations generated automatically at the construction point.** This distinguishes three
views:

| View                                                                              | Form        | Arity                                                                         | Who produces the evidence                                                                                                                                                  |
| --------------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source-level / compile-time construction view                                     | `Box(3, 5)` | **compile-time arity = runtime arity = number of data fields (2 here: v, z)** | The construction point automatically generates a `FieldObligation`; the producer delivers a candidate `Certificate` → `Kernel::check`                                      |
| Runtime construction view (`translator::synthesize_constructor` / `CreateStruct`) | `Box(3, 5)` | runtime arity = 2                                                             | None: `CreateStruct.operands` contains only the local data slots `v=3, z=5` (`z` has runtime index 1, skipping `w`), and `local_count` counts data fields; there is no `w` |

The automatic synthesis path substitutes the compile-time values of strictly preceding data fields
into `P` to obtain a closed proposition; the producer (§8: kernel-direct rules / `Γ` evidence /
proof library / SMT search) delivers a candidate `Certificate`, and
`Kernel::check(signature, certificate, proposition)` verifies it. `Proved(checked)` is retained at
compile time and erased at runtime according to §13.

**Default values serve data fields only.** Under the source-level rule in §14.1, evidence fields
must not have default values: the `optional` flags generated by `bind_args_to_params` come only from
the `has_default` flags of **data fields**; witness fields enter neither `param_names` nor
`optional`; default filling in `translator::synthesize_constructor` and default-field slicing in
`ir_gen` follow the non-witness field order, so no default expression or zero register is generated
for an evidence field. When `Box(3, 5)` omits a data field, the data-field default/zero value is
filled in; omitting an evidence field is not a concept, because it has no argument position to omit.

**Explicit evidence is not a constructor argument; it is an independent declaration.** When the
automatic producer cannot deliver a verifiable certificate (`Unproven`), the programmer supplies
explicit evidence under §4.3—write an **independent proof function** (whose return type equals the
proposition to be proved) or register a named proof in a `proof library`, referenced as
`EvidenceRef::ExplicitProof` / `LibraryProof` by the `FieldObligation`; do not rewrite the
construction expression, introduce a `Box(..., proof)` form, or give an evidence field a default
value (§14.1 rejects DefaultOnEvidence at declaration time). **Passing one additional evidence
argument at the source level** (`Box(3, 5, evidence)`) is **not permitted**: under D60’s six binding
rules, the extra argument cannot be assigned to any data parameter slot, so an argument error in the
E1010 / E1014 family is reported rather than silently discarded.

A `FieldObligation` must represent both “no certificate has been produced yet” and “a verified
certificate exists,” so the original `evidence: EvidenceRef` (which could refer only to explicit
evidence and could not hold a producer-synthesized `Certificate`) is split into two slots:

```text
FieldObligation {
    record         : TypeId,
    field          : FieldId,
    proposition    : Proposition,
    evidence_ref   : Option<EvidenceRef>,   // Explicit evidence supplied by the programmer; None during automatic synthesis
    certificate_ref: Option<CertificateRef>,// A producer-synthesized certificate verified by check; None while unverified
    signature_hash : ContentHash,
}
CertificateRef { id: CertId, signature_hash: ContentHash }  // Points to Proved(checked) in the compile-time cache from §13
```

The legal combinations of the two slots are: `(Some(ev), None)` = explicit evidence awaiting replay;
`(None, Some(cr))` = automatically synthesized and verified; `(None, None)` = awaiting producer
search; `(Some, Some)` is illegal (one obligation must not consume two proofs). After `check`
succeeds, `certificate_ref` is written, while `evidence_ref` is retained as a provenance annotation.

`FieldObligation` is generated in the refined walk in `checker/refinement.rs`; field binding reuses
`bind_args_to_params` (D60’s six rules), and decisions reuse the sole unit
`judge_refined_obligation`. **No value is fabricated when proof fails**: under §4.3, explicit
evidence is supplied or `Unproven` is returned (E2031 / general E4023); there is no such thing as an
incorrectly written “witness value.”

A self-referential refinement field (`v: Pos(v)`) is a data field: its `Refined` constraint is
decided at the construction point under the existing call-site obligations (§3.1 / §3.3). It is not
an evidence field, does not generate a `FieldObligation`, and does not change `runtime_field_index`.

#### 14.3 Generic Records: Proof Family Parameter `P` (kind `T -> Proposition`)

A type is a “parameterized record + **proof family parameter**,” not “a generic witness to be
handled later.” The kind of `P` has the same source as the proof-space criterion in §14.1: **the
result sort must be `space=Proof`**, meaning that `P` is a proof/type family (proposition family),
not a data family. `T -> Proposition` is kind notation (`Proposition` is an alias for the
`SortSpace::Proof` sort in §4.1), **not a source-level type name**: the `yaoxiang` block below uses
the `P: (T: Type) -> Type` form accepted by the current parser, with kind constraints checked at
declaration and instantiation rather than embedded in parseable code.

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
  result of `P` has `space=Proof`. `Array: (T: Type, N: Int) -> Type` has `space=Data`, so passing
  it to `Cell` is a **kind mismatch and is rejected at declaration time** (the parameter
  declaration’s kind is `T -> Proposition`, so binding a data family is a type error, not “`w`
  degrades into a data field”; row 2 of §14.1 applies only to fields that **directly name** a data
  type family). The multi-argument form `(T, U) -> Proposition` is analogous, with all arguments
  falling on strictly preceding data fields in declaration order.
- `P` and `T: Type` are both compile-time parameter positions (q=0, RFC-011 §4.1). When
  `Cell(Int, Pos)` is instantiated, `P` is bound to the concrete predicate `Pos` (`space=Proof`),
  and the proposition for `w` becomes `Pos(v)`; the position rules are exactly those in §14.1.
  `Cell(Int, Array)` is rejected at declaration/instantiation because of the kind mismatch.
- `P` is not a value parameter and occupies no runtime slot. When `P` is unbound (an uninstantiated
  generic), the result sort of `w` remains `Proof` (the kind has been declared), but the proposition
  is open, so the **construction point returns `Unproven`** (not a declaration-time rejection);
  there is no silent pass and no degradation into a data field. After instantiation, classification
  is rerun according to §14.1.
- Evidence classification for a generic record is rerun according to §14.1 **after instantiation**:
  as `substitute` substitutes `P`, the `quantity` in `StructType` remains `Some(Zero)`; the Struct
  arms of `mono::substitute` and `types::solver` propagate the field `quantity` rather than creating
  a second classification.
- Export of the generic `Cell` carries the kind of `P` (`T -> Proposition` notation) and the
  field-obligation template (§14.4) in `TypeDefPayload`; the correspondence between the source-level
  `(T) -> Type` and the kind notation is established by the declaration/instantiation criteria.

#### 14.4 Cross-Module Export: `TypeDefPayload` Carries Proposition/Evidence Slots

When a record containing evidence fields is used across modules, its propositions and evidence slots
must be exported with the type definition; the importer must not “re-guess them from field names.”
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
        evidence_ref   : Option<EvidenceRef>,            // Explicit evidence source; None during automatic synthesis
        certificate_ref: Option<CertificateRef>,         // Reference to a verified certificate; None while unverified
        signature_hash : ContentHash,                    // Invalidated according to the signature in §4.1
    }]
    libraries        : [LibraryRef]                      // Libraries referenced by propositions/evidence; exported across modules
}
```

- Export surface: when `checker.rs` constructs `TypeDefPayload`, it fills one `EvidenceSlot` for
  every field with `quantity == Some(Zero)`; `field_obligations` and `evidence_slots` have the same
  source and are not written separately.
- Import surface: after receiving an `EvidenceSlot`, the importer verifies it using **its own
  current `LogicSignature`**—if the `signature_hash` differs, the three cross-signature paths in
  §4.1 are used (translation term / bridging axiom / reproofing), and the exporter’s `Proved` result
  must not be reused directly. Every library referenced in `libraries` must appear in the importer’s
  `libraries` list; otherwise the result is `Unproven`, with a prompt to register the library or
  write a bridge.
- Construction points in multiple files/modules reuse the same `judge_refined_obligation`;
  cross-module use does not introduce a second decision entry point (RFC-039 D55 silent-channel
  prohibition).

#### 14.5 Type-Level `b.w`: `ConstExpr::FieldPath` + Evidence Term

A witness field may **only be used at type level** (`b.w` is user-writable type-level syntax, as
described at the end of this section); value-level reads are always rejected:

- **Term representation**: `ConstExpr::FieldPath { base: Box<ConstExpr>, field: FieldId }` is a
  field-path term in the unified term world. `b.w` in type position resolves to
  `FieldPath(NamedVar("b"), "w")`; it does not denote a runtime value, but the **evidence term**
  (proof term) corresponding to `EvidenceSlot.evidence_ref`.
- **Evidence term**: `EvidenceTerm(FieldPath, EvidenceRef, signature_hash)` is one source of
  `EvidenceRef` for a `Certificate` (§4.2), corresponding to the explicit evidence or verified
  certificate in the two slots `FieldObligation.evidence_ref` / `certificate_ref` (§14.2). When
  `b.w` is used in another proposition (such as `Q(b.w)`), the producer uses `EvidenceTerm` as a
  premise of the candidate certificate, and `Kernel::check` replays it according to the signature;
  if the `signature_hash` differs, it is not reused.
- **Type-level uses**: `FieldPath` may appear in predicate arguments, type-constructor argument
  positions, as an evidence-term premise for another proposition, and in
  `TypeDefPayload.evidence_slots[i].proposition` (indexed per slot). Note: when `FieldPath` points
  to a witness, it is a **proof-term premise**, not a “data-field argument”—the rule in §14.1 that
  “evidence-field arguments accept only strictly preceding data fields” constrains the data
  arguments of `P`; it does not prohibit using the proof term of a preceding evidence field as a
  premise for a new proposition (the latter is replayed by `check` through `EvidenceTerm`). `b.w`
  produces no runtime value and occupies no data slot.
- **Value-level rejection**: when `b.w` appears in a value expression (the `Struct` arm of
  `FieldAccess` and the `TypeRef→Struct` arm), report **E1114** (`witness_field_not_readable`).
  `FieldPath` is not a value term, and neither the `Evaluator` nor IR generation materializes a
  value for it.
- The exhaustive `match` over `ConstExpr` (`const_eval::substitute`, `free_names`, `Display`, the
  `translate::` family, `proof_execution`, `fold_proof_call_arg`, etc.) must handle `FieldPath`; an
  evaluator that does not support the term language returns `Unproven` upon encountering
  `FieldPath`, and must not silently treat it as `NamedVar`.

```text
// Design sketch (non-compilable fragment: Proved / FieldPath / evidence term are kernel terms, defined by this section’s interfaces)
// Type-level use: treat b.w as an evidence premise, not as a value read
useWitness: (b: Box) -> Type = { Proved(b.w) }   // b.w resolves to FieldPath + evidence term
// Value-level b.w => E1114
```

> **Supersedes 4A’s “no user syntax at type level.”** The premise of the 4A ruling (09a §P6 6.9.5 H0
> ruling, 2026-11-13) was that “`b.w` would require `ConstExpr::FieldPath` and a proof token,
> constituting an independent term-language design and creating a second term world,” so ruling V1
> did not provide user-facing `b.w` projection. `ConstExpr::FieldPath`, `EvidenceTerm`, and
> `TypeDefPayload.evidence_slots` all belong to **the same unified term world** (§4, not a second
> term world), so that premise does not hold: **type-level `b.w` is user-facing syntax** (the
> `useWitness` example above). The provision in 4A that “there is no user-facing `b.w` at type
> level” is superseded by this section and is no longer effective. The other two provisions of
> 4A—witnesses are compile-time fields, proved at construction, and erased at runtime; and field
> representation uses `quantity` + proposition type—remain in effect. Value-level `b.w` is still
> rejected under 4A (E1114). `FieldPath` exists only in type/proposition positions, is not a runtime
> field path, and does not constitute an “A-style runtime field.”

#### 14.6 Immutability, Erasure, and Diagnostic Codes

- **5A: Immutability as a Whole**: a record containing any evidence field is immutable as a
  whole—data fields must not be declared `mut`, must not be assigned, must not be borrowed through
  `&mut`, and must not be write-method receivers (including automatic `WriteBorrow`). Otherwise, the
  witness proposition could become invalid due to field mutation, and there is currently no runtime
  check/revalidation; violations report **E1115** (`witness_record_immutable`). Finer-grained
  linearity/uniqueness designs are deferred.
- **Erasure**: an evidence field’s `quantity=Zero` ⇒ it occupies no runtime slot; `CreateStruct` /
  `GetField` / `SetField` / `resolve_field_index` recognize only the data order of
  `runtime_field_index`; a `Proved(checked)` certificate is retained at compile time and erased at
  runtime according to §13; Bool checks in Runtime mode are unrelated to witnesses.
- **Division of responsibility among diagnostic codes** (each code covers one aspect; no code
  borrowing and no silent behavior):

| Code      | Trigger                                                                                                                          | Meaning                                                                         |
| --------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| **E4018** | An evidence proposition at a construction point is disproved (a counterexample confirmed by `check` / a kernel-direct decision)  | Refinement-predicate violation, with counterexample                             |
| **E2031** | An evidence proposition at a construction point is `Unproven` (refinement channel, with the `assigned`/`var`/`constraint` slots) | The refinement constraint cannot be proved; prompt for explicit evidence (§4.3) |
| **E1114** | Value-level read of a witness field `b.w`                                                                                        | A witness field cannot be read at value level                                   |
| **E1115** | `mut` / field assignment / `&mut` / write receiver on a witness-bearing record                                                   | The record is immutable as a whole                                              |
| **E4023** | General `Unproven` outside the refinement channel (for example, the `FieldPath` evaluator does not support a term)               | General inability to prove, with proposition/reason/location (§12.3)            |

`ProofFunctionRequired` still uses E4020; the division of responsibility between general `Unproven`
and E2031 remains unchanged (§12.3, RFC-013). The **declaration-time rejection list** aligns item by
item with row 4 of §14.1 and the kind criterion in §14.3 and contains four categories:

| #   | Rejected form                                                                                            | Description                                                                                                     |
| --- | -------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| 1   | `space=Proof` with a **forward reference** to a later field `fₕ (h > i)`                                 | Invalid position                                                                                                |
| 2   | **Referring to another evidence field** as an argument                                                   | Invalid position                                                                                                |
| 3   | **Using `fᵢ` itself as an evidence argument while mixing it with other fields** (SelfMixed: `P(fᵢ, fₕ)`) | Invalid position; pure self-reference `P(fᵢ)` is the admission data field in row 3 of §14.1 and is not rejected |
| 4   | The **kind of `P` is data family** (§14.3)                                                               | Kind mismatch; a type error at declaration/instantiation                                                        |

**In addition, supplying a default value for an evidence field** (`w: Pos(v) = expr`,
DefaultOnEvidence, §14.1 source-level rule / §14.2 constructor rule) is also rejected at declaration
time—it has no runtime value to supply and is not silently treated as a data field. Rows 1–3 above
are the invalid-position forms in row 4 of §14.1 (item 3 is SelfMixed), and item 4 is the kind
mismatch in §14.3; together, they constitute the four declaration-time rejection categories. Default
values are an independent field-declaration check and are not counted with those four. The above
items are rejected directly at declaration and do not proceed to construction or silently degrade
into data fields. Data type family applications with `space=Data` (including `Array(Int, n)`) and
pure self-referential refinements with `space=Proof` (`v: Pos(v)`, admission) are valid data fields.
**An uninstantiated generic is not a declaration-time rejection**: under §14.3, when `P` is unbound,
the result sort of `w` remains `Proof` and the proposition is open, so the **construction point**
returns `Unproven` (E2031 / E4023), with no silent pass and no degradation into a data field.
Assignment of declaration-time diagnostic codes (new declaration-time codes or reuse of
E1001/E1114/E1092) is to be decided in implementation round D2; this section defines only the
rejection list and semantics and does not reserve code numbers.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                                      | After (this RFC)                                                                                                                                                            |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`                   | Compile-time predicate as a parameter type `(b: Positive(b))`                                                                                                               |
| `//! ensures: ExistsMax(result, arr)`                 | Return type uses a return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                                       |
| `/*! invariant: ... !*/`                              | Compile-time predicate type annotation on a variable—a Floyd–Hoare invariant                                                                                                |
| `//! decreases: n`                                    | The measure is written in the refinement-type position (`Terminates`); the compiler explores automatically first and requires an explicit measure only if exploration fails |
| Specifications are comments                           | Specifications are part of the type system                                                                                                                                  |
| No evidence recorded outside `/*! invariant: ... !*/` | Dependent record Σ: evidence fields (q=0) are distinguished from data family fields using the proof-space criterion; see §14                                                |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}` is the proof space and is exactly
consistent with the existing type-definition syntax. A compile-time predicate is simply a function
returning Type—`name: (params) -> Type = { assertions }`. At a use site, it is simply a function
call—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# A compile-time predicate = a function returning Type, with compiler-verified assertions inside {}
# Uses existing function/type syntax; no new BNF rules are needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**The arguments of a predicate application must have compile-time-constant forms**—literals,
variables (bound by name), type applications (recursively extracted), or **compile-time-denotable
function references** (function names). An argument shape that cannot be converted to a constant
expression reports **E1092**, and a mismatch between the number of arguments and the predicate’s
declared formal parameters reports **E1093**. Arguments are bound positionally to the
formal-parameter list, and the predicate’s arity is determined by its declaration—`Positive(x)` is
unary, `IsMax(T, arr, result)` is ternary, and `Terminates(m)` and `Terminates(FnType, m)` are unary
and binary. Refinement constraints are **never silently discarded** (previously, an argument that
could not be converted caused the constraint to disappear silently, allowing a binding that violated
the constraint to pass).

**New syntactic concept: return-value formal parameter**—in `-> (name: Type)`, `name` is a
return-value formal parameter.

A return-value formal parameter is one of the syntactic concepts YaoXiang introduces over its
existing function syntax (record evidence fields reuse existing field syntax; see §14). Its
semantics are:

- The value of `name` is supplied by a `return` statement.
- `name` exists only in the type signature and is referenced by postcondition predicates (such as
  `-> (result: IsMax(T, arr, result))`).
- `name` does not enter the function-body scope and does not appear at the call site.
- Return-value formal parameters are **optional**—when there is no postcondition, the signature is
  identical to that of an ordinary function (`-> Int`), with no additional burden.

The reason for introducing it is that a postcondition needs to refer to “the value the function is
about to return.” Without return-value formal parameters, the compiler would need a special rule
(such as an implicit variable `$result` or `__retval__`) to let a predicate refer to the return
value. A return-value formal parameter makes this reference explicit—it is merely a formal parameter
whose value is supplied by `return` rather than by the caller.

**A proof function** is not a new concept—it is simply a YaoXiang function whose return type is the
proposition being asserted. When the compiler returns Unproven, the programmer supplies a proof
function, and the type checker verifies it in exactly the same way it verifies any function’s return
type. No new syntax, keywords, or rules are needed.

> **The boundary between the two.** In the correctness domain (a predicate is Unproven), proof is
> supplied in the **body**: write a function whose return type is the proposition to be proved. In
> the termination domain (§6.9), the measure is supplied in **type position**: write
> `Terminates(measure)` as the type of a binding or function signature. The former means “write a
> proof for a proposition that cannot be proved,” while the latter means “explicitly declare a
> measure that cannot be found by exploration”—the mechanism is the same (both are refinement-type
> applications), but the location differs. The termination domain does **not** require writing a
> `_proof` function.

### Type-System Impact

- **Type universe**: compile-time predicates reside at the Type₂ layer—functions that accept values
  and return Type, at the same level as type constructors.
- **Generic interaction**: compile-time predicates may take generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`.
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules and may
  only be read, not written.
- **Type inference**: compile-time predicate parameters participate in HM type inference.
- **Dependent record Σ**: record fields are divided into data fields and evidence fields
  (`Quantity::Zero`) according to §14’s proof-space + position rules; the proof family parameter
  `P: T -> Proposition` and `TypeDefPayload.evidence_slots` provide the complete semantics for
  generics and cross-module use.

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables are known at compile time): after `Kernel::check`
  verifies the certificate, `Proved(checked)` is retained at compile time and completely erased at
  runtime. For `Positive: (x: Int) -> Type = { x > 0 }`, the runtime representation of a parameter
  such as `b: Positive(5)` is simply `Int`; the refinement condition `{ 5 > 0 }` has passed, and the
  certificate does not enter the runtime layout. Σ evidence fields follow the same rule under §14:
  `base` does not enter the layout, `CreateStruct` is data-only, and a value-level `b.w` reports
  E1114.
- **Runtime mode** (runtime free variables exist): retain the runtime check—execute a Bool check at
  the value level and inject into the flow-sensitive assumption set Γ. See the dispatch pipeline in
  §11 and the erasure model in §13.

Placing a compile-time predicate in type position (such as `f(x: Positive(x))`) produces no wrapper
type and allocates no extra memory. However, when `x` comes from runtime input, a runtime Bool check
**is** inserted.

**Interaction constraint with `ref`**: compile-time predicates may refer only to immutable borrows
or values whose ownership has been transferred. A compile-time predicate that refers to a mutable
borrow cannot guarantee at compile time that the verification result remains valid at runtime—such
use is a direct compile error.

### Compiler Changes

1. **Parser**: compile-time predicates and proof functions use standard function syntax and require
   no additional parsing rules.
2. **Proof kernel**: `Kernel::check(signature, certificate, proposition)` and `CheckVerdict`;
   replaying `Certificate.steps` according to `signature.rules` is the sole exit to `Proved`.
3. **Logic signatures and libraries**: `LogicSignature` (sorts / rules / axioms /
   `libraries: [LibraryRef]` / hash), with each `LibraryRef` carrying `kind/id/manifest/hash`; the
   signature hash covers every library. Signatures are replaceable, and cross-signature reasoning
   uses explicit translation or a bridging axiom.
4. **Certificate producers** (untrusted): kernel-direct rules, Γ/`q=0` evidence, proof functions,
   SMT-LIB 2.6 translation (Z3 by default, CVC5 as an alternative), tactics, and proof search; all
   deliver only candidate `Certificate`s, and `unsat` does not directly reach `Proved` (§8).
5. **Verification-condition generation**: WP/SP calculation, loop invariants, and `FieldObligation`
   for record construction under §14 (field position rules / `bind_args_to_params` /
   `judge_refined_obligation`) produce propositions and candidate certificates without deciding them
   independently.
6. **Error reporting**: counterexamples confirmed by `check` + unsolved proposition + reason +
   supplementary-evidence prompt + associated source location (§12.3).

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged.
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; Runtime mode retains
  only the necessary Bool checks.
- ⚠️ RFC-022’s `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden.
- ⚠️ Record types gain §14’s evidence-field position rules: programs in which “a field name happened
  to be a predicate application and was silently treated as a data field” now receive compile-time
  witness classification (value-level reads report E1114; witness-bearing records are immutable
  under E1115). This is an externally visible behavior change and is registered as a D/E-class
  change under RFC-039 D59.

## Trade-offs

### Advantages

- **Full realization of the Curry–Howard correspondence**: types are propositions, programs are
  proofs, and `name: Proposition = Proof`; propositions and certificates inhabit D62’s unified term
  world.
- **Unification**: compile-time predicates and ordinary functions use exactly the same syntax, with
  no conceptual split.
- **Producer transparency**: programmers need not know that SMT exists; the mental model is “supply
  explicit evidence → the kernel verifies it according to the signature”; backends are replaceable.
- **Gradual adoption**: adoption can begin with one compile-time predicate and expand incrementally.
- **Minimal runtime overhead**: certificates and `q=0` terms are retained at compile time and erased
  at runtime; Runtime mode retains only the necessary Bool checks.

### Disadvantages

- **Compile time**: producer search and kernel replay both increase compile time, but producer
  budgets (§8.4) and a certificate-step limit keep this under control.
- **Limits of automatic proof**: when a producer cannot deliver a verifiable certificate, the kernel
  returns `Unproven` and prompts for evidence. This is not a language defect—it is an inevitable
  consequence of the halting problem. The kernel honestly distinguishes “cannot compute it” from
  “does not hold.”
- **Learning curve**: writing effective compile-time predicates and proof functions, and choosing a
  logic signature, requires understanding the Curry–Howard correspondence and the basic intuition of
  “signature relativity.”
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design.

### Risk Mitigation

- Hard producer budget limits (10,000 search steps / 100ms / instantiation depth 3) and a
  certificate-replay step limit; exceeding the budget returns `Unproven` (§8.4).
- Dependent-type pre-reduction: deterministic value computation is consumed first by kernel-direct
  rules, leaving only nondeterministic work for producers.
- `Unproven` is not a dead end: supply explicit evidence (a proof function, a proof-library entry,
  or a changed signature / bridging axiom, §4.3); in the termination domain, supply a measure in
  type position (§6.9)—all are verified by `Kernel::check`.
- Incremental verification: `Proved(checked)` is cached by
  `signature_hash + proposition + certificate`; only modules whose signatures have changed or whose
  certificates have been invalidated are recomputed.
- Clear error messages + counterexamples confirmed by `check` + unsolved proposition + reason +
  supplementary-evidence prompt + a one-line budget summary (§12.3)

## Alternatives

| Alternative                                                        | Why it was not chosen                                                                                                                                                                                                                                   |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: comment-based specifications using `//!`                  | Separates specifications from types, violating the Curry–Howard correspondence                                                                                                                                                                          |
| Separate specification files (such as CVL)                         | Separates specifications from code and increases maintenance costs                                                                                                                                                                                      |
| Runtime assertions only                                            | Cannot statically guarantee correctness                                                                                                                                                                                                                 |
| External proof assistants (such as Coq)                            | Disconnect from the compiler and require a separate proof language and trust boundary. YaoXiang’s choice is that proofs are YaoXiang code and certificates are verified by `Kernel::check` under a replaceable signature, with no hard-coded trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                                                                                                      |

## Implementation Strategy

### Phase Breakdown

| Phase       | Contents                                                                                                                                                                                           |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | `Kernel::check` + `LogicSignature` + `Certificate` + `CheckVerdict`; kernel-direct rules (structural equivalence, βδι reduction, defeq)                                                            |
| **Phase 2** | Manifests and hashes for axiom libraries / proof libraries; integration of proof functions and explicit evidence; a proof-producing adapter expands `unsat` into `ProofStep`s for registered rules |
| **Phase 3** | Loop-invariant VC generation + termination checking (§6, §6.9) + dependent record Σ field position rules, construction obligations, and type-level `FieldPath` use (§14) all use the same `check`  |
| **Phase 4** | Certificate caching by `signature_hash` + incremental verification + `TypeDefPayload` cross-module evidence slots + cross-signature translation/bridging axioms + IDE support (§14.4)              |

### Dependencies

- RFC-010: Unified Type Syntax—compile-time predicates are based on `name: type = value`.
- RFC-011: Generic Type System—compile-time predicates may take generic parameters.
- RFC-009: Ownership Model—compile-time predicate expressions obey ownership rules.

## Open Questions

- [x] **Availability of Z3 for the wasm32 target**: Z3 is unavailable under wasm (all SMT code is
      excluded by `cfg(not(target_arch = "wasm32"))`). Scope and degradation direction: -
      `ownership.rs` `smt_cut` always returns `false` → back-edge traversal → **conservative
      rejection** (sound direction, merely less precise). - Termination-check Strategy 1 and
      predicate levels 2b/3 are skipped entirely.

  **Correction (2026-10-01)**: the original text said these paths were “currently **unreachable**”
  (on the grounds that `predicate_defs` had never been populated in production, `parser` did not
  produce `MonoType::Refined`, and `with_solver` was never invoked in production), which is no
  longer true—all three have been implemented (predicate-definition registration #377-3;
  refinement-type normalization; production-pipeline solver injection). Therefore, wasm/native
  differences are no longer limited to `smt_cut` precision:

  | Path                                               | native   | wasm       | Degradation direction                 |
  | -------------------------------------------------- | -------- | ---------- | ------------------------------------- |
  | Explicit-measure decision for termination checking | Decided  | `Unjudged` | Does not report E4022 (fewer reports) |
  | Well-foundedness decision                          | Decided  | `Unjudged` | Does not report E4022 (fewer reports) |
  | SMT implication level for refinement predicates    | Executed | Skipped    | Falls back to `Unproven`              |

  All are **conservative** directions (the producer has one fewer search path ⇒ fewer candidate
  certificates ⇒ `Unproven`; `Kernel::check` itself is unchanged), and soundness is unaffected.
  Under the terminology of §8.7, the table below is interpreted in terms of “producer availability,”
  not “decision precision.” To achieve equal precision, the options and costs are described in issue
  #376 (conclusion: prefer a JS-side Z3 instance rather than linking Z3 into the main wasm build—
  the latter would require replacing the emcc build system and would increase the artifact from 3 MB
  to approximately 20 MB).

- [x] **SMT solver choice**: Z3 by default (MIT license and the broadest verification support). CVC5
      is an SMT-LIB-compatible alternative selected through a compiler flag. SMT-LIB 2.6 is the
      producer’s translation target—it is an input format for candidate certificates, not a trust
      boundary; the sole exit to `Proved` remains `Kernel::check` (§8). No custom general solver
      interface is implemented, and `unsat` is not treated as a decision result.
- [x] **Specific solver-budget values**: producer search steps 10,000 / time 100ms / quantifier
      instantiation depth 3, plus a certificate-replay step limit (§8.4). These are fixed internally
      in the compiler, with no knob. They are adjusted only if actual use cases demonstrate that
      they are insufficient (rather than “the user wrote it incorrectly”).
- [x] **Scope of quantifier support**: the language does not restrict quantifier order. Compile-time
      predicates accept Type parameters—Type includes function types—so higher-order quantifiers are
      a natural consequence of the type system and require no special syntax. Producers can handle
      first-order quantifiers (forall/exists, including interleaved nesting, limited by budget
      depth 3) and deliver candidate certificates; when they cannot deliver a verifiable certificate
      for a higher-order quantifier, the kernel returns `Unproven` and prompts for explicit
      evidence—write a YaoXiang proof function whose return type equals the proposition (§4.3),
      verified by `Kernel::check` under the current signature. No external export, AI, or
      interactive proof mode is needed. Everything is YaoXiang code, and everything is verified by
      the proof kernel.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefixes to avoid collisions). The `sat + model` returned by a producer is a
      candidate counterexample and must be confirmed by `Kernel::check` before the proposition is
      rendered as false. Output format: variable name = concrete value + source location +
      predicate-definition location. No complex mapping layer is implemented.
- [x] ~~**Interaction between compile-time predicates and the `ref` smart pointer?**~~ → Decision:
      compile-time predicates permit only immutable borrows or values whose ownership has been
      transferred. A value under a mutable borrow cannot appear in a compile-time predicate.
- [x] **Extending the `forall` predicate-violation-count measure to non-adjacent operations?** → Do
      not extend it. The current coverage (adjacent swaps, adjacent moves) is complemented by
      Strategy 1 (the linear rank function)—the outer interval contraction in quicksort is covered
      by Strategy 1, while heap sort is covered by Strategy 1 (the array-index pattern). A loop that
      cannot be proved terminating by any strategy causes a direct compiler error—this is the strict
      safety philosophy, not a defect. If a real-world (rather than academic) algorithm arises that
      none of the four strategies can cover, the question will be reconsidered. → **Reconsideration
      triggered (2026-09-14, #318)**: non-structural recursion (gcd-like non-direct decrease, mutual
      recursion, merge partitioning) is exactly the kind of real-world scenario anticipated by this
      provision—the template sequence for measure exploration cannot handle it, and it cannot be
      rewritten as an analyzable iteration pattern without severely harming readability. Conclusion:
      the strict safety philosophy is unchanged (an incorrect measure is still rejected by a
      counterexample confirmed by `check`; a human can fail to prove something but cannot prove
      something false), but “failure of exploration implies rejection” is relaxed to “failure of
      exploration may be handled by the programmer explicitly supplying a measure in type
      position”—see §6.9.
- [x] **Combinatorial explosion in linear-rank-function enumeration**: candidate enumeration is
      capped at three bounded variables. For ≤3, all linear combinations are enumerated and
      candidate certificates are delivered to `Kernel::check` one by one. For >3, only
      single-variable measures are attempted (`v_i`, `u_i - v_i`); failure produces a direct compile
      error prompting the programmer: “the loop has more than three bounded variables, and the
      automatic producer cannot synthesize a multivariable measure; please supply explicit
      evidence.” This is not an engineering compromise—it forces programmers to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](011-generic-type-system.md)
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
