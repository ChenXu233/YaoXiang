# Refactoring Equivalence Criteria

> **Subsidiary design document.** This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, the grading of acceptance criteria, and the ordering of execution phases are
> defined in the main text of RFC-039; for the positioning of each subsidiary document, see the
> [index of this directory](index.md).

## Positioning and Scope

This document defines the criteria for "proving behavior remains unchanged" during the refactoring
process. The criteria are **graded by refactoring step category**—because different categories
require different strengths of equivalence: pure relocation requires byte-for-byte equality, whereas
SSA-ification inevitably changes IR shape and can only be judged as behaviorally equivalent.

**This document is a precondition for the entire refactoring.** Without criteria,
[SSA-ification](04-ssa.md) cannot be safely implemented.

This document **does not cover**: the phase contracts themselves (see `02`), the design of type
representation convergence (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The most dangerous code today happens to be the code with zero test coverage

`src/middle/core/ir_gen.rs` (8,448 lines):

- The file contains **no `#[cfg(test)]` and no `mod tests`**
- Among the 29 tests in `src/middle/core/tests/`, `def_assign` and `local_slots` only positively pin
  the sentinel behaviors of DefId allocation and slot naming; the 4 assertions in `bytecode.rs` that
  mention ir_gen are all **negative assertions** ("ir_gen does not construct a certain instruction")
- That is: **IR structural invariants (dominance, unique definition, jump targets, type consistency)
  currently have zero verification**—and this is precisely what the first-layer criterion in this
  document aims to fill

Yet this file happens to contain three classes of defects that **produce no error when they fail**:

**Defect class 1: Register misalignment.** The file contains 6 pairs of manual save/restore
statements, separated by more than 70 lines:

| Save point            | Restore point | Function                    |
| --------------------- | ------------- | --------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`        |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`      |
| `2116`                | `2180`        | curry intermediate function |
| `2224-2229`           | `2284-2288`   | curry innermost function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`  |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`   |

The file itself states this contract at `ir_gen.rs:248-249`: "**Must be physically adjacent to the
save/restore point of `next_temp`**; otherwise the names from the inner function will leak into the
outer function."

The consequence of missing one restore: `next_temp` leaks, temporary values skip numbers. **The IR
remains self-consistent, still passes all checks, still compiles—it is only the values that are
wrong.** Ordinary assertion tests cannot catch this.

**Defect class 2: `arg_regs` semantic-rearrangement misalignment.** Inside `generate_call_expr_ir`
(`7269-8014`, 747 lines), 6 branches share the same `Vec<Operand>` and perform semantic rewrites:

| Line        | Rearrangement semantics                      |
| ----------- | -------------------------------------------- |
| `7395`      | Namespace call: named-argument rearrangement |
| `7724`      | Struct construction: field rearrangement     |
| `7835-7848` | Function call: named-argument rearrangement  |

And in the middle, default-value registers are additionally appended. **Misalignment produces no
error—only wrong values.**

What is more dangerous: this class of errors **may also not be detectable** under a "normalized
snapshot"—if the snapshot tool normalizes away temporary register numbers (which is a common
practice), then "the 3rd argument should have used the 5th register" looks **completely identical**
in the snapshot.

