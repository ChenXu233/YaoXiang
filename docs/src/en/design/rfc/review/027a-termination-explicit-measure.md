---
title: 'RFC-027a: Explicit Measures for Termination Checking'
status: 'Under Review'
author: 'Chenxu'
created: '2026-09-14'
updated: '2026-09-30'
issue: '#318'
impl_status: 'partial'
---

# RFC-027a: Explicit Measures for Termination Checking

## Summary

RFC-027 §7 establishes the termination-checking criterion as **refinement types** (refinement types
enter verification mode), and §6.9 gives the linguistic form of explicit measures (the builtin
predicate `Terminates`, a core primitive alongside `Int` and `Never`). Both are language design
decisions, belong to the host RFC, and are finalized.

This sub-RFC tackles the implementation mechanism: how obligations are generated from computational
structure, how the judgment pipeline is orchestrated, how diagnostics provide direction when the
prover fails, how mutual-recursion measures are shared (SCC optimization), and how error codes are
registered. **It does not re-state language semantics**; it only fixes the mechanism.

## Motivation

### Why a Sub-RFC Is Needed

The criterion (refinement types entering verification mode) and the form (`Terminates` written in
the type position) were already settled in RFC-027, at language-level granularity. The remaining
questions are too fine-grained; folding them into the host RFC would bloat its body:

- Where do obligations come from (recursive call sites, loop back-edges, path guards)
- How to compare when the measure returns a tuple (lexicographic expansion)
- Whether well-foundedness and strict decrease are two independent obligations or one
- How automatic exploration and explicit measures coexist in the same pipeline
- For mutual recursion, how to avoid duplicating the same measure and re-verifying it
- How to provide **direction** when proving fails rather than flatly rejecting

### Trigger

