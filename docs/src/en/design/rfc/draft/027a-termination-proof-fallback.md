---
title: 'RFC-027a: Proof Function Fallback for Termination Checking'
status: 'Draft'
author: 'Chenxu'
created: '2026-09-14'
updated: '2026-09-14'
issue: '#318'
---

# RFC-027a: Proof Function Fallback for Termination Checking

## Summary

Deliver on the re-discussion clause reserved in the RFC-027 decision record: for **non-structural
recursion** (gcd-like non-directly-decreasing recursion, mutual recursion) that the four automatic
strategies cannot cover, provide a **proof function fallback**—both the measure and the proof are
ordinary YaoXiang functions, with zero new syntax and verified by the type checker. The mechanism is
positioned as **an application of the refinement type proof protocol**: zero new mechanism on the
proof side (the existing RFC-027 Unproven fallback channel is reused as-is), with only call-site
obligation generation on the compiler side and a single proposition anchor newly added. The
fully-automatic-priority principle is unchanged: automatic strategies remain the first path; user
intervention only occurs after Unproven.

## Motivation

### Hosting Qualification and Scope of Amendment

RFC-027 did not reserve a sub-RFC slot (unlike RFC-029's pattern of reserving 029a). The hosting
qualification for this sub-RFC comes from the re-discussion clause in the RFC-027 decision record
itself:

> Loops whose termination cannot be proved by any strategy cause the compiler to error directly—this
> is hard-safety philosophy, not a defect. **If, in the future, real-world scenarios (not academic
> constructions) arise where all four strategies fail to cover an algorithm, we will re-discuss.** —
> RFC-027 decision record

This RFC is the formal vehicle for that "re-discussion." It does **not overturn** the recorded
decision—the clause itself is part of the decision, and triggering it is the decision working as
designed; it is the contemplated exercise of the clause.

The scope of amendment to RFC-027 is precisely limited to three points:

| Amendment Point                             | Original Record                                              | After Amendment                                                                                                                                                                                                                            |
| ------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| §7 "No half-automatic annotation loophole"  | Termination checking is fully closed                         | Narrowed to "no **annotation syntax** loophole"—the proof function fallback is not an annotation (no new syntax position, not attached to the function definition), in the same worldview as "no new keywords for compile-time predicates" |
| Decision record "Unproven → error directly" | Termination is the only proof domain without a fallback      | Changed to "Unproven → may provide proof function"—aligned in behavior with the correctness domain (predicate Unproven → proof function, RFC-027 §Proof Function)                                                                          |
| §6.8 error message                          | "Please use an iterative pattern analyzable by the compiler" | Appended "or provide a measure function and a termination proof function"                                                                                                                                                                  |

**Not amended** are: the priority status of the four automatic strategies, the
fully-automatic-priority principle, the rest of the hard-safety philosophy (Disproved remains a
compile error + counterexample; correctness domain behavior is unchanged).

### Current Problem (tested 2026-09-14, v0.7.14+ dev)

1. **TerminationChecker can only prove structural recursion.** `termination.rs` is currently in
   Phase 1: strategy 3 (bounded increase/decrease), direct decrease of recursive parameters
   (`factorial(n-1)`), and `for` loops that terminate by nature. Strategies 1/2/4 are not yet
   implemented (per RFC-027's planned extension). Non-structural recursion fails to compile
   directly:

   ```yaoxiang
   gcd: (a: Int, b: Int) -> Int = {
       if b == 0 { return a }
       return gcd(b, a % b)
   }
   ```

   The recursive arguments `(b, a % b)` are not a direct decrease of the parameters—the current code
   errors and demands rewriting into an iterative pattern. gcd cannot be rewritten into an
   "analyzable iterative pattern" without sacrificing readability; merge sort partition, mutual
   recursion (even/odd check), tree traversal are the same. These are real algorithms, not academic
   constructions—**the triggering condition for the re-discussion clause is met**.

2. **Asymmetric fallback across proof domains.** Correctness propositions in RFC-027 (predicates
   like `Positive(b)`, `IsMax`) have a proof function fallback when Unproven; termination is the
   only proof domain where Unproven is a dead end. There is no principled reason for this asymmetry
   within the same worldview.

3. **Infrastructure is ready.** #292 has already wired up the Z3 backend (the `ConstExpr` → SMTLib →
   Unsat/Sat determination chain). The measure-decrease obligation at recursive call sites
   `measure(args') < measure(args)` is a first-order linear arithmetic proposition, exactly within
   the decidable fragment that this chain has already opened up—reusing the same pipeline, with no
   new backend.

## Proposal

### Core Design

**Positioning: this RFC adds no proof mechanism; it is an application of the refinement type proof
protocol.** RFC-027 has already established that "a proof function is simply a YaoXiang function
whose return type is the proposition being asserted"—the proposition is a predicate application,
predicate application is refinement typing. The human side of termination proof takes this existing
path throughout (Unproven → write a proof function → type checker verifies → witness erased), with
exactly the same protocol as the correctness domain. The new surface converges to three things:

1. **Measure function**—an ordinary compile-time function whose parameters mirror the signature of
   the function being measured, returning `Int` or an `Int` tuple (lexicographic order). Not an
   annotation, not attached to the function definition; a first-class function that can be
   referenced and unit-tested.
2. **Call-site obligation generation**—the compiler extracts the decrease obligation at every
   recursive call site from `f`'s body. This is the only new compiler logic: the call-site actuals
   live inside the function body, and the obligation (semantic constraint) must be written by the
   compiler on the user's behalf—otherwise the proof is no longer bound to the actual code (see
   design philosophy point 3).