**Defect class 3: span-keyed cross-layer contracts silently fail.** `ReleasePlan` uses `Span` as a
key (produced by `layers/ownership.rs:31`), passed through `checker.rs:1440`, and finally matched
and emitted as `Instruction::Drop` by `self.release_plan.get(&stmt.span)` at `ir_gen.rs:1942`. Any
inconsistency in the way the two sides compute span → **Drop instructions silently disappear, with
no error at all**. `overload_resolutions` is likewise span-keyed.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> with keys being the unmodule-qualified `"Type.method"`)—its risk profile is different
> (definition-order/naming conflicts rather than span mismatches), and SSA-ification does not solve
> it (handling is in [04](04-ssa.md), mechanism 3). Therefore the span-keyed contracts number **2**
> (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for criteria is sufficient:

| Asset                               | Scale                                                                                                                                                             | Usable for                                                                                         |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpus files, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see the limitation below | End-to-end behavior diff (**single-file side only**)                                               |
| `src/std/tests/*.yx`                | a batch                                                                                                                                                           | Standard-library behavior diff                                                                     |
| `tests/integration/`                | 18 modules / 5,846 lines                                                                                                                                          | CLI end-to-end (of which the 27 tests in `multifile.rs` are the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1,131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, per-opcode roundtrip)                                       | Bytecode-layer roundtrip                                                                           |

> **⚠️ Structural limitation in corpus coverage (added 2026-10-03)**
>
> There is **no `yaoxiang.toml` at all** under the `tests/` directory (verified 0). Therefore all
> 293 corpus files go through the `standalone` branch of `check_files_with_diagnostics` at
> `util/diagnostic/mod.rs:565` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence**: the third-layer criterion **can only verify changes on the single-file path**.
> For the multi-file side of [`02`](02-stage-contract.md) (`orchestrator`'s `check_project` /
> `compile_project` / `check_source_in_project` / `compile_embedded_module`) and the multi-file
> behavior of [`03`](03-type-unification.md) / [`04`](04-ssa.md), **there is currently no corpus
> coverage**.
>
> **A multi-file corpus layer must be built in P2**; otherwise the largest refactoring phase,
> "Unified Driver," has no executable criterion. The 27 tests in `tests/integration/multifile.rs`
> are the only existing multi-file-side coverage, but they test semantic results, not
> compilation-phase contracts.

**But these assets share a common defect**: they test "input → output," not "whether the
intermediate representation is correct." Correctness at the IR layer currently has **no criteria
whatsoever**.

## Target Design

### Core principle: criteria graded by refactoring category

**This is the single most important point in this document.** Using a single criterion (typically
"IR snapshot equality") to cover all refactoring types is wrong—because some refactorings
**inevitably change IR shape**.

| Refactoring category                                                                  | Criterion type                                                                 | Strength  | Related documents                                     |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | --------- | ----------------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | IR normalized snapshot **zero-diff**                                           | Strongest | checker splitting in `02`, lowering splitting in `04` |
| **C2 Orchestration change** (phased, unified driver)                                  | **Same diagnostic set** at each entry + same corpus behavior                   | Strong    | `02`                                                  |
| **C3 Type representation convergence**                                                | Same diagnostic **codes and messages** (order may be normalized)               | Strong    | `03`                                                  |
| **C4 IR shape change** (SSA-ification)                                                | **Behavioral equivalence** (same execution result) + IR structural invariants  | Medium    | `04`                                                  |
| **C5 Frontend paradigm change** (grammar-driven)                                      | Same AST snapshot + **same diagnostic code+span item-by-item** + same behavior | Strong    | `05`                                                  |
| **C6 Pure deletion**                                                                  | No equivalence criterion needed; just confirm no references                    | —         | `06`                                                  |

**The distinction between C1 and C4 is the key**: C1 requires IR to be byte-for-byte identical (any
change is a regression); C4 acknowledges the IR will change and uses "same program behavior + IR
satisfies invariants" instead.

> **⚠️ There is explicitly no C5′**
>
> Diagnostic requirements at the error-recovery dimension are **not relaxed**: the LALRPOP
> error-recovery model differs from `synchronize()`; precisely for that reason, the grammar in
> [05](05-frontend-paradigm.md) **must use explicit error productions to model the behavior of
> `synchronize()`**, so that diagnostics are item-by-item consistent. Failing to do so is an
> implementation defect, **not grounds for relaxing the criteria**.
>
> If phase 3a/3b empirically finds that "under LALRPOP, the existing diagnostics cannot be
> reproduced," the correct action is to **truthfully report that finding and re-evaluate the plan**
> (including the option of "staying with Pratt"), **not to modify the criteria**.

### Three Layers of Criteria

#### First layer: IR static verifier (covers all categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is much
stronger than snapshots because it checks **semantic invariants** rather than concrete shape.

| Invariant                                                                                      | What defect it catches                                                                                                       | Typical cause                                                 |
| ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **Use-before-def dominance**                                                                   | Every read of `Operand::Local(n)` must have a definition path on the CFG that dominates the current point                    | Register misalignment, missed restore                         |
| **Unique value definition** (only under SSA shape)                                             | Every SSA value has exactly one definition point; at merge points there must be a `Phi`                                      | Correctness of SSA-ification itself                           |
| **Phi consistency**                                                                            | The number of `Phi` inputs equals the number of corresponding predecessor basic blocks, and each input has a consistent type | Missing predecessor, type mismatch                            |
| **Jump target existence**                                                                      | Every jump's label exists in `blocks`                                                                                        | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot out-of-bounds**                                                                  | `Operand::Global(i)` has `i < globals.len()`                                                                                 | Slot-allocation error                                         |
| **Type consistency**                                                                           | The `MonoType` of an instruction's `dst` is compatible with that of its `src`                                                | Bridging type-representation error                            |
| **Inner-function isolation** (corresponding to the explicit commitment at `ir_gen.rs:248-249`) | Function A's `cur_locals` does not contain function B's temporary names                                                      | 6 save/restore contracts                                      |

**The verifier must exist before SSA-ification**, and it must **first run green on the current
(non-SSA) IR**—otherwise it cannot be used to prove "equivalence before and after the change."

#### Second layer: normalized IR snapshot (covers C1/C3/C5)

IR snapshots must be **normalized**; otherwise any change in register numbering will cause a full
diff to fail:

| Normalization item      | Rule                                                                                                     |
| ----------------------- | -------------------------------------------------------------------------------------------------------- |
| Strip `Span`            | All span fields set to a sentinel value                                                                  |
| Rename temporaries      | Renamed in first-occurrence order to `%0 %1 %2 ...`, **preserving the definition-use topological order** |
| Relativize global slots | Renumber in module-local first-occurrence order                                                          |
| Predecessor sort        | `successors` and `Phi` inputs sorted by label                                                            |
| Struct field sort       | Sorted only when field names are known (construction order is semantic, not sorted)                      |

**The output of the normalization tool must be checked in** (`src/middle/core/tests/snapshots/`),
updated explicitly when the IR definition changes, and reviewed manually at update time.

**Known limitation (must be stated explicitly)**: normalization erases the information "which
register did the Nth argument use," so **the second layer alone is insufficient to cover defect
class 2**. Safety of `arg_regs` rearrangement during C1 must be ensured jointly by the first layer's
dominance check and the corpus behavior diff.

#### Third layer: end-to-end corpus diff (covers C2/C4/C5)

Run each of the **293** `.yx` files under `tests/yaoxiang/` + `src/std/tests/*.yx` once before and
once after the change, and compare:

| Comparison object      | Normalization rule                                                                                                                             |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sorted by `(code, span.file, span.line)`; **message text is not part of the comparison** (type-representation convergence will change wording) |
| Exit code              | Direct comparison                                                                                                                              |
| Runtime stdout/stderr  | Comparison after sorting                                                                                                                       |
| `dump_bytecode` output | Compared only in C1/C2 phases (bytecode changes with the IR)                                                                                   |

**The behavior diff in C4 (SSA-ification) must cover**: ref/borrow/move ownership transfers, closure
capture, currying (`generate_curry_*`), spawn and iterator for, sum-type pattern matching,
existential forcing points, `?` / Try propagation, method overloading (`overload_resolutions`
span-keyed), and the Drop sequence for refinement constraints (`ReleasePlan`).

**Performance baseline**: corpus diff guarantees "same behavior," not "same elapsed time." When P2
establishes the diff baseline, use the `criterion` already in dev-dependencies to build 2-3 smoke
benchmarks (full-corpus compile time / typical-program interpreter throughput / CLI cold start) and
check them in, as the performance criterion for subsequent pipeline-class changes—deleting slot
reuse, expanding Phi into Move chains, and Driver indirection layers can all change elapsed time,
and without a baseline nobody intercepts this.

### Dedicated criteria for fixing the correctness bug

The bug that [`02`](02-stage-contract.md) is going to fix requires an **independent criterion** and
cannot be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets of the single-file and multi-file paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written failing (red)**, confirming that it can catch the existing bug,
before the fix is started. If it is unexpectedly green, the bug analysis needs to be re-examined.

Criteria of the same kind (paired with [`02`](02-stage-contract.md)):

| Test                               | Assertion                                                                         | Defect intercepted                                   |
| ---------------------------------- | --------------------------------------------------------------------------------- | ---------------------------------------------------- |
| `test_program_stage_coverage`      | Every `Program::stages()` ⊆ `Stage::ALL` and equals the expected array            | Future missed stage wiring                           |
| `test_no_silent_pass_on_unproven`  | `ProofResult::Unproven` must produce a diagnostic in any mode                     | The `extend` branches at `checker.rs:5179/5318/5448` |
| `test_release_plan_spans_consumed` | The Span set produced by ownership ⊆ the Span set consumed during IR construction | Silent Drop loss due to `ReleasePlan` span key       |
| `test_obligations_drained`         | CI fails if any `Obligations` field has no consumer in the full repo              | The entire class of such defects                     |

### Regression Gates

New CI checks (isomorphic with existing `scripts/ci/` conventions):

| Check                      | Trigger                                                                   | Failure condition                                                                           |
| -------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | C1/C3/C5 class changes                                                    | Snapshot file has diff but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns non-empty for the full corpus                                            |
| `check-corpus-parity.py`   | C2/C4/C5 class changes                                                    | Corpus diff is non-empty                                                                    |
| `check-stage-contract.py`  | All                                                                       | `Program::stages()` does not match `Stage::ALL`                                             |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | Performance baseline regression >10% and the PR has no explanation                          |

### Bindings with each subsidiary document

| Document                                                                        | Must first have                                                                  | Criterion strength         |
| ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------- |
| [02-stage-contract.md](02-stage-contract.md) (phased)                           | The bug criterion in this document (first red)                                   | C2: same diagnostic set    |
| [03-type-unification.md](03-type-unification.md) (type representation)          | The type-consistency invariant of the first-layer verifier                       | C3: same diagnostic codes  |
| [04-ssa.md](04-ssa.md) (SSA-ification)                                          | **All three layers**, and the verifier must first run green on the non-SSA shape | C4: behavioral equivalence |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generator-ization) | AST snapshot + diagnostic criterion                                              | C5: all three used         |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)              | Reference scan                                                                   | None                       |

