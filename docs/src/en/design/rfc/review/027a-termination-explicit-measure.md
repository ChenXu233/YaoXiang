---
title: 'RFC-027a: Explicit Measure for Termination Checking'
status: 'Under Review'
author: 'ChenXu'
created: '2026-09-14'
updated: '2026-10-02'
issue: '#318'
impl_status: 'partial'
---

# RFC-027a: Explicit Measure for Termination Checking

## Summary

RFC-027 §7 sets the termination checking criterion as **refinement types** (refinement enters
verification mode), and §6.9 gives the language form of the explicit measure (the built-in predicate
`Terminates`, a core primitive alongside `Int` and `Never`). Both are language design, belonging to
the host RFC, and are finalized.

This sub-RFC carries the implementation mechanism: how obligations are generated from computation
structure, how the judgment pipeline is orchestrated, how diagnostics point in the right direction
when the proof fails, how measures for mutual recursion are shared (SCC optimization), and how error
codes are registered. **It does not repeat the language semantics**—only the mechanism is specified.

## Motivation

### Why a sub-RFC is needed

The criterion (refinement types entering verification mode) and the form (`Terminates` written in
the type position) have been finalized in RFC-027, at the language level. The remaining questions
are too fine-grained and would bloat the host document if included:

- Where do obligations come from (recursive call sites, loop back-edges, path guards)
- How to compare when a measure returns a tuple (lexicographic unfolding)
- Are well-foundedness and strict decrease two independent obligations or a single one
- How automatic exploration and explicit measures coexist in the same pipeline
- How to avoid repeated writing and repeated verification of the same measure for mutual recursion
- How to point in a **direction** rather than just reject when the proof fails

### Trigger