3. **Termination proposition `Terminates(f, m)`**—a **refinement predicate whose body is written by
   the compiler**: its "constraint content" is the obligation set above, and its signature is just
   an anchor binding `(f, m)` together. The necessity of being built-in is limited to this;
   mechanistically there is nothing new. Both the function and the measure are taken into the type
   rather than only the measure: in mutual recursion `is_even`/`is_odd` have the same signature and
   can only be distinguished by measure discovery (see examples).

The three roles of the proof function follow naturally from the positioning: explicitly bind measure
`m` to `f` (discovery anchor, replacing all annotation syntax); provide a discharge point for
residual obligations (body = proof space); universally quantify over `f`'s parameter names (so `a`,
`b` can be referenced in the body to write facts). Syntax is pinned down point by point in "Detailed
Syntax"; a complete walkthrough is in "Examples."

### Examples

#### End-to-End Walkthrough: gcd

```yaoxiang
-- The function being measured (correctness domain as usual: parameter refinement, guard, recursive call)
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

-- Measure: an ordinary function (can be unit-tested, can be reused)
gcd_measure: (a: Int, b: Int) -> Int = { b }

-- Proof function: empty body—all obligations automatically closed by SMT
gcd_proof: (a: Int, b: Int) -> Terminates(gcd, gcd_measure) = { }
```

The compiler's full processing of `gcd_proof`:

1. **Signature check**: the proof function's parameters are matched one-by-one to `gcd`'s formal
   parameters at the **base type** level (`Int`, `Int`). Formal parameter refinements (`b >= 0`) are
   not repeated in the proof function signature—they are already held by the compiler as obligation
   context, and both the body and the obligations are verified within that context.
2. **Well-foundedness obligation**: `gcd_measure(a, b) >= 0`, i.e., `b >= 0`—derived directly from
   the formal parameter refinement `NonNegative(b)`. The measure's bound requires no new mechanism;
   it is just the parameter's refinement constraint (the semantic-typed realization of refinement
   typing's role for well-foundedness).
3. **Call-site obligation generation**: the only recursive call site `gcd(b, a % b)`, with path
   guard `b != 0`, generates `gcd_measure(b, a % b) < gcd_measure(a, b)`; substituting the measure
   body expands to `a % b < b`.
4. **SMT determination**: verifies that the negation of the obligation `b != 0 ∧ a % b >= b` is
   unsatisfiable—linear arithmetic + modular axioms, millisecond-level → Proved.
5. **Acceptance**: body is empty, no residual obligations to discharge. `gcd`'s termination is
   proved; the measure and proof function are witness-erased, never entering the runtime binary.

