---
title: 'RFC-027a: Termination Check Proof Function Fallback'
status: 'Draft'
author: '晨煦'
created: '2026-09-14'
updated: '2026-09-14'
issue: '#318'
---

# RFC-027a: Termination Check Proof Function Fallback

## Summary

Fulfilling the re-discussion clause reserved in the RFC-027 decision record: for **non-structural
recursion** (gcd-class non-directly-decreasing, mutual recursion) that the four automatic strategies
cannot cover for termination proofs, provide a **proof function fallback**—measures and proofs are
ordinary YaoXiang functions, zero new syntax, type-checker verified. The mechanism is positioned as
an application of the refinement type proof protocol: zero new mechanism on the proof side (the
existing Unproven fallback channel from RFC-027 is reused as-is), the only additions are call-site
obligation generation on the compiler side and a proposition anchor. The fully-automated-first
principle is unchanged: automatic strategies remain the first path, and user intervention only
occurs after Unproven. Scope ruling (2026-09-14): termination check targets only **compile-time
evaluation** (triggered by dependency tracking); runtime code does not require termination
proofs—adopting the original text of RFC-027 §6.7's scope table, the current implementation's
full-coverage check is narrowed accordingly.

## Motivation

### Host Qualification and Amendment Scope

RFC-027 did not reserve a sub-RFC slot (unlike RFC-029 reserving 029a). The host qualification of
this sub-RFC comes from the re-discussion clause in the RFC-027 decision record itself:

> Loops that cannot be proven terminating by any strategy are reported as errors directly by the
> compiler—this is hard safety philosophy, not a defect. **If, in the future, there are real-world
> scenarios (not academic constructions) where no algorithm can be covered by the four strategies,
> it will be discussed again.** — RFC-027 Decision Record

This RFC is the formal carrier of that "re-discussion". It does not **overturn** the recorded
decision—the clause itself is part of the decision, and triggering it is precisely the decision
working as designed; it is the anticipated exercise of the clause.

The amendment scope to RFC-027 is precisely limited to four places:

| Amendment Point                                  | Original Record                                                                             | After Amendment                                                                                                                                                                                                               |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| §7 "Leave no semi-automatic annotation loophole" | Termination check is fully closed                                                           | Narrowed to "no **annotation syntax** loophole"—proof function fallback is not annotation (no new syntax position, not attached to the function definition), same worldview as "compile-time predicates have no new keywords" |
| Decision Record "Unproven → Direct Error"        | The only proof domain without fallback in the termination domain                            | Changed to "Unproven → proof function may be provided"—aligned with the correctness domain behavior (predicate Unproven → proof function, RFC-027 §Proof Function)                                                            |
| §6.8 Error Message                               | "Please use an iteration pattern that can be analyzed by the compiler"                      | Added "or provide a measure function and a termination proof function"                                                                                                                                                        |
| §7 Scope                                         | "Loops" not positionally limited, implemented as unconditional check on all recursion/loops | Clarified as **compile-time evaluation** (triggered by dependency tracking), aligned with §6.7's scope table; runtime code does not require termination proofs (design ruling 2026-09-14)                                     |

**Not amended** parts: the priority of the four automatic strategies, the fully-automated-first
principle, the remainder of the hard safety philosophy (Disproved is still a compile error +
counterexample; correctness domain behavior is zero-changed).

### Current Problems (2026-09-14 actual test, post v0.7.14 dev)

1. **Scope Deviation: Documentation says compile-time, implementation checks everything**. RFC-027
   §6.7's scope table clearly states "runtime calls (non-type positions) do not require termination
   check"; the current `TerminationChecker` instead unconditionally traverses all recursion and
   loops (`checker.rs:405`)—pure-runtime-purpose non-structural recursion directly fails to compile,
   and `while` loops without analyzable measures are also rejected. This RFC narrows according to
   the design ruling (2026-09-14): termination check only serves compile-time evaluation,
   obligations are triggered by dependency tracking.

