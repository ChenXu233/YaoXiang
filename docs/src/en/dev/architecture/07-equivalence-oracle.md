# Refactoring Equivalence Criteria

> **Companion design document.** This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiering of acceptance criteria, and execution-phase order are defined in the
> main RFC-039 text; see the [directory index](index.md) for the purpose of each companion document.

## Purpose and Scope

This document defines the criteria for proving that behavior remains unchanged during refactoring.
The criteria are **tiered by refactoring-step category**, because different categories require
different strengths of equivalence: a pure relocation requires byte-for-byte identity, whereas SSA
conversion necessarily changes the shape of the IR and can only be judged behaviorally equivalent.

**This document is a prerequisite for all refactoring.** Without these criteria,
[SSA conversion](04-ssa.md) cannot be carried out safely.

This document **does not cover**: the stage contracts themselves (see `02`), the design for
converging type representations (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The most dangerous code is precisely the code with zero test coverage

`src/middle/core/ir_gen.rs` (8,448 lines):

- The file contains **neither `#[cfg(test)]` nor `mod tests`**
- Among the 29 tests in `src/middle/core/tests/`, `def_assign` and `local_slots` only positively pin
  down sentinel behavior for DefId allocation and slot naming; all four assertions in `bytecode.rs`
  that mention ir_gen are **negative assertions** (“ir_gen does not construct a certain
  instruction”)
- In other words, **IR structural invariants—dominance, unique definitions, jump targets, and type
  consistency—currently have no validation whatsoever**. This is precisely what the first layer of
  criteria is intended to address.

This file also happens to contain three classes of defects that **produce no error when they fail**:

**Defect Class 1: Register Misalignment.** The file contains six manually paired save/restore
statements separated by more than 70 lines:

| Save Point            | Restore Point | Enclosing Function          |
| --------------------- | ------------- | --------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`        |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`      |
| `2116`                | `2180`        | Curry intermediate function |
| `2224-2229`           | `2284-2288`   | Innermost curry function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`  |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`   |

The file itself states this contract at `ir_gen.rs:248-249`: “**The save/restore point for
`next_temp` must be physically adjacent; otherwise, inner-function names will bleed into the outer
function.**”

The consequence of missing one restore: `next_temp` leaks, and temporary-value numbering skips
values. **The IR remains internally consistent, still passes every check, and still compiles—only
the values are wrong.** Ordinary assertion tests cannot catch this.

**Defect Class 2: Misaligned Semantic Reordering of `arg_regs`.** The six branches inside
`generate_call_expr_ir` (`7269-8014`, 747 lines) share the same `Vec<Operand>` and rewrite it
semantically:

| Line        | Reordering Semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

Default-value registers are also appended along the way. **Misalignment produces no error; it only
produces wrong values.**

This class is even more dangerous because it **may also evade detection under a “canonicalized
snapshot.”** If the snapshot tool normalizes temporary register numbers—which is standard
practice—then an error such as “the third argument should have used the fifth register” is
**completely identical** in the snapshot.

**Defect Class 3 (Eliminated): Silent Failure of a Span-Keyed Cross-Layer Contract.** `ReleasePlan`
once used `Span` as its key (produced at `layers/ownership.rs:31`), passed it through
`checker.rs:1440`, and ultimately had it matched by `self.release_plan.get(&stmt.span)` in `ir_gen`,
which emitted `Instruction::Drop`. Any discrepancy in how either side computed spans would cause the
**Drop instruction to disappear silently, with no error**. `overload_resolutions` had the same
shape. **On 2026-10-13 (RFC-039 D20 / WBS 4.5)**, both contracts were changed to
**structural-identity keys** based on `PlanId` (the sole consumption entry points are
`ReleasePlan::consume` / `OverloadResolutions::get`; reading and accounting occur in the same
invocation; any nonempty difference produces a grade-E `W1090`). Defects in this category are now
reduced to zero—the implementation form and criteria are described in the revision note in
[02](02-stage-contract.md) §2.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> whose key is the module-unqualified `"Type.method"`). Its risk profile is different—a
> definition-order or duplicate-name mismatch rather than a span mismatch—and SSA conversion does
> not address it (see mechanism three in [04](04-ssa.md)). There are therefore **two** span-keyed
> contracts (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for these criteria is sufficient:

| Asset                               | Scale                                                                                                                                                              | Can Be Used For                                                                                                |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293 corpus files**, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All use a single-file path**—see the limitation below | End-to-end behavioral differential (**single-file side only**)                                                 |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                            | Standard-library behavioral differential                                                                       |
| `tests/integration/`                | 18 modules / 5,846 lines                                                                                                                                           | CLI end-to-end testing (of these, the 27 tests in `multifile.rs` provide the **only** multifile-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1,131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, round-tripping every opcode)                                 | Bytecode-layer round-tripping                                                                                  |

> **⚠️ Structural limitation of corpus coverage (added 2026-10-03)**
>
> There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (measured count: zero).
> Consequently, all 293 corpus files take the `standalone` branch of `check_files_with_diagnostics`
> at `util/diagnostic/mod.rs:565` → `check_single_file` → **`Pipeline::run` (the single-file
> path)**.
>
> **Consequence**: The third layer of criteria can **only validate changes to the single-file
> path**. There is currently **no corpus coverage** for the multifile side of
> [`02`](02-stage-contract.md) (`orchestrator`'s `check_project` / `compile_project` /
> `check_source_in_project` / `compile_embedded_module`) or for the multifile behavior of
> [`03`](03-type-unification.md) / [`04`](04-ssa.md).
>
> **A multifile corpus layer must be established in P2**; otherwise, the largest refactoring
> phase—the “unified Driver”—has no executable criteria. The 27 tests in
> `tests/integration/multifile.rs` are the only existing multifile-side coverage, but they test
> semantic results rather than compilation-stage contracts.

**However, these assets share one common flaw**: they test “input → output,” not whether the
intermediate representation is correct. The IR layer currently has **no criteria whatsoever**.

## Target Design

### Core Principle: Criteria Are Tiered by Refactoring Category

**This is the single most important principle in this document.** Using one criterion—usually “full
IR snapshot equality”—for every kind of refactoring is incorrect, because some refactorings
**necessarily change the shape of the IR**.

| Refactoring Category                                                                      | Criterion Type                                                                        | Strength  | Documents Involved                                    |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | --------- | ----------------------------------------------------- |
| **C1 Pure Relocation** (splitting files, reorganizing directories, extracting submodules) | Normalized IR snapshot **zero-diff**                                                  | Strongest | The checker split in `02`; the lowering split in `04` |
| **C2 Orchestration Change** (staging, unified Driver)                                     | Same **diagnostic set** at every entry point + same corpus behavior                   | Strong    | `02`                                                  |
| **C3 Type-Representation Convergence**                                                    | Same diagnostic codes and messages (**order may be normalized**)                      | Strong    | `03`                                                  |
| **C4 IR-Shape Change** (SSA conversion)                                                   | **Behavioral equivalence** (same execution results) + IR structural invariants        | Medium    | `04`                                                  |
| **C5 Frontend-Paradigm Change** (grammar-driven)                                          | Same AST snapshot + **identical diagnostic code+span for every item** + same behavior | Strong    | `05`                                                  |
| **C6 Pure Deletion**                                                                      | No equivalence criterion; only confirmation that no references remain                 | —         | `06`                                                  |

**The distinction between C1 and C4 is critical**: C1 requires byte-for-byte IR identity (any change
is a regression), whereas C4 acknowledges that the IR will change and instead requires “identical
program behavior + IRs that satisfy invariants.”

> **⚠️ C5′ explicitly does not exist**
>
> Diagnostic requirements in the error-recovery dimension are **not relaxed**: LALRPOP's
> error-recovery model differs from `synchronize()`, which is precisely why the grammar in
> [05](05-frontend-paradigm.md) **must model the behavior of `synchronize()` using explicit error
> productions**, so that diagnostics remain identical item by item. Failing to do so is an
> implementation defect and **does not justify weakening the criteria**.
>
> If phases 3a/3b empirically find that existing diagnostics cannot be reproduced under LALRPOP, the
> correct action is to **report the finding honestly and reassess the design** (including the option
> of “retaining Pratt”), **not to modify the criteria**.

### Three-Layer Criteria

#### First Layer: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is much
stronger than snapshots because it checks **semantic invariants**, not a specific shape.

| Invariant                                                                          | Defects Caught                                                                                                              | Typical Cause                                                 |
| ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **Use-before-def dominance**                                                       | Every read of `Operand::Local(n)` must have a definition path on the CFG that dominates the current point                   | Register misalignment, missed restore                         |
| **Uniqueness of value definitions** (in SSA form only)                             | Every SSA value must have exactly one definition point; joins must have a `Phi`                                             | Incorrectness introduced by SSA conversion itself             |
| **Phi consistency**                                                                | The number of `Phi` inputs must equal the number of corresponding predecessor basic blocks, and every input type must agree | Missing predecessor, type mismatch                            |
| **Jump target exists**                                                             | The label of every jump must exist in `blocks`                                                                              | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot out of bounds**                                                      | `Operand::Global(i)` must satisfy `i < globals.len()`                                                                       | Slot-allocation error                                         |
| **Type consistency**                                                               | The `MonoType` of an instruction's `dst` and `src` must be compatible                                                       | Bridging type-representation errors                           |
| **Inner isolation** (corresponding to the explicit promise at `ir_gen.rs:248-249`) | Function A's `cur_locals` must not contain a temporary name from function B                                                 | The six save/restore contracts                                |

**The verifier must exist before SSA conversion**, and it must first **pass green on the current
non-SSA IR**; otherwise, it cannot be used to prove equivalence before and after the transformation.

#### Second Layer: Normalized IR Snapshots (Covers C1/C3/C5)

IR snapshots must be **normalized**; otherwise, any change to register numbering would cause the
full diff to fail:

| Normalization Item         | Rule                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Strip `Span`               | Set all span fields to sentinel values. **This rule only became genuinely effective starting with WBS 6.6.12**: previously, the `type_decl` line printed the Debug text of the type-declaration carrier's `ast::Type`, and **100% of all span leakage measured in IR snapshots came from that line** (96 lines / 69 files). After the carrier was replaced with an IR-native narrowed shape, no spans appear anywhere in the snapshot |
| Temporary-value renaming   | Rename in order of first appearance to `%0 %1 %2 ...`, **preserving definition-use topological order**                                                                                                                                                                                                                                                                                                                                |
| Global-slot relativization | Renumber in order of first appearance within the module                                                                                                                                                                                                                                                                                                                                                                               |
| Predecessor ordering       | Sort `Phi` inputs by predecessor label (`successors` has not been included in snapshot rendering since 7b.3; after 7b.4, slots are reshaped by the allocator)                                                                                                                                                                                                                                                                         |
| Struct-field ordering      | Sort only when field names are known (construction order is semantic and must not be sorted)                                                                                                                                                                                                                                                                                                                                          |

**The normalization tool's output must be checked into the repository**
(`src/middle/ir/tests/snapshots/`). It must be explicitly updated whenever the IR definition
changes, and the diff must be manually reviewed during updates.

**Known limitation (must be stated explicitly)**: normalization erases information about “which
register the Nth argument used,” so **the second layer alone is insufficient to cover Defect Class
2**. During C1, the safety of `arg_regs` reordering must be guaranteed jointly by the first layer's
dominance checks and corpus behavioral differentials.

#### Third Layer: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the change, then compare:

| Comparison Target      | Normalization Rule                                                                                                                        |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is excluded from comparison** (type-representation convergence may change wording) |
| Exit code              | Compare directly                                                                                                                          |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                     |
| `dump_bytecode` output | Compare only during C1/C2 (the bytecode may change with the IR)                                                                           |

**The behavioral differential in the C4 (SSA conversion) phase must cover**: the three ownership
transfers ref/borrow/move, closure captures, currying (`generate_curry_*`), spawn and iterator for,
sum-type pattern matching, existential forcing points, `?` / Try propagation, method overloading
(`overload_resolutions` span-keyed), and the Drop sequence for refinement constraints (`ReleasePlan`
span-keyed).

**Performance baseline**: the corpus differential guarantees “same behavior,” not “same elapsed
time.” While establishing the differential baseline in P2, use the `criterion` crate already present
in dev-dependencies to build and check in 2–3 smoke benchmarks (full-corpus compilation time /
interpretation throughput for a typical program / CLI cold-start time) as the performance criteria
for later pipeline changes. Removing slot reuse, expanding a `Phi` into a chain of `Move`s, and
adding a Driver indirection layer can all affect elapsed time; without a baseline, nobody will catch
the regression.

### Dedicated Criteria for Correctness-Bug Fixes

The vulnerability that [`02`](02-stage-contract.md) is intended to fix requires an **independent
criterion** and must not be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets of the single-file and multifile paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written to fail** (red), confirming that it catches the existing
vulnerability before the fix begins. If it unexpectedly passes, the vulnerability analysis must be
revisited.

Similar criteria accompanying [`02`](02-stage-contract.md):

| Test                                 | Assertion                                                                                                                                                                    | Defects Caught                                                                                                                                           |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_program_stage_coverage`        | Every `Program::stages()` ⊆ `Stage::ALL` and equals the expected array                                                                                                       | A stage omitted in the future                                                                                                                            |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must always produce a diagnostic in every mode                                                                                                       | The three `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                                            |
| `test_release_plan_spans_consumed`   | The set of `PlanId`s produced by ownership must be a subset of the set consumed by IR construction (**settled by structural identity, with no allowlist**)                   | Silent loss of Drop through the `ReleasePlan` span key (historical form; after the key migration, the consumption ledger + `W1090` provide the backstop) |
| `test_overload_resolutions_consumed` | Corpus file by file: produced overload resolutions must be a subset of consumed resolutions, with a **minimum evidence surface** of `examined ≥ 1` / actually consumed `≥ 2` | Resolution entries being silently discarded (silently incorrect answers from fallback to the default binding)                                            |
| `test_obligations_drained`           | CI must fail if any `Obligations` field has no consumer anywhere in the repository                                                                                           | This entire class of defects                                                                                                                             |

### Routing Criteria for Semantic Corrections (D59)

RFC-039's refactoring contract consists of **tiered equivalence (C1–C6) + a dedicated
semantic-correction channel** (P3 / P3.5 + the decision registry). The constraints “C5′ does not
exist” and “no relaxation whatsoever” govern the **strength of the criteria**, not the requirement
to preserve erroneous semantics verbatim. The routing boundary has exactly one **decidable entry
criterion**:

> **Question: Did the old behavior count as “successful”?** (Success = compilation succeeds and
> produces the correct result)

**If it did not count as successful → Classes A/B/C may be fixed along the way**, and all four of
the following are mandatory:

1. **Red before green**—the red criterion must exist first and must currently be red (a self-check
   item in [HOWTO](../HOWTO.md)). “Green, then red” does not qualify: without a red criterion, there
   is no way afterward to distinguish a “fix” from a regression.
2. **Do not change the externally observable behavior of “previously successful” programs**—a
   program that originally compiled and ran correctly must not become a failure because of this
   change, and its existing diagnostics (code / wording / exit code) must not change. It is
   **permitted** to make a “previously unsuccessful” path more loud (no diagnostic → adding a W/E
   code or escalating the code are both allowed); it is **not permitted** to alter the externally
   observable behavior of a “previously successful” path.
3. **Register it in the same commit**—mark the corresponding WBS row with the error type (A/B/C) +
   **correct the affected body text in place** (the body text describes only the current state; the
   before/after correction diff and red→green evidence are retained in the commit message/record) +
   fill in the required PR block's “Error Type” line.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshot / corpus
   differential). Classification is **not** a reason to exempt that review.

