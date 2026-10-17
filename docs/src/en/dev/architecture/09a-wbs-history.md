# WBS Historical Notes (Landing Record · Audit · Ruling Traces)

> This document is a **historical note** to [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task table): landing records, audit findings, ruling traces, and errata for each
> stage. **Division of labor**: 09 only describes the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document only records **history** (what was done, what was
> found, how it was ruled), appended chronologically without rewriting history.

## P2 Establishing the Equivalence Criterion Baseline

> **P2 landing (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green run.
> **D38 prediction fulfilled**——the green run process actually measured and fixed three real defects
> in ir_gen (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime tolerated by Frame::set_slot), unannotated parameters dropped by
> filter_map (three places, params metadata misaligned with frame layout), match result slot relying
> on frame's implicit Void initialization (now explicitly pre-initialized). The actual semantics of
> jump target is flattened instruction index (not label-keyed as in the 07 text), CFG built at
> instruction level——registered in the verify.rs file header. **Commit comparison**: ir_gen fix =
> `a9ea199b`; verify main = `d5b18390`（⚠ parallel stream COMMIT_EDITMSG collision caused its
> message to be mislabeled as「docs(rfc): RFC-027a…」, content is actually verify.rs + six test
> files, buried in history unamendable, hereby recorded).
>
> **P2 narrowed portion landing (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3 stop-bleed
> channel preconditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract uses the same `TestFileSpec` as the
>   single-file corpus (single point on library side), binary positioning/subprocess invocation
>   extracted to `tests/common/` and yx_runner migrated in sync (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed by rfc027 tests (return position/call point/binding position)——02's
>   `Sorted(3)` annotation example, when substituted in current code, would be directly evaluated as
>   Disproved without silent Unproven; the `Sorted(3)` in 07 pseudocode is for illustration only.
>   Red state measurement data is in the test file header.
> - The two red criteria are permanently attached with `#[ignore]` per the 2.4.5 red skeleton
>   precedent: default suite is all green, `-- --ignored` reproduces red state; **removing the
>   attribute when P3 fix lands turns green**.
> - The reason for deliberately diverging from the 07 (code, file, line) table in comparison
>   normalization (dropping file/line, only comparing error code sets) is documented in the 2.4.1
>   test comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **removed from P2**: the `Program` / `Obligations` types they reference only exist in P4, and P2
> cannot compile. The former is P4's 4.1.4 (do not double-register), and the latter's `#[ignore]`
> red skeleton is merged into 4.3.1.
>
> **2.3.3 landing (2026-10-07)**: C4 mandatory coverage list item-by-item checked (07 §143), 10
> categories of semantic corpus layer full coverage, all within corpus-parity.jsonl differential
> baseline (26/26 hit on 26 representative files, grep verified)——ref/borrow/move entire
> 05-ownership directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep,
> etc.), closure captures closures + closure_arg_inference + spawn_capture, currying
> curry_value_fix + curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files,
> sum types sum_type__+ pattern_or_guard + match_call_scrutinee, existential enforcement point
> interface_dynamic_dispatch (Vec(Animal) existential type heterogeneous wrapping triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (+
> method_overload_ambiguous_err on diagnostic diff surface), refinement constraints Drop sequence
> refined__ ×5 + 05-ownership driven ReleasePlan (span key contract has another 2.4.4 red criterion
> permanent residence). Performance baseline `benches/pipeline.rs` committed (07 §145 three smoke
> tests, two of three): `corpus_compile_frontend` first full corpus compile test 17.6s,
> `cli_cold_start_hello` process cold start 169.7ms; interpretation throughput already covered by
> benches/lib.rs hotpath group (interp_fib_recursive_27 / interp_loop_steady_10m), not duplicated.
> Regression gate check-perf-regression.sh goes to P9 unified list (2.5).

## P3 Fixing Correctness Vulnerabilities (Minimum Solution)

> **Error type annotations for landed rows** (retrospective annotation, does not change history):
> 3.1 = B ｜ 3.2 = B (defensive empty arm, criterion turning green is borne by 3.1) ｜ 3.3 =
> A ｜ 3.4.1 / 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 /
> 3.4.7 = D (user-visible surface, already user-ruled) ｜ 3.4.8 ①② = C (main) + A (three
> attached) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-ruled).
>
> **P3 diagnosis (2026-10-12, user requested)**: A diagnostic accounting triage in response to the
> perception that "P3 entries continue to grow".
>
> **I. First clarify the scale (perception amplified by two accounting defects)**
>
> 1. **Duplicate numbering**: `3.5.x` is shared by two task families——(a) **029g `pub` visibility
>    convergence** on 2026-10-08 (7 three-level: lexical and parser branches / AST `is_pub` /
>    checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) **3.5 higher-order type parameter call path
>    completion** registered on 2026-10-10 (3 three-level). Both share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 renumbered per user ruling: 029g family changed to `029g.x`**), and
>    caused 029g's rows to be miscounted into P3. **Pending ruling**: rename the 029g family to
>    `029g.x` (or move it into its own stage), and update other document references in sync.
> 2. **Missing check marks**: 3.7's 4 three-level items (3.7.1–3.7.4) are all completed (d922a551 /
>    38bd4b9e) but unchecked ⇒ this entry is corrected in this round.
>
> **II. P3 actual accounting (12 second-level rows)**
>
> | Row                                                                  | Status                                                                                                                                                                             | Disposition recommendation                                                                                                            |
> | -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                      | Completed                                                                                                                                                                          | —                                                                                                                                     |
> | 3.4 Silent drop point addendum (8 three-level)                       | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlocked by 4.3.1                                                                              |
> | 3.5 Higher-order type parameter call path completion (3 three-level) | Not started (declaration side live, call side broken)                                                                                                                              | Keep in P3; **does not block refactor main line**, but is "accepted feature not implemented", higher priority than pure cleanup items |
> | 3.6 Function type value call path (1 three-level)                    | Not started (semantics already cut (i))                                                                                                                                            | **Keep**: correctness defect, clear criteria                                                                                          |
> | 3.7 init segment call name qualification (4 three-level)             | **All completed**                                                                                                                                                                  | Check and close this time                                                                                                             |
> | 3.8 native shadowing (1 three-level)                                 | Not started                                                                                                                                                                        | **Stuck on decision** (first decide name resolution priority), not a scheduling issue                                                 |
> | 3.9 / 3.10 / 3.11                                                    | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                                     |
> | 3.12 Runtime inter-mode executor config propagation (1 three-level)  | Not started (Class D, user ruling B: separate team)                                                                                                                                | **Recommend move out of P3 → P8 (execution layer)**, pending ruling                                                                   |
>
> **IV. 4.3.1's leverage (main line marker)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the precondition for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ taking it once advances the main line and clears two P3 stragglers. 3.5 (HKT),
> 3.6 (function type value call) and 4.6.1/4.9.1 all **have no unmet preconditions**, freely doable;
> 3.8 is stuck on semantic decision (user-defined priority vs explicit diagnostic).
>
> **III. Mechanism recommendation (to prevent recurrence)**: Newly registered second/third-level
> rows **must include two fields**——"belonging stage" (which P it truly belongs to) and "whether it
> blocks the main line"; `registered awaiting scheduling` is no longer a default state, unmarked is
> treated as unschedulable. This way, bloat is visible and sortable, not piled up indiscriminately.
> **Pending ruling**: whether to write this into this document's registration convention and
> `src/dev/coding-rules.md`.
>
> **P3 landing (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter (recorded as
> `pub(crate)` at the time; **2026-10-07 verified actual landing is `pub(super)`**——the `#434`
> ruling referenced in code comments is not registered on docs side, see 3.4.3; fully private
> requires rewriting 11 constructor literals, not taken in the bleeding stop stage; collected when
> P4 Obligations migrates); execution mechanism extracted to `src/frontend/proof_execution.rs`
> (prohibition three single point——cannot be placed in typecheck/, that would create L3→L4 reverse
> dependency); 3.1.2 supplemented **four entry points** per 02 evidence chain step 5 (WBS row number
> missed compile_project `:99`, must be supplemented and registered per 3.1.1 acceptance "criterion
> turning green"); 3.2.1 empty arm changed to record-keeping (proof_calls throws up) + diagnostics
> (`into_result` established conversion path); 3.3.1 SOLVER slot made Option, Z3 missing
> conservatively degraded to `SMTResult::Unknown` (E2031 family diagnostics replacing process
> crash). Two vulnerability criteria removed `#[ignore]` turned to normal green, supplemented corpus
> fixture `proof-obligation-honored`; check-fanout stock migrated accordingly (wiring point
> 19=stock). Measured: criteria 2.4.1/2.4.3 turned green, full suite lib 2494 / integration 272 /
> corpus dual-layer all green.
>
> **3.4 audit (2026-10-07, P3 post-landing recheck)**: For defects of the class "failure produces no
> signal" (definition in 02 §41-52), re-inventoried on unchanged code, located 8 more, all
> registered as 3.4.x above. Methods and evidence:
>
> - **Consumer-side inventory** (16 obligation fields grep'd for all repo read points):
>   `variant_ctor_calls` **zero consumers**——the only read at `checker.rs:1468` is "pump it into
>   `TypeCheckResult` itself", IR side uses shape detection at `ir_gen.rs:7374`;
>   `rfc011b.rs:659/688` only asserts producer side, exactly the form criticized in 02 §186 "only
>   test producer side, naturally immune to consumer side defects" → 3.4.4.
>   `method_overload_ir_names` result-level field also has zero external consumers, IR uses parallel
>   `overload_resolutions` → 3.4.5.
> - **Stage coverage recheck**: 02 §91 table row 1 (four entry points "no proof_execution") fixed by
>   3.1.2, current state is 5 locations sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 unchanged (multi-file `run` still doesn't
>   output W1003, doesn't run dead code family) → 3.4.6; LSP non-project-internal path (02 §77
>   "manual lex→parse") also doesn't run proof → 3.4.7.
> - **Same-shape empty arm scan** (only 4 `Unproven` consumers in whole repo): checker three
>   refinement branches (covered by 3.1), 3.2.1 fixed ownership arm (**unreachable in source
>   code**), and `environment.rs:422` (**reachable, zero diagnostics, zero record-keeping, and zero
>   registry in whole repo docs**) → 3.4.1, the only "unknown to all" item in this audit.
> - **Solver acquisition side**: `default_solver()` has three consumer forms with different shapes
>   (`predicate.rs:32` process singleton / `checker.rs:1266` injected each `check_module` /
>   `ownership.rs:621` created each back-edge judgment)——3.3.1 only eliminated one panic; the
>   **silent side** of the other two goes to 3.4.2, **singleton-ization and cache hit** goes to P4's
>   4.4.2 (do not double-register).
> - **Existing attribution not counted in 3.4**: 2.4.4 (`ReleasePlan` span key mismatch, 168 files /
>   373 keys unconsumed, Drop silently lost) is still P2 red state permanent residence, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer sequence declaration opposite to reality, `equivalence` layer
>   not in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4
>   registry.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions, can land in place at any
> time (recommend doing 3.4.1 first——the only "whole repo zero registry" live vulnerability,
> smallest change surface); 3.4.4–3.4.8 go with P4 execution, their criteria depend on obligation
> ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish a new criterion system**,
> uniformly reuses existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 record (registered 2026-10-13 → closed 2026-10-15)**: User 2026-10-14 ruled to
> merge four correlation minor items (#451 method binding panic / #452 named actual parameter in
> resolution / #453 dead inference entry / #454 native export named actual parameter——the last item
> was a boundary finding when verifying 3.14, same day attribution experiment verified **not
> introduced by #452**, user ruling ① merged into this batch) into one team for handling. Closure
> status: all four completed, ruling registered as RFC-039 **D60** (six binding rules + shared
> criterion single-point-ization).
>
> **I. Two contract corrections (lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradictory**: #452 task contract required both "mixed actual
>    parameters output 20" and "positional and named dispute same position reports error", but the
>    nailed fixture `Temp.pick(self, a)` has only one value formal parameter——the same actual
>    parameter combination is both "mixed" and "conflict". First assignee truthfully reported
>    blocker and set task failed, overall revert (no silent for green light). Disposition: task
>    reassignment + contract revision——after giving **six binding rules** and **8 value-level
>    matrix** (fixture changed to two value formal parameters `pick(self, n, a)`), attempt 2 was
>    completed by second assignee. The six rules in full are verbatim in D60.
> 2. **inScope missed mechanical necessities**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only `code-tables --fix` was
>    run, **`scripts/docs/gen-error-code-docs.py` was not re-run** ⇒ CI Docs Truth red ("source
>    registered/emitted but doc missing 1: E1110", source 154 / doc 153, run **38518624964**——the
>    only CI red line of this batch). Fix = mechanical regeneration (8 files +83/−70, only touched
>    `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **II. Review → fix → re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls with named actual parameters silently pass through on two
>    paths: multi-candidate zero hit / single candidate hit) → fix round added external code **E1111
>    (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece set in same
>    commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared resolution entry + native arm settle by name) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations
>    mechanically reversible and replayable via `scratch.py trial` all caught, restore byte-by-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inlined copy** of a
>    binding criterion, missing rule ④ ⇒ **ruling divergence** for method vs native same-shape
>    calls——method reports E1111, native silent exit=0; ir_gen has a third existing copy reporting
>    old code E1015) → convergence round consolidated criterion to single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call sites `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught a **regression** introduced by the
>    convergence round (IR convergence block lost native omittable position ⇒ programs that
>    previously succeeded by omitting optional positions were falsely reported E1010) → fix round
>    (optional position criterion and formal parameter name from same source `signature.rs:236`
>    unique exit) → round-3 review PASS (independently reproduced red→green; mutations nailed
>    fixtures in repo FAILED, empirical criterion can fail). **Lesson**: review must cover "unfixed
>    branches of similar paths" (this batch's two needs_revision were both caused by probes all
>    falling on hit surface); the fix round itself can also introduce regressions (omittable
>    position), caught by next round's replacement reviewer.
>
> **III. Process events (registered truthfully, not pointing at individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: The working tree was silently
>    restored by an external party——the implementer's in-progress fix (typecheck/ir_gen/code
>    table/locales/RFC-013, etc.) all disappeared, **at the same time the two review documents
>    edited by the user themselves also disappeared from git status** ⇒ determined to be
>    repository-level discard (most likely user-side manual cleanup/IDE "discard all changes").
>    Three pieces of evidence excluded team members: stash empty, no new commit, no reset in reflog.
>    Implementer replayed in-progress work by context and pushed; discipline changed to "**save
>    patch to scratch first, then take red state**".
> 2. **Verifier context exhausted, replacement**: The independent verifier verifying 3.16 ran out of
>    room mid-way (no closing message), replaced per load rules——successor continued with their
>    ready products (green results, corpus equivalence script, mutation plan), verification finally
>    PASS. Positive precedent of partial product handover.
> 3. **Review round 5 rotation and historical round return**: When rotating the ⑤th review per load
>    cap, `remove_member` returned the already-ruled final review/verification rounds **together to
>    the shared pool**, scheduler immediately re-ran (t3/t8 were picked up again by new members,
>    duplicated work and blocked downstream). Disposition: team lead took over and canceled the
>    returned historical rounds (t3/t8/t13), based on this added AGENTS.md item 12 (handle returned
>    rounds immediately after member change). This batch's closure record registers this event per
>    that item.
> 4. **scratch governance (about 6.3 GB)**: Early batch used one external tree per task, 6
>    directories scattered outside the repo (about 780 MB, already cleaned) + two members' external
>    verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md item 9
>    (batch tree unique entry), item 10 (verification environment six rules: mechanically reversible
>    mutation / line number from referenced revision / CI-only gates must run, etc.), item 11
>    (writer load cap), item 12 (handle returned rounds after member change) and top meta rules two
>    (no private content / verify before assert).
> 5. **Contract defect (not pointing at members)**: #454 fix round's contract inScope only circled 2
>    files, actual mechanical necessities 8 (item 6 is exactly to prevent this——contract template
>    should pre-set mechanical necessities list); also that task's terminal field was once occupied
>    by probe payload, formal acceptance/command evidence was supplemented by team lead per spec.
>
> **IV. Stragglers (not fixed in this batch, registered for reference)**: round-3 review three
> low-severity findings——① method path's `build_arg_slots` (`ir_gen.rs:521`) is still a **second**
> implementation of the criterion (existing, per previous round's scope registered at closure, not
> consolidated)——**2026-10-16 pre-release audit supplemental three points**: (a) it has **two** call
> sites: `ir_gen.rs:7888` (method binding path, uses bound function `binding.function`'s formal
> parameter name) and `:7975` (overload resolution path, uses resolved mixed-name leaf segment),
> original registration only wrote "`ir_gen.rs:521` area"; (b) it has **already diverged from shared
> implementation `bind_args_to_params`**: missing rule ⑤ (duplicate naming → E1015), rule ⑥ (missing
> required formal parameter → E1010) and "optional position" concept, and hardcodes receiver
> position to `1 + i`; both call sites `continue` on empty slot (comment writes "diagnostic goes to
> typecheck (it guarantees resolution passes means no empty slot)"), a **dangling invariant written
> in comments without assertion**; (c) **RFC-039 D60's "all call sites converge to it" enumeration
> missed these two**, declaration and fact mismatch, and the cited line numbers were not re-fetched
> on the referenced revision, D60 text corrected together with this entry; ② batch verification tree
> (worktree) `cargo fmt --all` error is a worktree workspace artifact, gate list should note fmt
> goes through main repo; ③ repair-type contract template should pre-set mechanical necessities into
> inScope (see III.5).
>
> **V. Final gate (closure revision, local two implementation commits + one doc commit pushed
> together——the only push of this batch)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 /
> corpus_probe **342 rows byte-by-byte consistent with baseline** (only regenerated row:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number in sync, manually
> reviewed) / IR snapshot zero drift.
>
> **VI. run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test was cancelled by subsequent
> push per concurrency——cancel≠fail, covered by subsequent run); governance tool push → CI
> 38546373435 / 38561405117; WBS registration (3.16 row + header 56→57 / 147→148) → CI 38528560364.
> The closure push's own CI / Deploy Docs run was supplementarily registered in the task record
> after push.
>
> **3.4.8 ③④ landing (2026-10-11, 8ecf56f93)**: ③ Added `Linking` to end of `CHECK_STAGES` (**pure
> merge**, entire block skips `is_bin_role` entry validation——E3020 criterion still belongs to
> `RoleClassification` to prevent double reporting; another hard reason: run path judges "has main"
> by merged IR, under collection semantics "entry itself errors → no IR" would be misjudged as
> missing main and hard-abort) → `Monomorphization` (check shape); `ir_generation_check` retains
> unit IR (**in memory only**, for pure merge and mono consumption, not emitted / not written to
> disk / not producing snapshot——if not retained, aggregate request is empty, mono passes with
> `NoObligations` false green); mono failure takes per-file collection channel in Check (diagnostic
> goes to entry file + `failed_stages`, not set to failure), MultiFile keeps `fail_with`;
> `into_outcome` changed to "take collection first then judge failure". ④ Empirically **zero
> production changes**: whole-program BFS already covers cross-unit mutual recursion (pathological →
> E3005, well-formed three-layer chain executes correct value), delivered four e2e + one mono unit
> test as living criterion.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-check with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-shape matrix
> (single/multi/check three shapes + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase).
>
> **F1 (discovered during verification, D59 A/B class, fixed in same round)**: Under standalone
> multi-file check without project root, aggregate request name (`lib.f`) and unit IR definition key
> (`f`) are not from same source → mono falls into "native has no IR definition" silent `continue`,
> check reports 0 error/exit 0. Fix: aggregate side added reverse lookup by each unit's **actual
> definition key** (exact enumeration + exact string equality + ambiguity abandon, not suffix
> heuristic; project shape not registered, existing path unchanged). **Erratum (same day)**: this
> reverse lookup was **deleted** in a subsequent regression fix; root cause solved from naming
> perspective by `module_key` dual-use decoupling (added `script_mode`)——IR definition key = IR call
> name = aggregate request name = `{unit key}.{name}`.
>
> **Regression record**: After ③ landed, CI's `Check tutorial/guide code examples compile` red (258
> blocks / 10 failures: 6 same-file same-name rebinding + 4 colliding with `<std/list>`, rootless
> check doesn't qualify triggering `link_module_irs` conflict check); fix = ir_gen split
> `script_mode` (qualified name ⊥ Script/Bin, E3023 gate changed to look at it) +
> `ir_generation_check` always passes unit key per unit + `.with_script_mode(rootless)` + Script
> top-level same-name rebinding "latter wins" (consistent with run path existing behavior);
> verification = doc gate 258/0, three-binary comparison (HEAD/③ tree/fix tree) three collision
> fixtures, check shape E3005 no regression, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic goes to entry file entry, span can cross units (same scope
> as run path `path=entry`); Check shape holds one ModuleIR per unit (in memory only, released when
> check ends).
>
> **Existing issues left for follow-up (not this round's regression)**: ① E3005's help text still
> writes "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user code error" out of E3005)——registered awaiting
> scheduling; ② multi-file E3005's span rendering label misalignment (reproduced with HEAD binary,
> pre-existing); ③ request collection side has **type parameter position dependency** (generic calls
> where type parameters are not at value parameter position don't emit instantiation requests, mono
> records `NoObligations`, converging mutual recursion is this shape)——same surface as 3.5 (HKT call
> path), registered to that row.

## External Track RFC-029g: Removing `pub` and Auto-binding (#399

> **P3.5 landing (2026-10-08)**: All seven second-level tasks completed, one commit for the whole
> batch (independently revertable).
>
> - **Measurement not matching plan (registered truthfully)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"——actually after deleting `KwPub`, `pub`
>   falls back to a regular identifier, old writing lands on semantic layer reporting **E1001
>   Unknown variable: 'pub'** (span points at `pub`). Per RFC-039 **D27** "specially detecting
>   removed syntax is an anti-pattern", **do not add** migration-specific diagnostics; corpus
>   fixture `pub_declaration_err.yx` nails E1001, documentation (`syntax.md`) truthfully writes this
>   consequence.
> - **Design convergence (beyond RFC literal, needs review)**: After pub deletion, `project_refs`
>   and `cross_file_refs` fields have fully overlapping semantics (same "package-internal reference
>   union", prohibition one forbids parallel representation) → merged into single `project_refs`,
>   `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally, Internal role's
>   non-pub items change from "file-internal reachability" to "reference pool judgment" (relaxed, in
>   the direction of false negative); Lib tightens from "pub absolute exemption" to reference pool
>   judgment; Script top-level bindings not referenced start reporting W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (producer pumps, whole
>   repo zero consumption).
> - **Test migration** (test count doesn't decrease): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs` "pub is recognized" assertion reversed to "pub is no
>   longer a modifier"; `passes/tests/dead_code.rs` 7 role semantics cases rewritten (assertion
>   reversed 2); `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption
>   assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` doesn't participate in IR); corpus differential
>   baseline adds only 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated, manually reviewed.
> - **Measured**: lib 2556 / integration 272 / corpus 322 entries baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-layer Contract PlanId-ization (D20)

> **4.5 landing record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS check / gates / push) completed by t7. Precondition = 4.3.1 obligation ledger
> (red state archive 173 files / 533 keys).
>
> **I. Carrier ruling (A: `PlanId` walks with AST node)**: Added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` for synthetic nodes
> outside parser), issued by **unique allocator** `ParserState::alloc_plan_id`; carriers two
> places——`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call point identity),
> `Expr::call` is the whole repo's unique call constructor. Rejected is the "two sides each derive
> same identity by span" position-coincidence form (D20's original defect).
>
> **II. Two-level verification and review (members independent)**: Each functional commit paired
> with an independent verifier and reviewer, **verification/review do not reuse implementer's
> evidence**: 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations)
> / t3 review; 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations)
> / t6 review. Review time point and reviewed revision both fall in immutable history (`git show`
> verifiable). **Provider rename caused t5 / t6 member change and re-assignment**: original
> verification/review members couldn't continue due to provider rename, replaced and re-assigned per
> orchestration convention **and reason written in record** (cross-round context retention priority,
> only replaced when unusable); re-assignment results consistent with original rounds (PASS),
> implementer's evidence not reused.
>
> **III. Two defects found and fixed during implementation self-check (t1, both within inScope)**: ①
> Consumption exemption criterion was once attached to `UnitState.ir.is_some()`——Linking/Mono in
> check shape takes `unit.ir` away, real non-consumption passes silently in check shape (mutation
> measurement: D41 red but CLI check exit 0); changed to read **the ledger's own completion
> marker**, and added regression cases. ② Drop sequence drifts with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion produced 106/120/114 line drift in three
> consecutive runs; changed to **total order sort** (same name takes max depth + name ascending),
> zero drift in two consecutive runs after.
>
> **IV. Criteria and evidence numbers**: After 4.5.1, `test_release_plan_spans_consumed` removed
> `#[ignore]` permanent residence (`examined ≥ 173` floor + no whitelist), 172 IR snapshots
> regenerated in same commit (programmatic review: added 1083 lines Drop + 86 lines jump index
> translation, zero deletion, no rearrangement), corpus 342 entries differential empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference set empty + evidence surface floor `examined ≥ 1`
> / `consumed ≥ 2`; measured examined=1, consumed entries 2, no resolution skipped 210), resolution
> shape 9 items nailed, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation drive: consumption key offset by one ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; restored to green.
>
> **V. Semantic correction classification basis (D59)**: `Type.method(positional arg, name = value)`
> **silently gave wrong answer** before the change (measured clean compile, runtime output `1`
> instead of `20`) ⇒ **Class C** (old behavior not successful), fix takes "loud": after change same
> program compile-time `error [W1090]`; the final semantics of "named actual parameter participates
> in resolution" is **Class D**, registered §3.14 and left issue #452 for dedicated team. Also:
> `Drop` is no-op in interpreter (`backends/interpreter/executor/ops/control.rs` wildcard arm only
> `advance()`)——key migration doesn't change runtime behavior.
>
> **VI. F-1 / F-2 disposition (t5 independent verification found)**: **F-1 (medium)** unbound
> consumption point (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, no red
> state falsifiable for key correctness ⇒ registered as "known boundary" into
> [02](02-stage-contract.md) §2 revision note fifth paragraph (preservation reason + no new task);
> **F-2 (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise
> pointers (produced `inference/expressions.rs:4550`, consumed `ir_gen.rs:7697` / `:7772`), separate
> small commit, not mixed into doc commit.
>
> **VII. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settle markers); 07
> criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one line behavior change; `check-obligations.py` and `check_tracking.py` green,
> CI and Deploy Docs green. **Accounting deviation found at closure**: pointers `ir_gen.rs:357` /
> `:3102` / `:1096` in t3 / t6 reports differ from code fact by one to three lines (actually `:360`
> / `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion)——registered per code fact, this note also
> retains trace.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (run half of 4.2.5 ruling A)

> **C-batch record (registered 2026-10-15 → closed 2026-10-15)**: User 2026-10-15 ruling C——this
> round only does 4.10.3 (same batch candidate 3.4.6 deferred to after 4.9 per B1
> ([02](02-stage-contract.md) §runtime behavior table "dead code family warnings don't enter compile
> path"), preconditions changed and noted); implementation form = **full single team** (user
> instructed no DAG, no team, single session implementation + self-verification). Pre-batch debt
> landed first: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit` from
> exit 1 restored to exit 0), WBS registration debt (#456 registered as 3.17 + incomplete rows
> supplemented with "belonging stage / whether blocks main line" fields).
>
> **I. Red before green**: 3 driver-level red criteria (`src/driver/tests/run_rootless.rs`——rootless
> merged IR must not carry entry_function (red state `Some("main.main")`), rootless top-level
> executable statements must not report E3023 (red state doesn't produce module), Bin control
> entry_function preserved (green state permanent residence)) + CLI e2e 3 nails (standalone relative
> import run (red state E5001), Script top-level statement execution and #413 hint two green state
> nails).
>
> **II. Implementation side**: CLI `run` uniformly goes through `compile_project` (the
> project/standalone bifurcation in `run_file_with_diagnostics` deleted); `entry_function` in
> Linking gated by `is_bin_role` (#413 T4——Script doesn't auto-call main); #413 hint key changed
> from `module.source_files.is_empty()` to call point project probe (after unification ModuleIR
> always carries source_files, original marker invalid); `script_main_never_called` adapted to
> qualified name form (function `{key}.main`, global slot measured also qualified name——based on IR
> evidence, not comment inference).
>
> **III. Existing gaps exposed by unification (fixed in same batch, A/B/C class——only fixing failing
> programs to succeed, zero impact on previously successful programs)**:
>
> 1. **Explicit type actual parameter pre-specialization name** (`mk(Int)(5)`): ir_gen directly
>    emits specialization name per D6.3 (mono strips unspecialized original), `resolve_call_target`
>    three arms (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call point dangling
>    E6006. Fix = that single point resolver added fourth arm (base name qualification, type
>    parameter string preserved as-is). **Bin project same-shape reproduction (pre-existing defect,
>    not introduced by this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign same
>    shape as declaration) per statement ⇒ same variable allocated 3 slots, initialization writes
>    later-allocated slot but read uses earlier-allocated slot (runtime Add type mismatch). Fix =
>    two places merged by name (same criterion same source).
> 3. **Same-module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table lookup verification (exact hit →
>    generic's unit key qualified hit → fall back to bare name current; not suffix heuristic, same
>    form as `resolve_generic_name`).
>
> **IV. Registration gap (not fixed in this batch)**: Constraint dispatch when **type and generic
> definition in different modules** still E6006 (rootless two-file probe verified; Bin same
> shape)——root cause = MonoType user type name doesn't carry definition module, correct fix belongs
> to P6 type identity convergence; registered WBS **3.18** (belongs to P3, precondition pending P6,
> does not block main line), `qualify_deferred_method` notes same surface.
>
> **V. Test adaptation (honest registration)**: `tests/integration/multifile.rs` fixtures
> supplemented with manifest——what it tests is RFC-029 project orchestration (Bin auto-entry) not
> Script semantics; `cli_e2e` one nail (top-level diagnostic variable name visibility) and
> `script_main_hint` fixtures (7 cases) synchronized to qualified name form——production and
> consumption only eat merged IR, fixtures drive same form directly, not using pipeline product
> substitute.
>
> **VI. Final gate (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> differential baseline regenerated **12 rows**——line-by-line structured check: exit / stdout /
> diagnostic code all identical per item, only stack frame function names and variable names
> qualified by unit key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process events (registered truthfully)**: In this batch, during pre-commit hook run,
> concurrently editing workspace doc was eaten by the hook's stash rollback and that commit failed
> (a concurrency form previewed in config comments)——discipline = don't touch workspace during hook
> run; #455's commit was split into two attempts (second all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 closure)

> **4.6 batch record (registered 2026-10-16 → closed 2026-10-16)**: User 2026-10-16 instructed to
> proceed in order 4.6 → 4.9 → 4.4 → P5, first do 4.6; implementation form = **full single team**
> (continuing previous batch's user instruction: no DAG, no team, single session implementation +
> self-verification). Task definition (09 three-level table): project-internal file's semantic data
> (SemanticDB) goes through orchestrator same pipeline, same source as diagnostics; cross-file
> references carry `resolves_to`. Acceptance: project-internal cross-file jump hits definition;
> semantic data and diagnostics come from same compilation.
>
> **I. Current state empirical (verify before assert)**: didOpen/didChange triggers **two trips** of
> compilation——diagnostics go through unified Driver pipeline (4.2.3/4.2.6 unified), semantic data
> goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` resolution fails, cross-file reference events not produced, jump/rename/reference lookup
> silently fails for imported symbols——D55's registered silent failure). Orchestration arm's checker
> uniform name `"<module>"` (SemanticDB file identity lost), `type_result` (containing semantic_db)
> discarded when diagnostics non-empty and never emitted; `checker.rs` discharge overwrites
> `resolves_to`'s DefId file identity to current file; `Export`/`ModuleInfo` both have no definition
> location surface.
>
> **II. Red before green**: LSP e2e nail (`src/lsp/tests/cross_file.rs`)——temp two-file project
> didOpen main.yx then GotoDefinition at `add_one` call point, **before fix returns None (red,
> empirically silent failure in previous section)**, after fix hits lib.yx line 0 definition
> (green). Mid-process probe location: compiler-side chain (driver probe dump) already fully
> connected but e2e still empty——root cause is `resolve_reference` exact match by reference **start
> point** (D55 precise-only current scope), test cursor falls in identifier middle; this tolerance
> is existing behavior surface, this batch doesn't touch (see VII).
>
> **III. Implementation side**: (1) `Export.definition_span` (extract_module_info harvested from AST
> identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled by DiscoveredFile, including
> vendor; std always None); (2) checker reference discharge via `import_def_locations` (use line
> binding span → definition file + definition span) redirects cross-file imported items'
> `resolves_to` out of this file; std import unregistered, maintains falling to use line; (3)
> checker module identity changed from unified `"<module>"` to unit file path (verified
> `env.module_name` all 4 consumption points zero diagnostic consumption); (4)
> `DriverOutcome.semantic_db` channel——CollectAll form (Lsp target file **regardless of diagnostic
> success or failure** + SingleFile fallback) merged in, Lsp arm tail injects non-target file
> top-level export definition surface from registry (`external_definitions`, can jump to unopened
> files too; type/submodule export not injected——typecheck side type registration doesn't produce
> definition events, both sides maintain consistent gap); (5)
> `orchestrator::analyze_source_in_project` (`check_source_in_project` calls half to take
> diagnostics); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key uniformly normalizes file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stably hits); 5 handlers +
> semanticTokens dual requests connected to `db_key`.
>
> **IV. Fixed in passing (D59 split B class——old behavior silently does nothing, previously
> successful programs zero external impact)**: ① project-internal file cross-file
> jump/rename/reference lookup from silent failure to hitting real definition; ② lexical errors: old
> manual pipeline deleted semantic info (tint disappeared) → residual AST tint preserved (4.2.6
> editor philosophy extension); ③ target file with type errors: semantic data no longer discarded
> with `type_result` (typing intermediate tint/jump not lost).
>
> **V. Nails**: 6 driver-level (`src/driver/tests/lsp_semantic_db.rs`——target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback to use line, error file preserved, single file fallback path key, single file
> error preserved) + 1 LSP e2e (red before green) + 4 URI normalization
> (`src/lsp/tests/uri.rs`——drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **VI. Mutation verification (scratch.py trial, mechanically reversible)**: Batch tree wbs-46 @
> closure revision 39546020, both mutations make hit nail turn red, after restore blob consistent
> with HEAD + git status CLEAN——m1 removed `import_def_locations` registration (statements.rs) ⇒
> cross-file driver nail + LSP e2e nail red, other 7 green; m2 removed `external_definitions`
> injection (arms.rs) ⇒ same two nails red (round-trip parsing loses anchor), other 7 green. Batch
> tree gc'd (released 4.8 GB, `status` no batch tree).
>
> **VII. Final gate (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new nails) / `--test integration` **287 passed** / corpus
> differential **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS
> (13·13·0·8, `semantic_db` non-obligation reason consumer anchor synced server.rs:482 and 02
> §obligation field table) / check-docs-truth 0 / docs-examples 258 entries 0 failure / docs-orphan
> 0 / markdownlint whole repo only remaining user-uncommitted 032 two MD056 (user's own edit,
> untouched) / pnpm docs:build 0. **Process event (registered truthfully)**:
> en/09a-wbs-history.md:54 one MD037 was introduced by auto-translate bot re-translating previous
> batch's 09a (zh original has no space), this batch fixed in passing——markdownlint red not from
> this batch's change surface but same file as 09a, registered together. **Residual observation
> (registered for discussion, untouched)**: `resolve_reference` only exact matches by reference
> start point——when cursor falls in identifier middle, jump/hover/rename silently does nothing (D55
> precise-only current state, whole-file form isomorphic), if tolerance matching needed is
> independent behavior change, report to user for ruling.
>
> **Same-day follow-up batch (4.6.2, landed after user ruling)**: After delivery of closing report,
> user ruled on "residual observation ①" as "OK, then change it"——lookup key tolerance implemented
> per plan A: four handlers' definition/hover/references/rename lookup keys changed from cursor
> position (via UTF-16 conversion) to **identifier's own start point** already located by
> `find_identifier_at_position` (same coordinate system as semantic events, no conversion needed);
> `resolve_reference` start point exact match semantics unchanged (D55 precise-only unchanged).
> references/rename's definition point fallback judgment (start <= col < end) holds isomorphically
> for identifier start point. Red before green: definition middle cursor nail first red (None) then
> green; four handlers middle cursor 1 nail each + cross-file e2e start/middle dual position
> assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2 (three-level
> count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 batch record (registered 2026-10-16 → closed 2026-10-16)**: User 2026-10-16 ruled to merge
> 4.9.1 / 4.9.2 / 4.9.3 three related three-level tasks **into one team, one round** (reason: they
> all belong to the "pool's source and identity" aspect, merged landing can avoid repeated rewriting
> of intermediate state after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong to consumption
> refinement and test pool separation, depend on 4.6.1's `use` path resolution, forcing same batch
> would expand gate surface). #450 attribution **pending ruling**——evidence indicates it's "overload
> resolution names not passed back to reference count" class, still reproduces in round 1 landed
> form, registered as independent WBS entry for handling (team lead recommends A: register as
> independent entry when merging into round 2; **pending user ruling**). Freeze rev = `3863a9ff0`
> (parent commit `62f6e1a59` is pre-fix baseline); verification PASS attempt `aab92fe0` (i.e. t2);
> review pass attempt `96452696` (i.e. t3). **Closure registration commit = `38d30b846` (docs(docs):
> 4.9 reference pool round 1 closure registration)**——this was actually written and pushed by
> closure member integration-refpool; its closing probe once mislabeled closure task as terminal
> failed and overwrote output, team lead took over new attempt, changed back to completed per
> verified delivery fact, hereby correct attribution and label. Evidence archive:
> `target/yx-scratch/yx49/EVIDENCE.md` (containing 8 mutation patches and warn_compare_v8.json).
>
> **I. Merge ruling**
>
> 1. **Ruling itself**: three related three-level tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round**——former depends on 4.6.1's `use` path
>    resolution (already landed at `39546020`, can start); latter with RFC-029f table consolidation
>    same batch, registered RFC-029f end section awaiting change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are preconditions for each other (4.9.2 changes pool type
>    → 4.9.1 assembly surface needs rewrite → 4.9.3 has denominator judgment role table), splitting
>    into rounds must write one intermediate state then change twice; (b) all three belong to
>    "pool's source and identity" aspect, 4.9.4 / 4.9.5 belong to another aspect, batch division is
>    done by "cohesion" rather than "independently revertable".
> 3. **Merge result**: single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: before push `git pull --rebase` merged in
>    auto-translate bot commit (`72f364169`); rebase landing point after the other party's commit,
>    this batch's two (implementation `3863a9ff0`, its pre-fix baseline兼 other party doc commit
>    `62f6e1a59`) **not rewritten, original hash preserved**——initial version mistakenly wrote here
>    "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "after rewrite happens to have same
>    value", actually "not rewritten"), corrected 2026-10-17 at closure; other party code commit
>    `33f14e8ab`→`b122b7ad1`. **External references all use post-push hash as standard** (rewritten
>    old hash unreachable in new clone, identity judged by patch blob comparison).
>
> **II. Five criteria red/green measurement scope**
>
> | #   | Criterion                                              | Red (before fix / P1 directory scan regression patch)                                                                                                      | Green (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                       |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool's root = package target (#334)                    | Directory recursive scan ⇒ tests/ files' identifiers also enter pool, production code unused definitions exempted                                          | `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` per `ExplicitSurfaces` root; full root degenerates to fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                            |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembly unique entry; `arms.rs:603-609`'s `other_module_refs` unions "all modules except this unit" entries and sends to `DeadCodeAnalyzer`; module's own name (top-level definition and local binding, `use` import name excepted) no longer exempts other modules' same-name definitions |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by other project-internal files' references                                                                                                | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test goes to `_ => {}` default arm (judged by file-internal definition-use graph)                                                                                                                                                                       |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Before fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                        | Red state measurement (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Before fix: reports `W1003@1`                                                                                                                              | Same as above                                                                                                                                                                                                                                                                                                                                                                                |
>
> **Criterion ② "deduced red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` in helper.yx; old pool under bare name union treated `foo` as "used by helper" as a
> whole, lib.yx's `foo` exempted. **Measured**: red2 fixture vs pre-fix binary → no W1001 (old scope
> exemption); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red state fixture **first version failed**: first
> version fixture in same file put same-name local variable triggering E2010 (this language doesn't
> allow same-module shadowing top-level definition), moved to another module (helper.yx) for deduced
> red state. This point t3 review and t2 verification both independently reproduced, **not contract
> defect**——same-module shadowing is this language's hard constraint, fixture must cross module.
>
> **III. +8 and +16 two count sets' correspondence** (**t3 review verified as authoritative**)
>
> `target/yx-scratch/yx49/EVIDENCE.md` section C writes `+16/−0`, but missed "**new fixture
> records**" component——v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff,
> `only_before=0`) scope **includes** new fixtures; t1 implementer's self-built "+8" is
> **warn-before / warn-after (325 each) aggregated by fixture** (each member file checked once).
> Review item-by-item mapping:
>
> - **+8** (8 new warnings, all `check <non-entry-file>`, `check <entry-file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: these 8 fixture items all **did not declare package target**
>     ⇒ 4.9.1 root degenerates to checked file's own compilation set, consumer main.yx not in it;
>     this is design-intended change (before fix these files' check surface originally had zero
>     warnings, not exemption path). Repo root `yaoxiang.toml` is empty file ⇒ same degenerate path
>     verified.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **doesn't exist**——are new fixtures added in this batch). So +8's "+ 0 − 0" differential
>     doesn't have them, +16's v8 symmetric differential has them.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` left: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture checks `probe` once per file), **cancels out** leaving only
>     lib's 1——three data sets (+8 / +16 / dual binary ABBA) mutually consistent.
> - **8 new fixtures** vs **+16 residual 8**: same batch of files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats each of their checks
>   as independent record——same file may trigger different diagnostic entries for multiple
>   `role / context`.
>
> **Conclusion**: **+16 = +8 (by fixture aggregation) ∪ new fixtures 8**——both counts are correct,
> **scope different**: t1 reported "8 new" is fixture-level scope, t2 reported "+16" is file-level
> scope (symmetric diff includes new fixtures). t3 review ① +8⊆+16 verified; **review ② writes
> "don't copy EVIDENCE.md section C"**——section C missed "new fixture records" component, this entry
> rewritten per t3 review `pass` conclusion.
>
> **IV. Mutation experiments (mechanically reversible, all `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b use import name
> removal, p3b script joins pool, p3 flat consumption, p4 std joins pool, p7final self-built 4
> criteria, p8 std close, p9 vendor joins pool + p_stack2 / p_stack3 overlay). Each trial output
> contains: HEAD blob → mutated blob → restored blob consistent with HEAD = yes + `git status`
> CLEAN. Local `core.autocrlf=true` needs `GIT_CONFIG_*` injection of `false` (`trial_env.ps1`),
> otherwise patch new lines converted to CRLF causing apply/restore double failure——**only affects
> patch mechanical property, doesn't change tested behavior**.
>
> **V. Process events / environment lessons (must read for handoff to next round)**
>
> 1. **core.autocrlf injection**: local `core.autocrlf=true` must inject `false` via `GIT_CONFIG_*`
>    to ensure patches are mechanically reversible; see `target/yx-scratch/yx49/trial_env.ps1`.
>    **Next round don't reuse** any trial script without injection.
> 2. **Same-tree trial serial**: `trial` and `cargo build` on the same tree **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch worktree, parallel will
>    see "patch not finished when build sees" or "build and apply deadlock" two kinds of races.
> 3. **trial rebuild binary needs sync confirmation**: after `trial` completes, it rebuilds
>    tree-internal binary (cargo's fingerprint contains source path, **change tree = full
>    rebuild**), before subsequent measurement must confirm `bin/yaoxiang-rs` synced with HEAD.
>    **This round once measured false data with residual mutation binary**, rebuilt and re-measured
>    to correct.
> 4. **Second tree yx49-b's creation reason**: dual comparison needs pre-fix binary (62f6e1a59),
>    none available within batch; `scratch.py init yx49-b` opened second tree (eat another full
>    build). **Tree released** (see VI), deletion leaves batch closure `gc` handling.
> 5. **cli_e2e two Internal cases fixtures supplemented to legal package (added `[run].main`,
>    assertion unchanged) reason**: before fix, in `cli_e2e` two `Internal` role nail fixtures **did
>    not declare `[run].main`** ⇒ after 4.9.1 landing, `compile_project` takes `ExplicitSurfaces`
>    root path, can't find `main` ⇒ whole batch 2 fixtures behavior drifted from "fallback per
>    Script" to "can't find entry"——is 4.9.1-introduced fallback behavior, not defect. After adding
>    `[run].main` field, two fixtures take normal package path, **assertions** (exit code /
>    diagnostic code / warning set) verbatim unchanged.
> 6. **Second round tree deletion safe way**: after release, use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" before
>    deletion. This batch released before closure, next step `gc` natural cleanup.
>
> **VI. Batch tree status**
>
> Before closure: yx49 released (4.8 GB released), yx49-b released. Both trees no mount, status can
> be rechecked in `scratch.py status`. This task output ends with `scratch.py gc` complete status.
>
> **VII. RFC-029f "Known Defects" table flip**
>
> Previous version (2026-10-09) described in present tense "reference pool's three design defects"
> (including `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396` etc.
> deleted code locations). This batch closure synchronously rewritten as "**Known defect: Landed
> (2026-10-16, WBS 4.9 round 1)**": ① current pool's root = `collect_package_refs`
> (`helpers.rs:481`), parsed per `ExplicitSurfaces`, not directory scan; ② current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly unique entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumption end `arms.rs:603-609`; ③ current Script doesn't join pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-fetched on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` were moved into
> `src/driver/{state,arms,helpers}.rs` with P4 4.2.9 split (`c59fb751`), old locations in table no
> longer match——this is "declaration and fact mismatch" fix (t3 review note ①).
>
> **VIII. Final gate (closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 rows** (two new fixture
> behavior runs, 338a339,340; another 342 rows byte-by-byte unchanged) / IR snapshot 204 entries
> zero drift / check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process
> event**: this batch one cargo build due to `core.autocrlf` converting script line endings to CRLF,
> team scratch path `yaoxiang-rs` launch reported `BadImage`, empirically encoding inconsistent with
> git index——passed after rebuild (parallel with 4.6 batch "external whole-tree discard" lesson,
> different reason).
>
> **IX. Residuals (handoff to next round)**
>
> 1. **#450 attribution ruling**: evidence indicates "overload resolution names not passed back to
>    reference count" class, **still reproduces in round 1 landed form**——per user ruling C process
>    to assign. **#450 probe**: dual binary same set `{W1002@2, W1005@4}` (measured, rootless form),
>    not directly related to round 1 change, but #450's fix may need to wait for 4.9.4 (consumption
>    refinement) to do together——because root cause is "overload resolution names pass reference
>    count" needs use item's precise consumption surface, 4.9.4's "real use after imported" is
>    same-class criterion's precise version.
> 2. **dead_code.rs:683 pointer nit**: comment writes "two pool assembly scope see driver side
>    collect_compilation_refs"——actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment and code main path scope don't match (**not
>    in this batch's writing scope**), left to 4.9.4 / 4.9.5 landing to correct together.
> 3. **4.9.4 / 4.9.5 preconditions unlocked**: 4.6.1 (`use` path resolution) already delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to first decide
>    RFC-029f table consolidation (already paved by this batch "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user ruling C deferred to after 4.9)——this round 1 not involved; next round 4.9.4 /
>    4.9.5 landing to handle in same batch.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 round 2 batch record (implemented 2026-10-16 → closed 2026-10-17)**: Covers 4.9.4
> (consumption criterion = real use after imported), 4.9.5 (test pool separation), 4.9.6 (method
> reachability scope, issue #450, user 2026-10-16 ruling A merged into round 2). Closure fix
> (verification-side remediation M1/M2/M4 compilable redo + F1 fix round + PA-2 delete dead branch)
> and registration gap issue #464 (WBS 4.9.7) settled together.
>
> **I. Judgment chain and commits (always post-push truth value)**
>
> | Step | Content                                                                                                                                            | Post-push commit | Pre-push local value | Judgment method                                                    |
> | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- | ------------------------------------------------------------------ |
> | 1    | fix(util): check explicit path not intercepted by default exclusion table + --exclude always empty (issue #463, P0-0.4)                            | 366bc5a69        | ef959ca55            | Patch blob identical                                               |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool attribution by import + Test doesn't contribute consumption surface)                                              | e6cf1aeaa        | 84a6f6978            | Patch blob identical                                               |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; including generated warning-codes.md)                                                     | ddcbb12d0        | cd629b5fe            | Patch blob identical                                               |
> | 4    | docs(meta): AGENTS item 10 ⑦ (commit-time stash window convention)                                                                                 | ec94fa9d8        | 9a4ed5f3b            | Patch blob identical                                               |
> | 5    | Fix round = F1 (overload sibling overload recovery report W1005) + F2 (verification-side M1/M2/M4 compilable redo) + F4 (snapshot generation note) | b8380e6ab        | 3d9ef6eab            | Patch blob identical (36962f318e42…, git range-diff no difference) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three false statements correction, register #464)                        | b940300ac        | e13984465            | Patch blob identical (patch-id 9bda3cfe4c…)                        |
>
> **Hash explanation**: Each pre-push git pull --rebase will rewrite local hash; "judgment method"
> column in table above is all "patch blob identical"
> (`git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent + `git range-diff` no
> difference). This article doesn't reference any pre-push hash as authoritative source.
>
> **II. Three criteria red/green measurement scope** (verifier self-built reproduction, doesn't
> trust implementer's criterion)
>
> | Criterion                  | Red (before binary 366bc5a69 measured)                                                                                                                                                    | Green (ec94fa9d8 and after)                                                                                                   |
> | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Project check a.yx from declaration surface only 1 W1001 (main's use b.{foo} + foo() rescues a.yx's same-name foo; guards only imported not used also rescued)                            | Same fixture 2 reported truthfully (collision doesn't rescue + unused import doesn't count as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print rescued by tests/ reference (only 1)                                                                                                                            | Same fixture 2 (test reference doesn't overflow); ref_pool's test_ref_pool_excludes_test_role_references passed               |
> | 4.9.6 method reachability  | Self-built fixture (2 groups ambiguous overload calls + 1 uncalled) 3 W1005 false positives; method_call_args.yx 3 / method_overload.yx 1 (overload by full name coverage only reports 1) | Same fixture just reports uncalled orphan; method_unused 3 nails passed; corpus call_args 3→0 / overload 1→0                  |
>
> **Fix round (F1) further tightening**: Non-hit resolution's sibling overload restored reporting
> W1005 (b8380e6ab——same fixture call ->Float reports ->String sibling, call ->String reports
> ->Float sibling, bidirectional line nail). **Fallback boundary (registered #464)**: Cross-unit
> (registry resolution) method call doesn't produce resolution entry, falls back to name-level
> exemption, cross-unit same-name same-order previously not reported——registered 4.9.7.
>
> **III. Process events and lessons (four hard requirements correspondence)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: Round 2 review once
>    registered "cross-unit same-name same-order not reported" as blocker (PA-1); then two fact
>    reconciliations (implementer archive + verifier on b8380e6ab/b940300ac dual binary re-test same
>    fixture set, diagnostic set byte-by-byte consistent) confirmed it's **capability gap existing
>    before round 2** not this round's regression——mechanism = cross-unit method call doesn't
>    produce resolution entry, falls back to name-level exemption; disposition = change to medium,
>    register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer publicly corrects before data and withdraws regression ruling**: Reviewer's
>    initially cited before data ("−42 includes called methods") doesn't match their original output
>    (W1005 count=0)——verifier confirmed before capture is 0, reviewer's regression ruling
>    **publicly withdrawn**; −42's correct scope = 26 files' called methods false positive
>    elimination (phase B), traceable to behavior-run exit=0 files (methods really called executed).
> 3. **t13 terminated due to contract premise not holding (not implementation defect)**: That task's
>    acceptance premise wrote "cross-unit same-name same-order reports W1005 regression fix", but
>    measured (t14 dual binary) confirmed this behavior is already-registered gap (#464) not
>    regression——contract premise doesn't exist, task terminated; its probe value taken over by t16
>    read-only judgment (confirming `strip_unit_prefix` is dead branch).
> 4. **Doc commit's real criterion is "really build once"**: Round 2 closure's doc commit
>    (`042c07860`) made Deploy Docs red on VitePress's Vue template compilation——placeholders like
>    `<rev>` / `<before-exe>` / `<after-exe>` in the record weren't in backticks, treated as
>    elements (reported `Element is missing end tag`, pointing at the `<rev>` line); markdownlint
>    can't catch this kind, and pre-commit's markdownlint hook in doc-only commit is judged
>    `no files to check` and skipped (measured). Disposition: placeholders all in backticks, before
>    push locally run `npm run docs:build` (fix commit `0c837e1b4` evidenced by local build exit=0);
>    if using `--no-verify` commit, commit-msg verification needs manual supplementary run.
> 5. **Process omission of unsynced contract description**: t14's task description text had residual
>    PA-1 old hypothesis (wrote expectation "b.yx reports W1005"), verifier in report proactively
>    pointed out "judging by contract field, description text not counted" and asked team lead to
>    confirm——disposition = task contract field as judgment basis, description text as process
>    lesson (**task description must sync with latest judgment chain when re-assigning/assigning**).
>
> **IV. Mutation experiments (mechanically reversible, all scratch.py trial)**
>
> Round 2 verifier 9 patches total (target/yx-scratch/yx49/ under m1b/m2b/m4b as F2 post-compilable
> re-do version——initial M1/M2/M4 commented out if condition line leaving dangling block, was ruled
> invalid by t8 review F2 as "compile failure mis-recorded as criterion flipped red"; re-do version
> passed per-patch cargo check exit=0 + trial flip red). M3 (collision attribution regression)
> initial version was real mutation, measured ref_pool 5 criteria flipped red, preserved original
> judgment. M1b anchor re-fetched on b8380e6ab (F1 post-fix probe segment refactored, old anchor
> invalid). All trials: HEAD blob → mutated blob → restored blob consistent with HEAD + git status
> CLEAN. Environment injection reused trial_env.ps1 (core.autocrlf=false via GIT_CONFIG_*).
>
> **V. Warning surface full dual comparison (scope first explained)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) four-tuple
> symmetric diff, project mode channel cwd=file parent dir, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Scope**: this round's before capture already includes all fixtures ⇒ matched set =
> full 337, no round 1's +8 (matched increment) / +16 (full set) dual scope problem. Results:
> only_before = 42 W1005 (26 files, all called methods' false positive elimination = phase B's −42;
> per-file verified all on behavior-run exit=0 baseline, methods really called executed——**no real
> dead code rescued**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface ×2 +
> ref-pool-test-pool's ../src/util.yx:2 = phase A criterion surface). **Snapshot generation (F4)**:
> Phase A time point's after capture (phase B pre 57 residual) and final version (phase B post
> 15+2=17) are **two generations**, must note when referencing; also 1 exception
> (interface_member_conflict.yx's Dog.name@18) is field access form, not in phase B's W1005 removal
> set, before/after consistent.
>
> **VI. Final gate (closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator ran no product changes.
> **Gate check arrangement (item 13)**: That batch's pushed CI / Deploy Docs / Auto Translate run
> conclusions registered by scheduled check (run id + conclusion + consecutive push coverage
> relationship: subsequent run covers previous), not waiting within session.
>
> **VII. Batch tree status**
>
> yx49 and yx49-b both released (deleted by gc at closure); no new directories outside repo from
> this batch.
