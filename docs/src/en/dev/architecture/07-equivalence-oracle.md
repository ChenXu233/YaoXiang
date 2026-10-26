# Equivalence Criteria for Refactoring

> **Companion design document.** This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiering of acceptance criteria, and execution-stage order are described in the
> main RFC-039 text; see the [directory index](index.md) for the role of each companion document.

## Scope and Positioning

This document defines the criteria for proving that “behavior remains unchanged” during refactoring.
The criteria are **tiered by refactoring-step category**—because different categories require
different strengths of equivalence: pure relocation requires byte-for-byte identity, whereas SSA
conversion necessarily changes the shape of the IR and can only be judged behaviorally equivalent.

**This document is a prerequisite for all refactoring.** Without these criteria,
[SSA conversion](04-ssa.md) cannot be implemented safely.

This document **does not cover**: the stage contracts themselves (see `02`), the design for
converging type representations (see `03`), or the inventory for dead-code cleanup (see `06`).

## Current State

### The Most Dangerous Code Today Is Exactly the Code with Zero Test Coverage

`src/middle/core/ir_gen.rs` (8,448 lines):

- The file contains **no `#[cfg(test)]` and no `mod tests`**
- Of the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively pin down
  the sentinel behavior of DefId allocation and slot naming; all 4 assertions in `bytecode.rs` that
  mention ir_gen are **negative assertions** (“ir_gen does not construct a particular instruction”)
- In other words: **IR structural invariants (dominance, unique definitions, jump targets, and type
  consistency) currently have zero validation**—which is exactly what the first layer of criteria
  must supply

This file also happens to contain three classes of defects that **produce no errors when they
fail**:

**Defect Class 1: Register Misalignment.** The file contains 6 manually paired save/restore
statements, separated by more than 70 lines:

| Save point            | Restore point | Containing function           |
| --------------------- | ------------- | ----------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`          |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`        |
| `2116`                | `2180`        | Curried intermediate function |
| `2224-2229`           | `2284-2288`   | Innermost curried function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`    |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`     |

The file itself states this contract at `ir_gen.rs:248-249`: “**It must be physically adjacent to
the `next_temp` save/restore point; otherwise names from the inner function will bleed into the
outer function.**”

The consequence of missing a single restore is: `next_temp` leaks and temporary-value numbers are
skipped. **The IR remains self-consistent, still passes every check, and still compiles—it merely
has incorrect values.** Ordinary assertion tests cannot catch this.

**Defect Class 2: Semantic Reordering Misalignment in `arg_regs`.** The 6 branches inside
`generate_call_expr_ir` (`7269-8014`, 747 lines) share the same `Vec<Operand>` and perform semantic
rewriting:

| Line        | Reordering semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

Default-value registers are also appended along the way. **Misalignment does not report an error; it
only produces incorrect values.**

This is even more dangerous because this class of error **may also go undetected under a “normalized
snapshot”**—if the snapshot tool normalizes temporary register numbers away, as is customary, then
an error such as “the third argument should have used the fifth register” is **completely
identical** in the snapshot.

