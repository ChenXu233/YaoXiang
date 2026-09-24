---
title: 'RFC-027a: Explicit Measures for Termination Checking'
status: 'Under Review'
author: 'Chen Xu'
created: '2026-09-14'
updated: '2026-09-22'
issue: '#318'
impl_status: 'not-started'
---

# RFC-027a: Explicit Measures for Termination Checking

## Summary

RFC-027 §7 establishes the criterion for termination checking as **refinement types** (refinement being the verification mode), and §6.9 defines the language form for explicit measures (the builtin predicate `Terminates`, belonging to core primitives alongside `Int` and `Never`). Both are language design decisions, belonging to the host RFC, which has been finalized.

This sub-RFC addresses the implementation mechanism: how obligations are generated from computational structures, how the judgment pipeline is orchestrated, how diagnostics provide direction when derivation fails, how measures are shared across mutually recursive functions (SCC optimization), and how error codes are registered. **No repetition of language semantics**; only mechanisms are defined here.

## Motivation

### Why This Sub-RFC Is Needed

The criterion (refinement types in verification mode) and form (`Terminates` written in type position) have been decided in RFC-027, at language-level granularity. The remaining questions are too fine-grained; including them in the host RFC would bloat the main text:

- Where do obligations come from (recursive call sites, loop back edges, path guards)
- How are tuple-returning measures compared (lexicographic expansion)
- Are well-foundedness and strict decrease two independent obligations or unified
- How do auto-exploration and explicit measures coexist in the same pipeline
- How do mutually recursive functions avoid repeated measure writing and repeated verification
- How to give **direction** instead of outright rejection when derivation fails

### Trigger

Triggered by #318: non-structural recursion (gcd-class non-direct decrease, mutual recursion, merge partitioning) exceeds the template sequence for measure exploration (the four strategies in RFC-027 §6.2–6.5 all take "variables of bounded type" or "target type + swap operation" as input), and cannot be rewritten into analyzable iterative patterns without destroying readability. The re-discussion clause in RFC-027's "Open Issues" section is activated here.

## Proposal

### Relationship with Auto-Exploration

Explicit measures **are not another pipeline**, but input after exploration failure. After providing a measure, the same SMT still runs to verify the same set of obligations; if not satisfied, it reports an error with a counterexample. Full automation priority remains unchanged: exploration always runs first, user intervention only occurs after exploration fails.

This determines that both paths share all downstream mechanisms—obligation generation, SMT judgment, lexicographic expansion, and diagnostic format have only one implementation each.

### Obligation Generation

For a function `f` and measure `m`, generate two **independent** obligations, expanded at each recursive call site (including cross-function calls within SCC):

1. **Well-foundedness**: `m(args) >= 0`—the measure lands on natural numbers; when the measure returns a non-natural type, take the lower bound of a suitable ordering on that type. Derived from parameter refinement; when derivation fails, enters residual obligation.
2. **Strict decrease**: at each call site `m(callee_args) < m(caller_args)`, judged under **path guards**—guards come from branch conditions at that call site, reusing RFC-009a path condition collection.

The two are independent rather than unified because the failure directions differ: well-foundedness failure indicates the measure's value domain is incorrect (e.g., `Int` may be negative), while decrease failure indicates the recursive parameters aren't moving in that direction. Diagnostics need to distinguish these to give the correct checking direction (see "Diagnostics" section).

**Loops follow the same pattern**, replacing "call site" with "back edge": on each execution path through the loop body, `m(next_round_state) < m(this_round_state)`, with guards from loop conditions and in-body branches.

**Lexicographic expansion**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation expands into a disjunction chain according to lexicographic comparison—`(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`. Expansion is completed on the **obligation generation side**, keeping the SMT side as linear fragments without relying on the solver's native lexicographic support.

**The measure itself must be compile-time evaluable**: `m` must be a function provable by constant folding or structural recursion—prohibiting recursive deferral of the termination problem to another unproven function (preventing infinite regress). Pathological measures are blocked by existing E4012 (constant recursion too deep) and structural checks.

### Judgment Pipeline

