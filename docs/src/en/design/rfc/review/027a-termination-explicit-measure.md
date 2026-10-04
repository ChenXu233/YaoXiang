---
title: 'RFC-027a: Explicit Measures for Termination Checking'
status: 'Under Review'
author: 'Chenxu'
created: '2026-09-14'
updated: '2026-10-02'
issue: '#318'
impl_status: 'partial'
---

# RFC-027a: Explicit Measures for Termination Checking

## Summary

RFC-027 §7 establishes the termination checking criterion as **refinement types** (refinement enters
verification mode), while §6.9 gives the linguistic form of explicit measures (the built-in
predicate `Terminates`, alongside `Int` and `Never` as core primitives). Both are language design
decisions, belong to the host RFC, and are finalized.

This sub-RFC addresses the landing mechanism: how obligations are generated from computational
structure, how the judgment pipeline is orchestrated, how diagnostics give direction when the proof
cannot be pushed through, how measures are shared in mutual recursion (SCC optimization), and how
error codes are registered. **It does not re-state language semantics**, only the mechanism.

## Motivation

### Why a sub-RFC is needed

The criterion (refinement types entering verification mode) and the form (`Terminates` written in
the type position) are already settled in RFC-027, at language-level granularity. The remaining
issues are too fine-grained to fit in the host document without bloating the main text:

- Where obligations come from (recursive call sites, loop back-edges, path guards)
- How to compare when a measure returns a tuple (lexicographic unfolding)
- Whether well-foundedness and strict decrease are two independent obligations or one unified one
- How automatic exploration and explicit measures coexist in the same pipeline
- How mutual recursion shares the same measure without duplicate writing and duplicate verification
- How to give **direction** rather than outright rejection when the proof fails

### Trigger

The direction was triggered by #318: non-structural recursion (gcd-like non-directly decreasing,
mutual recursion, merge partitioning) exceeds the template sequence of measure exploration (RFC-027
§6.2–6.5's four strategies all take "bounded-typed variable" or "target type + swap operation" as
input), and cannot be rewritten into an analyzable iteration pattern without sacrificing
readability. RFC-027's "Open Problems" section's re-discussion clause is activated here.

## Proposal

### Relationship with automatic exploration

Explicit measures are **not a separate pipeline**, but the input after exploration fails. Once a
measure is given, it still flows through the same SMT verification of the same set of obligations;
if it doesn't hold, an error is reported with a counterexample. Full automation priority is
unchanged: exploration always runs first, and user intervention only happens after exploration
fails.

This decision means the two paths share all downstream mechanisms—obligation generation, SMT
judgment, lexicographic unfolding, and diagnostic format each have only one implementation.

### Obligation Generation

For function `f` and measure `m`, two **independent** obligations are generated, unfolded at every
recursive call site (including cross-function calls within an SCC):

1. **Well-foundedness**: `m(args) >= 0`—the measure lands on natural numbers; when the measure
   returns a non-natural-number type, the lower bound of the well-ordering on that type is taken.
   Derived from parameter refinements; falls into residual obligations when it cannot be derived.
2. **Strict decrease**: at every call site `m(callee_args) < m(caller_args)`, judged under **path
   guards**—the guards come from the branch conditions at that call site, reusing RFC-009a's path
   condition collection.

The two are independent rather than unified, because the failure directions differ: a
well-foundedness failure means the measure's value range is wrong (e.g., `Int` may be negative),
while a decrease failure means the recursive parameter isn't moving in that direction. Diagnostics
must distinguish these to give the right checking direction (see "Diagnostics" section).

**Loops follow the same rule**, swapping "call site" for "back-edge": on every execution path of the
loop body, `m(next-round state) < m(current-round state)`, with guards coming from the loop
condition and branches within the body.

**Lexicographic unfolding**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation's lexicographic
comparison unfolds into a disjunction chain—`(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`. The unfolding
is done on the **obligation generation side**; the SMT side stays in the linear fragment, not
relying on the solver's native support for lexicographic ordering.