2. **Non-structural recursion at compile-time positions has no fallback**. `termination.rs` is
   currently in Phase 1: strategy 3 (bounded increase/decrease), direct decrease of recursive
   parameters (`factorial(n-1)`), `for` loops naturally terminate. Strategies 1/2/4 are not yet
   implemented (RFC-027's established extensions). When non-structural recursion enters compile-time
   evaluation (such as invocation inside predicate bodies), the obligation cannot be closed:

   ```yaoxiang
   gcd: (a: Int, b: Int) -> Int = {
       if b == 0 { return a }
       return gcd(b, a % b)
   }
   ```

   The recursive arguments `(b, a % b)` are not a direct decrease of the parameters—currently errors
   and requires rewriting into an iteration pattern. gcd cannot be rewritten into an "analyzable
   iteration pattern" without breaking readability; merge sort partitioning, mutual recursion
   (even/odd judgment), tree traversal are similar. These are real algorithms, not academic
   constructions—**the trigger condition for the re-discussion clause is established**.

3. **Asymmetric fallback in the proof domain**. RFC-027's correctness propositions (`Positive(b)`,
   `IsMax` and other predicates) have proof function fallbacks when Unproven; termination is the
   only domain in the compile-time proof domain where Unproven is a dead end. Within the same
   worldview, this asymmetry has no principled reason.

4. **Infrastructure is ready**. #292 has truly connected the Z3 backend (`ConstExpr` → SMTLib →
   Unsat/Sat judgment link). The measure-decrease obligation `measure(args') < measure(args)` at
   recursive call sites is a first-order linear arithmetic proposition, falling precisely within the
   decidable fragment already opened by this link—reusing the same pipeline, no new backend.

## Proposal

### Core Design

**Positioning: this RFC adds no new proof mechanism; it is an application of the refinement type
proof protocol.** RFC-027 has already established that "a proof function is a YaoXiang function
whose return type is the proposition being asserted"—a proposition is a predicate application, a
predicate application is a refinement type. The human side of termination proof uses this existing
channel throughout (Unproven → write proof function → type-checker verifies → witness erasure),
fully aligned with the correctness domain protocol. The new surface converges to three things:

1. **Measure function**—an ordinary compile-time function, with the same input signature as the
   measured function, returning `Int` or a tuple of `Int` (lexicographic). Not an annotation, not
   attached to the function definition; a first-class function that can be referenced and
   unit-tested.
2. **Call-site obligation generation**—the compiler extracts the decrease obligation from each
   recursive call site in `f`'s function body. This is the only new compiler logic: the call-site
   actual arguments live in the function body, the obligation (semantic constraint) must be
   generated by the compiler, humans cannot write it themselves—otherwise the proof is no longer
   bound to the actual code (see Design Philosophy item 3).
3. **Termination proposition `Terminates(f, m)`**—a **refinement predicate whose body is generated
   by the compiler**: its "constraint content" is the obligation set above, its signature is just
   the anchor point binding `(f, m)` together. The necessity of being built-in is limited to this;
   there is nothing new mechanism-wise. Both the function and the measure enter the type rather than
   just the measure: in mutual recursion `is_even`/`is_odd` have the same signature, and
   disambiguating by measure alone would cause ambiguity (see example).

The three roles of the proof function are naturally derived from the positioning: explicitly binding
measure `m` to `f` (discovery anchor, replacing all annotation syntax); providing a discharge
position for residual obligations (body = proof space); universally quantifying and holding `f`'s
parameter names (can reference `a`, `b` in the body to write facts). See "Detailed Syntax" for the
syntax at each point, and "Examples" for a complete walkthrough.

### Examples

#### End-to-End Walkthrough: gcd

```yaoxiang
-- Measured function (correctness domain as usual: parameter refinement, guard, recursive call)
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

-- Measure: ordinary function (can be unit-tested, reusable)
gcd_measure: (a: Int, b: Int) -> Int = { b }

-- Proof function: empty body—all obligations are automatically closed by SMT
gcd_proof: (a: Int, b: Int) -> Terminates(gcd, gcd_measure) = { }
```

