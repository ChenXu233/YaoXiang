# Equivalence Criteria for Refactoring

> **Companion design document.** This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiering of acceptance criteria, and execution-stage ordering are described in
> the main RFC-039 text; see [the index for this directory](index.md) for the role of each companion
> document.

## Purpose and Scope

This document defines the criteria for proving “that behavior has not changed” during refactoring.
The criteria are **tiered by refactoring-step category** because different categories require
different strengths of equivalence: pure relocation requires byte-for-byte identity, whereas SSA
conversion necessarily changes the IR shape and can only be judged behaviorally equivalent.

**This document is a prerequisite for all refactoring.** Without these criteria,
[SSA conversion](04-ssa.md) cannot be implemented safely.

This document **does not cover**: the stage contracts themselves (see `02`), the design for
type-representation convergence (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The most dangerous code currently has zero test coverage

`src/middle/core/ir_gen.rs` (8,448 lines):

- The file contains **neither `#[cfg(test)]` nor `mod tests`**
- Of the 29 tests in `src/middle/core/tests/`, only `def_assign` and `local_slots` positively lock
  down the sentinel behavior of DefId allocation and slot naming; all four assertions in
  `bytecode.rs` that mention `ir_gen` are **negative assertions** (“ir_gen does not construct a
  particular instruction”)
- In other words, **IR structural invariants (dominance, unique definitions, jump targets, and type
  consistency) currently have zero validation**—exactly what the first layer of criteria is intended
  to supply

This file also happens to contain three classes of defects that **produce no error when they fail**:

**Defect class 1: Register misalignment.** The file contains six manually paired save/restore
statements, separated by more than 70 lines:

| Save point            | Restore point | Enclosing function          |
| --------------------- | ------------- | --------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`        |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`      |
| `2116`                | `2180`        | Intermediate curry function |
| `2224-2229`           | `2284-2288`   | Innermost curry function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`  |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`   |

The file states this contract itself at `ir_gen.rs:248-249`: “**It must be physically adjacent to
the `next_temp` save/restore point; otherwise the inner function’s names will bleed into the outer
function**.”

Missing a single restore causes `next_temp` to leak and temporary-value numbering to skip values.
**The IR remains internally consistent, still passes all checks, and still compiles; only the values
are wrong.** Ordinary assertion tests cannot catch this.

**Defect class 2: Misalignment caused by semantic reordering of `arg_regs`.** Inside
`generate_call_expr_ir` (`7269-8014`, 747 lines), six branches share the same `Vec<Operand>` and
rewrite it semantically:

| Line        | Reordering semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

Default-value registers are also appended along the way. **Misalignment produces no error; it only
produces incorrect values.**

This class of error is even more dangerous because it **may not be detected even under a
“canonicalized snapshot”**—if the snapshot tool normalizes temporary register numbers away, as is
customary. An error such as “the third argument should use the fifth register” is then **completely
identical** in the snapshot.

**Defect class 3 (eliminated): Silent failure of a cross-layer, span-keyed contract.** `ReleasePlan`
was once keyed by `Span` (produced at `layers/ownership.rs:31`), passed through `checker.rs:1440`,
and ultimately matched by `self.release_plan.get(&stmt.span)` in `ir_gen` to emit
`Instruction::Drop`. Any inconsistency in how the two sides calculate spans causes the **Drop
instruction to disappear silently, with no error**. `overload_resolutions` follows the same pattern.
**2026-10-13 (RFC-039 D20 / WBS 4.5)**: Both contracts were changed to use `PlanId` as a
**structural-identity key** (the sole consumption entry points `ReleasePlan::consume` /
`OverloadResolutions::get`; reading and accounting occur in the same invocation; a non-empty set
difference produces a level-E `W1090`). Defects of this class are now eliminated—see
[02](02-stage-contract.md) §2 for the implemented form and criteria.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> with an unqualified `"Type.method"` key). Its risk profile is different—a definition-order or
> duplicate-name mismatch rather than a span mismatch—and SSA conversion does not address it (see
> mechanism three in [04](04-ssa.md)). Therefore, there are **2** span-keyed contracts
> (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for these criteria is sufficient:

| Asset                               | Scale                                                                                                                                                                | Can be used for                                                                                          |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpus cases, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All use the single-file path**—see the limitation below | End-to-end behavioral differential (**single-file side only**)                                           |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                              | Standard-library behavioral differential                                                                 |
| `tests/integration/`                | 18 modules / 5,846 lines                                                                                                                                             | CLI end-to-end tests (of which the 27 tests in `multifile.rs` are the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1,131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, round-tripping every opcode)                                   | Bytecode-layer round trips                                                                               |

> **⚠️ Structural limitation of corpus coverage (added 2026-10-03)**
>
> There are **no `yaoxiang.toml` files** under the `tests/` directory (measured: 0). Therefore, all
> 293 corpus cases follow the `standalone` branch of `util/diagnostic/mod.rs:565`
> `check_files_with_diagnostics` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence**: The third layer of criteria can **only validate changes to the single-file
> path**. There is currently **no corpus coverage** for the multi-file side of
> [`02`](02-stage-contract.md) (`orchestrator`’s `check_project` / `compile_project` /
> `check_source_in_project` / `compile_embedded_module`) or for the multi-file behavior of
> [`03`](03-type-unification.md) / [`04`](04-ssa.md).
>
> **A multi-file corpus layer must be established at P2**; otherwise, the largest refactoring stage,
> “unified Driver,” has no executable criteria. The 27 tests in `tests/integration/multifile.rs` are
> the only existing multi-file-side coverage, but they test semantic results rather than
> compilation-stage contracts.

**However, these assets share a common defect**: they test “input → output,” not “whether the
intermediate representation is correct.” The IR layer currently has **no criteria whatsoever**.

## Target Design

### Core Principle: Tier the Criteria by Refactoring Category

**This is the most important rule in this document.** Using a single criterion—usually “full
equality of IR snapshots”—for every refactoring type is incorrect because some refactorings
**necessarily change the IR shape**.

| Refactoring category                                                                  | Criterion type                                                                                    | Strength  | Documents involved                                    |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | --------- | ----------------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | **Zero-diff** canonicalized IR snapshot                                                           | Strongest | The checker split in `02`; the lowering split in `04` |
| **C2 Orchestration changes** (staging, unified Driver)                                | Identical **diagnostic sets** at every entry point + identical corpus behavior                    | Strong    | `02`                                                  |
| **C3 Type-representation convergence**                                                | Identical diagnostic codes and messages (order may be normalized)                                 | Strong    | `03`                                                  |
| **C4 IR-shape changes** (SSA conversion)                                              | **Behavioral equivalence** (identical execution results) + IR structural invariants               | Medium    | `04`                                                  |
| **C5 Frontend paradigm changes** (grammar-driven)                                     | Identical AST snapshots + **identical diagnostic code+span for every entry** + identical behavior | Strong    | `05`                                                  |
| **C6 Pure deletion**                                                                  | No equivalence criterion; confirmation that no references remain                                  | —         | `06`                                                  |

**The distinction between C1 and C4 is critical**: C1 requires byte-for-byte identical IR (any
change is a regression), whereas C4 accepts that the IR will change and instead requires “identical
program behavior + IR satisfying the invariants.”

> **⚠️ C5′ explicitly does not exist**
>
> The diagnostic requirements in the error-recovery dimension are **not relaxed**: the set of
> synchronization points in `ParserState::synchronize()` and the `Expr::Error` / `StmtKind::Error`
> placeholders jointly determine each diagnostic’s code and span. When changes to the syntax surface
> or precedence tables touch these contracts, they must remain identical item by item. Failure to do
> so is an implementation defect and **does not justify relaxing the criteria**.
>
> If testing reveals that a change cannot preserve the existing diagnostics, the correct action is
> to **report the finding faithfully and reassess the proposal** (including “maintain the status
> quo”), **not to modify the criteria**.

### Three-Layer Criteria

#### First Layer: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is much
stronger than snapshots because it checks **semantic invariants**, not a concrete shape.

| Invariant                                                                             | Defects caught                                                                                                                      | Typical cause                                                 |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **Use-before-def dominance**                                                          | Every read of `Operand::Local(n)` must have a definition path that dominates the current point in the CFG                           | Register misalignment, omitted restore                        |
| **Unique value definition** (only in SSA form)                                        | Every SSA value must have exactly one definition point; merge points must have a `Phi`                                              | Incorrect SSA conversion                                      |
| **Phi consistency**                                                                   | The number of `Phi` inputs must equal the number of corresponding predecessor basic blocks, and every input type must be consistent | Missing predecessor, type mismatch                            |
| **Jump target exists**                                                                | Every jump label must exist in `blocks`                                                                                             | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot out of bounds**                                                         | The `i` in `Operand::Global(i)` must satisfy `i < globals.len()`                                                                    | Slot-allocation error                                         |
| **Type consistency**                                                                  | The `MonoType` of an instruction’s `dst` and `src` must be compatible                                                               | Bridged type-representation error                             |
| **Inner isolation** (corresponding to the explicit commitment at `ir_gen.rs:248-249`) | Function A’s `cur_locals` must not contain function B’s temporary names                                                             | The six save/restore contracts                                |

**The verifier must exist before SSA conversion**, and it must **first pass on the current non-SSA
IR**; otherwise, it cannot be used to prove equivalence before and after the transformation.

#### Second Layer: Canonicalized IR Snapshots (Covers C1/C3/C5)

IR snapshots must be **canonicalized**; otherwise, any change to register numbering will cause the
entire diff to fail:

| Canonicalization item   | Rule                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Strip `Span`            | Set every span field to a sentinel value. **This requirement only became fully effective starting with WBS 6.6.12**: previously, the `type_decl` line printed the Debug text of the type declaration carrier’s `ast::Type`, and **100% of span leakage in IR snapshots came from that line** (96 lines / 69 files). After replacing the carrier with an IR-native narrowed shape, spans no longer appear anywhere in snapshots |
| Rename temporary values | Rename to `%0 %1 %2 ...` in order of first appearance, **preserving definition-use topological order**                                                                                                                                                                                                                                                                                                                         |
| Relativize global slots | Renumber according to order of first appearance within the module                                                                                                                                                                                                                                                                                                                                                              |
| Sort predecessors       | Sort `Phi` inputs by predecessor label (`successors` has not been included in snapshot rendering since 7b.3; after 7b.4, slots are reshaped by the allocator)                                                                                                                                                                                                                                                                  |
| Sort struct fields      | Sort only when field names are known; construction order is semantic and must not be sorted                                                                                                                                                                                                                                                                                                                                    |

**The normalization tool’s output must be checked in** (`src/middle/ir/tests/snapshots/`),
explicitly updated whenever the IR definition changes, and manually reviewed when the diff is
updated.

**Known limitation (must be documented)**: Normalization erases information about “which register
the Nth argument used,” so the **second layer alone cannot cover defect class 2**. Safety of
`arg_regs` reordering during C1 must instead be guaranteed jointly by the first layer’s dominance
checks and the corpus behavioral differential.

#### Third Layer: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the transformation, then compare:

| Comparison target      | Normalization rule                                                                                                                        |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is excluded from comparison** (type-representation convergence may change wording) |
| Exit code              | Compare directly                                                                                                                          |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                     |
| `dump_bytecode` output | Compare only during C1/C2 (bytecode may change with the IR)                                                                               |

**Behavioral differential coverage during C4 (SSA conversion) must include**: the three ownership
transfers ref/borrow/move, closure captures, currying (`generate_curry_*`), spawn and iterator
`for`, sum-type pattern matching, existential coercion points, `?` / Try propagation, method
overloads (`overload_resolutions` span keying), and Drop sequences for refinement constraints
(`ReleasePlan`).

**Performance baseline**: A corpus differential guarantees “identical behavior,” but not “identical
elapsed time.” While establishing the differential baseline at P2, use the `criterion`
dev-dependency that is already present to create 2–3 smoke benchmarks (full-corpus compilation time
/ typical-program interpretation throughput / CLI cold-start time), check them in, and use them as
performance criteria for subsequent pipeline changes—removing slot reuse, expanding Phi into a chain
of Moves, or adding Driver indirection may all affect timing, and without a baseline no one will
catch such regressions.

### Specialized Criterion for Correctness Bugs

The bug that [`02`](02-stage-contract.md) is intended to fix requires an **independent criterion**
and must not be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets from the single-file and multi-file paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written to fail** (red), confirming that it catches the existing bug,
before the fix begins. If it unexpectedly passes, the bug analysis must be re-reviewed.

Companion criteria (accompanying [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                                                             | Defects caught                                                                                                                                   |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_program_stage_coverage`        | Every `Program::stages()` is a subset of `Stage::ALL` and equals the expected array                                                                                   | A stage omitted in the future                                                                                                                    |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must produce a diagnostic in every mode                                                                                                       | The three `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                                    |
| `test_release_plan_spans_consumed`   | The set of `PlanId`s produced by ownership is a subset of the set consumed during IR construction (**settled by structural identity, with no whitelist**)             | `ReleasePlan` span keys silently dropping Drop (historical form; after the key migration, the consumption ledger + `W1090` provide the backstop) |
| `test_overload_resolutions_consumed` | For each corpus file: produced overload resolutions are a subset of consumed resolutions, and the **evidence-surface floor** is `examined ≥ 1` / truly consumed `≥ 2` | Resolution entries being silently discarded (silently incorrect answers that fall back to the default binding)                                   |
| `test_obligations_drained`           | CI fails if any `Obligations` field has no consumption point anywhere in the repository                                                                               | This entire class of defects                                                                                                                     |

### Semantic-Correction Routing Criterion (D59)

RFC-039’s refactoring contract is **tiered equivalence (C1–C6) + a dedicated semantic-correction
channel** (P3 / P3.5 + decision registry): “There is no C5′” and “no relaxation of any kind”
constrain the **strength of the criteria**, not the requirement to preserve erroneous semantics
verbatim. There is only one **decidable entry criterion**:

> **Question: Is the old behavior consistent with the written specification?** (Criterion: Can you
> identify the RFC/SPEC section and provide a minimal counterexample?)

- **You can identify it**: The old behavior violates a clause—it failed to report something that
  should have been reported, failed to pass something that should have passed, or silently produced
  an incorrect result—and the counterexample is reproducible ⇒ **types A/B/C may be fixed alongside
  the refactoring**.
- **You cannot identify it** (the specification is silent or ambiguous, or the old behavior was
  already compliant) ⇒ **types D/E/F require user adjudication first**.

**“Success” does not mean that the compiler accepted the program**: A program that the compiler
incorrectly accepted is not a success—the acceptance itself is the defect that must be fixed.
Recording it as a success means protecting the defect with rules.

**All five A/B/C requirements are mandatory**:

1. **Red before green**—The red criterion must already exist and must currently be red (a self-check
   item in [HOWTO](../HOWTO.md)). “Green then red” does not qualify: without a red criterion, it is
   impossible afterward to distinguish a “fix” from a regression.
2. **Do not change the externally observable behavior of programs that originally complied with the
   specification**—A program that was originally compliant and ran correctly must not become a
   failure because of this change, and its existing diagnostics (code / message / exit code) must
   not change. It is **permitted** to make a path that originally violated the specification fail or
   become more conspicuous (no diagnostic → a new W/E code or increased severity is allowed)—that is
   precisely the correction itself.
3. **Register in the same commit**—Mark the error type (A/B/C) on the corresponding WBS line,
   **correct the affected body text in place** (the body describes only the current state; the
   before/after diff and red→green evidence are retained in the commit message or record), and
   include the “Error type” line in the required PR block.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshots / corpus
   differentials). The classification is **not** a reason to exempt the diff from review.
5. **Clause + counterexample**—The commit message and the PR’s “Error type” line must identify the
   **section** of the RFC/SPEC and provide the **minimal counterexample** for the behavior (a piece
   of compilable source or a command and its output). If this cannot be provided, the change must
   not be handled as A/B/C and must fall back to D/E/F.

| Type                                                   | Criterion                                                                             | Typical form                                                                                                                                       | Precedent                           |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A Silent failure**                                   | An error that should have been reported was swallowed                                 | Empty `Unproven` branch, silently missing solver, `unwrap_or_default` fallback                                                                     | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent mismatch**                                  | Key mismatch / choosing one of two parallel representations / span-keyed misalignment | Span key silently dropping Drop, synonymous concepts represented twice                                                                             | 3.4.5 / D20 / D57                   |
| **C Undefined behavior and uncontrolled resource use** | Crash, stack overflow, uninitialized slot, bytecode version mismatch                  | Pathological generic recursion (compile-time stack overflow), two hard-coded `.42` discard sites (the third is a false positive from a dead field) | 3.4.8 ①/② / D52                     |

**If no clause can be identified, or the old behavior was already compliant but the existing
behavior is to change, the change still requires user adjudication as D/E/F** (see “Mandatory When
Requesting User Adjudication” in the repository-root `AGENTS.md`):

| Type                         | Criterion                                                                                                                                 | Examples                  |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-visible semantics** | Externally observable additions, removals, or changes to diagnostic codes / externally visible error text / module semantics / exit codes | D54 / D55 / RFC-029g      |
| **E Public contract tables** | Changes to the error-code table, opcode table, type table, or entry-stage table themselves                                                | D1 / D6 / D12 / D20 / D23 |
| **F RFC design changes**     | Metatype universe, new E codes, wholesale removal of syntax                                                                               | D53 / RFC-033 / RFC-029g  |

**An unspecified specification ≠ a specification violation**: Whenever the specification is silent
or ambiguous, fall back to D/E/F—“I think the new behavior is more correct” is not an entry
criterion.

**Primary-error attribution**: Classify each change according to its **primary error type**;
incidental behavioral changes within the minimal fix set for the same root cause may be listed
separately on the PR’s “D3 patch determination” line (precedent: 3.4.8 ① has a primary type-C error
and three incidental asymmetric type-A defects).

**Relationship to C1/C2**: Changes of types A/B/C **must not** be declared C1 (byte-for-byte
invariance) or C2 (identical diagnostic sets)—the definitions of those two categories require
behavioral invariance. Incidental fixes must instead be declared under the corresponding category
among C3/C4/C6.

**Why not use a thematic whitelist**: [08](08-maintenance-mechanism.md) already states that “the
exemption count itself is included in the gate report”; the list will grow, and cases involving a
primary error plus an incidental fix inevitably fall on a boundary. The entry criterion is
Boolean-decidable.

### Regression Gates

Add the following CI checks, following the existing conventions in `scripts/ci/`:

| Check                      | Trigger                                                                    | Failure condition                                                                                |
| -------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `check-snapshot-drift.sh`  | C1/C3/C5 changes                                                           | Snapshot files have a diff, but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                             | `verify()` returns errors for the full corpus                                                    |
| `check-corpus-parity.py`   | C2/C4/C5 changes                                                           | The corpus differential is non-empty                                                             |
| `check-stage-contract.py`  | All changes                                                                | `Program::stages()` does not match `Stage::ALL`                                                  |
| `check-perf-regression.sh` | Changes affecting the compilation pipeline or execution path (P4/P6/P7/P8) | Performance regresses by >10% and the PR contains no explanation                                 |

### Binding to Companion Documents

| Document                                                                       | Must exist first                                                           | Criterion strength             |
| ------------------------------------------------------------------------------ | -------------------------------------------------------------------------- | ------------------------------ |
| [02-stage-contract.md](02-stage-contract.md) (staging)                         | The bug criterion from this document (red first)                           | C2: identical diagnostic sets  |
| [03-type-unification.md](03-type-unification.md) (type representation)         | The first layer’s type-consistency invariant                               | C3: identical diagnostic codes |
| [04-ssa.md](04-ssa.md) (SSA conversion)                                        | **All three layers**, and the verifier must first pass on the non-SSA form | C4: behavioral equivalence     |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generatorization) | AST snapshots + diagnostic criteria                                        | C5: use all three              |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)             | Reference scan                                                             | None                           |

