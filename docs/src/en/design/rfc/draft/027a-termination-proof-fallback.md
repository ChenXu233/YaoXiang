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

Fulfills the re-discussion clause reserved in the RFC-027 decision record: for **non-structural
recursion** that the four automatic strategies cannot cover for termination proof
(non-directly-decreasing recursion like gcd, mutual recursion), this RFC provides a **proof function
fallback**—measures and proofs are both ordinary YaoXiang functions, with zero new syntax, validated
by the type checker. The principle of full automation priority remains unchanged: automatic
strategies are still the first path, and user intervention only occurs after Unproven.

## Motivation

### Host Qualification and Revision Scope

RFC-027 did not reserve a sub-RFC slot (unlike RFC-029's pattern of reserving 029a). The host
qualification for this sub-RFC comes from the re-discussion clause within the RFC-027 decision
record itself:

> Loops whose termination cannot be proven by any strategy cause the compiler to error directly—this
> is hard-safety philosophy, not a defect. **If, in the future, there are real-world scenarios (not
> academic constructions) where none of the four strategies can cover an algorithm, it will be
> re-discussed.** — RFC-027 Decision Record

This RFC is the formal vehicle for that "re-discussion." It **does not overturn** the recorded
decision—the clause itself is part of the decision, and triggering it is precisely the decision
working as designed; it is the anticipated exercise of the clause.

The revision scope of RFC-027 is precisely limited to three points:

| Revision Point                            | Original Record                                              | After Revision                                                                                                                                                                                                                      |
| ----------------------------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| §7 "No half-automatic annotation opening" | Termination checking is fully closed                         | Narrowed to "no **annotation syntax** opening"—proof function fallback is not an annotation (no new syntax slot, not attached to the function definition), aligned with the "compile-time predicates have no new keyword" worldview |
| Decision record "Unproven → direct error" | Termination is the only proof domain without a fallback      | Changed to "Unproven → can provide a proof function"—aligned with the correctness domain (predicate Unproven → proof function, RFC-027 §Proof Function) behavior                                                                    |
| §6.8 Error message                        | "Please use an iterative pattern analyzable by the compiler" | Added "or provide a measure function and a termination proof function"                                                                                                                                                              |

**Unrevised** parts: the priority of the four automatic strategies, the full-automation priority
principle, the rest of hard-safety philosophy (Disproved remains a compile error + counterexample;
correctness domain behavior unchanged).

### Current Problems (measured 2026-09-14, dev after v0.7.14)

1. **TerminationChecker can only prove structural recursion**. `termination.rs` is currently in
   Phase 1: strategy 3 (bounded increase/decrease), direct decrease of recursive arguments
   (`factorial(n-1)`), `for` loops naturally terminate. Strategies 1/2/4 are not yet implemented
   (RFC-027's planned extensions). Non-structural recursion fails to compile directly:

   ```yaoxiang
   gcd: (a: Int, b: Int) -> Int = {
       if b == 0 { return a }
       return gcd(b, a % b)
   }
   ```

   The recursive arguments `(b, a % b)` are not direct decreases of the parameters—currently it
   errors and requires rewriting into an iterative pattern. gcd cannot be rewritten into an
   "analyzable iterative pattern" without sacrificing readability; merge sort partitioning, mutual
   recursion (even/odd judgment), tree traversal are the same. These are real-world algorithms, not
   academic constructions—**the trigger condition for the re-discussion clause is met**.

2. **Asymmetric proof domain fallback**. RFC-027's correctness propositions (predicates like
   `Positive(b)`, `IsMax`) have a proof function fallback when Unproven; termination is the only
   proof domain where Unproven is a dead end. Within the same worldview, this asymmetry has no
   principled reason.

3. **Infrastructure is ready**. #292 has truly connected the Z3 backend (`ConstExpr` → SMTLib →
   Unsat/Sat decision chain). The measure-decrease obligation at recursive call sites,
   `measure(args') < measure(args)`, is a first-order linear arithmetic proposition, falling exactly
   within the decidable fragment already cleared by this chain—reusing the same pipeline, no new
   backend needed.

## Proposal

### Core Design

Three concepts, all following "everything is a YaoXiang function":

1. **Measure function**—an ordinary compile-time function whose input parameters match the measured
   function's signature, returning `Int` or a tuple of `Int` (lexicographic order). It is not an
   annotation, not attached to the function definition; it is a first-class function that can be
   referenced and unit-tested.
2. **Termination proposition `Terminates(f, m)`**—a compiler built-in proposition (not
   user-redefinable): "function `f` terminates with respect to measure `m`." Its obligations are
   auto-generated by the compiler from `f`'s body (see Detailed Design), and therefore cannot be
   expressed as a pure library predicate.
3. **Termination proof function**—a compile-time function whose return type is `Terminates(f, m)`.
   Its two roles: explicitly **bind** measure `m` to `f` (the only discovery and hookup point,
   replacing all annotation syntax); its body discharges residual obligations that SMT cannot close
   (empty body in most cases).