**The measure itself must be compile-time evaluable**: `m` must be a function provable by constant
folding or structural recursion—it's forbidden to recurse the termination problem back to another
unproven function (to prevent infinite regression). Pathological measures are caught by the existing
E4012 (constant recursion too deep) and structural checks.

### Judgment Pipeline

```
1. Parameter decrease (structural recursion, strongest path, try first)
2. Measure exploration: four-strategy template sequence (RFC-027 §6.2–6.5), stop on first hit
3. Exploration succeeded → generate obligations → SMT judgment → Proved
4. Exploration failed → check whether an explicit measure is given in the type position (Terminates)
     Yes → take that measure to generate obligations → SMT judgment
     No → E4021 (termination unprovable, with suggested checking direction)
5. Obligation judged false by SMT → E4022 (measure does not hold, with counterexample)
```

Steps 1–3 are RFC-027's established path; this RFC adds step 4 and the two error codes. The entire
pipeline only runs when refinement types trigger it (RFC-027 §7)—ordinary types without refinements
generate no obligations at all.

### Anchor: Unary and Binary Forms

RFC-027 §6.9 defines two arities; this RFC explains their respective landing points:

| Form                    | Anchor                 | Landing Point                                                                                                                                   |
| ----------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The binding's name     | Default form at definition—self-recursive functions, loops                                                                                      |
| `Terminates(FnType, m)` | Explicit function type | When the measure needs to be explicitly assigned to a function type (measure defined elsewhere, the same measure serving multiple computations) |

The two are not two constructs but two arities of the same predicate: the termination obligation
always lands on "the computation annotated at the type position where the refinement sits." Mutual
recursion does **not** need the binary form—each of the two functions can write its own unary form,
and the sharing relation is identified by SCC (see next section).

**Landing status (2026-09-30)**:

- **The unary form has landed**, with measured landing points in two places: the function **return
  type position** (`gcd: (a: Int, b: Int) -> Terminates(b)` → key `gcd`) and the variable **binding
  position** (`acc: Terminates(n - i) = ...` → key `acc`).
- **The binary form `Terminates(FnType, m)` is parked**. The reason: the implementation surface is
  unary—when `Terminates` enters the type parser it's handled as a unary predicate (first argument
  is the measure expression), but the binary form's first argument is a **function type**, requiring
  an independent parsing path for "which function type the measure belongs to." There are currently
  no consumers: mutual recursion can be expressed using each function's unary form + SCC sharing,
  and SCC has not yet landed, so doing the binary form first would have no comparable consumer.
  Condition to un-park: after SCC lands, if "the same measure serving multiple computations" is
  still needed, then add the binary parsing. The gcd example (binary) in §Example of this document
  is the form to be rewritten.

### SCC: Measure Sharing Optimization

For a set of mutually recursive functions (strong connected components in the call graph) sharing
the same measure, the cross-function edge obligation is
`m_callee(callee_args) < m_caller(caller_args)`, and when members share the measure it degenerates
to a same-measure decrease.

**This is an optimization, not a correctness prerequisite**: when not sharing, each function writes
its own measure and closes its own loop to pass. SCC's value is recognizing that "this group uses
the same measure," avoiding duplicate writing and duplicate verification.

This requires a new **function-level call graph and SCC collection**—the existing `TypeDepGraph`
records type-annotation dependencies between variables (the VC trigger from RFC-027 §6.1), which is
a variable-level graph and cannot be reused.

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

1. **Well-foundedness**: `gcd_measure(a, b) >= 0` i.e. `b >= 0`—derived directly from parameter
   refinement `NonNegative(b)`
2. **Strict decrease**: the only recursive call site is `gcd(b, a % b)`, with path guard `b != 0`,
   obligation `gcd_measure(b, a % b) < gcd_measure(a, b)`; expanding by substituting the measure
   body yields `a % b < b`
3. **SMT**: verify the negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved

#### Loop: Anonymous Construct Gets a Designation

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

The `Terminates(m)` refinement is on the value type of the loop body's tail expression, with the
binding name `acc` providing the anchor—the loop therefore **can be designated**, no longer having a
dead corner of "anonymous constructs without designation." This also explains the unity of the two
arities: the obligation always lands on the computation annotated at the type position where the
refinement sits, with no distinction between functions and loops.

