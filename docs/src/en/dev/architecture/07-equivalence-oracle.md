# Refactoring Equivalence Criteria

> **Companion design document.** This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiering, and execution phase order are covered in the main
> RFC-039 text; see the [directory index](index.md) for the positioning of each companion document.

## Purpose and Scope

This document defines the criteria for determining “how to prove that behavior has not changed”
during refactoring. The criteria are **tiered by refactoring step category** because different
categories require different strengths of equivalence: pure relocation requires byte-for-byte
identity, whereas SSA conversion necessarily changes the shape of the IR and can only be judged
behaviorally equivalent.

**This document is a prerequisite for the entire refactoring.** Without these criteria,
[SSA conversion](04-ssa.md) cannot be implemented safely.

This document **does not cover**: the stage contract itself (see `02`), the design for converging
type representations (see `03`), or the dead-code cleanup inventory (see `06`).

## Current State

### The riskiest current code happens to have zero test coverage

`src/middle/core/ir_gen.rs` (8448 lines):

- There are **no `#[cfg(test)]` modules and no `mod tests`** in the file.
- Of the 29 tests in `src/middle/core/tests/`, `def_assign` and `local_slots` only positively pin
  down the sentinel behavior of DefId allocation and slot naming; all four assertions in
  `bytecode.rs` that mention ir_gen are **negative assertions** (“ir_gen does not construct a
  particular instruction”).
- In other words, **IR structural invariants (dominance, unique definitions, jump targets, and type
  consistency) currently have zero verification**—precisely what the first layer of criteria is
  intended to address.

This file also happens to contain three classes of defects that **produce no errors when they
fail**:

**Defect Class 1: Register misalignment.** The file contains six manually written pairs of
save/restore statements, separated from their counterparts by more than 70 lines:

| Save point            | Restore point | Enclosing function            |
| --------------------- | ------------- | ----------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`          |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`        |
| `2116`                | `2180`        | Intermediate curried function |
| `2224-2229`           | `2284-2288`   | Innermost curried function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`    |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`     |

The file itself states this contract at `ir_gen.rs:248-249`: “**The save/restore point for
`next_temp` must be physically adjacent; otherwise, names from inner functions will bleed into the
outer scope**.”

The consequence of missing one restore is: `next_temp` leaks and temporary values skip numbers.
**The IR remains self-consistent, still passes every check, and still compiles; only the values are
wrong.** Ordinary assertion tests cannot catch this.

**Defect Class 2: Misaligned semantic reordering of `arg_regs`.** Six branches inside
`generate_call_expr_ir` (`7269-8014`, 747 lines) share the same `Vec<Operand>` and perform semantic
rewriting:

| Line        | Reordering semantics                    |
| ----------- | --------------------------------------- |
| `7395`      | Namespace call: reorder named arguments |
| `7724`      | Struct construction: reorder fields     |
| `7835-7848` | Function call: reorder named arguments  |

Default-value registers are also appended along the way. **Misalignment produces no error; it only
produces incorrect values.**

This class of error is even more dangerous because it **may also go undetected under a “normalized
snapshot”**—if the snapshot tool normalizes temporary register numbers away, as is standard
practice, then an error such as “the third argument should have used the fifth register” is
**completely identical** in the snapshot.

