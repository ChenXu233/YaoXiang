# Equivalence Criteria for Refactoring

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, grading of acceptance criteria, and execution phase ordering are covered in the
> main RFC-039 text; the role of each companion document is described in the
> [index for this directory](index.md).

## Purpose and Scope

This document defines the criteria for demonstrating that behavior remains unchanged during the
refactoring process. The criteria are **graded according to the category of the refactoring step**,
because different categories require different strengths of equivalence: a pure relocation requires
byte-for-byte identity, whereas SSA conversion necessarily changes the shape of the IR and can only
be judged behaviorally equivalent.

**This document is a prerequisite for the entire refactoring.** Without these criteria,
[SSA conversion](04-ssa.md) cannot be implemented safely.

This document **does not cover**: the stage contracts themselves (see `02`), the design for
converging type representations (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The most dangerous code is exactly the code with zero test coverage

`src/middle/core/ir_gen.rs` (8,448 lines):

- The file contains **neither `#[cfg(test)]` nor `mod tests`**
- Of the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively pin down
  the sentinel behavior of DefId allocation and slot naming; all four assertions in `bytecode.rs`
  that mention ir_gen are **inverse assertions** (“ir_gen does not construct a particular
  instruction”)
- In other words: **IR structural invariants (dominance, unique definitions, jump targets, and type
  consistency) currently have zero validation**—which is exactly what the first tier of criteria is
  intended to fill

This file also happens to contain three classes of defects that **produce no error when they fail**:

**Defect Class 1: Register misalignment.** The file contains six manually written pairs of
save/restore statements, separated by more than 70 lines:

| Save point            | Restore point | Enclosing function            |
| --------------------- | ------------- | ----------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`          |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`        |
| `2116`                | `2180`        | Intermediate curried function |
| `2224-2229`           | `2284-2288`   | Innermost curried function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`    |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`     |

The file itself states this contract at `ir_gen.rs:248-249`: “**The save/restore points for
`next_temp` must be physically adjacent; otherwise, names from the inner function will leak into the
outer function**.”

The consequence of missing one restore is: `next_temp` leaks and temporary-value numbers skip ahead.
**The IR remains internally consistent, still passes all checks, and still compiles; it merely
produces wrong values.** Ordinary assertion tests cannot catch this.

**Defect Class 2: Misalignment caused by semantic reordering of `arg_regs`.** Inside
`generate_call_expr_ir` (`7269-8014`, 747 lines), six branches share the same `Vec<Operand>` and
rewrite it semantically:

| Line        | Reordering semantics                    |
| ----------- | --------------------------------------- |
| `7395`      | Namespace call: reorder named arguments |
| `7724`      | Struct construction: reorder fields     |
| `7835-7848` | Function call: reorder named arguments  |

Default-value registers are also appended along the way. **Misalignment produces no error, only
incorrect values.**

This class of error is even more dangerous because it **may also go undetected under “normalized
snapshots”**—if the snapshot tool normalizes temporary register numbers away, as is conventional,
then an error such as “the third argument should have used the fifth register” is **completely
identical** in the snapshot.

**Defect Class 3 (Eliminated): Silent failure of a span-keyed cross-layer contract.** `ReleasePlan`
previously used `Span` as its key (produced by `layers/ownership.rs:31`), passed it through
`checker.rs:1440`, and ultimately matched it through `self.release_plan.get(&stmt.span)` in `ir_gen`
to emit `Instruction::Drop`; if the two sides computed spans differently in any respect, the **Drop
instruction would silently disappear without any error**. `overload_resolutions` has the same form.
**2026-10-13 (RFC-039 D20 / WBS 4.5)**: both contracts were changed to use `PlanId`
**structural-identity keys** (the unique consumption entry points `ReleasePlan::consume` /
`OverloadResolutions::get`, with the read and accounting performed in the same call; a non-empty
difference triggers E-level `W1090`), reducing this class of defects to zero—the implemented form
and criteria are described in the revision note in [02](02-stage-contract.md) §2.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> with keys being module-unqualified `"Type.method"` strings). Its risk profile differs
> (definition-order/duplicate-name mismatch rather than span mismatch), and SSA conversion does not
> address it (see [04](04-ssa.md), Mechanism 3 for its disposition). Therefore, there are **2**
> span-keyed contracts (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that the raw material for the criteria is sufficient:

| Asset                               | Scale                                                                                                                                                             | Can be used for                                                                                                |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpus cases, numbered by specification chapter (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All are single-file paths**—see the limitation below | End-to-end behavioral differential (**single-file path only**)                                                 |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                           | Standard-library behavioral differential                                                                       |
| `tests/integration/`                | 18 modules / 5,846 lines                                                                                                                                          | CLI end-to-end testing (of which the 27 tests in `multifile.rs` provide the **only** multi-file-path coverage) |
| `src/middle/core/tests/bytecode.rs` | 1,131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, round-tripping every opcode)                                | Bytecode-tier round-trip testing                                                                               |

> **⚠️ Structural limitation of corpus coverage (added 2026-10-03)**
>
> There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (measured: 0). Consequently,
> all 293 corpus cases take the `standalone` branch of `util/diagnostic/mod.rs:565`
> `check_files_with_diagnostics` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence**: The third tier of criteria can **only validate changes to the single-file path**.
> There is currently **no corpus coverage** for the multi-file side of [02](02-stage-contract.md)
> (`orchestrator`'s `check_project` / `compile_project` / `check_source_in_project` /
> `compile_embedded_module`) or the multi-file behavior of [03](03-type-unification.md) /
> [04](04-ssa.md).
>
> **A multi-file corpus tier must be established in P2**; otherwise, the largest refactoring phase,
> “Unified Driver,” has no executable criteria. The 27 tests in `tests/integration/multifile.rs` are
> the only existing multi-file-path coverage, but they test semantic results rather than compiler
> stage contracts.

**However, these assets share a common defect**: they test “input → output,” not whether the
intermediate representation is correct. The IR tier currently has **no criteria whatsoever**.

## Target Design

### Core Principle: Criteria Are Graded by Refactoring Category

**This is the single most important point in this document.** Using a single criterion (usually
“full equality of IR snapshots”) for every kind of refactoring is incorrect, because some
refactorings **necessarily change the shape of the IR**.

| Refactoring category                                                                  | Criterion type                                                                         | Strength  | Related document                              |
| ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | --------- | --------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | IR normalized snapshot **zero-diff**                                                   | Strongest | Checker split in `02`; lowering split in `04` |
| **C2 Orchestration changes** (staging, unified Driver)                                | Same **diagnostic set** at every entry point + same corpus behavior                    | Strong    | `02`                                          |
| **C3 Type representation convergence**                                                | Same **diagnostic codes and messages** (order may be normalized)                       | Strong    | `03`                                          |
| **C4 IR shape changes** (SSA conversion)                                              | **Behavioral equivalence** (same execution results) + IR structural invariants         | Medium    | `04`                                          |
| **C5 Frontend paradigm changes** (grammar-driven)                                     | Same AST snapshot + **identical diagnostic code+span for every entry** + same behavior | Strong    | `05`                                          |
| **C6 Pure deletion**                                                                  | No equivalence criterion required; only confirmation that no references remain         | —         | `06`                                          |

**The distinction between C1 and C4 is critical**: C1 requires byte-for-byte identical IR (any
change is a regression); C4 acknowledges that the IR will change and instead requires “same program
behavior + IR satisfies the invariants.”

> **⚠️ C5′ explicitly does not exist**
>
> Diagnostic requirements in the error-recovery dimension are **not relaxed**: the set of
> synchronization points in `ParserState::synchronize()` and the `Expr::Error` / `StmtKind::Error`
> placeholders jointly determine each diagnostic's code and span. When changes to the syntax surface
> or precedence table touch these contracts, every diagnostic must remain identical item by item.
> Failure to do so is an implementation defect and **does not justify relaxing the criteria**.
>
> If testing reveals that a change cannot preserve the existing diagnostics, the correct action is
> to **report the finding truthfully and reassess the approach** (including “maintain the status
> quo”), **not to modify the criteria**.

### Three Tiers of Criteria

#### Tier 1: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is much
stronger than snapshots because it checks **semantic invariants**, not a concrete shape.

| Invariant                                                                                    | Defects caught                                                                                                                | Typical cause                                                 |
| -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **Use-before-def dominance**                                                                 | Every read of an `Operand::Local(n)` must have a definition path that dominates the current point in the CFG                  | Register misalignment, omitted restore                        |
| **Unique value definition** (only in SSA form)                                               | Every SSA value has exactly one definition point; every join point must have a `Phi`                                          | Incorrect SSA conversion                                      |
| **Phi consistency**                                                                          | The number of `Phi` inputs equals the number of corresponding predecessor basic blocks, and every input has a consistent type | Missing predecessor, type mismatch                            |
| **Jump target exists**                                                                       | Every jump label exists in `blocks`                                                                                           | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot within bounds**                                                                | `Operand::Global(i)` satisfies `i < globals.len()`                                                                            | Slot allocation error                                         |
| **Type consistency**                                                                         | The `MonoType` of an instruction's `dst` and `src` is compatible                                                              | Bridged type-representation error                             |
| **Inner-function isolation** (corresponding to the explicit contract at `ir_gen.rs:248-249`) | Function A's `cur_locals` does not contain a temporary name from function B                                                   | The six save/restore contracts                                |

**The verifier must exist before SSA conversion**, and it must **first pass on the current (non-SSA)
IR**; otherwise, it cannot be used to prove equivalence before and after the transformation.

#### Tier 2: Normalized IR Snapshots (Covers C1/C3/C5)

IR snapshots **must be normalized**; otherwise, any change to a register number will cause the full
diff to fail:

| Normalization item         | Rule                                                                                                                                                                                                                                                                                                                                                                                                                   |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Strip `Span`               | Set every span field to a sentinel value. **This requirement only truly took effect with WBS 6.6.12**: previously, the `type_decl` line printed the Debug text of the type declaration carrier, `ast::Type`, and **100% of span leakage in IR snapshots came from that line** (96 lines / 69 files); after the carrier was replaced with a natively narrowed IR shape, spans no longer appear anywhere in the snapshot |
| Rename temporary values    | Rename to `%0 %1 %2 ...` in order of first appearance, **preserving definition-use topology**                                                                                                                                                                                                                                                                                                                          |
| Make global slots relative | Renumber according to first appearance within the module                                                                                                                                                                                                                                                                                                                                                               |
| Sort predecessors          | Sort `Phi` inputs by predecessor label (`successors` has not been included in snapshot rendering since 7b.3; after 7b.4, slots are reshaped by the allocator)                                                                                                                                                                                                                                                          |
| Sort struct fields         | Sort only when field names are known (construction order is semantic and must not be sorted)                                                                                                                                                                                                                                                                                                                           |

**The normalization tool's output must be checked in** (`src/middle/ir/tests/snapshots/`),
explicitly updated whenever the IR definition changes, and reviewed manually when the diff is
updated.

**Known limitation (must be stated explicitly)**: normalization erases the information about “which
register the Nth argument used,” so **Tier 2 alone is insufficient to cover Defect Class 2**. During
C1, the safety of `arg_regs` reordering must be guaranteed jointly by Tier 1's dominance checks and
the corpus behavioral differential.

#### Tier 3: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the changes, and compare:

| Comparison target      | Normalization rule                                                                                                            |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is not compared** (type representation convergence may change wording) |
| Exit code              | Compare directly                                                                                                              |
| Runtime stdout/stderr  | Compare after sorting                                                                                                         |
| `dump_bytecode` output | Compare only during C1/C2 (bytecode may change with the IR)                                                                   |

**The behavioral differential during C4 (SSA conversion) must cover**: the three ownership transfers
ref/borrow/move, closure capture, currying (`generate_curry_*`), spawn and iterator for, union-type
pattern matching, existential forced points, `?` / Try propagation, method overloads
(`overload_resolutions` span keying), and Drop sequences for refinement constraints (`ReleasePlan`).

**Performance baseline**: A corpus differential guarantees “same behavior,” but not “same elapsed
time.” In P2, while establishing the differential baseline, use the existing `criterion`
dev-dependency to add and check in 2–3 smoke benchmarks (full-corpus compilation time /
interpretation throughput for a typical program / CLI cold-start time) as performance criteria for
subsequent pipeline changes—removing slot reuse, expanding `Phi` into Move chains, or adding a
Driver indirection layer can all affect elapsed time, and without a baseline nobody will catch the
regression.

### Dedicated Criteria for Correctness Vulnerabilities

The vulnerability to be fixed in [`02`](02-stage-contract.md) requires an **independent criterion**
and must not be blended into the three tiers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // The same source containing x: Sorted(3) must produce the same diagnostic set through the single-file and multi-file paths.
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written as failing** (red), confirming that it can catch the existing
vulnerability, before the fix begins. If it unexpectedly passes, the vulnerability analysis must be
reviewed again.

Companion criteria (for [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                                                       | Defect prevented                                                                                                                                |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_program_stage_coverage`        | Every `Program::stages()` is a subset of `Stage::ALL` and equals the expected array                                                                             | A future stage being left unconnected                                                                                                           |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must produce a diagnostic in every mode                                                                                                 | The three `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                                   |
| `test_release_plan_spans_consumed`   | The set of `PlanId`s produced by ownership is a subset of the set consumed by IR construction (**settled by structural identity, with no allowlist**)           | Silent loss of Drop instructions due to span keys (historical form; after key migration, the consumption ledger + `W1090` provide the backstop) |
| `test_overload_resolutions_consumed` | Corpus file by file: produced overload resolutions are a subset of consumed resolutions, and the **evidence floor** is `examined ≥ 1` / actually consumed `≥ 2` | Resolution entries being silently dropped (producing a silently incorrect answer by falling back to the default binding)                        |
| `test_obligations_drained`           | CI fails if any `Obligations` field has no consumption point anywhere in the repository                                                                         | The entire class of defects                                                                                                                     |

### Semantic-Correction Triage Criteria (D59)

RFC-039's refactoring contract consists of **graded equivalence (C1–C6) + a dedicated
semantic-correction channel** (P3 / P3.5 + the decision registry). The constraints that “C5′ does
not exist” and “no relaxation is permitted” govern the **strength of the criteria**, not the
requirement to preserve incorrect semantics. The triage line has exactly one **decidable entry
criterion**:

> **Question: Is the old behavior consistent with the written specification?** (The criterion is
> whether an RFC/SPEC section number can be identified and a minimal counterexample provided.)

- **A section can be identified**: The old behavior violates a provision—it failed to report an
  error that should have been reported, failed to accept a program that should have passed, or
  silently produced an incorrect result—and the counterexample is reproducible ⇒ **Classes A/B/C may
  be fixed alongside the refactoring**.
- **No section can be identified** (the specification is silent or ambiguous, or the old behavior
  was already compliant) ⇒ **Classes D/E/F require user adjudication first**.

**“Success” does not depend on whether the compiler accepts the program**: a program that the
compiler erroneously accepts does not count as success—the erroneous acceptance is itself the defect
that must be fixed. Recording it as success would mean protecting the defect with the rules.

**All five A/B/C requirements are indispensable**:

1. **Red before green**—the red criterion must exist first and must currently fail (see the
   self-check in [HOWTO](../HOWTO.md)). “Green before red” does not qualify: without a red
   criterion, there is no way afterward to distinguish a “fix” from a regression.
2. **Do not change the externally observable behavior of programs that were previously
   specification-compliant**—a program that was previously compliant and ran correctly must not
   become a failure because of this change, and its existing diagnostics (code / message / exit
   code) must not change. It is **permitted** for a path that previously violated the specification
   to fail or become more conspicuous (previously no diagnostic → a new W/E code or increased
   severity are both acceptable)—that is precisely the correction itself.
3. **Register it in the same commit**—the corresponding WBS line must identify the error type
   (A/B/C) + **correct the affected main text in place** (the main text describes only the current
   state; the before/after correction diff and red→green evidence are retained with the commit
   information / record) + the required PR block must contain an “Error type” line.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshots / corpus
   differential). The classification is **not** a reason to exempt that review.
5. **Provision + counterexample**—the commit information and the PR “Error type” line must identify
   the relevant RFC/SPEC **section number** and provide the behavior's **minimal counterexample** (a
   compilable source snippet or a command and its output). If this cannot be provided ⇒ the change
   must not be treated as A/B/C and must instead fall back to D/E/F.

| Class                                                 | Criterion                                                                                       | Typical manifestation                                                                                                                                                                                                 | Precedent                           |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A Silent failure**                                  | An error that should have been reported is swallowed                                            | Empty `Unproven` branch, silently missing solver, `unwrap_or_default` fallback                                                                                                                                        | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent mismatch**                                 | Key mismatch / choosing between parallel representations / span-keyed misalignment              | Span key silently dropping Drop, two representations of the same concept                                                                                                                                              | 3.4.5 / D20 / D57                   |
| **C Undefined behavior and loss of resource control** | Crash, stack overflow, uninitialized slot, bytecode version mismatch, silently incorrect answer | Pathological generic recursion (compile-time stack overflow), two hard-coded discards of `.42` (the third is a false report about a dead field), random dispatch of an ambiguous bare name to the wrong value (3.8.2) | 3.4.8 ①/② / D52                     |

**If no provision can be identified, or the old behavior was already compliant but the current
behavior is to change → Classes D/E/F still require user adjudication first** (see “Mandatory When
Requesting User Adjudication” in the repository-root `AGENTS.md`):

| Class                          | Criterion                                                                                                                              | Example                   |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-visible semantics**   | Externally visible aspects of diagnostic code additions/removals/changes, externally surfaced error text, module semantics, exit codes | D54 / D55 / RFC-029g      |
| **E External contract tables** | The error code table, opcode table, type table, or entry-stage table themselves                                                        | D1 / D6 / D12 / D20 / D23 |
| **F RFC design changes**       | Metatype universe, new E codes, wholesale removal of syntax                                                                            | D53 / RFC-033 / RFC-029g  |

**An unspecified specification ≠ a specification violation**: whenever the specification is silent
or ambiguous, the change must always fall back to D/E/F—“I think the new behavior is better” is not
an entry criterion.

**Primary-error attribution**: A change is classified by its **primary error type**; incidental
behavior changes within the minimal fix set for the same root cause may be listed separately in the
PR's “D3 patch determination” line (precedent: 3.4.8 ① has a primary Class C defect, with three
asymmetric Class A defects attached).

