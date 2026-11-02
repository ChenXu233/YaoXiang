# Refactoring Equivalence Criteria

> **Ancillary design document.** This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main text of RFC-039 for the four-layer model, tiering of acceptance criteria, and
> execution-phase order; see the [index for this directory](index.md) for the positioning of each
> ancillary document.

## Scope and Positioning

This document defines the criteria for determining “how to prove that behavior has not changed”
during refactoring. The criteria are **tiered by the category of the refactoring step**, because
different categories require different strengths of equivalence: pure relocation requires
byte-for-byte identity, whereas SSA conversion necessarily changes the shape of the IR and can only
be judged behaviorally equivalent.

**This document is a prerequisite for all refactoring.** Without criteria,
[SSA conversion](04-ssa.md) cannot be implemented safely.

This document **does not cover**: the stage contracts themselves (see `02`), the design for
converging type representations (see `03`), or the checklist for dead-code cleanup (see `06`).

## Current State

### The most dangerous code today is precisely the code with zero test coverage

`src/middle/core/ir_gen.rs` (8448 lines):

- The file contains **neither `#[cfg(test)]` nor `mod tests`**
- Of the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively lock
  down the sentinel behavior of DefId allocation and slot naming; all four references to ir_gen in
  `bytecode.rs` are **negative assertions** (“ir_gen does not construct a certain instruction”)
- In other words, **IR structural invariants (dominance, unique definitions, jump targets, type
  consistency) currently receive zero verification**—which is precisely what the first layer of
  criteria must address

This file also happens to contain three classes of defects that **produce no error when they fail**:

**Defect class 1: Register misalignment.** The file contains six manually written save/restore
statement pairs, separated by more than 70 lines:

| Save point            | Restore point | Containing function           |
| --------------------- | ------------- | ----------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`          |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`        |
| `2116`                | `2180`        | Curried intermediate function |
| `2224-2229`           | `2284-2288`   | Curried innermost function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`    |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`     |

The file itself states this contract at `ir_gen.rs:248-249`: “**The save/restore points for
`next_temp` must be physically adjacent; otherwise names from inner functions will bleed into outer
functions**.”

The consequence of missing one restore is: `next_temp` leaks and temporary values skip numbers.
**The IR remains self-consistent, still passes all checks, and still compiles; only the values are
wrong.** Ordinary assertion tests cannot catch this.

**Defect class 2: Misalignment caused by semantic reordering of `arg_regs`.** The six branches
inside `generate_call_expr_ir` (`7269-8014`, 747 lines) share the same `Vec<Operand>` and perform
semantic rewrites:

| Line        | Reordering semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

Default-value registers are also appended midway through the process. **Misalignment produces no
error; it only produces incorrect values.**

More dangerously: this class of error may also go undetected under “normalized snapshots”—if the
snapshot tool normalizes temporary register numbers away (as is standard practice), then an error
such as “the third argument should use the fifth register” is **exactly the same** in the snapshot.