## Key Decisions and Rationale

**Graded, not single criterion.** Using "IR snapshot equality" as the criterion for SSA-ification
(C4) will fail directly, which would induce the team to relax the criterion; grading lets each step
use the strength it can bear. At the same time, the first-layer verifier covers dominance checks,
turning "errors that cannot be tested" into "invariants that can be tested"—something a snapshot
cannot do. Criterion assets are checked in and evolve with the code; this is not a one-time
investment.

### Approaches Not Adopted

- **Using only end-to-end corpus diff**—defect classes 1/2/3 are precisely the class where "the
  program still finished running in some way," and end-to-end diff is insensitive to them.
  SSA-ification is especially dangerous: register-number skipping may only affect performance and
  not output, while Drop loss caused by span mismatch **only manifests for specific inputs**, which
  the corpus may not cover.
- **Using only IR snapshots**—snapshots are inherently immune to defect class 2 (see the
  "Normalization" section), and they completely fail in phase C4.
- **Using only property-based testing** (proptest is already in dependencies)—random programs
  struggle to cover ownership, existential, and refinement paths that require a specific shape to
  trigger. **But it can serve as a supplement**: for the first class (simple expressions/arithmetic)
  it is indeed better than a fixed corpus; it is recommended to introduce it as a supplementary
  criterion for phase C4 once the corpus diff is stable.
