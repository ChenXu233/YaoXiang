# WBS Historical Notes (Landing Records · Audit · Ruling Traces)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (Construction Task Table): landing records, audit findings, ruling traces, and corrections for
> each phase. **Division of labor**: 09 only describes the **plan** (Kanban, three-level task table,
> dependencies, risks, acceptance); this document only records **history** (what was done, what was
> found, how it was ruled), appended chronologically, without rewriting history.

## P2 Establishing Equivalence Criterion Baseline

> **P2 Landing (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green run.
> **D38 prediction fulfilled**—the green run process measured and fixed three real defects in ir_gen
> (details in 06 §G6): method locals table truncated to parameter count (local_count underestimated,
> runtime relies on Frame::set_slot for tolerance), unannotated parameters discarded by filter_map
> (three places, params metadata misaligned with frame layout), match result slot depends on frame
> implicit Void initialization (now explicitly pre-initialized). The actual semantic of jump target
> is flattened instruction index (not the label-keyed form in 07 text), CFG built at instruction
> level—registered in verify.rs file header. **Commit comparison**: ir_gen fix = `a9ea199b`; verify
> main body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its message to be
> mislabeled as "docs(rfc): RFC-027a…", actual content is verify.rs + test six files, buried in
> history middle and cannot be amended, hereby recorded).
>
> **P2 Narrowed Parts Landing (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3 stop-bleeding
> channel prerequisite unlocked**.
>
> - 2.3.2: 5 fixtures in `tests/yaoxiang-multifile/` (4 behavior + 1 compile-error) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpus (library side single point), binary location/subprocess launch extracted to
>   `tests/common/` and yx_runner migrated synchronously (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses the `SumUpTo(3, r)`
>   form nailed by rfc027 tests (return position/call site/binding position)—the `Sorted(3)`
>   annotation example in 02 would be substituted and evaluated directly to Disproved in the current
>   code, not showing Unproven silently, the `Sorted(3)` in 07 pseudocode is for illustration only.
>   Red state measured data is in the test file header.
> - Two red criteria are marked `#[ignore]` per the 2.4.5 red skeleton precedent: default suite all
>   green, `-- --ignored` reproduces red state; **when P3 fix lands, removing the attribute turns
>   green**.
> - The reason for the intentional deviation from the 07 (code, file, line) table in comparison
>   normalization (removing file/line, only comparing error code sets) is written in 2.4.1 test
>   comments.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **removed from P2**: the `Program` / `Obligations` types they reference only exist in P4, P2
> cannot compile. The former is P4's 4.1.4 (no double registration), the latter's `#[ignore]` red
> skeleton is merged into 4.3.1.
>
> **2.3.3 Landing (2026-10-07)**: C4 mandatory coverage list checked item by item (07 §143), 10
> categories of semantic corpus layers fully covered and all within corpus-parity.jsonl diff
> baseline (26 representative files 26/26 hits, grep verified)—ref/borrow/move 05-ownership full
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for and other 16 files, sum types
> sum_type__+ pattern_or_guard + match_call_scrutinee, existential forcing point
> interface_dynamic_dispatch (Vec(Animal) existential type heterogeneous packaging triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (+
> method_overload_ambiguous_err in diagnostic diff surface), refined constraints Drop sequence
> refined__ ×5 + 05-ownership driven ReleasePlan (span key contract has another 2.4.4 red criterion
> always-on). Performance baseline `benches/pipeline.rs` checked in (07 §145 three smokes two):
> `corpus_compile_frontend` full corpus compilation first measurement 17.6s, `cli_cold_start_hello`
> process cold start 169.7ms; interpreter throughput is covered by benches/lib.rs hotpath group
> (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate construction. Regression gate
> check-perf-regression.sh attributed to P9 unified list (2.5).

## P3 Fix Correctness Vulnerabilities (Minimum Solution)

> **Error type annotation for landed rows** (retrospective annotation, does not change history): 3.1
> = B ｜ 3.2 = B (defensive empty arm, criterion turns green by 3.1) ｜ 3.3 = A ｜ 3.4.1 / 3.4.2 = A
> (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D (user-visible
> surface, already user-ruled) ｜ 3.4.8 ①② = C (main) + A (incidental three places) ｜ 3.5 / 3.6 /
> 3.7 / 3.8 = D (all user-ruled).
>
> **P3 Diagnosis (2026-10-12, user requested)**: Conducting a triage of the impression that "P3
> entries continue to grow".
>
> **I. First Clarify the Scale (impression magnified by two accounting defects)**
>
> 1. **Numbering collision**: `3.5.x` is shared by two task families—(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level: lexical and parser branches / AST `is_pub` / checker
>    side / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10 registered **3.5 Higher-order type
>    parameter call path completion** (3 three-level). They share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 already renumbered per user ruling: 029g family changed to `029g.x`**),
>    and 029g's rows are miscounted into P3. **Pending ruling**: rename the 029g family to `029g.x`
>    (or move it to its belonging phase), and synchronize other references in the document.
> 2. **Missing check**: 3.7's 4 three-level (3.7.1–3.7.4) are all completed (d922a551 / 38bd4b9e)
>    but not checked ⇒ this entry is corrected in this round.
>
> **II. P3 Actual Accounting (12 second-level rows)**
>
> | Row                                                                          | Status                                                                                                                                                                             | Disposition Suggestion                                                                                                                |
> | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                              | Completed                                                                                                                                                                          | —                                                                                                                                     |
> | 3.4 Silent discard point supplements (8 three-level)                         | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlocked with 4.3.1                                                                            |
> | 3.5 Higher-order type parameter call path completion (3 three-level)         | Not started (declaration side live, call side broken)                                                                                                                              | Keep P3; **does not block main refactoring line**, but is "accepted feature not implemented", priority higher than pure cleanup items |
> | 3.6 Function type value call path (1 three-level)                            | Not started (semantic already cut (i))                                                                                                                                             | **Keep**: correctness defect, criterion clear                                                                                         |
> | 3.7 init segment call name qualification (4 three-level)                     | **All completed**                                                                                                                                                                  | Check and close this round                                                                                                            |
> | 3.8 native shadowing (1 three-level)                                         | Not started                                                                                                                                                                        | **Stuck at decision** (first define name resolution priority), not a scheduling issue                                                 |
> | 3.9 / 3.10 / 3.11                                                            | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                                     |
> | 3.12 Runtime mode-to-mode executor configuration propagation (1 three-level) | Not started (D class, user ruling B: start a separate team)                                                                                                                        | **Suggest moving out of P3 → P8 (execution layer)**, pending ruling                                                                   |
>
> **IV. The Lever of 4.3.1 (Main Line Landmark)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the prerequisite of **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ doing it once advances the main line and clears two P3 stranded items. 3.5
> (HKT), 3.6 (function type value call) and 4.6.1/4.9.1 have **no unsatisfied prerequisites**, can
> be done anytime; 3.8 is stuck at semantic decision (user-defined priority vs explicit diagnostic).
>
> **III. Mechanism Suggestion (prevent recurrence)**: Newly registered second/third-level rows
> **must carry two fields**—"Belonging phase" (which P it truly belongs to) and "Whether blocks main
> line"; `registered waiting for scheduling` is no longer the default state, if not annotated it is
> considered unschedulable. This way expansion is visible and sortable, rather than undifferentiated
> accumulation. **Pending ruling**: whether to write this into the registration convention of this
> document and `src/dev/coding-rules.md`.
>
> **P3 Landing (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter (recorded as
> `pub(crate)` at the time; **2026-10-07 verified actual landing as `pub(super)`**—the `#434` ruling
> referenced by code comments is not registered on docs side, see 3.4.3; full private requires
> rewriting 11 constructor literals, not taken at stop-bleeding stage; closed when P4 Obligations
> migration); execution mechanism extracted to `src/frontend/proof_execution.rs` (prohibition three
> single point—cannot be put in typecheck/, that would create L3→L4 reverse dependency); 3.1.2 added
> **four entry points** per 02 evidence chain step 5 (WBS row number missing `compile_project :99`,
> must be added and registered per 3.1.1 acceptance "criterion turns green"); 3.2.1 empty arm
> changed to bookkeeping (proof_calls throws up) + diagnostics (`into_result` established conversion
> path); 3.3.1 SOLVER slot Option-ized, Z3 missing conservatively downgrades per
> `SMTResult::Unknown` (E2031 family diagnostic replaces process crash). Two vulnerability criteria
> remove `#[ignore]` to turn normal green, supplementary corpus fixture `proof-obligation-honored`;
> check-fanout inventory migrates accordingly (wiring point 19=inventory). Measurement: criteria
> 2.4.1/2.4.3 turn green, full suite lib 2494 / integration 272 / corpus double layer all green.
>
> **3.4 Audit (2026-10-07, post-P3-landing re-review)**: For defects of the type "failure produces
> no signal" (definition in 02 §41-52), recounted on unchanged code, located 8 more places, all
> registered as 3.4.x in the above table. Method and evidence:
>
> - **Consumer-side inventory** (16 obligation fields one-by-one grep of full warehouse read
>   points): `variant_ctor_calls` **zero consumers**—the only reader `checker.rs:1468` is
>   "self-injected into `TypeCheckResult`", IR side uses form detection `ir_gen.rs:7374`;
>   `rfc011b.rs:659/688` only asserts production side, which is exactly the "only test production
>   side, naturally immune to consumer-side defects" form criticized in 02 §186 → 3.4.4.
>   `method_overload_ir_names` result-level field also has zero external consumers, IR uses parallel
>   `overload_resolutions` → 3.4.5.
> - **Stage coverage review**: 02 §91 table row 1 (four entry points "no proof_execution") has been
>   fixed by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 unchanged (multi-file `run` still doesn't
>   output W1003, doesn't run dead code family) → 3.4.6; LSP non-in-project path (02 §77 "manual
>   lex→parse") likewise doesn't run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` full warehouse consumers only four places): checker
>   three refined branches (3.1 already covered), ownership arm fixed by 3.2.1 (**source
>   unreachable**), and `environment.rs:422` (**reachable, zero diagnostic, zero bookkeeping, and
>   zero registration in full warehouse docs**) → 3.4.1, the only "all-unknown" live vulnerability
>   in this audit.
> - **Solver acquisition side**: `default_solver()` three consumption forms differ
>   (`predicate.rs:32` process singleton / `checker.rs:1266` injected per `check_module` /
>   `ownership.rs:621` new per back-edge judgment)—3.3.1 only eliminated one panic; the **silent
>   surface** of the other two is attributed to 3.4.2, **singleton-ization and cache hit**
>   attributed to P4's 4.4.2 (no double registration).
> - **Not counted in 3.4's existing attributions**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys not consumed, Drop silently lost) is still P2 red state always-on, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer sequence declaration opposite to actual, `equivalence` layer
>   not in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger downgrade) already registered in
>   P4.
>
> **Execution Convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites, can be landed on-site
> anytime (suggest doing 3.4.1 first—the only "full warehouse zero registration" live vulnerability,
> smallest change surface); 3.4.4–3.4.8 execute with P4, their criteria depend on obligation ledger
> (4.3.1) or unified Driver (4.2.x). **3.4 does not establish new criteria system**, uniformly
> reuses existing C2/C3 and P9's 9.2 gate.

> **Batch 3.13–3.16 Record (2026-10-13 registered → 2026-10-15 closed)**: User 2026-10-14 ruled to
> merge four correlation items (#451 method binding panic / #452 named actual argument resolution /
> #453 dead inference entry / #454 native export named actual argument—the last item is a boundary
> finding while verifying 3.14, confirmed by attribution experiment on the same day to be **not
> introduced by #452**, user ruling ① merged into this batch) for one team to handle. Closure
> status: all four items completed, ruling registered as RFC-039 **D60** (six binding rules + shared
> criterion single-point).
>
> **I. Two Contract Corrections (lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: #452 task contract simultaneously required "mixed
>    actual arguments output 20" and "position and naming competing for same position report error",
>    but the nail board fixture `Temp.pick(self, a)` only has one value parameter—the same actual
>    argument combination is both "mixed" and "conflicting". The first implementer truthfully
>    reported blocker and set task to failed, overall revert (not exchange silent for green light).
>    Disposition: task re-dispatch + contract revision—after giving **six binding rules** and **8
>    value-level matrix** (fixture changed to two value parameters `pick(self, n, a)`), attempt 2
>    completed by second implementer. The full text of the six rules has been verbatim incorporated
>    into D60.
> 2. **inScope missing mechanically required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only `code-tables --fix` was
>    run, **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964**—the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touching `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **II. Review → Fix → Re-review Two Closed Loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls containing named arguments in multi-candidate
>    zero-hit/single-candidate hit two paths silent whole chain) → fix round added external code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-set same
>    submission; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm named settlement) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations
>    mechanically reversibly replayed via `scratch.py trial` all caught, restoration byte-by-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inlined** a copy of
>    binding criterion, missing rule ④ ⇒ **divergent rulings** for same-form method and native
>    calls—method reports E1111, native silent exit=0; ir_gen has a third existing copy reporting
>    old code E1015) → convergence round converged criterion to single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call points `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught **regression** introduced by convergence
>    round (IR convergence block lost native omittable position ⇒ previously successful programs
>    with omittable optional positions were misreported as E1010) → fix round (omittable position
>    criterion with parameter name from same output `signature.rs:236` unique exit) → round-3 review
>    PASS (independent reproduction red→green; mutation nailed under-board FAILED proves criterion
>    can fail). **Lesson**: review must cover "unfixed branches of similar paths" (this batch's two
>    needs_revisions were caused by probes all falling on hit surface); fix round itself can also
>    introduce regressions (omittable position), caught by next round of reviewer change.
>
> **III. Process Events (truthfully registered, not pointing to individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: The working tree was externally
>    restored silently—the implementer's fix in progress (typecheck/ir_gen/code
>    table/locales/RFC-013 etc.) all disappeared, **at the same moment two review documents edited
>    by the user themselves also disappeared from git status** ⇒ determined as repository-level
>    discard (most likely user-side manual cleanup/IDE "discard all changes"). Three pieces of
>    evidence excluded group members: stash empty, no new commits, reflog no reset. The implementer
>    replayed the in-progress work in context and pushed; discipline subsequently changed to
>    "**first save patch to scratch, then sample red state**".
> 2. **Verification member ran out of context, changed person**: Independent verifier for 3.16 ran
>    out of room midway (no closing message), changed person per load rules—successor continued
>    using ready products (green state results, corpus equivalence scripts, mutation plan),
>    verification finally PASS. Positive precedent for partial product handover.
> 3. **Review round 5 rotation and historical round retraction**: When rotating reviewers per load
>    upper limit rule ⑤, `remove_member` **also retracted** the already-ruled terminal
>    review/verification rounds to the shared pool, scheduler immediately re-ran (t3/t8 were picked
>    up by new members, duplicate work and blocking downstream). Disposition: team lead took over
>    and cancelled the retracted historical rounds (t3/t8/t13), consequently added AGENTS.md rule 12
>    (handle retracted rounds immediately after person change). This batch's closure record is
>    registered per this rule.
> 4. **scratch governance (about 6.3 GB)**: Early batch each task had an external tree, 6
>    directories scattered outside the warehouse (about 780 MB, cleaned) + two members' external
>    verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Output governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md rule 9
>    (batch tree unique entry), rule 10 (verification environment six rules: mechanically reversible
>    mutation / line number taken from referenced revision / CI-only gates must run, etc.), rule 11
>    (writer load upper limit), rule 12 (handle retracted rounds after person change) and two top
>    meta rules (no private content / verify before asserting).
> 5. **Contract defects (not pointing to members)**: #454 fix round's contract inScope only circled
>    2 files, actual mechanically required surface 8 files (rule 6 is meant to prevent exactly
>    this—contract template should preset mechanically required surface list); additionally, that
>    task's terminal status field was once occupied by probe payload, formal acceptance/command
>    evidence supplemented by team lead per convention.
>
> **IV. Stranded Items (not fixed in this batch, registered for review)**: round-3 review three
> low-severity findings—① method path's `build_arg_slots` (`ir_gen.rs:521`) is still the **second
> copy** of the criterion (existing, per previous round's caliber registered at closure, not
> converged)—**2026-10-16 pre-release audit added three points**: (a) it has **two** call points:
> `ir_gen.rs:7888` (method binding path, uses bound function `binding.function`'s parameter names)
> and `:7975` (overload resolution path, uses resolved mixed name leaf segment), original
> registration only wrote "`ir_gen.rs:521` area"; (b) it has **already diverged from shared
> implementation `bind_args_to_params`**: missing rule ⑤ (duplicate naming → E1015), rule ⑥ (missing
> required parameter → E1010) and "optional position" concept, and hard-codes receiver position to
> `1 + i`; the two call points uniformly `continue` for empty slots (comment writes "diagnostics go
> to typecheck (it guarantees no empty slots if resolution passes)"), is a **suspension invariant
> written in comments with no assertion backing**; (c) **RFC-039 D60's "all call points converge to
> it" enumeration missed these two places**, belongs to declaration inconsistent with fact, and the
> cited line numbers were not re-taken on the referenced revision, has been corrected in D60 text
> with this entry; ② batch verification tree (worktree) inner `cargo fmt --all` error is worktree
> workspace artifact, gate list should note fmt goes to main warehouse; ③ repair-type contract
> template should preset mechanically required surface into inScope (see III.5).
>
> **V. Final Gate (closure revision, locally two implementation commits + one documentation commit
> unified push—the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 /
> corpus_probe **342 lines consistent with baseline line by line** (only regenerated line:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number synchronized,
> manually reviewed) / IR snapshot zero drift.
>
> **VI. Run Links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (MSRV/Test of same run cancelled by subsequent push
> per concurrency—cancellation≠failure, covered by later run); governance tool push → CI 38546373435
> / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) → CI 38528560364. The
> push's own CI / Deploy Docs run is supplemented in the task record after push.

> **3.4.8 ③④ Landing (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail adds `Linking` (**pure
> merge**, entire block skips `is_bin_role` entry validation—E3020 criterion still attributed to
> `RoleClassification` to prevent double reporting; also hard reason: run path judges "has main" per
> merged IR, collection semantic "entry itself error→no IR" will be misjudged as missing main and
> hard-aborted) → `Monomorphization` (check form); `ir_generation_check` retains unit IR (**memory
> only**, for pure merge and mono consumption, not sent out / not written to disk / not produce
> snapshot—if not retained, aggregation request is empty, mono takes `NoObligations` false green);
> mono failure goes through per-file collection channel in Check (diagnostic attributed to entry
> file + `failed_stages`, not set as failure), MultiFile keeps `fail_with`; `into_outcome` changed
> to "first take collection then judge failure". ④ empirically **zero production changes**: whole
> program BFS already covers cross-unit mutual recursion (pathological → E3005, well-formed
> three-layer chain executes correct value), delivers four e2e + one mono single test as live
> criterion.
>
> **Verification Chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-check with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase, no decrease).
>
> **F1 (found in verification period, D59 A/B class, same-round fix)**: Under standalone multi-file
> check without project root, aggregation request name (`lib.f`) and unit IR definition key (`f`)
> have different sources → mono lands "native no IR definition" silent `continue`, check reports 0
> error/exit 0. Fix: aggregation side adds reverse lookup skip per each unit's **true definition
> key** (precise enumeration + character-by-character equality + ambiguity abandon, not suffix
> heuristic; project form not registered, existing path unchanged). **Errata (same day)**: that
> reverse lookup skip was **deleted** in subsequent regression fix; root cause solved by
> `module_key` dual-use decoupling (added `script_mode`) from naming caliber—IR definition key = IR
> call name = aggregation request name = `{unit_key}.{name}`. **Regression Record**: ③ after
> landing, CI's `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6 items
> same file same name rebinding + 4 items conflicting with `<std/list>`, rootless check doesn't do
> qualification triggers `link_module_irs` conflict check); fix = ir_gen split `script_mode`
> (qualified name ⊥ Script/Bin, E3023 gate looks at it) + `ir_generation_check` per-unit
> consistently passes unit key + `.with_script_mode(rootless)` + Script top-level same-name
> rebinding "latter takes effect" (consistent with run path existing behavior); verification =
> document gate 258/0, three-binary comparison (HEAD/③ tree/fix tree) three types of collision
> fixtures, check form E3005 no regression, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic attributed to entry file entry, span can cross units (same
> caliber as run path `path=entry`); Check form holds one ModuleIR per unit (memory only, released
> after check ends).
>
> **Issues Left for Follow-up (not this round's regression)**: ① E3005's help text still writes
> "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of E3005)—registered for
> scheduling; ② multi-file E3005's span rendering label misaligned (reproduced with HEAD binary,
> pre-existing); ③ request collection surface has **type parameter position dependency** (generic
> calls where type parameters are not in value parameter position don't send instantiation request,
> mono records `NoObligations`, convergent mutual recursion is exactly this form)—same surface as
> 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Remove `pub` and Auto-binding (#399)

> **P3.5 Landing (2026-10-08)**: All seven second-level tasks completed, entire batch one commit
> (independently revertible).
>
> - **Measurement vs plan inconsistency (truthfully registered)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—after actually deleting `KwPub`, `pub`
>   falls back to ordinary identifier, old writing lands at semantic layer reporting **E1001 Unknown
>   variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specially detecting removed
>   syntax is anti-pattern", **no new** migration-specific diagnostic added; corpus fixture
>   `pub_declaration_err.yx` nails E1001, document (`syntax.md`) truthfully states the consequence.
> - **Design Convergence (exceeds RFC literal, needs review)**: `project_refs` and `cross_file_refs`
>   two fields become semantically completely coincident after pub deletion (same "package internal
>   reference union", prohibition one prohibits parallel representation) → merged into single
>   `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally
>   Internal role's non-pub items change from "file-internal reachability" to "reference pool
>   judgment" (relaxed, prefer false negative direction); Lib tightened from "pub absolute
>   exemption" to reference pool judgment; Script top-level bindings not referenced start reporting
>   W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (production side
>   injection table, full warehouse zero consumption).
> - **Test Migration** (test count not decreased): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub recognized" assertion reversed to "pub is no longer
>   modifier"; `passes/tests/dead_code.rs` 7 role semantic use cases rewritten (assertion reversed
>   2); `tests/integration/cli_e2e.rs` 8 role use cases rewritten (`lib`/`internal` exemption
>   assertion reversed to report assertion).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not in IR); corpus diff baseline only added 1
>   (`pub_declaration_err.yx` → E1001), command `cargo run --quiet --example corpus_probe`
>   regenerated then manually reviewed.
> - **Measurement**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 Landing Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS check / gate / push) completed by t7. Prerequisite = 4.3.1 obligation ledger
> (red state retention 173 files / 533 keys).
>
> **I. Carrier Ruling (A: `PlanId` follows AST node)**: Added `PlanId` (`src/util/plan_id.rs`, `u32`
> monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` represents synthetic nodes
> outside parser), issued by **unique allocator** `ParserState::alloc_plan_id`; carriers in two
> places—`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call site identity),
> `Expr::call` is the unique call constructor in the whole warehouse. The vetoed one is "both sides
> derive same identity from span" position coincidence form (D20's original defect).
>
> **II. Two-Level Verification and Review (members independent)**: Each functional commit has an
> independent verifier and reviewer, **verification/review does not reuse implementer evidence**:
> 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations) / t3 review;
> 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations) / t6 review.
> Review time points and reviewed revisions both fall in immutable history (`git show` verifiable).
> **Provider rename caused t5 / t6 to change members and re-dispatch**: original verification/review
> members couldn't continue due to provider rename, re-dispatched per scheduling convention and
> **wrote reason in record** (cross-round context preservation priority, change person only when
> compression is unavailable); after re-dispatch conclusion consistent with original round (PASS),
> implementer evidence not reused.
>
> **III. Two Defects Self-discovered and Fixed During Implementation (t1, both in inScope)**: ①
> Consumption exemption criterion was previously attached to `UnitState.ir.is_some()`—`check` form's
> Linking/Mono takes `unit.ir`, real unconsumed in check form **silently passes** (mutation
> measurement: D41 red but CLI check exit 0); changed to read **ledger's own completion marker**,
> and added regression cases. ② Drop sequence drifted with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion produced 106/120/114 line drift in three
> consecutive runs; changed to **total order sort** (same name takes max depth + name ascending) and
> ran twice zero drift.
>
> **IV. Criteria and Evidence Numbers**: After 4.5.1 `test_release_plan_spans_consumed` removes
> `#[ignore]` always-on (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in
> same submission (programmatic review: added 1083 lines Drop + 86 lines jump index translation,
> zero deletion, no reordering), corpus 342 entries diff empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference set empty + evidence surface floor `examined ≥ 1`
> / `consumed ≥ 2`; measured examined=1, consumed entries 2, no resolution skip 210), resolution
> form 9 items nail board, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation-driven: consumption key shifted one position ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; reverted and turned green.
>
> **V. Semantic Correction Classification Basis (D59)**:
> `Type.method(position actual argument, name = value)` **silently gives wrong answer** before
> change (measured clean compile, run output `1` not `20`) ⇒ **Class C** (old behavior not
> successful), fix takes "loud": same program after change compile period `error [W1090]`; "named
> actual argument participates in resolution" terminal semantic belongs to **Class D**, registered
> in §3.14 and left issue #452 for special team to take on. Additionally: `Drop` is no-op in
> interpreter (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`)—key
> migration doesn't change runtime behavior.
>
> **VI. F-1 / F-2 Disposition (t5 independent verification found)**: **F-1 (medium)** unbound
> consumption point (`ir_gen.rs:7766-7790`) is unreachable in current corpus and test surface, key
> correctness has no red state falsifiable ⇒ per "known boundary" registered in
> [02](02-stage-contract.md) §2 revision note fifth paragraph (keep reason + no new task); **F-2
> (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise pointer
> (produced `inference/expressions.rs:4550`, consumed `ir_gen.rs:7697` / `:7772`), separate small
> commit, not mixed into documentation commit.
>
> **VII. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement marker);
> 07 criterion table and 04-ssa defect class 3 changed to past tense; 09 checks 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG records one line of behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Closure-discovered accounting deviation**: t3 / t6 report's
> pointer `ir_gen.rs:357` / `:3102` / `:1096` differs from code fact by one to three lines (actually
> `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion)—registered with code fact as standard,
> this note also retained.

## P4 4.10.3 Standalone Run and MultiFile Orchestration Unification (4.2.5 Ruling A's Run Half)

> **Batch C Record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15 ruling C—this
> round only does 4.10.3 (same batch candidate 3.4.6 per B1 ([02](02-stage-contract.md) §Runtime
> Behavior Table "dead code family warnings don't enter compile path") deferred until after 4.9,
> prerequisite changed and noted); implementation form = **complete single team** (user instruction
> no DAG, no team building, single session implementation + self-verification). Pre-batch arrears
> first landed: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit`
> recovered from exit 1 to exit 0), WBS registration debt (#456 registered as 3.17 + incomplete rows
> add "belonging phase / whether blocks main line" fields).
>
> **I. Red Before Green**: driver-level red criteria 3 (`src/driver/tests/run_rootless.rs`—rootless
> merged IR must not carry entry_function (red state `Some("main.main")`), rootless top-level
> executable statement must not report E3023 (red state doesn't produce module), Bin comparison
> entry_function kept (green state always-on)) + CLI e2e 3 nails (standalone relative import run
> (red state E5001), Script top-level statement execution and #413 hint two green state nail
> boards).
>
> **II. Implementation Surface**: CLI `run` uniformly takes `compile_project`
> (`run_file_with_diagnostics`'s project/standalone bifurcation deleted); Linking's `entry_function`
> gated by `is_bin_role` (#413 T4—Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call site project probe (after unification ModuleIR
> consistently carries source_files, original mark invalidated); `script_main_never_called` adapted
> to qualified name form (function `{key}.main`, global slot measured also qualified name—based on
> IR forensics, not inferred from comments).
>
> **III. Unified Exposure's Existing Gaps (Same Batch Side Fix, A/B/C Class—only fix failed programs
> to successful, zero impact on previously successful programs)**:
>
> 1. **Explicit type actual argument pre-specialization name** (`mk(Int)(5)`): ir_gen directly emits
>    specialization name per D6.3 (mono strips unspecialized original), `resolve_call_target` three
>    arms (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call site suspended E6006.
>    Fix = add fourth arm to this single point resolver (base name qualification, type parameter
>    string preserved as-is). **Bin project same-form reproduction (pre-existing defect, not
>    introduced by this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign same
>    form as declaration) statement by statement ⇒ same variable allocated 3 slots, initialization
>    writes to later-allocated slot but read uses earlier-allocated slot (runtime Add type
>    mismatch). Fix = both places merge by name (same criterion same source).
> 3. **Same module constrained generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table verification (precise hit → generic
>    belongs-to unit key qualified hit → fall back to bare name status; not suffix heuristic, same
>    form as `resolve_generic_name`).
>
> **IV. Registration Gap (not fixed in this batch)**: Constraint dispatch in **type and generic
> definition different modules** still E6006 (rootless dual-file probe verified; Bin same form)—root
> cause = MonoType user type name doesn't carry definition module, correct fix belongs to P6 type
> identity convergence; registered WBS **3.18** (belongs to P3, prerequisite waits for P6, does not
> block main line), `qualify_deferred_method` notes same surface.
>
> **V. Test Adaptation (honestly registered)**: `tests/integration/multifile.rs` fixture
> supplemented manifest—what it tests is RFC-029 project orchestration (Bin auto entry) not Script
> semantic; `cli_e2e` one nail board (top-level diagnostic variable name visibility) and
> `script_main_hint` fixture (7 cases) synchronized per qualified name form—production consumer side
> only takes merged IR, fixture direct drives same form, no need for pipeline product stand-in.
>
> **VI. Final Gate (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regeneration **12 lines**—line-by-line structured check: exit / stdout / diagnostic
> code all identical item by item, only stack frame function name and variable name qualified by
> unit key (manually reviewed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process Events (truthfully registered)**: In this batch, during pre-commit hook running,
> the workspace was concurrently edited, and the hook's stash rollback ate it and that commit failed
> (concurrent form predicted in config comments)—discipline = don't move workspace during hook
> running; #455's commit therefore split into two attempts (second all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 instructed to
> proceed in order 4.6 → 4.9 → 4.4 → P5, first do 4.6; implementation form = **complete single
> team** (following previous batch user instruction: no DAG, no team building, single session
> implementation + self-verification). Task definition (09 three-level table): project's internal
> file semantic data (SemanticDB) goes through orchestrator same pipeline, same source as
> diagnostics; cross-file reference carries `resolves_to`. Acceptance: project-internal cross-file
> jump hits definition; semantic data and diagnostics come from same compilation.
>
> **I. Current State Verification (verify before asserting)**: didOpen/didChange triggers **two
> passes** of compilation—diagnostics go through unified Driver pipeline (4.2.3/4.2.6 already
> unified), semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ in-project `use`
> parsing fails, cross-file reference events not produced, jump/rename/reference lookup silently
> fails for imported symbols—D55 registered silent failure). Orchestration arm's checker unified
> naming `"<module>"` (SemanticDB file identity lost), `type_result` (including semantic_db)
> discarded when diagnostics non-empty and never externally sent; `checker.rs` drainage always
> covers `resolves_to`'s DefId file identity to current file; `Export`/`ModuleInfo` both lack
> definition location surface.
>
> **II. Red Before Green**: LSP e2e nail (`src/lsp/tests/cross_file.rs`)—temp dual-file project
> after didOpen main.yx sends GotoDefinition to `add_one` call site, **before fix returns None (red,
> empirically the silent failure in previous section)**, after fix hits lib.yx line 0 definition
> (green). During process one probe positioning: compiler-side link (driver probe dump) was already
> throughout while e2e still empty—root cause is `resolve_reference` matches by reference **starting
> point** precisely (D55 precise-only current caliber), test cursor falls in identifier middle; this
> tolerance is existing behavior surface, this batch doesn't touch (see VII).
>
> **III. Implementation Surface**: (1) `Export.definition_span` (extract_module_info harvested from
> AST identifier node, same source and value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled per DiscoveredFile, including
> vendor; std always None); (2) checker reference drainage via `import_def_locations` (use line
> binding span → definition file + definition span) redirects cross-file import items' `resolves_to`
> out of this file; std import not registered, maintains falling to use line; (3) checker module
> identity changed from unified `"<module>"` to unit file path (verified `env.module_name` all 4
> consumption points zero diagnostic consumption); (4) `DriverOutcome.semantic_db`
> channel—CollectAll form (Lsp target file **regardless of diagnostic success or failure** +
> SingleFile fallback) converges, Lsp arm tail injects non-target file top-level export definition
> surface from registry (`external_definitions`, unopened files can also jump; type/submodule
> exports not injected—typecheck side type registration doesn't produce definition events, both
> sides keep consistent gap); (5) `orchestrator::analyze_source_in_project`
> (`check_source_in_project` redirects to take diagnostic half); (6) LSP `analysis.rs` single
> compilation entry replaces manual pipeline (`update_semantic_db` deleted), World key uniformly
> normalizes file URI (`uri.rs`: `path_to_uri(uri_to_path(u))` idempotent normalization, drive
> letter case and %XX encoding differences normalized to same key, cross-file DefId therefore stably
> hits); 5 handlers + semanticTokens dual requests connect to `db_key`.
>
> **IV. Side Fix (D59 split B class—old behavior silently doesn't act, previously successful
> programs zero external impact)**: ① project-internal file cross-file jump/rename/reference lookup
> changes from silently falling to hitting real definition; ② when lexical error, old manual
> pipeline deleted semantic information (coloring disappeared) → incomplete AST coloring retained
> (4.2.6 editor philosophy extension); ③ when target file has type error, semantic data no longer
> discarded with `type_result` (typing intermediate state coloring/jump not lost).
>
> **V. Nail Boards**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`—target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback use line, error file retention, single-file fallback path key, single-file
> error retention) + LSP e2e 1 (red before green) + uri normalization 4
> (`src/lsp/tests/uri.rs`—drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **VI. Mutation Verification (scratch.py trial, mechanically reversible)**: Batch tree wbs-46 @
> closure revision 39546020, two mutations both make hit nail boards turn red, after revert blob
> consistent with HEAD + git status CLEAN—m1 delete `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver nail + LSP e2e nail red, remaining 7 green; m2 delete
> `external_definitions` injection (arms.rs) ⇒ same two nails red (round-trip parsing loses anchor),
> remaining 7 green. Batch tree gc-ed (released 4.8 GB, `status` no batch tree).
>
> **VII. Final Gate (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new nails) / `--test integration` **287 passed** / corpus
> diff **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synchronized server.rs:482 and 02 §Obligation
> Field Table) / check-docs-truth 0 / docs-examples 258 entries 0 failures / docs-orphan 0 /
> markdownlint warehouse only user's unsubmitted 032 two MD056 (user's own edits, untouched) / pnpm
> docs:build 0. **Process Events (truthfully registered)**: en/09a-wbs-history.md:54 one MD037 was
> introduced by auto-translate bot re-translating previous batch 09a (zh original has no space),
> this batch incidentally fixed—markdownlint red is not this batch's change surface but is in the
> same file as 09a, registered together. **Stranded Observation (registered pending discussion, not
> touched)**: `resolve_reference` only precisely matches by reference starting point—when cursor
> falls in identifier middle, jump/hover/rename silently doesn't act (D55 precise-only current
> status, whole file form isomorphic), if tolerance matching needed it's independent behavior
> change, report to user for ruling.
>
> **Same Day Subsequent Batch (4.6.2, landed after user ruling)**: After delivery of closing report,
> user on "stranded observation ①" ruled "OK, then change"—lookup key tolerance implemented per plan
> A: definition/hover/references/rename four handlers' lookup key changed from cursor position (via
> UTF-16 conversion) to use `find_identifier_at_position`'s already-positioned **identifier's own
> starting point** (same coordinate system as semantic events, no conversion needed);
> `resolve_reference`'s starting point precise match semantic unchanged (D55 precise-only
> unchanged). references/rename's definition point fallback judgment (start <= col < end) holds
> isomorphic for identifier starting point. Red before green: definition middle cursor nail first
> red (None) then green; four handlers middle cursor each 1 nail + cross-file e2e start/middle dual
> position assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2
> (three-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)— Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 ruled to merge
> 4.9.1 / 4.9.2 / 4.9.3 three related three-level tasks **into one team one round** (reason: they
> belong to "pool's source and identity" surface, merged landing can avoid `collect_project_refs`
> deletion's intermediate state repeated rewriting; 4.9.4 / 4.9.5 belong to consumption refinement
> and test pool separation, depend on 4.6.1's `use` path resolution, forcing same batch would expand
> gate surface). #450 attribution **pending ruling**—evidence shows belongs to "overload resolution
> name not passed back to reference count" class, still reproduced under round 1's already landed
> form, registered as independent WBS entry per disposition (team lead recommends A: merge into
> round 2 registration as independent entry; **pending user ruling**). Frozen rev = `3863a9ff0`
> (parent commit `62f6e1a59` is pre-fix baseline); verification PASS attempt `aab92fe0` (i.e., t2);
> review pass attempt `96452696` (i.e., t3). **Closure registration commit = `38d30b846`
> (docs(docs): 4.9 reference pool round 1 closure registration)**—this commit was actually written
> and pushed by closure member integration-refpool; its closing probe once mistakenly marked closure
> task as terminal failed and overwrote output, team lead took over new attempt and changed back to
> completed per verified delivery facts, hereby correcting attribution and label. Evidence
> retention: evidence summary (8 mutation patch list, `warn_compare_v8.json`'s caliber and
> conclusion) is inline in this section "III. +8 and +16 two counting correspondence relationship"
> and "IV. mutation experiment"; original files reclaimed with workspace, **not checked in, must not
> be used as document reference source** (AGENTS meta rule ①).
>
> **I. Merge Ruling**
>
> 1. **Ruling itself**: Three related three-level tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round**—former depends on 4.6.1 delivered `use`
>    path resolution (already landed at `39546020`, can start); latter same batch as RFC-029f table
>    collection, registered at RFC-029f end section for change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are prerequisites of each other (4.9.2 changes pool type
>    → 4.9.1 assembly surface needs rewrite → 4.9.3 has role table for denominator judgment),
>    splitting into rounds will inevitably write one intermediate state then change twice; (b) all
>    three belong to "pool's source and identity" surface, 4.9.4 / 4.9.5 belong to another surface,
>    batch splitting is by "cohesion" not "independently revertible" segmentation.
> 3. **Merge Result**: Single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: before push `git pull --rebase` merged
>    auto-translate bot commit (`72f364169`); rebase landing point after the other party's commit,
>    this batch's two commits (implementation `3863a9ff0`, its pre-fix baseline and other party's
>    documentation commit `62f6e1a59`) **not rewritten, kept original hash**—initial version
>    mistakenly wrote "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "same value after
>    rewrite", actually "not rewritten"), corrected at 2026-10-17 closure; other party's code commit
>    `33f14e8ab`→`b122b7ad1`. **External references uniformly use post-push hash as standard**
>    (rewritten old hash unreachable in new clone, identity determined by patch blob comparison).
>
> **II. Five Criteria Red/Green Measured Caliber**
>
> | #   | Criterion                                              | Red State (pre-fix / P1 directory scan backflow patch)                                                                                                     | Green State (round 1 landing)                                                                                                                                                                                                                                                                                                                                                                 |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool root = package target (#334)                      | Directory recursive scan ⇒ tests/ files' identifiers also enter pool, exempting production code unused definitions                                         | `collect_package_refs` (`helpers.rs:481`) per `ExplicitSurfaces` root calls `discover_with_used`; full root degenerate falls back to `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                            |
> | ②   | Pool granularity = `(module, name)`                    | Same name local variable (`let foo = …`) accidentally exempts same name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembly unique entry; `arms.rs:603-609`'s `other_module_refs` unions "all modules except this unit" entries and sends to `DeadCodeAnalyzer`; module's own names (top-level definition and local binding, `use` import name exception) no longer exempt other modules' same name definitions |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by other files' references in project                                                                                                      | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test takes `_ => {}` default arm (per file-internal definition-use graph judgment)                                                                                                                                                                       |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Pre-fix: report `W1003@1` (actually "unused import"), not W1001                                                                                            | Red state measured (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                    |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Pre-fix: report `W1003@1`                                                                                                                                  | Same as above                                                                                                                                                                                                                                                                                                                                                                                 |
>
> **Criterion ②'s "Deduced Red"** (implementer self-stated + verifier closure): red2 fixture's
> `let foo = …` is in helper.yx; old pool under bare name union treats `foo` as a whole "used by
> helper", lib.yx's `foo` exempt. **Measurement**: red2 fixture vs pre-fix binary → no W1001 (old
> caliber exempt); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool)'s red state fixture **first version failed**: first
> version fixture placed same name local variable in same file triggering E2010 (this language
> doesn't allow same module shadowing top-level definition), moved to another module (helper.yx) for
> deduced red state. This point t3 review and t2 verification both independently reproduced, **not
> contract defect**—same module shadowing is this language's hard constraint, fixture must be
> cross-module.
>
> **III. +8 and +16 Two Counting Correspondence Relationship** (**t3 review proven** as standard)
>
> 4.9 round 1 evidence summary's section C writes `+16/−0`, but missed "**new fixture records**"
> component—v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff, `only_before=0`)
> caliber **includes** new fixtures; t1 implementer's self-built "+8" is **warn-before / warn-after
> (each 325 entries) aggregated per fixture** (each member file checks once). Review mapped item by
> item:
>
> - **+8** (8 new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: These 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degenerates to checked file's own compilation set, consumer main.yx not in it; this
>     is in-design change (before, these files' check surface had zero warnings, not exemption
>     path). Warehouse root `yaoxiang.toml` is empty file ⇒ same degeneration path empirically
>     holds.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files' `exit=None` in t1's `warn-before.jsonl` (capture time point
>     **doesn't exist**—is this batch's newly added fixtures). Hence +8's "+ 0 − 0" diff doesn't
>     have them, +16's v8 symmetric diff has them.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` left: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once per file), **cancels** leaving only lib's
>     1—three data sources (+8 / +16 / dual binary ABBA) mutually consistent.
> - **New fixtures 8** vs **+16 residual 8**: same batch of files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats their each check as
>   independent record—same file may trigger different diagnostic entries for multiple
>   `role/context`.
>
> **Conclusion**: **+16 = +8 (aggregated per fixture) ∪ new fixtures 8**—both countings are correct,
> **different caliber**: t1 reports "added 8" is fixture-level caliber, t2 reports "+16" is
> file-level caliber (symmetric diff includes new fixtures). t3 review ① +8⊆+16 proven; **review ②
> writes "don't copy EVIDENCE.md section C"**—section C missed "new fixture records" component, this
> entry rewritten per t3 review `pass` conclusion.
>
> **IV. Mutation Experiment (mechanically reversible, all `scratch.py trial`)**
>
> 8 mutation patches (p1 directory scan, p2b use import name removal, p3b script connect to pool, p3
> flat consumption, p4 std connect to pool, p7final self-built 4 criteria, p8 std close, p9 vendor
> connect to pool + p_stack2 / p_stack3 overlay surface). Each trial output contains: HEAD blob →
> mutated blob → reverted blob consistent with HEAD = yes + `git status` CLEAN. Local
> `core.autocrlf=true` needs injection via `GIT_CONFIG_*` to `false` (`trial_env.ps1`), otherwise
> patch new lines convert to CRLF causing apply/revert double failure—**only affects patch
> mechanical, doesn't change tested behavior**.
>
> **V. Process Events / Environment Lessons (must read for next round handover)**
>
> 1. **core.autocrlf injection**: Local `core.autocrlf=true` must be injected via `GIT_CONFIG_*` to
>    `false` to ensure patch mechanical reversibility; that injection script reclaimed with
>    workspace (criteria already listed in previous entry, can be rewritten accordingly). **Next
>    round don't reuse** any trial script without injection.
> 2. **Same tree trial serial**: `trial` and `cargo build` on same tree **must be strictly serial**.
>    `trial` internal `git apply` and `cargo build` both touch worktree, parallel produces "patch
>    incomplete when build sees it" or "build and apply deadlock" two kinds of race.
> 3. **trial rebuild binary needs synchronization confirmed**: After `trial` completes, will rebuild
>    tree's binary (cargo's fingerprint includes source path, **change tree = full rebuild**),
>    before subsequent measurement must confirm `bin/yaoxiang-rs` synchronized with HEAD. **This
>    round once used residual mutation binary to get false data**, already rebuilt and re-measured
>    for correction.
> 4. **Reason for second tree yx49-b**: Comparison needs pre-fix binary (62f6e1a59), no ready-made
>    in batch; `scratch.py init yx49-b` opened second tree (another full build). **Tree released**
>    (see VI), deletion left to batch closure `gc` disposition.
> 5. **Reason for cli_e2e two Internal use case fixtures supplemented to legal package (add
>    `[run].main`, assertion unchanged)**: Pre-fix `cli_e2e`'s two `Internal` role nail board
>    fixtures **didn't declare `[run].main`** ⇒ after 4.9.1 landing `compile_project` takes
>    `ExplicitSurfaces` root call path, can't find `main` ⇒ entire batch 2 fixtures behavior drifts
>    from "Script fallback" to "can't find entry"—is 4.9.1 introduced fallback behavior not defect.
>    After adding `[run].main` field two fixtures take normal package path, **assertion** (exit code
>    / diagnostic code / warning set) unchanged word by word.
> 6. **Safe way to delete second round tree**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" then delete.
>    This batch has released before closure, next step `gc` natural cleanup.
>
> **VI. Batch Tree State**
>
> Before closure: yx49 released (4.8 GB released), yx49-b released. Two trees no mount, status can
> be re-verified in `scratch.py status`. This task output end attaches `scratch.py gc` complete
> status.
>
> **VII. RFC-029f "Known Defects" Table Flip**
>
> Previous version (2026-10-09) described "reference pool's three design defects" in present tense
> (including already deleted code locations like `collect_project_refs` / `orchestrator.rs:603-648`
> / `orchestrator.rs:393-396`). This batch closure synchronously rewritten to "**Known Defects:
> Landed (2026-10-16, WBS 4.9 Round 1)**": ① current pool root = `collect_package_refs`
> (`helpers.rs:481`), per `ExplicitSurfaces` resolution, no directory scan; ② current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly unique entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumer side `arms.rs:603-609`; ③ current Script doesn't connect to pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-taken on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` already split per P4 4.2.9
> (`c59fb751`) moved to `src/driver/{state,arms,helpers}.rs`, table's old positions no longer
> hit—this is "declaration inconsistent with fact" fix (t3 review note ①).
>
> **VIII. Final Gate (closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixture
> behavior runs, 338a339,340; other 342 lines byte-by-byte unchanged) / IR snapshot 204 zero drift /
> check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process Event**: This batch
> one cargo build had `core.autocrlf` convert script line endings to CRLF, team scratch path's
> `yaoxiang-rs` startup reported `BadImage`, measured as encoding inconsistent with git index—passed
> after rebuild (parallel with 4.6 batch "external whole-tree discard" lesson, different cause).
>
> **IX. Stranded (handed over to next round)**
>
> 1. **#450 attribution ruling**: Evidence shows belongs to "overload resolution name not passed
>    back to reference count" class, **still reproduced under round 1's already landed form**—per
>    user ruling C process dispatch. **#450 probe**: dual binary same set `{W1002@2, W1005@4}`
>    (measured, rootless form), no direct relationship with round 1 modification, but #450's fix may
>    need to wait for 4.9.4 (consumption refinement)—because root cause is "overload resolution name
>    passing reference count" needs use item's precise consumption surface, 4.9.4's "real use after
>    import" is same class criterion's precise version.
> 2. **dead_code.rs:683 pointer nit**: comment writes "two pools' assembly caliber see driver side
>    collect_compilation_refs"—actual `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment inconsistent with code main path caliber
>    (**not in this batch's write scope**), left for 4.9.4 / 4.9.5 landing to correct together.
> 3. **4.9.4 / 4.9.5 prerequisites unlocked**: 4.6.1 (`use` path resolution) already delivered at
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs first define
>    RFC-029f table collection (already laid by this batch's "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user ruling C deferred to after 4.9)—this batch round 1 not involved; next round
>    4.9.4 / 4.9.5 landing handle in same batch.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)— Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 Batch Record (2026-10-16 implemented → 2026-10-17 closed)**: Covers 4.9.4
> (consumption criterion = real use after import), 4.9.5 (test pool separation), 4.9.6 (method
> reachability caliber, issue #450, user 2026-10-16 ruling A merged into round 2). Closure fix
> (verification side remedy M1/M2/M4 compilable redo + F1 fix round + PA-2 dead branch deletion) and
> registration gap issue #464 (WBS 4.9.7) settled together.
>
> **I. Judgment Chain and Commits (uniformly post-push true value)**
>
> | Step | Content                                                                                                                                                | Post-push Commit | Pre-push Local Value | Judgment Method                                               |
> | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------- | -------------------- | ------------------------------------------------------------- |
> | 1    | fix(util): check explicit path not blocked by default exclude list + --exclude always empty (issue #463, P0-0.4)                                       | 366bc5a69        | ef959ca55            | Patch blob same                                               |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool per import attribution + Test doesn't contribute consumption surface)                                                 | e6cf1aeaa        | 84a6f6978            | Patch blob same                                               |
> | 3    | Phase B = 4.9.6 (#450 per candidate method reachability; including generation warning-codes.md)                                                        | ddcbb12d0        | cd629b5fe            | Patch blob same                                               |
> | 4    | docs(meta): AGENTS rule 10 ⑦ (commit period stash window convention)                                                                                   | ec94fa9d8        | 9a4ed5f3b            | Patch blob same                                               |
> | 5    | Fix round = F1 (overload brother overload restoration report W1005) + F2 (verification side M1/M2/M4 compilable redo) + F4 (snapshot generation noted) | b8380e6ab        | 3d9ef6eab            | Patch blob same (36962f318e42…, git range-diff no difference) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three false statement corrections, register #464)                            | b940300ac        | e13984465            | Patch blob same (patch-id 9bda3cfe4c…)                        |
>
> **Hash explanation**: Pre-push `git pull --rebase` rewrites local hash; above "Judgment Method"
> column is all "patch blob same" (`git diff <rev>~1 <rev> | git hash-object --stdin` both sides
> consistent + `git range-diff` no difference). This article doesn't reference any pre-push hash as
> authoritative source.
>
> **II. Three Criteria Red/Green Measured Caliber** (verifier self-built reproduction, doesn't trust
> implementer criteria)
>
> | Criterion                  | Red State (before binary 366bc5a69 measured)                                                                                                                                            | Green State (ec94fa9d8 and after)                                                                                |
> | -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() saves a.yx same name foo; guard only imported not used also saved)                                      | Same fixture 2 truthfully reported (collision name not saved + unused import not consumption);ref_pool 8 passed  |
> | 4.9.5 test pool separation | src/util.yx's debug_print saved by tests/ reference (only 1)                                                                                                                            | Same fixture 2 (test reference doesn't overflow);ref_pool's test_ref_pool_excludes_test_role_references passed   |
> | 4.9.6 method reachability  | Self-built fixture (2 disambiguation overload calls + 1 uncalled) 3 W1005 false positives;method_call_args.yx 3 / method_overload.yx 1 (overload per full name coverage only reports 1) | Same fixture accurately reports uncalled orphan;method_unused 3 nails passed;corpus call_args 3→0 / overload 1→0 |
>
> **Fix round (F1) further tightening**: Unresolved resolution brother overload restores reporting
> W1005 (b8380e6ab—same fixture calling ->Float reports ->String brother, calling ->String reports
> ->Float brother, bidirectional line number nail). **Fallback boundary (registered #464)**:
> Cross-unit (registry resolution) method calls don't produce resolution entries, fall back to
> name-level exemption, cross-unit same name same order not previously reported—registered 4.9.7.
>
> **III. Process Events and Lessons (four hard requirements correspondence)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: Round 2 review once
>    registered "cross-unit same name same order not reported" as blocker (PA-1); after two fact
>    reconciliation (implementer retention + verifier at b8380e6ab/b940300ac dual binary
>    re-measurement of same fixture set, diagnostic set byte-by-byte consistent) confirmed it's
>    **capability gap existing before round 2** not this round's regression—mechanism = cross-unit
>    method calls don't produce resolution entries, fall back to name-level exemption; disposition =
>    changed to medium, registered issue #464, WBS 4.9.7 scheduled next round.
> 2. **Reviewer publicly corrected before data and withdrew regression judgment**: Reviewer's
>    initially cited before data ("−42 entries include called methods") inconsistent with its
>    original output (W1005 count=0)—verifier confirmed after checking before capture was 0,
>    reviewer's regression judgment **publicly withdrew** accordingly;−42's correct caliber = 26
>    files' called method false positive elimination (phase B), item by item traceable to
>    behavior-run exit=0 files (method truly called executed).
> 3. **t13 terminated due to contract premise not holding (not implementation defect)**: That task's
>    acceptance premise wrote "cross-unit same name same order will report W1005 regression fix",
>    but measurement (t14 dual binary) confirmed that behavior is already registered gap (#464) not
>    regression—contract premise doesn't exist, task terminated; its probe value taken over by t16
>    read-only judgment (confirmed `strip_unit_prefix` is dead branch).
> 4. **Documentation commit's true criterion is "really build once"**: Round 2 closure's
>    documentation commit (`042c07860`) made Deploy Docs red at VitePress's Vue template
>    compilation—placeholders like `<rev>` / `<before-exe>` / `<after-exe>` in the record didn't go
>    into backticks, were parsed as elements (report `Element is missing end tag`, pointing to
>    `<rev>` line); markdownlint can't catch this kind, and pre-commit's markdownlint hook is judged
>    `no files to check` skipped in commits containing only docs (measured). Disposition:
>    placeholders uniformly go into backticks, before push locally run `npm run docs:build` once
>    (fix commit `0c837e1b4` proven by local build exit=0); if use `--no-verify` commit, commit-msg
>    check needs manual supplement.
> 5. **Process omission where contract description not synchronized**: t14's task description text
>    retained PA-1 old hypothesis (wrote "b.yx reports W1005" expectation), verifier in report
>    proactively pointed out "per contract field judgment, description text not counted" and asked
>    team lead to confirm—disposition = use task contract field as judgment basis, description text
>    left as process lesson (**task description must be synchronized with latest judgment chain when
>    re-dispatching/dispatching**).
>
> **IV. Mutation Experiment (mechanically reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (m1b/m2b/m4b are post-F2 compilable redoing versions—initial
> version M1/M2/M4 commented out if condition lines leaving dangling block, was invalidated by t8
> review F2 judgment "compile failure mistakenly recorded as criterion red"; redone version per
> patch cargo check exit=0 + trial red). M3 (collision name attribution backflow) initial version
> was real mutation, measured ref_pool 5 criteria red, kept original judgment. M1b anchor re-taken
> on b8380e6ab (after F1 fix probe section refactored, old anchor invalid). All trials: HEAD blob →
> mutated blob → reverted blob consistent with HEAD + git status CLEAN. Environment injection uses
> trial_env.ps1 (core.autocrlf=false via GIT_CONFIG_*).
>
> **V. Warning Surface Full Comparison (caliber first clarified)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) 4-tuple symmetric
> diff, project state channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: This round's before capture already includes all fixtures ⇒ matched set =
> universe 337, no round 1's +8 (matched increment) / +16 (universe) dual caliber problem. Result:
> only_before = 42 W1005 (26 files, all called methods' false positive elimination = phase B's −42;
> per file check all in behavior-run exit=0 baseline, method truly called executed—**no one true
> dead code saved**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface ×2 +
> ref-pool-test-pool's ../src/util.yx:2 = phase A criterion surface). **Snapshot Generation (F4)**:
> Phase A time point's after capture (phase B before 57 residual) and final version (phase B after
> 15+2=17) are **two generations**, must note when referencing; additionally 1 exception
> (interface_member_conflict.yx's Dog.name@18) belongs to field access form, not in phase B's W1005
> removal set, consistent before and after.
>
> **VI. Final Gate (closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator run no output change. **Gate
> Check Arrangement (Rule 13)**: This batch's pushed CI / Deploy Docs / Auto Translate run
> conclusions registered by scheduled check (run id + conclusion + push override relationship: later
> run covers former), not waited in session.
>
> **VII. Batch Tree State**
>
> yx49 and yx49-b both released (uniformly deleted at closure gc); no this batch's newly added
> directory outside warehouse.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)— Round 3 Closure (4.9.7)

> **4.9 Round 3 Batch Record (2026-10-17 implemented → 2026-10-17 closed)**: Covers 4.9.7
> (cross-unit method call resolution registration and receiver attribution, issue `#464`—WBS 4.9's
> last three-level task; this round after closure 4.9 entire second-level task kanban checked). Link
> = production side t19 → independent verification t20 PASS → review t21 needs_revision (F1:
> injection surface false positive) → fix round t24 → independent verification t25 PASS → review t26
> PASS (incremental, verdict=pass). All commit numbers are post-push true values (judgment method =
> patch blob same; `git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent).
>
> **I. Judgment Chain and Commits (post-push true value)**
>
> | Step | Content                                                                                                                                                                                               | Post-push Commit | Judgment Method        |
> | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ---------------------- |
> | 1    | Production side: cross-unit method calls produce D20 resolution entries (`feat(typecheck)`, t19)                                                                                                      | `b8b3cc3f6`      | Patch blob same        |
> | 2    | Driver side attribution: method reachability per definition unit gives marker, name groups with resolution records stop pool's name-level exemption (t20 verification PASS / t21 review caught F1)    | `d5c00235a`      | Patch blob same        |
> | 3    | Injection surface fix round F1: candidate injection per "name ← providing module" **pair by pair** filter, eliminate E1107 false positive (t24, including F2 statement and F3 nail form single point) | `d5dd1a37b`      | Patch blob same        |
> | 4    | Nail test return: F3 nail board in `src/frontend/core/typecheck/tests/environment.rs` migrated to declaration surface `src/driver/tests/overload_injection.rs`                                        | `c2f887843`      | Patch blob same        |
> | 5    | dev top                                                                                                                                                                                               | `e9675c9dd`      | merge commit, see "II" |
>
> **II. Hash Mapping and dev top's two merges (source not confirmed, don't write as member's
> action)**: Round 3's four commits (above table 1–4) all pushed to origin/dev;
> `git worktree list`'s main mount at `e9675c9dd`. dev top has two **non-this-team command surface
> produced** merge commits (`5da05b885` / `e9675c9dd`, reflog shows `pull --tags` + push form),
> source not confirmed—per rule 10 ④ (reference must be resolvable by anyone) and rule 13 (gate to
> dedicated check) same closure caliber, here only register fact and reflog evidence, no member
> attribution; also see "V." lesson ② (push attribution convention must be verifiable).
>
> **III. D20 Contract Extension's Actual Form (Key Form and Consumption Points)**: Production side
> packs "candidate / mixed IR name / declaration period parameter name" three parallel tables **same
> order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) follows unit
> cross-boundary, registry per module key ascending deterministic merge
> (`src/frontend/module/registry.rs:150`); typecheck arm per this unit import face pair-by-pair
> inject (`src/driver/arms.rs:354` → `visible_method_overloads`, `src/driver/arms.rs:1521-1579`:
> name ← providing module pair by pair, same name conflict full name skip, own key position not
> externally merged), nail form single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same order append + equal length
> debug assertion); consumer side attributes D20 resolution output per **definition unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`;`typed_owners` `:591`;`resolved_method_marks`
> `:621`), and lets name groups with resolution records stop pool's name-level exemption
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = entry name
> `{Type}.{method}#k` (`{Type}` is definition side bare name;`#k` declaration order).
>
> **IV. Key Scope and False Positive Defense Measurement (Fix Round F1 and Five Residual
> Boundaries)**: F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set"
> combination merged another module's same name type candidate into same key position ⇒ fixture
> `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, method same name same
> order) `check` reports 2×E1107, exit 1 (previously successful program compile failed); after fix
> `check`/`run` both exit 0, 4 W1005 truthfully report both sides' **uncalled** same name
> definitions (reverse missing report surface also verified). Checked-in fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`;driver-level nail
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green;main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set consistent item by item
> before and after fix (not rolled back). Five residual boundaries and per-item measurement
> (separate `check` definition file not report / name layer ambiguity E5006 full name skip /
> boundary(c) per t20 measurement rewrite / inline alias and `ns.T` not inject candidate surface /
> overall import `use M` and std candidate surface not produce entries) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §Cross-Unit Method Reachability Attribution
> "Residual Boundaries", not repeated here.
>
> **V. Process Events and Lessons (four types continuation)**:
>
> 1. **Contract wording written narrow directly becomes defect**: One of F1's root causes is
>    contract only wrote "only inject module surface that can be named by bare type name", didn't
>    write "attribute pair by pair by name"—implementation took literally "name set × module set"
>    combination, review caught E1107 false positive. When contract describes behavior boundary must
>    write **combinatorics** dead (per name pair by pair / full combination), cannot leave to
>    implementation self-understanding.
> 2. **Convention must be verifiable: "push by team lead" only written in dispatch message ⇒ when
>    third party push/merge appears cannot attribute**. This round dev top has two
>    source-unconfirmed merges (see "II"), finally relied on reflog reverse-deduction and withdrew
>    one error attribution. Verifiable form = members don't run `git push`/`fetch`/`pull`, sync
>    uniformly `pull --rebase`, use reflog to verify.
> 3. **Infrastructure unstable, first land patch then change person**: This batch total 5
>    unrecoverable failures (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background
>    sub-agent no message exit); disposition convention = produced evidence and patches first land
>    in **workspace** batch directory (`target/` under, not in repo), but **conclusion and numbers
>    must land in this record in-place**—workspace path must not be used as document reference
>    source (AGENTS meta rule ①). Then change person to continue, same session two consecutive
>    failures don't bet on third.
> 4. **Documentation commit's true criterion is "really build locally once"**: Round 2 closure's
>    documentation commit (`042c07860`) made Deploy Docs red at VitePress's Vue template compilation
>    (angle bracket placeholders `<rev>` etc. didn't go into backticks were parsed as elements,
>    report `Element is missing end tag`);markdownlint can't catch this kind, and pre-commit's
>    markdownlint hook is judged `no files to check` skipped in commits containing only docs
>    (measured twice). Fix commit `0c837e1b4` proven by local build exit=0. This round's lesson
>    red→green run: Deploy Docs **39011946793** (red) → **39014696984** (green, see "VI").
>
> **VI. Round 2 Gate Conclusion Supplementary Registration (Rule 13, scheduled check backfill)**:
> Round 2 closure commit `b940300ac`'s push—CI **39011946778** success / Auto Translate
> **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red, i.e.,
> "V.4"'s angle bracket placeholders;fixed by `0c837e1b4`);fix commit `0c837e1b4` after push three
> green: CI **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**. Push
> override relationship: CI@`0c837e1b4`'s **39014356642** was cancelled by later run per concurrency
> (cancellation ≠ failure, later run covers former, consistent with round 2 §VI caliber).
>
> **VII. Final Gate (closure revision `e9675c9dd`, local numbers; CI conclusion to scheduled check,
> see "VIII")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR
> snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent (rerun
> `scripts/docs/gen-error-code-docs.py` after `git status` clean);corpus baseline 346 → 347 → 348
> lines (each +1 behavior record, other lines byte-by-byte unchanged;baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 lines).
>
> **VIII. Gate Check Arrangement and Conclusion (Rule 13)**: This batch's post-push CI / Auto
> Translate / Deploy Docs conclusion to be registered by scheduled check (not waited in session,
> also don't write "didn't wait" as "already green"). **Backfill (2026-10-17 scheduled check)**:
> `d5c00235a`'s CI `39023417670` = **failure**—Docs Truth reports "error code document lags behind
> code table/locales", i.e., that commit missed generated object same submission;fixed by its
> follow-up `d5dd1a37b` (regenerate generated object):`5da05b885` CI `39027788246` / Deploy Docs
> `39027788268` / Auto Translate `39027788340` all success, covering former;`e9675c9dd` CI
> `39028827871` / Deploy Docs `39028827896` success;`d0cfc4e62` CI `39033963240` / Deploy Docs
> `39033963278` / Auto Translate `39033963261` success. Three commits' runs each ran to terminal
> state, this round no concurrency cancellation. Lesson: **"local generator idempotent" doesn't
> equal "generated object already in submission"**—before closure should per-commit check CI
> conclusion, not just look at latest commit.
>
> **IX. Batch Tree State (Closure GC, Status Output)**: Before closure `scratch.py status`—t25
> (holder=-) and yx49b (holder=-, previously held by deactivated member verify-r3, team lead
> `release --force` released) two trees no holder;`gc` deletes two, releases about 12.7 GB;after gc
> `status` = "no batch tree", `git worktree list` only main mount left. Additionally: `yx49r3` and
> `yx49r4` are not batch trees (no meta.json), are members' evidence/probe **workspace**
> directories, not "residual batch trees". No this batch's newly added directory outside warehouse.

## P4 4.4 Proof Layer and wasm Closing — Closure (4.4.1 / 4.4.2 / 4.4.3)

> **I. Batch Composition and Rounds (Batch name `yx44`)**: Three three-level tasks 4.4.1 / 4.4.2 /
> 4.4.3 with wasm attribute reachability judgment same batch;batch verification tree built by
> `scripts/dev/scratch.py init` in workspace `target/` (first full build;deleted after closure gc).
> Roles and rounds: implement 4.4.2 = t1 (12 files;local `1fc135437` → post-push
> `24028ffe8`) ｜ implement 4.4.1 layer order rewrite = t2 (only document/comments, zero behavior
> change, based on `b5e68b3c9`) ｜ implement 4.4.3 = t3 (11 files, including new code W1082;local
> `2d5e610cd` → post-push `52d2276ca`) ｜ wasm reachability judgment = t4 (only judgment, zero
> cleanup, judgment revision `b5e68b3c9`) ｜ 4.4.1 closure and one documentation commit = t8 (7
> files 173+/43-;local `29ebd2145` → post-push `d07993157`) ｜ independent verification = t5 (frozen
> `29ebd2145`) ｜ round 1 review = t6 (revision `1b593e993`) ｜ supplement = t9 (3 files
> 9+/10-;local `1ebddaf95` → post-push `1b593e993`) ｜ fix round = t10 (3 files 23+/23-;local
> `657c3d5e3` → post-push `e184d88b6`) ｜ round 2 review = t11 (`657c3d5e3` = post-push `e184d88b6`,
> patch-id all equal) ｜ closure = t7 (check / record / batch tree gc / documentation real build /
> one documentation commit;local `c8412565c` → post-push `da724940b`). **Failure and redo**: t6
> terminally failed with `needs_revision` (review round per contract must fail and give findings,
> not accident);t7 first round interrupted at batch tree gc stage, completed products left in main
> warehouse working tree not committed, second round took over "fill gap + review"—didn't redo
> completed parts (this round new: 4.4 check and evidence annotation, this record, three rewrites of
> paths pointing to deleted batch directories, documentation real build and closure commit). No
> cancelled rounds.
>
> **II. Per-Task Output and Measurement Numbers (raw commands and output see evidence summary and
> retention;only conclusions recorded here)**:
>
> - **4.4.1**: `layers/README.md` (including `layers/mod.rs` top comment)'s layer order table
>   rewritten to **actual call order** (predicate → termination → ownership, call point anchored
>   `checker.rs::check_module_impl`), `equivalence` per real identity description (no production
>   call point);same source declaration 7 places closed (`01-routing` / `02-stage-contract` /
>   RFC-039 / `06` / typecheck README / layers two files);mechanical reconciliation script on batch
>   tree green baseline **11/11 PASS** (criterion 11 items = README side 4: actual call order
>   declaration / two entry anchors / old order declaration disappeared;code side 7: 5 line
>   anchors + `check_module_impl` no short circuit + `equivalence` zero production
>   call;reconciliation script is one-time script, reclaimed with workspace, 11 criteria listed one
>   by one can be rewritten accordingly), mutation (c) revert old declaration → reconciliation 2
>   FAIL i.e. red.
> - **4.4.2**: Solver acquisition surface unified to `proof/smt/backend.rs`'s process-level shared
>   singleton + unique closure entry `with_shared_solver(consumer, f)`;three consumption points
>   (predicate / termination / ownership) all changed to shared entry, **compile production chain
>   `default_solver()` call point = 0** (only backend selection point sank to private
>   `new_backend()`). `wasm/src/lib.rs`'s `test_smt` self-check call point is **existing** (earlier
>   than this batch, from `b5e68b3c9`) non-test call point, belongs to "test and independent scene"
>   surface, doesn't constitute D58 prohibition violation. Independent verification self-built
>   reading (one compilation): predicate acq=2 hit=1 miss=1 ｜ termination acq=1 hit=2
>   miss=2 ｜ ownership acq=2 hit=1 miss=1, three points hit/miss sum (4,4) = shared instance
>   accumulated `Some((4,4))`;mutation (a) singleton degenerate → predicate hit=0 i.e. red.
>   **Modification Surface Disclosure**: That commit includes `z3_backend.rs` 15 lines—`cache_stats`
>   migrated from inherent method to `Solver` trait block (return `Some(instance-internal count)`) +
>   1 line comment changed reference, no semantic coupling. D58 prohibition mechanized: same thread
>   reentry → panic contains D58, probe `recv_timeout(120s)` doesn't hang.
> - **4.4.3**: `body_checker` missing silent downgrade converged to `TypeChecker::ownership_inputs`
>   (unique criterion point) + new code **W1082** (not overlapping with existing W code facts: W1080
>   constraint downgrade / W1081 solver missing / W1090 opposite direction);nail test 2/2
>   green;healthy path corpus 348:348 zero diff;**downgrade branch production surface unreachable
>   exhaustive holds** (CLI four entry + eval/repl + format + LSP + wasm playground + all
>   `ProgramKind` converge to `check_module_impl`:`init_body_checker` unconditionally before
>   ownership block, function interior 0 return/break/continue, `body_checker` no `.take()`) ⇒ W1082
>   no user-visible surface. Mutation (b) delete emission → nail test red. **W1082 belongs to
>   external contract table change (Class E), recognized by user**;`locales/zh.json` /
>   `locales/en.json` same submission as code, other languages supplemented by i18n bot after push
>   (run see "IX").
> - **wasm Attribute Reachability Judgment (t4)**: `orchestrator.rs` current tree **10 items = 4
>   positive arms + 6 negative arms** (caliber = same line's `#[cfg(...)]` attribute text, positive
>   and negative both counted;`cfg!` runtime macro 0 hit);4 wasm positive arms unreachable at
>   playground runtime but **compile-time load-bearing** (call point not gated, separately delete
>   arm ⇒ wasm compile failure), 6 `not(wasm32)` negative arms native three stage table load-bearing
>   ⇒ D58's "delete unreachable" in this file **has no applicable target**, real cleanup form =
>   entire project-mode subtree gated on wasm32 (registered P10 S7, three unverified original
>   retained).
>
> **III. Independent Verification (t5, frozen `29ebd2145` = post-push `d07993157`'s content—two
> revisions per commit patch-id all equal, whole tree difference only in `docs/src/en/**`,
> `docs/src/.i18n-cache.json` etc. bot output surface—`git range-diff` three commits all `=`)**:
> Gate **8/8 all green**—`cargo fmt --check` 0 ｜
> `clippy --all --all-features --all-targets -- -D warnings` 0 ｜ `cargo test` **3128 passed / 0
> failed** (7 test targets total;baseline 3126 + this batch 2) ｜ corpus diff **348:348**
> line-by-line zero difference (sort diff 0 lines) ｜ IR snapshot `UPDATE_SNAPSHOTS=1` rerun after
> `git status` zero drift ｜ `check-obligations.py` 21 = 13 + 8, containment 0 gap ｜
> `check-docs-truth.py` source 156 / document 156, CLI 21 commands consistent ｜ error code
> generator rerun before and after tree both clean. Build warning only 4 bot language W1082 missing
> entries (known state) + Z3 download noise, zero code warning. **Contract verify caliber correction
> (must two JSONL paths)**: Corpus gate's call caliber =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generation>.jsonl`—**two
> JSONL paths must be given**;missing second parameter is comparing baseline to baseline (constant
> zero difference, criterion idling). This batch executed and evidenced per two-path caliber (t5's
> `corpus-r44.jsonl` product).
>
> **IV. Mutation Evidence (3/3, mechanically reversible)**: All via
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per-file blob
> vs HEAD comparison → require `git status` clean):
>
> - (a) Singleton degenerate → shared reading predicate hit=0 miss=2, test FAILED exit
>   101;`backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → revert = HEAD.
> - (b) Delete W1082 emission → nail test FAILED exit 101;`checker.rs` HEAD=`70c04a41e020` →
>   mutation `db0a623c9b9a` → revert = HEAD.
> - (c) Revert old layer order declaration → reconciliation 2 FAIL exit 1;`layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → revert = HEAD.
>
> Three trials' revert `git status` all CLEAN. Trial (c) first run failed because absolute path was
> resolved by WSL exit 127 (not criterion red), changed to relative path rerun as standard.
>
> **V. Review Rounds and Conclusions**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision**—3 findings all are documentation line number anchor inaccuracy (F-01 medium: `02`
> §"wasm current state" 4 group outdated line numbers;F-02 low: `02` and `09`'s 4.4.3 delivery
> anchor `checker.rs:1283-1293`;F-03 low: `06`'s comparison statement), four substantial deliveries
> all pass.→ Fix round (t10, local `657c3d5e3` = post-push `e184d88b6`, 3 files 23+/23-, with two
> mechanical criteria) → Round 2 (t11, same revision) **verdict = pass, new finding = 0**. **Final
> check basis = round 2's pass**. **Round 3 (closure tip `da724940b`)**: independent verification
> t12 **verdict = pass** (bound `da724940b`: four rule texts byte-by-byte, 3.19 / S7 / S8 fields and
> scope complete, count table and judgment chain recount, dead reference zero, `check-docs-truth` 0,
> `markdownlint` 0);review t13 **verdict = needs_revision**, 4 findings—F-01 medium (t10 / round 2
> only gave pre-rebase hash, §14's list inconsistent with origin/dev actual), F-02 low (`02:379`'s
> 09 4.2.8 anchor pushed to `:274` by this batch's own insertion of one line each at
> `09:144`/`:182`), F-03 low (`09a` writes local drive letter path), F-04 low (three-level count row
> annotation and 157 ≠ table header 156) → fix round t14 (this commit) closes one by one. **t10
> contract caliber ruling**: Fix surface based on **task record's 5 F-01…F-05** (F-04/F-05 are
> mechanical criteria), dispatch brief is not criterion.
>
> **VI. User Ruling (2026-10-18) and Rule Solidification**:
>
> 1. **D58 entry name in-place rewrite**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, in-place correct body, **no revision note
>    written** (t8 completed in 4.4.1 one commit).
> 2. **wasm judgment registered as P10 S7** (ruling B): with "belonging phase: P10;whether blocks
>    main line: no" + three unverified **original retained**;`06`'s F2 "needs decision" row points
>    to this registration;t4 produced "### F2 Judgment" subsection untouched.
> 3. **Crash item registered as P3 3.19** (ruling A): symptom / cause point to
>    `proof/smt/translate.rs`'s sort inference / D59 **A/B class** (crash) / acceptance row (this
>    form no longer abort—conservative downgrade or diagnostic, corpus and IR snapshot zero
>    drift);**characterization = pre-existing** (t5 two revision comparison, see "VII" ⑤).
> 4. **Rule solidification one** "correction in-place written into body, no old-new comparison
>    retained": lands `coding-rules.md` (new clause), `07-equivalence-oracle.md` §Semantic
>    correction split judgment clause 3, RFC-039 **D59 row** clause 3 (same submission registration
>    = WBS row marked error type + in-place correct affected body + PR must-fill block "error type"
>    one line).
> 5. **Rule solidification two** "contract `inScope` uniformly write path prefix resolvable by path
>    matcher": lands `AGENTS.md` team scheduling convention rule 6.
> 6. **New code W1082 recognized** (external contract table change, Class E).
> 7. **Register P10 S8** "documentation correction form convergence (in-place rewrite, delete
>    revision note)": includes 7 audit list scope and acceptance row.
> 8. `09` top count table: second-level **63** (60 + 3.19 / S7 / S8), three-level **157** (154 + 3),
>    with 2026-10-18 annotation;three-level item's account review see "XV".
>
> **VII. Process Facts Must Be Truthfully Recorded**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks—shared working tree had teammates'
> in-progress work, `--fix` would incidentally modify others' files;**equivalent gate already
> manually fully run in final tree** (fmt --check / clippy `-D warnings` both 0). ② `pre-commit`'s
> stash window indeed happened (`Stashing unstaged files…` / `Restored changes from…`), after commit
> `git status` check shows teammates' in-progress work intact. ③ **4.4.3's inScope lesson**: Prose
> limited `docs/src/ (only error code generated objects)` was rejected by `changedPaths` check ⇒
> `inScope` uniformly write matchable path prefix (already solidified as rule, see "VI" 5). That
> task actually 11 files, of which 7 because declaration writing couldn't be resolved by path
> matcher weren't listed in `changedPaths`, listed one by one in task result (`codes/w1xxx.rs`,
> RFC-013 code table, `reference/error-code/`'s index and each E section page etc.). ④ **W1082's
> locale window closed**: `locales/zh.json` / `locales/en.json` same submission as code, other
> languages supplemented by i18n bot after push (run see "IX"). ⑤ **Z3 `Sort mismatch` crash
> judgment (t5 criterion ⑦)**: **pre-existing**—same probe, same input at `b5e68b3c9` and frozen
> revision `29ebd2145` stdout **byte-by-byte consistent**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1;static attribution consistent (4.4.2 to `translate.rs` 0 line changes, to
> `z3_backend.rs` only `cache_stats` migration). **Downgrade branch production surface unreachable
> exhaustive holds** (W1082 no user-visible surface). Probe three lines source:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **t5 frozen surface and t9's coverage
> difference**: t9 (three places comment/document supplement, `1b593e993`) commit later than t5's
> frozen revision `29ebd2145`, belongs to zero behavior change, its acceptance covered by t9's own
> mechanical check + t6 review (t6 at `1b593e993` **reran** t5's key measurement, not retold). ⑦
> `docs/src/en/**` not manually modified, regenerated by post-push auto-translate workflow (this
> batch's bot landed two commits: `92d490695` i18n locales supplement W1082 other languages,
> `63b6cdf37` auto-translate documentation). ⑧ **t9 out-of-scope observation's three rulings**:㈠ t9
> incidentally corrected `02` same sentence's `wasm/src/lib.rs` line count and gate point line
> numbers (73 / 139·153·170 → 152 / 151·170·187)—**accept** (same caliber same number required
> surface, commit info already listed);㈡ RFC-039 **D35 row** still writes "wasm 27 files' cfg
> branches"—**keep unchanged** (ruling row is historical ruling record, its premise correction
> carried by P10 S7 registration, no rewrite history);㈢ `docs/src/en/**`'s old entry name
> translation aligned by auto-translate. ⑨ `09` / `09a` originally two places pointing to deleted
> batch directory paths changed to **in-place inline** at closure round (evidence summary merged
> into this record's section, probe three lines source inlined). **2026-10-21 Correction**: What was
> changed to point to was still workspace path at the time, equivalent to moving the violation down
> one level;from this round summary's delivery commit, gate numbers and CI run id all inlined,
> document no longer references any workspace path (AGENTS meta rule ① / rule 10 ④).
>
> **VIII. t8 Acceptance ④ Caliber Ruling**: Acceptance original text required
> `git diff | grep '修订注记\|原为\|现改为'` to be empty. Measured **default context caliber
> unreachable**—D58 row's adjacent row D59 / D61 ruling rows themselves contain "修订注记 / 原为"
> wording (not this task's changes). Ruling: per **added lines / deleted lines / `-U0` increment
> lines** three calibers all empty accepted, and write **subsequent similar acceptance only grep
> added/deleted lines**—criterion should land on this change's lines, not entire diff's context.
>
> **IX. CI Gate Conclusion (AGENTS rule 13 ①, scheduled check 2026-10-18 17:26 backfill;run id and
> conclusion in-place inline)**: First push (3 commits, tip = `d07993157`): CI **39110908163**
> success ｜ Deploy Docs **39110908155** success ｜ Auto Translate Docs **39110908193**
> success ｜ Auto Translate I18n **39110908144** success;that tip covers `24028ffe8` (4.4.2) /
> `52d2276ca` (4.4.3) / `d07993157` (4.4.1) three commits' all content. Second push (1 commit, tip =
> `1b593e993`): CI **39112293391** success ｜ Deploy Docs **39112293062** success ｜ Auto Translate
> Docs **39112292903** success;covers all four commits including t9 supplement.**No concurrency
> cancellation**, latter covers former. Third push (2 commits, tip = `da724940b`): CI
> **39121538555** success ｜ Deploy Docs **39121538562** success ｜ Auto Translate Docs
> **39121538541** success—three runs' headSha all = `da724940b` and **all in terminal state**
> (2026-10-18 third round scheduled check backfill);round 3 independent verification t12's "CI not
> terminal state" observation closed. Unrelated: Nightly **39081392694** failure's headSha =
> `290f9ebc`, not this batch's commit surface.
>
> **X. Three `pull --rebase` (auto-commits from peer during push)**: Peers were auto-translate and
> i18n commits; six hashes rewritten and **patch-id all equal one by one**—local `1fc135437` →
> `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`, `1ebddaf95` → `1b593e993`,
> `657c3d5e3` → `e184d88b6`, `c8412565c` → `da724940b` (after third rebase origin/dev head =
> `da724940b`);`git range-diff` verified **per commit content unchanged** (three commits all `=`).
> Members didn't execute `git push` / `fetch` / `pull`, sync and push uniformly done by team lead.
> **Forensics reminder**: From now on reference commit numbers uniformly use **post-push hash**
> (right values above);`09`'s check row per convention doesn't write commit numbers.
>
> **XI. Uncovered Surface (must not write "not tested" as "passed")**: ① doc-tests 0/0 didn't carry
> criterion;② **wasm32 target actual compile not done** (this surface covered by CI's
> `_build-wasm.yml`, this batch's wasm judgment is static argument);③ mutation only covers 3 places
> named in contract, other gates didn't do reverse damage verification;④ corpus diff didn't run
> separately on `b5e68b3c9` (only ran on frozen revision);⑤ i18n bot supplemented other languages
> W1082 text behavior not verified (only verified `zh.json` / `en.json`);⑥ round 1 review and fix
> round reported **same class but not in contract findings existing drift** not fixed (list see
> "XII"), attribution pending user ruling.
>
> **XII. `02` Whole Existing Drift Observation List (Reported to User, Pending Ruling)**: Caliber =
> all are **pre-existing drift** (not introduced by this batch), **doesn't constitute this batch's
> finding**;list discovered by fix round, round 2 review at `657c3d5e3` (= post-push `e184d88b6`)
> independently sampled and gave mapping, copied one by one (list in-place inline as follows,
> original files reclaimed with workspace):
>
> 1. `02` §(a) entry table 3a-3d / 5 / 5b:`02:72` quotes `orchestrator.rs:273-399` → current
>    `:155`;`02:73` `:450` → `:250`;`02:74` `:1374` → `:985`;`02:78` `util/diagnostic/mod.rs:565` →
>    `:681`;`lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` no longer at that place;**content
>    level**:`run_diagnostics` and "otherwise manual lex→parse" already replaced by 4.6.1 (delete
>    manual sequence).
> 2. `02:1016`:`compile_project :99-237` → current `:107-134` (28 lines Program constructor).
> 3. `02:360-361`:"missing downgrades to warning" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435);`build.rs:65-72` already changed to `panic!` hard
>    failure;`_build-wasm.yml:35-59` interval needs re-taking.
> 4. `02:127` mechanism pointer: mechanism actually in `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-statement);conclusion still holds (multi-file arm drops warnings), inaccurate
>    is "result's only downstream use" sentence (`type_result` also via `arms.rs:1389` →
>    `helpers.rs:259/:636` feeds `overload_resolutions`);suggested fix = pointer change to
>    `typecheck_orchestrated`, "only downstream use" change to "warnings field no read point".
> 5. `02:128`:`check_project:385-395` / `250-252` need re-taking.
> 6. `02:334` (§S5 (d) investigation section): cited old code snippets already replaced by 4.4.3's
>    `ownership_inputs` criterion point.
> 7. `02` change list table other rows:`predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This closure doesn't preset S9 WBS item** (user not ruled);obtain ruling then register per
>    registration form in `09`.
>
> **XIII. Batch Tree Recovery (GC and Status Raw Output;Tree Paths in Output Recorded per Warehouse
> Relative Path)**: `python scripts/dev/scratch.py gc` →
> `[scratch] already deleted yx44-evidence (released about 0.0 MB)` /
> `[scratch] gc completed, total released about 0.0 MB`;`python scripts/dev/scratch.py status` →
> `yx44-evidence            holder=-` next line `<workspace>/yx44/tree`—registry recorded path
> **doesn't exist**, that name is registry residue (`gc`'s `_remove` deletes is `meta["path"]`'s
> parent directory i.e. workspace's `yx44`, residual `meta.json` still in scan surface;its content
> batch=`yx44`, `base_rev=29ebd2145`, `lock=null`, includes t5 / t6 / t11 hold history);closure
> round renamed that registry file to `meta.retired.json` to make it exit scan (**not delete any
> evidence file, not create any new tree**) → `python scripts/dev/scratch.py status` =
> `[scratch] no batch tree`;rerun `gc` =
> `[scratch] no batch tree, no need to recover`;`git worktree list` only main mount left (`dev`).
> Batch original evidence directory (no `meta.json`, **not batch tree**) at that time kept as
> retention;**from 2026-10-21 workspace directory no longer as evidence source**—closure summary
> already inlined in this section, original directory deleted with cleanup (AGENTS meta rule ①).
> Acceptance expression (resolvable): `git status` clean;batch tree after gc
> `python scripts/dev/scratch.py status` = no batch tree;`git worktree list` only main mount left;no
> this batch's newly added directory outside warehouse.
>
> **XIV. Documentation Real Build and Commit Hygiene**: `npm run docs:build` local **exit 0**
> (AGENTS rule 10 ⑨—markdownlint can't catch VitePress's Vue compilation error, and commits
> containing only docs will be judged `no files to check` skip markdownlint);this batch's check /
> annotation / registration / rule modification accumulated into **one** documentation commit (rule
> 8), landed in batch closure;members didn't execute `git push` / `fetch` / `pull`. **This batch's
> final two commits both pushed** (closure tip's `git rev-list --count origin/dev..HEAD` = 0,
> origin/dev head = `da724940b`):`e184d88b6` (round 1 findings' documentation anchor fix, 3 files
> 23+/23-) + `da724940b` (closure documentation commit, 7 files 96+/10-;includes check / record /
> three path rewrites / registration and rule modification).
>
> **XV. Count Table Account Review (Round 3 Fix F-04;method and result both verifiable)**: Judgment
> target = `09` top "Position and Scope" table's three-level count row. **Method** (two independent
> calibers, per revision use `git show <rev>:<file>` to take bytes recount): ① **Sum per
> second-level entry**—take each second-level kanban row's parentheses "N three-level tasks" (row
> beginning `- [x]`/`- [ ]` bold title beginning, row contains "个三级任务", including S7 row inline
> written "1 个三级任务"): current tree 70 second-level entries, **sum = 157**. ② **Historical
> self-consistency**—same caliber at `366bc5a69^` gets 66 entries, **sum = 151 = current period
> table header 151** (self-consistent);`366bc5a69` day newly registered `4.9.6` and `0.4.1` two
> three-level (sum 151 → 153), while table header only writes 151 → 152 ⇒ **table header since that
> commit undercounted 1**;each subsequent revision differs by 1 (`92d490695` / `1b593e993` /
> `d07993157` / `e184d88b6`:sum 154 vs table header 153;`da724940b`:sum 157 vs table header 156).
> **Result**: true value = **157**, so correct table header (`09:13` three-level 156 →
> 157);annotation 144+2+2+2+2+1+1+3 = 157 originally correct, not changed;this batch's previous base
> = **154** (not 153). **Second-level count row not touched**—this row's caliber never aligned with
> kanban entry count (at `366bc5a69^` is 66 entries vs table header 59), this time no verifiable
> evidence to overturn 53 + 10 = 63, not change number to make it match. Attached correction (same
> round): §II's 4.4.1 parenthetical decomposition per reconciliation script actual item count
> changed to "README side 4 + code side 7 = 11", consistent with **11/11** and script 11 `ck` calls.

## P3 Debt Round 1 Closure (3.5 / 3.6 / 3.4.6 / 0.5)

> **I. Batch Composition and Rounds**: This batch closes P3 side remaining debt—**3.5 Higher-order
> type parameter call path** (3.5.1–3.5.3), **3.6 Function type value call path** (3.6.1–3.6.4),
> **3.4.6 compile path consumes warnings** (Class D, user 2026-10-19 ruling "do"), and registers
> together **0.5 Test suite layering** (P0, issue #486 slice) and **3.20 Dead code analysis existing
> false positive** (3.20.1 / 3.20.2). Roles and rounds:
>
> - Implementation: 3.5.1–3.5.3 = t1 ｜ 3.6.1 = t2 ｜ 3.4.6 = t6 ｜ 3.6.2 = t7 ｜ 3.6.3 = t8 (main
>   body + closing two commits) ｜ 3.6.4 = t9 ｜ 0.5.1 = t10 (#486 slice)
> - Independent verification: round 1 = t3 (frozen revision `81d5e414e`, covers 3.5 / 3.6.1–3.6.3 /
>   3.4.6) ｜ 3.6.4 = t11 (round 2, frozen `8e32e8636`)
> - Review: round 1 = t4 (verdict = **needs_revision**, F1/F2/F3) ｜ fix round = t12 (one
>   commit) ｜ round 2 = t13 (verdict = **pass**)
> - Closure = t5 (check / registration / this record / batch tree gc / documentation real build /
>   one documentation commit)
>
> **Failure and redo**: t4 terminally failed with needs_revision (review round per contract must
> fail and give findings, not accident) → t12 real fix → t13 review pass. t8 internal one attempt
> interruption (process crash), previous round evidence retained in batch directory, continuation
> attempt supplemented two commits to inventory. No cancelled rounds.
>
> **II. Per-Task Output and Measurement Numbers (conclusion caliber;raw commands and output see each
> batch evidence directory)**:
>
> - **3.5.1** (unifier matches constructor variables in head position): Criterion converged to
>   **single implementation** `is_ctor_reference` (`pub(super)` free function, warehouse 1
>   definition 3 consumption)—this is t12's real fix for t4's F1 (originally
>   `inference/statements.rs` had another same-name method copy, constituting same-name two
>   implementations in prohibition three meaning). Head renaming no longer inlined
>   copy:`environment.rs` and `expressions.rs` each first traverses TypeRef / variable table to
>   construct head_map, then uniformly gives shared implementation `substitute_ctor_heads`
>   (`types/substitute.rs`) to execute—two places' head_map construction criterion unchanged,
>   renaming only acts on head by name, empty table returned as-is (t12's F2 real fix). Equivalence
>   argument: traversal doesn't change Generic head names.
> - **3.5.2** (explicit type actual argument accepts type constructor, E1010 disappears): Red state
>   E1010 / E1002, green state exit 0.
> - **3.5.3** (mono product replaces constructor variable): Symbolic type actual argument criterion
>   narrowed per container generic parameter name;constructor variable in product replaced with real
>   constructor (IR dump verified:specialized function `Function probe(Box, int64)`).
> - **3.6.1** (local function type value called by name): Criterion single source =
>   `is_static_fn_name` (warehouse 1 definition 1 call), aligned with variable evaluation path's
>   name resolution priority (local → global), no second resolution table added;IR verified:module
>   initialization segment changed from `LoadConst String("f")` + `CallNative` to `LoadGlobal` +
>   `CallDyn`. Regression surface:same name "function binding + value binding" unreachable
>   (uniformly compile period E2010), non-function value called by name still E1065.
> - **3.6.2** (`use` alias imported function value called by name): Originally reported
>   `E6006 Native function not found: lib.f` (`use lib.{f}` and `use lib.{f as g}` two forms). Fix =
>   **add one arm** in `is_static_fn_name` (not copy), added unique implementation
>   `use_alias_value_slot` handles "use alias → qualified name → global slot" two-table joint query,
>   consumption points 2 (call form criterion + variable evaluation path).
> - **3.6.3** (nested function body references outer frame value): **Criterion changed**—not "make
>   it run correctly", but this form per RFC-009 §2.3 + SPEC §12.3 should be **compile period
>   rejected** (D59 Class C:change silent wrong value to loud;per `09` §P3 D59 row, "silently give
>   wrong answer" belongs to Class C—this batch closure brief recorded as "A/B class", both refer to
>   "old behavior not successful", landing form is both "red before green + compile period loud
>   rejection"). Original form = **no error, calculate wrong value (void)**: red state stdout
>   `void`, EXIT=0 (silent wrong value retention);green state compile period E1001
>   `Unknown variable`, EXIT=1, **same code same text** as existing "unknown variable" path.
>   Criterion single source = capture barrier (`enter_capture_barrier`), compile period name (type
>   parameter) not blocking;E1001 still only emitted in typecheck, registered unique. **Assignment
>   path caught by E2010** (t8's closing commit:declaration / assignment judgment doesn't apply
>   capture barrier)—otherwise this fix itself would open a new "same name assignment silent wrong
>   value" channel (c1 form:only barrier degenerates to exit 0 print 10).
> - **3.6.4** (`use lib` overall import then `lib.f(...)`): Takes FieldAccess callee's namespace
>   branch, originally reported E6006 `lib.f`. Fix = namespace branch resolves qualified name then
>   first judges global slot layout → change to indirect call;qualified name resolution and slot
>   query reuse `resolve_namespace_data_access` (same source as data access), no resolution table
>   added.
> - **3.4.6** (compile path consumes warnings, Class D): Change before and after external
>   performance = **multi-file `run` previously always didn't output any W code** (red state
>   retention:multi-file [] vs single-file [(W1001,3),(W1002,5),(W1003,1)]), now four paths
>   (single-file run / multi-file run / single-file check / multi-file check)'s W code set
>   constantly = {W1003@1, W1001@3, W1002@5} (non-empty set). Corpus baseline:of 348 records **46
>   changed**, new W code **118** (W1001 34 / W1002 12 / W1003 13 / W1004 42 / W1005 17), **118/118
>   item by item attributed to file's real existing unused definitions (0 unlocated)**, pure
>   increment (0 deletion, 0 exit / stdout / check change). **D59 four fulfilled**:① red before
>   green (red state retention + turn to nail test) ② doesn't change "previously successful
>   program"'s external performance (pure increment, 0 exit / stdout / check change) ③ same
>   submission registration (this record + `09`'s 3.4.6 row) ④ baseline same submission regenerate
>   and manually review diff. **Superset approved**:W1004 / W1005 / W1006 also enter `run` (user
>   2026-10-19 approved), reason = **one criterion, one channel**—only putting W1001/W1002/W1003 is
>   equivalent to creating second filtering ruling. **External contract surface
>   change**:`orchestrator::compile_project` return value changed from `Result<ModuleIR, _>` to
>   `Result<(ModuleIR, FileDiagnostics), _>` (warehouse 2 consumers + 2 test direct drive points
>   synchronously adapted).
> - **0.5.1** (test resource profile marker + exclusive lock + fast/full minimum consumption
>   surface): Added `scripts/dev/testsuite.py` (`markers --check` and `cargo test --list`
>   bidirectional reconciliation, `plan` budget bin packing, `run fast|full` budget insufficient
>   reports gap not silently skip, `manual` file never implicitly executed) and
>   `src/util/test_support.rs` (cross-process file lock, test body takes lock ⇒ any call path
>   mutually exclusive);in-place `cost:` markers 3 (2600 file exclusive, peak from 5000 file
>   measured linear extrapolation, **not measured**;5000 file exclusive, 15.6 GiB / 72.6 s
>   measured;snapshot update manual, peak and time both extrapolated). Self-verification
>   numbers:`markers --check` checked 3149 `#[test]`, 3 mechanical mutations each exit 1, `run fast`
>   batch wave 2793 passed / 47.7 s + exclusive wave 1 passed / 14.6 s, budget 1 GiB when `run full`
>   lists gap 7.11 GiB, two process lock competition measured `held→blocked→held(waited 2.0s)`.
>   **Independent verification belongs to #486 subsequent scheduling** (this batch only has
>   implementer self-verification). **0.5.2 pending scheduling** (pre-commit / CI wiring + 2600 file
>   measured calibration + `test-specification` detail section).
>
> **III. Independent Verification (t3, frozen `81d5e414e`;red state baselines `c599de7bb` /
> `1725d0215` / `ef2334de5` all are HEAD ancestors, `git merge-base --is-ancestor` verified)**:
> Probes all self-built by verifier (batch tree built by `scripts/dev/scratch.py init` in workspace,
> probes and patches in its `probes/` / `patches/` subdirectories), doesn't retell implementer
> conclusion;that batch tree together with its tree-internal evidence deleted in this batch
> closure's gc (see "IX"), so this section's measurement conclusions and numbers **as-is recorded in
> record** for retention. 3.5 red state four probes all report E1003+E1002 / E1010, green state exit
> 0 and value correct (stdout 1 / 7 / 1), and has IR value-level evidence (specialized function
> `Function probe(Box, int64)`);3.6.1 green state output 6 / 10, compared to "value position call"
> green, "non-function value called by name" compile period E1065;3.6.2 red state two forms E6006,
> green state exit 0 (probe interior `assert` failure is non-0, exit 0 is value correct);3.6.3 red
> state stdout `void` / EXIT=0 (silent wrong value verifiable), green state compile period E1001 /
> EXIT=1, while **expression position inline lambda captures this function's parameter** passes
> (EXIT=0, output 6)—pass surface not accidentally harmed. 3.4.6's C2 normalization independently
> reproduced by verifier (four paths W set identical, non-empty), baseline diff independently
> recounted (changed=46, added=0, removed=0, expectation / run.exit / run.stdout / check change = 0,
> new W code heads 118 consistent with implementer), attribution sampling 10 all hit real
> definitions. Gate on frozen tree all green:fmt / clippy (`-D warnings`) / `cargo test --all`
> (**lib 2790 passed + 2 ignored, integration 297 passed**, 0 failed) / corpus diff 348:348
> difference empty / IR snapshot 204 zero drift / obligation ledger / docs-truth. **t11 (round 2,
> frozen `8e32e8636`)** independently reproduced 3.6.4's red→green (red state parent commit
> 16e2a5c88:E6006 `lib.f`, EXIT=1;green state EXIT=0, stdout 27, probe interior 4 assertions all
> passed), criterion single source (`resolve_namespace_data_access` 1 definition 2 consumption, no
> third "qualified name → slot" joint query), no regression eight forms all passed, verdict =
> **pass**.
>
> **IV. Review Rounds and Conclusions**: Round 1 (t4, judged `20abd3ebc`'s parent revision) verdict
> = **needs_revision**, three findings all are real defects:F1 = `is_ctor_reference` same name two
> copies (`inference/statements.rs` method copy), F2 = head renaming in `environment.rs` and
> `expressions.rs` each inlined one copy, F3 = visibility out of bounds (what should be private not
> private, what should be `pub(super)` not converged).→ Fix round t12 (one commit, 4 files +96/−85)
> → Round 2 (t13, reviewed revision `20abd3ebc`) verdict = **pass**, new finding = 0:8 self-built
> probes' value and exit code consistent with pre-fix item by item, 4 IR dumps **byte-by-byte
> consistent** with round 1 green state, corpus 348/348 zero drift, IR snapshot 204 zero drift,
> `cargo test --all` EXIT=0 (lib 2794 passed + 2 ignored, integration 297 passed, 0 failed), fmt /
> clippy / docs-truth all exit 0, `check-boundary`'s `pub(crate)` count 167 / frontend-core 26 (=
> 3.5 pre-baseline), `check-fanout` same name definition zero hit. **Final check basis = round 2's
> pass**.
>
> **V. Mutation Evidence (all via `python scripts/dev/scratch.py trial`, apply → run command → apply
> -R → per-file blob comparison + require `git status` clean)**:
>
> | #   | Target                                        | Mutated Blob    | Capture Result                                                                                                                                                        |
> | --- | --------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | M1  | 3.6.1 `is_static_fn_name`'s global lookup arm | `022986cbea65…` | Two probes reproduce E6006, EXIT=1 (after revert blob = HEAD, CLEAN)                                                                                                  |
> | M2  | 3.5.2 explicit type actual argument branch    | `abbcc5ecf116…` | Probe check reports E1002, EXIT=1 (compared function form still green ⇒ attribution precise to 3.5.1)                                                                 |
> | M3  | 3.4.6's `MULTI_FILE_STAGES` dead code arm     | `372f7b92755d…` | run channel W code all off (C2 red);M3b attribution shows check channel criterion still solid, what's broken is exactly 3.4.6's newly opened compile external channel |
> | M4  | 3.6.3 capture barrier entry                   | `7a7204c60383…` | Capture type turn to nail test 4 failed, TEST_EXIT=101                                                                                                                |
> | M5  | 3.6.4 value slot → CallDyn block              | `25853376639b…` | Probe reproduces E6006, EXIT=1 (blob chain `0d837e9d30de…` → `25853376639b…` → `0d837e9d30de…`)                                                                       |
> | M8  | 3.5.1 turn to nail test                       | —               | After fix rerun still can fail (E1003, TEST_EXIT=101, mechanically reversible revert CLEAN) ⇒ criterion not lost fail-ability                                         |
>
> **VI. User Ruling and This Batch Registration**:
>
> 1. **3.4.6 = Class D** (externally observable:multi-file `run` new W code), user 2026-10-19 ruled
>    "do";**superset (W1004 / W1005 / W1006) also approved**, reason = one criterion / one channel.
>    Original 2026-10-15 ruling C "per decision B1 defer" **prerequisite** (4.9 pool semantic
>    correction) closed 2026-10-17.
> 2. **3.6.3 criterion changed**:from "make it run correctly" to "nested function body references
>    outer frame value → compile period E1001";assignment path caught by E2010.
> 3. **Two "earlier than this task" silent form registration not changed**:① same-block brother
>    nested function reference—baseline i.e. E1001 (outer block local family), this batch doesn't
>    expand scope;② nested function annotation reference outer type parameter still prints `void`
>    (t1 registered item). Both **not** introduced by this batch, both original retained.
> 4. **3.20 "Dead Code Analysis Existing False Positive" registered** (user 2026-10-19 agreed per C
>    disposition then register):3.20.1 = W1001 doesn't track in-closure references
>    (`docs/src/reference/stdlib/test.md`'s `t_add` form:referenced in closure but reported
>    unused);3.20.2 = W1003 judges variant destructuring required import as unused
>    (`docs/src/reference/stdlib/weak.md`'s `use std.option` form:delete i.e. E1002). Both forms
>    also reproduced on baseline before 3.4.6, **not introduced by 3.4.6**. **Rollback Obligation
>    (written dead)**: After this group is fixed must `weak.md` / `test.md` in modified to bypass
>    false positive examples **restore to natural writing** and **regenerate baseline** (baseline
>    will move once more).
> 5. **0.5 Test Suite Layering** (P0, issue #486):0.5.1 completed;0.5.2 (pre-commit / CI wiring +
>    2600 file measured calibration + `test-specification` detail section) pending scheduling.
> 6. **#489 ureq 3 migration**: Dependency surface tracking item, **not in
>    WBS**;`.cargo/audit.toml`'s `RUSTSEC-2026-0381` exemption **with hard removal condition**—after
>    ureq migrated to >= 3 delete that ignore, if still red then cannot extend deadline, must first
>    fix dependency. 0.26 line no fix version (0.26.12 doesn't exist), ureq 2.x no update version.
> 7. **3.20 Number Conflict Disposition (Newly Registered This Round)**:3.20 number originally
>    occupied by 2026-10-18 registered "pre-release audit (v0.8.4) supplement gap" (3.20.1–3.20.4),
>    while user 2026-10-19 ruled registered "dead code analysis existing false positive" needs to
>    use 3.20. Whole document number uniqueness rule doesn't allow two 3.20, so **move pre-release
>    audit group entirely to 3.21** (three-level change 3.21.1–3.21.4, issue anchor #468–#471
>    unchanged, entry content unchanged), 3.20 number given to "dead code analysis existing false
>    positive". Migration only changes number, doesn't change any registered entry's content and
>    attribution;historical commit info not rewritten (migration fact recorded in this record and
>    `09`'s count table annotation).
>
> **VII. Process Facts Must Be Truthfully Recorded**:
>
> 1. **Two inScope narrow writings each exchanged for one round of rework**:① 3.6 series implementer
>    wrote inScope narrow to `ir_gen.rs` predicate layer, didn't cover `types/`'s shared
>    substitution implementation → review round 1's F2 when landing discovered head renaming in two
>    files each inlined one copy, needed another round convergence;② 3.5's fix round when moving
>    changes between `types/` shared implementation and two call sites, again hit inScope and actual
>    change surface mismatch (AGENTS.md rule 6:inScope must cover mechanically required surface,
>    path prefix resolvable by matcher). Both reworks are "implementer inScope narrow" as cause,
>    **not implementation defect**—but cost is two rounds rework and one review findings.
> 2. **External Sync (2026-10-19 13:08 `pull --rebase --autostash origin dev`, 13:19
>    `pull --tags origin dev`) and V0.8.5 Release Commit**: Per user 2026-10-19 confirmed as
>    **normal release process, completed, doesn't affect this team**;member commits intact not lost.
>    Forensics (`git reflog --date=iso` / `git reflog show origin/dev --date=iso`):13:08:43's
>    `pull --rebase --autostash` picked up release chain's "supplement CHANGELOG retention
>    strategy's criterion and self-check", 13:19:34's `pull --tags` is fast forward;V0.8.5's release
>    commit (`:twisted_rightwards_arrows: V0.8.5: …` and its `:bookmark:` companion commit,
>    CHANGELOG generation and retention strategy two commits) all in `v0.8.5` tag reachable range.
> 3. **Three rebase caused local hash rewrite (reference old numbers will be empty)**: `b3245aac9` →
>    `56ae469e1` (3.6.3 main body), `415f0154b` → `ef2334de5` (3.6.2), `bdbe859e1` → `8e32e8636`
>    (3.6.4). Verifier proved three are **pure rebase equivalent** with " `src/`, `tests/` tree diff
>    empty" (content unchanged, only parent chain changed), this record uniformly uses **currently
>    reachable** hash.
> 4. **F2's Unfixable Part**: 3.5.1's commit info wrote "unique implementation" is historical, **not
>    rewritten**;correction can only be carried by "convergence to shared implementation's commit +
>    code comment per fact correct"—this record retains this sentence, to avoid readers mistakenly
>    thinking no one discovered the problem at the time.
> 5. **Three Observation Items (verified, not blocking, but registered in case to avoid being
>    treated as unhandled violations in the future)**: **O1** After head renaming converges to
>    shared implementation, coverage digs deeper into `MetaType` / `AssocType` / `Literal`, but
>    three coverage probes **all fail to parse** (`Pipe` / `Ampersand` / old `Type[...]`
>    syntax;`ast::Type::Union` has no construction point in parser) ⇒ **stronger in design,
>    currently unreachable**, not behavior change;**O2** `check-fanout.py` after fix round newly
>    reports `add_type_param: 2 files`, introduced by 3.6.3's closing commit (`inference/scope.rs`)
>    **not** this batch's fix round, review judged as **heuristic false positive** (delegate
>    wrapping + unique implementation division of labor, same type as existing `add_param`);**O3**
>    `09`'s 3.6 row original still writes "1 three-level task" while actually has 3.6.1–3.6.4, 3.5
>    three rows missing completion annotation—supplemented by this closure round (see `09`).
> 6. **`src/driver/arms.rs`'s "Decision B1" comment not rewritten same submission as code (stranded,
>    not changed this batch)**: 3.4.6 already changed `typecheck_orchestrated`'s FailFast branch to
>    **temporarily store** `result.warnings` (originally directly discard), but that function's
>    above doc comment still writes "`result.warnings` discard—Decision B1", opposite to post-3.4.6
>    code fact;same source's other place comment (`src/driver/tests/program.rs`'s stage table
>    description) **already same submission as 3.4.6 rewritten** to "Decision B1 'dead code family
>    warnings don't enter compile path' hereby revoked". This batch `src/**` out of scope, so **not
>    changed** that comment, registered for next team (belongs to AGENTS.md rule 6 "declaration
>    carrier directly named by ruling" class debt). `02`'s runtime behavior table B1 row already
>    in-place corrected to current state (dead code family warnings now via same criterion enter
>    MultiFile `run` path).
> 7. **Members Didn't Execute `git push` / `fetch` / `pull`**: Sync and push uniformly done by team
>    lead (in `git reflog show origin/dev` each is "update by push", member side no push record).
>
> **VIII. Uncovered Surface (verbatim, must not write "not tested" as "passed")**:
>
> - **t3's 7 items**:① wasm32 target actual compile not done;② performance impact not measured
>   (3.4.6 new two stages, 3.6.x's indirect call overhead all no baseline numbers);③ **W1005 item by
>   item attribution not independently reproduced** (verifier's message form regex didn't cover its
>   text form, only checked new W code total 118 consistent);④ 3.6.3 closing commit (declaration /
>   assignment not apply barrier) not separately mutated, c1 form not independently re-measured;⑤
>   t8's 29 probe matrix not full re-run (only re-measured three representative forms + M4 turn to
>   nail);⑥ 3.6.4 belongs to round 2 scope, round 1 not independently measured;⑦ CI gate per
>   convention not waited in session, unified to closure round check.
> - **t11's 4 items**:① qualified name callee with generic actual argument (`lib.g(Int)(3)`) and
>   named actual argument call function type value—no warehouse sample, this round not constructed
>   re-measurement;② under `use lib` overall import without item import `p.get_x()` (existing E6006
>   boundary, earlier than this task) not re-measured mutation state;③ at `8e32e8636` didn't repeat
>   full gate (covered by t9's gate log and t3 at `81d5e414e` containing that commit's all green);④
>   CI to closure round check.
> - **This Closure Round Newly Registered**: This round's documentation changes **didn't rerun
>   `cargo test --all`** (change surface is `docs/**` three files, zero code surface);closure
>   round's gate is `npm run docs:build` + `check-docs-truth.py` + batch tree recovery +
>   `git status` check.
>
> **IX. Batch Tree Recovery (GC and Status Raw Output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] already deleted verify-p3-t3 (released about 13220.5 MB)` /
> `[scratch] already deleted verify-p3-t4 (released about 14428.0 MB)` /
> `[scratch] gc completed, total released about 27648.5 MB`;`python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`;`git worktree list` → `E:/git/YaoXiang 20abd3ebc [dev]` (only main mount
> left). **Also deleted by GC are two batch tree's internal verification evidence** (`verify-p3-t3/`
> contains independent verification's probes, patches, raw logs and conclusion text;`verify-p3-t4/`
> contains review round 2's evidence)—they are **registered batch directories**, recovered with
> tree;this section's third, fourth, fifth sections have already recorded its conclusions, mutation
> table and gate numbers **as-is into record** for retention, conclusion not invalidated by file
> deletion. **Unaffected Retention** (not registered batch directory, GC doesn't touch): each
> batch's evidence directory `wbs35` / `wbs35repair2` / `wbs346` / `wbs36` / `wbs362` / `wbs363b` /
> `wbs364` / `wbs486` (in workspace `target/` under, **not in repo**;conclusion already item by item
> in this record, workspace path not as reference source). Acceptance expression (resolvable):
> `git status` clean;`python scripts/dev/scratch.py status` = no batch tree;`git worktree list` only
> main mount left;no this batch's newly added directory outside warehouse.
>
> **X. Documentation Real Build and Commit Hygiene**: This round's check / annotation / registration
> / documentation correction **accumulated into one** documentation commit (AGENTS.md rule 8),
> landed in batch closure;generator regenerated documentation merged into its functional commit.
> Before commit locally ran `npm run docs:build` (AGENTS.md rule 10 ⑨—markdownlint can't catch
> VitePress's Vue compilation error, and commits containing only docs will be judged
> `no files to check` skip markdownlint) and `python scripts/ci/check-docs-truth.py`. **Commit
> number taken after push**: This record per historical record convention records each round
> actually occurred commits and hashes (**note division of labor with `09` check row**—`09`'s check
> / completion registration only writes "task id + evidence (gate values, issue number, file:line)"
> not commit number, because `pull --rebase` rewrites local hash;this record is the round's process
> history, retains hash and gives rebase before-after comparison, see "VII" 3). **CI / Deploy Docs
> conclusion to "unified check" round**, not waited in session (AGENTS.md rule 13)—this batch's CI
> run id and conclusion backfilled by team lead at unified check.
>
> **XI. Count Table Account Review (This Round Caliber)**: `09` top's second-level / three-level
> count table continues "**base + increment**" caliber (same as 2026-10-18 registration). This round
> increment = second-level +2 (0.5 / 3.20, of which 3.20 is post-relocation new occupied number,
> pre-release audit group change 3.21 belongs to net zero), three-level +7 (3.6.2 / 3.6.3 / 3.6.4 /
> 0.5.1 / 0.5.2 / 3.20.1 / 3.20.2). **Remaining pre-existing deviation (not corrected this round,
> for subsequent account review use)**: Using "sum per second-level entry parentheses 'N three-level
> tasks'" as caliber recount, this tree gets 162, differs from table header by 7—difference comes
> from 2026-10-18 registration when newly added three-level but didn't synchronously rewrite parent
> entry count annotation's 5 places (4.9 wrote 7 actually 8, 3.14 wrote 1 actually 2, 4.10 wrote 3
> actually 5, 4.4 wrote 3 actually 4, S2 wrote 1 actually 2) and 1 place reverse deviation (7b wrote
> 6 actually 5), additionally 1.2 / 1.3 / 9.1 / S7 four parent entries originally don't carry this
> annotation (not counted in caliber). This difference **not introduced by this batch**, this batch
> doesn't change others' entries, only registered here.
>
> **XII. Stranded TODO (This Batch Registered, Not Fixed)**: ① `src/driver/arms.rs`'s "Decision B1"
> comment opposite to post-3.4.6 code fact (see "VII" 6, `src/**` out of scope);② 3.20.1 / 3.20.2
> two existing false positives (after fix must restore `weak.md` / `test.md`'s natural writing and
> regenerate baseline);③ 0.5.2 (pre-commit / CI wiring + 2600 file measured calibration +
> `test-specification` detail section);④ #489 ureq 3 migration (dependency surface, exemption with
> hard removal condition);⑤ 3.8 (native shadows user function name) and 3.12 (runtime mode-to-mode
> executor configuration propagation) pending ruling, 3.18 prerequisite waits for P6—**all not in
> this team's scope**.

## P3 3.17 Closure (Round 2: 3.17.1 Manual FFI Named Actual Argument + Second Binding Criterion Merge)

> **I. Scope and Round Relationship**: This section is **round 2**'s closure registration, scope =
> **3.17.1** (issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)), not including other
> second-level tasks. Round 1 (3.5 / 3.6 / 3.4.6)'s closure see previous section "P3 Debt Round 1
> Closure";two rounds' relationship stated here:**round 1's review round 1 (t4) that needs_revision
> (F1 / F2 / F3) already covered by t12 fix round + t13 review round 2 (verdict = pass), not
> repeated in this section**. This section's own criterion chain = implementation **t15** (attempt 2
> delivery;**previous attempt due to provider timeout, zero product**) → independent verification
> **t16** (verdict = pass) → adversarial review **t17** (verdict = pass, **no unhandled findings**,
> registered 4 observation items).
>
> **II. Per-Item Output and Measurement Numbers (3.17.1)**:
>
> - **Parameter name table same source**:`add_native_signature` changed to take **signature string**
>   and return `Result`—`signature::parse_signature` outputs type, unique exit
>   `signature::export_signature_param_info` same-fills "parameter name + required minimum";key same
>   as signature table (same name). Warehouse zero caller so signature can be changed;documentation
>   explicitly writes "callers who can only get `MonoType` should resolve signature string on their
>   own side then take this API", and this function **must not** resolve parameter names by itself.
> - **Red Before Green** (red state measured at parent commit `537b46b74`):manual surface four
>   forms + duplicate naming + legal call total 6, **5 failed, diagnostics all `[]`** (named actual
>   argument silently discarded);green state same file **12 all passed**—four forms E1014 / E1010 /
>   E1111 / E1111, duplicate naming E1015, legal named call and omitting optional position pass.
> - **Three call surface same criterion same code**:manual FFI (typecheck phase) / ordinary user
>   function (IR phase) / stdlib native export (`assert.assert`) three surfaces **code by code equal
>   assertion** (E1014 / E1010 / E1111;E1015 see manual surface and user surface). Different
>   observation phase only reflects "who discovers first";host native doesn't enter registry, IR
>   phase originally unresolvable (**earlier than this task's embedded surface limitation**).
> - **Criterion single source**:`bind_args_to_params` **1 definition**
>   (`inference/arg_binding.rs:35`) + **6 calls** (typecheck 2 + ir_gen 4);second criterion
>   `build_arg_slots` **0 place** (deleted, `ir_gen.rs:516` only has comment explaining its
>   history);`ir_gen` only has pure transport `slot_order_arg_indices` (`ir_gen.rs:521`, 1
>   definition 2 calls, zero judgment). Line count:`ir_gen` **+58 / −92 (net −34)**;whole warehouse
>   4 files **+448 / −102** (of which 353 lines are new tests).
> - **No regression**:8 CLI probes (u1–u4 / d1 / d2 / n1 / n3) **code by code consistent** before
>   and after mutation;`ir_gen` blob `0d837e9d…` ↔ `ffe1af8e…` checked. Merge initial version one
>   place self-injury fixed:IR side re-judges "missing parameter" mistakenly reported
>   `t.pick2(a = 2)` legal overload as E1010 (named_args single test caught) ⇒ missing parameter
>   judgment goes to typecheck, IR only replays slot binding;after fix 19 named_args + 12 new single
>   tests all green. Corpus 348-348 zero drift, IR snapshot zero drift.
>
> **III. Independent Verification (t16, frozen `c3082a160`;red state baseline `537b46b74`, both
> verified as HEAD ancestors via `git merge-base --is-ancestor`;probes all self-built)**:
>
> - **Red→Green Independent Reproduction** (manual FFI surface):Self-built independent Rust single
>   test `verify_host_ffi.rs` (4:unknown name E1014 / contest slot E1111 / named then position E1111
>   / legal named call pass), via `scratch.py trial` temporarily mounted in `tests/mod.rs` (apply →
>   test → apply -R → CLEAN). Two states each use **respective interface form**'s probe variant
>   (parent commit `add_native_signature(name, MonoType)` old signature / fix state
>   `(name, &str) -> Result` new signature—interface itself is fix surface):parent commit **3 failed
>   / 1 passed**, 3 failed diagnostics **all `[]`** (`got []`), TEST_EXIT=101;fix state **4 passed /
>   0 failed**, TEST_EXIT=0. Red state additionally in fix tree's revert fix mutation state
>   reproduced (`M7v2` merge patch:revert fix + remove implementer test + mount self-built old API
>   probe)—3 failed `got []`, TEST_EXIT=101, after revert four files blob consistent with HEAD,
>   `git status` CLEAN, after revert `ir_gen.rs` blob = `0d837e9d30de` (i.e., parent commit original
>   blob). CLI surface (user function + stdlib) two states code by code consistent—expected:these
>   two surfaces 3.16.1 already loud, this commit's fix surface only in manual FFI.
> - **Four Forms Item by Item** (CLI self-built probe, `c3082a160` measured):parameter name error
>   E1014 `has no parameter named x (available: a, b)`, missing required position E1010, position
>   and naming contest slot E1111, named then position E1111;additional:duplicate naming E1015,
>   stdlib unknown name E1014, stdlib missing required position E1010 (help names cond), stdlib
>   contest slot E1111, legal named call EXIT=0 output 3/3, omitting optional position EXIT=0.
> - **Criterion really deleted not renamed**:`git show c3082a160:src/middle/core/ir_gen.rs` interior
>   `build_arg_slots` **0 hit** (parent commit same file 3 hit, definition at
>   :521);`slot_order_arg_indices` (fix state :521) line by line check =
>   `sort_by_key(|&arg_idx| positions[arg_idx])` pure sort, no position/name/optional position
>   judgment residue, doc comment explicit "no position judgment, no name judgment, no optional
>   position judgment";deleted 71 lines judgment block (parent commit :521-593, six rules:empty
>   table loud / position occupies slot / order illegal E1111 / unknown name E1014 / contest slot
>   E1111 / into slot) no longer exists;`bind_args_to_params` 1 definition 6 calls (typecheck
>   2:`expressions.rs:2854` / `:5653`;ir_gen 4::7828 / :7926 / :8021 / :8351), consistent with
>   implementer report;parameter name unique exit = `export_signature_param_info` (environment.rs
>   manual / typecheck::mod stdlib / ir_gen lookup), no second place self-parse.
> - **Self-Injury Regression Nailed** (`t.pick2(a = 2)`):`git show c3082a160 --name-only` **doesn't
>   contain `named_args.rs`** (4 files:environment.rs / tests/mod.rs / native_param_names.rs /
>   ir_gen.rs) ⇒ nail test not changed;independently rerun that case 1 passed, named_args full set
>   **19 passed / 0 failed**.
> - **No Regression Gate** (`c3082a160` tree):`cargo test --all --quiet` (4×4 caliber) EXIT=0—lib
>   **2806** + 2 ignored, integration **297**, 52/44/9/2/1/9 all green 0 failed;`cargo fmt --check`
>   / `clippy -D warnings` both EXIT=0;corpus diff (dual parameter) 348/348 difference
>   empty;`check-snapshot-drift.sh` PASS (204 snapshots zero drift);`check-docs-truth.py`
>   EXIT=0;3.16.1 four forms comparison (stdlib surface) E1014 / E1010 / E1111 / pass.
> - **Uncovered Surface (t16 five items, verbatim)**:① `testsuite.py run full` (CI caliber resource
>   profile channel) not rerun—implementer has fallen back per ruling (exclusive wave peak 8.11
>   GiB > local budget 7.51 GiB, exit 2 is tool by design reject);this round uses 4×4 and 2×1 two
>   calibers' `cargo test --all` to cover.② Manual FFI surface (host-FFI) can only be driven via
>   Rust test (`add_native_signature` warehouse zero caller, no CLI surface)—"host embed process
>   interior real FFI call" not tested.③ Variadic parameter (`...`) named actual argument form not
>   constructed sample (no warehouse criterion sample).④ `check-boundary.py` / `check-fanout.py` not
>   rerun (implementer has passed;no cross with behavior criterion).⑤ CI (GitHub Actions) per
>   convention not waited, to closure round check.
>
> **IV. Adversarial Review (t17, reviewed revision `c3082a16051cb1bfb8b84bdc62ec6bd29d90ff2d2`,
> parent = `537b46b7488a60b42cdc35110e0a413c80a58169`;all content taken with
> `git show <rev>:<file>`, line numbers re-taken on referenced revision)**:
>
> - **① Criterion Single Source Reverse Check**:`slot_order_arg_indices` (ir_gen.rs:521-527)
>   **doesn't reorder by name** (sort key is shared criterion's given slot number, name only appears
>   in `arg_binding::bind_args_to_params`), **missing parameter not fallback** (absent slot doesn't
>   produce actual argument index), **doesn't distinguish optional position** (IR two places pass
>   all optional, another two places pass true value `i >= required_min`). Whole warehouse "find
>   position by parameter name / find member" grep checked one by one:**slot calculation only
>   `arg_binding.rs:76` one place**, other 7 hits each belong to different concern points (generic
>   type parameter's constraint interface check / const generic actual argument substitution /
>   generic type variable binding / generic type parameter substitution / const generic parameter
>   lookup / method definition lookup / struct field lookup).
> - **② Responsibility Boundary (Missing parameter only loud in typecheck, IR only replays slot
>   binding)**:typecheck side `bind_named_args` (`expressions.rs:2854`) passes all required ⇒
>   missing any value parameter E1010;native export arm (`:5653`) passes true optional position
>   `i >= required_min` ⇒ only E1010 for true required position, `?msg` omission pass. IR side this
>   commit's changed two places (`ir_gen.rs:7926` RFC-004 binding path / `:8021` overload resolution
>   method path) **uniformly pass all optional, only replay slot binding** (comment explicitly
>   writes "missing parameter goes to typecheck" and "per declaration table re-judge missing
>   parameter will mistakenly report E1010 for legal resolution (`t.pick2(a = 2)` measured)").
> - **③ API Shape**:environment.rs:990-993 explicitly writes "this function **must not** resolve
>   parameter names by itself" and points to unique exit `export_signature_param_info`;:997-999
>   writes "callers who can only get `MonoType` should resolve signature string on their own side
>   then take this API".
> - **④ Behavior Change Surface**:Code four differences (same parameter named twice E1111→E1015 code
>   change / unknown name E1014 text change / empty table E1014 text change / position actual
>   argument exceeds IR side declaration table:silently discard→send as-is), **measured reachable
>   form has no diagnostic code / text change** (typecheck first judges, same code), IR product on 5
>   self-built probes **byte-by-byte consistent** (p1 / p5 take this commit's changed branch and
>   product unchanged, p2 / p3 / p4 two states same code, p9 can't be constructed, p6–p8 source
>   layer unreachable);t15's 8 CLI probes two states code by code consistent. Differences all land
>   on **originally failed** programs (programs still fail) ⇒ D59 "doesn't change previously
>   successful program's external performance" not touched.
> - **⑤ Test Quality** (not just look at red green):12 new tests
>   (`typecheck/tests/native_param_names.rs`) item by item to `test-specification`—naming
>   `test_<what>_<scenario>`, three-section AAA comment, assertions all carry custom message, setup
>   repeats ≥3 times abstracted helper, helper interior panic carries context;**no "only check exit
>   code" type weak assertion** (first `assert!(`diagnostic non-empty`)` then `assert_eq!(`code by
>   code equal`)`, both sides empty won't pass through);3 negative cases (legal named call pass /
>   optional position omit pass / explicit give optional position pass) under M1 red state still
>   pass ⇒ they **only prevent false positive**, exactly their purpose. **M1** (remove
>   `native_param_names` filling) → `cargo test --lib native_param_names` **9 failed / 3 passed**,
>   TEST_EXIT=101 ⇒ new test **really nails fix**. **M2** (entire revert `ir_gen` merge) →
>   `cargo test --lib` **2806 passed / 0 failed / 2 ignored**, exit 0 ⇒ merge surface **no test
>   coverage**. `pick2` nail (existing `named_args.rs:253-283`, this commit not changed) takes
>   **real full link** and asserts **runtime value** `Int(20)` ⇒ **really nails overload
>   resolution** (not "no error"), and it's a live gatekeeper for "IR must not re-judge missing
>   parameter".
> - **⑥ Limitation Registration**:Host native only enters `TypeEnvironment`'s two tables, **doesn't
>   enter `ModuleRegistry`** ⇒ IR generation per registry resolves namespace member, so host native
>   originally unresolvable in IR phase (E3006), **earlier than this task's embedded surface
>   limitation**;this limitation and "why manual surface must be observed in typecheck phase"
>   written in **test file doc comment** and `environment.rs` field doc (not only in report).
> - **Gate eight items all rerun all green on reviewed revision**:fmt --check / clippy -D warnings /
>   `cargo test --all` (**3220 passed, 0 failed**;lib 2806 + 2 ignored, integration 297) / corpus
>   diff 348-348 zero drift / IR snapshot 204 zero drift / docs-truth / check-boundary (0 exemption
>   0 violation) / check-fanout (new symbol no same name multiple definition entry).
> - **Four Observation Items (verbatim registered:verified, judgment doesn't constitute finding, no
>   need to dispose, but registered in case to avoid being treated as unhandled violations in the
>   future)**:
>   - **O1**: Commit info says "after this merge **whole warehouse only has shared criterion one
>     place**"—for **slot binding** it's true;but typecheck's **method overload arm** still has one
>     "by name lookup candidate table (E1014) + arity (E1010)"'s own judgment
>     (`inference/expressions.rs:5578-5600`), it uses **candidate group representative**'s parameter
>     table (`method_overload_param_names`), different from shared criterion's **already resolved
>     candidate** table. Judgment basis:that place **existed earlier than this commit, this commit
>     not touched** (`git show c3082a160 --name-only` only 4 files, no expressions.rs), and belongs
>     to "candidate matching" concern point (different table, different semantic), not slot binding.
>     **Wording too broad, registered as observation**.
>   - **O2**: `add_native_signature` changed to take signature string, **no**
>     `MonoType → signature string` shared helper ⇒ future callers who can only get MonoType can
>     only each write one serialization. Judgment basis:parameter name / required minimum already
>     doc-locked at unique exit `signature::export_signature_param_info`;currently **zero
>     caller**;doc already writes "callers who can only get MonoType should resolve signature string
>     on their own side then take this API" ⇒ residual risk only in "how to construct string" one
>     paragraph, **low**.
>   - **O3**: This commit's **merge part** (ir_gen deduplication) **no any test coverage**
>     (M2:entire revert ir_gen then `cargo test --lib` still 2806 passed all green). Judgment
>     basis:"deduplication" no new behavior to nail, by nature;**single source** nature guaranteed
>     by grep + `check-fanout.py` gate. **Optional strengthening**:in `native_param_names.rs` add
>     one **IR layer** case (covers ir_gen:7926 / :8021 path), let merge surface also have
>     guardrail.
>   - **O4 (Attribution Correction)**: Commit info and report attribute d2's E1010 to "**method
>     arm** arity pre-check first triggered"—measured that E1010's help is generic text "check the
>     argument count of the function call", **not** shared criterion "missing parameter"'s help
>     ("missing required parameter:…") ⇒ trigger is **native (native) arm**'s arity pre-check
>     (`native_arity` table). **Code behavior conclusion unchanged** (parent commit existing, this
>     time not changed, verified), only **attribution wording needs correction**;and "**arity
>     mismatch + duplicate naming**" must be qualified for "**`list.push` duplicate naming i.e.
>     E1010**"—only duplicate naming with matching arity (`list.push(list = v, list = 9)`) two
>     states are **E1015** (duplicate naming earlier than arity judgment).
>
> **V. Gate and Resource Channel (one external link)**: This task **first really used**
> `python scripts/dev/testsuite.py run full` → **exit 2**—exclusive wave peak 8.11 GiB > local
> budget 7.51 GiB (gap 618 MiB), tool by design **reject silent skip** ⇒ fall back
> `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test --all --quiet` → **exit 0** (3220 passed / 0
> failed). This channel status chain **#486**:`full` on local machine because peak extrapolation is
> too conservative unavailable, **calibration (2600 file by measured calibration) belongs to WBS
> 0.5.2**.
>
> **VI. Count Table: This Round Not Touched**—`3.17.1` already 2026-10-15 registered and counted in
> table header ("2026-10-15:+2 = 3.17.1 / 3.18.1"), this round only check + annotation,
> **three-level count unchanged** ⇒ `09` top's second-level 67 / three-level 176 **not touch**. This
> round **doesn't add** count deviation (parent entry 3.17's "1 three-level task" annotation
> originally correct);previous section "XI" recorded 6 pre-existing deviations (parent entry count
> annotation lagging) **unrelated to this round, not introduced by this round**, still per that
> section's caliber left for account review.
>
> **VII. Batch Tree Recovery (GC and Status Raw Output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] already deleted verify-p3-t16 (released about 10235.4 MB)` /
> `[scratch] already deleted verify-p3-t17 (released about 10079.2 MB)` /
> `[scratch] gc completed, total released about 20314.6 MB`;`python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`;`git worktree list` → `E:/git/YaoXiang c3082a160 [dev]` (only main mount
> left). **Also deleted by GC are two batch tree's internal verification evidence**
> (`verify-p3-t16/`'s `EVIDENCE.md`, `verify-p3-t17/`'s `EVIDENCE-t17.md` and probes / patches / raw
> logs)—they are **registered batch directories**, recovered with tree;this section's third, fourth
> sections have already recorded its measurement numbers, gate values, mutation table and four
> observation items **as-is into record** for retention, conclusion not invalidated by file
> deletion. **Unaffected Retention** (not registered batch directory, GC doesn't touch):`wbs317`
> (implementer evidence:evidence-3.17.1.md / red-test.txt / trial-before-after.txt + fix-3171.patch
> / attribution table etc.;in workspace `target/` under, **not in repo**;conclusion already in this
> section, workspace path not as reference source). Acceptance expression (resolvable):`git status`
> clean;`python scripts/dev/scratch.py status` = no batch tree;`git worktree list` only main mount
> left;no this batch's newly added directory outside warehouse.
>
> **VIII. Documentation Real Build and Commit Hygiene**: This round's check / annotation /
> registration **accumulated into one** documentation commit (AGENTS.md rule 8);before commit
> locally ran `npm run docs:build` (AGENTS.md rule 10 ⑨) and
> `python scripts/ci/check-docs-truth.py`, markdownlint zero warning. **Not push** (members don't
> push, see AGENTS.md rule 10 ⑧);**CI / Deploy Docs conclusion to unified check round** backfill,
> not waited in session.
>
> **IX. Stranded Items and Registration**:
>
> 1. **Round 1 Closure Registration's Stranded Item ① Closed**:`src/driver/arms.rs`'s "Decision B1"
>    comment opposite to post-3.4.6 code fact one item, by `537b46b74`
>    (`:memo: docs(driver): correct opposite-to-fact B1 comment (3.4.6 closing missed)`) in-place
>    rewritten to current state fact (behavior neutral, only comment line;wording aligned with
>    `src/driver/tests/program.rs`'s same fact's expression).
> 2. **This Round Newly Registered**:O1–O4 four observation items (verified, not blocking,
>    registered in case);0.5.2 (`full` channel's 2600 file measured calibration + pre-commit / CI
>    wiring);#489 ureq 3 migration and `.cargo/audit.toml` exemption's hard removal condition;3.8 /
>    3.12 pending ruling, 3.18 prerequisite waits for P6—**all not in this team's scope**.
