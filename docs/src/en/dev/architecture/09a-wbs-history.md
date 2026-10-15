# WBS History Notes (Implementation Log · Audit · Decision Record)

> This document is the **history notes** for [09-execution-wbs.md](./09-execution-wbs.md) (execution
> task table): implementation logs, audit findings, decision records, and errata for each phase.
> **Division of labor**: 09 only describes the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document only records **history** (what was done, what was
> found, how decisions were made), appended chronologically without rewriting history.

## P2 Establish Equivalence Criterion Baseline

> **P2 implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green.
> **D38 prediction fulfilled**—the green run actually measured and fixed three real defects in
> ir_gen (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime relies on Frame::set_slot for tolerance), unannotated parameters dropped
> by filter_map (three places, params metadata misaligned with frame layout), match result slot
> relied on frame's implicit Void initialization (now explicitly pre-initialized). The actual
> semantics of jump target is the flattened instruction index (not the label-keyed text in 07), CFG
> is built at instruction level—recorded in verify.rs file header. **Commit comparison**: ir_gen fix
> = `a9ea199b`; verify main body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its
> message to be mislabeled as "docs(rfc): RFC-027a…", content actually verify.rs + six test files,
> buried in history middle and unamendable, hereby recorded).
>
> **P2 narrowing partial implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> bleeding-channel preconditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpus (single point on the library side), binary location/subprocess invocation extracted to
>   `tests/common/` with yx_runner migrated in sync (rule three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses the `SumUpTo(3, r)`
>   form nailed down by rfc027 tests (return position/call site/binding position)—the `Sorted(3)`
>   annotation example in 02 would be substituted and evaluated in current code, directly Disproved,
>   not presenting Unproven silent; the Sorted(3) in 07 pseudo-code is only illustrative. Red state
>   measurement data see test file header.
> - Two red criteria are permanently marked with `#[ignore]` per the 2.4.5 red skeleton precedent:
>   default suite all green, `-- --ignored` reproduces red state; **removed when P3 fix lands,
>   turning green**.
> - The reason for the intentional deviation from 07 (code, file, line) table in comparison
>   normalization (excluding file/line, only comparing error code sets) is written in 2.4.1 test
>   comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, P2
> cannot compile. The former is P4's 4.1.4 (no double registration), the latter's `#[ignore]` red
> skeleton is merged into 4.3.1.
>
> **2.3.3 implementation (2026-10-07)**: C4 mandatory coverage list item-by-item check (07 §143), 10
> categories of semantic corpus layer fully covered and all within corpus-parity.jsonl differential
> baseline (26 representative files 26/26 hit, grep verified)—ref/borrow/move 05-ownership full
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum type
> sum_type_* + pattern_or_guard + match_call_scrutinee, existential coercion point
> interface_dynamic_dispatch (Vec(Animal) existential heterogeneous wrapping triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (+
> method_overload_ambiguous_err in diagnostic differential), refinement constraint Drop sequence
> refined_* ×5 + 05-ownership driven ReleasePlan (span key contract also has 2.4.4 red criterion
> permanent). Performance baseline `benches/pipeline.rs` merged (07 §145 three smokes, two of them):
> `corpus_compile_frontend` full corpus compile first measurement 17.6s, `cli_cold_start_hello`
> process cold start 169.7ms; interpreter throughput is covered by benches/lib.rs hotpath group
> (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate construction. Regression gate
> check-perf-regression.sh goes to P9 unified list (2.5).

## P3 Correctness Bug Fixes (Minimal Solution)

> **Retrospective error type labels for landed rows** (retrospective labeling, not modifying
> history): 3.1 = B | 3.2 = B (defensive empty arm, criterion green conversion borne by 3.1) | 3.3 =
> A | 3.4.1 / 3.4.2 = A (new W codes) | 3.4.3 = equivalent refactor (manual review) | 3.4.6 / 3.4.7
> = D (user-visible, already user adjudicated) | 3.4.8 ①② = C (main) + A (incidental three places) |
> 3.5 / 3.6 / 3.7 / 3.8 = D (all user adjudicated).

> **P3 diagnosis (2026-10-12, user requested)**: A diagnostic accounting in response to the
> perception that "P3 entries keep growing".
>
> **First, clarify the scale (perception amplified by two accounting defects)**
>
> 1. **Numbering reuse**: `3.5.x` is shared by two task families—(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 tertiary: lexical and parser branches / AST `is_pub` / checker side
>    / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10 registered **3.5 higher-kinded type
>    parameter call path completion** (3 tertiary). Both share `3.5.x` ⇒ cross-reference ambiguity
>    (**2026-10-12 already renumbered per user ruling: 029g family changed to `029g.x`**), and makes
>    029g's rows be miscounted into P3. **Pending ruling**: change the 029g family to `029g.x` (or
>    move into its belonging phase), other references in document synchronized.
> 2. **Missed check**: 3.7's 4 tertiary (3.7.1–3.7.4) are all completed (d922a551 / 38bd4b9e) but
>    not checked ⇒ this entry corrected this time.
>
> **Two, P3 actual accounting (12 secondary rows)**
>
> | Row                                                                 | Status                                                                                                                                                                             | Disposition suggestion                                                                                                    |
> | ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                     | Completed                                                                                                                                                                          | —                                                                                                                         |
> | 3.4 silent discard point supplement (8 tertiary)                    | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlock with 4.3.1                                                                  |
> | 3.5 higher-kinded type parameter call path completion (3 tertiary)  | Not started (declaration side live, call side broken)                                                                                                                              | Keep P3; **does not block refactor mainline**, but is "accepted feature unimplemented", priority higher than pure cleanup |
> | 3.6 function type value call path (1 tertiary)                      | Not started (semantics cut (i))                                                                                                                                                    | **Keep**: correctness defect, clear criterion                                                                             |
> | 3.7 init segment call name qualification (4 tertiary)               | **All completed**                                                                                                                                                                  | This time check and close                                                                                                 |
> | 3.8 native shadowing (1 tertiary)                                   | Not started                                                                                                                                                                        | **Stuck on decision** (first determine name resolution priority), not a scheduling problem                                |
> | 3.9 / 3.10 / 3.11                                                   | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                         |
> | 3.12 executor config propagation between runtime modes (1 tertiary) | Not started (D class, user ruling B: separate team)                                                                                                                                | **Suggest move out of P3 → P8 (execution layer)**, pending ruling                                                         |
>
> **Four, 4.3.1's leverage (mainline mark)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the precondition for **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ take it once to advance mainline and clear two P3 stranded items. 3.5 (HKT), 3.6
> (function type value call) and 4.6.1/4.9.1 all have **no unmet preconditions**, can be done
> anytime; 3.8 stuck on semantic decision (user definition priority vs explicit diagnostic).
>
> **Three, mechanism suggestion (prevent recurrence)**: newly registered secondary/tertiary rows
> **must carry two fields**—"belonging phase" (which P it truly belongs to) and "whether blocks
> mainline"; `registered pending scheduling` no longer as default state, unlabeled considered
> unschedulable. This way expansion is visible, sortable, not indiscriminately piled up. **Pending
> ruling**: whether to write this into this document's registration convention and
> `src/dev/coding-rules.md`.

> **P3 implementation (2026-10-06)**: 3.1.1 `proof_calls` narrow visibility + sole getter (recorded
> as `pub(crate)` at the time; **2026-10-07 verified actual landing is `pub(super)`**—the `#434`
> ruling referenced by code comments has no record on docs side, see 3.4.3; full private requires
> rewriting 11 places of construction literals, not taken in bleeding phase; P4 Obligations
> migration will close); execution mechanism extracted to `src/frontend/proof_execution.rs` (rule
> three single point—cannot put in typecheck/, that would create L3→L4 reverse dependency); 3.1.2
> according to 02 evidence chain step 5 supplemented **four entry points** (WBS row number missed
> compile_project `:99`, must supplement and register per 3.1.1 acceptance "criterion green
> conversion"); 3.2.1 empty arm change to accounting (proof_calls throw) + diagnostic (`into_result`
> established conversion path); 3.3.1 SOLVER slot Option-ized, Z3 missing per `SMTResult::Unknown`
> conservative degradation (E2031 family diagnostic replaces process crash). Two bug criteria remove
> `#[ignore]` to normal green, add corpus fixture `proof-obligation-honored`; check-fanout inventory
> migrated accordingly (connection point 19=inventory). Measurement: criteria 2.4.1/2.4.3 green,
> full suite lib 2494 / integration 272 / corpus double-layer all green.

> **3.4 audit (2026-10-07, P3 post-implementation recheck)**: targeting the type of defect "failure
> produces no signal" (definition see 02 §41-52), re-inventoried on unmodified code, located 8 more
> places, all registered as 3.4.x above. Method and evidence:
>
> - **Consumption-side inventory** (16 obligation fields grep each repository-wide read point):
>   `variant_ctor_calls` **zero consumers**—the only read `checker.rs:1468` is "self-poured into
>   `TypeCheckResult`", IR side takes form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts producer side, exactly the 02 §186 criticized "only test producer side, naturally immune
>   to consumer-side defects" form → 3.4.4. `method_overload_ir_names` result-level field similarly
>   zero external consumers, IR uses parallel `overload_resolutions` → 3.4.5.
> - **Phase coverage recheck**: 02 §91 table row 1 (four entry points "no proof_execution") fixed by
>   3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 state unchanged (multi-file `run` still does
>   not output W1003, does not run dead code family) → 3.4.6; LSP non-project-internal path (02 §77
>   "manual lex→parse") similarly does not run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` whole-repository consumers only four places): checker
>   three refinement branches (covered by 3.1), 3.2.1 fixed ownership arm (**source unreachable**),
>   and `environment.rs:422` (**reachable, zero diagnostic, zero accounting, and whole repository
>   docs zero record**) → 3.4.1, the only "all-unknown" item in this audit.
> - **Solver acquisition side**: `default_solver()` three consumption forms differ
>   (`predicate.rs:32` process singleton / `checker.rs:1266` per `check_module` injection /
>   `ownership.rs:621` per back-edge judgment newly created)—3.3.1 only eliminated one panic; the
>   other two's **silent side** goes to 3.4.2, **singleton-ization and cache hit** goes to P4's
>   4.4.2 (no double registration).
> - **Not counted in 3.4's existing attribution**: 2.4.4 (`ReleasePlan` span key mismatch, 168 files
>   / 373 keys not consumed, Drop silently lost) still P2 red state permanent, attribution 4.5.1/D20
>   unchanged; 4.4.1 (layer sequence declaration vs actual reversed, `equivalence` layer not in
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4 roster.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 no P4 preconditions, can land in place anytime
> (suggest doing 3.4.1 first—only "whole-repository zero record" living bug, smallest change
> surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on obligation ledger (4.3.1) or
> unified Driver (4.2.x). **3.4 does not establish new criterion system**, uniformly reuse existing
> C2/C3 and P9's 9.2 gate.

> **Batch 3.13–3.16 record (2026-10-13 registered → 2026-10-15 closed)**: user 2026-10-14 ruled to
> merge four relevance small items (#451 method binding panic / #452 named actual argument into
> resolution / #453 dead inference entry / #454 native export named actual argument—last item is
> boundary discovery when verifying 3.14, same day via attribution experiment verified **not
> introduced by #452**, user ruling ① merged into this batch) into one team for handling. Closing
> state: all four completed, ruling registered as RFC-039 **D60** (six binding rules + shared
> criterion single-pointing).
>
> **One, two contract corrections (lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradictory**: #452 task contract simultaneously required "mixed
>    actual argument output 20" and "position and name compete for same position report error", but
>    the nailed fixture `Temp.pick(self, a)` only has one value parameter—the same actual argument
>    combination is both "mixed" and "conflict". First implementer honestly reported blocker and set
>    task failed, overall revert (not getting green for silent). Disposition: task reassign +
>    contract revision—given **six binding rules** and **8 value-level matrix** (fixture changed to
>    two value parameters `pick(self, n, a)`) attempt 2 completed by second implementer. Six rules
>    full text verbatim entered into D60.
> 2. **inScope missing mechanical requirement side**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed generator regeneration
>    `docs/src/reference/error-code/`**. E1110 after registration only ran `code-tables --fix`,
>    **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source code
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964**—this batch's only one CI red line). Fix = mechanical regeneration (8 files
>    +83/−70, only touching `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **Two, review → fix → re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls containing named actual arguments in multi-candidate
>    zero-hit/single-candidate hit two paths whole chain silent) → fix round added outward code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-set same
>    commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared resolution entry + native arm settle by name) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations via
>    `scratch.py trial` mechanical reversible replay all caught, restoration byte-identical) →
>    round-1 review needs_revision (F1 = typecheck native arm **inline copied** a binding criterion,
>    missing rule ④ ⇒ method and native same-shape call **ruling divergence**—method reports E1111,
>    native silent exit=0; ir_gen additionally has third existing copy with old code E1015) →
>    convergence round collapsed criterion to single implementation `bind_args_to_params`
>    (`arg_binding.rs:35`, four call points `expressions.rs:2727` / `:5416`, `ir_gen.rs:7776` /
>    `:8279`) → round-2 review caught regression introduced by convergence round (IR convergence
>    block lost native omittable position ⇒ originally successful program with omittable optional
>    position was falsely reported E1010) → fix round (optional position criterion and parameter
>    name same exit `signature.rs:236` sole exit) → round-3 review PASS (independent reproduction
>    red→green; mutation under repository nailed FAILED empirical criterion can fail). **Lesson**:
>    review must cover "unmodified branches of same-type path" (this batch's two needs_revision both
>    because probes all fell on hit side); fix round itself can also introduce regression (omittable
>    position), caught by next round's personnel change review.
>
> **Three, process events (truthfully recorded, not pointing at individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: working tree was silently restored
>    externally—implementer's fix in-progress (typecheck/ir_gen/code table/locales/RFC-013 etc.) all
>    disappeared, **at same moment user's two review documents also disappeared from git status** ⇒
>    determined to be repository-level discard (most likely user-side manual cleanup/IDE "discard
>    all changes"). Three evidence excluding group members: stash empty, no new commit, reflog no
>    reset. Implementer replayed in-progress work from context and pushed; discipline changed to
>    "**first save patch to scratch, then capture red state**".
> 2. **Verifier context exhausted personnel change**: independent verifier verifying 3.16 ran out of
>    room mid-way (no closing message), per load rule changed personnel—successor continued using
>    ready products (green state result, corpus equivalence script, mutation plan), verification
>    ultimately PASS. Positive precedent for partial product handoff.
> 3. **Review round 5 rotation and historical rounds retreat**: when rotating reviewer per load
>    upper limit rule ⑤, `remove_member` returned **together** already adjudicated terminal
>    review/verification rounds, scheduler immediately re-ran (t3/t8 taken by new members again,
>    duplicate work and block downstream). Disposition: team lead took over and cancelled retreated
>    historical rounds (t3/t8/t13), accordingly added AGENTS.md rule 12 (immediately dispose
>    retreated rounds after personnel change). This batch closing record registers this event per
>    that rule.
> 4. **scratch governance (about 6.3 GB)**: batch early period one external tree per task, scattered
>    6 directories outside repository (about 780 MB, cleaned) + two members' external verification
>    trees (about 2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md rule 9
>    (batch tree sole entry), rule 10 (verification environment six rules: mechanical reversible
>    mutation / line number take referenced revision / CI unique gate must run etc.), rule 11
>    (writer-side load upper limit), rule 12 (dispose retreated rounds after personnel change) and
>    top meta-rules two (no private content / verify before assert).
> 5. **Contract defect (not pointing at members)**: #454 fix round's contract inScope only circled 2
>    files, actual mechanical requirement 8 (rule 6 is exactly to prevent this—contract template
>    should preset mechanical requirement list); additionally that task's terminal field was once
>    occupied by probe payload, formal acceptance/command evidence supplemented by team lead per
>    spec.
>
> **Four, leftover items (not fixed this batch, registered for future reference)**: round-3 review
> three low severity findings—① method path's `build_arg_slots` (`ir_gen.rs:521` area) is still
> criterion's third implementation (existing, per last round's convention register with closing, not
> converge); ② batch verification tree (worktree) `cargo fmt --all` error belongs to worktree
> workspace artifact, gate list should note fmt goes to main repository; ③ repair type contract
> template should preset mechanical requirement into inScope (see 3.5).
>
> **Five, final gate (closing revision, local two implementation commits + one documentation commit
> unified push—this batch's only one push)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 /
> corpus_probe **342 rows line-by-line consistent with baseline** (only regenerated row:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number synchronized,
> manually reviewed) / IR snapshot zero drift.
>
> **Six, run link (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test cancelled by subsequent push
> per concurrency—cancellation≠failure, covered by descendant run); governance tool push → CI
> 38546373435 / 38561405117; WBS registration (3.16 row + header 56→57 / 147→148) → CI 38528560364.
> Closing push's own CI / Deploy Docs run task record supplemented after push.

> **3.4.8 ③④ implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail add `Linking` (**pure
> merge**, whole block skip `is_bin_role` entry check—E3020 criterion still at `RoleClassification`
> to prevent double report; also hard reason: run path judges "has main" per merged IR, collection
> semantics "entry itself error→no IR" would be misjudged as missing main and hard abort) →
> `Monomorphization` (check form); `ir_generation_check` keep unit IR (**memory only**, for pure
> merge and mono consumption, no outward/no disk/no snapshot—not kept then aggregation request
> empty, mono with `NoObligations` fake green); mono failure in Check takes per-file collection
> channel (diagnostic to entry file + `failed_stages`, not set failure), MultiFile keeps
> `fail_with`; `into_outcome` changed to "first take collection then judge failure". ④ empirically
> **zero production change**: whole program BFS already covered cross-unit mutual recursion
> (pathological → E3005, well-formed three-layer chain execution produces correct value), deliver
> four groups e2e + one mono unit test as living criterion.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixture + with
> HEAD binary byte-by-byte cross), t4 (④, 10 fixture + `max_depth` parameterized boundary), t10 (F1
> fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix (single/multi/check three
> forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review PASS; test count 2643 → 2656
> (only increase not decrease).
>
> **F1 (found in verification period, D59 A/B class, same round fix)**: without project root
> standalone multi-file check, aggregation request name (`lib.f`) and unit IR definition key (`f`)
> different sources → mono falls "native no IR definition" silent `continue`, check reports 0
> error/exit 0. Fix: aggregation side adds per unit **real definition key** reverse lookup jump
> (precise enumeration + character-equal + ambiguity abandon, not suffix heuristic; project form not
> registered, existing path unchanged). **Errata (same day)**: that reverse lookup jump has been
> **deleted** in subsequent regression fix; root cause changed to be solved by `module_key`
> dual-duty decoupling (add `script_mode`) from naming perspective—IR definition key = IR call name
> = aggregation request name = `{unit key}.{name}`. **Regression record**: ③ after landing CI's
> `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6 same-file same-name
> rebinding + 4 collide with `<std/list>`, rootless check no qualification triggers
> `link_module_irs` conflict check); fix = ir_gen split `script_mode` (qualified name ⊥ Script/Bin,
> E3023 gate changed to look at it) + `ir_generation_check` per unit always pass unit key +
> `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter takes effect"
> (consistent with run path existing behavior); verification = document gate 258/0, three binary
> comparison (HEAD/③ tree/fix tree) three types of collision fixture, check form E3005 no
> regression, E3023 not triggered.
>
> **F2/F3 (accepted)**: mono diagnostic to entry file entry, span can cross unit (same as run path
> `path=entry`); Check form holds one ModuleIR per unit (memory only, released when check ends).
>
> **Left for subsequent existing issues (not this round's regression)**: ① E3005's help text still
> writes "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved same type "user writing error" out of E3005)—registered
> pending scheduling; ② multi-file E3005 span rendering label misalignment (reproduced with HEAD
> binary, pre-existing); ③ request collection side has **type parameter position dependency** (type
> parameter not in value parameter position generic call does not send instantiation request, mono
> records `NoObligations`, convergence mutual recursion is this form)—same face as 3.5 (HKT call
> path), registered to that row.

## External Track RFC-029g: Delete `pub` and Auto-binding (#399

> **P3.5 implementation (2026-10-08)**: seven secondary tasks all completed, whole batch one commit
> (independently revertible).
>
> - **Measurement inconsistent with plan (truthfully recorded)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—actually after deleting `KwPub` `pub`
>   falls back to ordinary identifier, old writing falls to semantic layer reports **E1001 Unknown
>   variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically probing removed
>   syntax is anti-pattern", **no new** migration-specific diagnostic; corpus fixture
>   `pub_declaration_err.yx` nails E1001, document (`syntax.md`) truthfully writes that consequence.
> - **Design convergence (beyond RFC literal, requires review)**: `project_refs` and
>   `cross_file_refs` two fields after pub deletion semantically fully coincide (same "in-package
>   reference union", rule one prohibits parallel representation) → merge to single `project_refs`,
>   `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally Internal role's
>   non-pub item from "in-file reachability" changed to "reference pool judgment" (relaxed, rather
>   false negative direction); Lib from "pub absolute exemption" tightened to reference pool
>   judgment; Script unreferenced top-level binding starts reporting W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs` exports write-back all deleted (producer-side pour-in,
>   whole repository zero consumption).
> - **Test migration** (test count not decreasing): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs` "pub recognized" assertion reversed to "pub is no longer
>   modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (assertion reverse 2);
>   `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption assertion
>   reversed to report assertion).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not involved in IR); corpus differential baseline
>   only adds 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated and manually reviewed.
> - **Measurement**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-layer Contract PlanId-ization (D20)

> **4.5 implementation record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two feature commits, closing
> (registration / WBS check / gate / push) done by t7. Precondition = 4.3.1 obligation ledger (red
> state archived 173 files / 533 keys).
>
> **One, carrier ruling (A: `PlanId` follows AST node)**: added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compile unit, `UNPLANNED = u32::MAX` represents synthetic node
> outside parser), issued by **sole allocator** `ParserState::alloc_plan_id`; carrier two places—
> `Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call site identity), `Expr::call` is
> whole repository sole call constructor. Vetoed is "two sides each derive same identity per span"
> position coincidence form (D20's original defect).
>
> **Two, two-level verification and review (member independent)**: each feature commit each with one
> independent verifier and one reviewer, **verifier/reviewer do not reuse implementer evidence**:
> 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations) / t3 review;
> 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations) / t6 review.
> Review time point and reviewed revision both fall on immutable history (`git show` verifiable).
> **Provider rename caused t5 / t6 personnel change continuation**: original verifier/reviewer
> members not continuable due to provider rename, per orchestration convention changed personnel and
> **write reason in record** (cross-round context retention priority, compression only when
> unavailable); after continuation conclusion same as original round (PASS), no implementer evidence
> reused.
>
> **Three, two defects found and fixed by implementation self-check (t1, both in inScope)**: ①
> consumption exemption criterion once hung on `UnitState.ir.is_some()`— `check` form's Linking/Mono
> takes `unit.ir` away, real unconsumed in check form **silently passes** (mutation measurement: D41
> red while CLI check exit 0); changed to read **ledger's own completion marker**, and added
> regression case. ② Drop sequence drifts with process hash seed (partial order `sort_by` +
> `HashSet` iteration): snapshot criterion three consecutive runs produced 106/120/114 line drift;
> changed to **total order sorting** (same name take max depth + name ascending) then two
> consecutive runs zero drift.
>
> **Four, criterion and evidence numbers**: 4.5.1 after `test_release_plan_spans_consumed` remove
> `#[ignore]` permanent (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated same
> commit (programmatic review: added 1083 lines Drop + 86 lines jump index shift, zero delete, no
> reorder), corpus 342 differential empty; 4.5.2 added `test_overload_resolutions_consumed`
> (difference set empty + evidence side floor `examined ≥ 1` / `consumed ≥ 2`; measured examined=1,
> consumed entries 2, no resolution skip 210), resolution form 9 nailed, `cargo test --lib` 2712
> passed / 0 failed, IR snapshot **zero drift**. Mutation drive: consumption key shift one position
> ⇒ criterion red + real program `error [W1090] ... produced key not consumed` + corpus gate
> difference; after restoration re-green.
>
> **Five, semantic modification classification basis (D59)**:
> `Type.method(position actual argument, name = value)` before change **silently gave wrong answer**
> (measured compile clean, run output `1` not `20`) ⇒ **C class** (old behavior not success), fix
> takes "loud": after change same program compile time `error [W1090]`; "named actual argument
> participates in resolution" terminal semantics is **D class**, already registered §3.14 and left
> issue #452 for specialized team. Additionally: `Drop` in interpreter is no-op
> (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`)—key migration does
> not change run behavior.
>
> **Six, F-1 / F-2 disposition (t5 independent verification found)**: **F-1 (medium)** no binding
> consumption point (`ir_gen.rs:7766-7790`) in current corpus and test side unreachable, key
> correctness no red state falsifiable ⇒ per "known boundary" register in [02](02-stage-contract.md)
> §2 revision annotation fifth paragraph (keep reason + no new task); **F-2 (low)** pointer drift ⇒
> `types.rs`'s `OverloadResolutions` documentation comment changed to precise pointer (produce
> `inference/expressions.rs:4550`, consume `ir_gen.rs:7697` / `:7772`), separate small commit, not
> mixed into documentation commit.
>
> **Seven, closing (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settle marker); 07
> criterion table and 04-ssa defect class 3 changed to past tense; 09 check 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG record one line behavior change; `check-obligations.py` and `check_tracking.py` green,
> CI and Deploy Docs green. **Accounting deviation found in closing**: t3 / t6 report's pointer
> `ir_gen.rs:357` / `:3102` / `:1096` differs from code fact by one to three lines (actually `:360`
> / `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion)—register per code fact, this annotation also
> leaves trace.