#### Non-Empty Body: Residual Obligations Referencing Proven Lemmas

When a call site's decrease depends on facts invisible to the compiler (in the example below,
`step`'s body is in a dependency module and is a black box for obligation generation), the human
writes that fact into the body by **referencing an already-proven predicate**—a bare expression, not
an Assert statement:

```yaoxiang
step: (n: Int) -> Int = { ... }

loop_x: (n: NonNegative(n)) -> Void = {
    if n == 0 { return () }
    return loop_x(step(n))
}

x_measure: (n: Int) -> Int = { n }

-- An exported already-proven postcondition from step's defining module
-- (correctness domain, ordinary predicate, already proved at the definition site):
step_decreases: (n: Int) -> Type = { n >= 1 ==> step(n) < n }

-- Termination proof = reference that lemma (referencing an already-proven predicate
-- = applying its established proof, Curry-Howard):
x_proof: (n: Int) -> Terminates(loop_x, x_measure) = {
    step_decreases(n)
}
```

Verification process: the obligation `x_measure(step(n)) < x_measure(n)` (guard `n != 0`, context
`n >= 0`) cannot be closed independently in `loop_x`'s module → a non-empty body is discovered →
assert that `step_decreases(n)` is an instance of an already-proven predicate, verification passes →
closes the remaining obligation as a lemma. If the body directly wrote `step(n) < n` out of thin
air, the verifier would honestly report Unproven—facts must come from traceable already-proven
sources.

#### Mutual Recursion: Obligations Collected by SCC

```yaoxiang
is_even: (n: NonNegative(n)) -> Bool = { if n == 0 { return true }  return is_odd(n - 1) }
is_odd:  (n: NonNegative(n)) -> Bool = { if n == 0 { return false } return is_even(n - 1) }

nat: (n: Int) -> Int = { n }

is_even_proof: (n: Int) -> Terminates(is_even, nat) = { }
is_odd_proof:  (n: Int) -> Terminates(is_odd, nat) = { }
```

Obligations are collected by SCC; obligations across function edges are
`m_callee(callee_args) < m_caller(caller_args)`—here `nat(n - 1) < nat(n)`, guard `n != 0`, and each
function closes its own loop. Both functions have the same signature and cannot be distinguished by
measure discovery alone—this is the direct reason `Terminates(f, m)` takes two arguments.

#### Negative Example: Measure Not Strictly Decreasing

```yaoxiang
loop_forever: (n: Int) -> Int = { return loop_forever(n) }
loop_proof: (n: Int) -> Terminates(loop_forever, nat) = { }
```

The obligation `nat(n) < nat(n)`, i.e., `n < n`, has a negation that is always satisfiable (Sat,
model `n = 0`) → compile error **E4022** (measure not strictly decreasing), with the counterexample
pointing at `loop_proof`. If `loop_proof` is deleted, automatic strategies fail and there is no
proof function → **E4021** (termination cannot be proved, prompting the user to provide a
`Terminates` proof function).

### Syntax Changes

**None.** This RFC adds no new syntax, keywords, or annotation positions. Compared with the existing
lineage:

| Generation        | User-side Expression                                  | Syntax Vehicle                        |
| ----------------- | ----------------------------------------------------- | ------------------------------------- |
| RFC-022 (retired) | `//! decreases: n` comment annotation                 | Comment convention, no type checking  |
| RFC-027 current   | None (fully automatic; if it cannot be proved, error) | —                                     |
| This RFC          | Measure + proof, both ordinary functions              | The function definition syntax itself |

## Design Philosophy: Separating Discovery from Verification

The mechanism choices of this RFC derive from a single principle: **termination checking = witness
discovery + witness verification, and the two have wildly different difficulty and must be
separated.**

| Role                                                                                  | Bearer                                                                                            | Difficulty                                                                                                     |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Discovery (synthesizing measure and proof)                                            | Four automatic strategies (machine, restricted templates) or human proof functions (unrestricted) | **Undecidable** in general (reduces to the halting problem)—no algorithm exists that can always find a measure |
| Verification (checking `m(args') < m(args)` for a fixed candidate at every call site) | SMT (RFC-009a pipeline)                                                                           | Decidable (first-order linear arithmetic), millisecond-level                                                   |