The direction was triggered by #318: non-structural recursion (gcd-style non-directly-decreasing
recursion, mutual recursion, merge partitioning) exceeds the template sequence of measure
exploration (the four strategies in RFC-027 §6.2–6.5 all take "variables of bounded type" or "target
type + swap operation" as input), and cannot be rewritten into analyzable iterative patterns without
destroying readability. The "re-discussion" clause in RFC-027's "Open Questions" section is
activated here.

## Proposal

### Relationship with automatic exploration

The explicit measure is **not a separate pipeline**, but the input after exploration fails. Once the
measure is given, it still goes through the same SMT solver to verify the same set of obligations;
if they don't hold, report an error with a counterexample. Full automation priority is preserved:
exploration always runs first; user intervention happens only after exploration fails.

This decision means both paths share all downstream mechanisms—obligation generation, SMT judgment,
lexicographic unfolding, and diagnostic formatting all have a single implementation.

### Obligation Generation

For function `f` and measure `m`, two **independent** obligations are generated, expanded at each
recursive call site (including cross-function calls within SCC):

1. **Well-foundedness**: `m(args) >= 0`—the measure falls on natural numbers; when the measure
   returns a non-natural-number type, take the lower bound of the well-defined order on that type.
   Derived from argument refinements; when it cannot be derived, it enters a residual obligation.
2. **Strict decrease**: For each call site `m(callee_args) < m(caller_args)`, judged under **path
   guards**—guards come from the branch conditions where the call site is located, reusing RFC-009a
   path condition collection.

The two are independent rather than unified, because the failure directions differ: well-foundedness
failure indicates the measure's value domain is wrong (e.g., `Int` may be negative), while decrease
failure indicates the recursive arguments don't move in the right direction. Diagnostics must
distinguish these to point to the correct checking direction (see the "Diagnostics" section).

**The same applies to loops**, replacing "call site" with "back-edge": for every execution path in
the loop body, `m(next state) < m(current state)`, with guards from the loop condition and branches
within the body.

**Lexicographic unfolding**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation is expanded as a
disjunction chain under lexicographic comparison—`(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`. The
expansion happens on the **obligation generation side**; the SMT side stays in the linear fragment
and does not depend on the solver's native support for lexicographic ordering.

**The measure itself must be compile-time evaluable**: `m` must be a function that can be
constant-folded or proven structurally recursive—prohibited from recursively deferring the
termination problem to another unproven function (to prevent infinite regress). Pathological
measures are caught by the existing E4012 (constant recursion too deep) and structural checks.

### Judgment Pipeline

```
1. Argument structural decrease (strongest path, tried first)
2. Measure exploration: four-strategy template sequence (RFC-027 §6.2–6.5), stop on first success
3. Exploration succeeds → generate obligations → SMT judgment → Proved
4. Exploration fails → check whether the type position provides an explicit measure (Terminates)
     Yes → take that measure to generate obligations → SMT judgment
     No → E4021 (termination cannot be proven, with suggested checking direction)
5. Obligations refuted by SMT → E4022 (measure doesn't hold, with counterexample)
```

Steps 1–3 are the existing path from RFC-027; this RFC adds step 4 and two error codes. The entire
pipeline runs only when refinement types are triggered (RFC-027 §7)—ordinary types without
refinements generate no obligations.

### Anchor: Unary and Binary Forms

RFC-027 §6.9 specifies two arities; this RFC explains where each lands:

| Form                    | Anchor           | Landing Point                                                                                                                        |
| ----------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `Terminates(m)`         | Binding's name   | Default form at definition—self-recursive functions, loops                                                                           |
| `Terminates(FnType, m)` | Explicit fn type | When the measure's owning function type needs to be explicit (measure defined elsewhere, same measure serving multiple computations) |

The two are not two constructs, but two arities of the same predicate: the termination obligation
always lands on "the computation annotated by the type position where the refinement sits." Mutual
recursion **does not need** the binary form—each of the two functions writes its own unary form, and
the sharing relation is identified by SCC (see the next section).

**Implementation status (2026-09-30)**:

- **The unary form is implemented**, with two verified landing points: the function's **return type
  position** (`gcd: (a: Int, b: Int) -> Terminates(b)` → key `gcd`) and a variable's **binding
  position** (`acc: Terminates(n - i) = ...` → key `acc`).
- **The binary form `Terminates(FnType, m)` is parked**. Reason: the implementation surface is
  unary—when `Terminates` enters the type parser, it's processed as a unary predicate (the first
  argument is the measure expression), while the binary form's first argument is a **function
  type**, requiring a separate resolution path for "which function type does this measure belong
  to." There's no current consumer: mutual recursion can be expressed by each function writing a
  unary form plus SCC sharing, and SCC isn't implemented yet, so implementing the binary form now
  has nothing to compare against. Release condition: after SCC is implemented, if "the same measure
  serving multiple computations" is still needed, add the binary parser. The gcd example in this
  document's §Example (binary) is the form awaiting rewriting.

### SCC: Measure Sharing Optimization

For a group of mutually recursive functions (a strongly connected component on the call graph) that
share the same measure, the obligation on cross-function edges is
`m_callee(callee_args) < m_caller(caller_args)`, which reduces to same-measure decrease when members
share the measure.

**This is an optimization, not a correctness prerequisite**: without sharing, each function writes
its own measure and verifies locally, and the proof still goes through. SCC's value is recognizing
that "this group uses the same measure," avoiding repeated writing and repeated verification.

A **function-level call graph and SCC collection** must be built—the existing `TypeDepGraph` records
variable-to-variable type annotation dependencies (the VC trigger in RFC-027 §6.1), which is a
variable-level graph and cannot be reused.

### Examples

#### gcd: Non-Structural Recursion