Trigger premise: the following obligations are only generated and consumed when `gcd` enters
compile-time evaluation (dependency tracking hit, such as being called inside a predicate body);
when `gcd` is only called at runtime, no termination proof is required.

The full process of the compiler handling `gcd_proof`:

1. **Signature check**: The proof function's parameters are matched one-by-one against `gcd`'s
   formal parameters by **base type** (`Int`, `Int`). Formal parameter refinements (`b >= 0`) are
   not repeated in the proof function signature—it is already held by the compiler as obligation
   context, both the proof function body and the obligations are verified within this context.
2. **Well-foundedness obligation**: `gcd_measure(a, b) >= 0`, i.e. `b >= 0`—directly derived from
   the formal parameter refinement `NonNegative(b)`. The bound of the measure needs no new
   mechanism; it is the parameter's refinement constraint (the semantic condition of refinement
   types realized in well-foundedness).
3. **Call-site obligation generation**: The only recursive call site `gcd(b, a % b)`, path guard
   `b != 0`, generates `gcd_measure(b, a % b) < gcd_measure(a, b)`; substituting the measure body
   expands to `a % b < b`.
4. **SMT judgment**: Verify that the negation of the obligation `b != 0 ∧ a % b >= b` is
   unsatisfiable—linear arithmetic + modulo axioms, millisecond-level → Proved.
5. **Accept**: The body is empty, no residual obligations need discharging. `gcd`'s termination is
   proven; the measure and the proof function are erased as witnesses, not entering the runtime
   binary.

#### Non-empty body: residual obligations referencing proven lemmas