**Defect class 3 (eliminated): Silent failure of a span-keyed cross-layer contract.** `ReleasePlan`
was once keyed by `Span` (produced by `layers/ownership.rs:31`), passed through `checker.rs:1440`,
and ultimately matched by `self.release_plan.get(&stmt.span)` in `ir_gen`, which emitted
`Instruction::Drop`; any inconsistency in how the two sides calculate spans → **the Drop instruction
silently disappears, with no error whatsoever**. `overload_resolutions` has the same form.
**2026-10-13 (RFC-039 D20 / WBS 4.5)**: both contracts were changed to use `PlanId`
**structural-identity keys** (the sole consumption entry points `ReleasePlan::consume` /
`OverloadResolutions::get` perform reading and accounting in the same invocation; any non-empty
difference raises an E-level `W1090`). Defects in this class are now reduced to zero—see
[02](02-stage-contract.md) §2 revision notes for the implementation form and criteria.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> whose key is the module-unqualified `"Type.method"`)—its risk profile is different
> (definition-order/duplicate-name mismatch rather than span mismatch), and SSA conversion does not
> address it (see mechanism three in [04](04-ssa.md) for its disposition). Therefore, there are
> **2** span-keyed contracts (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw materials for the criteria are abundant:

| Asset                               | Scale                                                                                                                                                             | Can be used for                                                                                            |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293 corpus files**, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see the limitation below | End-to-end behavioral differential (**single-file side only**)                                             |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                           | Standard-library behavioral differential                                                                   |
| `tests/integration/`                | 18 modules / 5846 lines                                                                                                                                           | CLI end-to-end testing (of which the 27 tests in `multifile.rs` are the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, round-tripping each opcode)                                  | Bytecode-layer round trips                                                                                 |

> **⚠️ Structural limitation of corpus coverage (added 2026-10-03)**
>
> There are **no `yaoxiang.toml` files at all** under the `tests/` directory (measured: 0).
> Therefore, all 293 corpus files take the `standalone` branch of `util/diagnostic/mod.rs:565`
> `check_files_with_diagnostics` → `check_single_file` → **`Pipeline::run` (the single-file path)**.
>
> **Consequence**: the third layer of criteria can **only verify changes to the single-file path**.
> For the multi-file side of [`02`](02-stage-contract.md) (`orchestrator`’s `check_project` /
> `compile_project` / `check_source_in_project` / `compile_embedded_module`) and the multi-file
> behavior of [`03`](03-type-unification.md) / [`04`](04-ssa.md), **there is currently no corpus
> coverage**.
>
> **A multi-file corpus layer must be established in P2**, otherwise the largest refactoring stage,
> “unified Driver,” has no executable criteria. The 27 tests in `tests/integration/multifile.rs` are
> the only existing multi-file-side coverage, but they test semantic outcomes rather than
> compilation-stage contracts.

**However, these assets share a common defect**: they test “input → output,” not “whether the
intermediate representation is correct.” The IR layer currently has **no criteria whatsoever**.

## Target Design

### Core Principle: Criteria Are Tiered by Refactoring Category

**This is the single most important principle in this document.** Using a single criterion (usually
“exact equality of IR snapshots”) for all types of refactoring is incorrect—because some
refactorings **necessarily change the shape of the IR**.

| Refactoring category                                                                  | Criterion type                                                                                    | Strength  | Related document                                      |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | --------- | ----------------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | **Zero-diff** normalized IR snapshots                                                             | Strongest | The checker split in `02`, the lowering split in `04` |
| **C2 Orchestration changes** (staged architecture, unified Driver)                    | Identical **diagnostic sets** for each entry point + identical corpus behavior                    | Strong    | `02`                                                  |
| **C3 Type representation convergence**                                                | Identical **diagnostic codes and messages** (order may be normalized)                             | Strong    | `03`                                                  |
| **C4 IR shape changes** (SSA conversion)                                              | **Behavioral equivalence** (identical execution results) + IR structural invariants               | Medium    | `04`                                                  |
| **C5 Front-end paradigm changes** (grammar-driven)                                    | Identical AST snapshots + **identical diagnostic code+span for every entry** + identical behavior | Strong    | `05`                                                  |
| **C6 Pure deletion**                                                                  | No equivalence criterion required; only confirmation that no references remain                    | —         | `06`                                                  |

**The distinction between C1 and C4 is critical**: C1 requires byte-for-byte identical IR (any
change is a regression); C4 acknowledges that the IR will change and instead uses “identical program
behavior + IR satisfies the invariants.”

> **⚠️ C5′ explicitly does not exist**
>
> The diagnostic requirements in the error-recovery dimension are **not relaxed**: LALRPOP’s
> error-recovery model differs from `synchronize()`, and precisely for that reason, the grammar in
> [05](05-frontend-paradigm.md) **must model the behavior of `synchronize()` using explicit error
> productions**, so that diagnostics remain identical entry by entry. Failing to do so is an
> implementation defect and **does not justify relaxing the criteria**.
>
> If phases 3a/3b empirically find that the existing diagnostics cannot be reproduced under LALRPOP,
> the correct action is to **report the finding honestly and reassess the design** (including the
> option of “retaining Pratt”), **not to modify the criteria**.

### Three Layers of Criteria

#### First Layer: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is far
stronger than snapshots because it checks **semantic invariants** rather than a concrete shape.

| Invariant                                                                          | Defects caught                                                                                                                      | Typical cause                                                  |
| ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **Use-before-def dominance**                                                       | Every `Operand::Local(n)` read must have a definition path that dominates the current point on the CFG                              | Register misalignment, omitted restore                         |
| **Unique value definition** (only in SSA form)                                     | Every SSA value must have exactly one definition point; joins must contain a `Phi`                                                  | Correctness of the SSA conversion itself                       |
| **Phi consistency**                                                                | The number of `Phi` inputs must equal the number of corresponding predecessor basic blocks, and every input type must be consistent | Missing predecessor, type mismatch                             |
| **Jump target exists**                                                             | The label of every jump must exist in `blocks`                                                                                      | Translation errors in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot out of bounds**                                                      | `Operand::Global(i)` must satisfy `i < globals.len()`                                                                               | Slot allocation errors                                         |
| **Type consistency**                                                               | The `MonoType` of an instruction’s `dst` and `src` must be compatible                                                               | Bridging type-representation errors                            |
| **Inner isolation** (corresponding to the explicit promise at `ir_gen.rs:248-249`) | Function A’s `cur_locals` must not contain function B’s temporary name                                                              | The six save/restore contracts                                 |

**The verifier must exist before SSA conversion** and must **pass first on the current (non-SSA)
IR**—otherwise it cannot be used to prove equivalence before and after the transformation.

#### Second Layer: Normalized IR Snapshots (Covers C1/C3/C5)

IR snapshots must be **normalized**; otherwise, any change in register numbering will cause the full
diff to fail:

| Normalization item         | Rule                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Strip `Span`               | Set all span fields to a sentinel value. **This finally holds starting with WBS 6.6.12**: previously, the `type_decl` line printed the `ast::Type` Debug text of the type-declaration carrier, and empirically **100% of span leakage in IR snapshots came from that line** (96 lines across 69 files; the pre-change originals are archived in 09a §P6 6.6.12); after the carrier was replaced with an IR-native narrowed shape, spans no longer appear anywhere in the snapshot |
| Temporary value renaming   | Rename to `%0 %1 %2 ...` in order of first appearance, **preserving the definition-use topological order**                                                                                                                                                                                                                                                                                                                                                                        |
| Global slot relativization | Renumber according to order of first appearance within the module                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Predecessor sorting        | Sort `successors` and `Phi` inputs by label                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Struct field sorting       | Sort only when field names are known (construction order is semantic and must not be sorted)                                                                                                                                                                                                                                                                                                                                                                                      |

**The normalization tool’s output must be checked into the repository**
(`src/middle/core/tests/snapshots/`), explicitly updated whenever the IR definition changes, and
manually reviewed when the diff is updated.

**Known limitation (must be documented)**: normalization erases information about “which register
the Nth argument used,” and therefore **the second layer alone is insufficient to cover defect class
2**. During C1, the safety of `arg_regs` reordering must be guaranteed jointly by the first layer’s
dominance checks and the corpus behavioral differential.

#### Third Layer: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the refactoring, and compare:

| Comparison target      | Normalization rule                                                                                                                                  |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is not included in the comparison** (type representation convergence may change the wording) |
| Exit code              | Compare directly                                                                                                                                    |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                               |
| `dump_bytecode` output | Compare only during the C1/C2 phases (bytecode may change as the IR changes)                                                                        |

**The behavioral differential in the C4 (SSA conversion) phase must cover**: all three
ownership-transfer forms, ref/borrow/move; closure captures; currying (`generate_curry_*`); spawn
and iterator for; sum-type pattern matching; existential coercion points; `?` / Try propagation;
method overloading (`overload_resolutions` span keying); and Drop sequences for refinement
constraints (`ReleasePlan`).

**Performance baseline**: the corpus differential guarantees “identical behavior,” not “identical
elapsed time.” While establishing the differential baseline in P2, use `criterion`, which is already
present in dev-dependencies, to establish and check in 2–3 smoke benchmarks (compile time for the
entire corpus / interpreter throughput for a typical program / CLI cold start) as performance
criteria for subsequent pipeline changes—removing slot reuse, expanding Phi into Move chains, or
adding a Driver indirection layer may all affect elapsed time, and without a baseline, nothing will
intercept such regressions.

### Dedicated Criteria for Correcting the Correctness Loophole

The loophole to be fixed in [`02`](02-stage-contract.md) requires an **independent criterion** and
must not be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // The diagnostic sets for the same source containing x: Sorted(3) must be identical across the single-file and multi-file paths
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written in a failing state** (red), confirming that it captures the
existing loophole before the fix begins. If it unexpectedly passes, the loophole analysis must be
reviewed again.

Related criteria (accompanying [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                                                   | Defect intercepted                                                                                                                          |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_program_stage_coverage`        | Every `Program::stages()` is a subset of `Stage::ALL` and equals the expected array                                                                         | A stage omitted in the future                                                                                                               |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must produce a diagnostic in every mode                                                                                             | The three `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                               |
| `test_release_plan_spans_consumed`   | The set of `PlanId`s produced by ownership is a subset of the set consumed by IR construction (**settled by structural identity, with no allowlist**)       | `ReleasePlan` span key silently dropping Drop (historical form; after key migration, the consumption ledger + `W1090` provide the backstop) |
| `test_overload_resolutions_consumed` | Corpus file by file: overload-resolution production is a subset of consumption, with an evidence floor of `examined ≥ 1` / at least `≥ 2` actually consumed | Resolution entries being silently dropped (silently incorrect answers from falling back to the default binding)                             |
| `test_obligations_drained`           | CI fails if any `Obligations` field has no consumption point anywhere in the repository                                                                     | The entire class of defects                                                                                                                 |

### Routing Criteria for Semantic Corrections (D59)

RFC-039’s refactoring contract consists of **tiered equivalence (C1–C6) + a dedicated
semantic-correction channel** (P3 / P3.5 + decision registry): the constraints “no C5′” and “no
relaxation of any kind” govern **the strength of the criteria**, not “incorrect semantics must be
preserved verbatim.” There is exactly one **decidable entry criterion** for routing:

> **Question: Does the old behavior count as “success”?** (Success = it compiles and produces a
> correct result)

**Not success → classes A/B/C may be fixed as part of the refactoring**, but all four of the
following are required:

1. **Red before green**—the red criterion must exist first and must genuinely be red at the current
   revision (a self-check item in [HOWTO](../HOWTO.md)). “Green before red” does not count: without
   a red criterion, a subsequent “fix” cannot be distinguished from a regression after the fact.
2. **Do not change the externally observable behavior of programs that originally succeeded**—a
   program that could compile and run correctly must not become a failure because of this change,
   and its existing diagnostics (code / wording / exit code) must not change either. It is
   **permitted** to make paths that were not originally successful more visible (previously no
   diagnostic → adding a W/E code or increasing severity are both permitted); it is **not
   permitted** to change the externally observable behavior of an originally successful path.
3. **Register in the same commit**—the corresponding WBS row is tagged with the error type (A/B/C),
   and **the affected body text is corrected in place** (the body describes only the current state;
   the before/after correction diff and red→green evidence are retained in the commit
   message/recording), plus the PR’s mandatory “Error type” line.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshots / corpus
   differential). The classification is **not** grounds to exempt this review.

| Class                                                  | Criterion                                                                             | Typical form                                                                                  | Precedent                           |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A Silent failure**                                   | An error that should have been reported is swallowed                                  | Empty `Unproven` branch, silent missing-solver behavior, `unwrap_or_default` degradation      | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent mismatch**                                  | Key mismatch / choosing one of two parallel representations / span-keyed misalignment | Span key silently dropping Drop, two representations of the same concept                      | 3.4.5 / D20 / D57                   |
| **C Undefined behavior and uncontrolled resource use** | Crash, stack overflow, uninitialized slot, bytecode version mismatch                  | Pathological generic recursion (compile-time stack overflow), three hard-coded `.42` discards | 3.4.8 ①/② / D52                     |

**Originally successful programs that now need to change → classes D/E/F still require user
arbitration first** (see “When requesting user arbitration (mandatory)” in the repository-root
`AGENTS.md`):

| Class                          | Criterion                                                                                                                      | Examples                  |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------- |
| **D User-visible semantics**   | Externally visible additions/removals/changes to diagnostic codes, externally visible error text, module semantics, exit codes | D54 / D55 / RFC-029g      |
| **E External contract tables** | The error-code table, opcode table, type table, or entry-stage table itself                                                    | D1 / D6 / D12 / D20 / D23 |
| **F RFC design change**        | Metatype universe, new E codes, wholesale removal of syntax                                                                    | D53 / RFC-033 / RFC-029g  |

**Primary-error attribution**: a change is classified by its **primary error type**; incidental
behavioral changes bundled into the minimal fix set for the same root cause may be listed separately
on the PR’s “D3 patch determination” line (precedent: 3.4.8 ① has primary class C, with three
asymmetric class A defects attached).

**Relationship to C1/C2**: A/B/C changes **must not** be declared C1 (byte-for-byte invariance) or
C2 (identical diagnostic set)—those categories define behavioral invariance. An opportunistic fix
must be declared under the corresponding category among C3/C4/C6.

**Why no topical allowlist**: [08](08-maintenance-mechanism.md) has already declared that “the
exemption count itself enters the gate report”; the list would grow without bound, and cases of “one
primary class + incidental fixes” inevitably fall on the boundary. The entry criterion is boolean
and decidable.

### Regression Gates

Add the following CI checks (following the existing conventions in `scripts/ci/`):

| Check                      | Trigger                                                                   | Failure condition                                                                                    |
| -------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | C1/C3/C5 changes                                                          | The snapshot files have a diff, but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns a non-empty error set for the entire corpus                                       |
| `check-corpus-parity.py`   | C2/C4/C5 changes                                                          | The corpus differential is non-empty                                                                 |
| `check-stage-contract.py`  | All changes                                                               | `Program::stages()` does not match `Stage::ALL`                                                      |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | The performance baseline regresses by >10%, and the PR provides no explanation                       |

### Binding to Each Ancillary Document

| Document                                                                      | Must have first                                                            | Criterion strength             |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------ |
| [02-stage-contract.md](02-stage-contract.md) (staged architecture)            | The loophole criteria from this document (red first)                       | C2: identical diagnostic set   |
| [03-type-unification.md](03-type-unification.md) (type representation)        | The type-consistency invariant from the first layer                        | C3: identical diagnostic codes |
| [04-ssa.md](04-ssa.md) (SSA conversion)                                       | **All three layers**, and the verifier must first pass on the non-SSA form | C4: behavioral equivalence     |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (grammar-driven front end) | AST snapshots + diagnostic criteria                                        | C5: all three used             |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)            | Reference scan                                                             | None                           |

## Key Decisions and Rationale

**Tiering rather than a single criterion.** Using “exact equality of IR snapshots” as the criterion
for SSA conversion (C4) would fail outright, potentially pressuring the team to weaken the criteria;
tiering ensures that each step uses a strength it can withstand. Meanwhile, the first layer’s
verifier covers dominance checks, turning “errors that cannot be tested” into “invariants that can
be tested”—something snapshots cannot do. The criterion assets are checked into the repository and
evolve with the code; they are not a one-time investment.

### Rejected Approaches

- **End-to-end corpus differential only**—defect classes 1/2/3 are precisely those where “the
  program still runs to completion in some fashion,” and the end-to-end differential is insensitive
  to them. SSA conversion is especially dangerous: skipped register numbers may affect only
  performance rather than output, while a Drop loss caused by span mismatch **manifests only for
  specific inputs**, and the corpus may not cover it.
- **IR snapshots only**—snapshots are inherently immune to defect class 2 (see “Normalization”) and
  fail completely in the C4 phase.
- **Property-based testing only** (`proptest` is already in the dependencies)—random programs have
  difficulty covering ownership, existential, and refinement paths that require particular shapes to
  trigger. **It can, however, serve as a supplement**: for the first category (simple
  expressions/arithmetic), it genuinely provides better coverage than a fixed corpus. It is
  recommended for introduction as a supplementary criterion in the C4 phase once the corpus
  differential is stable.
- **Refactor first, add tests later**—this project has already encountered that trap. The 8 orphan
  test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1005 lines / 78 tests
  that have never run) are the product of creating the directory first and then failing to wire it
  up. Criteria must precede refactoring.