## Key Decisions and Rationale

**Tiering rather than a single criterion.** Using “full equality of IR snapshots” as the criterion
for SSA conversion (C4) would fail immediately and would therefore tempt the team to relax the
criteria. Tiering lets each step use the strength it can support. Meanwhile, the first layer’s
dominance checks turn “errors that cannot be tested” into “testable invariants,” which snapshots
cannot do. The criteria assets are checked in and evolve with the code; they are not a one-time
investment.

### Rejected Approaches

- **Using only end-to-end corpus differentials**—Defect classes 1/2/3 are precisely those in which
  “the program still runs in some fashion,” and end-to-end differentials are insensitive to them.
  SSA conversion is especially dangerous: skipped register numbers may affect only performance
  rather than output, while a Drop instruction lost through a span mismatch **only manifests for
  particular inputs**, and the corpus may not cover them.
- **Using only IR snapshots**—Snapshots are inherently immune to defect class 2 (see
  “Canonicalization”) and fail completely during C4.
- **Using only property-based testing** (`proptest` is already a dependency)—Random programs have
  difficulty covering ownership, existential, and refinement paths that require specific shapes to
  be triggered. **It can, however, serve as a supplement**: for the first class (simple
  expressions/arithmetic), it is indeed better than fixed-corpus coverage. Introduce it as a
  supplementary C4 criterion after the corpus differential has stabilized.