When the decrease at a call site depends on facts invisible to the compiler (in the following
example, `step`'s body is in the dependency module, opaque to obligation generation), humans write
this fact into the body as **referencing a proven predicate**—a bare expression, not an Assert
statement:

```yaoxiang
step: (n: Int) -> Int = { ... }

loop_x: (n: NonNegative(n)) -> Void = {
    if n == 0 { return () }
    return loop_x(step(n))
}

x_measure: (n: Int) -> Int = { n }

-- Proven postcondition exported from step's definition module (correctness domain, ordinary predicate, already proven at definition site):
step_decreases: (n: Int) -> Type = { n >= 1 ==> step(n) < n }

-- Termination proof = referencing that lemma (referencing a proven predicate = applying its established proof, Curry-Howard):
x_proof: (n: Int) -> Terminates(loop_x, x_measure) = {
    step_decreases(n)
}
```

Verification process: the obligation `x_measure(step(n)) < x_measure(n)` (guard `n != 0`, context
`n >= 0`) cannot be independently closed in the module where `loop_x` is located → discovers
non-empty body → asserts that `step_decreases(n)` is an instance of a proven predicate, verification
passes → as a lemma closes the remaining obligations. If the body directly writes `step(n) < n` out
of thin air, the verifier will honestly report Unproven—facts must come from a traceable proven
source.

#### Mutual Recursion: obligations collected by SCC

```yaoxiang
is_even: (n: NonNegative(n)) -> Bool = { if n == 0 { return true }  return is_odd(n - 1) }
is_odd:  (n: NonNegative(n)) -> Bool = { if n == 0 { return false } return is_even(n - 1) }

nat: (n: Int) -> Int = { n }

is_even_proof: (n: Int) -> Terminates(is_even, nat) = { }
is_odd_proof:  (n: Int) -> Terminates(is_odd, nat) = { }
```

Obligations are collected by SCC, the obligation across function edges is
`m_callee(callee_args) < m_caller(caller_args)`—here `nat(n - 1) < nat(n)`, guard `n != 0`, each
function closes its own loop. The two functions have the same signature, and disambiguating by
measure alone cannot distinguish targets—this is the direct reason for `Terminates(f, m)` having two
parameters.

#### Negative Example: measure does not strictly decrease

```yaoxiang
loop_forever: (n: Int) -> Int = { return loop_forever(n) }
loop_proof: (n: Int) -> Terminates(loop_forever, nat) = { }
```

Obligation `nat(n) < nat(n)` i.e. `n < n`, its negation is always satisfiable (Sat, model `n = 0`) →
compile error **E4022** (measure does not strictly decrease), counterexample points to `loop_proof`.
If `loop_proof` is deleted, the automatic strategy fails and there is no proof function → **E4021**
(termination cannot be proven, suggesting providing a `Terminates` proof function).

### Syntax Changes

**None.** This RFC adds no new syntax, keywords, or annotation positions. Compared with the existing
lineage:

| Generation           | User-side Expression                        | Syntax Carrier                        |
| -------------------- | ------------------------------------------- | ------------------------------------- |
| RFC-022 (deprecated) | `//! decreases: n` comment annotation       | Comment convention, no type checking  |
| RFC-027 current      | None (fully automatic, error if unprovable) | ——                                    |
| This RFC             | Measure + proof, both ordinary functions    | The function definition syntax itself |

## Design Philosophy: Discovery and Verification Separation

The mechanism choice of this RFC is derived from one principle: **termination check = witness
discovery + witness verification; the two difficulties are vastly different and must be separated.**

| Role                                                                             | Bearer                                                                                            | Difficulty                                                                                        |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Discovery (synthesizing measure and proof)                                       | Four automatic strategies (machine, restricted templates) or human proof functions (unrestricted) | Generally **undecidable** (reduces to the halting problem)—no algorithm can always find a measure |
| Verification (checking `m(args') < m(args)` per call site for a fixed candidate) | SMT (RFC-009a pipeline)                                                                           | Decidable (first-order linear arithmetic), millisecond-level                                      |

"Undecidably find, decidably check"—structurally isomorphic to the P/NP witness/verifier separation,
but more extreme: the discovery side is not just hard, it is mathematically impossible to always
have a solution; the verification side is completely mechanical for a fixed candidate. RFC-027's
"honest reporting of Unproven" and human fallback are therefore not engineering compromises, but
direct corollaries of this boundary.

Three corollaries form this RFC's design constraints:

1. **The prover is not trusted, and that is fine.** Soundness entirely lives in the verifier: if the
   measure or assertion is wrong, SMT judges Sat, E4022 with counterexample sends it back—a person
   can only fail to prove, not prove incorrectly. Disproved is still constructive (provides a
   counterexample model): the verifier does half a thing more than an NP verifier, refutation also
   gives evidence.
2. **Insights freeze as re-verifiable artifacts.** Once human intellectual labor is written as a
   proof function, it becomes a machine-checkable certificate, re-verified every compilation,
   permanently regression-proof (proof-carrying).
3. **Propositions go to the compiler, proofs go to humans.** Obligations (what counts as proven)
   must be generated by the compiler from the function body; letting humans write the constraints
   too equals writing your own acceptance criteria, and the certificate no longer proves the
   actually-written code.

The four automatic strategies thus get a unified explanation: they are **a restricted template
sequence for witness synthesis**—automatic measure synthesis in the literature is template
enumeration under constraint solving (linear rank function synthesis is the precedent). Strategy
extension = template broadening; discovery outside templates is done by humans; the verifier is
always the same SMT. The non-goal "no measure inference" is also given its theoretical shape here:
general measure inference belongs to the undecidable side of discovery; only its boundary
(templates) can be agreed upon, completeness cannot be promised.

## Detailed Design

### Detailed Syntax

The proof function has no dedicated syntax—it is "compile-time predicate definition" used
differently. The three points are nailed down one by one:

**Return type: predicate application, with function references as actual arguments.**
`Terminates(gcd, gcd_measure)` is isomorphic to existing refinement applications in type position
(`f(x: Positive(x))`, `-> (result: IsMax(T, arr, result))`); function names are ordinary bindings
bound by name (RFC-010), falling within the existing predicate actual argument rules "literals,
variables (bound by name), type applications". The only rule-level addition: the E1092
actual-argument form judgment adds "**function reference is a compile-time constant form**"—changes
rule text, doesn't change grammar.

**Parameters: mirror `f`'s base types, universally quantified.** Parameter names and base types are
matched one-by-one with `f`'s formal parameters; mismatches report compile errors (reuse the E1093
family or add per registry, decided at implementation time). Formal parameter refinements are not
repeated—refinements and guard assumptions are injected by the compiler as obligation context.
Universal quantification is precisely the parameter semantics of the predicate definition itself
(the `x` in `Positive: (x: Int) -> Type = { x > 0 }` is also universally quantified). The
zero-argument form is rejected: residual obligations reference `f`'s parameter names; names must be
writable in the body.