| Class                                                 | Criterion                                                                             | Typical Form                                                                                     | Precedent                           |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------- |
| **A Silent Failure**                                  | An error that should have been reported is swallowed                                  | Empty `Unproven` branch, silent absence of a solver, `unwrap_or_default` fallback                | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent Mismatch**                                 | Key mismatch / choosing one of two parallel representations / span-keyed misalignment | Span key silently losing Drop, two representations of the same concept                           | 3.4.5 / D20 / D57                   |
| **C Undefined Behavior and Loss of Resource Control** | Crash, stack overflow, uninitialized slot, bytecode-version mismatch                  | Pathological generic recursion (compile-time stack overflow), three hard-coded discards in `.42` | 3.4.8 ①/② / D52                     |

**If it was previously successful but must now change → Classes D/E/F still require prior user
adjudication** (see “Mandatory User Adjudication” in the repository-root `AGENTS.md`):

| Class                          | Criterion                                                                                                                           | Examples                  |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-Visible Semantics**   | Externally visible aspect of diagnostic-code additions/removals/changes, externally visible error text, module semantics, exit code | D54 / D55 / RFC-029g      |
| **E External Contract Tables** | The error-code table, opcode table, type table, or entry-stage table itself                                                         | D1 / D6 / D12 / D20 / D23 |
| **F RFC Design Change**        | Metatype universe, new E codes, wholesale removal of syntax                                                                         | D53 / RFC-033 / RFC-029g  |

