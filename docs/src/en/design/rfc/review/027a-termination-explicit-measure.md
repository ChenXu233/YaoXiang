---
title: 'RFC-027a: Explicit Measures for Termination Checking'
status: 'Under Review'
author: '晨煦'
created: '2026-09-14'
updated: '2026-09-22'
issue: '#318'
impl_status: 'not-started'
---

# RFC-027a: Explicit Measures for Termination Checking

## Summary

RFC-027 §7 establishes the termination-checking criterion as **refinement types** (refinement is the
entry to verification mode), and §6.9 defines the language form of explicit measures (the builtin
predicate `Terminates`, alongside `Int` and `Never` as core primitives). Both are language design,
belonging to the host RFC, and are finalized.

This sub-RFC takes on the implementation mechanism: how obligations are generated from computation
structure, how the judgment pipeline is orchestrated, how diagnostics point the way when nothing can
be proved, how mutual-recursion measures are shared (SCC optimization), and how error codes are
registered. **It does not restate language semantics**, only defining mechanisms.

## Motivation

### Why a Sub-RFC Is Needed

The criterion (refinement type entry to verification mode) and the form (`Terminates` written at the
type position) have been settled in RFC-027, at the language level. The remaining issues are too
fine-grained to be written into the host without bloating its body:

- Where obligations come from (recursive call sites, loop back edges, path guards)
- How to compare when a measure returns a tuple (lexicographic unfolding)
- Whether well-foundedness and strict decrease are two independent obligations or unified
- How automatic exploration and explicit measures coexist in the same pipeline
- How the same measure avoids repeated writing and verification for mutual recursion
- How to point a **direction** rather than simply reject when nothing can be proved

### Trigger

