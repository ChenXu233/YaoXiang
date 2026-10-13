# Refactoring Equivalence Criteria

> **Ancillary design document**. This document is an ancillary companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, grading of acceptance criteria, and execution stage ordering are defined in the
> RFC-039 main text; the positioning of each ancillary document is described in
> [this directory index](index.md).

## Positioning and Scope

This document defines the criteria for "proving behavior is unchanged" during the refactoring.
Criteria are **graded by the category of the refactoring step**—because different categories require
different strengths of equivalence: pure relocation requires byte-for-byte identity, while SSA
conversion necessarily changes IR form and can only be judged by behavioral equivalence.

**This document is a prerequisite for all refactoring.** Without these criteria,
[SSA conversion](04-ssa.md) cannot be safely performed.

This document **does not cover**: stage contracts themselves (see `02`), type representation
convergence design (see `03`), or the dead code cleanup inventory (see `06`).

## Current Status

### The most dangerous code today happens to have zero test coverage

`src/middle/core/ir_gen.rs` (8448 lines):

- The file has **no `#[cfg(test)]`, no `mod tests`**
- Among the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively pin
  sentinel behavior of DefId allocation and slot naming; the 4 assertions in `bytecode.rs` that
  mention ir_gen are all **negative assertions** ("ir_gen does not construct a certain instruction")
- That is: **IR structural invariants (dominance, single definition, jump targets, type consistency)
  currently have zero verification**—this is exactly what the first-layer criterion must fill

And this file happens to contain three categories of **defects that produce no error when they
fail**:

**Defect category 1: register misalignment.** The file contains 6 pairs of manual save/restore
statements, separated by more than 70 lines:

