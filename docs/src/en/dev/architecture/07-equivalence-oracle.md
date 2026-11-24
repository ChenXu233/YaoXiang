# Refactoring Equivalence Criteria

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiering of acceptance criteria, and execution-phase ordering are covered in the
> main RFC-039 text; the role of each companion document is described in the
> [directory index](index.md).

## Purpose and Scope

This document defines the criteria for “proving that behavior has not changed” during the
refactoring. The criteria are **tiered by the category of the refactoring step**, because different
categories require different strengths of equivalence: pure relocation requires byte-for-byte
identity, whereas SSA transformation necessarily changes the shape of the IR and can only be judged
behaviorally equivalent.

**This document is a prerequisite for all refactoring.** Without these criteria,
[SSA transformation](04-ssa.md) cannot be implemented safely.

This document **does not cover**: the stage contracts themselves (see `02`), the design for
converging type representations (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The most dangerous code is also the code with zero test coverage

`src/middle/core/ir_gen.rs` (8,448 lines):

- The file contains **no `#[cfg(test)]` and no `mod tests`**
- Of the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively pin down
  sentinel behavior for DefId allocation and slot naming; all four assertions in `bytecode.rs` that
  mention `ir_gen` are **inverse assertions** (“`ir_gen` does not construct a particular
  instruction”)
- In other words, **IR structural invariants (dominance, unique definitions, jump targets, and type
  consistency) currently have zero validation**—precisely what the first-tier criterion is intended
  to fill

This file also happens to contain three classes of defects that **produce no error at all when they
fail**:

**Defect class 1: Register misalignment.** The file contains six manually written pairs of
save/restore statements, separated by more than 70 lines:

| Save point            | Restore point | Containing function           |
| --------------------- | ------------- | ----------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`          |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`        |
| `2116`                | `2180`        | Intermediate curried function |
| `2224-2229`           | `2284-2288`   | Innermost curried function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`    |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`     |

The file itself states this contract at `ir_gen.rs:248-249`: “**The save/restore point for
`next_temp` must be physically adjacent; otherwise, inner-function names will leak into the outer
scope**.”

Missing even one restore causes `next_temp` to leak and temporary-value numbering to skip values.
**The IR remains internally consistent, still passes every check, and still compiles; only the
values are wrong.** Ordinary assertion tests cannot catch this.

**Defect class 2: Semantic reordering misalignment in `arg_regs`.** The six branches inside
`generate_call_expr_ir` (`7269-8014`, 747 lines) share the same `Vec<Operand>` and perform semantic
rewrites:

| Line        | Reordering semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

Default-value registers are also appended along the way. **A misalignment reports no error; it only
produces incorrect values.**

This class of error is even more dangerous because it **may also go undetected under a “normalized
snapshot”**: if the snapshot tool normalizes temporary register numbers away, as is standard
practice, then an error such as “the third argument should have used the fifth register” appears
**completely identical** in the snapshot.

**Defect class 3 (eliminated): Silent failure of a span-keyed cross-layer contract.** `ReleasePlan`
previously used `Span` as its key (produced by `layers/ownership.rs:31`), passed it through
`checker.rs:1440`, and ultimately matched it in `ir_gen` through `self.release_plan.get(&stmt.span)`
before emitting `Instruction::Drop`. Any inconsistency between the two sides’ span computation →
**the Drop instruction silently disappears, with no error whatsoever**. `overload_resolutions` has
the same shape. **2026-10-13 (RFC-039 D20 / WBS 4.5):** Both contracts were changed to use `PlanId`
**structural-identity keys** (through the unique consumption entry points `ReleasePlan::consume` /
`OverloadResolutions::get`, with the read and accounting occurring in the same invocation; any
non-empty difference is E-level `W1090`). Defects in this class are now zero—the implemented form
and criteria are described in the revision note in [02](02-stage-contract.md) §2.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> with the module-unqualified key `"Type.method"`)—its risk characteristics differ
> (definition-order/name-collision mismatch rather than span mismatch), and SSA transformation does
> not address it (see mechanism three in [04](04-ssa.md)). There are therefore **2** span-keyed
> contracts (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that there are sufficient raw materials for the criteria:

| Asset                               | Scale                                                                                                                                                             | Can be used for                                                                                            |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293 corpus cases**, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see the limitation below | End-to-end behavioral differential (**single-file side only**)                                             |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                           | Standard-library behavioral differential                                                                   |
| `tests/integration/`                | 18 modules / 5,846 lines                                                                                                                                          | CLI end-to-end testing (of which the 27 tests in `multifile.rs` are the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1,131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, round-tripping each opcode)                                 | Bytecode-layer round-tripping                                                                              |

> **⚠️ Structural limitation of corpus coverage (added 2026-10-03)**
>
> There is **no `yaoxiang.toml` anywhere under `tests/`** (measured: 0). Consequently, all 293
> corpus cases take the `standalone` branch of `util/diagnostic/mod.rs:565`
> `check_files_with_diagnostics` → `check_single_file` → **`Pipeline::run` (the single-file path)**.
>
> **Consequence:** The third-tier criterion can **only validate changes on the single-file path**.
> For the multi-file side of [02](02-stage-contract.md) (`orchestrator`’s `check_project` /
> `compile_project` / `check_source_in_project` / `compile_embedded_module`) and the multi-file
> behavior of [03](03-type-unification.md) / [04](04-ssa.md), **there is currently no corpus
> coverage**.
>
> **A multi-file corpus tier must be established in P2**; otherwise, the largest refactoring
> stage—the “unified Driver”—has no executable criterion. The 27 tests in
> `tests/integration/multifile.rs` are the only existing multi-file-side coverage, but they test
> semantic outcomes rather than compilation-stage contracts.

**However, these assets share a common flaw**: they test “input → output,” not whether the
intermediate representation is correct. Correctness at the IR layer currently has **no criterion at
all**.

## Target Design

### Core Principle: Tier the Criteria by Refactoring Category

**This is the most important principle in this document.** Using a single criterion (usually “full
equality of IR snapshots”) for every type of refactoring is wrong, because some refactors
**necessarily change the shape of the IR**.

| Refactoring category                                                                  | Criterion type                                                                      | Strength  | Relevant document                                     |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | --------- | ----------------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | IR normalized snapshot **zero-diff**                                                | Strongest | Checker splitting in `02`; lowering splitting in `04` |
| **C2 Orchestration changes** (staged architecture, unified Driver)                    | **Same diagnostic set** for each entry point + same corpus behavior                 | Strong    | `02`                                                  |
| **C3 Type-representation convergence**                                                | **Same diagnostic codes and messages** (order may be normalized)                    | Strong    | `03`                                                  |
| **C4 IR-shape changes** (SSA transformation)                                          | **Behavioral equivalence** (same execution results) + IR structural invariants      | Medium    | `04`                                                  |
| **C5 Frontend paradigm changes** (grammar-driven)                                     | Same AST snapshot + **identical diagnostic code+span item by item** + same behavior | Strong    | `05`                                                  |
| **C6 Pure deletion**                                                                  | No equivalence criterion; only confirmation that no references remain               | —         | `06`                                                  |

**The distinction between C1 and C4 is critical**: C1 requires byte-for-byte identical IR (any
change is a regression), whereas C4 acknowledges that the IR will change and instead requires “the
program behaves the same + the IR satisfies its invariants.”

> **⚠️ C5′ explicitly does not exist**
>
> Diagnostic requirements in the error-recovery dimension are **not relaxed**: LALRPOP’s
> error-recovery model differs from `synchronize()`, which is precisely why the grammar in
> [05](05-frontend-paradigm.md) **must model the behavior of `synchronize()` using explicit error
> productions**, so that diagnostics remain identical item by item. Failure to do so is an
> implementation defect and **does not constitute grounds for relaxing the criterion**.
>
> If phase 3a/3b testing finds that existing diagnostics cannot be reproduced under LALRPOP, the
> correct action is to **report the finding truthfully and reassess the design** (including the
> option of “retaining Pratt”), **not to modify the criteria**.

### Three-Tier Criteria

#### First Tier: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is far
stronger than a snapshot because it checks **semantic invariants**, rather than a concrete shape.

| Invariant                                                                                | Defects caught                                                                                                                      | Typical cause                                                  |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **Use-before-def dominance**                                                             | Every read of `Operand::Local(n)` must have a definition path that dominates the current point on the CFG                           | Register misalignment, missed restore                          |
| **Unique value definition** (only in SSA form)                                           | Every SSA value must have exactly one definition point; joins must contain a `Phi`                                                  | Correctness of SSA transformation itself                       |
| **Phi consistency**                                                                      | The number of `Phi` inputs must equal the number of corresponding predecessor basic blocks, and every input type must be consistent | Omitted predecessor, type mismatch                             |
| **Jump target exists**                                                                   | Every jump label must exist in `blocks`                                                                                             | Translation errors in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot bounds**                                                                   | `Operand::Global(i)` must satisfy `i < globals.len()`                                                                               | Slot-allocation errors                                         |
| **Type consistency**                                                                     | The `MonoType` of an instruction’s `dst` and `src` must be compatible                                                               | Errors in bridging type representations                        |
| **Inner-scope isolation** (corresponding to the explicit promise at `ir_gen.rs:248-249`) | Function A’s `cur_locals` must not contain temporary names from function B                                                          | The six save/restore contracts                                 |

**The verifier must exist before SSA transformation** and must **pass first on the current (non-SSA)
IR**—otherwise, it cannot be used to prove equivalence before and after the transformation.

#### Second Tier: Normalized IR Snapshots (Cover C1/C3/C5)

IR snapshots must be **normalized**; otherwise, any change in register numbering will cause the full
diff to fail:

| Normalization item         | Rule                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Strip `Span`               | Set every span field to a sentinel value. **Only starting with WBS 6.6.12 does this actually hold**: previously, the `type_decl` line printed the Debug text of the type-declaration carrier's `ast::Type`, and measurements showed that **100% of span leaks in IR snapshots came from that line** (96 lines / 69 files); after the carrier was replaced with an IR-native narrowed shape, spans no longer appear anywhere in the snapshot |
| Temporary-value renaming   | Rename in order of first occurrence to `%0 %1 %2 ...`, **preserving the definition-use topological order**                                                                                                                                                                                                                                                                                                                                                                                              |
| Global-slot relativization | Renumber in order of first occurrence within the module                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Predecessor ordering       | Sort `successors` and `Phi` inputs by label                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Struct-field ordering      | Sort only when field names are known (construction order is semantic and must not be sorted)                                                                                                                                                                                                                                                                                                                                                                                                            |

**The normalization tool’s output must be checked in** (`src/middle/ir/tests/snapshots/`) and
explicitly updated whenever the IR definition changes; diffs must be manually reviewed during
updates.

**Known limitation (must be documented)**: Normalization erases information about “which register
the Nth argument used,” so **the second tier alone is insufficient to cover defect class 2**. Safety
of `arg_regs` reordering during C1 must be guaranteed jointly by the first tier’s dominance checks
and corpus behavioral differentials.

#### Third Tier: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the transformation, and compare:

| Comparison target      | Normalization rule                                                                                                            |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is not compared** (type-representation convergence may change wording) |
| Exit code              | Compare directly                                                                                                              |
| Runtime stdout/stderr  | Compare after sorting                                                                                                         |
| `dump_bytecode` output | Compare only during C1/C2 (bytecode may change with the IR)                                                                   |

**Behavioral differentials during the C4 (SSA transformation) phase must cover**: all three
ownership transfers—ref/borrow/move; closure capture; currying (`generate_curry_*`); `spawn` and
iterator `for`; sum-type pattern matching; existential forcing points; `?` / Try propagation; method
overloading (`overload_resolutions`, span-keyed); and the Drop sequence for refinement constraints
(`ReleasePlan`).

**Performance baseline**: A corpus differential guarantees “the same behavior,” but not “the same
elapsed time.” While P2 establishes the differential baseline, it must also use `criterion`, already
present in dev-dependencies, to create and check in 2–3 smoke benchmarks (whole-corpus compilation
time / typical-program interpretation throughput / CLI cold-start time) as the performance criterion
for subsequent pipeline changes. Removing slot reuse, expanding Phi into Move chains, and adding a
Driver indirection layer can all affect elapsed time; without a baseline, no one can catch such
regressions.

### Dedicated Criterion for Correctness-Hole Fixes

The hole that [`02`](02-stage-contract.md) is to fix requires an **independent criterion** and must
not be mixed into the three tiers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets of the single-file and multifile paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must be written to fail first** (red), confirming that it can catch the existing hole
before the fix begins. If it unexpectedly passes (green), the vulnerability analysis must be
reconsidered.

Analogous criteria (accompanying [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                                                | Defect caught                                                                                                                                    |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_program_stage_coverage`        | Every `Program::stages()` ⊆ `Stage::ALL` and equals the expected array                                                                                   | A stage omitted in the future                                                                                                                    |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must produce a diagnostic in every mode                                                                                          | The three `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                                    |
| `test_release_plan_spans_consumed`   | The set of `PlanId`s produced by ownership ⊆ the set consumed during IR construction (**settled by structural identity, with no allowlist**)             | `ReleasePlan` span keys silently dropping Drop (historical form; after the key migration, the consumption ledger + `W1090` provide the backstop) |
| `test_overload_resolutions_consumed` | Corpus case by corpus case: produced overload resolutions ⊆ consumed resolutions, with an **evidence floor** of `examined ≥ 1` / actually consumed `≥ 2` | Resolution entries being silently discarded (a silent wrong answer caused by falling back to a default binding)                                  |
| `test_obligations_drained`           | CI fails if any `Obligations` field has no consumption point anywhere in the repository                                                                  | This entire class of defects                                                                                                                     |

### Triage Criterion for Semantic Corrections (D59)

RFC-039’s refactoring contract consists of **tiered equivalence (C1–C6) + a specially designated
semantic-correction channel** (P3 / P3.5 + decision registry). The constraints “C5′ does not exist”
and “no relaxation is permitted” govern the **strength of the criteria**, not whether erroneous
semantics must be preserved verbatim. There is only one **decidable entry criterion**:

> **Question: Did the old behavior count as a “success”?** (Success = compilation succeeds and
> produces the correct result)

**If it did not count as success → A/B/C may be fixed alongside the refactoring, but all four of the
following are required:**

1. **Red before green**—The red criterion must exist first and must genuinely be red at the time
   (the self-check item in [HOWTO](../HOWTO.md)). A green-first/red-later sequence does not qualify:
   without a red criterion, it is impossible afterward to distinguish a “fix” from a regression.
2. **Do not change the externally observable behavior of programs that previously succeeded**—A
   program that previously compiled and ran correctly must not become a failure because of this
   change, and its existing diagnostics (code / wording / exit code) must not change either. It **is
   permitted** to make a path that previously did not count as success more visible (previously no
   diagnostic → adding a W/E code or increasing severity are both allowed); it is **not permitted**
   to alter the externally observable behavior of a path that previously succeeded.
3. **Register it in the same commit**—The corresponding WBS row must be labeled with the error type
   (A/B/C), and the affected body text must be **corrected in place** (the body describes only the
   current state; the before/after correction diff and red→green evidence are retained in the commit
   message or recording), and the PR’s required “Error type” block must contain one line.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshot / corpus
   differential). Classification is **not** a reason to exempt this review.

| Class                                                    | Criterion                                                                           | Typical form                                                                                 | Precedent                           |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A Silent failure**                                     | An error that should have been reported is swallowed                                | Empty `Unproven` arm, silently missing solver, `unwrap_or_default` fallback                  | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent mismatch**                                    | Key mismatch / choosing one of two parallel representations / span-key misalignment | Span key silently dropping Drop, two representations of synonymous concepts                  | 3.4.5 / D20 / D57                   |
| **C Undefined behavior and uncontrolled resource usage** | Crash, stack overflow, uninitialized slot, bytecode version mismatch                | Pathological generic recursion (compile-time stack overflow), three hardcoded `.42` discards | 3.4.8 ①/② / D52                     |

**If it previously succeeded and now needs to change → D/E/F still require user arbitration first**
(see “When requesting user arbitration (mandatory)” in the repository-root `AGENTS.md`):

| Class                        | Criterion                                                                                                                           | Examples                  |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-visible semantics** | Externally visible aspects of adding/removing/changing diagnostic codes, externally visible error text, module semantics, exit code | D54 / D55 / RFC-029g      |
| **E Public contract tables** | The error-code table, opcode table, type table, or entry-stage table itself                                                         | D1 / D6 / D12 / D20 / D23 |
| **F RFC design changes**     | Metatype universe, new E codes, wholesale removal of syntax                                                                         | D53 / RFC-033 / RFC-029g  |

**Primary-error attribution**: A change is classified by its **primary error type**; incidental
behavioral changes within the minimal modification set for the same root cause are listed separately
on the PR’s “D3 patch determination” line (precedent: 3.4.8 ① has a primary class C error, with
three incidental asymmetric class A defects).

**Relationship to C1/C2**: Class A/B/C changes **must not** be declared as C1 (byte-for-byte
unchanged) or C2 (same diagnostic set)—the definitions of those classes are behavior unchanged.
Opportunistic fixes must be declared as the corresponding category among C3/C4/C6.

**Why not use a thematic allowlist**: [08](08-maintenance-mechanism.md) already states that “the
exemption count itself enters the gate report”; the list will grow, and cases involving a “primary
class + incidental fix” inevitably sit on the boundary. The entry criterion is boolean-decidable.

### Regression Gates

Add the following CI checks (following the existing conventions in `scripts/ci/`):

| Check                      | Trigger                                                                   | Failure condition                                                                           |
| -------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | Class C1/C3/C5 changes                                                    | Snapshot files differ, but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns a non-empty result across the full corpus                                |
| `check-corpus-parity.py`   | Class C2/C4/C5 changes                                                    | The corpus differential is non-empty                                                        |
| `check-stage-contract.py`  | All changes                                                               | `Program::stages()` does not match `Stage::ALL`                                             |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | The performance baseline regresses by >10%, and the PR provides no explanation              |

### Binding to the Companion Documents

| Document                                                                     | Must exist first                                                      | Criterion strength         |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------- |
| [02-stage-contract.md](02-stage-contract.md) (staged architecture)           | This document’s hole criterion (red first)                            | C2: same diagnostic set    |
| [03-type-unification.md](03-type-unification.md) (type representations)      | The first-tier verifier’s type-consistency invariant                  | C3: same diagnostic codes  |
| [04-ssa.md](04-ssa.md) (SSA transformation)                                  | **All three tiers**, and the verifier must first pass on non-SSA form | C4: behavioral equivalence |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (grammar-driven frontend) | AST snapshot + diagnostic criteria                                    | C5: all three              |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)           | Reference scan                                                        | None                       |

## Key Decisions and Rationale

**Tiering rather than a single criterion.** Using “full equality of IR snapshots” as the criterion
for SSA transformation (C4) would fail immediately, tempting the team to weaken the criteria;
tiering ensures that each step uses a strength it can support. At the same time, the first-tier
verifier covers dominance checks, turning errors that cannot be tested into invariants that can be
tested—something snapshots cannot do. The criterion assets are checked in and evolve with the
codebase; they are not a one-time investment.

### Rejected Approaches

- **End-to-end corpus differentials only**—Defect classes 1/2/3 are precisely those in which “the
  program still runs to completion in some form,” so end-to-end differentials are insensitive to
  them. SSA transformation is especially dangerous: a skipped register number may affect only
  performance and not output, while a span mismatch causing a lost Drop **may only manifest for
  particular inputs**, and the corpus may not cover it.
- **IR snapshots only**—Snapshots are inherently immune to defect class 2 (see the “Normalization”
  section) and fail completely during C4.
- **Property-based testing only** (`proptest` is already a dependency)—Random programs have
  difficulty covering ownership, existential, and refinement paths that require particular shapes to
  trigger. **It can, however, serve as a supplement**: for the first class (simple
  expressions/arithmetic), it is indeed better than fixed-corpus coverage. It is recommended for
  introduction as a supplementary C4 criterion after corpus differentials have stabilized.
- **Refactoring first and adding tests later**—This project has already encountered this pitfall.
  The 8 orphan test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1,005
  lines / 78 tests that have never run) are the product of “creating the directory first, then
  failing to wire it up.” The criteria must precede the refactoring.