**Primary-error attribution**: classify a change by its **primary error type**; behavior changes
incidentally included in the minimal modification set for the same root cause need only be listed
separately on the PR's “D3 Patch Determination” line (precedent: 3.4.8 ① has primary error class C,
with three asymmetric class-A defects included).

**Relationship to C1/C2**: class A/B/C changes **must not** be declared as C1 (byte-for-byte
invariance) or C2 (identical diagnostic set), because the definitions of those classes are
behavioral invariance; opportunistic fixes must instead be declared as the corresponding category
among C3/C4/C6.

**Why not use a topical allowlist**: [08](08-maintenance-mechanism.md) already states that “the
exemption count itself enters the gate report”; the list will expand, and cases involving “one
primary error + incidental fixes” inevitably fall on the boundary. The entry criterion is
Boolean-decidable.

### Regression Gates

Add CI checks following the existing conventions in `scripts/ci/`:

| Check                      | Trigger                                                                   | Failure Condition                                                                                    |
| -------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | C1/C3/C5 changes                                                          | The snapshot files have a diff, but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns a nonempty result for the full corpus                                             |
| `check-corpus-parity.py`   | C2/C4/C5 changes                                                          | The corpus differential is nonempty                                                                  |
| `check-stage-contract.py`  | All changes                                                               | `Program::stages()` does not match `Stage::ALL`                                                      |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | Performance baseline regresses by >10%, and the PR contains no explanation                           |

