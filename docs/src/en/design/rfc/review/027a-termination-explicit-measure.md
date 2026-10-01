---
title: 'RFC-027a: Explicit Measure for Termination Check'
status: 'Under Review'
author: '晨煦'
created: '2026-09-14'
updated: '2026-10-01'
issue: '#318'
impl_status: 'partial'
---

# RFC-027a: Explicit Measure for Termination Check

## Summary

RFC-027 §7 sets the criterion for termination check as **refinement type** (refinement enters
verification mode), and §6.9 specifies the syntactic form of explicit measure (the built-in
predicate `Terminates`, belonging to the same core primitives as `Int` and `Never`). Both are
language design, belong to the host RFC, and are finalized.

This sub-RFC carries forward the landing mechanism: how obligations are generated from computational
structure, how the judgment pipeline is orchestrated, how diagnostics give direction when the proof
cannot be derived, how measures are shared under mutual recursion (SCC optimization), and how error
codes are registered. **Language semantics are not repeated**; only the mechanism is decided.

## Motivation

### Why a sub-RFC is needed

The criterion (refinement type entering verification mode) and the form (`Terminates` written at the
type position) are decided in RFC-027, with language-level granularity. The remaining issues have
too fine a granularity; writing them into the host would bloat the main text:

- Where obligations come from (recursive call sites, loop back-edges, path guards)
- How to compare when the measure returns a tuple (lexicographic unfolding)
- Whether well-foundedness and strict decrease are two independent obligations or one unified
- How automatic exploration and explicit measure coexist in the same pipeline
- How to avoid repeated writing and repeated verification of the same measure in mutual recursion
- How to give **direction** when the proof cannot be derived, rather than outright rejection

### Trigger