The direction is triggered by #318: non-structural recursion (gcd-style non-directly- decreasing,
mutual recursion, merge partitioning) exceeds the template sequences of measure exploration (all
four strategies in RFC-027 §6.2–6.5 take "variable of a bounded type" or "target type + swap
operation" as input), and cannot be rewritten into an analyzable iterative pattern without
destroying readability. The re-discussion clause in RFC-027's "Open Questions" section activates
here.

## Proposal

### Relationship with Automatic Exploration

An explicit measure is **not a separate pipeline**, but the input after exploration fails. Once the
measure is given, the same SMT runs the same set of obligations; if they fail, an error is reported
with a counterexample. Full automation takes priority as before: exploration always runs first, and
user intervention happens only after exploration fails.

This means the two paths share all downstream mechanisms — obligation generation, SMT judgment,
lexicographic expansion, diagnostic format all have one implementation.

### Obligation Generation

For a function `f` and a measure `m`, two **independent** obligations are generated and expanded at
each recursive call site (including cross-function calls within an SCC):

1. **Well-foundedness**: `m(args) >= 0` — the measure must land on the natural numbers; when the
   measure returns a non-natural-number type, take the lower bound under the appropriate order on
   that type. Derived from parameter refinements; if it cannot be derived, it enters a residual
   obligation.
2. **Strict decrease**: at every call site, `m(callee_args) < m(caller_args)`, judged **under path
   guards** — guards come from the branch conditions of the call site, reusing the path-condition
   collection from RFC-009a.

The two are independent rather than unified, because the failure directions differ: a
well-foundedness failure means the measure's range is wrong (e.g. `Int` may be negative); a decrease
failure means the recursive parameter is not moving in the intended direction. The diagnostics need
to distinguish them to give the right checking direction (see the "Diagnostics" section).

**Loops are analogous**: replace "call site" with "back-edge" — on every execution path through the
loop body, `m(next_state) < m(current_state)`, with guards coming from loop conditions and in-body
branches.

**Lexicographic expansion**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation is expanded into
a disjunction chain under lexicographic comparison — `(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`. The
expansion is done on the **obligation-generation side**, keeping the SMT side as a linear fragment,
not relying on native support for lexicographic orders in the solver.

**The measure itself must be compile-time evaluable**: `m` must be a function provable via constant
folding or structural recursion — it is forbidden to recurse the termination problem back onto
another unproven function (preventing infinite regress). Pathological measures are caught by the
existing E4012 (constant recursion too deep) and structural checks.

### Judgment Pipeline

```
1. Parameter decrease (structural recursion, strongest path, try first)
2. Measure exploration: the four-strategy template sequence (RFC-027 §6.2–6.5), stop on first hit
3. Exploration succeeds → generate obligations → SMT judge → Proved
4. Exploration fails → check whether an explicit measure is given in the type position (Terminates)
     Yes → take that measure, generate obligations → SMT judge
     No  → E4021 (termination unprovable, with suggested checking direction)
5. Obligations judged false by SMT → E4022 (measure does not hold, with counterexample)
```

Steps 1–3 are the existing RFC-027 path; this RFC adds step 4 and two error codes. The whole
pipeline runs only when refinement types trigger it (RFC-027 §7) — plain types without refinement
generate no obligations.

### Arity: Unary and Binary Forms

RFC-027 §6.9 fixes two arities; this RFC explains the landing point of each:

| Form                    | Anchor                   | Landing point                                                                                                                                                |
| ----------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Terminates(m)`         | Name of the host binding | Default form at definition sites — self-recursive functions, loops                                                                                           |
| `Terminates(FnType, m)` | Explicit function type   | When the measure must be explicitly assigned to a particular function type (the measure is defined elsewhere, the same measure serves multiple computations) |

The two are not two different constructs but two arities of the same predicate: the termination
obligation always lands on "the computation annotated at the type position where the refinement
sits." Mutual recursion does **not** need the binary form — each of two functions writes its own
unary form, and the shared relation is recognized by SCC (see the next section).

**Implementation status (2026-09-30)**:

- **The unary form has landed**, with two measured landing points: the function **return-type
  position** (`gcd: (a: Int, b: Int) -> Terminates(b)` → key `gcd`) and the variable **binding
  position** (`acc: Terminates(n - i) = ...` → key `acc`).
- **The binary form `Terminates(FnType, m)` is parked**. Reason: the implementation surface is unary
  — when `Terminates` enters the type parser it is treated as a unary predicate (the first argument
  is the measure expression), whereas the binary form's first argument is a **function type**,
  requiring an independent parsing path for "which function type does the measure belong to." There
  is no consumer right now: mutual recursion is fully expressible via each function's unary form
  plus SCC sharing, and SCC is not yet landed, so doing the binary form first has no comparable
  consumer. Unpark condition: after SCC lands, if there is still a need for "one measure serving
  multiple computations," add the binary parser. The gcd example in §Examples of this document
  (binary) is the form to rewrite.

### SCC: Measure-Sharing Optimization

For a group of mutually recursive functions (a strongly connected component in the call graph)
sharing the same measure, the cross-function edge obligation is
`m_callee(callee_args) < m_caller(caller_args)`, which collapses to "same-measure decrease" when
members share the measure.

**This is an optimization, not a correctness prerequisite**: without sharing, each function writes
its own measure and closes its own loop, and the check still passes. SCC's value is recognizing
"this group uses the same measure," eliminating duplicate writing and duplicate verification.

A **function-level call graph and SCC collection** must be built — the existing `TypeDepGraph`
records variable-level type-annotation dependencies (the VC trigger from RFC-027 §6.1); it is a
variable-level graph and cannot be reused.

### Examples

#### gcd: Non-structural Recursion

```yaoxiang
// Measure: a normal function, unit-testable, reusable
gcd_measure: (a: Int, b: Int) -> Int = { b }

gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

Obligation generation:

1. **Well-foundedness**: `gcd_measure(a, b) >= 0`, i.e. `b >= 0` — derived directly from the
   parameter refinement `NonNegative(b)`.
2. **Strict decrease**: the sole recursive call site `gcd(b, a % b)`, with path guard `b != 0`; the
   obligation `gcd_measure(b, a % b) < gcd_measure(a, b)`, expanded by inlining the measure body to
   `a % b < b`.
3. **SMT**: verify that the negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved.

#### Loops: Anonymous Constructs Gain a Denotation

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

The `Terminates(m)` refinement annotates the value type of the loop body's tail expression, and the
anchor is provided by the binding name `acc` — so the loop **becomes denotable**, and the "anonymous
construct has no denotation" blind spot is closed. This also explains the unity of the two arities:
the obligation always falls on the computation annotated at the type position where the refinement
sits; there is no distinction between functions and loops.

Obligation: on the back-edge `(n - i') < (n - i)`, with guard `i < n`, substituting `i' = i + 1`
gives `1 > 0`, which is tautologically true → Proved.

**Implementation status (2026-09-30): this loop form is blocked (D5)**. Currently `while`
expressions are typed as `Void`; there is no value type to carry a refinement — so `Terminates` in
`acc: Terminates(n - i) = while ...` cannot attach to any computation. This form is pushed to Open
Questions, to be re-evaluated once `while` gains a value semantics (or loops switch to a
value-bearing form). What has landed is the **function form** (return-type-position unary measure),
see the gcd example in §Examples. Termination checking on the loop side goes through the existing
refinement-variable gate + E4021 (§Stages and Acceptance).

#### Mutual Recursion: SCC-Shared Measure

```yaoxiang
nat: (n: Int) -> Int = { n }

is_even: Terminates(nat) = {
    if n == 0 { return true }
    return is_odd(n - 1)
}
is_odd: Terminates(nat) = {
    if n == 0 { return false }
    return is_even(n - 1)
}
```

Both functions carry the unary `Terminates(nat)`. After SCC collection, the shared measure is
recognized; the cross-function edge obligation `nat(n - 1) < nat(n)` is tautologically true under
guard `n != 0`, and both functions close their loop in one verification pass.

Without SCC recognition, both functions still pass verifying the same obligation individually — just
with one duplicate pass. This confirms SCC's role as an optimization.

### Diagnostics

**When well-foundedness cannot be derived, do not reject outright — suggest a checking direction.**
This must be distinguished from the diagnostic for a failed decrease obligation: the former points
at the measure's range, the latter at the recursive parameter. The three failure categories and
their suggestions:

| Failure                        | Suggested direction                                                                                       |
| ------------------------------ | --------------------------------------------------------------------------------------------------------- |
| Well-foundedness undetermined  | Does the measure attain its lower bound (e.g. does an `Int` measure need `>= 0`)?                         |
| Strict decrease disproven      | Is the recursive parameter really moving in the measure's decreasing direction; attach SMT counterexample |
| No measure, exploration failed | Suggest binding a name to the computation and supplying a measure in the type position (RFC-027 §6.9)     |

Counterexample rendering follows the RFC-013 diagnostic-message convention. Under non-linear path
guards, the Sat counterexample may be unintuitive — this is recorded as a known limitation, to be
iterated along with RFC-013.

### Error Codes

Aligned with the E4xxx proof-failure family (E4018 refinement-predicate violation, E4020 proof
function required):

| Proposed code | Name                   | Trigger                                                                                                     |
| ------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------- |
| E4021         | Termination unprovable | Exploration failed and the type position has no explicit measure (suggest providing a `Terminates` measure) |
| E4022         | Measure does not hold  | Measure obligation disproven by SMT (attach counterexample)                                                 |

The final numbers follow the RFC-013 registry at implementation time (segment legality is guaranteed
by the build.rs threshold).

### Existing Diagnostic-Layer Defect

In the current implementation, in-scope termination failures are reported as **E8001 "Internal
Compiler Error"** (`Unproven` is formatted as an ICE) — termination checking is not an ICE, and
occupying that code slot both misleads users and masks real faults. This RFC fixes this in passing:
in-scope termination failures go to E4021/E4022, and the ICE code slot is returned to genuine
internal errors.

### Compiler Changes

| Component                           | Change                                                                                                                                                                                                               |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`   | Interface unification (exploration path and explicit-measure path share obligation generation and judgment); wire in Z3 (production pipeline injection; `with_z3` currently only exists in unit tests)               |
| Function call graph + SCC (**new**) | No cross-function call graph anywhere in the repo — `TypeDepGraph` is variable-level type dependency, not reusable. Build a function-level call graph and SCC collection to support mutual-recursion measure sharing |
| Obligation generation               | New: well-foundedness / strict-decrease obligations, path-guard injection, lexicographic expansion                                                                                                                   |
| Proof pipeline (RFC-009a / #292)    | Reuse the `ConstExpr → SMTLib` chain and `Mod` etc. operator mappings, no backend changes                                                                                                                            |
| `util/diagnostic/codes/e4xxx.rs`    | Register E4021/E4022 (via RFC-013 registry, build.rs threshold takes effect)                                                                                                                                         |
| locales ×6                          | Six-language templates for the two new codes                                                                                                                                                                         |
| Diagnostic layer                    | In-scope termination failures move out of E8001 into the proof-failure family                                                                                                                                        |

### Backward Compatibility

- Programs previously rejected by termination checking but **without refinement annotations**: under
  the new criterion they do not enter verification mode and compile directly — an intentional
  relaxation, only loosening, never tightening.
- Programs previously rejected by termination checking **with** refinement annotations: passing
  after supplying a measure.
- Structural-recursion positive cases and existing termination tests: expected output unchanged.

## Trade-offs

### Pros

- **Zero new syntax**: the measure sits in the type position, reusing the refinement-predicate
  application mechanism; no `decreases`-style syntactic slot.
- **Aligned behavior with the proof domain**: the termination domain gains a fallback channel
  consistent with the correctness domain, but with a different landing point — propositions the
  correctness domain cannot prove get a proof function written **in the body**; measures the
  termination domain cannot explore get declared **in the type position**. Both share the same
  mechanism (both are refinement-type applications).
- **Reuse of infrastructure**: obligations are linear arithmetic plus path guards, flowing through
  the already-wired #292 pipeline, no new backend.
- **Loops gain a denotation**: the binding name is the anchor, and loops are fully isomorphic to
  functions in obligation generation — no special case for loops.

### Cons and Risks

- **Prerequisite infrastructure is no less than the "wiring" itself**: a function- level call graph
  and SCC collection must be built, and Z3 is not yet injected into the production pipeline. The SCC
  portion can be deferred (it is an optimization); the call graph shares the same source as SCC, so
  deferring it means mutual recursion can only write per-function measures — still passes, just with
  duplicate verification.
- **Explicit measures impose writing overhead on a low-frequency path**: the two-part setup (a
  measure function + a type-position declaration) is more verbose than an inline annotation.
  Accepted — the fallback is a low-frequency path, and in return the measure is reusable and
  unit-testable.
- **Well-foundedness-obligation noise**: when the measure returns `Int`, every case must prove
  `>= 0`. Mitigation: if parameter refinements already supply the lower bound, it is derived
  automatically; only when undetermined does it enter a residual obligation, and then only direction
  is given, not rejection.
- **Counterexample quality**: under non-linear guards, the Sat counterexample is unintuitive. Known
  limitation.
- **`Terminates` is the only predicate whose body the compiler writes**: this departs from the
  purity of "all predicate bodies are user-writable." Reason: its assertion (measure decrease at
  every call site / back-edge) lives inside the computational structure; a user-written predicate
  has no way to reference a function body or loop body. The builtin surface collapses to a single
  name; the mechanism adds nothing.

## Alternatives

- **`with decreases (b)` inline-annotation syntax**: an early proposal in this issue, withdrawn. "No
  **annotation-syntax** slot for termination checking" is a settled decision from RFC-027; inline
  annotations would turn unsupported termination modes into syntactic slots rather than
  type-position ones, conflicting with the worldview "everything is a YaoXiang function, everything
  is verified by the type checker."
- **Standalone proof function (`gcd_proof` returning `Terminates(f, m)`, discovered by scanning
  return types)**: an early design. Abandoned — it introduces four kinds of complexity: "discovery
  mechanism," "naming convention," "pick the first of multiple candidates," and "how names inside a
  proof function are resolved," all of which stem from putting the proof outside the type. Moving it
  to the type position makes all four go away.
- **Only let the measure enter the type (`Terminates(m)`), drop the binary form**: shorter, but
  loses the expressiveness of "explicitly assigning the measure to a specific function type" (when
  the measure is defined elsewhere, or the same measure serves multiple computations, there is
  nowhere to put it). The two arities are two arities of the same predicate; keeping them costs
  nearly nothing.
- **No fallback, require the user to rewrite into an analyzable iterative pattern**: i.e. the status
  quo. gcd / merge partitioning / mutual recursion cannot be rewritten without destroying
  readability — that is precisely the trigger condition for the re-discussion clause.

## Non-Goals

- No new syntax / keywords / annotation slots.
- No generalization of automatic measure synthesis (the four strategies stay on the RFC-027 plan;
  anything outside goes through explicit measures). Generic measure inference lives on the
  undecidable side of "discovery"; only template boundaries can be agreed on, completeness cannot be
  promised.
- No joint solving with RFC-009a borrowed propositions (each side judges independently, sharing the
  backend).
- No totality checking at the dependent-type level.
- The measure is not forced to return a natural-number type (no aspiration to Lean's
  `WellFoundedRelation` typeclass mechanism) — the measure's return type is unconstrained, and
  well-foundedness is delegated as an independent obligation to refinement inference or SMT.

## Stages and Acceptance

- [x] Obligation generation: well-foundedness / strict decrease, path-guard injection
- [x] Explicit-measure wiring: unary-form anchor resolution (return-type position / binding
      position) + type-position measure extraction
- [x] SMT-judgment wiring (reuse the #292 pipeline, inject Z3 into the production pipeline — the
      injection point had never been called before)
- [ ] Function-level call graph + SCC collection (measure-sharing optimization)
- [ ] Lexicographic expansion (when the measure returns a tuple)
- [x] E4021/E4022 registration + six-language locales (three-party consistency: codes ↔ locales ↔
      RFC-013 code table)
- [x] Diagnostic-layer fix: in-scope termination failures moved out of E8001, into the proof-failure
      family. E4021 exists; E4022 goes through `DisproofKind::MeasureNotDecreasing` to the user
      domain, with a test case pinned to prevent it being downgraded to an ICE
- [x] E2E positive: gcd (including the "residual obligation, not rejected" form), zero regression
      for structural recursion
- [x] E2E negative: measure does not hold → E4022 (with counterexample); no measure → E4021
- [ ] E2E pending: loop `Terminates(n - i)` (D5: `while` has no value type); `is_even`-`is_odd`
      mutual recursion (waiting for SCC)
- [x] Criterion regression: recursion and loops without refinement annotations are no longer
      rejected by termination checking; with refinement annotations, obligations are triggered as
      before
- [x] Acceptance demo: write a measure-not-holding declaration → compile fails and the
      counterexample is readable (`measure_not_decreasing_err.yx`); fix it and it passes
      (`measure_decreasing.yx`)

### Implementation Landing Log (2026-09-30)

Landing surface: unary `Terminates` enters the type parser → extract the measure from the AST
(**not** via the lossy `MonoType` conversion, which would drop the measure to `Int(64)`) →
obligation generation at recursive call sites (with path guards) → well-foundedness and
strict-decrease dual SMT judgment → diagnostic emission.

The **two gates** for emitting E4022 (both under-report-only, never misreport):

1. **Disproven, not merely undisprovable** — the solver must return a counterexample (`Sat`) before
   reporting; `Unknown` is "cannot judge" and is handled per the "give direction, do not reject"
   principle from §Trade-offs.
2. **Measure well-foundedness is already proven** — `<` on integers is **not** well founded, so "the
   measure does not decrease" does **not** equal "does not terminate" (gcd terminates even when `b`
   is negative). When well-foundedness has not been proven, even a disproven decrease does not
   trigger a report; it enters a residual obligation.

The existence of these two gates resolves the apparent contradiction in §Trade-offs between "give
direction, do not reject" and "disproven means report": well-foundedness is precisely that "no
decrease = no termination" license.

## Related

- #318 (this RFC's originating issue), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement-type criterion, §6.1–6.5 the four measure-exploration strategies,
  §6.9 explicit measures, §Open Questions re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path-condition collection — infrastructure prerequisite)
- RFC-013 (error-code registry and proof-failure semantic family)
