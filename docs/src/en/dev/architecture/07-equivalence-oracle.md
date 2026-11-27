# Refactoring Equivalence Criteria

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution phase order are covered in the main
> RFC-039 text; see the [Index for this directory](index.md) for the positioning of each companion
> document.

## Positioning and Scope

This document defines the criteria for “how to prove that behavior has not changed” during
refactoring. The criteria are **tiered by refactoring-step category** because different categories
require different strengths of equivalence: pure moves require byte-for-byte identity, whereas SSA
conversion necessarily changes the IR shape and can establish only behavioral equivalence.

**This document is a prerequisite for all refactoring.** Without criteria,
[SSA conversion](04-ssa.md) cannot be implemented safely.

This document **does not cover**: the stage contracts themselves (see `02`), the design of
type-representation convergence (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The most dangerous code currently happens to have zero test coverage

`src/middle/core/ir_gen.rs` (8,448 lines):

- The file contains **no `#[cfg(test)]` and no `mod tests`**
- Of the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively pin down
  the sentinel behavior of DefId allocation and slot naming; all four assertions in `bytecode.rs`
  that mention ir_gen are **negative assertions** (“ir_gen does not construct a certain
  instruction”)
- In other words, **IR structural invariants (dominance, unique definitions, jump targets, and type
  consistency) currently have zero validation**—precisely the gap that the first tier of criteria
  must fill

This file also contains exactly three classes of defects that **produce no errors when they fail**:

**Defect class 1: Register misalignment.** The file contains six manually written save/restore
statement pairs, separated by more than 70 lines:

| Save point            | Restore point | Containing function         |
| --------------------- | ------------- | --------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`        |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`      |
| `2116`                | `2180`        | curry intermediate function |
| `2224-2229`           | `2284-2288`   | innermost curry function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`  |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`   |

The file itself states this contract at `ir_gen.rs:248-249`: “**The save/restore point must be
physically adjacent to that of `next_temp`; otherwise, inner-function names will leak into the outer
function**.”

The consequence of missing a restore is: `next_temp` leaks and temporary-value numbering skips
values. **The IR remains internally consistent, still passes all checks, and still compiles; only
the values are wrong.** Ordinary assertion tests cannot catch it.

**Defect class 2: Misalignment caused by semantic reordering of `arg_regs`.** The six branches
inside `generate_call_expr_ir` (`7269-8014`, 747 lines) share the same `Vec<Operand>` and rewrite it
semantically:

| Line        | Reordering semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named-argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named-argument reordering  |

Default-value registers are also appended along the way. **Misalignment produces no error, only
incorrect values.**

This class of error is even more dangerous because it **may also go undetected under a normalized
snapshot**—if the snapshot tool normalizes temporary register numbers, as is standard practice, then
an error such as “the third argument should use the fifth register” produces a **completely
identical** snapshot.

**Defect class 3 (eliminated): Silent failure of a span-keyed cross-layer contract.** `ReleasePlan`
formerly used `Span` as its key (produced at `layers/ownership.rs:31`), passed it through
`checker.rs:1440`, and ultimately matched it via `self.release_plan.get(&stmt.span)` in `ir_gen` to
emit `Instruction::Drop`; any discrepancy in how the two sides compute spans → **the Drop
instruction silently disappears, with no error**. `overload_resolutions` has the same shape.
**2026-10-13 (RFC-039 D20 / WBS 4.5)**: both contracts were changed to `PlanId`
**structural-identity keys** (the sole consumption entry points are `ReleasePlan::consume` /
`OverloadResolutions::get`, with reading and accounting performed in the same call; a non-empty set
difference is level-E `W1090`). This class of defects is now zero; see [02](02-stage-contract.md) §2
revision note for the implemented form and criteria.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> with the module-unqualified key `"Type.method"`). Its risk is different (definition-order or
> duplicate-name mismatch rather than span mismatch), and SSA conversion does not address it (see
> [04](04-ssa.md), Mechanism Three). Therefore, there are **2** span-keyed contracts (`ReleasePlan`
> / `overload_resolutions`).### Existing Test Assets