### Examples

```yaoxiang
# Positive example 1: non-direct decrease—gcd
gcd: (a: Int, b: Int) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

gcd_measure: (a: Int, b: Int) -> Int = { b }

# Empty body is enough: the obligation a % b < b (guard b ≠ 0) is closed directly by SMT
gcd_proof: () -> Terminates(gcd, gcd_measure) = { }
```

```yaoxiang
# Positive example 2: mutual recursion—obligations collected by SCC
is_even: (n: Int) -> Bool = { if n == 0 { return true }  return is_odd(n - 1) }
is_odd:  (n: Int) -> Bool = { if n == 0 { return false } return is_even(n - 1) }

nat: (n: Int) -> Int = { n }

is_even_proof: () -> Terminates(is_even, nat) = { }
is_odd_proof:  () -> Terminates(is_odd,  nat) = { }
```

```yaoxiang
# Negative example: measure does not strictly decrease
loop_forever: (n: Int) -> Int = { return loop_forever(n) }
# The obligation n < n for Terminates(loop_forever, nat) is false—
# SMT returns Sat → counterexample → compile error E4022 (measure not strictly decreasing, with counterexample)
```

### Syntax Changes

**None.** This RFC adds no new syntax, keywords, or annotation slots. Comparison with the existing
lineage:

| Generation           | User-side Expression                                  | Syntax Vehicle                       |
| -------------------- | ----------------------------------------------------- | ------------------------------------ |
| RFC-022 (deprecated) | `//! decreases: n` comment annotation                 | Comment convention, no type checking |
| RFC-027 current      | None (fully automatic, errors if it cannot be proven) | ——                                   |
| This RFC             | Measure + proof, both ordinary functions              | Function definition syntax itself    |

## Detailed Design

### Obligation Generation

For function `f` and measure `m`, the compiler generates two sets of first-order linear arithmetic
obligations, expanded at each recursive call site (including cross-function calls within SCC):

1. **Well-foundedness**: `m(args) >= 0`—the measure falls on natural numbers, derived from parameter
   type bounds; enters residual obligations when it cannot be derived.
2. **Strict decrease**: at each call site, `m(callee_args) < m(caller_args)`, judged under the
   **path guard** (the guard comes from the branch condition of the call site, reusing the RFC-009a
   path condition collection).

**Lexicographic expansion**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation is expanded as a
disjunction chain following lexicographic comparison—`(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`.
Expansion is done on the obligation generation side, keeping the SMT side in the linear fragment,
without depending on the solver's native support for lexicographic order.

**Boundedness of the measure function itself**: `m` must be compile-time evaluable (constant
folding) or provable by structural recursion—prohibited from recursively delegating the termination
problem to another unproven function (preventing infinite regression). Pathological measures are
caught by the existing E4012 (constant recursion too deep) and structural checks.

### Judgment Pipeline

`TerminationChecker` is extended to five levels. The first three are the current/planned paths of
RFC-027; this RFC only adds the latter two:

```
1. Structural recursion directly provable (current logic, no regression)
2. Automatic measure synthesis: try the four strategies one by one (strategies 1/2/4 land per RFC-027 plan, this RFC reserves the interface)
3. Synthesis succeeds → obligation SMT judgment → Proved
4. Synthesis fails (Unproven) → search for proof function (return type shaped like Terminates(f, _)):
     a. Found → take its measure m, generate obligations → SMT judgment
     b. Residual obligations → verified as assertions within the proof function body (same as the correctness domain proof function protocol)
5. No proof function / proof function obligations still fail → compile error (E4021 / E4022)
```

**Proof function discovery domain**: defined in the same module, or visible via `import`—following
existing visibility rules, no global magic search. When there are multiple candidate
`Terminates(f, _)` for the same `f`, the first one with all obligations green is taken; if multiple
fail, the first's counterexample is reported (deterministic order: declaration order).

**Built-in declaration of `Terminates`**: it is a special case in the predicate system—its
"assertions" are generated by the compiler from `f`'s body and cannot be expressed with RFC-027's
predicate syntax. This is an intentional unique built-in proposition, with reasons recorded in the
Trade-offs section.

### Compiler Changes

