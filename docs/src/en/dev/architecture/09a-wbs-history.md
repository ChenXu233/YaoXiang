# WBS Historical Notes (Landing Record · Audit · Decision Trail)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md) (the
> construction task table): landing records, audit findings, decision trails, and corrections for
> each stage. **Division of labor**: 09 describes only the **plan** (Kanban, three-level task table,
> dependencies, risks, acceptance); this document records only the **history** (what was done, what
> was discovered, how it was decided), appended chronologically without rewriting history.

## P2 Establishing the Equivalence Criterion Baseline

> **P2 landing (2026-10-06, 2.1)**: `verify.rs` dual mode + 7 invariants + all corpus runs green.
> **D38 prediction fulfilled** — the green-run process actually measured and fixed three real
> defects in `ir_gen` (see 06 §G6 for details): the method `locals` table was truncated to the
> parameter count (underestimated `local_count`, the runtime relied on `Frame::set_slot` to absorb
> the error), unannotated parameters were dropped by `filter_map` (three locations, mismatch between
> `params` metadata and frame layout), and the `match` result slot depended on the frame's implicit
> `Void` initialization (now explicitly pre-initialized). The actual semantics of the jump `target`
> is a flattened instruction index (not the label-keyed form in the 07 text); the CFG is built at
> the instruction level — recorded in the `verify.rs` file header. **Commit cross-reference**:
> `ir_gen` fix = `a9ea199b`; `verify` main body = `d5b18390` (⚠ a parallel-stream `COMMIT_EDITMSG`
> collision caused its message to be mislabeled as "docs(rfc): RFC-027a…", but the content was
> actually `verify.rs` plus six test files; it is buried in history and cannot be amended, so it is
> recorded here).
>
> **P2 narrowing landing (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 are all in place; **the P3
> stop-bleeding channel preconditions are unlocked**.
>
> - 2.3.2: 5 fixtures in `tests/yaoxiang-multifile/` (behavior ×4 + compile-error ×1) plus runner
>   `tests/yx_multifile_runner.rs`; the judgment contract uses the same `TestFileSpec` as the
>   single-file corpus (single source of truth on the library side), and binary location /
>   subprocess spawning is abstracted into `tests/common/`, with `yx_runner` migrated in lockstep
>   (prohibition three).
> - 2.4.1 / 2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed by the rfc027 tests (return site / call site / binding site) — the
>   `Sorted(3)` annotation example from 02 would currently be substituted and evaluated and directly
>   yield `Disproved`, not the silent `Unproven`; the `Sorted(3)` in 07's pseudocode is illustrative
>   only. Red-state measurement data is in the test file header.
> - The two red criteria are pinned with `#[ignore]` as persistent based on the 2.4.5 red-skeleton
>   precedent: the default suite is all green, and `-- --ignored` reproduces the red state; **when
>   the P3 fix lands, removing the attribute turns it green**.
> - The reason for the deliberate deviation from 07's (code, file, line) table in normalization
>   (dropping `file`/`line`, only comparing the set of error codes) is written in the 2.4.1 test
>   comment.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, so
> P2 cannot compile them. The former becomes P4's 4.1.4 (no double registration), and the latter's
> `#[ignore]` red skeleton is merged into 4.3.1.
>
> **2.3.3 landing (2026-10-07)**: The C4 required-coverage checklist was item-by-item verified (07
> §143); 10 categories of semantic corpus layers are fully covered and all within the
> `corpus-parity.jsonl` differential baseline (26 representative files, 26/26 hit, proven by grep) —
> ref/borrow/move under 05-ownership (full directory: `ref_shared`,
> `borrow_immutable/mutable/return`, `move_basic`, `ownership_deep`, etc.), closure capture
> (`closures` + `closure_arg_inference` + `spawn_capture`), currying (`curry_value_fix` +
> `curry_tail_expr`), spawn (04-concurrency ×13), iterators (16 files including `for`, `spawn_for`,
> etc.), sum types (`sum_type_*` + `pattern_or_guard` + `match_call_scrutinee`), existential
> coercion forcing points (`interface_dynamic_dispatch` — heterogeneous wrapping of `Vec(Animal)`
> triggers `existential_coercions` + `interface_rebind_dispatch`), `?`/Try propagation
> (`question_propagation` + `option_try` + `try_methods` + `user_try_type`), method overloading
> (`method_overload` + `method_overload_ambiguous_err` on the diagnostic-differential surface),
> refinement constraints with the Drop sequence (`refined_*` ×5 + 05-ownership driving
> `ReleasePlan`; the span-key contract has another persistent red criterion at 2.4.4). The
> performance baseline `benches/pipeline.rs` is in the repo (07 §145, two of three smoke tests):
> first-pass full-corpus compile in `corpus_compile_frontend` 17.6s, cold process start in
> `cli_cold_start_hello` 169.7ms; interpreter throughput is already covered by the `benches/lib.rs`
> hotpath group (`interp_fib_recursive_27` / `interp_loop_steady_10m`), no duplicate construction.
> The regression gate `check-perf-regression.sh` is folded into P9's unified checklist (2.5).

## P3 Correctness Vulnerability Fixes (Minimal Plan)

> **Error-type annotations for landed rows** (retroactive annotation, history is not changed): 3.1 =
> B ｜ 3.2 = B (defensive empty arm, criterion turning green is carried by 3.1) ｜ 3.3 = A ｜ 3.4.1
> / 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D
> (user-visible surface, already user-decided) ｜ 3.4.8 ①② = C (main) + A (incidental three
> locations) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-decided).
>
> **P3 diagnosis (2026-10-12, requested by user)**: a one-time accounting triage in response to the
> perception that "P3 items keep growing."
>
> **1. First, clarify scale (the perception was amplified by two accounting defects)**
>
> 1. **Duplicate numbering**: `3.5.x` was shared by two task families — (a) the 2026-10-08 **029g
>    `pub` visibility convergence** (7 three-level rows: lexical and parser branches / AST `is_pub`
>    / checker side / dead-code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) the 2026-10-10 logged **3.5 higher-kinded
>    type parameter call-path completion** (3 three-level rows). Both used `3.5.x` ⇒ cross-reference
>    ambiguity (**on 2026-10-12, per the user's decision, renumbered: the 029g family is now
>    `029g.x`**), which also caused the 029g rows to be mistakenly counted under P3. **Pending
>    decision**: rename the 029g family to `029g.x` (or move it into its own stage), and synchronize
>    all other references in the document.
> 2. **Missing checks**: the 4 three-level rows under `3.7` (3.7.1–3.7.4) are all complete
>    (`d922a551` / `38bd4b9e`) but were not checked off ⇒ corrected in this entry.
>
> **2. P3 actual accounting (12 second-level rows)**
>
> | Row                                                                   | Status                                                                                                                                                                                                 | Disposition suggestion                                                                                                              |
> | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                       | Complete                                                                                                                                                                                               | —                                                                                                                                   |
> | 3.4 silent-drop point additions (8 three-level)                       | 3.4.1 / 3.4.2 / 3.4.3 / 3.4.7 / 3.4.8 complete; **3.4.6 is unlocked** (precondition 4.2.1 was completed on 2026-10-09, `be60e01f`); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4 / 3.4.5 unlocked by 4.3.1                                                                          |
> | 3.5 higher-kinded type parameter call-path completion (3 three-level) | Not started (declaration side alive, call side broken)                                                                                                                                                 | Keep in P3; **does not block the main refactor line**, but is "accepted feature not implemented" and has priority over pure cleanup |
> | 3.6 function-type value call path (1 three-level)                     | Not started (semantics already decided (i))                                                                                                                                                            | **Keep**: a correctness defect with a clear criterion                                                                               |
> | 3.7 init-segment call-name qualification (4 three-level)              | **All complete**                                                                                                                                                                                       | Check off and close in this entry                                                                                                   |
> | 3.8 native shadowing (1 three-level)                                  | Not started                                                                                                                                                                                            | **Stuck on a decision** (name-resolution priority must be decided first), not a scheduling issue                                    |
> | 3.9 / 3.10 / 3.11                                                     | Complete (2026-10-12)                                                                                                                                                                                  | —                                                                                                                                   |
> | 3.12 runtime-mode executor configuration propagation (1 three-level)  | Not started (category D, user decision 乙: separate team)                                                                                                                                              | **Suggest moving out of P3 → P8 (execution layer)**, pending decision                                                               |
>
> **4. The leverage of 4.3.1 (main-line landmark)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the precondition of **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ doing it once advances the main line and clears two P3 stranded items. 3.5
> (HKT), 3.6 (function-type value call), 4.6.1, and 4.9.1 have **no unmet preconditions** and are
> doable any time; 3.8 is stuck on a semantic decision (user-defined first vs. explicit diagnostic).
>
> **3. Mechanism suggestion (prevention of recurrence)**: every newly registered second- /
> third-level row **must carry two fields** — "owning stage" (which P it really belongs to) and
> "whether it blocks the main line"; "registered, pending scheduling" is no longer a default state;
> if not annotated, it is considered unschedulable. This makes bloat visible and sortable, rather
> than undifferentiated accumulation. **Pending decision**: whether to write this into the
> registration conventions of this document and into `src/dev/coding-rules.md`.
>
> **P3 landing (2026-10-06)**: 3.1.1 `proof_calls` visibility narrowed + sole getter (recorded at
> the time as `pub(crate)`; **on 2026-10-07 it was verified that the actual landing was
> `pub(super)`** — the `#434` decision referenced in code comments has no entry on the docs side,
> see 3.4.3; going fully private would require rewriting 11 construction literals, not taken in the
> stop-bleeding phase; the final closure happens when P4 migrates the Obligations); the execution
> mechanism is extracted into `src/frontend/proof_execution.rs` (prohibition three, single point —
> must not live in `typecheck/`, that would create a reverse L3→L4 dependency); 3.1.2 follows step 5
> of 02's evidence chain to plug all **four entry points** (the WBS row omitted
> `compile_project :99`, which had to be added and registered per 3.1.1's acceptance "criterion
> turns green"); 3.2.1 turns the empty arm into bookkeeping (`proof_calls` propagates upward) +
> diagnostics (`into_result` is the established conversion path); 3.3.1 makes the SOLVER slot
> `Option`-typed, and Z3 absence falls back conservatively as `SMTResult::Unknown` (E2031-family
> diagnostics replace process crashes). The `#[ignore]` is removed from the two vulnerability
> criteria and they turn normally green; the supplementary corpus fixture `proof-obligation-honored`
> is added; the `check-fanout` repo migrates accordingly (wiring point 19 = repo). Measured:
> criteria 2.4.1 / 2.4.3 turn green; the full suite is lib 2494 / integration 272 / dual-layer
> corpus all green.
>
> **3.4 audit (2026-10-07, re-check after P3 landing)**: For the class of defects "failure produces
> no signal" (definition in 02 §41–52), a fresh inventory was taken on unchanged code, locating 8
> more locations, all registered as 3.4.x in the table above. Method and evidence:
>
> - **Consumer-side inventory** (every one of the 16 obligation fields grepped across the whole repo
>   for read sites): `variant_ctor_calls` has **zero consumers** — the only read at
>   `checker.rs:1468` is "writes it into `TypeCheckResult` itself"; the IR side uses shape detection
>   at `ir_gen.rs:7374`; `rfc011b.rs:659/688` only assert the production side, which is exactly the
>   form critiqued in 02 §186 ("only tests the production side, naturally immune to consumer-side
>   defects") → 3.4.4. The result-level field `method_overload_ir_names` likewise has zero external
>   consumers; IR uses the parallel `overload_resolutions` → 3.4.5.
> - **Stage-coverage re-check**: row 1 of the table in 02 §91 (the four-entry-point "no
>   `proof_execution`") was fixed by 3.1.2, and the current state is 5 locations sharing a single
>   implementation (`pipeline.rs:189`, `orchestrator.rs:141/340/507/1448`); but rows 2 / 4 / 5 are
>   unchanged (the multi-file `run` still does not emit W1003 and still does not run the dead-code
>   family) → 3.4.6; the LSP out-of-project path (02 §77 "manual lex→parse") likewise does not run
>   `proof` → 3.4.7.
> - **Same-shape empty-arm scan** (`Unproven`'s only four consumers in the repo): three refinement
>   branches in checker (covered by 3.1), the ownership arm fixed by 3.2.1 (**unreachable in
>   source**), and `environment.rs:422` (**reachable, zero diagnostics, zero bookkeeping, and zero
>   entries across the whole repo's docs**) → 3.4.1, the only "nobody knows about it" living defect
>   in this audit.
> - **Solver-acquisition surface**: `default_solver()` has three call sites with different
>   consumption shapes (`predicate.rs:32` per-process singleton / `checker.rs:1266` injected per
>   `check_module` / `ownership.rs:621` new instance per back-edge decision) — 3.3.1 only removed
>   the panic; the **silent surface** of the other two is 3.4.2, and **singleton-ization and cache
>   hit** belong to P4's 4.4.2 (no double registration).
> - **Items not counted in 3.4 that already have an owner**: 2.4.4 (`ReleasePlan` span-key mismatch,
>   168 files / 373 keys not consumed, Drop silently lost) remains a P2 persistent red state, owned
>   by 4.5.1 / D20; 4.4.1 (layer order declared vs. actual reversed, `equivalence` layer not in the
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty-ledger degradation) are already in P4.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions and can land in place any
> time (suggested first: 3.4.1 — the only "zero repo-wide entries" living defect, with the smallest
> change surface); 3.4.4–3.4.8 follow P4 execution, since their criteria depend on the obligations
> ledger (4.3.1) or the unified Driver (4.2.x). **3.4 does not stand up a new criterion system**; it
> uniformly reuses the existing C2 / C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 landing record (registered 2026-10-13 → closed 2026-10-15)**: On 2026-10-14, the
> user decided to merge four relevance sub-items (#451 method-binding panic / #452
> named-actual-parameter resolution / #453 dead-inference entry / #454 native export
> named-actual-parameter — the last one being a boundary discovered while verifying 3.14, verified
> the same day by attribution experiment to be **not introduced by #452**, user decision ① merged
> into this batch) for a single team to handle. Closeout status: all four items complete, the
> decision is logged as RFC-039 **D60** (six binding rules + single-point shared criterion).
>
> **1. Two contract corrections (lesson: the contract itself can be wrong)**
>
> 1. **Self-contradictory acceptance fixture**: the #452 task contract required both "mixed actuals
>    output 20" and "position and named at the same slot produce an error", but the pinned-board
>    fixture `Temp.pick(self, a)` has only a single value parameter — the same actual-argument
>    combination is both "mixed" and "conflicting". The first implementer honestly reported a
>    blocker, marked the task failed, and reverted the whole thing (no silent green). Disposition:
>    task reassigned + contract revised — the **six binding rules** and the **8-value-level matrix**
>    were given (fixture changed to two value parameters `pick(self, n, a)`), and attempt 2 was
>    completed by a second implementer. The full text of the six rules has been incorporated
>    verbatim into D60.
> 2. **inScope missed a mechanically required surface**: the #452 contract `inScope` listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missing the regenerated
>    `docs/src/reference/error-code/`** under the generator. After E1110 was registered, only
>    `code-tables --fix` was run, and **`scripts/docs/gen-error-code-docs.py` was NOT rerun** ⇒ CI
>    Docs Truth red ("source has registered/emitted 1 missing from docs: E1110", source 154 / docs
>    153, run **38518624964** — the only CI red line in this batch). Fix = mechanical regeneration
>    (8 files, +83/−70, only `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **2. Two closed loops: review → fix → re-review**
>
> 1. **#452 line**: round-1 review `needs_revision` (F1–F5; F1 = the arity guard
>    `named_args.is_empty()` did not narrow with the resolution ⇒ calls with named actuals on the
>    multi-candidate zero-hit and single-candidate hit paths went silently throughout the chain) →
>    the fix round added the outward code **E1111 (`argument_order_invalid`)** (the code table,
>    locales, RFC-013, and the generator four-piece set in the same commit; code count 154→155) →
>    round-2 review pass.
> 2. **#454 line**: implementation (shared parse entry + native arm resolves by name) → independent
>    verification PASS (12-cell value-level matrix, corpus 342/342, 5 mutations all mechanically
>    reversibly replayed via `scratch.py trial` and captured, restoration byte-for-byte identical) →
>    round-1 review `needs_revision` (F1 = the typecheck native arm **inlined a copy** of the
>    binding criterion, missing rule ④ ⇒ method-vs-native same-shape calls **diverge in decision** —
>    methods report E1111, native silently exits 0; ir_gen has a third existing copy reporting the
>    old code E1015) → convergence round collapsed the criterion into a single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call sites `expressions.rs:2727`/`:5416`,
>    `ir_gen.rs:7776`/`:8279`) → round-2 review caught a **regression** introduced by the
>    convergence round (the IR convergence block lost the native optional slot ⇒ programs that
>    previously succeeded by omitting the optional slot were falsely flagged E1010) → fix round (the
>    optional-slot criterion comes from the same `signature.rs:236` sole exit as the parameter name)
>    → round-3 review PASS (independent reproduction red→green; under mutation, the in-repo pinned
>    board FAILED, empirically proving the criterion can fail). **Lesson**: review must cover
>    "un-fixed branches of similar paths" (both `needs_revision` rounds in this batch happened
>    because probes all landed on the hit surface); the fix round itself can also introduce
>    regressions (the optional-slot case), caught by the next round's fresh reviewer.
>
> **3. Process events (recorded faithfully, not pointing at individuals)**
>
> 1. **External whole-tree discard (around 2026-10-14 17:14)**: the working tree was externally
>    restored silently — the implementer's in-progress fixes (typecheck/ir_gen/code
>    table/locales/RFC-013, etc.) all vanished, and **at the same moment two review documents the
>    user themselves had been editing also disappeared from `git status`** ⇒ judged a repo-level
>    discard (most likely a user-side manual cleanup / IDE "discard all changes"). Three pieces of
>    evidence rule out group members: empty stash, no new commits, no `reset` in reflog. The
>    implementer replayed the in-progress work from context and pushed; discipline was changed to
>    "**first save the patch to scratch, then sample the red state**".
> 2. **Verifier ran out of context, member swap**: the independent verifier for 3.16 ran out of room
>    mid-way (no closing message), and per load rules a swap was made — the successor reused the
>    already-ready artifacts (green-state results, corpus equivalence script, mutation plan), and
>    verification ultimately passed. A positive precedent for handing off partial artifacts.
> 3. **Round-5 reviewer rotation and historical-round roll-back**: when rotating the reviewer per
>    the load cap rule ⑤, `remove_member` rolled back already-decided terminal review/verification
>    rounds **into the shared pool** along with them, and the scheduler immediately re-ran them (t3
>    / t8 were re-claimed by new members, causing duplicate work and blocking downstream).
>    Disposition: the lead took over and canceled the rolled-back historical rounds (t3 / t8 / t13),
>    and added AGENTS.md rule 12 (handle rolled-back rounds immediately after a member swap)
>    accordingly. The closeout record for this batch is logged under that rule.
> 4. **scratch governance (~6.3 GB)**: in the early part of the batch, each task had an external
>    tree, with 6 directories scattered outside the repo (~780 MB, cleaned up) plus two members'
>    external verification trees (~2.2 GB / 3.3 GB, self-deleted after use). The output is the
>    governance tool `scripts/dev/scratch.py` (`init` / `use` / `build` / `release` / **trial** /
>    `status` / `rm` / `gc`) and AGENTS.md rules 9 (batch-tree single entry), 10 (verifier
>    environment six rules: mechanically reversible mutation / take line numbers from the referenced
>    revision / CI-only gates must run, etc.), 11 (writer-side load cap), 12 (handle rolled-back
>    rounds after a member swap), and two top-of-file meta rules (no private content / verify before
>    asserting).
> 5. **Contract defect (not pointing at a member)**: the fix-round #454 contract `inScope` only
>    circled 2 files, but the actually mechanically required surface was 8 files (rule 6 is exactly
>    the prevention — the contract template should preset the mechanically required surface list);
>    additionally, the terminal-state field of that task had been occupied by probe payload, and the
>    formal acceptance / command evidence was supplemented by the lead per the convention.
>
> **4. Carry-over items (not fixed in this batch, logged for reference)**: three low-severity
> findings from the round-3 review — ① the method-path `build_arg_slots` (`ir_gen.rs:521`) is still
> a **second copy** of the criterion (existing, per the previous round's practice logged at
> closeout, not converged) — **2026-10-16 pre-release audit added three notes**: (a) it has **two**
> call sites: `ir_gen.rs:7888` (method-binding path, using the bound function `binding.function`'s
> parameter names) and `:7975` (overload-resolution path, using the resolved mixed-name leaf
> segment); the original log only wrote "`ir_gen.rs:521` area"; (b) it has **already diverged from
> the shared implementation `bind_args_to_params`**: missing rule ⑤ (duplicate named → E1015), rule
> ⑥ (missing required parameter → E1010), and the concept of "optional slot", and it hardcodes the
> receiver slot as `1 + i`; both call sites `continue` on an empty slot (the comment says
> "diagnostics belong to typecheck (which guarantees that resolution passing means no empty
> slots)"), a **dangling invariant written in a comment with no assertion backing it**; (c) **the
> "all call sites converge into it" enumeration in RFC-039 D60 missed these two**, a mismatch
> between declaration and fact, and the cited line numbers were not retaken on the referenced
> revision — the D60 text has been corrected along with this note; ② the `cargo fmt --all` error
> inside the batch verification tree (worktree) is a worktree workspace artifact, and the gate
> checklist should note that `fmt` runs on the main repo; ③ the repair-class contract template
> should preset the mechanically required surface into `inScope` (see 3.5).
>
> **5. Final gates (closeout revision, two local implementation commits and one docs commit pushed
> together — the only push in this batch)**: `fmt --check` 0 / `clippy -D warnings` 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / `check-obligations` **PASS (13·13·0·8)** / `check-docs-truth` exit=0
> / `corpus_probe` **342 lines byte-for-byte identical to baseline** (the only regenerated line:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number moved in sync,
> manually reviewed) / IR snapshot zero drift.
>
> **6. Run links (push-side evidence)**: 3.16 implementation push (`cb45de1fc`) → CI **38550504517**
> (8/8 jobs success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (`9ec7d89a8`) → Docs Truth job 38545762349 (the same run's MSRV/Test was cancelled by the
> subsequent push per concurrency — cancel ≠ failure, the later run overrides); governance-tool push
> → CI 38546373435 / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) →
> CI 38528560364. The CI / Deploy Docs run for the closeout push itself is back-filled into the task
> record after the push.
>
> **3.4.8 ③④ landing (2026-10-11, `8ecf56f93`)**: ③ Add `Linking` to the tail of `CHECK_STAGES`
> (**pure merge**, skipping the `is_bin_role` entry validation entirely — the E3020 criterion still
> belongs to `RoleClassification` to prevent double reports; another hard reason: the `run` path
> judges "has main" from the merged IR, and under the collection semantic "entry itself errors → no
> IR" it would be misjudged as missing main and hard-aborted) → `Monomorphization` (shape check);
> `ir_generation_check` retains the unit IR (**memory only**, consumed by pure-merge and mono, not
> emitted / not written to disk / not producing snapshots — without retention, the aggregate request
> set is empty, and mono would falsely greenlight as `NoObligations`); mono failures in the Check
> path go through a per-file collection channel (diagnostics attributed to the entry file +
> `failed_stages`, not set to `failure`); `MultiFile` keeps `fail_with`; `into_outcome` is changed
> to "collect first, then judge failure". ④ Empirically **zero production change**: the
> whole-program BFS already covers cross-unit mutual recursion (pathological → E3005, well-formed
> three-layer chain executes correct values); four e2e groups and one mono unit test are delivered
> as living criteria.
>
> **Verification chain** (all independent of the implementation): t3 (③, 9 self-built fixtures +
> byte-for-byte cross-check with the HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundaries), t10 (F1 fix, 11 cases + dual-binary comparison) all pass; t5 cross-shape matrix
> (single / multi / check three shapes + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6
> review PASS; test count 2643 → 2656 (only increases).
>
> **F1 (discovered during verification, D59 A/B class, fixed in the same round)**: in the standalone
> multi-file `check` without a project root, the aggregate request name (`lib.f`) and the unit IR
> definition key (`f`) do not share a source → mono silently `continue`s on "native has no IR
> definition", and `check` reports 0 error / exit 0. Fix: on the aggregate side, add a lookup by
> each unit's **actual definition key** (exact enumeration + byte-equal + ambiguity drop, not a
> suffix heuristic; the project shape is not registered, existing paths unchanged). **Correction
> (same day)**: that lookup was **deleted** in the subsequent regression fix; the root cause is now
> resolved at the naming layer by **`module_key` decoupling into dual use** (added `script_mode`) —
> IR definition key = IR call name = aggregate request name = `{unit-key}.{name}`. **Regression
> record**: after ③ landed, CI's `Check tutorial/guide code examples compile` went red (258 blocks /
> 10 failures: 6 cases of same-file same-name rebinding + 4 cases of name collision with
> `<std/list>`; rootless `check` does no qualification, triggering the conflict check in
> `link_module_irs`); fix = `ir_gen` splits `script_mode` (qualified name ⊥ Script / Bin, the E3023
> gate now checks it) + `ir_generation_check` per unit always passes through the unit key +
> `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter wins" (consistent
> with the existing `run` path behavior); verification = docs gate 258/0, three-binary cross-check
> (HEAD / ③-tree / fix-tree) for the three collision fixture categories, E3005 does not regress in
> the check shape, E3023 not opened.
>
> **F2 / F3 (accepted)**: mono diagnostics attributed to the entry file's item, span can cross units
> (same caliber as the `run` path's `path=entry`); in the Check shape, each unit holds one
> `ModuleIR` (memory only, released at the end of `check`).
>
> **Pre-existing issues left for follow-up (not regressions in this round)**: ① E3005's help text
> still reads "internal error; please report this issue", which doesn't match the
> resource-protection semantics (precedent: `e3xxx.rs:47` once moved the same kind of "user code
> error" out of E3005) — registered, pending scheduling; ② the span render label of multi-file E3005
> is misaligned (reproduced with the HEAD binary, pre-existing); ③ there is a **type-parameter
> position dependency** on the request-collection side (a generic call whose type parameter is not
> at the value-parameter position does not emit an instantiation request, and mono records
> `NoObligations` — convergent mutual recursion is this shape) — same surface as 3.5 (HKT call
> path), logged into that row.

## External Track RFC-029g: Deleting `pub` and Auto-Binding (#399)

> **P3.5 landing (2026-10-08)**: all seven second-level tasks complete, the entire batch is one
> commit (independently revertible).
>
> - **Measurements diverged from the plan (recorded faithfully)**: the acceptance for 029g.1.2 /
>   029g.1.3 originally said "`pub f = ...` reports a **parse error**" — after deleting `KwPub`,
>   `pub` actually falls back to a normal identifier, and the old form reports **E1001 Unknown
>   variable: 'pub'** at the semantic layer (span pointing at `pub`). Per RFC-039 **D27**
>   ("specifically detecting removed syntax is an anti-pattern"), **no** migration-specific
>   diagnostic is added; the corpus fixture `pub_declaration_err.yx` nails E1001, and the document
>   (`syntax.md`) honestly describes the consequence.
> - **Design convergence (beyond the RFC literal, requires review)**: after `pub` was deleted, the
>   semantics of `project_refs` and `cross_file_refs` are fully identical (the same "set of
>   in-package references", prohibition one forbids parallel representations) → merged into a single
>   `project_refs`, the `set_exempt_pub` / `set_cross_file_refs` switches deleted. Behaviorally,
>   non-`pub` items in the Internal role changed from "file-internal reachability" to
>   "reference-pool judgment" (loosened, in the under-reporting direction); Lib tightened from
>   "absolute `pub` exemption" to reference-pool judgment; Script top-level bindings that are not
>   referenced now report W1001 / W1002 / W1004 / W1005.
> - **Dead-table cleanup**: `env.exports` + `add_export` / `is_exported` / `is_visible` +
>   `auto_bind_to_type` + the `exports` write-back in `typecheck/mod.rs` all deleted
>   (production-side writes, zero repo-wide consumers).
> - **Test migration** (test count does not decrease): added `lexer/tests/rfc029g_lexer.rs` (1); the
>   "pub is recognized" assertion in `parser/statements/tests/bindings.rs` is reversed to "pub is no
>   longer a modifier"; 7 role-semantic cases in `passes/tests/dead_code.rs` rewritten (2 assertions
>   reversed); 8 role cases in `tests/integration/cli_e2e.rs` rewritten (the exemption assertions
>   for `lib` / `internal` reversed into report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` is not part of IR); the corpus differential
>   baseline only adds 1 entry (`pub_declaration_err.yx` → E1001), and the command
>   `cargo run --quiet --example corpus_probe` was run to regenerate and manually reviewed.
> - **Measurements**: lib 2556 / integration 272 / 322-entry corpus baseline all green; `cargo fmt`
>   / `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 landing record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, and the closeout
> (registration / WBS check-off / gate / push) was done by t7. Precondition = 4.3.1 obligations
> ledger (red-state archive 173 files / 533 keys).
>
> **1. Carrier decision (option A: `PlanId` travels with the AST node)**: Added `PlanId`
> (`src/util/plan_id.rs`, `u32` monotonic, unique within a compilation unit, `UNPLANNED = u32::MAX`
> denotes synthetic nodes outside the parser), issued by the **sole allocator**
> `ParserState::alloc_plan_id`; two carrier locations — `Stmt.plan_id` (statement identity) and
> `Expr::Call.plan_id` (call-site identity), with `Expr::call` as the only call constructor
> repo-wide. The rejected alternative was the "two sides each derive the same identity from span"
> positional-coincidence form (D20's original defect).
>
> **2. Two-level verification and review (members independent)**: each functional commit is paired
> with an independent verifier and a reviewer, **verifier/reviewer do not reuse the implementer's
> evidence**: 4.5.1 = t1 implement / t2 independent verify (external tree + 5 mutations) / t3
> review; 4.5.2 = t4 implement / t5 independent verify (external tree + 5 mutations) / t6 review.
> The review point in time and the reviewed revision both fall on immutable history (verifiable via
> `git show`). **Provider rename caused t5 / t6 to swap members for continuation**: the original
> verifier / reviewer members could not continue due to a provider rename, and per the orchestration
> convention, a swap was made and **the reason is written into the record** (cross-round context
> preservation is preferred; only compress when unusable); after the swap, the conclusion was
> consistent with the original round (PASS), and the implementer's evidence was not reused.
>
> **3. Two defects self-discovered and fixed during implementation (t1, both within `inScope`)**: ①
> The consumption-exemption criterion was once attached to `UnitState.ir.is_some()` — in the `check`
> shape, Linking / Mono take `unit.ir` away, and real non-consumption in the `check` shape
> **silently passes** (mutation measurement: D41 red while CLI `check` exits 0); changed to read the
> **ledger's own completion marker**, and a regression case was added. ② The Drop sequence drifted
> with the process hash seed (partial order `sort_by` + `HashSet` iteration): the snapshot criterion
> ran three times in a row producing line-count drift 106 / 120 / 114; after switching to
> **total-order sort** (same name takes max depth + name ascending) two consecutive runs were
> zero-drift.
>
> **4. Criteria and evidence numbers**: after 4.5.1, `test_release_plan_spans_consumed` removes the
> persistent `#[ignore` (floor `examined ≥ 173` + no whitelist), 172 IR snapshots regenerated in the
> same commit (programmatic review: 1083 new lines of Drop + 86 lines of jump-index shift, zero
> deletions, no reordering), the 342-entry corpus differential is empty; 4.5.2 added
> `test_overload_resolutions_consumed` (empty difference set + evidence surface floor `examined ≥ 1`
> / `consumed ≥ 2`; measured `examined=1`, consumption items 2, no skipped resolution 210), 9
> resolution-shape pinned boards, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero
> drift**. Mutation-driven: shift the consumption key by one → criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate divergence; restored to green.
>
> **5. Classification basis for the semantic fix (D59)**:
> `Type.method(positional args, name = value)` before the change **silently gave the wrong answer**
> (measured to compile cleanly, run output `1` instead of `20`) ⇒ **class C** (old behavior was not
> successful), and the fix takes the "loud" path: after the change the same program produces
> `error [W1090]` at compile time; the terminal semantic of "named actual parameters participating
> in resolution" is **class D**, already registered in §3.14 and left as issue #452 for a dedicated
> team to take. Additionally: `Drop` is a no-op in the interpreter
> (`backends/interpreter/executor/ops/control.rs` wildcard arm is only `advance()`) — key migration
> does not change runtime behavior.
>
> **6. F-1 / F-2 disposition (found by t5 independent verification)**: **F-1 (medium)** the unbound
> consumption point (`ir_gen.rs:7766-7790`) is unreachable in the current corpus and test surface,
> and the key-controlled correctness has no red state that can be falsified ⇒ per "known boundary",
> register into [02](02-stage-contract.md) §2 revision note fifth paragraph (preservation
> rationale + no new task); **F-2 (low)** pointer drift ⇒ the `OverloadResolutions` doc comment in
> `types.rs` is changed to precise pointers (produced at `inference/expressions.rs:4550`, consumed
> at `ir_gen.rs:7697` / `:7772`), as a separate small commit, not mixed into the docs commit.
>
> **7. Closeout (t7)**: the two obligation-field rows in 02 are changed to `PlanId` keys + current
> consumption-point line numbers (`:360` read / `:3103` use / `:1095` and `:1097` settlement
> marker); the criteria table in 07 and defect class 3 in 04-ssa are changed to past tense; 09
> checks off 4.5 / 4.5.1 / 4.5.2; CHANGELOG logs a one-line behavior change; `check-obligations.py`
> and `check_tracking.py` are green, CI and Deploy Docs are green. **Closeout-discovered bookkeeping
> deviation**: the pointers `ir_gen.rs:357` / `:3102` / `:1096` in the t3 / t6 reports are one to
> three lines off from the code fact (actually `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2's
> insertion) — registered as the code fact dictates, and this note is kept as a trace.

## P4 4.10.3 Standalone `run` and MultiFile Orchestration Unification (the `run` half of 4.2.5 decision A)

> **丙 batch record (registered 2026-10-15 → closed 2026-10-15)**: On 2026-10-15 the user ruled 丙 —
> this round only does 4.10.3 (the same-batch candidate 3.4.6 is deferred to after 4.9 per B1
> ([02](02-stage-contract.md) §Runtime Behavior Table "dead-code family warnings do not enter the
> `compile` path"), the precondition has been re-attached and noted); implementation form =
> **fully-integrated single team** (user's instruction: no DAG, no team building, single-session
> implementation + self-verification). Pre-batch debt landed first: #455 dependency announcement fix
> (rustls 0.23.46 + serde_yaml_ng, `cargo audit` recovered from exit 1 to exit 0), WBS registration
> debt (#456 registered as 3.17 + incomplete rows filled in with "owning stage / whether it blocks
> the main line" fields).
>
> **1. Red before green**: 3 driver-level red criteria (`src/driver/tests/run_rootless.rs` —
> rootless merged IR must not carry an `entry_function` (red state `Some("main.main")`), rootless
> top-level executable statements must not report E3023 (red state produces no module), Bin control
> `entry_function` preserved (persistent green state)) + 3 CLI e2e pins (standalone relative import
> `run` (red state E5001), Script top-level statement execution, and two persistent green pins for
> the #413 hint).
>
> **2. Implementation surface**: CLI `run` uniformly goes through `compile_project` (the project /
> standalone fork in `run_file_with_diagnostics` deleted); the `entry_function` in Linking is gated
> by `is_bin_role` (#413 T4 — Script does not auto-call `main`); the #413 hint keying changes from
> `module.source_files.is_empty()` to a call-site project probe (after unification, `ModuleIR`
> always carries `source_files`, so the original marker is invalid); `script_main_never_called`
> adapted to the qualified-name form (function `{key}.main`, the global slot is measured to be the
> same qualified name — IR evidence is authoritative, not inferred from comments).
>
> **3. Pre-existing gaps exposed by unification (fixed along the way, A/B/C class — only turn
> failing programs into successful ones, zero impact on previously successful programs)**:
>
> 1. **Explicit type-argument pre-specialized name** (`mk(Int)(5)`): `ir_gen` directly emits the
>    pre-specialized name per D6.3 (mono will strip the un-specialized original), and the three arms
>    of `resolve_call_target` (rename / aliases / dot prefix) don't cover the `mk(int64)` form ⇒ the
>    call site is dangling with E6006. Fix = this single resolver point adds a fourth arm (base-name
>    qualification, type-parameter strings kept verbatim). **Same shape reproduced in the Bin
>    project** (pre-existing defect, not introduced by this batch).
> 2. **Script top-level `mut` assignment statements duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (an `Assign` of
>    the same shape as the declaration) statement by statement ⇒ the same variable gets 3 slots, the
>    initialization writes to the later-allocated slot but the read uses the earlier-allocated one
>    (Add type mismatch at runtime). Fix = the two locations merge by name (same criterion, same
>    source).
> 3. **Same-module constrained generic operator dispatch** (RFC-011 §5.2): mono dispatch rewriting
>    produces the bare name `Vec3.add`, while the merged table actually holds `{key}.Vec3.add`. Fix
>    = `qualify_deferred_method` deterministic candidate + table lookup validation (exact hit →
>    qualified hit by the generic's owning unit key → fall back to bare name; not a suffix
>    heuristic, same form as `resolve_generic_name`).
>
> **4. Registration gap (not fixed in this batch)**: constrained dispatch still produces E6006 when
> the **type and generic definition are in different modules** (verified by a rootless two-file
> probe; same shape in Bin) — root cause = `MonoType` user type names do not carry the defining
> module, the proper fix belongs to P6 type-identity convergence; registered as WBS **3.18** (owned
> by P3, precondition pending P6, does not block the main line), with the same surface noted in
> `qualify_deferred_method`.
>
> **5. Test adaptation (honest recording)**: `tests/integration/multifile.rs` fixture was
> supplemented with a manifest — what it tests is RFC-029 project orchestration (Bin auto-entry)
> rather than Script semantics; one pin in `cli_e2e` (top-level diagnostic variable-name visibility)
> and the `script_main_hint` fixture (7 cases) were synced to the qualified-name form — the
> production consumer side only eats the merged IR, and the fixture drives the same form directly,
> no need for a pipeline-product stand-in.
>
> **6. Final gate (closeout revision `084eaf87b`)**: `fmt --check` 0 / `clippy -D warnings` 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> `check-obligations` PASS (13·13·0·8) / `check-docs-truth` exit=0 / `check_tracking` exit=0 /
> boundary three gates report-only / `markdownlint` 0 / `pnpm docs:build` 0 / IR snapshot zero drift
> / corpus differential baseline regenerated **12 lines** — structured per-line check: exit / stdout
> / diagnostic codes all match line by line, only stack-frame function names and variable names are
> qualified by the unit key (passed manual review); after regeneration, `check-corpus-parity.py`
> exit=0.
>
> **7. Process event (recorded faithfully)**: in this batch, during the pre-commit hook run, the
> workspace docs were concurrently edited, and the hook's stash rollback consumed them and the
> commit failed (a concurrent form warned about in the config comment) — discipline = no editing of
> the workspace while the hook is running; the #455 commit was therefore split into two attempts
> (the second was fully green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closeout)

> **4.6 batch record (registered 2026-10-16 → closed 2026-10-16)**: On 2026-10-16 the user directed
> progression in the order 4.6 → 4.9 → 4.4 → P5, starting with 4.6; implementation form =
> **fully-integrated single team** (following the previous batch's user instruction: no DAG, no team
> building, single-session implementation + self-verification). Task definition (09 three-level
> table): project-internal files' semantic data (SemanticDB) goes through the orchestrator's same
> pipeline as diagnostics, with the same source; cross-file references carry `resolves_to`.
> Acceptance: project-internal cross-file navigation hits the definition; semantic data and
> diagnostics come from the same compilation.
>
> **1. Current-state verification (verify before asserting)**: `didOpen` / `didChange` triggers
> **two** compilations — diagnostics go through the unified Driver pipeline (already unified in
> 4.2.3 / 4.2.6), and semantic data goes through the manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` in `src/lsp/server.rs` (no module
> registry ⇒ project-internal `use` resolution fails, cross-file reference events are not produced,
> navigation / rename / reference-find silently fail on imported symbols — the silent failure
> registered as D55). The orchestration arm's checker uniformly names `"<module>"` (the SemanticDB
> file identity is lost); `type_result` (containing `semantic_db`) is dropped when diagnostics are
> non-empty and never emitted; the checker.rs's reference-drain unconditionally overwrites the
> `resolves_to` DefId's file identity to the current file; neither `Export` nor `ModuleInfo` has a
> definition-location surface.
>
> **2. Red before green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`) — after `didOpen` on
> `main.yx` in a temp two-file project, send `GotoDefinition` for the `add_one` call site; **before
> the fix it returns None (red, empirically the silent failure described above)**, and after the fix
> it hits the definition at line 0 of `lib.yx` (green). One probe localization during the process:
> the compiler-side chain (driver probe dump) was already fully connected, but the e2e was still
> empty — the root cause is that `resolve_reference` does an exact match on the reference **start
> point** (D55's current precise-only convention), and the test cursor landed in the middle of the
> identifier; this tolerance is an existing behavior surface and is not touched in this batch (see
> 7).
>
> **3. Implementation surface**: (1) `Export.definition_span` (harvested in `extract_module_info`
> from the AST identifier node, same source and value as the typecheck definition event's
> `def_span`; native `std` is always `None`) + `ModuleInfo.definition_file` (filled in the Registry
> arm from `DiscoveredFile`, includes vendor; `std` always `None`); (2) the checker's reference
> drain uses `import_def_locations` (the `use` line binding span → definition file + definition
> span) to redirect the cross-file imported item's `resolves_to` out of the current file; std
> imports are not registered and fall back to the `use` line as before; (3) the checker's module
> identity is changed from the unified `"<module>"` to the unit file path (empirically all 4
> consumer points of `env.module_name` are diagnostic-free); (4) the `DriverOutcome.semantic_db`
> channel — collected in `CollectAll` form (LSP target file **regardless of diagnostic success or
> failure** + `SingleFile` fallback) into the pool, and the LSP arm tail injects non-target files'
> top-level export definition surface from the registry (`external_definitions`, so even un-opened
> files can be navigated; type / submodule exports are not injected — the typecheck side's type
> registration does not produce definition events, and both sides keep the same empty area); (5)
> `orchestrator::analyze_source_in_project` (re-routes `check_source_in_project` to take the
> diagnostic half); (6) the LSP `analysis.rs` single compilation entry replaces the manual pipeline
> (`update_semantic_db` deleted), the World key uniformly normalizes the file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive-letter case and `%XX` encoding
> differences normalized to the same key, cross-file DefIds therefore hit stably); 5 handlers + the
> `semanticTokens` dual request connect to `db_key`.
>
> **4. Drive-by fixes (D59 split B class — old behavior silently did nothing, previously successful
> programs are externally unaffected)**: ① project-internal file cross-file navigation / rename /
> reference-find changed from silently failing to hitting the real definition; ② the old manual
> pipeline deleted semantic info on lexical errors (highlighting disappeared) → the malformed AST's
> highlighting is preserved (extension of the 4.2.6 editor philosophy); ③ when the target file has
> type errors, semantic data is no longer dropped along with `type_result` (typing-in-progress
> highlighting / navigation does not disappear).
>
> **5. Pins**: 6 driver-level (`src/driver/tests/lsp_semantic_db.rs` — target-file path key +
> same-source diagnostic channel, cross-file `resolves_to` + external-definition surface round trip,
> std import falls back to `use` line, errored file retention, single-file fallback path key,
> single-file error retention) + 1 LSP e2e (red before green) + 4 URI normalization
> (`src/lsp/tests/uri.rs` — drive-letter conversion, space encoding, idempotent normalization,
> non-`file` scheme passthrough).
>
> **6. Mutation verification (`scratch.py trial`, mechanically reversible)**: batch tree `wbs-46` @
> closeout revision `39546020`; both mutations made the hit pin turn red, and after restoration the
> blob matches HEAD byte-for-byte + `git status` CLEAN — m1 removes the `import_def_locations`
> registration (`statements.rs`) ⇒ cross-file driver pin + LSP e2e pin turn red, the other 7 pins
> stay green; m2 removes the `external_definitions` injection (`arms.rs`) ⇒ the same two pins turn
> red (round-trip resolution loses anchor), the other 7 stay green. The batch tree has been `gc`-ed
> (released 4.8 GB, `status` shows no batch tree).
>
> **7. Final gate (closeout revision `39546020`)**: `fmt --check` 0 / `clippy -D warnings` 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> differential **byte-for-byte zero drift** / IR snapshot zero drift / `check-obligations` PASS
> (13·13·0·8, the consumer anchor for `semantic_db`'s non-obligation reason is synchronized with
> `server.rs:482` and the 02 §Obligation Field Table) / `check-docs-truth` 0 / `docs-examples` 258
> entries 0 failures / `docs-orphan` 0 / `markdownlint` only two MD056 in the user's unsubmitted 032
> remain in the whole repo (user's own edits, untouched) / `pnpm docs:build` 0. **Process event
> (recorded faithfully)**: an MD037 at `en/09a-wbs-history.md:54` was introduced by the
> auto-translate bot when re-translating the previous batch's 09a (no space in the zh original) and
> was fixed in this batch — `markdownlint` red was not on this batch's change surface but is in the
> same file as 09a, and is recorded together. **Carry-over observation (registered for discussion,
> not touched)**: `resolve_reference` only does exact match on the reference start point — when the
> cursor lands in the middle of the identifier, navigation / hover / rename silently does nothing
> (D55 precise-only current state, same form across the file); if tolerance matching is required, it
> is an independent behavior change and is reported to the user for a decision.
>
> **Same-day follow-up batch (4.6.2, landed after user decision)**: after the wrap-up report was
> delivered, the user ruled on "carry-over observation ①" with "yes, then change it" — the
> lookup-key tolerance was implemented per plan A: the four handlers' (definition / hover /
> references / rename) lookup keys changed from the cursor position (after UTF-16 conversion) to the
> **identifier's own start point** located by `find_identifier_at_position` (same coordinate system
> as the semantic event, no conversion); `resolve_reference`'s start-point exact-match semantics are
> unchanged (D55 precise-only unchanged). The definition-point fallback decision for `references` /
> `rename` (`start <= col < end`) holds under the same form for the identifier start point. Red
> before green: the middle-cursor pin for `definition` first red (None) then green; one
> middle-cursor pin for each of the four handlers + cross-file e2e start-point / middle-point
> dual-position assertion; `--lib` 2748 (+4) all green, `clippy -D warnings` 0. Registered as WBS
> 4.6.2 (three-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 1 Closeout (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 batch record (registered 2026-10-16 → closed 2026-10-16)**: On 2026-10-16 the user decided
> to merge the three related three-level tasks 4.9.1 / 4.9.2 / 4.9.3 **into one team for one round**
> (reason: they all belong to "the pool's source and identity" side; merged landing avoids repeated
> rewrites of the intermediate state after `collect_project_refs` is deleted; 4.9.4 / 4.9.5 belong
> to consumer refinement and test-pool separation, depend on the `use` path resolution from 4.6.1,
> and forcing them into the same batch would widen the gate surface). #450 ownership **pending
> decision** — the evidence points to "overload-resolved name not passed back to the reference
> count" class, and it still reproduces under the form landed in round 1; per user decision 丙 it is
> dispatched. Frozen rev = `7e62b0454` (parent commit `492f25eef` is the pre-fix baseline);
> verification PASS attempt `aab92fe0` (i.e. t2); review pass attempt `96452696` (i.e. t3). Evidence
> archived at `target/yx-scratch/yx49/EVIDENCE.md` (containing 8 mutation patches and
> `warn_compare_v8.json`).
>
> **1. Merge decision**
>
> 1. **The decision itself**: three related three-level tasks merged into round 1. **4.9.4 (consumer
>    refinement) / 4.9.5 (test-pool separation) next round** — the former depends on the `use` path
>    resolution delivered by 4.6.1 (already landed at `39546020`, can start); the latter needs to be
>    in the same batch as the RFC-029f table absorption, registered at the end of RFC-029f pending
>    change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutual preconditions (4.9.2 changes the pool type →
>    4.9.1's assembly side must be rewritten → only then does 4.9.3 have the role table for
>    denominator judgment); doing them in separate rounds would necessarily write an intermediate
>    state and then change it twice; (b) all three belong to the "pool's source and identity" side,
>    while 4.9.4 / 4.9.5 are another side, and the split is by "cohesion" rather than "independently
>    revertible".
> 3. **Merge result**: a single commit `7e62b0454` (one line `:recycle: refactor(driver)`), a single
>    push, a single batch of gates.
>
> **2. Red / green measurement criteria for the five criteria**
>
> | #   | Criterion                                                    | Red state (pre-fix / P1 directory-scan replay patch)                                                                                                               | Green state (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                                                |
> | --- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool root = package target (#334)                            | Directory recursive scan ⇒ identifiers in files under `tests/` also enter the pool, exempting unused production-code definitions                                   | `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` per `ExplicitSurfaces` root; full-root degradation falls back to `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                                                         |
> | ②   | Pool granularity = `(module, name)`                          | Same-named local variable (`let foo = …`) accidentally exempts the same-named top-level definition (`fn foo()`); `foo` in a.yx and `foo` in b.yx cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) is the sole assembly entry; `arms.rs:603-609`'s `other_module_refs` takes the union of all other modules' entries and sends them to `DeadCodeAnalyzer`; the module's own names (top-level definitions and local bindings, with `use`-imported names as exceptions) no longer exempt same-name definitions in other modules |
> | ③   | `FileRole::Script` does not use the pool                     | Script is affected by references from other files in the project                                                                                                   | `arms.rs:611-623`'s `match us.role` only assembles for `Bin` / `Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`); Script / Test go through the `_ => {}` default arm (judged by the file-internal definition-use graph)                                                                                                                                                                                       |
> | ④   | Fixture A: definition at line 4 of lib.yx must report W1001  | Pre-fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                                   | Red-state measurement (red 1 fixture `red1`) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                                             |
> | ⑤   | Fixture B: definition at line 4 of main.yx must report W1001 | Pre-fix: reports `W1003@1`                                                                                                                                         | Same as above                                                                                                                                                                                                                                                                                                                                                                                                               |
>
> **"Reasoned red" for criterion ②** (self-stated by implementer, closed by verifier): the
> `let foo = …` in the red2 fixture is in `helper.yx`; under the old pool's bare-name union, `foo`
> is treated as a whole as "used by helper", and `foo` in `lib.yx` is exempted. **Measurement**:
> red2 fixture vs. pre-fix binary → no W1001 (old-caliber exemption); red2 vs. post-fix binary →
> W1001@1.
>
> **Criterion ③** (Script does not use the pool) red-state fixture **first version failed**: the
> first version of the fixture placed a same-named local variable in the same file, triggering E2010
> (the language does not allow same-module shadowing of a top-level definition); after moving to
> another module (`helper.yx`), it became a reasoned red state. Both t3's review and t2's verifier
> independently reproduced this, **not a contract defect** — same-module shadowing is a hard
> constraint of the language, and the fixture must cross modules.
>
> **3. Correspondence between the +8 and +16 counts** (**as established by t3's review**)
>
> The C section of `target/yx-scratch/yx49/EVIDENCE.md` writes `+16/−0`, but omits the **"new
> fixture records"** component — the v8 instrument (`warn_compare_v8.json`, 331-file symmetric diff,
> `only_before=0`) convention **includes** new fixtures; the t1 implementer's self-built "+8" is
> **warn-before / warn-after (325 entries each) aggregated by fixture** (each member file's `check`
> runs once). The review maps these line by line:
>
> - **+8** (8 new warnings, all `check <non-entry-file>`, `check <entry-file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: these 8 fixtures all **did not declare a package target** ⇒
>     4.9.1's root degrades to the checked file's own compilation set, and the consumer `main.yx` is
>     not in it; this is a designed-in change (before, these files' `check` surface was already
>     zero-warning, not an exemption path). The repo-root `yaoxiang.toml` is empty ⇒ the same
>     degradation path is empirically valid.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check-surface files have `exit=None` in t1's `warn-before.jsonl` (the sampling point in
>     time **does not exist** — these are new fixtures added in this batch). So the "+8"'s "+ 0 − 0"
>     differential does not include them, while the +16 v8 symmetric differential does.
>   - In `v8`, only `W1001@2(probe)` remains for `stray.yx`: the reason is that the pre-fix binary
>     also reports `probe@stray.yx:4` (the fixture runs `probe` once per file), and after
>     **cancellation** only one lib entry remains — the three data sets (+8 / +16 / dual-binary
>     ABBA) are mutually consistent.
> - **8 new fixture entries** vs. **+16 residual 8 entries**: the same batch of files
>   (`ref-pool-compile-set/` and `ref-pool-module-identity/` newly added in 4.9.1 / 4.9.2), the
>   instrument treats each of their `check` runs as an independent record — the same file may
>   trigger different diagnostic entries for multiple `role / context`.
>
> **Conclusion**: **+16 = +8 (aggregated by fixture) ∪ 8 new fixture entries** — both counts are
> correct, **different conventions**: t1's "8 new entries" is the fixture-layer convention, t2's
> "+16 entries" is the file-layer convention (symmetric diff includes new fixtures). The review's ①
> +8⊆+16 is established; **review ② writes "do not copy EVIDENCE.md's C section verbatim"** — C
> section omits the "new fixture records" component, and this entry is rewritten based on t3's
> review `pass` conclusion.
>
> **4. Mutation experiments (mechanically reversible, all via `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b `use` imported name
> removal, p3b Script access to the pool, p3 flat consumption, p4 std access to the pool, p7final
> self-built 4 criteria, p8 std close, p9 vendor access to the pool + `p_stack2` / `p_stack3`
> stacking surfaces). Each trial output contains: HEAD blob → mutated blob → restored blob matches
> HEAD = yes + `git status` CLEAN. The local `core.autocrlf=true` requires `GIT_CONFIG_*` injection
> of `false` (`trial_env.ps1`), otherwise the patch's new lines become CRLF and both apply / restore
> fail — **only affects the mechanical nature of the patch, not the tested behavior**.
>
> **5. Process events / environment lessons (must read for handoff to next round)**
>
> 1. **`core.autocrlf` injection**: the local `core.autocrlf=true` must have `false` injected via
>    `GIT_CONFIG_*` to keep the patch mechanically reversible; see
>    `target/yx-scratch/yx49/trial_env.ps1`. **Do not reuse** any trial script that lacks the
>    injection in the next round.
> 2. **Same-tree `trial` must be strictly serial**: `trial` and `cargo build` **must be strictly
>    serial** on the same tree. `trial`'s internal `git apply` and `cargo build` both touch the
>    worktree; in parallel you get two kinds of races — "patch not yet complete but already seen by
>    build" or "build and apply interlock".
> 3. **Confirm sync when `trial` rebuilds the binary**: after `trial` completes it rebuilds the
>    tree's binary (cargo's fingerprint includes the source path, **switching tree = full
>    rebuild**); before subsequent measurement you must confirm `bin/yaoxiang-rs` is in sync with
>    HEAD. **This round once used a residual mutation binary to measure fake data**, which was
>    corrected by rebuilding and re-measuring.
> 4. **Reason for the second tree `yx49-b`**: cross-comparison needs the pre-fix binary
>    (`492f25eef`), which is not available in the batch; `scratch.py init yx49-b` opened a second
>    tree (eating another full build). **The tree has been released** (see 6), and the deletion is
>    left to the closeout's `gc` for handling.
> 5. **Reason for supplementing the two Internal-case fixtures in `cli_e2e` to be a legal package
>    (adding `[run].main`, assertions untouched)**: the pre-fix `cli_e2e`'s two `Internal` role
>    pinned-board fixtures **did not declare `[run].main`** ⇒ after 4.9.1 landed, `compile_project`
>    went through the `ExplicitSurfaces` root-call path and could not find `main` ⇒ the entire batch
>    of 2 fixtures' behavior drifted from "Script fallback" to "entry not found" — this is a
>    fallback behavior introduced by 4.9.1, not a defect. After adding the `[run].main` field, the
>    two fixtures go through the normal package path, and the **assertions** (exit code / diagnostic
>    code / warning set) are unchanged verbatim.
> 6. **Safe way to delete the second-round tree**: after `release`, use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`); it will verify "no mount" before
>    deletion. It has been released before the batch closeout, and the next step is `gc` to clean up
>    naturally.
>
> **6. Batch tree state**
>
> Before closeout: `yx49` has been released (4.8 GB freed), `yx49-b` has been released. Neither tree
> has a mount, and the status can be re-checked via `scratch.py status`. The task output ends with
> the full `scratch.py gc` status.
>
> **7. RFC-029f "Known Defects" table flip**
>
> The previous version (2026-10-09) described "three design defects in the reference pool" in the
> present tense (including deleted code locations such as `collect_project_refs` /
> `orchestrator.rs:603-648` / `orchestrator.rs:393-396`). The closeout of this batch synchronously
> rewrites it to "**known defect: landed (2026-10-16, WBS 4.9 round 1)**": ① the current pool's root
> = `collect_package_refs` (`helpers.rs:481`), resolved per `ExplicitSurfaces`, no directory scan; ②
> the current pool type `HashMap<String, HashSet<String>>` (key = module key), the sole assembly
> entry `collect_ref_pool` (`helpers.rs:429-449`), the consumer `arms.rs:603-609`; ③ the current
> Script does not access the pool, `arms.rs:611-623`'s `match us.role` default arm. **All
> `file:line` are retaken on the referenced revision `7e62b0454`**. `collect_project_refs` and
> `orchestrator.rs` have been split per P4 4.2.9 (`c59fb751`) and moved into
> `src/driver/{state,arms,helpers}.rs`; the old locations in the table no longer hit — this is a fix
> of "declaration not matching fact" (t3 review note ①).
>
> **8. Final gate (closeout revision `7e62b0454`)**
>
> `fmt --check` 0 / `clippy -D warnings` 0 / `cargo test --lib` 2754 passed / `--test integration`
> 287 passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new
> fixtures' behavior run, 338, a339, 340; the other 342 lines are byte-for-byte unchanged) / IR
> snapshot 204 entries zero drift / `check-obligations` PASS (13·13·0·8) + selftest /
> `check-docs-truth` 0. **Process event**: in this batch one `cargo` build converted script line
> endings to CRLF due to `core.autocrlf`, and `yaoxiang-rs` under the team scratch path reported
> `BadImage` on startup, empirically due to encoding inconsistent with the git index — passed after
> rebuild (parallel with the 4.6 batch's "external whole-tree discard" lesson, but with a different
> cause).
>
> **9. Carry-over (handoff to the next round)**
>
> 1. **#450 ownership decision**: the evidence points to the "overload-resolved name not passed back
>    to the reference count" class, **and it still reproduces under the form landed in round 1** —
>    per user decision 丙, it is dispatched. **#450 probe**: the dual-binary same set
>    `{W1002@2, W1005@4}` (empirically, rootless form) has no direct relationship with the round 1
>    changes, but the fix for #450 may need to wait for 4.9.4 (consumer refinement) — because the
>    root cause is "passing the overload-resolved name to the reference count" which needs the
>    precise consumption surface of the `use` item, and 4.9.4's "actually used after import" is the
>    precise version of the same criterion.
> 2. **`dead_code.rs:683` pointer nit**: the comment reads "the two pools' assembly convention is in
>    `collect_compilation_refs` on the driver side" — but `collect_compilation_refs` is actually the
>    fallback path, and the main path is `collect_package_refs` (`helpers.rs:481`). The comment is
>    out of step with the code's main path (**not in this batch's write scope**); it is left to be
>    corrected when 4.9.4 / 4.9.5 land.
> 3. **4.9.4 / 4.9.5 preconditions unlocked**: 4.6.1 (`use` path resolution) has been delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test-pool separation) needs to first settle the
>    RFC-029f table absorption (paved by this batch's "7.").
> 4. **`yaoxiang check` multi-file `run` never reports dead-code-family warnings** (WBS 3.4.6,
>    deferred to after 4.9 per user decision 丙 on 2026-10-15) — not covered in this round 1; to be
>    handled in the same batch when 4.9.4 / 4.9.5 land in the next round.
