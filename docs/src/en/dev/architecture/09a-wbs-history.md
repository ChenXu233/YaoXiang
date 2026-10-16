# WBS Historical Notes (Implementation Records · Audits · Ruling Traces)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task table): implementation records, audit findings, ruling traces, and errata for
> each phase. **Division of labor**: 09 describes only the **plan** (Kanban, three-level task table,
> dependencies, risks, acceptance); this document only records **history** (what was done, what was
> found, how it was ruled), appended chronologically, without rewriting history.

## P2 Establishing Equivalence Criterion Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green.
> **D38 prophecy fulfilled**—the green-run process empirically measured and fixed three real ir_gen
> defects (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime relies on Frame::set_slot for fallback), un-annotated parameters discarded
> by filter_map (three places, params metadata misaligned with frame layout), match result slot
> relies on frame's implicit Void initialization (now explicitly pre-initialized). The actual
> semantics of jump targets is flattened instruction indices (not label-keyed as in 07's text), CFG
> is built at instruction level—registered in verify.rs file header. **Commit correspondence**:
> ir_gen fix = `a9ea199b`; verify body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision
> caused its message to be mis-labeled as "docs(rfc): RFC-027a…", but the content is actually
> verify.rs + six test files, buried in history's middle and cannot be amended, hereby registered).
>
> **P2 narrowing partial implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> hemostasis channel precondition unlocked**.
>
> - 2.3.2: 5 fixtures in `tests/yaoxiang-multifile/` (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract uses the same `TestFileSpec` as
>   single-file corpus (single point on the library side), binary location/subprocess launch
>   abstracted to `tests/common/` and yx_runner migrated in sync (Prohibition Three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form pinned by rfc027 tests (return position/call site/binding position)—02's
>   `Sorted(3)` annotation example would be substituted and evaluated in the current code, directly
>   yielding Disproved rather than Unproven silent, so the Sorted(3) in 07's pseudocode is
>   illustrative only. Red-state empirical data is in the test file header.
> - The two red criteria are pinned with `#[ignore]` always-on following the 2.4.5 red-skeleton
>   precedent: default suite is all green, `-- --ignored` reproduces red state; **remove the
>   attribute when P3 fix lands to turn green**.
> - The reason for the intentional deviation from the 07 (code, file, line) table in comparison
>   normalization (excluding file/line, only comparing error code sets) is written in the 2.4.1 test
>   comment.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4 and
> cannot be compiled in P2. The former is P4's 4.1.4 (avoid double registration), and the latter's
> `#[ignore]` red skeleton is merged into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: C4 mandatory coverage list item-by-item check (07 §143), 10
> semantic corpus layers fully covered and all within corpus-parity.jsonl differential baseline (26
> representative files 26/26 hits, grep verified)—ref/borrow/move 05-ownership entire directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure capture
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum type sum_type_* +
> pattern_or_guard + match_call_scrutinee, existential enforcement point interface_dynamic_dispatch
> (Vec(Animal) existential type heterogeneous wrapping triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err on diagnostic
> differential), refined constraint Drop sequence refined_* ×5 + 05-ownership driven ReleasePlan
> (span key contract has another 2.4.4 red criterion always-on). Performance baseline
> `benches/pipeline.rs` included (07 §145 second of three smoke tests): `corpus_compile_frontend`
> full corpus first compile test 17.6s, `cli_cold_start_hello` process cold start 169.7ms;
> interpreter throughput is already covered by benches/lib.rs hotpath group (interp_fib_recursive_27
> / interp_loop_steady_10m), no duplicate construction. Regression gate check-perf-regression.sh
> assigned to P9 unified list (2.5).

## P3 Correctness Vulnerability Fixes (Minimal Plan)

> **Already-landed row error type annotations** (retrospective annotation, not changing history):
> 3.1 = B ｜ 3.2 = B (defensive empty arm, criterion green-turning borne by 3.1) ｜ 3.3 = A ｜ 3.4.1
> / 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D
> (user-visible, already ruled by user) ｜ 3.4.8 ①② = C (main) + A (incidental three places) ｜ 3.5
> / 3.6 / 3.7 / 3.8 = D (all already ruled by user).
>
> **P3 Diagnosis (2026-10-12, user request)**: A diagnostic review in response to the perception
> that "P3 entries keep growing".
>
> **One, first clarify the scale (perception is amplified by two accounting defects)**
>
> 1. **Duplicate numbering**: `3.5.x` is shared by two task families—(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level: lexical and parser branches / AST `is_pub` / checker
>    side / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10 registered **3.5 Higher-Kinded Type
>    parameter call path completion** (3 three-level). Both share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 has been re-numbered per user ruling: 029g family changed to
>    `029g.x`**), and caused 029g's rows to be mistakenly counted into P3. **Pending ruling**:
>    rename the 029g family to `029g.x` (or move it into its proper phase), and synchronize other
>    references in the document.
> 2. **Missing checkmarks**: 3.7's 4 three-levels (3.7.1–3.7.4) are all completed (d922a551 /
>    38bd4b9e) but not checked ⇒ this entry is corrected here.
>
> **Two, P3 Actual Account (12 second-level rows)**
>
> | Row                                                                   | Status                                                                                                                                                                             | Disposition                                                                                                                        |
> | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                       | Completed                                                                                                                                                                          | —                                                                                                                                  |
> | 3.4 Silent Discard Point Addendum (8 three-level)                     | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlocked with 4.3.1                                                                         |
> | 3.5 Higher-Kinded Type parameter call path completion (3 three-level) | Not started (declaration side live, call side broken)                                                                                                                              | Keep P3; **does not block refactor main line**, but is "accepted feature not implemented", priority higher than pure cleanup items |
> | 3.6 Function type value call path (1 three-level)                     | Not started (semantics already cut (i))                                                                                                                                            | **Keep**: correctness defect, clear criterion                                                                                      |
> | 3.7 init segment call name qualification (4 three-level)              | **All completed**                                                                                                                                                                  | Check and close this time                                                                                                          |
> | 3.8 native shadowing (1 three-level)                                  | Not started                                                                                                                                                                        | **Stuck on decision** (first determine name resolution priority), not a scheduling issue                                           |
> | 3.9 / 3.10 / 3.11                                                     | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                                  |
> | 3.12 Runtime mode-to-mode executor config propagation (1 three-level) | Not started (D class, user ruling B: separate team)                                                                                                                                | **Recommend move out of P3 → P8 (execution layer)**, pending ruling                                                                |
>
> **Four, 4.3.1's Leverage (Main Line Landmark)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the precondition for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ take it once to advance the main line and clear two P3 stranded items. 3.5
> (HKT), 3.6 (function type value call), and 4.6.1/4.9.1 have **no unsatisfied preconditions** and
> are doable anytime; 3.8 is stuck on semantic decision (user-defined first vs. explicit
> diagnostic).
>
> **Three, Mechanism Recommendation (Prevent Recurrence)**: Newly registered second/third-level rows
> **must carry two fields**— "Belonging phase" (which P it really belongs to) and "Whether it blocks
> the main line"; `registered pending scheduling` is no longer the default state, unmarked is
> considered unschedulable. This way inflation is visible and sortable, not indiscriminately piled
> up. **Pending ruling**: whether to write this into this document's registration convention and
> `src/dev/coding-rules.md`.
>
> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter (at
> the time recorded as `pub(crate)`; **2026-10-07 verified actual implementation is
> `pub(super)`**—the `#434` ruling referenced by the code comment has no registration on the docs
> side, see 3.4.3; full privacy requires rewriting 11 construction literals, not taken at the
> hemostasis stage; will be sealed during P4 Obligations migration); execution mechanism abstracted
> to `src/frontend/proof_execution.rs` (Prohibition Three single point—cannot be placed in
> typecheck/, that would create L3→L4 reverse dependency); 3.1.2 according to 02 evidence chain step
> 5 supplemented **four entry points** (WBS row number missing compile_project `:99`, supplemented
> and registered as required by 3.1.1's acceptance "criterion green-turning"); 3.2.1 empty arm
> changed to bookkeeping (proof_calls upward throw) + diagnostic (`into_result` established
> conversion path); 3.1.3 SOLVER slot Option-ized, Z3 missing gracefully degrades per
> `SMTResult::Unknown` (E2031 family diagnostic replaces process crash). Two vulnerability criteria
> remove `#[ignore]` to turn normal green, supplement corpus fixture `proof-obligation-honored`;
> check-fanout inventory migrated accordingly (wiring point 19 = inventory). Empirical: criteria
> 2.4.1/2.4.3 turn green, full suite lib 2494 / integration 272 / corpus dual-layer all green.
>
> **3.4 Audit (2026-10-07, P3 Implementation Post-Review)**: For the category of "failure produces
> no signal" defects (definition in 02 §41-52), re-inventoried on unmodified code, located 8 more
> places, all registered as 3.4.x in the above table. Method and evidence:
>
> - **Consumer-side inventory** (16 obligation fields one-by-one grep full repository read points):
>   `variant_ctor_calls` **zero consumers**—the only read `checker.rs:1468` is "self-injected into
>   `TypeCheckResult`", IR side goes through shape detection `ir_gen.rs:7374`; `rfc011b.rs:659/688`
>   only asserts the production side, which is exactly the "only tests production side, naturally
>   immune to consumer-side defects" pattern critiqued in 02 §186 → 3.4.4.
>   `method_overload_ir_names` result-level field also has zero external consumers, IR uses parallel
>   `overload_resolutions` → 3.4.5.
> - **Phase Coverage Review**: 02 §91 table row 1 (four entry points "no proof_execution") fixed by
>   3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 current state unchanged (multi-file `run`
>   still does not output W1003, does not run dead code family) → 3.4.6; LSP non-project-internal
>   path (02 §77 "manual lex→parse") also does not run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` full repository consumers only four places): three
>   checker refined branches (covered by 3.1), 3.2.1's fixed ownership arm (**source code
>   unreachable**), and `environment.rs:422` (**reachable, zero diagnostic, zero bookkeeping, and
>   zero registration in full repository docs**) → 3.4.1, the only "all-unknown" item in this audit.
> - **Solver acquisition side**: `default_solver()` three consumer forms are different
>   (`predicate.rs:32` process singleton / `checker.rs:1266` injected each `check_module` /
>   `ownership.rs:621` newly created each back-edge judgment)—3.3.1 only eliminated one panic; the
>   other two's **silent face** goes to 3.4.2, **singleton-ization and cache hit** goes to P4's
>   4.4.2 (avoid double registration).
> - **Not counted in 3.4, existing attribution**: 2.4.4 (`ReleasePlan` span key mismatch, 168 files
>   / 373 keys not consumed, Drop silently lost) still P2 red-state always-on, attribution 4.5.1/D20
>   unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not in
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4.
>
> **Execution Convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions, can be landed in place
> anytime (recommend doing 3.4.1 first—the only "full repository zero registration" living
> vulnerability, smallest change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on
> obligation ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish new criterion
> system**, all reuse existing C2/C3 and P9's 9.2 gate.

> **Batch 3.13–3.16 Implementation Record (2026-10-13 registered → 2026-10-15 closed)**: User
> 2026-10-14 ruling merged four related items (#451 method binding panic / #452 named actual
> parameter in resolution / #453 dead inference entry / #454 native export named actual
> parameter—the last item is a boundary finding when verifying 3.14, same day attribution experiment
> verified **not introduced by #452**, user ruling ① merged into this batch) into one team for
> handling. Closure status: all four items completed, ruling registered as RFC-039 **D60** (six
> binding rules + shared criterion single point).
>
> **One, Two Contract Corrections (Lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: #452 task contract simultaneously required "mixed
>    actual parameters output 20" and "position and name competing for same position error", but the
>    pinned board fixture `Temp.pick(self, a)` only had one value parameter—the same actual
>    parameter combination is both "mixed" and "conflicting". The first implementer honestly
>    reported the blocker and set the task to failed, overall revert (no silence for green light).
>    Disposition: task reassigned + contract revision—gave **six binding rules** and **8 value-level
>    matrix** (fixture changed to two value parameters `pick(self, n, a)`) then attempt 2 was
>    completed by the second implementer. Full text of the six rules has been incorporated verbatim
>    into D60.
> 2. **inScope missed mechanically required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered only `code-tables --fix` was
>    run, **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964**—the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touched `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **Two, Review → Fix → Re-review Two Closed Loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls containing named actual parameters have silent whole chain
>    in both multi-candidate zero-hit/single-candidate hit paths) → fix round added new external
>    code **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece
>    same commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm settled by name) → independent
>    verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations mechanically
>    reversible replayed via `scratch.py trial` all caught, restoration byte-by-byte consistent) →
>    round-1 review needs_revision (F1 = typecheck native arm **inlined copy** of binding criterion,
>    missing rule ④ ⇒ **ruling divergence** for same-shape method and native calls—method reports
>    E1111, native silently exit=0; ir_gen has another third existing copy reporting old code E1015)
>    → convergence round collapsed criterion to single implementation `bind_args_to_params`
>    (`arg_binding.rs:35`, four call points `expressions.rs:2727` / `:5416`, `ir_gen.rs:7776` /
>    `:8279`) → round-2 review caught the **regression** introduced by the convergence round (IR
>    convergence block lost native optional position ⇒ previously successful program omitting
>    optional position was misreported as E1010) → fix round (optional position criterion and
>    parameter name from single exit `signature.rs:236`) → round-3 review PASS (independently
>    reproduced red→green; under mutation, repository pinned board FAILED verified criterion can
>    fail). **Lesson**: review must cover "unfixed branches of similar paths" (both needs_revision
>    in this batch were caused by probes all falling on hit face); fix rounds themselves can
>    introduce regressions (optional position), caught by next round's different reviewer.
>
> **Three, Process Events (Honestly Registered, Not Pointing to Individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: Working tree was silently restored
>    externally—the implementer's fix-in-progress (typecheck/ir_gen/code table/locales/RFC-013 etc.)
>    all disappeared, **at the same moment the user's own two edited review documents also
>    disappeared from git status** ⇒ judged as repository-level discard (most likely user-side
>    manual cleanup/IDE "discard all changes"). Three evidence excluded team members: stash empty,
>    no new commits, reflog no reset. Implementer replayed the in-progress work in context and
>    pushed; discipline changed to "**save patch to scratch first, then take red state**".
> 2. **Verifier ran out of context, changed person**: The independent verifier verifying 3.16 ran
>    out of room midway (no closing message), changed person per load rules—the successor continued
>    using their already-ready products (green state result, corpus equivalence script, mutation
>    plan), verification ultimately PASS. Positive precedent of partial product handoff.
> 3. **Review Round 5 Rotation and Historical Round Retreat**: When rotating reviewer per load upper
>    limit rule ⑤, `remove_member` retreated the already-adjudicated final-state review/verification
>    rounds **together back to shared pool**, scheduler immediately re-ran (t3/t8 were taken by new
>    members again, duplicate work and blocking downstream). Disposition: team leader took over and
>    cancelled retreated historical rounds (t3/t8/t13), accordingly added AGENTS.md rule 12 (handle
>    retreated rounds immediately after person change). This batch closure record is registered per
>    this rule.
> 4. **scratch governance (about 6.3 GB)**: Early batch each task one external tree, 6 directories
>    scattered outside repository (about 780 MB, cleaned) + two members' external verification trees
>    (about 2.2 GB / 3.3 GB, self-deleted after use). Output governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md rule 9
>    (batch tree single entry), rule 10 (verification environment six rules: mechanically reversible
>    mutation / line number from referenced revision / CI unique gate must run etc.), rule 11
>    (writer load upper limit), rule 12 (handle retreated rounds after person change) and top
>    meta-rules two (no private content / verify before asserting).
> 5. **Contract defect (not pointing to members)**: #454 fix round's contract inScope only circled 2
>    files, actual mechanically required surface 8 (rule 6 is exactly to prevent this—contract
>    template should pre-place mechanically required surface list); also that task's terminal state
>    field was once occupied by probe payload, formal acceptance/command evidence was supplemented
>    by team leader per convention.
>
> **Four, Leftover Items (Not Fixed This Batch, Registered for Future Reference)**: round-3 review
> three low-severity findings—① method path's `build_arg_slots` (`ir_gen.rs:521` area) is still the
> third implementation of the criterion (existing, per previous round's caliber registered at
> closure, not converged); ② batch verification tree (worktree) internal `cargo fmt --all` error is
> worktree workspace artifact, gate list should note fmt goes to main repo; ③ repair class contract
> template should pre-place mechanically required surface in inScope (see Three.5).
>
> **Five, Final Gate (Closure Revision, Local Two Implementation Commits + One Document Commit
> Unified Push—This Batch's Only Push)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib`
> **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner`
> 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> lines byte-by-byte consistent with baseline** (only regenerated line: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line number synchronized, manually reviewed) / IR snapshot
> zero drift.
>
> **Six, Run Links (Push-side Evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test cancelled by subsequent push
> per concurrency—cancellation ≠ failure, covered by later run); governance tool push → CI
> 38546373435 / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) →
> CI 38528560364. Closure push's own CI / Deploy Docs run registered in task record after push.

> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ Append `Linking` at the tail of
> `CHECK_STAGES` (**pure merge**, entire block skips `is_bin_role` entry check—E3020 criterion still
> belongs to `RoleClassification` to prevent double reporting; another hard reason: run path judges
> "has main" per merged IR, under collection semantics "entry itself errors → no IR" would be
> misjudged as missing main and hard-aborted) → `Monomorphization` (checks shape);
> `ir_generation_check` retains unit IR (**in-memory only**, for pure merge and mono consumption,
> not externally emitted / not written to disk / not producing snapshot—without retention,
> aggregation request is empty, mono gives `NoObligations` false green); mono failure in Check goes
> through per-file collection channel (diagnostic attributed to entry file + `failed_stages`, not
> set to failure), MultiFile keeps `fail_with`; `into_outcome` changed to "first take collection
> then judge failure". ④ Empirically verified **zero production changes**: full program BFS already
> covers cross-unit mutual recursion (pathological → E3005, well-formed three-layer chain executes
> correct value), delivers four groups of e2e + one mono unit test as living criterion.
>
> **Verification Chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized boundary),
> t10 (F1 fix, 11 examples + dual binary comparison) all pass; t5 cross-shape matrix
> (single/multi/check three shapes + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase, no decrease).
>
> **F1 (Found in Verification Phase, D59 A/B Class, Fixed in Same Round)**: Under standalone
> multi-file check without project root, aggregation request name (`lib.f`) and unit IR definition
> key (`f`) are not from the same source → mono falls into silent `continue` of "native has no IR
> definition", check reports 0 error/exit 0. Fix: aggregation side adds reverse lookup by each
> unit's **real definition key** (precise enumeration + exact character match + ambiguity
> abandonment, not suffix heuristic; project shape not registered, existing path unchanged).
> **Erratum (Same Day)**: This reverse lookup was **deleted** in subsequent regression fix; root
> cause solved from naming caliber by `module_key` dual-use decoupling (new `script_mode`)—IR
> definition key = IR call name = aggregation request name = `{unit key}.{name}`. **Regression
> Record**: After ③ landed, CI's `Check tutorial/guide code examples compile` red (258 blocks / 10
> failures: 6 same-file same-name rebinding + 4 collided with `<std/list>`, rootless check does not
> do qualification triggered `link_module_irs`'s conflict check); fix = ir_gen split `script_mode`
> (qualification name ⊥ Script/Bin, E3023 gate changed to look at it) + `ir_generation_check`
> per-unit constant-pass unit key + `.with_script_mode(rootless)` + Script top-level same-name
> rebinding "latter wins" (consistent with run path's existing behavior); verification =
> documentation gate 258/0, three-binary comparison (HEAD/③ tree/fix tree) three types of collision
> fixtures, check shape E3005 no regression, E3023 not opened.
>
> **F2/F3 (Accepted)**: mono diagnostic attributed to entry file entry, span can cross units
> (consistent with run path's `path=entry` caliber); Check shape holds one ModuleIR per unit
> (in-memory only, released at check end).
>
> **Issues Left for Follow-up (Not This Round's Regression)**: ① E3005's help text still writes
> "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of E3005)—registered pending
> scheduling; ② multi-file E3005's span rendering label misalignment (already reproduced with HEAD
> binary, pre-existing); ③ request collection side has **type parameter position dependency**
> (generic calls where type parameters are not at value parameter positions do not emit
> instantiation request, mono records `NoObligations`, convergence mutual recursion is this
> form)—same face as 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Remove `pub` and Auto-binding (#399

> **P3.5 Implementation (2026-10-08)**: All seven second-level tasks completed, entire batch one
> commit (independently revertible).
>
> - **Measurement differs from plan (honestly registered)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—actually after removing `KwPub`, `pub`
>   reverts to ordinary identifier, the old writing falls to semantic layer reporting **E1001
>   Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically detecting
>   removed syntax is anti-pattern", **do not add** migration-specific diagnostic; corpus fixture
>   `pub_declaration_err.yx` pins E1001, document (`syntax.md`) honestly writes this consequence.
> - **Design Convergence (Beyond RFC Letter, Requires Review)**: After `pub` removal, `project_refs`
>   and `cross_file_refs` two fields' semantics completely overlap (same "package-internal reference
>   union", Prohibition One prohibits parallel representation) → merge into single `project_refs`,
>   `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally, Internal role's
>   non-pub items change from "file-internal reachability" to "reference pool judgment" (relaxed, in
>   the direction of false negatives); Lib tightens from "pub absolute exemption" to reference pool
>   judgment; Script top-level bindings not referenced start reporting W1001/W1002/W1004/W1005.
> - **Dead Table Cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports write-back all deleted (production side fills
>   table, full repository zero consumption).
> - **Test Migration** (test count does not decrease): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer a modifier"; `passes/tests/dead_code.rs` 7 role semantic use cases rewritten (2 assertion
>   reversals); `tests/integration/cli_e2e.rs` 8 role use cases rewritten (`lib`/`internal`
>   exemption assertion reversed to report assertion).
> - **Baseline**: IR snapshot **zero-diff** (`pub` does not participate in IR); corpus differential
>   baseline only added 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated then manually reviewed.
> - **Empirical**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS checkmark / gate / push) completed by t7. Precondition = 4.3.1 obligation
> ledger (red state filing 173 files / 533 keys).
>
> **One, Carrier Ruling (A: `PlanId` goes with AST node)**: Added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` represents synthetic nodes
> outside the parser), issued by **unique allocator** `ParserState::alloc_plan_id`; carrier two
> places—`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call point identity),
> `Expr::call` is the full repository's only call constructor. What was rejected is the "both sides
> derive same identity per span" position coincidence form (D20's original defect).
>
> **Two, Two-Level Verification and Review (Members Independent)**: Each functional commit paired
> with an independent verifier and a reviewer, **verification/review does not reuse implementer's
> evidence**: 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations)
> / t3 review; 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations)
> / t6 review. Review time point and reviewed revision both fall in immutable history (`git show`
> verifiable). **Provider rename caused t5 / t6 to change members for continuation**: original
> verification/review members could not continue due to provider rename, per scheduling convention
> changed person and continued dispatch **with reason written in record** (cross-round context
> preservation priority, only change person when compression unavailable); after continuation,
> conclusion consistent with original round (PASS), did not reuse implementer's evidence.
>
> **Three, Two Defects Found and Fixed During Self-Check in Implementation (t1, Both Within
> inScope)**: ① Consumption exemption criterion was once attached to
> `UnitState.ir.is_some()`—`check` shape's Linking/Mono will take `unit.ir` away, real
> non-consumption in check shape **silently passes** (mutation empirical: D41 red but CLI check exit
> 0); changed to read **the ledger's own completion mark**, and supplemented regression use case. ②
> Drop sequence drifted with process hash seed (partial order `sort_by` + `HashSet` iteration):
> snapshot criterion ran three consecutive times producing 106/120/114 line count drift; changed to
> **total order sort** (same name takes maximum depth + name ascending) then ran twice consecutively
> with zero drift.
>
> **Four, Criteria and Evidence Numbers**: After 4.5.1, `test_release_plan_spans_consumed` removed
> `#[ignore]` always-on (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in
> same commit (programmatic review: added 1083 lines Drop + 86 lines jump index translation, zero
> deletion, no rearrangement), corpus 342 differential empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference set empty + evidence face floor `examined ≥ 1` /
> `consumed ≥ 2`; empirical examined=1, consumed entries 2, no resolution skipped 210), resolution
> form 9 pinned boards, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation-driven: consumption key offset by one ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; restored then re-greened.
>
> **Five, Semantic Modification Classification Basis (D59)**:
> `Type.method(positional actual, name = value)` before change **silently gave wrong answer**
> (empirically compiled clean, ran output `1` instead of `20`) ⇒ **Class C** (old behavior not
> successful), fix takes "loud": after change, same program compile-time `error [W1090]`; "named
> actual parameter participates in resolution" final state semantics belong to **Class D**,
> registered §3.14 and left issue #452 for dedicated team to undertake. Additionally: `Drop` is
> no-op in interpreter (`backends/interpreter/executor/ops/control.rs` wildcard arm only
> `advance()`)—key migration does not change runtime behavior.
>
> **Six, F-1 / F-2 Disposition (Found by t5 Independent Verification)**: **F-1 (medium)** no binding
> consumption point (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, key
> correctness has no red state to falsify ⇒ per "known boundary" registered into
> [02](02-stage-contract.md) §2 revision note fifth paragraph (retention reason + do not open new
> task); **F-2 (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` documentation comment
> changed to precise pointer (produced `inference/expressions.rs:4550`, consumed `ir_gen.rs:7697` /
> `:7772`), separate small commit, not mixed into document commit.
>
> **Seven, Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement marks);
> 07 criterion table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 /
> 4.5.2; CHANGELOG recorded one line behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Bookkeeping Deviation Found at Closure**: pointers
> `ir_gen.rs:357` / `:3102` / `:1096` in t3 / t6 reports differ from code fact by one to three lines
> (actually `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2's insertion)—registered per code fact,
> this note also retains trace.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (4.2.5 Ruling A's run Half)

> **Batch C Record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15 ruling C—this
> round only does 4.10.3 (same batch candidate 3.4.6 per B1 ([02](02-stage-contract.md) §Runtime
> Behavior Table "dead code family warnings do not enter compile path") deferred to after 4.9,
> precondition re-attached and noted); implementation form = **complete single team** (user
> instruction not to open DAG, not to build team, single-session implementation +
> self-verification). Pre-batch arrears first landed: #455 dependency announcement fix (rustls
> 0.23.46 + serde_yaml_ng, `cargo audit` restored from exit 1 to exit 0), WBS registration debt
> (#456 registered as 3.17 + incomplete row supplemented "belonging phase / whether blocks main
> line" fields).
>
> **One, Red Before Green**: driver-level red criteria 3
> (`src/driver/tests/run_rootless.rs`—rootless merged IR must not carry entry_function (red state
> `Some("main.main")`), rootless top-level executable statement must not report E3023 (red state
> does not produce module), Bin comparison entry_function maintained (green state always-on)) + CLI
> e2e 3 pins (standalone relative import run (red state E5001), Script top-level statement execution
> and #413 hint two green state pins).
>
> **Two, Implementation Side**: CLI `run` unified through `compile_project` (deleted
> `run_file_with_diagnostics`'s project/standalone two-way fork); Linking's `entry_function` gated
> per `is_bin_role` (#413 T4—Script does not auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call point project probe (after unification ModuleIR always
> carries source_files, original mark invalid); `script_main_never_called` adapted to qualification
> name form (function `{key}.main`, global slot empirically also qualification name—based on IR
> evidence, not inferred from comment).
>
> **Three, Existing Gaps Exposed by Unification (Same Batch Incidental Fix, A/B/C Class—only fixing
> failing programs to succeed, zero impact on originally successful programs)**:
>
> 1. **Explicit type actual parameter pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3
>    directly emits specialization name (mono will strip unspecialized original),
>    `resolve_call_target` three arms (rename / aliases / dot prefix) do not cover `mk(int64)` form
>    ⇒ call point dangling E6006. Fix = that single-point parser adds fourth arm (base name
>    qualification, type parameter string preserved as-is). **Bin project same-shape reproduction
>    (pre-existing defect, not introduced by this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign same
>    shape as declaration) statement by statement ⇒ same variable allocated 3 slots, initialization
>    writes the later-allocated slot while reading uses the earlier-allocated slot (runtime Add type
>    mismatch). Fix = both places merge by name (same criterion same source).
> 3. **Same-module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table is actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidates + table lookup verification (precise hit →
>    generic's unit key qualification hit → fallback to bare name current state; not suffix
>    heuristic, same form as `resolve_generic_name`).
>
> **Four, Registration Gap (Not Fixed This Batch)**: Constraint dispatch still E6006 when **type and
> generic definition are in different modules** (rootless two-file probe empirical; Bin same
> shape)—root cause = MonoType's user type name does not carry definition module, correct fix
> belongs to P6 type identity convergence; registered WBS **3.18** (belongs to P3, precondition
> pending P6, does not block main line), `qualify_deferred_method` note same face.
>
> **Five, Test Adaptation (Honestly Registered)**: `tests/integration/multifile.rs` fixture
> supplemented manifest—what it tests is RFC-029 project orchestration (Bin auto entry) not Script
> semantics; `cli_e2e` one pin (top-level diagnostic variable name visibility) and
> `script_main_hint` fixture (7 examples) synchronized per qualification name
> form—production-consumer side only eats merged IR, fixture directly drives same form, no pipeline
> product placeholder used.
>
> **Six, Final Gate (Closure Revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> differential baseline regeneration **12 lines**—line-by-line structured check: exit / stdout /
> diagnostic code all identical per entry, only stack frame function name and variable name
> qualified per unit key (manually reviewed); after regeneration `check-corpus-parity.py` exit=0.
>
> **Seven, Process Events (Honestly Registered)**: This batch once concurrently edited workspace
> documents during pre-commit hook running period, eaten by hook's stash rollback and that commit
> failed (concurrent form pre-announced in configuration comment)—discipline = do not touch
> workspace during hook running period; #455's commit therefore split into two attempts (second all
> green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 instructed to
> push forward in order 4.6 → 4.9 → 4.4 → P5, do 4.6 first; implementation form = **complete single
> team** (following previous batch user instruction: not open DAG, not build team, single-session
> implementation + self-verification). Task definition (09 three-level table): project-internal
> file's semantic data (SemanticDB) goes through orchestrator same pipeline, same source as
> diagnostic; cross-file reference carries `resolves_to`. Acceptance: project-internal cross-file
> jump hits definition; semantic data and diagnostic from same compilation.
>
> **One, Current State Empirical (Verify Before Asserting)**: didOpen/didChange triggers **two
> passes** of compilation—diagnostic goes through unified Driver pipeline (4.2.3/4.2.6 already
> unified), semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registration table ⇒
> project-internal `use` parse fails, cross-file reference events not produced,
> jump/rename/reference finding silently fails for imported symbols—D55 registered silent failure).
> Orchestration arm's checker unified naming `"<module>"` (SemanticDB file identity lost),
> `type_result` (containing semantic_db) discarded when diagnostic non-empty and never externally
> emitted; `checker.rs` draining uniformly covers `resolves_to`'s DefId file identity as current
> file; `Export`/`ModuleInfo` both lack definition position surface.
>
> **Two, Red Before Green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`)—after temp two-file project
> didOpen main.yx, GotoDefinition sent to `add_one` call point, **before fix returns None (red,
> empirically previous section's silent failure)**, after fix hits lib.yx line 0 definition (green).
> One probe localization during process: compiler side link (driver probe dump) already fully
> connected while e2e still empty—root cause is `resolve_reference` precisely matches per reference
> **starting point** (D55 precise-only current caliber), test cursor falls in identifier middle;
> this tolerance belongs to existing behavior surface, this batch does not touch (see Seven).
>
> **Three, Implementation Side**: (1) `Export.definition_span` (extract_module_info harvested from
> AST identifier node, same source and value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled per DiscoveredFile, including
> vendor; std always None); (2) checker reference draining via `import_def_locations` (use line
> binding span → definition file + definition span) redirects cross-file imported items'
> `resolves_to` out of this file; std import unregistered, maintains falling to use line; (3)
> checker module identity changed from unified `"<module>"` to unit file path (empirically
> `env.module_name`'s all 4 consumer points zero diagnostic consumption); (4)
> `DriverOutcome.semantic_db` channel—CollectAll form (Lsp target file **regardless of diagnostic
> success or failure** + SingleFile fallback) converges, Lsp arm tail from registry injects
> non-target file top-level export definition surface (`external_definitions`, even unopened files
> can jump; type/submodule exports not injected—typecheck side type registration does not produce
> definition event, both sides maintain consistent vacancy); (5)
> `orchestrator::analyze_source_in_project` (`check_source_in_project` transferred to take
> diagnostic half); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key unified normalization file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId thus stably hits); 5 handlers +
> semanticTokens dual requests receive `db_key`.
>
> **Four, Incidental Fix (D59 Diverge Class B—old behavior silently does nothing, originally
> successful programs' external performance zero impact)**: ① project-internal file cross-file
> jump/rename/reference finding from silent failure to hitting real definition; ② old manual
> pipeline when lexical error deleted semantic information (staining disappeared) → incomplete AST
> staining preserved (4.2.6 editor philosophy extension); ③ when target file has type errors
> semantic data no longer discarded along with `type_result` (typing intermediate state
> staining/jump not disappearing).
>
> **Five, Pins**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`—target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback use line, error file preserved, single-file fallback path key, single-file
> error preserved) + LSP e2e 1 (red before green) + uri normalization 4
> (`src/lsp/tests/uri.rs`—drive letter conversion, space encoding, idempotent normalization,
> non-file scheme pass-through).
>
> **Six, Mutation Verification (scratch.py trial, Mechanically Reversible)**: Batch tree wbs-46 @
> closure revision 39546020, two mutations both made hit pins turn red, after restoration blob
> consistent with HEAD + git status CLEAN—m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver pin + LSP e2e pin red, other 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two pins red (round-trip parsing loses anchor),
> other 7 green. Batch tree gc'd (released 4.8 GB, `status` no batch tree).
>
> **Seven, Final Gate (Closure Revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> differential **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS
> (13·13·0·8, `semantic_db` non-obligation reason consumer anchor synchronized server.rs:482 and 02
> §Obligation Field Table) / check-docs-truth 0 / docs-examples 258 entries 0 failures / docs-orphan
> 0 / markdownlint whole repository only user's uncommitted 032 two MD056 left (user's own editing,
> not touched) / pnpm docs:build 0. **Process Events (Honestly Registered)**:
> en/09a-wbs-history.md:54 one MD037 was introduced by auto-translate bot re-translating last batch
> 09a (zh original has no space), this batch fixed it back—markdownlint red is not this batch's
> change surface but in same file as 09a, registered together. **Leftover Observation (Registered
> Pending Discussion, Not Touched)**: `resolve_reference` only precisely matches per reference
> starting point—when cursor falls in identifier middle, jump/hover/rename silently does nothing
> (D55 precise-only current state, whole file form is isomorphic), if tolerance matching is needed
> it is an independent behavior change, report to user for ruling.