**Defect Class 3 (Eliminated): Silent failure of span-keyed cross-layer contracts.** `ReleasePlan`
previously used `Span` as its key (produced by `layers/ownership.rs:31`), passed it through
`checker.rs:1440`, and ultimately matched it through `self.release_plan.get(&stmt.span)` in `ir_gen`
to emit `Instruction::Drop`; any inconsistency in the two sides’ span computation → **the Drop
instruction silently disappears, with no error**. `overload_resolutions` has the same shape.
**2026-10-13 (RFC-039 D20 / WBS 4.5)**: both contracts were changed to use `PlanId`
**structural-identity keys** (`ReleasePlan::consume` / `OverloadResolutions::get` are the sole
consumption entry points; reading and accounting occur in the same invocation; a nonempty difference
set produces an E-level `W1090`), reducing this class of defects to zero—the implemented form and
criteria are documented in the revision notes in [02](02-stage-contract.md) §2.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> with the unqualified key `"Type.method"`)—its risk profile is different (definition-order or
> name-collision mismatch rather than span mismatch), and SSA conversion does not address it (see
> [04](04-ssa.md) Mechanism 3 for its disposition). Therefore, there are **2** span-keyed contracts
> (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that there are ample raw materials for these criteria:

| Asset                               | Scale                                                                                                                                                                     | Can be used for                                                                                                |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpus files, organized by specification chapter number (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All use single-file paths**—see the limitation below | End-to-end behavioral differential (**single-file side only**)                                                 |
| `src/std/tests/*.yx`                | A collection                                                                                                                                                              | Standard-library behavioral differential                                                                       |
| `tests/integration/`                | 18 modules / 5846 lines                                                                                                                                                   | CLI end-to-end testing (of which the 27 tests in `multifile.rs` provide the **only** multi-file-side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, round-tripping each opcode)                                          | Bytecode-layer round trips                                                                                     |

> **⚠️ Structural limitation of corpus coverage (added 2026-10-03)**
>
> There are **no `yaoxiang.toml` files anywhere under `tests/`** (measured: 0). Consequently, all
> 293 corpus files take the `standalone` branch of `util/diagnostic/mod.rs:565`’s
> `check_files_with_diagnostics` → `check_single_file` → **`Pipeline::run` (the single-file path)**.
>
> **Consequence**: the third layer of criteria can **only validate changes to the single-file
> path**. There is currently **no corpus coverage** for the multi-file side of
> [02](02-stage-contract.md) (`orchestrator`’s `check_project` / `compile_project` /
> `check_source_in_project` / `compile_embedded_module`) or for the multi-file behavior of
> [03](03-type-unification.md) / [04](04-ssa.md).
>
> **A multi-file corpus layer must be established in P2**; otherwise, the largest refactoring phase,
> the “unified Driver,” has no executable criteria. The 27 tests in `tests/integration/multifile.rs`
> are the only existing multi-file-side coverage, but they test semantic outcomes rather than
> compilation-stage contracts.

**However, these assets share a common flaw**: they test “input → output,” not “whether the
intermediate representation is correct.” The IR layer currently has **no criteria whatsoever**.

## Target Design

### Core Principle: Criteria Are Tiered by Refactoring Category

**This is the most important point in this document.** Using a single criterion—usually “full IR
snapshot equality”—for all refactoring types is incorrect because some refactors **necessarily
change the shape of the IR**.

| Refactoring category                                                                  | Criterion type                                                                              | Strength  | Related document                                      |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | --------- | ----------------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | Normalized IR snapshot **zero-diff**                                                        | Strongest | checker splitting in `02`, lowering splitting in `04` |
| **C2 Orchestration changes** (staged architecture, unified Driver)                    | Same **diagnostic set** at each entry point + same corpus behavior                          | Strong    | `02`                                                  |
| **C3 Type representation convergence**                                                | Same **diagnostic codes and messages** (order may be normalized)                            | Strong    | `03`                                                  |
| **C4 IR shape changes** (SSA conversion)                                              | **Behavioral equivalence** (same execution results) + IR structural invariants              | Medium    | `04`                                                  |
| **C5 Frontend paradigm changes** (grammar-driven)                                     | Same AST snapshot + **identical diagnostic code+span for every diagnostic** + same behavior | Strong    | `05`                                                  |
| **C6 Pure deletion**                                                                  | No equivalence criterion required; only confirmation that there are no references           | —         | `06`                                                  |

**The distinction between C1 and C4 is critical**: C1 requires byte-for-byte identical IR (any
change is a regression), whereas C4 acknowledges that the IR will change and instead requires “the
program behaves the same + the IR satisfies its invariants.”

> **⚠️ C5′ explicitly does not exist**
>
> The diagnostic requirements for the error-recovery dimension are **not relaxed**: the
> synchronization-point set of `ParserState::synchronize()` and the `Expr::Error` /
> `StmtKind::Error` placeholders jointly determine each diagnostic’s code and span; changes to the
> syntax surface or precedence table that touch these contracts must preserve them item by item.
> Failure to do so is an implementation defect and is **not grounds for relaxing the criteria**.
>
> If testing reveals that existing diagnostics cannot be preserved, the correct action is to
> **report the finding truthfully and reassess the design** (including “maintain the status quo”),
> **not to modify the criteria**.

### Three Layers of Criteria

#### Layer 1: Static IR Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` in `src/middle/ir/verify.rs`. It is far
stronger than snapshots because it checks **semantic invariants**, not a concrete shape.

| Invariant                                                                          | Defects caught                                                                                                         | Typical cause                                                 |
| ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **Use-before-def dominance**                                                       | Every `Operand::Local(n)` read must have a definition path that dominates the current point in the CFG                 | Register misalignment, omitted restore                        |
| **Unique value definition** (only in SSA form)                                     | Every SSA value has exactly one definition point; join points must contain a `Phi`                                     | Incorrect SSA conversion itself                               |
| **Phi consistency**                                                                | The number of `Phi` inputs == the number of corresponding predecessor basic blocks, and every input type is consistent | Missing predecessor, type mismatch                            |
| **Jump target exists**                                                             | Every jump label exists in `blocks`                                                                                    | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot bounds**                                                             | `Operand::Global(i)` satisfies `i < globals.len()`                                                                     | Incorrect slot allocation                                     |
| **Type consistency**                                                               | An instruction’s `dst` and `src` have compatible `MonoType`s                                                           | Bridged type representation error                             |
| **Inner isolation** (corresponding to the explicit promise at `ir_gen.rs:248-249`) | Function A’s `cur_locals` does not contain temporary names from Function B                                             | The six save/restore contracts                                |

**The verifier must exist before SSA conversion** and must **first pass on the current (non-SSA)
IR**; otherwise, it cannot prove equivalence before and after the transformation.

#### Layer 2: Normalized IR Snapshots (Covers C1/C3/C5)

IR snapshots must be **normalized**; otherwise, any change in register numbering will cause the full
diff to fail:

| Normalization item      | Rule                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Strip `Span`            | Set all span fields to a sentinel value. **This only became fully effective starting with WBS 6.6.12**: previously, the `type_decl` line printed the Debug text of the type declaration carrier’s `ast::Type`, and **100% of span leakage in IR snapshots came from that line** (96 lines / 69 files); after the carrier was replaced with an IR-native narrowed shape, spans no longer appear anywhere in the snapshot |
| Rename temporary values | Rename to `%0 %1 %2 ...` in order of first appearance, **preserving definition-use topological order**                                                                                                                                                                                                                                                                                                                  |
| Relativize global slots | Renumber in order of first appearance within the module                                                                                                                                                                                                                                                                                                                                                                 |
| Sort predecessors       | Sort `Phi` inputs by predecessor label (`successors` has not been included in snapshot rendering since 7b.3; after 7b.4, slots are reshaped by the allocator)                                                                                                                                                                                                                                                           |
| Sort struct fields      | Sort only when field names are known (construction order is semantic and must not be sorted)                                                                                                                                                                                                                                                                                                                            |

**The normalization tool’s output must be checked into the repository**
(`src/middle/ir/tests/snapshots/`), explicitly updated whenever the IR definition changes, and its
diff manually reviewed during updates.

**Known limitation (must be documented)**: normalization erases information about “which register
the Nth argument used,” so **the second layer alone is insufficient to cover Defect Class 2**.
During C1, the safety of `arg_regs` reordering must be ensured jointly by the Layer 1 dominance
checks and corpus behavioral differentials.

#### Layer 3: End-to-End Corpus Differential (Covers C2/C4/C5)

Run all **293** `.yx` files under `tests/yaoxiang/` plus `src/std/tests/*.yx` once before and once
after the change, and compare:

| Comparison target      | Normalization rule                                                                                                            |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text is not compared** (type representation convergence may change wording) |
| Exit code              | Compare directly                                                                                                              |
| Runtime stdout/stderr  | Compare after sorting                                                                                                         |
| `dump_bytecode` output | Compare only during C1/C2 (the bytecode may change with the IR)                                                               |

**The behavioral differential for C4 (SSA conversion) must cover**: ref/borrow/move ownership
transfers, closure captures, currying (`generate_curry_*`), `spawn` and iterator `for`, `and`-type
pattern matching, existential forcing points, `?` / Try propagation, method overloads
(`overload_resolutions` span keying), and the Drop sequence for refinement constraints
(`ReleasePlan`).

**Performance baseline**: a corpus differential guarantees “same behavior,” not “same elapsed time.”
In P2, while establishing the differential baseline, use the `criterion` benchmark framework already
present in dev-dependencies to establish 2–3 smoke benchmarks (full-corpus compilation time /
typical program interpretation throughput / CLI cold startup) and check them into the repository as
performance criteria for subsequent pipeline changes—removing slot reuse, expanding `Phi` into
`Move` chains, and adding Driver indirection can all affect elapsed time; without a baseline, no one
will intercept such regressions.

### Dedicated Criteria for Correctness-Hole Fixes

The hole that [`02`](02-stage-contract.md) is intended to fix requires an **independent criterion**
and must not be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets of the single-file and multi-file paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must first be written to fail** (red), confirming that it can catch the existing hole,
before the fix begins. If it unexpectedly passes, the vulnerability analysis must be reviewed again.

Related criteria (accompanying [`02`](02-stage-contract.md)):

| Test                                 | Assertion                                                                                                                                                 | Defect caught                                                                                                                                    |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_program_stage_coverage`        | Every `Program::stages()` ⊆ `Stage::ALL`, and the result equals the expected array                                                                        | A stage omitted from the wiring in the future                                                                                                    |
| `test_no_silent_pass_on_unproven`    | `ProofResult::Unproven` must always produce a diagnostic in every mode                                                                                    | The three `extend` branches in `checker/refinement.rs` (formerly `checker.rs:5179/5318/5448`)                                                    |
| `test_release_plan_spans_consumed`   | The set of `PlanId`s produced by ownership is a subset of the set consumed during IR construction (**settled by structural identity, with no whitelist**) | `ReleasePlan` span keys silently dropping Drop (historical form; after the key migration, the consumption ledger + `W1090` provide the fallback) |
| `test_overload_resolutions_consumed` | Corpus, file by file: overload-resolution output is a subset of consumption, with an **evidence floor** of `examined ≥ 1` and actual consumption `≥ 2`    | Resolution entries being silently dropped (silent incorrect answers caused by fallback to default bindings)                                      |
| `test_obligations_drained`           | CI fails if any `Obligations` field has no consumption point anywhere in the repository                                                                   | The entire class of defects                                                                                                                      |

### Routing Criteria for Semantic Corrections (D59)

RFC-039’s refactoring contract consists of **graded equivalence (C1–C6) + a dedicated
semantic-correction channel** (P3 / P3.5 + the decision registry): “C5′ does not exist” and “no
relaxation whatsoever” constrain **criterion strength**, not the requirement to preserve incorrect
semantics. There is only one **decidable entry criterion** for routing:

> **Question: Did the old behavior count as “successful”?** (Success = compilation succeeds and
> produces the correct result)

**If it did not count as success → Classes A/B/C may be corrected as part of the refactoring**, and
all four requirements are mandatory:

1. **Red before green**—the red criterion must exist first and must currently be red
   ([HOWTO](../HOWTO.md) self-check item). “Green then red” does not qualify: without a red
   criterion, it is impossible afterward to distinguish a “fix” from a regression.
2. **Do not change the externally observable behavior of “previously successful” programs**—a
   program that originally compiled and ran correctly must not become a failure because of this
   change, and its existing diagnostics (code / text / exit code) must not change either. It is
   **permitted** to make previously unsuccessful paths more conspicuous (previously no diagnostic →
   a new W/E code or increased severity are both allowed); it is **not permitted** to change the
   externally observable behavior of previously successful paths.
3. **Register in the same commit**—the corresponding WBS row must identify the error type (A/B/C) +
   **correct the affected body text in place** (the body text describes only the current state; the
   before/after diff and red→green evidence are retained in the commit message/record) + the PR’s
   mandatory “Error type” line is required.
4. **Regenerate the baseline in the same commit + manually review the diff** (IR snapshots / corpus
   differentials). Classification is **not** grounds to exempt this review.

| Class                                                 | Criterion                                                                             | Typical form                                                                                     | Precedent                           |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------- |
| **A Silent failure**                                  | An error that should have been reported was swallowed                                 | Empty `Unproven` branch, silent missing solver, `unwrap_or_default` fallback                     | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent mismatch**                                 | Key mismatch / choosing one of two parallel representations / span-keyed misalignment | Span key silently dropping Drop, the same concept represented by two different representations   | 3.4.5 / D20 / D57                   |
| **C Undefined behavior and resource loss of control** | Crash, stack overflow, uninitialized slot, bytecode version mismatch                  | Pathological generic recursion (compile-time stack overflow), three hard-coded discards at `.42` | 3.4.8 ①/② / D52                     |

**A previously successful path that now needs to change → Classes D/E/F still require user
arbitration first** (see “Mandatory When Requesting User Arbitration” in the repository-root
`AGENTS.md`):

| Class                        | Criterion                                                                                                            | Examples                  |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-visible semantics** | External-facing surface of added/removed/changed diagnostic codes, external error text, module semantics, exit codes | D54 / D55 / RFC-029g      |
| **E Public contract tables** | The error-code table, opcode table, type table, or entry-stage table itself                                          | D1 / D6 / D12 / D20 / D23 |
| **F RFC design changes**     | Meta-type universe, new E codes, wholesale removal of syntax                                                         | D53 / RFC-033 / RFC-029g  |

**Primary-error attribution**: classify each change by its **primary error type**; incidental
behavioral changes bundled into the minimal change set for the same root cause may be listed
separately on the PR’s “D3 Patch Determination” line (precedent: 3.4.8 ① has primary error Class C,
with three asymmetric Class A defects attached).

**Relationship to C1/C2**: A/B/C changes **must not** be declared C1 (byte-for-byte unchanged) or C2
(same diagnostic set), because those categories are defined by unchanged behavior; incidental
corrections should instead be declared as the corresponding category among C3/C4/C6.

**Why not use a thematic whitelist**: [08](08-maintenance-mechanism.md) already states that “the
exemption count itself enters the gate report”; the list will inevitably grow. Cases of “one
primary-error class + incidental fixes” will always sit at the boundary. The entry criterion is
Boolean-decidable.

### Regression Gates

Add the following CI checks (following the existing conventions under `scripts/ci/`):

| Check                      | Trigger                                                                   | Failure condition                                                                                |
| -------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `check-snapshot-drift.sh`  | C1/C3/C5 changes                                                          | Snapshot files have a diff, but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns a nonempty result for the full corpus                                         |
| `check-corpus-parity.py`   | C2/C4/C5 changes                                                          | The corpus differential is nonempty                                                              |
| `check-stage-contract.py`  | All changes                                                               | `Program::stages()` does not match `Stage::ALL`                                                  |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | The performance baseline regresses by >10%, and the PR provides no explanation                   |

### Binding to Companion Documents

| Document                                                                       | Required prerequisite                                                      | Criterion strength         |
| ------------------------------------------------------------------------------ | -------------------------------------------------------------------------- | -------------------------- |
| [02-stage-contract.md](02-stage-contract.md) (staged architecture)             | The hole criterion in this document (red first)                            | C2: same diagnostic set    |
| [03-type-unification.md](03-type-unification.md) (type representation)         | The type-consistency invariant in the first-layer verifier                 | C3: same diagnostic codes  |
| [04-ssa.md](04-ssa.md) (SSA conversion)                                        | **All three layers**, and the verifier must first pass on the non-SSA form | C4: behavioral equivalence |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generatorization) | AST snapshots + diagnostic criteria                                        | C5: use all three          |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)             | Reference scan                                                             | None                       |

## Key Decisions and Rationale

**Tiering rather than a single criterion.** Using “full IR snapshot equality” as the criterion for
SSA conversion (C4) would fail immediately, encouraging the team to relax the criteria; tiering
ensures that each step uses a strength it can sustain. At the same time, the first-layer verifier
covers dominance checks, turning errors that cannot be tested into testable invariants—something
snapshots cannot do. The criterion assets are checked into the repository and evolve with the code;
they are not a one-time investment.

### Rejected Approaches

- **Using only end-to-end corpus differentials**—Defect Classes 1/2/3 are precisely those in which
  “the program still ran to completion in some form”; end-to-end differentials are not sensitive to
  them. SSA conversion is especially dangerous: skipped register numbers may affect only performance
  rather than output, while a span mismatch that drops a Drop **may only manifest on particular
  inputs**, and the corpus may not cover them.
- **Using only IR snapshots**—snapshots are inherently immune to Defect Class 2 (see
  “Normalization”) and fail completely during C4.
- **Using only property-based testing** (`proptest` is already a dependency)—random programs have
  difficulty covering paths that require particular shapes for ownership, existentials, and
  refinements. **It can, however, serve as a supplement**: for Class 1 (simple
  expressions/arithmetic), it does provide better coverage than a fixed corpus; it is recommended
  for introduction as a supplemental C4 criterion once the corpus differential is stable.
- **Refactoring first and adding tests later**—this project has already fallen into this trap. The
  eight orphan test trees recorded in [`06-cleanup-inventory.md`](06-cleanup-inventory.md) (1005
  lines / 78 tests that have never run) are products of creating directories first and then failing
  to wire them up. The criteria must precede the refactoring.

## Known Limitations and Risks

- **The first-layer verifier requires pre-SSA lookahead information.** In non-SSA form, the notion
  of a “unique definition point” does not exist (`Operand::Local(n)` refers to a slot that may be
  written multiple times). The verifier therefore needs two modes: `verify_ssa` (strict, including
  Phi checks) and `verify_loose` (dominance and types only, allowing multiple definitions).
  **`verify_loose` must first pass on the existing IR**; this is the largest single implementation
  effort in the design.
- **The second layer of normalization misses Defect Class 2.** This is a known limitation and can
  only be covered jointly by Layer 1 + Layer 3; it cannot be solved by strengthening normalization.
- **The third layer, the corpus differential, is sensitive to nondeterministic output.** If any
  `.yx` corpus file produces output that depends on hash iteration order or time, the differential
  may fail spuriously. Such corpus files must be identified when establishing the baseline.
- **Checking snapshots into the repository creates maintenance overhead.** Whenever the IR
  definition changes, snapshots must be explicitly updated, which requires review discipline.
- **The corpus covers only the single-file path.** There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus files use the single-file path—see the limitation under “Existing Test Assets.”

> **All open questions previously listed in this section have been adjudicated.** The individual
> decisions are recorded in the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **No
> pending items remain in this document.**

## See Also

- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — overarching master plan
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, an
  existing “item-by-item round-trip” criterion example in this repository
- `src/middle/core/tests/bytecode.rs:769-814` — a sentinel test that uses `include_str!` to read
  source code for a static side-by-side comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — bidirectional difference-set assertions + a
  snapshot table, requiring updates whenever a new frontend is integrated
- `ir_gen.rs:248-249` — the explicit statement of the physical-adjacency contract for save/restore
  points