**Body: proof space, bare Boolean expressions.** Same protocol as predicate definition body
`{ x > 0 }`—each item in the body is a bare expression asserted to be True. An empty body is legal
and is the common case (all obligations closed by SMT); non-empty assertions, after verification,
are fed to the remaining obligations as lemmas; the legal source of assertions is the application of
proven predicates (see example "Non-empty body").

**Discovery and Binding.** The compiler scans, in the same module and visible imports, for functions
whose return type is of the form `Terminates(f, _)` for `f` where automatic synthesis fails;
discovered by return type, independent of name (`f_proof` is just a natural convention). Among
multiple candidates, the first with all-green obligations is taken (declaration order); if all fail,
the first counterexample is reported. Forward references are legal: discovery happens at the check
phase, and does not depend on declaration order.

**Runtime Representation.** The proof function is a pure compile-time entity, following RFC-027's
CompileTime pattern of witness erasure—neither the measure nor the proof function enters the runtime
binary.

### Trigger Scope: Use-Site Refinement

**When** termination obligations are required is answered by RFC-027's existing dependency tracking
(it is the VC generation trigger, not an independent verification mechanism):

- **Root**: compile-time positions—type indices, predicate bodies, constant evaluation. The compiler
  itself executes these codes; totality is the load-bearing requirement.
- **Propagation**: spreads outward from the root along the call graph; hit functions are required to
  terminate—automatic strategy proves, or a `Terminates` certificate exists in the environment.
- **Runtime calls never trigger**—adopting the original text of RFC-027 §6.7's scope table; an
  infinite loop at runtime is a user program's behavior, not the compiler's safety boundary.

For consumers, this is where "type carries termination" lands: function type has no overall
attribute position syntactically (parameter and return positions have them; the function itself does
not), so "this function terminates" is not written into the type at the definition, but is checked
at **each compile-time use site**—a certificate existing in the environment is sufficient. Design
ruling (2026-09-14): the current implementation's unconditional full-coverage check is narrowed
accordingly.

### Obligation Generation

For function `f` and measure `m`, the compiler generates two sets of first-order linear arithmetic
obligations, expanded per recursive call site (including cross-function calls within an SCC); the
obligation across function edges is `m_callee(callee_args) < m_caller(caller_args)`, which
degenerates to same-measure decrease when members share the same measure:

1. **Well-foundedness**: `m(args) >= 0`—the measure falls on natural numbers, derived from parameter
   type bounds; when not derivable, it enters residual obligations.
2. **Strict decrease**: each call site `m(callee_args) < m_caller(caller_args)`, judged under **path
   guards** (guards come from branch conditions at the call site, reusing RFC-009a's path condition
   collection).

**Lexicographic expansion**: when `m` returns a tuple `(m₁, …, mₖ)`, the obligation's lexicographic
comparison expands into a disjunction chain—`(m₁' < m₁) ∨ (m₁' == m₁ ∧ m₂' < m₂) ∨ …`. Expansion
happens on the obligation generation side; the SMT side remains in the linear fragment, and does not
depend on the solver's native lexicographic support.

**Boundedness of the measure function itself**: `m` must be compile-time evaluable (constant
folding) or provable by structural recursion—prohibited from recursively pushing the termination
problem to another unproven function (prevents infinite regress). Pathological measures are caught
by existing E4012 (constant recursion too deep) and structural checks.

### Judgment Pipeline