**Defect Class 3 (Eliminated): Silent Failure of a Span-Keyed Cross-Layer Contract.** `ReleasePlan`
previously used `Span` as its key (produced by `layers/ownership.rs:31`), passed it through
`checker.rs:1440`, and ultimately matched it with `self.release_plan.get(&stmt.span)` in `ir_gen`
before emitting `Instruction::Drop`; any inconsistency in how either side computed spans meant that
the **Drop instruction silently disappeared, with no error**. `overload_resolutions` has the same
shape. **2026-10-13 (RFC-039 D20 / WBS 4.5):** Both contracts were changed to **structural-identity
keys** using `PlanId` (the sole consumption entry points are `ReleasePlan::consume` /
`OverloadResolutions::get`, with the read and accounting performed in the same invocation; a
non-empty difference produces an E-level `W1090`). This class of defect has been reduced to zero—see
the revision note in [02](02-stage-contract.md) §2 for its implemented form and criteria.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> with the unqualified key `"Type.method"`)—its risk profile is different (definition-order or
> name-collision mismatch rather than span mismatch), and SSA conversion does not address it (see
> Mechanism 3 in [04](04-ssa.md)). Therefore, there are **2** span-keyed contracts (`ReleasePlan` /
> `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for the criteria is sufficient:

| Asset                               | Scale                                                                                                                                                             | Can be used for                                                                                            |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293 corpus files**, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see the limitation below | End-to-end behavioral differential (**single-file side only**)                                             |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                           | Standard-library behavioral differential                                                                   |
| `tests/integration/`                | 18 modules / 5,846 lines                                                                                                                                          | CLI end-to-end testing (of which the 27 tests in `multifile.rs` are the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1,131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, round trip for every opcode)                                | Bytecode-layer round trips                                                                                 |

> **⚠️ Structural coverage limitation of the corpus (added 2026-10-03)**
>
> There is no `yaoxiang.toml` anywhere under the `tests/` directory (measured count: 0). Therefore,
> all 293 corpus files take the `standalone` branch of `check_files_with_diagnostics` at
> `util/diagnostic/mod.rs:565` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence:** The third layer of criteria can **only validate changes on the single-file
> path**. There is currently **no corpus coverage** for the multi-file side of
> [`02`](02-stage-contract.md) (the orchestrator’s `check_project` / `compile_project` /
> `check_source_in_project` / `compile_embedded_module`) or for the multi-file behavior of
> [`03`](03-type-unification.md) / [`04`](04-ssa.md).
>
> **A multi-file corpus layer must be established in P2**; otherwise, the largest refactoring
> stage—the “unified Driver”—has no executable criterion. The 27 tests in
> `tests/integration/multifile.rs` are the only existing multi-file-side coverage, but they test
> semantic outcomes rather than compilation-stage contracts.

**However, these assets have one common shortcoming**: they test “input → output,” not whether the
intermediate representation is correct. The correctness of the IR layer currently has **no criterion
whatsoever**.

## Target Design

### Core Principle: Criteria Are Tiered by Refactoring Category

**This is the most important point in this document.** Using a single criterion—usually “full
equality of the IR snapshot”—for every type of refactoring is incorrect, because some refactorings
**necessarily change the shape of the IR**.

| Refactoring category                                                                      | Criterion type                                                                                | Strength  | Documents involved                                    |
| ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | --------- | ----------------------------------------------------- |
| **C1 Pure Relocation** (splitting files, reorganizing directories, extracting submodules) | Normalized IR snapshot **zero-diff**                                                          | Strongest | The checker split in `02`, the lowering split in `04` |
| **C2 Orchestration Change** (staged architecture, unified Driver)                         | Identical **diagnostic sets** at every entry point + identical corpus behavior                | Strong    | `02`                                                  |
| **C3 Type-Representation Convergence**                                                    | Identical **diagnostic codes and messages** (order may be normalized)                         | Strong    | `03`                                                  |
| **C4 IR Shape Change** (SSA conversion)                                                   | **Behavioral equivalence** (same execution result) + IR structural invariants                 | Medium    | `04`                                                  |
| **C5 Frontend Paradigm Change** (grammar-driven)                                          | Identical AST snapshot + **identical diagnostic code+span item by item** + identical behavior | Strong    | `05`                                                  |
| **C6 Pure Deletion**                                                                      | No equivalence criterion; only confirmation that there are no references                      | —         | `06`                                                  |

**The distinction between C1 and C4 is critical**: C1 requires byte-for-byte identical IR (any
change is a regression), whereas C4 acknowledges that the IR will change and instead requires “the
program behaves identically + the IR satisfies its invariants.”

> **⚠️ C5′ explicitly does not exist**
>
> The diagnostic requirements in the error-recovery dimension are **not relaxed**: LALRPOP’s
> error-recovery model differs from `synchronize()`, which is precisely why the grammar in
> [05](05-frontend-paradigm.md) **must model the behavior of `synchronize()` using explicit error
> productions**, so that diagnostics remain identical item by item. Failure to do so is an
> implementation defect and **does not justify relaxing the criteria**.
>
> If stages 3a/3b empirically reveal that existing diagnostics cannot be reproduced under LALRPOP,
> the correct action is to **report that finding faithfully and reassess the design** (including the
> option of retaining Pratt), **not to modify the criteria**.

### Three-Layer Criteria

#### First Layer: Static IR Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is much
stronger than snapshots because it checks **semantic invariants** rather than a concrete shape.

| Invariant                                                                             | Defects caught                                                                                                             | Typical cause                                              |
| ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| **Use-before-def dominance**                                                          | Every `Operand::Local(n)` that is read must have a definition path in the CFG that dominates the current point             | Register misalignment, omitted restore                     |
| **Unique value definition** (only in SSA form)                                        | Every SSA value has exactly one definition point; a `Phi` is required at join points                                       | Incorrectness of the SSA conversion itself                 |
| **Phi consistency**                                                                   | The number of `Phi` inputs equals the number of corresponding predecessor basic blocks, and every input type is consistent | Missing predecessor, type mismatch                         |
| **Jump target exists**                                                                | Every jump label exists in `blocks`                                                                                        | Rebasing error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot bounds**                                                                | The `i` in `Operand::Global(i)` satisfies `i < globals.len()`                                                              | Slot-allocation error                                      |
| **Type consistency**                                                                  | The `MonoType`s of an instruction’s `dst` and `src` are compatible                                                         | Error bridging type representations                        |
| **Inner isolation** (corresponding to the explicit commitment at `ir_gen.rs:248-249`) | Function A’s `cur_locals` does not contain a temporary name from function B                                                | The 6 save/restore contracts                               |

**The verifier must exist before SSA conversion**, and it must **first run green against the current
(non-SSA) IR**—otherwise, it cannot be used to prove equivalence before and after the
transformation.

#### Second Layer: Normalized IR Snapshot (Covers C1/C3/C5)

IR snapshots must be **normalized**; otherwise, any change to a register number will cause the full
diff to fail:

| Normalization item      | Rule                                                                                                       |
| ----------------------- | ---------------------------------------------------------------------------------------------------------- |
| Strip `Span`            | Set all span fields to a sentinel value                                                                    |
| Rename temporary values | Rename by order of first appearance to `%0 %1 %2 ...`, **preserving the definition-use topological order** |
| Relativize global slots | Renumber by order of first appearance within the module                                                    |
| Order predecessors      | Sort `successors` and `Phi` inputs by label                                                                |
| Order struct fields     | Sort only when field names are known (construction order is semantic and must not be sorted)               |

**The normalization tool’s output must be checked in** (`src/middle/core/tests/snapshots/`),
explicitly updated whenever the IR definitions change, and its diff manually reviewed.

**Known limitation (must be documented)**: Normalization erases information about “which register
the Nth argument used,” so **the second layer alone is insufficient to cover Defect Class 2**.
During C1, the safety of `arg_regs` reordering must rely jointly on the first layer’s dominance
checks and the corpus behavioral differential.

#### Third Layer: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the refactoring, and compare:

| Comparison target      | Normalization rule                                                                                                                            |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is excluded from comparison** (type-representation convergence may change the wording) |
| Exit code              | Compare directly                                                                                                                              |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                         |
| `dump_bytecode` output | Compare only during the C1/C2 stages (bytecode changes with the IR)                                                                           |

**The C4 (SSA conversion) behavioral differential must cover**: the three ownership transfers
ref/borrow/move, closure capture, currying (`generate_curry_*`), spawn and iterator-based `for`,
sum-type pattern matching, existential forcing points, `?` / `Try` propagation, method overloading
(`overload_resolutions` span keying), and the Drop sequence for refinement constraints
(`ReleasePlan`).

**Performance baseline**: The corpus differential guarantees “identical behavior,” but not
“identical elapsed time.” While establishing the differential baseline in P2, use `criterion`, which
is already present in dev-dependencies, to create and check in 2–3 smoke benchmarks (whole-corpus
compilation time / interpretation throughput for typical programs / CLI cold startup) as performance
criteria for subsequent pipeline-related changes. Removing slot reuse, expanding `Phi` into chains
of `Move`, or introducing a Driver indirection layer can all change performance; without a baseline,
nothing would catch such regressions.

### Dedicated Criteria for Correctness Bug Fixes

The bug that [`02`](02-stage-contract.md) is to fix needs an **independent criterion** and must not
be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets of the single-file and multi-file paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written to fail (red)** to confirm that it catches the existing bug; only
then may the fix begin. If it unexpectedly passes, the vulnerability analysis must be reviewed
again.

Companion criteria (for [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                                                 | Defects blocked                                                                                                                                               |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_program_stage_coverage`        | Every `Program::stages()` is a subset of `Stage::ALL` and equals the expected array                                                                       | A future stage that is not wired in                                                                                                                           |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must always produce a diagnostic in every mode                                                                                    | The 3 `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                                                     |
| `test_release_plan_spans_consumed`   | The set of `PlanId`s produced by ownership is a subset of the set consumed during IR construction (**settled by structural identity, with no whitelist**) | `ReleasePlan` span keys silently dropping Drop instructions (historical form; after the key migration, the consumption ledger + `W1090` provide the backstop) |
| `test_overload_resolutions_consumed` | Corpus file by file: overload-resolution output is a subset of consumption, with an evidence floor of `examined ≥ 1` / actually consumed `≥ 2`            | Resolution entries being silently discarded (silent incorrect answers from falling back to the default binding)                                               |
| `test_obligations_drained`           | CI fails if any `Obligations` field has no consumption point anywhere in the repository                                                                   | The entire class of this defect                                                                                                                               |

### Semantic-Correction Routing Criteria (D59)

The RFC-039 refactoring contract consists of **tiered equivalence (C1–C6) + a dedicated
semantic-correction channel** (P3 / P3.5 + decision registry). The constraints “C5′ does not exist”
and “no relaxation whatsoever” govern **the strength of the criteria**, not the requirement to
preserve incorrect semantics verbatim. The routing boundary has only one **decidable entry
criterion**:

> **Question: Did the old behavior count as “successful”?** (Success = compilation succeeds and
> produces the correct result)

**Not successful → Classes A/B/C may be fixed along the way**, provided that all 4 of the following
are satisfied:

1. **Red before green**—the red criterion must exist first and must genuinely be red at the current
   revision ([HOWTO](../HOWTO.md) self-check item). “Green then red” does not count: without a red
   criterion, there is no way afterward to distinguish a “fix” from a “regression.”
2. **Do not change the externally visible behavior of programs that were previously successful**—a
   program that previously compiled and ran correctly must not become a failure because of this
   change, and its existing diagnostics (code / message / exit code) must not change. It is
   **permitted** to make a path that was previously unsuccessful more conspicuous (no diagnostic → a
   new W/E code or increased severity are both allowed); it is **not permitted** to change the
   externally visible behavior of a path that was previously successful.
3. **Register it in the same commit**—mark the corresponding WBS row with the error type (A/B/C) +
   **correct the affected body text in place** (the body describes only the current state; the
   before/after correction diff and red→green evidence are retained in the commit message/record) +
   the PR’s required “Error type” field.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshot / corpus
   differential). The classification is **not** grounds for exempting this review.

| Class                                                    | Criterion                                                                           | Typical form                                                                             | Precedent                               |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | --------------------------------------- |
| **A Silent Failure**                                     | An error that should have been reported is swallowed                                | Empty `Unproven` branch, silently missing solver, `unwrap_or_default` fallback           | 3.3 / 3.4.1 (`W1063`) / 3.4.2 (`W1081`) |
| **B Silent Mismatch**                                    | Key mismatch / choosing one of two parallel representations / span-key misalignment | Span key silently dropping Drop, two representations of synonymous concepts              | 3.4.5 / D20 / D57                       |
| **C Undefined Behavior and Uncontrolled Resource Usage** | Crash, stack overflow, uninitialized slot, bytecode version mismatch                | Pathological generic recursion (compile-time stack overflow), 3 hardcoded `.42` discards | 3.4.8 ①/② / D52                         |

**Previously successful behavior that now must change → Classes D/E/F still require prior user
arbitration** (see “When to Request User Arbitration (Mandatory)” in the repository-root
`AGENTS.md`):

| Class                          | Criterion                                                                                                                                        | Examples                  |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------- |
| **D User-Visible Semantics**   | The externally visible surface of diagnostic codes being added, removed, or changed; externally visible error text; module semantics; exit codes | D54 / D55 / RFC-029g      |
| **E External Contract Tables** | The error-code table, opcode table, type table, or entry-stage table itself                                                                      | D1 / D6 / D12 / D20 / D23 |
| **F RFC Design Change**        | The meta-type universe, new E codes, wholesale syntax removal                                                                                    | D53 / RFC-033 / RFC-029g  |

**Primary-error attribution**: A change is classified by its **primary error type**; incidental
behavior changes within the minimal fix set for the same root cause may simply be listed separately
on the PR’s “D3 patch determination” line (precedent: 3.4.8 ① has primary Class C, with 3 asymmetric
Class A defects included alongside it).

**Relationship to C1/C2**: A/B/C changes **must not** be declared as C1 (byte-for-byte identity) or
C2 (identical diagnostic sets)—those categories are defined as behavior-preserving. Changes that fix
such defects opportunistically should instead be declared as the corresponding category among
C3/C4/C6.

**Why no topic-based whitelist**: [08](08-maintenance-mechanism.md) already states that “the
exemption count itself enters the gate report,” and the list will inevitably expand; cases involving
one primary class plus incidental fixes will always fall on the boundary. The entry criterion is
Boolean-decidable.

### Regression Gates

Add the following CI checks (consistent with the existing conventions in `scripts/ci/`):

| Check                      | Trigger                                                                   | Failure condition                                                                               |
| -------------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | Changes in the C1/C3/C5 categories                                        | The snapshot files differ, but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns a non-empty result for the full corpus                                       |
| `check-corpus-parity.py`   | Changes in the C2/C4/C5 categories                                        | The corpus differential is non-empty                                                            |
| `check-stage-contract.py`  | All changes                                                               | `Program::stages()` does not match `Stage::ALL`                                                 |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | Performance baseline regression is >10% and the PR contains no explanation                      |

### Binding to the Companion Documents

| Document                                                                     | Must have first                                                             | Criterion strength             |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------ |
| [02-stage-contract.md](02-stage-contract.md) (staged architecture)           | This document’s vulnerability criteria (red first)                          | C2: identical diagnostic sets  |
| [03-type-unification.md](03-type-unification.md) (type representation)       | The first layer’s type-consistency invariant                                | C3: identical diagnostic codes |
| [04-ssa.md](04-ssa.md) (SSA conversion)                                      | **All three layers**, and the verifier must first run green in non-SSA form | C4: behavioral equivalence     |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (grammar-driven frontend) | AST snapshot + diagnostic criteria                                          | C5: all three layers           |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)           | Reference scan                                                              | None                           |

## Key Decisions and Rationale

**Tiering rather than a single criterion.** Using “full IR snapshot equality” as the criterion for
SSA conversion (C4) would fail outright, tempting the team to relax the criteria; tiering lets each
step use a strength it can support. Meanwhile, the first-layer verifier covers dominance checks,
turning “errors that cannot be tested” into “invariants that can be tested”—something snapshots
cannot do. The criteria assets are checked in and evolve with the code; they are not a one-time
investment.

### Rejected Approaches

- **End-to-end corpus differential only**—Defect Classes 1/2/3 are precisely the cases where “the
  program still runs to completion in some form,” to which end-to-end differentials are insensitive.
  SSA conversion is especially dangerous: a skipped register number may affect only performance and
  not the output, while a span mismatch that causes a Drop to disappear **manifests only for
  specific inputs**, which the corpus may not cover.
- **IR snapshot only**—snapshots are inherently blind to Defect Class 2 (see “Normalization”), and
  they fail completely during the C4 stage.
- **Property-based testing only** (`proptest` is already in the dependencies)—random programs have
  difficulty covering paths involving ownership, existentials, and refinement constraints that are
  triggered only by particular shapes. **However, it can serve as a supplement**: for Defect Class 1
  (simple expressions/arithmetic), it genuinely provides better coverage than a fixed corpus, and is
  recommended as a supplemental criterion for the C4 stage once the corpus differential is stable.
- **Refactor first, add tests later**—this project has already encountered that pitfall. The 8
  orphan test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1,005 lines /
  78 tests that have never run) are the result of creating directories first and then failing to
  wire them up. The criteria must precede the refactoring.

## Known Limitations and Risks

- **The first-layer verifier requires SSA lookahead information.** In non-SSA form, the concept of a
  unique definition point does not exist (`Operand::Local(n)` refers to a slot that may be written
  multiple times). The verifier therefore needs two modes: `verify_ssa` (strict, including Phi
  checks) and `verify_loose` (only dominance and type checks, permitting multiple definitions).
  **`verify_loose` must first run green against the existing IR**; this is the largest single
  implementation effort in the design.
- **The second layer’s normalization misses Defect Class 2.** This is a known limitation that can
  only be addressed jointly by the first and third layers; it cannot be solved by strengthening
  normalization.
- **The third layer’s corpus differential is sensitive to nondeterministic output.** If a `.yx`
  corpus file produces output that depends on hash iteration order or time, the differential will
  fail spuriously. Such corpus files must be identified when establishing the baseline.
- **Checking in snapshots creates a maintenance cost.** Whenever the IR definitions change, the
  snapshots must be explicitly updated, which requires review discipline.
- **The corpus covers only the single-file path.** There is no `yaoxiang.toml` under `tests/`, so
  all 293 corpus files take the single-file path—see the limitation discussion in “Existing Test
  Assets.”

> **All open questions originally listed in this section have now been adjudicated.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) for the item-by-item
> decisions (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — the higher-level master plan
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, this
  repository’s existing “per-item round-trip” criterion example
- `src/middle/core/tests/bytecode.rs:769-814` — a sentinel test that uses `include_str!` to read
  source code for static comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional set-difference assertions + a
  snapshot table, forcing an update whenever a new frontend is integrated
- `ir_gen.rs:248-249` — the explicit statement of the physical-adjacency save/restore contract
