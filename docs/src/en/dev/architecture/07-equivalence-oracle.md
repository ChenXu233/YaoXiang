# Refactoring Equivalence Criteria

> **Subsidiary design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution phase ordering are in the body of
> RFC-039; the role of each companion document is described in the
> [index for this directory](index.md).

## Position and Scope

This document defines the criteria for "proving behavior is unchanged" during the refactoring
process. Criteria are **graded by the category of refactoring step**—because different categories
require different strengths of equivalence: pure moves require byte-for-byte equality, while
SSA-ification necessarily changes IR form and can only judge behavioral equivalence.

**This document is a prerequisite for all refactoring.** Without these criteria,
[SSA-ification](04-ssa.md) cannot be safely implemented.

This document does **not cover**: the stage contract itself (see `02`), the type representation
convergence design (see `03`), or the dead code cleanup inventory (see `06`).

## Current State

### The Most Dangerous Code Today Happens to Have Zero Test Coverage

`src/middle/core/ir_gen.rs` (8448 lines):

- The file contains **no `#[cfg(test)]`, no `mod tests`**
- Among the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only forward-pin the
  sentinel behaviors of DefId allocation and slot naming; the 4 assertions in `bytecode.rs` that
  mention ir_gen are all **negative assertions** ("ir_gen does not construct a certain instruction")
- That is: **IR structural invariants (dominance, unique definition, jump targets, type consistency)
  currently have zero verification**—and this is precisely what the first-layer criteria in this
  document must add

This file, by coincidence, contains three classes of defects that **produce no error on failure**:

**Defect class 1: Register misalignment.** Six manually-paired save/restore statements in the file,
separated by more than 70 lines:

| Save point            | Restore point | Containing function        |
| --------------------- | ------------- | -------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`       |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`     |
| `2116`                | `2180`        | curry middle function      |
| `2224-2229`           | `2284-2288`   | curry innermost function   |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir` |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`  |

The file itself writes down this contract at `ir_gen.rs:248-249`: "**Must be physically adjacent to
the `next_temp` save/restore point**, otherwise the inner function's names will leak to the outer
function".

The consequence of one missed restore: `next_temp` leaks, temp values skip. **The IR is still
self-consistent, still passes all checks, still compiles—only values are wrong.** Ordinary assertion
tests cannot catch it.

**Defect class 2: `arg_regs` semantic reordering misalignment.** Inside `generate_call_expr_ir`
(`7269-8014`, 747 lines), 6 branches share the same `Vec<Operand>` and perform semantic rewrites:

| Line        | Rewrite semantics                         |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

And default-value registers are appended along the way. **Misalignment doesn't error, only values go
wrong.**

Even more dangerous: this class of error **may also not be caught** under "normalized snapshots"—if
the snapshot tool normalizes away temp register numbers (standard practice), then an error like "the
3rd argument should use the 5th register" appears **completely identical** in snapshots.