**Note on the loop body's tail**: the value of `while` is the value of the loop body block (spec
§2.9); if the body tail is written as an assignment statement `i = i + 1`, the block value is
`Void`, and `return acc` returning `Void` from `-> Int` is not legal. The example above appends a
tail expression `i` after the assignment to give that value—**this is the only correct form for this
example; this document does not give the pre-fix erroneous form** (the pre-fix form and fix records
are in the "Landing Status" section below).

Obligation: on the back-edge `(n - i') < (n - i)`, with guard `i < n`, substituting `i' = i + 1`
gives `1 > 0`, always true → Proved.

**Landing status (registered 2026-10-01; revised 2026-10-02, 2026-10-04): the type-side and
runtime-side of this loop form are both working; the zero-iteration value is a known remaining issue
(see "Known Defect D6" below)**.

The prerequisite is that `while` has value semantics: this form was once blocked (D5)—the `while`
expression at that time was **always** typed as `Void`, with no value type to carry a refinement,
and `Terminates` couldn't attach to any computation. That defect has been fixed: per spec §2.9 "all
`{}` block values are given by the tail expression, without exception," the loop body is also a `{}`
block, so the value of `while` = the value of the loop body block.

That rule simultaneously exposed an **early type error in this example**: the pre-fix form wrote the
loop body tail as an assignment statement (e.g. `while i < n { i = i + 1 }`), making the block value
`Void` (spec §2.9 same rule), and `return acc` returning `Void` from `-> Int` is not legal. The fix
is to make the body tail give a value: append a tail expression `i` after the assignment—the example
above is already the corrected form, and this document no longer retains a copyable erroneous form.

