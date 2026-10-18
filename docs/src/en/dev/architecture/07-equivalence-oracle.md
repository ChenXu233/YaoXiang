# Refactoring Equivalence Criteria

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage sequence are in the RFC-039
> main text; the positioning of each subsidiary document is in the [directory index](index.md).

## Positioning and Scope

This document defines the criteria for "proving behavior is unchanged" during the refactoring. The
criteria are **graded by the category of refactoring steps**—because different categories require
different strengths of equivalence: pure relocation requires byte-for-byte identity, while
SSA-ification necessarily changes IR form and can only be judged behaviorally equivalent.

**This document is a prerequisite for all refactoring.** Without these criteria, the
[SSA-ification](04-ssa.md) cannot be safely implemented.

This document **does not cover**: the stage contracts themselves (see `02`), the design of type
representation convergence (see `03`), or the dead code cleanup inventory (see `06`).

## Current State

### The most dangerous code today is exactly the code with zero test coverage

`src/middle/core/ir_gen.rs` (8448 lines):

- The file has **no `#[cfg(test)]` and no `mod tests`**
- Among the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively pin
  the sentinel behavior of DefId allocation and slot naming; the 4 assertions in `bytecode.rs` that
  mention ir_gen are all **negative assertions** ("ir_gen does not construct a certain instruction")
- That is: **IR structural invariants (dominance, single definition, jump targets, type consistency)
  currently have zero verification**—this is exactly what the first-tier criteria in this document
  aim to supplement

And this file happens to contain three categories of **defects that produce no error when they
fail**:

**Defect category 1: register misalignment.** The file contains 6 hand-written save/restore pairs,
separated by more than 70 lines:

| Save point            | Restore point | Function                    |
| --------------------- | ------------- | --------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`        |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`      |
| `2116`                | `2180`        | curry intermediate function |
| `2224-2229`           | `2284-2288`   | curry innermost function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`  |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`   |

The file itself writes down this contract at `ir_gen.rs:248-249`: "**must be physically adjacent to
the `next_temp` save/restore points**, otherwise the inner function's name will leak to the outer
one."

The consequence of missing a restore: `next_temp` leaks, temporary values skip numbers. **The IR is
still self-consistent, still passes all checks, still compiles, just with wrong values.** Ordinary
assertion tests can't catch it.

**Defect category 2: `arg_regs` semantic rearrangement misalignment.** `generate_call_expr_ir`
(`7269-8014`, 747 lines) has 6 internal branches that share the same `Vec<Operand>` and perform
semantic rewriting:

| Line        | Rearrangement semantics                      |
| ----------- | -------------------------------------------- |
| `7395`      | Namespace call: named argument rearrangement |
| `7724`      | Struct construction: field rearrangement     |
| `7835-7848` | Function call: named argument rearrangement  |

And it additionally appends default value registers mid-way. **Misalignment does not report an
error, only produces wrong values.**

What's more dangerous: this category of error **may also not be caught** under "normalized
snapshots"—if the snapshot tool normalizes away temporary register numbers (a common practice), then
"the 3rd argument should have used the 5th register" is **completely identical** in the snapshot.

