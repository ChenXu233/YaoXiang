---
title: 'RFC-027a: Explicit Measure for Termination Checking'
status: 'Under Review'
author: 'Chenxu'
created: '2026-09-14'
updated: '2026-10-02'
issue: '#318'
impl_status: 'partial'
---

# RFC-027a: Explicit Measure for Termination Checking

## Summary

RFC-027 §7 sets the termination checking criterion as **refinement types** (refinement enters
verification mode), and §6.9 gives the language form of explicit measures (the built-in predicate
`Terminates`, alongside `Int` and `Never` as core primitives). Both are language design issues
belonging to the host RFC, already finalized.

This sub-RFC takes on the landing mechanism: how obligations are generated from the computation
structure, how the judgment pipeline is orchestrated, how diagnostics give direction when proof
fails, how measures are shared in mutual recursion (SCC optimization), and how error codes are
registered. **It does not repeat language semantics**, only the mechanism.

## Motivation

### Why a sub-RFC is needed

The criterion (refinement type enters verification mode) and the form (`Terminates` written in the
type position) were decided in RFC-027, at the language-level granularity. The remaining issues are
too fine-grained — putting them in the host would bloat the main text:

- Where do obligations come from (recursive call sites, loop back-edges, path guards)
- How to compare when the measure returns a tuple (lexicographic unfolding)
- Whether well-foundedness and strict decrease are two independent obligations or unified
- How automatic exploration and explicit measures coexist in the same pipeline
- How the same measure avoids repeated writing and repeated verification in mutual recursion
- How to give **direction** rather than outright rejection when proof fails

### Trigger

The direction is triggered by #318: non-structural recursion (gcd-style non-directly-decreasing,
mutual recursion, merge partition) exceeds the template sequence of measure exploration (RFC-027
§6.2–6.5's four strategies all take "a variable of bounded type" or "target type + swap operation"
as input), and cannot be rewritten into an analyzable iteration pattern without sacrificing
readability. The re-discussion clause in RFC-027's "Open Questions" section is activated here.

## Proposal

### Relationship with automatic exploration

The explicit measure is **not another pipeline**, but input after exploration fails. Once a measure
is given, the same SMT still validates the same set of obligations; if they fail, an error with a
counterexample is reported. Full automation priority remains: exploration always runs first, user
intervention only happens after exploration fails.

This decision means the two paths share all downstream mechanisms — obligation generation, SMT
judgment, lexicographic unfolding, and diagnostic format all have a single implementation.

### Obligation generation

For function `f` and measure `m`, two **independent** obligations are generated, expanded at every
recursive call site (including cross-function calls within an SCC):

1. **Well-foundedness**: `m(args) >= 0` — the measure lands on the natural numbers; when the measure
   returns a non-natural-number type, the lower bound of the well-defined order on that type is
   taken. Derived from argument refinements; falls into residual obligations if it cannot be
   derived.
2. **Strict decrease**: at every call site `m(callee_args) < m(caller_args)`, judged under **path
   guards** — guards come from the branch conditions of the call site, reusing RFC-009a's path
   condition collection.

The two are independent rather than unified, because the failure directions differ: well-foundedness
failure means the measure's value domain is wrong (e.g., `Int` may be negative), and decrease
failure means the recursive arguments don't move in that direction. The diagnostic needs to
distinguish them to give the correct checking direction (see the "Diagnostics" section).

**Loops work the same way**, replacing "call site" with "back-edge": on every execution path of the
loop body `m(next-round state) < m(this-round state)`, with guards from the loop condition and
branches in the body.

**Lexicographic unfolding**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation is unfolded as a
lexicographic comparison into a disjunction chain — `(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`. The
unfolding is done on the **obligation generation side**; the SMT side stays in the linear fragment
and does not depend on the solver's native support for lexicographic orders.

**The measure itself must be compile-time evaluable**: `m` must be a function provable by constant
folding or structural recursion — it is forbidden to recursively push the termination problem onto
another unproven function (preventing infinite regression). Pathological measures are blocked by the
existing E4012 (constant recursion too deep) and structural checks.

### Judgment pipeline