The direction is triggered by #318: non-structural recursion (gcd-like non-directly-decreasing,
mutual recursion, merge partitioning) goes beyond the template sequence of measure exploration (all
four strategies in RFC-027 §6.2–6.5 take "variable of bounded type" or "target type + exchange
operation" as input), and cannot be rewritten into an analyzable iteration pattern without breaking
readability. The re-discussion clause in the "Open Questions" section of RFC-027 is activated here.

## Proposal

### Relationship with automatic exploration

The explicit measure is **not a separate pipeline**, but input after exploration fails. Once the
measure is given, the same SMT runs the same set of obligations; if they fail, an error is reported
with a counterexample. Full automation priority is preserved: exploration always runs first, and
user intervention occurs only after exploration fails.

This decision makes the two paths share the entire downstream mechanism—obligation generation, SMT
judgment, lexicographic unfolding, and diagnostic format all have only one implementation.

### Obligation generation

For function `f` and measure `m`, two **independent** obligations are generated, expanded at each
recursive call site (including cross-function calls within an SCC):

1. **Well-foundedness**: `m(args) >= 0`—the measure falls on natural numbers; when the measure
   returns a non-natural-number type, take the lower bound of the appropriate order on that type.
   Derived from argument refinements; enters residual obligation when derivation fails.
2. **Strict decrease**: at each call site `m(callee_args) < m(caller_args)`, judged under the **path
   guard**—the guard comes from the branch condition where the call site is located, reusing
   RFC-009a path condition collection.

The two are independent rather than unified, because the failure directions differ: well-foundedness
failure indicates the measure's range is wrong (e.g., `Int` can be negative), while decrease failure
indicates the recursive arguments do not move in the right direction. Diagnostics must distinguish
between them to give the right checking direction (see "Diagnostics" section).

**The same applies to loops**, with "call site" replaced by "back-edge": on every execution path of
the loop body `m(next-state) < m(current-state)`, the guard comes from the loop condition and the
in-body branches.

**Lexicographic unfolding**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation comparison
expands according to lexicographic order into a disjunction
chain—`(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`. The unfolding is done on the **obligation
generation side**; the SMT side stays in a linear fragment and does not depend on the solver's
native support for lexicographic order.

**The measure itself must be evaluable at compile-time**: `m` must be a function provable via
constant folding or structural recursion—it is forbidden to recursively push the termination problem
to another unproven function (preventing infinite regress). Pathological measures are caught by the
existing E4012 (constant recursion too deep) and structural checks.

### Judgment pipeline

```
1. Structural recursion (strongest path, try first)
2. Measure exploration: four-strategy template sequence (RFC-027 §6.2–6.5), stop when one is found
3. Exploration success → generate obligations → SMT judgment → Proved
4. Exploration failure → check whether an explicit measure is given at the type position (Terminates)
     Yes → use that measure to generate obligations → SMT judgment
     No  → E4021 (termination unprovable, with suggested checking direction)
5. Obligation falsified by SMT → E4022 (measure does not hold, with counterexample)
```

Steps 1–3 are the path already fixed by RFC-027; this RFC adds step 4 and two error codes. The whole
pipeline runs only when a refinement type is triggered (RFC-027 §7)—plain types without refinement
generate no obligations.

### Anchor: unary and binary forms

RFC-027 §6.9 fixes the two arities; this RFC explains the landing point of each:

| Form                    | Anchor                     | Landing Point                                                                                                                                      |
| ----------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the bound location | Default form at definition site—self-recursive functions, loops                                                                                    |
| `Terminates(FnType, m)` | Explicit function type     | When the function type to which the measure belongs must be explicitly named (measure defined elsewhere, one measure serves multiple computations) |

The two are not two distinct constructs, but two arities of the same predicate: the termination
obligation always lands on "the computation annotated by the type position where the refinement
resides." Mutual recursion **does not** need the binary form—each of the two functions writes its
own unary form, and the sharing relation is identified by SCC (see the next section).

**Landing status (2026-09-30)**:

- **The unary form is landed**, with two verified landing points: the function's **return type
  position** (`gcd: (a: Int, b: Int) -> Terminates(b)` → key `gcd`) and the variable's **binding
  position** (`acc: Terminates(n - i) = ...` → key `acc`).
- **The binary form `Terminates(FnType, m)` is parked**. Reason: the implementation surface is
  unary—`Terminates` is processed as a unary predicate when entering the type parser (the first
  actual argument is the measure expression), while the binary form's first actual argument is a
  **function type**, requiring an independent resolution path for "which function type the measure
  belongs to." There is currently no consumer: mutual recursion is fully expressible via "each side
  writes its own unary form + SCC sharing," and SCC has not landed yet, so building the binary form
  first has no comparable consumer. Unblocking condition: after SCC lands, if "one measure serving
  multiple computations" is still needed, add the binary resolution path. The gcd form (binary) in
  §Examples of this file is the form to be rewritten.

### SCC: measure sharing optimization

If a group of mutually recursive functions (strongly connected components on the call graph) share
the same measure, the obligation on cross-function edges is
`m_callee(callee_args) < m_caller(caller_args)`, which degenerates into a same-measure decrease when
members share the measure.

**This is an optimization, not a correctness prerequisite**: without sharing, each function writes
its own measure and closes its own loop, which still passes. The value of SCC is recognizing that
"this group uses the same measure," avoiding repeated writing and repeated verification.

A **function-level call graph and SCC collection** needs to be created—the existing `TypeDepGraph`
records type-annotation dependencies between variables (the VC trigger in RFC-027 §6.1), which is a
variable-level graph and cannot be reused.

### Examples

#### gcd: non-structural recursion

```yaoxiang
// Measure: an ordinary function, can be unit-tested and reused
gcd_measure: (a: Int, b: Int) -> Int = { b }

gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

Obligation generation:

1. **Well-foundedness**: `gcd_measure(a, b) >= 0`, i.e., `b >= 0`—derived directly from the
   formal-parameter refinement `NonNegative(b)`
2. **Strict decrease**: the sole recursive call site `gcd(b, a % b)`, path guard `b != 0`,
   obligation `gcd_measure(b, a % b) < gcd_measure(a, b)`; substituting the measure body gives
   `a % b < b`
3. **SMT**: verify that the negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved

#### Loop: anonymous construct obtains a referent

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

The `Terminates(m)` refinement refines the value type of the loop body's tail expression; the anchor
is provided by the binding name `acc`—so the loop is **referable** and no longer has the dead angle
of "anonymous constructs without a referent." This also explains the unity of the two arities: the
obligation always lands on the computation annotated by the type position where the refinement
resides; there is no difference between functions and loops.

Obligation: on the back-edge `(n - i') < (n - i)`, with guard `i < n` and substitution `i' = i + 1`
yields `1 > 0`, a tautology → Proved.

**Landing status (2026-10-01): this loop form is end-to-end working**.

The prerequisite is that `while` has value semantics: this form was once blocked (D5)—the `while`
expression was then **always** typed as `Void`, with no value type to carry a refinement, and
`Terminates` could not attach to any computation. This defect has been fixed: per spec §2.15, "the
value of every `{}` block is given by the tail expression, without exception," and the loop body is
also a `{}` block, so the value of `while` = the value of the loop body block.

Along with this, a **type error in the example itself** must be fixed: when the body tail is an
assignment statement, the block value is `Void` (spec §2.15, same rule), so the original example's
`while i < n { i = i + 1 }` has loop value `Void`, and `return acc` returning `Void` from `-> Int`
is not legal. The correct form requires the body tail to give a value:

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

The back-edge obligation is **isomorphic** to the recursive back-edge
(`m[var := back-edge value] < m`), and the loop condition is the source of the well-foundedness
lower bound—`i < n` derives `n - i > 0`. Verified: `n - i` judges Proved, returns 4 when `n = 4`.

The two landed forms are the **function form** (unary measure at the return type position, see
§Examples gcd) and the **loop form**; on the loop side, the existing refinement-variable gating +
E4021 is preserved (the exploration path when no explicit measure is given).

#### Mutual recursion: SCC shares the measure

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

Both functions carry the unary form `Terminates(nat)`. After SCC collection, the two are identified
as sharing the same measure; the cross-function-edge obligation `nat(n - 1) < nat(n)` is a tautology
under guard `n != 0`, and both functions close the loop with a single verification.

Without SCC identification, each function still passes when verifying the same obligation on its
own—only once more. This confirms that SCC is positioned as an optimization.

### Diagnostics

**When well-foundedness cannot be derived, do not reject outright; instead, suggest a checking
direction.** This must be distinguished from the diagnostics of a failed decrease obligation: the
former points to the measure's range, the latter to the recursive arguments. Suggested directions
for each of the three failure types:

| Failure                            | Suggested Direction                                                                                               |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Well-foundedness cannot be derived | Whether the measure holds a lower bound on its possible values (e.g., whether an `Int` measure needs `>= 0`)      |
| Strict decrease falsified          | Whether the recursive arguments actually move in the decreasing direction of the measure; with SMT counterexample |
| No measure and exploration failed  | Hint that a name can be bound for the computation and a measure provided at the type position (RFC-027 §6.9)      |

Counterexample presentation follows the RFC-013 diagnostic message specification. Under nonlinear
path guards, Sat counterexamples may be unintuitive—recorded as a known limitation, to be iterated
along with RFC-013.

### Error codes

Aligned with the E4xxx proof-failure family (E4018 refinement predicate violation, E4020 proof
function required):

| Proposed Code | Name                   | Trigger                                                                                                |
| ------------- | ---------------------- | ------------------------------------------------------------------------------------------------------ |
| E4021         | Termination unprovable | Exploration failed and no explicit measure at the type position (hint to provide `Terminates` measure) |
| E4022         | Measure does not hold  | Measure obligation falsified by SMT (with counterexample)                                              |

Final numbering is subject to the actual RFC-013 registry at implementation time (code range
validity is guaranteed by the build.rs threshold).

### Existing defect in the diagnostic layer

In the current implementation, in-scope termination failures are reported in the form of **E8001
"internal compiler error"** (`Unproven` is formatted as an ICE)—termination check is not an ICE, and
occupying that code position both misleads users and masks real faults. This RFC fixes it as well:
in-scope termination failures go through E4021/E4022, and the ICE code position is returned to
genuine internal errors.

### Compiler changes

| Component                           | Change                                                                                                                                                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`   | Interface unification (exploration path and explicit-measure path share obligation generation and judgment); wire in Z3 (production pipeline injection, `with_z3` currently only exists in unit tests)              |
| Function call graph + SCC (**new**) | No cross-function call graph in the entire repo—`TypeDepGraph` is a variable-level type dependency, cannot be reused. Build a new function-level call graph and SCC collection for mutual-recursion measure sharing |
| Obligation generation               | New: well-foundedness / strict decrease two obligations, path-guard injection, lexicographic unfolding                                                                                                              |
| Proof pipeline (RFC-009a / #292)    | Reuse `ConstExpr → SMTLib` chain and `Mod` etc. operator mapping, no backend changes                                                                                                                                |
| `util/diagnostic/codes/e4xxx.rs`    | Register new E4021/E4022 (via RFC-013 registry, build.rs threshold takes effect)                                                                                                                                    |
| locales ×6                          | Six-language templates for the two new codes                                                                                                                                                                        |
| Diagnostic layer                    | Move in-scope termination failures out of E8001, route them through the proof-failure family                                                                                                                        |

### Backward compatibility

- Programs previously rejected by termination check but **without refinement annotations**: under
  the new criterion, they do not enter verification mode and compile directly—this is an intentional
  relaxation, only relaxing, not tightening.
- Programs previously rejected by termination check and **with** refinement annotations: they pass
  after supplementing the measure.
- Structural-recursion positive cases and existing termination tests: expected output is unchanged.

## Trade-offs

### Advantages

- **Zero new syntax**: the measure is written at the type position, reusing the refinement-predicate
  application mechanism; no `decreases`-style syntactic position.
- **Proof-domain behavior alignment**: the termination domain adds a fallback channel consistent
  with the correctness domain, but with a different landing point—in the correctness domain, an
  unprovable proposition is written as a **proof function in the body**; in the termination domain,
  an unprovable measure is declared at the **type position**. Both mechanisms are the same
  (refinement-type application).
- **Reuse of infrastructure**: obligations are linear arithmetic plus path guards, going through the
  #292-already-wired pipeline, with no new backend.
- **Loops obtain a referent**: the binding name is the anchor, and loops are completely isomorphic
  to functions in obligation generation, requiring no special case for loops.

### Disadvantages and risks

- **Prerequisite infrastructure is no less than "wiring"**: the function-level call graph and SCC
  collection need to be built from scratch, and Z3 is not yet wired into the production pipeline.
  The SCC part can be deferred (it is an optimization); the call-graph part shares its source with
  SCC, and deferring it means mutual recursion can only have each side write its own measure—still
  passable, just with repeated verification.
- **Explicit measure imposes a writing burden on a low-frequency path**: the two-piece set (measure
  function + type-position declaration) is more verbose than inline annotation. Accepted—the
  fallback is a low-frequency path, and in exchange the measure is reusable and unit-testable.
- **Noise from the well-foundedness obligation**: when the measure returns `Int`, every call
  requires proving `>= 0`. Mitigation: when argument refinement already gives a lower bound, it is
  automatically derived; only when derivation fails does it enter a residual obligation, and only
  the direction is given, not a rejection.
- **Counterexample quality**: Sat counterexamples under nonlinear guards are unintuitive. Known
  limitation.
- **`Terminates` is the only predicate whose body is generated by the compiler**: deviates from the
  purity of "all predicate bodies are user-writable." Reason: its assertion (decrease of the measure
  at every call site / back-edge) lives in the computational structure, and the user-written
  predicate cannot refer to the function body or loop body. The built-in surface converges to a
  single name, with zero new mechanism.

## Alternatives

- **`with decreases (b)` inline-annotation syntax**: an early proposal in this issue, withdrawn.
  "Termination check does not leave an **annotation syntax** opening" is a finalized decision in
  RFC-027; inline annotation would turn unsupported termination modes into syntactic positions
  rather than type positions, contrary to the worldview of "everything is a YaoXiang function,
  everything is verified by the type checker."
- **Independent proof function (`gcd_proof` returns `Terminates(f, m)`, discovered by scanning the
  return type)**: an early design. Deprecated—it introduces four kinds of complexity: "discovery
  mechanism," "naming convention," "pick the first among multiple candidates," and "how names inside
  the proof function are resolved," all because the proof is placed outside the function. Moving it
  to the type position makes all four kinds of complexity disappear.
- **Only let the measure enter the type (`Terminates(m)`), drop the binary form**: shorter, but
  loses the expression position for "explicitly naming which function type the measure belongs to"
  (no place to put the measure when it is defined elsewhere or one measure serves multiple
  computations). The two arities are two arities of the same predicate, and the retention cost is
  near zero.
- **No fallback; require users to rewrite into analyzable iteration patterns**: the current state.
  gcd / merge partitioning / mutual recursion cannot be rewritten without breaking readability—this
  is exactly the trigger condition for the re-discussion clause.

## Non-goals

- No new syntax / keywords / annotation positions.
- No generalization of automatic measure synthesis (the four strategies stay within the RFC-027
  plan; anything beyond goes through explicit measure). General measure inference belongs to the
  undecidable side of "discovery"; only template boundaries can be agreed upon, completeness cannot
  be promised.
- No joint solving with RFC-009a borrowed propositions (each judges independently, sharing the
  backend).
- No totality checking at the dependent-type level.
- No enforcement that measures return natural-number types (no pursuit of Lean's
  `WellFoundedRelation` type-class mechanism)—the measure's return type is unrestricted, and
  well-foundedness is given to refinement derivation or SMT as an independent obligation.

## Stages and Acceptance

- [x] Obligation generation: well-foundedness / strict decrease, path-guard injection
- [x] Explicit-measure wiring: unary-form anchor resolution (return type position / binding
      position) + type-position measure extraction
- [x] SMT judgment wiring (reuse the #292 pipeline, Z3 injected into the production
      pipeline—previously the injection point was never called)
- [ ] Function-level call graph + SCC collection (measure sharing optimization)
- [ ] Lexicographic unfolding (when the measure returns a tuple)
- [x] E4021/E4022 registration + six-language locales (three-way consistency: codes ↔ locales ↔
      RFC-013 code table)
- [x] Diagnostic-layer fix: in-scope termination failures moved out of E8001, routed through the
      proof-failure family. E4021 already exists; E4022 goes through the user domain via
      `DisproofKind::MeasureNotDecreasing`, pinned by a use case to never be demoted to ICE
- [x] E2E positive: gcd (including the "residual obligation does not reject" form),
      structural-recursion zero-regression
- [x] E2E negative: measure does not hold → E4022 (with counterexample); no measure → E4021
- [x] E2E positive (loop): `Terminates(n - i)` binding form now runnable (D5 resolved)
- [ ] E2E to be landed: `is_even`-`is_odd` mutual recursion (awaiting SCC)
- [x] Criterion regression: recursion and loops without refinement annotations are no longer
      rejected by termination check; obligations are triggered as usual when there is a refinement
      annotation
- [x] Acceptance demo: write a declaration where the measure does not hold → compile fails and the
      counterexample is readable (`measure_not_decreasing_err.yx`); fix it and it passes
      (`measure_decreasing.yx`)

### Implementation landing log (2026-09-30 ~ 2026-10-01)

Landed surface: unary `Terminates` enters the type parser → AST extracts the measure (**not** via
the lossy `MonoType` conversion, which would drop the measure into `Int(64)`) → obligation
generation (recursive call sites, loop back-edges, with path guards) → well-foundedness and strict
decrease two-path SMT judgment → diagnostic emission.

The two **gates** for emitting E4022 (both only under-report, never false-positive):

1. **Falsified, not merely undecidable**—the solver must return a counterexample (`Sat`); `Unknown`
   is "cannot decide" and is handled by the §Trade-offs rule of "only give direction, do not
   reject."
2. **Measure well-foundedness is already proven**—the integer `<` is **not** well-founded, and a
   measure not decreasing **does not** equal non-termination (`gcd` still terminates when `b` is
   negative). When well-foundedness is unproven, even a falsified decrease does not trigger the
   report; it enters a residual obligation.

The presence of these two gates makes the §Trade-offs "only give direction, do not reject" and
"falsified means error" no longer contradictory: well-foundedness is exactly that license for
"non-decrease implies non-termination."

Existing defects fixed along the way (all are the prerequisite foundation for this RFC):

- **Predicate definition unregistered** (#377-3): `PredicateResolver`'s predicate-body substitution
  was already implemented, but short-circuited by an empty table. After registering it, refinement
  constraints became symbolically reasoned for the first time—the prerequisite for well-foundedness
  `b >= 0`.
- **`while` lacking value semantics** (D5): see the §Examples loop section.
- **evaluator missing `Neg`/`Pos`/`BitNot`**: inconsistent with the same-named implementation in
  `const_eval`, causing negative literals like `-5` to fail to fold to a constant on every path that
  goes through that evaluator.

## References

- #318 (the issue that initiated this RFC), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement-type criterion, §6.1–6.5 measure exploration four strategies, §6.9
  explicit measure, §Open Questions re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path-condition collection—prerequisite infrastructure)
- RFC-013 (error-code registry and proof-failure semantic family)