```yaoxiang
// Measure: a regular function, unit-testable and reusable
gcd_measure: (a: Int, b: Int) -> Int = { b }

gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

Obligation generation:

1. **Well-foundedness**: `gcd_measure(a, b) >= 0`, i.e., `b >= 0`—derived directly from the argument
   refinement `NonNegative(b)`
2. **Strict decrease**: the only recursive call site `gcd(b, a % b)`, path guard `b != 0`,
   obligation `gcd_measure(b, a % b) < gcd_measure(a, b)`; substituting the measure body yields
   `a % b < b`
3. **SMT**: verify that the negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved

#### Loops: Anonymous Constructs Get Designation

```yaoxiang
loop: (n: Int) -> Int = {
    mut i: Int = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

`Terminates(m)` refines the type of the value of the loop body tail expression; the anchor is
provided by the binding name `acc`—the loop can therefore **be designated**, eliminating the dead
end of "anonymous constructs cannot be designated." This also explains the unity of the two arities:
the obligation always lands on the computation annotated by the type position where the refinement
sits; functions and loops are not distinguished.

**Note the body tail of the loop**: the value of `while` is the value of the loop body block (spec
§2.9); if the body tail is written as an assignment statement `i = i + 1`, the block value is
`Void`, and `return acc` from `-> Int` returning `Void` is not legal. The example above adds a tail
expression `i` after the assignment to give a value—**this is the only correct form for this
example; this document does not give the incorrect pre-fix form** (pre-fix form and fix history see
"Implementation status" below).

Obligation: on the back-edge `(n - i') < (n - i)`, with guard `i < n`, substituting `i' = i + 1`
gives `1 > 0`, which is a tautology → Proved.

**Implementation status (logged 2026-10-01; revised 2026-10-02, 2026-10-04): both the type-side and
runtime-side of this loop form are working; the zero-iteration value case is still a known residual
(see "Known Defect D6" below)**.

This presumes that `while` has value semantics: this form was once blocked (D5)—at the time, the
`while` expression was **fixed** as type `Void`, with no value type to carry a refinement, and
`Terminates` couldn't attach to any computation. That defect has been fixed: per spec §2.9, "the
value of every `{}` block is given by its tail expression, without exception," and a loop body is
also a `{}` block, so the value of `while` equals the value of the loop body block.

This rule also exposed a **type error in an earlier version of this example**: the pre-fix form
wrote the loop body tail as an assignment statement (e.g., `while i < n { i = i + 1 }`), making the
block value `Void` (same rule in spec §2.9), so `return acc` from `-> Int` returning `Void` is not
legal. The fix is to let the body tail provide a value: add a tail expression `i` after the
assignment—the example above is already the corrected form; this document no longer retains a
copyable incorrect form.

The back-edge obligation is **isomorphic** to the recursive call back-edge
(`m[var := back-edge value] < m`), and the loop condition is the source of its well-foundedness
lower bound—`i < n` derives `n - i > 0`. Verified: `n - i` is judged Proved—this is a **type-side**
conclusion. On the runtime side, this form **returns `4`**: the binding receives the body block
value of the last iteration (#409 fixed, verified 2026-10-04). The "returns 4 when `n = 4`"
previously written in this document was at the time an **unreproduced false conclusion** (deleted
2026-10-02); that conclusion holds after the fix for issue #409, so it is restored as a reproduced
conclusion.

What has been implemented is the **function form** (return-type-position unary measure, see §Example
gcd) and the **loop form**, two paths; the loop side's existing refined-variable gating + E4021 is
preserved (the exploration path when no explicit measure is given).

**Known Defect D6 (runtime `while` value-taking; logged 2026-10-02, fixed 2026-10-04)**. Type-side
standing used to not equal runtime usability: the above example `check`ed 0 errors, but at
**runtime** the value the binding received was `Void`, and any use of that value would crash at
runtime. **This defect (#409) has been fixed**: at runtime, the value of `while` (and the isomorphic
`for`) is changed to the value of the tail expression of the body block in the last iteration,
matching the type-side `block_value_ty`. The above example `check`s 0 errors and `run` exits 0.

Minimum reproduction (the loop form in this document's §Example with an explicit `main()` call at
the end—script-mode `main` isn't implicitly executed). The following code and output are **pre-fix**
historical records, retained as defect evidence; with the fix, the same code `run` exits 0 with no
output:

```yaoxiang
use std.assert

loop: (n: Int) -> Int = {
    mut i: Int = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}

main: () -> Void = {
    assert.assert(loop(4) == 4, "loop value should be 4")
}

main()
```

Verified (this worktree's binary; baseline `2a038f93` reproduces the same):

```
error [E6007] Runtime error: type mismatch in comparison Eq: Void vs Int(4)
 --> loop_runtime.yx:14:5
```

- **Independent of iteration count** (pre-fix): `loop(0)` (zero iterations) reports
  `Eq: Void vs Int(0)`—the runtime `while` expression never produced a value, not "the last
  iteration's value is wrong."
- **Impact extends beyond the `Terminates` form** (pre-fix): any use of `while` as a value is
  affected. Verified: `x: Int = while i < 3 { i = i + 1; i }` and then using `x` at runtime →
  `Eq: Void vs Int(3)`. The type-side criterion, obligation generation, and diagnostics are all
  normal; the defect is only on the runtime value-taking path.
- **Fix implemented (2026-10-04, issue #409)**: `src/middle/core/ir_gen.rs`'s
  `generate_while_expr_ir` / `generate_iterator_for_loop_ir`—before the loop, write a `Void` initial
  value to the value register; the tail expression generated by the loop body writes directly to
  that register; and remove the unconditional `Void`-overwriting instruction at the exit. Verified:
  `loop(4) == 4`, `countdown(3) == 30`, `x: Int = while i < 3 { i = i + 1; i }` (`x == 3`), and the
  **main form of the zero-iteration case is all green**.
- **Known residual (zero-iteration)**: when the loop body never executes (e.g., `loop(0)`), there is
  no "body value of the last iteration," and the value register keeps the initial `Void`—binding to
  a non-`Void` type position and using it at runtime reports `E6007` type mismatch (loud failure,
  not silent wrong value). This is the boundary of static approximation: the type-side
  `block_value_ty` gives the tail expression's type and has no judgment ability for "whether the
  loop executes at least once"; downgrading it to `Void` would break the value type the `Terminates`
  loop binding in this section depends on, so it is **not** downgraded. The residual contract is
  pinned by `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`
  (`// expect: runtime-error E6007`).
- **Release condition (historical, completed)**: at runtime, produce the value of the `while`
  expression as the value of the tail expression of the loop body block, and let
  `tests/yaoxiang/02-type-system/explicit_termination_measure_loop.yx` and `while_block_value.yx`
  pass after adding `main()` calls. Both corpora have now had `main()` added and run successfully
  (the zero-iteration assertion in `explicit_termination_measure_loop.yx` is changed to the
  non-zero-iteration `loop(1) == 1`; the zero-iteration case is handed off to the new test case in
  the previous item).
- **Tracking**: issue [#409](https://github.com/ChenXu233/YaoXiang/issues/409) (runtime `while`
  value-taking)—main form is fixed, zero-iteration residual is tracked in that issue; this section
  retains the minimum reproduction, fix record, and residual boundary (en mirror is handled by the
  auto-translation bot, no manual edits).

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

Both functions carry the unary form `Terminates(nat)`. After SCC collection, the same shared measure
is identified, and the cross-function edge obligation `nat(n - 1) < nat(n)` is a tautology under the
guard `n != 0`; both functions close the loop in a single verification.

Without SCC identification, each function verifying the same obligation still passes—it's just
duplicated once. This confirms SCC's positioning as an optimization.

### Diagnostics

**When well-foundedness cannot be derived, don't reject directly—suggest a checking direction.**
This must be distinguished from the diagnostic for a decrease obligation failure: the former points
to the measure's value domain, the latter to the recursive arguments. The suggestion for each of
three failure types:

| Failure                            | Suggested Direction                                                                                       |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Well-foundedness cannot be derived | Whether the measure has a lower bound over its possible values (e.g., does an `Int` measure need `>= 0`?) |
| Strict decrease refuted            | Whether recursive arguments really move in the decreasing direction; attach SMT counterexample            |
| No measure and exploration fails   | Suggest binding a name to the computation and providing a measure in the type position (RFC-027 §6.9)     |

Counterexample presentation follows the RFC-013 diagnostic message specification. Under non-linear
path guards, Sat counterexamples may not be intuitive—logged as a known limitation, iterated along
with RFC-013.

### Error Codes

Aligned with the E4xxx proof failure family (E4018 refinement predicate violation, E4020 proof
function needed):

| Proposed Code | Name                         | Trigger                                                                                                     |
| ------------- | ---------------------------- | ----------------------------------------------------------------------------------------------------------- |
| E4021         | Termination cannot be proven | Exploration failed and the type position has no explicit measure (suggest providing a `Terminates` measure) |
| E4022         | Measure doesn't hold         | Measure obligations refuted by SMT (with counterexample)                                                    |

Final numbering follows the actual RFC-013 registry at implementation time (segment legality is
guaranteed by the build.rs threshold).

### Existing Defects in the Diagnostic Layer

In the current implementation, in-scope termination failures are reported as **E8001 "internal
compiler error"** (`Unproven` is formatted as an ICE)—termination checking is not an ICE, and
occupying that code slot both misleads users and masks real failures. This RFC fixes that: in-scope
termination failures go to E4021/E4022, and the ICE code slot is returned to genuine internal
errors.

### Compiler Changes

| Component                           | Changes                                                                                                                                                                                                                                   |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`   | Unified interface (exploration path and explicit measure path share obligation generation and judgment); connect to Z3 (production injects `default_solver()` in `checker.rs`; wasm does not inject; unit tests can inject a stub solver) |
| Function call graph + SCC (**new**) | No function-level call graph anywhere—`TypeDepGraph` is variable-level type dependencies and cannot be reused. Build a function-level call graph and SCC collection for mutual-recursion measure sharing                                  |
| Obligation generation               | New: well-foundedness / strict decrease two obligations, path guard injection, lexicographic unfolding                                                                                                                                    |
| Proof pipeline (RFC-009a / #292)    | Reuse the `ConstExpr → SMTLib` chain and `Mod` and other operator mappings, no backend changes                                                                                                                                            |
| `util/diagnostic/codes/e4xxx.rs`    | Register E4021/E4022 (via RFC-013 registry, build.rs threshold takes effect)                                                                                                                                                              |
| locales ×6                          | Six-language templates for the two new codes                                                                                                                                                                                              |
| Diagnostic layer                    | Move in-scope termination failures out of E8001, into the proof failure family                                                                                                                                                            |

### Backward Compatibility

- Programs previously rejected by termination checking but **without refinement annotations**: under
  the new criterion, they don't enter verification mode and compile directly through—an intentional
  relaxation, only loosening, not tightening.
- Programs previously rejected by termination checking **with refinement annotations**: pass after
  adding the measure.
- Structural recursion positive cases and existing termination tests: expected output unchanged.

## Trade-offs

### Advantages

- **Zero new syntax**: the measure is written in the type position, reusing the refinement predicate
  application mechanism; no `decreases`-style syntax position.
- **Proof-domain behavior alignment**: the termination domain adds a fallback channel consistent
  with the correctness domain, but with a different landing point—propositions that can't be proven
  in the correctness domain are written as proof functions in the **body**, while measures that
  can't be explored in the termination domain are declared in the **type position**. The mechanism
  is the same (both refinement type applications).
- **Reuse infrastructure**: obligations are linear arithmetic plus path guards, going through the
  already-connected #292 pipeline, no new backend.
- **Loops gain designation**: the binding name is the anchor, and loops are fully isomorphic to
  functions in obligation generation, no special case needed for loops.

### Disadvantages and Risks

- **Upfront infrastructure is no smaller than "wiring"**: function-level call graph and SCC
  collection need to be built from scratch, and Z3 isn't yet connected to the production pipeline.
  The SCC part can be deferred (it's an optimization); the call graph part shares the same source as
  SCC, so deferring it means mutual recursion can only write each function's own measure—still
  passable, just repeated verification.
- **Explicit measure is a writing burden for a low-frequency path**: the two-piece set (measure
  function + type position declaration) is more verbose than inline annotations. Accepted—the
  fallback is a low-frequency path, and in return the measure is reusable and unit-testable.
- **Noise from the well-foundedness obligation**: when the measure returns `Int`, `>= 0` has to be
  proven every time. Mitigation: if the argument refinement already provides a lower bound, it's
  derived automatically; only when it can't be derived does it enter a residual obligation, and even
  then it only gives a direction rather than rejecting.
- **Counterexample quality**: Sat counterexamples under non-linear guards are not intuitive. Known
  limitation.
- **`Terminates` is the only predicate whose body the compiler writes for the user**: deviates from
  the purity of "all predicate bodies are user-writable." Reason: its assertion (that the measure
  decreases at every call site / back-edge) lives in the computation structure, which user-written
  predicates cannot reference. The built-in surface converges to a single name, with zero new
  mechanism.

## Alternatives

- **Inline annotation syntax `with decreases (b)`**: an early proposal in this issue, withdrawn. "No
  annotation syntax entry for termination checking" is a finalized decision in RFC-027; inline
  annotations would turn unsupported termination modes into syntax positions rather than type
  positions, going against the worldview of "everything is a YaoXiang function, everything is
  verified by the type checker."
- **Separate proof function (`gcd_proof` returns `Terminates(f, m)`, discovered by scanning return
  types)**: an early design. Abandoned—it introduces four classes of complexity: "discovery
  mechanism," "naming convention," "picking the first of multiple candidates," and "how names inside
  a proof function are resolved," all because the proof is placed outside. Moving it into the type
  position makes all four classes of complexity vanish.
- **Only let the measure enter the type (`Terminates(m)`), drop the binary form**: shorter, but
  loses the expression position for "explicitly specifying measure ownership" (when the measure is
  defined elsewhere, or the same measure serves multiple computations, there's nowhere to place it).
  The two arities are two arities of the same predicate, with near-zero cost to keep both.
- **No fallback, require the user to rewrite into analyzable iterative patterns**: the status quo.
  gcd / merge partitioning / mutual recursion cannot be rewritten without destroying
  readability—this is exactly what triggers the re-discussion clause.

## Non-Goals

- No new syntax / keywords / annotation positions.
- No generalized automatic measure synthesis (the four strategies stay within RFC-027's plan,
  anything beyond falls to explicit measure). Generalized measure inference lies on the undecidable
  side of "discovery"; only template boundaries can be agreed upon, completeness cannot be promised.
- No joint solving with RFC-009a borrowing propositions (each judged independently, sharing
  backend).
- No totality checking at the dependent type level.
- No forcing the measure to return a natural number type (not pursuing Lean's `WellFoundedRelation`
  typeclass mechanism)—the measure's return type is unrestricted; well-foundedness is left to
  refinement derivation or SMT as an independent obligation.

## Phases and Acceptance

- [x] Obligation generation: well-foundedness / strict decrease, path guard injection
- [x] Explicit measure wiring: unary form anchor resolution (return type position / binding
      position) + type position measure extraction
- [x] SMT judgment wiring (reusing the #292 pipeline, Z3 injected into the production
      pipeline—previous injection points were never called)
- [ ] Function-level call graph + SCC collection (measure sharing optimization). Release condition:
      a need arises for "the same measure serving multiple computations" or mutual recursion
      avoiding repeated verification
- [ ] Lexicographic unfolding (when the measure returns a tuple). Release condition: a real use case
      with a tuple-returning measure
- [x] E4021/E4022 registration + six-language locales (consistent across three parties: codes ↔
      locales ↔ RFC-013 code table)
- [x] Diagnostic layer fix: in-scope termination failures moved out of E8001, into the proof failure
      family. E4021 already in place; E4022 goes through `DisproofKind::MeasureNotDecreasing` into
      the user domain, with use cases pinning it from being downgraded to ICE
- [x] E2E positive cases: gcd (including the "residual obligation doesn't reject" form), structural
      recursion zero regression
- [x] E2E negative cases: measure doesn't hold → E4022 (with counterexample); no measure → E4021
- [x] E2E positive case (loop, type-side): the type judgment of the `Terminates(n - i)` binding form
      holds (D5 resolved)
- [x] E2E positive case (loop, runtime): runtime value-taking of `while` as a value (D6 fixed, #409,
      2026-10-04)—`explicit_termination_measure_loop.yx` and `while_block_value.yx` pass after
      adding `main()`
- [ ] E2E residual (zero-iteration): when `while` is used as a value and the loop body executes zero
      times, the value is `Void` (static approximation boundary). Contract test case
      `02-type-system/while_zero_iteration_void_err.yx` (`// expect: runtime-error E6007`); release
      condition see §Example loop section "Known Residual"
- [ ] E2E pending: `is_even`-`is_odd` mutual recursion (awaiting SCC)
- [ ] Binary form `Terminates(FnType, m)` (parked). Release condition see §Anchor
- [ ] Return-position multi-argument predicate + symbolic arguments (clear diagnostics since
      2026-10-02, still not supported). Trigger form / alternative writing / release condition see
      §Implementation Landing Record (2026-10-02, this round's hardening)
- [x] Criterion regression: recursion and loops without refinement annotations are no longer
      rejected by termination checking; with refinement annotations, obligations are triggered as
      usual
- [x] Acceptance demo: write a declaration whose measure doesn't hold → compile fails and the
      counterexample is readable (`measure_not_decreasing_err.yx`); after correction it passes
      (`measure_decreasing.yx`)
- [x] Return-point refinement (2026-10-02): return-position formal parameters are recognized by
      **declaration name**, and predicate arguments are really substituted into constraints;
      undeclared free variables report E1001
- [x] Termination strategy 1 back in effect (2026-10-02): candidates are no longer emptied by
      boundary filtering, measure change direction is derived by Direction; new flag loop (strategy
      1b)
- [x] Documentation and implementation alignment (2026-10-02): the first loop example and landing
      records in this document, two examples in `language-spec/type-system.md`, the `§2.15` → `§2.9`
      reference fix, and the RFC-011 loop example

### Implementation Landing Record (2026-09-30 ~ 2026-10-01)

Implemented surface: unary `Terminates` enters the type parser → AST extracts the measure (**not**
through the lossy `MonoType` conversion, which would degrade the measure to `Int(64)`) → obligation
generation (recursive call sites, loop back-edges, with path guards) → well-foundedness and strict
decrease two-path SMT judgment → diagnostic emission.

The two **gates** for issuing E4022 (both only under-report, never over-report):

1. **Refuted, not just unproven**—the solver gives a counterexample (`Sat`) before reporting;
   `Unknown` means "can't decide" and is handled per the §Trade-offs section's "only give a
   direction, don't reject."
2. **Measure well-foundedness already proven**—integer `<` is **not** well-founded, so the measure
   not decreasing does **not** equal non-termination (e.g., `gcd` still terminates for negative
   `b`). When well-foundedness is unproven, even if decrease is refuted, don't report; enter a
   residual obligation.

These two gates reconcile "only give a direction, don't reject" and "refuted means error" in
§Trade-offs: well-foundedness is that "no decrease means no termination" license.

Existing defects fixed in passing during implementation (all are prerequisites for this RFC):

- **Predicate definition not registered** (#377-3): the predicate body substitution in
  `PredicateResolver` had long been implemented, but was short-circuited by an empty table. After
  registering, refinement constraints can be symbolically reasoned for the first time—the premise of
  well-foundedness `b >= 0`.
- **`while` has no value semantics** (D5): see §Example loop section.
- **evaluator missing `Neg`/`Pos`/`BitNot`**: inconsistent with the `const_eval` implementation of
  the same name, causing negative literals like `-5` to fail to fold into a constant on all paths
  that go through this evaluator.

### Implementation Landing Record (2026-10-02, this round's hardening)

Four tracks of work, each one-to-one with actual changes; each item below is a reproducible verified
conclusion (commands and outputs given per item), not planned scope.

#### 1. Return-Point Refinement: Formal Parameters Recognized by Declaration Name + Predicate Arguments Really Substituted into Constraints

Pre-fix (baseline `2a038f93`) had three deviations: the predicate argument of `-> (r: P(r + 100))`
was discarded, degrading to `P(r)`; the undefined `m` in `-> (r: Eq2(m, r))` was silently
substituted into the return value, making the constraint degrade to a tautology. Fix: the parser
preserves single-named parentheses as `NamedParen` (carrying the declared name), the type checker
recognizes **only that name** as the return-position formal parameter; other out-of-scope free
variables are reported one by one as E1001 and the return-point obligation is abandoned. Verified:

| Test case                                                | Pre-fix                                 | Post-fix (verified)                                                     |
| -------------------------------------------------------- | --------------------------------------- | ----------------------------------------------------------------------- |
| `d1: () -> (r: IsPositive(r + 100)) = { return -5 }`     | E4018 (constraint computed as `-5 > 0`) | `0 error` (constraint `-5 + 100 > 0` holds)                             |
| `d2: () -> (r: IsPositive(r - 100)) = { return 5 }`      | Silently passes                         | `error [E4018]` (constraint `5 - 100 > 0` refuted, with counterexample) |
| `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` undefined) | Silently passes                         | `error [E1001] Unknown variable: 'm'`                                   |

#### 2. Return-Position Multi-Argument Predicate + Symbolic Arguments: Clear Diagnostics Given (Known Gap, Not Supported)

- **Trigger form** (minimal example):

  ```yaoxiang
  SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }
  f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }
  ```

  Verified diagnostic:
  `error [E2031] After assigning to 'f', the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven in the proof kernel`—the
  object is the **return position** (`'r'`), not the previously mis-attached formal parameter name
  `'b'`; the constraint text gives the form after argument substitution.

- **Cause**: the proof kernel only executes proof function calls where "all arguments can be taken
  as values (literals / already-bound variables)"; a multi-argument predicate with variables can't
  get a call in the return position, and when it can't be judged it reports.
- **Alternative writing** (the diagnostic's help gives both): change the arguments to
  compile-time-foldable literals (e.g., `SumUpTo(3, r)`), or provide a proof function returning
  `Type` for that predicate to participate in compile-time execution.
- **Release condition**: the proof kernel supports argument substitution for "multi-argument
  predicate + symbolic arguments" (the same substitution path as the previous item).
- **Tracking**: no new issue opened—this section is the sole tracking location.

#### 3. Termination Strategy 1 Back in Effect (defects a / b of #377)

- **Defect a**: the boundary inputs of strategy 1 were filtered out by "the boundary must be a loop
  invariant" → candidates were always zero. Fix: strategy 1 now takes the **unfiltered** boundary
  set, soundness is borne by per-assignment verification.
- **Defect b**: the change direction of the candidate measure was written wrong (`m'` was always
  constructed as `v + 1`) → judgment always failed. Fix: the direction is determined by the boundary
  operator (`Lt`/`Le` ⇒ Increasing, `Gt`/`Ge` ⇒ Decreasing), the measure change is derived by
  Direction (Increasing `Δm = δbound - δv`; Decreasing `Δm = δv - δbound`), `Δm >= 0` is directly
  rejected on the synthesis side and doesn't go to the solver.
- **New flag loop (strategy 1b)**: bare boolean condition + clear flag within threshold guard +
  variable moves unconditionally toward the threshold each round, the measure is "distance to the
  threshold."
- **Soundness standard unchanged**: better to report "cannot be proven" than to misjudge
  termination—the measure variable's advance must be **unconditional** (at the top level of the
  body), the boundary displacement takes the **worst-side** including the guard, an unknown
  displacement is rejected; rebinding the measure variable inside a guarded block is also rejected.

Verified (this worktree's binary):

| Corpus                                                                                 | Verified              |
| -------------------------------------------------------------------------------------- | --------------------- |
| `02-type-system/rank_function_dual_variable_loop.yx` (`i < j { i += 1; j -= 1 }`)      | `0 error`             |
| `02-type-system/flag_loop_guarded_exit.yx`                                             | `0 error`             |
| `06-compile-errors/termination_unproven_err.yx`                                        | Still `error [E4021]` |
| 4 negative-example corpora (rebind in guard block / unknown-displacement rebind, etc.) | All `error [E4021]`   |

#### 4. Documentation and Implementation Alignment

The first loop example and this landing record in this document, the two loop examples in
`docs/src/reference/language-spec/type-system.md`, the loop example in RFC-011 §Termination
Checking, and the `§2.15` → `§2.9` reference correction—details in each document's diff (`§2.15` is
"Scope," the block value rule is in `§2.9` Block Expressions).

#### 5. Not Yet Implemented After This Round

SCC collection, lexicographic unfolding, binary form `Terminates(FnType, m)`, mutual-recursion E2E,
the **zero-iteration residual** of runtime `while` value-taking (D6 main form is fixed),
return-position multi-argument predicate + symbolic arguments—all remain in the unchecked items of
§Phases and Acceptance, with release conditions noted per item.

## Related

- #318 (the issue for this RFC), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement type criterion, §6.1–6.5 measure exploration four strategies, §6.9
  explicit measure, §Open Questions re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path condition collection—infrastructure prerequisite)
- RFC-013 (error code registry and proof failure semantic family)