The pipeline only runs when dependency tracking hits (see "Trigger Scope"); runtime code does not
enter this pipeline. `TerminationChecker` extends to five levels; the first three are RFC-027's
current/established paths, this RFC only adds the latter two:

```
1. Structural recursion directly provable (current logic, no regression)
2. Automatic measure synthesis: four strategies attempted one by one (strategies 1/2/4 land per RFC-027's plan; this RFC reserves the interface)
3. Synthesis success → obligation SMT judgment → Proved
4. Synthesis failure (Unproven) → search proof function (return type of form Terminates(f, _)):
     a. Found → take its measure m to generate obligations → SMT judgment
     b. Residual obligations → verified as assertions within the proof function body (same protocol as correctness-domain proof functions)
5. No proof function / proof function obligations still fail → compile error (E4021 / E4022)
```

**Proof function discovery scope**: defined in the same module, or exported and visible via
`import`—follows existing visibility rules, no global magic search. For multiple candidates
`Terminates(f, _)` for the same `f`, the first with all-green obligations is taken; if multiple
fail, the first counterexample is reported (deterministic order: declaration order).

**`Terminates` built-in declaration**: it is a special case for the predicate system—its
"assertions" are generated by the compiler from `f`'s function body, and cannot be expressed with
RFC-027's predicate syntax. This is intentionally the only built-in proposition, with the reason
recorded in the Tradeoffs section.

### Compiler Changes