```
1. Formal parameter decrease (structural recursion, strongest path, tried first)
2. Measure exploration: four-strategy template sequence (RFC-027 §6.2–6.5), stops at first match
3. Exploration success → Generate obligations → SMT judgment → Proved
4. Exploration failure → Check if type position provides explicit measure (Terminates)
     Has → Take that measure to generate obligations → SMT judgment
     No → E4021 (termination unprovable, with suggested checking direction)
5. Obligations judged false by SMT → E4022 (measure does not hold, with counterexample)
```

Steps 1–3 are the established path from RFC-027; this RFC adds step 4 and two error codes. The entire pipeline only runs when refinement types are triggered (RFC-027 §7)—ordinary types without refinement generate no obligations.

### Anchor Points: Unary and Binary Forms

RFC-027 §6.9 defines two arities; this RFC explains their respective landing points:

| Form                       | Anchor Point      | Landing Point                                                                          |
| -------------------------- | ----------------- | -------------------------------------------------------------------------------------- |
| `Terminates(m)`            | Name of binding   | Default form at definition site—self-recursive functions, loops                        |
| `Terminates(FnType, m)`    | Explicit fn type  | When explicit specification of which function type the measure belongs to is needed (measure defined elsewhere, same measure serving multiple computations) |

The two are not two different constructs, but two arities of the same predicate: termination obligations always land on "the computation annotated by the type position where the refinement resides". Mutually recursive functions **do not need** the binary form—each function can write its own unary form, with the sharing relationship recognized by SCC (see next section).

### SCC: Measure Sharing Optimization

For a set of mutually recursive functions (strongly connected component in call graph) sharing the same measure, cross-function edge obligations are `m_callee(callee_args) < m_caller(caller_args)`, which degrade to same-measure decrease when members share the measure.

**This is an optimization, not a correctness prerequisite**: without sharing, each function writes its own measure and closes individually, which still passes. SCC's value is recognizing "this group uses the same measure", eliminating repeated writing and repeated verification.

Requires new **function-level call graph and SCC collection**—existing `TypeDepGraph` records type annotation dependencies between variables (the VC trigger from RFC-027 §6.1), which is a variable-level graph and cannot be reused.

### Examples

#### gcd: Non-Structural Recursion

