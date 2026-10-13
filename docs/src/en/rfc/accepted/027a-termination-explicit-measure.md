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

RFC-027 §7 sets the termination checking criterion as **refinement type** (refinement enters
verification mode), and §6.9 gives the language form of explicit measure (the built-in predicate
`Terminates`, in the same core primitive family as `Int` and `Never`). Both are language design,
belonging to the host RFC, and are finalized.

This sub-RFC takes over the landing mechanism: how obligations are generated from computation
structure, how the decision pipeline is orchestrated, how diagnostics give direction when
unprovable, how measures in mutual recursion are shared (SCC optimization), and how error codes are
registered. **It does not repeat language semantics**, but only defines the mechanism.

## Motivation

### Why a sub-RFC is needed

The criterion (refinement type enters verification mode) and form (`Terminates` written in the type
position) have been settled in RFC-027, at the language level. The remaining issues are too
fine-grained to fit into the host without bloating its main text:

- Where do obligations come from (recursive call sites, loop back-edges, path guards)
- How to compare when a measure returns a tuple (lexicographic unfolding)
- Are well-foundedness and strict decrease two independent obligations or one unified
- How automatic exploration and explicit measure coexist in the same pipeline
- How mutual recursion avoids repeated writing and repeated verification of the same measure
- How to give **direction** when unprovable rather than just rejecting

### Trigger

The direction is triggered by #318: non-structural recursion (gcd-style non-direct decrease, mutual
recursion, merge partition) goes beyond the template sequence of measure exploration (RFC-027
§6.2–6.5's four strategies all take "variable of bounded type" or "target type + swap operation" as
input), and cannot be rewritten into an analyzable iterative pattern without harming readability.
The re-discussion clause in RFC-027's "Open Problems" section is activated here.

## Proposal

### Relationship with automatic exploration

Explicit measure is **not another pipeline**, but the input after exploration fails. Once the
measure is given, it still goes through the same SMT to verify the same set of obligations; if not,
an error with counterexample is reported. Full automation priority is unchanged: exploration always
runs first, and user intervention only occurs after exploration fails.

This determines that both paths share all downstream mechanisms — obligation generation, SMT
decision, lexicographic unfolding, diagnostic format only have one implementation.

### Obligation Generation

For function `f` and measure `m`, two **independent** obligations are generated, expanded at each
recursive call site (including cross-function calls within SCC):

1. **Well-foundedness**: `m(args) >= 0` — the measure falls on natural numbers; when the measure
   returns a non-natural-number type, take the lower bound of the appropriate order on that type.
   Derived from argument refinements; when not derivable, enter residual obligations.
2. **Strict Decrease**: at each call site `m(callee_args) < m(caller_args)`, decided under the
   **path guard** — the guard comes from the branch condition at that call site, reusing RFC-009a's
   path condition collection.

The two are independent rather than unified, because the failure directions differ: well-foundedness
failure means the measure's range is wrong (e.g., `Int` may be negative), decrease failure means the
recursive argument is not moving in the expected direction. Diagnostics need to distinguish them, to
give the correct checking direction (see the "Diagnostics" section).

**The same applies to loops**, replacing "call site" with "back-edge": on every execution path of
the loop body, `m(next_round_state) < m(this_round_state)`, with the guard from the loop condition
and branches in the body.

**Lexicographic unfolding**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligations, compared in
lexicographic order, expand into a disjunction chain — `(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`.
The unfolding is done on the **obligation generation side**; the SMT side stays in linear fragment,
without depending on the solver's native support for lexicographic order.

**The measure itself must be evaluable at compile time**: `m` must be a function provable via
constant folding or structural recursion — it is forbidden to recursively defer the termination
problem to another unproven function (preventing infinite regression). Pathological measures are
caught by the existing E4012 (constant recursion too deep) and structural check.

### Decision Pipeline

```
1. Structural recursion on parameters (strongest path, try first)
2. Measure exploration: four-strategy template sequence (RFC-027 §6.2–6.5), stop on first success
3. Exploration success → generate obligations → SMT decision → Proved
4. Exploration failure → check whether the type position gives an explicit measure (Terminates)
     Yes → take that measure to generate obligations → SMT decision
     No → E4021 (termination unprovable, with suggested checking direction)
5. Obligations judged false by SMT → E4022 (measure invalid, with counterexample)
```