"Undecidable to find, decidable to check"—structurally isomorphic to the P/NP witness/verifier
separation, but more extreme: the discovery side is not just hard, it is mathematically impossible
to always solve; the verification side is fully mechanical for a fixed candidate. From this,
RFC-027's "honestly report Unproven" and the human fallback are not engineering compromises but
direct consequences of this boundary.

Three corollaries form the design constraints of this RFC:

1. **The prover is untrustworthy, and that's fine.** Soundness lives entirely in the verifier: if
   the measure or assertion is wrong, SMT judges Sat, and E4022 reports with a counterexample—the
   human can only fail to prove, never prove wrongly. Disproved is still constructive (provides a
   counterexample model): the verifier does half a thing more than an NP verifier, also providing
   evidence for refutation.
2. **Insight is frozen into a re-verifiable artifact.** Once the human's intellectual labor is
   written as a proof function, it becomes a machine-checkable certificate, reverified at every
   compile, permanently regression-proof (proof-carrying).
3. **Obligations belong to the compiler, proofs belong to the human.** Obligations ("what counts as
   proved") must be generated by the compiler from the function body; letting the human also write
   the constraint is equivalent to writing one's own acceptance standard, and the certificate no
   longer proves the actually-written code.

The four automatic strategies thus receive a unified explanation: they are a **restricted template
sequence for witness synthesis**—automatic measure synthesis in the literature is template
enumeration under constraint solving (linear rank function synthesis is a precedent). Strategy
extension = broadening the templates; discovery outside the templates is the human's job; the
verifier remains the same SMT instance throughout. The theoretical form of the non-goal "no measure
inference" is also here: general measure inference lies on the undecidable discovery side; only its
boundary (templates) can be agreed upon, completeness cannot be promised.

## Detailed Design

### Detailed Syntax

The proof function has no dedicated syntax—it is just "compile-time predicate definition" used in
another way. Three landing points are pinned down one by one:

**Return type: predicate application, with function references as actuals.**
`Terminates(gcd, gcd_measure)` is isomorphic to existing refinement applications in type position
(`f(x: Positive(x))`, `-> (result: IsMax(T, arr, result))`); function names are ordinary name
bindings (RFC-010), falling within the existing predicate-actual rules of "literals, variables
(bound by name), type applications." The only rule-level addition: E1092 actual-form determination
appends "**function references belong to the compile-time constant form**"—amending the rule text,
leaving the grammar untouched.

**Parameters: mirror `f`'s base types, universally quantified.** Parameter names and base types
match `f`'s formal parameters one by one; mismatch reports a compile error (reusing E1093 family or
adding a new one per registry, decided at implementation time). Formal parameter refinements are not
repeated—refinements and guard assumptions are injected by the compiler as obligation context.
Universal quantification is exactly the parameter semantics of the predicate definition itself (the
`x` in `Positive: (x: Int) -> Type = { x > 0 }` is also universally quantified). The zero-parameter
form was rejected: residual obligations reference `f`'s parameter names, and the names must be
writable in the body.

**Body: proof space, bare boolean expressions.** Same protocol as predicate definition bodies
`{ x > 0 }`—every item in the body is a bare expression asserted to be True. Empty bodies are legal
and cover the majority of cases (all obligations closed by SMT); non-empty assertions are verified
in the body and then fed to the remaining obligations as lemmas; the legal source for assertions is
the application of already-proven predicates (see example "Non-Empty Body").

**Discovery and binding.** For `f` where automatic synthesis fails, the compiler scans the same
module and visible imports for functions whose return type is of the form `Terminates(f, _)`;
discovered by return type, independent of the name (`f_proof` is only a natural convention). For
multiple candidates, take the first whose obligations are all green (declaration order); if all
fail, report the first counterexample. Forward references are legal: discovery happens in the check
phase, independent of declaration order.

**Runtime representation.** Proof functions are pure compile-time entities, following the RFC-027
CompileTime pattern with witness erasure—neither the measure nor the proof function enters the
runtime binary.