The direction is triggered by #318: non-structural recursion (gcd-style non-directly-decreasing,
mutual recursion, merge partitioning) goes beyond the template sequences of measure exploration (all
four strategies in RFC-027 §6.2–6.5 take "a variable of a bounded type" or "a target type + swap
operation" as input), and cannot be rewritten into analyzable iteration patterns without destroying
readability. The "Open Questions" section's re-discussion clause of RFC-027 is activated here.

## Proposal

### Relationship with Automatic Exploration

Explicit measures are **not a separate pipeline**, but the input after exploration fails. Once a
measure is given, the same SMT validates the same set of obligations; if they fail, an error is
reported with a counterexample. The principle of full automation first is preserved: exploration
always runs first, and user intervention only happens after exploration has failed.

This determines that both paths share all downstream mechanisms—obligation generation, SMT judgment,
lexicographic unfolding, diagnostic format—each implemented only once.

### Obligation Generation

For a function `f` and a measure `m`, two **independent** obligations are generated, expanded at
every recursive call site (including cross-function calls within SCCs):

1. **Well-foundedness**: `m(args) >= 0`—the measure falls on the natural numbers; when the measure
   returns a non-natural-number type, take the lower bound of the well-defined order on that type.
   This is derived from argument refinements; when it cannot be derived, it falls into a residual
   obligation.
2. **Strict decrease**: at every call site `m(callee_args) < m(caller_args)`, judged under **path
   guards**—the guards come from the branch conditions of the call site, reusing RFC-009a
   path-condition collection.

The two are independent rather than unified, because the failure directions differ: a
well-foundedness failure means the measure's range is wrong (e.g. `Int` may be negative); a decrease
failure means recursive arguments are not heading in the right direction. Diagnostics need to
distinguish them, so that the correct checking direction can be given (see the "Diagnostics"
section).

**The same applies to loops**, replacing "call site" with "back edge": on every execution path of
the loop body `m(next state) < m(current state)`, with guards coming from loop conditions and
in-body branches.

**Lexicographic unfolding**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligations are expanded
via lexicographic comparison into a disjunction chain—`(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`.
Unfolding is done on the **obligation-generation side**; the SMT side remains a linear fragment, not
depending on the solver's native support for lexicographic orders.

**The measure itself must be compile-time evaluable**: `m` must be a function that is
constant-foldable or provable by structural recursion—it is forbidden to recurse the termination
problem into another unproven function (preventing infinite regression). Pathological measures are
blocked by the existing E4012 (constant recursion too deep) and structural checks.

### Judgment Pipeline

```
1. Structural-parameter decrease (structural recursion, strongest path, try first)
2. Measure exploration: four-strategy template sequence (RFC-027 §6.2–6.5),
   stop as soon as one is found
3. Exploration succeeds → generate obligations → SMT judgment → Proved
4. Exploration fails → check whether the type position provides an explicit measure (Terminates)
     Yes → take that measure to generate obligations → SMT judgment
     No  → E4021 (termination cannot be proved, with a suggested checking direction)
5. Obligation refuted by SMT → E4022 (measure does not hold, with counterexample)
```

Levels 1–3 are the path already defined by RFC-027; this RFC adds level 4 and the two error codes.
The whole pipeline only runs when refinement types are triggered (RFC-027 §7)—plain types without
refinements generate no obligations at all.

### Anchors: Unary and Binary Forms

RFC-027 §6.9 defined two arities; this RFC explains their respective landing points:

| Form                    | Anchor                     | Landing Point                                                                                                              |
| ----------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `Terminates(m)`         | The name of the binding    | The default form at the definition site—self-recursive functions, loops                                                    |
| `Terminates(FnType, m)` | The explicit function type | When the measure's ownership must be made explicit (measure defined elsewhere, same measure serving multiple computations) |

The two are not two distinct constructs, but two arities of the same predicate: the termination
obligation always lands on "the computation annotated by the type position where the refinement
sits". **Mutual recursion does not need the binary form**—each of the two functions writes its own
unary form, and the sharing relation is identified by SCC (see next section).

### SCC: Measure Sharing Optimization

For a group of mutually recursive functions (a strongly connected component on the call graph)
sharing the same measure, the obligation on cross-function edges is
`m_callee(callee_args) < m_caller(caller_args)`, which degenerates to same-measure decrease when
members share the measure.

**This is an optimization, not a correctness prerequisite**: when not shared, each function writes
its own measure and closes its own loop to pass. The value of SCC is in recognizing "this group uses
the same measure", avoiding repeated writing and verification.

A new **function-level call graph and SCC collection** must be built—the existing `TypeDepGraph`
records variable-level type-annotation dependencies (the VC trigger in RFC-027 §6.1); it is a
variable-level graph and cannot be reused.

### Examples

#### gcd: Non-Structural Recursion

```yaoxiang
// Measure: an ordinary function, unit-testable and reusable
gcd_measure: (a: Int, b: Int) -> Int = { b }

gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

Obligation generation:

1. **Well-foundedness**: `gcd_measure(a, b) >= 0`, i.e. `b >= 0`—derived directly from the parameter
   refinement `NonNegative(b)`
2. **Strict decrease**: the only recursive call site `gcd(b, a % b)`, with path guard `b != 0`, the
   obligation is `gcd_measure(b, a % b) < gcd_measure(a, b)`; substituting the measure body expands
   it to `a % b < b`
3. **SMT**: verify that the negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved

#### Loops: Anonymous Constructs Acquire Designation

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
anchor is provided by the binding name `acc`—the loop is therefore **designable**, with no more dead
corner of "anonymous construct lacking designation". This also explains the unity of the two
arities: the obligation always lands on the computation annotated by the type position where the
refinement sits; functions and loops are not distinguished.

Obligation: on the back edge, `(n - i') < (n - i)`, with guard `i < n`, substituting `i' = i + 1`
yields `1 > 0`, which is a tautology → Proved.

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

Both functions carry the unary form `Terminates(nat)`. After SCC collection, the two are recognized
as sharing the same measure, and the cross-function edge obligation `nat(n - 1) < nat(n)` is a
tautology under guard `n != 0`; both functions close their loop in a single verification.

Without SCC recognition, each function verifies the same obligation independently and still
passes—only once more. This confirms that SCC is positioned as an optimization.

### Diagnostics

**When well-foundedness cannot be derived, do not directly reject—suggest a checking direction.**
This must be distinguished from the diagnostic for decrease-obligation failure: the former points to
the measure's range, the latter to the recursive arguments. The suggestions for the three failure
types are:

| Failure                            | Suggested Direction                                                                                            |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Well-foundedness cannot be derived | Whether the measure attains a lower bound over its possible values (e.g. does an `Int` measure need `>= 0`?)   |
| Strict decrease refuted            | Whether the recursive arguments actually go in the measure's decreasing direction; SMT counterexample included |
| No measure and exploration failed  | Hint that the computation can be bound to a name and given a measure at the type position (RFC-027 §6.9)       |

Counterexample presentation follows the RFC-013 diagnostic-message spec. Under nonlinear path
guards, Sat counterexamples may be unintuitive—recorded as a known limitation, to be iterated along
with RFC-013.

### Error Codes

Aligned with the E4xxx proof-failure family (E4018 refinement-predicate violation, E4020 proof
function needed):

| Proposed Code | Name                     | Trigger                                                                                              |
| ------------- | ------------------------ | ---------------------------------------------------------------------------------------------------- |
| E4021         | Termination not provable | Exploration failed and no explicit measure at the type position (hint: provide `Terminates` measure) |
| E4022         | Measure does not hold    | Measure obligation refuted by SMT (counterexample included)                                          |

The final numbers are subject to the actual state of the RFC-013 registry at implementation time
(segment validity is guaranteed by the build.rs threshold).

### Existing Defect in the Diagnostic Layer

In the current implementation, in-scope termination failures are reported as **E8001 "Internal
Compiler Error"** (`Unproven` is formatted as an ICE)—termination checking is not an ICE, and
occupying that code position is both misleading for users and obscures the real fault. This RFC
fixes it as well: in-scope termination failures go to E4021/E4022, and the ICE code position is
returned to genuine internal errors.

### Compiler Changes

| Component                           | Change                                                                                                                                                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`   | Interface unification (exploration path and explicit-measure path share obligation generation and judgment); connect Z3 (injected into the production pipeline; `with_z3` currently exists only in unit tests)      |
| Function call graph + SCC (**new**) | No cross-function call graph in the whole repo—`TypeDepGraph` is variable-level type dependency, not reusable. Build a new function-level call graph and SCC collection to support mutual-recursive measure sharing |
| Obligation generation               | Add: well-foundedness / strict-decrease obligations, path-guard injection, lexicographic unfolding                                                                                                                  |
| Proof pipeline (RFC-009a / #292)    | Reuse the `ConstExpr → SMTLib` link and `Mod` and other operator mappings, no backend changes                                                                                                                       |
| `util/diagnostic/codes/e4xxx.rs`    | Register E4021/E4022 (via the RFC-013 registry, build.rs threshold takes effect)                                                                                                                                    |
| locales ×6                          | Six-language templates for the two new codes                                                                                                                                                                        |
| Diagnostic layer                    | Move in-scope termination failures out of E8001, into the proof-failure family                                                                                                                                      |

### Backward Compatibility

- Programs previously rejected for termination checking but **without refinement annotations**:
  under the new criterion, they do not enter verification mode and compile directly—a deliberate
  relaxation, only loosening, never tightening.
- Programs previously rejected for termination checking with refinement annotations: pass after
  writing a measure.
- Structural-recursion positive cases and existing termination tests: expected output unchanged.

## Trade-offs

### Advantages

- **Zero new syntax**: the measure is written at the type position, reusing the refinement-predicate
  application mechanism; no `decreases`-style syntactic slot.
- **Proof-domain behavior alignment**: the termination domain gains a fallback channel consistent
  with the correctness domain, but with a different landing point—propositions that cannot be proved
  in the correctness domain are written in the **body** as a proof function, while measures that
  cannot be found in the termination domain are declared in the **type position**. Both mechanisms
  are the same (refinement-type applications).
- **Reuse infrastructure**: the obligations are linear arithmetic plus path guards, going through
  the pipeline already connected by #292, with no new backend.
- **Loops gain designation**: the binding name is the anchor, so loops and functions are fully
  isomorphic in obligation generation, no special case needed for loops.

### Disadvantages and Risks

- **The amount of prerequisite infrastructure is no less than "wiring"**: the function-level call
  graph and SCC collection need to be built, and Z3 is not yet injected into the production
  pipeline. The SCC part can be deferred (it is an optimization); the call-graph part shares the
  same source as SCC, and deferring it means mutual recursion can only have each function write its
  own measure—it still passes, but with repeated verification.
- **Explicit measures are a writing burden on a low-frequency path**: the two-piece set (measure
  function + type-position declaration) is more verbose than inline annotations. Accepted—the
  fallback is a low-frequency path, and the trade-off is that the measure becomes reusable and
  unit-testable.
- **Noise from the well-foundedness obligation**: when the measure returns `Int`, one must prove
  `>= 0` every time. Mitigation: if the parameter refinement already provides a lower bound, it is
  derived automatically; only when it cannot be derived does it fall into a residual obligation, and
  even then it only gives a direction, not a rejection.
- **Counterexample quality**: Sat counterexamples under nonlinear guards are unintuitive. Known
  limitation.
- **`Terminates` is the only predicate for which the compiler writes the body**: this deviates from
  the purity of "all predicate bodies are user-writable". Justification: its assertion (decrease at
  every call site/back edge) is rooted in computation structure, and a user-written predicate cannot
  reach into a function or loop body. The builtin surface converges to a single name, with zero new
  mechanism.

## Alternatives

- **`with decreases (b)` inline-annotation syntax**: an early proposal in this issue, withdrawn. The
  settled decision in RFC-027 is "no termination-checking **annotation-syntax** opening"; inline
  annotations would turn unsupported termination patterns into a syntactic slot rather than a type
  position, contradicting the worldview of "everything is a YaoXiang function, everything is
  verified by the type checker".
- **Independent proof function (`gcd_proof` returns `Terminates(f, m)`, discovered by scanning the
  return type)**: an earlier design. Aborted—it introduces four kinds of complexity ("discovery
  mechanism", "naming conventions", "first among multiple candidates", "how names inside the proof
  function are resolved"), all arising from putting the proof outside. Moving it to the type
  position makes all four kinds of complexity vanish.
- **Only let the measure enter the type (`Terminates(m)`), dropping the binary form**: shorter, but
  loses the expression slot for "explicitly indicating measure ownership" (when the measure is
  defined elsewhere or the same measure serves multiple computations). The two arities are two
  arities of the same predicate, and the cost of keeping them is nearly zero.
- **No fallback, require the user to rewrite into analyzable iteration patterns**: the status quo.
  gcd / merge partitioning / mutual recursion cannot be rewritten without destroying
  readability—this is exactly what triggers the re-discussion clause.

## Non-Goals

- No new syntax / keywords / annotation slots.
- No generalization extension of automatic measure synthesis (the four strategies keep RFC-027's
  settled plan; anything beyond that goes to explicit measures). Generic measure inference lies on
  the undecidable side of "discovery"; only template boundaries can be agreed upon, completeness
  cannot be promised.
- No joint solving of proposition unions with RFC-009a (each judged independently, sharing the
  backend).
- No totality checking at the dependent-type level.
- No requirement that the measure return a natural-number type (not pursuing Lean's
  `WellFoundedRelation` type-class mechanism)—the measure's return type is unrestricted, and
  well-foundedness is handled as an independent obligation by refinement derivation or SMT.

## Phases and Acceptance

- [ ] Obligation generation (well-foundedness / strict decrease, path-guard injection, lexicographic
      unfolding)
- [ ] Explicit-measure wiring (anchor resolution for both `Terminates` arities, type-position
      measure extraction)
- [ ] SMT-judgment wiring (reusing the #292 pipeline, Z3 injected into the production pipeline)
- [ ] Function-level call graph + SCC collection (measure-sharing optimization)
- [ ] E4021/E4022 registration + six-language locales
- [ ] Diagnostic-layer fix: in-scope termination failures moved out of E8001, into the proof-failure
      family
- [ ] E2E: positive cases (gcd / loop `Terminates(n - i)` / `is_even`-`is_odd` mutual recursion) /
      negative cases (measure does not hold → E4022; no measure → E4021) / structural-recursion zero
      regression
- [ ] Criterion regression: recursion and loops without refinement annotations are no longer
      rejected by termination checking; obligations trigger as usual when refinements are present
- [ ] Acceptance demo: write a declaration whose measure does not hold → compile fails and the
      counterexample is readable; correct it and it passes

## Related

- #318 (the RFC's project-originating issue), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement-type criterion, §6.1–6.5 four measure-exploration strategies, §6.9
  explicit measures, §Open Questions re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path-condition collection—infrastructure prerequisites)
- RFC-013 (error-code registry and proof-failure semantic family)
