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

RFC-027 §7 sets the criterion for termination checking as **refinement type** (refinement means
entering verification mode), §6.9 gives the language form of explicit measure (the builtin predicate
`Terminates`, which along with `Int` and `Never` belongs to the core primitives). Both are language
design and belong to the host RFC, which is finalized.

This sub-RFC takes over the implementation mechanism: how obligations are generated from computation
structure, how the judgment pipeline is organized, how diagnostics give direction when it cannot be
proven, how mutual recursive measures are shared (SCC optimization), and how error codes are
registered. **It does not restate language semantics**, only the mechanism is defined.

## Motivation

### Why a sub-RFC is needed

The criterion (refinement type entering verification mode) and the form (`Terminates` written at the
type position) have been decided in RFC-027, at the language-level granularity. The remaining
problems are too fine-grained; writing them into the host would bloat the main text:

- Where do obligations come from (recursive call sites, loop back edges, path guards)
- How are tuples returned by the measure compared (lexicographic expansion)
- Whether well-foundedness and strict decrease are two independent obligations or unified
- How do automatic exploration and explicit measures coexist in the same pipeline
- How do mutual recursive cases avoid duplicate writing and duplicate verification of the same
  measure
- How to give a **direction** rather than directly refuse when it cannot be proven

### Trigger

The direction is triggered by #318: non-structural recursion (gcd-style non-direct decrease, mutual
recursion, merge partitioning) exceeds the template sequence of measure exploration (RFC-027
§6.2–6.5 four strategies all take "bounded-typed variables" or "target type + swap operation" as
input), and cannot be rewritten into analyzable iterative patterns without breaking readability. The
re-discussion clause in RFC-027's "Open Questions" section is activated here.

## Proposal

### Relationship with automatic exploration

The explicit measure is **not a separate pipeline**, but the input after exploration fails. After
providing a measure, the same SMT still verifies the same set of obligations; if they don't hold, an
error is reported with a counterexample. The priority of full automation is unchanged: exploration
always runs first, and user intervention only occurs after exploration fails.

This determines that both paths share all downstream mechanisms — obligation generation, SMT
judgment, lexicographic expansion, diagnostic format all have only one implementation.

### Obligation generation

For function `f` and measure `m`, generate two **independent** obligations, expanded at each
recursive call site (including cross-function calls within SCC):

1. **Well-foundedness**: `m(args) >= 0` — the measure lands on the natural numbers; when the measure
   returns a non-natural-number type, take the lower bound of the well-defined order on that type.
   Derived from parameter refinements; enter the residual obligation when it cannot be derived.
2. **Strict decrease**: at each call site `m(callee_args) < m(caller_args)`, judged under the **path
   guard** — the guard comes from the branch condition at the call site, reusing RFC-009a path
   condition collection.

The two are independent rather than unified, because the failure directions differ: well-foundedness
failure means the measure's value range is wrong (e.g., `Int` may be negative), while decrease
failure means the recursive parameter is not moving in the measure's direction. Diagnostics need to
distinguish them to give the correct inspection direction (see the "Diagnostics" section).

**Loops are handled the same way**: replace "call site" with "back edge": on every execution path of
the loop body, `m(next_iteration_state) < m(current_iteration_state)`; the guard comes from the loop
condition and from branches inside the body.

**Lexicographic expansion**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation is expanded by
lexicographic comparison into a disjunction chain — `(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`. The
expansion is done on the **obligation-generation side**; the SMT side stays in the linear fragment
and does not rely on the solver's native support for lexicographic order.

**The measure itself must be evaluable at compile time**: `m` must be a function provable by
constant folding or structural recursion — it is forbidden to recurse the termination problem onto
another unproven function (to prevent infinite regression). Pathological measures are caught by the
existing E4012 (constant recursion too deep) and structural checks.

### Judgment pipeline

```
1. Parameter decrease (structural recursion, strongest path, try first)
2. Measure exploration: four-strategy template sequence (RFC-027 §6.2–6.5), stop when one is found
3. Exploration success → generate obligations → SMT judgment → Proved
4. Exploration failure → check whether the type position provides an explicit measure (Terminates)
     Yes → take that measure to generate obligations → SMT judgment
     No  → E4021 (termination cannot be proven, with suggested inspection direction)
5. Obligation judged false by SMT → E4022 (measure not proven, with counterexample)
```