### Obligation Generation

For function `f` and measure `m`, the compiler generates two sets of first-order linear arithmetic
obligations, expanded per recursive call site (including cross-function calls within the SCC);
obligations across function edges are `m_callee(callee_args) < m_caller(caller_args)`, which
collapses to same-measure decrease when members share the same measure:

1. **Well-foundedness**: `m(args) >= 0`—the measure lands on natural numbers, derived from the
   parameter type's bounds; if not derivable, falls into residual obligations.
2. **Strict decrease**: for each call site `m(callee_args) < m(caller_args)`, determined under the
   **path guard** (the guard comes from the branch condition where the call site sits, reusing
   RFC-009a path-condition collection).

**Lexicographic expansion**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation expands via
lexicographic comparison into a disjunction chain—`(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`.
Expansion is performed on the obligation-generation side, keeping the SMT side in the linear
fragment, independent of the solver's native lexicographic support.

**Boundedness of the measure function itself**: `m` must be compile-time evaluable (constant
folding) or provably structurally recursive—banning the recursive deferral of the termination
problem to another unproved function (preventing infinite regression). Pathological measures are
caught by existing E4012 (constant recursion too deep) and structural checks.

### Determination Pipeline

`TerminationChecker` is extended to five levels; the first three are the current/planned paths of
RFC-027, and this RFC only adds the last two:

```
1. Structural recursion directly provable (current logic, non-regressive)
2. Automatic measure synthesis: attempt the four strategies in turn
   (strategies 1/2/4 land per RFC-027's plan, this RFC reserves the interface)
3. Synthesis succeeds → SMT determination of obligations → Proved
4. Synthesis fails (Unproven) → search for proof function (return type of form Terminates(f, _)):
     a. Found → take its measure m to generate obligations → SMT determination
     b. Residual obligations → verified in the proof function body as assertions
        (same protocol as correctness domain proof functions)
5. No proof function / proof function obligations still fail → compile error (E4021 / E4022)
```

**Discovery scope for proof functions**: defined in the same module, or exported and visible via
`import`—follows existing visibility rules, no global magic search. When multiple candidates
`Terminates(f, _)` exist for the same `f`, take the first whose obligations are all green; if
multiple fail, report the first counterexample (deterministic order: declaration order).

**`Terminates` as a built-in declaration**: it is a special case for the predicate system—its
"assertion" is generated by the compiler from `f`'s function body, and cannot be expressed by
RFC-027's predicate syntax. This is the only intentionally built-in proposition; the rationale is
recorded in the Trade-offs section.

### Compiler Changes

