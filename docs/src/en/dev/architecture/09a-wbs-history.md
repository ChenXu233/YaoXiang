# WBS Historical Notes (Implementation Records · Audits · Ruling Traces)

> This article is the **historical note** of [09-execution-wbs.md](./09-execution-wbs.md)
> (Construction Task Table): implementation records, audit findings, ruling traces, and errata from
> each stage. **Division of work**: 09 describes only the **plan** (Kanban, three-level task table,
> dependencies, risks, acceptance); this article records only the **history** (what was done, what
> was found, how it was ruled), appended chronologically without rewriting history.

## P2 Establishing Equivalence Criterion Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus pass.
> **D38 prophecy fulfilled**—the green-pass process measured and fixed three real defects in ir_gen
> (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime tolerates via Frame::set_slot), unannotated parameters dropped by
> filter_map (three places, params metadata and frame layout misaligned), match result slot depends
> on frame's implicit Void initialization (now explicitly pre-initialized). The actual semantics of
> jump targets is flattened instruction indices (not the label-keyed form in the 07 text), CFG is
> constructed at instruction level—recorded in the verify.rs file header. **Commit comparison**:
> ir_gen fixes = `a9ea199b`; verify body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision
> caused its message to be mislabeled as "docs(rfc): RFC-027a…", but the content is actually
> verify.rs + six test files, buried in history unamendable, hereby recorded).
>
> **P2 Narrowing Partial Implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> hemorrhage channel pre-unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (4 behavior + 1 compile-error) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract uses the same `TestFileSpec` as
>   single-file corpus (library-side single point), binary location/subprocess launch extracted to
>   `tests/common/` and yx_runner migrated in sync (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form pinned by rfc027 tests (return position/call site/binding position)—the
>   `Sorted(3)` annotation example in 02 would be substituted and evaluated directly as Disproved in
>   the current code, not presenting silent Unproven, the Sorted(3) in 07 pseudocode is illustrative
>   only. Red-state measured data see test file header.
> - The two red criteria hang `#[ignore]` permanently per the 2.4.5 red skeleton precedent: default
>   suite all green, `-- --ignored` reproduces red state; **remove the attribute when P3 fix lands
>   and they turn green**.
> - The reason for the intentional deviation from 07's (code, file, line) table for comparison
>   normalization (removing file/line, comparing only error code sets) is written in 2.4.1 test
>   comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, cannot
> compile in P2. The former is P4's 4.1.4 (do not double-register), the latter's `#[ignore]` red
> skeleton is merged into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: C4 mandatory coverage list item-by-item check (07 §143), 10
> semantic corpus layers fully covered and all within corpus-parity.jsonl diff baseline (26
> representative files 26/26 hit, grep verified)—ref/borrow/move 05-ownership entire directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closure capture
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterator for spawn_for and 16 files, sum types sum_type_* +
> pattern_or_guard + match_call_scrutinee, existential coercion point interface_dynamic_dispatch
> (Vec(Animal) existential heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err in diagnostic diff
> surface), refinement constraints Drop sequence refined_* ×5 + 05-ownership driven ReleasePlan
> (span key contract has another 2.4.4 red criterion permanent). Performance baseline
> `benches/pipeline.rs` checked in (07 §145 second of three smoke tests): `corpus_compile_frontend`
> full corpus compile first test 17.6s, `cli_cold_start_hello` cold process start 169.7ms;
> interpreter throughput covered by benches/lib.rs hotpath group (interp_fib_recursive_27 /
> interp_loop_steady_10m), no duplicate construction. Regression gate check-perf-regression.sh
> assigned to P9 unified list (2.5).

## P3 Correctness Bug Fixes (Minimum Plan)

> **Error type annotation for landed rows** (retrospective annotation, no history modification): 3.1
> = B ｜ 3.2 = B (defensive empty arm, criterion turn-green borne by 3.1) ｜ 3.3 = A ｜ 3.4.1 /
> 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D
> (user-visible surface, already user-ruled) ｜ 3.4.8 ①② = C (main) + A (incidental three
> places) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-ruled).
>
> **P3 Diagnosis (2026-10-12, user request)**: Conduct an account triage in response to the
> perception of "P3 items continuously growing".
>
> **I. First clarify scale (the perception is amplified by two account defects)**
>
> 1. **Numbering collision**: `3.5.x` is shared by two task families—(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level: lexical and parser branch / AST `is_pub` / checker
>    side / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10 registered **3.5 higher-order type
>    parameter call path completion** (3 three-level). Both share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 already renumbered per user ruling: 029g family changed to `029g.x`**),
>    and 029g's rows were miscounted into P3. **Pending ruling**: rename 029g's family to `029g.x`
>    (or move to its belonging stage), other references in the document sync accordingly.
> 2. **Missed check**: `3.7`'s 4 three-level (3.7.1–3.7.4) are all completed (d922a551 / 38bd4b9e)
>    but not checked ⇒ this entry corrected this time.
>
> **II. P3 Actual Account (12 secondary rows)**
>
> | Row                                                                           | Status                                                                                                                                                                             | Disposition suggestion                                                                                                       |
> | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                               | Completed                                                                                                                                                                          | —                                                                                                                            |
> | 3.4 Silent Drop Point Addendum (8 three-level)                                | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlock with 4.3.1                                                                     |
> | 3.5 Higher-order type parameter call path completion (3 three-level)          | Not started (declaration side active, call side broken)                                                                                                                            | Keep in P3; **does not block refactor mainline**, but is "accepted feature unimplemented", priority higher than pure cleanup |
> | 3.6 Function type value call path (1 three-level)                             | Not started (semantics cut to (i))                                                                                                                                                 | **Keep**: correctness defect, clear criterion                                                                                |
> | 3.7 init segment call name qualification (4 three-level)                      | **All completed**                                                                                                                                                                  | Check and close this time                                                                                                    |
> | 3.8 native shadowing (1 three-level)                                          | Not started                                                                                                                                                                        | **Stuck on decision** (first determine name resolution priority), not a scheduling issue                                     |
> | 3.9 / 3.10 / 3.11                                                             | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                            |
> | 3.12 Executor configuration propagation between runtime modes (1 three-level) | Not started (D class, user ruling B: separate team)                                                                                                                                | **Suggest move out of P3 → P8 (execution layer)**, pending ruling                                                            |
>
> **IV. 4.3.1's Leverage (mainline beacon)**: `4.3.1 obligations.rs + assert_drained()` is also the
> prerequisite of **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract PlanId-ization) ⇒ taking
> it once simultaneously advances the mainline and clears two P3 stranded items. 3.5 (HKT), 3.6
> (function type value call) and 4.6.1/4.9.1 all **have no unmet prerequisites**, available anytime;
> 3.8 stuck on semantic decision (user-defined priority vs explicit diagnostic).
>
> **III. Mechanism Suggestion (prevent recurrence)**: Newly registered secondary/three-level rows
> **must carry two fields**—"belonging stage" (which P it truly belongs to) and "whether it blocks
> the mainline"; `registered pending scheduling` is no longer the default state, unmarked is deemed
> unschedulable. This way bloat is visible, sortable, rather than undifferentiated accumulation.
> **Pending ruling**: whether to write this clause into this document's registration convention and
> `src/dev/coding-rules.md`.

> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter (then
> recorded as `pub(crate)`; **2026-10-07 verified actual landing was `pub(super)`**—the `#434`
> ruling referenced in code comments has no record on docs side, see 3.4.3; full private would
> require rewriting 11 construction literals, not taken during hemorrhage phase; sealed when P4
> Obligations migrates); execution mechanism extracted to `src/frontend/proof_execution.rs`
> (prohibition three single point—cannot be placed in typecheck/, that would create L3→L4 reverse
> dependency); 3.1.2 per 02 evidence chain step 5 supplemented **four entries** (WBS row number
> missed compile_project `:99`, must be supplemented and registered per 3.1.1 acceptance "criterion
> turn-green"); 3.2.1 empty arm changed to accounting (proof_calls throws up) + diagnostic
> (`into_result` established conversion path); 3.3.1 SOLVER slot Option-ized, Z3 missing falls back
> conservatively to `SMTResult::Unknown` (E2031 family diagnostic replaces process crash). Two bug
> criteria remove `#[ignore]` to normal green, supplementary corpus fixture
> `proof-obligation-honored`; check-fanout inventory migrated accordingly (wiring point
> 19=inventory). Measured: criteria 2.4.1/2.4.3 turn green, full suite lib 2494 / integration 272 /
> corpus dual layer all green.

> **3.4 Audit (2026-10-07, P3 implementation post-review)**: Regarding defects of the type "failure
> produces no signal" (definition see 02 §41-52), re-inventoried on unmodified code, located 8 more
> places, all registered as 3.4.x above. Method and evidence:
>
> - **Consumption surface inventory** (16 obligation fields grep all warehouse read points one by
>   one): `variant_ctor_calls` **zero consumers**—the only read at `checker.rs:1468` is "self-pour
>   into `TypeCheckResult`", IR side uses form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts production side, exactly the "only test production side, naturally immune to consumption
>   side defects" form criticized in 02 §186 → 3.4.4. `method_overload_ir_names` result-level field
>   also zero external consumers, IR uses parallel `overload_resolutions` → 3.4.5.
> - **Stage coverage review**: 02 §91 table row 1 (four entries "no proof_execution") already fixed
>   by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 state unchanged (multi-file `run` still
>   doesn't output W1003, still doesn't run dead code family) → 3.4.6; LSP non-project-internal path
>   (02 §77 "manual lex→parse") likewise doesn't run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` only four consumers in the warehouse): checker three
>   refinement branches (covered by 3.1), 3.2.1's fixed ownership arm (**source unreachable**), and
>   `environment.rs:422` (**reachable, zero diagnostic, zero accounting, and all warehouse docs zero
>   registration**) → 3.4.1, the only "all-unknown" living defect in this audit.
> - **Solver acquisition surface**: `default_solver()` three consumption forms differ
>   (`predicate.rs:32` process singleton / `checker.rs:1266` inject per `check_module` /
>   `ownership.rs:621` new per back-edge determination)—3.3.1 only eliminated the panic one; the
>   **silent surface** of the other two goes to 3.4.2, **singleton-ization and cache hit** goes to
>   P4's 4.4.2 (do not double-register).
> - **Not counted into 3.4's existing attribution**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys unconsumed, Drop silently lost) still P2 red permanent, attribution 4.5.1/D20
>   unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not in
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4.
>
> **Execution Convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites, can be landed in place
> anytime (recommend doing 3.4.1 first—only "all-warehouse-zero-registration" living defect,
> smallest change surface); 3.4.4–3.4.8 execute with P4, their criteria depend on obligation ledger
> (4.3.1) or unified Driver (4.2.x). **3.4 does not establish a new criterion system**, uniformly
> reuses existing C2/C3 and P9's 9.2 gates.

> **Batch 3.13–3.16 Implementation Record (2026-10-13 registered → 2026-10-15 closed)**: On
> 2026-10-14 the user ruled to merge four related minor items (#451 method binding panic / #452
> named argument into resolution / #453 dead inference entry / #454 native export named argument—the
> last item is a boundary discovery when verifying 3.14, same day confirmed by attribution
> experiment **not introduced by #452**, user ruling ① merged into this batch) into one team for
> processing. Closure status: all four completed, ruling registered as RFC-039 **D60** (six binding
> rules + shared criterion single point).
>
> **I. Two Contract Corrections (lesson: the contract itself can also be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: #452 task contract simultaneously required "mixed
>    argument output 20" and "position and naming competing for same position reports error", but
>    the pinned fixture `Temp.pick(self, a)` has only one value parameter—the same argument
>    combination is both "mixed" and "conflicting". The first implementer honestly reported blocker
>    and set task failed, reverted overall (not exchanging silence for green light). Disposition:
>    task re-assigned + contract revision—provided **six binding rules** and **8 value-level
>    matrix** (fixture changed to two value parameters `pick(self, n, a)`), then attempt 2 completed
>    by the second implementer. The full text of the six rules has been verbatim recorded in D60.
> 2. **inScope missed mechanical requirement surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered only ran `code-tables --fix`,
>    **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964**—the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touched `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **II. Review → Fix → Re-review Two Closed Loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    did not narrow with resolution ⇒ calls containing named arguments silently slipped through both
>    paths of multiple candidates zero-hit/single-candidate hit) → fix round added external code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece suite
>    same commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm by-name settlement) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations
>    mechanically reversibly replayed via `scratch.py trial` all caught, restore byte-by-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inline copy** of a
>    binding criterion, missing rule ④ ⇒ method and native same-form call **ruling
>    divergence**—method reports E1111, native silently exits 0; ir_gen has a third existing copy
>    reporting old code E1015) → convergence round collapsed criterion to single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call points `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught **regression** introduced by convergence
>    round (IR convergence block lost native optional position ⇒ originally successful program with
>    optional position omitted falsely reported E1010) → fix round (optional position criterion and
>    parameter name from `signature.rs:236` single exit) → round-3 review PASS (independent
>    reproduction red→green; under mutation warehouse pinned fixtures FAILED verifying criterion can
>    fail). **Lesson**: review must cover "unfixed branches of similar paths" (this batch's two
>    needs_revision were both due to probes falling entirely on the hit surface); the fix round
>    itself can also introduce regressions (optional position), caught by the next round of
>    personnel rotation review.
>
> **III. Process Events (truthfully recorded, not pointing to individuals)**
>
> 1. **External whole-tree discard (2026-10-14 ~17:14)**: The working tree was externally restored
>    silently—the implementer's in-progress fixes (typecheck/ir_gen/code table/locales/RFC-013 etc.)
>    all disappeared, **at the same moment the user's own two edited review documents also
>    disappeared from git status** ⇒ determined to be a repository-level discard (most likely
>    user-side manual cleanup/IDE "discard all changes"). Three evidences exclude group members:
>    stash empty, no new commits, no reset in reflog. The implementer replayed in-progress work from
>    context and pushed; discipline subsequently changed to "**first save patch to scratch, then
>    sample red state**".
> 2. **Verification member context exhaustion and rotation**: The independent verifier for 3.16 ran
>    out of room mid-way (no closing message), rotated per load rules—successor continued using the
>    ready products (green-state result, corpus equivalence script, mutation plan), verification
>    finally PASS. Positive precedent for partial product handover.
> 3. **Round 5 review rotation and historical round rollback**: When rotating review per load limit
>    clause ⑤, `remove_member` rolled back already-decided terminal review/verification rounds
>    **together into the shared pool**, and the scheduler immediately re-ran them (t3/t8 were picked
>    up by new members, duplicate work and blocked downstream). Disposition: team lead took over and
>    cancelled the rolled-back historical rounds (t3/t8/t13), added AGENTS.md clause 12 accordingly
>    (handle rolled-back rounds immediately after rotation). This batch's closure record registers
>    this event per that clause.
> 4. **scratch governance (~6.3 GB)**: Pre-batch each task had an external tree, 6 directories
>    scattered outside the warehouse (~780 MB, cleaned) + two members' external verification trees
>    (~2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool `scripts/dev/scratch.py`
>    (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md clause 9 (batch tree single
>    entry), clause 10 (verification environment six rules: mechanically reversible mutation / line
>    number taken from referenced revision / CI-exclusive gates must run etc.), clause 11
>    (writer-side load limit), clause 12 (handle rolled-back rounds after rotation) and two top
>    meta-rules (no private content / verify before assert).
> 5. **Contract defect (not pointing to members)**: #454 fix round's contract inScope only circled 2
>    files, actually 8 mechanical requirement files (clause 6 was meant to prevent exactly
>    this—contract template should pre-set mechanical requirement surface list); also that task's
>    terminal state field was once occupied by probe payload, formal acceptance/command evidence was
>    supplemented by team lead per convention.
>
> **IV. Carryover Items (not fixed in this batch, registered for record)**: round-3 review three
> low-severity findings—① Method path's `build_arg_slots` (`ir_gen.rs:521`) is still criterion's
> **second copy** (existing, per previous round's stance registered with closure, not
> converged)—**2026-10-16 pre-release audit supplementary record three points**: (a) it has **two**
> call points: `ir_gen.rs:7888` (method binding path, uses bound function `binding.function`'s
> parameter name) and `:7975` (overload resolution path, uses resolved mangled name leaf), original
> registration only wrote "`ir_gen.rs:521` area"; (b) it has **already diverged from shared
> implementation `bind_args_to_params`**: missing rule ⑤ (duplicate name → E1015), rule ⑥ (missing
> required parameter → E1010) and "optional position" concept, and hardcodes receiver position as
> `1 + i`; the two call points `continue` for empty slots uniformly (comment writes "diagnostic goes
> to typecheck (it guarantees resolution passes means no empty slots)"), a **dangling invariant
> written in comments, without assertion backstop**; (c) **RFC-039 D60's "all call points converged
> to it" enumeration missed these two places**, statement inconsistent with fact, and referenced
> line number not re-taken on referenced revision, has been corrected in D60 text along with this
> entry; ② Batch verification tree (worktree) `cargo fmt --all` error is a worktree workspace
> artifact, gate list should note fmt runs in main warehouse; ③ repair-type contract template should
> pre-set mechanical requirement surface into inScope (see III.5).
>
> **V. Final Gates (closure revision, local two implementation commits + one document commit unified
> push—the only push of this batch)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib`
> **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner`
> 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> rows consistent line-by-line with baseline** (only regenerated row: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line number synced, manually reviewed) / IR snapshot zero
> drift.
>
> **VI. Run Links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test was cancelled by successor
> push per concurrency—cancellation ≠ failure, covered by successor run); governance tool push → CI
> 38546373435 / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) →
> CI 38528560364. The closure push's own CI / Deploy Docs run is registered after push to supplement
> task records.

> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ Added `Linking` to end of `CHECK_STAGES`
> (**pure merge**, entirely skipping `is_bin_role` entry validation—E3020 criterion still belongs to
> `RoleClassification` to prevent double reporting; another hard reason: run path judges "has main"
> by merged IR, under collection semantics "entry itself errored → no IR" would be misjudged as
> missing main and hard-aborted) → `Monomorphization` (check form); `ir_generation_check` retains
> unit IR (**memory only**, for pure merge and mono consumption, no external output/no disk write/no
> snapshot production—if not retained aggregation request is empty, mono passes as `NoObligations`
> false green); mono failure in Check goes through per-file collection channel (diagnostic
> attributed to entry file + `failed_stages`, not set failure), MultiFile keeps `fail_with`;
> `into_outcome` changed to "first take collection then judge failure". ④ Empirically **zero
> production changes**: whole-program BFS already covers cross-unit mutual recursion (pathological →
> E3005, well-formed three-layer chain executes correct value), delivered four e2e groups + one mono
> unit test as living criterion.
>
> **Verification Chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-check with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase no decrease).
>
> **F1 (discovered during verification period, D59 A/B class, fixed in same round)**: In standalone
> multi-file check without project root, aggregation request name (`lib.f`) and unit IR definition
> key (`f`) are from different sources → mono falls into "native has no IR definition" silent
> `continue`, check reports 0 error/exit 0. Fix: aggregation side added reverse lookup by each
> unit's **real definition key** (precise enumeration + literal equality + ambiguity abandon, not
> suffix heuristic; project form not registered, existing path unchanged). **Erratum (same day)**:
> The reverse lookup was **deleted** in subsequent regression fix; root cause solved from naming
> perspective by `module_key` dual-use decoupling (added `script_mode`)—IR definition key = IR call
> name = aggregation request name = `{unit_key}.{name}`. **Regression Record**: After ③ landed, CI's
> `Check tutorial/guide code examples compile` was red (258 blocks / 10 failures: 6 items same-file
> same-name rebinding + 4 items colliding with `<std/list>`, rootless check doesn't do qualification
> triggering `link_module_irs`'s conflict check); fix = ir_gen split `script_mode` (qualified name ⊥
> Script/Bin, E3023 gate changed to look at it) + `ir_generation_check` per-unit constant pass unit
> key + `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter takes effect"
> (consistent with run path existing behavior); verification = document gate 258/0, three-binary
> comparison (HEAD/③ tree/fix tree) three collision fixtures, check form E3005 no regression, E3023
> not opened.
>
> **F2/F3 (accepted)**: mono diagnostic attributed to entry file entry, span can cross units (same
> as run path `path=entry`); Check form holds one ModuleIR per unit (memory only, released after
> check ends).
>
> **Issues left for follow-up (not this round's regression)**: ① E3005's help text still writes
> "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of E3005)—registered
> pending; ② multi-file E3005's span rendering label misalignment (reproduced with HEAD binary,
> pre-existing); ③ Request collection surface has **type parameter position dependency** (generic
> calls with type parameters not in value parameter position don't issue instantiation request, mono
> records `NoObligations`, convergence mutual recursion is exactly this form)—same surface as 3.5
> (HKT call path), registered to that row.

## External Track RFC-029g: Delete `pub` and Auto-binding (#399

> **P3.5 Implementation (2026-10-08)**: All seven secondary tasks completed, entire batch one commit
> (independently revertible).
>
> - **Measurement inconsistent with plan (truthfully recorded)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—actually after deleting `KwPub`, `pub`
>   falls back to ordinary identifier, old writing falls to semantic layer reporting **E1001 Unknown
>   variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically probing removed
>   syntax is anti-pattern", **do not add** migration-specific diagnostic; corpus fixture
>   `pub_declaration_err.yx` pins E1001, document (`syntax.md`) honestly writes this consequence.
> - **Design convergence (beyond RFC literal, requires review)**: After pub deletion, `project_refs`
>   and `cross_file_refs` two fields are semantically identical (same "package-internal reference
>   union", prohibition one prohibits parallel representation) → merged into single `project_refs`,
>   `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally Internal role's
>   non-pub items changed from "intra-file reachability" to "reference pool judgment" (relaxed,
>   false negative direction); Lib tightened from "pub absolute exemption" to reference pool
>   judgment; Script unreferenced top-level bindings start reporting W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs` exports writeback all deleted (production-side table
>   filling, all-warehouse zero consumption).
> - **Test migration** (test count not decreasing): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub recognized" assertion reversed to "pub no longer
>   modifier"; `passes/tests/dead_code.rs` 7 role semantic use cases rewritten (assertion reversed
>   2); `tests/integration/cli_e2e.rs` 8 role use cases rewritten (`lib`/`internal` exemption
>   assertion reversed to report assertion).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not in IR); corpus diff baseline only added 1
>   (`pub_declaration_err.yx` → E1001), command `cargo run --quiet --example corpus_probe`
>   regenerated with manual review.
> - **Measured**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS check / gates / push) completed by t7. Prerequisite = 4.3.1 obligation ledger
> (red state archive 173 files / 533 keys).
>
> **I. Carrier Ruling (A: `PlanId` goes with AST node)**: Added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compile unit, `UNPLANNED = u32::MAX` represents synthesized nodes
> outside parser), issued by **sole allocator** `ParserState::alloc_plan_id`; two
> carriers—`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call site identity),
> `Expr::call` is the warehouse's only call constructor. Voted down is the "two sides each derive
> same identity by span" position coincidence form (D20's original defect).
>
> **II. Two-level Verification and Review (members independent)**: Each functional commit paired
> with one independent verifier and one reviewer, **verifier/reviewer don't reuse implementer's
> evidence**: 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations)
> / t3 review; 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations)
> / t6 review. Review time and reviewed revision both fall on immutable history (verifiable via
> `git show`). **Provider rename caused t5/t6 member rotation**: original verifier/reviewer couldn't
> continue due to provider rename, rotated per orchestration convention and **record reason**
> (cross-round context preservation priority, only rotate when compression unavailable);
> post-rotation conclusion same as original round (PASS), didn't reuse implementer's evidence.
>
> **III. Two Defects Found and Fixed During Implementation Self-check (t1, both within inScope)**: ①
> Consumption exemption criterion was previously hung on `UnitState.ir.is_some()`—`check` form's
> Linking/Mono takes `unit.ir` away, truly unconsumed in check form **silently passes** (mutation
> measured: D41 red but CLI check exit 0); changed to read **ledger's own completion flag**, and
> added regression test case. ② Drop sequence drifted with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion ran 106/120/114 line drift three times in a
> row; changed to **total order sort** (same name takes max depth + name ascending) then ran zero
> drift twice.
>
> **IV. Criteria and Evidence Numbers**: After 4.5.1 `test_release_plan_spans_consumed` removed
> `#[ignore]` permanent (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated same
> commit (programmatic review: added 1083 lines Drop + 86 lines jump index shift, zero deletion, no
> reordering), corpus 342 diff empty; 4.5.2 added `test_overload_resolutions_consumed` (diff set
> empty + evidence surface floor `examined ≥ 1` / `consumed ≥ 2`; measured examined=1, consumption
> entries 2, no resolution skipped 210), resolution form 9 pinned, `cargo test --lib` 2712 passed /
> 0 failed, IR snapshot **zero drift**. Mutation driven: consumption key offset by one ⇒ criterion
> red + real program `error [W1090] ... produced key unconsumed` + corpus gate diff; restored to
> green.
>
> **V. Semantic Correction Classification Basis (D59)**: `Type.method(position arg, name = value)`
> **silently gave wrong answer** before change (measured compile clean, runtime output `1` instead
> of `20`) ⇒ **C class** (old behavior not success), fix takes "loud": after change same program
> compile-time `error [W1090]`; "named argument participates in resolution" terminal semantic is **D
> class**, already registered §3.14 with issue #452 left for specialized team. Another: `Drop` is
> no-op in interpreter (`backends/interpreter/executor/ops/control.rs` wildcard arm only
> `advance()`)—key migration doesn't change runtime behavior.
>
> **VI. F-1 / F-2 Disposition (t5 independent verification discovery)**: **F-1 (medium)** No binding
> consumption point (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, key
> correctness has no red state to falsify ⇒ per "known boundary" registered in
> [02](02-stage-contract.md) §2 revision note paragraph 5 (preservation reason + no new task); **F-2
> (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise pointers
> (production `inference/expressions.rs:4550`, consumption `ir_gen.rs:7697` / `:7772`), single small
> commit, not mixed into document commit.
>
> **VII. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement flag);
> 07 criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one line behavior change; `check-obligations.py` and `check_tracking.py` green,
> CI and Deploy Docs green. **Closure-discovered accounting deviation**: pointers `ir_gen.rs:357` /
> `:3102` / `:1096` in t3/t6 report differ from code fact by one to three lines (actually `:360` /
> `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion)—registered per code fact, this note
> simultaneously records.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (4.2.5 Ruling A's run half)

> **Batch C Record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15 ruling C—this
> round only does 4.10.3 (same batch candidate 3.4.6 per B1 ([02](02-stage-contract.md) §Runtime
> behavior table "dead code family warning doesn't enter compile path") deferred until after 4.9,
> prerequisite changed hook and noted); implementation form = **complete body single team** (user
> instructed no DAG, no team building, single session implementation + self-verification). Pre-batch
> debt first landed: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit`
> from exit 1 restored to exit 0), WBS registration debt (#456 registered as 3.17 + incomplete rows
> supplemented with "belonging stage / whether blocks mainline" fields).
>
> **I. Red Before Green**: 3 driver-level red criteria (`src/driver/tests/run_rootless.rs`—rootless
> merged IR must not have entry_function (red state `Some("main.main")`), rootless top-level
> executable statement must not report E3023 (red state doesn't produce module), Bin control
> entry_function maintained (green permanent)) + CLI e2e 3 pins (standalone relative import run (red
> E5001), Script top-level statement execution and #413 hint two green pins).
>
> **II. Implementation Surface**: CLI `run` uniformly goes through `compile_project`
> (`run_file_with_diagnostics`'s project/standalone bifurcation deleted); Linking's `entry_function`
> gated by `is_bin_role` (#413 T4—Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call site project probe (after unification ModuleIR always
> carries source_files, original flag invalid); `script_main_never_called` adapted to qualified name
> form (function `{key}.main`, global slot position measured also qualified name—takes IR as
> evidence, not comment inference).
>
> **III. Existing Gaps Exposed by Unification (fixed along the way, A/B/C class—only fix failed
> programs to success, zero impact on originally successful programs)**:
>
> 1. **Explicit type argument pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3 directly
>    sends specialization name (mono will peel unspecialized original), `resolve_call_target` three
>    arms (rename / aliases / dot prefix) all don't cover `mk(int64)` form ⇒ call site dangling
>    E6006. Fix = that single-point resolver adds fourth arm (base name qualification, type
>    parameter string preserved as-is). **Bin project same-form reproduction (pre-existing defect,
>    not introduced by this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot assignment**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign
>    same-form as declaration) per-statement ⇒ same variable assigned 3 slots, initialization writes
>    the later slot while read uses the earlier slot (runtime Add type mismatch). Fix = two places
>    merge by name (same criterion same source).
> 3. **Same-module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewriting
>    produces bare name `Vec3.add`, merged table actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + lookup validation (precise hit → generic
>    owning unit key qualified hit → fall back bare name current; not suffix heuristic, same form as
>    `resolve_generic_name`).
>
> **IV. Registration Gaps (not fixed in this batch)**: Constraint dispatch **when type and generic
> definition are in different modules** still E6006 (rootless dual-file probe verified; Bin same
> form)—root cause = MonoType user type name doesn't carry defining module, correct fix belongs to
> P6 type identity convergence; registered WBS **3.18** (belonging P3, prerequisite pending P6,
> doesn't block mainline), `qualify_deferred_method` same surface noted.
>
> **V. Test Adaptation (honestly recorded)**: `tests/integration/multifile.rs` fixtures supplemented
> manifest—what it tests is RFC-029 project orchestration (Bin auto-entry) not Script semantics;
> `cli_e2e` one pin (top-level diagnostic variable name visibility) and `script_main_hint` fixture
> (7 cases) synced per qualified name form—production consumption side only takes merged IR,
> fixtures directly drive same form, no pipeline product surrogate.
>
> **VI. Final Gates (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regenerated **12 rows**—line-by-line structured verification: exit / stdout /
> diagnostic code all identical, only stack frame function name and variable name per unit key
> qualified (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process Events (truthfully recorded)**: This batch once concurrently edited workspace
> documents during pre-commit hook run, was eaten by the hook's stash rollback and that commit
> failed (concurrency form previewed in config comments)—discipline = don't move workspace during
> hook run; #455's commit was therefore split into two attempts (second all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 instructed to
> advance in 4.6 → 4.9 → 4.4 → P5 order, do 4.6 first; implementation form = **complete body single
> team** (following previous batch user instruction: no DAG, no team building, single session
> implementation + self-verification). Task definition (09 three-level table): project's internal
> file semantic data (SemanticDB) goes through orchestrator same pipeline, same source as
> diagnostic; cross-file reference carries `resolves_to`. Acceptance: project-internal cross-file
> jump hits definition; semantic data and diagnostic come from same compilation.
>
> **I. Current State Empirical (verify before assert)**: didOpen/didChange triggers **two rounds**
> of compilation—diagnostic goes through unified Driver pipeline (4.2.3/4.2.6 already unified),
> semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` parse fails, cross-file reference event doesn't produce, jump/rename/reference lookup
> silently fails for imported symbols—silent failure registered in D55). Orchestration arm's checker
> uniformly names `"<module>"` (SemanticDB file identity lost), `type_result` (including
> semantic_db) is discarded when diagnostic non-empty and never externally sent; `checker.rs` drain
> uniformly overwrites `resolves_to`'s DefId file identity to current file; `Export`/`ModuleInfo`
> both have no definition location surface.
>
> **II. Red Before Green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`)—temp dual-file project
> didOpen main.yx then GotoDefinition on `add_one` call site, **before fix returns None (red,
> empirically silent failure in previous section)**, after fix hits lib.yx line 0 definition
> (green). One probe positioning during process: compiler-side chain (driver probe dump) already
> fully connected but e2e still empty—root cause is `resolve_reference` precise-matches by reference
> **start** (D55 precise-only current stance), test cursor falls in identifier middle; this
> tolerance belongs to existing behavior surface, not touched in this batch (see VII).
>
> **III. Implementation Surface**: (1) `Export.definition_span` (extract_module_info harvested from
> AST identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled per DiscoveredFile, including
> vendor; std always None); (2) checker reference drain via `import_def_locations` (use line binding
> span → definition file + definition span) redirects cross-file imported items' `resolves_to` out
> of this file; std import unregistered, maintains falling to use line; (3) checker module identity
> changed from unified `"<module>"` to unit file path (empirically `env.module_name` all 4
> consumption points zero diagnostic consumption); (4) `DriverOutcome.semantic_db`
> channel—CollectAll form (Lsp target file **regardless of diagnostic success** + SingleFile
> fallback) converges, Lsp arm tail injects non-target file top-level export definition surface from
> registry (`external_definitions`, unopened files can also jump; type/submodule exports not
> injected—typecheck side type registration doesn't produce definition event, both sides maintain
> consistent gap); (5) `orchestrator::analyze_source_in_project` (`check_source_in_project`
> redirected to take diagnostic half); (6) LSP `analysis.rs` single compile entry replaces manual
> pipeline (`update_semantic_db` deleted), World key uniformly normalizes file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stably hits); 5 handlers +
> semanticTokens dual requests connected to `db_key`.
>
> **IV. Fixed Along the Way (D59 branch B class—old behavior silently does nothing, originally
> successful programs zero external impact)**: ① Project-internal file cross-file
> jump/rename/reference lookup changed from silently failing to hitting real definition; ② When
> lexical error occurs, old manual pipeline deletes semantic info (coloring disappears) → residual
> AST coloring preserved (4.2.6 editor philosophy extension); ③ When target file has type error,
> semantic data no longer discarded with `type_result` (mid-typing coloring/jump doesn't disappear).
>
> **V. Pins**: 6 driver-level (`src/driver/tests/lsp_semantic_db.rs`—target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback use line, error file retention, single file fallback path key, single file
> error retention) + 1 LSP e2e (red before green) + 4 uri normalization
> (`src/lsp/tests/uri.rs`—drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **VI. Mutation Verification (scratch.py trial, mechanically reversible)**: Batch tree wbs-46 @
> closure revision 39546020, both mutations cause hitting pins to turn red, after restore blob
> consistent with HEAD + git status CLEAN—m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver pin + LSP e2e pin red, other 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two pins red (round-trip resolution loses
> anchor), other 7 green. Batch tree gc'd (released 4.8 GB, `status` no batch tree).
>
> **VII. Final Gates (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> diff **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synced server.rs:482 and 02 §Obligation field
> table) / check-docs-truth 0 / docs-examples 258 entries 0 failures / docs-orphan 0 / markdownlint
> whole warehouse only remaining user's unsubmitted 032 two MD056s (user's own editing, untouched) /
> pnpm docs:build 0. **Process Event (truthfully recorded)**: en/09a-wbs-history.md:54 one MD037 was
> introduced by auto-translate bot retranslating previous batch 09a (zh original no space), this
> batch fixed in passing—markdownlint red not this batch's change surface but same file as 09a,
> registered together. **Carryover Observation (registered pending, not touched)**:
> `resolve_reference` only precise-matches by reference start—when cursor falls in identifier
> middle, jump/hover/rename silently does nothing (D55 precise-only current state, whole-file form
> isomorphic), if tolerance matching is needed it's an independent behavior change, report to user
> for ruling.
>
> **Same-day Subsequent Batch (4.6.2, landed after user ruling)**: After closing report delivery,
> user ruled on "carryover observation ①" "OK, then change it"—lookup key tolerance implemented per
> plan A: definition/hover/references/rename four handlers' lookup key changed from cursor position
> (after UTF-16 conversion) to **identifier self-start** already located by
> `find_identifier_at_position` (same coordinate system as semantic event, no conversion needed);
> `resolve_reference`'s start precise match semantic unchanged (D55 precise-only unchanged).
> references/rename's definition point fallback judgment (start <= col < end) holds isomorphically
> for identifier start. Red before green: definition middle cursor pin first red (None) then green;
> four handlers middle cursor each 1 pin + cross-file e2e start/middle dual position assertion,
> `--lib` 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2 (three-level count
> 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 ruled to merge
> three related three-level tasks 4.9.1 / 4.9.2 / 4.9.3 **into one team one round** (reason: they
> all belong to "pool source and identity" surface, merged landing avoids repeated rewriting of
> intermediate state after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong to consumption
> refinement and test pool separation, depend on 4.6.1's `use` path resolution, forced same batch
> would expand gate surface). #450 attribution **pending ruling**—evidence shows it belongs to
> "overload resolved name not returned to reference count" class, still reproducible under round 1
> landed form, register as independent WBS entry per disposition (team lead recommends A: register
> as independent entry when merging into round 2; **pending user ruling**). Frozen rev = `3863a9ff0`
> (parent commit `62f6e1a59` is pre-fix baseline); verification PASS attempt `aab92fe0` (i.e. t2);
> review pass attempt `96452696` (i.e. t3). **Closure registration commit = `38d30b846` (docs(docs):
> 4.9 reference pool round 1 closure registration)**—this one was actually written and pushed by
> closure member integration-refpool; its closing probe once mislabeled closure task as terminal
> failed and overwrote output, team lead took over new attempt to change back to completed per
> verified delivery facts, hereby corrected attribution and label. Evidence archive: evidence
> summary (8 mutation patch list, `warn_compare_v8.json` stance and conclusion) inlined in this
> section "III. Correspondence between +8 and +16 two sets of counts" and "IV. Mutation experiment";
> original files recycled with workspace, **not checked in, must not be used as document reference
> source** (AGENTS meta-rule ①).
>
> **I. Merge Ruling**
>
> 1. **Ruling itself**: three related three-level tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round**—the former depends on 4.6.1 delivered
>    `use` path resolution (already landed at `39546020`, can start); the latter same batch as
>    RFC-029f table incorporation, registered at RFC-029f end section pending change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutual prerequisites (4.9.2 changes pool type → 4.9.1
>    assembly surface needs rewrite → 4.9.3 has denominator-determined role table), splitting into
>    rounds must write one intermediate state then change twice; (b) all three belong to "pool
>    source and identity" surface, 4.9.4 / 4.9.5 belong to another surface, batch division is by
>    "cohesion" not "independently revertible".
> 3. **Merge result**: single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gates. **Hash explanation**: before push `git pull --rebase` merged in
>    auto-translate bot commit (`72f364169`); rebase drop point after the other party's commit, this
>    batch's two (implementation `3863a9ff0`, its pre-fix baseline and the other party's document
>    commit `62f6e1a59`) **were not rewritten, kept original hash**—initial version mistakenly wrote
>    "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "same value after rewrite", actually
>    "not rewritten"), corrected at 2026-10-17 closure; the other party's code commit
>    `33f14e8ab`→`b122b7ad1`. **External references uniformly use post-push hash as standard**
>    (rewritten old hash unreachable in new clone, identity determined by patch blob comparison).
>
> **II. Red/Green Measured Stance of Five Criteria**
>
> | #   | Criterion                                              | Red state (pre-fix / P1 directory scan regression patch)                                                                                                   | Green state (round 1 landed)                                                                                                                                                                                                                                                                                                                                                               |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | ①   | Pool root = package target (#334)                      | Directory recursive scan ⇒ tests/'s file identifiers also enter pool, exemption for unused definitions in production code                                  | `collect_package_refs` (`helpers.rs:481`) per `ExplicitSurfaces` root calls `discover_with_used`; full root degradation fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                             |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) sole assembly entry; `arms.rs:603-609`'s `other_module_refs` unions "all modules except this unit" entries and sends to `DeadCodeAnalyzer`; module self-names (top-level definitions and local bindings, `use` import names excepted) no longer exempt other module same-name definitions |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by other files' references in project                                                                                                      | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test goes to `_ => {}` default arm (per file-internal definition-use graph judgment)                                                                                                                                                                  |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Pre-fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                           | Red state measured (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                 |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Pre-fix: reports `W1003@1`                                                                                                                                 | Same as above                                                                                                                                                                                                                                                                                                                                                                              |
>
> **Criterion ②'s "deduced red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` in helper.yx; old pool under bare-name union treats `foo` as "used by helper"
> entirely, lib.yx's `foo` exempt. **Measured**: red2 fixture vs pre-fix binary → no W1001 (old
> stance exempts); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool)'s red state fixture **first version failed**: first
> version fixture put same-name local variable in same file triggering E2010 (this language doesn't
> allow same-module shadowing of top-level definition), moved to another module (helper.yx) for
> deduced red state. This was independently reproduced by t3 review and t2 verification, **not
> contract defect**—same-module shadowing is the language's hard constraint, fixture must
> cross-module.
>
> **III. Correspondence between +8 and +16 Two Sets of Counts** (**t3 review verified as standard**)
>
> 4.9 round 1 evidence summary section C writes `+16/−0`, but missed "**new fixture records**"
> component—v8 instrument (`warn_compare_v8.json`, 331 file symmetric diff, `only_before=0`) stance
> **includes** new fixtures; t1 implementer self-built "+8" is **warn-before / warn-after (325 each)
> aggregated per fixture** (each member file checked once). Review mapped one by one:
>
> - **+8** (`8` new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: These 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degraded to checked file's own compilation set, consumer main.yx not in it; this is
>     in-design change (before, these files' check surface was zero warnings, not exemption path).
>     Warehouse root `yaoxiang.toml` is empty file ⇒ same degradation path empirically holds.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **doesn't exist**—new fixtures in this batch). So they don't appear in +8's "+ 0 − 0" diff,
>     appear in +16's v8 symmetric diff.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` left: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once per file), **cancels out** leaving only
>     lib's 1—three data sets (+8 / +16 / dual binary ABBA) mutually consistent.
> - **8 new fixtures** vs **+16 residual 8**: same batch of files (4.9.1 / 4.9.2 added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats each of their checks
>   as independent record—same file may trigger different diagnostic entries for multiple
>   `roles (role) / context`.
>
> **Conclusion**: **+16 = +8 (aggregated per fixture) ∪ 8 new fixture entries**—both counts are
> correct, **different stances**: t1 reports "8 new" is fixture-layer stance, t2 reports "+16" is
> file-layer stance (symmetric diff includes new fixtures). t3 review ① +8⊆+16 verified; **review ②
> writes "do not copy EVIDENCE.md section C"**—section C missed "new fixture records" component,
> this entry rewritten per t3 review `pass` conclusion.
>
> **IV. Mutation Experiment (mechanically reversible, all `scratch.py trial`)**
>
> 8 mutation patches (p1 directory scan, p2b use import name removal, p3b script connection pool, p3
> flat consumption, p4 std connection pool, p7final self-built 4 criteria, p8 std close, p9 vendor
> connection pool + p_stack2 / p_stack3 overlay surface). Each trial output includes: HEAD blob →
> mutated blob → restored blob consistent with HEAD = yes + `git status` CLEAN. This machine's
> `core.autocrlf=true` needs to be injected as `false` via `GIT_CONFIG_*` (`trial_env.ps1`),
> otherwise patch added lines convert to CRLF causing apply/restore double failure—**only affects
> patch mechanicality, doesn't change tested behavior**.
>
> **V. Process Events / Environment Lessons (must read for next round handover)**
>
> 1. **core.autocrlf injection**: This machine's `core.autocrlf=true` must be injected as `false`
>    via `GIT_CONFIG_*` to ensure patch mechanical reversibility; the injection script at the time
>    was recycled with workspace (criteria listed in previous item, can be rewritten based on it).
>    **Next round don't reuse** any trial script without injection.
> 2. **Same-tree trial serial**: `trial` and `cargo build` on the same tree **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch worktree, parallel can
>    cause "patch incomplete seen by build" or "build and apply deadlock" two kinds of race.
> 3. **Trial rebuild binary needs sync confirmation**: After `trial` completes will rebuild
>    tree-internal binary (cargo's fingerprint contains source path, **switch tree = full
>    rebuild**), before subsequent measurement must confirm `bin/yaoxiang-rs` synced with HEAD.
>    **This round once used residual mutated binary to measure fake data**, already rebuilt and
>    re-measured to correct.
> 4. **Second tree yx49-b's tree-building reason**: For comparison need pre-fix binary (62f6e1a59),
>    no ready product in batch; `scratch.py init yx49-b` opened second tree (one more full build).
>    **Tree released** (see VI), delete leave batch closure `gc` disposition.
> 5. **cli_e2e two Internal use case fixtures supplemented to legal package (supplemented
>    `[run].main`, assertion untouched) reason**: Pre-fix `cli_e2e`'s two `Internal` role pinned
>    fixtures **didn't declare `[run].main`** ⇒ after 4.9.1 landed `compile_project` goes through
>    `ExplicitSurfaces` root call path, can't find `main` ⇒ entire batch 2 fixtures behavior drifted
>    from "Script fallback" to "entry not found"—this is 4.9.1's introduced fallback behavior not
>    defect. After supplementing `[run].main` field two fixtures go through normal package path,
>    **assertions** (exit code / diagnostic code / warning set) byte-for-byte unchanged.
> 6. **Second round tree deletion safe method**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" then delete.
>    This batch pre-closure released, next step `gc` natural cleanup.
>
> **VI. Batch Tree State**
>
> Pre-closure: yx49 released (4.8 GB released), yx49-b released. Two trees no mount, status can be
> verified in `scratch.py status`. This task output end attached `scratch.py gc` complete status.
>
> **VII. RFC-029f "Known Defects" Table Flip**
>
> Previous version (2026-10-09) described in present tense "three design defects of reference pool"
> (including `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396` and
> other deleted code positions). This batch closure sync rewritten to "**Known Defect: Landed
> (2026-10-16, WBS 4.9 Round 1)**": ① Current pool root = `collect_package_refs` (`helpers.rs:481`),
> per `ExplicitSurfaces` resolution, doesn't scan directory; ② Current pool type
> `HashMap<String, HashSet<String>>` (key = module key), sole assembly entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumption side `arms.rs:603-609`; ③ Current Script doesn't connect to
> pool, `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-taken on referenced
> revision `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` already moved with P4 4.2.9
> split (`c59fb751`) into `src/driver/{state,arms,helpers}.rs`, old positions in table no longer
> hit—this is "statement inconsistent with fact" fix (t3 review note ①).
>
> **VIII. Final Gates (closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 rows** (two new fixture
> behavior runs, 338a339,340; another 342 rows byte-by-byte unchanged) / IR snapshot 204 zero drift
> / check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process Event**: This
> batch once cargo build due to `core.autocrlf` converting script line endings to CRLF, under team
> scratch path `yaoxiang-rs` startup reported `BadImage`, measured as encoding and git index
> inconsistency—passed after rebuild (parallel with 4.6 batch "external whole-tree discard" lesson,
> different cause).
>
> **IX. Carryover (handed over to next round)**
>
> 1. **#450 attribution ruling**: Evidence shows it belongs to "overload resolved name not returned
>    to reference count" class, **still reproducible under round 1 landed form**—dispatched per user
>    ruling C flow. **#450 probe**: dual binary same set `{W1002@2, W1005@4}` (measured, rootless
>    form), no direct relationship with round 1 change, but #450's fix may need to wait for 4.9.4
>    (consumption refinement)—because root cause is "overload resolved name passed to reference
>    count" needs use items' precise consumption surface, 4.9.4's "really used after imported" is
>    same criterion's precise version.
> 2. **dead_code.rs:683 pointer nit**: Comment writes "two pools' assembly stance see driver side
>    collect_compilation_refs"—actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment inconsistent with code main path stance
>    (**not in this batch's writing scope**), left for 4.9.4 / 4.9.5 to fix together.
> 3. **4.9.4 / 4.9.5 prerequisites unlocked**: 4.6.1 (`use` path resolution) delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs first determine
>    RFC-029f table incorporation (already laid by this batch's "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user ruling C deferred to after 4.9)—this batch round 1 not involved; next round
>    4.9.4 / 4.9.5 landing same batch processing.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 Batch Record (2026-10-16 implemented → 2026-10-17 closed)**: Covers 4.9.4
> (consumption criterion = really used after imported), 4.9.5 (test pool separation), 4.9.6 (method
> reachability stance, issue #450, user 2026-10-16 ruling A merged into round 2). Closure fix
> (verification side remedy M1/M2/M4 compilable redo + F1 fix round + PA-2 dead branch deletion) and
> registration gap issue #464 (WBS 4.9.7) landed together.
>
> **I. Decision Chain and Commits (uniformly post-push truth)**
>
> | Step | Content                                                                                                                                           | Post-push commit | Pre-push local | Judgment method                                         |
> | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------- | ------------------------------------------------------- |
> | 1    | fix(util): check explicit path not blocked by default exclude list + --exclude always empty (issue #463, P0-0.4)                                  | 366bc5a69        | ef959ca55      | Patch blob same                                         |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool per import attribution + Test doesn't contribute consumption surface)                                            | e6cf1aeaa        | 84a6f6978      | Patch blob same                                         |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; including generated warning-codes.md)                                                    | ddcbb12d0        | cd629b5fe      | Patch blob same                                         |
> | 4    | docs(meta): AGENTS clause 10⑦ (commit period stash window convention)                                                                             | ec94fa9d8        | 9a4ed5f3b      | Patch blob same                                         |
> | 5    | Fix round = F1 (overload sibling overload restore report W1005) + F2 (verification side M1/M2/M4 compilable redo) + F4 (snapshot generation note) | b8380e6ab        | 3d9ef6eab      | Patch blob same (36962f318e42…, git range-diff no diff) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three false statements correction, register #464)                       | b940300ac        | e13984465      | Patch blob same (patch-id 9bda3cfe4c…)                  |
>
> **Hash Explanation**: Each pre-push git pull --rebase rewrites local hash; above table "Judgment
> method" column all "patch blob same" (`git diff <rev>~1 <rev> | git hash-object --stdin` same on
> both sides + `git range-diff` no diff). This article doesn't reference any pre-push hash as
> authoritative source.
>
> **II. Red/Green Measured Stance of Three Criteria** (verifier self-built reproduction, not
> trusting implementer's criteria)
>
> | Criterion                  | Red state (before binary 366bc5a69 measured)                                                                                                                                           | Green state (ec94fa9d8 and after)                                                                                                  |
> | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() revives a.yx same-name foo; guard only imported not used also revived)                                 | Same fixture 2 honestly reported (same-name collision not revived + unused import doesn't count as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print revived by tests/ reference (only 1)                                                                                                                         | Same fixture 2 (test reference doesn't spill); ref_pool's test_ref_pool_excludes_test_role_references passed                       |
> | 4.9.6 method reachability  | Self-built fixture (2 disambiguation overload call groups + 1 uncalled) 3 W1005 false positives; method_call_args.yx 3 / method_overload.yx 1 (overload per full name override only 1) | Same fixture exactly reports uncalled orphan; method_unused 3 pins passed; corpus call_args 3→0 / overload 1→0                     |
>
> **Fix Round (F1) Further Tightening**: Unresolved sibling overload restores reporting W1005
> (b8380e6ab—same fixture calling ->Float reports ->String sibling, calling ->String reports ->Float
> sibling, bidirectional line pin). **Fallback Boundary (registered #464)**: Cross-unit (registry
> resolution) method calls don't produce resolution entries, fall back to name-level exemption,
> cross-unit same-name same-order previously not reported—registered 4.9.7.
>
> **III. Process Events and Lessons (Four Hard Requirements Corresponding)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: Round 2 review once
>    registered "cross-unit same-name same-order not reported" as blocker (PA-1); subsequent two
>    fact reconciliations (implementer archive + verifier's dual binary re-measurement of same
>    fixture set at b8380e6ab/b940300ac, diagnostic set byte-by-byte consistent) confirmed it's
>    **capability gap existing before round 2** not this round's regression—mechanism = cross-unit
>    method calls don't produce resolution entries, fall back to name-level exemption; disposition =
>    change judgment to medium, register issue #464, schedule WBS 4.9.7 next round.
> 2. **Reviewer publicly corrected before data and withdrew regression judgment**: Reviewer's
>    originally cited before data ("−42 contains called methods") inconsistent with its original
>    output (W1005 count=0)—verifier confirmed before capture was 0, reviewer's regression judgment
>    **publicly withdrawn** based on that; −42 correct stance = 26 file called method false positive
>    elimination (phase B), each can be traced to behavior-run exit=0 file (method really called
>    executed).
> 3. **t13 terminated due to contract premise not holding (not implementation defect)**: That task's
>    acceptance premise wrote "cross-unit same-name same-order will report W1005 regression fix",
>    but measured (t14 dual binary) confirmed this behavior is registered gap (#464) not
>    regression—contract premise doesn't exist, task terminated; its probe value taken over by t16
>    read-only judgment (confirmed `strip_unit_prefix` is dead branch).
> 4. **Document commit's real criterion is "really build once"**: Round 2 closure document commit
>    (`042c07860`) made Deploy Docs red on VitePress's Vue template compilation—placeholders like
>    `<rev>` / `<before-exe>` / `<after-exe>` in records weren't in backticks, parsed as elements
>    (reported `Element is missing end tag`, pointing to `<rev>` line); markdownlint doesn't catch
>    this type, and pre-commit's markdownlint hook is judged `no files to check` skipped in
>    docs-only commits (measured). Disposition: placeholders uniformly in backticks, before push
>    locally run once `npm run docs:build` (fix commit `0c837e1b4` evidenced by local build exit=0);
>    if using `--no-verify` commit, commit-msg check needs manual re-run.
> 5. **Contract description out-of-sync process omission**: t14's task description text retained
>    PA-1 old hypothesis (wrote "b.yx reports W1005" expectation), verifier actively pointed out in
>    report "judge per contract field, description text doesn't count" and asked team lead to
>    confirm—disposition = use task contract field as judgment basis, description text kept as
>    process lesson (**when reassigning/dispatching, task description must sync with latest decision
>    chain**).
>
> **IV. Mutation Experiment (mechanically reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (m1b/m2b/m4b are F2-post compilable redo version—initial M1/M2/M4
> commented out if condition lines leaving dangling block bodies, judged by t8 review F2 as "compile
> failure mislabeled as criterion turn red" and voided; redo version per patch cargo check exit=0 +
> trial turn red). M3 (collision attribution regression) initial version is real mutation, measured
> ref_pool 5 criteria turn red, kept original judgment. M1b anchor re-taken on b8380e6ab (F1
> post-fix probe section refactor, old anchor invalid). All trials: HEAD blob → mutated blob →
> restored blob consistent with HEAD + git status CLEAN. Environment injection follows trial_env.ps1
> (core.autocrlf=false via GIT_CONFIG_*).
>
> **V. Warning Surface Full Comparison (Stance First Clarified)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, col) four-tuple symmetric
> diff, project mode channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Stance**: This round's before capture already includes all fixtures ⇒ matched set =
> full set 337, no round 1's +8 (matched increment) / +16 (full set) dual stance issue. Result:
> only_before = 42 W1005 (26 files, all called method false positive elimination = phase B's −42;
> per file check all on behavior-run exit=0 baseline, method really called executed—**no real dead
> code revived**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface ×2 +
> ref-pool-test-pool's ../src/util.yx:2 = phase A criterion surface). **Snapshot Generation (F4)**:
> Phase A time point's after capture (phase B pre 57 residual) and final version (phase B post
> 15+2=17) are **two generations**, must note when referencing; another 1 case exception
> (interface_member_conflict.yx's Dog.name@18) is field access form, not in phase B's W1005 removal
> set, consistent before and after.
>
> **VI. Final Gates (closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator run no product change.
> **Gate Check Arrangement (clause 13)**: This batch's pushed CI / Deploy Docs / Auto Translate run
> conclusions registered by scheduled check session (run id + conclusion + push coverage
> relationship: later run covers previous one), not waited in session.
>
> **VII. Batch Tree State**
>
> yx49 and yx49-b both released (unified deletion at closure gc); no this-batch new directories
> outside warehouse.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 3 Closure (4.9.7)

> **4.9 Round 3 Batch Record (2026-10-17 implemented → 2026-10-17 closed)**: Covers 4.9.7
> (cross-unit method call resolution registration and receiver attribution, issue `#464`—WBS 4.9's
> last three-level task; after this round's closure 4.9 entire secondary task kanban checked). Chain
> = production side t19 → independent verification t20 PASS → review t21 needs_revision (F1:
> injection surface false positive) → fix round t24 → independent verification t25 PASS → review t26
> PASS (incremental, verdict=pass). All commit IDs are post-push truth (judgment method = patch blob
> same; `git diff <rev>~1 <rev> | git hash-object --stdin` same on both sides).
>
> **I. Decision Chain and Commits (post-push truth)**
>
> | Step | Content                                                                                                                                                                                              | Post-push commit | Judgment method        |
> | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ---------------------- |
> | 1    | Production side: cross-unit method calls produce D20 resolution entries (`feat(typecheck)`, t19)                                                                                                     | `b8b3cc3f6`      | Patch blob same        |
> | 2    | Driver-side attribution: method reachability per defining unit gives flag, name groups with resolution records disable pool's name-level exemption (t20 verification PASS / t21 review catches F1)   | `d5c00235a`      | Patch blob same        |
> | 3    | Injection surface fix round F1: candidate injection per "name ← providing module" **pairwise** filter, eliminate E1107 false positive (t24, including F2 statement and F3 landing form single point) | `d5dd1a37b`      | Patch blob same        |
> | 4    | Pin test return: F3 pin in `src/frontend/core/typecheck/tests/environment.rs` migrated to declaration surface `src/driver/tests/overload_injection.rs`                                               | `c2f887843`      | Patch blob same        |
> | 5    | dev tip                                                                                                                                                                                              | `e9675c9dd`      | merge commit, see "II" |
>
> **II. Hash Mapping and Two Merges at dev Tip (source unconfirmed, must not write as member
> action)**: Round 3's four commits (above table 1–4) all pushed to origin/dev;
> `git worktree list`'s main mount at `e9675c9dd`. dev tip has two
> **non-this-team-command-surface-generated** merge commits (`5da05b885` / `e9675c9dd`, reflog shows
> as `pull --tags` + push form), source unconfirmed—per clause 10④ (reference must be resolvable by
> anyone) and clause 13 (gates to specialized check) same closure stance, here only register fact
> and reflog evidence, no member attribution; also see "V." lesson ② (push attribution convention
> must be verifiable).
>
> **III. D20 Contract Extension's Actual Form (Key Form and Consumption Points)**: Production side
> packages "candidate / mangled IR name / declaration period parameter name" three parallel tables
> **in same order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) with unit
> crossing, registry merges deterministically per module key ascending
> (`src/frontend/module/registry.rs:150`); typecheck arm per this unit's import face pairwise
> injects (`src/driver/arms.rs:354` → `visible_method_overloads`, `src/driver/arms.rs:1521-1579`:
> name ← providing module pairwise, same name conflict skip whole name, own key position not
> externally merged), landing form single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same order append + equal length
> debug assert); consumption side attributes D20 resolution product per **defining unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`; `resolved_method_marks`
> `:621`), and makes name groups with resolution records disable pool's name-level exemption
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = entry name
> `{Type}.{method}#k` (`{Type}` is defining side bare name; `#k` declaration order).
>
> **IV. Key Scope and False Positive Defense Measurement (Fix Round F1 and Five Residual
> Boundaries)**: F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set"
> combination putting another module's same-name type candidate into same key position ⇒
> `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, methods same name
> same order) fixture `check` reports 2×E1107, exit 1 (pre-fix successful program compilation
> failed); after fix `check`/`run` both exit 0, 4 W1005 honestly report two sides' **uncalled**
> same-name definitions (reverse underreport surface also verified). Warehouse fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level pin
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set before and after fix
> identical per item (no rollback). Five residual boundaries and per-item measurement (alone `check`
> defining file not reported / name-level ambiguity E5006 skip whole name / boundary(c) rewritten
> per t20 measurement / inline alias and `ns.T` not injected candidate surface / whole import
> `use M` and std candidate surface don't produce entries) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §Cross-unit method reachability attribution
> "Residual Boundaries", not repeated here.
>
> **V. Process Events and Lessons (Four Continuing Categories)**:
>
> 1. **Narrow contract wording becomes defect directly**: F1's root cause is contract only wrote
>    "only inject module surface that can be named by bare type name", didn't write "per name
>    pairwise attribution"—implementation literally took "name set × module set" combination, review
>    caught E1107 false positive. When describing behavior boundary in contract, must nail
>    **combinatorics** dead (per name pairwise / full combination), can't leave to implementation
>    self-understanding.
> 2. **Convention must be verifiable: "push by team lead" only in dispatch message ⇒ cannot
>    attribute when third party push/merge appears**. This round's dev tip has two
>    source-unconfirmed merges (see "II"), ultimately relied on reflog backtracking and withdrew one
>    wrong attribution. Verifiable form = members don't run `git push`/`fetch`/`pull`, sync
>    uniformly `pull --rebase`, use reflog to verify.
> 3. **Infrastructure unstable, first land patch then rotate**: This batch total 5 unrecoverable
>    failures (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background sub-agent no message
>    exit); disposition convention = produced evidence and patches first to **workspace** batch
>    directory (under `target/`, not checked in), but **conclusion and numbers must land in-place in
>    this record**—workspace path must not be used as document reference source (AGENTS meta-rule
>    ①). Then rotate to continue, same session two consecutive failures don't gamble third time.
> 4. **Document commit's real criterion is "local really build once"**: Round 2 closure document
>    commit (`042c07860`) made Deploy Docs red on VitePress's Vue template compilation (angle
>    bracket placeholders `<rev>` etc. not in backticks parsed as elements, reported
>    `Element is missing end tag`); markdownlint doesn't catch this type, and pre-commit's
>    markdownlint hook is judged `no files to check` skipped in docs-only commits (measured twice).
>    Fix commit `0c837e1b4` evidenced by local build exit=0. This round's lesson red→green run:
>    Deploy Docs **39011946793** (red) → **39014696984** (green, see "VI").
>
> **VI. Round 2 Gate Conclusion Supplementary Registration (clause 13, scheduled check session
> backfill)**: Round 2 closure commit `b940300ac`'s push—CI **39011946778** success / Auto Translate
> **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red, i.e.
> "V.4"'s angle bracket placeholders; fixed by `0c837e1b4`); fix commit `0c837e1b4` after push three
> green: CI **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**. Push
> coverage relationship: CI@`0c837e1b4`'s **39014356642** was cancelled by successor run per
> concurrency (cancellation ≠ failure, later run covers previous one, consistent with round 2 §VI
> stance).
>
> **VII. Final Gates (closure revision `e9675c9dd`, local numbers; CI conclusions to scheduled
> check, see "VIII")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR
> snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent
> (re-run `scripts/docs/gen-error-code-docs.py` then `git status` clean); corpus baseline 346 → 347
> → 348 rows (each +1 behavior record, other rows byte-by-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 rows).
>
> **VIII. Gate Check Arrangement and Conclusion (clause 13)**: This batch's post-push CI / Auto
> Translate / Deploy Docs conclusions registered by scheduled check session (not waited in session,
> and don't write "not waited" as "already green"). **Backfill (2026-10-17 scheduled check)**:
> `d5c00235a`'s CI `39023417670` = **failure**—Docs Truth reported "error code documentation behind
> code table/locales", i.e. that one missed generated product same commit; fixed by its successor
> `d5dd1a37b` (regenerate generated product): `5da05b885` CI `39027788246` / Deploy Docs
> `39027788268` / Auto Translate `39027788340` all success, covering previous one; `e9675c9dd` CI
> `39028827871` / Deploy Docs `39028827896` success; `d0cfc4e62` CI `39033963240` / Deploy Docs
> `39033963278` / Auto Translate `39033963261` success. Three's runs each ran to terminal, this
> round no concurrency cancellation. Lesson: **"local generator idempotent" ≠ "generated product
> already same commit checked in"**—before closure should verify CI conclusion per commit, not only
> look at latest one.
>
> **IX. Batch Tree State (closure gc, status output)**: Pre-closure `scratch.py status`—t25
> (holder=-) and yx49b (holder=-, previously held by deactivated member verify-r3 stale, team lead
> `release --force` released) two trees both no holder; `gc` deleted two, released about 12.7 GB;
> post-gc `status` = "no batch tree", `git worktree list` only main mount. Another: `yx49r3` and
> `yx49r4` are not batch trees (no meta.json), are members' **workspace** directories for
> evidence/probes, not "residual batch tree". No this-batch new directories outside warehouse.

## P4 4.4 Proof Layer and wasm Cleanup — Closure (4.4.1 / 4.4.2 / 4.4.3)

> **I. Batch Composition and Rounds (batch name `yx44`)**: Three three-level tasks 4.4.1 / 4.4.2 /
> 4.4.3 and wasm attribute reachability judgment same batch; batch verification tree built by
> `scripts/dev/scratch.py init` in workspace `target/` (first full build; deleted after closure gc).
> Roles and rounds: implementation 4.4.2 = t1 (12 files; local `1fc135437` → post-push
> `24028ffe8`) ｜ implementation 4.4.1's layer order rewrite = t2 (only document/comments, zero
> behavior change, based on `b5e68b3c9`) ｜ implementation 4.4.3 = t3 (11 files, including new code
> W1082; local `2d5e610cd` → post-push `52d2276ca`) ｜ wasm reachability judgment = t4 (only
> judgment, zero cleanup, judgment revision `b5e68b3c9`) ｜ 4.4.1 closure and one document commit =
> t8 (7 files 173+/43-; local `29ebd2145` → post-push `d07993157`) ｜ independent verification = t5
> (frozen `29ebd2145`) ｜ round 1 review = t6 (revision `1b593e993`) ｜ supplement = t9 (3 files
> 9+/10-; local `1ebddaf95` → post-push `1b593e993`) ｜ fix round = t10 (3 files 23+/23-; local
> `657c3d5e3` → post-push `e184d88b6`) ｜ round 2 review = t11 (`657c3d5e3` = post-push `e184d88b6`,
> patch-id all equal) ｜ closure = t7 (check / record / batch tree gc / document real build / one
> document commit; local `c8412565c` → post-push `da724940b`). **Failure and Redo**: t6 terminal
> failed with `needs_revision` (review round per contract must fail and give findings, not
> accident); t7 first round interrupted at batch tree gc stage, completed product left in main
> warehouse working tree not committed, second round took over "supplement gap + review"—didn't redo
> completed part (this round new: 4.4 check and evidence note, this record, three
> point-to-deleted-batch-directory path rewrite, document real build and closure commit). No
> cancelled rounds.
>
> **II. Per-task Production and Measurement Numbers (original commands and output see evidence
> summary and archive; conclusion only here)**:
>
> - **4.4.1**: `layers/README.md` (including `layers/mod.rs` top comment)'s layer order table
>   rewritten to **actual call order** (predicate → termination → ownership, call point anchor
>   `checker.rs::check_module_impl`), `equivalence` per real identity description (no production
>   call point); same-source declaration 7 places closed (`01-routing` / `02-stage-contract` /
>   RFC-039 / `06` / typecheck README / layers two files); mechanical reconciliation script on batch
>   tree green baseline **11/11 PASS** (criteria 11 = README side 4: actual call order declaration /
>   two entry anchors / old order declaration disappeared; code side 7: 5 line anchors +
>   `check_module_impl` no short circuit + `equivalence` zero production call; reconciliation script
>   is one-time script, recycled with workspace, criteria 11 listed per item can be rewritten based
>   on), mutation (c) rollback old declaration → reconciliation 2 FAIL i.e. red.
> - **4.4.2**: Solver acquisition surface unified to `proof/smt/backend.rs`'s process-level shared
>   singleton + sole closure entry `with_shared_solver(consumer, f)`; three consumption points
>   (predicate / termination / ownership) all changed to shared entry, **production chain compile
>   `default_solver()` call points = 0** (sole backend selection point sank to private
>   `new_backend()`). `wasm/src/lib.rs`'s `test_smt` self-check call point is **existing** (earlier
>   than this batch, since `b5e68b3c9`) non-test call point, belongs to "test and independent
>   scenario" surface, doesn't constitute D58 prohibition violation. Independent verification
>   self-constructed reading (one compile): predicate acq=2 hit=1 miss=1 ｜ termination acq=1 hit=2
>   miss=2 ｜ ownership acq=2 hit=1 miss=1, three points hit/miss sum (4,4) = shared instance
>   cumulative `Some((4,4))`; mutation (a) singleton degradation → predicate hit=0 i.e. red.
>   **Change Verb Surface Disclosure**: This commit includes `z3_backend.rs` 15 lines—`cache_stats`
>   migrated from inherent method into `Solver` trait block (return
>   `Some(instance internal count)`) + 1 line comment changed pointer, no semantic coupling. D58
>   prohibition mechanized: same-thread reentry → panic contains D58, probe `recv_timeout(120s)`
>   doesn't hang.
> - **4.4.3**: `body_checker` missing silent degradation converged to
>   `TypeChecker::ownership_inputs` (sole criterion point) + new code **W1082** (not overlapping
>   with existing W code facts: W1080 constraint degradation / W1081 solver missing / W1090 opposite
>   direction); pin test 2/2 green; healthy path corpus 348:348 zero diff; **degradation branch
>   production surface unreachable exhaustive holds** (CLI four entries + eval/repl + format + LSP +
>   wasm playground + all `ProgramKind` converge into `check_module_impl`: `init_body_checker`
>   unconditionally before ownership block, 0 return/break/continue in function, `body_checker` no
>   `.take()`) ⇒ W1082 no user-visible surface. Mutation (b) delete emission → pin test i.e. red.
>   **W1082 is external contract table change (E class), user-approved**; `locales/zh.json` /
>   `locales/en.json` same commit as code, other languages supplemented by i18n bot after push (run
>   see "IX").
> - **wasm attribute reachability judgment (t4)**: `orchestrator.rs` current tree **10 = 4
>   positive + 6 negative** (stance = same line `#[cfg(...)]` attribute text, both positive and
>   negative count; `cfg!` runtime macro 0 hit); 4 wasm positive arms unreachable at playground
>   runtime but **compile-time load-bearing** (call point not gated, alone delete arm ⇒ wasm compile
>   failure), 6 `not(wasm32)` negative arms native three-stage table load-bearing ⇒ D35 envisioned
>   "delete unreachable" **no applicable target** in this file, real cleanup form = whole
>   project-mode subtree gated on wasm32 (registered P10 S7, three unverified retained as-is).
>
> **III. Independent Verification (t5, frozen `29ebd2145` = post-push `d07993157`'s content—two
> revisions per-commit patch-id all equal, whole tree difference only in `docs/src/en/**`,
> `docs/src/.i18n-cache.json` etc. bot product surface—`git range-diff` three commits all `=`))**:
> Gates **8/8 all green**—`cargo fmt --check` 0 ｜
> `clippy --all --all-features --all-targets -- -D warnings` 0 ｜ `cargo test` **3128 passed / 0
> failed** (7 test targets total; baseline 3126 + this batch 2) ｜ corpus diff **348:348** per line
> zero diff (sort diff 0 line) ｜ IR snapshot `UPDATE_SNAPSHOTS=1` re-run then `git status` zero
> drift ｜ `check-obligations.py` 21 = 13 + 8, containment 0 gap ｜ `check-docs-truth.py` source 156
> / document 156, CLI 21 commands consistent ｜ error code generator re-run pre and post tree both
> clean. Build warnings only 4 bot language W1082 missing entries (known state) + Z3 download noise,
> zero code warnings. **Contract verify Stance Correction (must two JSONL paths)**: Corpus gate's
> call stance =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generation>.jsonl`—**two
> JSONL paths both required**; missing second parameter is comparing baseline with baseline
> (constant zero diff, criterion idling). This batch executed and recorded per two-path stance (t5's
> `corpus-r44.jsonl` product).
>
> **IV. Mutation Evidence (3/3, mechanically reversible)**: All via
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per file blob
> compare with HEAD → require `git status` clean):
>
> - (a) Singleton degradation → shared reading predicate hit=0 miss=2, test FAILED exit 101;
>   `backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → restore = HEAD.
> - (b) Delete W1082 emission → pin test FAILED exit 101; `checker.rs` HEAD=`70c04a41e020` →
>   mutation `db0a623c9b9a` → restore = HEAD.
> - (c) Rollback old layer order declaration → reconciliation 2 FAIL exit 1; `layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → restore = HEAD. Three trials after restore
>   `git status` all CLEAN. Trial (c) first run failed due to absolute path parsed failure by WSL
>   exit 127 (not criterion red), changed to relative path re-run as standard.
>
> **V. Review Rounds and Conclusion**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision**—3 findings all document line number anchor inaccuracy (F-01 medium: `02` §"wasm
> current state" 4 group expired line numbers; F-02 low: `02` and `09`'s 4.4.3 delivery anchor
> `checker.rs:1283-1293`; F-03 low: `06`'s control statement), four substantive deliveries all pass.
> → Fix round (t10, local `657c3d5e3` = post-push `e184d88b6`, 3 files 23+/23-, and attached two
> mechanical criteria) → Round 2 (t11, same revision) **verdict = pass, new finding = 0**. **Final
> check basis = round 2's pass**. **Round 3 (closure tip `da724940b`)**: Independent verification
> t12 **verdict = pass** (bound `da724940b`: four rules text byte-by-byte, 3.19 / S7 / S8 fields and
> scope complete, count table and decision chain recalculation, dead reference zero,
> `check-docs-truth` 0, `markdownlint` 0); review t13 **verdict = needs_revision**, 4 findings—F-01
> medium (t10 / round 2 only gave pre-rebase hash, §14's list inconsistent with origin/dev actual),
> F-02 low (`02:379`'s 09 4.2.8 anchor was pushed by this batch itself at `09:144`/`:182` each
> inserted one line to `:274`), F-03 low (`09a` wrote local disk path), F-04 low (three-level count
> row note 157 ≠ table header 156) → fix round t14 (this entry) closes one by one. **t10 contract's
> stance ruling**: Fix surface based on **task record's 5 F-01…F-05** (F-04/F-05 are mechanical
> criteria), dispatch brief not criterion.
>
> **VI. User Ruling (2026-10-18) and Rule Solidification**:
>
> 1. **D58 entry name rewritten in place**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, rewrite body in place, **don't write
>    revision note** (t8 in 4.4.1 one commit completed).
> 2. **wasm judgment registered as P10 S7** (ruling B): with "belonging stage: P10; whether blocks
>    mainline: no" + three unverified **retained as-is**; `06`'s F2 "needs decision" line repointed
>    to that registration; t4 produced "### F2 Judgment" subsection untouched.
> 3. **Crash item registered as P3 3.19** (ruling A): symptom / cause points to
>    `proof/smt/translate.rs`'s sort inference / D59 **A/B class** (crash) / acceptance row (this
>    shape no longer aborts—conservative degradation or diagnostic, corpus and IR snapshot zero
>    drift); **nature = pre-existing** (t5 two revision comparison, see "VII" ⑤).
> 4. **Rule Solidification 1** "correction written in place, no old-new comparison": land
>    `coding-rules.md` (new clause), `07-equivalence-oracle.md` §Semantic correction branch
>    criterion clause 3, RFC-039 **D59 row** clause 3 (same commit registration = WBS row label
>    error type + in-place correct affected body + PR required block "error type" one line).
> 5. **Rule Solidification 2** "contract `inScope` uniformly writes path prefix that can be resolved
>    by path matcher": land `AGENTS.md` team orchestration convention clause 6.
> 6. **New code W1082 approved** (external contract table change, E class).
> 7. **Register P10 S8** "document correction form convergence (in-place rewrite, delete revision
>    note)": includes 7 audit checklist scope and acceptance row.
> 8. `09` top count table: secondary **63** (60 + 3.19 / S7 / S8), three-level **157** (154 + 3),
>    with 2026-10-18 note; three-level item's account review see "XV".
>
> **VII. Process Facts to Be Truthfully Recorded**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks—shared working tree had teammate
> in-progress work, `--fix` would incidentally rewrite others' files; **equivalent gate already
> manually fully run in final tree** (fmt --check / clippy `-D warnings` both 0). ② `pre-commit`'s
> stash window did occur (`Stashing unstaged files…` / `Restored changes from…`), post-commit
> `git status` check shows teammate in-progress work intact. ③ **4.4.3's inScope lesson**: prose
> qualification `docs/src/（only error code generated products）` was rejected by `changedPaths`
> validation ⇒ `inScope` uniformly writes path prefix that can be matched (solidified as rule, see
> "VI" 5). That task **actually 11 files**, of which 7 due to declaration writing couldn't be parsed
> by path matcher not listed in `changedPaths`, listed one by one in task result (`codes/w1xxx.rs`,
> RFC-013 code table, `reference/error-code/` index and each E section page etc.). ④ **W1082's
> locale window closed**: `locales/zh.json` / `locales/en.json` same commit as code, other languages
> supplemented by i18n bot after push (run see "IX"). ⑤ **Z3 `Sort mismatch` crash judgment (t5
> criterion ⑦)**: **pre-existing**—same probe, same input on `b5e68b3c9` and frozen revision
> `29ebd2145` stdout **byte-by-byte consistent**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1; static attribution consistent (4.4.2 0 line changes to `translate.rs`, only
> `cache_stats` migration to `z3_backend.rs`). **Degradation branch production surface unreachable
> exhaustive holds** (W1082 no user-visible surface). Probe three source lines:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **t5 frozen surface and t9's coverage
> difference**: t9 (three comment/document supplements, `1b593e993`) commit later than t5's frozen
> revision `29ebd2145`, zero behavior change, its acceptance covered by t9's own mechanical check +
> t6 review (t6 on `1b593e993` **re-ran** t5's key measurement, not relayed). ⑦ `docs/src/en/**` not
> hand-edited, regenerated by post-push auto-translate workflow (this batch's bot landed two
> commits: `92d490695` i18n locales supplement W1082 other languages, `63b6cdf37` auto-translate
> documents). ⑧ **t9 out-of-scope observation's three rulings**: (i) t9 incidentally fixed `02` same
> sentence's `wasm/src/lib.rs` line count and gating point line numbers (73 / 139·153·170 → 152 /
> 151·170·187)—**accepted** (same stance same number requirement surface, commit message listed);
> (ii) RFC-039 **D35 row** still writes "wasm 27 files' cfg branches"—**keep unchanged** (resolution
> row is historical ruling record, its premise correction already carried by P10 S7 registration, no
> history rewrite); (iii) `docs/src/en/**`'s old entry name translation aligned by auto-translate. ⑨
> `09` / `09a` originally pointing to deleted batch directory's two places in closure round changed
> to **in-place inline** (evidence summary merged into this record's section, probe three source
> lines inlined). **2026-10-21 Correction**: At that time changed to still workspace path,
> equivalent to moving violation down one level; from this round on the summary's delivery commit,
> gate numbers and CI run id all inlined, document no longer references any workspace path (AGENTS
> meta-rule ① / clause 10④).
>
> **VIII. t8 Acceptance ④ Stance Ruling**: Acceptance original required
> `git diff | grep '修订注记\|原为\|现改为'` empty. **Measured default context stance
> unreachable**—D58 row's neighboring rows D59 / D61 resolution rows themselves contain
> "修订注记 / 原为" words (not this task's change). Ruling: per **new line / deleted line / `-U0`
> increment line** three stances all empty accepted, and write **subsequent same-type acceptance
> only grep new/deleted line**—criterion should fall on this change's lines, not whole diff's
> context.
>
> **IX. CI Gate Conclusion (AGENTS clause 13①, scheduled check 2026-10-18 17:26 backfill; run id and
> conclusion in-place inline)**: First push (3 commits, tip = `d07993157`): CI **39110908163**
> success ｜ Deploy Docs **39110908155** success ｜ Auto Translate Docs **39110908193**
> success ｜ Auto Translate I18n **39110908144** success; this tip covers `24028ffe8` (4.4.2) /
> `52d2276ca` (4.4.3) / `d07993157` (4.4.1) three commits all content. Second push (1 commit, tip =
> `1b593e993`): CI **39112293391** success ｜ Deploy Docs **39112293062** success ｜ Auto Translate
> Docs **39112292903** success; covers all four commits including t9 supplement. **No concurrency
> cancellation**, latter covers former. Third push (2 commits, tip = `da724940b`): CI
> **39121538555** success ｜ Deploy Docs **39121538562** success ｜ Auto Translate Docs
> **39121538541** success—three runs' headSha all = `da724940b` and **all terminal** (2026-10-18
> third round scheduled check backfill); round 3 independent verification t12's "CI not terminal"
> observation closed. Unrelated: Nightly **39081392694** failure's headSha = `290f9ebc`, not in this
> batch's commit surface.
>
> **X. Three `pull --rebase` (Automatic Commit Counterpart During Push)**: Counterparts are
> auto-translate and i18n commits; six hashes rewritten and one by one **patch-id all equal**—local
> `1fc135437` → `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`, `1ebddaf95` →
> `1b593e993`, `657c3d5e3` → `e184d88b6`, `c8412565c` → `da724940b` (third rebase post origin/dev
> tip = `da724940b`); `git range-diff` verified **per commit content unchanged** (three all `=`).
> Members didn't execute `git push` / `fetch` / `pull`, sync and push uniformly by team lead.
> **Forensic Reminder**: Subsequently reference commit id uniformly use **post-push hash** (above
> right value); `09`'s check row per convention doesn't write commit id.
>
> **XI. Uncovered Surface (must not write "not tested" as "passed")**: ① doc-tests 0/0 didn't carry
> criterion; ② **wasm32 target actual compile not done** (that surface covered by CI's
> `_build-wasm.yml`, this batch's wasm judgment is static argument); ③ mutation only covers 3 places
> named by contract, other gates no reverse break verification; ④ corpus diff didn't individually
> run on `b5e68b3c9` (only ran on frozen revision); ⑤ i18n bot supplemented other languages' W1082
> text behavior unverified (only verified `zh.json` / `en.json`); ⑥ round 1 review and fix round
> reported **same-type but not in contract findings' existing drift** not fixed (list see "XII"),
> attribution pending user ruling.
>
> **XII. `02` Whole Existing Drift Observation List (user 2026-10-18 agreed to register; 2026-10-21
> landed as S10)**: Stance = all **pre-existing drift** (not introduced by this batch), **doesn't
> constitute this batch's finding**; list found by fix round, round 2 review independently
> spot-checked on `657c3d5e3` (= post-push `e184d88b6`) and gave mapping, transcribed per item (list
> in-place inlined as follows, original files recycled with workspace):
>
> 1. `02` §(a) Entry table 3a-3d / 5 / 5b: `02:72` references `orchestrator.rs:273-399` → current
>    `:155`; `02:73` `:450` → `:250`; `02:74` `:1374` → `:985`; `02:78` `util/diagnostic/mod.rs:565`
>    → `:681`; `lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` no longer at that place;
>    **content level**: `run_diagnostics` and "otherwise manual lex→parse" has been replaced by
>    4.6.1 (delete manual sequence).
> 2. `02:1016`: `compile_project :99-237` → current `:107-134` (28 line Program constructor).
> 3. `02:360-361`: "missing degraded to warning" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435); `build.rs:65-72` already changed to `panic!` hard failure;
>    `_build-wasm.yml:35-59` interval needs re-take.
> 4. `02:127` mechanism pointer: mechanism actually at `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-statement); conclusion still holds (multi-file arm drops warnings), inaccurate
>    is "result only downstream use" sentence (`type_result` also via `arms.rs:1389` →
>    `helpers.rs:259/:636` feeds `overload_resolutions`); suggestion = pointer change to
>    `typecheck_orchestrated`, "only downstream use" change to "warnings field no read point".
> 5. `02:128`: `check_project:385-395` / `250-252` needs re-take.
> 6. `02:334` (§S5 (d) investigation paragraph): referenced old code fragment replaced by 4.4.3's
>    `ownership_inputs` criterion point.
> 7. `02` change list table other rows: `predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This closure doesn't pre-set S9's WBS entry** (user not ruled); after getting ruling then per
>    registration form land `09`. **Subsequent (2026-10-21 landed)**: User 2026-10-18 agreed to
>    register, this list has been as `S10` "02 body and fact alignment (line number anchor + content
>    level expired)" landed `09` (secondary +1 / three-level +1 = S10.1)—**originally planned S9
>    number was occupied by "pre-release audit's abstraction closure"** (S9.1 = issue #473 / S9.2 =
>    issue #474), per 09 "number must be unique in whole document" registration convention delayed
>    to S10. **Registration timing deviated from original plan** (originally wrote "land with P5's
>    closure document commit"), per user 2026-10-21 instruction this batch lands immediately.
>
> **XIII. Batch Tree Recovery (gc and status original output; tree path in output recorded as
> warehouse relative path)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted yx44-evidence (released about 0.0 MB)` /
> `[scratch] gc complete, total released about 0.0 MB`; `python scripts/dev/scratch.py status` →
> `yx44-evidence            holder=-` next line `<workspace>/yx44/tree`—registry recorded path
> **doesn't exist**, this name is registry residual (`gc`'s `_remove` deletes `meta["path"]`'s
> parent directory i.e. workspace's `yx44`, residual `meta.json` still falls in scan surface; its
> content batch=`yx44`, `base_rev=29ebd2145`, `lock=null`, contains t5 / t6 / t11 holding history);
> Closure round renamed that registry file to `meta.retired.json` to make it exit scan (**not delete
> any evidence file, not create any new tree**) → `python scripts/dev/scratch.py status` =
> `[scratch] no batch tree`; re-run `gc` = `[scratch] no batch tree, no recovery needed`;
> `git worktree list` only main mount (`dev`). Batch original evidence directory (no `meta.json`,
> **not batch tree**) kept as archive at that time; **from 2026-10-21 workspace directory no longer
> as evidence source**—closure summary inlined in this section, original directory deleted with
> cleanup (AGENTS meta-rule ①). Acceptance statement (resolvable): `git status` clean; batch tree
> after gc `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main
> mount; no this-batch new directories outside warehouse.
>
> **XIV. Document Real Build and Commit Hygiene**: `npm run docs:build` local **exit 0** (AGENTS
> clause 10⑨—markdownlint can't catch VitePress's Vue compile error, and only docs commit will be
> judged `no files to check` skipped markdownlint); this batch's check / note / registration / rule
> modification accumulated into **one** document commit (clause 8), landed at batch closure; members
> didn't execute `git push` / `fetch` / `pull`. **This batch's final two both pushed** (closure tip
> `git rev-list --count origin/dev..HEAD` = 0, origin/dev tip = `da724940b`): `e184d88b6` (round 1
> findings' document anchor fix, 3 files 23+/23-) + `da724940b` (closure document commit, 7 files
> 96+/10-; including check / record / three place path rewrite / registration and rule
> modification).
>
> **XV. Count Table Account Review (Round 3 Fix F-04; method and result both verifiable)**: Judgment
> object = `09` top "Location and Scope" table's three-level count row. **Method** (two independent
> stances, per revision use `git show <rev>:<file>` take byte recalculation): ① **Sum per secondary
> entry**—take each secondary kanban row's bracket "N three-level tasks" (row start `- [x]`/`- [ ]`
> bold title start, row contains "个三级任务", including S7 row inline written "1 个三级任务"):
> current tree 70 secondary entries, **sum = 157**. ② **Historical self-consistency**—same stance on
> `366bc5a69^` got 66 entries, **sum = 151 = current period table header 151** (self-consistent);
> `366bc5a69` that day newly registered `4.9.6` and `0.4.1` two three-level (sum 151 → 153), but
> table header only wrote 151 → 152 ⇒ **table header from that commit undercounted 1**; each
> subsequent revision all off by 1 (`92d490695` / `1b593e993` / `d07993157` / `e184d88b6`: sum 154
> vs table header 153; `da724940b`: sum 157 vs table header 156). **Result**: truth = **157**, so
> correct table header (`09:13` three-level 156 → 157); note 144+2+2+2+2+1+1+3 = 157 originally
> correct, not changed; pre-this-batch base = **154** (not 153). **Secondary count row not
> touched**—this row's stance never aligned with kanban entry count (on `366bc5a69^` it's 66 entries
> vs table header 59), this time no verifiable evidence overturning 53 + 10 = 63, not changed for
> number fitting. Incidental correction (same round): §2's 4.4.1 bracket decomposition per
> reconciliation script actual item count changed to "README side 4 + code side 7 = 11", consistent
> with **11/11** and script 11 `ck` calls.

## P3 Debt Round 1 Closure (3.5 / 3.6 / 3.4.6 / 0.5)

> **I. Batch Composition and Rounds**: This batch closes P3 side remaining debt—**3.5 higher-order
> type parameter call path** (3.5.1–3.5.3), **3.6 function type value call path** (3.6.1–3.6.4),
> **3.4.6 compile path consumption warnings** (D class, user 2026-10-19 ruling "do it"), and
> together registers **0.5 test suite layering** (P0, issue #486 slice) and **3.20 dead code
> analysis existing false positives** (3.20.1 / 3.20.2). Roles and rounds:
>
> - Implementation: 3.5.1–3.5.3 = t1 ｜ 3.6.1 = t2 ｜ 3.4.6 = t6 ｜ 3.6.2 = t7 ｜ 3.6.3 = t8 (main +
>   closing two commits) ｜ 3.6.4 = t9 ｜ 0.5.1 = t10 (#486 slice)
> - Independent verification: round 1 = t3 (frozen revision `81d5e414e`, covers 3.5 / 3.6.1–3.6.3 /
>   3.4.6) ｜ 3.6.4 = t11 (round 2, frozen `8e32e8636`)
> - Review: round 1 = t4 (verdict = **needs_revision**, F1/F2/F3) ｜ fix round = t12 (one
>   commit) ｜ round 2 = t13 (verdict = **pass**)
> - Closure = t5 (check / register / this record / batch tree gc / document real build / one
>   document commit)
>
> **Failure and Redo**: t4 terminal failed with needs_revision (review round per contract must fail
> and give findings, not accident) → t12 real fix → t13 review pass. t8 internal one attempt
> interruption (process crash), previous round's evidence kept in batch directory, continued attempt
> completed then two commits in. No cancelled rounds.
>
> **II. Per-task Production and Measurement Numbers (conclusion stance; original commands and output
> see each batch evidence directory)**:
>
> - **3.5.1** (unifier matches constructor variable at head position): criterion converged to
>   **single implementation** `is_ctor_reference` (`pub(super)` free function, warehouse 1
>   definition 3 consumption)—this is t12's real fix for t4's F1 (originally
>   `inference/statements.rs` had another same-name method copy, constituting prohibition three's
>   same-name two-implementation). Head rename no longer in-place copy: `environment.rs` and
>   `expressions.rs` each first traverse TypeRef / variable table to construct head_map, then
>   uniformly hand to shared implementation `substitute_ctor_heads` (`types/substitute.rs`) to
>   execute—two head_map construction criteria unchanged, rename only acts on head per name, empty
>   table returned as-is (t12's F2 real fix). Equivalence argument: traversal doesn't change Generic
>   head name.
> - **3.5.2** (explicit type argument accepts type constructor, E1010 disappears): red state E1010 /
>   E1002, green state exit 0.
> - **3.5.3** (mono product replaces constructor variable): symbolic type argument criterion
>   narrowed per container generic parameter name; product's constructor variable replaced with real
>   constructor (IR dump verified: specialized function `Function probe(Box, int64)`).
> - **3.6.1** (local function type value called by name): criterion single source =
>   `is_static_fn_name` (warehouse 1 definition 1 call), aligned with variable evaluation path's
>   name resolution priority (local → global), no second resolution table added; IR verified: module
>   initialization section changed from `LoadConst String("f")` + `CallNative` to `LoadGlobal` +
>   `CallDyn`. Regression surface: same-name "function binding + value binding" unreachable
>   (uniformly compile-time E2010), non-function value called by name still E1065.
> - **3.6.2** (`use` alias imported function value called by name): originally reported
>   `E6006 Native function not found: lib.f` (`use lib.{f}` and `use lib.{f as g}` two forms). Fix =
>   in `is_static_fn_name` **add one arm** (not copy), new sole implementation
>   `use_alias_value_slot` undertakes "use alias → qualified name → global slot" two-table joint
>   lookup, consumption point 2 places (call form criterion + variable evaluation path).
> - **3.6.3** (nested function body references outer frame value): **criterion change of
>   judgment**—not "let it run right", but this form per RFC-009 §2.3 + SPEC §12.3 should be
>   **compile-time rejected** (D59 C class: change silent wrong value to loud; per `09` §P3 D59 row,
>   "silently give wrong answer" goes to C class—this batch closure brief records as "A/B class",
>   both mean "old behavior not success", landing method same as "red before green + compile-time
>   loud rejection"). Original form = **no error, calculate wrong value (void)**: red state stdout
>   `void`, EXIT=0 (silent wrong value archive); green state compile-time E1001 `Unknown variable`,
>   EXIT=1, with existing "unknown variable" path **same code same text**. Criterion single source =
>   capture barrier (`enter_capture_barrier`), compile-time name (type parameter) doesn't block;
>   E1001 still only emitted in typecheck, registered sole. **Assignment path held by E2010** (t8's
>   closing commit: declaration / assignment judgment doesn't apply capture barrier)—otherwise this
>   fix itself would open new "same-name assignment silent wrong value" channel (c1 form: only
>   barrier degenerates to exit 0 print 10).
> - **3.6.4** (`use lib` whole import then `lib.f(...)`): goes through FieldAccess callee's
>   namespace branch, originally reported E6006 `lib.f`. Fix = namespace branch extracts qualified
>   name then first judges global slot layout → change to indirect call; qualified name resolution
>   and slot lookup reuse `resolve_namespace_data_access` (same source as data access), no new
>   resolution table.
> - **3.4.6** (compile path consumption warnings, D class): before and after change external
>   performance = **multi-file `run` previously never output any W code** (red state archive:
>   multi-file [] vs single-file [(W1001,3),(W1002,5),(W1003,1)]), now four paths (single-file run /
>   multi-file run / single-file check / multi-file check)'s W code set constant = {W1003@1,
>   W1001@3, W1002@5} (non-empty set). Corpus baseline: 348 records **46 changes**, new W code
>   **118** (W1001 34 / W1002 12 / W1003 13 / W1004 42 / W1005 17), **118/118 attributed per item to
>   real unused definitions in file (0 unlocated)**, pure increment (0 deletions, 0 exit / stdout /
>   check changes). **D59 four items fulfilled**: ① red before green (red state archive + turn pin
>   test) ② doesn't change "originally successful program"'s external performance (pure increment, 0
>   exit / stdout / check changes) ③ same commit registration (this record + `09`'s 3.4.6 row) ④
>   baseline same commit regenerated and manually reviewed diff. **Superset approved**: W1004 /
>   W1005 / W1006 also enter `run` (user 2026-10-19 approved), reason = **one criterion, one
>   channel**—only putting W1001/W1002/W1003 equals creating second filter ruling. **External
>   contract surface change**: `orchestrator::compile_project` return value changed from
>   `Result<ModuleIR, _>` to `Result<(ModuleIR, FileDiagnostics), _>` (warehouse 2 consumers + 2
>   test direct drive points sync adaptation).
> - **0.5.1** (test resource profile marking + exclusive lock + fast/full minimum consumption
>   surface): added `scripts/dev/testsuite.py` (`markers --check` and `cargo test --list`
>   bidirectional reconciliation, `plan` budget binning, `run fast|full` budget insufficient reports
>   gap not silently skip, `manual` file never implicit execution) and `src/util/test_support.rs`
>   (cross-process file lock, lock taken in test body ⇒ any call path mutually exclusive); in-place
>   `cost:` mark 3 places (2600 file exclusive, peak extrapolated from 5000 file measured linear,
>   **not measured**; 5000 file exclusive, 15.6 GiB / 72.6 s measured; snapshot update manual, peak
>   and time both extrapolated). Self-verification numbers: `markers --check` check 3149 `#[test]`,
>   3 mechanical mutations each exit 1, `run fast` batch wave 2793 passed / 47.7 s + exclusive wave
>   1 passed / 14.6 s, budget 1 GiB `run full` lists gap 7.11 GiB, two-process lock test
>   `held→blocked→held(waited 2.0s)`. **Independent verification belongs to #486 subsequent
>   scheduling** (this batch only implementer self-verification). **0.5.2 pending** (pre-commit / CI
>   wiring + 2600 file measured calibration + `test-specification` detailed rules).
>
> **III. Independent Verification (t3, frozen `81d5e414e`; red state baselines `c599de7bb` /
> `1725d0215` / `ef2334de5` all HEAD ancestors, `git merge-base --is-ancestor` verified)**: probes
> all self-built by verifier (batch tree built by `scripts/dev/scratch.py init` in workspace, probes
> and patches in its `probes/` / `patches/` subdirectories), not relaying implementer's conclusion;
> that batch tree together with its in-tree evidence deleted in this batch's closure gc (see "IX"),
> so this section **in-places the measurement conclusions and numbers** for record. 3.5 red state
> four probes all report E1003+E1002 / E1010, green state exit 0 and value correct (stdout 1 / 7 /
> 1), with IR value-level verification (specialized function `Function probe(Box, int64)`); 3.6.1
> green state output 6 / 10, control "value position call" green, "non-function value called by
> name" compile-time E1065; 3.6.2 red state two forms E6006, green state exit 0 (probe's `assert`
> failure means non-0, exit 0 means value correct); 3.6.3 red state stdout `void` / EXIT=0 (silent
> wrong value verifiable), green state compile-time E1001 / EXIT=1, while **expression position
> inline lambda capturing this function parameter** allowed (EXIT=0, output 6)—allowed surface not
> mistakenly hurt. 3.4.6's C2 normalization independently reproduced by verifier's self-built
> fixture (four-path W set identity, non-empty), baseline diff independently recalculated
> (changed=46, added=0, removed=0, expectation / run.exit / run.stdout / check change = 0, new W
> code 118 consistent with implementer), attribution sample 10 all hit real definitions. Gates on
> frozen tree all green: fmt / clippy (`-D warnings`) / `cargo test --all` (**lib 2790 passed + 2
> ignored, integration 297 passed**, 0 failed) / corpus diff 348:348 diff empty / IR snapshot 204
> zero drift / obligation ledger / docs-truth. **t11 (round 2, frozen `8e32e8636`)** independently
> reproduced 3.6.4's red→green (red state parent commit 16e2a5c88: E6006 `lib.f`, EXIT=1; green
> state EXIT=0, stdout 27, probe's 4 assertions all pass), criterion single source
> (`resolve_namespace_data_access` 1 definition 2 consumption, no third "qualified name → slot"
> joint lookup), no regression eight forms all pass, verdict = **pass**.
>
> **IV. Review Rounds and Conclusion**: Round 1 (t4, judgment of `20abd3ebc`'s parent revision)
> verdict = **needs_revision**, three findings all real defects: F1 = `is_ctor_reference` same-name
> two copies (`inference/statements.rs` method copy), F2 = head rename inlined one each in
> `environment.rs` and `expressions.rs`, F3 = visibility out-of-bound (should-be-private not
> private, should-be-`pub(super)` not converged). → Fix round t12 (one commit, 4 files +96/−85) →
> Round 2 (t13, reviewed revision `20abd3ebc`) verdict = **pass**, new finding = 0: 8 self-built
> probes' values and exit codes per item same as pre-fix, 4 IR dumps **byte-by-byte consistent**
> with round 1 green state, corpus 348/348 zero drift, IR snapshot 204 zero drift,
> `cargo test --all` EXIT=0 (lib 2794 passed + 2 ignored, integration 297 passed, 0 failed), fmt /
> clippy / docs-truth all exit 0, `check-boundary`'s `pub(crate)` count 167 / frontend-core 26 (=
> 3.5 pre-baseline), `check-fanout` same-name definition zero hit. **Final check basis = round 2's
> pass**.
>
> **V. Mutation Evidence (all via `python scripts/dev/scratch.py trial`, apply → run command → apply
> -R → per file blob compare + require `git status` clean)**:
>
> | #   | Target                                        | Post-mutation blob | Capture result                                                                                                                                                 |
> | --- | --------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | M1  | 3.6.1 `is_static_fn_name`'s global lookup arm | `022986cbea65…`    | Two probes reproduce E6006, EXIT=1 (post-restore blob = HEAD, CLEAN)                                                                                           |
> | M2  | 3.5.2 explicit type argument branch           | `abbcc5ecf116…`    | Probe check reports E1002, EXIT=1 (control function form still green ⇒ attribution precise to 3.5.1)                                                           |
> | M3  | 3.4.6's `MULTI_FILE_STAGES` dead code arm     | `372f7b92755d…`    | run channel W code all extinct (C2 red); M3b attribution shows check channel criterion intact, broken is exactly 3.4.6's newly opened compile external channel |
> | M4  | 3.6.3 capture barrier entry                   | `7a7204c60383…`    | Capture class turn pin test 4 failed, TEST_EXIT=101                                                                                                            |
> | M5  | 3.6.4 value slot → CallDyn block              | `25853376639b…`    | Probe reproduces E6006, EXIT=1 (blob chain `0d837e9d30de…` → `25853376639b…` → `0d837e9d30de…`)                                                                |
> | M8  | 3.5.1 turn pin test                           | —                  | Post-fix re-run still can fail (E1003, TEST_EXIT=101, mechanically reversible restore CLEAN) ⇒ criterion didn't lose failability                               |
>
> **VI. User Ruling and This Batch Registration**:
>
> 1. **3.4.6 = D class** (externally observable: multi-file `run` new W code), user 2026-10-19
>    ruling "do it"; **superset (W1004 / W1005 / W1006) also approved**, reason = one criterion /
>    one channel. Original 2026-10-15 ruling C "per decision B1 deferred"'s **prerequisite** (4.9
>    pool semantic correction) closed 2026-10-17.
> 2. **3.6.3 criterion change of judgment**: from "let it run right" to "nested function body
>    references outer frame value → compile-time E1001"; assignment path held by E2010.
> 3. **Two "pre-this-task" silent forms registration not changed**: ① Same-block sibling nested
>    function reference—baseline is E1001 (outer block local family), this batch doesn't expand
>    scope; ② Nested function annotation references outer type parameter still prints `void` (t1
>    registered item). Both **are not** introduced by this batch, both retained as-is.
> 4. **3.20 "dead code analysis existing false positives" registration** (user 2026-10-19 agreed per
>    C disposition then registered): 3.20.1 = W1001 doesn't track closure-internal reference
>    (`docs/src/reference/stdlib/test.md`'s `t_add` form: referenced in closure but reports unused);
>    3.20.2 = W1003 judges variant destructuring required import as unused
>    (`docs/src/reference/stdlib/weak.md`'s `use std.option` form: delete i.e. E1002). Two forms
>    also reproduce on pre-3.4.6 baseline, **not introduced by 3.4.6**. **Rollback Obligation
>    (written dead)**: After this group is fixed must put `weak.md` / `test.md`'s examples rewritten
>    to bypass false positives **restore to natural writing** and **regenerate baseline** (baseline
>    will move once more).
> 5. **0.5 test suite layering** (P0, issue #486): 0.5.1 completed; 0.5.2 (pre-commit / CI wiring +
>    2600 file measured calibration + `test-specification` detailed rules) pending.
> 6. **#489 ureq 3 migration**: dependency surface tracking item, **doesn't enter WBS**;
>    `.cargo/audit.toml`'s `RUSTSEC-2026-0381` exemption **with hard removal condition**—after ureq
>    migrates to >= 3 delete that ignore, if still red at that time cannot extend period, must first
>    fix dependency. 0.26 line has no fix version (0.26.12 doesn't exist), ureq 2.x has no updated
>    version.
> 7. **3.20 number conflict disposition (this round new registration)**: 3.20 number originally
>    occupied by 2026-10-18 registered "pre-release audit (v0.8.4) supplementary registration gap"
>    (3.20.1–3.20.4), and user 2026-10-19 ruling registered "dead code analysis existing false
>    positives" needs 3.20. Whole document number uniqueness rule doesn't allow two 3.20, so **move
>    pre-release audit group entirely to 3.21** (three-level change 3.21.1–3.21.4, issue anchor
>    #468–#471 unchanged, entry content unchanged), 3.20 number given to "dead code analysis
>    existing false positives". Migration only changes number, doesn't change any registered content
>    and attribution; historical commit message not rewritten (migration fact recorded in this
>    record and `09`'s count table note).
>
> **VII. Process Facts to Be Truthfully Recorded**:
>
> 1. **Two inScope narrow each traded for one rework round**: ① 3.6 series implementer wrote inScope
>    narrow to `ir_gen.rs` predicate layer, didn't cover `types/`'s shared replacement
>    implementation → review round 1's F2 found at landing that head rename inlined one each in two
>    files, needed another convergence round; ② 3.5's fix round moving changes between `types/`
>    shared implementation and two call sites, again hit inScope and actual change surface mismatch
>    (AGENTS.md clause 6: inScope must cover mechanical requirement surface, path prefix can be
>    resolved by matcher). Both reworks are caused by "implementer inScope too narrow", **not
>    implementation defects**—but cost is two rework rounds and one review findings.
> 2. **External Sync (2026-10-19 13:08 `pull --rebase --autostash origin dev`, 13:19
>    `pull --tags origin dev`) and V0.8.5 Release Commit**: User 2026-10-19 confirmed as **normal
>    release process, completed, doesn't affect this team**; member commits intact without loss.
>    Evidence (`git reflog --date=iso` / `git reflog show origin/dev --date=iso`): 13:08:43's
>    `pull --rebase --autostash` picked up is release chain's "supplement CHANGELOG retention
>    strategy's criterion and self-check", 13:19:34's `pull --tags` is fast-forward; V0.8.5's
>    release commit (`:twisted_rightwards_arrows: V0.8.5: …` and its `:bookmark:` companion commit,
>    CHANGELOG generation and retention strategy two commits) all within `v0.8.5` tag reachable
>    range.
> 3. **Three rebase caused local hash rewrite (referencing old number will miss)**: `b3245aac9` →
>    `56ae469e1` (3.6.3 main), `415f0154b` → `ef2334de5` (3.6.2), `bdbe859e1` → `8e32e8636` (3.6.4).
>    Verifier proved three are **pure rebase equivalent** with "src/, tests/ tree diff empty"
>    (content unchanged, only parent chain changed), this record uniformly uses **currently
>    reachable** hash.
> 4. **F2's unfixable part**: 3.5.1's commit message wrote "sole implementation" is history, **not
>    rewritten**; correction can only be borne by "shared implementation convergence's that commit +
>    code comment per fact correction"—this record keeps this sentence, lest readers mistakenly
>    think no one found this problem at that time.
> 5. **Three observations (verified, not blocking, but registered to avoid being mistaken for
>    unhandled violation later)**: **O1** After head rename converged to shared implementation,
>    coverage additionally drills down `MetaType` / `AssocType` / `Literal`, but three coverage
>    probes **all parse failure** (`Pipe` / `Ampersand` / old `Type[...]` syntax; `ast::Type::Union`
>    has no construction point in parser) ⇒ **stronger in design, currently unreachable**, not
>    behavior change; **O2** `check-fanout.py` after fix round new reports
>    `add_type_param: 2 files`, introducer is 3.6.3's closing commit (`inference/scope.rs`) **not**
>    this batch's fix round, review judged as **heuristic false positive** (delegate wrapper + sole
>    implementation's division of labor, same type as existing `add_param`); **O3** `09`'s 3.6 row
>    original still writes "1 three-level task" but actually has 3.6.1–3.6.4, 3.5 three rows missing
>    completion note—supplemented by this closure round together (see `09`).
> 6. **`src/driver/arms.rs`'s "Decision B1" comment not rewritten same commit as code (carryover,
>    this batch not fixed)**: 3.4.6 has changed `typecheck_orchestrated`'s FailFast branch to
>    **temporarily store** `result.warnings` (originally directly discarded), but that function's
>    above doc comment still writes "`result.warnings` discarded—Decision B1", opposite to
>    post-3.4.6 code fact; same source's another comment (`src/driver/tests/program.rs`'s stage
>    table description) **has been rewritten same commit as 3.4.6** to "Decision B1 'dead code
>    family warning doesn't enter compile path' hereby revoked". This batch `src/**` out of scope,
>    so **not changed** that comment, registered for next team (belongs to AGENTS.md clause 6
>    "directly named by ruling's declaration carrier" type debt). `02`'s runtime behavior table B1
>    row has been corrected in place to current state (dead code family warning now enters MultiFile
>    `run` path through same criterion).
> 7. **Members didn't execute `git push` / `fetch` / `pull`**: sync and push uniformly by team lead
>    (in `git reflog show origin/dev` each is "update by push", member side has no push record).
>
> **VIII. Uncovered Surface (transcribed as-is, must not write "not tested" as "passed")**:
>
> - **t3's 7**: ① wasm32 target actual compile not done; ② performance impact not measured (3.4.6
>   added two stages, 3.6.x's indirect call overhead both have no baseline number); ③ **W1005
>   per-item attribution not independently reproduced** (verifier's message form regex didn't cover
>   its text form, only verified new W code total 118 consistent); ④ 3.6.3 closing commit
>   (declaration / assignment no barrier) not individually mutated, c1 form not independently
>   re-tested; ⑤ t8's 29 probe matrix not fully re-run (only re-tested three representative forms +
>   M4 turn pin); ⑥ 3.6.4 belongs to round 2 scope, round 1 not independently measured; ⑦ CI gate
>   per convention not waited in session, handed to closure session unified check.
> - **t11's 4**: ① Qualified name callee with generic argument (`lib.g(Int)(3)`) and named argument
>   call function type value—no warehouse sample, this round didn't construct re-test; ② Under
>   `use lib` whole import's `p.get_x()` without item import (existing E6006 boundary, earlier than
>   this task) not re-tested mutation state; ③ On `8e32e8636` didn't repeat full gate (covered by
>   t9's gate log and t3's all green on `81d5e414e` containing that commit); ④ CI handed to closure
>   session check.
> - **This closure round new registration**: This round's document changes **didn't re-run
>   `cargo test --all`** (change surface is `docs/**` three files, zero code surface); closure
>   round's gate is `npm run docs:build` + `check-docs-truth.py` + batch tree recovery +
>   `git status` check.
>
> **IX. Batch Tree Recovery (gc and status original output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted verify-p3-t3 (released about 13220.5 MB)` /
> `[scratch] deleted verify-p3-t4 (released about 14428.0 MB)` /
> `[scratch] gc complete, total released about 27648.5 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`; `git worktree list` → `E:/git/YaoXiang 20abd3ebc [dev]` (only main
> mount left). **Also deleted by gc are two batch trees' internal verification evidence**
> (`verify-p3-t3/` contains independent verification's probes, patches, original logs and conclusion
> text; `verify-p3-t4/` contains review round 2's evidence)—they are **registered batch
> directories**, deleted with tree; this section's third, fourth, fifth have in-placed their
> conclusions, mutation table and gate numbers, conclusions don't become invalid due to file
> deletion. **Unaffected archive** (non-registered batch directories, gc doesn't touch): each
> batch's evidence directories `wbs35` / `wbs35repair2` / `wbs346` / `wbs36` / `wbs362` / `wbs363b`
> / `wbs364` / `wbs486` (under workspace `target/`, **not checked in**; conclusions per item landed
> in this record, workspace path not as reference source). Acceptance statement (resolvable):
> `git status` clean; `python scripts/dev/scratch.py status` = no batch tree; `git worktree list`
> only main mount; no this-batch new directories outside warehouse.
>
> **X. Document Real Build and Commit Hygiene**: This round's check / note / registration / document
> correction **accumulated into one** document commit (AGENTS.md clause 8), landed at batch closure;
> generator regenerated document merged into its functional commit. Before commit locally run
> `npm run docs:build` (AGENTS.md clause 10⑨—markdownlint can't catch VitePress's Vue compile error,
> and only docs commit will be judged `no files to check` skipped markdownlint) and
> `python scripts/ci/check-docs-truth.py`. **Commit id taken after push**: This record per
> historical record convention records each round actually occurred commit and hash (**note division
> of labor with `09` check row**—`09`'s check / completion registration only writes "task id +
> evidence (gate numbers, issue number, file:line)" not commit id, because `pull --rebase` rewrites
> local hash; this record is the round's process historical record, keeps hash and gives rebase
> before and after comparison, see "VII" 3). **CI / Deploy Docs conclusion to "unified check"
> session**, not waited in session (AGENTS.md clause 13)—this batch's CI run id and conclusion
> backfilled by team lead at unified check.
>
> **XI. Count Table Account Review (this round's stance)**: `09` top's secondary / three-level count
> table follows "**base + increment**" stance (same as 2026-10-18's registration). This round's
> increment = secondary +2 (0.5 / 3.20, of which 3.20 is after migration new number, pre-release
> audit group changes to 3.21 is net zero), three-level +7 (3.6.2 / 3.6.3 / 3.6.4 / 0.5.1 / 0.5.2 /
> 3.20.1 / 3.20.2). **Remaining pre-existing deviation (this round not corrected, for subsequent
> account review)**: With "sum per secondary entry bracket 'N three-level tasks'" as stance
> recalculation, this tree gets 162, differs from table header by 7—difference from 2026-10-18
> registration's new three-level but not sync rewrite parent entry count note's 5 places (4.9 writes
> 7 actually 8, 3.14 writes 1 actually 2, 4.10 writes 3 actually 5, 4.4 writes 3 actually 4, S2
> writes 1 actually 2) and 1 place reverse deviation (7b writes 6 actually 5), another 1.2 / 1.3 /
> 9.1 / S7 four parent entries originally don't carry that note (not counted in stance). This
> difference **not introduced by this batch**, this batch doesn't change numbers for others'
> entries, only registered here.
>
> **XII. Carryover Pending (this batch registered, not fixed)**: ① `src/driver/arms.rs`'s "Decision
> B1" comment opposite to post-3.4.6 code fact (see "VII" 6, `src/**` out of scope); ② 3.20.1 /
> 3.20.2 two existing false positives (after fix must restore `weak.md` / `test.md`'s natural
> writing and regenerate baseline); ③ 0.5.2 (pre-commit / CI wiring + 2600 file measured
> calibration + `test-specification` detailed rules); ④ #489 ureq 3 migration (dependency surface,
> exemption with hard removal condition); ⑤ 3.8 (native shadowing of user function name) and 3.12
> (executor configuration propagation between runtime modes) pending ruling, 3.18 prerequisite
> pending P6—**all not in this team's scope**.

## P3 3.17 Closure (Round 2: 3.17.1 Hand-annotated FFI Named Argument + Second Binding Criterion Merge)

> **I. Scope and Round Relationship**: This section is **round 2**'s closure registration, scope =
> **3.17.1** (issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)), doesn't include other
> secondary tasks. Round 1 (3.5 / 3.6 / 3.4.6) closure see previous section "P3 Debt Round 1
> Closure"; the two rounds' relationship stated here: **Round 1's review round 1 (t4) that
> needs_revision (F1 / F2 / F3) has been covered by t12 fix round + t13 review round 2 (verdict =
> pass), not repeated in this section**. This section's own criterion chain = implementation **t15**
> (attempt 2 delivered; **previous attempt due to provider timeout, zero product**) → independent
> verification **t16** (verdict = pass) → adversarial review **t17** (verdict = pass, **no unhandled
> findings**, 4 observation items registered).
>
> **II. Per-item Production and Measurement Numbers (3.17.1)**:
>
> - **Parameter name table same source**: `add_native_signature` changed to take **signature
>   string** and return `Result`—`signature::parse_signature` produces type, sole exit
>   `signature::export_signature_param_info` same-pour "parameter name + required floor"; key and
>   signature table same (same name). Warehouse zero callers so signature can be changed; document
>   explicitly writes "only callers that can get `MonoType` should parse signature string on their
>   own side then go through this API", and this function **must not** parse parameter name itself.
> - **Red before green** (red state measured on parent commit `537b46b74`): hand-annotated surface
>   four forms + duplicate name + legal call total 6, **5 failed, diagnosis all `[]`** (named
>   argument silently discarded); green state same file **12 all pass**—four forms E1014 / E1010 /
>   E1111 / E1111, duplicate name E1015, legal named call and optional position omission allowed.
> - **Three call surface same criterion same code**: hand-annotated FFI (typecheck stage) / ordinary
>   user function (IR stage) / stdlib native export (`assert.assert`) three surfaces **per code
>   equal assertion** (E1014 / E1010 / E1111; E1015 see hand-annotated and user surface). Different
>   observation stage only reflects "who discovers first"; host native doesn't enter registry, IR
>   stage inherently unresolvable (**pre-this-task embedded surface limitation**).
> - **Criterion single source**: `bind_args_to_params` **1 definition**
>   (`inference/arg_binding.rs:35`) + **6 calls** (typecheck 2 + ir_gen 4); second criterion
>   `build_arg_slots` **0 places** (deleted, `ir_gen.rs:516` only has comment explaining its
>   history); `ir_gen` only has pure transport `slot_order_arg_indices` (`ir_gen.rs:521`, 1
>   definition 2 calls, zero judgment). Line count: `ir_gen` **+58 / −92 (net −34)**; warehouse 4
>   files **+448 / −102** (of which 353 lines are new tests).
> - **No regression**: 8 CLI probes (u1–u4 / d1 / d2 / n1 / n3) before and after mutation **per code
>   consistent**; `ir_gen` blob `0d837e9d…` ↔ `ffe1af8e…` check. Merge initial one self-injury
>   fixed: IR side re-judge "missing parameter" falsely reported E1010 for `t.pick2(a = 2)` legal
>   overload (named_args unit test caught) ⇒ missing parameter judgment goes to typecheck, IR only
>   replays slot binding; after fix 19 named_args + 12 new unit tests all green. Corpus 348-348 zero
>   drift, IR snapshot zero drift.
>
> **III. Independent Verification (t16, frozen `c3082a160`; red state baseline `537b46b74`, both
> verified by `git merge-base --is-ancestor` as HEAD ancestors; probes all self-built)**:
>
> - **Red→green independent reproduction** (hand-annotated FFI surface): self-built independent Rust
>   unit test `verify_host_ffi.rs` (4: unknown name E1014 / slot conflict E1111 / named then
>   positional E1111 / legal named call allowed), via `scratch.py trial` temporarily hung into
>   `tests/mod.rs` (apply → test → apply -R → CLEAN). Two states each use **their own interface
>   form**'s probe variant (parent commit `add_native_signature(name, MonoType)` old signature / fix
>   state `(name, &str) -> Result` new signature—interface itself is fix surface): parent commit **3
>   failed / 1 passed**, 3 failed diagnosis **all `[]`** (`got []`), TEST_EXIT=101; fix state **4
>   passed / 0 failed**, TEST_EXIT=0. Red state also in fix tree undoing fix's mutation state
>   reproduced (`M7v2` merge patch: undo fix + undo implementer test + hang self-built old API
>   probe)—3 failed `got []`, TEST_EXIT=101, after restore four file blob consistent with HEAD,
>   `git status` CLEAN, after undo `ir_gen.rs` blob = `0d837e9d30de` (i.e. parent commit original
>   blob). CLI surface (user function + stdlib) two states per code consistent—matches expectation:
>   these two surfaces 3.16.1 already loud, this commit's fix surface only in hand-annotated FFI.
> - **Four forms per item** (CLI self-built probe, `c3082a160` measured): parameter name error E1014
>   `has no parameter named x (available: a, b)`, missing required position E1010, positional and
>   named slot conflict E1111, named then positional E1111; additional: duplicate name E1015, stdlib
>   unknown name E1014, stdlib missing required position E1010 (help points to cond), stdlib slot
>   conflict E1111, legal named call EXIT=0 output 3/3, omit optional position EXIT=0.
> - **Criterion really deleted not renamed**: `git show c3082a160:src/middle/core/ir_gen.rs` in
>   `build_arg_slots` **0 hit** (parent commit same file 3 hit, definition at :521);
>   `slot_order_arg_indices` (fix state :521) per line check =
>   `sort_by_key(|&arg_idx| positions[arg_idx])` pure sort, no position/name/optional position
>   judgment residual, doc comment explicitly states "no position judgment, no name judgment, no
>   optional position judgment"; deleted 71 line judgment block (parent commit :521-593, six rules:
>   empty table loud / positional occupies slot / order illegal E1111 / unknown name E1014 / slot
>   conflict E1111 / enter slot) doesn't exist; `bind_args_to_params` 1 definition 6 calls
>   (typecheck 2: `expressions.rs:2854` / `:5653`; ir_gen 4: :7828 / :7926 / :8021 / :8351),
>   consistent with implementer report; parameter name sole exit = `export_signature_param_info`
>   (environment.rs hand-annotated / typecheck::mod stdlib / ir_gen lookup), no second self-parse.
> - **Self-injury regression pinned** (`t.pick2(a = 2)`): `git show c3082a160 --name-only` **doesn't
>   contain `named_args.rs`** (4 files: environment.rs / tests/mod.rs / native_param_names.rs /
>   ir_gen.rs) ⇒ pin test not changed; independently re-run that case 1 passed, named_args full set
>   **19 passed / 0 failed**.
> - **No regression gates** (`c3082a160` tree): `cargo test --all --quiet` (4×4 stance) EXIT=0—lib
>   **2806** + 2 ignored, integration **297**, 52/44/9/2/1/9 all green 0 failed; `cargo fmt --check`
>   / `clippy -D warnings` both EXIT=0; corpus diff (dual parameter) 348/348 diff empty;
>   `check-snapshot-drift.sh` PASS (204 snapshots zero drift); `check-docs-truth.py` EXIT=0; 3.16.1
>   four forms control (stdlib surface) E1014 / E1010 / E1111 / allowed.
> - **Uncovered Surface (t16 five items, transcribed as-is)**: ① `testsuite.py run full` (CI stance
>   resource profile channel) not re-run—implementer has fallen back per ruling (exclusive wave peak
>   8.11 GiB > budget 7.51 GiB, exit 2 is tool's designed refusal); this round covered with 4×4 and
>   2×1 two stances' `cargo test --all`. ② Hand-annotated FFI surface (host-FFI) can only be driven
>   via Rust test (`add_native_signature` warehouse zero callers, no CLI surface)—"host embedded
>   process real FFI call" not tested. ③ Variadic (`...`) named argument form no sample constructed
>   (no warehouse criterion sample). ④ `check-boundary.py` / `check-fanout.py` not re-run
>   (implementer already passed; no intersection with behavior criterion). ⑤ CI (GitHub Actions) per
>   convention not waited, handed to closure session check.
>
> **IV. Adversarial Review (t17, reviewed revision `c3082a16051cb1bfb8b84bdc62ec6bd29d90ff2d2`,
> parent = `537b46b7488a60b42cdc35110e0a413c80a58169`; all content with `git show <rev>:<file>`
> taken, line number re-taken on referenced revision)**:
>
> - **① Criterion single source reverse check**: `slot_order_arg_indices` (ir_gen.rs:521-527)
>   **doesn't re-sort by name** (sort key is slot number given by shared criterion, name only
>   appears in `arg_binding::bind_args_to_params`), **doesn't backstop missing parameter** (absent
>   slot doesn't produce argument index at all), **doesn't divide optional position** (IR two places
>   pass all optional, another two pass true value `i >= required_min`). Warehouse "find position by
>   parameter name / find member" grep per item check: **slot calculation only `arg_binding.rs:76`
>   one place**, other 7 hits each belong to other concern (generic type parameter's constraint
>   interface check / const generic argument replacement / generic type variable binding / generic
>   type parameter replacement / const generic parameter lookup / method definition lookup / struct
>   field lookup).
> - **② Responsibility Boundary (missing parameter only loud in typecheck, IR only replays slot
>   binding)**: Typecheck side `bind_named_args` (`expressions.rs:2854`) passes all required ⇒
>   missing any value parameter E1010; native export arm (`:5653`) passes true optional position
>   `i >= required_min` ⇒ only true required position E1010, `?msg` omission allowed. IR side this
>   commit's two changes (`ir_gen.rs:7926` RFC-004 binding path / `:8021` overload resolution method
>   path) **uniformly pass all optional, only replay slot binding** (comment explicitly writes
>   "missing parameter goes to typecheck" and "re-judge missing parameter per declaration table will
>   falsely report E1010 for legal resolution (`t.pick2(a = 2)` measured)").
> - **③ API Form**: environment.rs:990-993 explicitly writes "this function **must not** parse
>   parameter name itself" and points to sole exit `export_signature_param_info`; :997-999 writes
>   "only callers that can get `MonoType` should parse signature string on their own side then go
>   through this API".
> - **④ Behavior Change Surface**: Code four differences (same parameter named twice E1111→E1015
>   code change / unknown name E1014 text change / empty table E1014 text change / positional
>   argument over IR side declaration table: silently discard→send as-is), **measured reachable form
>   no diagnostic code / text change** (typecheck judges first, same code), IR product on 5
>   self-built probes **byte-by-byte consistent** (p1 / p5 go through this commit's changed branch
>   and product unchanged, p2 / p3 / p4 two states same code, p9 unconstructible, p6–p8 source layer
>   unreachable); t15's 8 CLI probes two states per code consistent. All differences fall on
>   **originally failed** programs (programs still fail) ⇒ D59 "doesn't change originally successful
>   program's external performance" not touched.
> - **⑤ Test Quality** (not just red/green): 12 new tests (`typecheck/tests/native_param_names.rs`)
>   per item vs `test-specification`—naming `test_<what>_<scenario>`, three-segment AAA comment,
>   assertion all carry custom message, setup repeated ≥3 times already extracted helper, helper
>   internal panic carries context; **no "only check exit code" weak assertion** (first
>   `assert!(`diagnostic non-empty`)` then `assert_eq!(`per code equal`)`, both sides empty won't
>   pass vacuously); 3 negative cases (legal named call allowed / optional position omission allowed
>   / explicitly give optional position allowed) under M1 red state still pass ⇒ they **only prevent
>   false positives**, exactly their purpose. **M1** (remove `native_param_names` filling) →
>   `cargo test --lib native_param_names` **9 failed / 3 passed**, TEST_EXIT=101 ⇒ new test **really
>   pinned fix**. **M2** (entire segment rollback `ir_gen` merge) → `cargo test --lib` **2806 passed
>   / 0 failed / 2 ignored**, exit 0 ⇒ merge surface **no test coverage**. `pick2` pin (existing
>   `named_args.rs:253-283`, this commit not changed) goes through **real full chain** and asserts
>   **runtime value** `Int(20)` ⇒ **really pinned overload resolution** (not "no error"), and it's
>   live gatekeeper for "IR must not re-judge missing parameter".
> - **⑥ Limitation Registration**: Host native only enters `TypeEnvironment`'s two tables, **doesn't
>   enter `ModuleRegistry`** ⇒ IR generation per registry resolves namespace members, so host native
>   in IR stage inherently unresolvable (E3006), **pre-this-task embedded surface limitation**; this
>   limitation and "why hand-annotated surface must observe in type check stage" written in **test
>   file doc comments** and `environment.rs` field docs (not only in report).
> - **Gates eight items in reviewed revision all re-run all green**: fmt --check / clippy -D
>   warnings / `cargo test --all` (**3220 passed, 0 failed**; lib 2806 + 2 ignored, integration 297)
>   / corpus diff 348-348 zero drift / IR snapshot 204 zero drift / docs-truth / check-boundary (0
>   exemption 0 violation) / check-fanout (new symbol no same-name multi-definition entry).
> - **Four observation items (transcribed as-is: verified, judgment doesn't constitute finding, no
>   need to dispose, but registered to avoid being mistaken for unhandled violation later)**:
>   - **O1**: Commit message claims "after this merge **whole warehouse only has shared criterion
>     one place**"—for **slot binding** holds; but typecheck's **method overload arm** still has a
>     "by name lookup candidate table (E1014) + arity (E1010)" own judgment
>     (`inference/expressions.rs:5578-5600`), it uses **candidate group representative**'s parameter
>     table (`method_overload_param_names`), different from shared criterion's **resolved
>     candidate** table. Judgment basis: this place **existed before this commit, this commit didn't
>     touch** (`git show c3082a160 --name-status` only 4 files, no expressions.rs), and belongs to
>     "candidate matching" concern (different table, different semantics), not slot binding.
>     **Wording too broad, registered as observation**.
>   - **O2**: After `add_native_signature` changed to take signature string, **no**
>     `MonoType → signature string` shared helper ⇒ future only-get-MonoType callers can only each
>     write one serialization. Judgment basis: parameter name / required floor already locked in
>     sole exit `signature::export_signature_param_info` by doc; current **zero callers**; doc
>     already written "only callers that can get `MonoType` should parse signature string on their
>     own side then go through this API" ⇒ residual risk only in "how to construct string" section,
>     **low**.
>   - **O3**: This commit's **merge part** (ir_gen dedup) **no test coverage at all** (M2: entire
>     segment rollback ir_gen then `cargo test --lib` still 2806 passed all green). Judgment basis:
>     "dedup" no new behavior to pin, by nature; "single source" nature guaranteed by grep +
>     `check-fanout.py` gate. **Optional strengthening**: add one **IR layer** case in
>     `native_param_names.rs` (covering ir_gen:7926 / :8021 path), let merge surface also have
>     guardrail.
>   - **O4 (Attribution Correction)**: Commit message and report attribute d2's E1010 to "**method
>     arm** arity pre-check first triggers"—measured this E1010's help is generic text "check the
>     argument count of the function call", **not** shared criterion's "missing parameter" help
>     ("缺少必需形参：…") ⇒ trigger is **native arm**'s arity pre-check (`native_arity` table).
>     **Code behavior conclusion unchanged** (parent commit existing, this round not changed,
>     re-verified), only **attribution wording needs correction**; also "`list.push` duplicate name
>     i.e. E1010" must qualify as "**arity mismatch + duplicate name**"—only duplicate name with
>     matching arity (`list.push(list = v, list = 9)`) two states are **E1015** (duplicate name
>     precedes arity judgment).
>
> **V. Gates and Resource Channel (one external link)**: This task **first truly used**
> `python scripts/dev/testsuite.py run full` → **exit 2**—exclusive wave peak 8.11 GiB > this
> machine's budget 7.51 GiB (gap 618 MiB), tool by design **refuses to silently skip** ⇒ fall back
> `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test --all --quiet` → **exit 0** (3220 passed / 0
> failed). This channel status chain **#486**: `full` unavailable on this machine due to
> conservative peak extrapolation, **calibration (2600 file measured calibration) belongs to WBS
> 0.5.2**.
>
> **VI. Count Table: This Round Not Moved**—`3.17.1` already registered 2026-10-15 and counted in
> table header ("2026-10-15: +2 = 3.17.1 / 3.18.1"), this round only check + note, **three-level
> count unchanged** ⇒ `09` top's secondary 67 / three-level 176 **not moved**. This round **doesn't
> add** count deviation (parent entry 3.17's "1 three-level task" note originally correct); previous
> section "XI"'s 6 pre-existing deviations (parent entry count note lag) **unrelated to this round,
> not introduced by this round**, still kept per that section's stance for account review.
>
> **VII. Batch Tree Recovery (gc and status original output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted verify-p3-t16 (released about 10235.4 MB)` /
> `[scratch] deleted verify-p3-t17 (released about 10079.2 MB)` /
> `[scratch] gc complete, total released about 20314.6 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`; `git worktree list` → `E:/git/YaoXiang c3082a160 [dev]` (only main
> mount left). **Also deleted by gc are two batch trees' internal verification evidence**
> (`verify-p3-t16/`'s `EVIDENCE.md`, `verify-p3-t17/`'s `EVIDENCE-t17.md` and probes / patches /
> original logs)—they are **registered batch directories**, deleted with tree; this section's third,
> fourth have in-placed their measurement numbers, gate values, mutation table and four observation
> items **in-place in record** for archive, conclusions don't become invalid due to file deletion.
> **Unaffected archive** (non-registered batch directories, gc doesn't touch): `wbs317` (implementer
> evidence: evidence-3.17.1.md / red-test.txt / trial-before-after.txt + fix-3171.patch /
> attribution table etc.; under workspace `target/`, **not checked in**; conclusions landed this
> section, workspace path not as reference source). Acceptance statement (resolvable): `git status`
> clean; `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main
> mount; no this-batch new directories outside warehouse.
>
> **VIII. Document Real Build and Commit Hygiene**: This round's check / note / registration
> **accumulated into one** document commit (AGENTS.md clause 8); before commit locally run
> `npm run docs:build` (AGENTS.md clause 10⑨) and `python scripts/ci/check-docs-truth.py`,
> markdownlint zero warnings. **Not push** (members don't push, see AGENTS.md clause 10⑧); **CI /
> Deploy Docs conclusions to unified check session** backfill, not waited in session.
>
> **IX. Carryover and Registration**:
>
> 1. **Round 1 closure registration's carryover ① closed**: `src/driver/arms.rs`'s "Decision B1"
>    comment opposite to post-3.4.6 code fact item, by `537b46b74`
>    (`:memo: docs(driver): correct fact-opposite B1 comment (3.4.6 closing omission)`) rewritten in
>    place to current state fact (behavior neutral, only comment line; wording aligned with
>    `src/driver/tests/program.rs`'s same fact's expression).
> 2. **This round new registration**: O1–O4 four observation items (verified, not blocking,
>    registered for record); 0.5.2 (`full` channel's 2600 file measured calibration + pre-commit /
>    CI wiring); #489 ureq 3 migration and `.cargo/audit.toml` exemption's hard removal condition;
>    3.8 / 3.12 pending ruling, 3.18 prerequisite pending P6—**all not in this team's scope**.

## Document Reference Governance and "Unresolvable Reference" Gate (2026-10-21)

> **Trigger**: When cleaning workspace evidence directories, found that in `09` / `09a` /
> `CHANGELOG` / branch maintenance guide there are 24 places treating "workspace-internal,
> `.gitignore`-excluded directories" (batch tree / probes / evidence directories) as **evidence
> source**—such directories don't exist after clone, external contributors can't verify any one
> number (AGENTS.md meta-rule ① and clause 10④). One of these is 4.4 closure round's fix: changed
> old path to point to another local path, equivalent to moving violation down one layer not
> eliminating.
>
> **I. Reference In-place Inline (47 = zh 24 + en 23)**: Conclusion, numbers, CI run id, delivery
> commit in-place written into this article and `09`, source changed to checked-in path (record
> section anchor) or revision / run id / issue number. CI run id details and S9 seven group list
> **already inlined**, only redundant path pointers deleted. en mirror regenerated by auto-translate
> bot per zh source.
>
> **II. New Mechanical Gate** `scripts/ci/check-docs-refs.py` (hooked into `ci.yml`'s docs job):
> tracks path-like tokens in backticks and markdown links in text files, any hit by `.gitignore`
> with specific sub-path i.e. red; placeholder form (rule statement) and bare directory name not
> judged; system runtime directories (package manager cache, build products, playground publish
> directory, local agent workbench) go through **per-item written reason** exemption table (26;
> details written into gate's own run report—uploaded as artifact in CI, not checked in).
> **Criterion can red can green**: self-test 9 cases (4 counterexample must red / 5 positive must
> green, including cross-batch boundary one); end-to-end mutation verification—knock back one
> violation reference → gate red → restore → green → that file byte-by-byte consistent with
> pre-mutation.
>
> **III. Rule Wording Supplement**: `AGENTS.md` clause 10④ explicitly states "'warehouse-internal
> path' means git-tracked checked-in files", clause 9 supplements "things in workspace are not
> evidence source", clause 10⑤'s local verify list supplements this gate (user 2026-10-21 approved).
>
> **IV. Two Platform Consistency (First Version Gate's Defect and Fix)**: First version gate
> **Windows local green, CI (Linux) red**—`git check-ignore` for "directory-form path ending with /"
> two platform behavior differs (Linux reports real rule, Windows reports one **empty pattern** =
> `.gitignore`'s empty line), only recognizing non-empty pattern Windows side missed
> `docs/public/wasm/` (playground publish directory, RFC-037 reference). Fix: each token check three
> forms (with slash / without slash / without slash + probe subitem), any form hit **non-empty**
> pattern i.e. count as hit. After fix Windows and Linux dual run **same number**: 1122 tracked text
> files / 5533 path-like tokens / 0 violation / 26 exemption / selftest 9/9. Lesson: gate script
> **at least must run on two OSes and checkout forms each** before dare hook CI (CI's job log needs
> admin permission, when can't get use "Linux real clone reproduce" to locate).
>
> **V. CI Evidence (AGENTS clause 13①, scheduled check backfill)**: Fix commit `c7a0dd417` → CI run
> **39626722284 success**; previous commit `fff9580f8` → CI run **39623038087 failure** (red on
> newly added "Check doc references are resolvable", i.e. above platform difference, fixed by
> `c7a0dd417`), same commit Deploy Docs **39623038228 success**, Auto Translate Docs **39623038027
> success**; earlier each commit (`ec2893e0d` / `ff2104b0c` / `ffd43b1d0`) CI run **cancelled** by
> concurrency due to consecutive push (39621547581 / 39620372322 / 39619834870), per "later run's
> conclusion covers previous one" take conclusion.
>
> **V.2. This Section Check-in Commit's CI Evidence (2026-10-22 scheduled check backfill)**: This
> section check-in commit `612c65322` → CI run **39798235412 success** (7 jobs all green, including
> newly added "Check doc references are resolvable" and "Error-code docs up to date" two steps),
> Deploy Docs **39798235484 success**; same commit Auto Translate Docs **39798235413 failure**,
> failed in last step "Commit translations"—translation itself success (Translate documents / Format
> / Lint all success), but push time remote already advanced by this batch's code commit and bot's
> own previous auto-translate commit, `git pull --rebase` + `git push` blocked, **result is this
> article's en mirror not yet contains this section** (zh is sole source; next push touching
> `docs/src/**/*.md` will trigger that workflow to re-translate). Parallel 4 bugfix commits
> (`eb4124cf0` / `14e9b2704` / `96638ec9f` / `06ac6d687`) CI run **39797917813 cancelled**
> (consecutive push cancelled by concurrency), per "later run's conclusion covers previous one" take
> conclusion.
>
> **VI. Registration Stance**: This section is **document governance** record, doesn't add WBS
> kanban row, `09` top count table unchanged.