The good news is that there is sufficient raw material for the criteria:

| Asset                               | Scale                                                                                                                                                             | Can be used for                                                                                          |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpus files, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see the limitation below | End-to-end behavioral differential (**single-file side only**)                                           |
| `src/std/tests/*.yx`                | A batch of files                                                                                                                                                  | Standard-library behavioral differential                                                                 |
| `tests/integration/`                | 18 modules / 5,846 lines                                                                                                                                          | CLI end-to-end tests (of which the 27 tests in `multifile.rs` are the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1,131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, round-tripping every opcode)                                | Bytecode-layer round trips                                                                               |

> **⚠️ Structural limitation of corpus coverage (added 2026-10-03)**
>
> There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (measured count: 0).
> Therefore, all 293 corpus files take the `standalone` branch of `util/diagnostic/mod.rs:565`
> `check_files_with_diagnostics` → `check_single_file` → **`Pipeline::run` (the single-file path)**.
>
> **Consequence**: The third tier of criteria can **verify only changes to the single-file path**.
> There is currently **no corpus coverage** for the multi-file side of [`02`](02-stage-contract.md)
> (`orchestrator`’s `check_project` / `compile_project` / `check_source_in_project` /
> `compile_embedded_module`) or for the multi-file behavior of [`03`](03-type-unification.md) /
> [`04`](04-ssa.md).
>
> **A multi-file corpus tier must be established in P2**; otherwise, the largest refactoring phase,
> the “Unified Driver,” has no executable criterion. The 27 tests in
> `tests/integration/multifile.rs` are the only existing multi-file-side coverage, but they test
> semantic results rather than compilation-stage contracts.

**All these assets share a common defect**: they test “input → output,” not “whether the
intermediate representation is correct.” IR-layer correctness currently has **no criterion at all**.

## Target Design

### Core Principle: Criteria Are Tiered by Refactoring Category

**This is the most important point in this document.** Using a single criterion (usually “full IR
snapshot equality”) for every kind of refactoring is wrong—some refactors **necessarily change the
IR shape**.

| Refactoring category                                                            | Criterion type                                                                       | Strength  | Related document                              |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | --------- | --------------------------------------------- |
| **C1 Pure move** (splitting files, changing directories, extracting submodules) | Normalized IR snapshot **zero-diff**                                                 | Strongest | Checker split in `02`, lowering split in `04` |
| **C2 Orchestration change** (staging, unified Driver)                           | Same **diagnostic set** at each entry point + same corpus behavior                   | Strong    | `02`                                          |
| **C3 Type-representation convergence**                                          | Same diagnostic **codes and messages** (order may be normalized)                     | Strong    | `03`                                          |
| **C4 IR-shape change** (SSA conversion)                                         | **Behavioral equivalence** (same execution results) + IR structural invariants       | Medium    | `04`                                          |
| **C5 Frontend-paradigm change** (grammar-driven)                                | Same AST snapshot + **identical diagnostic code+span, item by item** + same behavior | Strong    | `05`                                          |
| **C6 Pure deletion**                                                            | No equivalence criterion; only confirm that there are no references                  | —         | `06`                                          |

**The distinction between C1 and C4 is critical**: C1 requires byte-for-byte identical IR (any
change is a regression), whereas C4 acknowledges that the IR will change and instead requires “same
program behavior + IR satisfies the invariants.”

> **⚠️ C5′ explicitly does not exist**
>
> Diagnostic requirements in the error-recovery dimension are **not relaxed**: the
> synchronization-point set of `ParserState::synchronize()` and the `Expr::Error` /
> `StmtKind::Error` placeholders jointly determine each diagnostic’s code and span. Changes to the
> syntactic surface or precedence table that touch these contracts must preserve them item by item.
> Failure to do so is an implementation defect and **does not justify weakening the criteria**.
>
> If testing reveals that a change cannot preserve the existing diagnostics, the correct action is
> to **report the finding honestly and reassess the design** (including “retain the status quo”),
> **not to modify the criteria**.