Levels 1–3 are the established path in RFC-027; this RFC newly adds level 4 and two error codes. The
entire pipeline only runs when refinement type is triggered (RFC-027 §7) — ordinary types without
refinement do not generate any obligations.

### Anchor: unary and binary forms

RFC-027 §6.9 defines two arities; this RFC explains the landing point of each:

| Form                    | Anchor                 | Landing point                                                                                                                                         |
| ----------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding    | Default form at definition — self-recursive functions, loops                                                                                          |
| `Terminates(FnType, m)` | Explicit function type | When the measure needs to explicitly belong to a particular function type (measure defined elsewhere, the same measure serving multiple computations) |

The two are not two different constructs, but two arities of the same predicate: termination
obligations always fall on "the computation annotated by the type position at which the refinement
is located". Mutual recursion **does not** need the binary form — each of the two functions can
write its own unary form, and the sharing relation is identified by SCC (see the next section).

**Implementation status (2026-09-30)**:

- **Unary form is implemented**, with two actual landing points: function **return type position**
  (`gcd: (a: Int, b: Int) -> Terminates(b)` → key `gcd`) and variable **binding position**
  (`acc: Terminates(n - i) = ...` → key `acc`).
- **Binary form `Terminates(FnType, m)` is parked**. Reason: the implementation surface is unary —
  when `Terminates` enters the type parser, it is processed as a unary predicate (the first argument
  is the measure expression), while the binary form's first argument is a **function type**, which
  needs an independent parsing path for "which function type the measure belongs to". There is
  currently no demand side: mutual recursion can be expressed with each function writing a unary
  form + SCC sharing, and SCC has not been implemented yet, so doing the binary form first has no
  comparable consumers. Unblocking condition: after SCC is implemented, if "the same measure serving
  multiple computations" is still needed, add binary parsing. The gcd writing in §Examples of this
  document (binary) is the form to be rewritten.

### SCC: measure sharing optimization

For a group of mutually recursive functions (strongly connected components on the call graph) that
share the same measure, the cross-function edge obligation is
`m_callee(callee_args) < m_caller(caller_args)`; when members share the measure, this degenerates to
same-measure decrease.

**This is an optimization, not a correctness prerequisite**: when not shared, each function writes
its own measure, and each can pass its own closed loop. The value of SCC is to recognize that "this
group uses the same measure", avoiding duplicate writing and duplicate verification.

A new **function-level call graph and SCC collection** is needed — the existing `TypeDepGraph`
records the type-annotation dependencies between variables (the VC trigger in RFC-027 §6.1), which
is a variable-level graph and cannot be reused.

### Examples

#### gcd: non-structural recursion