```
1. Parameter structural decrease (strongest path, try first)
2. Measure exploration: four-strategy template sequence (RFC-027 §6.2–6.5), stop once one succeeds
3. Exploration succeeds → generate obligations → SMT judgment → Proved
4. Exploration fails → check whether the type position gives an explicit measure (Terminates)
     Yes → use that measure to generate obligations → SMT judgment
     No  → E4021 (termination cannot be proved, with suggested checking directions)
5. Obligation judged false by SMT → E4022 (measure does not hold, with counterexample)
```

Steps 1–3 are the existing path from RFC-027; this RFC adds step 4 and two error codes. The entire
pipeline runs only when the refinement type is triggered (RFC-027 §7) — ordinary types without
refinement generate no obligations.

### Anchor: unary and binary forms

RFC-027 §6.9 defines two arities; this RFC specifies the landing point for each:

| Form                    | Anchor                 | Landing point                                                                                                                                |
| ----------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding    | Default form at definition site — self-recursive functions, loops                                                                            |
| `Terminates(FnType, m)` | Explicit function type | When the measure's belonging function type must be stated explicitly (measure defined elsewhere, same measure serving multiple computations) |

The two are not two constructors, but two arities of the same predicate: the termination obligation
always lands on "the computation annotated at the type position where the refinement sits." Mutual
recursion does **not** require the binary form — each function writes its own unary form, and the
sharing relationship is identified by SCC (see next section).

**Landing status (2026-09-30)**:

- **The unary form has landed**, with two observed landing points: the function **return type
  position** (`gcd: (a: Int, b: Int) -> Terminates(b)` → key `gcd`) and the variable **binding
  position** (`acc: Terminates(n - i) = ...` → key `acc`).
- **The binary form `Terminates(FnType, m)` is parked (deferred)**. Reason: the implementation
  surface is unary — `Terminates` enters the type parser as a unary predicate (the first argument is
  the measure expression), while the binary form's first argument is a **function type**, requiring
  a separate parsing path for "which function type the measure belongs to." There is currently no
  demand: mutual recursion is expressible via each function writing its own unary form + SCC
  sharing, but SCC has not landed yet, so doing the binary form first has no comparable consumer.
  Lift condition: after SCC lands, if "the same measure serving multiple computations" is still
  needed, then add the binary parser. The `gcd` writing in this file's §Examples (binary form) is
  the form to be rewritten.

### SCC: measure sharing optimization

For a group of mutually recursive functions (a strongly connected component on the call graph) that
share the same measure, the cross-function edge obligation is
`m_callee(callee_args) < m_caller(caller_args)`, which degrades to a same-measure decrease when
members share the measure.

**This is an optimization, not a correctness prerequisite**: without sharing, each function writes
its own measure and closes its own loop to pass. SCC's value is recognizing "this group uses the
same measure," avoiding repeated writing and repeated verification.

A new **function-level call graph and SCC collection** is required — the existing `TypeDepGraph`
records variable-to-variable type annotation dependencies (the VC trigger in RFC-027 §6.1); it is a
variable-level graph and cannot be reused.

### Examples

#### gcd: non-structural recursion