| Component                            | Change                                                                                                                                                                                                             |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `typecheck/layers/termination.rs`    | SCC collection (Tarjan, recursive call graph already exists); unify the four-strategy synthesis interface (current Phase 1 logic subsumed as strategy 3); proof function discovery and residual obligation sinking |
| `typecheck/checker.rs:405` call site | Unchanged—the termination check location (after type checking, before ownership checking) is maintained                                                                                                            |
| Proof pipeline (RFC-009a/#292)       | Reuse the `ConstExpr → SMTLib` chain; new obligation shapes limited to linear arithmetic + path guards, no pipeline changes                                                                                        |
| `util/diagnostic/codes/e4xxx.rs`     | Register new E4021/E4022 (via RFC-013 registry, build.rs threshold takes effect)                                                                                                                                   |
| locales ×6                           | Two new codes' six-language templates                                                                                                                                                                              |
| RFC-027 document                     | §7 and the decision record are calibrated in place per the "Revision Scope" table, with a 027a pointer; original narrative not rewritten                                                                           |

### Error Codes

Aligned with the E4xxx proof failure family (E4018 refined predicate violation, E4020 proof function
needed):

| Proposed Code | Name                            | Trigger                                                                                       |
| ------------- | ------------------------------- | --------------------------------------------------------------------------------------------- |
| E4021         | Termination unprovable          | Automatic synthesis fails and no proof function (hint: provide a `Terminates` proof function) |
| E4022         | Measure not strictly decreasing | Proof function's obligation is judged Sat by SMT (with counterexample input)                  |

Final numbering is subject to the actual state of the RFC-013 registry at implementation time
(segment legality guaranteed by the build.rs threshold); W1080 (compile-time proof degradation)
semantics are unrelated to this RFC and are untouched.

### Backward Compatibility

- Previously compiling programs: zero behavior change (the first three pipeline levels execute
  before the fallback).
- Previously failing-to-compile programs: pass after adding the measure and proof function—this is a
  tightening-then-relaxing authorized by the re-discussion clause, and only affects programs that
  "originally errored".
- Existing test corpus (structural recursion positive cases, non-structural negative cases) expected
  output unchanged, only the non-structural negative cases' error codes/messages updated per the new
  codes.

## Trade-offs

### Pros

- **Zero new syntax**: compared to Dafny's inline `decreases` annotation, this design does not
  occupy syntax space; the measure is a unit-testable first-class function.
- **Proof domain behavior alignment**: the termination domain gains an Unproven fallback consistent
  with the correctness domain, "hard-safety philosophy" evolves from "reject if unprovable" to "hand
  to the programmer if unprovable"—consistent with RFC-027's overall worldview.
- **Reuse infrastructure**: obligations are linear arithmetic, going through the pipeline already
  connected by #292, no new backend, no new judgment logic family.
- **Clause fulfillment as governance**: the re-discussion is left in traceable form as a sub-RFC,
  the decision record remains auditable.

### Cons and Risks

- **Writing overhead**: the two-piece set (measure + proof function) is more verbose than a single
  inline annotation. Accepted—the fallback is a low-frequency path, and it gains reusability and
  testability of the measure.
- **`Terminates` is the unique built-in proposition**: deviates from the purity of "all predicates
  are user-definable". Reason: obligations originate from the internal structure of the function
  body, which in principle cannot be expressed by pure library predicates; this is the minimal
  built-in surface the system needs.
- **Before strategies 1/2/4 land, the fallback may be triggered prematurely**: the automatic surface
  in Phase 1 is narrow, so more functions will fall to the proof function path. Mitigation: after
  the four strategies land, the fallback surface automatically narrows; leading with proof functions
  is not wasted—they continue to serve as discharge vehicles for residual obligations after the
  strategy upgrade.
- **Counterexample quality**: Sat counterexamples may not be intuitive for non-linear guards.
  Recorded as a known limitation; counterexample presentation is iterated per the RFC-013 diagnostic
  message specification.

## Non-Goals

- No new syntax/keywords/annotation slots.
- No generalized extension of automatic measure synthesis (the four strategies maintain RFC-027's
  planned scope; beyond that, take the fallback).
- No joint solving with RFC-009a borrow propositions (each judges independently, sharing the
  backend).
- No totality checking at the dependent type level.
- No measure inference (guessing the measure from the function body falls within the scope of
  strategies 1/2/4; this RFC only consumes explicit measures).

## Stages and Acceptance

- [x] RFC-027 in-place calibration (§7, decision record, §6.8 message, with 027a pointer)—lands in
      the same commit as this draft
- [ ] SCC collection + obligation generation (including lexicographic expansion, path guards) +
      proof function discovery
- [ ] SMT judgment wiring (reuse #292 pipeline) + empty-body / non-empty-body two-tier verification
- [ ] E4021/E4022 registration + six-language locales
- [ ] E2E: positive cases (gcd / merge partition / is_even-is_odd mutual recursion) / negative cases
      (measure does not decrease → E4022; no proof function → E4021) / structural recursion and
      existing termination tests zero regression
- [ ] Acceptance demo: deliberately write a proof function with a non-decreasing measure → compile
      fails and counterexample is readable; fix and it passes

## Related

- #318 (the issue that initiated this RFC), #251 (parent milestone P1)
- RFC-027 (host: §6.7 recursive termination checking, §7 termination checking, decision record
  re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline—infrastructure prerequisite)
- RFC-013 (error code registry and proof failure semantic family)