Levels 1–3 are the path already established by RFC-027; this RFC adds level 4 and two error codes.
The entire pipeline only runs when a refinement type is triggered (RFC-027 §7) — no refinement, no
obligations generated.

### Anchors: Unary and Binary Forms

RFC-027 §6.9 defines two arities; this RFC explains where each lands:

| Form                    | Anchor                 | Landing Point                                                                                                                                         |
| ----------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the bound      | Default form at definition — self-recursive functions, loops                                                                                          |
| `Terminates(FnType, m)` | Explicit function type | When the measure's ownership of a function type needs to be made explicit (measure defined elsewhere, the same measure serving multiple computations) |

The two are not two constructs, but two arities of the same predicate: termination obligations
always fall on "the computation annotated by the type position where the refinement sits". Mutual
recursion **does not need** the binary form — each of the two functions can write its own unary
form, and the shared relation is identified by SCC (see next section).

**Landing status (2026-09-30)**:

- **Unary form has landed**, with two actual landing points: function **return type position**
  (`gcd: (a: Int, b: Int) -> Terminates(b)` → key `gcd`) and variable **binding position**
  (`acc: Terminates(n - i) = ...` → key `acc`).
- **Binary form `Terminates(FnType, m)` parked**. Reason: the implementation face is unary — when
  `Terminates` enters the type parser it is handled as a unary predicate (first argument is the
  measure expression), while the binary form's first argument is a **function type**, requiring a
  separate resolution path for "which function type the measure belongs to". There is no current
  demand: mutual recursion can be expressed by each writing a unary form + SCC sharing, and SCC has
  not yet landed, so doing the binary form first has no comparable consumer. Unblocking condition:
  after SCC lands, if "the same measure serving multiple computations" is still needed, then add
  binary resolution. The gcd writing in §Examples of this document (binary) is exactly the form
  awaiting rewriting.

### SCC: Measure Sharing Optimization

If a group of mutually recursive functions (strongly connected components on the call graph) share
the same measure, the obligation across function edges is
`m_callee(callee_args) < m_caller(caller_args)`, which degenerates to the same-measure decrease when
members share the measure.

**This is an optimization, not a correctness prerequisite**: when not shared, each function writes
its own measure and each loops closed on its own will pass. The value of SCC is recognizing "this
group uses the same measure", saving repeated writing and repeated verification.

A **function-level call graph and SCC collection** need to be created — the existing `TypeDepGraph`
records variable-to-variable type annotation dependencies (the VC trigger from RFC-027 §6.1), which
is a variable-level graph and cannot be reused.

### Examples

#### gcd: Non-structural Recursion

