# Refactoring Equivalence Criteria

> **Subsidiary Design Document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, tiered acceptance criteria, and execution phase ordering are defined in the
> RFC-039 main text; the positioning of each subsidiary document is given in the
> [directory index](index.md).

## Positioning and Scope

This document defines the criteria for "how to prove behavior is unchanged" during the refactoring.
The criteria are **tiered by refactoring step category**—because different categories demand
different strengths of equivalence: pure moves require byte-for-byte identity, while SSA-ization
necessarily changes IR shape and can only be judged by behavioral equivalence.

**This document is a prerequisite for all refactoring.** Without these criteria,
[SSA-ization](04-ssa.md) cannot be safely executed.

This document **does not cover**: stage contracts themselves (see `02`), type representation
convergence design (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The Most Dangerous Code Today Happens to Be Zero-Coverage Code

`src/middle/core/ir_gen.rs` (8448 lines):

- The file contains **no `#[cfg(test)]`, no `mod tests`**
- Among the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only forward-pin the
  sentinel behaviors of DefId allocation and slot naming; the 4 assertions in `bytecode.rs` that
  mention ir_gen are all **reverse assertions** ("ir_gen does not construct a given instruction")
- In other words: **IR structural invariants (dominance, unique definitions, jump targets, type
  consistency) currently have zero validation**—exactly what the first layer of criteria in this
  document must add

And this file happens to contain three classes of defects that **produce no error at all on
failure**:

**Defect Class 1: Register Misalignment.** The file contains 6 hand-written save/restore pairs,
separated by over 70 lines:

| Save Point            | Restore Point | Function                    |
| --------------------- | ------------- | --------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`        |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`      |
| `2116`                | `2180`        | curry intermediate function |
| `2224-2229`           | `2284-2288`   | curry innermost function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`  |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`   |

The file itself states this contract at `ir_gen.rs:248-249`: "**Must be physically adjacent to the
`next_temp` save/restore point, otherwise an inner function's name will bleed into the outer one**".

The consequence of a missed restore: `next_temp` leaks, temporary values skip numbers. **The IR
remains self-consistent, passes all checks, and still compiles—just with wrong values.** Ordinary
assertion tests cannot catch this.

**Defect Class 2: `arg_regs` Semantic Reordering Misalignment.** Inside `generate_call_expr_ir`
(`7269-8014`, 747 lines), 6 branches share the same `Vec<Operand>` and perform semantic rewriting:

| Line        | Reordering Semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

And it also appends default-value registers in the middle. **Misalignment produces no error, only
wrong values.**

What is more dangerous: this class of error **may also go undetected under "normalized
snapshots"**—if the snapshot tool normalizes away temporary register numbers (a common practice),
then "the 3rd argument should have used the 5th register" becomes **completely identical** in the
snapshot.

**Defect Class 3: Span-Keyed Cross-Layer Contract Silently Failing.** `ReleasePlan` uses `Span` as
the key (produced at `layers/ownership.rs:31`), is passed through `checker.rs:1440`, and is finally
matched and emitted as `Instruction::Drop` by `self.release_plan.get(&stmt.span)` at
`ir_gen.rs:1942`. Any inconsistency in how the two sides compute spans → **Drop instructions
silently disappear, with no error at all**. `overload_resolutions` is similarly span-keyed.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> with the key being the unqualified `"Type.method"`)—its risk nature is different (definition-order
> / name-collision mismatch rather than span mismatch), and SSA-ization does not solve it (see
> mechanism 3 in [04](04-ssa.md) for disposition). Therefore, the span-keyed contracts number **2**
> (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for criteria is abundant:

| Asset                               | Scale                                                                                                                                                    | Usable For                                                                                      |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** samples, organized by spec section number (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see limitations below | End-to-end behavior diffing (**single-file side only**)                                         |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                  | Standard library behavior diffing                                                               |
| `tests/integration/`                | 18 modules / 5846 lines                                                                                                                                  | CLI end-to-end (of which `multifile.rs` with 27 tests is the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, per-opcode round-trip)                              | Bytecode layer round-trip                                                                       |

> **⚠️ Structural Coverage Limitation in Samples (Added 2026-10-03)**
>
> There are **no `yaoxiang.toml`** anywhere under the `tests/` directory (verified: 0). Therefore,
> all 293 samples go through the `standalone` branch of `check_files_with_diagnostics` at
> `util/diagnostic/mod.rs:565` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence**: the third layer of criteria **can only validate changes to the single-file
> path**. For the multi-file side of [`02`](02-stage-contract.md) (`orchestrator`'s `check_project`
> / `compile_project` / `check_source_in_project` / `compile_embedded_module`) and the multi-file
> behavior of [`03`](03-type-unification.md) / [`04`](04-ssa.md), **there is currently no sample
> coverage at all**.
>
> **A multi-file sample layer must be built in P2**, otherwise the largest refactoring
> phase—"unified Driver"—has no executable criteria. The 27 tests in
> `tests/integration/multifile.rs` are the only existing multi-file-side coverage, but they test
> semantic results rather than compilation-stage contracts.

**But these assets share one common shortcoming**: they test "input → output", not "is the
intermediate representation correct". The IR layer's correctness currently has **no criteria at
all**.

## Target Design

### Core Principle: Criteria Are Tiered by Refactoring Category

**This is the most important point in this document.** Using a single criterion (usually "IR
snapshot equality") to cover all refactoring types is wrong—because some refactorings **necessarily
change IR shape**.

| Refactoring Category                                                       | Criterion Type                                                                             | Strength  | Involved Documents                          |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | --------- | ------------------------------------------- |
| **C1 Pure Move** (file splitting, directory changes, submodule extraction) | IR normalized snapshot **zero-diff**                                                       | Strongest | `02`'s checker split, `04`'s lowering split |
| **C2 Orchestration Change** (staging, unified Driver)                      | Each entry point's **diagnostic set** identical + sample behavior identical                | Strong    | `02`                                        |
| **C3 Type Representation Convergence**                                     | Diagnostic codes and messages **identical** (order may be normalized)                      | Strong    | `03`                                        |
| **C4 IR Shape Change** (SSA-ization)                                       | **Behavioral equivalence** (execution result identical) + IR structural invariants         | Medium    | `04`                                        |
| **C5 Frontend Paradigm Change** (grammar-driven)                           | AST snapshot identical + **diagnostic code+span per-entry identical** + behavior identical | Strong    | `05`                                        |
| **C6 Pure Deletion**                                                       | No equivalence criterion needed, just confirm no references                                | —         | `06`                                        |

**The distinction between C1 and C4 is the key one**: C1 requires IR to be byte-for-byte identical
(any change is a regression); C4 acknowledges that the IR will change, and instead uses "program
behavior identical + IR satisfies invariants".

> **⚠️ C5′ Explicitly Does Not Exist**
>
> The diagnostic requirements for the error-recovery dimension are **not relaxed**: LALRPOP's
> error-recovery model differs from `synchronize()`, which is precisely why the grammar in
> [05](05-frontend-paradigm.md) **must use explicit error productions to model the behavior of
> `synchronize()`**, so that diagnostics are per-entry consistent. Failing to do so is an
> implementation defect, **not grounds for relaxing the criteria**.
>
> If stages 3a/3b empirically find that "the existing diagnostics cannot be reproduced under
> LALRPOP", the correct action is to **report the finding honestly and re-evaluate the approach**
> (including the option "maintain Pratt"), **not to modify the criteria**.

### Three Layers of Criteria

#### First Layer: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is far
stronger than snapshots, because it checks **semantic invariants** rather than concrete shape.

| Invariant                                                                             | What Defect It Catches                                                                                                  | Typical Cause                                                  |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **use-before-def dominance**                                                          | Every read of `Operand::Local(n)` must have a definition path on the CFG that dominates the current point               | Register misalignment, missed restore                          |
| **Unique value definition** (SSA shape only)                                          | Every SSA value has exactly one definition point; join points must have a `Phi`                                         | Correctness of SSA-ization itself                              |
| **Phi consistency**                                                                   | The number of `Phi` inputs == the number of corresponding predecessor basic blocks, and each input's type is consistent | Missing predecessor, type mismatch                             |
| **Jump target exists**                                                                | Every jump's label exists in `blocks`                                                                                   | Translation errors in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot out-of-bounds**                                                         | `Operand::Global(i)`'s `i < globals.len()`                                                                              | Slot allocation error                                          |
| **Type consistency**                                                                  | The instruction's `dst` and `src` `MonoType`s are compatible                                                            | Bridging type representation error                             |
| **Inner isolation** (corresponding to the explicit commitment at `ir_gen.rs:248-249`) | Function A's `cur_locals` does not contain function B's temporary names                                                 | The 6 save/restore contracts                                   |

**The verifier must exist before SSA-ization**, and must run green on the **current (non-SSA) IR
first**—otherwise it cannot be used to prove "pre- and post-change equivalence".

#### Second Layer: Normalized IR Snapshots (Covers C1/C3/C5)

IR snapshots must be **normalized**, otherwise any register number change will cause full-diff
failure:

| Normalization Item      | Rule                                                                                            |
| ----------------------- | ----------------------------------------------------------------------------------------------- |
| Strip `Span`            | All span fields set to sentinel values                                                          |
| Rename temporaries      | Rename in first-occurrence order to `%0 %1 %2 ...`, **preserving define-use topological order** |
| Relativize global slots | Renumber by first-occurrence order within the module                                            |
| Sort predecessors       | `successors` and `Phi` inputs sorted by label                                                   |
| Sort struct fields      | Only sort when field names are known (construction order is semantic, not sorted)               |

**The output of the normalization tool must be checked in** (`src/middle/core/tests/snapshots/`),
and updated explicitly when the IR definition changes, with manual review of the diff on update.

**Known Limitation (Must Be Stated)**: Normalization erases the information about "which register
the Nth argument used", so **the second layer alone is insufficient to cover defect class 2**. The
safety of `arg_regs` reordering in the C1 phase must be jointly guaranteed by the first layer's
dominance check and sample-behavior diffing.

#### Third Layer: End-to-End Sample Diffing (Covers C2/C4/C5)

For all **293** `.yx` files in `tests/yaoxiang/` plus `src/std/tests/*.yx`, run once before and once
after the change, and compare:

| Comparison Object      | Normalization Rule                                                                                                                                    |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text does not participate in the comparison** (type representation convergence will change wording) |
| Exit code              | Direct comparison                                                                                                                                     |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                                 |
| `dump_bytecode` output | Only compare in C1/C2 phases (bytecode will change with the IR)                                                                                       |

**The behavior diffing in the C4 (SSA-ization) phase must cover**: ref/borrow/move three ownership
transfers, closure capture, currying (`generate_curry_*`), spawn and iterator for, sum-type pattern
matching, existential coercion points, `?` / Try propagation, method overloading
(`overload_resolutions` span-keyed), refinement constraint Drop sequences (`ReleasePlan`).

**Performance Baseline**: Sample diffing guarantees "behavior identical", not "time identical".
While P2 is establishing the diff baseline, use the `criterion` already in dev-dependencies to build
2-3 smoke benchmarks (full-sample compile time / typical program interpretation throughput / CLI
cold start) and check them in, serving as the performance criterion for subsequent pipeline-class
changes—slot reuse deletion, Phi expansion into Move sequences, Driver indirection layers can all
change time cost, and without a baseline no one will catch it.

### Dedicated Criteria for Fixing the Correctness Hole

That hole which [`02`](02-stage-contract.md) must fix requires **independent criteria** and cannot
be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // The same source containing x: Sorted(3) must produce identical diagnostic sets
    // on the single-file and multi-file paths
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written to fail** (red), confirming that it catches the existing hole,
before starting the fix. If it unexpectedly passes, the hole analysis needs to be re-examined.

Companion criteria ([`02`](02-stage-contract.md)配套):

| Test                               | Assertion                                                                     | Defect Caught                                      |
| ---------------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------- |
| `test_program_stage_coverage`      | Each `Program::stages()` ⊆ `Stage::ALL` and equals the expected array         | Future missed stage wiring                         |
| `test_no_silent_pass_on_unproven`  | `ProofResult::Unproven` must produce a diagnostic in any mode                 | The extend branches at `checker.rs:5179/5318/5448` |
| `test_release_plan_spans_consumed` | The Span set produced by ownership ⊆ the Span set consumed by IR construction | Silent Drop loss from `ReleasePlan` span keying    |
| `test_obligations_drained`         | Any `Obligations` field with zero consumers in the full repo fails CI         | The entire category of this defect class           |

### Regression Gates

New CI checks (isomorphic to existing `scripts/ci/` conventions):

| Check                      | Trigger                                                                   | Failure Condition                                                        |
| -------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `check-snapshot-drift.sh`  | C1/C3/C5 class changes                                                    | Snapshot file has diff but commit message lacks `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns non-empty for the full sample set                     |
| `check-corpus-parity.py`   | C2/C4/C5 class changes                                                    | Sample diff is non-empty                                                 |
| `check-stage-contract.py`  | All                                                                       | `Program::stages()` does not match `Stage::ALL`                          |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | Performance baseline regression >10% and PR has no explanation           |

### Binding with Subsidiary Documents

| Document                                                                 | Prerequisite                                                                     | Criterion Strength             |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------------- | ------------------------------ |
| [02-stage-contract.md](02-stage-contract.md) (staging)                   | This document's hole criteria (red first)                                        | C2: diagnostic set identical   |
| [03-type-unification.md](03-type-unification.md) (type representation)   | First layer verifier's type consistency invariant                                | C3: diagnostic codes identical |
| [04-ssa.md](04-ssa.md) (SSA-ization)                                     | **All three layers**, and the verifier must run green on the non-SSA shape first | C4: behavioral equivalence     |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generation) | AST snapshot + diagnostic criteria                                               | C5: all three used             |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)       | Reference scan                                                                   | None                           |

## Key Decisions and Rationale

**Tiered rather than a single criterion.** Using "IR snapshot equality" as the criterion for
SSA-ization (C4) will simply fail, thereby tempting the team to relax the criteria; tiering lets
each step use the strength it can bear. At the same time, the first layer verifier covers dominance
checks, turning "errors that cannot be tested" into "testable invariants"—something snapshots cannot
do. The criteria assets are checked in, evolving with the code, not a one-time investment.

### Directions Not Adopted

- **Using only end-to-end sample diffing**—defect classes 1/2/3 are precisely the kind where "the
  program still runs in some fashion", and end-to-end diffing is insensitive to them. SSA-ization is
  especially dangerous: register number skipping may affect only performance, not output, while
  span-mismatch-induced Drop loss **only manifests under specific inputs**, which the sample library
  may not cover.
- **Using only IR snapshots**—snapshots are inherently immune to defect class 2 (see the
  "Normalization" section), and completely fail in the C4 phase.
- **Using only property-based testing** (proptest is already in the dependencies)—random programs
  are hard to cover ownership, existential, and refinement paths that require specific shapes to
  trigger. **But it can serve as a supplement**: for the first class (simple
  expressions/arithmetic), it does cover better than fixed samples, and it is recommended to
  introduce it as a supplementary criterion for the C4 phase once sample diffing stabilizes.
- **Refactoring first, then adding tests**—this is a pitfall this project has already stepped in.
  The 8 orphan test trees (1005 lines / 78 tests that never ran) recorded in
  [`06-cleanup-inventory.md`](06-cleanup-inventory.md) are the product of "build the directory
  first, wire-up failed later". Criteria must precede refactoring.

## Known Limitations and Risks

- **The first layer verifier needs SSA look-ahead information.** In non-SSA shape, the concept of
  "unique definition point" does not exist (`Operand::Local(n)` points to a slot that can be written
  multiple times). Therefore the verifier needs two modes: `verify_ssa` (strict, includes Phi
  checks) and `verify_loose` (dominance and types only, multiple definitions allowed).
  **`verify_loose` must run green on the existing IR first**—this is the single largest workload in
  this design.
- **The second layer's normalization will miss defect class 2.** This is a known limitation, which
  can only be covered jointly by the first and third layers, not by strengthening normalization.
- **The third layer's sample diffing is sensitive to non-deterministic output.** If any `.yx` sample
  produces output depending on hash iteration order / time, diffing will produce false failures.
  Such samples need to be screened out when establishing the baseline.
- **Checking in snapshots brings maintenance cost.** When the IR definition changes, snapshots must
  be updated explicitly, requiring review discipline.
- **Samples only cover the single-file path.** There is no `yaoxiang.toml` under `tests/`; all 293
  samples go through the single-file path—see the limitations note in "Existing Test Assets" for
  details.

> **The open questions originally listed in this section have all been adjudicated.** Per-item
> decisions are in [RFC-039 Decision Register](../../rfc/draft/039-compiler-architecture.md) (D1–D50).
> **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Feature Routing Directory Design](../../rfc/draft/039-compiler-architecture.md) —
  overarching charter
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, the
  existing "per-item round-trip" criterion example in this repo
- `src/middle/core/tests/bytecode.rs:769-814` — sentinel tests using `include_str!` to read source
  for static comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional set-difference assertions + snapshot
  table, forced update when new frontends are wired in
- `ir_gen.rs:248-249` — explicit declaration of the save/restore physical-adjacency contract