| Component                            | Change                                                                                                                                                                                                                    |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/layers/termination.rs`    | SCC collection (Tarjan; the recursive call graph already exists); unify the four-strategy synthesis interface (current Phase 1 logic reabsorbed as strategy 3); proof function discovery and residual-obligation sinking  |
| `typecheck/checker.rs:405` call site | Trigger method change—from unconditional full-module traversal to dependency-tracking driven (compile-time positions as roots; see "Trigger Scope"); check position (after type check, before ownership check) maintained |
| Proof pipeline (RFC-009a/#292)       | Reuse the `ConstExpr → SMTLib` link; new obligation shapes limited to linear arithmetic + path guards, no pipeline changes                                                                                                |
| `util/diagnostic/codes/e4xxx.rs`     | Add E4021/E4022 registration (via RFC-013 registry; build.rs threshold takes effect)                                                                                                                                      |
| locales ×6                           | Six-language templates for the two new codes                                                                                                                                                                              |
| RFC-027 documentation                | §7 and the decision record calibrated per the "Amendment Scope" table, add a 027a pointer; original narrative not rewritten                                                                                               |

### Error Codes

Aligning with the E4xxx proof failure family (E4018 refinement predicate violation, E4020 proof
function needed):

| Proposed Code | Name                               | Trigger                                                                                                       |
| ------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| E4021         | Termination cannot be proven       | Automatic synthesis fails and there is no proof function (suggesting providing a `Terminates` proof function) |
| E4022         | Measure does not strictly decrease | The proof function's obligation is judged Sat by SMT (with a counterexample input attached)                   |

Final numbering is subject to the RFC-013 registry actuality at implementation time (legality
guaranteed by the build.rs threshold); W1080 (compile-time proof degradation) semantics is unrelated
to this RFC, and is not touched.

### Backward Compatibility

- Previously passing programs: zero behavior change (the first three levels of the pipeline run
  before the fallback).
- Previously failing programs: those failing due to compile-time termination obligations can pass
  after supplying a measure and proof function—this is a relaxation authorized by the re-discussion
  clause, affecting only "originally erroring" programs.
- Scope narrowing impact: **runtime** programs previously rejected by the termination check
  (non-structural recursion, loops without analyzable measures) directly pass compilation under the
  new scope—an intentional relaxation (design ruling 2026-09-14); only loosens, never tightens; the
  failure surface at compile-time positions is unchanged (still rescuable via the fallback).
- Existing test corpus (structural recursion positives, non-structural negatives) expected output
  unchanged; only the non-structural negatives' error codes/messages are updated per the new codes.

## Tradeoffs

### Pros

- **Zero new syntax**: Compared with Dafny's inline `decreases` annotation, this design occupies no
  syntax space; the measure is a unit-testable first-class function.
- **Proof domain behavior alignment**: The termination domain fills in the Unproven fallback
  consistent with the correctness domain; "hard safety philosophy" evolves from "reject if not
  provable" to "hand to the programmer if not provable"—consistent with the worldview of RFC-027
  throughout.
- **Reuse infrastructure**: Obligations are linear arithmetic, walking the pipeline #292 has already
  connected; no new backend, no new judgment logic family.
- **Clause fulfillment as governance**: Re-discussion is preserved in sub-RFC form; the decision
  record remains auditable.

### Cons and Risks

- **Writing burden**: The two-piece set (measure + proof function) is more verbose than a single
  inline annotation. Accepted—this is a low-frequency path, and in exchange the measure is reusable
  and testable.
- **`Terminates` is the only predicate with a compiler-generated body**: Deviates from the purity of
  "all predicate bodies are user-writable". Reason: call-site actual arguments live in the function
  body, library predicates in principle cannot reference them; the built-in surface converges to one
  proposition name, with zero mechanism additions (see Core Design and Design Philosophy).
- **While strategies 1/2/4 are not yet implemented, the fallback may be triggered prematurely**:
  Phase 1's automatic surface is narrow, so more functions will fall to the proof function path.
  Mitigation: after the four strategies are implemented, the fallback surface automatically narrows;
  proof functions first is not wasted—they continue to serve as residual-obligation discharge
  carriers after strategy upgrades.
- **Counterexample quality**: Sat counterexamples may not be intuitive for nonlinear guards.
  Recorded as a known limitation; counterexample presentation iterated per the RFC-013 diagnostic
  message specification.

## Non-Goals

- No new syntax/keywords/annotation positions.
- No general-purpose extension of automatic measure synthesis (the four strategies maintain
  RFC-027's established plan; beyond scope goes to the fallback).
- No joint solving with RFC-009a's borrowed propositions (each judges independently; shared
  backend).
- No totality checking at the dependent type level.
- No proof function fallback for loops: loops are anonymous constructions, `Terminates(f, m)` has
  nothing to refer to, and loop measure variables are `mut` locals rather than parameters, so the
  proof function's parameter-mirroring rule fails. Loops in scope maintain automatic
  strategies—compile-time code is code the compiler itself executes, and the hard line is preserved
  at the load-bearing point. If real needs arise in the future, a separate sub-RFC via loop
  annotation syntax.
- No measure inference implementation (guessing the measure from the function body belongs to
  strategies 1/2/4's scope; this RFC only consumes explicit measures—see the theoretical boundary in
  the last paragraph of Design Philosophy).

## Phases and Acceptance

- [x] RFC-027 in-place calibration (§7, decision record, §6.8 message; add 027a pointer)—landing in
      the same commit as this draft
- [ ] SCC collection + obligation generation (including lexicographic expansion, path guards) +
      proof function discovery
- [ ] SMT judgment wiring (reusing the #292 pipeline) + empty-body / non-empty-body two-tier
      verification
- [ ] E4021/E4022 registration + six-language locales
- [ ] E2E: positives (gcd / merge partition / `is_even`-`is_odd` mutual recursion) / negatives
      (measure does not decrease → E4022; no proof function → E4021) / structural recursion and
      existing termination tests zero regression
- [ ] Scope narrowing regression: pure-runtime recursion and loops no longer rejected by the
      termination check; obligations trigger normally when compile-time positions are hit
- [ ] Acceptance demo: deliberately write a proof function with a non-decreasing measure →
      compilation fails and the counterexample is readable; correcting it passes

## Related

- #318 (this RFC's project issue), #251 (parent milestone P1)
- RFC-027 (host: §6.7 recursion termination check, §7 termination check, decision record
  re-discussion clause)
- RFC-009a / #292 (shared SMT pipeline—infrastructure prerequisite)
- RFC-013 (error code registry and proof failure semantics family)