### Dependencies on Companion Documents

| Document                                                                     | Must Exist First                                                                 | Criterion Strength             |
| ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ------------------------------ |
| [02-stage-contract.md](02-stage-contract.md) (staging)                       | This document's vulnerability criterion (red first)                              | C2: identical diagnostic set   |
| [03-type-unification.md](03-type-unification.md) (type representation)       | The first-layer verifier's type-consistency invariant                            | C3: identical diagnostic codes |
| [04-ssa.md](04-ssa.md) (SSA conversion)                                      | **All three layers**, and the verifier must first pass green on the non-SSA form | C4: behavioral equivalence     |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (grammar-driven frontend) | AST snapshots + diagnostic criteria                                              | C5: use all three              |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)           | Reference scan                                                                   | None                           |

## Key Decisions and Rationale

**Tiering rather than a single criterion.** Using “full IR snapshot equality” as the criterion for
SSA conversion (C4) would fail outright, tempting the team to weaken the criteria; tiering lets each
step use the strength it can bear. At the same time, the first-layer verifier covers dominance,
turning errors that cannot be tested into testable invariants—something snapshots cannot do. The
criteria assets are checked into the repository and evolve with the code; they are not a one-time
investment.

### Rejected Approaches

- **End-to-end corpus differential only**—Defect Classes 1/2/3 are precisely those in which “the
  program still finishes in some way,” so end-to-end differentials are insensitive to them. SSA
  conversion is particularly dangerous: skipped register numbers may affect only performance rather
  than output, while a Drop lost through span mismatch appears only under specific inputs and may
  not be covered by the corpus.