```yaoxiang
// Measure: ordinary function, unit-testable and reusable
gcd_measure: (a: Int, b: Int) -> Int = { b }

gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

Obligation generation:

1. **Well-foundedness**: `gcd_measure(a, b) >= 0`, i.e., `b >= 0` — directly derived from the
   parameter refinement `NonNegative(b)`
2. **Strict decrease**: the only recursive call site `gcd(b, a % b)`, path guard `b != 0`,
   obligation `gcd_measure(b, a % b) < gcd_measure(a, b)`; after substituting the measure body, it
   expands to `a % b < b`
3. **SMT**: verify that the negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved

#### Loop: anonymous construct gaining denotation

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

The `Terminates(m)` refinement is exactly the value type of the loop body's tail expression, and the
anchor is provided by the binding name `acc` — so the loop **can be denoted**, no longer having the
dead angle of "anonymous construct having no denotation". This also explains the unity of the two
arities: the obligation always falls on the computation annotated by the type position at which the
refinement is located, and there is no difference between functions and loops.

**Note the body-tail of the loop body**: the value of `while` is the value of the loop body block
(spec §2.9); if the body-tail is written as an assignment statement `i = i + 1`, the block value is
`Void`, and `return acc` returning `Void` from `-> Int` is invalid. The above example appends a tail
expression `i` after the assignment to give that value — **this is the only correct form of this
example, and this document does not give the wrong form before the fix** (the form before the fix
and the correction record are in "Implementation status" below).

Obligation: on the back edge `(n - i') < (n - i)`, guard `i < n`, substitute `i' = i + 1` to get
`1 > 0`, which is a tautology → Proved.

**Implementation status (2026-10-01, revised 2026-10-02): the type side of this loop form has been
wired through; the runtime value taking is still not working (known defect D6, see below)**.

The premise is that `while` has value semantics: this form was once blocked (D5) — the `while`
expression was **constantly** typed as `Void` at the time, with no value type to carry the
refinement, so `Terminates` could not attach to any computation. This defect has been fixed:
according to spec §2.9 "the value of all `{}` blocks is given by the tail expression, with no
exception", the loop body is also a `{}` block, so the value of `while` = the value of the loop body
block.

This rule also exposes the **type error in the early form of this example**: the form before the fix
wrote the loop body-tail as an assignment statement (such as `while i < n { i = i + 1 }`), so the
block value would be `Void` (spec §2.9 same rule), and `return acc` returning `Void` from `-> Int`
is invalid. The fix is to make the body-tail give a value: append a tail expression `i` after the
assignment — the example above is already the fixed form, and this document no longer retains a
reproducible wrong form.

The back edge obligation and the recursive back edge are **isomorphic**
(`m[variable := back_edge_value] < m`), and the loop condition is the source of the well-foundedness
lower bound — `i < n` derives `n - i > 0`. Actual test: `n - i` is judged Proved — this is a
**type-side** conclusion. At runtime this form currently **does not** return `4`: the value obtained
by the binding is `Void`, and any use of that value crashes at runtime (see "Known defect D6"
below). The previous statement in this document that "it returns 4 when `n = 4`" is an
**unreproduced false conclusion**, deleted on 2026-10-02.

The **function form** (return type position unary measure, see §Examples gcd) and the **loop form**
have both been wired through; the original refinement variable gating + E4021 on the loop side is
retained (the exploration path when there is no explicit measure).

**Known defect D6 (runtime `while` value taking; registered 2026-10-02)**. Type-side success does
not imply runtime usability: the example above `check` 0 error, but **at runtime** the value
obtained by the binding is `Void`, and any use of that value crashes at runtime.

Minimum reproduction (the loop form from §Examples of this document + an explicit `main()` call at
the end — script mode does not implicitly call `main()`):

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

- **Independent of iteration count**: `loop(0)` (zero iterations) reports `Eq: Void vs Int(0)` — the
  runtime `while` expression did not produce a value at all, not "the last iteration took the wrong
  value".
- **The impact is not limited to the `Terminates` form**: any writing that uses `while` as a value
  hits the problem. Actual test: after `x: Int = while i < 3 { i = i + 1; i }`, using `x` at runtime
  → `Eq: Void vs Int(3)`. The type-side criterion, obligation generation and diagnostics are all
  normal; the defect is only in the runtime value taking path.
- **Unblocking condition**: at runtime, produce the value of the `while` expression as the value of
  the loop body block's tail expression (`for` already has this caliber on the type side), and have
  `tests/yaoxiang/02-type-system/explicit_termination_measure_loop.yx` and `while_block_value.yx`
  pass after adding the `main()` call — these two corpora currently **deliberately do not execute**
  `main()` (only doing `check`), to avoid dyeing the corpora red with known defects.
- **Tracking**: issue [#409](https://github.com/ChenXu233/YaoXiang/issues/409) has been opened
  (runtime `while` value taking); this section retains the minimum reproduction and unblocking
  condition as the body basis of that issue (the en mirror is the responsibility of the automatic
  translation bot, no manual changes).

#### Mutual recursion: SCC shared measure

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

The two functions each carry the unary form `Terminates(nat)`. After SCC collection, it is
identified that both share the same measure, and the cross-function edge obligation
`nat(n - 1) < nat(n)` is a tautology under the guard `n != 0`; the two functions are closed in a
single verification pass.

Without SCC identification, each function verifying the same obligation still passes — it just
verifies twice. This confirms that SCC's positioning is an optimization.

### Diagnostics

**When well-foundedness cannot be derived, do not refuse directly, but suggest an inspection
direction.** This must be distinguished from the diagnostic of a decrease obligation failure: the
former points to the measure's value range, the latter points to the recursive parameter. The
suggested directions for the three kinds of failure:

| Failure                            | Suggested direction                                                                                               |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Well-foundedness cannot be derived | Whether the measure holds a lower bound on all possible values (e.g., whether an `Int` measure needs `>= 0`)      |
| Strict decrease judged false       | Whether the recursive parameter is really moving in the measure's decreasing direction; attach SMT counterexample |
| No measure and exploration failed  | Hint that a name can be bound to the computation and a measure provided at the type position (RFC-027 §6.9)       |

Counterexample presentation follows the RFC-013 diagnostic message specification. Under non-linear
path guards, the Sat counterexample may not be intuitive — recorded as a known limitation, to be
iterated with RFC-013.

### Error codes

Aligned with the E4xxx proof failure family (E4018 refinement predicate violation, E4020 proof
function required):

| Proposed code | Name                         | Trigger                                                                                                  |
| ------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------- |
| E4021         | Termination cannot be proven | Exploration failed and no explicit measure at the type position (hint to provide a `Terminates` measure) |
| E4022         | Measure not proven           | Measure obligation judged false by SMT (with counterexample)                                             |

The final number is subject to the actual RFC-013 registry at implementation time (segment legality
is guaranteed by the build.rs threshold).

### Existing defects in the diagnostic layer

In the current implementation, termination failures within scope are reported as **E8001 "Internal
Compiler Error"** (`Unproven` is formatted as ICE) — termination checking is not ICE, and occupying
that code position both misleads users and obscures real failures. This RFC fixes this at the same
time: within-scope termination failures go to E4021/E4022, and the ICE code position is returned to
real internal errors.

### Compiler changes

| Component                           | Change                                                                                                                                                                                                                                    |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`   | Interface unification (exploration path and explicit measure path share obligation generation and judgment); Z3 access (production injects `default_solver()` in `checker.rs`, wasm does not inject; unit tests can inject a stub solver) |
| Function call graph + SCC (**new**) | No cross-function call graph exists in the repo — `TypeDepGraph` is a variable-level type dependency, cannot be reused. Create a new function-level call graph and SCC collection, for mutual recursive measure sharing.                  |
| Obligation generation               | Newly added: well-foundedness / strict decrease two obligations, path guard injection, lexicographic expansion                                                                                                                            |
| Proof pipeline (RFC-009a / #292)    | Reuse the `ConstExpr → SMTLib` chain and `Mod` etc. operator mapping, no backend change                                                                                                                                                   |
| `util/diagnostic/codes/e4xxx.rs`    | Newly add E4021/E4022 registration (via the RFC-013 registry, build.rs threshold takes effect)                                                                                                                                            |
| locales ×6                          | Six language templates for the two new codes                                                                                                                                                                                              |
| Diagnostic layer                    | Within-scope termination failures migrated out of E8001, into the proof failure family                                                                                                                                                    |

### Backward compatibility

- Programs that were previously rejected for termination checking but **had no refinement
  annotation**: under the new criterion, they do not enter verification mode and pass directly — an
  intentional relaxation, only relaxing and never tightening.
- Programs that were previously rejected for termination checking and **had** a refinement
  annotation: pass after adding a measure.
- Structural recursion positive cases, existing termination tests: expected output unchanged.

## Trade-offs

### Advantages

- **Zero new syntax**: the measure is written at the type position, reusing the refinement predicate
  application mechanism; no `decreases`-style syntax position.
- **Proof-domain behavior aligned**: the termination domain supplements a fallback channel
  consistent with the correctness domain, but the landing point differs — propositions that the
  correctness domain cannot prove are written as proof functions in the **body**, while measures
  that the termination domain's exploration cannot find are declared at the **type position**. Both
  share the same mechanism (both are refinement type applications).
- **Reuse infrastructure**: the obligation is linear arithmetic plus path guards, taking the
  already-connected pipeline of #292, with no new backend.
- **Loop gains denotation**: the binding name is the anchor; loops and functions are completely
  isomorphic in obligation generation, with no need to design special cases for loops.

### Disadvantages and risks

- **The amount of preceding infrastructure work is no less than the "wiring" itself**: a
  function-level call graph and SCC collection need to be built, and Z3 has not been wired into the
  production pipeline. The SCC part can be deferred (it is an optimization); the call graph part is
  of the same source as SCC, and if deferred, mutual recursion can only have each function write its
  own measure — still passable, just with duplicate verification.
- **Explicit measure is a writing burden on the low-frequency path**: the two-piece set (measure
  function + type-position declaration) is more verbose than inline annotation. Accepted — the
  fallback is a low-frequency path, and in exchange the measure is reusable and unit-testable.
- **Well-foundedness obligation noise**: when the measure returns `Int`, `>= 0` must be proven every
  time. Mitigation: when the parameter refinement already gives a lower bound, it is derived
  automatically; only when it cannot be derived does it enter the residual obligation, and only a
  direction is given, not a refusal.
- **Counterexample quality**: the Sat counterexample under non-linear guards is not intuitive. Known
  limitation.
- **`Terminates` is the only predicate whose body is written by the compiler**: deviates from the
  purity of "all predicate bodies can be user-written". Reason: its assertions (measure decrease at
  each call site / back edge) are embedded in the computation structure, and user-written predicates
  cannot refer to the function body or loop body. The built-in surface converges to one name, with
  zero new mechanism.

## Alternatives

- **`with decreases (b)` inline annotation syntax**: an earlier proposal in this issue, withdrawn.
  "No **annotation syntax** port left for termination checking" is the finalized decision of
  RFC-027; inline annotation would turn unsupported termination modes into syntax positions rather
  than type positions, going against the worldview of "everything is a YaoXiang function, everything
  is verified by the type checker".
- **Independent proof function (`gcd_proof` returns `Terminates(f, m)`, discovered by return type
  scanning)**: earlier design. Voided — it introduces four kinds of complexity: "discovery
  mechanism", "naming convention", "multiple candidates take the first", and "how names inside the
  proof function body are resolved", all because the proof was placed outside the body. After moving
  it to the type position, all four kinds of complexity disappear.
- **Only let the measure enter the type (`Terminates(m)`), cancel the binary form**: shorter, but
  loses the expression position for "explicitly specifying the measure's belonging" (when the
  measure is defined elsewhere, the same measure serves multiple computations and has nowhere to put
  it). The two arities are two arities of the same predicate, and the retention cost is nearly zero.
- **No fallback, requiring users to rewrite into analyzable iterative patterns**: i.e., the current
  state. gcd / merge partitioning / mutual recursion cannot be rewritten without breaking
  readability — this is exactly the trigger condition of the re-discussion clause.

## Non-goals

- No new syntax / keywords / annotation positions.
- No generalization of automatic measure synthesis (the four strategies maintain the plan
  established by RFC-027; beyond scope go to explicit measure). General measure inference lies on
  the undecidable side of "discovery"; only template boundaries can be agreed upon, completeness
  cannot be promised.
- No joint solving with RFC-009a borrowed propositions (judged independently, sharing backend).
- No totality checking at the dependent-type level.
- No forcing the measure to return a natural-number type (not pursuing Lean's `WellFoundedRelation`
  type-class mechanism) — the measure's return type is not restricted, and well-foundedness is left
  to refinement derivation or SMT as an independent obligation.

## Phases and acceptance

- [x] Obligation generation: well-foundedness / strict decrease, path guard injection
- [x] Explicit measure wiring: unary form anchor resolution (return type position / binding
      position) + type position measure extraction
- [x] SMT judgment wiring (reuse the #292 pipeline, production pipeline injects Z3 — the previous
      injection point was never called)
- [ ] Function-level call graph + SCC collection (measure sharing optimization). Unblocking
      condition: a need arises for "the same measure serving multiple computations" or mutual
      recursion to avoid duplicate verification
- [ ] Lexicographic expansion (when the measure returns a tuple). Unblocking condition: a real use
      case for a tuple-returning measure appears
- [x] E4021/E4022 registration + six-language locales (consistent across three parties: codes ↔
      locales ↔ RFC-013 code table)
- [x] Diagnostic-layer fix: within-scope termination failures migrated out of E8001, into the proof
      failure family. E4021 exists; E4022 is routed through `DisproofKind::MeasureNotDecreasing` to
      the user domain, with use cases pinned so it cannot be downgraded to ICE
- [x] E2E positive case: gcd (including the "residual obligation not refused" form), structural
      recursion zero regression
- [x] E2E negative case: measure not proven → E4022 (with counterexample); no measure → E4021
- [x] E2E positive case (loop · type side): type judgment of the `Terminates(n - i)` binding form
      holds (D5 resolved)
- [ ] E2E positive case (loop · runtime): runtime value taking of `while` as a value (**known defect
      D6**). Unblocking condition: see §Examples loop section
- [ ] E2E to be wired: `is_even`-`is_odd` mutual recursion (waiting for SCC)
- [ ] Binary form `Terminates(FnType, m)` (parked). Unblocking condition: see §Anchor
- [ ] Return-position multi-parameter predicate + symbolic arguments (clear diagnosis since
      2026-10-02, still not supported). Trigger form / alternative writing / unblocking condition:
      see §Implementation landing record (2026-10-02, this round of hardening)
- [x] Criterion regression: recursion and loops without refinement annotation are no longer rejected
      by termination checking; obligations trigger as usual when there is a refinement annotation
- [x] Acceptance demo: write a declaration where the measure does not hold → compilation fails and
      the counterexample is readable (`measure_not_decreasing_err.yx`); passes after fixing it
      (`measure_decreasing.yx`)
- [x] Return-point refinement (2026-10-02): return-position formal parameter identified by
      **declaration name**, predicate arguments really substituted into the constraint; undeclared
      free variables report E1001
- [x] Termination strategy 1 restored to working order (2026-10-02): candidates are no longer
      filtered to empty by boundaries, measure change is derived by Direction; newly added flag loop
      (strategy 1b)
- [x] Documentation and implementation alignment (2026-10-02): the first loop example in this
      document and the implementation record, the two examples in `language-spec/type-system.md`,
      the reference from `§2.15` to `§2.9`, and the loop example in RFC-011

### Implementation landing record (2026-09-30 ~ 2026-10-01)

Implemented side: unary `Terminates` enters the type parser → AST extracts the measure (**not**
through the lossy `MonoType` conversion, which would lose the measure as `Int(64)`) → obligation
generation (recursive call sites, loop back edges, with path guards) → well-foundedness and strict
decrease two-way SMT judgment → diagnostic emission.

The **two gates** for releasing E4022 (both only under-report, never over-report):

1. **Judged false rather than unable to judge** — only report when the solver gives a counterexample
   (`Sat`); `Unknown` belongs to "cannot be judged" and is handled according to §Trade-offs's "only
   give direction, not refuse".
2. **Measure well-foundedness has been proven** — integer `<` is **not** well-founded, so measure
   not decreasing **does not** equal non-termination (`gcd` still terminates when `b` is negative).
   When well-foundedness is not proven, even if the decrease is judged false, it is not reported,
   and it enters the residual obligation.

The existence of these two gates makes "only give direction, not refuse" and "judged false, then
error" in §Trade-offs no longer contradictory: well-foundedness is precisely the license for "not
decreasing implies non-termination".

Existing defects fixed during landing (all are part of the preceding infrastructure for this RFC):

- **Predicate definition not registered** (#377-3): the `PredicateResolver`'s predicate-body
  substitution was already implemented, but short-circuited by an empty table. After adding the
  registration, refinement constraints are symbolically reasoned for the first time — the premise of
  well-foundedness `b >= 0`.
- **`while` has no value semantics** (D5): see §Examples loop section.
- **evaluator missing `Neg`/`Pos`/`BitNot`**: inconsistent with the same-named implementation in
  `const_eval`, causing negative literals like `-5` to fail to fold out constants on every path that
  goes through this evaluator.

### Implementation landing record (2026-10-02, this round of hardening)

Four paths of work, each corresponding one-to-one to actual changes; every item below is a
reproducible actual-test conclusion (commands and outputs given per item), not a planned statement.

#### 1. Return-point refinement: formal parameter identified by declaration name + predicate arguments really substituted into the constraint

Three deviations before the fix (baseline `2a038f93`): the predicate argument of
`-> (r: P(r + 100))` was discarded and degenerated to `P(r)`; the undefined `m` in
`-> (r: Eq2(m, r))` was silently substituted into the return value, and the constraint degenerated
to a tautology. Fix: the parsing layer keeps the single named parenthesis as `NamedParen` (carrying
the declared name), and the type checker **only recognizes this name** as the return value's formal
parameter; other free variables not in scope are reported one by one with E1001, and the
return-point obligation is given up. Actual test:

| Use case                                                 | Before fix                               | After fix (actual test)                                                         |
| -------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------- |
| `d1: () -> (r: IsPositive(r + 100)) = { return -5 }`     | E4018 (constraint evaluated as `-5 > 0`) | `0 error` (constraint `-5 + 100 > 0` holds)                                     |
| `d2: () -> (r: IsPositive(r - 100)) = { return 5 }`      | Silently passes                          | `error [E4018]` (constraint `5 - 100 > 0` is judged false, with counterexample) |
| `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` undefined) | Silently passes                          | `error [E1001] Unknown variable: 'm'`                                           |

#### 2. Return-position multi-parameter predicate + symbolic arguments: a clear diagnosis is given (known gap, not yet supported)

- **Trigger form** (minimum example):

  ```yaoxiang
  SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }
  f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }
  ```

  Actual-test diagnosis:
  `error [E2031] After assigning to 'f', the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven in the proof kernel`
  — the object is the **return position** (`'r'`), no longer the formal parameter name `'b'` that
  was wrongly attached before the fix; the constraint text gives the form after the argument
  substitution.

- **Reason**: the proof kernel only executes "proof function calls whose arguments are all valueable
  (literals / already bound variables)"; multi-parameter predicates containing variables cannot get
  a call at the return position, and if it cannot be judged, it is reported.
- **Alternative writing** (the help of the diagnosis gives two): change the arguments to literals
  that can be folded at compile time (such as `SumUpTo(3, r)`), or provide a proof function that
  returns `Type` for this predicate to participate in compile-time execution.
- **Unblocking condition**: the proof kernel supports argument substitution for "multi-parameter
  predicate + symbolic arguments" (the same-source substitution path as the previous item).
- **Tracking**: no new issue opened — this section is the only tracking place.

#### 3. Termination strategy 1 restored to working order (defects a / b of #377)

- **Defect a**: the boundary input of strategy 1 was filtered to empty by a "boundary must be a loop
  invariant" filter → candidates are always zero. Fix: strategy 1 now consumes the **unfiltered**
  boundary set, and soundness is borne by per-assignment verification.
- **Defect b**: the change direction of the candidate measure was written wrong (`m'` was always
  constructed as `v + 1`) → judgment always failed. Fix: the direction is determined by the boundary
  operator (`Lt`/`Le` ⇒ Increasing, `Gt`/`Ge` ⇒ Decreasing), and the measure change is derived by
  Direction (Increasing `Δm = δbound - δv`; Decreasing `Δm = δv - δbound`); `Δm >= 0` is denied
  directly on the composition side, without entering the solver.
- **Newly added flag loop (strategy 1b)**: bare boolean condition + flag cleared inside a threshold
  guard + variable unconditionally moves toward the threshold each round; the measure takes
  "distance to the threshold".
- **Soundness caliber unchanged**: better to report "cannot be proven" than to misjudge termination
  — the advance of the measure variable must be **unconditional** (top level of the body), the
  boundary displacement takes the **most unfavorable end** including the guard, and if the
  displacement is unknown, it is refused; rebinding the measure variable inside the guard block is
  also refused.

Actual test (this worktree binary):

| Corpus                                                                                              | Actual test           |
| --------------------------------------------------------------------------------------------------- | --------------------- |
| `02-type-system/rank_function_dual_variable_loop.yx` (`i < j { i += 1; j -= 1 }`)                   | `0 error`             |
| `02-type-system/flag_loop_guarded_exit.yx`                                                          | `0 error`             |
| `06-compile-errors/termination_unproven_err.yx`                                                     | still `error [E4021]` |
| 4 reverse-example corpora (rebinding inside the guard block / unknown-displacement rebinding, etc.) | all `error [E4021]`   |

#### 4. Documentation and implementation alignment

The first loop example in §Examples of this document and this implementation record, the two loop
examples in `docs/src/reference/language-spec/type-system.md`, the loop example in RFC-011
§Termination Checking, and the reference correction from `§2.15` to `§2.9` — details are in the diff
of each of those documents (`§2.15` is "Scope"; the block-value rule lives in `§2.9` Block
Expression).

#### 5. Not yet wired up after this round

SCC collection, lexicographic expansion, binary form `Terminates(FnType, m)`, mutual-recursion E2E,
runtime `while` value taking (D6), return-position multi-parameter predicate + symbolic arguments —
all remain in the unchecked items of §Phases and acceptance, with the unblocking conditions written
alongside the items.

## Related

- #318 (the issue that initiated this RFC), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement-type criterion, §6.1–6.5 four measure-exploration strategies, §6.9
  explicit measure, §Open Questions re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path condition collection — infrastructure prerequisite)
- RFC-013 (error-code registry and proof-failure semantic family)