### Three Tiers of Criteria

#### First Tier: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is much
stronger than a snapshot because it checks **semantic invariants**, not a concrete shape.

| Invariant                                                                          | Defects caught                                                                                                                      | Typical cause                                               |
| ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| **Use-before-def dominance**                                                       | Every read of `Operand::Local(n)` must have a definition path on the CFG that dominates the current point                           | Register misalignment, missing restore                      |
| **Unique value definition** (only in SSA form)                                     | Every SSA value must have exactly one definition point; every join point must have a `Phi`                                          | Errors in SSA conversion itself                             |
| **Phi consistency**                                                                | The number of `Phi` inputs must equal the number of corresponding predecessor basic blocks, and every input type must be consistent | Missing predecessor, type mismatch                          |
| **Jump target exists**                                                             | The label of every jump must exist in `blocks`                                                                                      | Rebasing errors in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot in bounds**                                                          | `i < globals.len()` for `Operand::Global(i)`                                                                                        | Slot-allocation errors                                      |
| **Type consistency**                                                               | The `MonoType` of an instruction’s `dst` and `src` must be compatible                                                               | Type-representation bridging errors                         |
| **Inner isolation** (corresponding to the explicit promise at `ir_gen.rs:248-249`) | Function A’s `cur_locals` must not contain a temporary name from function B                                                         | The six save/restore contracts                              |

**The verifier must exist before SSA conversion** and must **first pass on the current (non-SSA)
IR**—otherwise, it cannot be used to prove equivalence before and after the transformation.

#### Second Tier: Normalized IR Snapshots (Covers C1/C3/C5)

IR snapshots must be **normalized**; otherwise, any change in register numbering causes the full
diff to fail:

| Normalization item      | Rule                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Strip `Span`            | Set all span fields to sentinel values. **This only actually took effect starting with WBS 6.6.12**: previously, the `type_decl` line printed the Debug text of the type-declaration carrier’s `ast::Type`, and **100% of span leakage in measured IR snapshots came from that line** (96 lines / 69 files). After the carrier was replaced with an IR-native narrowed shape, spans no longer appear anywhere in the snapshots |
| Rename temporary values | Rename them to `%0 %1 %2 ...` in order of first appearance, **preserving definition-use topological order**                                                                                                                                                                                                                                                                                                                    |
| Relativize global slots | Renumber them in order of first appearance within the module                                                                                                                                                                                                                                                                                                                                                                   |
| Sort predecessors       | Sort `Phi` inputs by predecessor label (`successors` has not appeared in snapshot rendering since 7b.3; after 7b.4, slots are reshaped by the allocator)                                                                                                                                                                                                                                                                       |
| Sort struct fields      | Sort only when field names are known (construction order is semantic and must not be sorted)                                                                                                                                                                                                                                                                                                                                   |

**The normalization tool’s output must be checked into the repository**
(`src/middle/ir/tests/snapshots/`), updated explicitly whenever the IR definitions change, and its
diff manually reviewed on each update.

**Known limitation (must be stated explicitly)**: normalization erases the information about “which
register the Nth argument used,” so **the second tier alone is insufficient to cover defect class
2**. During C1, the safety of `arg_regs` reordering must rely jointly on first-tier dominance
checking and corpus behavioral differentials.

#### Third Tier: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the change, then compare:

| Comparison target      | Normalization rule                                                                                                                        |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is excluded from comparison** (type-representation convergence may change wording) |
| Exit code              | Compare directly                                                                                                                          |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                     |
| `dump_bytecode` output | Compare only during C1/C2 (bytecode may change with the IR)                                                                               |

The behavioral differential for **C4 (SSA conversion)** must cover: all three ownership
transfers—ref/borrow/move—closure capture, currying (`generate_curry_*`), spawn and iterator for,
sum-type pattern matching, existential coercion points, `?` / Try propagation, method overloading
(`overload_resolutions` span-keyed), and Drop sequences for refinement constraints (`ReleasePlan`).