## Known Limitations and Risks

- **The first-tier verifier requires information that only exists after SSA transformation.** In
  non-SSA form, the concept of a “unique definition point” does not exist (`Operand::Local(n)`
  refers to a slot that may be written multiple times). The verifier therefore needs two modes:
  `verify_ssa` (strict, including Phi checks) and `verify_loose` (dominance and types only, allowing
  multiple definitions). **`verify_loose` must pass on the existing IR first**; this is the largest
  single implementation effort in the design.
- **Second-tier normalization misses defect class 2.** This is a known limitation and can only be
  addressed jointly by the first and third tiers; it cannot be solved by strengthening
  normalization.
- **Third-tier corpus differentials are sensitive to nondeterministic output.** If any `.yx` corpus
  case produces output that depends on hash iteration order or time, the differential may fail
  spuriously. Such cases must be identified and filtered out when establishing the baseline.
- **Checking in snapshots creates a maintenance cost.** Whenever the IR definition changes,
  snapshots must be explicitly updated, requiring review discipline.
- **The corpus covers only the single-file path.** There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus cases use the single-file path—see the limitation described under “Existing Test
  Assets.”

> **All open questions originally listed in this section have been adjudicated.** The decisions are
> listed individually in the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document contains no outstanding items.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — overarching blueprint
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, an
  existing “item-by-item round-trip” criterion exemplar in this repository
- `src/middle/core/tests/bytecode.rs:769-814` — sentinel test that performs a static source
  comparison using `include_str!`
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional difference-set assertions + snapshot
  table, requiring updates whenever a new frontend is integrated
- `ir_gen.rs:248-249` — explicit statement of the physical-adjacency contract for save/restore