```yaoxiang
// Measure: ordinary function, can be unit-tested, reusable
gcd_measure: (a: Int, b: Int) -> Int = { b }

gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

Obligations generated:

1. **Well-foundedness**: `gcd_measure(a, b) >= 0`, i.e. `b >= 0` — derived directly from the
   parameter refinement `NonNegative(b)`
2. **Strict decrease**: the only recursive call site `gcd(b, a % b)`, path guard `b != 0`,
   obligation `gcd_measure(b, a % b) < gcd_measure(a, b)`; substituting the measure body unfolds to
   `a % b < b`
3. **SMT**: verifies that the negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved

#### Loop: Anonymous Construct Gains Denotation

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

The `Terminates(m)` refinement refines exactly the type of the loop body's tail expression, the
anchor is provided by the binding name `acc` — the loop is thus **denotable**, no longer leaving the
dead corner of "anonymous construct with no denotation". This also explains the unity of the two
arities: the obligation always falls on the computation annotated by the type position where the
refinement sits; there is no difference between functions and loops.

**Note the loop body's tail**: the value of `while` is the value of the loop body block (spec §2.9);
if the body tail is written as an assignment statement `i = i + 1`, the block value is `Void`, and
`return acc` returning `Void` from `-> Int` is not legal. The above example adds a tail expression
`i` after the assignment to give the value — **this is the only correct form of this example; this
document does not give the pre-fix incorrect form** (the pre-fix form and the fix record are in the
"Landing Status" section below).

Obligation: on the back-edge `(n - i') < (n - i)`, guard `i < n`, substituting `i' = i + 1` gives
`1 > 0`, which is a tautology → Proved.

**Landing status (registered 2026-10-01; revised 2026-10-02, 2026-10-04): the type side and runtime
side of this loop form are both unblocked; zero-iteration value retrieval is still a known residual
(see "Known Defect D6" below)**.

The premise is that `while` has value semantics: this form was once blocked (D5) — `while`
expressions at the time were **constantly** typed as `Void`, with no value type available to carry a
refinement, so `Terminates` could not attach to any computation. This defect has been fixed: per
spec §2.9 "the value of all `{}` blocks is given by the tail expression, no exception", the loop
body is also a `{}` block, so the value of `while` = the value of the loop body block.

This rule also exposed the **early type error in this example**: the pre-fix form wrote the loop
body tail as an assignment statement (such as `while i < n { i = i + 1 }`), so the block value was
`Void` (spec §2.9 same rule), and `return acc` returning `Void` from `-> Int` is not legal. The fix
is to let the body tail give a value: add a tail expression `i` after the assignment — the example
above is already the fixed form, this document no longer retains a reproducible incorrect form.

The back-edge obligation is **isomorphic** to the recursive back-edge
(`m[variable := back-edge value] < m`), and the loop condition is the source of the well-foundedness
lower bound — `i < n` derives `n - i > 0`. Actual test: `n - i` judged Proved — this is the
**type-side** conclusion. At runtime, this form **returns `4`**: the binding gets the value of the
last iteration's body block (#409 fixed, verified 2026-10-04). The "returns 4 when `n = 4`" written
earlier in this document was at the time an unreproduced **false conclusion** (deleted 2026-10-02);
this conclusion became true after the fix of issue #409, so it is restored to a reproduced
conclusion.

What has landed are the **function form** (return-type-position unary measure, see §Examples gcd)
and the **loop form** two paths; the original loop-side refinement variable gating + E4021 is
preserved (exploration path when no explicit measure).

**Known Defect D6 (runtime `while` value retrieval; registered 2026-10-02, fixed 2026-10-04)**.
Type-side being valid was not equal to runtime usability: the above `check` 0 error, but at
**runtime** the value the binding got was `Void`, any use of that value would crash at runtime.
**This defect (#409) has been fixed**: the value of runtime `while` (and the isomorphic `for`) is
changed to the value of the loop body's last iteration's body block tail expression, with the same
caliber as the type-side `block_value_ty`. The above `check` 0 error and `run` exits 0.

Minimum reproduction (the loop form from §Examples in this document + an explicit `main()` call at
the end — script mode's `main` does not implicitly execute). The following code and output are the
**pre-fix** historical record, kept as defect evidence; the post-fix same code `run` exits 0 with no
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

Actual test (this worktree binary; baseline `2a038f93` reproduces the same):

```
error [E6007] Runtime error: type mismatch in comparison Eq: Void vs Int(4)
 --> loop_runtime.yx:14:5
```

- **Independent of iteration count** (pre-fix): `loop(0)` (zero iterations) reports
  `Eq: Void vs Int(0)` — the runtime `while` expression didn't even produce a value, not "the last
  iteration took a wrong value".
- **Impact is not limited to the `Terminates` form** (pre-fix): any writing of `while` as a value
  will hit this. Actual test `x: Int = while i < 3 { i = i + 1; i }` then using `x` at runtime →
  `Eq: Void vs Int(3)`. Type-side criteria, obligation generation, and diagnostics are all normal,
  the defect is only in the runtime value retrieval path.
- **Landed fix (2026-10-04, issue #409)**: `generate_while_expr_ir` /
  `generate_iterator_for_loop_ir` in `src/middle/core/ir_gen.rs` — write the `Void` initial value to
  the value register before the loop, the tail expression generated by the loop body is written
  directly to this register, and the unconditional overwrite `Void` instruction at the exit is
  deleted. Actual test `loop(4) == 4`, `countdown(3) == 30`, `x: Int = while i < 3 { i = i + 1; i }`
  (`x == 3`), the main forms of zero-iteration cases all green.
- **Known residual (zero iteration)**: when the loop body has never been evaluated (e.g.,
  `loop(0)`), there is no "value of the last iteration", the value register keeps its initial value
  `Void` — binding to a non-`Void` type position and using it at runtime will report `E6007` type
  mismatch (loud failure, not a silent wrong value). This is the static approximation boundary: the
  type-side `block_value_ty` gives the type of the body tail expression, and has no judgment ability
  for "whether the loop executes at least once"; downgrading it to `Void` would break the value type
  that the `Terminates` loop binding in this section depends on, so it is **not** downgraded. The
  residual contract is pinned by
  `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
  (`// expect: runtime-error E6007`).
- **Unblocking condition (historical, completed)**: the runtime makes the value of the `while`
  expression be the value of the loop body block tail expression, and after
  `tests/yaoxiang/02-type-system/termination/explicit_termination_measure_loop.yx` and
  `while_block_value.yx` add `main()` calls they can pass. Both corpora now have `main()` added and
  pass in actual runs (the zero-iteration assertion in `explicit_termination_measure_loop.yx` is
  changed to the non-zero-iteration `loop(1) == 1`, the zero-iteration case is handed over to the
  new test case in the previous item).
- **Tracking**: issue [#409](https://github.com/ChenXu233/YaoXiang/issues/409) (runtime `while`
  value retrieval) — the main form is fixed and **closed**; the zero-iteration residual is an
  **intentionally preserved design boundary** (ruled 2026-10-04, not currently fixed), pinned by the
  contract case `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`, no
  separate tracking item opened. This section retains the minimum reproduction, fix record, and
  residual boundary (en mirror is handled by the auto-translation bot, not manually edited).

#### Mutual Recursion: SCC Shared Measure

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

Each of the two functions carries the unary form `Terminates(nat)`. After SCC collection, it is
recognized that they share the same measure, and the cross-function-edge obligation
`nat(n - 1) < nat(n)` is a tautology under the guard `n != 0`, so both functions close the loop in
one verification.

Without SCC recognition, each function's own verification of the same obligation still passes — just
repeated once. This confirms that SCC is positioned as an optimization.

### Diagnostics

**When well-foundedness is unprovable, do not directly reject, but suggest a checking direction.**
This must be distinguished from the diagnostics of decrease obligation failure: the former points to
the measure's range, the latter to the recursive parameter. Suggestions for the three types of
failure:

| Failure                           | Suggested Direction                                                                                                  |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Well-foundedness unprovable       | Whether the measure has a lower bound on its possible values (e.g., does the `Int` measure need `>= 0`)              |
| Strict decrease judged false      | Whether the recursive parameter is really moving in the decreasing direction of the measure; with SMT counterexample |
| No measure and exploration failed | Hint that the computation can be given a name and a measure provided in the type position (RFC-027 §6.9)             |

Counterexample presentation follows the RFC-013 diagnostic message spec. Under nonlinear path
guards, Sat counterexamples may not be intuitive — recorded as a known limitation, to be iterated
with RFC-013.

### Error Codes

Aligned with the E4xxx proof-failure family (E4018 refinement predicate violation, E4020 proof
function needed):

| Proposed Code | Name                   | Trigger                                                                                                  |
| ------------- | ---------------------- | -------------------------------------------------------------------------------------------------------- |
| E4021         | Termination unprovable | Exploration failed and no explicit measure at the type position (hint to provide a `Terminates` measure) |
| E4022         | Measure invalid        | Measure obligations judged false by SMT (with counterexample)                                            |

Final numbering is subject to the actual RFC-013 registry at implementation time (segment validity
is guaranteed by the build.rs threshold).

### Existing Defects in the Diagnostic Layer

In the current implementation, in-scope termination failures are reported as **E8001 "internal
compiler error"** (`Unproven` is formatted as ICE) — termination check is not an ICE, and occupying
that code slot both misleads users and masks real failures. This RFC fixes this at the same time:
in-scope termination failures go to E4021/E4022, and the ICE code slot is returned to real internal
errors.

### Compiler Changes

| Component                           | Change                                                                                                                                                                                                                                    |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`   | Interface unification (exploration path and explicit measure path share obligation generation and decision); connect Z3 (production injects `default_solver()` in `checker.rs`, wasm does not inject; unit tests can inject stub solvers) |
| Function call graph + SCC (**new**) | No cross-function call graph in the whole repo — `TypeDepGraph` is variable-level type dependency, not reusable. Create function-level call graph and SCC collection for mutual recursion measure sharing                                 |
| Obligation generation               | New: well-foundedness / strict decrease two obligations, path guard injection, lexicographic unfolding                                                                                                                                    |
| Proof pipeline (RFC-009a / #292)    | Reuse `ConstExpr → SMTLib` chain and `Mod` and other operator mapping, no backend changes                                                                                                                                                 |
| `util/diagnostic/codes/e4xxx.rs`    | New E4021/E4022 registration (via RFC-013 registry, build.rs threshold takes effect)                                                                                                                                                      |
| locales ×6                          | Six-language templates for the two new codes                                                                                                                                                                                              |
| Diagnostic layer                    | In-scope termination failures moved out of E8001, into the proof-failure family                                                                                                                                                           |

### Backward Compatibility

- Programs previously rejected for termination check but **without refinement annotations**: under
  the new criterion they do not enter verification mode, and compile directly — an intentional
  relaxation, only loosening, not tightening.
- Programs previously rejected for termination check with refinement annotations: can pass after
  adding the measure.
- Structural recursion positive cases, existing termination tests: expected output unchanged.

## Trade-offs

### Advantages

- **Zero new syntax**: the measure is written at the type position, reusing the refinement predicate
  application mechanism; no `decreases`-like syntax position.
- **Proof domain behavior alignment**: the termination domain adds a fallback channel consistent
  with the correctness domain, but the landing point is different — propositions unprovable in the
  correctness domain are written in the **body** as proof functions, while measures unprovable in
  the termination domain are declared in the **type position**. Both mechanisms are the same
  (refinement type application).
- **Reuse infrastructure**: obligations are linear arithmetic plus path guards, going through the
  pipeline already connected in #292, no new backend.
- **Loops gain denotation**: the binding name is the anchor, loop and function are completely
  isomorphic in obligation generation, no need to design a special case for loops.

### Disadvantages and Risks

- **Prerequisite infrastructure is no less than "wiring"**: function-level call graph and SCC
  collection need to be created from scratch, and Z3 has not been connected to the production
  pipeline. The SCC part can be deferred (it is an optimization); the call graph part is from the
  same source as SCC, and deferring means mutual recursion can only write each its own measure —
  still passable, just with duplicate verification.
- **Explicit measure is a writing burden for the low-frequency path**: the two-piece set (measure
  function + type position declaration) is more verbose than inline annotations. Accepted — the
  fallback is a low-frequency path, and in return the measure is reusable and unit-testable.
- **Well-foundedness obligation noise**: when the measure returns `Int` it must be proved `>= 0`
  every time. Mitigation: when the parameter refinement already gives a lower bound it is
  automatically derived; only when it cannot be derived does it enter the residual obligation, and
  only the direction is given, not rejected.
- **Counterexample quality**: Sat counterexamples under nonlinear guards are not intuitive. Known
  limitation.
- **`Terminates` is the only predicate for which the compiler writes the body**: deviates from the
  purity of "predicate bodies are all user-writable". Reason: its assertion (the measure decrease at
  every call site / back-edge) lives in the computation structure, and user-written predicates
  cannot reference function or loop bodies. The built-in face converges into a single name, with
  zero new mechanism.

## Alternatives

- **`with decreases (b)` inline annotation syntax**: an early proposal in this issue, withdrawn.
  "Termination check leaves no **annotation syntax** slot" is RFC-027's settled decision, and inline
  annotations would turn unsupported termination modes into syntax positions rather than type
  positions, contrary to the worldview of "everything is a YaoXiang function, everything is verified
  by the type checker".
- **Independent proof function (`gcd_proof` returns `Terminates(f, m)`, discovered by return type
  scanning)**: early design. Deprecated — it introduces four kinds of complexity: "discovery
  mechanism", "naming convention", "multiple candidates take the first", and "how are names inside
  the proof function resolved", all because the proof was placed outside the type. After moving it
  to the type position, all four kinds of complexity disappear.
- **Only letting the measure enter the type (`Terminates(m)`), removing the binary form**: shorter,
  but loses the position for "explicitly specifying measure ownership" (when the measure is defined
  elsewhere, or the same measure serves multiple computations, there is nowhere to put it). The two
  arities are two arities of the same predicate, the retention cost is near zero.
- **No fallback, requiring users to rewrite into analyzable iterative patterns**: i.e. the status
  quo. gcd / merge partition / mutual recursion cannot be rewritten without harming readability —
  this is exactly the trigger condition for the re-discussion clause.

## Non-Goals

- No new syntax / keywords / annotation positions.
- No general-purpose extension of automatic measure synthesis (the four strategies maintain
  RFC-027's established plan, beyond scope goes to explicit measure). General measure inference
  belongs to the undecidable side of "discovery", only template boundaries can be agreed upon,
  completeness cannot be promised.
- No joint solving with RFC-009a's borrowed propositions (each decides independently, sharing the
  backend).
- No totality checking at the dependent-type level.
- No requirement that the measure return a natural-number type (not pursuing Lean's
  `WellFoundedRelation` type class mechanism) — the measure's return type is unrestricted,
  well-foundedness is given to refinement derivation or SMT as an independent obligation.

## Stages and Acceptance

- [x] Obligation generation: well-foundedness / strict decrease, path guard injection
- [x] Explicit measure wiring: unary form anchor resolution (return type position / binding
      position) + type position measure extraction
- [x] SMT decision wiring (reuse #292 pipeline, production pipeline injects Z3 — the previous
      injection point was never called)
- [ ] Function-level call graph + SCC collection (measure sharing optimization). Unblocking
      condition: demand appears for "the same measure serving multiple computations" or mutual
      recursion avoiding repeated verification
- [ ] Lexicographic unfolding (when the measure returns a tuple). Unblocking condition: a real use
      case of returning a tuple measure appears
- [x] E4021/E4022 registration + six-language locales (three-party consistency: codes ↔ locales ↔
      RFC-013 code table)
- [x] Diagnostic layer fix: in-scope termination failures moved out of E8001, into the proof-failure
      family. E4021 existing; E4022 goes to the user domain via
      `DisproofKind::MeasureNotDecreasing`, with test cases pinned so it cannot be downgraded to ICE
- [x] E2E positive cases: gcd (including "residual obligation not rejected" form), structural
      recursion zero regression
- [x] E2E negative cases: measure invalid → E4022 (with counterexample); no measure → E4021
- [x] E2E positive case (loop, type-side): the type judgment of the `Terminates(n - i)` binding form
      holds (D5 resolved)
- [x] E2E positive case (loop, runtime): the value of `while` as a value (D6 fixed, #409,
      2026-10-04) — `explicit_termination_measure_loop.yx` and `while_block_value.yx` pass in actual
      runs after adding `main()`
- [ ] E2E residual (zero iteration): `while` as a value, with the loop body never executing, takes
      `Void` (static approximation boundary). Contract case
      `02-type-system/termination/while_zero_iteration_void_err.yx`
      (`// expect: runtime-error E6007`); unblocking condition see §Examples loop section "Known
      Residual"
- [ ] E2E awaiting landing: `is_even`-`is_odd` mutual recursion (awaiting SCC)
- [ ] Binary form `Terminates(FnType, m)` (parked). Unblocking condition see §Anchors
- [ ] Return-position multi-parameter predicate + symbolic arguments (clear diagnostics from
      2026-10-02, still not supported). Trigger form / alternative writing / unblocking condition
      see §Implementation Landing Record (2026-10-02, this round of hardening)
- [x] Criterion regression: recursion and loops without refinement annotations are no longer
      rejected by termination check; obligations trigger normally when refinement annotations exist
- [x] Acceptance demo: write a measure-invalid declaration → compile failure with readable
      counterexample (`measure_not_decreasing_err.yx`); passes after fixing
      (`measure_decreasing.yx`)
- [x] Return-point refinement (2026-10-02): return-position formal parameters identified by
      **declaration name**, predicate arguments actually substituted into constraints; undeclared
      free variables report E1001
- [x] Termination strategy 1 restored (2026-10-02): candidates no longer emptied by boundary
      filtering, measure change derived by Direction; new flag loop (strategy 1b)
- [x] Documentation-implementation alignment (2026-10-02): the first loop example and landing record
      in this document, two examples in `language-spec/type-system.md`, the `§2.15` → `§2.9`
      references, RFC-011 loop example

### Implementation Landing Record (2026-09-30 ~ 2026-10-01)

Landed face: unary `Terminates` into the type parser → AST extracts the measure (**not** through the
lossy `MonoType` conversion, which would lose the measure to `Int(64)`) → obligation generation
(recursive call sites, loop back-edges, with path guards) → well-foundedness and strict decrease
two-path SMT decision → diagnostic emission.

The **two gates** for releasing E4022 (both only under-report, never false-positive):

1. **Judged false rather than unable to judge** — the solver gives a counterexample (`Sat`) to
   report; `Unknown` is "unable to judge", handled per §Trade-offs "only give direction, not
   reject".
2. **Measure well-foundedness already proved** — integer `<` is **not** well-founded, measure not
   decreasing **does not** mean non-termination (`gcd` still terminates when `b` is negative). When
   well-foundedness is not proved, even if decrease is judged false, no report, enter residual
   obligation.

The existence of these two gates means that in §Trade-offs, "only give direction, not reject" and
"judged false means error" are no longer contradictory: well-foundedness is exactly the license for
"not decreasing means not terminating".

Existing defects fixed during the landing (all are prerequisite groundwork for this RFC):

- **Predicate definition not registered** (#377-3): the predicate body substitution in
  `PredicateResolver` was long implemented, but short-circuited by an empty table. After
  registration, refinement constraints can be symbolically reasoned for the first time — the premise
  of well-foundedness `b >= 0`.
- **while has no value semantics** (D5): see §Examples loop section.
- **evaluator missing `Neg`/`Pos`/`BitNot`**: inconsistent with the `const_eval` implementation of
  the same name, so that negative literals like `-5` cannot be folded into a constant on all paths
  through this evaluator.

### Implementation Landing Record (2026-10-02, This Round of Hardening)

Four streams of work, each corresponding to actual changes; each item below is a reproducible actual
test conclusion (commands and outputs given per item), not a planned caliber.

#### 1. Return-point refinement: formal parameters identified by declaration name + predicate arguments actually substituted into constraints

Three deviations in the pre-fix (baseline `2a038f93`): the predicate argument of
`-> (r: P(r + 100))` was discarded, degenerating to `P(r)`; the undefined `m` in `-> (r: Eq2(m, r))`
was silently substituted into the return value, and the constraint degenerated to a tautology. Fix:
the parsing layer keeps a single named parenthesis as `NamedParen` (carrying the declared name), the
type checker **only recognizes this name** as the return formal parameter; other out-of-scope free
variables report E1001 one by one and abandon the return-point obligation. Actual test:

| Test case                                                | Pre-fix                                   | Post-fix (actual test)                                                       |
| -------------------------------------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------- |
| `d1: () -> (r: IsPositive(r + 100)) = { return -5 }`     | E4018 (constraint calculated as `-5 > 0`) | `0 error` (constraint `-5 + 100 > 0` holds)                                  |
| `d2: () -> (r: IsPositive(r - 100)) = { return 5 }`      | Silently passed                           | `error [E4018]` (constraint `5 - 100 > 0` judged false, with counterexample) |
| `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` undefined) | Silently passed                           | `error [E1001] Unknown variable: 'm'`                                        |

#### 2. Return-position multi-parameter predicate + symbolic arguments: give clear diagnostics (known gap, not supported)

- **Trigger form** (minimum example):

  ```yaoxiang
  SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }
  f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }
  ```

  Actual test diagnostic:
  `error [E2031] After assigning to 'f', the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven in the proof kernel`
  — the object is the **return position** (`'r'`), no longer the parameter name `'b'` that was
  wrongly hung pre-fix; the constraint text given is the form after substituting the arguments.

- **Reason**: the proof kernel only executes proof function calls "where all arguments can take
  values (literals / already bound variables)"; multi-parameter predicates containing variables get
  no call at the return position, and report when undecidable.
- **Alternative writing** (the diagnostic's help also gives two): change the arguments to
  compile-time-foldable literals (e.g., `SumUpTo(3, r)`), or provide a proof function returning
  `Type` for that predicate to participate in compile-time execution.
- **Unblocking condition**: the proof kernel supports argument substitution for "multi-parameter
  predicate + symbolic arguments" (the same source substitution path as the previous item).
- **Tracking**: no new issue — this section is the only tracking point.

#### 3. Termination Strategy 1 Restored (Defect a / b of #377)

- **Defect a**: the boundary input of strategy 1 was emptied by the filter "boundary must be a loop
  invariant" → candidates were always zero. Fix: strategy 1 now takes the **unfiltered** boundary
  set, with soundness borne by per-assignment verification.
- **Defect b**: the change direction of the candidate measure was written wrong (`m'` always
  constructed as `v + 1`) → judgment always failed. Fix: direction determined by the boundary
  operator (`Lt`/`Le` ⇒ Increasing, `Gt`/`Ge` ⇒ Decreasing), measure change derived by Direction
  (Increasing `Δm = δbound - δv`; Decreasing `Δm = δv - δbound`), `Δm >= 0` rejected on the
  synthesis side, not entering the solver.
- **New flag loop (strategy 1b)**: bare boolean condition + zeroing flag inside the threshold
  guard + variable moving unconditionally toward the threshold each round, measure takes "distance
  to threshold".
- **Soundness caliber unchanged**: rather report "unable to prove" than misjudge termination — the
  advancement of the measure variable must be **unconditional** (at the top of the body), boundary
  displacement takes the **worst end** including the guard, unknown displacement is rejected;
  rebinding the measure variable inside the guard block is also rejected.

Actual test (this worktree binary):

| Corpus                                                                                        | Actual test           |
| --------------------------------------------------------------------------------------------- | --------------------- |
| `02-type-system/termination/rank_function_dual_variable_loop.yx` (`i < j { i += 1; j -= 1 }`) | `0 error`             |
| `02-type-system/termination/flag_loop_guarded_exit.yx`                                        | `0 error`             |
| `06-compile-errors/termination/termination_unproven_err.yx`                                   | still `error [E4021]` |
| 4 negative case corpora (rebinding inside guard block / unknown displacement rebinding etc.)  | all `error [E4021]`   |

#### 4. Documentation-Implementation Alignment

The first loop example in this §Examples and the landing record of this round, the two loop examples
in `docs/src/reference/language-spec/type-system.md`, the loop example in RFC-011 §Termination
Check, and the `§2.15` → `§2.9` reference correction — see the diff of each respective document for
details (`§2.15` is "scoping", the block value rule is in `§2.9` Block Expressions).

#### 5. Not Yet Landed After This Round

SCC collection, lexicographic unfolding, binary form `Terminates(FnType, m)`, mutual recursion E2E,
runtime `while` value retrieval's **zero-iteration residual** (D6 main form fixed), return-position
multi-parameter predicate + symbolic arguments — all remain in the unchecked items of §Stages and
Acceptance, with unblocking conditions given per item.

## Related

- #318 (this RFC's project issue), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement type criterion, §6.1–6.5 measure exploration four strategies, §6.9
  explicit measure, §Open Problems re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path condition collection — infrastructure prerequisite)
- RFC-013 (error code registry and proof-failure semantic family)