- **Refactoring first, then writing tests**—this is a pitfall this project has already stepped in.
  The 8 orphan test trees (1,005 lines / 78 tests that have never run) recorded in
  [`06-cleanup-inventory.md`](06-cleanup-inventory.md) are the product of "build the directory
  first, fail to wire it later." Criteria must precede refactoring.

## Known Limitations and Risks

- **The first-layer verifier needs SSA forward-looking information.** Under the non-SSA shape, the
  concept of "unique definition point" does not exist (`Operand::Local(n)` points to a slot that can
  be written multiple times). Therefore the verifier needs two modes: `verify_ssa` (strict,
  including Phi checks) and `verify_loose` (dominance and types only, allowing multiple
  definitions). **`verify_loose` must first run green on the existing IR**—this is the largest
  single workload in this design.
- **The second-layer normalization will miss defect class 2.** This is a known limitation; it can
  only be covered jointly by the first layer + the third layer, and cannot be resolved by
  strengthening normalization.
- **The third-layer corpus diff is sensitive to non-deterministic output.** If any `.yx` corpus file
  has output that depends on hash iteration order / time, the diff will falsely fail. Such corpus
  files need to be filtered out when the baseline is established.
- **Checking in snapshots brings maintenance cost.** Snapshots must be explicitly updated when the
  IR definition changes, which requires review discipline.
- **The corpus only covers the single-file path.** There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus files go through the single-file path—see the limitation note in "Existing Test
  Assets" for details.

> **The open questions originally listed in this section have all been adjudicated.** Decisions on
> each item are recorded in [RFC-039 Decision Log](../../rfc/draft/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Feature Routing Directory Design](../../rfc/draft/039-compiler-architecture.md)
  — upper-level master plan
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, the
  existing "per-item roundtrip" criterion exemplar in this repo
- `src/middle/core/tests/bytecode.rs:769-814` — sentinel test using `include_str!` to read source
  code for static comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional set-difference assertion + snapshot
  table, forcing updates when a new frontend is wired in
- `ir_gen.rs:248-249` — explicit statement of the save/restore physical-adjacency contract