| Component                            | Change                                                                                                                                                                                                                      |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`    | SCC collection (Tarjan, recursive call graph already exists); unification of the four-strategy synthesis interface (current Phase 1 logic absorbed as strategy 3); proof function discovery and residual obligation sinking |
| `typecheck/checker.rs:405` call site | Unchanged—termination check position (after type checking, before ownership checking) is maintained                                                                                                                         |
| Proof pipeline (RFC-009a/#292)       | Reuse the `ConstExpr → SMTLib` chain; only the obligation shape (linear arithmetic + path guards) is added, no pipeline changes                                                                                             |
| `util/diagnostic/codes/e4xxx.rs`     | Register new E4021/E4022 (via the RFC-013 registry; build.rs threshold takes effect)                                                                                                                                        |
| locales ×6                           | Six-language templates for the two new codes                                                                                                                                                                                |
| RFC-027 documentation                | §7 and decision record aligned in place per the "Amendment Scope" table, add a 027a pointer; original narrative not rewritten                                                                                               |

### Error Codes

Aligned with the E4xxx proof-failure family (E4018 refinement predicate violation, E4020 proof
function required):

| Proposed Code | Name                            | Trigger                                                                                           |
| ------------- | ------------------------------- | ------------------------------------------------------------------------------------------------- |
| E4021         | Termination cannot be proved    | Automatic synthesis fails and no proof function (prompt to provide a `Terminates` proof function) |
| E4022         | Measure not strictly decreasing | Proof function's obligations are judged Sat by SMT (counterexample input attached)                |

The final numbering is subject to the actual state of the RFC-013 registry at implementation time
(segment legality guaranteed by the build.rs threshold); W1080 (compile-time proof degradation)
semantics are unrelated to this RFC, unchanged.

### Backward Compatibility

- Programs that previously compiled successfully: zero behavioral change (the first three pipeline
  levels execute before the fallback).
- Programs that previously failed to compile: can succeed after supplying the measure and proof
  function—this is a tightening-loosening authorized by the re-discussion clause, and only affects
  "programs that were already erroring."
- Existing test corpus (structural recursion positives, non-structural negatives): expected outputs
  unchanged, only error codes/messages for non-structural negatives updated to the new codes.

## Trade-offs

### Advantages

- **Zero new syntax**: compared with Dafny's inline `decreases` annotation, this design occupies no
  syntax space; the measure is a first-class function that can be unit-tested.
- **Aligned behavior across proof domains**: the termination domain gains an Unproven fallback
  consistent with the correctness domain; "hard-safety philosophy" evolves from "refuse if it cannot
  be proved" to "hand it to the programmer if it cannot be proved"—consistent with RFC-027's overall
  worldview.
- **Reusing infrastructure**: obligations are linear arithmetic, going through the #292 pipeline,
  with no new backend and no new family of determination logic.
- **Clause fulfillment as governance**: the re-discussion is documented as a sub-RFC, and the
  decision record remains auditable.

### Disadvantages and Risks

- **Writing burden**: a two-piece set (measure + proof function) is more verbose than a single
  inline annotation. Accepted—the fallback is a low-frequency path, and in exchange the measure
  becomes reusable and testable.
- **`Terminates` is the only predicate with a compiler-written body**: deviates from the purity of
  "all predicate bodies user-writable." Rationale: call-site actuals live inside the function body,
  and library predicates cannot reference them by principle; the built-in surface converges to a
  single proposition name, with zero new mechanism (see Core Design and Design Philosophy).
- **Fallback may be triggered prematurely while strategies 1/2/4 are not yet implemented**: Phase
  1's automatic surface is narrow, and more functions will land in the proof function path.
  Mitigation: after the four strategies are implemented, the fallback surface automatically narrows;
  the proof functions written first are not wasted—they continue to serve as discharge carriers for
  residual obligations after strategy upgrades.
- **Counterexample quality**: Sat counterexamples for non-linear guards may not be intuitive.
  Recorded as a known limitation; counterexample presentation will be iterated per the RFC-013
  diagnostic message specification.

## Non-Goals

- No new syntax/keywords/annotation positions.
- No generalization of automatic measure synthesis (the four strategies stay on RFC-027's planned
  schedule; beyond scope goes to the fallback).
- No joint solving with RFC-009a borrowed propositions (each determined independently, sharing the
  backend).
- No totality checking at the dependent type level.
- No measure inference (guessing a measure from a function body is within strategies 1/2/4's scope;
  this RFC only consumes explicit measures—see the theoretical boundary in the last paragraph of
  Design Philosophy).

## Stages and Acceptance

- [x] RFC-027 aligned in place (§7, decision record, §6.8 message, add 027a pointer)—landing in the
      same commit as this draft
- [ ] SCC collection + obligation generation (including lexicographic expansion, path guards) +
      proof function discovery
- [ ] SMT determination wiring (reusing the #292 pipeline) + verification for both empty-body and
      non-empty-body
- [ ] E4021/E4022 registration + six-language locales
- [ ] E2E: positives (gcd / merge partition / is_even-is_odd mutual recursion) / negatives (measure
      not decreasing → E4022; no proof function → E4021) / zero regression on structural recursion
      and existing termination tests
- [ ] Acceptance demo: a human-written proof function with a non-decreasing measure → compile fails
      with a readable counterexample; fixed version passes

## References

- #318 (this RFC's project-initiating issue), #251 (parent milestone P1)
- RFC-027 (host: §6.7 recursive termination checking, §7 termination checking, decision record
  re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline—infra prerequisite)
- RFC-013 (error code registry and proof-failure semantics family)
