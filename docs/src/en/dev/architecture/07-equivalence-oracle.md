# Refactoring Equivalence Criteria

> **Subsidiary Design Document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiered acceptance criteria, and execution stage ordering are in the RFC-039 main
> text; the positioning of each subsidiary document is in [this directory index](index.md).

## Positioning and Scope

This document defines the criteria for "proving behavior is unchanged" during the refactoring
process. Criteria are **tiered by refactoring step category**—because different categories require
different strengths of equivalence: pure relocation requires byte-for-byte identity, while
SSA-ification necessarily changes IR form and can only judge behavioral equivalence.

**This document is a precondition for all refactoring.** Without criteria,
[SSA-ification](04-ssa.md) cannot be implemented safely.

This document does **not cover**: stage contracts themselves (see `02`), the design of type
representation convergence (see `03`), or the dead code cleanup inventory (see `06`).

## Current State

### The most dangerous code today happens to be code with zero test coverage

`src/middle/core/ir_gen.rs` (8,448 lines):

- The file contains **no `#[cfg(test)]`, no `mod tests`**
- Among the 29 tests in `src/middle/core/tests/`, `def_assign` / `local_slots` only positively pin
  down the sentinel behavior of DefId allocation and slot naming; the 4 assertions in `bytecode.rs`
  referencing ir_gen are all **negative assertions** ("ir_gen does not construct a certain
  instruction")
- That is: **IR structural invariants (dominance, unique definitions, jump targets, type
  consistency) currently have zero validation**—this is exactly what the first-layer criteria must
  address

And this file happens to contain three classes of defects **that produce no error when they fail**:

**Defect class 1: Register misalignment.** 6 manual save/restore pairs scattered over 70+ lines
apart:

| Save point            | Restore point | Containing function         |
| --------------------- | ------------- | --------------------------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | `generate_method_ir`        |
| `1858-1866`           | `2026-2029`   | `generate_function_ir`      |
| `2116`                | `2180`        | curry intermediate function |
| `2224-2229`           | `2284-2288`   | curry innermost function    |
| `2768-2777`           | `2820-2851`   | `generate_anon_binding_ir`  |
| `4686-4695`           | `4767-4774`   | `generate_lambda_body_ir`   |

The file itself writes this contract at `ir_gen.rs:248-249`: "**Must be physically adjacent to the
save/restore points of `next_temp`**, otherwise the inner function's name will leak into the outer
function."

Consequence of missing a single restore: `next_temp` leaks, temporary values skip numbers. **The IR
remains self-consistent, still passes all checks, still compiles—only the values are wrong.**
Ordinary assertion tests cannot catch this.

**Defect class 2: `arg_regs` semantic reordering misalignment.** Inside `generate_call_expr_ir`
(`7269-8014`, 747 lines), 6 branches share the same `Vec<Operand>` and perform semantic rewriting:

| Line        | Reordering semantics                      |
| ----------- | ----------------------------------------- |
| `7395`      | Namespace call: named argument reordering |
| `7724`      | Struct construction: field reordering     |
| `7835-7848` | Function call: named argument reordering  |

And in the middle, default-value registers are also appended. **Misalignment produces no error, only
wrong values.**

Even more dangerous: this class of error **may also not be caught** under "normalized snapshots"—if
the snapshot tool normalizes away temporary register numbers (standard practice), then "the 3rd
argument should have used the 5th register"-class errors are **completely identical** in the
snapshot.

**Defect class 3: span-keyed cross-layer contracts silently failing.** `ReleasePlan` uses `Span` as
a key (produced at `layers/ownership.rs:31`), passed through `checker.rs:1440`, and finally matched
and emits `Instruction::Drop` at `ir_gen.rs:1942` via `self.release_plan.get(&stmt.span)`. Any
inconsistency in span computation on either side → **Drop instructions silently disappear, with no
error whatsoever**. `overload_resolutions` is also span-keyed.

> **`method_def_ordinals` is not span-keyed**: it is a `HashMap<String, usize>` (`ir_gen.rs:224`,
> key is the unqualified `"Type.method"`)—its risk nature is different (definition order / name
> collision mismatch rather than span mismatch), and SSA-ification does not resolve it (see
> [04](04-ssa.md) mechanism 3 for handling). Therefore span-keyed contracts number **2**
> (`ReleasePlan` / `overload_resolutions`).

### Existing Test Assets

The good news is that raw material for criteria is abundant:

| Asset                               | Scale                                                                                                                                                       | Use for                                                                                         |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `tests/yaoxiang/**/*.yx`            | **293** corpus items, numbered by specification section (`00-smoke` … `06-compile-errors`, `99-demos`). **⚠️ All single-file paths**—see restrictions below | End-to-end behavior diff (**single-file side only**)                                            |
| `src/std/tests/*.yx`                | A batch                                                                                                                                                     | Standard library behavior diff                                                                  |
| `tests/integration/`                | 18 modules / 5,846 lines                                                                                                                                    | CLI end-to-end (of which `multifile.rs` with 27 tests is the **only** multi-file side coverage) |
| `src/middle/core/tests/bytecode.rs` | 1,131 lines / 24 tests, including `test_every_opcode_roundtrips_not_silently_nop` (`bytecode.rs:958`, per-opcode roundtrip)                                 | Bytecode-layer roundtrip                                                                        |

> **⚠️ Structural coverage limitations of the corpus (added 2026-10-03)**
>
> There are **no `yaoxiang.toml`** anywhere under the `tests/` directory (verified: 0). Therefore
> all 293 corpus items go through the `standalone` branch of `check_files_with_diagnostics` at
> `util/diagnostic/mod.rs:565` → `check_single_file` → **`Pipeline::run` (single-file path)**.
>
> **Consequence**: the third-layer criteria **can only verify changes to the single-file path**. For
> the multi-file side of [`02`](02-stage-contract.md) (the `orchestrator`'s `check_project` /
> `compile_project` / `check_source_in_project` / `compile_embedded_module`) and the multi-file
> behavior of [`03`](03-type-unification.md) / [`04`](04-ssa.md), **there is currently no corpus
> coverage**.
>
> **A multi-file corpus layer must be established at P2**, otherwise the largest refactoring
> stage—"Unified Driver"—has no executable criteria. The 27 tests in
> `tests/integration/multifile.rs` are the only existing multi-file side coverage, but they test
> semantic results rather than compilation-stage contracts.

**But these assets share a common deficiency**: they test "input → output", not "whether the
intermediate representation is correct". The correctness of the IR layer currently has **no criteria
whatsoever**.

## Target Design

### Core Principle: Criteria Tiered by Refactoring Category

**This is the single most important point in this document.** Using a single criterion (usually "IR
snapshot equality") to cover all refactoring types is wrong—because some refactorings **necessarily
change the IR form**.

| Refactoring category                                                                  | Criterion type                                                                            | Strength  | Documents involved                          |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------- | ------------------------------------------- |
| **C1 Pure relocation** (splitting files, changing directories, extracting submodules) | IR normalized snapshot **zero-diff**                                                      | Strongest | `02`'s checker split, `04`'s lowering split |
| **C2 Orchestration change** (staging, unified Driver)                                 | Each entry's **diagnostic set** is identical + corpus behavior identical                  | Strong    | `02`                                        |
| **C3 Type representation convergence**                                                | Diagnostic code and message **identical** (order normalizable)                            | Strong    | `03`                                        |
| **C4 IR form change** (SSA-ification)                                                 | **Behavioral equivalence** (execution results identical) + IR structural invariants       | Medium    | `04`                                        |
| **C5 Frontend paradigm change** (grammar-driven)                                      | AST snapshot identical + **diagnostic code+span identical per item** + behavior identical | Strong    | `05`                                        |
| **C6 Pure deletion**                                                                  | No equivalence criterion needed, just confirm no references                               | —         | `06`                                        |

**The distinction between C1 and C4 is key**: C1 requires IR to be byte-for-byte identical (any
change is a regression); C4 acknowledges that IR will change, switching to "program behavior
identical + IR satisfies invariants".

> **⚠️ C5′ explicitly does not exist**
>
> The diagnostic requirements of the error recovery dimension are **not relaxed**: LALRPOP's error
> recovery model differs from `synchronize()`—precisely for this reason, the grammar of
> [05](05-frontend-paradigm.md) **must use explicit error productions to model the behavior of
> `synchronize()`**, making diagnostics identical per item. Failing to do so is an implementation
> defect, **not a reason to relax the criteria**.
>
> If stages 3a/3b empirically discover that "the existing diagnostics cannot be reproduced under
> LALRPOP", the correct action is to **truthfully report this finding and re-evaluate the plan**
> (including the option of "staying with Pratt"), **not to modify the criteria**.

### Three-Layer Criteria

#### First Layer: IR Static Verifier (Covers All Categories)

Provide `verify(ir: &ModuleIR) -> Result<(), IrError>` at `src/middle/ir/verify.rs`. It is much
stronger than snapshots because it checks **semantic invariants** rather than concrete form.

| Invariant                                                                        | What defect it catches                                                                                                      | Typical cause                                                 |
| -------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **Use-before-def dominance**                                                     | Every `Operand::Local(n)` read must have a definition path on the CFG that dominates the current point                      | Register misalignment, missed restore                         |
| **Unique value definition** (only under SSA form)                                | Each SSA value has exactly one definition point; at join points there must be a `Phi`                                       | SSA-ification's own correctness                               |
| **Phi consistency**                                                              | The number of `Phi` inputs equals the number of corresponding predecessor basic blocks, and each input's type is consistent | Missing predecessor, type mismatch                            |
| **Jump target exists**                                                           | Every jump's label exists in `blocks`                                                                                       | Translation error in `rebase_jump_targets` (`ir_gen.rs:1337`) |
| **Global slot in bounds**                                                        | `Operand::Global(i)`'s `i < globals.len()`                                                                                  | Slot allocation error                                         |
| **Type consistency**                                                             | The `dst` and `src` of an instruction have compatible `MonoType`                                                            | Type representation bridging error                            |
| **Inner isolation** (corresponds to the explicit promise at `ir_gen.rs:248-249`) | Function A's `cur_locals` does not contain function B's temporary names                                                     | 6 save/restore contracts                                      |

**The verifier must exist before SSA-ification**, and must **first run green on the current
(non-SSA) IR**—otherwise it cannot be used to prove "equivalence before and after the change".

#### Second Layer: Normalized IR Snapshot (Covers C1/C3/C5)

IR snapshots must be **normalized**; otherwise any register numbering change will cause full diff
failure:

| Normalization item    | Rule                                                                                                    |
| --------------------- | ------------------------------------------------------------------------------------------------------- |
| Strip `Span`          | All span fields set to sentinel values                                                                  |
| Rename temporaries    | Rename by first-occurrence order to `%0 %1 %2 ...`, **preserving the definition-use topological order** |
| Relative global slots | Renumber by module-internal first-occurrence order                                                      |
| Predecessor sorting   | Sort `successors` and `Phi` inputs by label                                                             |
| Struct field sorting  | Sort only when field names are known (construction order is semantic, do not sort)                      |

**The output of the normalization tool must be checked into the repo**
(`src/middle/core/tests/snapshots/`), updated explicitly when IR definition changes, with manual
review of the diff at update time.

**Known limitation (must be stated)**: normalization erases the information "which register was used
for the Nth argument", therefore **the second layer alone is insufficient to cover defect class 2**.
The safety of `arg_regs` reordering at the C1 stage must be guaranteed jointly by first-layer
dominance checks + corpus behavior diffs.

#### Third Layer: End-to-End Corpus Diff (Covers C2/C4/C5)

For all **293** `.yx` in `tests/yaoxiang/` + `src/std/tests/*.yx`, run once before and once after
the change, comparing:

| Comparison object      | Normalization rule                                                                                                                                    |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostic list        | Sort by `(code, span.file, span.line)`; **message text does not participate in the comparison** (type representation convergence will change wording) |
| Exit code              | Direct comparison                                                                                                                                     |
| Runtime stdout/stderr  | Compare after sorting                                                                                                                                 |
| `dump_bytecode` output | Compare only at C1/C2 stages (bytecode will change with IR)                                                                                           |

**The behavior diff at the C4 (SSA-ification) stage must cover**: ref/borrow/move ownership
transfers, closure capture, currying (`generate_curry_*`), spawn and iterator for, sum type pattern
matching, existential forcing points, `?` / Try propagation, method overloading
(`overload_resolutions` span-keyed), refinement constraint Drop sequences (`ReleasePlan`).

**Performance baseline**: corpus diffs guarantee "identical behavior", not "identical time". While
P2 establishes the diff baseline, use `criterion` already in dev-dependencies to build 2-3 smoke
benchmarks (full-corpus compile time / typical program interpretation throughput / CLI cold start)
and check them in, as performance criteria for subsequent pipeline-class changes—slot reuse
deletion, Phi expansion into Move chains, and Driver indirection layers may all change timing, and
without a baseline no one will catch it.

### Dedicated Criteria for Fixing Correctness Vulnerabilities

The vulnerability that [`02`](02-stage-contract.md) intends to fix requires **independent criteria**
that cannot be mixed into the three layers above:

```rust
#[test]
fn test_multifile_proof_obligation_not_dropped() {
    // For the same source containing x: Sorted(3), the diagnostic sets of
    // the single-file and multi-file paths must be equal
    let single = compile_via_pipeline(SOURCE);
    let multi  = compile_via_orchestrator(PROJECT, SOURCE);
    assert_eq!(norm_diagnostics(single), norm_diagnostics(multi));
}
```

**This test must be written first as failing** (red), confirming it captures the existing
vulnerability, before beginning the fix. If it accidentally turns green, the vulnerability analysis
needs re-verification.

Similar criteria (paired with [`02`](02-stage-contract.md)):

| Test                               | Assertion                                                                       | Defect it catches                                  |
| ---------------------------------- | ------------------------------------------------------------------------------- | -------------------------------------------------- |
| `test_program_stage_coverage`      | Every `Program::stages()` ⊆ `Stage::ALL` and equals the expected array          | Future missed stage hookups                        |
| `test_no_silent_pass_on_unproven`  | `ProofResult::Unproven` must produce a diagnostic in any mode                   | The extend branches at `checker.rs:5179/5318/5448` |
| `test_release_plan_spans_consumed` | The Span set produced by ownership ⊆ the Span set consumed by IR construction   | `ReleasePlan` span-key silently losing Drop        |
| `test_obligations_drained`         | Any `Obligations` field with no consumer anywhere in the repo causes CI to fail | The entire class of this kind of defect            |

### Triaging Criteria for Semantic Corrections (D59)

The refactoring contract of RFC-039 is **tiered equivalence (C1–C6) + dedicated semantic correction
channel** (P3 / P3.5 + decision registry): "C5′ does not exist" and "no relaxation is set" constrain
the **strength of criteria**, not "incorrect semantics must be preserved as-is". The triage line has
only one **judgeable entry criterion**:

> **Question: Does the old behavior count as "successful"?** (Success = compiles and produces
> correct results)

**Not successful → A/B/C three classes can be fixed along with refactoring**, all four are required:

1. **Red before green**—the red criterion exists first and is indeed red right now
   ([HOWTO](../HOWTO.md) self-check item). "Green before red" does not hold: without a red
   criterion, you cannot later distinguish "fix" from "regression".
2. **Do not change the external behavior of "previously successful" programs**—programs that
   previously compiled and ran correctly must not become failures due to this change, and their
   existing diagnostics (code / wording / exit code) must not change. **Allowed**: making
   "previously unsuccessful" paths louder (previously no diagnostic → adding W/E codes, adding codes
   is OK); **not allowed**: touching the external behavior of "previously successful" paths.
3. **Same-commit registration**—the WBS corresponding line labels the error type (A/B/C) + a
   revision note for that stage + the PR's required block "Error type" line.
4. **Baseline regenerated in the same commit + manual review of diff** (IR snapshot / corpus diff).
   Classification is **not** a reason to exempt that review.

| Class                                         | Criterion                                                               | Typical form                                                                                 | Precedent                           |
| --------------------------------------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A Silent failure**                          | An error that should be reported is swallowed                           | `Unproven` empty arm, solver absence silent, `unwrap_or_default` fallback                    | 3.3 / 3.4.1 (W1063) / 3.4.2 (W1081) |
| **B Silent mismatch**                         | Key mismatch / parallel representation choice / span-keyed misalignment | span-key silently losing Drop, two parallel representations of the same concept              | 3.4.5 / D20 / D57                   |
| **C Undefined behavior and resource runaway** | Crash, stack overflow, uninitialized slot, bytecode version mismatch    | Pathological generic recursion (compile-time stack overflow), `.42` three hardcoded discards | 3.4.8 ①/② / D52                     |

**Previously successful, now to be changed → D/E/F three classes must still go through user
adjudication first** (see repository root `AGENTS.md` "When requesting user adjudication
(mandatory)"):

| Class                                 | Criterion                                                                                                        | Example                   |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------- |
| **D User-visible semantics**          | External-facing diagnostic code addition/removal/change, external-facing error text, module semantics, exit code | D54 / D55 / RFC-029g      |
| **E External-facing contract tables** | Error code table, opcode table, type table, entry stage table itself                                             | D1 / D6 / D12 / D20 / D23 |
| **F RFC design change**               | Meta-type universe, new E codes, wholesale syntax removal                                                        | D53 / RFC-033 / RFC-029g  |

**Main error attribution**: a single change is classified by its **main error type**; behavior
changes accompanying the minimal modification set of the same root cause can be listed separately in
the PR "D3 patch determination" line (precedent: 3.4.8 ① main error class C, with three attached
class A asymmetry defects).

**Relationship to C1/C2**: A/B/C-class changes **must not** be declared as C1 (byte-for-byte
unchanged) or C2 (diagnostic set identical)—those two classes are defined as behavior unchanged;
opportunistic fixes should be declared as the corresponding category in C3/C4/C6.

**Why not use a topic whitelist**: [08](08-maintenance-mechanism.md) has already declared that "the
exemption count itself enters the gate report", and the list will inflate; moreover, "main error one
class + attached fix" cases will inevitably get stuck at the boundary. The entry criterion is
Boolean-judgeable.

### Regression Gates

New CI checks (isomorphic to the existing `scripts/ci/` convention):

| Check                      | Trigger                                                                   | Failure condition                                                                             |
| -------------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `check-snapshot-drift.sh`  | C1/C3/C5-class changes                                                    | Snapshot file has a diff but the commit message does not contain the `snapshot-update` marker |
| `check-ir-verifier.sh`     | All IR changes                                                            | `verify()` returns non-empty on the full corpus                                               |
| `check-corpus-parity.py`   | C2/C4/C5-class changes                                                    | Corpus diff is non-empty                                                                      |
| `check-stage-contract.py`  | All                                                                       | `Program::stages()` does not match `Stage::ALL`                                               |
| `check-perf-regression.sh` | Changes touching the compilation pipeline or execution path (P4/P6/P7/P8) | Performance baseline regression >10% and PR has no explanation                                |

### Binding to Each Subsidiary Document

| Document                                                                       | Must first have                                                                 | Criterion strength            |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- | ----------------------------- |
| [02-stage-contract.md](02-stage-contract.md) (staging)                         | The vulnerability criteria of this document (red first)                         | C2: diagnostic set identical  |
| [03-type-unification.md](03-type-unification.md) (type representation)         | The type-consistency invariant of the first-layer verifier                      | C3: diagnostic code identical |
| [04-ssa.md](04-ssa.md) (SSA-ification)                                         | **All three layers**, and the verifier must first run green on the non-SSA form | C4: behavioral equivalence    |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) (frontend generatorization) | AST snapshot + diagnostic criteria                                              | C5: all three used            |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) (pure deletion)             | Reference scan                                                                  | None                          |

## Key Decisions and Rationale

**Tiered rather than a single criterion.** Using "IR snapshot equality" as the criterion for
SSA-ification (C4) will fail directly, thereby inducing the team to relax the criterion; tiering
lets each step use the strength it can bear. Meanwhile, the first-layer verifier covers dominance
checks, turning "untestable errors" into "testable invariants"—this is something snapshots cannot
do. Criterion assets are checked into the repo and evolve with the code; this is not a one-time
investment.

### Unadopted Directions

- **Use only end-to-end corpus diff**—defect classes 1/2/3 are precisely the class where "the
  program still ran in some way", and end-to-end diffs are insensitive to them. SSA-ification is
  especially dangerous: register skip-numbers may affect only performance not output, and span
  mismatches causing Drop loss **only manifest on specific inputs**, which the corpus may not cover.
- **Use only IR snapshots**—snapshots are inherently immune to defect class 2 (see the
  "Normalization" section), and completely fail at the C4 stage.
- **Use only property-based testing** (proptest is already in dependencies)—random programs have
  difficulty covering paths that require specific shapes to trigger ownership, existential, and
  refinement. **But it can serve as a supplement**: for the first class (simple
  expressions/arithmetic) it is indeed better coverage than a fixed corpus, and is recommended as
  supplementary criteria at the C4 stage after the corpus diff stabilizes.
- **Refactor first, then add tests**—this is a pitfall this project has already stepped into. The 8
  orphan test trees (1,005 lines / 78 tests that never ran) recorded in
  [`06-cleanup-inventory.md`](06-cleanup-inventory.md) are the product of "build the directory
  first, then fail to wire up". Criteria must precede refactoring.

## Known Limitations and Risks

- **The first-layer verifier requires SSA forward-looking information.** Under the non-SSA form, the
  concept of "unique definition point" does not exist (`Operand::Local(n)` points to a slot that can
  be written multiple times). Therefore the verifier needs two modes: `verify_ssa` (strict, includes
  Phi checks) and `verify_loose` (only dominance and types, allows multiple definitions).
  **`verify_loose` must first run green on the existing IR**—this is the largest single workload of
  this design.
- **The second-layer normalization will miss defect class 2.** This is a known limitation and can
  only be covered jointly by the first layer + the third layer; it cannot be solved by strengthening
  normalization.
- **The third-layer corpus diff is sensitive to non-deterministic output.** If the `.yx` corpus
  contains output that depends on hash iteration order / time, the diff will false-fail. Such corpus
  items need to be screened out when establishing the baseline.
- **Checking snapshots into the repo brings maintenance cost.** When IR definitions change,
  snapshots must be updated explicitly and reviewed with discipline.
- **The corpus only covers the single-file path.** There is no `yaoxiang.toml` under `tests/`, and
  all 293 corpus items go through the single-file path—see the restrictions in "Existing Test
  Assets" for details.

> **All open questions originally listed in this section have been adjudicated.** Per-item decisions
> are in the [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Feature Router Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — Top-level master
- `src/middle/core/tests/bytecode.rs:958` — `test_every_opcode_roundtrips_not_silently_nop`, the
  existing "per-item roundtrip" criteria paradigm in this repo
- `src/middle/core/tests/bytecode.rs:769-814` — Sentinel tests using `include_str!` to read source
  for static pair-comparison
- `src/middle/core/tests/bytecode.rs:1073-1131` — Bidirectional difference-set assertion + snapshot
  table, forcing updates when new frontends are connected
- `ir_gen.rs:248-249` — Explicit statement of the save/restore physical-adjacency contract
