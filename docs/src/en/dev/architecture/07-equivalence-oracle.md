# Equivalence Criteria for Refactoring

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiering, and execution-phase order are covered in the main
> RFC-039 text; the role of each companion document is described in the
> [index for this directory](index.md).

## Scope and Purpose

This document defines the criteria for proving that behavior remains unchanged during the refactor.
The criteria are **tiered by refactoring-step category**—because different categories require
different strengths of equivalence: pure relocation requires byte-for-byte identity, whereas SSA
transformation necessarily changes the shape of the IR and can only be judged behaviorally
equivalent.

**This document is a prerequisite for all refactoring.** Without these criteria,
[SSA transformation](04-ssa.md) cannot be implemented safely.

This document **does not cover**: the stage contracts themselves (see `02`), the design of
type-representation convergence (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The Most Dangerous Code Is Precisely the Code with Zero Test Coverage

`src/middle/core/ir_gen.rs` (8448 lines):

- The file contains **neither `#[cfg(test)]` nor `mod tests`**
- Among the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively pin
  down the sentinel behavior of DefId allocation and slot naming; all four assertions in
  `bytecode.rs` that mention ir_gen are **negative assertions** ("ir_gen does not construct a
  particular instruction")
- In other words: **IR structural invariants (dominance, unique definition, jump targets, and type
  consistency) currently have zero validation**—which is exactly what the Layer 1 criterion is meant
  to add

And this file happens to contain three classes of defects that produce no errors when they fail:

**Defect class 1: Register misalignment.** The file contains six manually written save/restore pairs
separated by more than 70 lines:

| Save point            | Restore point | Containing function         |
| --------------------- | ------------- | --------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`        |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`      |
| `2116`                | `2180`        | Intermediate curry function |
| `2224-2229`           | `2284-2288`   | Innermost curry function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`  |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`   |

The file itself states this contract at `ir_gen.rs:248-249`: "**The save/restore points for
`next_temp` must be physically adjacent; otherwise names from the inner function will leak into the
outer function.**"

The consequence of missing a restore once: `next_temp` leaks and temporary-value numbering skips
values. **The IR remains self-consistent, still passes every check, and still compiles; only the
values are wrong.** Ordinary assertion tests cannot catch this.

**Defect class 2: `arg_regs` semantic-reordering misalignment.** `generate_call_expr_ir`
(`7269-8014`, 747 lines) has six branches internally sharing the same `Vec<Operand>` and performing
semantic rewrites:

| Line        | Reordering semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

Default-value registers are also appended in the middle. **Misalignment produces no error, only
wrong values.**

More dangerously, this class of defect may **also go undetected under normalized snapshots**—if the
snapshot tool normalizes temporary register numbers, as is standard practice, then an error such as
"the third argument should have used the fifth register" is **completely identical** in the
snapshot.

**Defect class 3 (eliminated): Silent failure of a span-keyed cross-layer contract.** `ReleasePlan`
once used `Span` as its key (produced at `layers/ownership.rs:31`), passed it through
`checker.rs:1440`, and ultimately matched it through `self.release_plan.get(&stmt.span)` in
`ir_gen`, which emitted `Instruction::Drop`; any discrepancy in how either side computed spans →
**the Drop instruction silently disappeared, with no error**. `overload_resolutions` belongs to the
same pattern. **2026-10-13 (RFC-039 D20 / WBS 4.5)**: Both contracts were changed to `PlanId`
**structural-identity keys** (the unique consumption entry points `ReleasePlan::consume` /
`OverloadResolutions::get`, with the read and accounting occurring in the same call; a nonempty
difference yields an E-level `W1090`), reducing this defect class to zero—the implemented form and
criteria are described in the [02](02-stage-contract.md) §2 revision note.

> **`method_def_ordinals` is not span-keyed**: It is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> whose key is the module-unqualified `"Type.method"`)—its risk is different (a
> definition-order/name mismatch rather than a span mismatch), and SSA transformation does not solve
> it (see [04](04-ssa.md), Mechanism Three). Therefore, there are **2** span-keyed contracts
> (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for the criteria is ample:

| Asset                               | Scale                                                                                                                                                             | Can be used for                                                                                    |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpus files, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see the limitation below | End-to-end behavioral differential (**single-file side only**)                                     |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                           | Standard-library behavioral differential                                                           |
| `tests/integration/`                | 18 modules / 5846 lines                                                                                                                                           | CLI end-to-end (of which the 27 tests in `multifile.rs` are the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, per-opcode round trip)                                       | Bytecode-layer round trip                                                                          |

> **⚠️ Structural coverage limitation of the corpus (added 2026-10-03)**
>
> There is **no `yaoxiang.toml` anywhere under `tests/`** (measured: zero). Therefore, all 293
> corpus files take the `standalone` branch of `util/diagnostic/mod.rs:565`
> `check_files_with_diagnostics` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence**: Layer 3 can verify only changes to the single-file path. For the multi-file side
> of [`02`](02-stage-contract.md) (the `orchestrator`'s `check_project` / `compile_project` /
> `check_source_in_project` / `compile_embedded_module`) and the multi-file behavior of
> [`03`](03-type-unification.md) / [`04`](04-ssa.md), there is currently **no corpus coverage**.
>
> **A multi-file corpus layer must be established in P2**, otherwise the largest refactoring phase,
> "Unified Driver," has no executable criterion. The 27 tests in `tests/integration/multifile.rs`
> are the only existing multi-file-side coverage, but they test semantic outcomes rather than
> compile-stage contracts.

**These assets nevertheless share a common flaw**: they test "input → output," not whether the
intermediate representation is correct. The IR layer currently has **no correctness criterion
whatsoever**.

## Target Design

### Core Principle: Criteria Are Tiered by Refactoring Category

**This is the most important rule in this document.** Using a single criterion (usually "full IR
snapshot equality") for every refactoring category is wrong—because some refactorings **necessarily
change the shape of the IR**.

| Refactoring category                                                                  | Criterion type                                                                      | Strength  | Documents involved                                    |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | --------- | ----------------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | Normalized IR snapshot **zero-diff**                                                | Strongest | The checker split in `02`, the lowering split in `04` |
| **C2 Orchestration changes** (staging, unified Driver)                                | Same **diagnostic set** at each entry point + same corpus behavior                  | Strong    | `02`                                                  |
| **C3 Type-representation convergence**                                                | Same **diagnostic codes and messages** (order may be normalized)                    | Strong    | `03`                                                  |
| **C4 IR-shape changes** (SSA transformation)                                          | **Behavioral equivalence** (same execution results) + IR structural invariants      | Moderate  | `04`                                                  |
| **C5 Frontend-paradigm changes** (grammar-driven)                                     | Same AST snapshot + **diagnostic code+span identical item by item** + same behavior | Strong    | `05`                                                  |
| **C6 Pure deletion**                                                                  | No equivalence criterion; only confirm that no references remain                    | —         | `06`                                                  |

**The distinction between C1 and C4 is critical**: C1 requires the IR to be byte-for-byte identical
(any change is a regression); C4 acknowledges that the IR will change and instead requires "the same
program behavior + satisfaction of IR invariants."

> **⚠️ C5′ explicitly does not exist**
>
> Diagnostic requirements in the error-recovery dimension are **not relaxed**: LALRPOP's
> error-recovery model differs from `synchronize()`, and precisely for this reason, the grammar in
> [05](05-frontend-paradigm.md) **must model the behavior of `synchronize()` using explicit error
> productions**, making the diagnostics identical item by item. Failure to do so is an
> implementation defect and **does not constitute grounds for relaxing the criterion**.
>
> If phases 3a/3b empirically find that "existing diagnostics cannot be reproduced under LALRPOP,"
> the correct action is to **report the finding truthfully and reassess the proposal** (including
> the option to "retain Pratt"), **not to modify the criterion**.

### Three-Layer Criteria

#### Layer 1: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is much
stronger than a snapshot because it checks **semantic invariants** rather than a specific shape.

| Invariant                                                                             | Defects caught                                                                                                                      | Typical cause                                                  |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **Use-before-def dominance**                                                          | Every `Operand::Local(n)` read must have a definition path that dominates the current point in the CFG                              | Register misalignment, missed restore                          |
| **Value-definition uniqueness** (only in SSA form)                                    | Every SSA value must have exactly one definition point; every merge point must have a `Phi`                                         | Correctness of SSA transformation itself                       |
| **Phi consistency**                                                                   | The number of `Phi` inputs must equal the number of corresponding predecessor basic blocks, and every input type must be consistent | Missing predecessor, type mismatch                             |
| **Jump-target existence**                                                             | Every jump's label must exist in `blocks`                                                                                           | Translation errors in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global-slot bounds**                                                                | `Operand::Global(i)` must satisfy `i < globals.len()`                                                                               | Slot-allocation errors                                         |
| **Type consistency**                                                                  | An instruction's `dst` and `src` `MonoType`s must be compatible                                                                     | Bridging type-representation errors                            |
| **Inner isolation** (corresponding to the explicit commitment at `ir_gen.rs:248-249`) | Function A's `cur_locals` must not contain a temporary name from function B                                                         | The six save/restore contracts                                 |

**The verifier must exist before SSA transformation** and must **first pass on the current (non-SSA)
IR**—otherwise, it cannot be used to prove equivalence before and after the transformation.

#### Layer 2: Normalized IR Snapshot (Covers C1/C3/C5)

IR snapshots must be **normalized**; otherwise, any change to register numbering will cause the full
diff to fail:

| Normalization item         | Rule                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Strip `Span`               | Set every span field to a sentinel value. **Only starting with WBS 6.6.12 does this actually hold**: previously, the `type_decl` line printed the Debug text of the type-declaration carrier's `ast::Type`, and measurements showed that **100% of span leaks in IR snapshots came from that line** (96 lines / 69 files; the original pre-change text is archived in 09a §P6 6.6.12); after the carrier was replaced with an IR-native narrowed shape, spans no longer appear anywhere in the snapshot |
| Temporary-value renaming   | Rename in order of first occurrence to `%0 %1 %2 ...`, **preserving the definition-use topological order**                                                                                                                                                                                                                                                                                                                                                                                              |
| Global-slot relativization | Renumber in order of first occurrence within the module                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Predecessor ordering       | Sort `successors` and `Phi` inputs by label                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Struct-field ordering      | Sort only when field names are known (construction order is semantic and must not be sorted)                                                                                                                                                                                                                                                                                                                                                                                                            |

**The normalization tool's output must be checked into the repository**
(`src/middle/ir/tests/snapshots/`), updated explicitly whenever the IR definition changes, and
manually reviewed whenever it is updated.

**Known limitation (must be documented)**: Normalization erases the information about "which
register the Nth argument uses," so **Layer 2 alone is insufficient to cover defect class 2**.
During C1, the safety of `arg_regs` reordering must rely jointly on Layer 1's dominance checks and
the corpus behavioral differential.

#### Layer 3: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the change, and compare:

| Comparison target      | Normalization rule                                                                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is excluded from the comparison** (type-representation convergence may change the wording) |
| Exit code              | Compare directly                                                                                                                                  |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                             |
| `dump_bytecode` output | Compare only during C1/C2 (the bytecode will change with the IR)                                                                                  |

**Behavioral differential coverage during C4 (SSA transformation) must include**: all three
ownership transfers—ref/borrow/move; closure captures; currying (`generate_curry_*`); spawn and
iterator for; sum-type pattern matching; existential forcing points; `?` / Try propagation; method
overloading (`overload_resolutions` span-keyed); and the Drop sequences for refinement constraints
(`ReleasePlan`).

**Performance baseline**: The corpus differential guarantees "the same behavior," not "the same
elapsed time." While establishing the differential baseline in P2, use the criterion dev-dependency
already present to build and check in 2–3 smoke benchmarks (compile time for the entire corpus /
interpretation throughput for typical programs / CLI cold startup) as performance criteria for later
pipeline changes—removing slot reuse, expanding `Phi` into `Move` chains, and adding a `Driver`
indirection layer can all change timing; without a baseline, nothing will catch such regressions.

### Dedicated Criteria for Fixing Correctness Holes

The correctness hole that [`02`](02-stage-contract.md) is to fix requires an **independent
criterion** and must not be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // The diagnostic sets for the same source containing x: Sorted(3) must be equal across the single-file and multi-file paths
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written to fail** (red), confirming that it catches the existing hole,
before the fix begins. If it unexpectedly passes, the hole analysis must be reviewed again.

Similar criteria (companion to [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                                  | Defect caught                                                                                                                               |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_program_stage_coverage`        | Every `Program::stages()` ⊆ `Stage::ALL` and equal to the expected array                                                                   | A stage omitted in the future                                                                                                               |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must produce a diagnostic in every mode                                                                            | The three `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                               |
| `test_release_plan_spans_consumed`   | The ownership-produced `PlanId` set ⊆ the IR-construction consumption set (**settled by structural identity, with no allowlist**)          | `ReleasePlan` span-keyed silent Drop loss (historical form; after the key migration, the consumption ledger + `W1090` provide the fallback) |
| `test_overload_resolutions_consumed` | For each corpus file: overload-resolution output ⊆ consumption, with an **evidence floor** of `examined ≥ 1` and real consumption of `≥ 2` | Silently discarded resolution entries (silent wrong answers from falling back to default bindings)                                          |
| `test_obligations_drained`           | CI fails if any `Obligations` field has no consumption point anywhere in the repository                                                    | This entire class of defects                                                                                                                |

### Triage Criteria for Semantic Corrections (D59)

RFC-039's refactoring contract is **tiered equivalence (C1–C6) plus a dedicated semantic-correction
channel** (P3 / P3.5 + decision registry). The constraints that "C5′ does not exist" and "no
relaxation is permitted" apply to the **strength of the criteria**, not to the requirement that
incorrect semantics must be preserved unchanged. There is only one **decidable entry criterion** for
triage:

> **Question: Does the old behavior count as "successful"?** (success = compiles and produces the
> correct result)

**Does not count as success → the three classes A/B/C may be fixed as part of the refactor; all four
of the following are required:**

1. **Red before green**—the red criterion must already exist and must currently be red (the
   self-check item in [HOWTO](../HOWTO.md)). "Green-then-red" does not qualify: without a red
   criterion, it is impossible after the fact to distinguish a "fix" from a regression.
2. **No change to the externally observable behavior of previously successful programs**—programs
   that originally compiled and ran correctly must not fail because of this change, and their
   existing diagnostics (code / wording / exit code) must not change either. It is **permitted** to
   make a path that was previously unsuccessful more visible (previously no diagnostic → adding a
   W/E code or increasing severity is permitted); it is **not permitted** to change the externally
   observable behavior of a previously successful path.
3. **Same-commit registration**—mark the error type (A/B/C) on the corresponding WBS row and
   **correct the affected body text in place** (the body describes only the current state; the
   before/after diff and red→green evidence are retained in the commit message or record), and
   include the line "Error Type" in the required PR block.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshot / corpus
   differential). Classification is **not** grounds for waiving this review.

| Class                                                    | Criterion                                                                                     | Typical form                                                                                           | Precedent                               |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | --------------------------------------- |
| **A Silent failure**                                     | An error that should have been reported is swallowed                                          | Empty `Unproven` arm, silently missing solver, `unwrap_or_default` fallback                            | 3.3 / 3.4.1 (`W1063`) / 3.4.2 (`W1081`) |
| **B Silent mismatch**                                    | Key mismatch / inconsistent choice between parallel representations / span-keyed misalignment | Span key silently loses Drop; synonymous concepts have two representations                             | 3.4.5 / D20 / D57                       |
| **C Undefined behavior and uncontrolled resource usage** | Crash, stack overflow, uninitialized slot, bytecode-version mismatch                          | Pathological generic recursion (compile-time stack overflow), the three hard-coded `.42` discard sites | 3.4.8 ①/② / D52                         |

**Previously successful behavior that now needs to change → classes D/E/F still require user
adjudication first** (see “When Requesting User Adjudication (Mandatory)” in the root `AGENTS.md`):

| Class                        | Criterion                                                                                                                             | Examples                  |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-visible semantics** | Externally visible aspects of diagnostic-code additions/removals/changes, externally visible error text, module semantics, exit codes | D54 / D55 / RFC-029g      |
| **E Public contract tables** | The error-code table, opcode table, type table, or entry-stage table itself                                                           | D1 / D6 / D12 / D20 / D23 |
| **F RFC design changes**     | The metatype universe, new E codes, wholesale syntax removal                                                                          | D53 / RFC-033 / RFC-029g  |

**Primary-error attribution**: Classify a change by its **primary error type**; behavioral changes
incidentally included in the minimal modification set for the same root cause may be listed
separately on the PR's “D3 Patch Determination” line (precedent: 3.4.8 ① has primary error class C,
with three asymmetric class-A defects fixed incidentally).

**Relationship to C1/C2**: Changes in classes A/B/C **must not** be declared C1 (byte-for-byte
invariance) or C2 (identical diagnostic sets)—the definitions of those classes require unchanged
behavior; opportunistic fixes should be declared in the corresponding category among C3/C4/C6.

**Why not use a topic-based allowlist**: [08](08-maintenance-mechanism.md) has already stated that
"the exemption count itself enters the gate report"; such lists will inevitably expand. Moreover,
cases involving "one primary class + incidental fixes" inevitably fall on the boundary. The entry
criterion is boolean-decidable.

### Regression Gates

Add CI checks (consistent with the existing conventions in `scripts/ci/`):

| Check                      | Trigger                                                                | Failure condition                                                                   |
| -------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | C1/C3/C5 changes                                                       | Snapshot files differ while the commit message contains no `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                         | `verify()` returns an error for the entire corpus                                   |
| `check-corpus-parity.py`   | C2/C4/C5 changes                                                       | The corpus differential is nonempty                                                 |
| `check-stage-contract.py`  | All changes                                                            | `Program::stages()` does not match `Stage::ALL`                                     |
| `check-perf-regression.sh` | Changes touching the compiler pipeline or execution path (P4/P6/P7/P8) | The performance baseline regresses by >10% and the PR provides no explanation       |

### Bindings to Companion Documents

| Document                                                                       | Prerequisite                                                               | Criterion strength             |
| ------------------------------------------------------------------------------ | -------------------------------------------------------------------------- | ------------------------------ |
| [02-stage-contract.md](02-stage-contract.md) (staging)                         | This document's hole criterion (red first)                                 | C2: identical diagnostic set   |
| [03-type-unification.md](03-type-unification.md) (type representation)         | Layer 1 verifier's type-consistency invariant                              | C3: identical diagnostic codes |
| [04-ssa.md](04-ssa.md) (SSA transformation)                                    | **All three layers**, and the verifier must first pass on the non-SSA form | C4: behavioral equivalence     |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generatorization) | AST snapshot + diagnostic criterion                                        | C5: all three layers           |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)             | Reference scan                                                             | None                           |

## Key Decisions and Rationale

### Tiering Rather Than a Single Criterion

Using "full IR snapshot equality" as the criterion for SSA transformation (C4) would fail outright,
tempting the team to relax the criterion; tiering lets each step use a strength it can support.
Meanwhile, Layer 1's coverage of dominance checks turns "errors that cannot be tested" into
"testable invariants"—something snapshots cannot do. The criteria assets are checked into the
repository and evolve with the code; they are not a one-time investment.

### Rejected Directions

- **Using only the end-to-end corpus differential**—defect classes 1/2/3 are precisely those in
  which "the program still runs somehow," and end-to-end differentials are insensitive to them. SSA
  transformation is especially dangerous: skipped register numbers may affect only performance
  rather than output, while a span mismatch that loses a Drop **manifests only for particular
  inputs**, which the corpus may not cover.
- **Using only IR snapshots**—snapshots are inherently immune to defect class 2 (see the
  “Normalization” section) and fail completely during C4.
- **Using only property-based testing** (`proptest` is already a dependency)—random programs have
  difficulty covering ownership, existential, and refinement paths that require particular shapes to
  trigger. **It can, however, serve as a supplement**: for class 1 (simple expressions/arithmetic),
  it does indeed provide better coverage than a fixed corpus. It is recommended for introduction as
  a supplementary criterion during C4 once the corpus differential is stable.
- **Refactoring first and adding tests afterward**—this project has already made that mistake. The
  eight orphan test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1005
  lines / 78 tests that never ran) are artifacts of creating a directory first and then failing to
  wire it up. The criteria must precede the refactor.

## Known Limitations and Risks

- **Layer 1's verifier needs SSA lookahead information**. In the non-SSA form, the concept of a
  "unique definition point" does not exist (`Operand::Local(n)` refers to a slot that may be written
  multiple times). The verifier therefore needs two modes: `verify_ssa` (strict, including Phi
  checks) and `verify_loose` (dominance and type checks only, allowing multiple definitions).
  **`verify_loose` must first pass on the existing IR**; this is the largest single workload in the
  design.
- **Layer 2's normalization misses defect class 2**. This is a known limitation and can only be
  covered jointly by Layer 1 and Layer 3; it cannot be addressed by strengthening normalization.
- **Layer 3's corpus differential is sensitive to nondeterministic output**. If the `.yx` corpus
  contains output that depends on hash iteration order or time, the differential will fail
  spuriously. Such corpus files must be identified when establishing the baseline.
- **Checking snapshots into the repository creates maintenance overhead**. Whenever the IR
  definition changes, the snapshots must be explicitly updated, which requires review discipline.
- **The corpus covers only the single-file path**. There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus files take the single-file path—see the limitation under “Existing Test Assets.”

> **All open questions originally listed in this section have been adjudicated.** The item-by-item
> decisions are recorded in the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — overarching master plan
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, this
  repository's existing example of an "item-by-item round-trip" criterion
- `src/middle/core/tests/bytecode.rs:769-814` — a sentinel test that uses `include_str!` to read
  source code for static differential comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional set-difference assertions + a
  snapshot table, requiring updates whenever a new frontend is integrated
- `ir_gen.rs:248-249` — the explicit statement of the physical-adjacency contract for save/restore
  points