```yaoxiang
// Measure: ordinary function, can be unit-tested, can be reused
gcd_measure: (a: Int, b: Int) -> Int = { b }

gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

Obligation generation:

1. **Well-foundedness**: `gcd_measure(a, b) >= 0` i.e. `b >= 0`—directly derived from formal parameter refinement `NonNegative(b)`
2. **Strict decrease**: single recursive call site `gcd(b, a % b)`, path guard `b != 0`, obligation `gcd_measure(b, a % b) < gcd_measure(a, b)`; expanding by substituting measure body gives `a % b < b`
3. **SMT**: verifies that negation `b != 0 ∧ a % b >= b` is unsatisfiable → Proved

#### Loop: Anonymous Construct Gains Denotation

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

`Terminates(m)` refines the value type of the loop body's tail expression, with anchor point provided by binding name `acc`—the loop is therefore **denotable**, eliminating the "anonymous construct has no way to refer" dead end. This also explains the unity of the two arities: obligations always land on "the computation annotated by the type position where the refinement resides"; functions and loops are no different.

Obligation: on back edge `(n - i') < (n - i)`, guard `i < n`, substituting `i' = i + 1` gives `1 > 0`, always true → Proved.

#### Mutually Recursive: SCC Shared Measure

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

Both functions carry unary form `Terminates(nat)`. After SCC collection, they are recognized as sharing the same measure; cross-function edge obligation `nat(n - 1) < nat(n)` is always true under guard `n != 0`, and both functions are verified closed in one pass.

If SCC recognition is not performed, verifying each function against the same obligation still passes—just one repetition. This confirms SCC's position as an optimization.

### Diagnostics

**When well-foundedness cannot be derived, do not reject outright; instead, suggest checking direction.**
This must be distinguished from diagnostics for decreasing obligation failure: the former points to measure value domain, the latter to recursive parameters. Suggested directions for each of the three failure types:

| Failure               | Suggested Direction                                                                              |
| --------------------- | ------------------------------------------------------------------------------------------------ |
| Well-foundedness undeducible | Whether measure has a valid lower bound on possible values (e.g., if `Int` measure needs `>= 0`) |
| Strict decrease judged false | Whether recursive parameters actually move in the direction of measure decrease; attach SMT counterexample |
| No measure and exploration failed | Hint that a name can be bound for that computation and a measure provided in type position (RFC-027 §6.9) |

Counterexample presentation follows RFC-013 diagnostic message specification. Under non-linear path guards, Sat counterexamples may not be intuitive—recorded as known limitation, iterated with RFC-013.

### Error Codes

Aligned with E4xxx proof failure family (E4018 refinement predicate violation, E4020 proof function required):

| Proposed Code | Name              | Trigger                                                                                           |
| ------------- | ----------------- | ------------------------------------------------------------------------------------------------- |
| E4021         | Termination unprovable | Exploration failed and no explicit measure in type position (hint to provide `Terminates` measure) |
| E4022         | Measure does not hold | Measure obligation judged false by SMT (with counterexample)                                       |

Final numbering subject to RFC-013 registry reality at implementation time (segment legality guaranteed by build.rs gate).

### Existing Diagnostic Layer Defects

In current implementation, in-scope termination failure is reported as **E8001 "Internal Compiler Error"** (`Unproven` is formatted as ICE)—termination checking is not an ICE; occupying this code misleads users and masks real faults. This RFC corrects this: in-scope termination failure follows E4021/E4022, ICE code position returned to genuine internal errors.

### Compiler Changes

| Component                               | Changes                                                                                                                          |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`       | Interface unification (exploration path and explicit measure path share obligation generation and judgment); integrate Z3 (inject into production pipeline, `with_z3` currently only in unit tests) |
| Function call graph + SCC (**NEW**)     | No cross-function call graph in entire codebase—`TypeDepGraph` is variable-level type dependency, cannot be reused. New function-level call graph and SCC collection for mutually recursive measure sharing |
| Obligation generation                   | New: well-foundedness / strict decrease obligations, path guard injection, lexicographic expansion                               |
| Proof pipeline (RFC-009a / #292)        | Reuse `ConstExpr → SMTLib` chain and `Mod` etc. operator mappings; no backend changes                                            |
| `util/diagnostic/codes/e4xxx.rs`        | Add E4021/E4022 registration (via RFC-013 registry, build.rs gate in effect)                                                      |
| Locales ×6                              | Six-language templates for two new codes                                                                                         |
| Diagnostic layer                        | Migrate in-scope termination failure from E8001, switch to proof failure family                                                  |

### Backward Compatibility

- Programs previously rejected by termination checking but **without refinement annotations**: under new criterion, they don't enter verification mode and compile through directly—intentional relaxation, only loosening, not tightening.
- Programs previously rejected by termination checking **with refinement annotations**: can pass after supplementing measure.
- Structural recursion positive examples, existing termination tests: expected output unchanged.

## Trade-offs

### Advantages

- **Zero new syntax**: measure written in type position, reusing refinement predicate application mechanism; no `decreases` syntax slot.
- **Aligned with proof domain behavior**: termination domain supplements the same fallback channel as correctness domain, but landing point differs—correctness domain propositions that can't be proven are written in **body** as proof functions, termination domain measures that can't be explored are declared in **type position**. Both mechanisms are identical (refinement type applications).
- **Infrastructure reuse**: obligations are linear arithmetic plus path guards, routing through #292's already-connected pipeline; no new backend.
- **Loops gain denotation**: binding name is the anchor point; loops and functions are completely homogeneous in obligation generation; no special case design needed for loops.

### Disadvantages and Risks

- **Nontrivial infrastructure prerequisite**: function-level call graph and SCC collection need to be built from scratch; Z3 is also not in production pipeline. SCC portion can be deferred (it's an optimization), but call graph portion shares origin with SCC—deferring means mutually recursive functions must each write their own measures—still passes, just repeated verification.
- **Explicit measures are writing overhead for low-frequency paths**: two-piece setup (measure function + type position declaration) is more verbose than inline annotation. Accepted—fallback is low-frequency path, and换来 measures become reusable and unit-testable.
- **Well-foundedness obligation noise**: when measure returns `Int`, need to prove `>= 0` every time. Mitigation: if parameter refinement already gives lower bound, automatically derived; only enters residual obligation when derivation fails, and only gives direction without rejecting.
- **Counterexample quality**: Sat counterexamples under non-linear guards are not intuitive. Known limitation.
- **`Terminates` is the only predicate with compiler-generated body**: deviates from "all predicate bodies can be user-written" purity. Reason: its assertion (measure decrease at each call site/back edge) is embedded in the computational structure; user-written predicates cannot reference function bodies or loop bodies. Builtin surface converges to a name; mechanism has zero additions.

## Alternative Approaches

- **`with decreases (b)` inline annotation syntax**: early proposal in this issue, withdrawn. "No annotation syntax opening for termination checking" is RFC-027's decided policy; inline annotation would turn unsupported termination patterns into syntax slots rather than type slots, which contradicts the worldview of "everything is YaoXiang functions, everything verified by type checker".
- **Separate proof function (`gcd_proof` returns `Terminates(f, m)`, discovered by return type scanning)**: early design. Rejected—it introduced four categories of complexity ("discovery mechanism", "naming convention", "pick first among multiple candidates", "name resolution in proof function body"), all because proof was moved outside the body. After changing to type position, all four complexity categories disappear.
- **Only measure in type (`Terminates(m)`), cancel binary form**: shorter, but loses the expression slot for "explicitly specifying measure attribution" (nowhere to place when measure defined elsewhere, same measure serving multiple computations). Two arities are two arities of the same predicate; retaining costs nearly zero.
- **No fallback, require user to rewrite into analyzable iterative patterns**: current status. gcd / merge partitioning / mutual recursion cannot be rewritten without destroying readability—this is exactly the trigger condition for the re-discussion clause.

## Non-Goals

- No new syntax/keywords/annotation slots.
- No generalization extension for automatic measure synthesis (four strategies maintain RFC-027's established plan; beyond that, go explicit measure). Generic measure inference belongs to the undecidable side of "discovery"; can only约定 template boundaries, cannot commit to completeness.
- No joint solving with RFC-009a borrowing propositions (each judged independently, share backend).
- No totality verification at dependent type level.
- No enforcement that measure returns natural number type (not pursuing Lean's `WellFoundedRelation` typeclass mechanism)—measure return type unrestricted; well-foundedness as independent obligation delegated to refinement derivation or SMT.

## Phases and Acceptance

- [ ] Obligation generation (well-foundedness / strict decrease, path guard injection, lexicographic expansion)
- [ ] Explicit measure wiring (anchor resolution for `Terminates` unary and binary forms, type position measure extraction)
- [ ] SMT judgment wiring (reuse #292 pipeline, Z3 production pipeline injection)
- [ ] Function-level call graph + SCC collection (measure sharing optimization)
- [ ] E4021/E4022 registration + six-language locales
- [ ] Diagnostic layer correction: in-scope termination failure migrated from E8001, switched to proof failure family
- [ ] E2E: positive examples (gcd / loop `Terminates(n - i)` / `is_even`-`is_odd` mutual recursion) / negative examples (measure does not hold → E4022; no measure → E4021) / structural recursion zero regression
- [ ] Criterion regression: recursion and loops without refinement annotations no longer rejected by termination checking; obligations still trigger with refinement annotations
- [ ] Acceptance demo: write a measure declaration that doesn't hold → compile fails with readable counterexample; after fixing, passes

## Related

- #318 (this RFC's triggering issue), #251 (parent milestone P1)
- RFC-027 (host: §7 refinement type criterion, §6.1–6.5 measure exploration four strategies, §6.9 explicit measures, §Open Issues re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline and path condition collection—infrastructure prerequisite)
- RFC-013 (error code registry and proof failure semantic family)