**Defect class 3: Span-keyed cross-layer contracts silently fail.** `ReleasePlan` is keyed by `Span`
(produced at `layers/ownership.rs:31`), passed through `checker.rs:1440`, and finally matched by
`self.release_plan.get(&stmt.span)` at `ir_gen.rs:1942` to emit `Instruction::Drop`. Any
inconsistency in how either side computes spans → **Drop instructions silently disappear, with no
error at all**. `overload_resolutions` is similarly span-keyed.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> key is the module-unqualified `"Type.method"`)—its risk nature is different (definition order /
> name-collision mismatch rather than span mismatch), and SSA-ification does not solve it
> (disposition in [04](04-ssa.md) mechanism three). Therefore there are **2** span-keyed contracts
> (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for the criteria is abundant:

| Asset                               | Size                                                                                                                                          | Applicable to                                                                                  |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** samples, organized by spec section (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All single-file paths**—see limitations below | End-to-end behavior diff (**single-file side only**)                                           |
| `src/std/tests/*.yx`                | A batch                                                                                                                                       | Standard library behavior diff                                                                 |
| `tests/integration/`                | 18 modules / 5846 lines                                                                                                                       | CLI end-to-end (among which `multifile.rs` 27 tests are the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, per-opcode roundtrip)                    | Bytecode layer roundtrip                                                                       |

> **⚠️ Structural limitations of sample coverage (added 2026-10-03)**
>
> There is **no `yaoxiang.toml`** under the `tests/` directory (verified count: 0). Therefore all
> 293 samples go through the `standalone` branch of `check_files_with_diagnostics` at
> `util/diagnostic/mod.rs:565` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence**: the third-layer criteria **can only verify single-file path changes**. For the
> multi-file side of [`02`](02-stage-contract.md) (`orchestrator`'s `check_project` /
> `compile_project` / `check_source_in_project` / `compile_embedded_module`) and the multi-file
> behavior of [`03`](03-type-unification.md) / [`04`](04-ssa.md), **there is currently no sample
> coverage**.
>
> **A multi-file sample layer must be built in P2**, otherwise the largest refactoring phase,
> "unified Driver", has no executable criteria. The 27 tests in `tests/integration/multifile.rs` are
> the only existing multi-file-side coverage, but they test semantic results rather than
> compile-stage contracts.

**But these assets have one common defect**: they test "input → output", not "whether intermediate
representations are correct". The IR layer's correctness currently **has no criteria at all**.

## Target Design

### Core Principle: Criteria Are Graded by Refactoring Category

**This is the single most important point in this document.** Using a single criterion (usually "IR
snapshot equality") to cover all refactoring types is wrong—because some refactorings **necessarily
change IR form**.

| Refactoring category                                                  | Criterion type                                                                            | Strength  | Related documents                           |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------- | ------------------------------------------- |
| **C1 Pure move** (file split, directory change, submodule extraction) | IR normalized snapshot **zero-diff**                                                      | Strongest | `02`'s checker split, `04`'s lowering split |
| **C2 Orchestration change** (staging, unified Driver)                 | Each entry's **diagnostic set is identical** + sample behavior identical                  | Strong    | `02`                                        |
| **C3 Type representation convergence**                                | Diagnostic code and message **identical** (order may be normalized)                       | Strong    | `03`                                        |
| **C4 IR form change** (SSA-ification)                                 | **Behavioral equivalence** (same execution result) + IR structural invariants             | Medium    | `04`                                        |
| **C5 Frontend paradigm change** (grammar-driven)                      | AST snapshot identical + **diagnostic code+span identical per item** + behavior identical | Strong    | `05`                                        |
| **C6 Pure deletion**                                                  | No equivalence criterion needed, only confirm no references                               | —         | `06`                                        |

**The difference between C1 and C4 is key**: C1 requires IR to be byte-for-byte identical (any
change is a regression); C4 admits the IR will change and instead uses "program behavior identical +
IR satisfies invariants".

> **⚠️ Clarification: C5′ does not exist**
>
> The diagnostic requirements for the error-recovery dimension are **not relaxed**: LALRPOP's
> error-recovery model differs from `synchronize()`, and precisely because of this, the grammar in
> [05](05-frontend-paradigm.md) **must use explicit error productions to model the behavior of
> `synchronize()`**, so that diagnostics are identical item-by-item. Failure to do so is an
> implementation defect, **not a reason to relax the criteria**.
>
> If phases 3a/3b empirically find that "under LALRPOP, the existing diagnostics cannot be
> reproduced", the correct action is **to honestly report that finding and re-evaluate the plan**
> (including the option of "maintain Pratt"), **not to modify the criteria**.

### Three-Layer Criteria

#### First Layer: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is much
stronger than snapshots, because it checks **semantic invariants** rather than specific form.

| Invariant                                                                          | What defect it catches                                                                                 | Typical cause                                                 |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| **use-before-def dominance**                                                       | Every `Operand::Local(n)` read must have a definition path on the CFG that dominates the current point | Register misalignment, missed restore                         |
| **Value definition uniqueness** (only under SSA form)                              | Each SSA value has exactly one definition point; join points must have `Phi`                           | SSA-ification's own correctness                               |
| **Phi consistency**                                                                | `Phi`'s input count == corresponding predecessor block count, and each input type is consistent        | Missed predecessor, type mismatch                             |
| **Jump target exists**                                                             | Every jump's label exists in `blocks`                                                                  | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot out-of-bounds**                                                      | `Operand::Global(i)`'s `i < globals.len()`                                                             | Slot allocation error                                         |
| **Type consistency**                                                               | The `MonoType` of an instruction's `dst` and `src` are compatible                                      | Bridging type representation error                            |
| **Inner isolation** (corresponding to the explicit promise at `ir_gen.rs:248-249`) | Function A's `cur_locals` does not contain function B's temp names                                     | 6 save/restore contracts                                      |

**The verifier must exist before SSA-ification**, and must run **green on the current (non-SSA)
IR**—otherwise it cannot be used to prove "equivalence before and after modification".

#### Second Layer: Normalized IR Snapshot (Covers C1/C3/C5)

IR snapshots must be **normalized**, otherwise any register number change will cause full diff
failure:

| Normalization item         | Rule                                                                                                |
| -------------------------- | --------------------------------------------------------------------------------------------------- |
| Strip `Span`               | All span fields set to sentinel value                                                               |
| Rename temp values         | Rename to `%0 %1 %2 ...` in first-occurrence order, **preserving definition-use topological order** |
| Global slot relativization | Renumber in order of first occurrence within the module                                             |
| Predecessor sorting        | `successors` and `Phi` inputs sorted by label                                                       |
| Struct field sorting       | Sort only when field names are known (construction order is semantic, do not sort)                  |

**The output of the normalization tool must be checked in** (`src/middle/core/tests/snapshots/`),
updated explicitly when IR definitions change, with manual review of diffs at update time.

**Known limitation (must be stated)**: normalization erases the information of "which register the
Nth argument used", so **the second layer alone is insufficient to cover defect class 2**. The
safety of `arg_regs` reordering in phase C1 must be guaranteed jointly by first-layer dominance
checks and sample behavior diff.

#### Third Layer: End-to-End Sample Diff (Covers C2/C4/C5)

Run once before and after modification on all **293** `.yx` files in `tests/yaoxiang/` +
`src/std/tests/*.yx`, and compare:

| Comparison target      | Normalization rule                                                                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text does not participate in comparison** (type representation convergence will change wording) |
| Exit code              | Direct comparison                                                                                                                                 |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                             |
| `dump_bytecode` output | Compare only in C1/C2 phases (bytecode changes with IR)                                                                                           |

**The behavior diff in phase C4 (SSA-ification) must cover**: ref/borrow/move three ownership
transfers, closure capture, currying (`generate_curry_*`), spawn and iterator for, sum-type pattern
matching, existential coercion points, `?` / Try propagation, method overload
(`overload_resolutions` span-keyed), refinement constraint Drop sequence (`ReleasePlan`).

**Performance baseline**: sample diff guarantees "behavior identical", not "elapsed time identical".
While P2 establishes the diff baseline, it also uses `criterion` (already in dev-dependencies) to
build 2-3 smoke benchmarks (full-sample compilation time / typical program interpretation throughput
/ CLI cold start) and check them in, as the performance criteria for subsequent pipeline-class
changes—deleting slot reuse, expanding Phi into Move sequences, Driver indirection layer can all
change elapsed time, and without a baseline nobody intercepts it.

### Dedicated Criteria for Fixing Correctness Vulnerabilities

The vulnerability to be fixed in [`02`](02-stage-contract.md) needs **independent criteria** that
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

**This test must be written first as failing** (red), confirming it captures the existing
vulnerability, before beginning the fix. If it is unexpectedly green, the vulnerability analysis
must be re-examined.

Companion criteria (paired with [`02`](02-stage-contract.md)):

| Test                               | Assertion                                                                     | Defect intercepted                                 |
| ---------------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------- |
| `test_program_stage_coverage`      | Every `Program::stages()` ⊆ `Stage::ALL` and equals the expected array        | Future missed stages                               |
| `test_no_silent_pass_on_unproven`  | `ProofResult::Unproven` must produce diagnostics in any mode                  | The extend branches at `checker.rs:5179/5318/5448` |
| `test_release_plan_spans_consumed` | The Span set produced by ownership ⊆ the Span set consumed by IR construction | `ReleasePlan` span key silently drops Drop         |
| `test_obligations_drained`         | CI fails when any `Obligations` field has no consumer site in the whole repo  | The entire class of this kind of defect            |

### Regression Gates

New CI checks (isomorphic to existing `scripts/ci/` conventions):

| Check                      | Trigger                                                           | Failure condition                                                        |
| -------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `check-snapshot-drift.sh`  | C1/C3/C5 class changes                                            | Snapshot file has diff and commit message lacks `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                    | `verify()` returns non-empty on full samples                             |
| `check-corpus-parity.py`   | C2/C4/C5 class changes                                            | Sample diff is non-empty                                                 |
| `check-stage-contract.py`  | All                                                               | `Program::stages()` does not match `Stage::ALL`                          |
| `check-perf-regression.sh` | Changes touching compile pipeline or execution path (P4/P6/P7/P8) | Performance baseline regression >10% and PR has no explanation           |

### Binding to Each Companion Document

| Document                                                                        | Must first have                                                             | Criterion strength             |
| ------------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------ |
| [02-stage-contract.md](02-stage-contract.md) (staging)                          | The vulnerability criteria of this document (red first)                     | C2: diagnostic sets identical  |
| [03-type-unification.md](03-type-unification.md) (type representation)          | First-layer verifier's type-consistency invariant                           | C3: diagnostic codes identical |
| [04-ssa.md](04-ssa.md) (SSA-ification)                                          | **All three layers**, and the verifier must run green on non-SSA form first | C4: behavioral equivalence     |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generator-ization) | AST snapshot + diagnostic criteria                                          | C5: all three used             |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)              | Reference scan                                                              | None                           |

## Key Decisions and Rationale

**Grading rather than a single criterion.** Using "IR snapshot equality" as the criterion for
SSA-ification (C4) will directly fail, thus tempting the team to relax the criterion; grading lets
each step use the strength it can bear. At the same time, the first-layer verifier covers dominance
checks, turning "errors that cannot be measured" into "invariants that can be measured"—this is
something snapshots cannot do. The criteria assets are checked in and evolve with the code, not a
one-time investment.

### Directions Not Adopted

- **Only use end-to-end sample diff**—defect classes 1/2/3 happen to be those where "the program
  still runs in some way", and end-to-end diff is insensitive to them. SSA-ification is especially
  dangerous: register skip may only affect performance without affecting output, and the Drop loss
  caused by span mismatch **only manifests with specific inputs**, which the sample library may not
  cover.
- **Only use IR snapshots**—snapshots are inherently immune to defect class 2 (see "Normalization"
  section), and they fail completely in phase C4.
- **Only use property-based testing** (proptest is already in dependencies)—random programs have
  difficulty covering ownership, existential, refinement, and other paths that require specific
  shapes to trigger. **But it can serve as a supplement**: for the first class (simple
  expressions/arithmetic), it does cover better than fixed samples; it is recommended to introduce
  it as a supplementary criterion for phase C4 after the sample diff stabilizes.
- **Refactor first, then add tests**—this is a pitfall the project has already stepped in. The 8
  orphan test trees (1005 lines / 78 tests that have never run) recorded in
  [`06-cleanup-inventory.md`](06-cleanup-inventory.md) are the product of "build directory first,
  wiring failed later". Criteria must precede refactoring.

## Known Limitations and Risks

- **The first-layer verifier needs SSA forward-looking information.** Under non-SSA form, the
  concept of "unique definition point" does not exist (`Operand::Local(n)` points to a slot that can
  be written multiple times). Therefore the verifier needs two modes: `verify_ssa` (strict, with Phi
  checks) and `verify_loose` (dominance and types only, allowing multiple definitions).
  **`verify_loose` must run green on the existing IR first**—this is the largest single workload in
  this design.
- **The second-layer normalization will miss defect class 2.** This is a known limitation and can
  only be covered jointly by the first and third layers, not by strengthening normalization.
- **The third-layer sample diff is sensitive to non-deterministic output.** If `.yx` samples contain
  output depending on hash iteration order / time, the diff will falsely fail. Such samples need to
  be filtered out when establishing the baseline.
- **Checking in snapshots brings maintenance cost.** When IR definitions change, snapshots must be
  updated explicitly and require review discipline.
- **Samples only cover the single-file path.** There is no `yaoxiang.toml` under `tests/`, and all
  293 samples go through the single-file path—see the limitations note in "Existing Test Assets".

> **The open questions originally listed in this section have all been adjudicated.** Decisions are
> item-by-item in the [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **No open items remain in this document.**

## See Also

- [RFC-039 Compiler Function Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — Upper-level master plan
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, the
  existing "per-item roundtrip" criteria example in this repository
- `src/middle/core/tests/bytecode.rs:769-814` — Sentinel tests that read source with `include_str!`
  for static comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — Bidirectional difference-set assertions + snapshot
  table, forcing new frontends to update when integrated
- `ir_gen.rs:248-249` — Explicit statement of the save/restore physical-adjacency contract