- **IR snapshots only**—snapshots are inherently blind to Defect Class 2 (see “Normalization”) and
  fail completely during C4.
- **Property-based testing only** (`proptest` is already a dependency)—random programs have
  difficulty covering ownership, existential, and refinement paths that require specific shapes to
  trigger them. **They can, however, serve as a supplement**: for the first class (simple
  expressions/arithmetic), they do provide better coverage than a fixed corpus. They should be
  introduced as a supplementary criterion for C4 after the corpus differential is stable.
- **Refactor first, add tests later**—this project has already fallen into this trap. The eight
  orphan test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1,005 lines /
  78 tests that never ran) are the product of creating directories before successfully wiring them
  up. The criteria must precede the refactoring.

## Known Limitations and Risks

- **The first-layer verifier requires SSA lookahead information.** In non-SSA form, the concept of a
  “unique definition point” does not exist (`Operand::Local(n)` points to a slot that may be written
  multiple times). The verifier therefore needs two modes: `verify_ssa` (strict, including Phi
  checks) and `verify_loose` (dominance and type checks only, allowing multiple definitions).
  **`verify_loose` must first pass green on the existing IR**; this is the largest single
  implementation workload in the design.
- **The second-layer normalization misses Defect Class 2.** This is a known limitation and can only
  be addressed jointly by the first and third layers; it cannot be solved by strengthening
  normalization.
- **The third-layer corpus differential is sensitive to nondeterministic output.** If output from a
  `.yx` corpus file depends on hash iteration order or time, the differential may fail spuriously.
  Such corpus files must be filtered out while establishing the baseline.
- **Checking snapshots into the repository creates maintenance overhead.** Whenever the IR
  definition changes, the snapshots must be explicitly updated, which requires review discipline.
- **The corpus covers only the single-file path.** There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus files use the single-file path—see the limitation described in “Existing Test
  Assets.”

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) for the individual
> decisions (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — overarching design
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, this
  repository's existing example of an “item-by-item round-trip” criterion
- `src/middle/core/tests/bytecode.rs:769-814` — sentinel test that reads source code using
  `include_str!` for static comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional difference-set assertions + snapshot
  table, requiring explicit updates when integrating a new frontend
- `ir_gen.rs:248-249` — explicit statement of the save/restore physical-adjacency contract
