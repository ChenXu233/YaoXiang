# WBS Historical Notes (Implementation Record · Audit · Decision Trail)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (execution work breakdown): implementation records, audit findings, decision trails, and errata
> for each phase. **Division of labor**: 09 describes the **plan** only (kanban, three-level task
> table, dependencies, risks, acceptance); this document records **history** only (what was done,
> what was found, how it was decided), appended chronologically without rewriting history.

## P2 Establishing the Equivalence Criterion Baseline

> **P2 implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green.
> **D38 prophecy fulfilled**—the green-run process empirically discovered and fixed three real
> defects in ir_gen (see 06 §G6 for details): the method locals table was truncated to parameter
> count (local_count underestimated, runtime tolerated via Frame::set_slot), unannotated parameters
> were discarded by filter_map (three locations, params metadata misaligned with frame layout), and
> match result slots relied on frame's implicit Void initialization (now explicitly
> pre-initialized). The actual semantics of jump targets is flattened instruction indices (not the
> label-keyed form in the 07 text), and CFG is built at instruction level—recorded in the verify.rs
> file header. **Commit correspondence**: ir_gen fixes = `a9ea199b`; verify main body = `d5b18390`
> (⚠ parallel stream COMMIT_EDITMSG collision caused its message to be mislabeled as "docs(rfc):
> RFC-027a…", when the content is actually verify.rs + six test files, buried in the history middle
> and cannot be amended, hereby recorded).
>
> **P2 narrowed portion implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> stop-bleeding channel prerequisite unlocked**.
>
> - 2.3.2: 5 fixtures in `tests/yaoxiang-multifile/` (4 behavior + 1 compile-error) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract and the single-file corpus share the same
>   `TestFileSpec` (single source on the library side), binary location/subprocess invocation
>   abstracted to `tests/common/` and yx_runner migrated in sync (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed down by rfc027 tests (return position/call point/binding
>   position)—the `Sorted(3)` annotation example from 02 would be substituted and evaluated directly
>   to Disproved under the current code, not presenting Unproven silently; the Sorted(3) in 07
>   pseudocode is illustrative only. Red-state empirical data is in the test file header.
> - The two red criteria are marked with `#[ignore]` per the 2.4.5 red skeleton precedent and stay
>   so: the default suite is all green, `-- --ignored` reproduces the red state; **remove the
>   attribute when P3 fix lands to turn green**.
> - The reason for the deliberate deviation from the 07 (code, file, line) table in comparison
>   normalization (removing file/line, comparing only error code sets) is written in the 2.4.1 test
>   comments.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **moved out of P2**: the `Program` / `Obligations` types they reference don't exist until P4,
> so P2 cannot compile. The former becomes 4.1.4 in P4 (no double registration), the latter's
> `#[ignore]` red skeleton merges into 4.3.1.
>
> **2.3.3 implementation (2026-10-07)**: C4 must-cover checklist item-by-item verification (07
> §143), 10 semantic corpus layers fully covered and all within the corpus-parity.jsonl diff
> baseline (26 representative files 26/26 hit, grep empirically confirmed)—ref/borrow/move
> 05-ownership entire directory (ref_shared, borrow_immutable/mutable/return, move_basic,
> ownership_deep, etc.), closure capture closures + closure_arg_inference + spawn_capture, currying
> curry_value_fix + curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for and 16 files,
> sum type sum_type__+ pattern_or_guard + match_call_scrutinee, existential forced point
> interface_dynamic_dispatch (Vec(Animal) existential type heterogeneous packaging triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overloading method_overload (+
> method_overload_ambiguous_err in diagnostic diff), refinement constraints Drop sequence refined__
> ×5 + 05-ownership-driven ReleasePlan (span key contract has another 2.4.4 red criterion on
> standby). Performance baseline `benches/pipeline.rs` checked in (07 §145 three smoke tests, two of
> them): `corpus_compile_frontend` full corpus compilation first measurement 17.6s,
> `cli_cold_start_hello` process cold start 169.7ms; interpretation throughput is already covered by
> the benches/lib.rs hotpath group (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate
> construction. The regression gate check-perf-regression.sh is unified into the P9 list (2.5).

## P3 Correctness Bug Fixes (Minimum Solution)

> **Error type labels for completed rows** (retrospective labeling, does not alter history): 3.1 =
> B ｜ 3.2 = B (defensive empty arm, criterion turns green via 3.1) ｜ 3.3 = A ｜ 3.4.1 / 3.4.2 = A
> (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D (user-visible
> surface, already user-decided) ｜ 3.4.8 ①② = C (main) + A (incidental three) ｜ 3.5 / 3.6 / 3.7 /
> 3.8 = D (all already user-decided).
>
> **P3 diagnosis (2026-10-12, user request)**: A diagnostic review of the perception that "P3 items
> keep growing".
>
> **1. First clarify the scale (the perception was amplified by two accounting defects)**
>
> 1. **Number collision**: `3.5.x` is shared by two task families—(a) the **029g `pub` visibility
>    convergence** from 2026-10-08 (seven level-3 items: lexical and parser branches / AST `is_pub`
>    / checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) the **3.5 higher-order type parameter call
>    path completion** registered on 2026-10-10 (three level-3 items). The two share `3.5.x` ⇒
>    cross-reference ambiguity (**2026-10-12 has been renumbered per user decision: 029g family
>    changed to `029g.x`**), and caused the 029g rows to be miscounted into P3. **Pending
>    decision**: rename the 029g family to `029g.x` (or move it into its own phase), and synchronize
>    all other references within the document.
> 2. **Missing checks**: The four level-3 items of `3.7` (3.7.1–3.7.4) are all completed (d922a551 /
>    38bd4b9e) but not checked ⇒ this entry is corrected in this round.
>
> **2. P3 actual accounting (12 level-2 rows)**
>
> | Row                                                              | Status                                                                                                                                                                                | Disposition                                                                                                                        |
> | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                  | Completed                                                                                                                                                                             | —                                                                                                                                  |
> | 3.4 Silent Discard Point Supplementary (8 level-3)               | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed on 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **now doable**; 3.4.4/3.4.5 unlock with 4.3.1                                                                                |
> | 3.5 Higher-order type parameter call path completion (3 level-3) | Not started (declaration side live, call side broken)                                                                                                                                 | Keep in P3; **does not block refactor mainline**, but is "accepted feature unimplemented", higher priority than pure cleanup items |
> | 3.6 Function type value call path (1 level-3)                    | Not started (semantics already cut (i))                                                                                                                                               | **Keep**: correctness defect, clear criteria                                                                                       |
> | 3.7 init section call name qualification (4 level-3)             | **All completed**                                                                                                                                                                     | This round check and close                                                                                                         |
> | 3.8 native shadowing (1 level-3)                                 | Not started                                                                                                                                                                           | **Stuck on decision** (decide name resolution priority first), not a scheduling issue                                              |
> | 3.9 / 3.10 / 3.11                                                | Completed (2026-10-12)                                                                                                                                                                | —                                                                                                                                  |
> | 3.12 Runtime mode executor config propagation (1 level-3)        | Not started (class D, user decision B: separate team)                                                                                                                                 | **Recommend moving out of P3 → P8 (execution layer)**, pending decision                                                            |
>
> **4. Leverage of 4.3.1 (main line milestone)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the prerequisite for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ taking it once advances the mainline and clears two P3 stranded items. 3.5
> (HKT), 3.6 (function type value call) and 4.6.1/4.9.1 have **no unmet prerequisites**, all are
> do-able at any time; 3.8 is stuck on a semantic decision (user-defined priority vs explicit
> diagnostic).
>
> **3. Mechanism recommendation (prevent recurrence)**: Newly registered level-2/level-3 rows **must
> carry two fields**—"Owning phase" (which P it truly belongs to) and "Whether it blocks the
> mainline"; `registered awaiting scheduling` is no longer a default state, and unannotated items
> are considered unschedulable. This way bloat is visible and sortable, rather than undifferentiated
> accumulation. **Pending decision**: whether to write this into the registration convention of this
> document and `src/dev/coding-rules.md`.
>
> **P3 implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + single getter (then
> recorded as `pub(crate)`; **2026-10-07 verified actual implementation is `pub(super)`**—the code
> comment references decision #434 with no record on the docs side, see 3.4.3; full privatization
> requires rewriting 11 constructor literals, not taken during the stop-bleeding phase; collected at
> P4 Obligations migration); execution mechanism extracted to `src/frontend/proof_execution.rs`
> (prohibition three single point—cannot go in typecheck/, that would create L3→L4 reverse
> dependency); 3.1.2 per 02 evidence chain step 5 fills in **four entry points** (the WBS row number
> omitted compile_project `:99`, which must be filled in and recorded per 3.1.1 acceptance
> "criterion turns green"); 3.2.1 empty arm rewritten to record (proof_calls throws up) + diagnostic
> (the `into_result` established conversion path); 3.3.1 SOLVER slot Option-ized, Z3 missing
> degrades conservatively as `SMTResult::Unknown` (E2031 family diagnostics replace process crash).
> The two bug criteria remove `#[ignore]` and turn to normal green, supplemented with corpus fixture
> `proof-obligation-honored`; check-fanout inventory migrated accordingly (wiring point 19 =
> inventory). Empirical: criteria 2.4.1/2.4.3 turn green, full suite lib 2494 / integration 272 /
> corpus double layer all green.
>
> **3.4 audit (2026-10-07, P3 post-implementation review)**: For the class of defects "failure
> produces no signal" (definition in 02 §41-52), on the unchanged code we re-inventoried and located
> 8 more, all registered as 3.4.x in the above table. Method and evidence:
>
> - **Consumer-side inventory** (16 obligation fields grep full warehouse read points one by one):
>   `variant_ctor_calls` **zero consumers**—the only read at `checker.rs:1468` is "self-poured into
>   `TypeCheckResult`", the IR side uses form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts the production side, which is exactly the form criticized in 02 §186 "only testing
>   production side, naturally immune to consumer-side defects" → 3.4.4. The result-level field
>   `method_overload_ir_names` similarly has zero external consumers, IR uses the parallel
>   `overload_resolutions` → 3.4.5.
> - **Phase coverage re-check**: Row 1 of 02 §91 table (four entry points "no proof_execution") has
>   been fixed by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 are unchanged (multi-file `run` still does
>   not output W1003, does not run dead code family) → 3.4.6; LSP non-project-internal path (02 §77
>   "manual lex→parse") similarly does not run proof → 3.4.7.
> - **Same-shape empty arm scan** (only four consumers of `Unproven` in the full warehouse): three
>   checker refinement branches (covered by 3.1), the ownership arm fixed by 3.2.1 (**source code
>   unreachable**), and `environment.rs:422` (**reachable, zero diagnostics, zero records, and zero
>   registrations in the full warehouse docs**) → 3.4.1, the only "all-unknown" live defect in this
>   audit.
> - **Solver acquisition surface**: The three consumer forms of `default_solver()` are different
>   (`predicate.rs:32` process singleton / `checker.rs:1266` per `check_module` injection /
>   `ownership.rs:621` new per back-edge determination)—3.3.1 only eliminated the panic one; the
>   **silent surface** of the other two goes to 3.4.2, **singleton-ization and cache hit** goes to
>   P4's 4.4.2 (no double registration).
> - **Not counted into 3.4, existing ownership**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys unconsumed, Drop silently lost) remains in P2 red standby, owned by 4.5.1/D20
>   unchanged; 4.4.1 (layer order declaration vs actual reversed, `equivalence` layer not in
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) are already in P4.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites and can land on-site at
> any time (recommend doing 3.4.1 first—the only "zero registration in full warehouse" live defect,
> with minimal change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on the
> obligation ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish a new criterion
> system**, uniformly reusing the existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 implementation record (2026-10-13 registration → 2026-10-15 close)**: User
> 2026-10-14 decided to merge four relevance small items (#451 method binding panic / #452 named
> argument into resolution / #453 dead inference entry / #454 native export named argument—the last
> item is a boundary discovery during verification of 3.14, on the same day attribution experiments
> confirmed it is **not introduced by #452**, user decision ① merged into this batch) into one team
> for processing. Close status: all four completed, decision registered as RFC-039 **D60** (six
> binding rules + shared criterion single-point).
>
> **1. Two contract corrections (lesson: the contract itself can be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: The #452 task contract simultaneously required
>    "mixed arguments output 20" and "position and name fight for the same position error", but the
>    nailed fixture `Temp.pick(self, a)` has only one value parameter—the same argument combination
>    is both "mixed" and "conflict". The first implementer truthfully reported blocker and set the
>    task failed, overall revert (did not take silent for green light). Disposition: task
>    re-dispatch + contract revision—after providing the **six binding rules** and **8 value-level
>    matrix** (fixture changed to two value parameters `pick(self, n, a)`), attempt 2 was completed
>    by the second implementer. The full text of the six rules has been verbatim incorporated into
>    D60.
> 2. **inScope missed mechanical necessities**: The #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only `code-tables --fix` was
>    run, **`scripts/docs/gen-error-code-docs.py` was not rerun** ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964**—the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touching `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **2. Review → fix → re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed along with the decision ⇒ calls containing named arguments in the multi-candidate
>    zero-hit/single-candidate-hit paths are silently whole-chain); fix round added external code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece set
>    same submission; code count 154→155) → round-2 review pass.
> 2. **#454 line**: Implementation (shared parsing entry + native arm settlement by name) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations
>    mechanically reversibly replayed via `scratch.py trial` all caught, restoration byte-for-byte
>    identical) → round-1 review needs_revision (F1 = typecheck native arm **inline copied** a
>    binding criterion, missing rule ④ ⇒ method and native same-form call **decision
>    divergence**—method reports E1111, native silently exit=0; ir_gen has a third existing copy
>    reporting old code E1015) → convergence round consolidated the criterion into a single
>    implementation `bind_args_to_params` (`arg_binding.rs:35`, four call points
>    `expressions.rs:2727` / `:5416`, `ir_gen.rs:7776` / `:8279`) → round-2 review caught a
>    **regression** introduced by the convergence round (IR convergence block lost native omittable
>    positions ⇒ programs that previously succeeded with optional positions omitted were misreported
>    as E1010) → fix round (optional position criterion and parameter name from single exit
>    `signature.rs:236`) → round-3 review PASS (independently reproduced red→green; mutation under
>    nailed board FAILED empirically confirmed criterion can fail). **Lesson**: Review must cover
>    "unmodified branches of similar paths" (this batch's two needs_revision were both because
>    probes fell entirely on the hit surface); the fix round itself can also introduce regression
>    (omittable positions), caught by the next round of review by a different person.
>
> **3. Process events (truthfully recorded, not pointing to individuals)**
>
> 1. **External whole-tree discard (2026-10-14 approx 17:14)**: The working tree was externally
>    restored silently—all the implementer's work-in-progress fixes (typecheck/ir_gen/code
>    table/locales/RFC-013 etc.) disappeared, **at the same moment two review documents the user
>    himself edited also disappeared from git status** ⇒ judged to be a repository-level discard
>    (most likely user-side manual cleanup/IDE "discard all changes"). Three pieces of evidence
>    exclude team members: stash empty, no new commits, reflog has no reset. The implementer
>    replayed the work-in-progress per context and pushed; discipline subsequently changed to
>    "**save patch to scratch first, then capture red state**".
> 2. **Verifier ran out of context, replaced**: The independent verifier verifying 3.16 ran out of
>    room mid-way (no closing message), per load rules replaced—the successor continued using the
>    ready products (green results, corpus equivalence script, mutation plan), verification
>    eventually PASS. Positive precedent for partial product handover.
> 3. **Review 5th round rotation and historical round withdrawal**: When rotating the reviewer per
>    load cap ⑤, `remove_member` **withdrew the already-decided terminal review/verification
>    rounds** back into the shared pool, the scheduler immediately re-ran them (t3/t8 were taken
>    over by new members, duplicate work and blocking downstream). Disposition: captain took over
>    and cancelled the withdrawn historical rounds (t3/t8/t13), accordingly added AGENTS.md item 12
>    (handle withdrawn rounds immediately after personnel change). The close record of this batch is
>    registered as this event per that rule.
> 4. **scratch governance (about 6.3 GB)**: Early in the batch each task had an external tree, 6
>    directories scattered outside the warehouse (about 780 MB, cleaned) + two members' external
>    verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Output governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md item 9
>    (batch tree single entry), item 10 (verification environment six rules: mechanically reversible
>    mutation / line numbers taken from referenced revision / CI-exclusive gate must run, etc.),
>    item 11 (writer load cap), item 12 (handle withdrawn rounds after personnel change) and top
>    meta rules two (no private content / verify before asserting).
> 5. **Contract defects (not pointing to members)**: #454 fix round contract inScope only circled 2
>    files, actually mechanical necessities 8 (rule 6 was meant to prevent this—the contract
>    template should preset mechanical necessities list); also this task's terminal state field was
>    once occupied by probe payload, formal acceptance/command evidence was added by captain per
>    spec.
>
> **4. Legacy items (not fixed in this batch, registered for record)**: round-3 review three
> low-severity findings——① method path's `build_arg_slots` (`ir_gen.rs:521`) is still the
> criterion's **second** implementation (existing, registered at close per last round's caliber, not
> consolidated)—**2026-10-16 pre-release audit supplementary three points**: (a) it has **two** call
> points: `ir_gen.rs:7888` (method binding path, uses the bound function `binding.function`'s
> parameter name) and `:7975` (overload resolution path, uses the resolved mixed-name leaf field),
> original registration only wrote "`ir_gen.rs:521` area"; (b) it has **diverged from the shared
> implementation `bind_args_to_params`**: missing rule ⑤ (duplicate naming → E1015), rule ⑥ (missing
> required parameter → E1010) and "optional position" concept, and hard-codes the receiver position
> to `1 + i`; the two call points uniformly `continue` on empty slots (comment writes "diagnostics
> go to typecheck (it guarantees that once resolution passes there are no empty slots)"), a
> **dangling invariant written in comments with no assertion backing**; (c) **RFC-039 D60's "all
> call points converge to it" enumeration missed these two**, declaration inconsistent with fact,
> and the referenced line numbers were not retaken on the referenced revision, D60 text corrected
> accordingly with this entry; ② batch verification tree (worktree) `cargo fmt --all` error is a
> worktree workspace artifact, gate list should note fmt goes to main warehouse; ③ repair type
> contract template should preset mechanical necessities into inScope (see 3.5).
>
> **5. Final gate (close revision, local two implementation commits + one documentation commit
> pushed together—the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 /
> corpus_probe **342 lines line-by-line consistent with baseline** (only regenerated line:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number synced, manually
> reviewed) / IR snapshot zero drift.
>
> **6. run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (MSRV/Test of the same run was cancelled by
> subsequent push per concurrency—cancellation≠failure, covered by later run); governance tool push
> → CI 38546373435 / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) →
> CI 38528560364. Close push's own CI / Deploy Docs run logged after push into the task record.
>
> **3.4.8 ③④ implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail adds `Linking` (**pure
> merge**, whole block skips `is_bin_role` entry check—E3020 criterion still goes to
> `RoleClassification` for double-report prevention; another hard reason: run path uses merged IR to
> determine "has main", collection semantic "entry itself errors → no IR" would be misjudged as
> missing main and hard-aborted) → `Monomorphization` (check form); `ir_generation_check` retains
> unit IR (**memory only**, for pure merge and mono consumption, no emit/write/snapshot—if not
> retained the aggregation request is empty, mono fakes green with `NoObligations`); mono failure in
> Check goes per-file collection channel (diagnostics to entry file + `failed_stages`, not set to
> failure), MultiFile keeps `fail_with`; `into_outcome` changed to "take collection first then judge
> failure". ④ empirically **zero production change**: whole program BFS already covers cross-unit
> mutual recursion (pathological → E3005, well-formed three-layer chain execution produces correct
> value), deliver four groups of e2e + one mono unit test as live criteria.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-check with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase, no decrease).
>
> **F1 (found during verification, D59 class A/B, fixed in same round)**: In standalone multi-file
> check without project root, the aggregation request name (`lib.f`) and unit IR definition key
> (`f`) are from different sources → mono falls into "native no IR defined" silent `continue`, check
> reports 0 error/exit 0. Fix: aggregation side adds lookup jump by each unit's **real definition
> key** (precise enumeration + character-for-character equality + ambiguity abandon, not suffix
> heuristic; project form not registered, existing path unchanged). **Errata (same day)**: This
> lookup jump was **removed** in subsequent regression fix; root cause instead solved from the
> naming caliber by decoupling `module_key`'s one-job-two-use (added `script_mode`)—IR definition
> key = IR call name = aggregation request name = `{unit_key}.{name}`. **Regression record**: After
> ③ landed, CI's `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6
> same-file same-name rebinding + 4 conflicting with `<std/list>`, rootless check does no
> qualification triggering `link_module_irs`'s conflict check); fix = ir_gen splits `script_mode`
> (qualified name ⊥ Script/Bin, E3023 gate looks at it instead) + `ir_generation_check` per-unit
> always passes unit key + `.with_script_mode(rootless)` + Script top-level same-name rebinding
> "later wins" (consistent with existing run path behavior); verification = documentation gate
> 258/0, three-binary comparison (HEAD/③tree/fix tree) three collision fixtures, check form E3005 no
> regression, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic goes to entry file entry, span can cross unit (same caliber
> as run path `path=entry`); Check form holds one ModuleIR per unit (memory only, released at check
> end).
>
> **Pre-existing issues left for follow-up (not regressions in this round)**: ① E3005's help text
> still writes "internal error; please report this issue", inconsistent with resource protection
> semantics (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of
> E3005)—registered awaiting scheduling; ② multi-file E3005's span rendering label misalignment
> (already reproduced with HEAD binary, pre-existing); ③ request collection surface has **type
> parameter position dependency** (generic calls with type parameters not in value parameter
> position don't issue instantiation requests, mono records `NoObligations`, converged mutual
> recursion is this form)—same surface as 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Delete `pub` and Auto-binding (#399

> **P3.5 implementation (2026-10-08)**: All seven level-2 tasks completed, the entire batch is one
> commit (independently revertable).
>
> - **Empirical vs plan mismatch (truthfully recorded)**: 029g.1.2/029g.1.3 acceptance originally
>   wrote "`pub f = ...` reports **parse error**"—actually after deleting `KwPub` `pub` falls back
>   to a normal identifier, the old writing pattern hits the semantic layer reporting **E1001
>   Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically probing
>   removed syntax is an anti-pattern", **no new migration-specific diagnostic is added**; corpus
>   fixture `pub_declaration_err.yx` nails E1001, documentation (`syntax.md`) truthfully writes this
>   consequence.
> - **Design convergence (exceeds RFC literal, requires review)**: `project_refs` and
>   `cross_file_refs` two fields have exactly overlapping semantics after pub deletion (same
>   "in-package reference union", prohibition one prohibits parallel representation) → merged into
>   single `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally
>   Internal role's non-pub items change from "intra-file reachability" to "reference pool judgment"
>   (loosened, prefer under-reporting direction); Lib tightened from "pub absolute exemption" to
>   reference pool judgment; Script un-referenced top-level bindings start reporting
>   W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (production-side
>   pour-in, full warehouse zero consumption).
> - **Test migration** (test count not decreased): Added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`' "pub recognized" assertion reversed to "pub no longer a
>   modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (assertion reversal 2);
>   `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption assertions
>   reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not participating in IR); corpus diff baseline
>   only added 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated then manually reviewed.
> - **Empirical**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-layer Contract PlanId-ization (D20)

> **4.5 implementation record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, close
> (registration / WBS check / gate / push) completed by t7. Prerequisite = 4.3.1 obligation ledger
> (red-state archive 173 files / 533 keys).
>
> **1. Carrier decision (A: `PlanId` follows AST node)**: Added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` represents synthesized
> nodes outside parser), issued by **sole allocator** `ParserState::alloc_plan_id`; carrier two
> locations——`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call point identity),
> `Expr::call` is the only call constructor in the warehouse. The rejected form is "both sides
> independently derive the same identity by span" position-coincidence form (D20's original defect).
>
> **2. Two-level verification and review (members independent)**: Each functional commit paired with
> an independent verifier and a reviewer, **verifier/reviewer do not reuse implementer evidence**:
> 4.5.1 = t1 implementer / t2 independent verification (external tree + 5 mutations) / t3 review;
> 4.5.2 = t4 implementer / t5 independent verification (external tree + 5 mutations) / t6 review.
> Review time point and reviewed revision both fall in immutable history (`git show` verifiable).
> **Provider rename caused t5 / t6 to change members and re-dispatch**: Original verification/review
> members could not continue due to provider rename, per orchestration convention changed members
> and re-dispatched **with reason written in the record** (cross-round context retention priority,
> only changed when compressed unusable); re-dispatched conclusion consistent with original round
> (PASS), did not reuse implementer evidence.
>
> **3. Two defects self-found and fixed during implementation (t1, both in inScope)**: ① The
> consumption exemption criterion was once hung on `UnitState.ir.is_some()`—Check form's
> Linking/Mono takes `unit.ir` away, the truly unconsumed in check form **silently passes**
> (mutation empirical: D41 red but CLI check exit 0); changed to read **the ledger's own completion
> flag**, and added regression cases. ② Drop sequence drifted with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion ran three times producing 106/120/114 line
> count drift; changed to **total order sort** (same name take max depth + name ascending), two
> consecutive runs zero drift.
>
> **4. Criteria and evidence numbers**: After 4.5.1 `test_release_plan_spans_consumed` removed
> `#[ignore]` (with `examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated same
> submission (programmatic review: added 1083 lines Drop + 86 lines jump index shift, zero
> deletions, no reordering), corpus 342 entries diff empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference set empty + evidence surface floor `examined ≥ 1`
> / `consumed ≥ 2`; empirical examined=1, consumed entries 2, no resolution skipped 210), resolution
> form 9 nailed, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**. Mutation
> driven: consumption key offset by one ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate diff; restored then re-greened.
>
> **5. Classification basis for semantic correction (D59)**:
> `Type.method(positional args, name = value)` **silently gave wrong answer** before change
> (empirically compiled clean, ran output `1` instead of `20`) ⇒ **class C** (old behavior not
> successful), fix takes "loud": same program compile-time `error [W1090]` after change; "named
> arguments participate in resolution" terminal semantics is **class D**, registered in §3.14 and
> issue #452 left for special team. Also: `Drop` in interpreter is no-op
> (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`)—key migration does
> not change runtime behavior.
>
> **6. F-1 / F-2 disposition (t5 independent verification found)**: **F-1 (medium)** Unbound
> consumption point (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, keying
> correctness has no red state falsifiable ⇒ registered as "known boundary" in
> [02](02-stage-contract.md) §2 revision note paragraph 5 (preserve reason + no new task); **F-2
> (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` document comment changed to precise
> pointer (produced `inference/expressions.rs:4550`, consumed `ir_gen.rs:7697` / `:7772`), a
> separate small commit, not mixed into documentation commit.
>
> **7. Close (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement flag);
> 07 criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one line behavior change; `check-obligations.py` and `check_tracking.py` green,
> CI and Deploy Docs green. **Accounting deviation found at close**: t3 / t6 reports' pointers
> `ir_gen.rs:357` / `:3102` / `:1096` differ from code fact by 1-3 lines (actually `:360` / `:3103`
> / `:1095`+`:1097`, due to 4.5.2 insertion)—registered per code fact, this note also keeps a trace.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (4.2.5 Decision A's run half)

> **C batch record (2026-10-15 registration → 2026-10-15 close)**: User 2026-10-15 decision C—this
> round only does 4.10.3 (same batch candidate 3.4.6 per B1 ([02](02-stage-contract.md) §runtime
> behavior table "dead code family warnings don't enter compile path") held over until after 4.9,
> prerequisites changed hook and noted); implementation form = **full single team** (user instructed
> no DAG, no team building, single-session implementation + self-verification). Pre-batch debts
> first landed: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit` from
> exit 1 recovered to exit 0), WBS registration debt (#456 registered as 3.17 + incomplete row
> supplemented "owning phase / whether it blocks mainline" fields).
>
> **1. Red before green**: 3 driver-level red criteria (`src/driver/tests/run_rootless.rs`—rootless
> merged IR must not carry entry_function (red state `Some("main.main")`), rootless top-level
> executable statements must not report E3023 (red state produces no module), Bin comparison
> entry_function maintained (green state resident)) + CLI e2e 3 nailed (standalone relative import
> run (red state E5001), Script top-level statement execution and #413 hint two green nails).
>
> **2. Implementation surface**: CLI `run` uniformly goes through `compile_project` (the
> `run_file_with_diagnostics` project/standalone bifurcation deleted); Linking's `entry_function`
> gated by `is_bin_role` (#413 T4——Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call point project probe (after unification ModuleIR always
> carries source_files, original marker ineffective); `script_main_never_called` adapted to
> qualified name form (function `{key}.main`, global slot empirically also qualified name—IR as
> evidence, not inferred from comments).
>
> **3. Existing gaps exposed by unification (same batch incidentally fixed, A/B/C class—only fix
> failing programs to succeed, originally successful programs zero impact)**:
>
> 1. **Explicit type argument pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3 directly
>    emits specialization name (mono strips unspecialized original), `resolve_call_target` three
>    arms (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call point dangling E6006.
>    Fix = add fourth arm at this single point (base name qualification, type parameter string
>    preserved as-is). **Bin project same form reproducible (pre-existing defect, not introduced by
>    this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign same
>    form as declaration) per statement ⇒ same variable gets 3 slots, initialization writes to
>    later-allocated slot while read uses earlier-allocated slot (runtime Add type mismatch). Fix =
>    both places merge by name (same criterion same source).
> 3. **Same-module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually is `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table lookup verification (precise hit →
>    generic owning unit key qualification hit → fall back to bare name; not suffix heuristic, same
>    form as `resolve_generic_name`).
>
> **4. Registration gap (not fixed in this batch)**: Constraint dispatch when **type and generic
> definition in different modules** still E6006 (rootless two-file probe empirical; Bin same
> form)—root cause = MonoType user type name doesn't carry defining module, correct fix belongs to
> P6 type identity convergence; registered WBS **3.18** (owning P3, prerequisite awaits P6, doesn't
> block mainline), `qualify_deferred_method` notes same surface.
>
> **5. Test adaptation (honestly recorded)**: `tests/integration/multifile.rs` fixture supplemented
> manifest—what it tests is RFC-029 project orchestration (Bin auto-entry) not Script semantics;
> `cli_e2e` one nail (top-level diagnostic variable name visibility) and `script_main_hint` fixture
> (7 cases) synced per qualified name form—production/consumer sides only consume merged IR,
> fixtures directly drive same form, no pipeline product stand-in.
>
> **6. Final gate (close revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regeneration **12 lines**——line-by-line structured check: exit / stdout / diagnostic
> codes all identical line by line, only stack frame function names and variable names qualified per
> unit key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **7. Process events (truthfully recorded)**: This batch once during pre-commit hook running
> concurrently edited workspace documentation, was eaten by hook's stash rollback and that commit
> failed (concurrent form warned in config comment)—discipline = don't touch workspace during hook
> running; #455's commit thus split into two attempts (second all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Close)

> **4.6 batch record (2026-10-16 registration → 2026-10-16 close)**: User 2026-10-16 instructed to
> proceed in 4.6 → 4.9 → 4.4 → P5 order, do 4.6 first; implementation form = **full single team**
> (per last batch user instruction: no DAG, no team building, single-session implementation +
> self-verification). Task definition (09 level-3 table): project-internal file semantic data
> (SemanticDB) goes through orchestrator same pipeline, same source as diagnostics; cross-file
> references carry `resolves_to`. Acceptance: project-internal cross-file jump hits definition;
> semantic data and diagnostics come from same compilation.
>
> **1. Current state empirical (verify before asserting)**: didOpen/didChange triggers **two
> passes** of compilation—diagnostics go through unified Driver pipeline (4.2.3/4.2.6 already
> unified), semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` parsing fails, cross-file reference events not produced, jump/rename/reference lookup for
> imported symbols silently fail—the silent failure registered in D55). Orchestration arm's checker
> uniformly names `"<module>"` (SemanticDB file identity lost), `type_result` (containing
> semantic_db) is discarded when diagnostics non-empty and never emitted; `checker.rs` exhaustion
> overrides `resolves_to`'s DefId file identity to current file; `Export`/`ModuleInfo` both lack
> definition location surface.
>
> **2. Red before green**: LSP e2e nail (`src/lsp/tests/cross_file.rs`)—temp two-file project
> didOpen main.yx then send GotoDefinition for `add_one` call point, **returns None before fix (red,
> empirically silent failure in above section)**, after fix hits lib.yx line 0 definition (green).
> During process one probe location: compiler side chain (driver probe dump) already fully connected
> but e2e still empty—root cause is `resolve_reference` precisely matches by reference **start
> point** (D55 precise-only current caliber), test cursor fell in middle of identifier; this
> tolerance belongs to existing behavior surface, this batch doesn't touch (see seven).
>
> **3. Implementation surface**: (1) `Export.definition_span` (extract_module_info harvested from
> AST identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled per DiscoveredFile, includes
> vendor; std always None); (2) checker reference exhaustion via `import_def_locations` (use row
> binding span → definition file + definition span) redirects cross-file imported items'
> `resolves_to` out of this file; std import has no registration, maintained falling to use row; (3)
> checker module identity changed from unified `"<module>"` to unit file path (empirically
> `env.module_name` all 4 consumer points zero diagnostic consumption); (4)
> `DriverOutcome.semantic_db` channel——CollectAll form (Lsp target file **regardless of diagnostic
> success or failure** + SingleFile fallback) gathered in, Lsp arm tail injects non-target file
> top-level export definition surface from registry (`external_definitions`, unopened files can also
> jump; type/submodule exports not injected——typecheck side type registration doesn't produce
> definition events, both sides maintain consistent gap); (5)
> `orchestrator::analyze_source_in_project` (`check_source_in_project` trans calls to take
> diagnostic half); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key uniformly normalized file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId thus stable hit); 5 handlers + semanticTokens
> dual requests connect `db_key`.
>
> **4. Incidental fix (D59 split class B—old behavior silently no-op, originally successful programs
> externally zero impact)**: ① project-internal file cross-file jump/rename/reference lookup changed
> from silent fail to hit real definition; ② when lexical error old manual pipeline deleted semantic
> information (coloring disappeared) → incomplete AST coloring retained (4.2.6 editor philosophy
> extension); ③ when target file has type errors semantic data no longer discarded along with
> `type_result` (typing intermediate state coloring/jump not lost).
>
> **5. Nails**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`——target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback to use row, error file retention, single-file fallback path key, single-file
> error retention) + LSP e2e 1 (red before green) + uri normalization 4
> (`src/lsp/tests/uri.rs`——drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **6. Mutation verification (scratch.py trial, mechanically reversible)**: Batch tree wbs-46 @
> close revision 39546020, two mutations both make hit nail turn red, after restoration blob
> identical to HEAD + git status CLEAN—— m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver nail + LSP e2e nail red, other 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two nails red (round-trip parsing unanchored),
> other 7 green. Batch tree gc'd (released 4.8 GB, `status` no batch tree).
>
> **7. Final gate (close revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new nails) / `--test integration` **287 passed** / corpus
> diff **byte-for-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synced server.rs:482 with 02 §obligation field
> table) / check-docs-truth 0 / docs-examples 258 entries 0 failures / docs-orphan 0 / markdownlint
> full warehouse only remaining user's unsubmitted 032 two MD056 (user's own edits, not touched) /
> pnpm docs:build 0. **Process events (truthfully recorded)**: en/09a-wbs-history.md:54 one MD037
> was introduced by auto-translate bot re-translating last batch 09a (zh original no space), this
> batch incidentally fixed——markdownlint red is not this batch's change surface but is in the same
> file as 09a, registered together. **Legacy observation (registered awaiting discussion, not
> touched)**: `resolve_reference` only precisely matches by reference start——when cursor falls in
> middle of identifier jump/hover/rename silently no-op (D55 precise-only current state, full file
> form isomorphic), if tolerance matching needed belongs to independent behavior change, report to
> user for decision.
>
> **Same day continuation batch (4.6.2, landed after user decision)**: After delivery of the close
> report user on "legacy observation ①" decided "OK, then change"——lookup key tolerance implemented
> per plan A: definition/hover/references/rename four handlers' lookup key changed from cursor
> position (via UTF-16 conversion) to use **identifier's own start point** (as located by
> `find_identifier_at_position`, same coordinate system as semantic events, no conversion needed);
> `resolve_reference`'s start precise match semantic unchanged (D55 precise-only unchanged).
> references/rename's definition point fallback judgment (start <= col < end) holds for identifier
> start point. Red before green: definition middle cursor nail first red (None) then green; four
> handlers middle cursor each 1 nail + cross-file e2e start/middle dual position assertion, `--lib`
> 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2 (level-3 count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)—Round 1 Close (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 batch record (2026-10-16 registration → 2026-10-16 close)**: User 2026-10-16 decided to
> merge 4.9.1 / 4.9.2 / 4.9.3 three related level-3 tasks **into one team one round** (reason: they
> all belong to the "pool's source and identity" surface, merged landing avoids the intermediate
> state repeated rewrites after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong to consumption
> refinement and test pool separation, depend on 4.6.1's `use` path resolution, forcing into same
> batch would expand the gate surface). #450 ownership **pending decision**——evidence shows it
> belongs to "after overload resolution name not returned to reference count" class, still
> reproducible under round 1 landed form, registered as independent WBS entry for disposition
> (captain recommends A: merge into round 2 register as independent entry; **pending user
> decision**). Frozen rev = `3863a9ff0` (parent commit `62f6e1a59` is pre-fix baseline);
> verification PASS attempt `aab92fe0` (i.e. t2); review pass attempt `96452696` (i.e. t3). **Close
> registration commit = `38d30b846` (docs(docs): 4.9 reference pool round 1 close
> registration)**——this actually was written and pushed by close member integration-refpool; its
> close probe once mistakenly marked close task as terminal failed and overwrote output, captain
> took over new attempt and reverted to completed per verified delivery facts, hereby correcting
> ownership and tag. Evidence archive: Evidence summary (8 mutation patch list,
> `warn_compare_v8.json` caliber and conclusion) inlined in this section "Three, +8 and +16 two sets
> of count correspondence" and "Four, mutation tests"; original files recycled with workspace, **not
> checked in, not as document reference source** (AGENTS meta rule ①).
>
> **1. Merge decision**
>
> 1. **The decision itself**: Three related level-3 tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round**——former depends on 4.6.1's delivered
>    `use` path resolution (already landed at `39546020`, can start); latter with RFC-029f table
>    incorporation same batch, registered at end of RFC-029f awaiting change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutual prerequisites (4.9.2 changes pool type → 4.9.1
>    assembly surface needs rewrite → 4.9.3 has role table for denominator judgment), splitting into
>    rounds must write one intermediate state then change twice; (b) all three belong to "pool's
>    source and identity" surface, 4.9.4 / 4.9.5 belong to another surface, batching is split by
>    "cohesion" not "independently revertable".
> 3. **Merge result**: Single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash note**: Before push `git pull --rebase` merged in
>    auto-translate bot commit (`72f364169`); rebase fell after the other party's commit, this
>    batch's two (implementation `3863a9ff0`, its pre-fix baseline兼other party document commit
>    `62f6e1a59`) **not rewritten, keep original hash**——first version mistakenly wrote here as
>    "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "after rewrite just same value",
>    actually "not rewritten"), corrected at 2026-10-17 close; other party code commit
>    `33f14e8ab`→`b122b7ad1`. **External references uniformly use the post-push hash as authority**
>    (old rewritten hash unreachable in fresh clone, identity determined by patch blob comparison).
>
> **2. Five criteria red/green empirical caliber**
>
> | #   | Criterion                                              | Red (before fix / P1 directory scan rollback patch)                                                                                            | Green (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                    |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool root = package target (#334)                      | Directory recursive scan ⇒ files in tests/ identifiers also enter pool, exempt production code unused definitions                              | `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` per `ExplicitSurfaces` root; when all roots degenerate fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                        |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` mixed | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembles sole entry; `arms.rs:603-609`'s `other_module_refs` unions "except this unit" all module entries then sends to `DeadCodeAnalyzer`; module's own name (top-level definition and local binding, `use` import name exception) no longer exempts other module same-name definition |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by project-internal other files' references                                                                                    | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test goes `_ => {}` default arm (per file-internal definition-use graph judgment)                                                                                                                                                                    |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Before fix: reports `W1003@1` (actually "unused import"), not W1001                                                                            | Red empirical (red 1 fixture red1) reports `W1001@3`; green all 6 fixtures PASS                                                                                                                                                                                                                                                                                                           |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Before fix: reports `W1003@1`                                                                                                                  | Same as above                                                                                                                                                                                                                                                                                                                                                                             |
>
> **Criterion ② "deduction red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` in helper.yx; old pool under bare name union treated `foo` as a whole "used by
> helper", lib.yx's `foo` exempt. **Empirical**: red2 fixture vs pre-fix binary → no W1001 (old
> caliber exempt); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red-state fixture **first version failed**: First
> version fixture placed same-name local variable in same file triggering E2010 (the language
> doesn't allow same-module shadowing top-level definition), changed to another module (helper.yx)
> to be deduction red. This point t3 review and t2 verification both independently reproduced, **not
> a contract defect**——same-module shadowing is the language's hard constraint, fixture must be
> cross-module.
>
> **3. +8 and +16 two sets of count correspondence** (**t3 review verified as definitive**)
>
> 4.9 round 1 evidence summary section C writes `+16/−0`, but missed the **"new fixture record"**
> component——v8 instrument (`warn_compare_v8.json`, 331 file symmetric diff, `only_before=0`)
> caliber **includes** new fixtures; t1 implementer's self-built "+8" is **warn-before / warn-after
> (each 325) aggregated per fixture** (each member file check once). Review line-by-line mapping:
>
> - **+8** (8 new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: These 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degenerates to inspected file's own compilation set, consumer main.yx not in it;
>     this is design-intended change (before fix these files' check surface originally had zero
>     warnings, not exemption path). Warehouse root `yaoxiang.toml` is empty file ⇒ same
>     degeneration path empirically holds.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **doesn't exist**——they are this batch's newly added fixtures). So +8's "+ 0 − 0" diff doesn't
>     have them, +16's v8 symmetric diff has.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` left: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once per file), **cancels** leaving only lib's
>     1——three data (+8 / +16 / dual binary ABBA) mutually consistent.
> - **New fixture 8** vs **+16 residual 8**: Same batch of files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats each of their checks
>   as independent record——same file may trigger different diagnostic entries for multiple
>   `role/context`.
>
> **Conclusion**: **+16 = +8 (aggregated per fixture) ∪ new fixture 8**——both counts are correct,
> **caliber different**: t1 reports "8 new" is fixture-layer caliber, t2 reports "+16" is file-layer
> caliber (symmetric diff includes new fixtures). t3 review ① +8⊆+16 verified; **review ② writes
> "don't copy EVIDENCE.md section C"**——section C missed the "new fixture record" component, this
> entry rewritten per t3 review `pass` conclusion.
>
> **4. Mutation tests (mechanically reversible, all `scratch.py trial`)**
>
> 8 mutation patches (p1 directory scan, p2b use import name removal, p3b script accept pool, p3
> flat consumption, p4 std accept pool, p7final self-built 4 criteria, p8 std close, p9 vendor
> accept pool + p_stack2 / p_stack3 stacking surface). Each trial output includes: HEAD blob →
> mutated blob → restored blob identical to HEAD = yes + `git status` CLEAN. Local
> `core.autocrlf=true` requires `GIT_CONFIG_*` inject `false` (`trial_env.ps1`), otherwise patch new
> lines convert to CRLF causing apply/restore double failure——**only affects patch mechanics, not
> changed behavior**.
>
> **5. Process events / environment lessons (must read for next round handover)**
>
> 1. **core.autocrlf injection**: Local `core.autocrlf=true` must inject `false` via `GIT_CONFIG_*`
>    to ensure patch mechanical reversibility; the injection script at that time was recycled with
>    workspace (criteria listed in previous entry, can rewrite per this). **Don't reuse next round**
>    any trial script without injection.
> 2. **Same tree trial serial**: `trial` and `cargo build` on the same tree **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch worktree, parallel can
>    cause "patch incomplete seen by build" or "build and apply deadlock" two kinds of races.
> 3. **Trial rebuild binary needs confirmation of sync**: After `trial` completes it rebuilds
>    tree-internal binary (cargo's fingerprint includes source path, **change tree = full
>    rebuild**), before subsequent measurement must confirm `bin/yaoxiang-rs` synced with HEAD.
>    **This round once used residual mutation binary to measure false data**, rebuilt and
>    re-measured to correct.
> 4. **Second tree yx49-b creation reason**: Dual-comparison needs pre-fix binary (62f6e1a59), no
>    ready product within batch; `scratch.py init yx49-b` opened second tree (another full build).
>    **Tree released** (see six), delete leaves batch close `gc` disposition.
> 5. **cli_e2e two Internal use case fixtures supplemented to legal package (supplemented
>    `[run].main`, assertion unchanged) reason**: Before fix `cli_e2e`'s two `Internal` role nail
>    fixture **didn't declare `[run].main`** ⇒ after 4.9.1 landed `compile_project` goes
>    `ExplicitSurfaces` root call path, can't find `main` ⇒ whole batch 2 fixtures behavior drift
>    from "Script fallback" to "entry not found"——this is 4.9.1 introduced fallback behavior not a
>    defect. After supplementing `[run].main` field two fixtures go normal package path,
>    **assertions** (exit code / diagnostic code / warning set) verbatim unchanged.
> 6. **Second round tree deletion safe way**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" then delete.
>    This batch released before close, next step `gc` natural cleanup.
>
> **6. Batch tree state**
>
> Before close: yx49 released (4.8 GB released), yx49-b released. Two trees no mount, status can
> verify in `scratch.py status`. Task output attached with `scratch.py gc` complete status.
>
> **7. RFC-029f "Known Defects" table flip**
>
> Previous version (2026-10-09) described in present tense "three design defects in reference pool"
> (including `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396` etc.
> deleted code positions). This batch close synchronized rewrite to "**Known defects: landed
> (2026-10-16, WBS 4.9 round 1)**": ① current pool root = `collect_package_refs` (`helpers.rs:481`),
> resolves per `ExplicitSurfaces`, no directory scan; ② current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly sole entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumer `arms.rs:603-609`; ③ current Script doesn't accept pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line retaken on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` moved into
> `src/driver/{state,arms,helpers}.rs` with P4 4.2.9 split (`c59fb751`), old positions in table no
> longer hit——this is a fix for "declaration inconsistent with fact" (t3 review note ①).
>
> **8. Final gate (close revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixtures
> behavior run, 338a339,340; another 342 lines byte-for-byte unchanged) / IR snapshot 204 zero drift
> / check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process event**: This
> batch once cargo build failed because `core.autocrlf` converted script line endings to CRLF, team
> scratch path `yaoxiang-rs` start reported `BadImage`, empirically encoding inconsistent with git
> index——passed after rebuild (parallel to 4.6 batch "external whole-tree discard" lesson, different
> reason).
>
> **9. Legacy (handed to next round)**
>
> 1. **#450 ownership decision**: Evidence shows it belongs to "after overload resolution name not
>    returned to reference count" class, **still reproducible under round 1 landed form**——per user
>    decision C process dispatch. **#450 probe**: Dual binary same set `{W1002@2, W1005@4}`
>    (empirical, rootless form), no direct relationship with round 1 refactoring, but #450's fix may
>    need to wait for 4.9.4 (consumption refinement) to be done together——because root cause is
>    "after overload resolution name passes reference count" needs use item's precise consumption
>    surface, 4.9.4's "really used after being imported" is the same criterion's precise version.
> 2. **dead_code.rs:683 pointer nit**: Comment writes "two pools' assembly caliber see driver side
>    collect_compilation_refs"——actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment doesn't match code main path caliber (**not
>    in this batch's write scope**), left for 4.9.4 / 4.9.5 to fix when landing.
> 3. **4.9.4 / 4.9.5 prerequisites unlocked**: 4.6.1 (`use` path resolution) delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to first define
>    RFC-029f table incorporation (prepared by this batch "Seven," flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user decision C held over until after 4.9)——this round not involved; next round
>    4.9.4 / 4.9.5 land same batch process.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)—Round 2 Close (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 round 2 batch record (2026-10-16 implementation → 2026-10-17 close)**: Covers 4.9.4
> (consumption criterion = really used after being imported), 4.9.5 (test pool separation), 4.9.6
> (method reachability caliber, issue #450, user 2026-10-16 decision A merged into round 2). Close
> fix (verification-side remedial M1/M2/M4 recompilable redo + F1 fix round + PA-2 delete dead
> branch) registered gap issue #464 (WBS 4.9.7) landed together.
>
> **1. Decision chain and commits (uniformly post-push truth)**
>
> | Step | Content                                                                                                                                                | Post-push commit | Pre-push local value | Judgment method                                         |
> | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------- | -------------------- | ------------------------------------------------------- |
> | 1    | fix(util): check explicit path not intercepted by default exclude table + --exclude always empty (issue #463, P0-0.4)                                  | 366bc5a69        | ef959ca55            | Patch blob same                                         |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool attribute by import + Test doesn't contribute consumption surface)                                                    | e6cf1aeaa        | 84a6f6978            | Patch blob same                                         |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; includes generated warning-codes.md)                                                          | ddcbb12d0        | cd629b5fe            | Patch blob same                                         |
> | 4    | docs(meta): AGENTS item 10 ⑦ (commit period stash window convention)                                                                                   | ec94fa9d8        | 9a4ed5f3b            | Patch blob same                                         |
> | 5    | Fix round = F1 (overload sibling overload recovery reports W1005) + F2 (verification-side M1/M2/M4 recompilable redo) + F4 (snapshot generation noted) | b8380e6ab        | 3d9ef6eab            | Patch blob same (36962f318e42…, git range-diff no diff) |
> | 6    | Close fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three inconsistent expressions correction, register #464)                      | b940300ac        | e13984465            | Patch blob same (patch-id 9bda3cfe4c…)                  |
>
> **Hash note**: Each push's pre-push `git pull --rebase` rewrites local hash; "Judgment method"
> column above all "patch blob same" (`git diff <rev>~1 <rev> | git hash-object --stdin` both sides
> consistent + `git range-diff` no diff). This article doesn't reference any pre-push hash as
> authoritative source.
>
> **2. Three criteria red/green empirical caliber** (verifier self-built reproduction, doesn't trust
> implementer's criteria)
>
> | Criterion                  | Red (before binary 366bc5a69 empirical)                                                                                                                                               | Green (ec94fa9d8 and after)                                                                                                   |
> | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() rescues a.yx same-name foo; guard only imported not used also rescued)                                | Same fixture 2 truthfully reported (collision doesn't rescue + unused import doesn't count as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print rescued by tests/ reference (only 1)                                                                                                                        | Same fixture 2 (test reference doesn't spill out); ref_pool's test_ref_pool_excludes_test_role_references passed              |
> | 4.9.6 method reachability  | Self-built fixture (2 disambiguation overload calls + 1 uncalled) 3 W1005 false reports; method_call_args.yx 3 / method_overload.yx 1 (overload by full name coverage only reports 1) | Same fixture exactly reports uncalled orphan; method_unused 3 nails passed; corpus call_args 3→0 / overload 1→0               |
>
> **Fix round (F1) further tightening**: Unhit resolution sibling overload restored to report W1005
> (b8380e6ab——same fixture call ->Float reports ->String sibling, call ->String reports ->Float
> sibling, bidirectional line number nail). **Fall-back boundary (registered #464)**: Cross-unit
> (registry resolution) method call doesn't produce resolution entry, falls back to name-level
> exemption, cross-unit same-name same-order previously not reported——registered 4.9.7.
>
> **3. Process events and lessons (four hard requirements corresponding)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: Round 2 review once
>    registered "cross-unit same-name same-order not reporting W1005" as blocker (PA-1); subsequent
>    two fact reconciliation (implementer's archive + verifier's dual binary re-test on
>    b8380e6ab/b940300ac of same fixture set, diagnostic set byte-for-byte consistent) confirmed
>    it's **capability gap existing before round 2** not this round's regression——mechanism =
>    cross-unit method call doesn't produce resolution entry, falls back to name-level exemption;
>    disposition = change to medium, register issue #464, schedule WBS 4.9.7 next round.
> 2. **Reviewer publicly corrected before data and withdrew regression judgment**: Reviewer
>    initially cited before data ("−42 contains called methods") doesn't match their original output
>    (W1005 count=0)——verifier after check confirmed before capture is 0, reviewer's regression
>    judgment **publicly withdrew** based on this; −42 correct caliber = 26 files' called method
>    false report elimination (phase B), line-by-line traceable to behavior-run exit=0 files
>    (methods really called executed).
> 3. **t13 terminated due to contract premise not holding (not implementation defect)**: This task's
>    acceptance premise wrote "cross-unit same-name same-order will report W1005 regression fix",
>    but empirical (t14 dual binary) confirmed this behavior is registered gap (#464) not
>    regression——contract premise doesn't exist, task terminated; its probe value undertaken by t16
>    read-only judgment (confirmed `strip_unit_prefix` is dead branch).
> 4. **Document commit's real criterion is "really build once"**: Round 2 close document commit
>    (`042c07860`) made Deploy Docs red at VitePress's Vue template compilation——in-text
>    placeholders like `<rev>` / `<before-exe>` / `<after-exe>` etc. didn't go in backticks, parsed
>    as elements (reported `Element is missing end tag`, pointing to `<rev>` that line);
>    markdownlint doesn't catch this kind, and pre-commit's markdownlint hook in docs-only commit is
>    judged `no files to check` to skip (empirical). Disposition: placeholders uniformly go in
>    backticks, before push locally run `npm run docs:build` once (fix commit `0c837e1b4` with local
>    build exit=0 as evidence); if use `--no-verify` commit, commit-msg validation needs manual
>    supplementary run.
> 5. **Contract description not synchronized process omission**: t14's task description text
>    residual PA-1 old hypothesis (wrote "b.yx reports W1005" expectation), verifier in report
>    actively pointed out "per contract field judgment, description text doesn't count" and asked
>    captain to confirm——disposition = use task contract field as judgment basis, description text
>    as process lesson (**when re-dispatching/dispatching task description must be synchronized with
>    latest decision chain**).
>
> **4. Mutation tests (mechanically reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (m1b/m2b/m4b as F2's recompilable redo version——initial M1/M2/M4
> commented out if condition row leaving dangling block, was invalidated by t8 review F2's "compile
> failure mis-recorded as criterion red" judgment; redo version after per-patch cargo check exit=0 +
> trial red turned). M3 (collision attribution rollback) initial was real mutation, empirical
> ref_pool 5 criteria red, kept original judgment. M1b anchor retaken on b8380e6ab (F1 fix post
> probe section refactor, old anchor invalid). All trials: HEAD blob → mutated blob → restored blob
> identical to HEAD + git status CLEAN. Environment injection continues trial_env.ps1
> (core.autocrlf=false via GIT_CONFIG_*).
>
> **5. Warning surface full-comparison (caliber first clear)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) 4-tuple symmetric
> diff, project state channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: This round before capture already includes all fixtures ⇒ matched set =
> full set 337, no round 1's +8 (matched increment) / +16 (full set) dual caliber issue. Result:
> only_before = 42 W1005 (26 files, all called methods' false report elimination = phase B's −42;
> per file check all in behavior-run exit=0 baseline, methods really called executed——**not one
> really dead code rescued**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface
> ×2 + ref-pool-test-pool's ../src/util.yx:2 = phase A criterion surface). **Snapshot generation
> (F4)**: Phase A time point after capture (phase B pre 57 residual) and final (phase B post
> 15+2=17) are **two generations**, must note when referencing; another 1 case exception
> (interface_member_conflict.yx's Dog.name@18) is field access form, not in phase B's W1005 removal
> set, consistent before and after.
>
> **6. Final gate (close revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no product
> change. **Gate check arrangement (item 13)**: This batch pushed CI / Deploy Docs / Auto Translate
> run conclusions handed to timed check link for registration (run id + conclusion + consecutive
> push coverage relationship: later run covers previous one), not waiting in session.
>
> **7. Batch tree state**
>
> Both yx49 and yx49-b released (gc unified deletion at close); no this-batch newly added
> directories outside warehouse.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)—Round 3 Close (4.9.7)

> **4.9 round 3 batch record (2026-10-17 implementation → 2026-10-17 close)**: Covers 4.9.7
> (cross-unit method call resolution registration and receiver attribution, issue `#464`——WBS 4.9's
> last level-3 task; this round closes 4.9 entire level-2 task kanban check). Chain = production
> side t19 → independent verification t20 PASS → review t21 needs_revision (F1: injection surface
> false report) → fix round t24 → independent verification t25 PASS → review t26 PASS (incremental,
> verdict=pass). All commit numbers are post-push truth (judgment method = patch blob same;
> `git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent).
>
> **1. Decision chain and commits (post-push truth)**
>
> | Step | Content                                                                                                                                                                                             | Post-push commit | Judgment method         |
> | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ----------------------- |
> | 1    | Production side: cross-unit method call produces D20 resolution entry (`feat(typecheck)`, t19)                                                                                                      | `b8b3cc3f6`      | Patch blob same         |
> | 2    | Driver-side attribution: method reachability gives mark per definition unit, with resolution record name group disables pool's name-level exemption (t20 verification PASS / t21 review caught F1)  | `d5c00235a`      | Patch blob same         |
> | 3    | Injection surface fix round F1: candidate injection filters by "name ← providing module" **per pair**, eliminates E1107 false report (t24, includes F2 expression and F3 landing form single point) | `d5dd1a37b`      | Patch blob same         |
> | 4    | Nail test homing: `src/frontend/core/typecheck/tests/environment.rs`'s F3 nail moved to declaration surface `src/driver/tests/overload_injection.rs`                                                | `c2f887843`      | Patch blob same         |
> | 5    | dev tip                                                                                                                                                                                             | `e9675c9dd`      | merge commit, see "Two" |
>
> **2. Hash mapping and dev tip's two merge commits (source unconfirmed, not allowed to write member
> action)**: Round 3's four commits (table 1–4 above) all pushed to origin/dev;
> `git worktree list`'s main mount at `e9675c9dd`. dev tip has two **non-this-team command surface
> generated** merge commits (`5da05b885` / `e9675c9dd`, reflog shows `pull --tags` + push form),
> source unconfirmed——per item 10 ④ (reference must be resolvable by anyone) and item 13 (gate to
> special check) same close caliber, here only register fact and reflog evidence, no member
> attribution; also see "Five," lesson ② (push attribution convention must be verifiable).
>
> **3. D20 contract extension actual form (key form and consumption point)**: Production side packs
> "candidate / mixed IR name / declaration period parameter name" three parallel tables **in same
> order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) with unit crossing,
> registry merges deterministically by module key ascending (`src/frontend/module/registry.rs:150`);
> typecheck arm injects per pair per this unit's import face (`src/driver/arms.rs:354` →
> `visible_method_overloads`, `src/driver/arms.rs:1521-1579`: name ← providing module per pair,
> same-name conflict full name skip, own key position not externally merged), landing form single
> point `append_overload_group` (`src/frontend/core/typecheck/environment.rs:399`: three tables
> same-order append + equal-length debug assert); consumer side attributes D20 resolution product
> per **definition unit** (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`;
> `resolved_method_marks` `:621`), and makes name groups with resolution record disable pool's
> name-level exemption (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key
> form = entry name `{Type}.{method}#k` (`{Type}` is definition side bare name; `#k` declaration
> order).
>
> **4. Key scope and false report defense empirical (fix round F1 and five residual boundaries)**:
> F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set" combination
> merging another module's defined same-name type candidates into same key position ⇒
> `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, methods same name
> same order) fixture `check` reports 2×E1107, exit 1 (previously successful program compile fails);
> after fix `check`/`run` both exit 0, 4 W1005 truthfully report both sides' **uncalled** same-name
> definitions (reverse under-report surface also verified). Checked-in fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level nail
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set before and after fix
> line-by-line consistent (not reverted). Five residual boundaries and per-item empirical (single
> `check` definition file not report / name layer ambiguity E5006 full name skip / boundary (c) per
> t20 empirical rewrite / inline alias and `ns.T` not inject candidate surface / whole import
> `use M` and std candidate surface not produce entry) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §cross-unit method reachability attribution
> "residual boundaries", not repeated here.
>
> **5. Process events and lessons (four types continuation)**:
>
> 1. **Contract wording written narrowly directly becomes defect**: F1's root cause is contract only
>    wrote "only inject module surface that can be named by bare type name", didn't write "attribute
>    by name per pair"——implementation literally took "name set × module set" combination, review
>    caught E1107 false report. When describing behavior boundary in contract must write
>    **combinatorics** dead (per name per pair / full combination), can't leave for implementation
>    self-understanding.
> 2. **Convention must be verifiable: "push by captain" only written in dispatch message ⇒ when
>    third-party push/merge appears can't attribute**. This round dev tip has two source-unconfirmed
>    merges (see "Two"), finally relied on reflog reverse inference and withdrew one error
>    attribution. Verifiable form = members don't run `git push`/`fetch`/`pull`, sync uniformly
>    `pull --rebase`, use reflog check.
> 3. **Infrastructure unstable, save patch first then change members**: This batch total 5
>    unrecoverable failures (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background
>    sub-agent no message exit); disposition convention = already produced evidence and patches
>    first land to **workspace** batch directory (under `target/`, not checked in), but **conclusion
>    and numbers must land in this record in-place**——workspace path not as document reference
>    source (AGENTS meta rule ①). Then change members to continue, same session two consecutive
>    failures don't bet third time.
> 4. **Document commit's real criterion is "really build locally once"**: Round 2 close document
>    commit (`042c07860`) made Deploy Docs red at VitePress's Vue template compilation (angle
>    bracket placeholders `<rev>` etc. not in backticks parsed as elements, reported
>    `Element is missing end tag`); markdownlint doesn't catch this kind, and pre-commit's
>    markdownlint hook in docs-only commit is judged `no files to check` to skip (empirical twice).
>    Fix commit `0c837e1b4` with local build exit=0 as evidence. This round the lesson's red→green
>    run: Deploy Docs **39011946793** (red) → **39014696984** (green, see "Six").
>
> **6. Round 2 gate conclusion supplementary registration (item 13, timed check link backfill)**:
> Round 2 close commit `b940300ac` push—— CI **39011946778** success / Auto Translate
> **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red, i.e.
> "Five, 4"'s angle bracket placeholders; fixed by `0c837e1b4`); fix commit `0c837e1b4` push after
> three green: CI **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**.
> Consecutive push coverage: CI@`0c837e1b4`'s **39014356642** cancelled by subsequent run per
> concurrency (cancellation ≠ failure, later run covers previous, consistent with round 2 §six
> caliber).
>
> **7. Final gate (close revision `e9675c9dd`, local number; CI conclusion handed to timed check,
> see "Eight")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR
> snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent (rerun
> `scripts/docs/gen-error-code-docs.py` after `git status` clean); corpus baseline 346 → 347 → 348
> lines (each +1 behavior record, other lines byte-for-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 lines).
>
> **8. Gate check arrangement and conclusion (item 13)**: This batch pushed CI / Auto Translate /
> Deploy Docs conclusion handed to timed check link for registration (not waiting in session, also
> don't write "not waited" as "green"). **Backfill (2026-10-17 timed check)**: `d5c00235a`'s CI
> `39023417670` = **failure**——Docs Truth reports "error code document lags behind code
> table/locales", i.e. this commit missed generation product same submission; fixed by its
> subsequent `d5dd1a37b` (regenerate generation product): `5da05b885` CI `39027788246` / Deploy Docs
> `39027788268` / Auto Translate `39027788340` all success, covering previous one; `e9675c9dd` CI
> `39028827871` / Deploy Docs `39028827896` success; `d0cfc4e62` CI `39033963240` / Deploy Docs
> `39033963278` / Auto Translate `39033963261` success. Three commits' runs each ran to terminal
> state, this round no concurrency cancellation. Lesson: **"local generator idempotent" ≠
> "generation product checked in same submission"**——should check CI conclusion per commit before
> close, not only look at latest one.
>
> **9. Batch tree state (close gc, status output)**: Before close `scratch.py status`——t25
> (holder=-) and yx49b (holder=-, previously stale held by deactivated member verify-r3, captain
> `release --force` released) two trees both no holder; `gc` deletes two, releases about 12.7 GB;
> after gc `status` = "no batch tree", `git worktree list` only main mount left. Also: `yx49r3` and
> `yx49r4` are not batch trees (no meta.json), are members' **workspace** directories for
> evidence/probes, not "residual batch trees". No this-batch newly added directories outside
> warehouse.

## P4 4.4 Proof Layer and wasm Wrap-up — Close (4.4.1 / 4.4.2 / 4.4.3)

> **1. Batch composition and rounds (batch name `yx44`)**: Three level-3 tasks 4.4.1 / 4.4.2 / 4.4.3
> with wasm attribute reachability judgment same batch; batch verification tree built by
> `scripts/dev/scratch.py init` in workspace `target/` (first full build; after close gc deleted).
> Roles and rounds: implement 4.4.2 = t1 (12 files; local `1fc135437` → post-push
> `24028ffe8`) ｜ implement 4.4.1's layer order rewrite = t2 (only document/comments, zero behavior
> change, based on `b5e68b3c9`) ｜ implement 4.4.3 = t3 (11 files, includes new code W1082; local
> `2d5e610cd` → post-push `52d2276ca`) ｜ wasm reachability judgment = t4 (only judgment, zero
> cleanup, judgment revision `b5e68b3c9`) ｜ 4.4.1 close and one document commit = t8 (7 files
> 173+/43-; local `29ebd2145` → post-push `d07993157`) ｜ independent verification = t5 (frozen
> `29ebd2145`) ｜ round 1 review = t6 (revision `1b593e993`) ｜ patch = t9 (3 files 9+/10-; local
> `1ebddaf95` → post-push `1b593e993`) ｜ fix round = t10 (3 files 23+/23-; local `657c3d5e3` →
> post-push `e184d88b6`) ｜ round 2 review = t11 (`657c3d5e3` = post-push `e184d88b6`, patch-id all
> equal) ｜ close = t7 (check / record / batch tree gc / document real build / one document commit;
> local `c8412565c` → post-push `da724940b`). **Failure and redo**: t6 terminated with
> `needs_revision` (review round per contract must fail and give findings, not an accident); t7
> first round interrupted at batch tree gc stage, already completed products left in main warehouse
> work tree uncommitted, second round took over "fill gap + review"——didn't redo already completed
> part (this round added: 4.4 check and evidence note, this record, three path rewrites pointing to
> deleted batch directory, document real build and close commit). No cancelled rounds.
>
> **2. Per-task output and measurement numbers (raw commands and output see evidence summary and
> archive; here only conclusions)**:
>
> - **4.4.1**: `layers/README.md` (including `layers/mod.rs` top comment)'s layer order table
>   rewritten to **actual call order** (predicate → termination → ownership, call point anchored at
>   `checker.rs::check_module_impl`), `equivalence` described per real identity (no production call
>   point); same source declaration 7 places closed (`01-routing` / `02-stage-contract` / RFC-039 /
>   `06` / typecheck README / layers two files); mechanical reconciliation script on batch tree
>   green baseline **11/11 PASS** (criteria 11 = README side 4: actual call order declaration / two
>   entry anchors / old order declaration disappeared; code side 7: 5 line anchors +
>   `check_module_impl` no short circuit + `equivalence` zero production calls; reconciliation
>   script one-time, recycled with workspace, criteria 11 listed can be rewritten per this),
>   mutation (c) revert old declaration → reconciliation 2 FAIL i.e. red.
> - **4.4.2**: Solver acquisition surface unified to `proof/smt/backend.rs`'s process-level shared
>   singleton + sole closure entry `with_shared_solver(consumer, f)`; three consumer points
>   (predicate / termination / ownership) all changed to shared entry, **compile production chain
>   `default_solver()` call points = 0** (only backend selection point sunk to private
>   `new_backend()`). `wasm/src/lib.rs`'s `test_smt` self-check call point is **existing** (earlier
>   than this batch, from `b5e68b3c9`) non-test call point, belongs to "test and independent scene"
>   surface, doesn't constitute D58 prohibition violation. Independent verification self-built
>   reading (one compilation): predicate acq=2 hit=1 miss=1 ｜ termination acq=1 hit=2
>   miss=2 ｜ ownership acq=2 hit=1 miss=1, three points hit/miss sum (4,4) = shared instance
>   accumulated `Some((4,4))`; mutation (a) singleton degenerate → predicate hit=0 i.e. red.
>   **Change verb surface disclosure**: This commit contains `z3_backend.rs` 15 lines——`cache_stats`
>   migrated from inherent method to `Solver` trait block (returns
>   `Some(instance-internal count)`) + 1 line comment redirected, no semantic coupling. D58
>   prohibition mechanized: same-thread reentrance → panic contains D58, probe `recv_timeout(120s)`
>   doesn't hang.
> - **4.4.3**: `body_checker` missing's silent degradation converged to
>   `TypeChecker::ownership_inputs` (sole criterion point) + new code **W1082** (no overlap with
>   existing W code fact: W1080 constraint degradation / W1081 solver missing / W1090 opposite
>   direction); nail tests 2/2 green; healthy path corpus 348:348 zero diff; **degradation branch
>   production surface unreachable exhaustive holds** (CLI four entry points + eval/repl + format +
>   LSP + wasm playground + all `ProgramKind` gathered into `check_module_impl`: `init_body_checker`
>   unconditionally before ownership block, function in 0 return/break/continue, `body_checker` no
>   `.take()`) ⇒ W1082 no user-visible surface. Mutation (b) delete emit → nail test i.e. red.
>   **W1082 belongs to external contract table change (class E), user approved**; `locales/zh.json`
>   / `locales/en.json` same submission as code, other languages filled by i18n bot after push (run
>   see "Nine").
> - **wasm attribute reachability judgment (t4)**: `orchestrator.rs` current tree **10 = 4 positive
>   arm + 6 negative arm** (caliber = same line in `#[cfg(...)]` attribute text, both positive and
>   negative count; `cfg!` runtime macro 0 hits); 4 wasm positive arms in playground runtime
>   unreachable but **compile-time load-bearing** (call point not gated, delete arm alone ⇒ wasm
>   compile fails), 6 `not(wasm32)` negative arms native three phase table load-bearing ⇒ D58's
>   envisioned "delete unreachable" in this file **no applicable target**, real cleanup form = whole
>   project-mode subtree gated on wasm32 (registered P10 S7, three unverified preserved as-is).
>
> **3. Independent verification (t5, frozen `29ebd2145` = post-push `d07993157`'s content——two
> revisions per-commit patch-id all equal, whole tree diff only in `docs/src/en/**`,
> `docs/src/.i18n-cache.json` etc. bot product surface——`git range-diff` three commits all `=`)**:
> gate **8/8 all green**——`cargo fmt --check` 0 ｜
> `clippy --all --all-features --all-targets -- -D warnings` 0 ｜ `cargo test` **3128 passed / 0
> failed** (7 test targets total; baseline 3126 + this batch 2) ｜ corpus diff **348:348**
> line-by-line zero diff (sort diff 0 lines) ｜ IR snapshot `UPDATE_SNAPSHOTS=1` rerun then
> `git status` zero drift ｜ `check-obligations.py` 21 = 13 + 8, containment 0 gap ｜
> `check-docs-truth.py` source 156 / document 156, CLI 21 commands consistent ｜ error code
> generator rerun before and after tree clean. Build warnings only 4 bot languages W1082 missing
> entries (known state) + Z3 download noise, zero code warnings. **Contract verify caliber
> correction (must two JSONL paths)**: Corpus gate call caliber =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generation>.jsonl`——**two
> JSONL paths both required**; miss second parameter is comparing baseline to baseline (always zero
> diff, criterion idles). This batch per two-path caliber executed and dropped evidence (t5's
> `corpus-r44.jsonl` product).
>
> **4. Mutation evidence (3/3, mechanically reversible)**: All via
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per-file blob
> compare to HEAD → require `git status` clean):
>
> - (a) Singleton degenerate → shared reading predicate hit=0 miss=2, test FAILED exit 101;
>   `backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → restore = HEAD.
> - (b) Delete W1082 emit → nail test FAILED exit 101; `checker.rs` HEAD=`70c04a41e020` → mutation
>   `db0a623c9b9a` → restore = HEAD.
> - (c) Revert old layer order declaration → reconciliation 2 FAIL exit 1; `layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → restore = HEAD. Three trials after restore
>   `git status` all CLEAN. trial (c) first run absolute path WSL parse failed exit 127 (not
>   criterion red), changed relative path rerun as standard.
>
> **5. Review rounds and conclusion**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision**——3 findings all document line number anchor inaccurate (F-01 medium: `02` §"wasm
> current state" 4 group expired line numbers; F-02 low: `02` and `09`'s 4.4.3 delivery anchor
> `checker.rs:1283-1293`; F-03 low: `06`'s comparison statement), four substantive deliveries all
> pass. → fix round (t10, local `657c3d5e3` = post-push `e184d88b6`, 3 files 23+/23-, plus two
> mechanical criteria) → round 2 (t11, same revision) **verdict = pass, new finding = 0**. **Final
> check basis = round 2's pass**. **Round 3 (close tip `da724940b` on)**: Independent verification
> t12 **verdict = pass** (binding `da724940b`: four rule texts byte-for-byte, 3.19 / S7 / S8 fields
> and scope complete, count table and decision chain recount, dead reference zero,
> `check-docs-truth` 0, `markdownlint` 0); review t13 **verdict = needs_revision**, 4 findings——F-01
> medium (t10 / round 2 only gave rebase pre-hash, §fourteen's list inconsistent with origin/dev
> actual), F-02 low (`02:379`'s 09 4.2.8 anchor shifted to `:274` by this batch itself inserting one
> line each at `09:144`/`:182`), F-03 low (`09a` wrote local drive letter path), F-04 low (level-3
> count row note and 157 ≠ table head 156) → fix round t14 (this commit) closes one by one. **t10
> contract caliber decision**: Fix surface uses **task record's 5 F-01…F-05** as standard (F-04/F-05
> are mechanical criteria), dispatch brief not criteria.
>
> **6. User decision (2026-10-18) and rule solidification**:
>
> 1. **D58 entry name rewritten in place**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, rewrite body in place, **no revision note**
>    (t8 completed in 4.4.1 one commit).
> 2. **wasm judgment registered as P10 S7** (decision B): with "owning phase: P10; whether it blocks
>    mainline: no" + three unverified **preserved as-is**; `06`'s F2 "needs decision" line
>    redirected to that registration; t4 output's "### F2 judgment" subsection not touched.
> 3. **Crash item registered as P3 3.19** (decision A): symptom / cause points to
>    `proof/smt/translate.rs`'s sort inference / D59 **class A/B** (crash) / acceptance line (this
>    shape no longer abort——conservative degradation or diagnostic, corpus and IR snapshot zero
>    drift); **characterization = pre-existing** (t5 two revision comparison, see "Seven" ⑤).
> 4. **Rule solidification 1** "correction written in place, no old-new comparison": land
>    `coding-rules.md` (added clause), `07-equivalence-oracle.md` §semantic correction split
>    criterion item 3, RFC-039 **D59 row** item 3 (same submission registration = WBS row marked
>    error type + in-place correction of affected body + PR must-fill block "error type" one line).
> 5. **Rule solidification 2** "contract `inScope` uniformly write path prefix resolvable by path
>    matcher": land `AGENTS.md` team orchestration convention item 6.
> 6. **New code W1082 approved** (external contract table change, class E).
> 7. **Register P10 S8** "documentation correction form convergence (in-place rewrite, delete
>    revision note)": includes 7 audit list scope and acceptance line.
> 8. `09` top count table: level-2 **63** (60 + 3.19 / S7 / S8), level-3 **157** (154 + 3), with
>    2026-10-18 note; level-3 item's accounting review see "Fifteen".
>
> **7. Process facts that must be truthfully recorded**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks——shared work tree had teammates' work
> in progress, `--fix` would incidentally rewrite others' files; **equivalent gate already manually
> fully run in final tree** (fmt --check / clippy `-D warnings` both 0). ② `pre-commit`'s stash
> window indeed happened (`Stashing unstaged files…` / `Restored changes from…`), after commit
> `git status` check shows teammates' work in progress intact. ③ **4.4.3's inScope lesson**: Prose
> qualified `docs/src/(only error code generation product)` rejected by `changedPaths` validation ⇒
> `inScope` uniformly write matchable path prefix (already solidified as rule, see "Six" 5). This
> task **actually 11 files**, of which 7 due to declaration writing not resolvable by path matcher
> not listed in `changedPaths`, already listed in task result one by one (`codes/w1xxx.rs`, RFC-013
> code table, `reference/error-code/`'s index and each E segment page etc.). ④ **W1082's locale
> window closed**: `locales/zh.json` / `locales/en.json` same submission as code, other languages
> filled by i18n bot after push (run see "Nine"). ⑤ **Z3 `Sort mismatch` crash judgment (t5
> criterion ⑦)**: **pre-existing**——same probe, same input at `b5e68b3c9` and frozen revision
> `29ebd2145` stdout **byte-for-byte consistent**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1; static attribution consistent (4.4.2 to `translate.rs` 0 line change, to
> `z3_backend.rs` only `cache_stats` migration). **Degradation branch production surface unreachable
> exhaustive holds** (W1082 no user-visible surface). Probe three lines source code:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **t5 frozen surface and t9's coverage
> difference**: t9 (three comment/document patches, `1b593e993`) commit later than t5's frozen
> revision `29ebd2145`, is zero behavior change, its acceptance covered by t9's own mechanical
> check + t6 review (t6 at `1b593e993` **reran** t5's key measurements, not retold). ⑦
> `docs/src/en/**` not manually modified, regenerated by post-push auto-translate workflow (this
> batch the bot landed two commits: `92d490695` i18n locales fill W1082 other languages, `63b6cdf37`
> auto-translate documentation). ⑧ **t9 cross-boundary observation three decisions**: ㈠ t9
> incidentally corrected `02` same sentence's `wasm/src/lib.rs` line count and gating point line
> numbers (73 / 139·153·170 → 152 / 151·170·187)——**accept** (same caliber same number necessary
> surface, commit information already listed); ㈡ RFC-039 **D35 row** still writes "wasm 27 files'
> cfg branches"——**keep unchanged** (decision row is historical decision record, its premise
> correction already carried by P10 S7 registration, no rewriting history); ㈢ `docs/src/en/**`'s
> old entry name translation aligned by auto-translate. ⑨ `09` / `09a`'s original pointing to
> deleted batch directory's two paths in close round changed to **inline in place** (evidence
> summary merged into this record this section, probe three lines source code inline). **2026-10-21
> correction**: At that time changed pointing was still workspace path, equal to moving violation
> down one layer; from this round on this summary's delivery commit, gate numbers and CI run id all
> inlined, document no longer references any workspace path (AGENTS meta rule ① / item 10 ④).
>
> **8. t8 acceptance ④'s caliber decision**: Acceptance original text required
> `git diff | grep 'revision note\|originally\|now changed to'` to be empty. Empirical **default
> context caliber unreachable**——D58 row adjacent D59 / D61 decision row itself contains "revision
> note / originally" wording (not this task's change). Decision: per **new line / deleted line /
> `-U0` increment line** three calibers all empty accept, and write **subsequent similar acceptance
> only grep new/deleted line**——criterion should fall on this change's line, not whole diff's
> context.
>
> **9. CI gate conclusion (AGENTS item 13 ①, timed check 2026-10-18 17:26 backfill; run id and
> conclusion inline)**: First push (3 commits, tip = `d07993157`): CI **39110908163**
> success ｜ Deploy Docs **39110908155** success ｜ Auto Translate Docs **39110908193**
> success ｜ Auto Translate I18n **39110908144** success; this tip covers `24028ffe8` (4.4.2) /
> `52d2276ca` (4.4.3) / `d07993157` (4.4.1) three commits all content. Second push (1 commit, tip =
> `1b593e993`): CI **39112293391** success ｜ Deploy Docs **39112293062** success ｜ Auto Translate
> Docs **39112292903** success; covers all four commits including t9 patch. **No concurrency
> cancellation**, latter covers former. Third push (2 commits, tip = `da724940b`): CI
> **39121538555** success ｜ Deploy Docs **39121538562** success ｜ Auto Translate Docs
> **39121538541** success——three runs' headSha all = `da724940b` and **all terminal state**
> (2026-10-18 third round timed check backfill); round 3 independent verification t12's "CI not
> terminal" observation thus closed. Unrelated: Nightly **39081392694** failure's headSha =
> `290f9ebc`, not this batch's commit surface.
>
> **10. Three `pull --rebase` (auto-commit counterpart during push)**: Counterparts are
> auto-translate and i18n commits; six hashes rewritten and per **patch-id all equal**——local
> `1fc135437` → `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`, `1ebddaf95` →
> `1b593e993`, `657c3d5e3` → `e184d88b6`, `c8412565c` → `da724940b` (after third rebase origin/dev
> head = `da724940b`); `git range-diff` verified **per-commit content unchanged** (three commits all
> `=`). Members didn't execute `git push` / `fetch` / `pull`, sync and push by captain uniformly.
> **Forensic reminder**: Henceforth reference commit number uniformly use **post-push hash** (above
> right value); `09`'s check row per convention doesn't write commit number.
>
> **11. Uncovered surface (must not write "not tested" as "passed")**: ① doc-tests 0/0 doesn't carry
> criterion; ② **wasm32 target real compile not done** (this surface covered by CI's
> `_build-wasm.yml`, this batch's wasm judgment is static argument); ③ mutation only covers 3 places
> contract named, other gates no reverse break validation; ④ corpus diff not separately run on
> `b5e68b3c9` (only run on frozen revision); ⑤ i18n bot filled other languages' W1082 text behavior
> not verified (only verified `zh.json` / `en.json`); ⑥ round 1 review and fix round reported **same
> class but not in contract findings' existing drift** not fixed (list see "Twelve"), ownership
> pending user decision.
>
> **12. `02` full article existing drift observation list (user 2026-10-18 agreed to register;
> 2026-10-21 landed as S10)**: Caliber = all **pre-existing drift** (not introduced by this batch),
> **doesn't constitute this batch's finding**; list discovered by fix round, round 2 review at
> `657c3d5e3` (= post-push `e184d88b6`) independently spot-checked and gave mapping, recorded per
> item (list inline below, original files recycled with workspace):
>
> 1. `02` §(a) entry table 3a-3d / 5 / 5b: `02:72` quotes `orchestrator.rs:273-399` → now `:155`;
>    `02:73` `:450` → `:250`; `02:74` `:1374` → `:985`; `02:78` `util/diagnostic/mod.rs:565` →
>    `:681`; `lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` no longer there; **content
>    level**: `run_diagnostics` and "otherwise manual lex→parse" already replaced by 4.6.1 (delete
>    manual sequence).
> 2. `02:1016`: `compile_project :99-237` → now `:107-134` (28 line Program constructor).
> 3. `02:360-361`: "missing degrades to warning" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435); `build.rs:65-72` already changed `panic!` hard fail;
>    `_build-wasm.yml:35-59` interval needs retake.
> 4. `02:127` mechanism pointer: mechanism actually in `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-description); conclusion still holds (multi-file arm loses warnings),
>    inaccurate is "result only downstream use" sentence (`type_result` also via `arms.rs:1389` →
>    `helpers.rs:259/:636` feeds `overload_resolutions`); suggested fix = pointer change
>    `typecheck_orchestrated`, "only downstream use" change to "warnings field no read point".
> 5. `02:128`: `check_project:385-395` / `250-252` needs retake.
> 6. `02:334` (§S5 (d) investigation paragraph): quoted old code fragment already replaced by
>    4.4.3's `ownership_inputs` criterion point.
> 7. `02` change list table other rows: `predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This close doesn't preset S9's WBS entry** (user didn't decide); after getting decision then
>    per registration form land `09`. **Subsequent (2026-10-21 landed)**: User 2026-10-18 agreed to
>    register, this list already as `S10` "`02` body and fact alignment (line number anchor +
>    content level expired)" land `09` (level-2 +1 / level-3 +1 = S10.1)——**originally planned S9
>    number already occupied by "pre-release audit abstraction close"** (S9.1 = issue #473 / S9.2 =
>    issue #474), per 09 "number must be unique in whole document" registration convention deferred
>    to S10. **Registration timing deviated from original plan** (originally wrote "with P5's close
>    document commit land together"), per user 2026-10-21 instruction this batch landed immediately.
>
> **13. Batch tree recycling (gc and status raw output; output's tree path per warehouse relative
> path)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted yx44-evidence (released about 0.0 MB)` /
> `[scratch] gc complete, total released about 0.0 MB`; `python scripts/dev/scratch.py status` →
> `yx44-evidence            holder=-` next line `<workspace>/yx44/tree`——registry recorded path
> **already doesn't exist**, this name is registry residual (`gc`'s `_remove` deletes
> `meta["path"]`'s parent directory i.e. workspace's `yx44`, residual `meta.json` still falls in
> scan surface; its content batch=`yx44`, `base_rev=29ebd2145`, `lock=null`, contains t5 / t6 /
> t11's holding history); Close round renamed that registry file to `meta.retired.json` to make it
> exit scan (**not delete any evidence file, not create any tree**) →
> `python scripts/dev/scratch.py status` = `[scratch] no batch tree`; then run `gc` =
> `[scratch] no batch tree, no recycling needed`; `git worktree list` only main mount left (`dev`).
> Batch original evidence directory (no `meta.json`, **not batch tree**) at that time kept as
> archive; **from 2026-10-21 on workspace directory no longer as evidence source**——close summary
> inlined in this section, original directory deleted with cleanup (AGENTS meta rule ①). Acceptance
> expression (resolvable): `git status` clean; after batch tree gc
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main mount; no
> this-batch newly added directories outside warehouse.
>
> **14. Document real build and commit hygiene**: `npm run docs:build` local **exit 0** (AGENTS item
> 10 ⑨——markdownlint doesn't catch VitePress's Vue compile error, and docs-only commit is judged
> `no files to check` to skip markdownlint); this batch's check / note / registration / rule
> modification accumulated into **one** document commit (item 8), fell in batch close; members
> didn't execute `git push` / `fetch` / `pull`. **This batch's final two commits both pushed**
> (close tip on `git rev-list --count origin/dev..HEAD` = 0, origin/dev head = `da724940b`):
> `e184d88b6` (round 1 findings' document anchor fix, 3 files 23+/23-) + `da724940b` (close document
> commit, 7 files 96+/10-; includes check / record / three path rewrites / registration and rule
> modification).
>
> **15. Count table accounting review (round 3 fix F-04; method and result both reviewable)**:
> Judgment object = `09` top "Location and Scope" table's level-3 count row. **Method** (two
> independent calibers, per revision use `git show <rev>:<file>` take bytes recount): ① **Per
> level-2 entry sum**——take each level-2 kanban row parenthetical "N level-3 tasks" (row start
> `- [x]`/`- [ ]` bold title start, row in contains "level-3 tasks", includes S7 row inline written
> "1 level-3 task"): current tree 70 level-2 entries, **sum = 157**. ② **Historical
> self-consistency**——same caliber at `366bc5a69^` get 66 entries, **sum = 151 = period table head
> 151** (self-consistent); on `366bc5a69` that day newly registered `4.9.6` and `0.4.1` two level-3
> (sum 151 → 153), but table head only writes 151 → 152 ⇒ **table head from that commit on missed
> 1**; subsequent each revision all diff 1 (`92d490695` / `1b593e993` / `d07993157` / `e184d88b6`:
> sum 154 vs table head 153; `da724940b`: sum 157 vs table head 156). **Result**: Truth = **157**,
> so correct table head (`09:13` level-3 156 → 157); note 144+2+2+2+2+1+1+3 = 157 originally
> correct, not changed; pre-this-batch base = **154** (not 153). **Level-2 count row not
> touched**——that row's caliber never aligned with kanban entry count (`366bc5a69^` is 66 entries vs
> table head 59), this time no reviewable evidence to overturn 53 + 10 = 63, don't change number to
> fit. Incidental correction (same round): §two's 4.4.1 parenthetical decomposition per
> reconciliation script actual item count changed to "README side 4 + code side 7 = 11", consistent
> with **11/11** and script 11 `ck` calls.

## P3 Debt Round 1 Close (3.5 / 3.6 / 3.4.6 / 0.5)

> **1. Batch composition and rounds**: This batch closes P3 side remaining debts——**3.5 Higher-order
> type parameter call path** (3.5.1–3.5.3), **3.6 Function type value call path** (3.6.1–3.6.4),
> **3.4.6 compile path consumption warnings** (class D, user 2026-10-19 decision "do"), and
> registered together **0.5 test suite layering** (P0, issue #486 slice) and **3.20 dead code
> analysis's existing false reports** (3.20.1 / 3.20.2). Roles and rounds:
>
> - Implementation: 3.5.1–3.5.3 = t1 ｜ 3.6.1 = t2 ｜ 3.4.6 = t6 ｜ 3.6.2 = t7 ｜ 3.6.3 = t8 (main
>   body + close two commits) ｜ 3.6.4 = t9 ｜ 0.5.1 = t10 (#486 slice)
> - Independent verification: Round 1 = t3 (frozen revision `81d5e414e`, covers 3.5 / 3.6.1–3.6.3 /
>   3.4.6) ｜ 3.6.4 = t11 (round 2, frozen `8e32e8636`)
> - Review: Round 1 = t4 (verdict = **needs_revision**, F1/F2/F3) ｜ fix round = t12 (one
>   commit) ｜ round 2 = t13 (verdict = **pass**)
> - Close = t5 (check / registration / this record / batch tree gc / document real build / one
>   document commit)
>
> **Failure and redo**: t4 terminated with needs_revision (review round per contract must fail and
> give findings, not an accident) → t12 real fix → t13 review pass. t8 internally experienced one
> attempt interrupt (process crash), previous round evidence kept in batch directory, continuing
> attempt completed then two commits checked in. No cancelled rounds.
>
> **2. Per-task output and measurement numbers (conclusion caliber; raw commands and output see each
> batch evidence directory)**:
>
> - **3.5.1** (unifier matches constructor variable at head position): Criterion converged to
>   **single implementation** `is_ctor_reference` (`pub(super)` free function, warehouse 1
>   definition 3 consumers)——this is t12's real fix for t4's F1 (originally
>   `inference/statements.rs` had a separate same-name method copy, constituting prohibition three
>   meaning same-name two copies). Head rename no longer inline copy: `environment.rs` and
>   `expressions.rs` each first traverse TypeRef / variable table construct head_map, then uniformly
>   hand to shared implementation `substitute_ctor_heads` (`types/substitute.rs`) execute——two
>   places' head_map construction criterion unchanged, rename only acts on head by name, empty table
>   returns as-is (t12's F2 real fix). Equivalence argument: traversal doesn't change Generic head
>   name.
> - **3.5.2** (explicit type argument accepts type constructor, E1010 disappears): Red state E1010 /
>   E1002, green state exit 0.
> - **3.5.3** (mono product replaces constructor variable): Symbolic type argument criterion
>   narrowed by container generic parameter name; in product constructor variable replaced with real
>   constructor (IR dump empirical: specialized function `Function probe(Box, int64)`).
> - **3.6.1** (local function type value called by name): Criterion sole source =
>   `is_static_fn_name` (warehouse 1 definition 1 call), aligned with variable evaluation path's
>   name resolution priority (local → global), no new second resolution table added; IR empirical:
>   module initialization section changed from `LoadConst String("f")` + `CallNative` to
>   `LoadGlobal` + `CallDyn`. Regression surface: same-name "function binding + value binding"
>   unreachable (all compile-time E2010), non-function value called by name still E1065.
> - **3.6.2** (`use` alias imported function value called by name): Originally reported
>   `E6006 Native function not found: lib.f` (`use lib.{f}` and `use lib.{f as g}` two forms). Fix =
>   in `is_static_fn_name` **add one arm** (not copy), add sole implementation
>   `use_alias_value_slot` undertakes "use alias → qualified name → global slot" two-table joint
>   lookup, consumer points 2 (call form criterion + variable evaluation path).
> - **3.6.3** (nested function body references outer frame value): **Criterion changed
>   judgment**——not "let it run correctly", but this form per RFC-009 §2.3 + SPEC §12.3 should **be
>   rejected at compile time** (D59 class C: change silent wrong value to loud; per `09` §P3's D59
>   row, "silently give wrong answer" goes to class C——this batch's close brief recorded as "A/B
>   class", both refer to "old behavior not successful", landing method both "red before green +
>   compile-time loud rejection"). Original form = **no error, calculate wrong value (void)**: red
>   state stdout `void`, EXIT=0 (silent wrong value archive); green state compile-time E1001
>   `Unknown variable`, EXIT=1, with existing "unknown variable" path **same code same text**.
>   Criterion sole source = capture barrier (`enter_capture_barrier`), compile-time name (type
>   parameter) not blocked; E1001 still only emitted in typecheck, registration sole. **Assignment
>   path caught by E2010** (t8's close commit: declaration / assignment judgment doesn't apply
>   capture barrier)——otherwise this fix itself would open a new "same-name assignment silent wrong
>   value" channel (c1 form: only barrier degenerates to exit 0 print 10).
> - **3.6.4** (`use lib` whole import then `lib.f(...)`): Goes FieldAccess callee's namespace
>   branch, originally reported E6006 `lib.f`. Fix = namespace branch extracts qualified name then
>   first judges global slot layout → changes to indirect call; qualified name resolution and slot
>   query reuse `resolve_namespace_data_access` (same source as data access), no new resolution
>   table added.
> - **3.4.6** (compile path consumption warnings, class D): Before and after change external
>   performance = **multi-file `run` previously always output no W code** (red state archive:
>   multi-file [] vs single-file [(W1001,3),(W1002,5),(W1003,1)]), now four paths (single-file run /
>   multi-file run / single-file check / multi-file check) W code set always {W1003@1, W1001@3,
>   W1002@5} (non-empty set). Corpus baseline: 348 records **46 changed**, new W code **118** (W1001
>   34 / W1002 12 / W1003 13 / W1004 42 / W1005 17), **118/118 line-by-line attributed to file's
>   real existing unused definitions (0 not located)**, pure increment (0 deleted, 0 exit / stdout /
>   check changed). **D59 four fulfilled**: ① red before green (red state archive + turn to nail
>   test) ② doesn't change "originally successful program"'s external performance (pure increment, 0
>   exit / stdout / check changed) ③ same submission registration (this record + `09`'s 3.4.6 row) ④
>   baseline same submission regenerate and manual review diff. **Superset approved**: W1004 / W1005
>   / W1006 also enter `run` (user 2026-10-19 approved), reason = **one criterion, one
>   channel**——only put W1001/W1002/W1003 equals creating second filter judgment. **External
>   contract surface change**: `orchestrator::compile_project` return value from
>   `Result<ModuleIR, _>` changed to `Result<(ModuleIR, FileDiagnostics), _>` (warehouse consumers
>   2 + 2 test direct drive points sync adapted).
> - **0.5.1** (test resource profile marker + exclusive lock + fast/full minimum consumption
>   surface): Added `scripts/dev/testsuite.py` (`markers --check` and `cargo test --list`
>   bidirectional reconciliation, `plan` budget bin packing, `run fast|full` budget insufficient
>   reports gap doesn't silently skip, `manual` tier never implicitly executed) and
>   `src/util/test_support.rs` (cross-process file lock, take lock in test body ⇒ any call path
>   mutually exclusive); in-place `cost:` markers 3 (2600 tier exclusive, peak extrapolated from
>   5000 tier measured linear, **not measured**; 5000 tier exclusive, 15.6 GiB / 72.6 s measured;
>   snapshot update manual, peak and time both extrapolated). Self-verification numbers:
>   `markers --check` check 3149 `#[test]`, 3 mechanical mutations each exit 1, `run fast` batch
>   wave 2793 passed / 47.7 s + exclusive wave 1 passed / 14.6 s, budget 1 GiB `run full` lists gap
>   7.11 GiB, two processes contend lock empirical `held→blocked→held(waited 2.0s)`. **Independent
>   verification belongs to #486 subsequent scheduling** (this batch only implementer
>   self-verification). **0.5.2 awaiting scheduling** (pre-commit / CI wiring + 2600 tier measured
>   calibration + `test-specification` detailed rules).
>
> **3. Independent verification (t3, frozen `81d5e414e`; red state baselines `c599de7bb` /
> `1725d0215` / `ef2334de5` all HEAD ancestors, `git merge-base --is-ancestor` verified)**: Probes
> all self-built by verifier (batch tree built by `scripts/dev/scratch.py init` in workspace, probes
> and patches in its `probes/` / `patches/` subdirectory), doesn't retell implementer's conclusions;
> that batch tree together with its in-tree evidence already deleted in this batch close's gc (see
> "Nine"), so this section inlines measurement conclusions and numbers **as-is into record** for
> archive. 3.5 red state four probes all report E1003+E1002 / E1010, green state exit 0 and value
> correct (stdout 1 / 7 / 1), with IR value-level empirical (specialized function
> `Function probe(Box, int64)`); 3.6.1 green state output 6 / 10, comparison "value position call"
> green, "non-function value called by name" compile-time E1065; 3.6.2 red state two forms E6006,
> green state exit 0 (probe in `assert` fail i.e. non 0, exit 0 i.e. value correct); 3.6.3 red state
> stdout `void` / EXIT=0 (silent wrong value verifiable), green state compile-time E1001 / EXIT=1,
> while **expression position inline lambda capture this function parameter** allowed (EXIT=0,
> output 6)——allow surface not false injured. 3.4.6's C2 unification by verifier self-built fixture
> independently reproduced (four paths W set identical, non-empty), baseline diff independently
> recounted (changed=46, added=0, removed=0, expectation / run.exit / run.stdout / check change = 0,
> new W code head 118 consistent with implementer), attribution sampling 10 all hit real
> definitions. Gate in frozen tree all green: fmt / clippy (`-D warnings`) / `cargo test --all`
> (**lib 2790 passed + 2 ignored, integration 297 passed**, 0 failed) / corpus diff 348:348 diff
> empty / IR snapshot 204 zero drift / obligation ledger / docs-truth. **t11 (round 2, frozen
> `8e32e8636`)** independently reproduced 3.6.4's red→green (red state parent commit 16e2a5c88:
> E6006 `lib.f`, EXIT=1; green state EXIT=0, stdout 27, probe in 4 assertions all pass), criterion
> sole source (`resolve_namespace_data_access` 1 definition 2 consumers, no third "qualified name →
> slot" joint lookup), no regression eight forms all pass, verdict = **pass**.
>
> **4. Review rounds and conclusion**: Round 1 (t4, judgment `20abd3ebc`'s parent revision) verdict
> = **needs_revision**, three findings all real defects: F1 = `is_ctor_reference` same name two
> copies (`inference/statements.rs` method copy), F2 = head rename in `environment.rs` and
> `expressions.rs` each inline one copy, F3 = visibility crossing (the private's not private, the
> `pub(super)`'s not converged). → fix round t12 (one commit, 4 files +96/−85) → round 2 (t13,
> reviewed revision `20abd3ebc`) verdict = **pass**, new finding = 0: 8 self-built probes' values
> and exit codes line-by-line same as before fix, 4 IR dumps with round 1 green state
> **byte-for-byte consistent**, corpus 348/348 zero drift, IR snapshot 204 zero drift,
> `cargo test --all` EXIT=0 (lib 2794 passed + 2 ignored, integration 297 passed, 0 failed), fmt /
> clippy / docs-truth all exit 0, `check-boundary`'s `pub(crate)` count 167 / frontend-core 26 (=
> 3.5 pre-baseline), `check-fanout` same name definition zero hit. **Final check basis = round 2's
> pass**.
>
> **5. Mutation evidence (all via `python scripts/dev/scratch.py trial`, apply → run command → apply
> -R → per-file blob compare + require `git status` clean)**:
>
> | #   | Target                                        | Post-mutation blob | Capture result                                                                                                                                                    |
> | --- | --------------------------------------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | M1  | 3.6.1 `is_static_fn_name`'s global lookup arm | `022986cbea65…`    | Two probes reproduce E6006, EXIT=1 (after restore blob = HEAD, CLEAN)                                                                                             |
> | M2  | 3.5.2 explicit type argument branch           | `abbcc5ecf116…`    | Probe check reports E1002, EXIT=1 (comparison function form still green ⇒ attribution precise to 3.5.1)                                                           |
> | M3  | 3.4.6's `MULTI_FILE_STAGES` dead code arm     | `372f7b92755d…`    | run channel W code all gone (C2 red); M3b attribution shows check channel criterion still healthy, broken is exactly 3.4.6's newly opened compile outward channel |
> | M4  | 3.6.3 capture barrier entry                   | `7a7204c60383…`    | Capture class turn to nail test 4 failed, TEST_EXIT=101                                                                                                           |
> | M5  | 3.6.4 value slot → CallDyn block              | `25853376639b…`    | Probe reproduce E6006, EXIT=1 (blob chain `0d837e9d30de…` → `25853376639b…` → `0d837e9d30de…`)                                                                    |
> | M8  | 3.5.1 turn to nail test                       | —                  | After fix rerun still can fail (E1003, TEST_EXIT=101, mechanically reversible restore CLEAN) ⇒ criterion didn't lose failability                                  |
>
> **6. User decision and this batch registration**:
>
> 1. **3.4.6 = class D** (externally observable: multi-file `run` new W code), user 2026-10-19
>    decision "do"; **superset (W1004 / W1005 / W1006) also approved**, reason = one criterion / one
>    channel. Original 2026-10-15 decision C "per decision B1 hold over" **prerequisite** (4.9 pool
>    semantic correction) already closed on 2026-10-17.
> 2. **3.6.3 criterion change**: from "let it run correctly" to "nested function body references
>    outer frame value → compile-time E1001"; assignment path caught by E2010.
> 3. **Two "earlier than this task" silent forms registration unchanged**: ① same-block sibling
>    nested function reference——baseline already E1001 (outer block local family), this batch
>    doesn't expand scope; ② nested function annotation reference outer type parameter still prints
>    `void` (t1 registration item). Both **not** introduced by this batch, both preserved as-is.
> 4. **3.20 "dead code analysis's existing false reports" registration** (user 2026-10-19 agreed per
>    C disposition then register): 3.20.1 = W1001 doesn't track closure internal reference
>    (`docs/src/reference/stdlib/test.md`'s `t_add` form: referenced in closure but reports unused);
>    3.20.2 = W1003 treats variant destructuring required import as unused
>    (`docs/src/reference/stdlib/weak.md`'s `use std.option` form: delete i.e. E1002). Two forms on
>    3.4.6's pre-baseline also reproducible, **not 3.4.6 introduced**. **Rollback obligation
>    (written dead)**: After this group fixed must put `weak.md` / `test.md`'s rewritten to avoid
>    false report examples **restored to natural writing** and **regenerate baseline** (baseline
>    will change again).
> 5. **0.5 test suite layering** (P0, issue #486): 0.5.1 completed; 0.5.2 (pre-commit / CI wiring +
>    2600 tier measured calibration + `test-specification` detailed rules) awaiting scheduling.
> 6. **#489 ureq 3 migration**: Dependency surface tracking item, **doesn't enter WBS**;
>    `.cargo/audit.toml`'s `RUSTSEC-2026-0381` exemption **with hard removal condition**——ureq
>    migrated to >= 3 then delete that ignore, if still red at that time must not extend deadline,
>    must first fix dependency. 0.26 line has no fix version (0.26.12 doesn't exist), ureq 2.x has
>    no update version.
> 7. **3.20 number conflict disposition (this round new registration)**: 3.20 number originally
>    occupied by 2026-10-18 registered "pre-release audit (v0.8.4) supplement gap" (3.20.1–3.20.4),
>    and user 2026-10-19 decision registered "dead code analysis's existing false reports" needs
>    3.20. Whole document number uniqueness rule doesn't allow two 3.20, so **move pre-release audit
>    group to 3.21** (level-3 change 3.21.1–3.21.4, issue anchor #468–#471 unchanged, entry content
>    unchanged), 3.20 number given to "dead code analysis's existing false reports". Migration only
>    changes number, doesn't change any registration's content and ownership; historical commit
>    information not rewritten (migration fact recorded in this record and `09`'s count table note).
>
> **7. Process facts that must be truthfully recorded**:
>
> 1. **Two inScope write narrow each cost one round of rework**: ① 3.6 series's implementer wrote
>    inScope narrow to `ir_gen.rs` predicate layer, didn't cover `types/`'s shared replacement
>    implementation → review round 1's F2 when landing discovered head rename in two files each
>    inline one copy, must open one more round of convergence; ② 3.5's fix round when transporting
>    changes between `types/` shared implementation and two call sites, again hit inScope and actual
>    change surface mismatch (AGENTS.md item 6: inScope must cover mechanical necessities, path
>    prefix resolvable by matcher). Both reworks due to "implementer inScope write narrow", **not
>    implementation defect**——but cost is two rounds of rework and one review findings.
> 2. **External sync (2026-10-19 13:08 `pull --rebase --autostash origin dev`, 13:19
>    `pull --tags origin dev`) and V0.8.5 release commit**: Per user 2026-10-19 confirmed as
>    **normal release process, completed, doesn't affect this team**; member commits intact no loss.
>    Forensics (`git reflog --date=iso` / `git reflog show origin/dev --date=iso`): 13:08:43's
>    `pull --rebase --autostash` picked up release chain's "CHANGELOG preservation policy's
>    criterion and self-check", 13:19:34's `pull --tags` was fast-forward; V0.8.5's release commit
>    (`:twisted_rightwards_arrows: V0.8.5: …` and its `:bookmark:` companion commit, CHANGELOG
>    generation and preservation policy two commits) all in `v0.8.5` tag reachable range.
> 3. **Three rebase causing local hash rewrite (citing old number will miss)**: `b3245aac9` →
>    `56ae469e1` (3.6.3 main body), `415f0154b` → `ef2334de5` (3.6.2), `bdbe859e1` → `8e32e8636`
>    (3.6.4). Verifier with "`src/`, `tests/` tree diff empty" proved three are **pure rebase
>    equivalence** (content unchanged, only parent chain changed), this record uniformly uses
>    **currently reachable** hash.
> 4. **F2's unfixable part**: 3.5.1's commit information wrote "sole implementation" belongs to
>    history, **not rewritten**; correction can only carry by "shared implementation convergence's
>    commit + code comment per fact correction"——this record keeps this sentence, to avoid readers
>    mistakenly thinking no one found this problem at that time.
> 5. **Three observation items (verified, don't block, but registered to avoid later being treated
>    as unhandled violation)**: **O1** head rename convergence to shared implementation then
>    coverage surface drills down `MetaType` / `AssocType` / `Literal`, but three coverage probes
>    **all parse fail** (`Pipe` / `Ampersand` / old `Type[...]` syntax; `ast::Type::Union` has no
>    construction point in parser) ⇒ **designed stronger, currently unreachable**, not behavior
>    change; **O2** `check-fanout.py` after fix round newly reports `add_type_param: 2 files`,
>    introducer is 3.6.3's close commit (`inference/scope.rs`) **not** this batch's fix round,
>    review judgment **heuristic false report** (delegate wrapper + sole implementation division of
>    labor, same type as existing `add_param`); **O3** `09`'s 3.6 row original text still writes "1
>    level-3 task" but actually has 3.6.1–3.6.4, 3.5 three rows missing completion note——by this
>    close round supplemented together (see `09`).
> 6. **`src/driver/arms.rs`'s "Decision B1" comment not changed same submission as code (legacy
>    item, this batch not fixed)**: 3.4.6 already changed `typecheck_orchestrated`'s FailFast branch
>    to **temporarily store** `result.warnings` (originally directly discard), but that function's
>    above document comment still writes "`result.warnings` discard——Decision B1", opposite to
>    3.4.6's post-code fact; same source's another comment (`src/driver/tests/program.rs`'s phase
>    table description) **already rewritten in 3.4.6's commit** as "Decision B1 'dead code family
>    warnings don't enter compile path' hereby revoked". This batch `src/**` out of scope, so **not
>    changed** that comment, registered awaiting next team (belongs to AGENTS.md item 6 "declaration
>    carrier directly named by decision" class debt). `02`'s runtime behavior table B1 row already
>    in-place corrected to current state (dead code family warnings now via same criterion enter
>    MultiFile `run` path).
> 7. **Members didn't execute `git push` / `fetch` / `pull`**: Sync and push by captain uniformly
>    (`git reflog show origin/dev` each commit is "update by push", member side no push record).
>
> **8. Uncovered surface (verbatim copy, must not write "not tested" as "passed")**:
>
> - **t3's 7 items**: ① wasm32 target real compile not done; ② performance impact not measured
>   (3.4.6's two new phases, 3.6.x's indirect call overhead no benchmark numbers); ③ **W1005
>   per-line attribution not independently reproduced** (verifier's message form regex doesn't cover
>   its text form, only verified new W code total 118 consistent); ④ 3.6.3 close commit (declaration
>   / assignment doesn't apply barrier) not separately mutated, c1 form not independently re-tested;
>   ⑤ t8's 29 probe matrix not fully re-run (only re-tested three representative forms + M4 turn to
>   nail); ⑥ 3.6.4 belongs to round 2 scope, round 1 not independently measured; ⑦ CI gate per
>   convention not waited in session, handed to close link unified check.
> - **t11's 4 items**: ① qualified name callee with generic argument (`lib.g(Int)(3)`) and named
>   argument call function type value——warehouse no sample, this round didn't construct re-test; ②
>   `use lib` whole import without item import's `p.get_x()` (existing E6006 boundary, earlier than
>   this task) not re-tested mutation state; ③ didn't repeat full gate on `8e32e8636` (covered by
>   t9's gate log and t3's all-green on `81d5e414e` containing this commit); ④ CI handed to close
>   link check.
> - **This close round new registration**: This round's document change **didn't rerun
>   `cargo test --all`** (change surface is `docs/**` three files, zero code surface); close round's
>   gate is `npm run docs:build` + `check-docs-truth.py` + batch tree recycling + `git status`
>   check.
>
> **9. Batch tree recycling (gc and status raw output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted verify-p3-t3 (released about 13220.5 MB)` /
> `[scratch] deleted verify-p3-t4 (released about 14428.0 MB)` /
> `[scratch] gc complete, total released about 27648.5 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`; `git worktree list` → only main mount (`dev`). **Also deleted with gc
> are two batch trees' in-tree verification evidence** (`verify-p3-t3/` contains independent
> verification's probes, patches, raw logs and conclusion text; `verify-p3-t4/` contains review
> round 2's evidence)——they are **registered batch directories**, recycled with tree; this record
> sections three, four, five have inlined their conclusions, mutation table and gate numbers **as-is
> into record**, conclusions don't become invalid due to file deletion. **Unaffected archive**
> (non-registered batch directory, gc doesn't touch): each batch's evidence directory `wbs35` /
> `wbs35repair2` / `wbs346` / `wbs36` / `wbs362` / `wbs363b` / `wbs364` / `wbs486` (in workspace
> `target/`, **not checked in**; conclusions already inlined in this record, workspace path not as
> reference source). Acceptance expression (resolvable): `git status` clean;
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main mount; no
> this-batch newly added directories outside warehouse.
>
> **10. Document real build and commit hygiene**: This batch's check / note / registration /
> document correction **accumulated into one** document commit (AGENTS.md item 8), fell in batch
> close; generator regenerated document merged into its functional commit. Before commit locally run
> `npm run docs:build` (AGENTS.md item 10 ⑨——markdownlint doesn't catch VitePress's Vue compile
> error, and docs-only commit is judged `no files to check` to skip markdownlint) and
> `python scripts/ci/check-docs-truth.py`. **Commit number taken after push**: This record per
> historical record's convention records each round's actually occurred commits and hashes (**note
> division of labor with `09` check row**——`09`'s check / completion registration only writes "task
> id + evidence (gate values, issue number, file:line)" not write commit number, because
> `pull --rebase` will rewrite local hash; this record is the round's process historical record,
> preserves hash and gives rebase before and after comparison, see "Seven" 3). **CI / Deploy Docs
> conclusion handed to "unified check" link**, not waiting in session (AGENTS.md item 13)——this
> batch's CI run id and conclusion by captain backfill at unified check.
>
> **11. Count table accounting review (this round caliber)**: `09` top's level-2 / level-3 count
> table continues "**base + increment**" caliber (same as 2026-10-18's registration). This round
> increment = level-2 +2 (0.5 / 3.20, of which 3.20 is post-move new occupied number, pre-release
> audit group change 3.21 is net zero), level-3 +7 (3.6.2 / 3.6.3 / 3.6.4 / 0.5.1 / 0.5.2 / 3.20.1 /
> 3.20.2). **Remaining pre-existing deviation (this round not corrected, for subsequent accounting
> review)**: with "per level-2 entry parenthetical 'N level-3 tasks' sum" as caliber recount, this
> tree gets 162, diff with table head 7——diff comes from 2026-10-18 registration time newly added
> level-3 but not synchronized rewriting parent entry count note's 5 places (4.9 write 7 actually 8,
> 3.14 write 1 actually 2, 4.10 write 3 actually 5, 4.4 write 3 actually 4, S2 write 1 actually 2)
> and 1 place reverse deviation (7b write 6 actually 5), another 1.2 / 1.3 / 9.1 / S7 four parent
> entries originally don't have that note (not counted in caliber). That diff **not introduced by
> this batch**, this batch doesn't change number for others' entries, only registered here.
>
> **12. Legacy awaiting (this batch registered, not fixed)**: ① `src/driver/arms.rs`'s "Decision B1"
> comment opposite to 3.4.6 post-code fact (see "Seven" 6, `src/**` out of scope); ② 3.20.1 / 3.20.2
> two existing false reports (after fixed must restore `weak.md` / `test.md`'s natural writing and
> regenerate baseline); ③ 0.5.2 (pre-commit / CI wiring + resource tier measured calibration +
> `test-specification` detailed rules); ④ #489 ureq 3 migration (dependency surface, exemption with
> hard removal condition); ⑤ 3.8 (native to user function name shadowing) and 3.12 (runtime mode
> executor config propagation) awaiting decision, 3.18 prerequisite awaiting P6——**all not in this
> team's scope**.

## P3 3.17 Close (Round 2: 3.17.1 Hand-annotated FFI Named Arguments + Second Binding Criterion Merge)

> **1. Scope and round relationship**: This section is **round 2**'s close registration, scope =
> **3.17.1** (issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)), doesn't include other
> level-2 tasks. Round 1 (3.5 / 3.6 / 3.4.6) close see previous section "P3 Debt Round 1 Close"; two
> rounds' relationship written here: **Round 1's review round 1 (t4) that needs_revision (F1 / F2 /
> F3) already covered by t12 fix round + t13 review round 2 (verdict = pass), not repeated in this
> section**. This section's own criterion chain = implementation **t15** (attempt 2 delivery;
> **previous attempt due to provider timeout, zero product**) → independent verification **t16**
> (verdict = pass) → adversarial review **t17** (verdict = pass, **no unhandled findings**, 4
> observation items registered).
>
> **2. Per-item output and measurement numbers (3.17.1)**:
>
> - **Parameter name table same source**: `add_native_signature` changed to receive **signature
>   string** and return `Result`——`signature::parse_signature` produces type, sole exit
>   `signature::export_signature_param_info` same pour "parameter name + required minimum"; key and
>   signature table same (same name). Warehouse zero caller so can change signature; document
>   clearly writes "callers that can only get `MonoType` should parse signature string on their own
>   side then go this API", and this function **must not** parse parameter name itself.
> - **Red before green** (red state measured at parent commit `537b46b74`): hand-annotated surface
>   four forms + duplicate naming + legal call 6 items, **5 failed, diagnostics all `[]`** (named
>   arguments silently discarded); green state same file **12 all pass**——four forms E1014 / E1010 /
>   E1111 / E1111, duplicate naming E1015, legal named call and omitted optional position allowed.
> - **Three call surfaces same criterion same code**: Hand-annotated FFI (typecheck phase) /
>   ordinary user function (IR phase) / stdlib native export (`assert.assert`) three surfaces **per
>   code equality assertion** (E1014 / E1010 / E1111; E1015 see hand-annotated surface and user
>   surface). Different observation phase only reflects "who discovers first"; host native doesn't
>   enter registry, IR phase can't resolve (**earlier than this task's embedding limitation**).
> - **Criterion sole source**: `bind_args_to_params` **1 definition**
>   (`inference/arg_binding.rs:35`) + **6 calls** (typecheck 2 + ir_gen 4); second criterion
>   `build_arg_slots` **0 places** (deleted, `ir_gen.rs:516` only has comment explaining its
>   history); `ir_gen` only has pure transport `slot_order_arg_indices` (`ir_gen.rs:521`, 1
>   definition 2 calls, zero judgment). Lines: `ir_gen` **+58 / −92 (net −34)**; full warehouse 4
>   files **+448 / −102** (of which 353 lines are new tests).
> - **No regression**: 8 CLI probes (u1–u4 / d1 / d2 / n1 / n3) before and after mutation **per code
>   consistent**; `ir_gen` blob `0d837e9d…` ↔ `ffe1af8e…` check. Initial merge version's self-injury
>   fixed: IR side re-judging "missing parameter" mistakenly reported legal overload
>   `t.pick2(a = 2)` as E1010 (named_args single test caught) ⇒ missing parameter judgment to
>   typecheck, IR only replays slot binding; after fix 19 named_args + 12 new single tests all
>   green. Corpus 348-348 zero drift, IR snapshot zero drift.
>
> **3. Independent verification (t16, frozen `c3082a160`; red state baseline `537b46b74`, both
> `git merge-base --is-ancestor` verified as HEAD ancestors; probes all self-built)**:
>
> - **Red→green independent reproduction** (hand-annotated FFI surface): Self-built independent Rust
>   single test `verify_host_ffi.rs` (4: unknown name E1014 / conflict slot E1111 / named then
>   positional E1111 / legal named call allowed), via `scratch.py trial` temporarily hung into
>   `tests/mod.rs` (apply → test → apply -R → CLEAN). Two states each use **their own interface
>   form**'s probe variant (parent commit `add_native_signature(name, MonoType)` old signature / fix
>   state `(name, &str) -> Result` new signature——the interface itself is the fix surface): parent
>   commit **3 failed / 1 passed**, 3 failures diagnostics **all `[]`** (`got []`), TEST_EXIT=101;
>   fix state **4 passed / 0 failed**, TEST_EXIT=0. Red state another reproduce in fix tree's
>   revocation fix mutation state (`M7v2` merge patch: revoke fix + revoke implementer test + hang
>   self-built old API probe)——3 failed `got []`, TEST_EXIT=101, after restore four file blob
>   identical to HEAD, `git status` CLEAN, after revoke `ir_gen.rs` blob = `0d837e9d30de` (i.e.
>   parent commit original blob). CLI surface (user function + stdlib) two states per code
>   consistent——as expected: these two surfaces 3.16.1 already loud, this commit's fix surface only
>   in hand-annotated FFI.
> - **Four forms per item** (CLI self-built probe, `c3082a160` measured): parameter name error E1014
>   `has no parameter named x (available: a, b)`, missing required position E1010, positional and
>   named conflict slot E1111, named then positional E1111; additional: duplicate naming E1015,
>   stdlib unknown name E1014, stdlib missing required position E1010 (help points to cond), stdlib
>   conflict slot E1111, legal named call EXIT=0 output 3/3, omitted optional position EXIT=0.
> - **Criterion truly deleted not renamed**: `git show c3082a160:src/middle/core/ir_gen.rs` in
>   `build_arg_slots` **0 hit** (parent commit same file 3 hit, definition at :521);
>   `slot_order_arg_indices` (fix state :521) line-by-line check =
>   `sort_by_key(|&arg_idx| positions[arg_idx])` pure sort, no position/name/optional position
>   judgment residual, document comment explicit "doesn't judge position, doesn't judge name,
>   doesn't judge optional position"; deleted 71 line judgment block (parent commit :521-593, six
>   rules: empty table loud / positional occupy slot / order illegal E1111 / unknown name E1014 /
>   conflict slot E1111 / enter slot) already doesn't exist; `bind_args_to_params` 1 definition 6
>   calls (typecheck 2: `expressions.rs:2854` / `:5653`; ir_gen 4: :7828 / :7926 / :8021 / :8351),
>   consistent with implementer's report; parameter name sole exit = `export_signature_param_info`
>   (environment.rs hand-annotated / typecheck::mod stdlib / ir_gen table lookup), no second place
>   self-parse.
> - **Self-injury regression nailed** (`t.pick2(a = 2)`): `git show c3082a160 --name-only` **doesn't
>   contain `named_args.rs`** (4 files: environment.rs / tests/mod.rs / native_param_names.rs /
>   ir_gen.rs) ⇒ nail test not modified; independently re-run that use case 1 passed, named_args
>   full set **19 passed / 0 failed**.
> - **No regression gate** (`c3082a160` tree): `cargo test --all --quiet` (4×4 caliber) EXIT=0——lib
>   **2806** + 2 ignored, integration **297**, 52/44/9/2/1/9 all green 0 failed; `cargo fmt --check`
>   / `clippy -D warnings` both EXIT=0; corpus diff (dual parameters) 348/348 diff empty;
>   `check-snapshot-drift.sh` PASS (204 snapshot zero drift); `check-docs-truth.py` EXIT=0; 3.16.1
>   four form comparison (stdlib surface) E1014 / E1010 / E1111 / allow.
> - **Uncovered surface (t16 five items, verbatim copy)**: ① `testsuite.py run full` (CI caliber
>   resource profile channel) not rerun——implementer already fell back per decision (exclusive wave
>   peak 8.11 GiB > budget 7.51 GiB, exit 2 is tool rejecting per design); this round uses 4×4 and
>   2×1 two calibers' `cargo test --all` coverage. ② Hand-annotated FFI surface (host-FFI) can only
>   be driven via Rust test (`add_native_signature` warehouse zero caller, no CLI surface)——"host
>   embedding process in real FFI call" not tested. ③ Variable argument (`...`) named argument form
>   not constructed sample (no warehouse criterion sample). ④ `check-boundary.py` /
>   `check-fanout.py` not rerun (implementer already passed; no intersection with behavior
>   criteria). ⑤ CI (GitHub Actions) per convention not waited, handed to close link check.
>
> **4. Adversarial review (t17, reviewed revision `c3082a16051cb1bfb8b84bdc62ec6bd29d90ff2d2`,
> parent = `537b46b7488a60b42cdc35110e0a413c80a58169`; all content use `git show <rev>:<file>` take,
> line numbers retaken on cited revision)**:
>
> - **① Criterion sole source reverse check**: `slot_order_arg_indices` (ir_gen.rs:521-527)
>   **doesn't reorder by name** (sort key is slot number given by shared criterion, name only
>   appears in `arg_binding::bind_args_to_params`), **missing parameter doesn't backstop** (absent
>   slot doesn't produce argument index at all), **doesn't distinguish optional position** (IR two
>   places pass full optional, another two places pass real value `i >= required_min`). Full
>   warehouse "find position by parameter name / find member" grep per item verified: **slot
>   calculation only `arg_binding.rs:76` one place**, other 7 hits each belong to other concerns
>   (generic type parameter's constraint interface check / const generic argument replacement /
>   generic type variable binding / generic type parameter replacement / const generic parameter
>   lookup / method definition lookup / struct field lookup).
> - **② Responsibility boundary (missing parameter only loud in typecheck, IR only replays slot
>   binding)**: Typecheck side `bind_named_args` (`expressions.rs:2854`) passes full required ⇒
>   missing any value parameter E1010; native export arm (`:5653`) passes real optional position
>   `i >= required_min` ⇒ only real required position E1010, `?msg` omit allowed. IR side this
>   commit's two changes (`ir_gen.rs:7926` RFC-004 binding path / `:8021` overload resolution method
>   path) **uniformly pass full optional, only replay slot binding** (comment explicit "missing
>   parameter to typecheck" and "per declaration table re-judge missing parameter will mistakenly
>   report legal resolution as E1010 (`t.pick2(a = 2)` measured)").
> - **③ API shape**: environment.rs:990-993 clearly writes "this function **must not** parse
>   parameter name itself" and points to sole exit `export_signature_param_info`; :997-999 writes
>   "callers that can only get `MonoType` should parse signature string on their own side then go
>   this API".
> - **④ Behavior change surface**: Code four places diff (same parameter named twice E1111→E1015
>   code change / unknown name E1014 text change / empty table E1014 text change / positional
>   argument over IR side declaration table: silently discard→send as is), **empirically reachable
>   form has no diagnostic code / text change** (typecheck first judges, same code), IR product on 5
>   self-built probes **byte-for-byte consistent** (p1 / p5 goes this commit's changed branch and
>   product unchanged, p2 / p3 / p4 two states same code, p9 can't construct, p6–p8 source layer
>   unreachable); t15's 8 CLI probes two states per code consistent. Diff all falls on **originally
>   failing** programs (program still fails) ⇒ D59 "doesn't change originally successful program's
>   external performance" not touched.
> - **⑤ Test quality** (not just red/green): 12 new tests (`typecheck/tests/native_param_names.rs`)
>   per item to `test-specification`——naming `test_<what>_<scenario>`, three-section AAA comment,
>   assertion all with custom message, setup repeat ≥3 times abstracted helper, helper in panic with
>   context; **no "only check exit code" weak assertion** (first `assert!(`diagnostic non-empty`)`
>   then `assert_eq!(`per code equal`)`), two sides both empty won't pass through hole); 3 negative
>   cases (legal named call allow / optional position omit allow / explicit optional position allow)
>   in M1 red state still pass ⇒ they **only defend false report**, exactly their use. **M1**
>   (remove `native_param_names` filling) → `cargo test --lib native_param_names` **9 failed / 3
>   passed**, TEST_EXIT=101 ⇒ new test **really nails fix**. **M2** (whole segment revert `ir_gen`
>   merge) → `cargo test --lib` **2806 passed / 0 failed / 2 ignored**, exit 0 ⇒ merge surface **no
>   test coverage**. `pick2` nail (existing `named_args.rs:253-283`, this commit not changed) goes
>   **real full chain** and asserts **runtime value** `Int(20)` ⇒ **really nails overload
>   resolution** (not "no error"), and it for "IR must not re-judge missing parameter" is alive
>   gatekeeper.
> - **⑥ Limitation registration**: Host native only enters `TypeEnvironment`'s two tables, **doesn't
>   enter `ModuleRegistry`** ⇒ IR generation per registry resolves namespace members, so host native
>   in IR phase can't resolve (E3006), **earlier than this task's embedding limitation**; that
>   limitation and "why hand-annotated surface must observe in type check phase" written in **test
>   file document comment** and `environment.rs` field document (not only in report).
> - **Gate eight items all re-run on reviewed revision all green**: fmt --check / clippy -D warnings
>   / `cargo test --all` (**3220 passed, 0 failed**; lib 2806 + 2 ignored, integration 297) / corpus
>   diff 348-348 zero drift / IR snapshot 204 zero drift / docs-truth / check-boundary (0 exemption
>   0 violation) / check-fanout (new symbol no same name multi-definition entry).
> - **Four observation items (verbatim registered: verified, judgment doesn't constitute finding, no
>   need to dispose, but registered to avoid later being treated as unhandled violation)**:
>   - **O1**: Commit information says "after this merge **whole warehouse only has one shared
>     criterion**"——in **slot binding** sense holds; but typecheck's **method overload arm** still
>     has one "find candidate by name (E1014) + arity (E1010)" own judgment
>     (`inference/expressions.rs:5578-5600`), it uses **candidate group representative**'s parameter
>     table (`method_overload_param_names`), different from shared criterion's **resolved
>     candidate** table. Judgment basis: that place **earlier than this commit exists, this commit
>     didn't touch** (`git show c3082a160 --name-status` only 4 files, no expressions.rs), and
>     belongs to "candidate match" concern (different table, different semantics), not slot binding.
>     **Wording too wide, registered as observation**.
>   - **O2**: `add_native_signature` changed to receive signature string, **no**
>     `MonoType → signature string`'s shared helper ⇒ future callers that can only get MonoType can
>     only each write one serialization. Judgment basis: parameter name / required minimum already
>     document-locked at sole exit `signature::export_signature_param_info`; current **zero
>     caller**; document already wrote "callers that can only get `MonoType` should parse signature
>     string on their own side then go this API" ⇒ residual risk only in "how to construct string"
>     one paragraph, **low**.
>   - **O3**: This commit's **merge part** (ir_gen deduplication) **no any test coverage** (M2:
>     whole segment revert ir_gen after `cargo test --lib` still 2806 passed all green). Judgment
>     basis: "deduplication" no new behavior to nail, nature so; "sole source" property guaranteed
>     by grep + `check-fanout.py` gate. **Optional strengthening**: In `native_param_names.rs` add
>     one **IR layer** case (cover ir_gen:7926 / :8021 path), let merge surface also have guardrail.
>   - **O4 (attribution correction)**: Commit information and report attribute d2's E1010 to
>     "**method arm** arity pre-check first triggers"——empirically that E1010's help is generic text
>     "check the argument count of the function call", **not** shared criterion's "missing
>     parameter"'s help ("missing required parameter:…") ⇒ trigger is **native arm**'s arity
>     pre-check (`native_arity` table). **Code behavior conclusion unchanged** (parent commit
>     existing, this time not changed, already reviewed), only **attribution wording needs
>     correction**; and "`list.push` duplicate naming i.e. E1010" must limit to "**arity mismatch +
>     duplicate naming**"——only duplicate naming and arity match (`list.push(list = v, list = 9)`)
>     two states are **E1015** (duplicate naming earlier than arity judge).
>
> **5. Gate and resource channel (one external link)**: This task **first time really used**
> `python scripts/dev/testsuite.py run full` → **exit 2**——exclusive wave peak 8.11 GiB > local
> budget 7.51 GiB (gap 618 MiB), tool per design **refuse silent skip** ⇒ fall back
> `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test --all --quiet` → **exit 0** (3220 passed / 0
> failed). That channel status chain **#486**: `full` on local machine due to peak extrapolation
> conservative not usable, **calibration (2600 tier from measured calibration) belongs to WBS
> 0.5.2**.
>
> **6. Count table: this round not moved**——`3.17.1` already registered on 2026-10-15 and counted
> into table head ("2026-10-15: +2 = 3.17.1 / 3.18.1"), this round only check + note, **level-3
> count unchanged** ⇒ `09` top's level-2 67 / level-3 176 **not moved**. This round **no new** count
> deviation (parent entry 3.17's "1 level-3 task" note originally correct); previous section
> "eleven"'s recorded 6 pre-existing deviation (parent entry count note lagging) **unrelated to this
> round, not introduced by this round**, still per that section caliber left for accounting review.
>
> **7. Batch tree recycling (gc and status raw output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted verify-p3-t16 (released about 10235.4 MB)` /
> `[scratch] deleted verify-p3-t17 (released about 10079.2 MB)` /
> `[scratch] gc complete, total released about 20314.6 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`; `git worktree list` → only main mount (`dev`). **Also deleted with gc
> are two batch trees' in-tree verification evidence** (`verify-p3-t16/`'s `EVIDENCE.md`,
> `verify-p3-t17/`'s `EVIDENCE-t17.md` and probes / patches / raw logs)——they are **registered batch
> directories**, recycled with tree; this section three, four have inlined their measurement
> numbers, gate values, mutation table and four observation items **as-is into record** for archive,
> conclusions don't become invalid due to file deletion. **Unaffected archive** (non-registered
> batch directory, gc doesn't touch): `wbs317` (implementer evidence: evidence-3.17.1.md /
> red-test.txt / trial-before-after.txt + fix-3171.patch / attribution table etc.; in workspace
> `target/`, **not checked in**; conclusions already in this section, workspace path not as
> reference source). Acceptance expression (resolvable): `git status` clean;
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main mount; no
> this-batch newly added directories outside warehouse.
>
> **8. Document real build and commit hygiene**: This round's check / note / registration
> **accumulated into one** document commit (AGENTS.md item 8); before commit locally run
> `npm run docs:build` (AGENTS.md item 10 ⑨) and `python scripts/ci/check-docs-truth.py`,
> markdownlint zero warning. **Don't push** (members don't push, see AGENTS.md item 10 ⑧); **CI /
> Deploy Docs conclusion handed to unified check link** for backfill, not waiting in session.
>
> **9. Legacy items and registration**:
>
> 1. **Round 1 close registration's legacy item ① closed**: `src/driver/arms.rs`'s "Decision B1"
>    comment opposite to 3.4.6 post-code fact item, by `537b46b74`
>    (`:memo: docs(driver): correct B1 comment opposite to fact (3.4.6 close omission)`) in-place
>    rewritten to current state fact (behavior neutral, only comment row; wording aligned with
>    `src/driver/tests/program.rs` same fact's expression).
> 2. **This round new registration**: O1–O4 four observation items (verified, don't block,
>    registered in case); 0.5.2 (`full` channel's 2600 tier measured calibration + pre-commit / CI
>    wiring); #489 ureq 3 migration and `.cargo/audit.toml` exemption's hard removal condition; 3.8
>    / 3.12 awaiting decision, 3.18 prerequisite awaiting P6——**all not in this team's scope**.
>
> **10. CI gate conclusion (AGENTS item 13 ①, timed check 2026-10-21 backfill)**: This batch's last
> push tip = `1f5eb8665`
> (`:memo: docs(meta): verify list supplement three CI-exclusive gates, and write "emit point" caliber`)
> → CI **39604109534 success**, Deploy Docs **39603305398 success**; that tip covers `e7a3fd4b1`
> (regenerate error code generation product) and all commits before it. `e7a3fd4b1` itself that
> round CI **39603305171 cancelled**——per item 10 ③ "consecutive push will cancel previous round",
> its conclusion undertaken by successor `1f5eb8665`'s success. Failure chain for trace: `c3082a160`
> (**39587784184 failure**) and `190a0e7a3` (**39599653111 failure**) both same job "Error-code docs
> up to date (generator idempotent)", root cause = 3.17.1 deleted judgment + line number shift after
> didn't rerun generator; by `e7a3fd4b1` regenerate, `1f5eb8665` close after turn green, cumulative
> red 5 rounds, triggered AGENTS item 10 ⑤'s wording patch (`1f5eb8665` is that patch).
>
> Check command and raw output (2026-10-21 13:19Z rerun): `gh run list --workflow ci.yml --limit 4`
> → `39604109534 success` (`1f5eb8665`) / `39603305171 cancelled` (`e7a3fd4b1`) /
> `39599653111 failure` (`190a0e7a3`) / `39587784184 failure` (`c3082a160`);
> `gh run list --workflow docs-deploy.yml --limit 3` → `39603305398` / `39599653238` / `39587784210`
> all success.

## Document Reference Governance and "Unresolvable References" Gate (2026-10-21)

> **Trigger**: When cleaning workspace evidence directory discovered, in `09` / `09a` / `CHANGELOG`
> / branch maintenance guide there are 24 places taking "workspace-internal, `.gitignore`-excluded
> directories" (batch tree / probes / evidence directory) as **evidence source**——this kind of
> directory doesn't exist after clone, external contributors can't verify any number (AGENTS.md meta
> rule ① and item 10 ④). One of them is 4.4 close round's fix: change old path to point to another
> local path, equal to moving violation down one layer not eliminating.
>
> **1. Reference inline in place (47 places = zh 24 + en 23)**: Conclusions, numbers, CI run id,
> delivery commit written in place into this document and `09`, source changed to checked-in path
> (record section anchor) or revision / run id / issue number. CI run id detail and S9 seven groups'
> list **already inlined**, deleted only redundant path pointer. en mirror regenerated by
> auto-translate bot per zh source.
>
> **2. New mechanical gate** `scripts/ci/check-docs-refs.py` (hooked into `ci.yml`'s docs job):
> tracks path-like tokens in text file backticks and markdown links, any hit by `.gitignore` with
> specific subpath is red; placeholder form (rule statement) and bare directory name not judged;
> system runtime directory (package manager cache, build product, playground publish directory,
> local agent workbench) goes **item-by-item written reason** exemption table (26 items; details
> written in gate's own run report——uploaded as artifact in CI, not checked in). **Criterion can red
> can green**: self-test 9 cases (4 negative must red / 5 positive must green, includes cross-batch
> boundary one); end-to-end mutation verification——change one violation reference back → gate red →
> restore → green → that file and pre-mutation byte-for-byte consistent.
>
> **3. Rule wording patch**: `AGENTS.md` item 10 ④ clear "'warehouse-internal path' means
> git-tracked checked-in file", item 9 supplement "things in workspace are not evidence source",
> item 10 ⑤'s local verify list supplement this gate (user 2026-10-21 approved).
>
> **4. Two-platform consistency (first version gate's defect and fix)**: First version gate
> **Windows local green, CI (Linux) red**——`git check-ignore` for "directory form path ending in
> `/`" two platforms behavior different (Linux reports real rule, Windows reports one **empty
> pattern** = `.gitignore`'s empty line), only recognize non-empty pattern when Windows side missed
> `docs/public/wasm/` (playground publish directory, RFC-037 references). Fix: each token check
> three forms (with slash / without slash / without slash + probe subitem), any form hit
> **non-empty** pattern counts as hit. After fix Windows and Linux dual run **same number**: 1122
> tracked text files / 5533 path-like tokens / 0 violations / 26 exemptions / selftest 9/9. Lesson:
> gate script **at least on two OS and checkout form each run once** before daring to hook CI (CI's
> job log needs admin permission, can't get when use "Linux real clone reproduce" location).
>
> **5. CI evidence (AGENTS.md item 13 ①, timed check backfill)**: Fix commit `c7a0dd417` → CI run
> **39626722284 success**; previous commit `fff9580f8` → CI run **39623038087 failure** (red in new
> added "Check doc references are resolvable", i.e. above platform difference, fixed by
> `c7a0dd417`), same commit Deploy Docs **39623038228 success**, Auto Translate Docs **39623038027
> success**; earlier each commit (`ec2893e0d` / `ff2104b0c` / `ffd43b1d0`)'s CI run due to
> consecutive push concurrency **cancelled** (39621547581 / 39620372322 / 39619834870), per "use
> later run's conclusion cover previous one" take conclusion.
>
> **5.2. This section's check-in commit's CI evidence (2026-10-22 timed check backfill)**: This
> section check-in commit `612c65322` → CI run **39798235412 success** (7 jobs all green, includes
> new added "Check doc references are resolvable" and "Error-code docs up to date" two steps),
> Deploy Docs **39798235484 success**; same commit Auto Translate Docs **39798235413 failure**,
> failure in last step "Commit translations"——translation itself success (Translate documents /
> Format / Lint all success), but push time remote already pushed by this batch's code commit and
> bot's own previous auto-translate commit, `git pull --rebase` + `git push` blocked, **result is
> this document's en mirror doesn't yet contain this section** (zh is sole source; next push
> touching `docs/src/**/*.md` will again trigger that workflow supplementary translation). Parallel
> 4 bugfix (`eb4124cf0` / `14e9b2704` / `96638ec9f` / `06ac6d687`)'s CI run **39797917813
> cancelled** (consecutive push cancelled by concurrency), per "use later run's conclusion cover
> previous one" take conclusion.
>
> **5.3. Gap closure and this batch close (2026-10-23 timed check backfill)**: After supplementary
> push bot's Auto Translate Docs run **39904034074 success** (bot commit `449360a5a`)——**this
> document's en mirror already contains this section**, previous round's gap closed; same commit CI
> run **39904034164 success**, Deploy Docs **39904034075 success**. Previous round 39798235413's
> failure is bot's own race: its previous translation commit and this batch's code commit
> simultaneously push remote, `git pull --rebase` + `git push` rejected. **Operation lesson**: After
> pushing don't immediately push again——leave landing window for bot (this time interval about 15
> minutes then success).
>
> **5.4. One commit bringing in parallel session changes correction and count table review
> (2026-10-23)**: This section's close commit `8a83485ef`'s **commit information only described this
> document's new section, but that commit actually also brought in parallel session's two document
> changes**——① `09`'s level-3 count table head 177 → 182 (added 3.6.5 / 3.6.6 / 4.10.6 / 4.10.7 /
> 4.10.8, and changed 4.10.4 / 4.10.5's description); ② `09a` two places `git worktree list`
> output's local path (`E:/git/YaoXiang <sha> [dev]`) cleaned to "only main mount (`dev`) left".
> Cause: when captain committed did `git add` on `09a` (sucked other's unstaged changes into stage
> area), and `09-execution-wbs.md`'s that commit's change was already in stage area.
>
> **Count table review conclusion: table head 182 not changed**. Per 09a §fifteen ①'s caliber (per
> level-2 kanban row parenthetical "N level-3 tasks" sum), before change at `449360a5a` **sum = 170
> / table head = 177 (diff +7)**, after change **sum = 177 / table head = 182 (diff +5)**——that
> commit converges two calibers' gap **from 7 to 5**, is correction toward self-consistent
> direction, not new introduced self-inconsistent; residual 5 belongs to that row's two calibers
> historically not fully aligned existing drift (§fifteen already recorded "level-2 count row not
> moved…not for fitting number"). **So this commit doesn't do any number change to count table**.
>
> **6. Registration caliber**: This section is **document governance** record, doesn't add WBS
> kanban row, `09` top count table unchanged.

## P4 4.10 Multi-file Semantic Gap Close (Round 1: 4.10.4 / 4.10.5 + Add Frame 3.6.5)

> **1. Scope and round relationship**: This section = **4.10** level-2 task's first round close,
> scope = **4.10.4** (issue [#467](https://github.com/ChenXu233/YaoXiang/issues/467), rootless
> multi-file initialization per `use` dependency topological order), **4.10.5** (issue
> [#476](https://github.com/ChenXu233/YaoXiang/issues/476), rootless defines `main` when not called
> exit code 1), **add frame and merge into 3.6.5** (issue
> [#479](https://github.com/ChenXu233/YaoXiang/issues/479), selective import's function value at
> top-level after rebinding silently `void`; **add frame from 4.10 team, user 2026-10-21
> approved**). This batch also contains one **cross-level-2 add frame** std fix (`src/std/json.yx`'s
> `\uXXXX` subscript + `src/std/tests/json_ops.yx` long-term zero coverage), it doesn't belong to
> 4.10's task surface, only registered in "Nine" and as "test false pass" example.
>
> **2. Four fixes' landing content and evidence** (commit number take **post-push** `origin/dev`'s
> hash; `09`'s check row per convention doesn't write commit number):
>
> - **4.10.4 (#467)**: Root cause = each unit's `init` section per **source file path dictionary
>   order** splice (discovery period's path dictionary order sort, **not** "discovery order"——`09`'s
>   original row wrote "discovery order" already corrected in this round; empirically rename
>   dependency file can flip result, so "dependency first" never was invariant). Fix = in Linking
>   arm per `use` dependency compute **Tarjan SCC topological order** (in-ring per path dictionary
>   order as deterministic convention, between SCCs per minimum unit index priority out of queue)
>   then reorder unit order, `link_module_irs` split into "file table order" and "initialization
>   execution order" two concepts, **zero IR shape change**. Red before green = change execution
>   order back to identity (fall back to discovery order) after 4 order use cases all red, CLI
>   reproduce "entry read dependency top-level value = `void`, exit 0" and "runtime E6007, exit 1"
>   two forms; after restore byte-for-byte back, 7/7 green. Corpus = added 2 multi-file folder
>   fixtures, baseline **+2 lines** and original 348 lines byte-for-byte unchanged; **IR snapshot
>   zero drift**.
> - **4.10.5 (#476)**: Run entry changed to return **outcome enum** (completed / defined but not
>   called), CLI only based on outcome determines exit code **1** (2 reserved for CLI /
>   configuration usage error); two lines hint text **verbatim unchanged**, doesn't add second
>   diagnostic; value type `main` already evaluated in initialization period not affected. Nail
>   tests 2 fell `tests/integration/cli_e2e.rs`.
> - **3.6.5 (#479)**: Value path's alias fallback arm per "**module source + export kind**"
>   branch——user module's exported function → materialize as closure; std constant / native function
>   → maintain original zero-argument call form; **don't add any diagnostic code, `src/std` zero
>   change**. Nail tests 3 (selective rebind / selective direct call / whole import rebind).
>   Generation product: `docs/src/reference/error-code/E1xxx.md` and `E8xxx.md` each 1 line number
>   shift (1789→1821 / 1976→2008), **generator regenerate, not manually changed**.
> - **std side (cross-level-2 add frame, user approved)**: `src/std/json.yx`'s `parse_string` in
>   `e == "u"` branch passes **backslash position** to `parse_uchar`, and `hex4`'s contract requires
>   from `'u'` position read 4 hex digits ⇒ all `\uXXXX` report "invalid \u escape"; change to pass
>   "`'u'` position" after BMP case adv=6, surrogate pair adv=12 arithmetic unchanged, error
>   position still takes backslash (user-visible position unchanged). Same commit fixed
>   `src/std/tests/json_ops.yx` this **never run test**, and incidentally fixed fixture's own three
>   places error (low surrogate segment extra splice one hex digit, trailing `\u0039` never spliced
>   into source string, error location assertion wrote "contains 2" but actual message is "invalid
>   literal at line 1, column 7"). **IR snapshot same submission regenerate**
>   (`src/middle/core/tests/snapshots/src/std/tests/json_ops.snap`), corpus baseline 1 line changed.
>
> **3. Review judgment (batch review round 1, different model family) and three uncovered branch
> registration**: Review gave two already-committed implementations **#467 = pass, #479 = pass**
> (two implementations themselves after independent check both hold), also pointed out **two
> high-level uncovered branches originally only existed in code comment, didn't enter WBS**——this
> round already per review opinion registered as **4.10.6** (import ring no warning, still report
> misleading E6007) and **4.10.7** (native function as value silently no-op), also registered
> **4.10.8** (pure yx's std export as value; review marked medium, **unverified**) and **3.6.6**
> (same class criterion convergence, refactor item). Review also pointed out **delay reason
> classification error**: delay reason wrote "need new diagnostic code ⇒ class D awaiting decision",
> but D59's split puts "silent mismatch / silent no-op" in **class C** (old behavior not successful,
> can fix along refactor) ⇒ must not use unregistered class D label to silently delay. Accordingly,
> three on 2026-10-23 got user decision (issue
> [#491](https://github.com/ChenXu233/YaoXiang/issues/491) /
> [#492](https://github.com/ChenXu233/YaoXiang/issues/492) /
> [#493](https://github.com/ChenXu233/YaoXiang/issues/493) /
> [#494](https://github.com/ChenXu233/YaoXiang/issues/494)): **4.10.6 = compile-time W code warning,
> ring must not be silent** (warning surface "ring exists" or "ring in read uninitialized value",
> whether allow `#[allow]` suppression, whether align with existing W10xx family's "evidence
> level"——three **to be deliberated**, before landing must finalize); **4.10.7 = really support**
> (materialize native function as value, **don't add diagnostic code**, terminal state =
> `use std.io.{println}` + `f = println` + `f("hello")` really prints hello); **4.10.8 = class C,
> don't add code, can directly fix**; **3.6.6 = refactor item** (behavior unchanged).
>
> **4. Overturned hypothesis (keep record to avoid repeated work)**: "Same file two `use` spellings
> ⇒ dependency edge lost" **doesn't hold**——that form reports existing diagnostic **E5001** (`use`
> target not in unit set), not "silently run per discovery order". Review listed it as overturned
> hypothesis, so 4.10.4's "no dependency information three cases" each have deterministic
> disposition, no fourth silent channel.
>
> **5. Task list and terminal state**:
>
> | Task | Topic                                                                              | Terminal state                                                                   |
> | ---- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
> | t1   | Reproduce and locate: two P1's red state forensics + #467 fix feasibility decision | completed                                                                        |
> | t2   | #467 initialization topological order (implementation + nail test + corpus)        | completed (3rd attempt close)                                                    |
> | t3   | #467 independent verification                                                      | completed, verdict = pass                                                        |
> | t4   | #479 selective import + top-level rebind (implementation + nail test)              | completed (2nd attempt close)                                                    |
> | t5   | #479 independent verification                                                      | completed, verdict = pass                                                        |
> | t6   | #476 exit code 1                                                                   | **failed** (provider side flow interruption accumulated, 6 attempts)             |
> | t7   | #476 independent verification                                                      | **cancelled** (cross-block + add frame timing, captain took over then cancelled) |
> | t8   | Batch review (#467 / #479 two implementations)                                     | completed, verdict = pass ×2                                                     |
> | t9   | Batch close (check + 3.6.5 add frame + record + gate + recycle batch tree)         | completed                                                                        |
>
> **6. Per-role attempt count and provider side failure disposition**:
>
> | Role             | Undertake                                                    | Attempt count                                                                                                                                                                                                                          |
> | ---------------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | Probe            | t1 (read only forensics), t6 (#476 implementation take over) | t1 one success; t6 **6 attempts not succeed**——during period multiple times encountered provider peak flow interruption (`PI_AI_ERROR`, 2064 "server briefly busy, usually recover in 1-5 minutes"), task terminal state judged failed |
> | Implementation A | t2 first dispatch, t4 take over, t6 take over review         | t4 one success (attempt 2); t6's implementation finally by its review and completed, with t4's "clean tree full" plan landed together                                                                                                  |
> | Implementation B | t4 first dispatch                                            | attempt 1 due to provider side **400 `invalid_request_error`** (in thinking mode `reasoning_content` must be returned) judged failed → re-dispatch                                                                                     |
> | Implementation C | t6 period's implementation take over                         | consecutive two same 400 `invalid_request_error` ⇒ per AGENTS item 11 "change member" disposition: **remove from team**, its task withdrawn to shared pool then re-dispatch                                                            |
> | Implementation D | t2 take over                                                 | attempt 2                                                                                                                                                                                                                              |
> | Implementation E | t2 close, t6 one attempt                                     | t2 3rd attempt complete; t6's one attempt also due to same 400 flow interruption judged failed                                                                                                                                         |
> | Verification     | t3, t5                                                       | each 1 time, verdict both pass                                                                                                                                                                                                         |
> | Review           | t8                                                           | 1 time, verdict = pass ×2                                                                                                                                                                                                              |
> | Close            | t9                                                           | 1 time                                                                                                                                                                                                                                 |
>
> **Provider side flow interruption and change member disposition caliber** (this round empirical
> reusable): ① **Flow interruption is not defect evidence**——task interrupted by flow interruption
> must not judge failure conclusion based on "zero product", can only judge "incomplete" and
> re-dispatch; ② After change member **don't rerun already concluded rounds** (AGENTS item 12:
> change member will withdraw all tasks of removed member to shared pool, already-decided rounds
> should not be rerun by scheduler), this round directly re-dispatched or cancelled withdrawn t6 /
> t7, no repeated work; ③ **Zero product during flow interruption** members all first look at board
> state then decide take-over party (this batch multiple times appeared "implementation already in
> work tree, just no one claims" situation).
>
> **7. Warehouse concurrent write and pre-commit stash window (this round two instances)**:
>
> 1. **Concurrent write**: During batch period main work tree was **written by another concurrent
>    session**, its stage area already contained `.github/workflows/ci.yml` and
>    `scripts/ci/check-docs-refs.py` etc. **other people's files**. Disposition = **commit right
>    closed to captain**: during batch period all members **must not execute `git commit`**
>    (`git add` still per explicit path), changes only stay in work tree, commit by captain
>    uniformly——otherwise one party's commit will roll in other's stage content. This batch's four
>    fixes finally by captain as four independent commits landed due to this. **Close commit's
>    collision instance** (this round empirical): Close commit and captain's concurrent commit
>    collided——captain that time's `git commit` read **this round's already staged two documents**,
>    brought this round's content into its own commit (commit information continued to use captain's
>    expression, only described "governance one section"'s CI backfill). Disposition = use
>    `git commit --amend` to change message to **simultaneously cover two places' content** (still
>    **one** commit, **one** message), and register this instance here. **Lesson**: Before commit
>    must confirm index in **only own path** (`git status --short` first column all `M ` and path
>    belongs to this task)——concurrent session's stage content will be rolled away by other's
>    `git commit` (this is exactly this section item 1's disposition direction: during batch period
>    commit right closed to captain).
> 2. **Batch directory deleted by third party**: `scratch.py`'s `rm` / `gc` will recursively delete
>    **whole batch directory** (that directory's reports and probes all disappear). This round
>    empirical: batch directory together with its in-location report and all probe logs during
>    implementation period deleted (**attribution unverified**: executor unknown). Disposition =
>    report and probes change to put **outside batch directory**, and key numbers and conclusions
>    **directly write into task output** (not depend on workspace file as source); after batch tree
>    rebuilded measurement party per frozen revision take over.
> 3. **pre-commit stash window**: This round instance = during commit std side that commit (amend
>    supplement IR snapshot) period, pre-commit hook **all unstaged changes** stash away, run then
>    restore ⇒ parallel reading work tree members see "only staged files left" illusion (this round
>    had member per this reported "changes lost"). Disposition per AGENTS item 10 ⑦: ① **Before
>    commit first give unstaged changes patch backup** (this round instance's three file sha256 =
>    `02e89141…` / `3736961f…` / `98eee1d7…`, after restore one by one same); ② During commit period
>    others only per **frozen sha** measure, not read work tree; ③ After each commit complete use
>    `git status` check remaining changes integrity.
>
> **8. Local hash rewrite comparison** (this batch's four commits during push experienced
> `git rebase` and `git commit --amend` two kinds of rewrite; `09`'s check row per convention
> doesn't write commit number, this record preserves **post-push** `origin/dev`'s hash and gives
> rewrite chain):
>
> | Content                             | Pre-push local → Post-push `origin/dev`                                                 |
> | ----------------------------------- | --------------------------------------------------------------------------------------- |
> | 4.10.4 (#467)                       | `950a5838e` → (rebase) → `eb4124cf0`                                                    |
> | 3.6.5 (#479)                        | `0ef2e8887` → (rebase + amend) → `14e9b2704`                                            |
> | std side (`\uXXXX` + `json_ops.yx`) | `48bc37af0` → (**amend supplement IR snapshot**) → `c67de7e2f` → (rebase) → `96638ec9f` |
> | 4.10.5 (#476)                       | `47c2d5292` → (rebase) → `06ac6d687`                                                    |
>
> `48bc37af0` → `c67de7e2f` this amend's content = take **regenerated IR snapshot**
> (`src/middle/core/tests/snapshots/src/std/tests/json_ops.snap`) supplement into same commit:
> original commit only contained `src/std/json.yx` / `src/std/tests/json_ops.yx` / corpus baseline
> three files, after amend four files——i.e. "change IR behavior must same submission regenerate
> baseline"'s red line fulfilled in **same one commit**, not leave one "baseline debt". Forensics =
> `git reflog --date=iso` (`commit` → `commit (amend)` → `rebase (pick)` three segments can read one
> by one).
>
> **9. Corpus / IR baseline in current byte re-verification + this round's two new findings**:
>
> - **Current byte re-verification** (close pre rerun on `origin/dev` head):
>   `tests/baselines/corpus-parity.jsonl` = **350 lines**, sha256 =
>   `76bd7a1cc82797fe0c2fb4f3406506ad65e7fe268d15b7d807448f4a30b5118b`; on that byte rerun corpus
>   diff probe, product and baseline **byte-for-byte same** (diff empty) ⇒ current byte zero drift.
>   **Note**: Two verification rounds (t3 / t5)'s judgment bound is **earlier** corpus byte
>   (`41cc0343…`), different from current `76bd7a1c…`——review already pointed this difference, this
>   round per current byte re-verify and account; thereafter reference corpus number must first
>   confirm cited byte.
> - **New finding ①: `src/std/tests/json_ops.yx` long-term zero coverage**——this file defined `main`
>   but **never called**; in script mode top-level statement is program body ⇒ 15 `check_*`
>   assertions **never executed once**, exit code always 0, i.e. **test false pass** typical
>   example. `std.json`'s `\uXXXX` defect therefore long-term not exposed. This round fixed
>   (supplement `main()` call + fix fixture's own three places error + fix `std.json`'s subscript),
>   after fix that file's 15 check assertions all really run and pass (exit 0). **Lesson**: "have
>   test file" ≠ "criterion executed"——in script form "defined `main` but not called"'s test file is
>   **false green** source, should be one class independent check item (comparison: #476's "defined
>   `main` not called" in rootless form exit 0, also let this kind of file long-term with exit 0 as
>   filler).
> - **New finding ②: ring import no W code warning**——4.10.4 made ring in-order **deterministic**,
>   but ring itself still silently tolerated: ring in unit read uninitialized top-level value ⇒
>   misleading E6007 "type mismatch". Already registered as **4.10.6** (issue
>   [#491](https://github.com/ChenXu233/YaoXiang/issues/491); user 2026-10-23 decision A =
>   compile-time W code warning, **must not be silent**; W's warning surface to be deliberated).
>
> **10. Gate numbers**:
>
> - **Close pre locally rerun on current `origin/dev` head**: `npm run docs:build` → exit 0 (real
>   build before document commit, AGENTS item 10 ⑨——markdownlint doesn't catch VitePress's Vue
>   compile error); `python scripts/ci/check-docs-truth.py` → exit 0;
>   `python scripts/ci/check-boundary.py` → exit 0 (0 violations, `pub(crate)` count guard
>   baseline); `python scripts/ci/check-fanout.py` → exit 0; `python scripts/ci/check-docs-refs.py`
>   → exit 0 (1124 tracked text files, 5535 path-like tokens, no unresolvable references); error
>   code generator rerun after `git diff --quiet` → exit 0 (generation product zero drift); corpus
>   diff rerun → with baseline byte-for-byte same (350 lines).
> - **Each fix surface's own gate** (each verification round measure on frozen revision): #467 ——
>   fmt 0 / clippy 0 / `cargo test --all` exit 0 (lib **2810 passed** + 2 ignored, integration 297
>   passed, 0 failed) / docs-truth 156/156 pages / boundary 0 / fanout 0 / docs-refs 0 / generator
>   rerun after zero drift / **IR snapshot diff empty**; #479 —— same caliber exit 0 (lib 2810,
>   integration **300** = 297 + new 3 nail tests per name pass); #476 —— **clean tree** full exit 0
>   (9 test target all ok, **3227 passed / 0 failed**, includes
>   `tests/yx_runner.rs::test_all_yx_files_pass`).
> - **Gate discipline one example**: Full `cargo test --all` when "shared work tree with uncommitted
>   changes" once red at `tests/yx_runner.rs::test_all_yx_files_pass` (`src/std/tests/json_ops.yx`
>   exit 1), once recorded as "this batch's defect"; on **clean frozen revision** re-measure that
>   case pass ⇒ that red point is "tree with #476 patch state"'s measurement, not fix surface's
>   property. **Lesson**: Full gate must on **isolated clean tree** measure, otherwise will mistake
>   "shared tree state" for "delivery defect".
>
> **11. Batch tree recycling (gc and status raw output)**:
>
> `python scripts/dev/scratch.py gc` → `[scratch] deleted wbs410 (released about 13268.2 MB)` /
> `[scratch] gc complete, total released about 13268.2 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`; `git worktree list` → only main mount (`dev`). **Also deleted with gc
> is batch tree's in-tree verification evidence** (each verification round's probes, mutation
> patches, raw logs and conclusion text)——they are **registered batch directories**, recycled with
> tree; this section two, three have inlined their conclusions, numbers, mutation table and gate
> numbers **as-is into record**, conclusions don't become invalid due to file deletion. **Unaffected
> archive**: non-registered batch directory's evidence directory (not checked in; conclusions
> already inlined in this record, workspace path not as reference source). Acceptance expression
> (resolvable): `git status` clean (after close document commit);
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main mount; no
> this-batch newly added directories outside warehouse.
>
> **12. Document real build and commit hygiene**: This round's check / note / registration
> **accumulated into one** document commit (AGENTS item 8); before commit locally run
> `npm run docs:build` (AGENTS item 10 ⑨) and `python scripts/ci/check-docs-truth.py`, markdownlint
> zero warning. **Don't push** (members don't push, see AGENTS item 10 ⑧); **CI / Deploy Docs
> conclusion handed to unified check link** for backfill, not waiting in session (AGENTS item 13).
> Also: this round incidentally rewrote two historical record's residual **local absolute path**
> (`git worktree list` output's reference) to expression not containing local path (meta rule ①).
>
> **13. Legacy items and registration**:
>
> 1. **Awaiting landing (got user 2026-10-23 decision, registered in `09`)**: 4.10.6 (issue #491,
>    compile-time W code warning, must not be silent; W's warning surface three to be deliberated),
>    4.10.7 (issue #492, really support native function as value, don't add code), 4.10.8 (issue
>    #493, class C, don't add code, can directly fix), 3.6.6 (issue #494, criterion convergence
>    refactor item, behavior unchanged).
> 2. **Known imperfect items**: ① **t6 task record terminal state failed, t7 cancelled** (provider
>    flow interruption + add frame timing) ⇒ #476's **independent verification round not separately
>    run**; its verification evidence from "clean tree full exit 0 (3227 passed / 0 failed)" and
>    that round's exit code nail test (`tests/integration/cli_e2.rs` two). ② Two verification rounds
>    (t3 / t5)'s judgment bound is earlier corpus byte, this round already per current byte
>    re-verified (see "Nine"), but **verification report itself's byte binding still old
>    value**——when reference must explain. ③ Review **didn't personally rerun** two verification
>    rounds' mutation construction (batch tree already released by multiple people, rerun needs
>    exclusive tree + one incremental build), use "product check + blob byte-by-byte check +
>    self-built probe" converted.
> 3. **Uncovered surface** (each verification round self-listed, this round not supplement): ① ring
>    import's W code warning not implemented (already registered 4.10.6); ② `check` and LSP form not
>    separately probed (share Linking arm with `run`, library-level nail test indirect coverage); ③
>    mutation only do "fall back to discovery order" one direction——ring in path order / minimum
>    index tie-break each failure form no reverse argument mutation (its behavior covered by nail
>    test assertion); ④ multi-package / workspace form not in CLI probe; ⑤ #479 side: native
>    function as value, three-segment module path, non-existing export name, `ExportKind` other
>    variants, mutation round not run full, LSP path; ⑥ review self-built 7 probes in, "pure yx's
>    std module function as value" one item marked **unverified** (already registered 4.10.8).
> 4. **Commit information scope rejected twice**: This batch's two commits' initial scope took from
>    **source directory name** (`ir_gen` / `cli`), twice rejected by
>    `scripts/hooks/check-commit-msg.py` (its scope allow table doesn't have these two words),
>    changed to **subsystem name** `codegen` / `util` then passed (allow table: `frontend` /
>    `middle` / `backends` / `std` / `formatter` / `lsp` / `package` / `util` / `proof` / `driver` /
>    `codegen` / `monomorphize` / `lifetime` etc.). **Lesson**: scope take **subsystem** not source
>    directory name.
> 5. **This batch didn't touch existing debt** (not in this team's scope, transferred as-is): 3.8
>    (native to user function name shadowing), 3.12 (runtime mode executor config propagation), 3.18
>    (cross-module type identity, prerequisite awaiting P6), 0.5.2 (pre-commit / CI wiring +
>    resource tier measured calibration), #489 ureq 3 migration.

> **14. CI gate conclusion (AGENTS item 13 ①, timed check 2026-10-23 backfill)**: Last push tip =
> `5c0a47570` (this batch close document commit) → CI **39907068941 success**, Deploy Docs
> **39907068812 success**, covers this batch's four fixes (`eb4124cf0` / `14e9b2704` / `96638ec9f` /
> `06ac6d687`) and document close. Coverage relationship (item 10 ③): `06ac6d687` itself that round
> CI **39797917813 cancelled**, `8a83485ef` that round **39906194379 cancelled**, both per
> "consecutive push cancel previous round, use successor conclusion" handle, respectively by
> `43cc68ee6`'s **39904034164 success** and tip's success undertake. **This batch no failure job.**