## Known Limitations and Risks

- **The first-layer verifier requires SSA lookahead information.** In the non-SSA form, the concept
  of a “unique definition point” does not exist (`Operand::Local(n)` refers to a slot that may be
  written multiple times). The verifier therefore needs two modes: `verify_ssa` (strict, including
  Phi checks) and `verify_loose` (only dominance and types, allowing multiple definitions).
  **`verify_loose` must pass first on the existing IR**; this is the single largest piece of work in
  this design.
- **Second-layer normalization misses defect class 2.** This is a known limitation and can only be
  addressed jointly by the first and third layers; it cannot be solved by strengthening
  normalization.
- **The third-layer corpus differential is sensitive to nondeterministic output.** If any `.yx`
  corpus program produces output that depends on hash iteration order or time, the differential will
  fail spuriously. Such corpus programs must be filtered out when establishing the baseline.
- **Checking snapshots into the repository creates maintenance overhead.** Whenever the IR
  definition changes, the snapshots must be explicitly updated, requiring review discipline.
- **The corpus covers only the single-file path.** There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus files take the single-file path—see the limitation notes under “Existing Test
  Assets.”

> **All open questions originally listed in this section have now been adjudicated.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) for the item-by-item
> decisions (D1–D50). **No pending items remain in this document.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — overarching master plan
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, an
  existing “item-by-item round-trip” criterion example in this repository
- `src/middle/core/tests/bytecode.rs:769-814` — a static source-to-output differential sentinel test
  that uses `include_str!` to read source code
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional set-difference assertions + snapshot
  table, forcing an update whenever a new front end is integrated
- `ir_gen.rs:248-249` — explicit statement of the physical-adjacency save/restore contract
