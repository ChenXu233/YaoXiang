# WBS Historical Notes (Landing Record · Audit · Adjudication Trail)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (Construction Task Table): landing records, audit findings, adjudication trails, and corrections
> for each phase. **Division of labor**: 09 only describes **plans** (Kanban, three-level task
> table, dependencies, risks, acceptance); this document only records **history** (what was done,
> what was found, how it was adjudicated), appended chronologically without rewriting history.

## P2 Establishing Equivalence Criteria Baseline

> **P2 Landing (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green. **D38
> prediction fulfilled** — the green-run process empirically tested and fixed three real defects in
> ir_gen (see 06 §G6 for details): the method locals table was truncated to parameter count
> (local_count underestimated, runtime relies on Frame::set_slot fallback), unannotated parameters
> were dropped by filter_map (three locations, params metadata misaligned with frame layout), and
> match result slots depended on the frame's implicit Void initialization (now explicitly pre-set).
> The actual semantics of jump targets is flattened instruction indices (not the label-keyed text in
> 07), and the CFG is built at the instruction level — recorded in the verify.rs file header.
> **Commit cross-reference**: ir_gen fix = `a9ea199b`; verify main body = `d5b18390` (⚠ Parallel
> stream COMMIT_EDITMSG collision caused its message to be incorrectly labeled "docs(rfc):
> RFC-027a…"; the content is actually verify.rs + six test files, buried in history unamendable,
> hereby recorded).
>
> **P2 Narrowing Part Landing (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3 stop-bleeding
> channel pre-conditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract uses the same `TestFileSpec` as
>   single-file corpus (library-side single point), binary location/subprocess launching extracted
>   to `tests/common/` with yx_runner migrated simultaneously (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed down by rfc027 tests (return position/call site/binding position) —
>   the `Sorted(3)` annotation example in 02 would be substituted and evaluated in current code to
>   be directly Disproved, not present as Unproven silently; the `Sorted(3)` in 07 pseudocode is
>   illustrative only. Red state empirical data is in the test file header.
> - The two red criteria are pinned to `#[ignore]` permanent per the 2.4.5 red skeleton precedent:
>   default suite is fully green, `-- --ignored` reproduces red state; **P3 fix landing will remove
>   the attribute to turn green**.
> - The comparison normalization's deliberate deviation from the 07 (code, file, line) table
>   (excluding file/line, comparing only error code sets) is documented in 2.4.1 test comments.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **removed from P2**: the `Program` / `Obligations` types they reference only exist in P4, and
> cannot be compiled in P2. The former becomes P4's 4.1.4 (no double registration), and the latter's
> `#[ignore]` red skeleton is merged into 4.3.1.
>
> **2.3.3 Landing (2026-10-07)**: C4 mandatory coverage checklist item-by-item verification (07
> §143), all 10 semantic corpus layers fully covered and within the corpus-parity.jsonl diff
> baseline (26/26 hits across 26 representative files, grep verified) — ref/borrow/move 05-ownership
> full directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.),
> closure captures closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterators for spawn_for and 16 other files, sum types
> sum_type__+ pattern_or_guard + match_call_scrutinee, existential coercion point
> interface_dynamic_dispatch (Vec(Animal) existential type heterogeneous wrapping triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (+
> method_overload_ambiguous_err at diagnostic diff surface), refinement constraints Drop sequence
> refined__ ×5 + 05-ownership-driven ReleasePlan (span key contract has another 2.4.4 red criterion
> permanent). Performance baseline `benches/pipeline.rs` checked in (07 §145 two of three smoke
> tests): `corpus_compile_frontend` first-time full corpus compilation 17.6s, `cli_cold_start_hello`
> process cold start 169.7ms; interpretation throughput already covered by benches/lib.rs hotpath
> group (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate construction. Regression
> gate check-perf-regression.sh deferred to P9 unified list (2.5).

## P3 Correctness Vulnerability Fixes (Minimal Solution)

> **Error type annotations for landed rows** (retrospective annotation, no history changes): 3.1 = B
> | 3.2 = B (defensive empty arm, criterion turn-green borne by 3.1) | 3.3 = A | 3.4.1 / 3.4.2 = A
> (new W codes) | 3.4.3 = equivalent refactor (manual review) | 3.4.6 / 3.4.7 = D (user-visible
> surface, already adjudicated by user) | 3.4.8 ①② = C (main) + A (incidental three locations) | 3.5
> / 3.6 / 3.7 / 3.8 = D (all already adjudicated by user).
>
> **P3 Diagnosis (2026-10-12, requested by user)**: A diagnostic triage for the perception of "P3
> items keep growing".
>
> **I. Clarifying the scale first (the perception is amplified by two accounting defects)**
>
> 1. **Numbering collision**: `3.5.x` is shared by two task families — (a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level items: lexer and parser branches / AST `is_pub` /
>    checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) 2026-10-10 registered **3.5 Higher-kinded
>    type parameter call path completion** (3 three-level items). The two share `3.5.x` ⇒
>    cross-reference ambiguity (**2026-10-12 already renumbered per user adjudication: 029g family
>    changed to `029g.x`**), and caused 029g's rows to be mistakenly counted in P3. **Pending
>    adjudication**: Renumber the 029g family to `029g.x` (or move them into their respective
>    phases), and synchronize all other references within the document.
> 2. **Missing checkmarks**: The 4 three-level items in `3.7` (3.7.1–3.7.4) are all completed
>    (d922a551 / 38bd4b9e) but not checked ⇒ corrected in this entry.
>
> **II. P3 Actual Accounts (12 second-level rows)**
>
> | Row                                                                           | Status                                                                                                                                                                                | Disposition Suggestion                                                                                                              |
> | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                               | Completed                                                                                                                                                                             | —                                                                                                                                   |
> | 3.4 Silent drop points addition (8 three-level)                               | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 by be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlock with 4.3.1                                                                            |
> | 3.5 Higher-kinded type parameter call path completion (3 three-level)         | Not started (declaration side live, call side broken)                                                                                                                                 | Keep in P3; **does not block refactor main line**, but is "accepted feature unimplemented", priority higher than pure cleanup items |
> | 3.6 Function type value call path (1 three-level)                             | Not started (semantics already cut to (i))                                                                                                                                            | **Keep**: correctness defect, criteria clear                                                                                        |
> | 3.7 init segment call name qualification (4 three-level)                      | **All completed**                                                                                                                                                                     | Check and close in this entry                                                                                                       |
> | 3.8 native shadowing (1 three-level)                                          | Not started                                                                                                                                                                           | **Stuck on decision** (first fix name resolution priority), not a scheduling issue                                                  |
> | 3.9 / 3.10 / 3.11                                                             | Completed (2026-10-12)                                                                                                                                                                | —                                                                                                                                   |
> | 3.12 Runtime pattern inter-executor configuration propagation (1 three-level) | Not started (Class D, user adjudication B: separate team)                                                                                                                             | **Suggest moving out of P3 → P8 (execution layer)**, pending adjudication                                                           |
>
> **IV. 4.3.1's Leverage (Main Line Marker)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the precondition for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId) ⇒ taking it once advances the main line while clearing two P3 stranded items. 3.5 (HKT),
> 3.6 (function type value call), and 4.6.1/4.9.1 have **no unsatisfied preconditions**, ready to do
> at any time; 3.8 is stuck on semantic decision (user definition priority vs explicit diagnostic).
>
> **III. Mechanism Suggestion (Prevention of Recurrence)**: Newly registered second/third-level rows
> **must have two fields** — "belonging phase" (which P it truly belongs to) and "whether it blocks
> the main line"; `registered awaiting scheduling` is no longer the default state, unmarked is
> considered unschedulable. This way, bloat is visible and sortable, rather than undifferentiated
> accumulation. **Pending adjudication**: Whether to write this into the registration convention of
> this document and `src/dev/coding-rules.md`.
>
> **P3 Landing (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + sole getter (recorded as
> `pub(crate)` at the time; **2026-10-07 verified actual landing is `pub(super)`** — the code
> comment references the `#434` adjudication with no record on docs side, see 3.4.3; fully private
> would require rewriting 11 construction literals, not taken during the stop-bleeding phase; will
> be closed off when P4 Obligations migrates); execution mechanism extracted to
> `src/frontend/proof_execution.rs` (prohibition three single point — cannot be placed in
> typecheck/, which would create L3→L4 reverse dependency); 3.1.2 supplemented per 02 evidence chain
> step 5 with **four entry points** (WBS row number missed compile_project `:99`, must be
> supplemented and registered as required by 3.1.1 acceptance "criterion turn-green"); 3.2.1 empty
> arm changed to accounting (proof_calls throws) + diagnostic (the established `into_result`
> conversion path); 3.3.1 SOLVER slot Option-ified, Z3 missing conservatively downgraded to
> `SMTResult::Unknown` (E2031 family diagnostics replace process crash). Two vulnerability criteria
> remove `#[ignore]` to turn normal green, supplementary corpus fixture `proof-obligation-honored`;
> check-fanout inventory migrated accordingly (wiring point 19 = inventory). Empirical test:
> criteria 2.4.1/2.4.3 turn green, full suite lib 2494 / integration 272 / corpus two-layer all
> green.
>
> **3.4 Audit (2026-10-07, P3 landing post-review)**: For the type of defect "failure produces no
> signal" (see definition 02 §41-52), without changing code, a re-inventory located 8 more
> locations, all registered as 3.4.x in the table above. Method and evidence:
>
> - **Consumer-side inventory** (16 obligation fields grep'd one-by-one for all read sites in the
>   repo): `variant_ctor_calls` **zero consumers** — the sole read at `checker.rs:1468` is "itself
>   fed into `TypeCheckResult`"; the IR side uses shape detection at `ir_gen.rs:7374`;
>   `rfc011b.rs:659/688` only asserts the production end, which is exactly the "only test production
>   end, naturally immune to consumer-end defects" pattern criticized in 02 §186 → 3.4.4.
>   `method_overload_ir_names` result-level field likewise zero external consumers, IR uses parallel
>   `overload_resolutions` → 3.4.5.
> - **Phase coverage review**: 02 §91 table row 1 (four entries "no proof_execution") already fixed
>   by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 state unchanged (multi-file `run` still
>   doesn't output W1003, doesn't run dead code family) → 3.4.6; LSP non-project-internal paths (02
>   §77 "manual lex→parse") also don't run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` has only four consumers in the entire repo): checker
>   three refinement branches (3.1 already covered), 3.2.1 fixed ownership arm (**source
>   unreachable**), and `environment.rs:422` (**reachable, zero diagnostics, zero accounting, and
>   zero records in all repo docs**) → 3.4.1, the only "all-unknown" item in this audit.
> - **Solver acquisition side**: `default_solver()` has three consumer forms (`predicate.rs:32`
>   process singleton / `checker.rs:1266` injected per `check_module` / `ownership.rs:621` newly
>   created per back-edge judgment) — 3.3.1 only eliminated one panic; the **silent surface** of the
>   other two goes to 3.4.2, **singletoning and cache hit** goes to P4's 4.4.2 (no double
>   registration).
> - **Not counted in 3.4, existing attributions**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys not consumed, Drop silently lost) still P2 red state permanent, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration vs actual reversed, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4 registry.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions, can land in place at any
> time (suggest doing 3.4.1 first — the only "all-repo zero-registration" live vulnerability,
> smallest change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on the obligation
> ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish new criteria systems**,
> uniformly reusing existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 Record (Registered 2026-10-13 → Closed 2026-10-15)**: User 2026-10-14
> adjudicated to merge four correlation minor items (#451 method binding panic / #452 named
> arguments in resolution / #453 dead inference entry / #454 native export named arguments — the
> last item was a boundary discovery during 3.14 verification, same day verified by attribution
> experiment to be **not introduced by #452**, user adjudication ① merged into this batch) into one
> team for handling. Closing state: all four items completed, adjudication registered as RFC-039
> **D60** (six binding rules + shared criteria single-point).
>
> **I. Two Contract Corrections (Lesson: Contracts Themselves Can Be Wrong)**
>
> 1. **Acceptance fixture self-contradicting**: #452 task contract simultaneously required "mixed
>    arguments output 20" and "position and named conflict at same position reports error", but the
>    nailed fixture `Temp.pick(self, a)` has only one value parameter — the same argument
>    combination is both "mixed" and "conflicting". The first implementer truthfully reported
>    blocker and set task failed, full revert (no green light via silence). Disposition: task
>    reassigned + contract revision — after providing **six binding rules** and **8 value-level
>    matrices** (fixture changed to two value parameters `pick(self, n, a)`), attempt 2 was
>    completed by the second implementer. The full text of the six rules has been verbatim recorded
>    in D60.
> 2. **inScope missing mechanically required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only `code-tables --fix` was
>    run, **`scripts/docs/gen-error-code-docs.py` was NOT re-run** ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964** — the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touching `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **II. Review → Fix → Re-review Two Closed Loops**
>
> 1. **#452 Line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls with named arguments silently passed through the entire
>    chain on both multi-candidate zero-hit and single-candidate hit paths) → fix round added
>    external code **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator
>    four-piece set in same commit; code count 154→155) → round-2 review pass.
> 2. **#454 Line**: implementation (shared parsing entry + native arm settles by name) → independent
>    verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations all captured via
>    `scratch.py trial` mechanically reversible replay, restoration byte-by-byte consistent) →
>    round-1 review needs_revision (F1 = typecheck native arm **inlined** a copy of the binding
>    criterion, missing rule ④ ⇒ **adjudication divergence** for same-shape calls between method and
>    native — method reports E1111, native silently exits=0; ir_gen has a third existing copy
>    reporting old code E1015) → convergence round consolidated criterion to a single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call sites `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught a **regression** introduced by the
>    convergence round (IR convergence block lost native omitable position ⇒ previously successful
>    programs omitting optional position were misreported as E1010) → fix round (optional position
>    criterion and parameter name from single source `signature.rs:236`) → round-3 review PASS
>    (independently reproduced red→green; mutation nailed fixture FAILED in repo empirically proved
>    criterion can fail). **Lesson**: Review must cover "unfixed branches of same-type paths" (both
>    needs_revisions in this batch were due to probes all falling on the hit surface); the fix round
>    itself can also introduce regressions (omittable position), caught by the next round of
>    reviewer change.
>
> **III. Process Events (Honestly Recorded, Not Pointing to Individuals)**
>
> 1. **External whole-tree discard (around 17:14 on 2026-10-14)**: The work tree was externally
>    restored silently — the implementer's fix in progress (typecheck/ir_gen/code
>    table/locales/RFC-013, etc.) all disappeared, **at the same moment two review documents edited
>    by the user themselves also disappeared from git status** ⇒ determined to be repository-level
>    discard (most likely user-side manual cleanup/IDE "discard all changes"). Three pieces of
>    evidence exclude team members: stash empty, no new commits, no reset in reflog. Implementer
>    replayed in-progress artifacts by context and pushed; discipline changed to "**save patch to
>    scratch first, then collect red state**".
> 2. **Verifier context exhaustion change of person**: The independent verifier for 3.16 ran out of
>    room mid-way (no closing message), per load rules changed person — successor continued using
>    their already-ready artifacts (green state results, corpus equivalence script, mutation plan),
>    verification ultimately PASS. Positive precedent for partial artifact handover.
> 3. **Round 5 reviewer rotation and historical round rollback**: When rotating reviewers per load
>    cap item ⑤, `remove_member` rolled back **already adjudicated final-state review/verification
>    rounds** to the shared pool, the scheduler immediately re-ran (t3/t8 were re-claimed by new
>    members, duplicate work and blocking downstream). Disposition: team lead took over and canceled
>    the rolled-back historical rounds (t3/t8/t13), based on this added AGENTS.md item 12
>    (immediately handle rolled-back rounds after person change). This batch's closing record is
>    registered per this item.
> 4. **scratch governance (about 6.3 GB)**: Early stage of batch had one external tree per task, 6
>    directories scattered outside the repo (about 780 MB, cleaned) + two members' external
>    verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md item 9
>    (batch tree single entry), item 10 (verification environment six rules: mechanically reversible
>    mutation / line number from referenced revision / CI unique gates must run, etc.), item 11
>    (writer load cap), item 12 (handle rolled-back rounds after person change) and top meta-rules
>    two (no private content / verify before asserting).
> 5. **Contract defect (not pointing to members)**: #454 fix round's contract inScope only circled 2
>    files, actually mechanically required surface is 8 (item 6 is exactly designed to prevent this
>    — contract template should preset mechanically required surface list); additionally the task
>    final-state field was once occupied by probe payload, formal acceptance/command evidence was
>    supplemented by team lead per convention.
>
> **IV. Remaining Items (Not Fixed in This Batch, Registered for Reference)**: round-3 review three
> low-severity findings — ① method path's `build_arg_slots` (`ir_gen.rs:521` area) is still the
> third implementation of the criterion (existing, per previous round's caliber registered at
> closure, not converged); ② batch verification tree (worktree) `cargo fmt --all` error is worktree
> workspace artifact, gate list should note fmt goes to main repo; ③ repair-type contract template
> should preset mechanically required surface in inScope (see III.5).
>
> **V. Final Gate (Closing Revision, Local Two Implementation Commits + One Doc Commit Unified Push
> — Only Push of This Batch)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` **2730
> passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner` 2
> passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> lines line-by-line consistent with baseline** (only regenerated line: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line number synchronized, manually reviewed) / IR snapshot
> zero drift.
>
> **VI. Run Links (Push-side Evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test canceled by subsequent push
> per concurrency — cancel≠failure, covered by later run); governance tool push → CI 38546373435 /
> 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) → CI 38528560364. The
> closing push's own CI / Deploy Docs run is registered in the task record after the push.
>
> **3.4.8 ③④ Landing (2026-10-11, 8ecf56f93)**: ③ Added `Linking` to end of `CHECK_STAGES` (**pure
> merging**, the entire block skips the `is_bin_role` entry validation — E3020 criterion still
> belongs to `RoleClassification` to prevent double reporting; another hard reason: run path judges
> "has main" by merged IR, in collection semantics "entry itself errors → no IR" would be misjudged
> as missing main and hard-aborted) → `Monomorphization` (check shape); `ir_generation_check`
> retains unit IR (**in-memory only**, consumed by pure merging and mono, not
> emitted/written/produce snapshot — not retained would result in empty aggregation request, mono
> with `NoObligations` fake green); mono failure in Check goes through per-file collection channel
> (diagnostics attributed to entry file + `failed_stages`, not set to failure), MultiFile maintains
> `fail_with`; `into_outcome` changed to "first take collection then judge failure". ④ Empirically
> **zero production changes**: full-program BFS already covers cross-unit mutual recursion
> (pathological → E3005, well-formed three-layer chain executes to correct value), delivered four
> e2e groups + one mono unit test as live criteria.
>
> **Verification Chain** (All Independent of Implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross comparison with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 examples + dual binary comparison) all pass; t5 cross-shape matrix
> (single/multi/check three shapes + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase no decrease).
>
> **F1 (Discovered During Verification, D59 Class A/B, Same Round Fix)**: Under standalone
> multi-file check without project root, the aggregation request name (`lib.f`) and the unit IR
> definition key (`f`) are from different sources → mono falls into silent `continue` of "native has
> no IR definition", check reports 0 error/exit 0. Fix: aggregation side added reverse lookup by
> each unit's **true definition key** (exact enumeration + literal equality + ambiguity abandonment,
> not suffix heuristic; project shape not registered, existing path unchanged). **Correction (Same
> Day)**: This reverse lookup was **deleted** in the subsequent regression fix; root cause instead
> solved from naming caliber by `module_key` dual-purpose decoupling (added `script_mode`) — IR
> definition key = IR call name = aggregation request name = `{unit key}.{name}`. **Regression
> Record**: After ③ landed, CI's `Check tutorial/guide code examples compile` red (258 blocks / 10
> failures: 6 items same-file same-name rebinding + 4 items colliding with `<std/list>`, rootless
> check not doing qualification triggered `link_module_irs` conflict check); fix = ir_gen split
> `script_mode` (qualified name ⊥ Script/Bin, E3023 gate changed to look at it) +
> `ir_generation_check` per unit always pass unit key + `.with_script_mode(rootless)` + Script
> top-level same-name rebinding "latter wins" (consistent with run path existing behavior);
> verification = doc gate 258/0, three binary comparison (HEAD/③ tree/fix tree) three types of
> collision fixtures, check shape E3005 no regression, E3023 not triggered.
>
> **F2/F3 (Accepted)**: mono diagnostics attributed to entry file entries, span can cross units
> (same caliber as run path `path=entry`); Check shape holds one ModuleIR per unit (in-memory only,
> released at check end).
>
> **Existing Issues Left for Follow-up (Not Regression of This Round)**: ① E3005's help text still
> writes "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user-written error" out of E3005) — registered
> awaiting scheduling; ② multi-file E3005's span rendering label misalignment (already reproduced
> with HEAD binary, pre-existing); ③ request collection surface has **type parameter position
> dependency** (generic calls where type parameters are not at value parameter position don't issue
> instantiation requests, mono records `NoObligations`, converging mutual recursion is this shape) —
> same surface as 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Remove `pub` and Auto-Binding (#399)

> **P3.5 Landing (2026-10-08)**: All seven second-level tasks completed, entire batch in one commit
> (independently revertable).
>
> - **Empirical vs Plan Discrepancy (Honestly Recorded)**: 029g.1.2/029g.1.3's acceptance originally
>   wrote "`pub f = ...` reports **parse error**" — actually after deleting `KwPub`, `pub` reverts
>   to a regular identifier, the old syntax falls to the semantic layer reporting **E1001 Unknown
>   variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically detecting removed
>   syntax is an anti-pattern", **no new migration-specific diagnostic added**; corpus fixture
>   `pub_declaration_err.yx` nails E1001, document (`syntax.md`) honestly states this consequence.
> - **Design Convergence (Beyond RFC Letter, Requires Review)**: `project_refs` and
>   `cross_file_refs` two fields have fully overlapping semantics after `pub` deletion (same
>   "intra-package reference union", prohibition one prohibits parallel representation) → merged
>   into single `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted.
>   Behaviorally, Internal role's non-pub items changed from "file-internal reachability" to
>   "reference pool judgment" (loosened, toward under-reporting); Lib tightened from "pub absolute
>   exemption" to reference pool judgment; Script's unreferenced top-level bindings start reporting
>   W1001/W1002/W1004/W1005.
> - **Dead Table Cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports write-back all deleted (production end feed,
>   entire repo zero consumption).
> - **Test Migration** (test count not decreasing): Added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer a modifier"; `passes/tests/dead_code.rs` 7 role semantic use cases rewritten (2 assertion
>   reversals); `tests/integration/cli_e2e.rs` 8 role use cases rewritten (`lib`/`internal`
>   exemption assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not involved in IR); corpus diff baseline only
>   added 1 item (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated then manually reviewed.
> - **Empirical**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId (D20)

> **4.5 Landing Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closing
> (registration / WBS check / gate / push) completed by t7. Precondition = 4.3.1 obligation ledger
> (red state archived 173 files / 533 keys).
>
> **I. Carrier Adjudication (A: `PlanId` Travels with AST Node)**: Added `PlanId`
> (`src/util/plan_id.rs`, `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX`
> represents synthetic nodes outside parser), issued by **sole allocator**
> `ParserState::alloc_plan_id`; carrier two locations — `Stmt.plan_id` (statement identity) and
> `Expr::Call.plan_id` (call site identity), `Expr::call` is the sole call constructor in the entire
> repo. The rejected form is "both sides derive the same identity by span" position coincidence (the
> original defect of D20).
>
> **II. Two-Level Verification and Review (Members Independent)**: Each functional commit has one
> independent verifier and one reviewer, **verification/review does not reuse implementer's
> evidence**: 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations)
> / t3 review; 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations)
> / t6 review. Review timing and reviewed revision are both in immutable history (verifiable by
> `git show`). **Provider rename caused t5 / t6 member change and re-dispatch**: Original
> verification/review members could not continue due to provider rename, per orchestration
> convention changed members and re-dispatched with **reason clearly stated in record** (cross-round
> context retention priority, change only when compression is unavailable); after re-dispatch
> conclusions consistent with original rounds (PASS), implementer evidence not reused.
>
> **III. Two Defects Found and Fixed During Self-Check at Implementation (t1, Within inScope)**: ①
> Consumer exemption criterion was hung on `UnitState.ir.is_some()` — `check` shape's Linking/Mono
> would take `unit.ir` away, the truly unconsumed in check shape **silently passed** (mutation
> empirical: D41 red while CLI check exit 0); changed to read the **ledger's own completion mark**,
> and added regression cases. ② Drop sequence drifted with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion consecutively ran three times producing
> 106/120/114 line count drift; changed to **total order sort** (same name takes max depth + name
> ascending) then ran twice with zero drift.
>
> **IV. Criteria and Evidence Numbers**: After 4.5.1, `test_release_plan_spans_consumed` removed
> `#[ignore]` permanent (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in
> same commit (programmatic review: added 1083 lines Drop + 86 lines jump index translation, zero
> deletion, no reordering), corpus 342 items diff empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference set empty + evidence surface floor `examined ≥ 1`
> / `consumed ≥ 2`; empirical examined=1, consumed entries 2, no resolution skipped 210), resolution
> shape 9 items nailed, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation driven: consumer key offset by one position ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; green after restore.
>
> **V. Semantic Correction Classification Basis (D59)**:
> `Type.method(positional argument, name = value)` before change **silently gave wrong answer**
> (empirically compiled clean, ran outputting `1` instead of `20`) ⇒ **Class C** (old behavior not
> successful), fix takes "loud": same program after change compile-time `error [W1090]`; the
> terminal semantics of "named arguments participating in resolution" is **Class D**, registered
> §3.14 and left issue #452 for dedicated team. Additionally: `Drop` in interpreter is no-op
> (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`) — key migration
> does not change runtime behavior.
>
> **VI. F-1 / F-2 Disposition (Found by t5 Independent Verification)**: **F-1 (medium)** unbound
> consumption point (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, key
> control correctness has no red state to falsify ⇒ registered as "known boundary" in
> [02](02-stage-contract.md) §2 revision note paragraph 5 (preservation reason + no new task); **F-2
> (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise pointers
> (production `inference/expressions.rs:4550`, consumption `ir_gen.rs:7697` / `:7772`), single small
> commit, not mixed into doc commit.
>
> **VII. Closing (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settle marks); 07
> criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one line of behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Closing-Discovered Accounting Deviation**: pointers
> `ir_gen.rs:357` / `:3102` / `:1096` in t3 / t6 reports differ from code reality by one to three
> lines (actually `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion) — registered per code
> reality, this note also leaves trace.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (run half of 4.2.5 Adjudication A)