- **Refactoring first and adding tests later**—This project has already fallen into that trap. The
  eight orphan test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1,005
  lines / 78 tests that have never run) are the result of “creating the directory first and then
  failing to wire it up.” Criteria must precede refactoring.

## Known Limitations and Risks

- **The first-layer verifier requires SSA lookahead information.** In non-SSA form, the concept of a
  “unique definition point” does not exist (`Operand::Local(n)` refers to a slot that may be written
  multiple times). The verifier therefore needs two modes: `verify_ssa` (strict, including Phi
  checks) and `verify_loose` (dominance and types only, allowing multiple definitions).
  **`verify_loose` must first pass on the existing IR**; this is the largest single work item in the
  design.
- **The second layer’s normalization misses defect class 2.** This is a known limitation and can
  only be covered jointly by the first and third layers; it cannot be solved by strengthening
  normalization.
- **The third layer’s corpus differential is sensitive to nondeterministic output.** If any `.yx`
  corpus case produces output that depends on hash iteration order or time, the differential will
  fail spuriously. Such cases must be identified and filtered when the baseline is established.
- **Checking in snapshots creates maintenance overhead.** Snapshots must be explicitly updated
  whenever the IR definition changes, which requires review discipline.
- **The corpus covers only the single-file path.** There are no `yaoxiang.toml` files under
  `tests/`, and all 293 corpus cases use the single-file path—see the limitation under “Existing
  Test Assets.”

> **All open questions originally listed in this section have now been adjudicated.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **No pending items remain in this document.**

## See Also

- [RFC-039 Compiler Feature Routing Directory Design](../../rfc/accepted/039-compiler-architecture.md)
  — Governing general framework
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, an
  existing “item-by-item round-trip” criterion in this repository
- `src/middle/core/tests/bytecode.rs:769-814` — A sentinel test that uses `include_str!` to read
  source code for a static differential comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — Bidirectional set-difference assertions + snapshot
  table, forcing an update whenever a new frontend is integrated
- `ir_gen.rs:248-249` — Explicit statement of the physical-adjacency contract for save/restore
  points