The back-edge obligation and the recursive back-edge are **isomorphic**
(`m[var := back-edge value] < m`), and the loop condition is the source of the well-foundedness
lower bound—`i < n` derives `n - i > 0`. Measured: `n - i` is judged Proved—this is a **type-side**
conclusion. At runtime this form **returns `4`**: the binding receives the body block value of the
last iteration (fixed in #409, measured 2026-10-04). The "returns 4 when `n = 4`" written previously
in this document was then an **unreproduced false conclusion** (deleted 2026-10-02); that conclusion
became reproducible after issue #409 was fixed, so it's restored to a reproduced conclusion.

What has landed is the **function form** (return type position with unary measure, see §Example gcd)
and the **loop form**, both paths; the loop side's original refinement variable gating + E4021 are
retained (the exploration path when no explicit measure is given).

**Known Defect D6 (runtime `while` value taking; registered 2026-10-02, fixed 2026-10-04)**.
Type-side being sound was not the same as runtime being usable: the example above `check` 0 error,
but **at runtime** the binding received `Void`, and any use of that value would crash at runtime.
**That defect (#409) has been fixed**: the runtime value of `while` (and the isomorphic `for`) was
changed to the value of the body block's tail expression on the last iteration, with the same
caliber as the type-side `block_value_ty`. The example above `check` 0 error and `run` exits 0.

Minimum reproduction (the loop form from §Example + an explicit `main()` call at the end—script
mode's `main` doesn't run implicitly). The following code and output are **pre-fix** historical
records, retained as defect evidence; after the fix the same code `run` exits 0 with no output:

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

Measured (this worktree's binary; baseline `2a038f93` reproduces the same):

```
error [E6007] Runtime error: type mismatch in comparison Eq: Void vs Int(4)
 --> loop_runtime.yx:14:5
```

- **Unrelated to iteration count** (pre-fix): `loop(0)` (zero iterations) reports
  `Eq: Void vs Int(0)`—the runtime `while` expression simply didn't produce a value, not "the last
  iteration took the wrong value."
- **Impact is not limited to the `Terminates` form** (pre-fix): any `while` as a value hits it.
  Measured `x: Int = while i < 3 { i = i + 1; i }` and then using `x` at runtime →
  `Eq: Void vs Int(3)`. Type-side criteria, obligation generation, and diagnostics were all normal;
  the defect is only in the runtime value path.
- **Landed fix (2026-10-04, issue #409)**: `src/middle/core/ir_gen.rs`'s `generate_while_expr_ir` /
  `generate_iterator_for_loop_ir`—before the loop write the initial value `Void` into the value
  register; the tail expression generated by the loop body writes directly to that register; delete
  the unconditional `Void`-overwriting instruction at the exit. Measured `loop(4) == 4`,
  `countdown(3) == 30`, `x: Int = while i < 3 { i = i + 1; i }` (`x == 3`), zero-iteration use
  case's **main form all green**.
- **Known remainder (zero iteration)**: when the loop body never executes even once (e.g.
  `loop(0)`), there's no "last iteration body value" and the value register keeps the initial
  `Void`—binding to a non-`Void` type position and using it at runtime reports `E6007` type mismatch
  (loud failure, not silent wrong value). This is the static approximation boundary: the type-side
  `block_value_ty` gives the type of the body tail expression and has no ability to judge "whether
  the loop executes at least once"; downgrading it to `Void` would break the value type the
  `Terminates` loop binding here depends on, so it is **not** downgraded. The remainder contract is
  pinned by `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`
  (`// expect: runtime-error E6007`).
- **Condition to lift (historical, completed)**: at runtime have `while` expression's value come out
  as the value of the loop body block's tail expression, and make
  `tests/yaoxiang/02-type-system/explicit_termination_measure_loop.yx` and `while_block_value.yx`
  pass after adding `main()` calls. The two corpora now have `main()` added and pass actual runs
  (`explicit_termination_measure_loop.yx`'s zero-iteration assertion changed to non-zero-iteration
  `loop(1) == 1`, the zero-iteration case handed off to the new use case from the previous item).
- **Tracking**: issue [#409](https://github.com/ChenXu233/YaoXiang/issues/409) (runtime `while`
  value taking)—the main form has been fixed and **closed**; the zero-iteration remainder is an
  **intentionally preserved design boundary** (ruled on 2026-10-04, not fixed for now), pinned by
  contract use case `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`, no separate
  tracker opened. This section keeps the minimum reproduction, fix record, and remainder boundary
  (en mirror is handled by the auto-translation bot, no manual edits).

#### Mutual Recursion: SCC-shared Measure

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

Each of the two functions carries the unary form `Terminates(nat)`. After SCC collection the system
recognizes that they share the same measure; the cross-function edge obligation
`nat(n - 1) < nat(n)` is always true under guard `n != 0`, and both functions close the loop in a
single verification.

Without SCC recognition, the two functions still pass the same obligation individually when verified
separately—just done twice. This confirms SCC's positioning as an optimization.

### Diagnostics

**When well-foundedness cannot be derived, don't reject directly, but suggest a checking
direction.** This must be distinguished from the diagnostic for a decrease-obligation failure: the
former points to the measure's value range, the latter to the recursive parameter. The three failure
categories and their respective suggestions:

| Failure                            | Suggested Direction                                                                                            |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Well-foundedness cannot be derived | Whether the measure holds a lower bound on its possible values (e.g. does an `Int` measure need `>= 0`)        |
| Strict decrease judged false       | Whether the recursive parameter actually moves in the measure's decreasing direction; with SMT counterexample  |
| No measure and exploration failed  | Hint that this computation can be given a name and have a measure provided in the type position (RFC-027 §6.9) |

Counterexample rendering follows RFC-013 diagnostic message conventions. Under non-linear path
guards, Sat counterexamples may not be intuitive—recorded as a known limitation, to be iterated with
RFC-013.

### Error Codes

Aligned with the E4xxx proof-failure family (E4018 refinement predicate violation, E4020 proof
function required):

| Proposed Code | Name                   | Trigger                                                                                                     |
| ------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------- |
| E4021         | Termination unprovable | Exploration failed and no explicit measure in the type position (suggests providing a `Terminates` measure) |
| E4022         | Measure does not hold  | Measure obligation judged false by SMT (with counterexample)                                                |

Final numbering is subject to the actual RFC-013 registry at implementation time (segment legality
is guaranteed by build.rs's threshold).

### Pre-existing Defects in the Diagnostic Layer

In the current implementation, in-scope termination failures are reported as **E8001 "internal
compiler error"** (`Unproven` is formatted as an ICE)—termination checking is not an ICE, and
occupying that code slot both misleads users and hides real failures. This RFC fixes this in
passing: in-scope termination failures go to E4021/E4022, and the ICE code slot is returned to
genuine internal errors.

### Compiler Changes

| Component                           | Changes                                                                                                                                                                                                                                  |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`   | Interface unification (exploration path and explicit measure path share obligation generation and judgment); plug in Z3 (production injects `default_solver()` in `checker.rs`, not injected under wasm; tests can inject a stub solver) |
| Function call graph + SCC (**new**) | No cross-function call graph repository-wide—`TypeDepGraph` is a variable-level type dependency, not reusable. New function-level call graph and SCC collection, for mutual-recursive measure sharing                                    |
| Obligation generation               | Add: well-foundedness / strict decrease two obligations, path guard injection, lexicographic unfolding                                                                                                                                   |
| Proof pipeline (RFC-009a / #292)    | Reuse `ConstExpr → SMTLib` link and operator mappings like `Mod`, no backend changes                                                                                                                                                     |
| `util/diagnostic/codes/e4xxx.rs`    | Add E4021/E4022 registration (via RFC-013 registry, build.rs threshold takes effect)                                                                                                                                                     |
| locales ×6                          | Six-language templates for the two new codes                                                                                                                                                                                             |
| Diagnostic layer                    | In-scope termination failures migrated out of E8001, into the proof-failure family                                                                                                                                                       |

### Backward Compatibility

- Programs previously rejected by termination checking but **without refinement annotations**: under
  the new criterion they don't enter verification mode and compile through directly—an intentional
  relaxation, only loosening, not tightening.
- Programs previously rejected by termination checking **with refinement annotations**: can pass
  after adding a measure.
- Structural recursion positive cases and existing termination tests: expected output unchanged.

## Trade-offs

### Advantages

- **Zero new syntax**: the measure is written in the type position, reusing the refinement predicate
  application mechanism; no `decreases`-style syntax slot.
- **Proof-domain behavior alignment**: the termination domain now has a fallback channel consistent
  with the correctness domain, but with different landing points—unsolvable propositions in the
  correctness domain are proven with a **body** proof function, while unsolvable measures in the
  termination domain are declared in the **type position**. Both use the same mechanism (both are
  refinement type applications).
- **Reuse infrastructure**: obligations are linear arithmetic plus path guards, going through the
  already-connected #292 pipeline, no new backend.
- **Loops get a designation**: the binding name is the anchor, and loops are fully isomorphic to
  functions in obligation generation, needing no special case for loops.

### Disadvantages and Risks

- **Prerequisite infrastructure work is no less than "wiring"**: function-level call graph and SCC
  collection must be built new, and Z3 is not yet plugged into the production pipeline. The SCC part
  can be deferred (it's an optimization); the call graph part shares sources with SCC, and deferring
  means mutual recursion can only have each function write its own measure—still passable, just with
  duplicate verification.
- **Explicit measure is a writing burden for low-frequency paths**: the pair (measure function +
  type-position declaration) is more verbose than inline annotation. Accepted—fallback is a
  low-frequency path, and the trade-off is that the measure becomes reusable and unit-testable.
- **Noise from well-foundedness obligations**: when the measure returns `Int`, every call must prove
  `>= 0`. Mitigation: when parameter refinements already give a lower bound it's auto-derived; only
  when it can't be derived does it enter residual obligations, and only direction is given, not
  rejection.
- **Counterexample quality**: Sat counterexamples under non-linear guards are not intuitive. Known
  limitation.
- **`Terminates` is the only predicate whose body is generated by the compiler**: deviates from the
  purity of "all predicate bodies user-writable." Rationale: its assertion (every call site /
  back-edge's measure decrease) lives in the computational structure, and user-written predicates
  can't reference function bodies or loop bodies. The built-in surface converges to a single name,
  with zero new mechanism.

## Alternatives

- **`with decreases (b)` inline annotation syntax**: an early proposal in this issue, withdrawn.
  "Termination checking leaves no **annotation syntax** opening" is RFC-027's settled decision;
  inline annotation would turn unsupported termination patterns into syntax slots rather than type
  slots, going against the worldview of "everything is a YaoXiang function, everything is verified
  by the type checker."
- **Separate proof function (`gcd_proof` returning `Terminates(f, m)`, discovered by return type
  scanning)**: early design. Invalidated—it introduces four kinds of complexity: "discovery
  mechanism," "naming convention," "pick first from multiple candidates," and "how names in the
  proof function body are resolved," all of which exist because the proof is put outside. Moving to
  the type position makes all four classes of complexity disappear.
- **Only let the measure into the type (`Terminates(m)`), drop the binary form**: shorter, but loses
  the "explicitly specify measure attribution" expression slot (no place when the measure is defined
  elsewhere and the same measure serves multiple computations). The two arities are two arities of
  the same predicate, and retention cost is nearly zero.
- **No fallback, require users to rewrite into analyzable iteration patterns**: i.e. the status quo.
  gcd / merge partitioning / mutual recursion cannot be rewritten without sacrificing
  readability—this is exactly the trigger for the re-discussion clause.

## Non-Goals

- No new syntax / keywords / annotation slots.
- No generalization extension of automatic measure synthesis (the four strategies maintain RFC-027's
  established plan, beyond scope use explicit measures). Generic measure inference belongs to the
  undecidable side of "discovery"; only template boundaries can be agreed upon, completeness cannot
  be promised.
- No joint solving with RFC-009a propositions (each judges independently, sharing the backend).
- No totality checking at the dependent-type level.
- No requirement that the measure return a natural-number type (not chasing Lean-style
  `WellFoundedRelation` typeclass mechanism)—the measure's return type is unrestricted, and
  well-foundedness as an independent obligation is left to refinement derivation or SMT.

## Stages and Acceptance

- [x] Obligation generation: well-foundedness / strict decrease, path guard injection
- [x] Explicit measure wiring: unary form anchor resolution (return type position / binding
      position) + type-position measure extraction
- [x] SMT judgment wiring (reuse #292 pipeline, production pipeline injects Z3—the injection point
      was never called before)
- [ ] Function-level call graph + SCC collection (measure sharing optimization). Condition to lift:
      need for "same measure serving multiple computations" or mutual recursion without duplicate
      verification
- [ ] Lexicographic unfolding (when measure returns a tuple). Condition to lift: real use case of a
      tuple-returning measure
- [x] E4021/E4022 registration + six-language locales (three-party consistent: codes ↔ locales ↔
      RFC-013 code table)
- [x] Diagnostic layer fix: in-scope termination failures migrated out of E8001, into the
      proof-failure family. E4021 already in place; E4022 goes through
      `DisproofKind::MeasureNotDecreasing` to the user domain, with a use case pinned to forbid
      downgrade to ICE
- [x] E2E positive cases: gcd (including "residual obligation doesn't reject" form), zero regression
      for structural recursion
- [x] E2E negative cases: measure doesn't hold → E4022 (with counterexample); no measure → E4021
- [x] E2E positive case (loop · type-side): `Terminates(n - i)` binding form's type judgment holds
      (D5 resolved)
- [x] E2E positive case (loop · runtime): `while` as a value's runtime taking (D6 fixed, #409,
      2026-10-04)—`explicit_termination_measure_loop.yx` and `while_block_value.yx` pass actual runs
      after adding `main()`
- [ ] E2E remainder (zero iteration): `while` as a value and the loop body never executing takes
      `Void` (static approximation boundary). Contract use case
      `02-type-system/while_zero_iteration_void_err.yx` (`// expect: runtime-error E6007`);
      condition to lift see §Example loop section "Known Remainder"
- [ ] E2E pending landing: `is_even`-`is_odd` mutual recursion (awaiting SCC)
- [ ] Binary form `Terminates(FnType, m)` (parked). Condition to lift see §Anchor
- [ ] Return-position multi-parameter predicate + symbolic argument (has clear diagnostics from
      2026-10-02, still unsupported). Triggering form / alternative writing / condition to lift see
      §Implementation Landing Record (2026-10-02, this round's hardening)
- [x] Criterion regression: recursion and loops without refinement annotations are no longer
      rejected by termination checking; obligations trigger as usual with refinement annotations
- [x] Acceptance demo: write a declaration whose measure doesn't hold → compile fails with readable
      counterexample (`measure_not_decreasing_err.yx`); passes after correction
      (`measure_decreasing.yx`)
- [x] Return-point refinement (2026-10-02): return-position formal parameters identified by
      **declared name**, predicate argument actually substituted into constraint; undeclared free
      variables report E1001
- [x] Termination strategy 1 restored (2026-10-02): candidates no longer emptied by boundary
      filtering, measure change derived by Direction; new flag loop (strategy 1b)
- [x] Documentation and implementation alignment (2026-10-02): loop first example and landing record
      in this document, the two examples in `language-spec/type-system.md`, the `§2.15` → `§2.9`
      reference fix, RFC-011 loop example

### Implementation Landing Record (2026-09-30 ~ 2026-10-01)

Landed surface: unary `Terminates` enters the type parser → AST extracts the measure (**not** via
lossy `MonoType` conversion, which would reduce the measure to `Int(64)`) → obligation generation
(recursive call sites, loop back-edges, with path guards) → well-foundedness and strict decrease
dual SMT judgment → diagnostic emission.

Two **gates** for releasing E4022 (both only under-report, never false-positive):

1. **Judged false, not judged undecidable**—only report when the solver gives a counterexample
   (`Sat`); `Unknown` means "can't judge," handled by the §Trade-offs "only give direction, don't
   reject."
2. **Measure well-foundedness already proven**—integer `<` is **not** well-founded; a measure not
   decreasing does **not** mean non-termination (gcd still terminates when `b` is negative). When
   well-foundedness isn't proven, even if decrease is judged false we don't report; it enters
   residual obligations.

The existence of these two gates reconciles §Trade-offs's "only give direction, don't reject" and
"judged false means error": well-foundedness is that "not decreasing implies non-terminating"
license.

Pre-existing defects fixed during landing (all prerequisites for this RFC):

- **Predicate definition not registered** (#377-3): the `PredicateResolver`'s predicate body
  substitution was long implemented, but short-circuited by an empty table. After registering,
  refinement constraints become symbolically reason-able for the first time—the prerequisite for
  well-foundedness `b >= 0`.
- **`while` without value semantics** (D5): see §Example loop section.
- **evaluator missing `Neg`/`Pos`/`BitNot`**: inconsistent with `const_eval`'s same-name
  implementation, causing negative literals like `-5` to fail to fold to a constant on all paths
  going through this evaluator.

### Implementation Landing Record (2026-10-02, This Round's Hardening)

Four tracks of work, each corresponding to actual changes; each item below is a reproducible
measured conclusion (commands and output given per item), not a plan.

#### 1. Return-point Refinement: Formal Parameters Identified by Declared Name + Predicate Arguments Actually Substituted into Constraints

Three pre-fix (baseline `2a038f93`) deviations: the predicate argument of `-> (r: P(r + 100))` was
discarded, degrading to `P(r)`; the undefined `m` in `-> (r: Eq2(m, r))` was silently substituted
into the return value, degrading the constraint to always-true. Fix: the parser layer keeps
single-named parentheses as `NamedParen` (carrying the declared name), the type checker **only
recognizes this name** as the return-position formal parameter; other free variables not in scope
each report E1001 and abandon the return-point obligation. Measured:

| Use Case                                                 | Pre-fix                                  | Post-fix (measured)                                                          |
| -------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------- |
| `d1: () -> (r: IsPositive(r + 100)) = { return -5 }`     | E4018 (constraint evaluated as `-5 > 0`) | `0 error` (constraint `-5 + 100 > 0` holds)                                  |
| `d2: () -> (r: IsPositive(r - 100)) = { return 5 }`      | Silently passes                          | `error [E4018]` (constraint `5 - 100 > 0` judged false, with counterexample) |
| `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` undefined) | Silently passes                          | `error [E1001] Unknown variable: 'm'`                                        |

#### 2. Return-position Multi-parameter Predicate + Symbolic Argument: Clear Diagnostics Given (Known Gap, Unsupported)

- **Triggering form** (minimum example):

  ```yaoxiang
  SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }
  f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }
  ```

  Measured diagnostic:
  `error [E2031] After assigning to 'f', the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven in the proof kernel`—the
  target is the **return position** (`'r'`), no longer the form-parameter name `'b'` that was
  mis-attached pre-fix; the constraint text is in the form after substituting arguments.

- **Reason**: the proof kernel only executes proof function calls with "all arguments evaluable
  (literal / already-bound variable)"; a multi-parameter predicate with variables at the return
  position can't get a call and reports unjudgeable.
- **Alternative writing** (the diagnostic's help gives both): change the arguments to
  compile-time-foldable literals (e.g. `SumUpTo(3, r)`), or provide a proof function for that
  predicate that returns `Type` and participates in compile-time execution.
- **Condition to lift**: the proof kernel supports "multi-parameter predicate + symbolic argument"
  argument substitution (same source as the previous item).
- **Tracking**: not opening a new issue—this section is the sole tracker.

#### 3. Termination Strategy 1 Restored (Defect a / b of #377)

- **Defect a**: strategy 1's boundary input is filtered out by "boundary must be a loop invariant" →
  candidates are always zero. Fix: strategy 1 now takes the **unfiltered** boundary set, with
  soundness carried by per-assignment verification.
- **Defect b**: the candidate measure's direction of change was written wrong (`m'` was always
  constructed as `v + 1`) → judgment always failed. Fix: direction determined by the boundary
  operator (`Lt`/`Le` ⇒ Increasing, `Gt`/`Ge` ⇒ Decreasing), measure change derived by Direction
  (Increasing `Δm = δbound - δv`; Decreasing `Δm = δv - δbound`), `Δm >= 0` is rejected on the
  synthesis side and doesn't enter the solver.
- **New flag loop (strategy 1b)**: bare boolean condition + clear flag inside threshold guard +
  variable unconditionally moves toward threshold each round, with the measure being "distance to
  threshold."
- **Soundness caliber unchanged**: rather report "unprovable" than mis-judge termination— the
  measure variable's progression must be **unconditional** (body top level), the boundary
  displacement takes the **least favorable end** including guards, unknown displacement rejects;
  rebinding the measure variable inside a guard block also rejects.

Measured (this worktree's binary):

| Corpus                                                                                           | Measured              |
| ------------------------------------------------------------------------------------------------ | --------------------- |
| `02-type-system/rank_function_dual_variable_loop.yx` (`i < j { i += 1; j -= 1 }`)                | `0 error`             |
| `02-type-system/flag_loop_guarded_exit.yx`                                                       | `0 error`             |
| `06-compile-errors/termination_unproven_err.yx`                                                  | Still `error [E4021]` |
| 4 negative-example corpora (rebinding inside guard block / unknown displacement rebinding, etc.) | All `error [E4021]`   |

#### 4. Documentation and Implementation Alignment

The §Example loop first example in this document and this landing record, the two loop examples in
`docs/src/reference/language-spec/type-system.md`, the loop example in RFC-011 §Termination
Checking, and the `§2.15` → `§2.9` reference fix—see the respective documents' diffs for details
(`§2.15` is "scoping," the block value rule is in `§2.9` block expressions).

#### 5. Still Not Landed After This Round

SCC collection, lexicographic unfolding, binary form `Terminates(FnType, m)`, mutual-recursion E2E,
runtime `while` value's **zero-iteration remainder** (D6 main form fixed), return-position
multi-parameter predicate + symbolic argument— all remain in the unchecked items of §Stages and
Acceptance, with conditions to lift written per item.

## References

- #318 (this RFC's initiation issue), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement type criterion, §6.1–6.5 four measure exploration strategies, §6.9
  explicit measure, §Open Problems re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path condition collection—infrastructure prerequisite)
- RFC-013 (error code registry and proof-failure semantic family)