**Defect category 3 (eliminated): silent failure of span-keyed cross-layer contracts.**
`ReleasePlan` was once keyed by `Span` (produced at `layers/ownership.rs:31`), passed through
`checker.rs:1440`, and finally matched by `ir_gen`'s `self.release_plan.get(&stmt.span)` to emit
`Instruction::Drop`; any inconsistency in how the two sides compute the span → **`Drop` instructions
silently disappear, with no error at all**. `overload_resolutions` belongs to the same pattern.
**2026-10-13 (RFC-039 D20 / WBS 4.5)**: both contracts have been changed to use `PlanId`
**structural identity keys** (consume via the sole entry points `ReleasePlan::consume` /
`OverloadResolutions::get`, read and account in the same call; difference set non-empty triggers
level-E `W1090`), and this category of defect has been driven to zero—the landing form and criteria
are in [02](02-stage-contract.md) §2 revision notes.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> key is unmodule-qualified `"Type.method"`)—its risk character is different (definition-order /
> name-collision mismatch rather than span mismatch), and SSA-ification does not solve it
> (disposition see [04](04-ssa.md) mechanism three). Therefore there are **2** span-keyed contracts
> (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for the criteria is abundant:

| Asset                               | Scale                                                                                                                                                      | Usable for                                                                                  |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpora, numbered by specification section (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see restrictions below | End-to-end behavioral diff (**single-file side only**)                                      |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                    | Standard library behavioral diff                                                            |
| `tests/integration/`                | 18 modules / 5846 lines                                                                                                                                    | CLI end-to-end (of which `multifile.rs` 27 tests are the **only** multi-file side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, per-opcode roundtrip)                                 | Bytecode layer roundtrip                                                                    |

> **⚠️ Structural limitations of corpus coverage (added 2026-10-03)**
>
> There are **no `yaoxiang.toml`** under the `tests/` directory (verified 0). Therefore all 293
> corpora go through the `standalone` branch of `check_files_with_diagnostics` at
> `util/diagnostic/mod.rs:565` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence**: the third-tier criteria **can only verify single-file path changes**. For the
> multi-file side of [`02`](02-stage-contract.md) (`orchestrator`'s `check_project` /
> `compile_project` / `check_source_in_project` / `compile_embedded_module`) and the multi-file
> performance of [`03`](03-type-unification.md) / [`04`](04-ssa.md), **there is currently no corpus
> coverage**.
>
> **A multi-file corpus layer must be established in P2**, otherwise the largest refactoring
> stage—"unified Driver"—has no executable criteria. The 27 tests in
> `tests/integration/multifile.rs` are the only existing multi-file side coverage, but they test
> semantic results rather than compilation-stage contracts.

**But these assets share a common deficiency**: they test "input → output", not "is the intermediate
representation correct". The IR layer's correctness **currently has no criteria at all**.

## Target Design

### Core principle: criteria are graded by refactoring category

**This is the most important point in this document.** Using a single criterion (usually "full IR
snapshot equality") to cover all refactoring types is wrong—because some refactorings **necessarily
change IR form**.

| Refactoring category                                                                  | Criteria type                                                                 | Strength  | Involved documents                          |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | --------- | ------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | IR normalized snapshot **zero-diff**                                          | Strongest | `02`'s checker split, `04`'s lowering split |
| **C2 Orchestration change** (staging, unified Driver)                                 | Same **diagnostic set** per entry + same corpus behavior                      | Strong    | `02`                                        |
| **C3 Type representation convergence**                                                | Same diagnostic code and message (order normalizable)                         | Strong    | `03`                                        |
| **C4 IR form change** (SSA-ification)                                                 | **Behavioral equivalence** (same execution result) + IR structural invariants | Medium    | `04`                                        |
| **C5 Frontend paradigm change** (grammar-driven)                                      | Same AST snapshot + **same diagnostic code+span per entry** + same behavior   | Strong    | `05`                                        |
| **C6 Pure deletion**                                                                  | No equivalence criterion needed, only confirm no references                   | —         | `06`                                        |

**The distinction between C1 and C4 is key**: C1 requires IR byte-for-byte equality (any change is a
regression); C4 acknowledges that IR will change, and uses "program behavior is the same + IR
satisfies invariants" instead.

> **⚠️ Explicitly no C5′**
>
> The diagnostic requirements for the error recovery dimension are **not relaxed**: LALRPOP's error
> recovery model differs from `synchronize()`. For this reason, the grammar in
> [05](05-frontend-paradigm.md) **must use explicit error productions to model the behavior of
> `synchronize()`**, so that diagnostics are identical entry by entry. Failure to do so is an
> implementation defect, **and does not constitute a reason to relax the criteria**.
>
> If stages 3a/3b empirically discover that "the existing diagnostics cannot be reproduced under
> LALRPOP", the correct action is to **truthfully report this finding and re-evaluate the plan**
> (including the option of "staying with Pratt"), **not to modify the criteria**.

### Three-Tier Criteria

#### Tier 1: IR Static Verifier (covers all categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is far
stronger than snapshots, because it checks **semantic invariants** rather than concrete form.

| Invariant                                                                          | What defects it catches                                                                                      | Typical cause                                                 |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| **use-before-def dominance**                                                       | Every read of `Operand::Local(n)` must have a definition path on the CFG that dominates the current point    | Register misalignment, missing restore                        |
| **Value definition uniqueness** (only under SSA form)                              | Each SSA value has exactly one definition point; merge points must have `Phi`                                | Correctness of the SSA-ification itself                       |
| **Phi consistency**                                                                | Number of `Phi` inputs == number of corresponding predecessor basic blocks, and each input has the same type | Missing predecessor, type mismatch                            |
| **jump target exists**                                                             | Every jump's label exists in `blocks`                                                                        | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot out of bounds**                                                      | `Operand::Global(i)`'s `i < globals.len()`                                                                   | Slot allocation error                                         |
| **Type consistency**                                                               | Instruction's `dst` and `src`'s `MonoType` are compatible                                                    | Bridging type representation error                            |
| **Inner isolation** (corresponding to the explicit promise at `ir_gen.rs:248-249`) | Function A's `cur_locals` does not contain function B's temporary names                                      | 6 save/restore contracts                                      |

**The verifier must exist before SSA-ification**, and must first **run green on the current
(non-SSA) IR**—otherwise it cannot be used to prove "equivalence before and after modification".

#### Tier 2: Normalized IR Snapshots (covers C1/C3/C5)

IR snapshots must be **normalized**, otherwise any register number change would cause total diff
failure:

| Normalization item         | Rule                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| Strip `Span`               | All span fields set to sentinel values                                                                  |
| Rename temporaries         | Rename in first-appearance order to `%0 %1 %2 ...`, **preserving the definition-use topological order** |
| Global slot relativization | Renumber in module-first-appearance order                                                               |
| Predecessor sorting        | Sort `successors` and `Phi` inputs by label                                                             |
| Struct field sorting       | Only sort when field names are known (construction order is semantic, do not sort)                      |

**The output of the normalization tool must be checked in** (`src/middle/core/tests/snapshots/`),
and explicitly updated when the IR definition changes; manual review of the diff is required on
update.

**Known limitation (must be stated)**: normalization will erase the information "which register the
Nth argument used", therefore **tier 2 alone is insufficient to cover defect category 2**. C1-stage
`arg_regs` rearrangement safety must be jointly guaranteed by tier 1's dominance check + corpus
behavioral diff.

#### Tier 3: End-to-end Corpus Diff (covers C2/C4/C5)

Run all **293** `.yx` files in `tests/yaoxiang/` + `src/std/tests/*.yx` once before and once after
the modification, and compare:

| Comparison target      | Normalization rule                                                                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text does not participate in comparison** (type representation convergence will change wording) |
| Exit code              | Direct comparison                                                                                                                                 |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                             |
| `dump_bytecode` output | Compare only in C1/C2 stages (bytecode will change with IR)                                                                                       |

**The behavioral diff for the C4 (SSA-ification) stage must cover**: ref/borrow/move three ownership
transfers, closure capture, currying (`generate_curry_*`), spawn and iterator for, sum-type pattern
matching, existential enforcement points, `?` / Try propagation, method overloading
(`overload_resolutions` span-keyed), refined constraint Drop sequence (`ReleasePlan`).

**Performance baseline**: corpus diff guarantees "same behavior", not "same time consumption". When
P2 establishes the diff baseline, it should also use `criterion`, which is already a dev-dependency,
to build 2-3 smoke benchmarks (full-corpus compilation time / typical program interpretation
throughput / CLI cold start) and check them in, as the performance criteria for subsequent
pipeline-like changes—slot reuse deletion, Phi expansion to Move sequences, Driver indirection
layers may all change time consumption, and without a baseline no one will catch it.

### Dedicated Criteria for Fixing Correctness Vulnerabilities

The vulnerability that [`02`](02-stage-contract.md) must fix requires **independent criteria**, and
cannot be mixed into the three tiers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // The same source containing x: Sorted(3) — diagnostic sets
    // for the single-file and multi-file paths must be equal.
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written as failing** (red), to confirm that it can catch the existing
vulnerability, before starting the fix. If it unexpectedly turns green, the vulnerability analysis
needs to be re-reviewed.

Similar criteria ([`02`](02-stage-contract.md) companion):

| Test                                 | Assertion                                                                                                                      | Defect caught                                                                                                                           |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `test_program_stage_coverage`        | Each `Program::stages()` ⊆ `Stage::ALL` and equals the expected array                                                          | Future missing stages                                                                                                                   |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must produce a diagnostic in any mode                                                                  | `checker.rs:5179/5318/5448` extend branches                                                                                             |
| `test_release_plan_spans_consumed`   | The `PlanId` set produced by ownership ⊆ the set consumed by IR construction (**settle by structural identity, no allowlist**) | `ReleasePlan` span key silently loses Drop (historical form; after key migration, the consumption ledger + `W1090` provides a backstop) |
| `test_overload_resolutions_consumed` | Per-file corpus: overload resolution output ⊆ consumption, and **evidence floor** `examined ≥ 1` / truly consumed `≥ 2`        | Resolution entries silently discarded (silently falling back to default binding's wrong answer)                                         |
| `test_obligations_drained`           | Any `Obligations` field with zero consumption points in the whole repository fails CI                                          | The entire category of this kind of defect                                                                                              |

### Triage Criteria for Semantic Corrections (D59)

The refactoring contract of RFC-039 is **graded equivalence (C1–C6) + dedicated semantic correction
channel** (P3 / P3.5 + decision register): the "no C5′" / "no relaxation set" constraints govern
**criteria strength**, not "incorrect semantics must be preserved as-is". The triage line has only
one **judgable entry criterion**:

> **Question: does the old behavior count as "successful"?** (Success = compilation passes and
> produces correct results)

**Not successful → A/B/C three categories can be fixed in passing during refactoring**, all four are
required:

1. **Red before green**—the red criterion must exist first and currently be actually red (self-check
   item in [HOWTO](../HOWTO.md)). "Green then red" doesn't hold: without a red criterion, "fix" and
   "regression" cannot be distinguished afterwards.
2. **Do not change the external behavior of "previously successful" programs**—programs that could
   compile and run correctly originally must not become failures due to this change, and their
   existing diagnostics (code / wording / exit code) must not change either. **Allowed**: making
   "previously not successful" paths louder (originally no diagnostic → adding W/E code, escalation
   all allowed); **not allowed**: touching the external behavior of "previously successful" paths.
3. **Register in the same commit**—the corresponding WBS line labels the error type (A/B/C) +
   **in-place correction of the affected main text** (main text only writes the current state;
   before/after correction diff and red→green evidence are retained with the commit message /
   record) + PR required block "Error Type" line.
4. **Regenerate the baseline in the same commit + manual review diff** (IR snapshot / corpus diff).
   The classification is **not** a reason to exempt this review.

| Category                                      | Criteria                                                                      | Typical form                                                                                       | Precedent                           |
| --------------------------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A Silent failure**                          | An error that should be reported is swallowed                                 | `Unproven` empty branch, solver missing silently, `unwrap_or_default` downgrade                    | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent mismatch**                         | Key mismatch / pick one of parallel representations / span-keyed misalignment | Span key silently drops Drop, two representations of the synonymous concept                        | 3.4.5 / D20 / D57                   |
| **C Undefined behavior and resource runaway** | Crashes, stack overflow, uninitialized slots, bytecode version mismatch       | Pathological generics recursion (compile-time stack overflow), `.42` three-place hardcoded discard | 3.4.8 ①/② / D52                     |

**Previously successful, now to change → D/E/F three categories still require user adjudication
first** (see repo root `AGENTS.md` "When requesting user adjudication (mandatory)"):

| Category                      | Criteria                                                                                                      | Example                   |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-visible semantics**  | **External-facing** of new/removed/changed diagnostic codes, external error text, module semantics, exit code | D54 / D55 / RFC-029g      |
| **E External contract table** | Error code table, opcode table, type table, entry stage table itself                                          | D1 / D6 / D12 / D20 / D23 |
| **F RFC design change**       | Meta type universe, new E codes, batch syntax removal                                                         | D53 / RFC-033 / RFC-029g  |

**Main fault attribution**: a single change is classified by **main fault type**; behavioral changes
incidentally attached to the minimum modification set of the same root cause need only be listed
separately on the PR "D3 patch judgment" line (precedent: 3.4.8 ① main fault C, attached three
A-category asymmetric defects).

**Relationship with C1/C2**: A/B/C category changes **must not** be declared as C1 (byte-for-byte
unchanged) or C2 (same diagnostic set)—the definitions of those two categories are unchanged
behavior; in-passing changes should be declared under the corresponding category in C3/C4/C6.

**Why not use a topic allowlist**: [08](08-maintenance-mechanism.md) has already declared "exemption
quantity itself is reported at the gate", and the list will bloat; moreover, "main fault one
category + attached fix" cases will inevitably get stuck at the boundary. The entry criterion is
Boolean-judgable.

### Regression Gates

New CI checks (isomorphic to existing `scripts/ci/` conventions):

| Check                      | Trigger                                                               | Failure condition                                                                       |
| -------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | C1/C3/C5 category changes                                             | Snapshot file has diff and commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                        | `verify()` returns non-empty for the full corpus                                        |
| `check-corpus-parity.py`   | C2/C4/C5 category changes                                             | Corpus diff is non-empty                                                                |
| `check-stage-contract.py`  | All                                                                   | `Program::stages()` does not match `Stage::ALL`                                         |
| `check-perf-regression.sh` | Changes touching compilation pipeline or execution path (P4/P6/P7/P8) | Performance baseline regression >10% and PR has no explanation                          |

### Binding to Subsidiary Documents

| Document                                                                       | Must have first                                                            | Criteria strength          |
| ------------------------------------------------------------------------------ | -------------------------------------------------------------------------- | -------------------------- |
| [02-stage-contract.md](02-stage-contract.md) (staging)                         | The vulnerability criteria in this document (red first)                    | C2: same diagnostic set    |
| [03-type-unification.md](03-type-unification.md) (type representation)         | Tier 1 verifier's type-consistency invariant                               | C3: same diagnostic code   |
| [04-ssa.md](04-ssa.md) (SSA-ification)                                         | **All three tiers**, and the verifier must run green first on non-SSA form | C4: behavioral equivalence |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generatorization) | AST snapshot + diagnostic criteria                                         | C5: all three used         |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)             | Reference scan                                                             | None                       |

## Key Decisions and Rationale

**Grading rather than a single criterion.** Using "IR snapshot equality" as the criterion for
SSA-ification (C4) would fail directly, thereby inducing the team to relax the criteria; grading
lets each step use the strength it can bear. At the same time, the tier 1 verifier covers the
dominance check, turning "untestable errors" into "testable invariants"—this is something snapshots
cannot do. Criteria assets are checked in and evolve with the code, not a one-time investment.

### Unadopted Directions

- **Using only end-to-end corpus diff**—defect categories 1/2/3 are exactly the kind where "the
  program still ran in some way", and end-to-end diff is insensitive to them. SSA-ification is
  especially dangerous: register number skipping may only affect performance, not output, while Drop
  loss caused by span mismatch **only manifests under specific inputs**, which the corpus may not
  cover.
- **Using only IR snapshots**—snapshots are inherently immune to defect category 2 (see
  "Normalization" section), and completely fail in the C4 stage.
- **Using only property-based testing** (proptest is already in dependencies)—random programs are
  hard to cover the paths that need specific shapes to trigger: ownership, existential, refinement.
  **But it can be a supplement**: for the first category (simple expressions/arithmetic), it does
  cover better than fixed corpora, and it is recommended to introduce it as a supplementary
  criterion for the C4 stage after the corpus diff stabilizes.
- **Refactor first, then add tests**—this is a pitfall this project has already stepped into. The 8
  orphan test trees (1005 lines / 78 tests that never ran) recorded in
  [`06-cleanup-inventory.md`](06-cleanup-inventory.md) are the product of "build the directory
  first, fail to wire it later". Criteria must precede refactoring.

## Known Limitations and Risks

- **Tier 1 verifier needs SSA lookahead information.** Under non-SSA form, the concept of "single
  definition point" does not exist (`Operand::Local(n)` points to a slot that can be written to
  multiple times). Therefore the verifier needs two modes: `verify_ssa` (strict, including Phi
  check) and `verify_loose` (only dominance and types, allowing multiple definitions).
  **`verify_loose` must run green on the existing IR first**—this is the largest single workload of
  this design.
- **Tier 2 normalization will miss defect category 2.** This is a known limitation, and can only be
  jointly covered by tier 1 + tier 3, not solved by strengthening normalization.
- **Tier 3 corpus diff is sensitive to non-deterministic output.** If there are `.yx` corpora that
  depend on hash iteration order / time in their output, the diff will fail falsely. Such corpora
  need to be filtered out when establishing the baseline.
- **Snapshot check-in will bring maintenance cost.** When the IR definition changes, snapshots need
  to be explicitly updated, and review discipline is required.
- **The corpus only covers single-file paths.** There is no `yaoxiang.toml` under `tests/`, and all
  293 corpora go through single-file paths—see the restrictions described in "Existing Test Assets"
  for details.

> **The open questions originally listed in this section have all been adjudicated.** The
> decision-by-decision list is in
> [RFC-039 decision register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document does not retain any pending items.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — upper-level outline
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, the
  existing "per-item roundtrip" criteria exemplar in this repository
- `src/middle/core/tests/bytecode.rs:769-814` — sentinel tests that use `include_str!` to read
  source code for static snapshot comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional difference set assertion + snapshot
  table, forcing updates when a new frontend is connected
- `ir_gen.rs:248-249` — explicit declaration of the save/restore physical-adjacency contract
