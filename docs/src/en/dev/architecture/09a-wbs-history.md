# WBS Historical Notes (Implementation Record · Audit · Decision Trail)

> This document is a **historical note** for [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task table): implementation records, audit findings, decision trails, and errata for
> each phase. **Division of work**: 09 describes only the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document records only the **history** (what was done, what
> was found, how it was decided), appended chronologically, never rewriting history.

## P2 Establish Equivalence Criterion Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green.
> **D38 prophecy fulfilled** — the green-running process actually measured and fixed three real
> defects in ir_gen (see 06 §G6): method locals table truncated to parameter count (local_count
> underestimated, runtime tolerates via Frame::set_slot), unannotated parameters dropped by
> filter_map (three places, params metadata misaligned with frame layout), match result slot relied
> on frame's implicit Void initialization (now explicitly pre-initialized). The actual semantic of
> jump target is flattened instruction index (not label-keyed as in 07 text), CFG built at
> instruction level — recorded in verify.rs file header. **Commit correspondence**: ir_gen fix =
> `a9ea199b`; verify main body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its
> message to be mislabeled as "docs(rfc): RFC-027a…", content is actually verify.rs + six test
> files, buried in history middle unamendable, hereby recorded).
>
> **P2 Narrowed-scope Implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> plug-channel prerequisites unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract shares the same `TestFileSpec` as single-file
>   corpus (single point on library side), binary location / subprocess launch abstracted to
>   `tests/common/` with yx_runner migrated in sync (Constraint 3).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source adopts the rfc027
>   test-pinned `SumUpTo(3, r)` form (return position / call point / binding position) — the
>   `Sorted(3)` annotation example in 02 would be substituted and evaluated, going directly to
>   Disproved under current code, not silently showing Unproven; the `Sorted(3)` in 07's pseudo-code
>   is illustrative only. Red-state measured data in test file header.
> - The two red criteria are pinned with `#[ignore]` constant per the 2.4.5 red-skeleton precedent:
>   default suite fully green, `-- --ignored` reproduces red state; **turn green immediately upon
>   removal when P3 fix lands**.
> - The rationale for the deliberate deviation from 07's (code, file, line) table in comparison
>   normalization (removing file/line, comparing only error code sets) is written in the 2.4.1 test
>   comment.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, P2
> cannot compile. The former becomes P4's 4.1.4 (no double registration), the latter's `#[ignore]`
> red skeleton merges into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: C4 must-cover checklist item-by-item verification (07
> §143), all 10 semantic corpus layers fully covered and all within the corpus-parity.jsonl diff
> baseline (26 representative files hit 26/26, grep verified) — ref/borrow/move entire 05-ownership
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for and 16 files total, sum type
> sum_type__+ pattern_or_guard + match_call_scrutinee, existential enforcement point
> interface_dynamic_dispatch (Vec(Animal) heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err on diagnostic
> diff), refinement constraint Drop sequence refined__ ×5 + 05-ownership driven ReleasePlan (span
> key contract has separate 2.4.4 red criterion constant). Performance baseline
> `benches/pipeline.rs` committed (07 §145 three smoke tests, two): `corpus_compile_frontend` first
> pass of full corpus compilation 17.6s, `cli_cold_start_hello` cold process startup 169.7ms;
> interpretation throughput already covered by benches/lib.rs hotpath group (interp_fib_recursive_27
> / interp_loop_steady_10m), not duplicated. Regression gate check-perf-regression.sh assigned to P9
> unified checklist (2.5).

## P3 Correctness Bug Fixes (Minimum Solution)

