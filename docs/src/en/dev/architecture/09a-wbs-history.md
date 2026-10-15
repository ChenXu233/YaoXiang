# WBS Historical Notes (Implementation Record · Audit · Ruling Traces)

> This document is a **historical note** to [09-execution-wbs.md](./09-execution-wbs.md) (the
> execution task table): implementation records, audit findings, ruling traces, and errata for each
> phase. **Division of labor**: 09 describes only the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document records only the **history** (what was done, what
> was found, how it was ruled), appended chronologically without rewriting history.

## P2 Establish Equivalence Criterion Baseline

> **P2 landed (2026-10-06, 2.1)**: verify.rs dual mode + 7 invariants + full corpus run green. **D38
> prediction fulfilled** — the run-green process empirically measured and fixed three real defects
> in ir_gen (see 06 §G6 for details): the locals table in methods was truncated to the parameter
> count (local_count underestimation, runtime relied on Frame::set_slot for fault tolerance),
> unannotated parameters were dropped by filter_map (three locations, params metadata misaligned
> with frame layout), and the match result slot depended on the frame's implicit Void initialization
> (now explicitly pre-initialized). The actual semantics of jump targets are flattened instruction
> indices (not the label-keyed form in the 07 text), and the CFG is built at the instruction level —
> recorded in the verify.rs file header. **Commit correspondence**: ir_gen fixes = `a9ea199b`;
> verify main body = `d5b18390` (⚠ a parallel-stream COMMIT_EDITMSG collision caused its message to
> be mis-labeled "docs(rfc): RFC-027a…", while the content is actually verify.rs + six test files,
> buried irreversibly in history, recorded here).
>
> **P2 narrowing parts landed (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place; **P3 hemostasis
> channel prerequisite unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (4 behavior + 1 compile-error) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract shares the same `TestFileSpec` (single
>   point on the library side) as the single-file corpus, with binary location / subprocess launch
>   abstracted into `tests/common/` and yx_runner migrated accordingly (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed down by the rfc027 test (return site / call site / binding site) —
>   the 02 `Sorted(3)` annotation example would be evaluated by substitution in the current code and
>   directly Disproved, not silently Unproven; the 07 pseudocode's `Sorted(3)` is illustrative only.
>   The red-state empirical data is in the test file header.
> - The two red criteria are pinned with `#[ignore]` as standing entries per the 2.4.5 red-skeleton
>   precedent: the default suite is all green, `-- --ignored` reproduces the red state; **the
>   `#[ignore]` attribute is removed when P3 fixes land to turn green**.
> - The reason for the deliberate deviation of comparison normalization from the 07 (code, file,
>   line) table (dropping file/line, comparing only error code sets) is written in the 2.4.1 test
>   comments.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **moved out of P2**: the `Program` / `Obligations` types they reference do not exist until P4,
> so P2 cannot compile. The former becomes P4's 4.1.4 (no double registration), and the latter's
> `#[ignore]` red skeleton merges into 4.3.1.
>
> **2.3.3 landed (2026-10-07)**: Item-by-item check of the C4 mandatory coverage list (07 §143), 10
> semantic corpus categories all covered and within the corpus-parity.jsonl diff baseline (26
> representative files 26/26 hit, verified by grep) — ref/borrow/move the entire 05-ownership
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for and 16 other files, sum types
> sum_type_* + pattern_or_guard + match_call_scrutinee, existential coercion forced at
> interface_dynamic_dispatch (Vec(Animal) heterogeneous existential-type wrapping triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (plus
> method_overload_ambiguous_err on the diagnostic diff surface), refinement constraints Drop
> sequence refined_* ×5 + 05-ownership driving ReleasePlan (span-key contract has another standing
> red criterion 2.4.4). Performance baseline `benches/pipeline.rs` committed (07 §145 second of
> three smoke tests): `corpus_compile_frontend` first measurement of full corpus compilation 17.6s,
> `cli_cold_start_hello` process cold start 169.7ms; interpreter throughput is already covered by
> the benches/lib.rs hotpath group (interp_fib_recursive_27 / interp_loop_steady_10m) and is not
> duplicated. Regression gating check-perf-regression.sh is unified under the P9 checklist (2.5).

## P3 Fix Correctness Bugs (Minimum Plan)

> **Already-landed rows tagged with error type** (retrospective tagging, does not alter history):
> 3.1 = B ｜ 3.2 = B (defensive empty arm, criterion turn-green borne by 3.1) ｜ 3.3 = A ｜ 3.4.1 /
> 3.4.2 = A (new W codes) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D
> (user-visible surface, already user-ruled) ｜ 3.4.8 ①② = C (primary) + A (incidental three
> locations) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all already user-ruled).
>
> **P3 diagnosis (2026-10-12, user-requested)**: A diagnostic accounting triage in response to the
> perception that "P3 entries keep growing."
>
> **I. First clarify the scale (perception is amplified by two accounting defects)**
>
> 1. **Numbering collision**: `3.5.x` is shared by two task families — (a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 third-level items: lexer and parser branch / AST `is_pub` / checker
>    side / dead-code exemption convergence / external echo / std and diagnostic copy / language
>    reference convergence, all checked); (b) 2026-10-10's registered **3.5 higher-kinded type
>    parameter call-path completion** (3 third-level items). The two share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 renumbered per user ruling: 029g family changed to `029g.x`**), and
>    causes 029g's rows to be miscounted into P3. **Pending ruling**: rename the 029g family to
>    `029g.x` (or move it into its owning phase), and synchronize all other references in the
>    document.
> 2. **Missed checks**: `3.7`'s 4 third-level items (3.7.1–3.7.4) are all completed (d922a551 /
>    38bd4b9e) but unchecked ⇒ this entry is corrected here.
>
> **II. P3 actual accounts (12 second-level rows)**
>
> | Row                                                                    | Status                                                                                                                                                                             | Disposition suggestion                                                                                                                 |
> | ---------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                        | Completed                                                                                                                                                                          | —                                                                                                                                      |
> | 3.4 Silent-drop additions (8 third-level)                              | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **doable now**; 3.4.4/3.4.5 unlock with 4.3.1                                                                                    |
> | 3.5 HKT parameter call-path completion (3 third-level)                 | Not started (declaration side live, call side broken)                                                                                                                              | Keep in P3; **does not block the refactor main line**, but is an "accepted feature not implemented", higher priority than pure cleanup |
> | 3.6 Function-type value call path (1 third-level)                      | Not started (semantics already cut (i))                                                                                                                                            | **Keep**: correctness defect, clear criterion                                                                                          |
> | 3.7 init-segment call name qualification (4 third-level)               | **All completed**                                                                                                                                                                  | Check and close this round                                                                                                             |
> | 3.8 native shadowing (1 third-level)                                   | Not started                                                                                                                                                                        | **Stuck on decision** (define name-resolution priority first), not a scheduling issue                                                  |
> | 3.9 / 3.10 / 3.11                                                      | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                                      |
> | 3.12 Executor-config propagation between runtime modes (1 third-level) | Not started (class D, user ruling option ②: separate team)                                                                                                                         | **Suggest moving out of P3 → P8 (execution layer)**, pending ruling                                                                    |
>
> **IV. Leverage of 4.3.1 (main-line marker)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the prerequisite of **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ taking it once advances the main line and clears two P3 stalled items. 3.5
> (HKT), 3.6 (function-type value call) and 4.6.1/4.9.1 all have **no unmet prerequisites** and are
> doable at any time; 3.8 is stuck on semantic decisions (user-definition priority vs explicit
> diagnostic).
>
> **III. Mechanism suggestion (to prevent recurrence)**: newly registered second/third-level rows
> **must carry two fields** — "owning phase" (which P it truly belongs to) and "whether it blocks
> the main line"; `registered pending scheduling` is no longer a default state — unlabeled means
> unschedulable. This way bloat is visible and sortable rather than undifferentiated accumulation.
> **Pending ruling**: whether to write this into the registration convention of this document and
> into `src/dev/coding-rules.md`.
>
> **P3 landed (2026-10-06)**: 3.1.1 `proof_calls` narrows visibility + sole getter (recorded at the
> time as `pub(crate)`; **2026-10-07 verified the actual landing was `pub(super)`** — the `#434`
> ruling referenced by the code comment has no doc-side entry, see 3.4.3; fully private would
> require rewriting 11 construction literals, not taken during the hemostasis phase; consolidated
> during the P4 Obligations migration); the execution mechanism extracted to
> `src/frontend/proof_execution.rs` (prohibition three, single point — cannot be placed under
> typecheck/, which would create a reverse L3→L4 dependency); 3.1.2 fills the **four entry points**
> per 02 evidence chain step 5 (the WBS row omitted `compile_project :99`; per 3.1.1 acceptance
> "criterion turn-green" this must be filled in and registered); 3.2.1 empty arm changes to
> bookkeeping (proof_calls throws) + diagnostic (the `into_result` established conversion path);
> 3.3.1 SOLVER slot Option-ized, Z3 absence conservatively downgrades to `SMTResult::Unknown` (E2031
> family diagnostic replaces process crash). The `#[ignore]` removed from the two bug criteria and
> turned normally green, the supplementary corpus fixture `proof-obligation-honored` added;
> check-fanout inventory migrated accordingly (wiring point 19 = inventory). Empirical: criteria
> 2.4.1/2.4.3 turned green, full suite lib 2494 / integration 272 / corpus two-layer all green.
>
> **3.4 audit (2026-10-07, re-review after P3 landing)**: re-inventoryed "failure produces no
> signal" defects of this category (definition in 02 §41-52) on unchanged code, found another 8
> locations, all registered as 3.4.x in the table above. Method and evidence:
>
> - **Consumption-side inventory** (grep all read sites for 16 obligation fields one by one):
>   `variant_ctor_calls` **zero consumers** — the sole read in `checker.rs:1468` is "self-injected
>   into `TypeCheckResult`", while the IR side uses form detection `ir_gen.rs:7374`;
>   `rfc011b.rs:659/688` only asserts the producer side, exactly the "only tests the producer side,
>   naturally immune to consumer-side defects" pattern criticized in 02 §186 → 3.4.4. The
>   result-level field `method_overload_ir_names` likewise has zero external consumers, the IR uses
>   a parallel `overload_resolutions` → 3.4.5.
> - **Stage coverage re-review**: 02 §91 table row 1 (four entry points "no proof_execution") has
>   been fixed by 3.1.2, current state is 5 locations sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 status unchanged (multi-file `run` still
>   does not emit W1003, does not run the dead-code family) → 3.4.6; LSP non-in-project path (02 §77
>   "manual lex→parse") likewise does not run proof → 3.4.7.
> - **Same-shape empty-arm scan** (only four `Unproven` consumers in the whole repo): checker three
>   refinement branches (covered by 3.1), the ownership arm fixed by 3.2.1 (**source unreachable**),
>   and `environment.rs:422` (**reachable, zero diagnostics, zero bookkeeping, and zero entries in
>   any doc in the whole repo**) → 3.4.1, the only "everyone-unknown" item in this audit.
> - **Solver acquisition surface**: the three consumption forms of `default_solver()` differ
>   (`predicate.rs:32` process singleton / `checker.rs:1266` injected per `check_module` /
>   `ownership.rs:621` newly created per back-edge decision) — 3.3.1 only removed the panic; the
>   **silent surface** of the other two is attributed to 3.4.2, **singleton-ization and cache hit**
>   is attributed to P4's 4.4.2 (no double registration).
> - **Existing attribution not counted in 3.4**: 2.4.4 (`ReleasePlan` span-key mismatch, 168 files /
>   373 keys unconsumed, Drop silently lost) remains a P2 standing red state, attribution 4.5.1/D20
>   unchanged; 4.4.1 (layer-order declaration opposite to reality, `equivalence` layer not in the
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty-ledger downgrade) are already on P4's books.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites and can land in place at
> any time (recommend doing 3.4.1 first — the only "zero-registered-in-the-whole-repo" living bug,
> with the smallest change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on the
> obligation ledger (4.3.1) or the unified Driver (4.2.x). **3.4 does not establish a new criterion
> system**; it reuses the existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 record (registered 2026-10-13 → closed 2026-10-15)**: on 2026-10-14 the user
> ruled to merge four correlation minor items (#451 method binding panic / #452
> named-argument-into-resolution / #453 dead-inference entry / #454 native export named-argument —
> the last is a boundary finding discovered when verifying 3.14, on the same day
> attribution-experiment-verified **not introduced by #452**, the user's ruling ① merges it into
> this batch) into a single team for handling. Closing status: all four completed, ruling registered
> as RFC-039 **D60** (six binding rules + shared criterion single-point-ization).
>
> **I. Two contract corrections (lesson: contracts can be wrong too)**
>
> 1. **Acceptance fixture self-contradiction**: the #452 task contract required both "mixed
>    arguments output 20" and "position and name conflict at the same position → error", but the
>    nailed fixture `Temp.pick(self, a)` has only one value parameter — the same argument
>    combination is both "mixed" and "conflicting". The first implementer honestly reported blocker
>    and set the task failed, reverted the whole thing (not trading silence for green light).
>    Disposition: task re-assigned + contract revised — **six binding rules** and an **8-row value
>    matrix** are given (fixture changed to two value parameters `pick(self, n, a)`), then attempt 2
>    was completed by a second implementer. The full text of the six rules has been incorporated
>    verbatim into D60.
> 2. **inScope misses the mechanically-required surface**: the #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missing the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered only `code-tables --fix` was
>    run, **`scripts/docs/gen-error-code-docs.py` was not re-run** ⇒ CI Docs Truth red ("source has
>    registered/emitted but doc missing 1: E1110", source 154 / doc 153, run **38518624964** — the
>    only CI red line in this batch). Fix = mechanical regeneration (8 files +83/−70, only touching
>    `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **II. Review → fix → re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with the resolution ⇒ calls with named arguments on the multi-candidate zero-hit /
>    single-candidate hit two paths silent end-to-end) → the fix round added a new external code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece set
>    in the same commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parse entry + native arm settled by name) → independent
>    verification PASS (value matrix 12 cells, corpus 342/342, 5 mutations mechanically reversibly
>    replayed via `scratch.py trial` all caught, byte-for-byte identical restoration) → round-1
>    review needs_revision (F1 = the typecheck native arm **inlined a copy** of the binding
>    criterion, missing rule ④ ⇒ **divergent rulings** for method vs. native same-shape calls —
>    methods report E1111, natives silently exit=0; ir_gen has a third existing copy reporting the
>    old code E1015) → the convergence round consolidated the criterion into a single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call sites `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught a **regression** introduced by the
>    convergence round (IR convergence block lost the native optional-position ⇒ programs that
>    previously succeeded with optional positions omitted are now falsely reported E1010) → fix
>    round (optional-position criterion and parameter name from the same `signature.rs:236` sole
>    exit) → round-3 review PASS (red→green independently reproduced; mutation with the in-repo
>    nail-board FAILED verified the criterion can fail). **Lesson**: review must cover "unfixed
>    branches of the same-class path" (both needs_revision in this batch were due to the probe all
>    falling on the hit side); the fix round itself can also introduce a regression (optional
>    position), caught by the next round of different-person review.
>
> **III. Process events (registered as-is, not pointing at individuals)**
>
> 1. **External whole-tree discard (around 2026-10-14 17:14)**: the working tree was externally
>    restored silently — the implementer's in-progress fixes (typecheck/ir_gen/code
>    table/locales/RFC-013 etc.) all disappeared; **at the same time two review documents the user
>    themselves had edited also disappeared from git status** ⇒ determined to be a repository-level
>    discard (most likely manual cleanup / IDE "discard all changes" on the user side). Three pieces
>    of evidence exclude group members: empty stash, no new commits, no reset in reflog. The
>    implementer replayed in-progress work from context and pushed; the discipline subsequently
>    became "**save patch to scratch first, then pick up the red state**".
> 2. **Verifier ran out of room, replaced**: the independent verifier verifying 3.16 ran out of room
>    mid-way (no closing message); per the load rule, replaced — the successor continued using their
>    already-ready artifacts (green-state results, corpus equivalence script, mutation plan),
>    verification finally PASS. A positive precedent for partial artifact handover.
> 3. **Round-5 review rotation and historical-round rollback**: when rotating review per load-cap
>    item ⑤, `remove_member` **rolled back the already-adjudicated final-state review/verification
>    rounds** together, the scheduler immediately re-ran them (t3/t8 were reassigned to new members,
>    duplicating work and blocking downstream). Disposition: the captain took over and cancelled the
>    rolled-back historical rounds (t3/t8/t13), and AGENTS.md item 12 was added accordingly (handle
>    rolled-back rounds immediately after personnel change). The closing record of this batch
>    registers this event per that item.
> 4. **scratch governance (about 6.3 GB)**: in the early stage of the batch each task had an
>    external tree, 6 directories scattered outside the repo (about 780 MB, cleaned) + two members'
>    external verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Output governance
>    tool `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md
>    item 9 (single entry for batch trees), item 10 (verification environment six rules:
>    mechanically reversible mutation / line number from cited revision / CI-exclusive gates must
>    run, etc.), item 11 (writer-side load cap), item 12 (handle rolled-back rounds after personnel
>    change) and two top meta rules (no private content / verify before asserting).
> 5. **Contract defect (not pointing at members)**: the #454 fix-round contract inScope only covered
>    2 files, but the actual mechanically-required surface is 8 (item 6 is exactly to guard against
>    this — contract template should pre-set the mechanically-required surface list); another task's
>    terminal-state field was occupied by probe payload, and the formal acceptance / command
>    evidence was supplementarily recorded by the captain per the protocol.
>
> **IV. Leftover items (not fixed in this batch, registered for follow-up)**: round-3 review three
> low-severity findings — ① method-path `build_arg_slots` (`ir_gen.rs:521` vicinity) remains a third
> implementation of the criterion (existing, per the previous round's principle registered at
> closing, not consolidated); ② `cargo fmt --all` errors in the batch's verification tree (worktree)
> are worktree-workspace artifacts, the gate checklist should note that fmt runs against the main
> repo; ③ repair-class contract template should pre-set the mechanically-required surface into
> inScope (see III.5).
>
> **V. Final gate (closing revision, local two implementation commits + one doc commit pushed
> together — the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 /
> corpus_probe **342 lines byte-for-byte identical to baseline** (the only regenerated line:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number in sync, manually
> reviewed) / IR snapshot zero drift.
>
> **VI. run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (the MSRV/Test in the same run were cancelled by a
> subsequent push due to concurrency — cancellation ≠ failure, covered by the later run); governance
> tool push → CI 38546373435 / 38561405117; WBS registration (3.16 row + table header 56→57 /
> 147→148) → CI 38528560364. The CI / Deploy Docs run of the closing push itself is supplementarily
> recorded in the task record after the push.
>
> **3.4.8 ③④ landed (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail adds `Linking` (**pure merge**,
> entirely skipping the `is_bin_role` entry check — the E3020 criterion still belongs to
> `RoleClassification` to prevent double-report; another hard reason: the run path judges "has main"
> by merged IR, under the collection semantics "the entry itself errors → no IR" is misjudged as
> missing main and hard-aborts) → `Monomorphization` (check form); `ir_generation_check` retains the
> unit IR (**memory-only**, consumed by pure merge and mono, not emitted / not written to disk / not
> producing snapshots — without retention the aggregation request is empty and mono fakes green with
> `NoObligations`); mono failures in Check go through a per-file collection channel (diagnostic
> attributed to entry file + `failed_stages`, not setting failure), MultiFile retains `fail_with`;
> `into_outcome` changed to "take collection first then judge failure". ④ empirically **zero
> production change**: full-program BFS already covers cross-unit mutual recursion (pathological →
> E3005, well-formed three-layer chain executes with correct value), delivering four e2e groups +
> one mono unit test as live criteria.
>
> **Verification chain** (all independent of the implementation): t3 (③, 9 self-built fixtures +
> byte-for-byte cross-check with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual-binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only added, not reduced).
>
> **F1 (found during verification, D59 class A/B, fixed in the same round)**: under rootless
> multi-file check without project root, the aggregation request name (`lib.f`) and the unit IR
> definition key (`f`) come from different sources → mono falls into a silent `continue` of "native
> without IR definition", check reports 0 error/exit 0. Fix: the aggregation side adds a
> reverse-lookup jump by each unit's **actual definition key** (exact enumeration + literal
> equality + ambiguity abandon, not a suffix heuristic; the project form is not registered, existing
> path unchanged). **Errata (same day)**: this reverse-lookup jump was **deleted** in a subsequent
> regression fix; the root cause is now solved by `module_key` doubling up (adding `script_mode`)
> from the naming caliber — IR definition key = IR call name = aggregation request name =
> `{unit key}.{name}`. **Regression record**: after ③ landed, CI's
> `Check tutorial/guide code examples compile` went red (258 blocks / 10 failures: 6 same-file
> same-name rebindings + 4 name clashes with `<std/list>`, rootless check doesn't do qualification,
> triggering `link_module_irs`'s clash check); fix = ir_gen splits `script_mode` (qualified name ⊥
> Script/Bin, E3023 gate looks at it instead) + `ir_generation_check` per-unit passes unit key +
> `.with_script_mode(rootless)` + Script top-level same-name rebinding "later takes effect"
> (consistent with existing run-path behavior); verification = doc gate 258/0, three-binary
> comparison (HEAD/③ tree/fix tree) three kinds of collision fixtures, check form E3005 not
> regressed, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic attributed to entry-file entry, span can cross units (same
> caliber as run path `path=entry`); Check form holds one ModuleIR per unit (memory only, released
> at check end).
>
> **Existing issues left for follow-up (not regressions this round)**: ① E3005's help text still
> says "internal error; please report this issue", which doesn't match the resource-protection
> semantics (precedent: `e3xxx.rs:47` once moved a similar "user-written error" out of E3005) —
> registered pending scheduling; ② multi-file E3005's span-rendering label is misaligned (reproduced
> with HEAD binary, pre-existing); ③ the request collection surface has **type-parameter position
> dependence** (generic calls where the type parameter is not at the value-parameter position don't
> emit instantiation requests, mono records `NoObligations`, converging mutual recursion is this
> form) — same surface as 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Remove `pub` and Auto-Binding (#399

> **P3.5 landed (2026-10-08)**: all seven second-level tasks completed, the whole batch in one
> commit (independently revertible).
>
> - **Empirical vs. plan mismatch (registered as-is)**: 029g.1.2/029g.1.3's acceptance originally
>   read "``pub f =
>   ...`reports a **parse error**" — actually after removing`KwPub`, `pub`falls back to a normal identifier, and the old syntax reports **E1001 Unknown variable: 'pub'** at the semantic layer (span points at`pub`). Per RFC-039 **D27** "specially probing removed syntax is an anti-pattern", **no new migration-specific diagnostic is added**; the corpus fixture `pub_declaration_err.yx` nails E1001, the doc (`syntax.md`)
>   honestly states this consequence.
> - **Design convergence (beyond RFC literal, requires review)**: after `pub` removal, the two
>   fields `project_refs` and `cross_file_refs` are semantically fully overlapping (the same "set of
>   in-package references", prohibition one forbids parallel representation) → merged into the
>   single `project_refs`, the two switches `set_exempt_pub` / `set_cross_file_refs` deleted. In
>   behavior, the non-pub item of the Internal role changes from "file-local reachability" to
>   "reference-pool judgment" (relaxed, in the direction of under-reporting); Lib tightens from "pub
>   absolute exemption" to reference-pool judgment; Script's unreferenced top-level bindings start
>   reporting W1001/W1002/W1004/W1005.
> - **Dead-table cleanup**: `env.exports` + `add_export` / `is_exported` / `is_visible` +
>   `auto_bind_to_type` + the exports writeback in `typecheck/mod.rs` all deleted (producer-side
>   fill, zero consumers in the whole repo).
> - **Test migration** (test count not reduced): added `lexer/tests/rfc029g_lexer.rs` (1); the "pub
>   is recognized" assertions in `parser/statements/tests/bindings.rs` reversed to "pub is no longer
>   a modifier"; the 7 role-semantics cases in `passes/tests/dead_code.rs` rewritten (2 assertions
>   reversed); the 8 role cases in `tests/integration/cli_e2e.rs` rewritten (the exemption
>   assertions for `lib`/`internal` reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` does not participate in IR); corpus diff baseline
>   adds only 1 entry (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated and manually reviewed.
> - **Empirical**: lib 2556 / integration 272 / corpus 322-entry baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 implementation record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two feature commits, closing
> (registration / WBS check-off / gating / push) done by t7. Prerequisite = 4.3.1 obligation ledger
> (red state retained: 173 files / 533 keys).
>
> **I. Carrier ruling (option A: `PlanId` goes with AST nodes)**: added `PlanId`
> (`src/util/plan_id.rs`, `u32` monotonic, unique within a compilation unit, `UNPLANNED = u32::MAX`
> for synthesized nodes outside the parser), issued by the **sole allocator**
> `ParserState::alloc_plan_id`; carrier in two places — `Stmt.plan_id` (statement identity) and
> `Expr::Call.plan_id` (call-site identity), `Expr::call` is the only call constructor in the whole
> repo. The rejected form is "each side derives the same identity from span" position-coincidence
> (D20's original defect).
>
> **II. Two-level verification and review (members independent)**: each feature commit is paired
> with an independent verifier and a reviewer, **verifier/reviewer do not reuse the implementer's
> evidence**: 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations)
> / t3 review; 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations)
> / t6 review. The re-review time and the reviewed revision both fall in immutable history
> (verifiable via `git show`). **The verifier/reviewer had to be replaced because of a provider
> rename**: the original verifier/reviewer could not continue due to provider rename, per the
> orchestration convention a new person was assigned and **the reason was written in the record**
> (cross-round context preservation takes priority, only swap when compression is unavailable);
> after reassignment the conclusion was consistent with the original round (PASS), without reusing
> the implementer's evidence.
>
> **III. Two defects found and fixed during implementation self-check (t1, both within inScope)**: ①
> the consumption-exemption criterion was once attached to `UnitState.ir.is_some()` — in the `check`
> form, Linking/Mono will take `unit.ir` away, so truly unconsumed in the check form **passes
> silently** (mutation test: D41 red but CLI check exit 0); changed to read **the ledger's own
> completion marker**, and a regression case was added. ② the Drop sequence drifted with the process
> hash seed (partial order `sort_by` + `HashSet` iteration): the snapshot criterion ran 106/120/114
> row-count drift on three consecutive runs; changed to **total-order sorting** (same name take max
> depth + name ascending) and ran twice with zero drift.
>
> **IV. Criteria and evidence figures**: after 4.5.1, `test_release_plan_spans_consumed` removed
> from `#[ignore]` standing (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in
> the same commit (programmatic review: 1083 new lines of Drop + 86 lines of jump-index translation,
> zero deletion, no reordering), corpus 342 diff entries empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference empty + evidence floor `examined ≥ 1` /
> `consumed ≥ 2`; empirical examined=1, consumed entries 2, no skipped resolutions 210), 9
> resolution forms nailed, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation drive: consumption key shifted by one ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; restored to green.
>
> **V. Classification basis for semantic correction (D59)**: `Type.method(位置实参, 名字 = 值)`
> **silently gave the wrong answer** before the change (empirically compiled clean, ran output `1`
> instead of `20`) ⇒ **class C** (old behavior is not a success), the fix takes "loud": after the
> change the same program emits `error [W1090]` at compile time; the terminal-state semantics of
> "named argument participates in resolution" is **class D**, registered in §3.14 and issue #452 is
> left for a dedicated team. Additionally: `Drop` is a no-op in the interpreter (the wildcard arm of
> `backends/interpreter/executor/ops/control.rs` only `advance()`) — the key migration does not
> change runtime behavior.
>
> **VI. F-1 / F-2 disposition (found by t5 independent verification)**: **F-1 (medium)** the unbound
> consumption point (`ir_gen.rs:7766-7790`) is unreachable in the current corpus and test surface,
> the keying correctness has no red state to falsify ⇒ per "known boundary" registered into the
> revision note paragraph 5 of [02](02-stage-contract.md) §2 (preservation reason + no new task);
> **F-2 (low)** pointer drift ⇒ the doc comment of `OverloadResolutions` in `types.rs` changed to
> precise pointers (produces `inference/expressions.rs:4550`, consumes `ir_gen.rs:7697` / `:7772`),
> a separate small commit, not mixed into the doc commit.
>
> **VII. Closing (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption-point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement
> markers); 07 criterion table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 /
> 4.5.1 / 4.5.2; CHANGELOG one line of behavior change recorded; `check-obligations.py` and
> `check_tracking.py` green, CI and Deploy Docs green. **Accounting deviation found at closing**:
> the pointers `ir_gen.rs:357` / `:3102` / `:1096` in t3 / t6 reports differ from the code fact by
> one to three lines (actually `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2's insertion) —
> registered with the code fact as authority, this note also retains the trace.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (Run half of 4.2.5 Ruling A)

> **Batch C record (registered 2026-10-15 → closed 2026-10-15)**: on 2026-10-15 the user ruled C —
> this round only does 4.10.3 (same-batch candidate 3.4.6 per B1 ([02](02-stage-contract.md)
> §runtime behavior table "dead-code family warning does not enter compile path") deferred to after
> 4.9, prerequisite changed and noted); implementation form = **single complete-body team** (user's
> instruction: no DAG, no team building, single-session implementation + self-verification).
> Pre-batch outstanding work landed first: #455 dependency advisory fix (rustls 0.23.46 +
> serde_yaml_ng, `cargo audit` restored from exit 1 to exit 0), WBS registration debt (#456
> registered as 3.17 + incomplete rows filled in "owning phase / whether blocks main line" fields).
>
> **I. Red before green**: 3 driver-level red criteria (`src/driver/tests/run_rootless.rs` —
> rootless merged IR must not carry entry_function (red state `Some("main.main")`), rootless
> top-level executable statements must not report E3023 (red state produces no module), Bin control
> entry_function maintained (green state standing)) + CLI e2e 3 nails (standalone relative import
> run (red state E5001), Script top-level statement execution and #413 hint two green-state nails).
>
> **II. Implementation surface**: CLI `run` unified through `compile_project` (the
> project/standalone fork in `run_file_with_diagnostics` removed); Linking's `entry_function` gated
> by `is_bin_role` (#413 T4 — Script does not auto-call main); #413 hint key changed from
> `module.source_files.is_empty()` to call-site project probe (after unification ModuleIR always
> carries source_files, the original marker is invalid); `script_main_never_called` adapted to the
> qualified-name form (function `{key}.main`, the global slot empirically is also the qualified name
> — IR-based evidence, not inferred from comments).
>
> **III. Existing gaps exposed by unification (fixed along the way in the same batch, class A/B/C —
> only fixing failing programs to succeed, zero impact on previously successful programs)**:
>
> 1. **Explicit type-argument pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3 directly
>    emits the specialized name (mono strips the un-specialized original), the three arms of
>    `resolve_call_target` (rename / aliases / dot prefix) do not cover the `mk(int64)` form ⇒ the
>    call site dangles E6006. Fix = this single point in the parser adds a fourth arm (base-name
>    qualified, type-parameter string preserved as-is). **Same shape reproduces in Bin projects**
>    (pre-existing defect, not introduced by this batch).
> 2. **Script top-level `mut` assignment statement duplicates slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign of the
>    same shape as the declaration) per statement ⇒ the same variable allocates 3 slots, the
>    initialization writes to the later slot while the read uses the earlier slot (runtime Add type
>    mismatch). Fix = both places merge by name (same criterion, same source).
> 3. **Same-module constrained generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces the bare name `Vec3.add`, but the merged table is actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table lookup validation (exact hit →
>    qualified hit on generic owning unit key → fall back to bare name; not a suffix heuristic, same
>    form as `resolve_generic_name`).
>
> **IV. Registration gap (not fixed in this batch)**: constraint dispatch still E6006 when **type
> and generic definition are in different modules** (rootless two-file probe empirical; Bin same
> shape) — root cause = MonoType user type name does not carry the defining module, the correct fix
> belongs to P6 type-identity convergence; registered in WBS as **3.18** (owning phase P3,
> prerequisite awaits P6, does not block the main line), `qualify_deferred_method` notes the same
> surface.
>
> **V. Test adaptation (honestly registered)**: `tests/integration/multifile.rs` fixture
> supplemented with manifest — what it tests is RFC-029 project orchestration (Bin auto-entry)
> rather than Script semantics; one nail in `cli_e2e` (top-level diagnostic variable name
> visibility) and `script_main_hint` fixture (7 cases) synchronized to the qualified-name form — the
> producer/consumer side only takes merged IR, fixtures drive the same form directly, not using
> pipeline-product stand-ins.
>
> **VI. Final gate (closing revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regenerated **12 lines** — line-by-line structured check: exit / stdout / diagnostic
> code all identical per entry, only stack-frame function name and variable name qualified by unit
> key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process events (registered as-is)**: this batch once concurrently edited the workspace
> documents during the pre-commit hook run, eaten by the hook's stash rollback and the commit failed
> (concurrent form pre-announced in the config comments) — discipline = do not touch the workspace
> during the hook run; the #455 commit was therefore split into two attempts (the second all green).