```yaoxiang
// Measure: an ordinary function, unit-testable and reusable
gcd_measure: (a: Int, b: Int) -> Int = { b }

gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

Obligation generation:

1. **Well-foundedness**: `gcd_measure(a, b) >= 0` i.e. `b >= 0` — derived directly from the
   parameter refinement `NonNegative(b)`
2. **Strict decrease**: the only recursive call site `gcd(b, a % b)`, path guard `b != 0`,
   obligation `gcd_measure(b, a % b) < gcd_measure(a, b)`; substituting the measure body unfolds to
   `a % b < b`
3. **SMT**: verify the negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved

#### Loop: anonymous construction gets denotation

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

The refinement `Terminates(m)` is on the value type of the loop body's tail expression, with the
anchor provided by the binding name `acc` — the loop is thus **denotable**, and no longer has the
dead end of "anonymous construction has no denotation." This also explains the unity of the two
arities: the obligation always lands on "the computation annotated at the type position where the
refinement sits"; there is no difference between functions and loops.

**Note on the loop body's tail**: the value of `while` is the value of the loop body block (spec
§2.9). If the body tail is written as an assignment statement `i = i + 1`, the block value is
`Void`, and `return acc` from `-> Int` returning `Void` is not legal. The above example adds a tail
expression `i` after the assignment to give it that value — **this is the only correct form for this
example, and this file does not give the pre-fix incorrect form** (the pre-fix form and correction
history are recorded in "Landing status" below).

Obligation: on the back-edge `(n - i') < (n - i)`, guard `i < n`, substituting `i' = i + 1` gives
`1 > 0`, which is a tautology → Proved.

**Landing status (recorded 2026-10-01; revised 2026-10-02, 2026-10-04): the type side and runtime
side of this loop form are both unblocked; the zero-iteration value remains a known residual (see
"Known defect D6" below)**.

The prerequisite is that `while` has value semantics: this form was once blocked (D5) — the `while`
expression at the time **always** typed to `Void`, with no value type to carry a refinement, so
`Terminates` had no computation to attach to. This defect has been fixed: per spec §2.9 "the value
of every `{}` block is given by its tail expression, with no exceptions", the loop body is also a
`{}` block, so the value of `while` equals the value of the loop body block.

This rule also exposed an **early type error in this example**: the pre-fix form wrote the loop body
tail as an assignment statement (e.g. `while i < n { i = i + 1 }`), making the block value `Void`
(spec §2.9 same rule), and `return acc` from `-> Int` returning `Void` is not legal. The fix is to
make the body tail give a value: add a tail expression `i` after the assignment — the example above
is already the corrected form, and this file no longer retains a copyable incorrect form.

The back-edge obligation and the recursive back-edge are **isomorphic**
(`m[var := back-edge value] < m`), and the loop condition is the source of the well-foundedness
lower bound — `i < n` derives `n - i > 0`. Observed: `n - i` judged Proved — this is a **type-side**
conclusion. At runtime, this form **returns `4`**: the binding gets the value of the last
iteration's body block (fixed in #409, observed 2026-10-04). What this file previously wrote as
"returns 4 when `n = 4`" was at the time an **unreproduced false conclusion** (deleted 2026-10-02);
that conclusion became true after issue #409 was fixed, so it is restored to a reproduced
conclusion.

What has landed is the **function form** (return-type-position unary measure, see §Examples gcd) and
the **loop form**, two paths; the loop side's existing refined-variable gating + E4021 is retained
(the exploration path when there is no explicit measure).

**Known defect D6 (runtime `while` value-taking; recorded 2026-10-02, fixed 2026-10-04)**. Type-side
holding once did not equal runtime usability: the above example `check` had 0 errors, but at
**runtime** the binding got the value `Void`, and any use of that value crashed at runtime. **This
defect (#409) has been fixed**: the runtime value of `while` (and isomorphic `for`) is changed to
the value of the loop body's last-iteration tail expression, same caliber as the type-side
`block_value_ty`. The above example `check` 0 errors and `run` exits 0.

Minimum reproduction (the loop form from this file's §Examples + an explicit `main()` call at the
end — script mode's `main` is not implicitly executed). The following code and output are
**pre-fix** historical records, retained as defect evidence; post-fix, the same code `run` exits 0,
with no output:

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

Observed (this worktree's binary; baseline `2a038f93` reproduces the same):

```
error [E6007] Runtime error: type mismatch in comparison Eq: Void vs Int(4)
 --> loop_runtime.yx:14:5
```

- **Independent of iteration count** (pre-fix): `loop(0)` (zero iterations) reports
  `Eq: Void vs Int(0)` — the runtime `while` expression simply did not produce a value, not "the
  last iteration got the wrong value."
- **Impact goes beyond the `Terminates` form** (pre-fix): any `while`-as-value writing hits it.
  Observed: `x: Int = while i < 3 { i = i + 1; i }` then using `x` at runtime →
  `Eq: Void vs Int(3)`. The type-side criteria, obligation generation and diagnostics are all
  normal; the defect is only in the runtime value-taking path.
- **Fix landed (2026-10-04, issue #409)**: `src/middle/core/ir_gen.rs`'s `generate_while_expr_ir` /
  `generate_iterator_for_loop_ir` — write `Void` to the value register as the initial value before
  the loop, the tail expression generated in the loop body writes directly to that register, and
  remove the unconditional `Void` overwrite at the exit. Observed: `loop(4) == 4`,
  `countdown(3) == 30`, `x: Int = while i < 3 { i = i + 1; i }` (`x == 3`), the **main form is fully
  green** for zero-iteration cases.
- **Known residual (zero iterations)**: when the loop body is never evaluated (e.g. `loop(0)`),
  there is no "last-iteration body value", and the value register keeps the initial `Void` — binding
  it to a non-`Void` type position and using it at runtime reports `E6007` type mismatch (a loud
  failure, not a silent wrong value). This is the boundary of static approximation: the type-side
  `block_value_ty` gives the type of the body tail expression, and has no judgment ability over
  "whether the loop runs at least once"; downgrading it to `Void` would break the value type that
  the `Terminates` loop binding in this section depends on, so it is **not** downgraded. The
  residual contract is pinned by `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`
  (`// expect: runtime-error E6007`).
- **Lift condition (historical, completed)**: at runtime, produce the value of a `while` expression
  as the value of the loop body block's tail expression, and let
  `tests/yaoxiang/02-type-system/explicit_termination_measure_loop.yx` and `while_block_value.yx`
  pass after adding `main()` calls. The two test files have now had `main()` added and run through
  (the zero-iteration assertion in `explicit_termination_measure_loop.yx` was changed to the
  non-zero-iteration `loop(1) == 1`, with the zero-iteration case handed off to the new test case
  from the previous item).
- **Tracking**: issue [#409](https://github.com/ChenXu233/YaoXiang/issues/409) (runtime `while`
  value-taking) — the main form has been fixed and **closed**; the zero-iteration residual is **an
  intentionally retained design boundary** (ruling 2026-10-04, not fixed for now), pinned by the
  contract test `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`, with no separate
  tracking item opened. This section retains the minimum reproduction, fix record, and residual
  boundary (en mirror is handled by the auto-translation bot, no manual edits).

#### Mutual recursion: SCC-shared measure

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

Both functions carry the unary form `Terminates(nat)`. After SCC collection, the shared same measure
is identified, and the cross-function edge obligation `nat(n - 1) < nat(n)` is a tautology under
guard `n != 0`; both functions close the loop in a single verification.

Without SCC identification, each function still passes when verifying the same obligation
independently — it just duplicates once. This confirms SCC's positioning as an optimization.

### Diagnostics

**When well-foundedness cannot be derived, do not reject outright, but suggest a checking
direction.** This must be distinguished from the diagnostic for a decrease obligation failure: the
former points to the measure's value domain, the latter points to the recursive arguments.
Suggestions for the three types of failure:

| Failure                            | Suggested direction                                                                                            |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Well-foundedness cannot be derived | Whether the measure holds a lower bound on possible values (e.g., does an `Int` measure need `>= 0`?)          |
| Strict decrease judged false       | Whether the recursive arguments actually move in the measure-decreasing direction; SMT counterexample attached |
| No measure and exploration failed  | Hint that a name can be bound to the computation and a measure provided at the type position (RFC-027 §6.9)    |

Counterexample presentation follows RFC-013 diagnostic message conventions. Under non-linear path
guards, Sat counterexamples may not be intuitive — recorded as a known limitation, iterated with
RFC-013.

### Error codes

Aligned with the E4xxx proof-failure family (E4018 refinement predicate violation, E4020 proof
function required):

| Proposed code | Name                         | Trigger                                                                                                |
| ------------- | ---------------------------- | ------------------------------------------------------------------------------------------------------ |
| E4021         | Termination cannot be proved | Exploration failed and no explicit measure at the type position (hint to provide `Terminates` measure) |
| E4022         | Measure does not hold        | Measure obligation judged false by SMT (with counterexample)                                           |

The final number is subject to the actual state of the RFC-013 registry at implementation time
(segment legality is guaranteed by the build.rs threshold).

### Existing diagnostic-layer defects

In the current implementation, in-scope termination failures are reported as **E8001 "Internal
Compiler Error"** (`Unproven` is formatted as an ICE) — termination checking is not an ICE, and
occupying that code position misleads users and conceals real faults. This RFC fixes that together:
in-scope termination failures go to E4021/E4022, and the ICE code position is returned to real
internal errors.

### Compiler changes

| Component                           | Change                                                                                                                                                                                                                                      |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`   | Interface unification (exploration path and explicit-measure path share obligation generation and judgment); Z3 injection (production injects `default_solver()` in `checker.rs`, wasm does not inject; unit tests can inject stub solvers) |
| Function call graph + SCC (**new**) | No cross-function call graph repository-wide — `TypeDepGraph` is a variable-level type dependency and cannot be reused. New function-level call graph and SCC collection for mutual-recursion measure sharing                               |
| Obligation generation               | Added: well-foundedness / strict decrease two obligations, path-guard injection, lexicographic unfolding                                                                                                                                    |
| Proof pipeline (RFC-009a / #292)    | Reuse the `ConstExpr → SMTLib` link and `Mod` etc. operator mappings, no backend changes                                                                                                                                                    |
| `util/diagnostic/codes/e4xxx.rs`    | Register E4021/E4022 (via RFC-013 registry, build.rs threshold takes effect)                                                                                                                                                                |
| locales ×6                          | Six-language templates for the two new codes                                                                                                                                                                                                |
| Diagnostic layer                    | In-scope termination failures migrated from E8001 to the proof-failure family                                                                                                                                                               |

### Backward compatibility

- Programs previously rejected by termination checking but with **no refinement annotation**: under
  the new criteria, they don't enter verification mode and pass compilation directly — a deliberate
  relaxation, only loosening, never tightening.
- Programs previously rejected by termination checking and with refinement annotations: can pass
  after adding a measure.
- Structural-recursion positive cases, existing termination tests: expected output unchanged.

## Trade-offs

### Advantages

- **Zero new syntax**: the measure is written in the type position, reusing the refinement-predicate
  application mechanism; no `decreases`-style syntax position.
- **Proof-domain behavior alignment**: the termination domain gets a fallback channel consistent
  with the correctness domain, but with a different landing point — propositions the correctness
  domain cannot prove have proof functions written in the **body**, while measures the termination
  domain cannot explore are declared in the **type position**. Both share the same mechanism (both
  are refinement type applications).
- **Reuse of infrastructure**: obligations are linear arithmetic plus path guards, going through the
  pipeline #292 has already connected, with no new backend.
- **Loops get a denotation**: the binding name is the anchor; loops and functions are completely
  isomorphic in obligation generation, no need for loop-specific special cases.

### Disadvantages and risks

- **Up-front infrastructure work is no less than the "wiring"**: function-level call graph and SCC
  collection need to be built from scratch, and Z3 has not been injected into the production
  pipeline. The SCC part can be deferred (it is an optimization); the call graph part shares the
  same source as SCC, and if deferred, mutual recursion can only write each function's own measure —
  still passing, just duplicating verification.
- **Explicit measures impose a writing burden on a low-frequency path**: the two-piece set (measure
  function + type-position declaration) is more verbose than inline annotation. Accepted — the
  fallback is a low-frequency path, and in exchange the measure becomes reusable and unit-testable.
- **Noise of well-foundedness obligations**: when the measure returns `Int`, it must be proven
  `>= 0` every time. Mitigation: derived automatically if the parameter refinement already gives a
  lower bound; falls into a residual obligation only when it cannot be derived, and only gives
  direction, not rejection.
- **Counterexample quality**: Sat counterexamples under non-linear guards are not intuitive. Known
  limitation.
- **`Terminates` is the only predicate whose body the compiler writes for the user**: deviates from
  the purity of "all predicate bodies are user-writable." Reason: its assertions (decrease at every
  call site / back-edge) live inside the computation structure, and user-written predicates cannot
  reference function bodies or loop bodies. The built-in surface converges to one name, with zero
  new mechanism.

## Alternatives

- **`with decreases (b)` inline annotation syntax**: an early proposal in this issue, withdrawn. "No
  **annotation-syntax** opening for termination checking" is the final decision of RFC-027; inline
  annotation would turn unsupported termination patterns into syntax positions rather than type
  positions, going against the worldview of "everything is a YaoXiang function, everything is
  verified by the type checker".
- **Independent proof function (`gcd_proof` returning `Terminates(f, m)`, discovered by scanning
  return types)**: early design. Abandoned — it introduces four kinds of complexity: "discovery
  mechanism", "naming convention", "multi-candidate, pick first", and "how names inside the proof
  function are resolved", all because the proof is placed outside the body. Moving it to the type
  position makes all four complexities disappear.
- **Only the unary form (`Terminates(m)`), drop the binary form**: shorter, but loses the position
  for "explicitly stating the measure's belonging" (nowhere to put it when the measure is defined
  elsewhere or the same measure serves multiple computations). The two arities are two arities of
  the same predicate, and the cost of keeping them is nearly zero.
- **No fallback, require users to rewrite into analyzable iteration patterns**: the current state.
  gcd / merge partition / mutual recursion cannot be rewritten without sacrificing readability —
  which is exactly the trigger condition for the re-discussion clause.

## Non-goals

- No new syntax / keywords / annotation positions.
- No generalizing extension of automatic measure synthesis (the four strategies keep RFC-027's
  existing plan; beyond scope, go to explicit measures). General measure inference belongs to the
  undecidable side of "discovery"; only template boundaries can be agreed upon, completeness cannot
  be promised.
- No joint solving with RFC-009a's borrowed propositions (each judges independently, sharing the
  backend).
- No totality checking at the dependent-type level.
- No forcing measures to return natural-number types (not pursuing Lean's `WellFoundedRelation`
  type-class mechanism) — the measure's return type is unrestricted, well-foundedness as an
  independent obligation is handed to refinement derivation or SMT.

## Phases and acceptance

- [x] Obligation generation: well-foundedness / strict decrease, path-guard injection
- [x] Explicit-measure wiring: unary-form anchor resolution (return type position / binding
      position) + type-position measure extraction
- [x] SMT judgment wiring (reuse #292 pipeline, inject Z3 into production pipeline — the injection
      point was never called before)
- [ ] Function-level call graph + SCC collection (measure-sharing optimization). Lift condition: a
      need appears for "the same measure serving multiple computations" or mutual-recursion
      deduplication
- [ ] Lexicographic unfolding (when the measure returns a tuple). Lift condition: a real use case
      appears for tuple-returning measures
- [x] E4021/E4022 registration + six-language locales (three-way consistent: codes ↔ locales ↔
      RFC-013 code table)
- [x] Diagnostic-layer fix: in-scope termination failures migrated from E8001 to the proof-failure
      family. E4021 exists; E4022 goes through `DisproofKind::MeasureNotDecreasing` to the user
      domain, with test cases pinning it from being downgraded to ICE
- [x] E2E positive: gcd (including "residual obligation not rejected" form), zero-regression for
      structural recursion
- [x] E2E negative: measure does not hold → E4022 (with counterexample); no measure → E4021
- [x] E2E positive (loop · type side): type judgment of the `Terminates(n - i)` binding form holds
      (D5 resolved)
- [x] E2E positive (loop · runtime): runtime value-taking of `while`-as-value (D6 fixed, #409,
      2026-10-04) — `explicit_termination_measure_loop.yx` and `while_block_value.yx` pass after
      `main()` is added
- [ ] E2E residual (zero iterations): `while`-as-value where the loop body executes zero times
      yields `Void` (static-approximation boundary). Contract test
      `02-type-system/while_zero_iteration_void_err.yx` (`// expect: runtime-error E6007`); lift
      condition in §Examples loop section "Known residual"
- [ ] E2E pending landing: `is_even`-`is_odd` mutual recursion (waiting for SCC)
- [ ] Binary form `Terminates(FnType, m)` (parked). Lift condition in §Anchor
- [ ] Return-position multi-parameter predicate + symbolic arguments (clear diagnostics since
      2026-10-02, still unsupported). Trigger form / alternative writing / lift condition in
      §Implementation landing record (2026-10-02, this round of hardening)
- [x] Criterion regression: recursion and loops without refinement annotation are no longer rejected
      by termination checking; with refinement annotation, obligations are triggered as usual
- [x] Acceptance demo: write a declaration whose measure does not hold → compilation fails and the
      counterexample is readable (`measure_not_decreasing_err.yx`); after fixing, it passes
      (`measure_decreasing.yx`)
- [x] Return-position refinement (2026-10-02): return-position formal parameters are identified by
      **declaration name**, predicate arguments are really substituted into constraints; undeclared
      free variables report E1001
- [x] Termination strategy 1 restored to work (2026-10-02): candidates are no longer emptied by the
      boundary filter, measure changes are derived by Direction; added flag loop (strategy 1b)
- [x] Documentation and implementation alignment (2026-10-02): the loop first-example and landing
      record in this file, the two examples in `language-spec/type-system.md`, the `§2.15` → `§2.9`
      reference, and the RFC-011 loop example

### Implementation landing record (2026-09-30 ~ 2026-10-01)

Landing surface: unary `Terminates` enters the type parser → AST extracts the measure (**not**
through lossy `MonoType` conversion, which would drop the measure to `Int(64)`) → obligation
generation (recursive call sites, loop back-edges, with path guards) → well-foundedness and
strict-decrease two-way SMT judgment → diagnostic emission.

The **two gates** for issuing E4022 (both only under-report, never over-report):

1. **Judged false, not judged unable** — only report when the solver gives a counterexample (`Sat`);
   `Unknown` is "unable to judge" and is handled per §Trade-offs' "only give direction, don't
   reject".
2. **Measure well-foundedness has been proved** — integer `<` is **not** well-founded, so measure
   not decreasing **does not** equal non-termination (`gcd` still terminates when `b` is negative).
   When well-foundedness is not proved, even if decrease is judged false, it is not reported, and
   falls into residual obligations.

The existence of these two gates resolves the apparent contradiction in §Trade-offs between "only
give direction, don't reject" and "judged false means error": well-foundedness is the license for
"not decreasing means non-termination".

Pre-existing defects fixed in the landing process (all are the prerequisite foundation for this
RFC):

- **Predicate definition not registered** (#377-3): the `PredicateResolver`'s predicate-body
  substitution was already implemented, but was short-circuited by an empty table. After
  registration, refinement constraints become symbolically reason-able for the first time — the
  prerequisite for well-foundedness `b >= 0`.
- **`while` has no value semantics** (D5): see §Examples loop section.
- **evaluator missing `Neg`/`Pos`/`BitNot`**: inconsistent with the same-name implementation in
  `const_eval`, causing negative literals like `-5` to be unable to fold to a constant on any path
  through that evaluator.

### Implementation landing record (2026-10-02, this round of hardening)

Four lanes of work, each corresponding to actual changes; each item below is a reproducible observed
conclusion (commands and output given per item), not a planned scope.

#### 1. Return-position refinement: formal parameters identified by declaration name + predicate arguments really substituted into constraints

The three deviations in the pre-fix state (baseline `2a038f93`): the predicate argument of
`-> (r: P(r + 100))` was discarded, degraded to `P(r)`; the undefined `m` in `-> (r: Eq2(m, r))` was
silently substituted into the return value, with the constraint degraded to a tautology. Fix: the
parser layer keeps the single named parenthesis as `NamedParen` (carrying the declared name); the
type checker **only recognizes this name** as the return-position formal parameter; other
out-of-scope free variables are each reported as E1001 and the return-position obligation is
abandoned. Observed:

| Test case                                                         | Pre-fix                                  | Post-fix (observed)                                                          |
| ----------------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------- |
| `d1: () -> (r: IsPositive(r + 100)) = { return -5 }`              | E4018 (constraint evaluated to `-5 > 0`) | `0 error` (constraint `-5 + 100 > 0` holds)                                  |
| `d2: () -> (r: IsPositive(r - 100)) = { return 5 }`               | Silently passes                          | `error [E4018]` (constraint `5 - 100 > 0` judged false, with counterexample) |
| `g: () -> (r: Eq2(m, r)) = { return 7 }` (where `m` is undefined) | Silently passes                          | `error [E1001] Unknown variable: 'm'`                                        |

#### 2. Return-position multi-parameter predicate + symbolic arguments: clear diagnostics (known gap, unsupported)

- **Trigger form** (minimum example):

  ```yaoxiang
  SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }
  f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }
  ```

  Observed diagnostic:
  `error [E2031] After assigning to 'f', the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven in the proof kernel`
  — the target is the **return position** (`'r'`), no longer the misattached parameter name `'b'`
  from the pre-fix state; the constraint text given is the form after substituting the arguments.

- **Reason**: the proof kernel only executes proof-function calls where "all arguments are valueable
  (literals / already-bound variables)"; multi-parameter predicates containing variables don't get a
  call at the return position, and report when they can't be judged.
- **Alternative writing** (the diagnostic's help also gives two): change the arguments to
  compile-time-foldable literals (e.g. `SumUpTo(3, r)`), or provide a proof function for that
  predicate that returns `Type` and participates in compile-time execution.
- **Lift condition**: the proof kernel supports the argument-substitution path of "multi-parameter
  predicate + symbolic arguments" (same source as the previous item).
- **Tracking**: no new issue opened — this section is the only tracking location.

#### 3. Termination strategy 1 restored to work (#377 defects a / b)

- **Defect a**: the boundary input of strategy 1 was emptied by the "boundary must be a loop
  invariant" filter → candidates were always zero. Fix: strategy 1 changed to take the
  **unfiltered** boundary set, with soundness carried by per-assignment verification.
- **Defect b**: the change direction of the candidate measure was written wrong (`m'` was always
  constructed as `v + 1`) → judgment always failed. Fix: the direction is determined by the boundary
  operator (`Lt`/`Le` ⇒ Increasing, `Gt`/`Ge` ⇒ Decreasing), and the measure change is derived by
  Direction (Increasing `Δm = δbound - δv`; Decreasing `Δm = δv - δbound`); `Δm >= 0` is rejected on
  the synthesis side directly, not sent to the solver.
- **New flag loop (strategy 1b)**: bare boolean condition + clear flag within the threshold guard +
  variable moves unconditionally toward the threshold each round, with the measure taken as
  "distance to threshold".
- **Soundness scope unchanged**: better to report "cannot prove" than to misjudge termination — the
  measure variable's advance must be **unconditional** (top of body), the boundary shift takes the
  **worst-case end** including the guard, and shifts with unknown direction are rejected; rebinding
  the measure variable inside a guard block is also rejected.

Observed (this worktree's binary):

| Test file                                                                         | Observed              |
| --------------------------------------------------------------------------------- | --------------------- |
| `02-type-system/rank_function_dual_variable_loop.yx` (`i < j { i += 1; j -= 1 }`) | `0 error`             |
| `02-type-system/flag_loop_guarded_exit.yx`                                        | `0 error`             |
| `06-compile-errors/termination_unproven_err.yx`                                   | Still `error [E4021]` |
| 4 negative test files (guard-block rebinding / unknown-shift rebinding, etc.)     | All `error [E4021]`   |

#### 4. Documentation and implementation alignment

The loop first-example in this file's §Examples and this landing record, the two loop examples in
`docs/src/reference/language-spec/type-system.md`, the loop example in RFC-011 §Termination
Checking, and the `§2.15` → `§2.9` reference fix — details in each document's diff (`§2.15` is
"Scope", the block-value rule is in `§2.9` Block Expressions).

#### 5. Still not landed after this round

SCC collection, lexicographic unfolding, binary form `Terminates(FnType, m)`, mutual-recursion E2E,
the **zero-iteration residual** of runtime `while` value-taking (D6 main form fixed),
return-position multi-parameter predicate + symbolic arguments — all remain in §Phases and
acceptance's unchecked items, with lift conditions noted per item.

## Related

- #318 (the issue that initiated this RFC), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement-type criteria, §6.1–6.5 four measure-exploration strategies, §6.9
  explicit measure, §Open Questions re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path-condition collection — infrastructure prerequisite)
- RFC-013 (error-code registry and proof-failure semantic family)