**Relationship to C1/C2**: A/B/C changes **must not** be declared as C1 (byte-for-byte invariance)
or C2 (same diagnostic set), because the definitions of those classes are behavior-preserving
changes; opportunistic fixes must be declared as the corresponding category among C3/C4/C6.

**Why not use a thematic allowlist**: [08](08-maintenance-mechanism.md) already states that “the
number of exemptions itself enters the gate report”; the list will grow, and cases involving “one
primary-error class + an attached fix” inevitably fall on the boundary. The entry criterion is
Boolean-decidable.

### Regression Gates

Add the following CI checks, following the existing conventions in `scripts/ci/`:

| Check                      | Trigger                                                                   | Failure condition                                                                              |
| -------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | C1/C3/C5 changes                                                          | Snapshot files differ but the commit information does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` reports an error over the full corpus                                               |
| `check-corpus-parity.py`   | C2/C4/C5 changes                                                          | The corpus differential is non-empty                                                           |
| `check-stage-contract.py`  | All changes                                                               | `Program::stages()` does not match `Stage::ALL`                                                |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | Performance baseline regresses by >10% and the PR provides no explanation                      |

### Dependencies on Companion Documents

| Document                                                                       | Must have first                                                           | Criterion strength         |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------- | -------------------------- |
| [02-stage-contract.md](02-stage-contract.md) (staging)                         | The vulnerability criterion from this document (red first)                | C2: same diagnostic set    |
| [03-type-unification.md](03-type-unification.md) (type representation)         | Tier 1 verifier's type-consistency invariant                              | C3: same diagnostic codes  |
| [04-ssa.md](04-ssa.md) (SSA conversion)                                        | **All three tiers**, and the verifier must first pass on the non-SSA form | C4: behavioral equivalence |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generatorization) | AST snapshot + diagnostic criteria                                        | C5: use all three          |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)             | Reference scan                                                            | None                       |

## Key Decisions and Rationale

**Grading rather than a single criterion.** Using “full equality of IR snapshots” as the criterion
for SSA conversion (C4) would fail outright, tempting the team to relax the criteria; grading
ensures that each step uses a criterion with the strength it can support. At the same time, Tier 1's
verifier covers dominance checks, turning errors that “cannot be tested” into testable
invariants—something snapshots cannot do. The criterion artifacts are checked in and evolve with the
code; they are not a one-time investment.

### Rejected Approaches

- **Using only an end-to-end corpus differential**—Defect Classes 1/2/3 are exactly the cases where
  “the program still completes in some way,” so the end-to-end differential is insensitive to them.
  SSA conversion is particularly dangerous: skipped register numbers may affect only performance
  rather than output, while a Drop instruction lost through span mismatch **manifests only for
  specific inputs**, which the corpus may not cover.
- **Using only IR snapshots**—snapshots are inherently unable to catch Defect Class 2 (see the
  “Normalization” section) and are entirely ineffective during C4.
- **Using only property-based testing** (`proptest` is already a dependency)—random programs have
  difficulty covering ownership, existential, and refinement paths that require specific shapes to
  be triggered. **However, it can serve as a supplement**: for Class 1 (simple
  expressions/arithmetic), it is indeed better than fixed corpus coverage, and it should be
  introduced as a supplementary criterion for C4 once the corpus differential is stable.
- **Refactoring first and adding tests later**—this project has already encountered that trap. The 8
  orphan test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1,005 lines /
  78 tests that have never run) are the result of creating directories before the wiring succeeded.
  The criteria must precede the refactoring.

## Known Limitations and Risks

- **Tier 1's verifier requires forward-looking SSA information.** In the non-SSA form, the concept
  of a “unique definition point” does not exist (`Operand::Local(n)` points to a slot that may be
  written multiple times). The verifier therefore needs two modes: `verify_ssa` (strict, including
  Phi checks) and `verify_loose` (only dominance and types, allowing multiple definitions).
  **`verify_loose` must first pass on the existing IR**; this is the largest single workload in the
  design.
- **Tier 2's normalization misses Defect Class 2.** This is a known limitation and can only be
  addressed jointly by Tier 1 + Tier 3; it cannot be solved by strengthening normalization.
- **Tier 3's corpus differential is sensitive to nondeterministic output.** If a `.yx` corpus case
  produces output that depends on hash iteration order or time, the differential may fail
  spuriously. Such corpus cases must be filtered out when the baseline is established.
- **Checking in snapshots creates a maintenance cost.** Whenever the IR definition changes, the
  snapshot must be explicitly updated, which requires review discipline.
- **The corpus covers only the single-file path.** There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus cases take the single-file path—see the limitation under “Existing Test Assets.”

> **All open questions originally listed in this section have been adjudicated.** The individual
> decisions are recorded in the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **No
> pending items remain in this document.**

## See Also

- [RFC-039 Compiler Feature Routing Directory Design](../../rfc/accepted/039-compiler-architecture.md)
  — the overarching blueprint
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, this
  repository's existing example of an “item-by-item round-trip” criterion
- `src/middle/core/tests/bytecode.rs:769-814` — a sentinel test that uses `include_str!` to read
  source code for static differential comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional set-difference assertions + snapshot
  table, requiring explicit updates when a new frontend is integrated
- `ir_gen.rs:248-249` — the explicit statement of the physical-adjacency contract for save/restore
  points