| Save site             | Restore site | Containing function        |
| --------------------- | ------------ | -------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`  | `generate_method_ir`       |
| `1858-1866`           | `2026-2029`  | `generate_function_ir`     |
| `2116`                | `2180`       | curry middle function      |
| `2224-2229`           | `2284-2288`  | curry innermost function   |
| `2768-2777`           | `2820-2851`  | `generate_anon_binding_ir` |
| `4686-4695`           | `4767-4774`  | `generate_lambda_body_ir`  |

The file itself writes down this contract at `ir_gen.rs:248-249`: "**must be physically adjacent to
the `next_temp` save/restore site**, otherwise inner-function names will leak into outer ones".

Consequence of a missing restore: `next_temp` leaks, temporary values jump slots. **The IR remains
self-consistent, still passes all checks, still compiles, but the values are wrong.** Ordinary
assertion tests won't catch it.

**Defect category 2: `arg_regs` semantic rearrangement misalignment.** Inside
`generate_call_expr_ir` (`7269-8014`, 747 lines) 6 branches share a single `Vec<Operand>` and
perform semantic rewriting:

| Line        | Rearrangement semantics                      |
| ----------- | -------------------------------------------- |
| `7395`      | Namespace call: named-argument rearrangement |
| `7724`      | Struct construction: field rearrangement     |
| `7835-7848` | Function call: named-argument rearrangement  |

And it also appends default-value registers mid-flight. **Misalignment produces no error, only wrong
values.**

More dangerously: this category of error **may also go undetected under "normalized snapshots"**—if
the snapshot tool normalizes away temporary register numbers (a common practice), then "the 3rd
argument should have used the 5th register" type errors are **completely identical** in the
snapshot.

**Defect category 3 (eliminated): silent failure of span-keyed cross-layer contracts.**
`ReleasePlan` was once keyed by `Span` (produced at `layers/ownership.rs:31`), passed through
`checker.rs:1440`, finally matched and emitted as `Instruction::Drop` by
`self.release_plan.get(&stmt.span)` in `ir_gen`; any inconsistency in how the two sides compute span
→ **`Drop` instructions silently disappear, with no error whatsoever**. `overload_resolutions`
follows the same pattern. **2026-10-13 (RFC-039 D20 / WBS 4.5)**: both contracts have been changed
to `PlanId` **structural-identity keys** (with single consumption entry points
`ReleasePlan::consume` / `OverloadResolutions::get`; read and bookkeeping in the same call;
non-empty difference triggers E-level `W1090`), this category of defect is reduced to zero—the
landing form and criteria are described in [02](02-stage-contract.md) §2 Revision Notes.

> **`method_def_ordinals` is not span-keyed**: it is `HashMap<String, usize>` (`ir_gen.rs:224`, with
> key being the non-module-qualified `"Type.method"`)—its risk profile is different
> (definition-order/naming mismatch rather than span mismatch), and SSA conversion does not solve it
> (disposition in [04](04-ssa.md) Mechanism 3). Therefore the span-keyed contracts are **2**
> (`ReleasePlan` / `overload_resolutions`).

### Existing test assets

The good news is that the raw material for criteria is ample:

| Asset                               | Scale                                                                                                                                                           | Usable for                                                                                   |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpus files, organized by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see limitations below | End-to-end behavior diff (**single-file side only**)                                         |
| `src/std/tests/*.yx`                | a batch                                                                                                                                                         | Standard library behavior diff                                                               |
| `tests/integration/`                | 18 modules / 5846 lines                                                                                                                                         | CLI end-to-end (of which `multifile.rs`'s 27 tests are the **only** multifile-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, per-opcode round-trip)                                     | Bytecode-layer round-trip                                                                    |

> **⚠️ Structural limitation of corpus coverage (added 2026-10-03)**
>
> There are **no `yaoxiang.toml` files at all** under the `tests/` directory (verified, 0 files).
> Therefore all 293 corpus files go through the `standalone` branch of
> `check_files_with_diagnostics` at `util/diagnostic/mod.rs:565` → `check_single_file` →
> **`Pipeline::run` (single-file path)**.
>
> **Consequence**: the third-layer criterion **can only verify changes on the single-file path**.
> For the multifile side in [`02`](02-stage-contract.md) (`orchestrator`'s `check_project` /
> `compile_project` / `check_source_in_project` / `compile_embedded_module`) and the multifile
> behavior in [`03`](03-type-unification.md) / [`04`](04-ssa.md), **there is currently no corpus
> coverage at all**.
>
> **A multifile corpus layer must be built in P2**, otherwise the "unified Driver" stage—the largest
> refactoring stage—has no executable criterion. The 27 tests in `tests/integration/multifile.rs`
> are the only existing multifile-side coverage, but they test semantic results, not
> compilation-stage contracts.

**But these assets share one common defect**: they test "input → output", not "whether the
intermediate representation is correct". Correctness at the IR layer currently has **no criteria at
all**.

## Target Design

### Core principle: criteria are graded by refactoring category

**This is the single most important point in this document.** Using a single criterion (usually "IR
snapshot equality") to cover all refactoring types is wrong—because some refactorings **necessarily
change the IR form**.

| Refactoring category                                                         | Criterion type                                                                                | Strength  | Involving document                          |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | --------- | ------------------------------------------- |
| **C1 Pure relocation** (split files, change directories, extract submodules) | IR normalized snapshot **zero-diff**                                                          | Strongest | `02`'s checker split, `04`'s lowering split |
| **C2 Orchestration change** (staged, unified Driver)                         | Each entry's **diagnostic set is identical** + corpus behavior identical                      | Strong    | `02`                                        |
| **C3 Type representation convergence**                                       | Diagnostic codes and messages **identical** (order can be normalized)                         | Strong    | `03`                                        |
| **C4 IR form change** (SSA conversion)                                       | **Behavioral equivalence** (execution results identical) + IR structural invariants           | Medium    | `04`                                        |
| **C5 Frontend paradigm change** (grammar-driven)                             | AST snapshot identical + **diagnostic code+span identical item by item** + behavior identical | Strong    | `05`                                        |
| **C6 Pure deletion**                                                         | No equivalence criterion needed, only confirm no references                                   | —         | `06`                                        |

**The distinction between C1 and C4 is key**: C1 requires IR to be byte-for-byte identical (any
change is a regression); C4 acknowledges that IR will change, and switches to "program behavior
identical + IR satisfies invariants".

> **⚠️ There is explicitly no C5′**
>
> Diagnostic requirements on the error-recovery dimension are **not relaxed**: LALRPOP's
> error-recovery model differs from `synchronize()`; for this very reason, the grammar in
> [05](05-frontend-paradigm.md) **must use explicit error productions to model the behavior of
> `synchronize()`**, so that diagnostics are item-by-item consistent. Failure to do so is an
> implementation defect, **not a reason to relax criteria**.
>
> If in stage 3a/3b it is empirically found that "existing diagnostics cannot be reproduced under
> LALRPOP", the correct action is to **truthfully report this finding and re-evaluate the plan**
> (including the option "stay with Pratt"), **not to modify the criteria**.

### Three-layer criteria

#### Layer 1: IR static verifier (covers all categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` at `src/middle/ir/verify.rs`. It is much
stronger than snapshots, because it checks **semantic invariants** rather than concrete form.

| Invariant                                                                                   | What defects it catches                                                                                        | Typical cause                                                 |
| ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **Use-before-def dominance**                                                                | Each `Operand::Local(n)` that is read must have a definition path on the CFG that dominates the current point  | Register misalignment, missed restore                         |
| **Value definition uniqueness** (only under SSA form)                                       | Each SSA value has exactly one definition point; join points must have `Phi`                                   | Correctness of SSA conversion itself                          |
| **Phi consistency**                                                                         | Number of `Phi` inputs == number of corresponding predecessor basic blocks, and each input has consistent type | Missing predecessor, type mismatch                            |
| **Jump target exists**                                                                      | Each jump's label exists in `blocks`                                                                           | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot bounds**                                                                      | `Operand::Global(i)`'s `i < globals.len()`                                                                     | Slot allocation error                                         |
| **Type consistency**                                                                        | The `dst` and `src` of an instruction have compatible `MonoType`                                               | Bridging type-representation error                            |
| **Inner-function isolation** (corresponding to the explicit promise at `ir_gen.rs:248-249`) | Function A's `cur_locals` does not contain function B's temporary names                                        | The 6 save/restore contracts                                  |

**The verifier must exist before SSA conversion**, and must be made to pass on **current (non-SSA)
IR first**—otherwise it cannot be used to prove "before and after modification are equivalent".

#### Layer 2: Normalized IR snapshot (covers C1/C3/C5)

IR snapshots must be **normalized**, otherwise any register-number change will cause full diff
failure:

| Normalization item      | Rule                                                                                                   |
| ----------------------- | ------------------------------------------------------------------------------------------------------ |
| Strip `Span`            | All span fields set to sentinel values                                                                 |
| Rename temporaries      | Rename in order of first appearance as `%0 %1 %2 ...`, **preserving the define-use topological order** |
| Relativize global slots | Renumber in order of first appearance within the module                                                |
| Sort predecessors       | `successors` and `Phi` inputs sorted by label                                                          |
| Sort struct fields      | Only when field names are known (construction order is semantic, do not sort)                          |

**The output of the normalization tool must be checked in** (`src/middle/core/tests/snapshots/`),
updated explicitly when IR definitions change, and the diff must be human-reviewed on update.

**Known limitation (must be stated)**: normalization will erase the information "which register the
Nth argument used", so **Layer 2 alone is insufficient to cover defect category 2**. The safety of
`arg_regs` rearrangement in the C1 stage must be guaranteed jointly by Layer 1's dominance check and
corpus behavior diff.

#### Layer 3: End-to-end corpus diff (covers C2/C4/C5)

Run all **293** `.yx` files in `tests/yaoxiang/` plus `src/std/tests/*.yx` before and after the
modification, then compare:

| Comparison target      | Normalization rule                                                                                                                                    |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text does not participate in the comparison** (type-representation convergence will change wording) |
| Exit code              | Direct comparison                                                                                                                                     |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                                 |
| `dump_bytecode` output | Only compare in C1/C2 stages (bytecode will change with IR)                                                                                           |

**The behavior diff for the C4 (SSA conversion) stage must cover**: ref/borrow/move three ownership
transfers, closure capture, currying (`generate_curry_*`), spawn and iterator `for`, sum-type
pattern matching, existential coercion points, `?` / Try propagation, method overloading
(`overload_resolutions` span-keyed), refinement-constraint `Drop` sequence (`ReleasePlan`).

**Performance baseline**: corpus diff guarantees "behavior identical", not "time-consumed
identical". When P2 establishes the diff baseline, also use the existing `criterion` dev-dependency
to create 2-3 smoke benchmarks (full corpus compile time / typical program interpretation throughput
/ CLI cold start) and check them in as performance criteria for subsequent pipeline changes—slot
reuse deletion, Phi expansion to Move chains, Driver indirection layer can all change time
consumption, and without a baseline no one can intercept it.

### Dedicated criteria for fixing correctness holes

The hole that [`02`](02-stage-contract.md) needs to fix requires **independent criteria**, which
cannot be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets
    // for the single-file and multifile paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must be written first as failing** (red), to confirm that it can catch the existing
hole, before starting the fix. If it is unexpectedly green, the hole analysis needs to be
re-verified.

Similar criteria (paired with [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                          | Defect intercepted                                                                                                                 |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `test_program_stage_coverage`        | Every `Program::stages()` ⊆ `Stage::ALL` and equals the expected array                                                             | Future stage connection miss                                                                                                       |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must produce a diagnostic in any mode                                                                      | `checker.rs:5179/5318/5448` extend branches                                                                                        |
| `test_release_plan_spans_consumed`   | The set of `PlanId` produced by ownership ⊆ the set consumed by IR construction (**settled by structural identity, no whitelist**) | `ReleasePlan` span key silently dropping `Drop` (historical form; after key migration, guaranteed by consumption ledger + `W1090`) |
| `test_overload_resolutions_consumed` | Corpus file-by-file: overload-resolution output ⊆ consumption, and **evidence floor** `examined ≥ 1` / genuinely consumed `≥ 2`    | Resolution entries silently discarded (falling back to silent wrong answer of default binding)                                     |
| `test_obligations_drained`           | Any `Obligations` field with no consumption point in the whole repo fails CI                                                       | The entire category of this class of defect                                                                                        |

### Splitting criteria for semantic corrections (D59)

The refactoring contract in RFC-039 is **graded equivalence (C1–C6) + dedicated semantic-correction
channels** (P3 / P3.5 + decision registry): "There is no C5′" / "no relaxation" constrains the
**strength of criteria**, not "the semantics of errors must be preserved as-is". The split line has
only one **judgable entry criterion**:

> **Q: Does the old behavior count as "successful"?** (Success = compilation passes and produces
> correct results)

**Does not count as success → A/B/C three categories can be fixed in passing during refactoring**;
all four conditions are required:

1. **Red before green**—the red criterion must exist first and be currently genuinely red (see
   [HOWTO](../HOWTO.md) self-check items). "Green then red" does not hold: without a red criterion,
   "fix" and "regression" cannot be distinguished afterwards.
2. **Does not change the external behavior of "originally successful" programs**—programs that
   previously compiled and ran correctly must not become failures due to this change, and their
   existing diagnostics (code / wording / exit code) must not change. **Allowed**: making paths that
   "were not originally successful" more visible (originally no diagnostic → newly added W/E code,
   increased severity are all fine); **not allowed**: touching the external behavior of "originally
   successful" paths.
3. **Same-commit registration**—the WBS corresponding row must annotate the error type (A/B/C) + the
   revision note of that stage + a mandatory "Error Type" line in the PR.
4. **Baseline regenerated in the same commit + human-review diff** (IR snapshot / corpus diff).
   Classification is **not** a reason to exempt that review.

| Class                                         | Criterion                                                                        | Typical form                                                                                  | Precedent                           |
| --------------------------------------------- | -------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A Silent failure**                          | The error that should be reported was swallowed                                  | `Unproven` empty arm, silent solver omission, `unwrap_or_default` degradation                 | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent mismatch**                         | Key mismatch / pick-one between parallel representations / span-key misalignment | Span key silently drops `Drop`, two parallel representations of the same concept              | 3.4.5 / D20 / D57                   |
| **C Undefined behavior and resource runaway** | Crashes, stack overflow, uninitialized slots, bytecode version mismatch          | Pathological generic recursion (compile-time stack overflow), `.42` discarded in three places | 3.4.8 ①/② / D52                     |

**Originally successful, now to be changed → D/E/F three categories must still go through user
arbitration first** (see repository-root `AGENTS.md` "When requesting user arbitration
(mandatory)"):

| Class                          | Criterion                                                                                                         | Example                   |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-visible semantics**   | External-facing side of diagnostic code addition/removal/change, external error text, module semantics, exit code | D54 / D55 / RFC-029g      |
| **E External contract tables** | Error code table, opcode table, type table, entry-stage table themselves                                          | D1 / D6 / D12 / D20 / D23 |
| **F RFC design change**        | Meta-type universe, new E codes, batch syntax removal                                                             | D53 / RFC-033 / RFC-029g  |

**Primary-error attribution**: a single change is classified by its **primary-error type**;
behavioral changes that come along within the same root-cause minimum modification set can be listed
separately in the PR's "D3 Patch Determination" line (precedent: 3.4.8 ① primary error class C, with
three asymmetric class-A defects attached).

**Relationship to C1/C2**: A/B/C-class changes **must not** be declared as C1 (byte-for-byte
identical) or C2 (diagnostic set identical)—those two classes are defined as behavior-unchanged; a
fix-in-passing should be declared as the corresponding category in C3/C4/C6.

**Why not use a topic whitelist**: [08](08-maintenance-mechanism.md) has already declared "the
number of exemptions itself enters the gate report", and the list would balloon; and "primary error
one class + attached fix" cases are inevitably caught on the boundary. The entry criterion is
Boolean-judgable.

### Regression gates

New CI checks (isomorphic to existing `scripts/ci/` conventions):

| Check                      | Trigger                                                                   | Failure condition                                                        |
| -------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `check-snapshot-drift.sh`  | C1/C3/C5 class changes                                                    | Snapshot file has diff but commit message lacks `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns non-empty for full corpus                             |
| `check-corpus-parity.py`   | C2/C4/C5 class changes                                                    | Corpus diff is non-empty                                                 |
| `check-stage-contract.py`  | All                                                                       | `Program::stages()` does not match `Stage::ALL`                          |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | Performance baseline regresses >10% and PR lacks explanation             |

### Bindings with each ancillary document

| Document                                                                        | Must pre-exist                                                                        | Criterion strength            |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ----------------------------- |
| [02-stage-contract.md](02-stage-contract.md) (staging)                          | The hole criteria in this document (red first)                                        | C2: diagnostic set identical  |
| [03-type-unification.md](03-type-unification.md) (type representation)          | Layer 1 verifier's type-consistency invariant                                         | C3: diagnostic code identical |
| [04-ssa.md](04-ssa.md) (SSA conversion)                                         | **All three layers**, and the verifier must be made to pass on the non-SSA form first | C4: behavioral equivalence    |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generator-ization) | AST snapshot + diagnostic criterion                                                   | C5: all three used together   |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)              | Reference scan                                                                        | None                          |

## Key Decisions and Rationale

**Graded rather than single criterion.** Using "IR snapshot equality" as the criterion for SSA
conversion (C4) will fail directly, which would induce the team to relax the criteria; grading lets
each step use the strength it can bear. At the same time Layer 1's verifier covers dominance checks,
turning "unmeasurable errors" into "measurable invariants"—something snapshots cannot do. Criteria
assets are checked in and evolve with the code; this is not a one-time investment.

### Directions not adopted

- **Using only end-to-end corpus diff**—defect categories 1/2/3 happen to be the kind "the program
  still ran in some way", and end-to-end diff is insensitive to them. SSA conversion is especially
  dangerous: register number-jumps may only affect performance without affecting output, while
  `Drop` loss from span mismatch **only manifests under specific inputs**, which the corpus may not
  cover.
- **Using only IR snapshots**—snapshots are inherently immune to defect category 2 (see the
  "Normalization" section), and completely fail in the C4 stage.
- **Using only property-based testing** (proptest is already a dependency)—random programs are hard
  to cover ownership, existential, and refinement paths that need specific shapes to trigger. **But
  it can be a complement**: for category 1 (simple expressions/arithmetic) it is indeed better than
  fixed-corpus coverage; recommend introducing it as a supplementary criterion for the C4 stage
  after the corpus diff stabilizes.
- **Refactoring first, then adding tests**—this is the trap the project has already stepped in. The
  8 orphan test trees (1005 lines / 78 tests that have never run) recorded in
  [`06-cleanup-inventory.md`](06-cleanup-inventory.md) are the product of "create directories first,
  fail to wire later". Criteria must precede refactoring.

## Known Limitations and Risks

- **Layer 1 verifier needs SSA look-ahead information.** Under the non-SSA form, the concept of
  "single definition point" does not exist (`Operand::Local(n)` points to a slot that can be written
  multiple times). Therefore the verifier needs two modes: `verify_ssa` (strict, with Phi check) and
  `verify_loose` (only dominance and type, allowing multiple definitions). **`verify_loose` must be
  made to pass on existing IR first**; this is the largest single workload of this design.
- **Layer 2 normalization will miss defect category 2**. This is a known limitation and can only be
  covered jointly by Layer 1 + Layer 3, not by strengthening normalization.
- **Layer 3's corpus diff is sensitive to non-deterministic output.** If `.yx` corpus contains
  output that depends on hash iteration order / time, the diff will falsely fail. Such corpus items
  need to be screened out when establishing the baseline.
- **Snapshot check-in brings maintenance cost.** When IR definitions change, snapshots must be
  updated explicitly, which requires review discipline.
- **Corpus only covers the single-file path.** There is no `yaoxiang.toml` under `tests/`, and all
  293 corpus files go through the single-file path—see the limitations note in "Existing test
  assets" for details.

> **The open questions originally listed in this section have all been adjudicated.** The decision
> for each is in [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — The overarching charter
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, the
  existing "per-item round-trip" criterion exemplar in this repository
- `src/middle/core/tests/bytecode.rs:769-814` — Sentinel test that uses `include_str!` to read
  source and perform static comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — Bidirectional difference-set assertion + snapshot
  table, forcing new frontend integrations to update
- `ir_gen.rs:248-249` — The explicit declaration of the save/restore physical-adjacency contract