> **Error type annotation for completed rows** (retrospective annotation, no history modification):
> 3.1 = B ｜ 3.2 = B (defensive empty branch, criterion turn-green borne by 3.1) ｜ 3.3 = A ｜ 3.4.1
> / 3.4.2 = A (new W codes) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D
> (user-visible surface, already user-decided) ｜ 3.4.8 ①② = C (main) + A (incidental three
> places) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-decided).
>
> **P3 Diagnosis (2026-10-12, user request)**: A diagnostic accounting to address the perception
> that "P3 items keep growing".
>
> **I. First, clarify scale (perception amplified by two accounting defects)**
>
> 1. **Numbering collision**: `3.5.x` is shared by two task families — (a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level: lexical and parser branches / AST `is_pub` / checker
>    side / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10 registered **3.5 Higher-Kinded Type
>    parameter call path completion** (3 three-level). Both share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 renumbered per user decision: 029g family changed to `029g.x`**), and
>    caused 029g's rows to be miscounted as P3. **Pending decision**: Rename 029g family to `029g.x`
>    (or move to its owning phase), synchronize all references in the document.
> 2. **Missed checkmarks**: `3.7`'s 4 three-level (3.7.1–3.7.4) are all completed (d922a551 /
>    38bd4b9e) but not checked ⇒ this item corrected in this pass.
>
> **II. P3 Actual Accounting (12 secondary rows)**
>
> | Row                                                                   | Status                                                                                                                                                                             | Disposition suggestion                                                                                                          |
> | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                       | Completed                                                                                                                                                                          | —                                                                                                                               |
> | 3.4 Silent discard points supplementation (8 three-level)             | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **now doable**; 3.4.4/3.4.5 unlocks with 4.3.1                                                                            |
> | 3.5 Higher-Kinded Type parameter call path completion (3 three-level) | Not started (declaration side live, call side broken)                                                                                                                              | Retain in P3; **does not block refactor main line**, but is "accepted feature unimplemented", priority above pure cleanup items |
> | 3.6 Function type value call path (1 three-level)                     | Not started (semantics already cut (i))                                                                                                                                            | **Retain**: correctness defect, criterion clear                                                                                 |
> | 3.7 init segment call name qualification (4 three-level)              | **All completed**                                                                                                                                                                  | This pass checks and closes                                                                                                     |
> | 3.8 native shadowing (1 three-level)                                  | Not started                                                                                                                                                                        | **Stuck on decision** (first define name resolution priority), not a scheduling issue                                           |
> | 3.9 / 3.10 / 3.11                                                     | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                               |
> | 3.12 Runtime-mode executor config propagation (1 three-level)         | Not started (D class, user decision ②: separate team)                                                                                                                              | **Suggest moving out of P3 → P8 (execution layer)**, pending decision                                                           |
>
> **IV. Leverage of 4.3.1 (main line marker)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the prerequisite for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ take it once to push the main line and clear two P3 stranded items at the same
> time. 3.5 (HKT), 3.6 (function type value call), and 4.6.1/4.9.1 all have **no unmet
> prerequisites**, are doable anytime; 3.8 is stuck on semantic decision (user definition priority
> vs. explicit diagnostic).
>
> **III. Mechanism suggestion (anti-recurrence)**: Newly registered secondary/tertiary rows **must
> carry two fields** — "owning phase" (which P it really belongs to) and "whether it blocks the main
> line"; `registered awaiting scheduling` no longer serves as default state, unmarked items are
> deemed unschedulable. This way expansion is visible and sortable, rather than indiscriminate
> accumulation. **Pending decision**: Whether to write this item into this document's registration
> convention and `src/dev/coding-rules.md`.
>
> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + single getter
> (recorded at the time as `pub(crate)`; **2026-10-07 verified actual implementation is
> `pub(super)`** — the `#434` decision referenced in code comments has no docs-side registration,
> see 3.4.3; fully private requires rewriting 11 construction literals, not taken in plug phase;
> closed at P4 Obligations migration); execution mechanism extracted to
> `src/frontend/proof_execution.rs` (Constraint 3 single point — cannot go in typecheck/, that would
> create L3→L4 reverse dependency); 3.1.2 per 02 evidence chain step 5, added **four entry points**
> (WBS row missed compile_project `:99`, must be added and registered per 3.1.1 acceptance
> "criterion turn-green"); 3.2.1 empty branch changed to accounting (proof_calls throws up) +
> diagnostic (`into_result` established conversion path); 3.3.1 SOLVER slot Option-ified, Z3 missing
> falls back conservatively per `SMTResult::Unknown` (E2031 family diagnostic replaces process
> crash). Two bug criteria removed `#[ignore]` and turned normal green, corpus fixture
> `proof-obligation-honored` added; check-fanout inventory migrated (wiring point 19 = inventory).
> Measurement: criteria 2.4.1/2.4.3 turned green, full suite lib 2494 / integration 272 / corpus
> two-layer all green.
>
> **3.4 Audit (2026-10-07, P3 post-implementation review)**: For the "failure produces no signal"
> category of defects (definition see 02 §41-52), an in-place re-inventory without code changes
> located 8 more places, all registered as the 3.4.x above. Method and evidence:
>
> - **Consumer-side inventory** (grep all-repo read points for 16 obligation fields field by field):
>   `variant_ctor_calls` **zero consumers** — the only read `checker.rs:1468` is "pumping into
>   `TypeCheckResult` itself", IR side goes through shape detection `ir_gen.rs:7374`;
>   `rfc011b.rs:659/688` only asserts production side, exactly the "only tests production side,
>   naturally immune to consumer-side defects" form criticized in 02 §186 → 3.4.4.
>   `method_overload_ir_names` result-level field also has zero external consumers, IR uses parallel
>   `overload_resolutions` → 3.4.5.
> - **Phase coverage review**: 02 §91 table row 1 (four entry points "no proof_execution") already
>   fixed by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 unchanged (multi-file `run` still doesn't
>   output W1003, still doesn't run dead code family) → 3.4.6; LSP non-in-project path (02 §77
>   "manual lex→parse") likewise doesn't run proof → 3.4.7.
> - **Same-form empty branch scan** (`Unproven` all-repo consumers only four places): checker three
>   refinement branches (3.1 covered), the 3.2.1 fixed ownership branch (**source unreachable**),
>   and `environment.rs:422` (**reachable, zero diagnostic, zero accounting, and all-repo docs zero
>   registration**) → 3.4.1, the only "all-unknown" live defect in this audit.
> - **Solver acquisition surface**: `default_solver()` three consumer forms vary (`predicate.rs:32`
>   process singleton / `checker.rs:1266` injected per `check_module` / `ownership.rs:621` newly
>   created per back-edge decision) — 3.3.1 only eliminated the panic; the **silent surface** of the
>   other two goes to 3.4.2, **singleton-ization and cache hits** go to P4's 4.4.2 (no double
>   registration).
> - **Existing ownership not counted in 3.4**: 2.4.4 (`ReleasePlan` span key mismatch, 168 files /
>   373 keys not consumed, Drop silently lost) remains P2 red-state constant, ownership 4.5.1/D20
>   unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not in
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4 register.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites, can land in place
> anytime (recommend 3.4.1 first — the only "all-repo zero registered" live defect, smallest change
> surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on obligation ledger (4.3.1) or
> unified Driver (4.2.x). **3.4 does not establish new criterion system**, reuses existing C2/C3 and
> P9's 9.2 gate.
>
> **Batch 3.13–3.16 Implementation Record (registered 2026-10-13 → closed 2026-10-15)**: User
> decided 2026-10-14 to merge four correlation small items (#451 method binding panic / #452 named
> argument into resolution / #453 dead inference entry / #454 native export named argument — the
> last is a boundary discovery while verifying 3.14, same-day attribution experiment verified **not
> introduced by #452**, user decision ① merges into this batch) for one team to handle. Closing
> status: all four items completed, decision registered as RFC-039 **D60** (six binding rules +
> shared criterion single-point).
>
> **I. Two contract corrections (lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradictory**: #452 task contract simultaneously required "mixed
>    argument output 20" and "position and naming fight for same position errors", but pinned
>    fixture `Temp.pick(self, a)` has only one value formal parameter — the same argument
>    combination is both "mixed" and "conflicting". First implementer honestly reported blocker and
>    set task failed, overall revert (no silent green). Disposition: task reassigned + contract
>    revision — after giving **six binding rules** and **8-row value matrix** (fixture changed to
>    two value formals `pick(self, n, a)`) attempt 2 was completed by second implementer. Full text
>    of six rules word-for-word recorded in D60.
> 2. **inScope missed mechanically-required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only ran `code-tables --fix`,
>    **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964** — the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **II. Review → Fix → Re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls with named arguments silently pass through both zero-hit
>    and single-hit paths of multi-candidates) → fix round added new external code **E1111
>    (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece suite in same
>    commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm settled by name) → independent
>    verification PASS (value matrix 12 cells, corpus 342/342, 5 mutations mechanically reversible
>    replayed all caught, restore byte-for-byte identical) → round-1 review needs_revision (F1 =
>    typecheck native arm **inlined copy** of binding criterion, missing rule ④ ⇒ **divergent
>    decisions** on same-form method and native calls — method reports E1111, native silently exits
>    with exit=0; ir_gen has a third stock copy reporting old code E1015) → convergence round
>    consolidated criterion to single implementation `bind_args_to_params` (`arg_binding.rs:35`,
>    four call points `expressions.rs:2727` / `:5416`, `ir_gen.rs:7776` / `:8279`) → round-2 review
>    caught a **regression** introduced by convergence round (IR convergence block lost native
>    omittable position ⇒ previously successful programs with optional positions omitted wrongly
>    reported E1010) → fix round (optional position criterion and formal name from same source
>    `signature.rs:236` single exit) → round-3 review PASS (independently reproduced red→green;
>    mutation under pinned board FAILED empirically criterion can fail). **Lesson**: review must
>    cover "unfixed branches of similar paths" (both needs_revision in this batch were probes
>    falling entirely on hit surface); the fix round itself can introduce regressions (optional
>    position), caught by next round of different reviewer.
>
> **III. Process events (recorded as-is, not pointing at individuals)**
>
> 1. **External whole-tree discard (around 17:14 2026-10-14)**: work tree was externally restored
>    silently — implementer's fix WIP (typecheck/ir_gen/code table/locales/RFC-013 etc.) all
>    disappeared, **at the same moment two review documents edited by the user themselves also
>    disappeared from git status** ⇒ determined to be repository-level discard (most likely
>    user-side manual cleanup / IDE "discard all changes"). Three pieces of evidence excluded group
>    members: stash empty, no new commits, reflog no reset. Implementer replayed WIP per context and
>    pushed; discipline changed to "**save patch to scratch first, then sample red state**".
> 2. **Verifier context exhausted, changed personnel**: independent verifier verifying 3.16 ran out
>    of room midway (no closing message), changed personnel per load rule — successor continued
>    using already-ready outputs (green-state result, corpus equivalence script, mutation plan),
>    verification finally PASS. Positive precedent for partial output handover.
> 3. **Round 5 review rotation and historical round retraction**: when rotating the ⑤th reviewer per
>    load cap, `remove_member` also retracted **already-decided terminal review/verification
>    rounds** into the shared pool, the scheduler immediately re-ran them (t3/t8 were re-picked by
>    new members, duplicated work and blocked downstream). Disposition: lead took over and cancelled
>    retracted historical rounds (t3/t8/t13), based on which AGENTS.md rule 12 added (handle
>    retracted rounds immediately after personnel change). This batch's closing record registers
>    this event per that rule.
> 4. **scratch governance (~6.3 GB)**: early batch had one external tree per task, 6 directories
>    scattered outside repo (~780 MB, cleaned) + two members' external verification trees (~2.2 GB /
>    3.3 GB, self-deleted after use). Produced governance tool `scripts/dev/scratch.py`
>    (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md rule 9 (batch tree single entry),
>    rule 10 (verification environment six points: mechanically reversible mutations / line numbers
>    from cited revision / CI-only gates must run etc.), rule 11 (writer load cap), rule 12 (handle
>    retracted rounds after personnel change) plus two top-level meta rules (no private content /
>    verify before asserting).
> 5. **Contract defect (not pointing at members)**: #454 fix round's contract inScope only covered 2
>    files, actual mechanically-required surface 8 (item 6 is designed to prevent this — contract
>    template should pre-set mechanically-required surface list); also that task's terminal field
>    was once occupied by probe payload, formal acceptance/command evidence was backfilled by lead
>    per convention.
>
> **IV. Remaining items (not fixed this batch, registered for tracking)**: round-3 review three
> low-severity findings — ① method path's `build_arg_slots` (`ir_gen.rs:521`) remains a **second
> copy** of the criterion (stock, registered per last round's convention, not converged) —
> **2026-10-16 pre-release audit added three points**: (a) it has **two** call points:
> `ir_gen.rs:7888` (method binding path, uses bound function `binding.function`'s formal name) and
> `:7975` (overload resolution path, uses resolved hybrid name leaf segment), original registration
> only wrote "around `ir_gen.rs:521`"; (b) it has **already diverged from the shared implementation
> `bind_args_to_params`**: missing rule ⑤ (duplicate naming → E1015), rule ⑥ (missing required
> formal → E1010) and the "optional position" concept, and hardcodes the receiver position as
> `1 + i`; both call points `continue` on empty slots uniformly (comment says "diagnostic belongs to
> typecheck (it guarantees no empty slots if resolution passes)"), is a **dangling invariant written
> in comment with no assertion backstop**; (c) **RFC-039 D60's "all call points converge on it"
> enumeration missed these two places**, declaration inconsistent with fact, and the cited line
> numbers were not re-fetched on the cited revision, D60 text corrected together with this item; ②
> batch verification tree (worktree) `cargo fmt --all` error is a worktree workspace artifact, gate
> checklist should note fmt runs in main repo; ③ repair-type contract templates should pre-set
> mechanically-required surface into inScope (see III.5).
>
> **V. Final gate (closing revision, local two implementation commits + one documentation commit
> pushed together — the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 /
> corpus_probe **342 rows byte-for-byte identical to baseline** (only regenerated row:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number synchronized,
> manually reviewed) / IR snapshot zero drift.
>
> **VI. Run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test cancelled by subsequent push
> per concurrency — cancellation ≠ failure, covered by later run); governance tool push → CI
> 38546373435 / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) →
> CI 38528560364. Closing push's own CI / Deploy Docs run logged into task record after push.
>
> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail added `Linking` (**pure
> merge**, entirely skipping `is_bin_role` entry validation — E3020 criterion still belongs to
> `RoleClassification` to prevent double reporting; additional hard reason: run path determines "has
> main" by merged IR, under collection semantics "entry itself errors → no IR" would be misjudged as
> missing main and hard-aborted) → `Monomorphization` (check shape); `ir_generation_check` retains
> unit IR (**in-memory only**, for pure merge and mono consumption, not externalized / not written
> to disk / not producing snapshot — without retention aggregation request is empty, mono
> false-greens with `NoObligations`); mono failure goes through per-file collection channel in Check
> (diagnostic attributed to entry file + `failed_stages`, not set to failure), MultiFile keeps
> `fail_with`; `into_outcome` changed to "first take collection then judge failure". ④ empirically
> **zero production change**: full-program BFS already covers cross-unit mutual recursion
> (pathological → E3005, well-formed three-layer chain executes correct values), delivers four e2e +
> one mono unit test as living criterion.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized boundary),
> t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-shape matrix
> (single/multi/check three shapes + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increased, not decreased).
>
> **F1 (found during verification, D59 A/B class, same-round fix)**: in rootless standalone
> multi-file check without project root, aggregation request name (`lib.f`) and unit IR definition
> key (`f`) are not from same source → mono falls into "native has no IR definition" silent
> `continue`, check reports 0 error/exit 0. Fix: aggregation side added lookup jump by each unit's
> **true definition key** (precise enumeration + byte-equal + ambiguity abandon, not suffix
> heuristic; project shape not registered, existing paths unchanged). **Errata (same day)**: the
> lookup jump was **deleted** in the subsequent regression fix; root cause instead solved from
> naming dimension by `module_key` dual-use decoupling (added `script_mode`) — IR definition key =
> IR call name = aggregation request name = `{unit_key}.{name}`. **Regression record**: after ③
> landed, CI's `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6 items
> same-file same-name rebinding + 4 items colliding with `<std/list>`, rootless check doesn't do
> qualification triggering `link_module_irs`'s conflict validation); fix = ir_gen split
> `script_mode` (qualified name ⊥ Script/Bin, E3023 gate changed to look at it) +
> `ir_generation_check` always passes unit key per unit + `.with_script_mode(rootless)` + Script
> top-level same-name rebinding "latter wins" (consistent with run path existing behavior);
> verification = documentation gate 258/0, three-binary comparison (HEAD/③-tree/fix-tree) three
> collision fixtures, check shape E3005 not regressed, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic attributed to entry file item, span can cross units (same
> dimension as run path `path=entry`); Check shape holds one ModuleIR per unit (in-memory only,
> released when check ends).
>
> **Existing issues for follow-up (not this round's regression)**: ① E3005's help text still says
> "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` moved similar "user-written error" out of E3005) — registered awaiting
> scheduling; ② multi-file E3005's span rendering label offset (reproduced with HEAD binary,
> pre-existing); ③ request collection surface has **type parameter position dependence** (generic
> calls with type parameters not in value parameter position don't issue instantiation requests,
> mono records `NoObligations`, converging mutual recursion is exactly this form) — same surface as
> 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Remove `pub` and Auto-Binding (#399

> **P3.5 Implementation (2026-10-08)**: all 7 secondary tasks completed, the whole batch in one
> commit (independently revertable).
>
> - **Measurement inconsistent with plan (recorded as-is)**: 029g.1.2/029g.1.3 acceptance originally
>   wrote "`pub f = ...` reports **parse error**" — actually after removing `KwPub`, `pub` falls
>   back to ordinary identifier, old usage hits semantic layer reporting **E1001 Unknown variable:
>   'pub'** (span points at `pub`). Per RFC-039 **D27** "specially detecting removed syntax is an
>   anti-pattern", **no new** migration-specific diagnostic added; corpus fixture
>   `pub_declaration_err.yx` pins E1001, document (`syntax.md`) honestly writes this consequence.
> - **Design convergence (beyond RFC literal, needs review)**: `project_refs` and `cross_file_refs`
>   two fields after pub removal have semantically identical meaning (same "in-package reference
>   union", Constraint 1 prohibits parallel representation) → merged into single `project_refs`,
>   `set_exempt_pub`/`set_cross_file_refs` two switches removed. Behaviorally, Internal role's
>   non-pub items changed from "in-file reachability" to "reference pool judgment" (relaxed,
>   preferring false-negative direction); Lib tightened from "pub absolute exemption" to reference
>   pool judgment; Script top-level bindings not referenced start reporting W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all removed (production-side
>   pumping, all-repo zero consumption).
> - **Test migration** (test count not decreased): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer modifier"; `passes/tests/dead_code.rs` 7 role semantic test cases rewritten (2 assertion
>   reversals); `tests/integration/cli_e2e.rs` 8 role test cases rewritten (`lib`/`internal`
>   exemption assertions reversed to reporting assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not in IR); corpus diff baseline only added 1
>   entry (`pub_declaration_err.yx` → E1001), command `cargo run --quiet --example corpus_probe`
>   regenerated then manually reviewed.
> - **Measurement**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closing
> (registration / WBS check / gate / push) by t7. Prerequisite = 4.3.1 obligation ledger (red state
> archive 173 files / 533 keys).
>
> **I. Carrier Decision (Option A: `PlanId` follows AST node)**: Added `PlanId`
> (`src/util/plan_id.rs`, `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX`
> represents synthetic nodes outside parser), issued by **sole allocator**
> `ParserState::alloc_plan_id`; carrier two places — `Stmt.plan_id` (statement identity) and
> `Expr::Call.plan_id` (call point identity), `Expr::call` is the only call constructor in the repo.
> The rejected form was "both sides derive same identity by span" position coincidence form (D20's
> original defect).
>
> **II. Two-Level Verification and Review (independent members)**: Each functional commit paired
> with an independent verifier and reviewer, **verifier/reviewer don't reuse implementer's
> evidence**: 4.5.1 = t1 implement / t2 independent verify (external tree + 5 mutations) / t3
> review; 4.5.2 = t4 implement / t5 independent verify (external tree + 5 mutations) / t6 review.
> Review point and reviewed revision both fall on immutable history (`git show` verifiable).
> **Provider renaming caused t5 / t6 personnel change continuation**: original verify/review member
> unavailable due to provider rename, per orchestration convention changed personnel and **record
> reason in writing** (cross-round context retention priority, personnel change only when
> compression unavailable); after continuation conclusion consistent with original round (PASS), did
> not reuse implementer's evidence.
>
> **III. Two defects self-found and fixed during implementation (t1, both within inScope)**: ①
> Consumption exemption criterion was hung on `UnitState.ir.is_some()` — `check` shape's
> Linking/Mono takes `unit.ir` away, real non-consumption in check shape **silently passes**
> (mutation measurement: D41 red but CLI check exit 0); changed to read **the ledger's own
> completion flag**, and added regression test. ② Drop sequence drifts with process hash seed
> (partial order `sort_by` + `HashSet` iteration): snapshot criterion ran three times in a row
> producing 106/120/114 line drift; changed to **total order sorting** (same name takes max depth +
> name ascending) then ran twice with zero drift.
>
> **IV. Criteria and Evidence Numbers**: After 4.5.1, `test_release_plan_spans_consumed` removed
> `#[ignore]` constant (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in same
> commit (programmatic review: added 1083 lines Drop + 86 lines jump index shift, zero deletions, no
> reordering), corpus 342 entries diff empty; 4.5.2 added `test_overload_resolutions_consumed`
> (difference set empty + evidence floor `examined ≥ 1` / `consumed ≥ 2`; measured examined=1,
> consumption items 2, no resolution skipped 210), 9 resolution form pins, `cargo test --lib` 2712
> passed / 0 failed, IR snapshot **zero drift**. Mutation-driven: consumption key shifted one
> position ⇒ criterion red + real program `error [W1090] ... produced key not consumed` + corpus
> gate difference; after restore re-green.
>
> **V. Semantic Correction Classification Basis (D59)**: `Type.method(position arg, name = value)`
> **silently gave wrong answer** before change (measured compile clean, runtime output `1` instead
> of `20`) ⇒ **Class C** (old behavior non-success), fix takes "loud": after change same program
> compile-time `error [W1090]`; terminal semantic of "named argument participates in resolution" is
> **Class D**, registered in §3.14 with issue #452 left for dedicated team. Also: `Drop` is no-op in
> interpreter (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`) — key
> migration doesn't change runtime behavior.
>
> **VI. F-1 / F-2 Disposition (found by t5 independent verification)**: **F-1 (medium)** unbound
> consumption point (`ir_gen.rs:7766-7790`) unreachable under current corpus and test surface,
> keying correctness has no red state falsifiable ⇒ per "known boundary" convention registered into
> [02](02-stage-contract.md) §2 revision note fifth paragraph (preserve reason + don't open new
> task); **F-2 (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` documentation comment
> changed to precise pointers (produced `inference/expressions.rs:4550`, consumed `ir_gen.rs:7697` /
> `:7772`), separate small commit, not mixed into documentation commit.
>
> **VII. Closing (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line numbers (`:360` read / `:3103` use / `:1095` and `:1097` settlement flag);
> 07 criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one line behavior change; `check-obligations.py` and `check_tracking.py` green,
> CI and Deploy Docs green. **Accounting deviation found at closing**: pointers `ir_gen.rs:357` /
> `:3102` / `:1096` in t3 / t6 reports are 1-3 lines off from code fact (actually `:360` / `:3103` /
> `:1095`+`:1097`, due to 4.5.2's insertion) — register per code fact, this note retains the trail.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (run half of 4.2.5 Decision A)

> **Batch C Record (registered 2026-10-15 → closed 2026-10-15)**: User decided 2026-10-15 ③ — this
> round only does 4.10.3 (same-batch candidate 3.4.6 deferred to after 4.9 per B1
> ([02](02-stage-contract.md) §Runtime Behavior Table "dead code family warnings don't enter compile
> path"), prerequisites already re-anchored and annotated); implementation form = **complete single
> team** (user directed no DAG, no team, single-session implementation + self-verification).
> Pre-batch outstanding work landed first: #455 dependency announcement fix (rustls 0.23.46 +
> serde_yaml_ng, `cargo audit` restored from exit 1 to exit 0), WBS registration debt (#456
> registered as 3.17 + incomplete rows added "owning phase / whether blocks main line" fields).
>
> **I. Red before green**: 3 driver-level red criteria (`src/driver/tests/run_rootless.rs` —
> rootless merged IR must not carry entry_function (red state `Some("main.main")`), rootless
> top-level executable statement must not report E3023 (red state produces no module), Bin control
> entry_function maintained (green state constant)) + 3 CLI e2e pins (standalone relative import run
> (red state E5001), Script top-level statement execution and #413 hint two green state pins).
>
> **II. Implementation surface**: CLI `run` uniformly goes through `compile_project` (the
> project/standalone fork in `run_file_with_diagnostics` deleted); Linking's `entry_function` gated
> by `is_bin_role` (#413 T4 — Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call point project probe (after unification ModuleIR always
> carries source_files, original marker invalid); `script_main_never_called` adapted to qualified
> name form (function `{key}.main`, global slot empirically same qualified name — based on IR
> evidence, not comment inference).
>
> **III. Existing gaps exposed by unification (same-batch along-the-way fixes, A/B/C class — only
> fixing failed programs to succeed, zero impact on previously successful programs)**:
>
> 1. **Explicit type argument pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3 directly
>    emits specialized name (mono strips unspecialized original), `resolve_call_target` three arms
>    (rename / aliases / dot prefix) all don't cover `mk(int64)` form ⇒ call point dangling E6006.
>    Fix = add fourth arm to this single-point resolver (base name qualified, type parameter string
>    preserved as-is). **Same-form reproduction in Bin project (pre-existing defect, not introduced
>    by this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign
>    same-form as declaration) per statement ⇒ same variable allocated 3 slots, initialization
>    writes to later-allocated slot but read uses earlier-allocated slot (runtime Add type
>    mismatch). Fix = two places merge by name (same criterion same source).
> 3. **Same-module constrained generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually is `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidates + table verification (precise hit → generic
>    owner unit key qualified hit → fallback to bare name current state; not suffix heuristic, same
>    form as `resolve_generic_name`).
>
> **IV. Registration gap (not fixed this batch)**: Constraint dispatch still E6006 when **type and
> generic definition are in different modules** (rootless two-file probe verified; Bin same form) —
> root cause = MonoType user type name doesn't carry definition module, correct fix belongs to P6
> type identity convergence; registered WBS **3.18** (owns P3, prerequisite awaits P6, doesn't block
> main line), `qualify_deferred_method` annotated same surface.
>
> **V. Test Adaptation (honest registration)**: `tests/integration/multifile.rs` fixtures add
> manifest — what it tests is RFC-029 project orchestration (Bin auto-entry) not Script semantics;
> `cli_e2e` one pin (top-level diagnostic variable name visibility) and `script_main_hint` fixture
> (7 cases) synchronized per qualified name form — production consumer side only takes merged IR,
> fixtures directly drive same form, no pipeline product stand-in needed.
>
> **VI. Final gate (closing revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regeneration **12 rows** — line-by-line structural check: exit / stdout / diagnostic
> codes all identical, only stack frame function names and variable names qualified per unit key
> (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process events (recorded as-is)**: This batch once concurrently edited workspace documents
> during pre-commit hook running, was eaten by hook's stash rollback and that commit failed (the
> concurrent form warned about in configuration comment) — discipline = don't move workspace during
> hook running; #455's commit was therefore split into two attempts (second all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closing)

> **4.6 Batch Record (registered 2026-10-16 → closed 2026-10-16)**: User instructed 2026-10-16 to
> advance in 4.6 → 4.9 → 4.4 → P5 order, do 4.6 first; implementation form = **complete single
> team** (continuing last batch's user direction: no DAG, no team, single-session implementation +
> self-verification). Task definition (09 three-level table): semantic data (SemanticDB) of
> in-project files goes through orchestrator's same pipeline, same source as diagnostics; cross-file
> references carry `resolves_to`. Acceptance: in-project cross-file jump hits definition; semantic
> data and diagnostics come from the same compilation.
>
> **I. Current state empirical (verify before asserting)**: didOpen/didChange triggers **two**
> compilations — diagnostics go through unified Driver pipeline (4.2.3/4.2.6 already unified),
> semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ in-project `use`
> resolution fails, cross-file reference events don't occur, jump/rename/reference lookup silently
> fails for imported symbols — D55's registered silent failure). Orchestrator arm's checker
> uniformly names `"<module>"` (SemanticDB file identity lost), `type_result` (contains semantic_db)
> is discarded when diagnostics non-empty and never externalized; `checker.rs` exhaustively
> overwrites `resolves_to`'s DefId file identity to current file; `Export`/`ModuleInfo` both lack
> definition location surface.
>
> **II. Red before green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`) — after didOpen main.yx on
> temp two-file project, send GotoDefinition to `add_one` call point, **before fix returns None
> (red, empirically the silent failure in last section)**, after fix hits lib.yx line 0 definition
> (green). One probe positioning during process: compiler-side link (driver probe dump) already
> fully connected but e2e still empty — root cause is `resolve_reference` precisely matches by
> reference **start point** (D55 precise-only current state), test cursor falls at identifier
> middle; this tolerance belongs to existing behavior surface, not changed this batch (see VII).
>
> **III. Implementation surface**: (1) `Export.definition_span` (extract_module_info harvested from
> AST identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm fills by DiscoveredFile, including
> vendor; std always None); (2) checker reference exhaustion via `import_def_locations` (use line
> binding span → definition file + definition span) redirects cross-file imported items'
> `resolves_to` out of this file; std imports unregistered, fall back to use line; (3) checker
> module identity changed from unified `"<module>"` to unit file path (empirically all 4 consumption
> points of `env.module_name` zero diagnostic consumption); (4) `DriverOutcome.semantic_db` channel
> — CollectAll form (Lsp target file **regardless of diagnostic success or failure** + SingleFile
> fallback) gathered, Lsp arm tail injects non-target file top-level export definition surface from
> registry (`external_definitions`, can also jump to unopened files; type / submodule exports not
> injected — typecheck-side type registration doesn't produce definition events, both sides maintain
> consistent gap); (5) `orchestrator::analyze_source_in_project` (`check_source_in_project`
> delegation takes diagnostic half); (6) LSP `analysis.rs` single compilation entry replaces manual
> pipeline (`update_semantic_db` deleted), World key uniformly normalizes file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stably hits); 5 handlers +
> semanticTokens dual request connect to `db_key`.
>
> **IV. Along-the-way fixes (D59 split Class B — old behavior silently does nothing, zero external
> impact on previously successful programs)**: ① in-project file cross-file jump/rename/reference
> lookup changed from silent empty to hitting real definition; ② when lexical errors occur, old
> manual pipeline deleted semantic information (coloring disappeared) → broken AST coloring
> preserved (4.2.6 editor philosophy extension); ③ when target file has type errors, semantic data
> no longer discarded with `type_result` (mid-typing coloring/jump not lost).
>
> **V. Pins**: 6 driver-level (`src/driver/tests/lsp_semantic_db.rs` — target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback use line, error file retention, single file fallback path key, single file
> error retention) + 1 LSP e2e (red before green) + 4 uri normalization (`src/lsp/tests/uri.rs` —
> drive letter conversion, space encoding, idempotent normalization, non-file scheme pass-through).
>
> **VI. Mutation verification (scratch.py trial, mechanically reversible)**: batch tree wbs-46 @
> closing revision 39546020, two mutations both cause hit pins to go red, after restoration blob
> identical to HEAD + git status CLEAN — m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver pin + LSP e2e pin red, remaining 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two pins red (round-trip parsing loses anchor),
> remaining 7 green. Batch tree gc'd (freed 4.8 GB, `status` no batch trees).
>
> **VII. Final gate (closing revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> diff **byte-for-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synchronized server.rs:482 and 02 §Obligation
> Field Table) / check-docs-truth 0 / docs-examples 258 entries 0 failures / docs-orphan 0 /
> markdownlint repo only remaining user-unsubmitted 032 two MD056 (user's own editing, untouched) /
> pnpm docs:build 0. **Process events (recorded as-is)**: en/09a-wbs-history.md:54 one MD037 was
> introduced by auto-translate bot when retranslating last batch's 09a (zh source had no space),
> this batch fixed in passing — markdownlint red not this batch's change surface but in same file as
> 09a, registered together. **Remaining observation (registered for discussion, untouched)**:
> `resolve_reference` only precisely matches by reference start point — when cursor falls at
> identifier middle, jump/hover/rename silently does nothing (D55 precise-only current state,
> all-file form isomorphic), tolerance matching would be independent behavior change, report to user
> for decision.
>
> **Same-day continued batch (4.6.2, implemented after user decision)**: After closing report
> delivery user decided on "remaining observation ①" "OK, then change it" — implemented lookup key
> tolerance per Option A: 4 handlers' (definition/hover/references/rename) lookup key changed from
> cursor position (via UTF-16 conversion) to **identifier's own start point** already located by
> `find_identifier_at_position` (same coordinate system as semantic events, no conversion needed);
> `resolve_reference`'s start point precise matching semantic unchanged (D55 precise-only
> unchanged). Definition point fallback judgment (start <= col < end) for references/rename also
> holds for identifier start point isomorphically. Red before green: definition middle cursor pin
> first red (None) then green; 4 handlers middle cursor 1 pin each + cross-file e2e start/middle
> dual position assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0. Register WBS 4.6.2
> (three-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 1 Closing (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 Batch Record (registered 2026-10-16 → closed 2026-10-16)**: User decided 2026-10-16 to merge
> 4.9.1 / 4.9.2 / 4.9.3 three related three-level tasks **into one team, one round** (reason: they
> all belong to the "pool source and identity" surface, merged implementation avoids the
> intermediate state repeated rewriting after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong
> to consumption refinement and test pool separation respectively, depend on 4.6.1's `use` path
> resolution, forcing them into same batch would expand the gate surface). #450 ownership **pending
> decision** — evidence shows it belongs to "overload resolution name not passed back to reference
> count" class, still reproduces under round 1's already-landed form, register and handle as
> independent WBS item (lead recommends A: register as independent item in round 2; **pending user
> decision**). Frozen rev = `3863a9ff0` (parent commit `62f6e1a59` is pre-fix baseline);
> verification PASS attempt `aab92fe0` (i.e. t2); review pass attempt `96452696` (i.e. t3). Evidence
> archive: `target/yx-scratch/yx49/EVIDENCE.md` (includes 8 mutation patches and
> warn_compare_v8.json).
>
> **I. Merge Decision**
>
> 1. **Decision itself**: three related three-level tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round** — the former depends on 4.6.1's
>    delivered `use` path resolution (already landed in `39546020`, can start); the latter grouped
>    with RFC-029f table incorporation, registered in RFC-029f end section pending change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 mutually prerequisite (4.9.2 changes pool type → 4.9.1
>    assembly surface needs rewriting → only then 4.9.3 has denominator-judged role table), doing
>    them in separate rounds inevitably means writing one intermediate state then changing twice;
>    (b) all three belong to "pool source and identity" surface, 4.9.4 / 4.9.5 belong to another
>    surface, batch division is by "cohesion" rather than "independently revertable".
> 3. **Merge result**: single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: pre-push `git pull --rebase` merged in
>    auto-translate bot's commit (`72f364169`), all this batch's hashes rewritten — implementation
>    `3863a9ff0`→`3863a9ff0`, pre-fix baseline combined with their documentation commit
>    `62f6e1a59`→`62f6e1a59`, their code commit `33f14e8ab`→`b122b7ad1`. **External references all
>    use post-push hash** (old hashes unreachable in fresh clone).
>
> **II. Five Criteria Red/Green Empirical Dimensions**
>
> | #   | Criterion                                              | Red state (pre-fix / P1 directory scan backflow patch)                                                                                                 | Green state (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                           |
> | --- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | ①   | Pool root = package target (#334)                      | Directory recursive scan ⇒ files under tests/ also enter pool, exempting production code unused definitions                                            | `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` per `ExplicitSurfaces` root; when all roots degenerate, fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                                    |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-pollute | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembly single entry; `arms.rs:603-609`'s `other_module_refs` takes union of "all modules except this unit" entries and sends to `DeadCodeAnalyzer`; module's own names (top-level definition and local binding, `use` import name exception) no longer exempts other module's same-name definitions |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by references from other in-project files                                                                                              | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test goes through `_ => {}` default arm (judged by in-file definition-use graph)                                                                                                                                                                                  |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Before fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                    | Red state measurement (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                          |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Before fix: reports `W1003@1`                                                                                                                          | Same as above                                                                                                                                                                                                                                                                                                                                                                                          |
>
> **Criterion ②'s "derived red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` in helper.yx; old pool under bare-name union treats `foo` as a whole "used by
> helper", lib.yx's `foo` exempt. **Measurement**: red2 fixture vs pre-fix binary → no W1001 (old
> dimension exempt); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red state fixture **first version failed**: first
> version fixture put same-name local variable in same file triggering E2010 (this language
> disallows same-module shadowing of top-level definition), moved to another module (helper.yx) for
> derived red state. This point was independently reproduced by t3 review and t2 verification, **not
> a contract defect** — same-module shadowing is this language's hard constraint, fixtures must be
> cross-module.
>
> **III. +8 and +16 Two Sets of Counting Correspondence** (**t3 review confirmed as authoritative**)
>
> `target/yx-scratch/yx49/EVIDENCE.md`'s section C writes `+16/−0`, but missed the **"new fixture
> records"** component — v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff,
> `only_before=0`) dimension **includes** new fixtures; t1 implementer's self-built "+8" is
> **warn-before / warn-after (325 entries each) aggregated by fixture** (each member file checked
> once). Review item-by-item mapping:
>
> - **+8** (`8` new warnings, all `check <non-entry-file>`, `check <entry-file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: these 8 fixtures all **did not declare package target** ⇒
>     4.9.1 root degenerated to checked file's own compilation set, consumer main.yx not in it; this
>     is design-intended change (before fix these files' check surface was zero warnings anyway, not
>     exemption path). Repo root `yaoxiang.toml` is empty file ⇒ same degenerate path empirically
>     holds.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (shooting time point
>     **didn't exist** — these are new fixtures added this batch). So +8's "+ 0 − 0" diff doesn't
>     include them, +16's v8 symmetric diff does.
>   - `v8`'s `stray.yx` has only `W1001@2(probe)` remaining: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` per file), **cancel out** leaving only lib's 1
>     entry — three data sources (+8 / +16 / dual binary ABBA) mutually consistent.
> - **8 new fixtures** vs **+16 residual 8 entries**: same batch of files (4.9.1 / 4.9.2 added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats each check as
>   independent record — same file may trigger different diagnostic entries for multiple
>   `role / context`.
>
> **Conclusion**: **+16 = +8 (aggregated by fixture) ∪ 8 new fixture entries** — both counts
> correct, **different dimensions**: t1's "added 8 entries" is fixture-layer dimension, t2's "+16
> entries" is file-layer dimension (symmetric diff includes new fixtures). t3 review ① +8⊆+16
> confirmed; **review ② wrote "don't copy EVIDENCE.md section C verbatim"** — section C missed "new
> fixture records" component, this item rewritten per t3 review's `pass` conclusion.
>
> **IV. Mutation Experiments (Mechanically Reversible, All `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b use import name
> removal, p3b script takes pool, p3 flat consumption, p4 std takes pool, p7final self-built 4
> criteria, p8 std off, p9 vendor takes pool + p_stack2 / p_stack3 stacked surface). Each trial
> output contains: HEAD blob → post-mutation blob → post-restore blob identical to HEAD = yes +
> `git status` CLEAN. Local `core.autocrlf=true` needs `GIT_CONFIG_*` injection of `false`
> (`trial_env.ps1`), otherwise patch new lines converted to CRLF causing apply/restore dual failure
> — **only affects patch mechanics, doesn't change tested behavior**.
>
> **V. Process Events / Environment Lessons (Required Reading for Next Round Handover)**
>
> 1. **core.autocrlf injection**: Local `core.autocrlf=true` must have `false` injected via
>    `GIT_CONFIG_*` to ensure patches are mechanically reversible; see
>    `target/yx-scratch/yx49/trial_env.ps1`. **Next round don't reuse** any trial scripts without
>    injection.
> 2. **Same-tree trial serial**: `trial` and `cargo build` on the same tree **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch worktree, parallel can
>    produce "patch incomplete when build sees it" or "build and apply deadlock" two types of races.
> 3. **Trial rebuild binary needs sync confirmation**: After `trial` completes it rebuilds in-tree
>    binary (cargo's fingerprint contains source paths, **switching tree = full rebuild**), before
>    subsequent measurement must confirm `bin/yaoxiang-rs` synchronized with HEAD. **This round once
>    used residual mutation binary to measure false data**, rebuilt and remeasured for correction.
> 4. **Second tree yx49-b's creation reason**: Comparison needed pre-fix binary (62f6e1a59), no
>    existing product in batch; `scratch.py init yx49-b` opened second tree (ate another full
>    build). **Tree released** (see VI), deletion handled by batch closing `gc` disposition.
> 5. **cli_e2e two Internal test case fixtures patched to legal package (add `[run].main`,
>    assertions unchanged) reason**: Before fix `cli_e2e`'s two `Internal` role pin fixtures
>    **didn't declare `[run].main`** ⇒ after 4.9.1 landed `compile_project` went through
>    `ExplicitSurfaces` root call path, couldn't find `main` ⇒ whole batch 2 fixtures behavior
>    drifted from "Script fallback" to "entry not found" — is 4.9.1-introduced fallback behavior not
>    defect. After adding `[run].main` field two fixtures go through normal package path,
>    **assertions** (exit code / diagnostic code / warning set) byte-for-byte unchanged.
> 6. **Second round tree deletion safe way**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" then delete.
>    Released before this batch closes, next step `gc` naturally cleans.
>
> **VI. Batch Tree Status**
>
> Before closing: yx49 released (4.8 GB freed), yx49-b released. Both trees unmounted, status
> verifiable in `scratch.py status`. This task output appends `scratch.py gc` complete status.
>
> **VII. RFC-029f "Known Defects" Table Flip**
>
> Previous version (2026-10-09) described in present tense "three design defects of reference pool"
> (including deleted code positions like `collect_project_refs` / `orchestrator.rs:603-648` /
> `orchestrator.rs:393-396`). This batch closing synchronously rewrites to "**known defects: landed
> (2026-10-16, WBS 4.9 Round 1)**": ① current pool's root = `collect_package_refs`
> (`helpers.rs:481`), resolves per `ExplicitSurfaces`, doesn't scan directories; ② current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly single entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumer end `arms.rs:603-609`; ③ current Script doesn't take pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-fetched on cited revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` already moved to
> `src/driver/{state,arms,helpers}.rs` with P4 4.2.9 split (`c59fb751`), old positions in table no
> longer hit — this is "declaration inconsistent with fact" fix (t3 review note ①).
>
> **VIII. Final Gate (Closing revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 rows** (two new fixtures
> behavior run, 338a339,340; another 342 rows byte-for-byte unchanged) / IR snapshot 204 entries
> zero drift / check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process
> event**: this batch once cargo build failed due to `core.autocrlf` converting script line endings
> to CRLF, team scratch path `yaoxiang-rs` started with `BadImage`, empirically encoding
> inconsistent with git index — passed after rebuild (parallel to 4.6 batch's "external whole-tree
> discard" lesson, different cause).
>
> **IX. Remaining (Handover to Next Round)**
>
> 1. **#450 ownership decision**: evidence shows it belongs to "overload resolution name not passed
>    back to reference count" class, **still reproduces under round 1's already-landed form** —
>    dispatch per user decision ③ process. **#450 probe**: dual binary same set `{W1002@2, W1005@4}`
>    (measured, rootless form), not directly related to round 1 changes, but #450's fix may need to
>    wait for 4.9.4 (consumption refinement) together — because root cause "name passed to reference
>    count after overload resolution" needs use item's precise consumption surface, 4.9.4's "really
>    used after import" is precise version of same criterion.
> 2. **dead_code.rs:683 pointer nit**: comment says "two pools' assembly dimensions see driver side
>    collect_compilation_refs" — actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment and code main path dimension inconsistent
>    (**not in this batch's write scope**), left to 4.9.4 / 4.9.5 landing for correction.
> 3. **4.9.4 / 4.9.5 prerequisites unlocked**: 4.6.1 (`use` path resolution) already delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs first determine
>    RFC-029f table incorporation (already paved by this batch's "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6, user
>    decided 2026-10-15 ③ deferred to after 4.9) — this round 1 not involved; next round 4.9.4 /
>    4.9.5 landing handles in same batch.