> **Batch C Record (Registered 2026-10-15 → Closed 2026-10-15)**: User 2026-10-15 adjudicated C —
> this round only does 4.10.3 (same-batch candidate 3.4.6 per B1 ([02](02-stage-contract.md)
> §Runtime Behavior Table "Dead code family warnings don't enter compile path") deferred to after
> 4.9, precondition changed and annotated); implementation form = **complete single team** (user
> instruction not to open DAG, not to build team, single session implementation +
> self-verification). Pre-batch debt first landed: #455 dependency announcement fix (rustls
> 0.23.46 + serde_yaml_ng, `cargo audit` restored from exit 1 to exit 0), WBS registration debt
> (#456 registered as 3.17 + incomplete rows added "belonging phase / whether blocks main line"
> fields).
>
> **I. Red Before Green**: 3 driver-level red criteria (`src/driver/tests/run_rootless.rs` —
> rootless merged IR must not carry entry_function (red state `Some("main.main")`), rootless
> top-level executable statements must not report E3023 (red state doesn't produce module), Bin
> control entry_function maintained (green state permanent)) + CLI e2e 3 nails (standalone relative
> import run (red state E5001), Script top-level statement execution and #413 hint two green state
> nails).
>
> **II. Implementation Surface**: CLI `run` uniformly goes through `compile_project` (the
> project/standalone bifurcation in `run_file_with_diagnostics` deleted); Linking's `entry_function`
> gated by `is_bin_role` (#413 T4 — Script doesn't automatically call main); #413 hint keying
> changed from `module.source_files.is_empty()` to call site project probe (after unification
> ModuleIR always carries source_files, original marker invalid); `script_main_never_called` adapted
> to qualified name shape (function `{key}.main`, global slot empirically also qualified name — per
> IR evidence, not per comment inference).
>
> **III. Existing Gaps Exposed by Unification (Same Batch Incidental Fix, Class A/B/C — Only Fixing
> Failed Programs to Succeed, Zero Impact on Previously Successful Programs)**:
>
> 1. **Explicit type argument pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3 directly
>    emits specialization name (mono strips unspecialized original), `resolve_call_target` three
>    arms (rename / aliases / dot prefix) all don't cover `mk(int64)` shape ⇒ call site dangling
>    E6006. Fix = this single point resolver adds fourth arm (base name qualification, type
>    parameter string preserved as-is). **Bin project same-shape reproduction (pre-existing defect,
>    not introduced by this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign
>    same-shape as declaration) per statement ⇒ same variable allocated 3 slots, initialization
>    writes the later slot while reading uses the earlier slot (runtime Add type mismatch). Fix =
>    two locations merge by name (same criterion same source).
> 3. **Same-module constrained generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidates + table lookup verification (exact hit →
>    generic's unit key qualified hit → fallback to bare name current state; not suffix heuristic,
>    same shape as `resolve_generic_name`).
>
> **IV. Registration Gap (Not Fixed This Batch)**: Constrained dispatch in **type and generic
> definition in different modules** still E6006 (rootless dual-file probe verified; Bin same shape)
> — root cause = MonoType user type name doesn't carry definition module, correct fix belongs to P6
> type identity convergence; registered WBS **3.18** (belongs to P3, precondition awaits P6, does
> not block main line), `qualify_deferred_method` annotated same surface.
>
> **V. Test Adaptation (Honestly Registered)**: `tests/integration/multifile.rs` fixtures added
> manifest — what it tests is RFC-029 project orchestration (Bin auto-entry) rather than Script
> semantics; one nail in `cli_e2e` (top-level diagnostic variable name visibility) and
> `script_main_hint` fixture (7 examples) synchronized to qualified name shape — production/consumer
> side only takes merged IR, fixtures directly drive same shape, no pipeline product stand-in
> needed.
>
> **VI. Final Gate (Closing Revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regenerated **12 lines** — line-by-line structured check: exit / stdout / diagnostic
> code all per item identical, only stack frame function names and variable names qualified by unit
> key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process Events (Honestly Registered)**: During this batch, concurrent editing of workspace
> documents during pre-commit hook run was eaten by the hook's stash rollback and that commit failed
> (the concurrent form previewed in config comments) — discipline = don't move workspace during hook
> run; #455's commit was therefore split into two attempts (second fully green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 Batch Record (Registered 2026-10-16 → Closed 2026-10-16)**: User 2026-10-16 instructed to
> proceed in order 4.6 → 4.9 → 4.4 → P5, do 4.6 first; implementation form = **complete single
> team** (following previous batch user instruction: no DAG, no team, single session
> implementation + self-verification). Task definition (09 three-level table): project-internal file
> semantic data (SemanticDB) goes through orchestrator same pipeline, same source as diagnostics;
> cross-file references carry `resolves_to`. Acceptance: project-internal cross-file jump hits
> definition; semantic data and diagnostics from the same compilation.
>
> **I. Current State Empirical (Verify Before Asserting)**: didOpen/didChange triggers **two
> passes** of compilation — diagnostics go through unified Driver pipeline (4.2.3/4.2.6 already
> unified), semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` resolution fails, cross-file reference events not produced, jump/rename/reference lookup
> silently fail for imported symbols — silent failure registered as D55). The orchestration arm's
> checker uniformly names `"<module>"` (SemanticDB file identity lost), `type_result` (containing
> semantic_db) is discarded when diagnostic is non-empty and never emitted; `checker.rs` emptying
> the `resolves_to` DefId file identity is always overwritten to current file; both
> `Export`/`ModuleInfo` have no definition position surface.
>
> **II. Red Before Green**: LSP e2e nail (`src/lsp/tests/cross_file.rs`) — temp dual-file project
> didOpen main.yx then on `add_one` call site send GotoDefinition, **before fix returns None (red,
> empirically the silent failure in previous section)**, after fix hits lib.yx line 0 definition
> (green). During process a probe located: compiler-side chain (driver probe dump) already fully
> connected while e2e still empty — root cause is `resolve_reference` precisely matches by reference
> **start point** (D55 precise-only current caliber), test cursor falls in middle of identifier;
> this tolerance is existing behavior surface, this batch doesn't move it (see VII).
>
> **III. Implementation Surface**: (1) `Export.definition_span` (extract_module_info harvests from
> AST identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm fills per DiscoveredFile, including
> vendor; std always None); (2) checker reference empty passes through `import_def_locations` (use
> line binding span → definition file + definition span) redirects cross-file import item's
> `resolves_to` out of this file; std import not registered, maintains falling to use line; (3)
> checker module identity changed from unified `"<module>"` to unit file path (empirically
> `env.module_name` all 4 consumer points zero diagnostic consumption); (4)
> `DriverOutcome.semantic_db` channel — CollectAll shape (Lsp target file **regardless of diagnostic
> success or failure** + SingleFile fallback) converges, Lsp arm tail from registry injects
> non-target file top-level export definition surface (`external_definitions`, files not opened can
> also jump; types/submodule exports not injected — typecheck side type registration doesn't produce
> definition events, both sides maintain consistent vacancy); (5)
> `orchestrator::analyze_source_in_project` (`check_source_in_project` redirected to take diagnostic
> half); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key uniformly normalized file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stably hits); 5 handlers +
> semanticTokens dual requests take `db_key`.
>
> **IV. Incidental Fix (D59 Routing Class B — Old Behavior Silently Does Nothing, Zero Impact on
> Previously Successful Programs' External Performance)**: ① Project-internal file cross-file
> jump/rename/reference lookup changed from silently failing to hitting real definition; ② Lexical
> error time old manual pipeline deletes semantic info (highlighting disappears) → residual AST
> highlighting preserved (4.2.6 editor philosophy extension); ③ Target file with type errors
> semantic data no longer discarded along with `type_result` (typing intermediate state
> highlighting/jump not lost).
>
> **V. Nails**: 6 driver-level (`src/driver/tests/lsp_semantic_db.rs` — target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback to use line, error file preserved, single-file fallback path key, single-file
> error preserved) + LSP e2e 1 (red before green) + uri normalization 4 (`src/lsp/tests/uri.rs` —
> drive letter conversion, space encoding, idempotent normalization, non-file scheme passthrough).
>
> **VI. Mutation Verification (scratch.py trial, Mechanically Reversible)**: Batch tree wbs-46 @
> closing revision 39546020, both mutations caused the hit nail to turn red, after restore blob
> consistent with HEAD + git status CLEAN — m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver nail + LSP e2e nail red, other 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two nails red (round-trip parsing lost anchor),
> other 7 green. Batch tree already gc'd (released 4.8 GB, `status` no batch tree).
>
> **VII. Final Gate (Closing Revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new nails) / `--test integration` **287 passed** / corpus
> diff **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synchronized server.rs:482 and 02 §Obligation
> Field Table) / check-docs-truth 0 / docs-examples 258 items 0 failures / docs-orphan 0 /
> markdownlint all-repo only remaining user-uncommitted 032 two locations MD056 (user's own edit,
> untouched) / pnpm docs:build 0. **Process Event (Honestly Registered)**: en/09a-wbs-history.md:54
> one MD037 was introduced by auto-translate bot retranslating previous batch 09a (zh original has
> no space), this batch incidentally fixed back — markdownlint red is not this batch's change
> surface but same file as 09a, registered together. **Remaining Observation (Registered for
> Discussion, Not Moved)**: `resolve_reference` only precisely matches by reference start point —
> when cursor falls in middle of identifier jump/hover/rename silently does nothing (D55
> precise-only current state, full-file shape same structure), if tolerance matching is needed it's
> independent behavior change, report to user for adjudication.
>
> **Same-Day Continuation Batch (4.6.2, Landed After User Adjudication)**: After the closing report
> was delivered, user adjudicated on "Remaining Observation ①" as "OK, then change it" — lookup key
> tolerance implemented per plan A: definition/hover/references/rename four handlers' lookup key
> changed from cursor position (via UTF-16 conversion) to use the **identifier itself's start
> point** already located by `find_identifier_at_position` (same coordinate system as semantic
> events, no conversion needed); `resolve_reference`'s start point precise matching semantics
> unchanged (D55 precise-only unchanged). references/rename's definition point fallback judgment
> (start <= col < end) holds same structure for identifier start point. Red before green: definition
> middle cursor nail first red (None) then green; four handlers middle cursor 1 nail each +
> cross-file e2e start/middle dual position assertion, `--lib` 2748 (+4) all green, clippy -D
> warnings 0. Registered WBS 4.6.2 (three-level count 150→151).