**Performance baseline**: A corpus differential guarantees “same behavior,” not “same elapsed time.”
While establishing the differential baseline in P2, use `criterion`, which is already present in
dev-dependencies, to build and check in 2–3 smoke benchmarks (compile time for the entire corpus /
interpretation throughput for a typical program / CLI cold startup) as the performance criterion for
subsequent pipeline changes. Removing slot reuse, expanding a Phi into a chain of Moves, and adding
a Driver indirection layer can all change elapsed time; without a baseline, no one will catch the
regression.

### Dedicated Criterion for Correctness Bugs

The bug that [`02`](02-stage-contract.md) is intended to fix requires an **independent criterion**
and must not be mixed into the three tiers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets of the single-file and multi-file paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written to fail** (red), confirming that it catches the existing bug,
before the fix begins. If it unexpectedly passes, the vulnerability analysis must be re-reviewed.

Companion criteria (paired with [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                                    | Defect prevented                                                                                                                                 |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_program_stage_coverage`        | Every `Program::stages()` ⊆ `Stage::ALL`, and the result equals the expected array                                                           | A stage omitted from the wiring in the future                                                                                                    |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must produce a diagnostic in every mode                                                                              | The three `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                                    |
| `test_release_plan_spans_consumed`   | The set of `PlanId`s produced by ownership ⊆ the set consumed during IR construction (**settled by structural identity, with no whitelist**) | Silent loss of Drop due to span-keyed `ReleasePlan` behavior (historical form; after key migration, covered by the consumption ledger + `W1090`) |
| `test_overload_resolutions_consumed` | Corpus, file by file: overload-resolution outputs ⊆ consumed outputs, and the **evidence floor** is `examined ≥ 1` / actually consumed `≥ 2` | Silent dropping of resolution entries (silently incorrect answers caused by fallback to the default binding)                                     |
| `test_obligations_drained`           | CI must fail if any `Obligations` field has no consumption point anywhere in the repository                                                  | This entire class of defects                                                                                                                     |

### Routing Criteria for Semantic Corrections (D59)

The refactoring contract in RFC-039 is **tiered equivalence (C1–C6) plus a dedicated
semantic-correction channel** (P3 / P3.5 + decision registry). “No C5′” and “no relaxation of any
kind” constrain the **strength of the criteria**; they do not require erroneous semantics to be
preserved as-is. The routing boundary is defined by one **decidable entry criterion**:

> **Question: Does the old behavior agree with the explicitly written specification?** (The
> criterion is whether an RFC/SPEC section can be cited and a minimal counterexample supplied.)

- **A section can be cited**: The old behavior violates the clause—it fails to report an error that
  should be reported / fails to accept a program that should pass / silently produces an incorrect
  result—and the counterexample is reproducible ⇒ **types A/B/C may be fixed alongside the
  refactoring.**
- **No section can be cited** (the specification is silent, is ambiguous, or the old behavior was
  already compliant) ⇒ **types D/E/F require user adjudication first.**

**“Success” is not determined by whether the compiler accepts the program**: A program that the
compiler incorrectly accepts is not a success—the acceptance itself is the defect that must be
fixed. Recording it as success amounts to allowing the rules to protect the defect.

**All five A/B/C requirements are mandatory**:

1. **Red before green**—The red criterion must exist first and must currently be red
   ([HOWTO](../HOWTO.md) self-check item). “Green then red” does not qualify: without a red
   criterion, there is no way after the fact to distinguish a “fix” from a regression.
2. **Do not change the externally observable behavior of programs that previously complied with the
   specification**—A program that was previously compliant and could run correctly must not become a
   failure because of this change, and its existing diagnostics (code / text / exit code) must not
   change. It **is permitted** for a path that previously violated the specification to fail or
   become more conspicuous (previously no diagnostic → newly adding a W/E code or escalating its
   level)—that is the correction itself.
3. **Register it in the same commit**—Mark the error type (A/B/C) on the corresponding WBS row and
   **correct the affected body text in place** (the body describes only the current state; retain
   the pre-/post-correction diff and red→green evidence in the commit message or recording) +
   include the required “Error type” line in the PR block.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshots / corpus
   differential). Classification is **not** grounds for exempting this review.
5. **Clause + counterexample**—The commit message and the PR’s “Error type” line must state the
   RFC/SPEC **section number** and a **minimal counterexample** for the behavior (a compilable
   source snippet or a command and its output). If this cannot be provided ⇒ the change must not be
   handled as A/B/C and must fall back to D/E/F.

| Type                                       | Criterion                                                                             | Typical form                                                                                       | Precedent                               |
| ------------------------------------------ | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------- |
| **A Silent failure**                       | An error that should have been reported is swallowed                                  | Empty `Unproven` arm, silent solver omission, `unwrap_or_default` fallback                         | 3.3 / 3.4.1 (`W1063`) / 3.4.2 (`W1081`) |
| **B Silent mismatch**                      | Key mismatch / choosing one of two parallel representations / span-keyed misalignment | Span key silently loses Drop, two representations of the same concept                              | 3.4.5 / D20 / D57                       |
| **C Undefined behavior and resource loss** | Crash, stack overflow, uninitialized slot, bytecode-version mismatch                  | Pathological generic recursion (compile-time stack overflow), three hard-coded `.42` discard sites | 3.4.8 ①/② / D52                         |

**If no clause can be cited, or the old behavior was already compliant and the existing behavior is
to change → types D/E/F still require user adjudication first** (see “When requesting user
adjudication (mandatory)” in the repository-root `AGENTS.md`):

| Type                           | Criterion                                                                                                                      | Examples                  |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------- |
| **D User-visible semantics**   | Externally visible surface of diagnostic-code additions/removals/changes, external error text, module semantics, or exit codes | D54 / D55 / RFC-029g      |
| **E External contract tables** | The error-code table, opcode table, type table, or entry-stage table itself                                                    | D1 / D6 / D12 / D20 / D23 |
| **F RFC design change**        | Meta-type universe, new E codes, wholesale removal of syntax                                                                   | D53 / RFC-033 / RFC-029g  |

**An unspecified specification ≠ a specification violation**: Whenever the specification is silent
or ambiguous, always fall back to D/E/F—“I think this is more correct” is not an entry criterion.

**Primary-error attribution**: Classify a change according to its **primary error type**; incidental
behavior changes bundled into the minimal change set for the same root cause may be listed
separately on the PR’s “D3 Patch Determination” line (precedent: 3.4.8 ① has a primary type-C error,
with three incidental type-A asymmetric defects).

**Relationship to C1/C2**: A/B/C changes **must not be declared as C1** (byte-for-byte invariance)
or **C2** (identical diagnostic sets)—the definitions of those categories require unchanged
behavior. A correction made alongside the refactoring must be declared under the corresponding
category among C3/C4/C6.

**Why not use a thematic whitelist**: [08](08-maintenance-mechanism.md) already states that “the
exemption count itself enters the gate report”; the list will inevitably grow, and cases of “one
primary type + incidental fixes” will always fall on a boundary. The entry criterion is boolean and
decidable.

### Regression Gates

Add the following CI checks (following the existing conventions in `scripts/ci/`):

| Check                      | Trigger                                                                   | Failure condition                                                                               |
| -------------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | C1/C3/C5 changes                                                          | The snapshot files differ, but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` reports an error over the full corpus                                                |
| `check-corpus-parity.py`   | C2/C4/C5 changes                                                          | The corpus differential is non-empty                                                            |
| `check-stage-contract.py`  | All changes                                                               | `Program::stages()` does not match `Stage::ALL`                                                 |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | The performance baseline regresses by >10%, and the PR contains no explanation                  |

### Binding to Each Companion Document

| Document                                                                           | Must have first                                                           | Criterion strength             |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------ |
| [02-stage-contract.md](02-stage-contract.md) (staging)                             | The vulnerability criterion from this document (red first)                | C2: identical diagnostic sets  |
| [03-type-unification.md](03-type-unification.md) (type representation)             | The first-tier verifier’s type-consistency invariant                      | C3: identical diagnostic codes |
| [04-ssa.md](04-ssa.md) (SSA conversion)                                            | **All three tiers**, and the verifier must first pass on the non-SSA form | C4: behavioral equivalence     |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generator conversion) | AST snapshot + diagnostic criteria                                        | C5: all three                  |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)                 | Reference scan                                                            | None                           |

## Key Decisions and Rationale

**Tiered rather than a single criterion.** Using “full IR snapshot equality” as the criterion for
SSA conversion (C4) would fail immediately, tempting the team to weaken the criterion; tiering lets
each step use the strength it can sustain. At the same time, the first-tier verifier covers
dominance checks, turning “errors that cannot be tested” into “testable invariants”—something
snapshots cannot do. The criterion assets are checked into the repository and evolve with the code;
they are not a one-time investment.

### Rejected Approaches

- **End-to-end corpus differential only**—defect classes 1/2/3 are precisely those in which “the
  program still ran somehow,” and end-to-end differentials are insensitive to them. SSA conversion
  is especially dangerous: register-number skips may affect only performance rather than output,
  while a span mismatch that causes a Drop to be lost **only manifests for particular inputs**, and
  the corpus may not cover them.
- **IR snapshots only**—snapshots are inherently immune to defect class 2 (see the “Normalization”
  section) and fail completely during C4.
- **Property-based testing only** (`proptest` is already a dependency)—random programs struggle to
  cover paths that require specific shapes involving ownership, existentials, and refinement.
  **However, it can serve as a supplement**: for the first class (simple expressions/arithmetic), it
  is genuinely better than fixed-corpus coverage. It is recommended that it be introduced as a
  supplementary criterion for C4 after the corpus differential has stabilized.
- **Refactor first, add tests later**—this is a pitfall the project has already encountered. The 8
  orphan test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1,005 lines /
  78 tests that never ran) are the result of “creating the directory first, then failing to wire it
  up.” The criteria must precede the refactoring.

## Known Limitations and Risks

- **The first-tier verifier needs SSA-oriented lookahead information.** In non-SSA form, the concept
  of a “unique definition point” does not exist (`Operand::Local(n)` refers to a slot that may be
  written multiple times). The verifier therefore needs two modes: `verify_ssa` (strict, including
  Phi checks) and `verify_loose` (dominance and types only, allowing multiple definitions).
  **`verify_loose` must first pass on the existing IR**; this is the largest single chunk of work in
  the design.
- **The second tier’s normalization misses defect class 2.** This is a known limitation and can only
  be addressed jointly by the first and third tiers; it cannot be solved by strengthening
  normalization.
- **The third tier’s corpus differential is sensitive to nondeterministic output.** If any `.yx`
  corpus file produces output that depends on hash iteration order or time, the differential will
  fail spuriously. Such corpus files must be filtered out when establishing the baseline.
- **Checking snapshots into the repository creates a maintenance cost.** Whenever the IR definitions
  change, the snapshots must be updated explicitly, which requires review discipline.
- **The corpus covers only the single-file path.** There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus files take the single-file path—see the limitation described in “Existing Test
  Assets.”

> **All open questions originally listed in this section have now been adjudicated.** The individual
> decisions are recorded in the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **No
> pending items remain in this document.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — the overarching specification
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, the
  repository’s existing exemplar of an “item-by-item round-trip” criterion
- `src/middle/core/tests/bytecode.rs:769-814` — a sentinel test that uses `include_str!` to read
  source code for static comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional set-difference assertions + a
  snapshot table, requiring updates whenever a new frontend is integrated
- `ir_gen.rs:248-249` — the explicit statement of the physical-adjacency save/restore contract

</result>
