# WBS Historical Notes (Landing Records · Audits · Ruling Traces)

> This document is the **historical notes** of [09-execution-wbs.md](./09-execution-wbs.md)
> (Construction Task Table): landing records, audit findings, ruling traces and errata for each
> phase. **Division of labor**: 09 only describes the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document only records the **history** (what was done, what
> was found, how rulings were made), appended chronologically without rewriting history.

## P2 Establish Equivalence Criterion Baseline

> **P2 landed (2026-10-06, 2.1)**: verify.rs dual mode + 7 invariants + full corpus runs green.
> **D38 prediction fulfilled** — the run-green process actually measured and fixed three real
> defects in ir_gen (see 06 §G6 for details): method locals table truncated to parameter count
> (underestimated local_count, runtime tolerates via Frame::set_slot), unannotated parameters
> dropped by filter_map (three places, params metadata and frame layout misaligned), match result
> slot relies on frame implicit Void initialization (now explicitly pre-initialized). The actual
> semantic of jump target is flattened instruction index (not the label-keyed text in 07), CFG is
> built at instruction level — recorded in verify.rs file header. **Commit cross-reference**: ir_gen
> fix = `a9ea199b`; verify body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its
> message to be mislabeled as "docs(rfc): RFC-027a…", content is actually verify.rs + 6 test files,
> already buried in history middle and cannot be amended, recorded here).
>
> **P2 narrow part landed (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3 stop-bleeding
> channel preconditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpus (library-side single point), binary location/subprocess spawn extracted to
>   `tests/common/` and yx_runner migrated in sync (Rule 3).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses the `SumUpTo(3, r)`
>   form nailed by rfc027 test (return position/call site/binding position) — the `Sorted(3)`
>   annotation example in 02 would be substituted for evaluation in current code and directly
>   Disproved, not presenting Unproven silently, the Sorted(3) in 07 pseudo-code is only
>   illustrative. Red-state measured data is in the test file header.
> - The two red criteria are attached with `#[ignore]` per the 2.4.5 red skeleton precedent: default
>   suite all green, `-- --ignored` reproduces red state; **removing the attribute when P3 fix lands
>   will turn it green**.
> - The reason for the intentional deviation from 07's (code, file, line) table in comparison
>   normalization (remove file/line, only compare error code set) is written in the 2.4.1 test
>   comment.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, P2
> cannot compile. The former is P4's 4.1.4 (avoid double registration), the latter's `#[ignore]` red
> skeleton is merged into 4.3.1.
>
> **2.3.3 landed (2026-10-07)**: C4 must-cover checklist verified item-by-item (07 §143), 10
> categories of semantic corpus layer all covered and within the corpus-parity.jsonl diff baseline
> (26 representative files 26/26 hit, grep verified) — ref/borrow/move 05-ownership full directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closure capture
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum types sum_type_* +
> pattern_or_guard + match_call_scrutinee, existential coercion point interface_dynamic_dispatch
> (Vec(Animal) existential type heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overloading method_overload (+ method_overload_ambiguous_err on diagnostic
> diff surface), refinement constraint Drop sequence refined_* ×5 + 05-ownership-driven ReleasePlan
> (span key contract has 2.4.4 red criterion standing). Performance baseline `benches/pipeline.rs`
> checked in (07 §145 three smokes, two of them): `corpus_compile_frontend` full corpus compilation
> first measurement 17.6s, `cli_cold_start_hello` process cold start 169.7ms; interpreter throughput
> is already covered by benches/lib.rs hotpath group (interp_fib_recursive_27 /
> interp_loop_steady_10m), no duplicate construction. Regression gate check-perf-regression.sh
> belongs to P9 unified checklist (2.5).

## P3 Fix Correctness Bugs (Minimum Plan)

> **Error type annotation for landed rows** (retrospective annotation, not changing history): 3.1 =
> B | 3.2 = B (defensive empty arm, criterion turning green is borne by 3.1) | 3.3 = A | 3.4.1 /
> 3.4.2 = A (new W code) | 3.4.3 = equivalent refactor (manual review) | 3.4.6 / 3.4.7 = D
> (user-visible surface, already user-ruled) | 3.4.8 ①② = C (main) + A (three incidental places) |
> 3.5 / 3.6 / 3.7 / 3.8 = D (all user-ruled).
>
> **P3 diagnosis (2026-10-12, user request)**: In response to the perception of "P3 items continuing
> to grow", conduct an account triage.
>
> **I. First clarify scale (perception is amplified by two account defects)**
>
> 1. **Numbering collision**: `3.5.x` is shared by two task families — (a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 third-level items: lexer and parser branches / AST `is_pub` /
>    checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) 2026-10-10 registered **3.5 higher-kinded
>    type parameter call path completion** (3 third-level items). The two share `3.5.x` ⇒
>    cross-reference ambiguity (**2026-10-12 renumbered per user ruling: 029g family changed to
>    `029g.x`**), and caused 029g's rows to be mistakenly counted into P3. **Pending ruling**:
>    renumber the 029g family to `029g.x` (or move it into its proper phase), other references in
>    the document synced.
> 2. **Unchecked items**: `3.7`'s 4 third-level items (3.7.1–3.7.4) are all completed (d922a551 /
>    38bd4b9e) but not checked ⇒ this item is corrected in this round.
>
> **II. P3 actual account (12 second-level rows)**
>
> | Row                                                                    | Status                                                                                                                                                                             | Disposition Suggestion                                                                                                             |
> | ---------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                        | Completed                                                                                                                                                                          | —                                                                                                                                  |
> | 3.4 Silent discard point supplement (8 third-level)                    | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **doable now**; 3.4.4/3.4.5 unlock with 4.3.1                                                                                |
> | 3.5 Higher-kinded type parameter call path completion (3 third-level)  | Not started (declaration side live, call side broken)                                                                                                                              | Keep P3; **does not block refactor main line**, but is "accepted feature not implemented", priority higher than pure cleanup items |
> | 3.6 Function type value call path (1 third-level)                      | Not started (semantics already cut (i))                                                                                                                                            | **Keep**: correctness defect, criterion clear                                                                                      |
> | 3.7 init segment call name qualification (4 third-level)               | **All completed**                                                                                                                                                                  | Check and close this time                                                                                                          |
> | 3.8 native shadowing (1 third-level)                                   | Not started                                                                                                                                                                        | **Stuck on decision** (first determine name resolution priority), not scheduling issue                                             |
> | 3.9 / 3.10 / 3.11                                                      | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                                  |
> | 3.12 Executor config propagation between runtime modes (1 third-level) | Not started (Class D, user ruling B: separate team)                                                                                                                                | **Suggest move out of P3 → P8 (execution layer)**, pending ruling                                                                  |
>
> **IV. The lever of 4.3.1 (main line marker)**: `4.3.1 obligations.rs + assert_drained()` is also
> the precondition for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract PlanId-ization) ⇒
> taking it once advances the main line and clears the two P3 stranded items. 3.5 (HKT), 3.6
> (function type value call) and 4.6.1/4.9.1 all have **no unmet preconditions**, are doable at any
> time; 3.8 is stuck on semantic decision (user-defined priority vs explicit diagnostics).
>
> **III. Mechanism suggestion (prevent recurrence)**: Newly registered second/third-level rows
> **must include two fields** — "belonging phase" (which P it really belongs to) and "whether it
> blocks the main line"; `registered pending scheduling` is no longer the default state, unannotated
> is deemed unschedulable. This way the bloat is visible and sortable, rather than undifferentiated
> accumulation. **Pending ruling**: whether to write this into the registration convention of this
> document and `src/dev/coding-rules.md`.
>
> **P3 landed (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter (was recorded
> as `pub(crate)`; **2026-10-07 verified the actual landing is `pub(super)`** — the `#434` ruling
> referenced in the code comment has no registration on docs side, see 3.4.3; full private requires
> rewriting 11 constructor literals, not taken in the stop-bleeding phase; closing when P4
> Obligations migration); execution mechanism extracted to `src/frontend/proof_execution.rs` (Rule 3
> single point — cannot be put in typecheck/, that would create L3→L4 reverse dependency); 3.1.2
> fills the **four entries** per 02 evidence chain step 5 (WBS line number missed compile_project
> `:99`, must be filled and registered per 3.1.1 acceptance "criterion turning green"); 3.2.1 empty
> arm changed to bookkeeping (proof_calls throws up) + diagnostics (`into_result` established
> conversion path); 3.3.1 SOLVER slot Option-ized, Z3 missing falls back conservatively per
> `SMTResult::Unknown` (E2031 family diagnostics replaces process crash). Two bug criteria remove
> `#[ignore]` to turn normal green, supplement corpus fixture `proof-obligation-honored`;
> check-fanout inventory migrates accordingly (connection point 19=inventory). Measured: criteria
> 2.4.1/2.4.3 turn green, full suite lib 2494 / integration 272 / corpus dual layer all green.
>
> **3.4 audit (2026-10-07, review after P3 landing)**: For this class of defects "failure produces
> no signal" (definition in 02 §41-52), re-inventoried on unchanged code, found 8 more places, all
> registered as 3.4.x in the table above. Methods and evidence:
>
> - **Consumer-side inventory** (grep all warehouse read points for 16 obligation fields one by
>   one): `variant_ctor_calls` **zero consumers** — the only read `checker.rs:1468` is "self-pour
>   into `TypeCheckResult`", IR side goes through form detection `ir_gen.rs:7374`;
>   `rfc011b.rs:659/688` only asserts the production side, exactly the form criticized in 02 §186
>   "only test production side, naturally immune to consumer-side defects" → 3.4.4.
>   `method_overload_ir_names` result-level field also has zero external consumers, IR uses parallel
>   `overload_resolutions` → 3.4.5.
> - **Phase coverage review**: Row 1 of 02 §91 table (four entries "no proof_execution") has been
>   fixed by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 status unchanged (multi-file `run` still
>   does not output W1003, does not run dead code family) → 3.4.6; LSP non-project-internal path (02
>   §77 "manual lex→parse") also does not run proof → 3.4.7.
> - **Same-form empty arm scan** (`Unproven` only has 4 warehouse consumers): checker three
>   refinement branches (3.1 already covered), ownership arm fixed by 3.2.1 (**source
>   unreachable**), and `environment.rs:422` (**reachable, zero diagnostics, zero bookkeeping, and
>   zero registration in warehouse docs**) → 3.4.1, the only "unknown to all" item in this audit.
> - **Solver acquisition surface**: `default_solver()` has three consumer forms in different states
>   (`predicate.rs:32` process singleton / `checker.rs:1266` injected per `check_module` /
>   `ownership.rs:621` recreated per back-edge determination) — 3.3.1 only eliminated the panic one;
>   the other two's **silent surface** belongs to 3.4.2, **singleton-ization and cache hit** belongs
>   to P4's 4.4.2 (avoid double registration).
> - **Existing attribution not counted in 3.4**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys unconsumed, Drop silently lost) is still P2 red-state standing, attribution
>   4.5.1/D20 unchanged; 4.4.1 (phase order declaration opposite to actual, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty bookkeeping downgrade) already on P4 books.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions, can land in place at any
> time (suggest doing 3.4.1 first — the only "warehouse zero registration" live bug, smallest change
> surface); 3.4.4–3.4.8 execute with P4, their criteria depend on obligation ledger (4.3.1) or
> unified Driver (4.2.x). **3.4 does not establish a new criterion system**, uniformly reuses
> existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 record (2026-10-13 registered → 2026-10-15 closed)**: User 2026-10-14 ruling
> merged four related small items (#451 method binding panic / #452 named argument into resolution /
> #453 dead inference entry / #454 native export named argument — the last item is a boundary
> finding when verifying 3.14, same day attribution experiment verified **not introduced by #452**,
> user ruling ① merged into this batch) into one team for processing. Close status: all four items
> completed, ruling registered as RFC-039 **D60** (six binding rules + shared criterion
> single-point).
>
> **I. Two contract corrections (lesson: the contract itself can also be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: #452 task contract simultaneously requires "mixed
>    argument output 20" and "positional and named competing for same position error", but the
>    pinned fixture `Temp.pick(self, a)` only has one value parameter — the same argument
>    combination is both "mixed" and "conflict". The first implementer truthfully reported blocker
>    and set task failed, overall revert (did not trade silence for green light). Disposition: task
>    reassigned + contract revised — given **six binding rules** and **8 value-level matrix**
>    (fixture changed to two value parameters `pick(self, n, a)`), attempt 2 was completed by the
>    second implementer. The full text of the six rules has been included verbatim in D60.
> 2. **inScope missing mechanical required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only ran `code-tables --fix`,
>    **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964** — the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touching `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **II. Review → fix → re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls containing named arguments in multi-candidate
>    zero-hit/single-candidate hit two paths whole-chain silence) → fix round added external code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece same
>    commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared resolution entry + native arm settled by name) →
>    independent verification PASS (12-cell value-level matrix, corpus 342/342, 5 mutations
>    mechanically reversibly replayed via `scratch.py trial` all caught, restoration byte-for-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inlined duplicated** a
>    copy of binding criterion, missing rule ④ ⇒ method and native same-form call **ruling
>    divergence** — method reports E1111, native silent exit=0; ir_gen has a third existing copy
>    reporting old code E1015) → convergence round collapsed criterion to single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call points `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught **regression** introduced by convergence
>    round (IR convergence block lost native omittable position ⇒ previously successful program
>    omitting optional position was misreported as E1010) → fix round (optional position criterion
>    and parameter name both from `signature.rs:236` single exit) → round-3 review PASS (independent
>    reproduction red→green; mutation-pinned boards in warehouse FAILED verified criterion can
>    fail). **Lesson**: review must cover "unfixed branches of similar paths" (this batch's two
>    needs_revision are both because probes all fell on hit surface); fix round itself can also
>    introduce regression (omittable position), caught by next round of person-swapping review.
>
> **III. Process events (truthfully recorded, not pointing at individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: The working tree was silently
>    restored externally — implementer's fix-in-progress (typecheck/ir_gen/code
>    table/locales/RFC-013 etc.) all disappeared, **at the same moment the two review documents the
>    user edited themselves also disappeared from git status** ⇒ judged as repo-level discard (most
>    likely user-side manual cleanup / IDE "discard all changes"). Three proofs excluded group
>    members: stash empty, no new commits, reflog has no reset. Implementer replayed fix-in-progress
>    from context and pushed; discipline changed to "**first save patch to scratch, then sample red
>    state**".
> 2. **Verification member ran out of context, person-swapped**: The independent verifier verifying
>    3.16 ran out of room midway (no closing message), person-swapped per load rules — successor
>    continued using their ready products (green state results, corpus equivalence script, mutation
>    plan), verification finally PASS. Positive precedent of partial product handover.
> 3. **Round 5 of review rotation and historical round retreat**: When rotating review per load
>    limit clause ⑤, `remove_member` returned the already-adjudicated final review/verification
>    rounds **together to the shared pool**, scheduler then re-ran (t3/t8 were retaken by new
>    members, duplicate work and blocked downstream). Disposition: team lead took over and cancelled
>    the returned historical rounds (t3/t8/t13), accordingly added AGENTS.md clause 12 (immediately
>    handle returned rounds after person swap). This batch's close-out record registers this event
>    per that clause.
> 4. **scratch governance (about 6.3 GB)**: In the early batch, one external tree per task, 6
>    directories scattered outside the repo (about 780 MB, cleaned) + two members' external
>    verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md clause 9
>    (batch tree sole entry), clause 10 (verification environment six rules: mechanically reversible
>    mutation / line numbers from referenced revision / CI-only gates must run etc.), clause 11
>    (writer load limit), clause 12 (handle returned rounds after person swap) and top meta-rules
>    two (no private content / verify before asserting).
> 5. **Contract defect (not pointing at members)**: #454 fix round's contract inScope only circled 2
>    files, actually 8 mechanical required surfaces (clause 6 is exactly to prevent this — contract
>    template should preset mechanical required surface list); also the task's final state field was
>    once occupied by probe payload, formal acceptance/command evidence was supplemented by team
>    lead per spec.
>
> **IV. Leftover items (not fixed in this batch, registered for reference)**: round-3 review three
> low-severity findings — ① the method path's `build_arg_slots` (`ir_gen.rs:521`) is still the
> criterion's **second copy** implementation (existing, per previous round's caliber registered with
> close-out, not converged) — **2026-10-16 pre-release audit supplementary three points**: (a) it
> has **two** call points: `ir_gen.rs:7888` (method binding path, using bound function
> `binding.function`'s parameter name) and `:7975` (overload resolution path, using resolved mixed
> name leaf segment), original registration only wrote "around `ir_gen.rs:521`"; (b) it has
> **diverged from shared implementation `bind_args_to_params`**: missing rule ⑤ (duplicate name →
> E1015), rule ⑥ (missing required parameter → E1010) and "optional position" concept, and hardcodes
> receiver position as `1 + i`; two call points `continue` on empty slot (comment says "diagnostics
> go to typecheck (it guarantees no empty slot after resolution passes)"), is a **dangling invariant
> written in comments, no assertion bottom line**; (c) **RFC-039 D60's "all call points converge to
> it" enumeration missed these two places**, is declaration inconsistent with fact, and cited line
> numbers not re-fetched on referenced revision, D60 text corrected accordingly with this item; ②
> batch verification tree (worktree) inside `cargo fmt --all` error is worktree workspace artifact,
> gate checklist should note fmt goes through main repo; ③ repair-type contract template should
> preset mechanical required surfaces into inScope (see III.5).
>
> **V. Final gate (close-out revision, local two implementation commits + one doc commit unified
> push — this batch's only push)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` **2730
> passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner` 2
> passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> lines line-by-line consistent with baseline** (only regenerated line: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line number synced, manually reviewed) / IR snapshot zero
> drift.
>
> **VI. run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test cancelled by subsequent push
> per concurrency — cancelled ≠ failed, covered by later run); governance tool push → CI 38546373435
> / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) → CI 38528560364.
> Close-out push's own CI / Deploy Docs run registered task record after push.
>
> **3.4.8 ③④ landed (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail adds `Linking` (**pure merge**,
> whole block skips `is_bin_role` entry check — E3020 criterion still belongs to
> `RoleClassification` to prevent double reporting; another hard reason: run path judges "has main"
> per merged IR, under collection semantics "entry itself errors → no IR" will be misjudged as
> missing main and hard abort) → `Monomorphization` (check form); `ir_generation_check` retains unit
> IR (**memory only**, for pure merge and mono consumption, no external send/no disk write/no
> snapshot — without retention, aggregation request is empty, mono falsely green with
> `NoObligations`); mono failure in Check goes per-file collection channel (diagnostic goes to entry
> file + `failed_stages`, not set failure), MultiFile keeps `fail_with`; `into_outcome` changed to
> "first take collection then judge fault". ④ verified **zero production change**: full program BFS
> already covers cross-unit mutual recursion (pathological → E3005, well-formed three-layer chain
> executes correct value), delivered four groups e2e + one mono unit test as live criterion.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixture groups +
> byte-for-byte cross with HEAD binary), t4 (④, 10 fixture groups + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase, no decrease).
>
> **F1 (found during verification, D59 A/B class, same-round fix)**: Under standalone multi-file
> check without project root, aggregation request name (`lib.f`) and unit IR definition key (`f`)
> are from different sources → mono falls into "native has no IR definition" silent `continue`,
> check reports 0 error/exit 0. Fix: aggregation side adds reverse query per each unit's **real
> definition key** (precise enumeration + byte-for-byte equality + ambiguity abandonment, not suffix
> heuristic; project form not registered, existing path unchanged). **Errata (same day)**: this
> reverse query has been **deleted** in the subsequent regression fix; root cause now resolved by
> `module_key` one-job-two-use decoupling (added `script_mode`) from the naming caliber — IR
> definition key = IR call name = aggregation request name = `{unit key}.{name}`. **Regression
> record**: after ③ landed, CI's `Check tutorial/guide code examples compile` red (258 blocks / 10
> failures: 6 items same-file same-name rebinding + 4 items colliding with `<std/list>`, rootless
> check doesn't do qualification triggering `link_module_irs` conflict check); fix = ir_gen split
> `script_mode` (qualified name ⊥ Script/Bin, E3023 gate changed to look at it) +
> `ir_generation_check` per unit always passes unit key + `.with_script_mode(rootless)` + Script
> top-level same-name rebinding "latter takes effect" (consistent with run path's existing
> behavior); verification = doc gate 258/0, three binary comparison (HEAD/③ tree/fix tree) three
> collision fixtures, check form E3005 no regression, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic goes to entry file item, span can cross units (same caliber
> as run path's `path=entry`); Check form holds one ModuleIR per unit (memory only, released when
> check ends).
>
> **Existing issues left for follow-up (not this round's regression)**: ① E3005's help text still
> writes "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of E3005) — registered
> pending scheduling; ② multi-file E3005's span rendering label misaligned (reproduced with HEAD
> binary, pre-existing); ③ request collection surface has **type parameter position dependency**
> (generic call where type parameter is not at value parameter position does not issue instantiation
> request, mono records `NoObligations`, convergent mutual recursion is this form) — same surface as
> 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Delete `pub` and Auto Binding (#399

> **P3.5 landed (2026-10-08)**: all seven second-level tasks completed, whole batch one commit
> (independently revertable).
>
> - **Actual measurement inconsistent with plan (truthfully registered)**: 029g.1.2/029g.1.3's
>   acceptance originally wrote "`pub f = ...` reports **parse error**" — actually after deleting
>   `KwPub`, `pub` falls back to ordinary identifier, old writing falls to semantic layer and
>   reports **E1001 Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27**
>   "specifically detecting removed syntax is anti-pattern", **do not add** migration-specific
>   diagnostic; corpus fixture `pub_declaration_err.yx` pins E1001, document (`syntax.md`)
>   truthfully states this consequence.
> - **Design convergence (beyond RFC literal, requires review)**: `project_refs` and
>   `cross_file_refs` two fields' semantics completely overlap after pub deletion (same
>   "package-internal reference union", Rule 1 prohibits parallel representation) → merged into
>   single `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted.
>   Behaviorally, Internal role's non-pub items change from "file-internal reachability" to
>   "reference pool determination" (loosened, prefer under-report direction); Lib tightens from "pub
>   absolute exemption" to reference pool determination; Script unreferenced top-level binding
>   starts to report W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (production-side table
>   fill, warehouse zero consumption).
> - **Test migration** (test count not decrease): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub recognized" assertion inverted to "pub is no longer
>   modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (2 assertions inverted);
>   `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal`'s exemption assertions
>   inverted to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` does not participate in IR); corpus diff baseline
>   only adds 1 entry (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated and manually reviewed.
> - **Measured**: lib 2556 / integration 272 / corpus 322 entry baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 landing record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two feature commits, close-out
> (registration / WBS check / gate / push) completed by t7. Precondition = 4.3.1 obligation ledger
> (red-state archived 173 files / 533 keys).
>
> **I. Carrier ruling (Option A: `PlanId` follows AST node)**: Added `PlanId`
> (`src/util/plan_id.rs`, `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX`
> representing synthetic nodes outside parser), distributed by **sole allocator**
> `ParserState::alloc_plan_id`; two carriers — `Stmt.plan_id` (statement identity) and
> `Expr::Call.plan_id` (call site identity), `Expr::call` is the warehouse's sole call constructor.
> What was rejected is the "two sides each derive same identity from span" position-coincidence form
> (D20's original defect).
>
> **II. Two-level verification and review (independent members)**: Each feature commit is paired
> with an independent verifier and a reviewer, **verification/review do not reuse implementer
> evidence**: 4.5.1 = t1 implement / t2 independent verify (external tree + 5 mutations) / t3
> review; 4.5.2 = t4 implement / t5 independent verify (external tree + 5 mutations) / t6 review.
> Review time point and reviewed revision both fall in immutable history (verifiable via
> `git show`). **Provider rename caused t5 / t6 member swap continuation**: original
> verification/review members cannot continue due to provider rename, person-swapped per
> orchestration convention and **reason written in record** (cross-round context preservation
> priority, swap only when compression unavailable); after continuation, conclusion consistent with
> original round (PASS), implementer evidence not reused.
>
> **III. Two defects found and fixed during implementation self-check (t1, both within inScope)**: ①
> Consumption exemption criterion was hooked on `UnitState.ir.is_some()` — `check` form's
> Linking/Mono takes away `unit.ir`, real non-consumption in check form **silently passes**
> (mutation measured: D41 red but CLI check exit 0); changed to read **ledger's own completion
> mark**, and added regression cases. ② Drop sequence drifts with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion ran three times consecutively producing
> 106/120/114 row drift; changed to **total order sort** (same name take max depth + name ascending)
> then ran twice with zero drift.
>
> **IV. Criteria and evidence numbers**: After 4.5.1, `test_release_plan_spans_consumed` removed
> `#[ignore]` standing (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in same
> commit (programmatic review: added 1083 lines Drop + 86 lines jump index shift, zero deletion, no
> rearrangement), corpus 342 entries diff empty; 4.5.2 added `test_overload_resolutions_consumed`
> (diff set empty + evidence surface floor `examined ≥ 1` / `consumed ≥ 2`; measured examined=1,
> consumed entries 2, no resolution skipped 210), resolution form 9 items pinned, `cargo test --lib`
> 2712 passed / 0 failed, IR snapshot **zero drift**. Mutation driven: consumption key offset by one
> ⇒ criterion red + real program `error [W1090] ... produced key not consumed` + corpus gate
> difference; restored back to green.
>
> **V. Classification basis for semantic correction (D59)**:
> `Type.method(positional arg, name = value)` **silently gives wrong answer** before change
> (measured: compiles clean, runs output `1` not `20`) ⇒ **Class C** (old behavior is not success),
> fix takes "loud": after change, same program compile-time `error [W1090]`; "named argument
> participates in resolution" final state semantics is **Class D**, registered in §3.14 and issue
> #452 left for dedicated team. Also: `Drop` is no-op in interpreter
> (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`) — key migration
> does not change runtime behavior.
>
> **VI. F-1 / F-2 disposition (t5 independent verification finding)**: **F-1 (medium)** no-binding
> consumption point (`ir_gen.rs:7766-7790`) is unreachable in current corpus and test surface, key
> correctness has no red state falsifiable ⇒ registered as "known boundary" into
> [02](02-stage-contract.md) §2 revision note paragraph five (preservation reason + no new task);
> **F-2 (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise
> pointer (production `inference/expressions.rs:4550`, consumption `ir_gen.rs:7697` / `:7772`),
> single small commit, not mixed into doc commit.
>
> **VII. Close-out (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement marks);
> 07 criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one line of behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Close-out discovered bookkeeping deviation**: pointers
> `ir_gen.rs:357` / `:3102` / `:1096` in t3 / t6 reports differ from code fact by one to three lines
> (actually `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion) — registered per code fact,
> this note also leaves trace.

## P4 4.10.3 standalone run and MultiFile Orchestration Unification (4.2.5 Ruling A's run half)

> **C-batch record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15 ruling C — this
> round only does 4.10.3 (same-batch candidate 3.4.6 per B1 ([02](02-stage-contract.md) §Runtime
> Behavior Table "dead code family warnings do not enter compile path") deferred until after 4.9,
> preconditions re-anchored and annotated); implementation form = **full single team** (user
> instructed not to open DAG, not to build team, single session implementation + self-verification).
> Pre-batch debt landed first: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng,
> `cargo audit` from exit 1 restored to exit 0), WBS registration debt (#456 registered as 3.17 +
> incomplete rows supplemented with "belonging phase / whether blocking main line" fields).
>
> **I. Red before green**: driver-level red criteria 3 items (`src/driver/tests/run_rootless.rs` —
> rootless merged IR must not carry entry_function (red state `Some("main.main")`), rootless
> top-level executable statement must not report E3023 (red state does not produce module), Bin
> comparison entry_function maintained (green state standing)) + CLI e2e 3 pins (standalone relative
> import run (red state E5001), Script top-level statement execution and #413 hint two green-state
> pinned boards).
>
> **II. Implementation surface**: CLI `run` uniformly goes through `compile_project`
> (`run_file_with_diagnostics`'s project/standalone two-branch deletion); Linking's `entry_function`
> gated by `is_bin_role` (#413 T4 — Script does not auto-call main); #413 hint key control changed
> from `module.source_files.is_empty()` to call-site project probe (after unification ModuleIR
> always carries source_files, original mark invalidated); `script_main_never_called` adapted to
> qualified name form (function `{key}.main`, global slot measured as same qualified name —
> IR-takes-evidence-precedence, not inferred from comments).
>
> **III. Existing gaps exposed by unification (fixed along the way, same batch, A/B/C class — only
> fix failing programs to success, zero impact on previously successful programs)**:
>
> 1. **Explicit type argument pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3 directly
>    emits specialized name (mono will peel unspecialized original), `resolve_call_target` three
>    arms (rename / aliases / dot prefix) all don't cover `mk(int64)` form ⇒ call site dangling
>    E6006. Fix = this single-point resolver adds fourth arm (base name qualification, type
>    parameter string preserved as-is). **Bin project same-form reproduction (pre-existing defect,
>    not introduced by this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign
>    same-form as declaration) per statement ⇒ same variable allocates 3 slots, initialization
>    writes to later-allocated slot while read uses earlier-allocated slot (runtime Add type
>    mismatch). Fix = two places merged by name (same criterion same source).
> 3. **Same-module constrained generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually is `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidates + table lookup verification (precise hit →
>    generic's unit key qualified hit → fallback to bare name status; not suffix heuristic, same
>    form as `resolve_generic_name`).
>
> **IV. Registered gap (not fixed in this batch)**: constraint dispatch when **type and generic
> definition in different modules** still E6006 (rootless two-file probe verified; Bin same form) —
> root cause = MonoType user type name does not carry definition module, correct fix belongs to P6
> type identity convergence; registered WBS **3.18** (belongs to P3, precondition pending P6, does
> not block main line), `qualify_deferred_method` annotation same surface.
>
> **V. Test adaptation (honestly registered)**: `tests/integration/multifile.rs` fixture
> supplemented manifest — it tests RFC-029 project orchestration (Bin auto entry) not Script
> semantics; `cli_e2e` one pin (top-level diagnostic variable name visibility) and
> `script_main_hint` fixture (7 cases) synced per qualified name form — production/consumption side
> only takes merged IR, fixture directly drives same form, no pipeline product stand-in.
>
> **VI. Final gate (close-out revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regenerated **12 lines** — structured check line by line: exit / stdout / diagnostic
> code all item-by-item identical, only stack frame function name and variable name qualified by
> unit key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process events (truthfully registered)**: This batch once edited workspace docs
> concurrently during pre-commit hook run, eaten by hook's stash rollback and that commit failed
> (concurrency form pre-announced in config comments) — discipline = don't touch workspace during
> hook run; #455's commit therefore split into two attempts (second all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Close-out)

> **4.6 batch record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 instructed to
> advance in order 4.6 → 4.9 → 4.4 → P5, doing 4.6 first; implementation form = **full single team**
> (per previous batch user instruction: not open DAG, not build team, single session
> implementation + self-verification). Task definition (09 third-level table): project-internal
> file's semantic data (SemanticDB) goes through orchestrator same pipeline, same source as
> diagnostics; cross-file references carry `resolves_to`. Acceptance: project-internal cross-file
> jump hits definition; semantic data and diagnostics come from the same compilation.
>
> **I. Current state verification (verify before asserting)**: didOpen/didChange triggers **two
> trips** of compilation — diagnostics go through unified Driver pipeline (4.2.3/4.2.6 unified),
> semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` resolution fails, cross-file reference events not produced, jump/rename/reference lookup
> silently fall through for imported symbols — D55 registered silent failure). Orchestration arm's
> checker uniformly names `"<module>"` (SemanticDB file identity lost), `type_result` (contains
> semantic_db) is discarded when diagnostics are non-empty and never externally sent; `checker.rs`
> emptying always overwrites `resolves_to`'s DefId file identity to current file;
> `Export`/`ModuleInfo` both lack definition position surface.
>
> **II. Red before green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`) — temp two-file project
> after didOpen main.yx sends GotoDefinition to `add_one` call site, **before fix returns None (red,
> verified silent failure in previous section)**, after fix hits lib.yx line 0 definition (green).
> During process a probe located: compiler-side chain (driver probe dump) fully connected but e2e
> still empty — root cause is `resolve_reference` precisely matches per reference **start** (D55
> precise-only current caliber), test cursor falls on identifier middle; this tolerance belongs to
> existing behavior surface, this batch doesn't touch (see VII).
>
> **III. Implementation surface**: (1) `Export.definition_span` (extract_module_info harvested from
> AST identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled per DiscoveredFile, including
> vendor; std always None); (2) checker reference emptying via `import_def_locations` (use line
> binding span → definition file + definition span) redirects cross-file imported items'
> `resolves_to` out of this file; std import unregistered, maintained to fall on use line; (3)
> checker module identity changed from uniform `"<module>"` to unit file path (verified all 4
> consumption points of `env.module_name` zero diagnostic consumption); (4)
> `DriverOutcome.semantic_db` channel — CollectAll form (Lsp target file **regardless of diagnostic
> success** + SingleFile fallback) merged in, Lsp arm tail injects non-target file top-level export
> definition surface from registry (`external_definitions`, unopened files can also jump;
> type/submodule exports not injected — typecheck side type registration doesn't produce definition
> event, two sides maintain consistent vacancy); (5) `orchestrator::analyze_source_in_project`
> (`check_source_in_project` re-invoked to take diagnostic half); (6) LSP `analysis.rs` single
> compilation entry replaces manual pipeline (`update_semantic_db` deleted), World key uniformly
> normalizes file URI (`uri.rs`: `path_to_uri(uri_to_path(u))` idempotent normalization, drive
> letter case and %XX encoding difference normalized to same key, cross-file DefId therefore stably
> hit); 5 handlers + semanticTokens dual request connect to `db_key`.
>
> **IV. Fix along the way (D59 split B class — old behavior silently no action, zero external impact
> on previously successful programs)**: ① project-internal file cross-file jump/rename/reference
> lookup from silent fall-through to hit real definition; ② on lexical error, old manual pipeline
> deleted semantic information (coloring disappeared) → partial AST coloring preserved (4.2.6 editor
> philosophy extension); ③ when target file has type error, semantic data no longer discarded with
> `type_result` (typing intermediate state coloring/jump not disappeared).
>
> **V. Pinned boards**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs` — target file path
> key + same-source diagnostic channel, cross-file resolves_to + external definition surface
> round-trip, std import fallback to use line, error file preservation, single-file fallback path
> key, single-file error preservation) + LSP e2e 1 (red before green) + uri normalization 4
> (`src/lsp/tests/uri.rs` — drive letter conversion, space encoding, idempotent normalization,
> non-file scheme pass-through).
>
> **VI. Mutation verification (scratch.py trial, mechanically reversible)**: batch tree wbs-46 @
> close-out revision 39546020, both mutations made hit pinned boards turn red, blob after
> restoration consistent with HEAD + git status CLEAN — m1 picked `import_def_locations`
> registration (statements.rs) ⇒ cross-file driver pin + LSP e2e pin red, other 7 green; m2 picked
> `external_definitions` injection (arms.rs) ⇒ same two pins red (round-trip resolution lost
> anchor), other 7 green. Batch tree gc'd (released 4.8 GB, `status` no batch tree).
>
> **VII. Final gate (close-out revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> diff **byte-for-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synced server.rs:482 and 02 §Obligation Field
> Table) / check-docs-truth 0 / docs-examples 258 entries 0 failure / docs-orphan 0 / markdownlint
> warehouse only has user's unsubmitted 032 two MD056 (user's own editing, not touched) / pnpm
> docs:build 0. **Process events (truthfully registered)**: en/09a-wbs-history.md:54 one MD037 was
> introduced when auto-translate bot re-translated previous batch 09a (zh original has no space),
> this batch fixed it along the way — markdownlint red is not this batch's change surface but same
> file as 09a, registered together. **Leftover observation (registered for discussion, not acted
> on)**: `resolve_reference` only precisely matches per reference start — when cursor falls on
> identifier middle, jump/hover/rename silently does nothing (D55 precise-only current state,
> whole-file form isomorphic), tolerance matching if needed is independent behavior change, report
> to user for ruling.
>
> **Same-day continuation batch (4.6.2, landed after user ruling)**: After the wrap-up report was
> delivered, user on "leftover observation ①" ruled "okay, then change" — lookup key tolerance
> implemented per option A: definition/hover/references/rename four handlers' lookup keys changed
> from cursor position (via UTF-16 conversion) to use **identifier's own start** already located by
> `find_identifier_at_position` (same coordinate system as semantic events, no conversion);
> `resolve_reference`'s start precise match semantics unchanged (D55 precise-only unchanged).
> references/rename's definition point fallback determination (start <= col < end) holds
> isomorphically for identifier start. Red before green: definition middle cursor pin first red
> (None) then green; four handlers middle cursor each 1 pin + cross-file e2e start/middle dual
> position assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2
> (third-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 1 Close-out (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 batch record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 ruled to merge
> three related third-level tasks 4.9.1 / 4.9.2 / 4.9.3 **into one team one round** (reason: they
> all belong to the "pool's source and identity" aspect, merged landing can avoid intermediate state
> repeated rewrites after `collect_project_refs` deletion; 4.9.4 / 4.9.5 separately belong to
> consumption refinement and test pool separation, depending on 4.6.1's `use` path resolution,
> forcing same batch will expand gate surface). #450 attribution **pending ruling** — evidence shows
> it belongs to "after overload resolution name not returned to reference count" class, still
> reproduces in round 1 landed form, registered per independent WBS item for disposition (team lead
> recommends A: register as independent item when merged into round 2; **pending user ruling**).
> Frozen rev = `3863a9ff0` (parent commit `62f6e1a59` is pre-fix baseline); verification PASS
> attempt `aab92fe0` (i.e., t2); review pass attempt `96452696` (i.e., t3). **Close-out registration
> commit = `38d30b846` (docs(docs): 4.9 Reference Pool Round 1 Close-out Registration)** — this was
> actually written and pushed by close-out member integration-refpool; its wrap-up probe once
> mislabeled close-out task as terminal failed and overwrote output, team lead took over new attempt
> and changed back to completed per verified delivery facts, hereby correcting attribution and
> label. Evidence archived: `target/yx-scratch/yx49/EVIDENCE.md` (contains 8 mutation patches and
> warn_compare_v8.json).
>
> **I. Merge Ruling**
>
> 1. **Ruling itself**: three related third-level tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round** — former depends on 4.6.1 delivered
>    `use` path resolution (already landed at `39546020`, can start); latter with RFC-029f table
>    collection same batch, registered in RFC-029f end section to be changed.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are each other's preconditions (4.9.2 changes pool type →
>    4.9.1 assembly surface needs rewrite → 4.9.3 has denominator-determined role table), splitting
>    into rounds will inevitably write intermediate state once then change twice; (b) all three
>    belong to "pool's source and identity" aspect, 4.9.4 / 4.9.5 belong to another aspect, batching
>    is by "cohesion" rather than "independently revertable" division.
> 3. **Merge result**: single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: pre-push `git pull --rebase` merged in
>    auto-translate bot commit (`72f364169`); rebase landing point is after counterpart's commit,
>    this batch's two (implementation `3863a9ff0`, its pre-fix baseline and counterpart's doc commit
>    `62f6e1a59`) **not rewritten, maintained original hash** — initial version miswrote this as
>    "`3863a9ff0`→`3863a9ff0`" style identity mapping (looks like "rewritten then happens to be same
>    value", actually "not rewritten"), corrected during 2026-10-17 close-out; counterpart's code
>    commit `33f14e8ab`→`b122b7ad1`. **External references uniformly use post-push hash as
>    authority** (rewritten old hash unreachable in new clone, identity determined by patch blob
>    comparison).
>
> **II. Red/Green Measured Caliber of Five Criteria**
>
> | #   | Criterion                                              | Red State (pre-fix / P1 directory scan regression patch)                                                                                               | Green State (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                     |
> | --- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | ①   | Pool's root = package target (#334)                    | Directory recursive scan ⇒ file identifiers under tests/ also enter pool, exempting production code unused definitions                                 | `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` per `ExplicitSurfaces` root; fallback `collect_compilation_refs` (`helpers.rs:454`) when all roots degraded                                                                                                                                                                                                                 |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-pollute | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembly sole entry; `arms.rs:603-609`'s `other_module_refs` takes union of all module entries except this unit then sends to `DeadCodeAnalyzer`; module's own name (top-level definition and local binding, `use` import name exception) no longer exempts other modules' same-name definition |
> | ③   | `FileRole::Script` does not use pool                   | Script affected by other files' references in project                                                                                                  | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test go through `_ => {}` default arm (judged by file-internal definition-use graph)                                                                                                                                                                        |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Pre-fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                       | Red state measured (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                       |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Pre-fix: reports `W1003@1`                                                                                                                             | Same as above                                                                                                                                                                                                                                                                                                                                                                                    |
>
> **Criterion ②'s "deduced red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` in helper.yx; old pool under bare-name union treats `foo` as whole "used by helper",
> lib.yx's `foo` exempt. **Measured**: red2 fixture vs pre-fix binary → no W1001 (old caliber
> exempt); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red-state fixture **first version failed**: first
> version fixture placed same-name local variable in same file triggering E2010 (this language
> doesn't allow same-module shadowing of top-level definition), changed to another module
> (helper.yx) for deduced red state. This point t3 review and t2 verification both independently
> reproduced, **not contract defect** — same-module shadowing is this language's hard constraint,
> fixture must cross modules.
>
> **III. Correspondence between +8 and +16 two count sets** (**per t3 review proof-dead** as
> authority)
>
> `target/yx-scratch/yx49/EVIDENCE.md`'s C section writes `+16/−0`, but missed the "**new fixture
> records**" component — v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff,
> `only_before=0`) caliber **includes** new fixtures; t1 implementer's self-built "+8" is
> **warn-before / warn-after (each 325 entries) aggregated by fixture** (each member file checked
> once). Review mapped item by item:
>
> - **+8** (`8` new warnings, all `check <non-entry-file>`, `check <entry-file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: these 8 fixtures all **did not declare package target** ⇒
>     4.9.1 root degrades to checked file's own compilation set, consumer main.yx not in it; is
>     design-intended change (these files' check surface was zero warning before, not exemption
>     path). Repo root `yaoxiang.toml` is empty file ⇒ same degradation path verified.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **does not exist** — is this batch's newly added fixtures). Hence +8's "+ 0 − 0" diff doesn't
>     have them, +16's v8 symmetric diff has them.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` one entry: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once per file), **cancels** leaving only lib's 1
>     entry — three data (+8 / +16 / dual binary ABBA) are mutually consistent.
> - **New fixtures 8 entries** vs **+16 residual 8 entries**: same batch of files (4.9.1 / 4.9.2
>   newly added `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats each of
>   their checks as independent record — same file may trigger different diagnostic entries for
>   multiple `role/context`.
>
> **Conclusion**: **+16 = +8 (aggregated by fixture) ∪ new fixtures 8 entries** — both counts are
> correct, **caliber different**: t1 reports "added 8" is fixture-level caliber, t2 reports "+16" is
> file-level caliber (symmetric diff includes new fixtures). t3 review ① +8⊆+16 already proven;
> **review ② writes "do not copy EVIDENCE.md C section"** — C section missed "new fixture records"
> component, this item rewritten per t3 review `pass` conclusion.
>
> **IV. Mutation Experiments (Mechanically Reversible, all `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b use import name
> removal, p3b script to pool, p3 flat consumption, p4 std to pool, p7final self-built 4 criteria,
> p8 std off, p9 vendor to pool + p_stack2 / p_stack3 stacking surfaces). Each trial output
> contains: HEAD blob → mutated blob → restored blob consistent with HEAD = yes + `git status`
> CLEAN. Local `core.autocrlf=true` needs `GIT_CONFIG_*` to inject `false` (`trial_env.ps1`),
> otherwise patch new lines convert to CRLF causing apply/restore double failure — **only affects
> patch mechanical property, doesn't change measured behavior**.
>
> **V. Process Events / Environment Lessons (must-read for handover to next round)**
>
> 1. **core.autocrlf injection**: local `core.autocrlf=true` must be injected with `false` via
>    `GIT_CONFIG_*` to ensure patches are mechanically reversible; see
>    `target/yx-scratch/yx49/trial_env.ps1`. **Next round do not reuse** any trial script without
>    injection.
> 2. **Same-tree trial serial**: `trial` and `cargo build` on the same tree **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch the worktree, parallelism
>    will cause two types of races: "patch incomplete but seen by build" or "build and apply
>    deadlock".
> 3. **trial rebuild binary needs sync confirmation**: after `trial` completes, it will rebuild the
>    in-tree binary (cargo's fingerprint includes source path, **tree switch = full rebuild**),
>    before subsequent measurement must confirm `bin/yaoxiang-rs` syncs with HEAD. **This round once
>    used residual mutated binary to get false data**, rebuilt and re-measured to correct.
> 4. **Reason for second tree yx49-b**: for comparison need pre-fix binary (62f6e1a59), no
>    ready-made in batch; `scratch.py init yx49-b` opened second tree (eats another full build).
>    **Tree released** (see VI), deletion left to batch close-out `gc` handling.
> 5. **Reason for cli_e2e two Internal use case fixtures supplemented to legal package (supplement
>    `[run].main`, assertions unchanged)**: pre-fix `cli_e2e` two `Internal` role pinned fixtures
>    **did not declare `[run].main`** ⇒ after 4.9.1 lands, `compile_project` goes through
>    `ExplicitSurfaces` root call path, can't find `main` ⇒ entire batch 2 fixtures behavior shifts
>    from "Script fallback" to "entry not found" — is fallback behavior introduced by 4.9.1 not
>    defect. After supplementing `[run].main` field, two fixtures go through normal package path,
>    **assertions** (exit code / diagnostic code / warning set) verbatim unchanged.
> 6. **Safe way to delete second tree**: after release, use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" before
>    deletion. This batch was already released before close-out, next step `gc` naturally cleans up.
>
> **VI. Batch Tree Status**
>
> Before close-out: yx49 released (4.8 GB released), yx49-b released. Two trees unmounted, status
> can be reviewed in `scratch.py status`. End of this task output attached with `scratch.py gc`
> complete status.
>
> **VII. RFC-029f "Known Defects" Table Flip**
>
> Previous version (2026-10-09) described "three design defects of reference pool" in present tense
> (including deleted code positions like `collect_project_refs` / `orchestrator.rs:603-648` /
> `orchestrator.rs:393-396`). This batch close-out synchronously rewrites to "**known defects:
> landed (2026-10-16, WBS 4.9 Round 1)**": ① current pool's root = `collect_package_refs`
> (`helpers.rs:481`), parsed per `ExplicitSurfaces`, not directory scanning; ② current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly sole entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumption end `arms.rs:603-609`; ③ current Script doesn't connect to
> pool, `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-fetched on referenced
> revision `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` already moved to
> `src/driver/{state,arms,helpers}.rs` along with P4 4.2.9 split (`c59fb751`), old positions in
> table no longer hit — this is "declaration inconsistent with fact" fix (t3 review note ①).
>
> **VIII. Final Gate (Close-out Revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixture
> behavior run, 338a339,340; another 342 lines byte-for-byte unchanged) / IR snapshot 204 zero drift
> / check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process event**: this
> batch once cargo build due to `core.autocrlf` converting script line endings to CRLF, team scratch
> path `yaoxiang-rs` startup reported `BadImage`, measured as encoding and git index inconsistency —
> passed after rebuild (parallel with 4.6 batch "external whole-tree discard" lesson, different
> cause).
>
> **IX. Leftover (handed over to next round)**
>
> 1. **#450 attribution ruling**: evidence shows belongs to "after overload resolution name not
>    returned to reference count" class, **still reproduces in round 1 landed form** — dispatched
>    per user ruling C process. **#450 probe**: dual binary same set `{W1002@2, W1005@4}` (measured,
>    rootless form), no direct relationship with round 1 refactor, but #450's fix may need to wait
>    for 4.9.4 (consumption refinement) — because root cause is "overload resolution then name
>    passed to reference count" needs use item's precise consumption surface, 4.9.4's "actually used
>    after being imported" is same-class criterion's precise version.
> 2. **dead_code.rs:683 pointer nit**: comment writes "two pools' assembly caliber see driver side
>    collect_compilation_refs" — actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment and code main path caliber inconsistent
>    (**not in this batch's write scope**), left to 4.9.4 / 4.9.5 landing to correct together.
> 3. **4.9.4 / 4.9.5 preconditions unlocked**: 4.6.1 (`use` path resolution) already delivered at
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to first determine
>    RFC-029f table collection (already paved by this batch's "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user ruling C deferred until after 4.9) — this round 1 not involved; next round
>    4.9.4 / 4.9.5 landing handle in same batch.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 2 Close-out (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 batch record (2026-10-16 implemented → 2026-10-17 closed)**: covers 4.9.4
> (consumption criterion = actually used after being imported), 4.9.5 (test pool separation), 4.9.6
> (method reachability caliber, issue #450, user 2026-10-16 ruling A merged into round 2). Close-out
> fix (verification side remediation M1/M2/M4 compilable redo + F1 fix round + PA-2 dead branch
> deletion) and registered gap issue #464 (WBS 4.9.7) settled together.
>
> **I. Adjudication Chain and Commits (uniformly post-push truth value)**
>
> | Step | Content                                                                                                                                           | Post-push Commit | Pre-push Local Value | Judgment Method                                              |
> | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- | ------------------------------------------------------------ |
> | 1    | fix(util): check explicit path not blocked by default exclude table + --exclude always empty (issue #463, P0-0.4)                                 | 366bc5a69        | ef959ca55            | Patch blob identical                                         |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool attribution by import + Test doesn't contribute consumption surface)                                             | e6cf1aeaa        | 84a6f6978            | Patch blob identical                                         |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; including generated warning-codes.md)                                                    | ddcbb12d0        | cd629b5fe            | Patch blob identical                                         |
> | 4    | docs(meta): AGENTS clause 10 ⑦ (stash window convention during commit)                                                                            | ec94fa9d8        | 9a4ed5f3b            | Patch blob identical                                         |
> | 5    | Fix round = F1 (overload sibling overload restore report W1005) + F2 (verification side M1/M2/M4 compilable redo) + F4 (snapshot generation note) | b8380e6ab        | 3d9ef6eab            | Patch blob identical (36962f318e42…, git range-diff no diff) |
> | 6    | Close-out fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three-place false statement correction, register #464)                | b940300ac        | e13984465            | Patch blob identical (patch-id 9bda3cfe4c…)                  |
>
> **Hash explanation**: pre-push `git pull --rebase` for each commit will rewrite local hash;
> "judgment method" column in above table is all "patch blob identical"
> (`git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent + `git range-diff` no
> diff). This article does not reference any pre-push hash as authoritative source.
>
> **II. Red/Green Measured Caliber of Three Criteria** (verifier self-built reproduction, doesn't
> trust implementer criteria)
>
> | Criterion                  | Red State (before binary 366bc5a69 measured)                                                                                                                                                                | Green State (ec94fa9d8 and after)                                                                                                          |
> | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() rescues a.yx's same-name foo; guard only imported not used also rescued)                                                    | Same fixture 2 entries truthfully reported (name collision doesn't rescue + unused import doesn't count as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print rescued by tests/ reference (only 1)                                                                                                                                              | Same fixture 2 entries (test reference doesn't spill); ref_pool's test_ref_pool_excludes_test_role_references passed                       |
> | 4.9.6 method reachability  | Self-built fixture (2 groups disambiguation overload call + 1 not called) 3 W1005 false reports; method_call_args.yx 3 entries / method_overload.yx 1 entry (overload by full name coverage only reports 1) | Same fixture correctly reports un-called orphan; method_unused 3 pins passed; corpus call_args 3→0 / overload 1→0                          |
>
> **Fix round (F1) tighten again**: un-hit resolution's sibling overload restore report W1005
> (b8380e6ab — same fixture calls ->Float reports ->String sibling, calls ->String reports ->Float
> sibling, bidirectional line number pin). **Fallback boundary (register #464)**: cross-unit
> (registry resolution) method call doesn't produce resolution entry, falls back to name-level
> exemption, cross-unit same-name same-order previously not reported — register 4.9.7.
>
> **III. Process Events and Lessons (corresponding to four hard requirements)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: round 2 review once
>    registered "cross-unit same-name same-order not reported" as blocker (PA-1); subsequent two
>    fact reconciliations (implementer archive + verifier at b8380e6ab/b940300ac dual binary
>    re-measured same fixture set, diagnostic set byte-for-byte consistent) confirmed it is
>    **capability gap existing before round 2** not this round's regression — mechanism = cross-unit
>    method call doesn't produce resolution entry, falls back to name-level exemption; disposition =
>    change to medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer publicly corrects before data and withdraws regression determination**: reviewer's
>    initially cited before data ("-42 entries contains called methods") inconsistent with its raw
>    output (W1005 count=0) — verifier after checking confirmed before capture is 0 entries,
>    reviewer's regression determination based on this **publicly withdrawn**; -42's correct caliber
>    = 26 files' called method false report elimination (Phase B), each item traceable to
>    behavior-run exit=0 file (method actually called and executed).
> 3. **t13 terminated due to non-existent contract premise (not implementation defect)**: this
>    task's acceptance premise wrote "cross-unit same-name same-order will report W1005 regression
>    fix", but actual measurement (t14 dual binary) confirmed this behavior is registered gap (#464)
>    not regression — contract premise doesn't exist, task terminated; its probe value taken over by
>    t16 read-only determination (confirm `strip_unit_prefix` is dead branch).
> 4. **--no-verify will also skip markdownlint and commit-msg checks**: round 2 close-out's doc
>    commit was once blocked by pre-push supplementary hook run (table column count / trailing space
>    / registration field missing); if doc commit wants to use it, at least manually run these two
>    hooks.
> 5. **Process omission of unsynced contract description**: t14's task description text retained
>    PA-1 old hypothesis (wrote "b.yx reports W1005" expectation), verifier actively pointed out in
>    report "judge by contract field, description text not counted" and asked team lead to confirm —
>    disposition = use task contract field as adjudication basis, description text left as process
>    lesson (**when reassigning/dispatching, task description must sync with latest adjudication
>    chain**).
>
> **IV. Mutation Experiments (Mechanically Reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (target/yx-scratch/yx49/ under m1b/m2b/m4b are F2 after
> compilable redo version — original M1/M2/M4 commented out if condition line leaving dangling block
> body, judged as "compilation failure mis-recorded as criterion red" by t8 review F2 and voided;
> redo version after per-patch cargo check exit=0 + trial red). M3 (name collision attribution
> regression) original is real mutation, measured ref_pool 5 criteria turn red, retain original
> judgment. M1b anchor point re-fetched on b8380e6ab (F1 fix after probe segment refactor, old
> anchor invalid). All trials: HEAD blob → mutated blob → restored blob consistent with HEAD + git
> status CLEAN. Environment injection continues trial_env.ps1 (core.autocrlf=false via
> GIT_CONFIG_*).
>
> **V. Warning Surface Full Comparison (caliber first)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) 4-tuple symmetric
> diff, project state channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: this round before capture already includes all fixtures ⇒ matched set =
> full set 337, doesn't have round 1's +8 (matched increment) / +16 (full set) dual caliber problem.
> Result: only_before = 42 W1005 (26 files, all called method false report elimination = Phase B's
> -42; per-file check all on behavior-run exit=0 baseline, methods actually called and executed —
> **no one real dead code rescued**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check
> surface ×2 + ref-pool-test-pool's ../src/util.yx:2 = Phase A criterion surface). **Snapshot
> generation (F4)**: Phase A time point's after capture (Phase B pre-57 residual) and final version
> (Phase B post-15+2=17) are **two generations**, must note when citing; also 1 exception
> (interface_member_conflict.yx's Dog.name@18) is field access form, not in Phase B's W1005 removal
> set, consistent before and after.
>
> **VI. Final Gate (Close-out Revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no product
> change. **Gate check arrangement (clause 13)**: this batch pushed CI / Deploy Docs / Auto
> Translate run conclusions handed to scheduled check link for registration (run id + conclusion +
> consecutive push coverage relation: later run covers previous one), not waiting in session.
>
> **VII. Batch Tree Status**
>
> yx49 and yx49-b both released (unified deletion by gc at close-out); no this batch newly added
> directory outside repo.
