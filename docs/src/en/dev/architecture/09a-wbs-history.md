# WBS Historical Notes (Implementation Records · Audits · Ruling Traces)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task table): implementation records, audit findings, ruling traces, and errata for
> each phase. **Division of labor**: 09 only describes the **plan** (Kanban, three-level task table,
> dependencies, risks, acceptance); this document only records the **history** (what was done, what
> was found, how it was ruled), appended chronologically without rewriting history.

## P2 Establishing the Equivalence Criterion Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus run
> green. **D38 prophecy fulfilled**—the green run process actually tested and fixed three real
> defects in ir_gen (see 06 §G6 for details): the method locals table was truncated to the parameter
> count (local_count underestimated, runtime relies on Frame::set_slot fallback), unannotated
> parameters were dropped by filter_map (three places, params metadata misaligned with frame
> layout), match result slots depended on implicit Void initialization of the frame (now explicitly
> pre-initialized). The real-world semantics of jump targets is flattened instruction index (not the
> label-keyed form in the 07 text), CFG is built at the instruction level—registered in the
> verify.rs file header. **Commit comparison**: ir_gen fix = `a9ea199b`; verify main body =
> `d5b18390` (⚠ parallel flow COMMIT_EDITMSG collision caused its message to be mislabeled as
> "docs(rfc): RFC-027a…", the content is actually verify.rs + six test files, buried in history
> where it cannot be amended, registered here).
>
> **P2 Narrowing Part Implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> stop-bleeding channel pre-unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract uses the same `TestFileSpec` as the
>   single-file corpus (single point on the library side), binary positioning/subprocess launching
>   abstracted into `tests/common/` with yx_runner migrated in sync (Prohibition Three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed by the rfc027 test (return position/call point/binding position)—the
>   `Sorted(3)` annotation example from 02 would be substituted for evaluation in the current code
>   and directly Disproved, not presenting Unproven silently; the Sorted(3) in 07's pseudocode is
>   for illustration only. Red-state measured data is in the test file header.
> - The two red criteria follow the 2.4.5 red skeleton precedent and are persistently marked with
>   `#[ignore]`: the default suite is all green, `-- --ignored` reproduces the red state; **remove
>   the attribute when P3 fix lands and they turn green**.
> - The reasons for the comparison normalization's deliberate deviation from the 07 (code, file,
>   line) table (removing file/line, only comparing error code sets) are written in the 2.4.1 test
>   comments.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **moved out of P2**: the `Program` / `Obligations` types they reference don't exist until P4,
> and P2 cannot compile. The former becomes P4's 4.1.4 (no double-registration), and the latter's
> `#[ignore]` red skeleton is merged into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: Item-by-item check of the C4 mandatory coverage list (07
> §143), all 10 semantic corpus layers fully covered and within the corpus-parity.jsonl diff
> baseline (26 representative files 26/26 hit, grep-verified)—ref/borrow/move 05-ownership full
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum type
> sum_type__+ pattern_or_guard + match_call_scrutinee, existential mandatory point
> interface_dynamic_dispatch (Vec(Animal) existential heterogeneous packaging triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (+
> method_overload_ambiguous_err in diagnostic diff surface), refinement constraint Drop sequence
> refined__ ×5 + 05-ownership-driven ReleasePlan (span key contract has another 2.4.4 red criterion
> persistently). Performance baseline `benches/pipeline.rs` added to the repository (07 §145 three
> smoke tests, two of them): `corpus_compile_frontend` first run full corpus compile 17.6s,
> `cli_cold_start_hello` process cold start 169.7ms; interpreter throughput is already covered by
> the hotpath group in benches/lib.rs (interp_fib_recursive_27 / interp_loop_steady_10m), no
> duplicate construction. Regression gate check-perf-regression.sh is unified under the P9 list
> (2.5).

## P3 Fixing Correctness Vulnerabilities (Minimum Plan)

> **Error type annotation for already-implemented rows** (retrospective annotation, does not modify
> history): 3.1 = B ｜ 3.2 = B (defensive empty arm, criterion turning green borne by 3.1) ｜ 3.3 =
> A ｜ 3.4.1 / 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 /
> 3.4.7 = D (user-visible surface, already user-ruled) ｜ 3.4.8 ①② = C (main) + A (three
> incidental) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-ruled).
>
> **P3 Diagnosis (2026-10-12, user request)**: A diagnosis of the perception that "P3 items keep
> growing".
>
> **I. First, clarifying the scale (the perception is amplified by two accounting defects)**
>
> 1. **Duplicate numbering**: `3.5.x` is shared by two task families—(a) **029g `pub` visibility
>    convergence** from 2026-10-08 (7 three-level items: lexical and parser branches / AST `is_pub`
>    / checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) **3.5 higher-order type parameter call path
>    completion** registered on 2026-10-10 (3 three-level items). Both use `3.5.x` ⇒ cross-reference
>    ambiguity (**already renumbered per user ruling on 2026-10-12: the 029g family is changed to
>    `029g.x`**), and 029g's rows were mistakenly counted in P3. **Pending ruling**: renumber the
>    029g family to `029g.x` (or move it into its own phase), and synchronize other references in
>    the document.
> 2. **Missed checks**: 3.7's 4 three-level items (3.7.1–3.7.4) are all completed (d922a551 /
>    38bd4b9e) but unchecked ⇒ this item is corrected this time.
>
> **II. P3 Actual Accounts (12 second-level rows)**
>
> | Row                                                                  | Status                                                                                                                                                                                | Disposition Suggestion                                                                                                                      |
> | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                      | Completed                                                                                                                                                                             | —                                                                                                                                           |
> | 3.4 Silent Drop Point Addendum (8 three-level)                       | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed on 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unblock with 4.3.1                                                                                   |
> | 3.5 Higher-order Type Parameter Call Path Completion (3 three-level) | Not started (declaration side live, call side broken)                                                                                                                                 | Keep in P3; **does not block the refactor main line**, but is an "accepted feature not implemented", with higher priority than pure cleanup |
> | 3.6 Function Type Value Call Path (1 three-level)                    | Not started (semantics already cut (i))                                                                                                                                               | **Keep**: correctness defect, clear criterion                                                                                               |
> | 3.7 init Section Call Name Qualification (4 three-level)             | **All completed**                                                                                                                                                                     | Check and close this time                                                                                                                   |
> | 3.8 native Shadowing (1 three-level)                                 | Not started                                                                                                                                                                           | **Stuck on decision** (first determine name resolution priority), not a scheduling issue                                                    |
> | 3.9 / 3.10 / 3.11                                                    | Completed (2026-10-12)                                                                                                                                                                | —                                                                                                                                           |
> | 3.12 Runtime Mode Executor Config Propagation (1 three-level)        | Not started (Class D, user ruling B: separate team)                                                                                                                                   | **Suggest moving out of P3 → P8 (execution layer)**, pending ruling                                                                         |
>
> **IV. Leverage of 4.3.1 (main line marker)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the prerequisite for **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract
> PlanId-ification) ⇒ take it once to advance the main line and clear two P3 stranded items. 3.5
> (HKT), 3.6 (function type value call) and 4.6.1/4.9.1 all have **no unmet prerequisites**, can be
> done anytime; 3.8 is stuck on semantic decision (user-defined priority vs explicit diagnostic).
>
> **III. Mechanism Suggestion (preventing recurrence)**: Newly registered second/third-level rows
> **must carry two fields**—"belonging phase" (which P it truly belongs to) and "whether it blocks
> the main line"; `registered pending scheduling` will no longer be the default state, and
> unannotated items will be considered unschedulable. This way, bloat is visible and sortable,
> rather than undifferentiated accumulation. **Pending ruling**: whether to write this item into the
> registration convention of this document and `src/dev/coding-rules.md`.
>
> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter (then
> recorded as `pub(crate)`; **verified on 2026-10-07 that the actual landing was `pub(super)`**—the
> code comment references ruling #434, which is not registered on the docs side, see 3.4.3; fully
> private would require rewriting 11 constructor literals, not taken during the stop-bleeding phase;
> collect when P4 Obligations migrates); execution mechanism extracted to
> `src/frontend/proof_execution.rs` (Prohibition Three single point—cannot be put in typecheck/,
> which would create L3→L4 reverse dependency); 3.1.2 according to 02 evidence chain step 5
> supplements **four entries** (WBS row number missing compile_project `:99`, must be supplemented
> and registered as required by 3.1.1 acceptance "criterion turning green"); 3.2.1 empty arm changed
> to accounting (proof_calls throws up) + diagnostic (`into_result` established conversion path);
> 3.1.3 SOLVER slot Option-ized, Z3 missing conservatively downgrades per `SMTResult::Unknown`
> (E2031 family diagnostic replaces process crash). Two vulnerability criteria remove `#[ignore]`
> and turn normal green, corpus fixture `proof-obligation-honored` added; check-fanout stock
> migrated accordingly (wiring point 19 = stock). Measured: criteria 2.4.1/2.4.3 turn green, full
> suite lib 2494 / integration 272 / corpus dual-layer all green.
>
> **3.4 Audit (2026-10-07, post-P3 re-review)**: Aimed at defects of the "failure produces no
> signal" type (definition in 02 §41-52), on unchanged code re-inventory, located 8 more, all
> registered as the above table's 3.4.x. Methods and evidence:
>
> - **Consumption surface inventory** (16 obligation fields one by one grep full warehouse read
>   points): `variant_ctor_calls` **zero consumers**—the only read `checker.rs:1468` is "self-pour
>   into `TypeCheckResult`", IR side goes through shape detection `ir_gen.rs:7374`;
>   `rfc011b.rs:659/688` only asserts the production end, exactly the "only test production end,
>   naturally immune to consumption end defects" form criticized in 02 §186 → 3.4.4.
>   `method_overload_ir_names` result-level field likewise has zero external consumers, IR uses
>   parallel `overload_resolutions` → 3.4.5.
> - **Phase coverage re-review**: 02 §91 table row 1 (four entries "no proof_execution") has been
>   fixed by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 state unchanged (multi-file `run` still does
>   not output W1003, does not run dead code family) → 3.4.6; LSP non-project-internal path (02 §77
>   "manual lex→parse") likewise does not run proof → 3.4.7.
> - **Same-form empty arm scan** (`Unproven` full warehouse consumers only four): checker three
>   refinement branches (covered by 3.1), 3.2.1 fixed ownership arm (**source unreachable**), and
>   `environment.rs:422` (**reachable, zero diagnostic, zero accounting, and full warehouse docs
>   zero registration**) → 3.4.1, the only "unknown to all" item in this audit.
> - **Solver acquisition surface**: `default_solver()` three places with different consumption forms
>   (`predicate.rs:32` process singleton / `checker.rs:1266` injected per `check_module` /
>   `ownership.rs:621` newly created per back-edge determination)—3.1.3 only eliminated one panic;
>   the other two's **silent surface** go to 3.4.2, **singleton-ization and cache hit** go to P4's
>   4.4.2 (no double-registration).
> - **Not counted in 3.4's existing ownership**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys not consumed, Drop silently lost) still in P2 red state persistently, ownership
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration vs actual opposite, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger downgrade) already on P4 register.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites, can be landed in place
> anytime (recommend doing 3.4.1 first—the only "full warehouse zero registration" live
> vulnerability, smallest change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on
> the obligation ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish a new criterion
> system**, uniformly reusing existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 Implementation Record (registered 2026-10-13 → closed 2026-10-15)**: User ruled
> on 2026-10-14 to merge four relevance minor items (#451 method binding panic / #452 named actual
> parameter in resolution / #453 dead inference entry / #454 native export named actual
> parameter—the last item is a boundary finding when verifying 3.14, on the same day attribution
> experiment verified **not introduced by #452**, user ruling ① merged into this batch) into one
> team for processing. Closure status: all four completed, ruling registered as RFC-039 **D60** (six
> binding rules + shared criterion single-point).
>
> **I. Two Contract Corrections (Lesson: the contract itself can also be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: #452 task contract simultaneously required "mixed
>    actual parameters output 20" and "position and name competing for the same position report
>    error", but the nailed board fixture `Temp.pick(self, a)` only has one value parameter—the same
>    actual parameter combination is both "mixed" and "conflicting". The first implementer
>    truthfully reported the blocker and set the task to failed, overall revert (not taking silence
>    in exchange for green light). Disposition: task re-dispatch + contract revision—given the **six
>    binding rules** and **8-row value-level matrix** (fixture changed to two value parameters
>    `pick(self, n, a)`), attempt 2 was completed by the second implementer. The full text of the
>    six rules has been verbatim included in D60.
> 2. **inScope missed mechanically required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only `code-tables --fix` was
>    run, **`scripts/docs/gen-error-code-docs.py` was not rerun** ⇒ CI Docs Truth red ("source
>    registered/emitted but doc missing 1: E1110", source 154 / doc 153, run **38518624964**—this
>    batch's only CI red line). Fix = mechanical regeneration (8 files +83/−70, only touches
>    `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **II. Re-review → Fix → Re-review Two Closed Loops**
>
> 1. **#452 line**: round-1 re-review needs_revision (F1–F5; F1 = arity guard
>    `named_args.is_empty()` not narrowed with resolution ⇒ calls containing named actual parameters
>    in multi-candidate zero-hit/single-candidate-hit two paths whole chain silent) → fix round
>    added external code **E1111 (argument_order_invalid)** (code table + locales + RFC-013 +
>    generator four-piece set same submission; code count 154→155) → round-2 re-review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm settling by name) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations
>    mechanically reversible replay via `scratch.py trial` all caught, restoration byte-by-byte
>    consistent) → round-1 re-review needs_revision (F1 = typecheck native arm **inline copied** a
>    binding criterion, missing rule ④ ⇒ **ruling divergence** between method and native same-form
>    calls—method reports E1111, native silent exit=0; ir_gen has another third existing copy
>    reporting old code E1015) → convergence round consolidated the criterion into a single
>    implementation `bind_args_to_params` (`arg_binding.rs:35`, four call points
>    `expressions.rs:2727` / `:5416`, `ir_gen.rs:7776` / `:8279`) → round-2 re-review caught a
>    **regression** introduced by the convergence round (IR convergence block lost native omittable
>    position ⇒ previously successful program with optional position omitted was falsely reported as
>    E1010) → fix round (optional position criterion and parameter name from the same source
>    `signature.rs:236` single exit) → round-3 re-review PASS (independent reproduction red→green;
>    mutation under nail board FAILED empirical criterion can fail). **Lesson**: re-review must
>    cover "unmodified branches of similar paths" (both needs_revision in this batch were because
>    probes all landed on the hit surface); the fix round itself can also introduce regression
>    (optional position), caught by the next round of change-of-person re-review.
>
> **III. Process Events (faithfully recorded, not pointing at individuals)**
>
> 1. **External whole-tree discard (around 2026-10-14 17:14)**: The work tree was silently restored
>    externally—the implementer's fix in-progress (typecheck/ir_gen/code table/locales/RFC-013 etc.)
>    all disappeared, **at the same moment the two review documents edited by the user themselves
>    also disappeared from git status** ⇒ determined to be a repository-level discard (most likely
>    user-side manual cleanup/IDE "discard all changes"). Three pieces of evidence exclude group
>    members: empty stash, no new commits, no reset in reflog. The implementer replayed the
>    in-progress work per context and pushed; the discipline was changed to "**first save the patch
>    to scratch, then capture red state**".
> 2. **Verifier ran out of room and changed person**: The independent verifier verifying 3.16 ran
>    out of room mid-way (no closing message), changed person per load rules—the successor continued
>    using their already-ready products (green state result, corpus equivalence script, mutation
>    plan), verification finally PASS. A positive precedent for partial product handoff.
> 3. **Re-review 5th round rotation and historical round retreat**: When rotating the re-review per
>    the load limit's ⑤, `remove_member` retreated the **already adjudicated final-state
>    re-review/verification rounds** back into the shared pool, the scheduler immediately re-ran
>    (t3/t8 were taken by new members, duplicating work and blocking downstream). Disposition: team
>    leader took over and cancelled the retreated historical rounds (t3/t8/t13), added AGENTS.md
>    12th item per this (immediately handle retreated rounds after person change). The batch closure
>    record registers this event per that item.
> 4. **scratch governance (about 6.3 GB)**: Each task had an external tree in the early batch, 6
>    directories scattered outside the repository (about 780 MB, cleaned) + two members' external
>    verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Governance tool
>    `scripts/dev/scratch.py` produced (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md
>    9th item (batch tree single entry), 10th item (verification environment six rules: mechanically
>    reversible mutation / line number takes the referenced revision / CI unique gate must run
>    etc.), 11th item (writer load limit), 12th item (handle retreated rounds after person change)
>    and top meta-rules two (no private content / verify before asserting).
> 5. **Contract defect (not pointing at members)**: #454 fix round's contract inScope only circled 2
>    files, actually mechanically required surface is 8 (item 6 is to prevent exactly this—the
>    contract template should preset the mechanically required surface list); another that task's
>    terminal state field was once occupied by probe payload, formal acceptance/command evidence was
>    added by the team leader per specification.
>
> **IV. Legacy Items (this batch does not fix, registered for future reference)**: round-3 re-review
> three low-severity findings—① Method path's `build_arg_slots` (`ir_gen.rs:521`) is still the
> criterion's **second** implementation (existing, per last round's caliber registered on closure,
> not consolidated)—**2026-10-16 pre-release audit supplementary three points**: (a) it has **two**
> call points: `ir_gen.rs:7888` (method binding path, using the bound function `binding.function`'s
> parameter name) and `:7975` (overload resolution path, using the resolved hybrid name leaf
> segment), the original registration only wrote "`ir_gen.rs:521` area"; (b) it has **already
> diverged from the shared implementation `bind_args_to_params`**: missing rule ⑤ (duplicate name →
> E1015), rule ⑥ (missing required parameter → E1010) and the "optional position" concept, and
> hard-codes the receiver position as `1 + i`; both call points unconditionally `continue` for empty
> slots (comment writes "diagnostic goes to typecheck (it guarantees that if resolution passes there
> are no empty slots)"), a **dangling invariant written in comments, without assertion fallback**;
> (c) **RFC-039 D60's "all call points converge to it" enumeration missed these two places**, a
> declaration-fact mismatch, and the cited line numbers were not re-taken on the referenced
> revision, with this item also correcting the D60 text; ② The batch verification tree (worktree)
> `cargo fmt --all` error is a worktree workspace artifact, the gate list should note that fmt goes
> to the main repository; ③ The repair class contract template should preset the mechanically
> required surface into inScope (see III.5).
>
> **V. Final Gate (closure revision, local two implementation submissions + one documentation
> submission unified push—this batch's only push)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 /
> corpus_probe **342 lines byte-by-byte consistent with baseline** (only regenerated line:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number synchronized,
> manually reviewed) / IR snapshot zero drift.
>
> **VI. Run Links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test was cancelled per concurrency
> by subsequent push—cancellation ≠ failure, covered by later run); governance tool push → CI
> 38546373435 / 38561405117; WBS registration (3.16 row + header 56→57 / 147→148) → CI 38528560364.
> The push's own CI / Deploy Docs run is back-filled into the task record after push.
>
> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail adds `Linking` (**pure
> merge**, whole block skips `is_bin_role` entry check—E3020 criterion still goes to
> `RoleClassification` to prevent double-reporting; another hard reason: the run path judges "has
> main" by merged IR, under collection semantics "entry itself errors → no IR" would be misjudged as
> missing main and hard-aborted) → `Monomorphization` (check shape); `ir_generation_check` retains
> unit IR (**in memory only**, for pure merge and mono consumption, not emitted/written to disk/no
> snapshot produced—if not retained, aggregation request is empty, mono gives `NoObligations` false
> green); mono failure in Check goes through the per-file collection channel (diagnostic goes to
> entry file + `failed_stages`, not set as failure), MultiFile keeps `fail_with`; `into_outcome`
> changed to "first take collection then judge fault". ④ Empirically **zero production change**:
> full program BFS already covers cross-unit mutual recursion (pathological → E3005, well-formed
> three-layer chain execution produces correct value), delivered four e2e groups + one mono unit
> test as living criteria.
>
> **Verification Chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-comparison with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-shape matrix
> (single/multi/check three shapes + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6
> re-review PASS; test count 2643 → 2656 (only increase, no decrease).
>
> **F1 (found during verification, D59 A/B class, same-round fix)**: Under standalone multi-file
> check without project root, the aggregation request name (`lib.f`) and the unit IR definition key
> (`f`) are from different sources → mono falls into "native has no IR definition" silent
> `continue`, check reports 0 error/exit 0. Fix: aggregation side adds reverse-lookup skip by each
> unit's **real definition key** (exact enumeration + literal equality + ambiguity abandon, not
> suffix heuristic; project shape not registered, existing path unchanged). **Errata (same day)**:
> the reverse-lookup skip was **deleted** in the subsequent regression fix; the root cause is now
> solved by `module_key` dual-use decoupling (added `script_mode`) from the naming caliber—IR
> definition key = IR call name = aggregation request name = `{unit key}.{name}`. **Regression
> Record**: after ③ landed, CI's `Check tutorial/guide code examples compile` red (258 blocks / 10
> failures: 6 same-file same-name rebinding + 4 colliding with `<std/list>`, rootless check without
> qualification triggers `link_module_irs` conflict check); fix = ir_gen splits `script_mode`
> (qualified name ⊥ Script/Bin, E3023 gate changes to look at it) + `ir_generation_check` per unit
> constantly passes unit key + `.with_script_mode(rootless)` + Script top-level same-name rebinding
> "latter wins" (consistent with run path existing behavior); verification = doc gate 258/0,
> three-binary comparison (HEAD/③ tree/fix tree) three collision fixtures, check shape E3005 not
> regressed, E3023 not triggered.
>
> **F2/F3 (accepted)**: mono diagnostic goes to entry file entry, span can cross units (same caliber
> as run path `path=entry`); Check shape holds one ModuleIR per unit (in memory only, released when
> check ends).
>
> **Pre-existing issues left for follow-up (not this round's regression)**: ① E3005's help text
> still writes "internal error; please report this issue", inconsistent with resource protection
> semantics (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of
> E3005)—registered pending scheduling; ② multi-file E3005's span rendering label misalignment
> (reproduced with HEAD binary, pre-existing); ③ request collection surface has **type parameter
> position dependency** (generic calls where type parameters are not at value parameter positions do
> not emit instantiation requests, mono records `NoObligations`, convergent mutual recursion is
> exactly this shape)—same surface as 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Removing `pub` and Auto-Binding (#399)

> **P3.5 Implementation (2026-10-08)**: All seven second-level tasks completed, the whole batch in
> one commit (independently revertible).
>
> - **Measurement vs Plan Mismatch (faithfully recorded)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—actually after deleting `KwPub`, `pub`
>   falls back to a regular identifier, the old writing falls to the semantic layer and reports
>   **E1001 Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically
>   probing removed syntax is an anti-pattern", **no new** migration-specific diagnostic; corpus
>   fixture `pub_declaration_err.yx` nails E1001, the document (`syntax.md`) faithfully states this
>   consequence.
> - **Design Convergence (beyond RFC literal, needs review)**: After pub deletion, the two fields
>   `project_refs` and `cross_file_refs` have completely overlapping semantics (the same
>   "package-internal reference union", Prohibition One prohibits parallel representation) → merged
>   into a single `project_refs`, the two switches `set_exempt_pub`/`set_cross_file_refs` are
>   deleted. Behaviorally, non-pub items of the Internal role change from "intra-file reachability"
>   to "reference pool judgment" (relaxed, better to miss direction); Lib is tightened from "pub
>   absolute exemption" to reference pool judgment; Script's unreferenced top-level bindings start
>   reporting W1001/W1002/W1004/W1005.
> - **Dead Table Cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (production end pours,
>   full warehouse zero consumption).
> - **Test Migration** (test count not reduced): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer a modifier"; `passes/tests/dead_code.rs` 7 role semantic test cases rewritten (2
>   assertion reversals); `tests/integration/cli_e2e.rs` 8 role test cases rewritten
>   (`lib`/`internal` exemption assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` does not participate in IR); corpus diff baseline
>   only added 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` manually reviewed after regeneration.
> - **Measured**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ification (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional submissions, closure
> (registration / WBS check / gate / push) completed by t7. Prerequisite = 4.3.1 obligation ledger
> (red state archive 173 files / 533 keys).
>
> **I. Carrier Ruling (A: `PlanId` travels with AST node)**: added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` represents synthetic nodes
> outside the parser), issued by the **unique allocator** `ParserState::alloc_plan_id`; carriers at
> two places—`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call point identity),
> `Expr::call` is the whole warehouse's unique call constructor. Rejected is the "two sides each
> derive the same identity from span" positional coincidence form (D20's original defect).
>
> **II. Two-Level Verification and Re-review (members independent)**: Each functional submission
> paired with an independent verifier and a re-reviewer, **verifier/re-reviewer do not reuse
> implementer evidence**: 4.5.1 = t1 implementation / t2 independent verification (external tree + 5
> mutations) / t3 re-review; 4.5.2 = t4 implementation / t5 independent verification (external
> tree + 5 mutations) / t6 re-review. Re-review time point and the re-reviewed revision both fall on
> immutable history (verifiable by `git show`). **t5 / t6 changed members due to provider
> renaming**: the original verifier/re-reviewer could not continue due to provider renaming, per the
> orchestration convention changed person to continue and **stated the reason in the record**
> (cross-round context retention priority, only change person when compression is unusable); after
> continuation the conclusion was consistent with the original round (PASS), no reuse of implementer
> evidence.
>
> **III. Two Defects Found and Fixed During Implementation Self-Check (t1, both within inScope)**: ①
> Consumption exemption criterion was once hung on `UnitState.ir.is_some()`—`check` shape's
> Linking/Mono will take away `unit.ir`, truly unconsumed in check shape **silently passes**
> (mutation measured: D41 red while CLI check exit 0); changed to read the **ledger's own completion
> flag**, and added regression cases. ② Drop sequence drifts with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion produced 106/120/114 row count drift three
> times in a row; changed to **total order sort** (same name takes max depth + name ascending) then
> ran twice with zero drift.
>
> **IV. Criteria and Evidence Numbers**: After 4.5.1, `test_release_plan_spans_consumed` removes
> `#[ignore]` persistent (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in
> the same submission (programmatic review: added 1083 lines Drop + 86 lines jump index translation,
> zero deletion, no rearrangement), corpus 342 rows diff empty; 4.5.2 added
> `test_overload_resolutions_consumed` (diff empty + evidence surface floor `examined ≥ 1` /
> `consumed ≥ 2`; measured examined=1, consumption entries 2, no resolution skipped 210), resolution
> shape 9 items nailed board, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation driven: consumption key offset one position ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate diff; restored to green after revert.
>
> **V. Classification Basis for Semantic Correction (D59)**:
> `Type.method(positional arg, name = value)` before the change **silently gave the wrong answer**
> (measured clean compile, runtime output `1` instead of `20`) ⇒ **Class C** (old behavior not
> successful), fix takes "loud": after the change the same program compile-time `error [W1090]`; the
> terminal semantics of "named actual parameter participates in resolution" is **Class D**,
> registered in §3.14 and left issue #452 for a dedicated team. Another: `Drop` is a no-op in the
> interpreter (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`)—key
> migration does not change runtime behavior.
>
> **VI. F-1 / F-2 Disposition (found by t5 independent verification)**: **F-1 (medium)** no-binding
> consumption point (`ir_gen.rs:7766-7790`) is unreachable under the current corpus and test
> surface, keying correctness has no red state falsifiable ⇒ per "known boundary" registered in
> [02](02-stage-contract.md) §2 revision note paragraph 5 (retention reason + no new task); **F-2
> (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise pointers
> (produced `inference/expressions.rs:4550`, consumed `ir_gen.rs:7697` / `:7772`), separate small
> submission, not mixed into doc submission.
>
> **VII. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement flag);
> 07 criteria table and 04-ssa defect class 3 changed to past tense; 09 checks 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG records one line of behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Closure Found Accounting Deviation**: pointers `ir_gen.rs:357`
> / `:3102` / `:1096` in t3 / t6 reports differ from code fact by one to three lines (actually
> `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2's insertion)—registered with code fact as
> standard, this annotation also retains trace.

## P4 4.10.3 standalone run and MultiFile Orchestration Unification (run half of 4.2.5 Ruling A)

> **Batch C Record (registered 2026-10-15 → closed 2026-10-15)**: User 2026-10-15 ruling C—this
> round only does 4.10.3 (same-batch candidate 3.4.6 per B1 ([02](02-stage-contract.md) §runtime
> behavior table "dead code family warning does not enter compile path") held in abeyance until
> after 4.9, prerequisites re-attached and noted); implementation form = **complete single team**
> (user instructed not to open DAG, not to build team, single-session implementation +
> self-verification). Pre-batch debt landed first: #455 dependency announcement fix (rustls
> 0.23.46 + serde_yaml_ng, `cargo audit` from exit 1 restored to exit 0), WBS registration debt
> (#456 registered as 3.17 + uncompleted rows add "belonging phase / whether blocks main line"
> fields).
>
> **I. Red Before Green**: driver-level red criteria 3 (`src/driver/tests/run_rootless.rs`—rootless
> merged IR must not carry entry_function (red state `Some("main.main")`), rootless top-level
> executable statement must not report E3023 (red state produces no module), Bin control
> entry_function maintained (green state persistent)) + CLI e2e 3 nails (standalone relative import
> run (red state E5001), Script top-level statement execution and #413 hint two green state nails).
>
> **II. Implementation Surface**: CLI `run` uniformly goes through `compile_project` (the
> project/standalone binary split in `run_file_with_diagnostics` deleted); Linking's
> `entry_function` gated by `is_bin_role` (#413 T4—Script does not auto-call main); #413 hint keying
> changed from `module.source_files.is_empty()` to call point project probe (after unification
> ModuleIR constantly carries source_files, original marker fails); `script_main_never_called`
> adapted to qualified name form (function `{key}.main`, global slot measured also qualified name—IR
> forensics as standard, not inferred from comments).
>
> **III. Unification Exposed Pre-existing Gaps (same-batch pass-through fix, A/B/C class—only fix
> failed programs to success, zero impact on previously successful programs)**:
>
> 1. **Explicit Type Argument Pre-Specialized Name** (`mk(Int)(5)`): ir_gen per D6.3 directly emits
>    specialized name (mono strips unspecialized original), `resolve_call_target` three arms (rename
>    / aliases / dot prefix) do not cover the `mk(int64)` form ⇒ call point dangling E6006. Fix =
>    this single point parser adds a fourth arm (base name qualification, type parameter string
>    preserved as-is). **Bin Project Same-Form Reproduction** (pre-existing defect, not introduced
>    by this batch).
> 2. **Script Top-Level `mut` Assignment Statement Repeated Slot Allocation**: `extract_global_defs`
>    and `register_global_slots` count `counter = counter + 1` (Assign same-form as declaration) per
>    statement ⇒ same variable allocated 3 slots, initialization writes the post-allocated slot
>    while read uses the pre-allocated slot (runtime Add type mismatch). Fix = both places merge by
>    name (same criterion same source).
> 3. **Same-Module Constraint Generic Operator Dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually is `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table lookup verification (exact hit →
>    generic's unit key qualified hit → fallback to bare name current state; not suffix heuristic,
>    same form as `resolve_generic_name`).
>
> **IV. Registration Gap (this batch does not fix)**: Constraint dispatch when **type and generic
> definition are in different modules** still E6006 (rootless two-file probe verified; Bin
> same-form)—root cause = MonoType user type name does not carry definition module, correct fix
> belongs to P6 type identity convergence; registered WBS **3.18** (belongs to P3, prerequisite
> awaits P6, does not block main line), `qualify_deferred_method` note same surface.
>
> **V. Test Adaptation (honestly recorded)**: `tests/integration/multifile.rs` fixture added
> manifest—what it tests is RFC-029 project orchestration (Bin auto-entry) not Script semantics; one
> nail in `cli_e2e` (top-level diagnostic variable name visibility) and `script_main_hint` fixture
> (7 cases) synchronized per qualified name form—production and consumption sides only consume
> merged IR, fixtures directly drive same form, no need for pipeline product substitute.
>
> **VI. Final Gate (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regeneration **12 lines**—line-by-line structured check: exit / stdout / diagnostic
> code all per item identical, only stack frame function names and variable names qualified by unit
> key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process Events (faithfully recorded)**: In this batch, during the pre-commit hook running,
> a concurrent edit of the workspace document was eaten by the hook's stash rollback and the commit
> failed (a concurrent form already warned about in the config comments)—discipline = don't move the
> workspace during hook running; the #455 commit was therefore split into two attempts (second time
> all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 Batch Record (registered 2026-10-16 → closed 2026-10-16)**: User 2026-10-16 instructed to
> proceed in order 4.6 → 4.9 → 4.4 → P5, do 4.6 first; implementation form = **complete single
> team** (following the previous batch's user instruction: no DAG, no team, single-session
> implementation + self-verification). Task definition (09 three-level table): the semantic data
> (SemanticDB) of project-internal files goes through the orchestrator's same pipeline, from the
> same source as diagnostics; cross-file references carry `resolves_to`. Acceptance:
> project-internal cross-file go-to-definition hits the definition; semantic data and diagnostics
> come from the same compilation.
>
> **I. Current State Verification (verify before asserting)**: didOpen/didChange triggers **two
> rounds** of compilation—diagnostics go through the unified Driver pipeline (4.2.3/4.2.6 unified),
> semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` parsing fails, cross-file reference events not produced, go-to-definition/rename/references
> lookup silently fails for imported symbols—D55's silent failure). The orchestration arm's checker
> uniformly names `"<module>"` (SemanticDB file identity lost), `type_result` (containing
> semantic_db) is discarded when diagnostics are non-empty and never emitted; `checker.rs` clearing
> forcibly overwrites `resolves_to`'s DefId file identity to current file; both
> `Export`/`ModuleInfo` have no definition position surface.
>
> **II. Red Before Green**: LSP e2e nail (`src/lsp/tests/cross_file.rs`)—temp two-file project
> didOpen main.yx then sends GotoDefinition to the `add_one` call point, **before fix returns None
> (red, empirically the silent failure in the previous section)**, after fix hits lib.yx line 0
> definition (green). One probe positioning in the process: the compiler-side chain (driver probe
> dump) was fully connected but e2e still empty—root cause is `resolve_reference` matches exactly
> per reference **starting point** (D55 precise-only current caliber), test cursor lands in the
> middle of the identifier; this tolerance belongs to the existing behavior surface, this batch does
> not touch (see VII).
>
> **III. Implementation Surface**: (1) `Export.definition_span` (extract_module_info reaped from AST
> identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled per DiscoveredFile, including
> vendor; std always None); (2) checker reference clearing via `import_def_locations` (use line
> binding span → definition file + definition span) redirects cross-file imported items'
> `resolves_to` out of this file; std import no registration, maintains fall to use line; (3)
> checker module identity changed from unified `"<module>"` to unit file path (verified
> `env.module_name` all 4 consumption points zero diagnostic consumption); (4)
> `DriverOutcome.semantic_db` channel—CollectAll form (Lsp target file **regardless of diagnostic
> success or failure** + SingleFile fallback) merged in, Lsp arm tail injects non-target file
> top-level export definition surface from registry (`external_definitions`, even unopened files can
> go-to-definition; type/submodule exports not injected—typecheck side type registration does not
> produce definition events, both sides maintain consistent gap); (5)
> `orchestrator::analyze_source_in_project` (`check_source_in_project` redirected to take diagnostic
> half); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key uniformly normalizes file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stably hits); 5 handlers +
> semanticTokens dual request connected to `db_key`.
>
> **IV. Pass-Through Fix (D59 Class B—old behavior silently does nothing, previously successful
> programs zero external impact)**: ① Project-internal file cross-file
> go-to-definition/rename/references lookup changes from silently failing to hitting real
> definition; ② When lexical errors occurred, old manual pipeline deleted semantic information
> (coloring disappeared) → residual AST coloring preserved (4.2.6 editor philosophy extension); ③
> When target file has type errors, semantic data no longer discarded with `type_result` (typing
> intermediate state coloring/go-to-definition not lost).
>
> **V. Nails**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`—target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback to use line, error file preservation, single-file fallback path key,
> single-file error preservation) + LSP e2e 1 (red before green) + uri normalization 4
> (`src/lsp/tests/uri.rs`—drive letter conversion, space encoding, idempotent normalization,
> non-file scheme pass-through).
>
> **VI. Mutation Verification (scratch.py trial, mechanically reversible)**: batch tree wbs-46 @
> closure revision 39546020, both mutations make hit nails turn red, blob after restoration
> consistent with HEAD + git status CLEAN—m1 removes `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver nail + LSP e2e nail red, remaining 7 green; m2 removes
> `external_definitions` injection (arms.rs) ⇒ same two nails red (round-trip parsing loses anchor),
> remaining 7 green. Batch tree gc'd (freed 4.8 GB, `status` no batch tree).
>
> **VII. Final Gate (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new nails) / `--test integration` **287 passed** / corpus
> diff **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synchronized with server.rs:482 and 02
> §obligation field table) / check-docs-truth 0 / docs-examples 258 items 0 failures / docs-orphan 0
> / markdownlint only remaining in the whole warehouse user's unsubmitted 032 two MD056 (user's own
> edits, not touched) / pnpm docs:build 0. **Process Event (faithfully recorded)**:
> en/09a-wbs-history.md:54 one MD037 was introduced by the auto-translate bot re-translating the
> previous batch's 09a (zh original has no space), this batch fixed it in passing—markdownlint red
> is not this batch's change surface but is in the same file as 09a, registered together. **Legacy
> Observation (registered for discussion, not touched)**: `resolve_reference` only matches exactly
> by reference starting point—when the cursor lands in the middle of the identifier,
> go-to-definition/hover/rename silently do nothing (D55 precise-only current state, whole file form
> is isomorphic), if tolerance matching is needed it belongs to independent behavior change,
> reported to user for ruling.
>
> **Same-Day Continuation Batch (4.6.2, implemented after user ruling)**: After the closing report
> was delivered, the user ruled on "Legacy Observation ①" "OK, then change it"—query key tolerance
> implemented per plan A: the four handlers' definition/hover/references/rename query keys changed
> from cursor position (via UTF-16 conversion) to the **identifier's own starting point** located by
> `find_identifier_at_position` (same coordinate system as semantic events, no conversion needed);
> `resolve_reference`'s starting point exact match semantics unchanged (D55 precise-only unchanged).
> The definition point fallback judgment (start <= col < end) for references/rename is isomorphic
> for identifier starting point. Red before green: definition middle cursor nail first red (None)
> then green; four handlers middle cursor 1 nail each + cross-file e2e start/middle dual position
> assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0. WBS 4.9.2 registered (three-level
> count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 Batch Record (registered 2026-10-16 → closed 2026-10-16)**: User 2026-10-16 ruling merged
> three related three-level tasks 4.9.1 / 4.9.2 / 4.9.3 **into one team in one round** (reason: they
> all belong to "the pool's source and identity" surface, merged landing can avoid repeated
> rewriting of the intermediate state after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong to
> consumption refinement and test pool separation, depend on 4.6.1's `use` path resolution, forcibly
> batching would expand the gate surface). #450 belongs to **pending ruling**—evidence shows it
> belongs to "name not passed back to reference count after overload resolution" class, still
> reproduces under the form landed in round 1, register per independent WBS item for disposition
> (team leader recommends A: register as independent item when merging into round 2; **pending user
> ruling**). Frozen rev = `3863a9ff0` (parent commit `62f6e1a59` is the pre-fix baseline);
> verification PASS attempt `aab92fe0` (i.e., t2); re-review pass attempt `96452696` (i.e., t3).
> **Closure registration submission = `38d30b846` (docs(docs): 4.9 reference pool round 1 closure
> registration)**—this was actually written and pushed by the closure member integration-refpool;
> the closing probe once mislabeled the closure task as terminal failed and overwrote the output,
> taken over by the team leader with a new attempt and changed back to completed per verified
> delivery fact, here corrects the attribution and label. Evidence archive:
> `target/yx-scratch/yx49/EVIDENCE.md` (including 8 mutation patches and warn_compare_v8.json).
>
> **I. Merge Ruling**
>
> 1. **Ruling Itself**: Three related three-level tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round**—the former depends on 4.6.1's delivered
>    `use` path resolution (already landed in `39546020`, can start); the latter goes in the same
>    batch as the RFC-029f table incorporation, registered at the end of RFC-029f pending change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutual prerequisites (4.9.2 changes pool type → 4.9.1
>    assembly surface must be rewritten → 4.9.3 only has the role table for denominator
>    determination), doing in rounds will inevitably write an intermediate state then change twice;
>    (b) all three belong to "the pool's source and identity" surface, 4.9.4 / 4.9.5 belong to
>    another surface, batch division is based on "cohesion" rather than "independently revertible".
> 3. **Merge Result**: single commit `3863a9ff0` (one-line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash Explanation**: `git pull --rebase` before push merged the
>    auto-translate bot commit (`72f364169`); the rebase drop point is after the other's commit, the
>    batch's two commits (implementation `3863a9ff0`, its pre-fix baseline and also the other's doc
>    commit `62f6e1a59`) **were not rewritten, maintain original hash**—the first version mistakenly
>    wrote "`3863a9ff0`→`3863a9ff0`" identity mapping here (looks like "same value after rewrite",
>    actually "not rewritten"), corrected on 2026-10-17 at closure; the other's code commit
>    `33f14e8ab`→`b122b7ad1`. **External references uniformly use post-push hash as standard**
>    (rewritten old hash is unreachable in new clones, identity determined by patch blob
>    comparison).
>
> **II. Five Criteria Red/Green Measured Caliber**
>
> | #   | Criterion                                                | Red State (Before Fix / P1 Directory Scan Recurrence Patch)                                                                                                 | Green State (Round 1 Landing)                                                                                                                                                                                                                                                                                                                                                                       |
> | --- | -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool's root = package target (#334)                      | Directory recursive scan ⇒ files/identifiers under tests/ also enter the pool, exempting production code unused definitions                                 | `collect_package_refs` (`helpers.rs:481`) per `ExplicitSurfaces` root calls `discover_with_used`; full root degenerate falls back to `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                                  |
> | ②   | Pool granularity = `(module, name)`                      | Same-named local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) unique assembly entry; `arms.rs:603-609`'s `other_module_refs` unions all module entries "other than this unit" then sends to `DeadCodeAnalyzer`; module's own name (top-level definitions and local bindings, `use` import name exception) no longer exempts other modules' same-name definitions |
> | ③   | `FileRole::Script` does not use pool                     | Script affected by references from other files in project                                                                                                   | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test goes through `_ => {}` default arm (per file-internal definition-usage graph)                                                                                                                                                                             |
> | ④   | Fixture A: lib.yx line 4 definition should report W1001  | Before fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                         | Red state measured (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                          |
> | ⑤   | Fixture B: main.yx line 4 definition should report W1001 | Before fix: reports `W1003@1`                                                                                                                               | Same as above                                                                                                                                                                                                                                                                                                                                                                                       |
>
> **Criterion ②'s "Deduced Red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` is in helper.yx; the old pool under bare name union treats `foo` as a whole "used by
> helper", lib.yx's `foo` exempt. **Measured**: red2 fixture vs pre-fix binary → no W1001 (old
> caliber exempt); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script does not use pool) red state fixture **first version failed**: the first
> fixture in the same file placed a same-name local variable triggering E2010 (the language does not
> allow same-module shadowing of top-level definitions), changed to another module (helper.yx) then
> deduced red state. Both t3 re-review and t2 verification independently reproduced this, **not a
> contract defect**—same-module shadowing is the language's hard constraint, fixtures must be
> cross-module.
>
> **III. Correspondence Between +8 and +16 Two Sets of Counts** (**t3 re-review's evidence as
> standard**)
>
> `target/yx-scratch/yx49/EVIDENCE.md`'s C section writes `+16/−0`, but missed the "**new fixture
> record**" component—v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff,
> `only_before=0`) caliber **includes** new fixtures; t1 implementer's self-built "+8" is
> **warn-before / warn-after (325 each) aggregated by fixture** (each member file checks once).
> Re-review mapped item by item:
>
> - **+8** (8 new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (Unified Attribution)**: these 8 fixtures **did not declare package target** ⇒ 4.9.1
>     root degenerate to checked file's own compilation set, consumer main.yx is not in it; this is
>     in-design change (before, these files' check surface had zero warnings, not exemption path).
>     Repository root `yaoxiang.toml` is empty file ⇒ same degenerate path measured to hold.
> - **+16 Residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files' in t1's `warn-before.jsonl` has `exit=None` (capture time point
>     **does not exist**—are this batch's newly added fixtures). Therefore +8's "+ 0 − 0" diff
>     doesn't have them, +16's v8 symmetric diff does.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` left: the reason is the pre-fix binary also
>     reports `probe@stray.yx:4` (the fixture runs `probe` once for each file), **cancel out**
>     leaving only lib's 1—three data sets (+8 / +16 / dual binary ABBA) mutually consistent.
> - **New Fixtures 8** vs **+16 Residual 8**: same batch of files (`ref-pool-compile-set/` and
>   `ref-pool-module-identity/` added by 4.9.1 / 4.9.2), the instrument treats each of their checks
>   as an independent record—the same file may trigger different diagnostic entries for multiple
>   `role/context`.
>
> **Conclusion**: **+16 = +8 (aggregated by fixture) ∪ new fixtures 8**—both counts are correct,
> **different calibers**: t1's "added 8" is fixture-layer caliber, t2's "+16" is file-layer caliber
> (symmetric diff includes new fixtures). t3 re-review ① +8⊆+16 verified; **re-review ② writes "do
> not copy EVIDENCE.md section C verbatim"**—section C missed the "new fixture record" component,
> this item is rewritten per t3 re-review `pass` conclusion.
>
> **IV. Mutation Experiment (Mechanically Reversible, All `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b use import name
> removal, p3b script connect pool, p3 flat consumption, p4 std connect pool, p7final self-built 4
> criteria, p8 std close, p9 vendor connect pool + p_stack2 / p_stack3 overlay). Each trial output
> contains: HEAD blob → mutated blob → restored blob consistent with HEAD = yes + `git status`
> CLEAN. Local `core.autocrlf=true` needs to be injected with `false` via `GIT_CONFIG_*`
> (`trial_env.ps1`), otherwise patch new lines convert to CRLF causing apply/restore double
> failure—**only affects patch mechanics, does not change measured behavior**.
>
> **V. Process Events / Environment Lessons (Must Read for Handoff to Next Round)**
>
> 1. **core.autocrlf Injection**: Local `core.autocrlf=true` must be injected with `false` via
>    `GIT_CONFIG_*` to ensure patch mechanical reversibility; see
>    `target/yx-scratch/yx49/trial_env.ps1`. **Do not reuse** any trial script without injection in
>    the next round.
> 2. **Same-Tree trial Serial**: On the same tree, `trial` and `cargo build` **must be strictly
>    serial**. `trial` internally `git apply` and `cargo build` both touch the worktree, parallel
>    will produce "patch incomplete but seen by build" or "build and apply interlock" two types of
>    races.
> 3. **trial Rebuild Binary Must Confirm Sync**: After `trial` completes, it will rebuild the
>    in-tree binary (cargo's fingerprint includes source path, **change tree = full rebuild**),
>    subsequent measurement must confirm `bin/yaoxiang-rs` is synced with HEAD. **This round once
>    used residual mutation binary to measure false data**, rebuilt and re-measured to correct.
> 4. **Second Tree yx49-b's Tree-Building Reason**: For comparison need pre-fix binary (62f6e1a59),
>    no existing in-batch; `scratch.py init yx49-b` opened a second tree (another full build).
>    **Tree released** (see VI), deletion left batch closure `gc` disposition.
> 5. **cli_e2e Two Internal Case Fixtures Supplemented to Legal Package (Added `[run].main`,
>    Assertion Unchanged) Reason**: Before fix, in `cli_e2e` two `Internal` role nail fixtures **did
>    not declare `[run].main`** ⇒ after 4.9.1 landing `compile_project` takes the `ExplicitSurfaces`
>    root call path, can't find `main` ⇒ whole batch 2 fixtures behavior drifted from "Script
>    fallback" to "entry not found"—a fallback behavior introduced by 4.9.1, not a defect. After
>    adding `[run].main` field, the two fixtures take the normal package path, **assertions** (exit
>    code / diagnostic code / warning set) verbatim unchanged.
> 6. **Safe Way to Delete Second Tree**: After release use `python scripts/dev/scratch.py rm yx49-b`
>    (not `rm -rf`), will verify "no mount" then delete. Released before this batch's closure, next
>    step `gc` naturally cleans up.
>
> **VI. Batch Tree State**
>
> Before closure: yx49 released (4.8 GB freed), yx49-b released. Two trees have no mounts, status
> can be re-checked in `scratch.py status`. This task output ends with the full `scratch.py gc`
> status.
>
> **VII. RFC-029f "Known Defects" Table Flip**
>
> Previous version (2026-10-09) described in present tense "three design defects of the reference
> pool" (including `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396`
> etc. already-deleted code positions). This batch's closure synchronously rewrites to "**Known
> Defects: Landed (2026-10-16, WBS 4.9 Round 1)**": ① Current pool's root = `collect_package_refs`
> (`helpers.rs:481`), parsed per `ExplicitSurfaces`, not directory scan; ② Current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly unique entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumption end `arms.rs:603-609`; ③ Current Script does not connect to
> pool, `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-taken on the referenced
> revision `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` were split per P4 4.2.9
> (`c59fb751`) and moved into `src/driver/{state,arms,helpers}.rs`, the old positions in the table
> no longer hit—this is a fix for "declaration-fact mismatch" (t3 re-review remark ①).
>
> **VIII. Final Gate (Closure Revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixture
> behavior runs, 338a339,340; the other 342 lines byte-by-byte unchanged) / IR snapshot 204 zero
> drift / check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process Event**:
> this batch one cargo build because `core.autocrlf` converted script line endings to CRLF, team
> scratch path under `yaoxiang-rs` startup reported `BadImage`, measured to be encoding inconsistent
> with git index—passed after rebuild (parallel to 4.6 batch "external whole-tree discard" lesson,
> different cause).
>
> **IX. Legacy (Handoff to Next Round)**
>
> 1. **#450 Attribution Ruling**: Evidence shows it belongs to "name not passed back to reference
>    count after overload resolution" class, **still reproduces under the form landed in round
>    1**—dispatch per user ruling C flow. **#450 Probe**: dual binary same set `{W1002@2, W1005@4}`
>    (measured, rootless form), no direct relationship with round 1 refactoring, but #450's fix may
>    need to wait for 4.9.4 (consumption refinement) together—because the root cause is "name pass
>    reference count after overload resolution" needs the precise consumption surface of use items,
>    4.9.4's "real use after being imported" is the precise version of the same kind of criterion.
> 2. **dead_code.rs:683 Pointer nit**: Comment writes "two pools' assembly caliber see driver-side
>    `collect_compilation_refs`"—actually `collect_compilation_refs` is the fallback path, main path
>    is `collect_package_refs` (`helpers.rs:481`). Comment and code main path caliber inconsistent
>    (**not in this batch's writing scope**), left for 4.9.4 / 4.9.5 to correct together.
> 3. **4.9.4 / 4.9.5 Prerequisites Unlocked**: 4.6.1 (`use` path resolution) delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to first determine
>    RFC-029f table incorporation (already laid by this batch's "VII" flip).
> 4. **`yaoxiang check` Multi-File `run` Never Reports Dead Code Family Warnings** (WBS 3.4.6,
>    2026-10-15 user ruling C held in abeyance until after 4.9)—this batch round 1 did not involve;
>    next round 4.9.4 / 4.9.5 to handle in the same batch.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 Batch Record (implemented 2026-10-16 → closed 2026-10-17)**: covers 4.9.4
> (consumption criterion = real use after being imported), 4.9.5 (test pool separation), 4.9.6
> (method reachability caliber, issue #450, user 2026-10-16 ruling A merged into round 2). Closure
> fixes (verifier-side remediation M1/M2/M4 recompile redo + F1 fix round + PA-2 delete dead branch)
> and registration gap issue #464 (WBS 4.9.7) settled together.
>
> **I. Adjudication Chain and Submissions (uniformly post-push truth)**
>
> | Step | Content                                                                                                                                      | Post-Push Commit | Pre-Push Local Value | Determination Method                                         |
> | ---- | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- | ------------------------------------------------------------ |
> | 1    | fix(util): check explicit path not blocked by default exclusion table + --exclude always empty (issue #463, P0-0.4)                          | 366bc5a69        | ef959ca55            | Patch blob identical                                         |
> | 2    | Stage A = 4.9.4 + 4.9.5 (RefPool attribution per import + Test does not contribute consumption surface)                                      | e6cf1aeaa        | 84a6f6978            | Patch blob identical                                         |
> | 3    | Stage B = 4.9.6 (#450 per-candidate method reachability; including generated artifact warning-codes.md)                                      | ddcbb12d0        | cd629b5fe            | Patch blob identical                                         |
> | 4    | docs(meta): AGENTS 10th item ⑦ (commit-during stash window convention)                                                                       | ec94fa9d8        | 9a4ed5f3b            | Patch blob identical                                         |
> | 5    | Fix round = F1 (overload sibling overload recover report W1005) + F2 (verifier-side M1/M2/M4 recompile redo) + F4 (snapshot generation note) | b8380e6ab        | 3d9ef6eab            | Patch blob identical (36962f318e42…, git range-diff no diff) |
> | 6    | Closure fix = PA-2 (delete `strip_unit_prefix` dead branch + 029f/CHANGELOG three places false statement correction, register #464)          | b940300ac        | e13984465            | Patch blob identical (patch-id 9bda3cfe4c…)                  |
>
> **Hash Explanation**: `git pull --rebase` before each push will rewrite local hash; the
> "Determination Method" column in the above table is all "patch blob identical"
> (`git diff <rev>~1 <rev> | git hash-object --stdin` consistent on both sides + `git range-diff` no
> diff). This article does not cite any pre-push hash as authoritative source.
>
> **II. Three Criteria Red/Green Measured Caliber** (Verifier Self-Built Reproduction, Not Adopting
> Implementer Criterion)
>
> | Criterion                  | Red State (before binary 366bc5a69 measured)                                                                                                                                                       | Green State (ec94fa9d8 and after)                                                                                          |
> | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() rescues a.yx same-name foo; guard only imported not used also rescued)                                             | Same fixture 2 reported as-is (collision does not rescue + unused import does not count as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print rescued by tests/ reference (only 1)                                                                                                                                     | Same fixture 2 (test reference does not leak); ref_pool's test_ref_pool_excludes_test_role_references passed               |
> | 4.9.6 method reachability  | Self-built fixture (2 groups of disambiguating overload calls + 1 uncalled) 3 W1005 false positives; method_call_args.yx 3 / method_overload.yx 1 (overload per full name coverage only reports 1) | Same fixture just reports uncalled orphan; method_unused 3 nails passed; corpus call_args 3→0 / overload 1→0               |
>
> **Fix Round (F1) Further Tightening**: Brother overload that did not hit resolution recovers
> report W1005 (b8380e6ab—same fixture calling ->Float reports ->String brother, calling ->String
> reports ->Float brother, bidirectional line nail). **Fallback Boundary (Registered #464)**:
> Cross-unit (registry resolution) method calls do not produce resolution entries, fall back to
> name-level exemption, cross-unit same-name same-order previously not reported—register 4.9.7.
>
> **III. Process Events and Lessons (Four Hard Requirements Corresponding)**
>
> 1. **PA-1 Changed from blocker to Existing Capability Gap (medium)**: Round 2 re-review once
>    registered "cross-unit same-name same-order not reported" as blocker (PA-1); subsequent two
>    fact reconciliations (implementer archive + verifier at b8380e6ab/b940300ac dual binary
>    re-measurement of same fixture set, diagnostic set byte-by-byte consistent) confirmed it is a
>    **capability gap existing before round 2** rather than this round's regression—mechanism =
>    cross-unit method calls do not produce resolution entries, fall back to name-level exemption;
>    disposition = re-judged medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Re-reviewer Publicly Corrected before Data and Withdrew Regression Judgment**: Re-reviewer
>    initially cited before data ("−42 contains called method") inconsistent with its original
>    output (W1005 count=0)—after verifier's reconciliation confirmed before capture was 0,
>    re-reviewer's regression judgment **publicly withdrawn** based on this; −42's correct caliber =
>    26 files' called method false positive elimination (Stage B), each item traceable to
>    behavior-run exit=0 files (method really called executed).
> 3. **t13 Terminated Due to Contract Premise Not Holding (Not Implementation Defect)**: This task's
>    acceptance premise wrote "cross-unit same-name same-order will report W1005 regression fix",
>    but measured (t14 dual binary) confirmed this behavior is a registered gap (#464) not
>    regression—contract premise does not exist, task terminated; its probe value taken over by t16
>    read-only judgment (confirmed `strip_unit_prefix` is dead branch).
> 4. **True Criterion for Doc Submission is "Really Build Once"**: Round 2 closure's doc submission
>    (`042c07860`) made Deploy Docs red in VitePress's Vue template compilation—angle bracket
>    placeholders like `<rev>` / `<before-exe>` / `<after-exe>` in the record didn't enter
>    backticks, parsed as elements (reported `Element is missing end tag`, pointing to `<rev>`
>    line); markdownlint can't catch this type, and pre-commit's markdownlint hook in submission
>    containing only docs is judged `no files to check` skipped (measured). Disposition:
>    placeholders uniformly enter backticks, before push locally run `npm run docs:build` once (fix
>    submission `0c837e1b4` with local build exit=0 as evidence); if using `--no-verify` to commit,
>    commit-msg verification needs manual supplementary run.
> 5. **Process Omission of Contract Description Not Synchronized**: t14's task description text
>    residue PA-1 old hypothesis (writes "b.yx reports W1005" expectation), verifier in report
>    actively pointed out "per contract field judgment, description text does not count" and asked
>    team leader to confirm—disposition = task contract field as adjudication basis, description
>    text left as process lesson (**when re-dispatching/dispatching, task description must be
>    synchronized with latest adjudication chain**).
>
> **IV. Mutation Experiment (Mechanically Reversible, All scratch.py trial)**
>
> Round 2 verifier total 9 patches (under target/yx-scratch/yx49/ m1b/m2b/m4b are F2's recompileable
> redo version—initial version M1/M2/M4 commented out if condition lines leaving dangling blocks,
> judged by t8 re-review F2 as "compile failure mis-recorded as criterion turning red" and voided;
> redo version per patch cargo check exit=0 + trial turns red). M3 (collision attribution
> recurrence) initial version is true mutation, measured ref_pool 5 criteria turn red, keep original
> judge. M1b anchor point re-taken on b8380e6ab (F1 fix after probe section refactoring, old anchor
> point fails). All trials: HEAD blob → mutated blob → restored blob consistent with HEAD + git
> status CLEAN. Environment injection follows trial_env.ps1 (core.autocrlf=false via GIT_CONFIG_*).
>
> **V. Warning Surface Full Comparison (Caliber Stated First)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, row, column) four-tuple symmetric
> diff, project state channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: this round before capture already contains all fixtures ⇒ matched set =
> full set 337, no round 1's +8 (matched increment) / +16 (full set) dual-caliber issue. Result:
> only_before = 42 W1005 (26 files, all called method false positive elimination = Stage B's −42;
> per file verified all on behavior-run exit=0 baseline, method really called executed—**no true
> dead code rescued**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface ×2 +
> ref-pool-test-pool's ../src/util.yx:2 = Stage A criterion surface). **Snapshot Generation (F4)**:
> Stage A time point's after capture (Stage B before's 57 residual) and final version (Stage B
> after's 15+2=17) are **two generations**, must be noted when citing; another 1 exception
> (interface_member_conflict.yx's Dog.name@18) belongs to field access form, not in Stage B's W1005
> removal set, consistent before and after.
>
> **VI. Final Gate (Closure Revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no artifact
> change. **Gate Check Arrangement (13th Item)**: This batch's pushed CI / Deploy Docs / Auto
> Translate run conclusions handed to scheduled check link for registration (run id + conclusion +
> consecutive push coverage relationship: later run covers previous one), not waited in-session.
>
> **VII. Batch Tree State**
>
> yx49 and yx49-b both released (gc uniformly deleted at closure); no new directories outside the
> repository in this batch.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 3 Closure (4.9.7)

> **4.9 Round 3 Batch Record (implemented 2026-10-17 → closed 2026-10-17)**: covers 4.9.7
> (cross-unit method call resolution registration and receiver attribution, issue `#464`—WBS 4.9's
> last three-level task; after this round's closure, the entire 4.9 second-level task Kanban is
> checked). Chain = production side t19 → independent verification t20 PASS → re-review t21
> needs_revision (F1: injection surface false positive) → fix round t24 → independent verification
> t25 PASS → re-review t26 PASS (increment, verdict=pass). All commit numbers are post-push truth
> (determination method = patch blob identical; `git diff <rev>~1 <rev> | git hash-object --stdin`
> consistent on both sides).
>
> **I. Adjudication Chain and Submissions (post-push truth)**
>
> | Step | Content                                                                                                                                                                                         | Post-Push Commit | Determination Method   |
> | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ---------------------- |
> | 1    | Production side: cross-unit method call produces D20 resolution entry (`feat(typecheck)`, t19)                                                                                                  | `b8b3cc3f6`      | Patch blob identical   |
> | 2    | Driver-side attribution: method reachability per definition unit gives mark, names with resolution records stop pool's name-level exemption (t20 verification PASS / t21 re-review caught F1)   | `d5c00235a`      | Patch blob identical   |
> | 3    | Injection surface fix round F1: candidate injection per "name ← provider module" **pair-by-pair** filter, eliminate E1107 false positive (t24, including F2 statement and F3 nail single point) | `d5dd1a37b`      | Patch blob identical   |
> | 4    | Nail test homing: `src/frontend/core/typecheck/tests/environment.rs`'s F3 nail moved into declaration surface `src/driver/tests/overload_injection.rs`                                          | `c2f887843`      | Patch blob identical   |
> | 5    | dev tip                                                                                                                                                                                         | `e9675c9dd`      | merge commit, see "II" |
>
> **II. Two Merge Commits at Hash Mapping and dev Tip (Source Unconfirmed, Not Allowed to Write as
> Member's Action)**: Round 3's four commits (above table 1–4) have all been pushed to origin/dev;
> `git worktree list`'s main mount is at `e9675c9dd`. The dev tip has two
> **non-this-team-command-surface-generated** merge commits (`5da05b885` / `e9675c9dd`, reflog shows
> as `pull --tags` + push form), source unconfirmed—per 10th item ④ (reference must be resolvable by
> anyone) and 13th item (gate handed to dedicated check) same closure caliber, here only registers
> the fact and reflog evidence, no member attribution; also see "V" lesson ② (push attribution
> convention must be verifiable).
>
> **III. D20 Contract Extension Actual Form (Key Form and Consumption Point)**: The production side
> packages the three parallel tables "candidate / hybrid IR name / declaration period parameter
> name" in **same order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) and
> carries it across units with the unit, registry deterministically merges by module key ascending
> (`src/frontend/module/registry.rs:150`); typecheck arm per this unit's import face pair-by-pair
> injects (`src/driver/arms.rs:354` → `visible_method_overloads`, `src/driver/arms.rs:1521-1579`:
> name ← provider module pair-by-pair, same-name conflict whole name skip, self key position not
> externally merged), nail form single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same order append + equal length
> debug assert); consumption end attributes D20 resolution product per **definition unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`; `resolved_method_marks`
> `:621`), and stops pool's name-level exemption for name groups with resolution records
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = entry name
> `{Type}.{method}#k` (`{Type}` is definition-side bare name; `#k` declaration order).
>
> **IV. Key Scope and False Positive Defense Measurement (Fix Round F1 and Five Residual
> Boundaries)**: F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set"
> combination merging another module's same-name type candidate into the same key position ⇒
> `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, method same-name
> same-order) fixture `check` reports 2×E1107, exits 1 (previously successful program compile
> fails); after fix `check`/`run` both exit 0, 4 W1005 as-is report **uncalled** same-name
> definitions on both sides (reverse under-report surface also verified). Repository fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level nail
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set before and after fix per
> item consistent (not regressed). Five residual boundaries and per-item measurement (separate
> `check` definition file not report / name layer ambiguity E5006 whole name skip / boundary (c) per
> t20 measured rewrite / inline alias and `ns.T` not inject candidate surface / whole import `use M`
> and std candidate surface not produce entry) full text in
> [029f](../../rfc/accepted/029f-target-semantics.md) §cross-unit method reachability attribution
> "Residual Boundaries", not repeated here.
>
> **V. Process Events and Lessons (Four Continuations)**:
>
> 1. **Contract Wording Written Too Narrow Will Directly Become a Defect**: One of F1's root causes
>    is the contract only wrote "only inject module surface that can be named by bare type name",
>    didn't write "attribute pair-by-pair by name"—implementation per literal took "name set ×
>    module set" combination, re-review only caught the E1107 false positive. When describing
>    behavior boundaries in the contract, the **combinatorics** must be written dead (per name
>    pair-by-pair / full combination), cannot be left to implementation's own understanding.
> 2. **Convention Must Be Verifiable: "push by team leader" only written in dispatch message ⇒ when
>    third-party push/merge appears, attribution impossible**. This round dev tip had two
>    source-unconfirmed merges (see "II"), finally relied on reflog reverse-deduction and withdrew
>    one wrong attribution. Verifiable form = members don't run `git push`/`fetch`/`pull`, sync
>    uniformly `pull --rebase`, use reflog to verify.
> 3. **Infrastructure Unstable, Land patch First Then Change Person**: This batch total 5
>    unrecoverable failures (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background
>    sub-agent no-message exit); disposition convention = produced evidence and patches first land
>    in repository internal path (`target/yx-scratch/<batch>/`) then change person to continue, same
>    session consecutive two failures don't bet the third.
> 4. **True Criterion for Doc Submission is "Locally Really Build Once"**: Round 2 closure's doc
>    submission (`042c07860`) made Deploy Docs red in VitePress's Vue template compilation (angle
>    bracket placeholders like `<rev>` not in backticks parsed as elements, reported
>    `Element is missing end tag`); markdownlint can't catch this type, and pre-commit's
>    markdownlint hook in submission containing only docs is judged `no files to check` skipped
>    (measured twice). Fix submission `0c837e1b4` with local build exit=0 as evidence. This round's
>    lesson red→green run: Deploy Docs **39011946793** (red) → **39014696984** (green, see "VI").
>
> **VI. Round 2 Gate Conclusion Supplementary Registration (13th Item, Scheduled Check Link
> Backfill)**: Round 2 closure submission `b940300ac`'s push—CI **39011946778** success / Auto
> Translate **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red,
> i.e., "V.4"'s angle bracket placeholders; already fixed by `0c837e1b4`); fix submission
> `0c837e1b4` after push three green: CI **39014697000** / Auto Translate **39014697044** / Deploy
> Docs **39014696984**. Consecutive push coverage relationship: CI@`0c837e1b4`'s **39014356642**
> cancelled by later run per concurrency (cancellation ≠ failure, later run covers previous one,
> consistent with round 2 §VI caliber).
>
> **VII. Final Gate (Closure Revision `e9675c9dd`, Local Number; CI Conclusion Handed to Scheduled
> Check, See "VIII")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR
> snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent (rerun
> `scripts/docs/gen-error-code-docs.py` after `git status` clean); corpus baseline 346 → 347 → 348
> rows (each +1 behavior record, other rows byte-by-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 rows).
>
> **VIII. Gate Check Arrangement (13th Item)**: This batch (including this article's doc submission)
> after-push CI / Auto Translate / Deploy Docs run conclusions **handed to scheduled check link for
> registration** (run id + conclusion + consecutive push coverage relationship), not waited
> in-session, and don't write "didn't wait" as "already green".
>
> **IX. Batch Tree State (Closure gc, status Output)**: Before closure `scratch.py status`—t25
> (holder=-) and yx49b (holder=-, previously held by discontinued member verify-r3 stale, team
> leader `release --force` released) two trees both have no holders; `gc` deleted both, freed about
> 12.7 GB; after gc `status` = "no batch tree", `git worktree list` only main mount remains.
> Another: `target/yx-scratch/yx49r3/` and `yx49r4/` are not batch trees (no meta.json), are
> directories where members put evidence/probes, don't belong to "residual batch tree". No new
> directories outside the repository in this batch.
